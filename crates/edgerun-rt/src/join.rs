//! `join!` macro — await multiple futures concurrently.

/// Await multiple futures concurrently and return a tuple of their results.
///
/// All futures are awaited in parallel (no spawning) — the macro
/// uses `poll_fn` to poll each future concurrently on the calling task.
/// This avoids the deadlock that occurs when `blocking_recv` ties up
/// a worker thread while waiting for other spawned tasks.
///
/// # Panics
/// If called outside a runtime.
#[macro_export]
macro_rules! join {
    ($fut:expr $(,)?) => {
        $fut.await
    };

    ($fut1:expr, $fut2:expr $(,)?) => {{
        let f1 = $fut1;
        let f2 = $fut2;
        $crate::join_internal::join2(f1, f2).await
    }};

    ($fut1:expr, $fut2:expr, $fut3:expr $(,)?) => {{
        let f1 = $fut1;
        let f2 = $fut2;
        let f3 = $fut3;
        $crate::join_internal::join3(f1, f2, f3).await
    }};

    ($fut1:expr, $fut2:expr, $fut3:expr, $fut4:expr $(,)?) => {{
        let f1 = $fut1;
        let f2 = $fut2;
        let f3 = $fut3;
        let f4 = $fut4;
        $crate::join_internal::join4(f1, f2, f3, f4).await
    }};
}

/// Internal module — do not use directly.
#[doc(hidden)]
pub mod join_internal {
    use std::future::Future;
    use std::pin::Pin;
    use std::task::{Context, Poll};

    struct Join2<F1: Future, F2: Future> {
        f1: Option<F1>,
        f2: Option<F2>,
        out1: Option<F1::Output>,
        out2: Option<F2::Output>,
    }

    impl<F1: Future, F2: Future> Future for Join2<F1, F2> {
        type Output = (F1::Output, F2::Output);

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let this = unsafe { self.get_unchecked_mut() };
            if let Some(f) = this.f1.as_mut() {
                if let Poll::Ready(v) = unsafe { Pin::new_unchecked(f) }.poll(cx) {
                    this.out1 = Some(v);
                    this.f1 = None;
                }
            }
            if let Some(f) = this.f2.as_mut() {
                if let Poll::Ready(v) = unsafe { Pin::new_unchecked(f) }.poll(cx) {
                    this.out2 = Some(v);
                    this.f2 = None;
                }
            }
            if this.out1.is_some() && this.out2.is_some() {
                Poll::Ready((this.out1.take().unwrap(), this.out2.take().unwrap()))
            } else {
                Poll::Pending
            }
        }
    }

    pub async fn join2<F1: Future, F2: Future>(f1: F1, f2: F2) -> (F1::Output, F2::Output) {
        Join2 { f1: Some(f1), f2: Some(f2), out1: None, out2: None }.await
    }

    struct Join3<F1: Future, F2: Future, F3: Future> {
        f1: Option<F1>, f2: Option<F2>, f3: Option<F3>,
        out1: Option<F1::Output>, out2: Option<F2::Output>, out3: Option<F3::Output>,
    }

    impl<F1: Future, F2: Future, F3: Future> Future for Join3<F1, F2, F3> {
        type Output = (F1::Output, F2::Output, F3::Output);

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let this = unsafe { self.get_unchecked_mut() };
            if let Some(f) = this.f1.as_mut() {
                if let Poll::Ready(v) = unsafe { Pin::new_unchecked(f) }.poll(cx) {
                    this.out1 = Some(v); this.f1 = None;
                }
            }
            if let Some(f) = this.f2.as_mut() {
                if let Poll::Ready(v) = unsafe { Pin::new_unchecked(f) }.poll(cx) {
                    this.out2 = Some(v); this.f2 = None;
                }
            }
            if let Some(f) = this.f3.as_mut() {
                if let Poll::Ready(v) = unsafe { Pin::new_unchecked(f) }.poll(cx) {
                    this.out3 = Some(v); this.f3 = None;
                }
            }
            if this.out1.is_some() && this.out2.is_some() && this.out3.is_some() {
                Poll::Ready((
                    this.out1.take().unwrap(),
                    this.out2.take().unwrap(),
                    this.out3.take().unwrap(),
                ))
            } else {
                Poll::Pending
            }
        }
    }

    pub async fn join3<F1: Future, F2: Future, F3: Future>(f1: F1, f2: F2, f3: F3) -> (F1::Output, F2::Output, F3::Output) {
        Join3 { f1: Some(f1), f2: Some(f2), f3: Some(f3), out1: None, out2: None, out3: None }.await
    }

    struct Join4<F1: Future, F2: Future, F3: Future, F4: Future> {
        f1: Option<F1>, f2: Option<F2>, f3: Option<F3>, f4: Option<F4>,
        out1: Option<F1::Output>, out2: Option<F2::Output>,
        out3: Option<F3::Output>, out4: Option<F4::Output>,
    }

    impl<F1: Future, F2: Future, F3: Future, F4: Future> Future for Join4<F1, F2, F3, F4> {
        type Output = (F1::Output, F2::Output, F3::Output, F4::Output);

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let this = unsafe { self.get_unchecked_mut() };
            if let Some(f) = this.f1.as_mut() {
                if let Poll::Ready(v) = unsafe { Pin::new_unchecked(f) }.poll(cx) {
                    this.out1 = Some(v); this.f1 = None;
                }
            }
            if let Some(f) = this.f2.as_mut() {
                if let Poll::Ready(v) = unsafe { Pin::new_unchecked(f) }.poll(cx) {
                    this.out2 = Some(v); this.f2 = None;
                }
            }
            if let Some(f) = this.f3.as_mut() {
                if let Poll::Ready(v) = unsafe { Pin::new_unchecked(f) }.poll(cx) {
                    this.out3 = Some(v); this.f3 = None;
                }
            }
            if let Some(f) = this.f4.as_mut() {
                if let Poll::Ready(v) = unsafe { Pin::new_unchecked(f) }.poll(cx) {
                    this.out4 = Some(v); this.f4 = None;
                }
            }
            if this.out1.is_some() && this.out2.is_some()
                && this.out3.is_some() && this.out4.is_some() {
                Poll::Ready((
                    this.out1.take().unwrap(), this.out2.take().unwrap(),
                    this.out3.take().unwrap(), this.out4.take().unwrap(),
                ))
            } else {
                Poll::Pending
            }
        }
    }

    pub async fn join4<F1: Future, F2: Future, F3: Future, F4: Future>(f1: F1, f2: F2, f3: F3, f4: F4)
        -> (F1::Output, F2::Output, F3::Output, F4::Output)
    {
        Join4 { f1: Some(f1), f2: Some(f2), f3: Some(f3), f4: Some(f4),
                out1: None, out2: None, out3: None, out4: None }.await
    }
}
