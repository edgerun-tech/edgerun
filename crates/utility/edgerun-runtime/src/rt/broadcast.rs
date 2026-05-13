//! Broadcast channel - multiple subscribers

use alloc::collections::VecDeque;
use alloc::sync::Arc;
use alloc::vec::Vec;

use core::cell::{Cell, RefCell};
use core::future::Future;
use core::pin::Pin;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use core::task::{Context, Poll, Waker};

pub fn broadcast<T: Clone + 'static>(cap: usize) -> (Publisher<T>, Subscriber<T>) {
    let inner = Arc::new(Inner {
        buffer: RefCell::new(VecDeque::new()),
        cap,
        start_seq: AtomicUsize::new(0),
        closed: AtomicBool::new(false),
        waiters: RefCell::new(Vec::new()),
    });
    (
        Publisher {
            inner: inner.clone(),
        },
        Subscriber {
            inner,
            next_seq: Cell::new(0),
            waker: None,
        },
    )
}

struct Inner<T> {
    buffer: RefCell<VecDeque<T>>,
    cap: usize,
    start_seq: AtomicUsize,
    closed: AtomicBool,
    waiters: RefCell<Vec<Waker>>,
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
        drop(buffer);

        let waiters = core::mem::take(&mut *self.inner.waiters.borrow_mut());
        for waker in waiters {
            waker.wake();
        }
    }

    pub fn closed(&self) -> bool {
        self.inner.closed.load(Ordering::Acquire)
    }

    pub fn close(&self) {
        self.inner.closed.store(true, Ordering::Release);
        let waiters = core::mem::take(&mut *self.inner.waiters.borrow_mut());
        for waker in waiters {
            waker.wake();
        }
    }
}

#[derive(Clone)]
pub struct Subscriber<T> {
    inner: Arc<Inner<T>>,
    next_seq: Cell<usize>,
    waker: Option<Waker>,
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
            if let Some(waker) = this.waker.take() {
                let mut waiters = this.inner.waiters.borrow_mut();
                remove_waker(&mut waiters, &waker);
            }
            return Poll::Ready(value);
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

impl<T> Drop for Subscriber<T> {
    fn drop(&mut self) {
        if let Some(waker) = self.waker.take() {
            remove_waker(&mut self.inner.waiters.borrow_mut(), &waker);
        }
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

fn register_waker(waiters: &mut alloc::vec::Vec<Waker>, waker: &Waker) {
    if !waiters.iter().any(|registered| registered.will_wake(waker)) {
        waiters.push(waker.clone());
    }
}

fn remove_waker(waiters: &mut alloc::vec::Vec<Waker>, waker: &Waker) {
    if let Some(pos) = waiters
        .iter()
        .position(|registered| registered.will_wake(waker))
    {
        waiters.remove(pos);
    }
}
