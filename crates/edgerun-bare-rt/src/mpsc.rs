//! MPSC channel for bare-metal async

extern crate alloc;

use alloc::collections::VecDeque;
use alloc::sync::Arc;

use core::cell::RefCell;
use core::future::Future;
use core::pin::Pin;
use core::sync::atomic::{AtomicUsize, Ordering};
use core::task::{Context, Poll};

pub fn mpsc_channel<T>(cap: usize) -> (Sender<T>, Receiver<T>) {
    let queue = Arc::new(Queue {
        data: RefCell::new(VecDeque::new()),
        sender_count: AtomicUsize::new(1),
        closed: AtomicUsize::new(0),
    });
    (Sender { queue: queue.clone(), cap }, Receiver { queue })
}

struct Queue<T> {
    data: RefCell<VecDeque<T>>,
    sender_count: AtomicUsize,
    closed: AtomicUsize,
}

pub struct Sender<T> {
    queue: Arc<Queue<T>>,
    cap: usize,
}

impl<T> Sender<T> {
    pub fn send(&self, value: T) -> Result<(), SendError> {
        if self.closed() {
            return Err(SendError);
        }
        if self.cap > 0 && self.queue.data.borrow().len() >= self.cap {
            return Err(SendError);
        }
        self.queue.data.borrow_mut().push_back(value);
        self.queue.sender_count.fetch_add(1, Ordering::AcqRel);
        Ok(())
    }

    pub fn closed(&self) -> bool {
        self.queue.closed.load(Ordering::Acquire) != 0
    }

    pub fn try_send(&self, value: T) -> Result<(), TrySendError<T>> {
        if self.closed() {
            return Err(TrySendError(value));
        }
        if self.cap > 0 && self.queue.data.borrow().len() >= self.cap {
            return Err(TrySendError(value));
        }
        self.queue.data.borrow_mut().push_back(value);
        Ok(())
    }
}

pub struct SendError;

pub struct TrySendError<T>(pub T);

impl core::fmt::Debug for SendError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Sender closed")
    }
}

impl core::fmt::Display for SendError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "channel closed")
    }
}

pub struct Receiver<T> {
    queue: Arc<Queue<T>>,
}

impl<T> Future for Receiver<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if !self.queue.data.borrow().is_empty() {
            return Poll::Ready(self.queue.data.borrow_mut().pop_front().unwrap());
        }
        if self.queue.closed.load(Ordering::Acquire) != 0 {
            return Poll::Pending;
        }
        cx.waker().wake_by_ref();
        Poll::Pending
    }
}

impl<T> Receiver<T> {
    pub fn try_recv(&self) -> Option<T> {
        self.queue.data.borrow_mut().pop_front()
    }

    pub fn len(&self) -> usize {
        self.queue.data.borrow().len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.data.borrow().is_empty()
    }

    pub fn close(&self) {
        self.queue.closed.store(1, Ordering::Release);
    }
}