//! Cooperative task yield.

use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

pub fn yieldnow() -> YieldNow {
    YieldNow::new()
}

pub struct YieldNow {
    pub yielded: bool,
}

impl YieldNow {
    pub fn new() -> Self {
        Self { yielded: false }
    }
}

impl Default for YieldNow {
    fn default() -> Self {
        Self::new()
    }
}

impl Future for YieldNow {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        if this.yielded {
            Poll::Ready(())
        } else {
            this.yielded = true;
            Poll::Pending
        }
    }
}
