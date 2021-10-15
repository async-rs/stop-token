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

use async_io::Timer;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Instant;

use pin_project_lite::pin_project;

pin_project! {
    /// A future that times out after a duration of time.
    #[must_use = "Futures do nothing unless polled or .awaited"]
    #[derive(Debug)]
    pub(crate) struct Deadline {
        instant: Instant,
        #[pin]
        delay: Timer,
    }
}

impl Clone for Deadline {
    fn clone(&self) -> Self {
        Self {
            instant: self.instant,
            delay: Timer::at(self.instant),
        }
    }
}

impl Future for Deadline {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        match this.delay.poll(cx) {
            Poll::Ready(_) => Poll::Ready(()),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl Into<crate::Deadline> for std::time::Duration {
    fn into(self) -> crate::Deadline {
        let instant = Instant::now() + self;

        let deadline = Deadline {
            instant,
            delay: Timer::after(self),
        };
        crate::Deadline {
            kind: crate::deadline::DeadlineKind::AsyncIo { t: deadline },
        }
    }
}

impl Into<crate::Deadline> for std::time::Instant {
    fn into(self) -> crate::Deadline {
        let deadline = Deadline {
            instant: self,
            delay: Timer::at(self),
        };
        crate::Deadline {
            kind: crate::deadline::DeadlineKind::AsyncIo { t: deadline },
        }
    }
}
