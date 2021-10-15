//! Extension methods and types for the `Future` trait.

use crate::{deadline::TimedOutError, IntoDeadline};
use core::future::Future;
use core::pin::Pin;

use pin_project_lite::pin_project;
use std::task::{Context, Poll};

/// Extend the `Future` trait with the `until` method.
pub trait FutureExt: Future {
    /// Run a future until it resolves, or until a deadline is hit.
    fn until<T, D>(self, target: T) -> Stop<Self, D>
    where
        Self: Sized,
        T: IntoDeadline<Deadline = D>,
    {
        Stop {
            deadline: target.into_deadline(),
            future: self,
        }
    }
}

impl<F: Future> FutureExt for F {}

pin_project! {
    /// Run a future until it resolves, or until a deadline is hit.
    ///
    /// This method is returned by [`FutureExt::deadline`].
    #[must_use = "Futures do nothing unless polled or .awaited"]
    #[derive(Debug)]
    pub struct Stop<F, D> {
        #[pin]
        future: F,
        #[pin]
        deadline: D,
    }
}

impl<F, D> Future for Stop<F, D>
where
    F: Future,
    D: Future<Output = ()>,
{
    type Output = Result<F::Output, TimedOutError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        if let Poll::Ready(()) = this.deadline.poll(cx) {
            return Poll::Ready(Err(TimedOutError::new()));
        }
        match this.future.poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(it) => Poll::Ready(Ok(it)),
        }
    }
}
