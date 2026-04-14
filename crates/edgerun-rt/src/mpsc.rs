//! Bounded mpsc channel with proper multi-sender backpressure.
//!
//! ## Fix: lost-wake race between sender registration and receiver wakeup.
//!
//! The original code checked the queue, found it full, THEN registered in
//! `pending_senders`. A receiver could dequeue between the check and the
//! registration, call `wake_one_pending_sender` on an empty list, and both
//! sides would wait forever.
//!
//! The fix: the value is stored in `Arc<PendingSend>` shared between
//! `SendFut` and `pending_senders`. `wake_one_pending_sender` and `SendFut::poll`
//! race to claim it — only one succeeds.
//!
//! ## Fixes applied in this version:
//! - **#2**: `SendFut` re-registration creates a fresh `PendingSend` and cancels
//!   the old one, preventing duplicate entries in `pending_senders`.
//! - **#8**: `SendFut::Drop` marks the entry as cancelled so
//!   `wake_one_pending_sender` skips it without wasting a wake.
//! - **#7**: `RecvFut::Drop` clears the registered waker.
//! - **#11**: `wake_one_pending_sender` pops the entry, releases the
//!   `pending_senders` lock, THEN does work — no nested lock acquisition.

use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use crate::sync::{Condvar, Mutex};
use std::task::{Context, Poll, Waker};

/// Shared state for a pending send. The value lives here so both
/// `wake_one_pending_sender` (receiver side) and `SendFut::poll` (sender side)
/// can race to claim it — only one succeeds, preventing double-enqueue.
///
/// The `cancelled` flag prevents zombie entries from wasting wake calls
/// when a `SendFut` is dropped or re-registered (fixes #2, #8).
struct PendingSend<T> {
    val: Mutex<Option<T>>,
    /// Set to `true` when the sender gives up on this entry (re-registered
    /// with a fresh `PendingSend`, or dropped). `wake_one_pending_sender`
    /// skips cancelled entries without attempting to claim or wake.
    cancelled: AtomicBool,
}

impl<T> PendingSend<T> {
    fn new(val: T) -> Self {
        Self {
            val: Mutex::new(Some(val)),
            cancelled: AtomicBool::new(false),
        }
    }
}

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
        trace_try_push_ok: AtomicUsize::new(0),
        trace_try_push_fail: AtomicUsize::new(0),
        trace_try_push_wake_recv: AtomicUsize::new(0),
        trace_recv_poll_dequeue: AtomicUsize::new(0),
        trace_recv_poll_pending: AtomicUsize::new(0),
        trace_recv_wake_sender: AtomicUsize::new(0),
        trace_sender_register_pending: AtomicUsize::new(0),
        trace_sender_recheck_ok: AtomicUsize::new(0),
        trace_sender_claimed_by_recv: AtomicUsize::new(0),
        trace_skip_cancelled_sender: AtomicUsize::new(0),
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
    /// Pending senders waiting for space. Each entry holds a waker and a
    /// shared `PendingSend` Arc so both sides can race to claim the value.
    pending_senders: Mutex<VecDeque<(Waker, std::sync::Arc<PendingSend<T>>)>>,
    /// Pending reserve() waiters (just wakers, no values).
    reserve_wakers: Mutex<VecDeque<Waker>>,
    send_cvar: Condvar,
    closed: AtomicBool,
    sender_count: Mutex<usize>,
    /// Number of reserved slots (held by Permits).
    reserved: AtomicUsize,
    // Trace counters for deadlock diagnosis.
    trace_try_push_ok: AtomicUsize,
    trace_try_push_fail: AtomicUsize,
    trace_try_push_wake_recv: AtomicUsize,
    trace_recv_poll_dequeue: AtomicUsize,
    trace_recv_poll_pending: AtomicUsize,
    trace_recv_wake_sender: AtomicUsize,
    trace_sender_register_pending: AtomicUsize,
    trace_sender_recheck_ok: AtomicUsize,
    trace_sender_claimed_by_recv: AtomicUsize,
    trace_skip_cancelled_sender: AtomicUsize,
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
        self.wake_one_pending_sender();
    }

    /// Try to push a value. Wakes the receiver if successful.
    fn try_push(&self, val: T) -> Result<(), T> {
        let mut q = self.q.lock();
        if q.len() >= self.cap {
            self.trace_try_push_fail.fetch_add(1, Ordering::Relaxed);
            return Err(val);
        }
        q.push_back(val);
        self.trace_try_push_ok.fetch_add(1, Ordering::Relaxed);
        if let Some(w) = self.recv_waker.lock().take() {
            self.trace_try_push_wake_recv.fetch_add(1, Ordering::Relaxed);
            w.wake();
        }
        self.send_cvar.notify_one();
        Ok(())
    }

    /// Wake the oldest pending sender. Races with `SendFut::poll` to claim the value.
    ///
    /// Fix #11: Pops the entry, releases the `pending_senders` lock, THEN
    /// does work (claiming value, acquiring `q` lock). No nested lock holding.
    fn wake_one_pending_sender(&self) {
        // Pop one entry while holding the lock.
        let entry = {
            let mut pending = self.pending_senders.lock();
            pending.pop_front()
        };

        let Some((waker, ps)) = entry else { return };

        // Skip cancelled entries — the sender gave up on this one.
        if ps.cancelled.load(Ordering::Relaxed) {
            self.trace_skip_cancelled_sender.fetch_add(1, Ordering::Relaxed);
            return;
        }

        self.trace_recv_wake_sender.fetch_add(1, Ordering::Relaxed);

        // Try to claim the value from shared state.
        let claimed = ps.val.lock().take();
        match claimed {
            Some(val) => {
                let mut q = self.q.lock();
                if q.len() < self.cap {
                    q.push_back(val);
                    drop(q);
                    if let Some(w) = self.recv_waker.lock().take() {
                        w.wake();
                    }
                    self.send_cvar.notify_one();
                } else {
                    // Still full — put value back and re-queue.
                    *ps.val.lock() = Some(val);
                    self.pending_senders.lock().push_front((waker, ps));
                    return;
                }
            }
            None => {
                self.trace_sender_claimed_by_recv.fetch_add(1, Ordering::Relaxed);
            }
        }
        waker.wake();
    }

    fn wake_all_pending_senders(&self) {
        let pending = self.pending_senders.lock().drain(..).collect::<Vec<_>>();
        for (waker, ps) in pending {
            ps.cancelled.store(true, Ordering::Relaxed);
            waker.wake();
        }
    }

    fn wake_reserve_waiters(&self) {
        let wakers = self.reserve_wakers.lock().drain(..).collect::<Vec<_>>();
        for w in wakers {
            w.wake();
        }
    }

    /// Remove the receiver's waker. Called by `RecvFut::Drop`.
    fn clear_recv_waker(&self) {
        self.recv_waker.lock().take();
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
    /// Dump internal trace counters. Useful for diagnosing deadlocks.
    pub fn dump_trace(&self) {
        eprintln!("    [mpsc trace] try_push_ok={} try_push_fail={} try_push_wake_recv={}",
            self.inner.trace_try_push_ok.load(Ordering::Relaxed),
            self.inner.trace_try_push_fail.load(Ordering::Relaxed),
            self.inner.trace_try_push_wake_recv.load(Ordering::Relaxed));
        eprintln!("    [mpsc trace] recv_dequeue={} recv_pending={} recv_wake_sender={}",
            self.inner.trace_recv_poll_dequeue.load(Ordering::Relaxed),
            self.inner.trace_recv_poll_pending.load(Ordering::Relaxed),
            self.inner.trace_recv_wake_sender.load(Ordering::Relaxed));
        eprintln!("    [mpsc trace] sender_register_pending={} sender_recheck_ok={} sender_claimed_by_recv={} skip_cancelled={}",
            self.inner.trace_sender_register_pending.load(Ordering::Relaxed),
            self.inner.trace_sender_recheck_ok.load(Ordering::Relaxed),
            self.inner.trace_sender_claimed_by_recv.load(Ordering::Relaxed),
            self.inner.trace_skip_cancelled_sender.load(Ordering::Relaxed));
    }

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
            pending: None,
        }
    }

    pub fn try_send(&self, val: T) -> Result<(), SendError<T>> {
        self.send_nowait(val)
    }

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

    pub fn reserve(&self) -> ReserveFut<T> {
        ReserveFut {
            inner: self.inner.clone(),
        }
    }
}

// ===========================================================================
// Permit — reserved send capacity
// ===========================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermitError {
    Closed,
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

pub struct Permit<T> {
    inner: std::sync::Arc<ChanInner<T>>,
}

impl<T> Permit<T> {
    pub fn send(self, val: T) {
        self.inner.send_with_permit(val);
        std::mem::forget(self);
    }
}

impl<T> Drop for Permit<T> {
    fn drop(&mut self) {
        self.inner.release_slot();
    }
}

// ===========================================================================
// ReserveFut — async reserve
// ===========================================================================

pub struct ReserveFut<T> {
    inner: std::sync::Arc<ChanInner<T>>,
}

impl<T> Future for ReserveFut<T> {
    type Output = Result<Permit<T>, PermitError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.inner.closed.load(Ordering::Relaxed) {
            return Poll::Ready(Err(PermitError::Closed));
        }

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

        self.inner.reserve_wakers.lock().push_back(cx.waker().clone());

        {
            let q_len = self.inner.q.lock().len();
            let reserved = self.inner.reserved.load(Ordering::Acquire);
            if q_len + reserved < self.inner.cap {
                self.inner.reserved.fetch_add(1, Ordering::AcqRel);
                // Fix #12: Remove our waker from the list on success.
                // We already pushed it above; drain it out now.
                {
                    let mut rw = self.inner.reserve_wakers.lock();
                    // Remove the last entry (it's ours — single consumer for this future).
                    if rw.back().map_or(false, |w| w.will_wake(cx.waker())) {
                        rw.pop_back();
                    }
                }
                return Poll::Ready(Ok(Permit {
                    inner: self.inner.clone(),
                }));
            }
        }

        Poll::Pending
    }
}

// ===========================================================================
// SendFut — async send
//
// ## Correctness: no lost wakes with multiple producers
//
// The value is moved into a shared `Arc<PendingSend>` before registration.
// Both `wake_one_pending_sender` (receiver side) and this `poll` race to
// claim it from the Arc. Only one succeeds — preventing double-enqueue and
// lost-wake scenarios.
//
// Registration order: we push to `pending_senders` FIRST, then re-check
// the queue. This ensures `wake_one_pending_sender` always sees us.
//
// ## Fix #2: Re-registration creates a fresh PendingSend and cancels the old.
// ## Fix #8: Drop marks the entry as cancelled.
// ===========================================================================

pub struct SendFut<T> {
    inner: std::sync::Arc<ChanInner<T>>,
    /// The value to send. `None` = moved into `pending.val` or already enqueued.
    val: Option<T>,
    /// Set after first Pending return. Shared with the entry in `pending_senders`.
    pending: Option<std::sync::Arc<PendingSend<T>>>,
}

impl<T> Future for SendFut<T> {
    type Output = Result<(), SendError<T>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };

        // Closed check.
        if this.inner.closed.load(Ordering::Relaxed) {
            if this.val.is_none() {
                return Poll::Ready(Ok(()));
            }
            let pending_empty = this.pending.as_ref().map_or(true, |ps| ps.val.lock().is_none());
            if this.val.is_none() && pending_empty {
                return Poll::Ready(Ok(()));
            }
            let v = this.val.take();
            return match v {
                Some(val) => Poll::Ready(Err(SendError(val))),
                None => Poll::Ready(Ok(())),
            };
        }

        // If already pending from a previous poll, try to complete.
        if let Some(ps) = this.pending.take() {
            // Try to claim the value — race with wake_one_pending_sender.
            let claimed = ps.val.lock().take();
            if claimed.is_none() {
                // wake_one_pending_sender already enqueued our value.
                return Poll::Ready(Ok(()));
            }
            match this.inner.try_push(claimed.unwrap()) {
                Ok(()) => {
                    return Poll::Ready(Ok(()));
                }
                Err(val) => {
                    // Queue still full. Mark the old entry as cancelled
                    // so wake_one_pending_sender skips it (fix #2).
                    ps.cancelled.store(true, Ordering::Relaxed);
                    // Create a fresh PendingSend for re-registration.
                    let ps2 = std::sync::Arc::new(PendingSend::new(val));
                    this.inner.pending_senders.lock().push_back((cx.waker().clone(), ps2.clone()));
                    this.inner.trace_sender_register_pending.fetch_add(1, Ordering::Relaxed);
                    // Re-check: receiver may have dequeued between re-reg and now.
                    if let Some(val) = ps2.val.lock().take() {
                        match this.inner.try_push(val) {
                            Ok(()) => {
                                this.inner.trace_sender_recheck_ok.fetch_add(1, Ordering::Relaxed);
                                return Poll::Ready(Ok(()));
                            }
                            Err(val) => {
                                *ps2.val.lock() = Some(val);
                            }
                        }
                    }
                    this.pending = Some(ps2);
                    return Poll::Pending;
                }
            }
        }

        // First poll for this value — try direct enqueue.
        if this.val.is_none() {
            return Poll::Ready(Ok(()));
        }
        if let Some(val) = this.val.take() {
            match this.inner.try_push(val) {
                Ok(()) => return Poll::Ready(Ok(())),
                Err(v) => this.val = Some(v),
            }
        }

        // Channel full — register as pending.
        let val = this.val.take().unwrap();
        let ps = std::sync::Arc::new(PendingSend::new(val));
        this.inner.pending_senders.lock().push_back((cx.waker().clone(), ps.clone()));
        this.inner.trace_sender_register_pending.fetch_add(1, Ordering::Relaxed);

        // Re-check: a receiver may have dequeued between our try_push failure
        // and our registration.
        if let Some(val) = ps.val.lock().take() {
            match this.inner.try_push(val) {
                Ok(()) => {
                    this.inner.trace_sender_recheck_ok.fetch_add(1, Ordering::Relaxed);
                    this.pending = None;
                    return Poll::Ready(Ok(()));
                }
                Err(val) => {
                    *ps.val.lock() = Some(val);
                }
            }
        } else {
            this.inner.trace_sender_claimed_by_recv.fetch_add(1, Ordering::Relaxed);
        }

        this.pending = Some(ps);
        Poll::Pending
    }
}

impl<T> Drop for SendFut<T> {
    fn drop(&mut self) {
        if let Some(ps) = &self.pending {
            ps.val.lock().take();
            // Fix #8: Mark as cancelled so wake_one_pending_sender skips
            // this entry instead of wasting a wake on a dead future.
            ps.cancelled.store(true, Ordering::Relaxed);
        }
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

impl<T> Drop for RecvFut<'_, T> {
    fn drop(&mut self) {
        // Fix #7: Clear the registered waker so a stale reference isn't held.
        self.inner.clear_recv_waker();
    }
}

impl<T> Future for RecvFut<'_, T> {
    type Output = Option<T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // Wake one pending sender BEFORE dequeuing so it can enqueue its
        // value to replace what we're about to consume.
        self.inner.wake_one_pending_sender();
        self.inner.wake_reserve_waiters();

        {
            let mut q = self.inner.q.lock();
            if let Some(v) = q.pop_front() {
                self.inner.trace_recv_poll_dequeue.fetch_add(1, Ordering::Relaxed);
                drop(q);
                // Wake another pending sender to keep the pipeline flowing.
                self.inner.wake_one_pending_sender();
                self.inner.wake_reserve_waiters();
                return Poll::Ready(Some(v));
            }
            if self.inner.closed.load(Ordering::Acquire) {
                return Poll::Ready(None);
            }
            *self.inner.recv_waker.lock() = Some(cx.waker().clone());

            // Double-check after registering — also wake pending senders
            // in case they enqueued between our first check and registration.
            self.inner.wake_one_pending_sender();
            self.inner.wake_reserve_waiters();

            if let Some(v) = q.pop_front() {
                self.inner.trace_recv_poll_dequeue.fetch_add(1, Ordering::Relaxed);
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
        self.inner.trace_recv_poll_pending.fetch_add(1, Ordering::Relaxed);
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

    #[test]
    fn mpsc_sendfut_drop_cancels() {
        let (tx, _rx) = channel::<i32>(1);
        tx.send_nowait(1).unwrap(); // Fill the channel
        let mut fut = tx.send(2);
        // First poll should register as pending
        assert_eq!(Pin::new(&mut fut).poll(&mut cx()), Poll::Pending);
        // Drop the future while pending
        drop(fut);
        // The cancelled entry should be skipped
        assert_eq!(tx.inner.pending_senders.lock().len(), 1); // Entry still in deque but cancelled
    }

    #[test]
    fn mpsc_recvfut_drop_clears_waker() {
        let (_tx, rx) = channel::<i32>(10);
        let mut fut = rx.recv();
        // Register waker
        assert_eq!(Pin::new(&mut fut).poll(&mut cx()), Poll::Pending);
        // Drop should clear the waker
        drop(fut);
        assert!(rx.inner.recv_waker.lock().is_none());
    }
}
