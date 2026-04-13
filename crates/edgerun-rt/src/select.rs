//! `select!` macro — race multiple futures, take the first to complete.
//!
//! This macro polls futures concurrently on the calling task (no spawning),
//! avoiding the deadlock that occurs when `blocking_recv` ties up worker threads.

/// Race two futures concurrently and return the result of the first
/// one to complete.
///
/// Both futures must have the same output type. The losing future is
/// dropped (its result is discarded and any side effects are aborted).
///
/// # Panics
/// If called outside a runtime.
#[macro_export]
macro_rules! select {
    ($fut1:expr, $fut2:expr $(,)?) => {{
        let f1 = $fut1;
        let f2 = $fut2;
        $crate::select_internal::select2(f1, f2).await
    }};
}

/// Internal module — do not use directly.
#[doc(hidden)]
pub mod select_internal {
    use std::future::Future;
    use std::marker::PhantomData;
    use std::pin::Pin;
    use std::task::{Context, Poll};

    struct Select2<F1, F2, O> {
        f1: Option<F1>,
        f2: Option<F2>,
        _output: PhantomData<O>,
    }

    impl<F1, F2, O> Future for Select2<F1, F2, O>
    where
        F1: Future<Output = O>,
        F2: Future<Output = O>,
    {
        type Output = O;

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let this = unsafe { self.get_unchecked_mut() };

            if let Some(f) = this.f1.as_mut() {
                if let Poll::Ready(v) = unsafe { Pin::new_unchecked(f) }.poll(cx) {
                    this.f1 = None;
                    this.f2 = None; // drop the loser
                    return Poll::Ready(v);
                }
            }
            if let Some(f) = this.f2.as_mut() {
                if let Poll::Ready(v) = unsafe { Pin::new_unchecked(f) }.poll(cx) {
                    this.f2 = None;
                    this.f1 = None; // drop the loser
                    return Poll::Ready(v);
                }
            }

            Poll::Pending
        }
    }

    pub async fn select2<F1, F2, O>(f1: F1, f2: F2) -> O
    where
        F1: Future<Output = O>,
        F2: Future<Output = O>,
    {
        Select2 { f1: Some(f1), f2: Some(f2), _output: PhantomData }.await
    }
}
