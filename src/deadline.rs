use std::future::Future;

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
