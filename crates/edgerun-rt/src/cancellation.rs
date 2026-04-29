//! Cooperative cancellation token for no_std executors.

extern crate alloc;

use crate::sync::Mutex;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::sync::atomic::{AtomicBool, Ordering};
use core::task::{Context, Poll, Waker};

struct Inner {
    cancelled: AtomicBool,
    wakers: Mutex<Vec<Waker>>,
}

#[derive(Clone)]
pub struct CancellationToken {
    inner: Arc<Inner>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Inner {
                cancelled: AtomicBool::new(false),
                wakers: Mutex::new(Vec::new()),
            }),
        }
    }

    pub fn cancel(&self) {
        self.inner.cancelled.store(true, Ordering::Release);
        let wakers = {
            let mut guard = self.inner.wakers.lock();
            core::mem::take(&mut *guard)
        };
        for waker in wakers {
            waker.wake();
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.inner.cancelled.load(Ordering::Acquire)
    }

    pub fn cancelled(&self) -> Cancelled<'_> {
        Cancelled {
            token: self,
            registered: false,
        }
    }

    pub fn wait(&self) {
        while !self.is_cancelled() {
            core::hint::spin_loop();
        }
    }

    pub fn child_token(&self) -> Self {
        self.clone()
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Cancelled<'a> {
    token: &'a CancellationToken,
    registered: bool,
}

impl Future for Cancelled<'_> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        if this.token.is_cancelled() {
            return Poll::Ready(());
        }

        if !this.registered {
            let mut wakers = this.token.inner.wakers.lock();
            let waker = cx.waker();
            if !wakers.iter().any(|stored| stored.will_wake(waker)) {
                wakers.push(waker.clone());
            }
            this.registered = true;
        }

        if this.token.is_cancelled() {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}
