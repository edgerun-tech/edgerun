//! Watch channel - single value observer

extern crate alloc;

use alloc::sync::Arc;
use core::cell::RefCell;
use core::future::Future;
use core::pin::Pin;
use core::sync::atomic::{AtomicUsize, Ordering};
use core::task::{Context, Poll};

pub fn watch<T: Clone>(value: T) -> (Sender<T>, Receiver<T>) {
    let inner = Arc::new(Inner {
        value: RefCell::new(value),
        version: AtomicUsize::new(0),
        closed: AtomicUsize::new(0),
    });
    (
        Sender {
            inner: inner.clone(),
        },
        Receiver {
            inner,
            seen_version: 0,
        },
    )
}

struct Inner<T> {
    value: RefCell<T>,
    version: AtomicUsize,
    closed: AtomicUsize,
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
    }

    pub fn closed(&self) -> bool {
        self.inner.closed.load(Ordering::Acquire) != 0
    }

    pub fn close(&self) {
        self.inner.closed.store(1, Ordering::Release);
    }
}

#[derive(Clone)]
pub struct Receiver<T> {
    inner: Arc<Inner<T>>,
    seen_version: usize,
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
            return Poll::Ready(this.borrow());
        }
        if this.inner.closed.load(Ordering::Acquire) != 0 {
            return Poll::Ready(this.borrow());
        }
        cx.waker().wake_by_ref();
        Poll::Pending
    }
}
