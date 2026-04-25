//! Yield now primitive.

#![no_std]

use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

// ===========================================================================
// YieldNow
// ===========================================================================

pub struct YieldNow {
    yielded: bool,
}

impl YieldNow {
    pub fn new() -> Self {
        Self { yielded: false }
    }
}

impl Future for YieldNow {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.yielded {
            return Poll::Ready(());
        }
        self.yielded = true;
        cx.waker().wake_by_ref();
        Poll::Pending
    }
}

pub async fn yieldnow() {
    YieldNow::new().await
}