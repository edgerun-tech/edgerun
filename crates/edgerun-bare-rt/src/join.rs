//! join! macro - await multiple futures concurrently.

#![no_std]

use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

// ===========================================================================
// Join
// ===========================================================================

pub enum Select2Enum<O> {
    _0(O),
    _1(O),
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

pub mod select_internal {
    use super::*;

    pub async fn select2<F1, F2, O>(f1: F1, f2: F2) -> O
    where
        F1: Future<Output = O>,
        F2: Future<Output = O>,
    {
        Join2 {
            f1: Option::Some(f1),
            f2: Option::Some(f2),
            out1: Option::None,
            out2: Option::None,
        }
        .await
    }

    struct Join2<F1: Future, F2: Future> {
        f1: Option<F1>,
        f2: Option<F2>,
        out1: Option<F1::Output>,
        out2: Option<F2::Output>,
    }

    impl<F1: Future<Output = O>, F2: Future<Output = O>> Future for Join2<F1, F2> {
        type Output = O;

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let this = self.as_mut().get_mut();
            if let Some(f) = this.f1.as_mut() {
                if let Poll::Ready(v) = Pin::new(f).poll(cx) {
                    this.out1 = Some(v);
                    this.f1 = None;
                }
            }
            if let Some(f) = this.f2.as_mut() {
                if let Poll::Ready(v) = Pin::new(f).poll(cx) {
                    this.out2 = Some(v);
                    this.f2 = None;
                }
            }
            if let Some(v) = this.out1.take() {
                return Poll::Ready(v);
            }
            if let Some(v) = this.out2.take() {
                return Poll::Ready(v);
            }
            Poll::Pending
        }
    }
}

pub use select_internal::select2 as select_internal;