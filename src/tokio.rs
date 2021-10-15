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
//!
//! ```
//! use std::time::Instant;
//! use async_std::prelude::*;
//! use stop_token::prelude::*;
//! use stop_token::StopToken;
//!
//! struct Event;
//!
//! async fn do_work(work: impl Stream<Item = Event> + Unpin, until: Instant) {
//!     let mut work = work.until(until);
//!     while let Some(Ok(event)) = work.next().await {
//!         process_event(event).await
//!     }
//! }
//!
//! async fn process_event(_event: Event) {
//! }
//! ```

use std::future::{pending, Future, Pending};
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::time::{timeout, timeout_at, Instant as TokioInstant, Timeout};

use crate::IntoDeadline;

/// A future that times out after a duration of time.
#[must_use = "Futures do nothing unless polled or .awaited"]
#[derive(Debug)]
pub struct Deadline {
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

impl IntoDeadline for std::time::Duration {
    type Deadline = Deadline;

    fn into_deadline(self) -> Self::Deadline {
        let instant = std::time::Instant::now() + self;
        Deadline {
            instant: instant.into(),
            delay: Box::pin(timeout(self, pending())),
        }
    }
}

impl IntoDeadline for std::time::Instant {
    type Deadline = Deadline;

    fn into_deadline(self) -> Self::Deadline {
        let instant = TokioInstant::from(self);
        Deadline {
            instant,
            delay: Box::pin(timeout_at(instant, pending())),
        }
    }
}
