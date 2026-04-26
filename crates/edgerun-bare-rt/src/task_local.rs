//! Task-local storage implementation for bare-metal

use core::cell::UnsafeCell;

pub struct TaskLocal<T: 'static> {
    ptr: UnsafeCell<Option<T>>,
}

impl<T: 'static> TaskLocal<T> {
    pub const fn new() -> Self {
        Self { ptr: UnsafeCell::new(None) }
    }

    pub fn get(&self) -> Option<&T> {
        unsafe { (*self.ptr.get()).as_ref() }
    }

    pub fn set(&self, value: T) {
        unsafe { *self.ptr.get() = Some(value) };
    }

    pub fn take(&self) -> Option<T> {
        unsafe { (*self.ptr.get()).take() }
    }

    pub fn replace(&self, value: T) -> Option<T> {
        unsafe { (*self.ptr.get()).replace(value) }
    }

    pub fn with<R>(&self, f: impl FnOnce(Option<&T>) -> R) -> R {
        f(self.get())
    }
}

impl<T: 'static> Default for TaskLocal<T> {
    fn default() -> Self {
        Self::new()
    }
}