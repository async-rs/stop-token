//! Create deadlines from `Duration` and `Instant` types.
//!
//! # Features
//!
//! This module is empty when no features are enabled. To implement deadlines
//! for `Instant` and `Duration` you can enable one of the following features:
//!
//! - `async-io`: use this when using the `async-std` or `smol` runtimes.
//! - `tokio`: use this when using the `tokio` runtime.
//!
//! # Examples

use std::future::{pending, Future, Pending};
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::time::{timeout_at, Instant as TokioInstant, Timeout};

use crate::IntoDeadline;

/// A future that times out after a duration of time.
#[must_use = "Futures do nothing unless polled or .awaited"]
#[derive(Debug)]
pub(crate) struct Deadline {
    instant: TokioInstant,
    delay: Pin<Box<Timeout<Pending<()>>>>,
}

impl Clone for Deadline {
    fn clone(&self) -> Self {
        let instant = self.instant.clone();
        Self {
            instant,
            delay: Box::pin(timeout_at(instant, pending())),
        }
    }
}

impl Future for Deadline {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match Pin::new(&mut self.delay).poll(cx) {
            Poll::Ready(_) => Poll::Ready(()),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl IntoDeadline for tokio::time::Instant {
    fn into_deadline(self) -> crate::Deadline {
        let instant = TokioInstant::from(self);
        let deadline = Deadline {
            instant,
            delay: Box::pin(timeout_at(instant, pending())),
        };

        crate::Deadline {
            kind: crate::deadline::DeadlineKind::Tokio { t: deadline },
        }
    }
}
