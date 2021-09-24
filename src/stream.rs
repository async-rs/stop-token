//! Extension methods and types for the `Stream` trait.

use crate::StopToken;
use core::future::Future;
use core::pin::Pin;

use futures_core::Stream;
use pin_project_lite::pin_project;
use std::task::{Context, Poll};

pub trait StreamExt: Stream {
    /// Applies the token to the `stream`, such that the resulting stream
    /// produces no more items once the token becomes cancelled.
    fn until(self, deadline: StopToken) -> StopStream<Self>
    where
        Self: Sized,
    {
        StopStream {
            stop_token: deadline,
            stream: self,
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
