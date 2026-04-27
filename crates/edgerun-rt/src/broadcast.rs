//! Broadcast channel - multiple subscribers

extern crate alloc;

use alloc::collections::VecDeque;
use alloc::sync::Arc;

use core::cell::RefCell;
use core::future::Future;
use core::pin::Pin;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use core::task::{Context, Poll};

pub fn broadcast<T: Clone + 'static>(cap: usize) -> (Publisher<T>, Subscriber<T>) {
    let inner = Arc::new(Inner {
        buffer: RefCell::new(VecDeque::new()),
        cap,
        seq: AtomicUsize::new(0),
        closed: AtomicBool::new(false),
    });
    (
        Publisher {
            inner: inner.clone(),
        },
        Subscriber { inner },
    )
}

struct Inner<T> {
    buffer: RefCell<VecDeque<T>>,
    cap: usize,
    seq: AtomicUsize,
    closed: AtomicBool,
}

pub struct Publisher<T> {
    inner: Arc<Inner<T>>,
}

impl<T: Clone> Publisher<T> {
    pub fn send(&self, value: T) {
        if self.closed() {
            return;
        }
        let mut buffer = self.inner.buffer.borrow_mut();
        if buffer.len() >= self.inner.cap {
            buffer.pop_front();
        }
        buffer.push_back(value);
        self.inner.seq.fetch_add(1, Ordering::Release);
    }

    pub fn closed(&self) -> bool {
        self.inner.closed.load(Ordering::Acquire)
    }

    pub fn close(&self) {
        self.inner.closed.store(true, Ordering::Release);
    }
}

#[derive(Clone)]
pub struct Subscriber<T> {
    inner: Arc<Inner<T>>,
}

impl<T: Clone + 'static> Future for Subscriber<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        let buffer = self.inner.buffer.borrow();
        if let Some(v) = buffer.front().cloned() {
            return Poll::Ready(v);
        }
        if self.inner.closed.load(Ordering::Acquire) {
            return Poll::Pending;
        }
        drop(buffer);
        Poll::Pending
    }
}

impl<T: Clone + 'static> Subscriber<T> {
    pub fn try_recv(&self) -> Option<T> {
        self.inner.buffer.borrow_mut().pop_front()
    }
}
