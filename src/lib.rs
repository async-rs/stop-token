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
    atomic::{AtomicBool, AtomicPtr, Ordering},
    Arc,
};
use std::task::{Context, Poll};

use event_listener::{Event, EventListener};
use futures::stream::Stream;
use pin_project_lite::pin_project;

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

#[derive(Debug)]
/// a custom implementation of a CondVar that short-circuits after
/// being signaled once
struct ShortCircuitingCondVar {
    // TODO: is there a better (safer) way to have an atomic `Option`?
    cached_listener: AtomicPtr<EventListener>, // This is a raw pointer to a Box
    event: Event,
    signaled: AtomicBool,
}

impl ShortCircuitingCondVar {
    fn notify(&self, n: usize) {
        // TODO: This can probably be `Relaxed` and `notify_relaxed`
        self.signaled.store(true, Ordering::SeqCst);
        self.event.notify(n);
    }

    fn listen(&self) -> Option<EventListener> {
        // TODO: These could maybe be `Aquire`
        // Check if the `CondVar` has already been triggered
        if self.signaled.load(Ordering::SeqCst) {
            return None; // The `CondVar` has already been triggered so we don't need to wait
        }

        let ptr = self.cached_listener.swap(null_mut(), Ordering::SeqCst);

        let listener = if ptr.is_null() {
            // Register a new listener
            self.event.listen()
        } else {
            // turn the cached listener back into a `EventListener` object

            // Safety: the `cached_listener` is not null and can only come from a Box::into_raw
            // it is also replaced with a `null` pointer when read so there can not be another owner.
            unsafe { *Box::from_raw(ptr) }
        };

        // Make sure the `CondVar` still has not been triggered to prevent race conditions
        if self.signaled.load(Ordering::SeqCst) {
            return None;
        }

        Some(listener)
    }

    fn cache_listener(&self, listener: EventListener) -> Result<(), EventListener> {
        // Check if there is already a cached listener
        if self.cached_listener.load(Ordering::SeqCst).is_null() {
            let listener = Box::new(listener);

            unsafe {
                let res = self.cached_listener.compare_and_swap(
                    null_mut(),
                    Box::into_raw(listener),
                    Ordering::SeqCst,
                );
                if res.is_null() {
                    Ok(())
                } else {
                    // We failed to write our new cached listener due to a race
                    // Turn it back into a Box and return it to the caller

                    // Safety: the `cached_listener` is not null and can only come from a Box::into_raw
                    // it is also replaced with a `null` pointer when read so there can not be another owner.
                    Err(*Box::from_raw(res))
                }
            }
        } else {
            Err(listener)
        }
    }
}

#[derive(Debug)]
pub struct StopSource {
    signal: Arc<ShortCircuitingCondVar>,
    stop_token: StopToken,
}

/// `StopToken` is a future which completes when the associated `StopSource` is dropped.
#[derive(Debug, Clone)]
pub struct StopToken(Arc<ShortCircuitingCondVar>);

impl Default for StopSource {
    fn default() -> StopSource {
        let signal = Arc::new(ShortCircuitingCondVar {
            cached_listener: AtomicPtr::new(null_mut()),
            event: Event::new(),
            signaled: AtomicBool::new(false),
        });
        StopSource {
            signal: signal.clone(),
            stop_token: StopToken(signal),
        }
    }
}

impl Drop for StopSource {
    fn drop(&mut self) {
        // TODO: notifying only one StopToken should be sufficient
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
        self.stop_token.clone()
    }
}

impl Future for StopToken {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if let Some(mut listener) = self.0.listen() {
            let result = match Future::poll(Pin::new(&mut listener), cx) {
                Poll::Pending => Poll::Pending,
                Poll::Ready(_) => Poll::Ready(()),
            };

            // Try to cache the listener, if there already is a cached listener
            // drop the one we have
            let _ = self.0.cache_listener(listener);

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
