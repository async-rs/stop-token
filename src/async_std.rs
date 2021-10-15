use async_std::task::JoinHandle;

/// Extend the `Task` type `until` method.
pub trait TaskExt {
    /// Run a future until it resolves, or until a deadline is hit.
    fn until<T>(self, target: T) -> Until<Self>
    where
        Self: Sized,
        T: Into<Deadline>,
    {
        Until {
            deadline: target.into(),
            join_handle: self,
        }
    }
}

impl<T> JoinHandleExt<T> for JoinHandle<T> {}

pin_project! {
    /// Run a future until it resolves, or until a deadline is hit.
    ///
    /// This method is returned by [`FutureExt::deadline`].
    #[must_use = "Futures do nothing unless polled or .awaited"]
    #[derive(Debug)]
    pub struct Until<F> {
        #[pin]
        futur_handlee: F,
        #[pin]
        deadline: Deadline,
    }
}

impl<F> Future for Until<F>
where
    F: Future,
{
    type Output = Result<F::Output, TimedOutError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        if let Poll::Ready(()) = this.deadline.poll(cx) {
            let _fut = this.join_handle.cancel();
            return Poll::Ready(Err(TimedOutError::new()));
        }
        match this.join_handle.poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(it) => Poll::Ready(Ok(it)),
        }
    }
}
