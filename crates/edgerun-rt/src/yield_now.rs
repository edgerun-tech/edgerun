//! Cooperative task yield.

use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll, Waker};

pub fn yieldnow() -> YieldNow {
    YieldNow { yielded: false }
}

pub struct YieldNow {
    pub yielded: bool,
    waker: Option<Waker>,
}

impl YieldNow {
    pub fn new() -> Self {
        Self {
            yielded: false,
            waker: None,
        }
    }
}

impl Default for YieldNow {
    fn default() -> Self {
        Self::new()
    }
}

impl Future for YieldNow {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        if this.yielded {
            this.waker = None;
            Poll::Ready(())
        } else {
            this.yielded = true;
            this.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

impl Drop for YieldNow {
    fn drop(&mut self) {
        self.waker = None;
    }
}
