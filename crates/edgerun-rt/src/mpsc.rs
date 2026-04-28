//! MPSC channel for bare-metal async

extern crate alloc;

use alloc::collections::VecDeque;
use alloc::sync::Arc;

use core::cell::RefCell;
use core::future::Future;
use core::pin::Pin;
use core::sync::atomic::{AtomicUsize, Ordering};
use core::task::{Context, Poll, Waker};

pub fn channel<T>(cap: usize) -> (Sender<T>, Receiver<T>) {
    let queue = Arc::new(Queue {
        data: RefCell::new(VecDeque::new()),
        waker: RefCell::new(None),
        sender_count: AtomicUsize::new(1),
        closed: AtomicUsize::new(0),
    });
    (
        Sender {
            queue: queue.clone(),
            cap,
        },
        Receiver { queue },
    )
}

pub fn mpsc_channel<T>(cap: usize) -> (Sender<T>, Receiver<T>) {
    channel(cap)
}

struct Queue<T> {
    data: RefCell<VecDeque<T>>,
    waker: RefCell<Option<Waker>>,
    sender_count: AtomicUsize,
    closed: AtomicUsize,
}

unsafe impl<T: core::marker::Send> core::marker::Send for Queue<T> {}
unsafe impl<T: core::marker::Send> Sync for Queue<T> {}

pub struct Sender<T> {
    queue: Arc<Queue<T>>,
    cap: usize,
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        self.queue.sender_count.fetch_add(1, Ordering::AcqRel);
        Self {
            queue: self.queue.clone(),
            cap: self.cap,
        }
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        if self.queue.sender_count.fetch_sub(1, Ordering::AcqRel) == 1 {
            self.queue.closed.store(1, Ordering::Release);
            wake_receiver(&self.queue);
        }
    }
}

impl<T> Sender<T> {
    pub fn send(&self, value: T) -> Send<T> {
        Send {
            sender: self.clone(),
            value: Some(value),
            result: None,
        }
    }

    pub fn closed(&self) -> bool {
        self.queue.closed.load(Ordering::Acquire) != 0
    }

    pub fn try_send(&self, value: T) -> Result<(), TrySendError<T>> {
        if self.closed() {
            return Err(TrySendError::Closed(value));
        }
        if self.cap > 0 && self.queue.data.borrow().len() >= self.cap {
            return Err(TrySendError::Full(value));
        }
        self.queue.data.borrow_mut().push_back(value);
        wake_receiver(&self.queue);
        Ok(())
    }

    pub fn send_nowait(&self, value: T) -> Result<(), SendError<T>> {
        self.try_send(value).map_err(|e| SendError(e.into_inner()))
    }
}

pub struct Send<T> {
    sender: Sender<T>,
    value: Option<T>,
    result: Option<Result<(), SendError<T>>>,
}

impl<T> Send<T> {
    pub fn unwrap(mut self)
    where
        T: core::fmt::Debug,
    {
        match self.result.take().unwrap_or_else(|| {
            let value = self.value.take().expect("send future already completed");
            self.sender.send_nowait(value)
        }) {
            Ok(()) => {}
            Err(e) => panic!("called `Result::unwrap()` on an `Err` value: {:?}", e),
        }
    }
}

impl<T: Unpin> Future for Send<T> {
    type Output = Result<(), SendError<T>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        if let Some(result) = this.result.take() {
            return Poll::Ready(result);
        }
        let Some(value) = this.value.take() else {
            return Poll::Ready(Ok(()));
        };
        match this.sender.try_send(value) {
            Ok(()) => Poll::Ready(Ok(())),
            Err(TrySendError::Closed(value)) => Poll::Ready(Err(SendError(value))),
            Err(TrySendError::Full(value)) => {
                this.value = Some(value);
                *this.sender.queue.waker.borrow_mut() = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

pub struct SendError<T>(pub T);

pub enum TrySendError<T> {
    Full(T),
    Closed(T),
}

impl<T> TrySendError<T> {
    pub fn into_inner(self) -> T {
        match self {
            Self::Full(value) | Self::Closed(value) => value,
        }
    }
}

impl<T: core::fmt::Debug> core::fmt::Debug for TrySendError<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Full(value) => f.debug_tuple("Full").field(value).finish(),
            Self::Closed(value) => f.debug_tuple("Closed").field(value).finish(),
        }
    }
}

impl<T: core::fmt::Debug> core::fmt::Debug for SendError<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("SendError").field(&self.0).finish()
    }
}

impl<T> core::fmt::Display for SendError<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "channel closed")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TryRecvError {
    Empty,
    Disconnected,
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
        *self.queue.waker.borrow_mut() = Some(cx.waker().clone());
        Poll::Pending
    }
}

impl<T> Receiver<T> {
    pub fn recv(&self) -> Recv<'_, T> {
        Recv { receiver: self }
    }

    pub fn try_recv(&self) -> Result<T, TryRecvError> {
        if let Some(value) = self.queue.data.borrow_mut().pop_front() {
            wake_receiver(&self.queue);
            Ok(value)
        } else if self.queue.closed.load(Ordering::Acquire) != 0 {
            Err(TryRecvError::Disconnected)
        } else {
            Err(TryRecvError::Empty)
        }
    }

    pub fn blocking_recv(&self) -> Option<T> {
        loop {
            match self.try_recv() {
                Ok(value) => return Some(value),
                Err(TryRecvError::Disconnected) => return None,
                Err(TryRecvError::Empty) => core::hint::spin_loop(),
            }
        }
    }

    pub fn len(&self) -> usize {
        self.queue.data.borrow().len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.data.borrow().is_empty()
    }

    pub fn close(&self) {
        self.queue.closed.store(1, Ordering::Release);
        wake_receiver(&self.queue);
    }
}

pub struct Recv<'a, T> {
    receiver: &'a Receiver<T>,
}

impl<T> Future for Recv<'_, T> {
    type Output = Option<T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.receiver.try_recv() {
            Ok(value) => Poll::Ready(Some(value)),
            Err(TryRecvError::Disconnected) => Poll::Ready(None),
            Err(TryRecvError::Empty) => {
                *self.receiver.queue.waker.borrow_mut() = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

fn wake_receiver<T>(queue: &Arc<Queue<T>>) {
    if let Some(waker) = queue.waker.borrow_mut().take() {
        waker.wake();
    }
}
