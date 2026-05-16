//! Watch channel - single value observer

use alloc::sync::Arc;
use alloc::vec::Vec;
use core::cell::RefCell;
use core::future::Future;
use core::pin::Pin;
use core::sync::atomic::{AtomicUsize, Ordering};
use core::task::{Context, Poll, Waker};

pub fn watch<T: Clone>(value: T) -> (Sender<T>, Receiver<T>) {
    let inner = Arc::new(Inner {
        value: RefCell::new(value),
        version: AtomicUsize::new(0),
        closed: AtomicUsize::new(0),
        waiters: RefCell::new(Vec::new()),
    });
    (
        Sender {
            inner: inner.clone(),
        },
        Receiver {
            inner,
            seen_version: 0,
            waker: None,
        },
    )
}

struct Inner<T> {
    value: RefCell<T>,
    version: AtomicUsize,
    closed: AtomicUsize,
    waiters: RefCell<Vec<Waker>>,
}

pub struct Sender<T> {
    inner: Arc<Inner<T>>,
}

impl<T: Clone> Sender<T> {
    pub fn send(&self, value: T) {
        if self.inner.closed.load(Ordering::Acquire) != 0 {
            return;
        }
        *self.inner.value.borrow_mut() = value;
        self.inner.version.fetch_add(1, Ordering::Release);
        let waiters = core::mem::take(&mut *self.inner.waiters.borrow_mut());
        for waker in waiters {
            waker.wake();
        }
    }

    pub fn closed(&self) -> bool {
        self.inner.closed.load(Ordering::Acquire) != 0
    }

    pub fn close(&self) {
        self.inner.closed.store(1, Ordering::Release);
        let waiters = core::mem::take(&mut *self.inner.waiters.borrow_mut());
        for waker in waiters {
            waker.wake();
        }
    }
}

#[derive(Clone)]
pub struct Receiver<T> {
    inner: Arc<Inner<T>>,
    seen_version: usize,
    waker: Option<Waker>,
}

impl<T: Clone> Receiver<T> {
    pub fn borrow(&self) -> T {
        self.inner.value.borrow().clone()
    }

    pub fn has_changed(&self, version: usize) -> bool {
        self.inner.version.load(Ordering::Acquire) > version
    }
}

impl<T: Clone + 'static> Future for Receiver<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let version = this.inner.version.load(Ordering::Acquire);
        if version > this.seen_version {
            this.seen_version = version;
            if let Some(waker) = this.waker.take() {
                let mut waiters = this.inner.waiters.borrow_mut();
                remove_waker(&mut waiters, &waker);
            }
            return Poll::Ready(this.borrow());
        }
        if this.inner.closed.load(Ordering::Acquire) != 0 {
            if let Some(waker) = this.waker.take() {
                let mut waiters = this.inner.waiters.borrow_mut();
                remove_waker(&mut waiters, &waker);
            }
            return Poll::Ready(this.borrow());
        }
        let should_register = match this.waker.as_ref() {
            Some(registered) => !registered.will_wake(cx.waker()),
            None => true,
        };
        if should_register {
            let mut waiters = this.inner.waiters.borrow_mut();
            if let Some(previous) = this.waker.replace(cx.waker().clone()) {
                remove_waker(&mut waiters, &previous);
            }
            register_waker(
                &mut waiters,
                this.waker.as_ref().expect("registered wakeup"),
            );
        }
        Poll::Pending
    }
}

impl<T> Drop for Receiver<T> {
    fn drop(&mut self) {
        if let Some(waker) = self.waker.take() {
            remove_waker(&mut self.inner.waiters.borrow_mut(), &waker);
        }
    }
}

fn register_waker(waiters: &mut Vec<Waker>, waker: &Waker) {
    if !waiters.iter().any(|registered| registered.will_wake(waker)) {
        waiters.push(waker.clone());
    }
}

fn remove_waker(waiters: &mut Vec<Waker>, waker: &Waker) {
    if let Some(pos) = waiters
        .iter()
        .position(|registered| registered.will_wake(waker))
    {
        waiters.remove(pos);
    }
}
