//! Watch channel — a single-value broadcast channel.
//!
//! A `Sender` holds the current value. A `Receiver` can subscribe
//! to changes. When the sender updates the value, all receivers
//! are notified. Reading the latest value is non-blocking.

use std::cell::Cell;
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use parking_lot::Mutex;
use std::task::{Context, Poll, Waker};

struct WatchData<T> {
    value: T,
    waiters: VecDeque<Waker>,
    closed: bool,
}

/// A watch channel sender. Holds the current value.
pub struct Sender<T> {
    data: Arc<Mutex<WatchData<T>>>,
    version: Arc<AtomicU64>,
}

/// A watch channel receiver. Subscribes to value changes.
pub struct Receiver<T> {
    data: Arc<Mutex<WatchData<T>>>,
    version: Arc<AtomicU64>,
    last_seen: Cell<u64>,
}

impl<T> Sender<T> {
    /// Creates a new watch channel with the given initial value.
    pub fn new(value: T) -> (Self, Receiver<T>) {
        let data = Arc::new(Mutex::new(WatchData {
            value,
            waiters: VecDeque::new(),
            closed: false,
        }));
        let version = Arc::new(AtomicU64::new(1));
        let sender = Self { data: data.clone(), version: version.clone() };
        let receiver = Receiver { data, version, last_seen: Cell::new(0) };
        (sender, receiver)
    }

    /// Updates the value and notifies all waiting receivers.
    pub fn send_replace(&mut self, value: T) {
        let mut guard = self.data.lock();
        guard.value = value;
        self.version.fetch_add(1, Ordering::Release);
        let waiters = std::mem::take(&mut guard.waiters);
        drop(guard);
        for waker in waiters {
            waker.wake();
        }
    }

    /// Closes the channel.
    pub fn close(&self) {
        let mut guard = self.data.lock();
            guard.closed = true;
            let waiters = std::mem::take(&mut guard.waiters);
            drop(guard);
            for waker in waiters {
                waker.wake();
            }
    }

    /// Returns a reference to the current value.
    pub fn borrow(&self) -> Result<WatchRef<'_, T>, ()> {
        let guard = self.data.lock();
        if guard.closed {
            return Err(());
        }
        Ok(WatchRef { guard })
    }
}

impl<T: Clone> Sender<T> {
    /// Returns a cloned copy of the current value.
    pub fn borrow_clone(&self) -> Result<T, ()> {
        let guard = self.borrow()?;
        Ok(guard.guard.value.clone())
    }
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        Self { data: self.data.clone(), version: self.version.clone() }
    }
}

impl<T> Receiver<T> {
    /// Returns a cloned copy of the latest value without blocking.
    pub fn borrow(&self) -> Result<T, ()>
    where
        T: Clone,
    {
        let guard = self.data.lock();
        if guard.closed { return Err(()); }
        Ok(guard.value.clone())
    }

    /// Returns the current version number.
    pub fn version(&self) -> u64 {
        self.version.load(Ordering::Acquire)
    }

    /// Has the value changed since we last observed it?
    pub fn has_changed(&self) -> bool {
        self.version.load(Ordering::Acquire) > self.last_seen.get()
    }

    /// Waits for the next value change.
    pub fn changed(&mut self) -> Changed<'_, T> {
        Changed { receiver: self, registered: false }
    }
}

/// A reference to the current watch value.
pub struct WatchRef<'a, T> {
    guard: parking_lot::MutexGuard<'a, WatchData<T>>,
}

impl<T> std::ops::Deref for WatchRef<'_, T> {
    type Target = T;
    fn deref(&self) -> &T { &self.guard.value }
}

/// Future returned by `Receiver::changed()`.
pub struct Changed<'a, T> {
    receiver: &'a Receiver<T>,
    registered: bool,
}

impl<T> Future for Changed<'_, T> {
    type Output = Result<(), ()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        let current_version = this.receiver.version.load(Ordering::Acquire);

        if current_version > this.receiver.last_seen.get() {
            this.receiver.last_seen.set(current_version);
            return Poll::Ready(Ok(()));
        }

        let mut guard = this.receiver.data.lock();

        if guard.closed {
            return Poll::Ready(Err(()));
        }

        let current_version = this.receiver.version.load(Ordering::Acquire);
        if current_version > this.receiver.last_seen.get() {
            this.receiver.last_seen.set(current_version);
            return Poll::Ready(Ok(()));
        }

        if !this.registered {
            guard.waiters.push_back(cx.waker().clone());
            this.registered = true;
        }

        Poll::Pending
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
    fn watch_initial_value() {
        let (tx, rx) = Sender::new(42);
        assert_eq!(*tx.borrow().unwrap(), 42);
        assert_eq!(rx.borrow(), Ok(42));
    }

    #[test]
    fn watch_send_updates() {
        let (mut tx, mut rx) = Sender::new(42);
        tx.send_replace(99);
        assert_eq!(rx.borrow(), Ok(99));
        assert!(rx.has_changed());
    }

    #[test]
    fn watch_changed_ready() {
        let (mut tx, mut rx) = Sender::new(42);
        tx.send_replace(99);
        let mut fut = rx.changed();
        assert_eq!(Pin::new(&mut fut).poll(&mut cx()), Poll::Ready(Ok(())));
    }

    #[test]
    fn watch_changed_pending() {
        let (mut tx, mut rx) = Sender::new(42);
        tx.send_replace(43);
        let mut fut = rx.changed();
        assert_eq!(Pin::new(&mut fut).poll(&mut cx()), Poll::Ready(Ok(())));
        let mut fut2 = rx.changed();
        assert_eq!(Pin::new(&mut fut2).poll(&mut cx()), Poll::Pending);
    }

    #[test]
    fn watch_wakes_on_send() {
        let (mut tx, mut rx) = Sender::new(42);
        let mut fut = rx.changed();
        assert_eq!(Pin::new(&mut fut).poll(&mut cx()), Poll::Ready(Ok(())));
        let mut fut2 = rx.changed();
        assert_eq!(Pin::new(&mut fut2).poll(&mut cx()), Poll::Pending);
        tx.send_replace(100);
        assert_eq!(Pin::new(&mut fut2).poll(&mut cx()), Poll::Ready(Ok(())));
    }
}
