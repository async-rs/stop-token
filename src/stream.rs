//! Extension methods and types for the `Stream` trait.

use crate::{deadline::TimedOutError, IntoDeadline};
use core::future::Future;
use core::pin::Pin;

use futures_core::Stream;
use pin_project_lite::pin_project;
use std::task::{Context, Poll};

/// Extend the `Stream` trait with the `until` method.
pub trait StreamExt: Stream {
    /// Applies the token to the `stream`, such that the resulting stream
    /// produces no more items once the token becomes cancelled.
    fn until<T, D>(self, target: T) -> Stop<Self, D>
    where
        Self: Sized,
        T: IntoDeadline<Deadline = D>,
    {
        Stop {
            stream: self,
            deadline: target.into_deadline(),
        }
    }
}

impl<S: Stream> StreamExt for S {}

pin_project! {
    /// Run a future until it resolves, or until a deadline is hit.
    ///
    /// This method is returned by [`FutureExt::deadline`].
    #[must_use = "Futures do nothing unless polled or .awaited"]
    #[derive(Debug)]
    pub struct Stop<S, D> {
        #[pin]
        stream: S,
        #[pin]
        deadline: D,
    }
}

impl<S, D> Stop<S, D> {
    /// Unwraps this `Stop` stream, returning the underlying stream.
    pub fn into_inner(self) -> S {
        self.stream
    }
}

impl<S, D> Stream for Stop<S, D>
where
    S: Stream,
    D: Future<Output = ()>,
{
    type Item = Result<S::Item, TimedOutError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        if let Poll::Ready(()) = this.deadline.poll(cx) {
            return Poll::Ready(Some(Err(TimedOutError::new())));
        }
        this.stream.poll_next(cx).map(|el| el.map(|el| Ok(el)))
    }
}
