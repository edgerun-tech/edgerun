//! Select utilities for racing futures

use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

#[macro_export]
macro_rules! select {
    (biased; $($body:tt)*) => {
        compile_error!("biased select is not implemented in edgerun-runtime")
    };
    ($($body:tt)*) => {
        $crate::select_impl!(2; $($body)*)
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
        $crate::rt::select_2(f1, f2).await
    }};
    (2; $pat1:pat = $fut1:expr => $body1:expr, $pat2:pat = $fut2:expr => $body2:expr $(,)?) => {{
        let f1 = $fut1;
        let f2 = $fut2;
        let result = $crate::rt::select_2(async { $crate::rt::Select2Enum::_0(f1.await) }, async {
            $crate::rt::Select2Enum::_1(f2.await)
        })
        .await;
        match result {
            $crate::rt::Select2Enum::_0($pat1) => $body1,
            $crate::rt::Select2Enum::_1($pat2) => $body2,
        }
    }};
}

pub enum Either<L, R> {
    Left(L),
    Right(R),
}

impl<L, R> Either<L, R> {
    pub fn into_left(self) -> core::option::Option<L> {
        match self {
            Either::Left(l) => Some(l),
            Either::Right(_) => None,
        }
    }
    pub fn into_right(self) -> core::option::Option<R> {
        match self {
            Either::Left(_) => None,
            Either::Right(r) => Some(r),
        }
    }
    pub fn is_left(&self) -> bool {
        matches!(self, Either::Left(_))
    }
    pub fn is_right(&self) -> bool {
        matches!(self, Either::Right(_))
    }

    fn as_pin_mut(self: Pin<&mut Self>) -> Either<Pin<&mut L>, Pin<&mut R>> {
        unsafe {
            match self.get_unchecked_mut() {
                Either::Left(l) => Either::Left(Pin::new_unchecked(l)),
                Either::Right(r) => Either::Right(Pin::new_unchecked(r)),
            }
        }
    }
}

impl<L: Future, R: Future<Output = L::Output>> Future for Either<L, R> {
    type Output = L::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.as_pin_mut() {
            Either::Left(l) => l.poll(cx),
            Either::Right(r) => r.poll(cx),
        }
    }
}

pub struct Select<F1, F2> {
    fut1: Option<F1>,
    fut2: Option<F2>,
}

impl<F1, F2> Unpin for Select<F1, F2> {}

impl<F1, F2> Select<F1, F2> {
    pub fn new(f1: F1, f2: F2) -> Self {
        Self {
            fut1: Some(f1),
            fut2: Some(f2),
        }
    }
}

impl<F1, F2> Future for Select<F1, F2>
where
    F1: Future,
    F2: Future<Output = F1::Output>,
{
    type Output = F1::Output;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Some(ref mut f1) = self.fut1 {
            if let Poll::Ready(v) = unsafe { Pin::new_unchecked(f1) }.poll(cx) {
                self.fut1 = None;
                self.fut2 = None;
                return Poll::Ready(v);
            }
        }
        if let Some(ref mut f2) = self.fut2 {
            return unsafe { Pin::new_unchecked(f2) }.poll(cx);
        }
        Poll::Pending
    }
}

pub fn select2<A, B>(a: A, b: B) -> Select<A, B>
where
    A: Future,
    B: Future<Output = A::Output>,
{
    Select::new(a, b)
}

pub fn select_2<A, B>(a: A, b: B) -> Select<A, B>
where
    A: Future,
    B: Future<Output = A::Output>,
{
    select2(a, b)
}

pub struct Select3<F1, F2, F3> {
    fut1: Option<F1>,
    fut2: Option<F2>,
    fut3: Option<F3>,
}

impl<F1, F2, F3> Unpin for Select3<F1, F2, F3> {}

impl<F1, F2, F3> Future for Select3<F1, F2, F3>
where
    F1: Future,
    F2: Future<Output = F1::Output>,
    F3: Future<Output = F1::Output>,
{
    type Output = F1::Output;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Some(ref mut f1) = self.fut1 {
            if let Poll::Ready(v) = unsafe { Pin::new_unchecked(f1) }.poll(cx) {
                self.fut1 = None;
                self.fut2 = None;
                self.fut3 = None;
                return Poll::Ready(v);
            }
        }
        if let Some(ref mut f2) = self.fut2 {
            if let Poll::Ready(v) = unsafe { Pin::new_unchecked(f2) }.poll(cx) {
                self.fut1 = None;
                self.fut2 = None;
                self.fut3 = None;
                return Poll::Ready(v);
            }
        }
        if let Some(ref mut f3) = self.fut3 {
            return unsafe { Pin::new_unchecked(f3) }.poll(cx);
        }
        Poll::Pending
    }
}

pub fn select_3<A, B, C>(a: A, b: B, c: C) -> Select3<A, B, C>
where
    A: Future,
    B: Future<Output = A::Output>,
    C: Future<Output = A::Output>,
{
    Select3 {
        fut1: Some(a),
        fut2: Some(b),
        fut3: Some(c),
    }
}

pub struct Select4<F1, F2, F3, F4> {
    fut1: Option<F1>,
    fut2: Option<F2>,
    fut3: Option<F3>,
    fut4: Option<F4>,
}

impl<F1, F2, F3, F4> Unpin for Select4<F1, F2, F3, F4> {}

impl<F1, F2, F3, F4> Future for Select4<F1, F2, F3, F4>
where
    F1: Future,
    F2: Future<Output = F1::Output>,
    F3: Future<Output = F1::Output>,
    F4: Future<Output = F1::Output>,
{
    type Output = F1::Output;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Some(ref mut f1) = self.fut1 {
            if let Poll::Ready(v) = unsafe { Pin::new_unchecked(f1) }.poll(cx) {
                self.fut1 = None;
                self.fut2 = None;
                self.fut3 = None;
                self.fut4 = None;
                return Poll::Ready(v);
            }
        }
        if let Some(ref mut f2) = self.fut2 {
            if let Poll::Ready(v) = unsafe { Pin::new_unchecked(f2) }.poll(cx) {
                self.fut1 = None;
                self.fut2 = None;
                self.fut3 = None;
                self.fut4 = None;
                return Poll::Ready(v);
            }
        }
        if let Some(ref mut f3) = self.fut3 {
            if let Poll::Ready(v) = unsafe { Pin::new_unchecked(f3) }.poll(cx) {
                self.fut1 = None;
                self.fut2 = None;
                self.fut3 = None;
                self.fut4 = None;
                return Poll::Ready(v);
            }
        }
        if let Some(ref mut f4) = self.fut4 {
            return unsafe { Pin::new_unchecked(f4) }.poll(cx);
        }
        Poll::Pending
    }
}

pub fn select_4<A, B, C, D>(a: A, b: B, c: C, d: D) -> Select4<A, B, C, D>
where
    A: Future,
    B: Future<Output = A::Output>,
    C: Future<Output = A::Output>,
    D: Future<Output = A::Output>,
{
    Select4 {
        fut1: Some(a),
        fut2: Some(b),
        fut3: Some(c),
        fut4: Some(d),
    }
}
