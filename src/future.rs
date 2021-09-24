//! Extension methods and types for the `Future` trait.

use crate::StopToken;
use core::future::Future;
use core::pin::Pin;

use pin_project_lite::pin_project;
use std::task::{Context, Poll};

pub trait FutureExt: Future {
    /// Applies the token to the `future`, such that the resulting future
    /// completes with `None` if the token is cancelled.
    fn until(self, deadline: StopToken) -> StopFuture<Self>
    where
        Self: Sized,
    {
        StopFuture {
            deadline,
            future: self,
        }
    }
}

pin_project! {
    #[derive(Debug)]
    pub struct StopFuture<F> {
        #[pin]
        deadline: StopToken,
        #[pin]
        future: F,
    }
}

impl<F: Future> Future for StopFuture<F> {
    type Output = Option<F::Output>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<F::Output>> {
        let this = self.project();
        if let Poll::Ready(()) = this.deadline.poll(cx) {
            return Poll::Ready(None);
        }
        match this.future.poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(it) => Poll::Ready(Some(it)),
        }
    }
}
