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

use std::pin::Pin;
use std::task::{Context, Poll};

use async_std::prelude::*;

use async_std::channel::{self, Receiver, Sender};
use pin_project_lite::pin_project;

enum Never {}

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
pub struct StopSource {
    /// Solely for `Drop`.
    _chan: Sender<Never>,
    stop_token: StopToken,
}

/// `StopToken` is a future which completes when the associated `StopSource` is dropped.
#[derive(Debug, Clone)]
pub struct StopToken {
    chan: Receiver<Never>,
}

impl Default for StopSource {
    fn default() -> StopSource {
        let (sender, receiver) = channel::bounded::<Never>(1);

        StopSource {
            _chan: sender,
            stop_token: StopToken { chan: receiver },
        }
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

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let chan = Pin::new(&mut self.chan);
        match Stream::poll_next(chan, cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Some(never)) => match never {},
            Poll::Ready(None) => Poll::Ready(()),
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
