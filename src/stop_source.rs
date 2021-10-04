use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

use async_channel::{bounded, Receiver, Sender};
use futures_core::stream::Stream;

enum Never {}

/// `StopSource` produces `StopToken` and cancels all of its tokens on drop.
///
/// # Example:
///
/// ```ignore
/// let source = StopSource::new();
/// let token = source.token();
/// schedule_some_work(token);
/// drop(source); // At this point, scheduled work notices that it is canceled.
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
    pub fn token(&self) -> StopToken {
        self.stop_token.clone()
    }
}

impl super::IntoDeadline for StopToken {
    type Deadline = Self;

    fn into_deadline(self) -> Self::Deadline {
        self
    }
}

impl Future for StopToken {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let chan = Pin::new(&mut self.chan);
        match Stream::poll_next(chan, cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Some(never)) => match never {},
            Poll::Ready(None) => Poll::Ready(()),
        }
    }
}
