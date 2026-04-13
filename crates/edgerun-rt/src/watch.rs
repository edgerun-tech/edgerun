//! Watch channel — single-value broadcast with version tracking.
//!
//! Uses `crate::sync::Mutex` consistently.

use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use crate::sync::Mutex;
use std::task::{Context, Poll, Waker};

struct WatchData<T> {
    value: T,
    waiters: VecDeque<Waker>,
    closed: bool,
}

/// Error returned when the watch channel is closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClosedError;

impl std::fmt::Display for ClosedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "watch channel closed")
    }
}

pub struct Sender<T> {
    data: Arc<Mutex<WatchData<T>>>,
    version: Arc<AtomicU64>,
}

pub struct Receiver<T> {
    data: Arc<Mutex<WatchData<T>>>,
    version: Arc<AtomicU64>,
    last_seen: AtomicU64,
}

impl<T> Sender<T> {
    pub fn new(value: T) -> (Self, Receiver<T>) {
        let data = Arc::new(Mutex::new(WatchData {
            value,
            waiters: VecDeque::new(),
            closed: false,
        }));
        let version = Arc::new(AtomicU64::new(1));
        let sender = Self {
            data: data.clone(),
            version: version.clone(),
        };
        let receiver = Receiver {
            data,
            version,
            last_seen: AtomicU64::new(0),
        };
        (sender, receiver)
    }

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

    pub fn close(&self) {
        let mut guard = self.data.lock();
        guard.closed = true;
        let waiters = std::mem::take(&mut guard.waiters);
        drop(guard);
        for waker in waiters {
            waker.wake();
        }
    }

    pub fn borrow(&self) -> Result<WatchRef<'_, T>, ClosedError> {
        let guard = self.data.lock();
        if guard.closed {
            return Err(ClosedError);
        }
        Ok(WatchRef { guard })
    }
}

impl<T: Clone> Sender<T> {
    pub fn borrow_clone(&self) -> Result<T, ClosedError> {
        let guard = self.borrow()?;
        Ok(guard.guard.value.clone())
    }
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            version: self.version.clone(),
        }
    }
}

impl<T> Receiver<T> {
    pub fn borrow(&self) -> Result<T, ClosedError>
    where
        T: Clone,
    {
        let guard = self.data.lock();
        if guard.closed {
            return Err(ClosedError);
        }
        Ok(guard.value.clone())
    }

    pub fn version(&self) -> u64 {
        self.version.load(Ordering::Acquire)
    }

    pub fn has_changed(&self) -> bool {
        self.version.load(Ordering::Acquire) > self.last_seen.load(Ordering::Acquire)
    }

    pub fn changed(&mut self) -> Changed<'_, T> {
        Changed {
            receiver: self,
            registered: false,
        }
    }
}

pub struct WatchRef<'a, T> {
    guard: crate::sync::MutexGuard<'a, WatchData<T>>,
}

impl<T> std::ops::Deref for WatchRef<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.guard.value
    }
}

pub struct Changed<'a, T> {
    receiver: &'a Receiver<T>,
    registered: bool,
}

impl<T> Future for Changed<'_, T> {
    type Output = Result<(), ClosedError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        let current_version = this.receiver.version.load(Ordering::Acquire);

        if current_version > this.receiver.last_seen.load(Ordering::Acquire) {
            this.receiver.last_seen.store(current_version, Ordering::Release);
            return Poll::Ready(Ok(()));
        }

        let mut guard = this.receiver.data.lock();

        if guard.closed {
            return Poll::Ready(Err(ClosedError));
        }

        let current_version = this.receiver.version.load(Ordering::Acquire);
        if current_version > this.receiver.last_seen.load(Ordering::Acquire) {
            this.receiver.last_seen.store(current_version, Ordering::Release);
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
}
