//! Core synchronization primitives

extern crate alloc;

use core::future::Future;
use core::pin::Pin;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use core::task::{Context, Poll};

pub struct Mutex<T> {
    locked: AtomicBool,
    data: core::cell::UnsafeCell<T>,
}

unsafe impl<T: Send> Send for Mutex<T> {}
unsafe impl<T: Send> Sync for Mutex<T> {}

pub struct SpinLock {
    locked: AtomicBool,
}

impl SpinLock {
    pub const fn new() -> Self {
        Self {
            locked: AtomicBool::new(false),
        }
    }

    pub fn lock(&self) -> SpinLockGuard<'_> {
        while self
            .locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }
        SpinLockGuard { lock: self }
    }

    pub fn try_lock(&self) -> Option<SpinLockGuard<'_>> {
        self.locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .ok()
            .map(|_| SpinLockGuard { lock: self })
    }

    pub fn is_locked(&self) -> bool {
        self.locked.load(Ordering::Acquire)
    }
}

impl Default for SpinLock {
    fn default() -> Self {
        Self::new()
    }
}

pub struct SpinLockGuard<'a> {
    lock: &'a SpinLock,
}

impl Drop for SpinLockGuard<'_> {
    fn drop(&mut self) {
        self.lock.locked.store(false, Ordering::Release);
    }
}

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

    pub fn acquire(&self) -> SemaphoreAcquire<'_> {
        SemaphoreAcquire { semaphore: self }
    }

    pub fn try_acquire(&self) -> Result<SemaphoreGuard<'_>, SemaphoreTryAcquireError> {
        let n = self.permits.load(Ordering::Acquire);
        if n == 0 {
            return Err(SemaphoreTryAcquireError::NoPermits);
        }
        if self.permits.compare_exchange(n, n - 1, Ordering::Acquire, Ordering::Acquire).is_ok() {
            return Ok(SemaphoreGuard(&self.permits));
        }
        Err(SemaphoreTryAcquireError::NoPermits)
    }
}

pub struct SemaphoreGuard<'a>(&'a AtomicUsize);

impl<'a> Drop for SemaphoreGuard<'a> {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::Release);
    }
}

pub type Permit<'a> = SemaphoreGuard<'a>;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum SemaphoreTryAcquireError {
    NoPermits,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct SemaphoreAcquireError;

pub struct SemaphoreAcquire<'a> {
    semaphore: &'a Semaphore,
}

impl<'a> Future for SemaphoreAcquire<'a> {
    type Output = Result<SemaphoreGuard<'a>, SemaphoreAcquireError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.semaphore.try_acquire() {
            Ok(guard) => Poll::Ready(Ok(guard)),
            Err(_) => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        }
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

impl<'a, T> core::ops::Deref for RwLockReadGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.lock.data.get() }
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

impl<'a, T> core::ops::Deref for RwLockWriteGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.lock.data.get() }
    }
}

impl<'a, T> core::ops::DerefMut for RwLockWriteGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.lock.data.get() }
    }
}

pub struct AsyncMutex<T> {
    inner: Mutex<T>,
}

impl<T> AsyncMutex<T> {
    pub fn new(data: T) -> Self {
        Self {
            inner: Mutex::new(data),
        }
    }

    pub fn lock(&self) -> AsyncMutexLock<'_, T> {
        AsyncMutexLock { mutex: self }
    }
}

pub struct AsyncMutexLock<'a, T> {
    mutex: &'a AsyncMutex<T>,
}

impl<'a, T> Future for AsyncMutexLock<'a, T> {
    type Output = MutexGuard<'a, T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.mutex.inner.try_lock() {
            Some(guard) => Poll::Ready(guard),
            None => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        }
    }
}
