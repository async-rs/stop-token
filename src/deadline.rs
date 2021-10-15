use core::fmt;
use std::{
    error::Error,
    future::Future,
    io,
    pin::Pin,
    task::{Context, Poll},
};

use crate::StopToken;

/// An error returned when a future times out.
#[derive(Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
pub struct TimedOutError {
    _private: (),
}

impl fmt::Debug for TimedOutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TimeoutError").finish()
    }
}

impl TimedOutError {
    pub(crate) fn new() -> Self {
        Self { _private: () }
    }
}

impl Error for TimedOutError {}

impl Into<io::Error> for TimedOutError {
    fn into(self) -> io::Error {
        io::Error::new(io::ErrorKind::TimedOut, "Future has timed out")
    }
}

impl fmt::Display for TimedOutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        "Future has timed out".fmt(f)
    }
}

pin_project_lite::pin_project! {
    /// A future that times out after a duration of time.
    #[must_use = "Futures do nothing unless polled or .awaited"]
    #[derive(Debug)]
    pub struct Deadline {
        #[pin]
        pub(crate) kind: DeadlineKind,
    }
}

cfg_if::cfg_if! {
    if #[cfg(all(feature = "tokio", feature = "async-io"))] {
        pin_project_lite::pin_project! {
            #[project = DeadlineKindProj]
            #[derive(Debug)]
            pub(crate) enum DeadlineKind {
                StopToken{ #[pin]t: StopToken},
                Tokio{#[pin]t: crate::tokio::Deadline},
                AsyncIo{#[pin]t: crate::async_io::Deadline},
            }
        }
    } else if #[cfg(feature = "tokio")] {
        pin_project_lite::pin_project! {
            #[project = DeadlineKindProj]
            #[derive(Debug)]
            pub(crate) enum DeadlineKind {
                StopToken{ #[pin]t: StopToken},
                Tokio{#[pin]t: crate::tokio::Deadline},
            }
        }
    } else if #[cfg(feature = "async-io")] {
        pin_project_lite::pin_project! {
            #[project = DeadlineKindProj]
            #[derive(Debug)]
            pub(crate) enum DeadlineKind {
                StopToken{ #[pin]t: StopToken},
                AsyncIo{#[pin]t: crate::async_io::Deadline},
            }
        }
    } else {
        pin_project_lite::pin_project! {
            #[project = DeadlineKindProj]
            #[derive(Debug)]
            pub(crate) enum DeadlineKind {
                StopToken{ #[pin]t: StopToken},
            }
        }
    }
}

impl Future for Deadline {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.project().kind.project() {
            DeadlineKindProj::StopToken { t } => t.poll(cx),
            #[cfg(feature = "tokio")]
            DeadlineKindProj::Tokio { t } => t.poll(cx),
            #[cfg(feature = "async-io")]
            DeadlineKindProj::AsyncIo { t } => t.poll(cx),
        }
    }
}
