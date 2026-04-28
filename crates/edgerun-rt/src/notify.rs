//! Async notification primitive for bare-metal tasks.

extern crate alloc;

use crate::sync::Mutex;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll, Waker};

struct NotifyInner {
    permits: usize,
    waiters: VecDeque<Waker>,
}

#[derive(Clone)]
pub struct Notify {
    inner: Arc<Mutex<NotifyInner>>,
}

impl Notify {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(NotifyInner {
                permits: 0,
                waiters: VecDeque::new(),
            })),
        }
    }

    pub fn notified(&self) -> Notified<'_> {
        Notified { notify: self }
    }

    pub fn notify_one(&self) {
        let mut inner = self.inner.lock();
        inner.permits = inner.permits.saturating_add(1);
        if let Some(waker) = inner.waiters.pop_front() {
            drop(inner);
            waker.wake();
        }
    }

    pub fn notify_waiters(&self) {
        let mut inner = self.inner.lock();
        inner.permits = inner.permits.saturating_add(inner.waiters.len());
        let waiters = core::mem::take(&mut inner.waiters);
        drop(inner);
        for waker in waiters {
            waker.wake();
        }
    }
}

impl Default for Notify {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Notified<'a> {
    notify: &'a Notify,
}

impl Future for Notified<'_> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut inner = self.notify.inner.lock();
        if inner.permits > 0 {
            inner.permits -= 1;
            Poll::Ready(())
        } else {
            register_waker(&mut inner.waiters, cx.waker());
            Poll::Pending
        }
    }
}

fn register_waker(waiters: &mut alloc::collections::VecDeque<Waker>, waker: &Waker) {
    if !waiters.iter().any(|registered| registered.will_wake(waker)) {
        waiters.push_back(waker.clone());
    }
}
