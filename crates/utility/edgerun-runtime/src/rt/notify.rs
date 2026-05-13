//! Async notification primitive for bare-metal tasks.

use crate::rt::sync::Mutex;
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
        Notified {
            notify: self,
            waker: None,
        }
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
    waker: Option<Waker>,
}

impl Future for Notified<'_> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut inner = self.notify.inner.lock();
        let this = self.get_mut();
        if inner.permits > 0 {
            inner.permits -= 1;
            if let Some(waker) = this.waker.take() {
                remove_waker(&mut inner.waiters, &waker);
            }
            Poll::Ready(())
        } else {
            let should_register = match this.waker.as_ref() {
                Some(registered) => !registered.will_wake(cx.waker()),
                None => true,
            };

            if should_register {
                if let Some(previous) = this.waker.replace(cx.waker().clone()) {
                    remove_waker(&mut inner.waiters, &previous);
                }
                register_waker(
                    &mut inner.waiters,
                    this.waker.as_ref().expect("registered wakeup"),
                );
            }
            Poll::Pending
        }
    }
}

impl Drop for Notified<'_> {
    fn drop(&mut self) {
        if let Some(waker) = self.waker.take() {
            let mut inner = self.notify.inner.lock();
            remove_waker(&mut inner.waiters, &waker);
        }
    }
}

fn remove_waker(waiters: &mut alloc::collections::VecDeque<Waker>, waker: &Waker) {
    if let Some(pos) = waiters
        .iter()
        .position(|registered| registered.will_wake(waker))
    {
        waiters.remove(pos);
    }
}

fn register_waker(waiters: &mut alloc::collections::VecDeque<Waker>, waker: &Waker) {
    if !waiters.iter().any(|registered| registered.will_wake(waker)) {
        waiters.push_back(waker.clone());
    }
}
