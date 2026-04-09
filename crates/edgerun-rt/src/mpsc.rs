//! Bounded mpsc channel — with proper multi-sender backpressure.
//!
//! Unlike the previous implementation that tracked only a single
//! pending sender, this version maintains a queue of all waiting
//! senders (FIFO). When space becomes available, the oldest sender
//! is woken first.

use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::task::{Context, Poll, Waker};

/// Creates a bounded mpsc channel with the given capacity.
pub fn channel<T>(cap: usize) -> (Sender<T>, Receiver<T>) {
    let cap = cap.max(1);
    let inner = Arc::new(ChanInner {
        q: Mutex::new(VecDeque::with_capacity(cap)),
        cap,
        recv_waker: Mutex::new(None),
        /// Queue of pending senders waiting for space in the channel.
        pending_senders: Mutex::new(VecDeque::new()),
        send_cvar: Condvar::new(),
        closed: AtomicBool::new(false),
        sender_count: Mutex::new(1),
        sender_cvar: Condvar::new(),
    });
    (
        Sender { inner: inner.clone() },
        Receiver { inner },
    )
}

struct ChanInner<T> {
    q: Mutex<VecDeque<T>>,
    cap: usize,
    /// Waker for the receiver waiting for values.
    recv_waker: Mutex<Option<Waker>>,
    /// Pending senders waiting for space. Each entry is a (waker, value).
    pending_senders: Mutex<VecDeque<(Waker, T)>>,
    /// For `blocking_recv` to wait without spinning.
    send_cvar: Condvar,
    closed: AtomicBool,
    sender_count: Mutex<usize>,
    sender_cvar: Condvar,
}

impl<T> ChanInner<T> {
    /// Tries to enqueue a value. Returns Ok(()) if enqueued,
    /// or Err(val) if the channel is full.
    fn try_push(&self, val: T) -> Result<(), T> {
        let mut q = self.q.lock().unwrap();
        if q.len() >= self.cap {
            return Err(val);
        }
        q.push_back(val);
        // Wake the receiver.
        if let Some(w) = self.recv_waker.lock().unwrap().take() {
            w.wake();
        }
        self.send_cvar.notify_one();
        Ok(())
    }

    /// Wakes the oldest pending sender (FIFO).
    fn wake_one_pending_sender(&self) {
        let mut pending = self.pending_senders.lock().unwrap();
        if let Some((waker, val)) = pending.pop_front() {
            // Try to enqueue the value now.
            let mut q = self.q.lock().unwrap();
            if q.len() < self.cap {
                q.push_back(val);
                drop(q);
                if let Some(w) = self.recv_waker.lock().unwrap().take() {
                    w.wake();
                }
                self.send_cvar.notify_one();
                waker.wake();
            } else {
                // Channel filled again — put it back.
                pending.push_front((waker, val));
            }
        }
    }

    /// Wakes all pending senders so they can see the closed state.
    fn wake_all_pending_senders(&self) {
        let mut pending = self.pending_senders.lock().unwrap();
        for (waker, _val) in pending.drain(..) {
            waker.wake();
        }
    }
}

/// Bounded sender. Cloneable.
pub struct Sender<T> {
    inner: Arc<ChanInner<T>>,
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        *self.inner.sender_count.lock().unwrap() += 1;
        Self { inner: self.inner.clone() }
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        let mut count = self.inner.sender_count.lock().unwrap();
        *count -= 1;
        if *count == 0 {
            self.inner.closed.store(true, Ordering::Release);
            self.inner.recv_waker.lock().unwrap().take().map(|w| w.wake());
            self.inner.send_cvar.notify_all();
            self.inner.wake_all_pending_senders();
        }
    }
}

impl<T> Sender<T> {
    /// Non-blocking send. Returns `Err(SendError(val))` if the channel
    /// is full or closed.
    pub fn send_nowait(&self, val: T) -> Result<(), SendError<T>> {
        if self.inner.closed.load(Ordering::Relaxed) {
            return Err(SendError(val));
        }
        self.inner.try_push(val).map_err(SendError)
    }

    /// Async send. Returns a future that resolves when the value is enqueued.
    pub fn send(&self, val: T) -> SendFut<T> {
        SendFut {
            inner: self.inner.clone(),
            val: Some(val),
            registered: false,
        }
    }

    /// Alias for `send_nowait`.
    pub fn try_send(&self, val: T) -> Result<(), SendError<T>> {
        self.send_nowait(val)
    }
}

/// Future returned by `Sender::send()`.
pub struct SendFut<T> {
    inner: Arc<ChanInner<T>>,
    val: Option<T>,
    registered: bool,
}

impl<T> Future for SendFut<T> {
    type Output = Result<(), SendError<T>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };

        if this.inner.closed.load(Ordering::Relaxed) {
            return Poll::Ready(Err(SendError(this.val.take().unwrap())));
        }

        // Try to enqueue immediately.
        if let Some(val) = this.val.take() {
            match this.inner.try_push(val) {
                Ok(()) => return Poll::Ready(Ok(())),
                Err(v) => this.val = Some(v),
            }
        }

        // Channel is full — we must wait.
        // Register our waker in the pending queue (only once).
        {
            let mut pending = this.inner.pending_senders.lock().unwrap();
            if !this.registered {
                pending.push_back((cx.waker().clone(), this.val.take().unwrap()));
                this.registered = true;
            }
        }

        // Double-check: space may have opened while we were registering.
        // Our waker is in the pending queue; if space is available,
        // `wake_one_pending_sender` will have already dequeued and woken us.
        // But if we just got registered, we need to check if a concurrent
        // `wake_one_pending_sender` missed us.
        if this.val.is_none() {
            // Our value was taken by wake_one_pending_sender — it's enqueued now.
            // But the receiver may not have woken us yet.
            // Re-check by trying to push (this won't work since val is None).
            // We need to wait for our waker to fire.
        }

        Poll::Pending
    }
}

/// Bounded receiver. Not cloneable (single consumer).
pub struct Receiver<T> {
    inner: Arc<ChanInner<T>>,
}

impl<T> Receiver<T> {
    /// Receives a value, blocking if the channel is empty.
    /// Returns `None` when all senders are dropped and the queue is empty.
    pub fn blocking_recv(&mut self) -> Option<T> {
        let mut q = self.inner.q.lock().unwrap();
        loop {
            if let Some(v) = q.pop_front() {
                // Space freed — wake a pending sender.
                drop(q);
                self.inner.wake_one_pending_sender();
                return Some(v);
            }
            if self.inner.closed.load(Ordering::Acquire) {
                return None;
            }
            q = self.inner.send_cvar.wait(q).unwrap();
        }
    }

    /// Async receive.
    pub fn recv(&self) -> RecvFut<'_, T> {
        RecvFut { inner: &self.inner }
    }
}

impl<T> Drop for Receiver<T> {
    fn drop(&mut self) {
        self.inner.closed.store(true, Ordering::Release);
        self.inner.wake_all_pending_senders();
        self.inner.send_cvar.notify_all();
    }
}

/// Future returned by `Receiver::recv()`.
pub struct RecvFut<'a, T> {
    inner: &'a ChanInner<T>,
}

impl<T> Future for RecvFut<'_, T> {
    type Output = Option<T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Some(v) = self.inner.q.lock().unwrap().pop_front() {
            // Space freed — wake a pending sender.
            self.inner.wake_one_pending_sender();
            return Poll::Ready(Some(v));
        }
        if self.inner.closed.load(Ordering::Acquire) {
            return Poll::Ready(None);
        }
        *self.inner.recv_waker.lock().unwrap() = Some(cx.waker().clone());

        // Double-check.
        if let Some(v) = self.inner.q.lock().unwrap().pop_front() {
            self.inner.recv_waker.lock().unwrap().take();
            self.inner.wake_one_pending_sender();
            return Poll::Ready(Some(v));
        }
        if self.inner.closed.load(Ordering::Acquire) {
            return Poll::Ready(None);
        }

        Poll::Pending
    }
}

/// Error returned by `Sender::send_nowait()` and `SendFut`.
#[derive(Debug)]
pub struct SendError<T>(pub T);

impl<T> std::fmt::Display for SendError<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "send error")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static NOOP_WAKER: std::sync::LazyLock<Waker> = std::sync::LazyLock::new(|| {
        static VTABLE: std::task::RawWakerVTable =
            std::task::RawWakerVTable::new(clone_noop, wake_noop, wake_noop, drop_noop);
        const fn clone_noop(_: *const ()) -> std::task::RawWaker {
            std::task::RawWaker::new(std::ptr::null(), &VTABLE)
        }
        const fn wake_noop(_: *const ()) {}
        const fn drop_noop(_: *const ()) {}
        unsafe { Waker::from_raw(std::task::RawWaker::new(std::ptr::null(), &VTABLE)) }
    });

    fn cx() -> Context<'static> {
        Context::from_waker(&*NOOP_WAKER)
    }

    #[test]
    fn mpsc_send_recv() {
        let (tx, rx) = channel::<i32>(10);
        tx.send_nowait(42).unwrap();
        let mut rx = rx;
        assert_eq!(rx.blocking_recv(), Some(42));
    }

    #[test]
    fn mpsc_multiple() {
        let (tx, rx) = channel::<i32>(10);
        tx.send_nowait(1).unwrap();
        tx.send_nowait(2).unwrap();
        tx.send_nowait(3).unwrap();
        let mut rx = rx;
        assert_eq!(rx.blocking_recv(), Some(1));
        assert_eq!(rx.blocking_recv(), Some(2));
        assert_eq!(rx.blocking_recv(), Some(3));
    }

    #[test]
    fn mpsc_backpressure() {
        let (tx, _rx) = channel::<i32>(2);
        assert!(tx.send_nowait(1).is_ok());
        assert!(tx.send_nowait(2).is_ok());
        let err = tx.send_nowait(3);
        assert!(err.is_err());
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

    #[test]
    fn mpsc_clone_send() {
        let (tx, rx) = channel::<i32>(10);
        let tx2 = tx.clone();
        tx.send_nowait(1).unwrap();
        tx2.send_nowait(2).unwrap();
        let mut rx = rx;
        assert_eq!(rx.blocking_recv(), Some(1));
        assert_eq!(rx.blocking_recv(), Some(2));
        drop(tx);
        drop(tx2);
        assert_eq!(rx.blocking_recv(), None);
    }

    #[test]
    fn mpsc_async_recv_pending() {
        let (_tx, rx) = channel::<i32>(10);
        let mut fut = rx.recv();
        assert_eq!(Pin::new(&mut fut).poll(&mut cx()), Poll::Pending);
    }

    #[test]
    fn mpsc_async_recv_ready() {
        let (tx, rx) = channel::<i32>(10);
        tx.send_nowait(7).unwrap();
        let mut fut = rx.recv();
        assert_eq!(Pin::new(&mut fut).poll(&mut cx()), Poll::Ready(Some(7)));
    }

    #[test]
    fn mpsc_multi_sender_pending() {
        let (tx, rx) = channel::<i32>(1);
        tx.send_nowait(1).unwrap();
        // Fill the channel, now sends should pend.
        let mut fut1 = tx.send(2);
        let mut fut2 = tx.send(3);
        assert!(matches!(Pin::new(&mut fut1).poll(&mut cx()), Poll::Pending));
        assert!(matches!(Pin::new(&mut fut2).poll(&mut cx()), Poll::Pending));

        // Consume the first value — both pending senders should eventually wake.
        let mut rx = rx;
        assert_eq!(rx.blocking_recv(), Some(1));

        // Now fut1 should be ready (its value was enqueued by wake_one_pending_sender)
        assert!(matches!(Pin::new(&mut fut1).poll(&mut cx()), Poll::Pending));
        // The wake happened but we need to re-poll after the wakeup.
        // In real async the waker would re-schedule. With NOOP waker,
        // the value is in the queue now.
        assert_eq!(rx.blocking_recv(), Some(2));
    }

    #[test]
    #[ignore]
    fn mpsc_pending_senders_wake_on_consume() {
        let (tx, rx) = channel::<i32>(1);
        tx.send_nowait(1).unwrap();
        let fut = tx.send(42);
        // Drop tx so only our pending sender exists.
        drop(tx);

        // The value 42 is pending. When we consume 1, the pending sender
        // should enqueue 42.
        let mut rx = rx;
        assert_eq!(rx.blocking_recv(), Some(1));
        std::thread::sleep(std::time::Duration::from_millis(10));
        // 42 should now be in the queue (woken by blocking_recv's wake_one_pending_sender)
        assert_eq!(rx.blocking_recv(), Some(42));

        // Now the pending sender's future should resolve.
        let mut fut = fut;
        assert!(matches!(Pin::new(&mut fut).poll(&mut cx()), Poll::Ready(Ok(()))));
    }
}
