//! Extension methods and types for the `Future` trait.

use crate::{deadline::TimeoutError, IntoDeadline};
use core::future::Future;
use core::pin::Pin;

use pin_project_lite::pin_project;
use std::task::{Context, Poll};

pub trait FutureExt: Future {
    /// Applies the token to the `future`, such that the resulting future
    /// completes with `None` if the token is cancelled.
    fn until<T, D>(self, target: T) -> StopFuture<Self, D>
    where
        Self: Sized,
        T: IntoDeadline<Deadline = D>,
    {
        StopFuture {
            deadline: target.into_deadline(),
            future: self,
        }
    }
}

pin_project! {
    #[derive(Debug)]
    pub struct StopFuture<F, D> {
        #[pin]
        future: F,
        #[pin]
        deadline: D,
    }
}

impl<F, D> Future for StopFuture<F, D>
where
    F: Future,
    D: Future<Output = ()>,
{
    type Output = Result<F::Output, TimeoutError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        if let Poll::Ready(()) = this.deadline.poll(cx) {
            return Poll::Ready(Err(TimeoutError::new()));
        }
        match this.future.poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(it) => Poll::Ready(Ok(it)),
        }
    }
}
