//! Lazy static and once cell for bare-metal

use core::cell::UnsafeCell;
use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicU8, Ordering};

const UNINITIALIZED: u8 = 0;
const INITIALIZING: u8 = 1;
const INITIALIZED: u8 = 2;

// `get()` can initialize from any thread and then publish a shared reference.
// `T: Send` is required for cross-thread initialization ownership, `T: Sync`
// is required because callers receive `&T` from a shared static.
unsafe impl<T: Send + Sync> Sync for LazyStatic<T> {}
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
                    #[cfg(not(target_os = "none"))]
                    let value = {
                        use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
                        match catch_unwind(AssertUnwindSafe(init)) {
                            Ok(value) => value,
                            Err(err) => {
                                self.state.store(UNINITIALIZED, Ordering::Release);
                                resume_unwind(err);
                            }
                        }
                    };
                    #[cfg(target_os = "none")]
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

impl<T> Default for LazyStatic<T> {
    fn default() -> Self {
        Self::new()
    }
}

// `set()` can publish a value through `&self`, so `T` must be `Send` to move
// into the cell from another thread and `Sync` to share by reference after init.
unsafe impl<T: Send + Sync> Sync for OnceCell<T> {}
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
        let mut pending = Some(value);

        loop {
            let Some(value) = pending.take() else {
                while self.state.load(Ordering::Acquire) == INITIALIZING {
                    core::hint::spin_loop();
                }
                continue;
            };
            match self.set(value) {
                Ok(()) => {
                    if let Some(value) = self.get() {
                        return Ok(value);
                    }
                    unreachable!("OnceCell set returned Ok but cell is not initialized");
                }
                Err(value) => match self.state.load(Ordering::Acquire) {
                    INITIALIZING => {
                        pending = Some(value);
                        while self.state.load(Ordering::Acquire) == INITIALIZING {
                            core::hint::spin_loop();
                        }
                    }
                    INITIALIZED => {
                        if let Some(existing) = self.get() {
                            return Err((existing, value));
                        }
                        unreachable!("OnceCell is initialized but get returned None");
                    }
                    UNINITIALIZED => {
                        pending = Some(value);
                    }
                    _ => unreachable!(),
                },
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
                #[cfg(not(target_os = "none"))]
                let value = {
                    use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
                    match catch_unwind(AssertUnwindSafe(init)) {
                        Ok(value) => value,
                        Err(err) => {
                            self.state.store(UNINITIALIZED, Ordering::Release);
                            resume_unwind(err);
                        }
                    }
                };
                #[cfg(target_os = "none")]
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
