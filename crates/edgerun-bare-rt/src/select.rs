//! select! macro - race multiple futures.


extern crate alloc;

use core::marker::PhantomData;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

pub async fn select_2<F1, F2, O>(f1: F1, f2: F2) -> O
where
    F1: Future<Output = O> + Unpin,
    F2: Future<Output = O> + Unpin,
    O: Unpin,
{
    struct Select<F1, F2, O> {
        f1: Option<F1>,
        f2: Option<F2>,
        done1: bool,
        done2: bool,
        _marker: PhantomData<O>,
    }

    impl<F1: Future<Output = O> + Unpin, F2: Future<Output = O> + Unpin, O: Unpin> Future for Select<F1, F2, O> {
        type Output = O;

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let this = self.get_mut();

            if !this.done1 {
                if let Some(ref mut f) = this.f1 {
                    if let Poll::Ready(v) = Pin::new(f).poll(cx) {
                        this.done1 = true;
                        return Poll::Ready(v);
                    }
                }
            }

            if !this.done2 {
                if let Some(ref mut f) = this.f2 {
                    if let Poll::Ready(v) = Pin::new(f).poll(cx) {
                        this.done2 = true;
                        return Poll::Ready(v);
                    }
                }
            }

            Poll::Pending
        }
    }

    Select {
        f1: Some(f1),
        f2: Some(f2),
        done1: false,
        done2: false,
        _marker: PhantomData,
    }
    .await
}