//! Core synchronization primitives

extern crate alloc;

use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

pub struct Mutex<T> {
    locked: AtomicBool,
    data: core::cell::UnsafeCell<T>,
}

unsafe impl<T: Send> Send for Mutex<T> {}
unsafe impl<T: Send> Sync for Mutex<T> {}

impl<T> Mutex<T> {
    pub fn new(data: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            data: core::cell::UnsafeCell::new(data),
        }
    }
}

impl<T> Mutex<T> {
    pub fn lock(&self) -> MutexGuard<'_, T> {
        while self.locked.load(Ordering::Acquire) {
            core::hint::spin_loop();
        }
        self.locked.store(true, Ordering::Release);
        MutexGuard { mutex: self }
    }

    pub fn try_lock(&self) -> Option<MutexGuard<'_, T>> {
        if self.locked.load(Ordering::Acquire) {
            return None;
        }
        if self.locked.compare_exchange(false, true, Ordering::Acquire, Ordering::Acquire).is_ok() {
            return Some(MutexGuard { mutex: self });
        }
        None
    }

    pub fn is_locked(&self) -> bool {
        self.locked.load(Ordering::Acquire)
    }
}

pub struct MutexGuard<'a, T> {
    mutex: &'a Mutex<T>,
}

impl<'a, T> Drop for MutexGuard<'a, T> {
    fn drop(&mut self) {
        self.mutex.locked.store(false, Ordering::Release);
    }
}

impl<'a, T> core::ops::Deref for MutexGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.mutex.data.get() }
    }
}

impl<'a, T> core::ops::DerefMut for MutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.mutex.data.get() }
    }
}

pub struct Semaphore {
    permits: AtomicUsize,
}

impl Semaphore {
    pub fn new(permits: usize) -> Self {
        Self {
            permits: AtomicUsize::new(permits),
        }
    }

    pub fn acquire(&self) -> Option<SemaphoreGuard<'_>> {
        loop {
            let n = self.permits.load(Ordering::Acquire);
            if n == 0 {
                return None;
            }
            if self.permits.compare_exchange(n, n - 1, Ordering::Acquire, Ordering::Acquire).is_ok() {
                return Some(SemaphoreGuard(&self.permits));
            }
        }
    }

    pub fn try_acquire(&self) -> Option<SemaphoreGuard<'_>> {
        let n = self.permits.load(Ordering::Acquire);
        if n == 0 {
            return None;
        }
        if self.permits.compare_exchange(n, n - 1, Ordering::Acquire, Ordering::Acquire).is_ok() {
            return Some(SemaphoreGuard(&self.permits));
        }
        None
    }
}

pub struct SemaphoreGuard<'a>(&'a AtomicUsize);

impl<'a> Drop for SemaphoreGuard<'a> {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::Release);
    }
}

pub struct Condvar;

impl Condvar {
    pub fn new() -> Self { Self }
    pub fn notify_one(&self) {}
    pub fn notify_all(&self) {}
}

pub struct RwLock<T> {
    data: core::cell::UnsafeCell<T>,
    writer: AtomicBool,
    readers: AtomicUsize,
}

unsafe impl<T: Send> Send for RwLock<T> {}
unsafe impl<T: Send> Sync for RwLock<T> {}

impl<T> RwLock<T> {
    pub fn new(data: T) -> Self {
        Self {
            data: core::cell::UnsafeCell::new(data),
            writer: AtomicBool::new(false),
            readers: AtomicUsize::new(0),
        }
    }

    pub fn read(&self) -> RwLockReadGuard<'_, T> {
        while self.writer.load(Ordering::Acquire) {
            core::hint::spin_loop();
        }
        self.readers.fetch_add(1, Ordering::AcqRel);
        RwLockReadGuard { lock: self }
    }

    pub fn try_read(&self) -> Option<RwLockReadGuard<'_, T>> {
        if self.writer.load(Ordering::Acquire) {
            return None;
        }
        self.readers.fetch_add(1, Ordering::AcqRel);
        Some(RwLockReadGuard { lock: self })
    }

    pub fn write(&self) -> RwLockWriteGuard<'_, T> {
        while self.writer.load(Ordering::Acquire) {
            core::hint::spin_loop();
        }
        self.writer.store(true, Ordering::Release);
        RwLockWriteGuard { lock: self }
    }

    pub fn try_write(&self) -> Option<RwLockWriteGuard<'_, T>> {
        if self.writer.load(Ordering::Acquire) {
            return None;
        }
        self.writer.store(true, Ordering::Release);
        Some(RwLockWriteGuard { lock: self })
    }
}

pub struct RwLockReadGuard<'a, T> {
    lock: &'a RwLock<T>,
}

impl<'a, T> Drop for RwLockReadGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.readers.fetch_sub(1, Ordering::Release);
    }
}

pub struct RwLockWriteGuard<'a, T> {
    lock: &'a RwLock<T>,
}

impl<'a, T> Drop for RwLockWriteGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.writer.store(false, Ordering::Release);
    }
}