use core::fmt;
use std::{error::Error, future::Future};

/// An error returned when a future times out.
#[derive(Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
pub struct TimeoutError {
    _private: (),
}

impl fmt::Debug for TimeoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TimeoutError").finish()
    }
}

impl TimeoutError {
    pub(crate) fn new() -> Self {
        Self { _private: () }
    }
}

impl Error for TimeoutError {}

impl fmt::Display for TimeoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        "Future has timed out".fmt(f)
    }
}

/// Conversion into a deadline.
///
/// A deadline is a future which elapses after a certain period or event, and
/// returns `()`.
pub trait IntoDeadline {
    /// Which kind of future are we turning this into?
    type Deadline: Future<Output = ()>;

    /// Creates a deadline from a value.
    fn into_deadline(self) -> Self::Deadline;
}
