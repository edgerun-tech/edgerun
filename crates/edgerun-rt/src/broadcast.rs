//! Broadcast channel - multiple subscribers

extern crate alloc;

use alloc::collections::VecDeque;
use alloc::sync::Arc;

use core::cell::{Cell, RefCell};
use core::future::Future;
use core::pin::Pin;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use core::task::{Context, Poll};

pub fn broadcast<T: Clone + 'static>(cap: usize) -> (Publisher<T>, Subscriber<T>) {
    let inner = Arc::new(Inner {
        buffer: RefCell::new(VecDeque::new()),
        cap,
        start_seq: AtomicUsize::new(0),
        closed: AtomicBool::new(false),
    });
    (
        Publisher {
            inner: inner.clone(),
        },
        Subscriber {
            inner,
            next_seq: Cell::new(0),
        },
    )
}

struct Inner<T> {
    buffer: RefCell<VecDeque<T>>,
    cap: usize,
    start_seq: AtomicUsize,
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
        if self.inner.cap > 0 && buffer.len() >= self.inner.cap {
            buffer.pop_front();
            self.inner.start_seq.fetch_add(1, Ordering::AcqRel);
        }
        buffer.push_back(value);
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
    next_seq: Cell<usize>,
}

impl<T: Clone + 'static> Future for Subscriber<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let mut next_seq = this.next_seq.get();
        let start = this.inner.start_seq.load(Ordering::Acquire);
        if next_seq < start {
            next_seq = start;
            this.next_seq.set(next_seq);
        }

        let buffer = this.inner.buffer.borrow();
        let index = next_seq.saturating_sub(start);
        if index < buffer.len() {
            let value = buffer[index].clone();
            next_seq += 1;
            this.next_seq.set(next_seq);
            return Poll::Ready(value);
        }

        cx.waker().wake_by_ref();
        Poll::Pending
    }
}

impl<T: Clone + 'static> Subscriber<T> {
    pub fn try_recv(&self) -> Option<T> {
        let mut next_seq = self.next_seq.get();
        let start = self.inner.start_seq.load(Ordering::Acquire);
        if next_seq < start {
            next_seq = start;
        }

        let buffer = self.inner.buffer.borrow();
        let index = next_seq.saturating_sub(start);
        if index < buffer.len() {
            let value = buffer[index].clone();
            self.next_seq.set(next_seq + 1);
            Some(value)
        } else {
            return None;
        }
    }
}
