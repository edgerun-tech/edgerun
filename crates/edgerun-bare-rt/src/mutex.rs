//! Async mutex - exclusive access with async wait support.


extern crate alloc;

use alloc::sync::Arc;
use core::cell::UnsafeCell;
use core::future::Future;
use core::pin::Pin;
use core::sync::atomic::{AtomicBool, Ordering};
use core::task::{Context, Poll, Waker};

pub fn new<T>(data: T) -> Mutex<T> {
    Mutex {
        state: Arc::new(State {
            acquired: AtomicBool::new(false),
            waker: UnsafeCell::new(None),
        }),
        data: UnsafeCell::new(data),
    }
}

struct State {
    acquired: AtomicBool,
    waker: UnsafeCell<Option<Waker>>,
}

pub struct Mutex<T> {
    state: Arc<State>,
    data: UnsafeCell<T>,
}

impl<T> Mutex<T> {
    pub fn new(data: T) -> Self {
        new(data)
    }

    pub fn lock(&self) -> LockFuture<'_, T> {
        LockFuture {
            state: self.state.clone(),
            data: &self.data,
            registered: false,
        }
    }

    pub fn into_inner(self) -> T {
        self.data.into_inner()
    }
}

unsafe impl<T: Send> Send for Mutex<T> {}
unsafe impl<T: Send> Sync for Mutex<T> {}

pub struct MutexGuard<'a, T> {
    data: &'a UnsafeCell<T>,
    state: Arc<State>,
}

unsafe impl<T: Send> Send for MutexGuard<'_, T> {}

impl<T> core::ops::Deref for MutexGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.data.get() }
    }
}

impl<T> core::ops::DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.data.get() }
    }
}

impl<T> Drop for MutexGuard<'_, T> {
    fn drop(&mut self) {
        self.state.acquired.store(false, Ordering::Release);
        unsafe {
            if let Some(w) = (*self.state.waker.get()).take() {
                w.wake();
            }
        }
    }
}

pub struct LockFuture<'a, T> {
    state: Arc<State>,
    data: &'a UnsafeCell<T>,
    registered: bool,
}

unsafe impl<T: Send> Send for LockFuture<'_, T> {}

impl<'a, T> Future for LockFuture<'a, T> {
    type Output = MutexGuard<'a, T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();

        if !this.state.acquired.load(Ordering::Acquire) {
            this.state.acquired.store(true, Ordering::Release);
            return Poll::Ready(MutexGuard {
                data: this.data,
                state: this.state.clone(),
            });
        }

        if !this.registered {
            unsafe { *this.state.waker.get() = Some(cx.waker().clone()) };
            this.registered = true;
        }

        Poll::Pending
    }
}

pub type MutexLockFuture<'a, T> = LockFuture<'a, T>;
pub type AsyncMutexGuard<'a, T> = MutexGuard<'a, T>;

impl<T: Default> Default for Mutex<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}