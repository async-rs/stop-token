use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

use async_channel::{bounded, Receiver, Sender};
use futures_core::stream::Stream;
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
        let (sender, receiver) = bounded::<Never>(1);

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
