//! Weak reference for Arc

extern crate alloc;

use alloc::sync::Arc;
use core::marker::PhantomData;
use core::sync::atomic::{AtomicUsize, Ordering};

pub struct Weak<T> {
    ptr: AtomicUsize,
    _marker: PhantomData<T>,
}

impl<T> Weak<T> {
    pub fn new() -> Self {
        Self {
            ptr: AtomicUsize::new(0),
            _marker: PhantomData,
        }
    }

    pub fn upgrade(&self) -> Option<Arc<T>> {
        let ptr = self.ptr.load(Ordering::Acquire);
        if ptr == 0 {
            return None;
        }
        let arc_ptr = ptr + 2 * core::mem::size_of::<T>();
        Some(unsafe { Arc::from_raw(arc_ptr as *const T) })
    }
}

impl<T> Clone for Weak<T> {
    fn clone(&self) -> Self {
        Self {
            ptr: AtomicUsize::new(self.ptr.load(Ordering::Acquire)),
            _marker: PhantomData,
        }
    }
}

impl<T> Default for Weak<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: core::fmt::Debug> core::fmt::Debug for Weak<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Weak").field("ptr", &self.ptr).finish()
    }
}
