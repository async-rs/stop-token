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

#[cfg(feature = "async-io")]
pub use asyncio::*;

#[cfg(any(feature = "async-io", feature = "docs"))]
mod asyncio {
    use async_io::Timer;
    use std::future::Future;
    use std::pin::Pin;
    use std::task::{Context, Poll};
    use std::time::Instant;

    use crate::IntoDeadline;

    use pin_project_lite::pin_project;

    pin_project! {
        /// A future that times out after a duration of time.
        #[must_use = "Futures do nothing unless polled or .awaited"]
        #[derive(Debug)]
        pub struct Deadline {
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

    impl IntoDeadline for std::time::Duration {
        type Deadline = Deadline;

        fn into_deadline(self) -> Self::Deadline {
            let instant = Instant::now() + self;
            Deadline {
                instant,
                delay: Timer::after(self),
            }
        }
    }

    impl IntoDeadline for std::time::Instant {
        type Deadline = Deadline;

        fn into_deadline(self) -> Self::Deadline {
            Deadline {
                instant: self,
                delay: Timer::at(self),
            }
        }
    }
}

#[cfg(feature = "tokio")]
pub use tokiooo::*;

#[cfg(feature = "tokio")]
mod tokiooo {
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
}
