//! Bounded mpsc channel with backpressure.

use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll, Waker};

use crate::sync::{Condvar, Mutex};

pub fn channel<T>(cap: usize) -> (Sender<T>, Receiver<T>) {
    let cap = cap.max(1);
    let inner = Arc::new(ChanInner {
        queue: Mutex::new(VecDeque::with_capacity(cap)),
        cap,
        recv_waker: Mutex::new(None),
        send_waiters: Mutex::new(VecDeque::new()),
        send_cvar: Condvar::new(),
        closed: AtomicBool::new(false),
        sender_count: AtomicUsize::new(1),
    });
    (
        Sender { inner: inner.clone() },
        Receiver { inner },
    )
}

struct ChanInner<T> {
    queue: Mutex<VecDeque<T>>,
    cap: usize,
    recv_waker: Mutex<Option<Waker>>,
    send_waiters: Mutex<VecDeque<(Waker, Option<T>)>>,
    send_cvar: Condvar,
    closed: AtomicBool,
    sender_count: AtomicUsize,
}

pub struct Sender<T> {
    inner: Arc<ChanInner<T>>,
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        self.inner.sender_count.fetch_add(1, Ordering::Relaxed);
        Sender { inner: self.inner.clone() }
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        let prev = self.inner.sender_count.fetch_sub(1, Ordering::Relaxed);
        if prev == 1 {
            self.inner.closed.store(true, Ordering::Release);
            if let Some(w) = self.inner.recv_waker.lock().take() {
                w.wake_by_ref();
            }
            self.inner.send_cvar.notify_all();
        }
    }
}

impl<T> Sender<T> {
    pub fn try_send(&self, val: T) -> Result<(), SendError<T>> {
        if self.inner.closed.load(Ordering::Relaxed) {
            return Err(SendError(val));
        }
        let mut queue = self.inner.queue.lock();
        if queue.len() >= self.inner.cap {
            return Err(SendError(val));
        }
        queue.push_back(val);
        drop(queue);
        
        if let Some(w) = self.inner.recv_waker.lock().take() {
            w.wake_by_ref();
        }
        self.inner.send_cvar.notify_one();
        
        Ok(())
    }

    pub fn send(&self, val: T) -> SendFut<T> {
        SendFut {
            inner: self.inner.clone(),
            val: Some(val),
        }
    }

    pub fn send_nowait(&self, val: T) -> Result<(), SendError<T>> {
        self.try_send(val)
    }

    pub fn blocking_send(&self, val: T) -> Result<(), SendError<T>> {
        if self.inner.closed.load(Ordering::Relaxed) {
            return Err(SendError(val));
        }
        let mut queue = self.inner.queue.lock();
        while queue.len() >= self.inner.cap {
            if self.inner.closed.load(Ordering::Relaxed) {
                return Err(SendError(val));
            }
            self.inner.send_cvar.wait(&mut queue);
        }
        queue.push_back(val);
        drop(queue);
        
        if let Some(w) = self.inner.recv_waker.lock().take() {
            w.wake_by_ref();
        }
        self.inner.send_cvar.notify_one();
        
        Ok(())
    }
}

pub struct SendFut<T> {
    inner: Arc<ChanInner<T>>,
    val: Option<T>,
}

impl<T: Unpin> Future for SendFut<T> {
    type Output = Result<(), SendError<T>>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.as_mut().get_mut();
        
        if this.val.is_none() {
            return Poll::Ready(Ok(()));
        }

        if this.inner.closed.load(Ordering::Relaxed) {
            return Poll::Ready(Err(SendError(this.val.take().unwrap())));
        }

        let mut queue = this.inner.queue.lock();
        
        if queue.len() < this.inner.cap {
            queue.push_back(this.val.take().unwrap());
            drop(queue);
            
            if let Some(w) = this.inner.recv_waker.lock().take() {
                w.wake_by_ref();
            }
            this.inner.send_cvar.notify_one();
            
            return Poll::Ready(Ok(()));
        }

        drop(queue);
        
        this.inner.send_waiters.lock().push_back((cx.waker().clone(), this.val.take()));

        Poll::Pending
    }
}

pub struct Receiver<T> {
    inner: Arc<ChanInner<T>>,
}

impl<T> Receiver<T> {
    pub fn try_recv(&self) -> Result<T, TryRecvError> {
        let mut queue = self.inner.queue.lock();
        if let Some(v) = queue.pop_front() {
            drop(queue);
            self.wake_one_sender();
            return Ok(v);
        }
        if self.inner.closed.load(Ordering::Acquire) {
            Err(TryRecvError::Disconnected)
        } else {
            Err(TryRecvError::Empty)
        }
    }

    pub fn blocking_recv(&mut self) -> Option<T> {
        let mut queue = self.inner.queue.lock();
        loop {
            if let Some(v) = queue.pop_front() {
                drop(queue);
                self.wake_one_sender();
                return Some(v);
            }
            if self.inner.closed.load(Ordering::Acquire) {
                return None;
            }
            self.inner.send_cvar.wait(&mut queue);
        }
    }

    pub fn recv(&self) -> RecvFut<'_, T> {
        RecvFut { inner: &self.inner }
    }

    fn wake_one_sender(&self) {
        if let Some((waker, _value)) = self.inner.send_waiters.lock().pop_front() {
            waker.wake_by_ref();
        }
    }

    fn wake_all_senders(&self) {
        let mut waiters = self.inner.send_waiters.lock();
        while let Some((waker, _value)) = waiters.pop_front() {
            waker.wake_by_ref();
        }
    }
}

impl<T> Drop for Receiver<T> {
    fn drop(&mut self) {
        self.inner.closed.store(true, Ordering::Release);
        self.wake_all_senders();
    }
}

pub struct RecvFut<'a, T> {
    inner: &'a ChanInner<T>,
}

impl<T> Future for RecvFut<'_, T> {
    type Output = Option<T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut queue = self.inner.queue.lock();
        
        if let Some(v) = queue.pop_front() {
            drop(queue);
            
            // Wake ONE sender so they can send their value
            if let Some((waker, _value)) = self.inner.send_waiters.lock().pop_front() {
                waker.wake_by_ref();
            }
            
            self.inner.send_cvar.notify_all();
            return Poll::Ready(Some(v));
        }
        
        if self.inner.closed.load(Ordering::Acquire) {
            return Poll::Ready(None);
        }
        
        *self.inner.recv_waker.lock() = Some(cx.waker().clone());
        
        if let Some(v) = queue.pop_front() {
            self.inner.recv_waker.lock().take();
            drop(queue);
            if let Some((waker, _value)) = self.inner.send_waiters.lock().pop_front() {
                waker.wake_by_ref();
            }
            self.inner.send_cvar.notify_all();
            return Poll::Ready(Some(v));
        }
        
        Poll::Pending
    }
}

#[derive(Debug)]
pub struct SendError<T>(pub T);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TryRecvError {
    Empty,
    Disconnected,
}

impl<T> std::fmt::Display for SendError<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "send error")
    }
}

impl std::fmt::Display for TryRecvError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TryRecvError::Empty => write!(f, "channel empty"),
            TryRecvError::Disconnected => write!(f, "channel disconnected"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mpsc_send_recv() {
        let (tx, mut rx) = channel::<i32>(10);
        tx.send_nowait(42).unwrap();
        assert_eq!(rx.blocking_recv(), Some(42));
    }

    #[test]
    fn mpsc_backpressure() {
        let (tx, _rx) = channel::<i32>(2);
        assert!(tx.send_nowait(1).is_ok());
        assert!(tx.send_nowait(2).is_ok());
        assert!(tx.send_nowait(3).is_err());
    }

    #[test]
    fn mpsc_close() {
        let (tx, rx) = channel::<i32>(10);
        tx.send_nowait(1).unwrap();
        tx.send_nowait(2).unwrap();
        drop(tx);
        let mut rx = rx;
        assert_eq!(rx.blocking_recv(), Some(1));
        assert_eq!(rx.blocking_recv(), Some(2));
        assert_eq!(rx.blocking_recv(), None);
    }
}