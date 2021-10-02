use core::fmt;
use std::{error::Error, future::Future, io};

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

/// Conversion into a deadline.
///
/// A deadline is a future which resolves after a certain period or event.
pub trait IntoDeadline {
    /// Which kind of future are we turning this into?
    type Deadline: Future<Output = ()>;

    /// Creates a deadline from a value.
    fn into_deadline(self) -> Self::Deadline;
}
