//! Async mutex — exclusive access with wait queue.
//!
//! A sync Mutex guards only the wait queue and an "acquired" flag.
//! Data is behind an UnsafeCell, protected by the async protocol.

use std::cell::UnsafeCell;
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use crate::sync::Mutex as SyncMutex;
use std::task::{Context, Poll, Waker};

struct State {
    acquired: bool,
    waiters: VecDeque<Waker>,
}

/// An async mutex for exclusive access to `T`.
pub struct Mutex<T> {
    state: Arc<SyncMutex<State>>,
    data: UnsafeCell<T>,
}

impl<T> Mutex<T> {
    pub fn new(data: T) -> Self {
        Self {
            state: Arc::new(SyncMutex::new(State {
                acquired: false,
                waiters: VecDeque::new(),
            })),
            data: UnsafeCell::new(data),
        }
    }

    pub fn lock(&self) -> MutexLockFuture<'_, T> {
        MutexLockFuture {
            state: Arc::clone(&self.state),
            data: &self.data,
            registered: false,
        }
    }

    pub fn into_inner(self) -> T {
        self.data.into_inner()
    }
}

impl<T: Default> Default for Mutex<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

// Safety: Mutex<T> is Send when T is Send because the async protocol
// ensures exclusive access — only one guard can hold a mutable reference.
unsafe impl<T: Send> Send for Mutex<T> {}
unsafe impl<T: Send> Sync for Mutex<T> {}

/// A guard that holds exclusive access to the mutex.
pub struct MutexGuard<'a, T> {
    data: &'a UnsafeCell<T>,
    state: Arc<SyncMutex<State>>,
}

// Safety: MutexGuard is Send when T is Send because it holds exclusive
// access to the data — no other guard can exist simultaneously.
unsafe impl<T: Send> Send for MutexGuard<'_, T> {}

impl<T> std::ops::Deref for MutexGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.data.get() }
    }
}

impl<T> std::ops::DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.data.get() }
    }
}

impl<T> Drop for MutexGuard<'_, T> {
    fn drop(&mut self) {
        let mut state = self.state.lock();
        state.acquired = false;
        if let Some(waker) = state.waiters.pop_front() {
            waker.wake();
        }
    }
}

/// A future that resolves to an async mutex lock guard.
pub struct MutexLockFuture<'a, T> {
    state: Arc<SyncMutex<State>>,
    data: &'a UnsafeCell<T>,
    registered: bool,
}

// Safety: MutexLockFuture is Send when T is Send because the data is
// protected by the async protocol — only one guard exists at a time.
unsafe impl<T: Send> Send for MutexLockFuture<'_, T> {}

impl<'a, T> Future for MutexLockFuture<'a, T> {
    type Output = MutexGuard<'a, T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        let mut state = this.state.lock();

        if !state.acquired {
            state.acquired = true;
            return Poll::Ready(MutexGuard {
                data: this.data,
                state: this.state.clone(),
            });
        }

        if !this.registered {
            state.waiters.push_back(cx.waker().clone());
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
    fn mutex_lock_immediate() {
        let m = Mutex::new(42);
        let mut fut = m.lock();
        let Poll::Ready(guard) = Pin::new(&mut fut).poll(&mut cx()) else {
            panic!("expected ready");
        };
        assert_eq!(*guard, 42);
    }

    #[test]
    fn mutex_lock_contention() {
        let m = Mutex::new(0u32);
        let mut fut1 = m.lock();
        let Poll::Ready(_g1) = Pin::new(&mut fut1).poll(&mut cx()) else {
            panic!("expected ready");
        };

        let mut fut2 = m.lock();
        assert!(matches!(Pin::new(&mut fut2).poll(&mut cx()), Poll::Pending));
    }

    #[test]
    fn mutex_unlock_wakes_next() {
        let m = Mutex::new(0u32);
        let mut fut1 = m.lock();
        let Poll::Ready(mut g1) = Pin::new(&mut fut1).poll(&mut cx()) else {
            panic!("expected ready");
        };
        *g1 = 99;

        let mut fut2 = m.lock();
        assert!(matches!(Pin::new(&mut fut2).poll(&mut cx()), Poll::Pending));

        drop(g1);
        let Poll::Ready(g2) = Pin::new(&mut fut2).poll(&mut cx()) else {
            panic!("expected ready after unlock");
        };
        assert_eq!(*g2, 99);
    }

    #[test]
    fn mutex_into_inner() {
        let m = Mutex::new(42);
        assert_eq!(m.into_inner(), 42);
    }

    #[test]
    fn mutex_default() {
        let m: Mutex<Vec<i32>> = Mutex::default();
        let v = m.into_inner();
        assert!(v.is_empty());
    }
}
