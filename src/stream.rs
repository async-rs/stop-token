//! Extension methods and types for the `Stream` trait.

use crate::{deadline::TimedOutError, Deadline, IntoDeadline};
use core::future::Future;
use core::pin::Pin;

use futures_core::Stream;
use pin_project_lite::pin_project;
use std::task::{Context, Poll};

/// Extend the `Stream` trait with the `until` method.
pub trait StreamExt: Stream {
    /// Applies the token to the `stream`, such that the resulting stream
    /// produces no more items once the token becomes cancelled.
    fn until<T>(self, target: T) -> Stop<Self>
    where
        Self: Sized,
        T: IntoDeadline,
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
    pub struct Stop<S> {
        #[pin]
        stream: S,
        #[pin]
        deadline: Deadline,
    }
}

impl<S> Stop<S> {
    /// Unwraps this `Stop` stream, returning the underlying stream.
    pub fn into_inner(self) -> S {
        self.stream
    }
}

impl<S> Stream for Stop<S>
where
    S: Stream,
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
