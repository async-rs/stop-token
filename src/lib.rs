//! Cooperative cancellation for [async-std](https://async.rs/).
//!
//! # Status
//!
//! Experimental. The library works as is, breaking changes will bump major
//! version, but there are no guarantees of long-term support.
//!
//! Additionally, this library uses unstable cargo feature feature of `async-std` and, for
//! this reason, should be used like this:
//!
//! ```toml
//! [dependencies.stop-token]
//! version = "0.1.0"
//! features = [ "unstable" ]
//! ```
//!
//! # Motivation
//!
//! Rust futures come with a build-in cancellation mechanism: dropping a future
//! prevents any further progress of the future. This is a *hard* cancellation
//! mechanism, meaning that the future can potentially be cancelled at any
//! `.await` expression.
//!
//! Sometimes, you need are more fine-grained cancellation mechanism. Imagine a
//! chat server that relays messages to peers. Sending a single message
//! potentially needs several writes on the socket object. That means that, if
//! we use hard-cancellation for client connections, a connection can be
//! abruptly terminated mid-message (even mid-emoji, if we are especially
//! unlucky). What we need here is cooperative cancellation: client connection
//! should be gracefully shutdown *between* the messages.
//!
//! More generally, if you have an event processing loop like
//!
//! ```ignore
//! while let Some(event) = work.next().await {
//!     process_event(event).await
//! }
//! ```
//!
//! you usually want to maintain an invariant that each event is either fully
//! processed or not processed at all. If you need to terminate this loop early,
//! you want to do this *between* iteration.
//!
//! # Usage
//!
//! You can use `stop_token` for this:
//!
//! ```
//! use async_std::prelude::*;
//! use stop_token::StopToken;
//!
//! struct Event;
//!
//! async fn do_work(work: impl Stream<Item = Event> + Unpin, stop_token: StopToken) {
//!     let mut work = stop_token.stop_stream(work);
//!     while let Some(event) = work.next().await {
//!         process_event(event).await
//!     }
//! }
//!
//! async fn process_event(_event: Event) {
//! }
//! ```
//!
//! # Lineage
//!
//! The cancellation system is a subset of `C#` [`CancellationToken / CancellationTokenSource`](https://docs.microsoft.com/en-us/dotnet/standard/threading/cancellation-in-managed-threads).
//! The `StopToken / StopTokenSource` terminology is borrowed from C++ paper P0660: https://wg21.link/p0660.

use std::future::Future;
use std::pin::Pin;
use std::ptr::null_mut;
use std::sync::{
    atomic::{AtomicPtr, Ordering},
    Arc,
};

use event_listener::{Event, EventListener};
use futures::stream::Stream;
use pin_project_lite::pin_project;
use std::task::{Context, Poll};

/// `StopSource` produces `StopToken` and cancels all of its tokens on drop.
///
/// # Example:
///
/// ```ignore
/// let stop_source = StopSource::new();
/// let stop_token = stop_source.stop_token();
/// schedule_some_work(stop_token);
/// drop(stop_source); // At this point, scheduled work notices that it is canceled.
/// ```

/// An immutable, atomic option type that store data in Boxed Arcs
struct AtomicOption<T>(AtomicPtr<Arc<T>>);

// TODO: relax orderings on atomic accesses
impl<T> AtomicOption<T> {
    fn is_none(&self) -> bool {
        self.0.load(Ordering::SeqCst).is_null()
    }

    #[allow(dead_code)]
    fn is_some(&self) -> bool {
        !self.is_none()
    }

    fn get(&self) -> Option<Arc<T>> {
        let ptr = self.0.load(Ordering::SeqCst);
        if ptr.is_null() {
            None
        } else {
            // Safety: we know that `ptr` is not null and can only have been created from a `Box` by `new` or `replace`
            // this means it's safe to turn back into a `Box`
            let arc_box = unsafe { Box::from_raw(ptr as *mut Arc<T>) };

            let arc = *arc_box.clone(); // Clone the Arc

            Box::leak(arc_box); // And make sure rust doesn't drop our inner value

            Some(arc)
        }
    }

    fn new(value: Option<T>) -> Self {
        let ptr = if let Some(value) = value {
            Box::into_raw(Box::new(Arc::new(value)))
        } else {
            null_mut()
        };

        Self(AtomicPtr::new(ptr))
    }

    fn take(&self) -> Option<Arc<T>> {
        self.replace(None)
    }

    fn replace(&self, new: Option<T>) -> Option<Arc<T>> {
        let new_ptr = if let Some(new) = new {
            Box::into_raw(Box::new(Arc::new(new)))
        } else {
            null_mut()
        };

        let ptr = self.0.swap(new_ptr, Ordering::SeqCst);

        if ptr.is_null() {
            None
        } else {
            // Safety: we know that `ptr` is not null and can only have been created from a `Box` by `new` or `replace`
            // this means it's safe to turn back into a `Box`
            Some(unsafe { *Box::from_raw(ptr) })
        }
    }
}

impl<T> Drop for AtomicOption<T> {
    fn drop(&mut self) {
        std::mem::drop(self.take());
    }
}

impl<T> std::fmt::Debug for AtomicOption<T>
where
    T: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_none() {
            write!(f, "None")
        } else {
            write!(f, "Some(<Opaque>)")
        }
    }
}

/// a custom implementation of a CondVar that short-circuits after
/// being signaled once
#[derive(Debug)]
struct ShortCircuitingCondVar(AtomicOption<Event>);

impl ShortCircuitingCondVar {
    fn is_done(&self) -> bool {
        self.0.is_none()
    }

    fn notify(&self, n: usize) -> bool {
        self.0.take().map(|x| x.notify(n)).is_some()
    }

    fn listen(&self) -> Option<EventListener> {
        self.0.get().map(|event| event.listen())
    }
}

#[derive(Debug)]
pub struct StopSource {
    signal: Arc<ShortCircuitingCondVar>,
}

/// `StopToken` is a future which completes when the associated `StopSource` is dropped.
#[derive(Debug)]
pub struct StopToken {
    cond_var: Arc<ShortCircuitingCondVar>,
    cached_listener: Option<EventListener>,
}

impl StopToken {
    fn new(cond_var: Arc<ShortCircuitingCondVar>) -> Self {
        Self {
            cond_var,
            cached_listener: None,
        }
    }

    fn listen(&mut self) -> Option<&mut EventListener> {
        if self.cond_var.is_done() {
            return None;
        }

        if self.cached_listener.is_none() {
            self.cached_listener = self.cond_var.listen();
        }
        self.cached_listener.as_mut()
    }
}

impl Clone for StopToken {
    fn clone(&self) -> Self {
        Self::new(self.cond_var.clone())
    }
}

impl Default for StopSource {
    fn default() -> StopSource {
        StopSource {
            signal: Arc::new(ShortCircuitingCondVar(AtomicOption::new(
                Some(Event::new()),
            ))),
        }
    }
}

impl Drop for StopSource {
    fn drop(&mut self) {
        self.signal.notify(usize::MAX);
    }
}

impl StopSource {
    /// Creates a new `StopSource`.
    pub fn new() -> StopSource {
        StopSource::default()
    }

    /// Produces a new `StopToken`, associated with this source.
    ///
    /// Once the source is destroyed, `StopToken` future completes.
    pub fn stop_token(&self) -> StopToken {
        StopToken::new(self.signal.clone())
    }
}

impl Future for StopToken {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if let Some(mut listener) = self.listen() {
            let result = match Future::poll(Pin::new(&mut listener), cx) {
                Poll::Pending => Poll::Pending,
                Poll::Ready(_) => Poll::Ready(()),
            };

            return result;
        } else {
            Poll::Ready(())
        }
    }
}

impl StopToken {
    /// Applies the token to the `stream`, such that the resulting stream
    /// produces no more items once the token becomes cancelled.
    pub fn stop_stream<S: Stream>(&self, stream: S) -> StopStream<S> {
        StopStream {
            stop_token: self.clone(),
            stream,
        }
    }

    /// Applies the token to the `future`, such that the resulting future
    /// completes with `None` if the token is cancelled.
    pub fn stop_future<F: Future>(&self, future: F) -> StopFuture<F> {
        StopFuture {
            stop_token: self.clone(),
            future,
        }
    }
}

pin_project! {
    #[derive(Debug)]
    pub struct StopStream<S> {
        #[pin]
        stop_token: StopToken,
        #[pin]
        stream: S,
    }
}

impl<S: Stream> Stream for StopStream<S> {
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        if let Poll::Ready(()) = this.stop_token.poll(cx) {
            return Poll::Ready(None);
        }
        this.stream.poll_next(cx)
    }
}

pin_project! {
    #[derive(Debug)]
    pub struct StopFuture<F> {
        #[pin]
        stop_token: StopToken,
        #[pin]
        future: F,
    }
}

impl<F: Future> Future for StopFuture<F> {
    type Output = Option<F::Output>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<F::Output>> {
        let this = self.project();
        if let Poll::Ready(()) = this.stop_token.poll(cx) {
            return Poll::Ready(None);
        }
        match this.future.poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(it) => Poll::Ready(Some(it)),
        }
    }
}
