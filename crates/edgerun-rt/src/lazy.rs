//! Lazy static and once cell for bare-metal

use core::cell::UnsafeCell;
use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicU8, Ordering};

const UNINITIALIZED: u8 = 0;
const INITIALIZING: u8 = 1;
const INITIALIZED: u8 = 2;

unsafe impl<T: Sync> Sync for LazyStatic<T> {}
unsafe impl<T: Send> Send for LazyStatic<T> {}

pub struct LazyStatic<T> {
    data: UnsafeCell<MaybeUninit<T>>,
    state: AtomicU8,
}

impl<T> LazyStatic<T> {
    pub const fn new() -> Self {
        Self {
            data: UnsafeCell::new(MaybeUninit::uninit()),
            state: AtomicU8::new(UNINITIALIZED),
        }
    }

    pub fn get(&self, init: impl FnOnce() -> T) -> &T {
        loop {
            if self.state.load(Ordering::Acquire) == INITIALIZED {
                return unsafe { &*self.data.get().cast::<T>() };
            }
            match self.state.compare_exchange(
                UNINITIALIZED,
                INITIALIZING,
                Ordering::Acquire,
                Ordering::Relaxed,
            ) {
                Ok(_) => {
                    let value = init();
                    unsafe { (*self.data.get()).write(value) };
                    self.state.store(INITIALIZED, Ordering::Release);
                    return unsafe { &*self.data.get().cast::<T>() };
                }
                Err(_) => core::hint::spin_loop(),
            }
        }
    }
}

impl<T: 'static> Default for LazyStatic<T> {
    fn default() -> Self {
        Self::new()
    }
}

unsafe impl<T: Sync> Sync for OnceCell<T> {}
unsafe impl<T: Send> Send for OnceCell<T> {}

pub struct OnceCell<T> {
    data: UnsafeCell<MaybeUninit<T>>,
    state: AtomicU8,
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
        if self
            .state
            .compare_exchange(
                UNINITIALIZED,
                INITIALIZING,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_ok()
        {
            unsafe {
                self.data.get().write(MaybeUninit::new(value));
            }
            self.state.store(INITIALIZED, Ordering::Release);
            Ok(())
        } else {
            Err(value)
        }
    }

    pub fn try_insert(&self, value: T) -> Result<&T, (&T, T)> {
        if self.state.load(Ordering::Acquire) == INITIALIZED {
            return Err((self.get().expect("cell is initialized"), value));
        }

        match self.set(value) {
            Ok(()) => Ok(self.get().unwrap()),
            Err(value) => {
                while self.state.load(Ordering::Acquire) == INITIALIZING {
                    core::hint::spin_loop();
                }
                Err((self.get().expect("cell is initialized"), value))
            }
        }
    }

    pub fn get_or_init(&self, init: impl FnOnce() -> T) -> &T {
        loop {
            if let Some(v) = self.get() {
                return v;
            }

            if self
                .state
                .compare_exchange(
                    UNINITIALIZED,
                    INITIALIZING,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                )
                .is_ok()
            {
                let value = init();
                unsafe {
                    self.data.get().write(MaybeUninit::new(value));
                }
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
