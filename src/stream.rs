//! Extension methods and types for the `Stream` trait.

use crate::IntoDeadline;
use core::future::Future;
use core::pin::Pin;

use futures_core::Stream;
use pin_project_lite::pin_project;
use std::task::{Context, Poll};

pub trait StreamExt: Stream {
    /// Applies the token to the `stream`, such that the resulting stream
    /// produces no more items once the token becomes cancelled.
    fn until<T, D>(self, target: T) -> StopStream<Self, D>
    where
        Self: Sized,
        T: IntoDeadline<Deadline = D>,
    {
        StopStream {
            stream: self,
            deadline: target.into_deadline(),
        }
    }
}

impl<S: Stream> StreamExt for S {}

pin_project! {
    #[derive(Debug)]
    pub struct StopStream<S, D> {
        #[pin]
        stream: S,
        #[pin]
        deadline: D,
    }
}

impl<S, D> Stream for StopStream<S, D>
where
    S: Stream,
    D: Future<Output = ()>,
{
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        if let Poll::Ready(()) = this.deadline.poll(cx) {
            return Poll::Ready(None);
        }
        this.stream.poll_next(cx)
    }
}
