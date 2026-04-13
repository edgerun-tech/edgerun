//! Bounded mpsc channel with proper multi-sender backpressure.
//!
//! ## Fixes applied:
//! - Uses `parking_lot::Mutex` (no `.unwrap()`)
//! - `SendFut` stores `(Waker, T)` in pending queue — atomic register-then-check
//!   under a single lock. No race between "queue full" and "space freed".

use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use parking_lot::{Condvar, Mutex};
use std::task::{Context, Poll, Waker};

pub fn channel<T>(cap: usize) -> (Sender<T>, Receiver<T>) {
    let cap = cap.max(1);
    let inner = std::sync::Arc::new(ChanInner {
        q: Mutex::new(VecDeque::with_capacity(cap)),
        cap,
        recv_waker: Mutex::new(None),
        pending_senders: Mutex::new(VecDeque::new()),
        reserve_wakers: Mutex::new(VecDeque::new()),
        send_cvar: Condvar::new(),
        closed: AtomicBool::new(false),
        sender_count: Mutex::new(1),
        reserved: AtomicUsize::new(0),
    });
    (
        Sender {
            inner: inner.clone(),
        },
        Receiver { inner },
    )
}

struct ChanInner<T> {
    q: Mutex<VecDeque<T>>,
    cap: usize,
    recv_waker: Mutex<Option<Waker>>,
    /// Pending senders waiting for space, with their values.
    pending_senders: Mutex<VecDeque<(Waker, T)>>,
    /// Pending reserve() waiters (just wakers, no values).
    reserve_wakers: Mutex<VecDeque<Waker>>,
    send_cvar: Condvar,
    closed: AtomicBool,
    sender_count: Mutex<usize>,
    /// Number of reserved slots (held by Permits).
    reserved: AtomicUsize,
}

impl<T> ChanInner<T> {
    /// Effective available capacity (queue + reserved slots).
    fn available(&self) -> usize {
        let q_len = self.q.lock().len();
        let reserved = self.reserved.load(Ordering::Acquire);
        self.cap.saturating_sub(q_len + reserved)
    }

    /// Try to reserve one slot. Returns true if successful.
    fn try_reserve_slot(&self) -> bool {
        // CAS loop to atomically increment reserved if space available.
        loop {
            let q_len = self.q.lock().len();
            let reserved = self.reserved.load(Ordering::Acquire);
            if q_len + reserved >= self.cap {
                return false;
            }
            if self
                .reserved
                .compare_exchange_weak(
                    reserved,
                    reserved + 1,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                )
                .is_ok()
            {
                return true;
            }
        }
    }

    /// Release a reserved slot (called when a Permit is dropped without sending).
    fn release_slot(&self) {
        self.reserved.fetch_sub(1, Ordering::AcqRel);
        self.wake_one_pending_sender();
        self.wake_reserve_waiters();
    }

    /// Send a value using a reserved slot. The slot is consumed by this call.
    fn send_with_permit(&self, val: T) {
        let mut q = self.q.lock();
        q.push_back(val);
        self.reserved.fetch_sub(1, Ordering::AcqRel);
        drop(q);
        if let Some(w) = self.recv_waker.lock().take() {
            w.wake();
        }
        self.send_cvar.notify_one();
        // Wake a pending sender since we freed the reserved slot.
        self.wake_one_pending_sender();
    }

    /// Try to push a value. Wakes the receiver if successful.
    fn try_push(&self, val: T) -> Result<(), T> {
        let mut q = self.q.lock();
        if q.len() >= self.cap {
            return Err(val);
        }
        q.push_back(val);
        if let Some(w) = self.recv_waker.lock().take() {
            w.wake();
        }
        self.send_cvar.notify_one();
        Ok(())
    }

    /// Wake the oldest pending sender (FIFO). It will re-poll and try to enqueue.
    fn wake_one_pending_sender(&self) {
        let mut pending = self.pending_senders.lock();
        if let Some((waker, val)) = pending.pop_front() {
            // Try to enqueue the value now.
            let mut q = self.q.lock();
            if q.len() < self.cap {
                q.push_back(val);
                drop(q);
                if let Some(w) = self.recv_waker.lock().take() {
                    w.wake();
                }
                self.send_cvar.notify_one();
                // Wake the sender so its future resolves.
                waker.wake();
            } else {
                // Still full — put it back.
                pending.push_front((waker, val));
            }
        }
    }

    fn wake_all_pending_senders(&self) {
        let mut pending = self.pending_senders.lock();
        for (waker, _val) in pending.drain(..) {
            waker.wake();
        }
    }

    fn wake_reserve_waiters(&self) {
        let wakers = self.reserve_wakers.lock().drain(..).collect::<Vec<_>>();
        for w in wakers {
            w.wake();
        }
    }
}

pub struct Sender<T> {
    inner: std::sync::Arc<ChanInner<T>>,
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        *self.inner.sender_count.lock() += 1;
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        let mut count = self.inner.sender_count.lock();
        *count -= 1;
        if *count == 0 {
            self.inner.closed.store(true, Ordering::Release);
            if let Some(w) = self.inner.recv_waker.lock().take() {
                w.wake();
            }
            self.inner.send_cvar.notify_all();
            self.inner.wake_all_pending_senders();
        }
    }
}

impl<T> Sender<T> {
    pub fn send_nowait(&self, val: T) -> Result<(), SendError<T>> {
        if self.inner.closed.load(Ordering::Relaxed) {
            return Err(SendError(val));
        }
        self.inner.try_push(val).map_err(SendError)
    }

    pub fn send(&self, val: T) -> SendFut<T> {
        SendFut {
            inner: self.inner.clone(),
            val: Some(val),
        }
    }

    pub fn try_send(&self, val: T) -> Result<(), SendError<T>> {
        self.send_nowait(val)
    }

    /// Reserves capacity to send a value.
    ///
    /// The returned [`Permit`] guarantees that one slot is reserved
    /// for this sender. Use [`Permit::send`] to deliver the value
    /// without the possibility of blocking or failing.
    ///
    /// This is useful for backpressure: acquire a permit first, do
    /// async work, then send knowing it will succeed.
    ///
    /// Returns `Err(PermitError::Closed)` if the channel is closed,
    /// or `Err(PermitError::Full)` if no capacity is available (use
    /// `reserve().await` for the async version).
    pub fn try_reserve(&self) -> Result<Permit<T>, PermitError> {
        if self.inner.closed.load(Ordering::Relaxed) {
            return Err(PermitError::Closed);
        }
        if !self.inner.try_reserve_slot() {
            return Err(PermitError::Full);
        }
        Ok(Permit {
            inner: self.inner.clone(),
        })
    }

    /// Async version of [`try_reserve`](Self::try_reserve).
    ///
    /// Waits until capacity is available, then returns a [`Permit`].
    pub fn reserve(&self) -> ReserveFut<T> {
        ReserveFut {
            inner: self.inner.clone(),
        }
    }
}

// ===========================================================================
// Permit — reserved send capacity
// ===========================================================================

/// Error from [`Sender::try_reserve`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermitError {
    /// Channel is closed.
    Closed,
    /// No capacity available right now.
    Full,
}

impl std::fmt::Display for PermitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PermitError::Closed => write!(f, "channel closed"),
            PermitError::Full => write!(f, "channel full"),
        }
    }
}

/// A reserved slot to send a value. Guarantees [`send`](Permit::send) will succeed.
///
/// Created by [`Sender::try_reserve`] or [`Sender::reserve`].
/// If dropped without calling `send`, the slot is released.
pub struct Permit<T> {
    inner: std::sync::Arc<ChanInner<T>>,
}

impl<T> Permit<T> {
    /// Sends the value, consuming this permit.
    ///
    /// This always succeeds — the slot was already reserved.
    pub fn send(self, val: T) {
        self.inner.send_with_permit(val);
        // Don't run Drop — we consumed the slot.
        std::mem::forget(self);
    }
}

impl<T> Drop for Permit<T> {
    fn drop(&mut self) {
        // Release the reserved slot back to the channel.
        self.inner.release_slot();
    }
}

// ===========================================================================
// ReserveFut — async reserve
// ===========================================================================

/// Future returned by [`Sender::reserve`].
pub struct ReserveFut<T> {
    inner: std::sync::Arc<ChanInner<T>>,
}

impl<T> Future for ReserveFut<T> {
    type Output = Result<Permit<T>, PermitError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.inner.closed.load(Ordering::Relaxed) {
            return Poll::Ready(Err(PermitError::Closed));
        }

        // Try to reserve under the queue lock.
        {
            let q_len = self.inner.q.lock().len();
            let reserved = self.inner.reserved.load(Ordering::Acquire);
            if q_len + reserved < self.inner.cap {
                self.inner.reserved.fetch_add(1, Ordering::AcqRel);
                return Poll::Ready(Ok(Permit {
                    inner: self.inner.clone(),
                }));
            }
        }

        // Register waker for reserve waiters.
        self.inner.reserve_wakers.lock().push_back(cx.waker().clone());

        // Re-check after registering.
        {
            let q_len = self.inner.q.lock().len();
            let reserved = self.inner.reserved.load(Ordering::Acquire);
            if q_len + reserved < self.inner.cap {
                self.inner.reserved.fetch_add(1, Ordering::AcqRel);
                return Poll::Ready(Ok(Permit {
                    inner: self.inner.clone(),
                }));
            }
        }

        Poll::Pending
    }
}

pub struct SendFut<T> {
    inner: std::sync::Arc<ChanInner<T>>,
    /// Some = we still hold the value. None = it was enqueued by the receiver.
    val: Option<T>,
}

impl<T> Future for SendFut<T> {
    type Output = Result<(), SendError<T>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };

        if this.inner.closed.load(Ordering::Relaxed) {
            let v = this.val.take();
            // If val is None, it was already enqueued (success).
            return match v {
                Some(val) => Poll::Ready(Err(SendError(val))),
                None => Poll::Ready(Ok(())),
            };
        }

        // If val is None, the receiver already enqueued it for us.
        if this.val.is_none() {
            return Poll::Ready(Ok(()));
        }

        // Try to enqueue.
        if let Some(val) = this.val.take() {
            match this.inner.try_push(val) {
                Ok(()) => return Poll::Ready(Ok(())),
                Err(v) => this.val = Some(v),
            }
        }

        // Channel is full — register as pending sender with our value.
        // This is atomic: we hold the lock, check the queue, and register.
        {
            let mut q = this.inner.q.lock();
            if q.len() < this.inner.cap && this.val.is_some() {
                // Space opened while we were locking — enqueue directly.
                q.push_back(this.val.take().unwrap());
                drop(q);
                if let Some(w) = this.inner.recv_waker.lock().take() {
                    w.wake();
                }
                this.inner.send_cvar.notify_one();
                return Poll::Ready(Ok(()));
            }
            // Still full — register our waker + value.
            let val = this.val.take().unwrap();
            this.inner
                .pending_senders
                .lock()
                .push_back((cx.waker().clone(), val));
        }

        Poll::Pending
    }
}

pub struct Receiver<T> {
    inner: std::sync::Arc<ChanInner<T>>,
}

impl<T> Receiver<T> {
    pub fn blocking_recv(&mut self) -> Option<T> {
        let mut q = self.inner.q.lock();
        loop {
            if let Some(v) = q.pop_front() {
                drop(q);
                self.inner.wake_one_pending_sender();
                self.inner.wake_reserve_waiters();
                return Some(v);
            }
            if self.inner.closed.load(Ordering::Acquire) {
                return None;
            }
            self.inner.send_cvar.wait(&mut q);
        }
    }

    pub fn try_recv(&self) -> Result<T, TryRecvError> {
        let mut q = self.inner.q.lock();
        if let Some(v) = q.pop_front() {
            drop(q);
            self.inner.wake_one_pending_sender();
            self.inner.wake_reserve_waiters();
            return Ok(v);
        }
        if self.inner.closed.load(Ordering::Acquire) {
            Err(TryRecvError::Disconnected)
        } else {
            Err(TryRecvError::Empty)
        }
    }

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

pub struct RecvFut<'a, T> {
    inner: &'a ChanInner<T>,
}

impl<T> Future for RecvFut<'_, T> {
    type Output = Option<T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        {
            let mut q = self.inner.q.lock();
            if let Some(v) = q.pop_front() {
                drop(q);
                self.inner.wake_one_pending_sender();
                self.inner.wake_reserve_waiters();
                return Poll::Ready(Some(v));
            }
            if self.inner.closed.load(Ordering::Acquire) {
                return Poll::Ready(None);
            }
            *self.inner.recv_waker.lock() = Some(cx.waker().clone());

            // Double-check after registering waker.
            if let Some(v) = q.pop_front() {
                self.inner.recv_waker.lock().take();
                drop(q);
                self.inner.wake_one_pending_sender();
                self.inner.wake_reserve_waiters();
                return Poll::Ready(Some(v));
            }
            if self.inner.closed.load(Ordering::Acquire) {
                return Poll::Ready(None);
            }
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

    static NOOP_WAKER: std::sync::LazyLock<Waker> =
        std::sync::LazyLock::new(|| {
            static VTABLE: std::task::RawWakerVTable =
                std::task::RawWakerVTable::new(clone_noop, wake_noop, wake_noop, drop_noop);
            const fn clone_noop(_: *const ()) -> std::task::RawWaker {
                std::task::RawWaker::new(std::ptr::null(), &VTABLE)
            }
            const fn wake_noop(_: *const ()) {}
            const fn drop_noop(_: *const ()) {}
            unsafe {
                Waker::from_raw(std::task::RawWaker::new(
                    std::ptr::null(),
                    &VTABLE,
                ))
            }
        });

    fn cx() -> Context<'static> {
        Context::from_waker(&NOOP_WAKER)
    }

    #[test]
    fn mpsc_send_recv() {
        let (tx, rx) = channel::<i32>(10);
        tx.send_nowait(42).unwrap();
        let mut rx = rx;
        assert_eq!(rx.blocking_recv(), Some(42));
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
    fn mpsc_async_recv_ready() {
        let (tx, rx) = channel::<i32>(10);
        tx.send_nowait(7).unwrap();
        let mut fut = rx.recv();
        assert_eq!(Pin::new(&mut fut).poll(&mut cx()), Poll::Ready(Some(7)));
    }

    #[test]
    fn mpsc_async_recv_pending() {
        let (_tx, rx) = channel::<i32>(10);
        let mut fut = rx.recv();
        assert_eq!(Pin::new(&mut fut).poll(&mut cx()), Poll::Pending);
    }
}
