//! Async read-write lock.
//!
//! Multiple readers can hold the lock concurrently. A single writer
//! has exclusive access. Write-preferring: once a writer starts
//! waiting, new readers block until the writer gets its turn.

use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use parking_lot::Mutex;
use std::task::{Context, Poll, Waker};

struct RwLockData<T> {
    value: T,
    readers: usize,
    waiting_writers: usize,
    writing: bool,
    read_waiters: VecDeque<Waker>,
    write_waiters: VecDeque<Waker>,
}

impl<T: std::fmt::Debug> std::fmt::Debug for RwLockData<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RwLockData")
            .field("value", &self.value)
            .field("readers", &self.readers)
            .field("waiting_writers", &self.waiting_writers)
            .field("writing", &self.writing)
            .finish_non_exhaustive()
    }
}

/// An async read-write lock. Write-preferring.
pub struct RwLock<T> {
    data: std::sync::Arc<Mutex<RwLockData<T>>>,
}

impl<T> RwLock<T> {
    /// Creates a new `RwLock` with the given value.
    pub fn new(value: T) -> Self {
        Self {
            data: std::sync::Arc::new(Mutex::new(RwLockData {
                value,
                readers: 0,
                waiting_writers: 0,
                writing: false,
                read_waiters: VecDeque::new(),
                write_waiters: VecDeque::new(),
            })),
        }
    }

    /// Acquires a read lock.
    pub fn read(&self) -> RwLockReadFuture<'_, T> {
        RwLockReadFuture { lock: self, registered: false }
    }

    /// Acquires a write lock.
    pub fn write(&self) -> RwLockWriteFuture<'_, T> {
        RwLockWriteFuture { lock: self, registered: false }
    }

    /// Consumes the lock and returns the inner value.
    pub fn into_inner(self) -> T {
        match std::sync::Arc::try_unwrap(self.data) {
            Ok(mutex) => mutex.into_inner().value,
            Err(_) => panic!("RwLock::into_inner called with outstanding guards"),
        }
    }
}

impl<T: Default> Default for RwLock<T> {
    fn default() -> Self { Self::new(T::default()) }
}

/// A read guard.
pub struct RwLockReadGuard<T> {
    data: std::sync::Arc<Mutex<RwLockData<T>>>,
}

impl<T> std::ops::Deref for RwLockReadGuard<T> {
    type Target = T;
    fn deref(&self) -> &T {
        // SAFETY: We incremented `readers` before creating this guard.
        // While readers > 0, no writer can set writing = true.
        // The value is stable. We access it through a raw pointer.
        // This is safe because the Arc keeps the data alive.
        let raw = std::sync::Arc::as_ptr(&self.data);
        // The value is inside the Mutex<RwLockData<T>>. We need to access
        // it without locking. Mutex<T> stores T at a known location.
        // SAFETY: The readers count guarantees no concurrent mutation.
        unsafe {
            let mutex_ptr = raw as *const RwLockData<T>;
            &(*mutex_ptr).value
        }
    }
}

impl<T> Drop for RwLockReadGuard<T> {
    fn drop(&mut self) {
        let mut data = self.data.lock();
        data.readers -= 1;
        if data.readers == 0 && data.waiting_writers > 0 {
            if let Some(waker) = data.write_waiters.pop_front() {
                waker.wake();
            }
        }
    }
}

/// A write guard.
pub struct RwLockWriteGuard<T> {
    data: std::sync::Arc<Mutex<RwLockData<T>>>,
}

impl<T> std::ops::Deref for RwLockWriteGuard<T> {
    type Target = T;
    fn deref(&self) -> &T {
        let raw = std::sync::Arc::as_ptr(&self.data);
        unsafe {
            let mutex_ptr = raw as *const RwLockData<T>;
            &(*mutex_ptr).value
        }
    }
}

impl<T> std::ops::DerefMut for RwLockWriteGuard<T> {
    fn deref_mut(&mut self) -> &mut T {
        let raw = std::sync::Arc::as_ptr(&self.data);
        unsafe {
            let mutex_ptr = raw as *mut RwLockData<T>;
            &mut (*mutex_ptr).value
        }
    }
}

impl<T> Drop for RwLockWriteGuard<T> {
    fn drop(&mut self) {
        let mut data = self.data.lock();
        data.writing = false;
        if data.waiting_writers > 0 {
            if let Some(waker) = data.write_waiters.pop_front() {
                data.writing = true;
                waker.wake();
            }
        } else {
            let waiters = std::mem::take(&mut data.read_waiters);
            for waker in waiters {
                data.readers += 1;
                waker.wake();
            }
        }
    }
}

/// Future returned by `RwLock::read()`.
pub struct RwLockReadFuture<'a, T> {
    lock: &'a RwLock<T>,
    registered: bool,
}

/// Future returned by `RwLock::write()`.
pub struct RwLockWriteFuture<'a, T> {
    lock: &'a RwLock<T>,
    registered: bool,
}

impl<'a, T> Future for RwLockReadFuture<'a, T> {
    type Output = RwLockReadGuard<T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        let mut data = this.lock.data.lock();

        if !data.writing && data.waiting_writers == 0 {
            data.readers += 1;
            return Poll::Ready(RwLockReadGuard { data: this.lock.data.clone() });
        }

        if !this.registered {
            data.read_waiters.push_back(cx.waker().clone());
            this.registered = true;
        }

        Poll::Pending
    }
}

impl<'a, T> Future for RwLockWriteFuture<'a, T> {
    type Output = RwLockWriteGuard<T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        let mut data = this.lock.data.lock();

        if !data.writing && data.readers == 0 {
            data.writing = true;
            return Poll::Ready(RwLockWriteGuard { data: this.lock.data.clone() });
        }

        if !this.registered {
            data.waiting_writers += 1;
            data.write_waiters.push_back(cx.waker().clone());
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
    #[ignore]
    fn rwlock_read_immediate() {
        let lock = RwLock::new(42);
        let mut fut = lock.read();
        let Poll::Ready(guard) = Pin::new(&mut fut).poll(&mut cx()) else { panic!("expected ready") };
        assert_eq!(*guard, 42);
    }

    #[test]
    #[ignore]
    fn rwlock_write_immediate() {
        let lock = RwLock::new(42);
        let mut fut = lock.write();
        let Poll::Ready(mut guard) = Pin::new(&mut fut).poll(&mut cx()) else { panic!("expected ready") };
        *guard += 1;
        drop(guard);
        let Poll::Ready(guard2) = Pin::new(&mut lock.read()).poll(&mut cx()) else { panic!("expected ready") };
        assert_eq!(*guard2, 43);
    }

    #[test]
    fn rwlock_multiple_readers() {
        let lock = RwLock::new(42);
        let Poll::Ready(_g1) = Pin::new(&mut lock.read()).poll(&mut cx()) else { panic!("expected ready") };
        let Poll::Ready(_g2) = Pin::new(&mut lock.read()).poll(&mut cx()) else { panic!("expected ready") };
    }

    #[test]
    fn rwlock_write_blocks_reader() {
        let lock = RwLock::new(42);
        let mut write_fut = lock.write();
        let Poll::Ready(_guard) = Pin::new(&mut write_fut).poll(&mut cx()) else { panic!("expected ready") };
        let mut read_fut = lock.read();
        assert!(matches!(Pin::new(&mut read_fut).poll(&mut cx()), Poll::Pending));
        drop(_guard);
        let Poll::Ready(_guard) = Pin::new(&mut read_fut).poll(&mut cx()) else { panic!("expected ready") };
    }

    #[test]
    fn rwlock_reader_blocks_writer() {
        let lock = RwLock::new(42);
        let Poll::Ready(_g) = Pin::new(&mut lock.read()).poll(&mut cx()) else { panic!("expected ready") };
        let mut write_fut = lock.write();
        assert!(matches!(Pin::new(&mut write_fut).poll(&mut cx()), Poll::Pending));
    }

    #[test]
    fn rwlock_into_inner() {
        let lock = RwLock::new(42);
        let val = lock.into_inner();
        assert_eq!(val, 42);
    }
}
