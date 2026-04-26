//! Select utilities for racing futures

use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

pub enum Either<L, R> {
    Left(L),
    Right(R),
}

impl<L, R> Either<L, R> {
    pub fn into_left(self) -> core::option::Option<L> {
        match self { Either::Left(l) => Some(l), Either::Right(_) => None }
    }
    pub fn into_right(self) -> core::option::Option<R> {
        match self { Either::Left(_) => None, Either::Right(r) => Some(r) }
    }
    pub fn is_left(&self) -> bool { matches!(self, Either::Left(_)) }
    pub fn is_right(&self) -> bool { matches!(self, Either::Right(_)) }
}

impl<L: Future, R: Future> Future for Either<L, R> {
    type Output = L::Output;
    
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.as_mut().poll(cx) {
            Poll::Ready(v) => Poll::Ready(v),
            Poll::Pending => Poll::Pending,
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
        Self { fut1: Some(f1), fut2: Some(f2) }
    }
}

impl<F1, F2> Future for Select<F1, F2>
where
    F1: Future + Unpin,
    F2: Future<Output = F1::Output> + Unpin,
{
    type Output = F1::Output;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Some(ref mut f1) = self.fut1 {
            if let Poll::Ready(v) = Pin::new(f1).poll(cx) {
                self.fut1 = None;
                self.fut2 = None;
                return Poll::Ready(v);
            }
        }
        if let Some(ref mut f2) = self.fut2 {
            return Pin::new(f2).poll(cx);
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