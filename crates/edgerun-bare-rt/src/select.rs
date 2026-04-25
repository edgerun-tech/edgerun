//! select! macro - race multiple futures.

extern crate alloc;

use core::marker::PhantomData;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

struct Select2<F1, F2, O> {
    f1: Option<F1>,
    f2: Option<F2>,
    done1: bool,
    done2: bool,
    _marker: PhantomData<O>,
}

impl<F1, F2, O> Future for Select2<F1, F2, O>
where
    F1: Future<Output = O> + Unpin,
    F2: Future<Output = O> + Unpin,
    O: Unpin,
{
    type Output = O;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if !self.done1 {
            if let Some(ref mut f) = self.f1 {
                if let Poll::Ready(v) = Pin::new(f).poll(cx) {
                    self.done1 = true;
                    return Poll::Ready(v);
                }
            }
        }

        if !self.done2 {
            if let Some(ref mut f) = self.f2 {
                if let Poll::Ready(v) = Pin::new(f).poll(cx) {
                    self.done2 = true;
                    return Poll::Ready(v);
                }
            }
        }

        Poll::Pending
    }
}

pub async fn select_2<F1, F2, O>(f1: F1, f2: F2) -> O
where
    F1: Future<Output = O> + Unpin,
    F2: Future<Output = O> + Unpin,
    O: Unpin,
{
    Select2 {
        f1: Some(f1),
        f2: Some(f2),
        done1: false,
        done2: false,
        _marker: PhantomData,
    }
    .await
}

struct Select3<F1, F2, F3, O> {
    f1: Option<F1>,
    f2: Option<F2>,
    f3: Option<F3>,
    done1: bool,
    done2: bool,
    done3: bool,
    _marker: PhantomData<O>,
}

impl<F1, F2, F3, O> Future for Select3<F1, F2, F3, O>
where
    F1: Future<Output = O> + Unpin,
    F2: Future<Output = O> + Unpin,
    F3: Future<Output = O> + Unpin,
    O: Unpin,
{
    type Output = O;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if !self.done1 {
            if let Some(ref mut f) = self.f1 {
                if let Poll::Ready(v) = Pin::new(f).poll(cx) {
                    self.done1 = true;
                    return Poll::Ready(v);
                }
            }
        }

        if !self.done2 {
            if let Some(ref mut f) = self.f2 {
                if let Poll::Ready(v) = Pin::new(f).poll(cx) {
                    self.done2 = true;
                    return Poll::Ready(v);
                }
            }
        }

        if !self.done3 {
            if let Some(ref mut f) = self.f3 {
                if let Poll::Ready(v) = Pin::new(f).poll(cx) {
                    self.done3 = true;
                    return Poll::Ready(v);
                }
            }
        }

        Poll::Pending
    }
}

pub async fn select_3<F1, F2, F3, O>(f1: F1, f2: F2, f3: F3) -> O
where
    F1: Future<Output = O> + Unpin,
    F2: Future<Output = O> + Unpin,
    F3: Future<Output = O> + Unpin,
    O: Unpin,
{
    Select3 {
        f1: Some(f1),
        f2: Some(f2),
        f3: Some(f3),
        done1: false,
        done2: false,
        done3: false,
        _marker: PhantomData,
    }
    .await
}

struct Select4<F1, F2, F3, F4, O> {
    f1: Option<F1>,
    f2: Option<F2>,
    f3: Option<F3>,
    f4: Option<F4>,
    done1: bool,
    done2: bool,
    done3: bool,
    done4: bool,
    _marker: PhantomData<O>,
}

impl<F1, F2, F3, F4, O> Future for Select4<F1, F2, F3, F4, O>
where
    F1: Future<Output = O> + Unpin,
    F2: Future<Output = O> + Unpin,
    F3: Future<Output = O> + Unpin,
    F4: Future<Output = O> + Unpin,
    O: Unpin,
{
    type Output = O;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if !self.done1 {
            if let Some(ref mut f) = self.f1 {
                if let Poll::Ready(v) = Pin::new(f).poll(cx) {
                    self.done1 = true;
                    return Poll::Ready(v);
                }
            }
        }

        if !self.done2 {
            if let Some(ref mut f) = self.f2 {
                if let Poll::Ready(v) = Pin::new(f).poll(cx) {
                    self.done2 = true;
                    return Poll::Ready(v);
                }
            }
        }

        if !self.done3 {
            if let Some(ref mut f) = self.f3 {
                if let Poll::Ready(v) = Pin::new(f).poll(cx) {
                    self.done3 = true;
                    return Poll::Ready(v);
                }
            }
        }

        if !self.done4 {
            if let Some(ref mut f) = self.f4 {
                if let Poll::Ready(v) = Pin::new(f).poll(cx) {
                    self.done4 = true;
                    return Poll::Ready(v);
                }
            }
        }

        Poll::Pending
    }
}

pub async fn select_4<F1, F2, F3, F4, O>(f1: F1, f2: F2, f3: F3, f4: F4) -> O
where
    F1: Future<Output = O> + Unpin,
    F2: Future<Output = O> + Unpin,
    F3: Future<Output = O> + Unpin,
    F4: Future<Output = O> + Unpin,
    O: Unpin,
{
    Select4 {
        f1: Some(f1),
        f2: Some(f2),
        f3: Some(f3),
        f4: Some(f4),
        done1: false,
        done2: false,
        done3: false,
        done4: false,
        _marker: PhantomData,
    }
    .await
}