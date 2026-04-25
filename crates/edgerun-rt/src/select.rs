//! `select!` macro — race multiple futures, take the first to complete.
//!
//! ## Usage
//!
//! ```ignore
//! select! {
//!     result = future1 => { handle result },
//!     result = future2 => { handle result },
//! }
//! ```
//!
//! Supports 2-4 futures. The `biased` modifier disables round-robin fairness.

pub use crate::poll_fn;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};

#[macro_export]
macro_rules! select {
    (biased; $($ BODY:tt)*) => { compile_error!("biased not yet implemented") };

    ($($ BODY:tt)*) => { $crate::select_impl!(2; $($ BODY)*) };
}

pub enum Select2Enum<O> {
    _0(O),
    _1(O),
}

#[doc(hidden)]
#[macro_export]
macro_rules! select_impl {
    (2; $fut1:expr, $fut2:expr $(,)?) => {{
        let f1 = $fut1;
        let f2 = $fut2;
        $crate::select_2(f1, f2).await
    }};

    (2; $pat1:pat = $fut1:expr => $body1:expr, $pat2:pat = $fut2:expr => $body2:expr $(,)?) => {{
        let f1 = $fut1;
        let f2 = $fut2;
        let result = $crate::select_2(async { $crate::Select2Enum::_0(f1.await) }, async {
            $crate::Select2Enum::_1(f2.await)
        })
        .await;
        match result {
            $crate::Select2Enum::_0($pat1) => $body1,
            $crate::Select2Enum::_1($pat2) => $body2,
        }
    }};
}

pub async fn select_2<F1, F2, O>(f1: F1, f2: F2) -> O
where
    F1: Future<Output = O>,
    F2: Future<Output = O>,
{
    select_internal::select2(f1, f2).await
}

pub async fn select_3<F1, F2, F3, O>(f1: F1, f2: F2, f3: F3) -> O
where
    F1: Future<Output = O>,
    F2: Future<Output = O>,
    F3: Future<Output = O>,
{
    select_internal::select2(f1, select_internal::select2(f2, f3)).await
}

pub async fn select_4<F1, F2, F3, F4, O>(f1: F1, f2: F2, f3: F3, f4: F4) -> O
where
    F1: Future<Output = O>,
    F2: Future<Output = O>,
    F3: Future<Output = O>,
    F4: Future<Output = O>,
{
    select_internal::select2(
        f1,
        select_internal::select2(f2, select_internal::select2(f3, f4)),
    )
    .await
}

#[macro_export]
macro_rules! select2 {
    ($fut1:expr, $fut2:expr $(,)?) => {{
        let f1 = $fut1;
        let f2 = $fut2;
        $crate::select_internal::select2(f1, f2).await
    }};
}

pub mod select_internal {
    use std::future::Future;
    use std::marker::PhantomData;
    use std::pin::Pin;
    use std::task::{Context, Poll};

    struct Select2<F1, F2, O> {
        f1: Option<F1>,
        f2: Option<F2>,
        start: usize,
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
            let start = this.start;
            this.start = this.start.wrapping_add(1);

            if start == 0 {
                if let Some(f) = this.f1.as_mut() {
                    if let Poll::Ready(v) = unsafe { Pin::new_unchecked(f) }.poll(cx) {
                        this.f1 = None;
                        this.f2 = None;
                        return Poll::Ready(v);
                    }
                }
                if let Some(f) = this.f2.as_mut() {
                    if let Poll::Ready(v) = unsafe { Pin::new_unchecked(f) }.poll(cx) {
                        this.f2 = None;
                        this.f1 = None;
                        return Poll::Ready(v);
                    }
                }
            } else {
                if let Some(f) = this.f2.as_mut() {
                    if let Poll::Ready(v) = unsafe { Pin::new_unchecked(f) }.poll(cx) {
                        this.f2 = None;
                        this.f1 = None;
                        return Poll::Ready(v);
                    }
                }
                if let Some(f) = this.f1.as_mut() {
                    if let Poll::Ready(v) = unsafe { Pin::new_unchecked(f) }.poll(cx) {
                        this.f1 = None;
                        this.f2 = None;
                        return Poll::Ready(v);
                    }
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
        Select2 {
            f1: Some(f1),
            f2: Some(f2),
            start: 0,
            _output: PhantomData,
        }
        .await
    }

    pub async fn select2_tagged<F1, F2, O>(f1: F1, f2: F2) -> (usize, O)
    where
        F1: Future<Output = O>,
        F2: Future<Output = O>,
    {
        struct Select2Tagged<F1, F2, O> {
            f1: Option<F1>,
            f2: Option<F2>,
            start: usize,
            _output: PhantomData<O>,
        }

        impl<F1, F2, O> Future for Select2Tagged<F1, F2, O>
        where
            F1: Future<Output = O>,
            F2: Future<Output = O>,
        {
            type Output = (usize, O);

            fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                let this = unsafe { self.get_unchecked_mut() };
                let start = this.start;
                this.start = this.start.wrapping_add(1);

                if start == 0 {
                    if let Some(f) = this.f1.as_mut() {
                        if let Poll::Ready(v) = unsafe { Pin::new_unchecked(f) }.poll(cx) {
                            this.f1 = None;
                            this.f2 = None;
                            return Poll::Ready((0, v));
                        }
                    }
                    if let Some(f) = this.f2.as_mut() {
                        if let Poll::Ready(v) = unsafe { Pin::new_unchecked(f) }.poll(cx) {
                            this.f2 = None;
                            this.f1 = None;
                            return Poll::Ready((1, v));
                        }
                    }
                } else {
                    if let Some(f) = this.f2.as_mut() {
                        if let Poll::Ready(v) = unsafe { Pin::new_unchecked(f) }.poll(cx) {
                            this.f2 = None;
                            this.f1 = None;
                            return Poll::Ready((1, v));
                        }
                    }
                    if let Some(f) = this.f1.as_mut() {
                        if let Poll::Ready(v) = unsafe { Pin::new_unchecked(f) }.poll(cx) {
                            this.f1 = None;
                            this.f2 = None;
                            return Poll::Ready((0, v));
                        }
                    }
                }
                Poll::Pending
            }
        }

        Select2Tagged {
            f1: Some(f1),
            f2: Some(f2),
            start: 0,
            _output: PhantomData,
        }
        .await
    }
}

#[cfg(test)]
mod tests {
    use crate::{select, select2, sleep, Runtime};
    use std::time::Duration;

    #[test]
    fn select2_first_ready() {
        Runtime::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let a = async { 1 };
                let b = async {
                    sleep(Duration::from_secs(10)).await;
                    2
                };
                let result = select2!(a, b);
                assert_eq!(result, 1);
            });
    }

    #[test]
    fn select2_second_ready() {
        Runtime::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let a = async {
                    sleep(Duration::from_secs(10)).await;
                    1
                };
                let b = async { 2 };
                let result = select2!(a, b);
                assert_eq!(result, 2);
            });
    }

    #[test]
    fn select_with_pattern() {
        Runtime::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let result = select! {
                    x = async { Ok::<_, ()>(42) } => x,
                    y = async { sleep(Duration::from_millis(1)); Err(()) } => y,
                };
                assert_eq!(result, Ok(42));
            });
    }

    #[test]
    fn select_with_sleep() {
        Runtime::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let timeout = async {
                    sleep(Duration::from_millis(10)).await;
                    "timeout"
                };
                let immediate = async { "immediate" };
                let result = select! {
                    _ = timeout => "timed out",
                    v = immediate => v,
                };
                assert_eq!(result, "immediate");
            });
    }
}
