//! OnceCell - lazy initialization with async wait support.

#![no_std]

extern crate alloc;

use alloc::sync::Arc;
use core::cell::UnsafeCell;
use core::future::Future;
use core::pin::Pin;
use core::sync::atomic::{AtomicBool, Ordering};
use core::task::{Context, Poll, Waker};

const Acquire: Ordering = Ordering::Acquire;
const Release: Ordering = Ordering::Release;

// ===========================================================================
// OnceCell
// ===========================================================================

struct OnceInner<T> {
    value: UnsafeCell<Option<T>>,
    waker: UnsafeCell<Option<Waker>>,
    initialized: AtomicBool,
}

impl<T> OnceInner<T> {
    fn new() -> Self {
        Self {
            value: UnsafeCell::new(None),
            waker: UnsafeCell::new(None),
            initialized: AtomicBool::new(false),
        }
    }
}

/// A cell that can be written to exactly once.
pub struct OnceCell<T> {
    inner: Arc<OnceInner<T>>,
}

impl<T> OnceCell<T> {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(OnceInner::new()),
        }
    }

    pub fn get(&self) -> Option<&T> {
        if self.inner.initialized.load(Acquire) {
            unsafe { (*self.inner.value.get()).as_ref() }
        } else {
            None
        }
    }

    pub fn get_or_init(&self, f: impl FnOnce() -> T) -> &T {
        if self.inner.initialized.load(Acquire) {
            return unsafe { (*self.inner.value.get()).as_ref().unwrap_unchecked() };
        }
        let value = f();
        unsafe { *self.inner.value.get() = Some(value) };
        self.inner.initialized.store(true, Release);
        unsafe { (*self.inner.value.get()).as_ref().unwrap_unchecked() }
    }

    pub fn get_mut(&mut self) -> Option<&mut T> {
        if self.inner.initialized.load(Acquire) {
            unsafe { (*self.inner.value.get()).as_mut() }
        } else {
            None
        }
    }

    pub fn try_insert(&self, value: T) -> Result<&T, T> {
        if self.inner.initialized.load(Acquire) {
            return Err(value);
        }
        unsafe { *self.inner.value.get() = Some(value) };
        self.inner.initialized.store(true, Release);
        unsafe {
            if let Some(w) = (*self.inner.waker.get()).take() {
                w.wake();
            }
        }
        Ok(unsafe { (*self.inner.value.get()).as_ref().unwrap_unchecked() })
    }

    pub fn wait(&self) -> WaitUntilReady<'_, T> {
        WaitUntilReady { cell: self }
    }

    pub fn into_inner(self) -> Option<T> {
        self.inner.initialized.store(true, Release);
        unsafe { (*self.inner.value.get()).take() }
    }
}

impl<T> Default for OnceCell<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone> Clone for OnceCell<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

/// Future that resolves when the cell is initialized.
pub struct WaitUntilReady<'a, T> {
    cell: &'a OnceCell<T>,
}

impl<T> Future for WaitUntilReady<'_, T> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        if this.cell.inner.initialized.load(Acquire) {
            return Poll::Ready(());
        }
        unsafe {
            *this.cell.inner.waker.get() = Some(cx.waker().clone());
        }
        if this.cell.inner.initialized.load(Acquire) {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}