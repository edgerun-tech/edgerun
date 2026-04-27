//! Lazy static and once cell for bare-metal

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, Ordering};

pub struct LazyStatic<T> {
    data: UnsafeCell<Option<T>>,
    done: AtomicBool,
}

impl<T> LazyStatic<T> {
    pub const fn new() -> Self {
        Self { data: UnsafeCell::new(None), done: AtomicBool::new(false) }
    }

    pub fn get(&self, init: impl FnOnce() -> T) -> &T {
        if !self.done.load(Ordering::Acquire) {
            let value = init();
            unsafe { *self.data.get() = Some(value) };
            self.done.store(true, Ordering::Release);
        }
        unsafe { (*self.data.get()).as_ref().unwrap() }
    }
}

impl<T> Default for LazyStatic<T> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct OnceCell<T> {
    data: UnsafeCell<Option<T>>,
}

impl<T> OnceCell<T> {
    pub const fn new() -> Self {
        Self { data: UnsafeCell::new(None) }
    }

    pub fn get(&self) -> Option<&T> {
        unsafe { (*self.data.get()).as_ref() }
    }

    pub fn set(&self, value: T) -> Result<(), T> {
        unsafe {
            if (*self.data.get()).is_some() {
                Err(value)
            } else {
                *self.data.get() = Some(value);
                Ok(())
            }
        }
    }

    pub fn try_insert(&self, value: T) -> Result<&T, (&T, T)> {
        if let Some(existing) = self.get() {
            return Err((existing, value));
        }
        let _ = self.set(value);
        Ok(self.get().unwrap())
    }

    pub fn get_or_init(&self, init: impl FnOnce() -> T) -> &T {
        if let Some(v) = self.get() {
            return v;
        }
        self.set(init()).ok();
        self.get().unwrap()
    }
}

impl<T> Default for OnceCell<T> {
    fn default() -> Self {
        Self::new()
    }
}
