//! Lazy static and once cell for bare-metal

use core::cell::UnsafeCell;
use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicBool, Ordering};

const UNINITIALIZED: u8 = 0;
const INITIALIZING: u8 = 1;
const INITIALIZED: u8 = 2;

pub struct LazyStatic<T> {
    data: UnsafeCell<MaybeUninit<T>>,
    state: AtomicBool,
}

impl<T> LazyStatic<T> {
    pub const fn new() -> Self {
        Self {
            data: UnsafeCell::new(MaybeUninit::uninit()),
            state: AtomicBool::new(false),
        }
    }

    pub fn get(&self, init: impl FnOnce() -> T) -> &T {
        while !self.state.load(Ordering::Acquire) {
            if self
                .state
                .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
            {
                let value = init();
                unsafe { (*self.data.get()).write(value) };
                self.state.store(true, Ordering::Release);
                break;
            }
            core::hint::spin_loop();
        }

        while !self.state.load(Ordering::Acquire) {
            core::hint::spin_loop();
        }

        unsafe { &*self.data.get().cast::<T>() }
    }
}

impl<T: 'static> Default for LazyStatic<T> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct OnceCell<T> {
    data: UnsafeCell<MaybeUninit<T>>,
    state: core::sync::atomic::AtomicU8,
}

impl<T> OnceCell<T> {
    pub const fn new() -> Self {
        Self {
            data: UnsafeCell::new(MaybeUninit::uninit()),
            state: AtomicU8::new(UNINITIALIZED),
        }
    }

    pub fn get(&self) -> Option<&T> {
        if self.state.load(Ordering::Acquire) != INITIALIZED {
            return None;
        }
        Some(unsafe { &*self.data.get().cast::<T>() })
    }

    pub fn set(&self, value: T) -> Result<(), T> {
        match self
            .state
            .compare_exchange(UNINITIALIZED, INITIALIZING, Ordering::AcqRel, Ordering::Acquire)
        {
            Ok(_) => {
                unsafe { self.data.get().write(MaybeUninit::new(value)); }
                self.state.store(INITIALIZED, Ordering::Release);
                Ok(())
            }
            Err(_) => Err(value),
        }
    }

    pub fn try_insert(&self, value: T) -> Result<&T, (&T, T)> {
        if let Some(existing) = self.get() {
            return Err((existing, value));
        }

        match self
            .set(value)
        {
            Ok(()) => Ok(self.get().unwrap()),
            Err(value) => self
                .get()
                .map(|v| (v, value))
                .unwrap_or((unsafe { self.data.get().as_ref().unwrap() }, value)),
        }
    }

    pub fn get_or_init(&self, init: impl FnOnce() -> T) -> &T {
        loop {
            if let Some(v) = self.get() {
                return v;
            }

            if self
                .state
                .compare_exchange(UNINITIALIZED, INITIALIZING, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                let value = init();
                unsafe { self.data.get().write(MaybeUninit::new(value)); }
                self.state.store(INITIALIZED, Ordering::Release);
                return unsafe { &*self.data.get().cast::<T>() };
            }

            while self.state.load(Ordering::Acquire) == INITIALIZING {
                core::hint::spin_loop();
            }
        }
    }
}

impl<T> Default for OnceCell<T> {
    fn default() -> Self {
        Self::new()
    }
}
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
        Self {
            data: UnsafeCell::new(None),
        }
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
