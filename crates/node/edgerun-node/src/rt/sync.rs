//! Core synchronization primitives

use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use core::task::{Context, Poll, Waker};

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
    pub const fn new(data: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            data: core::cell::UnsafeCell::new(data),
        }
    }
}

impl<T> Mutex<T> {
    pub fn lock(&self) -> MutexGuard<'_, T> {
        while self
            .locked
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            while self.locked.load(Ordering::Relaxed) {
                core::hint::spin_loop();
            }
        }
        MutexGuard { mutex: self }
    }

    pub fn try_lock(&self) -> Option<MutexGuard<'_, T>> {
        if self.locked.load(Ordering::Acquire) {
            return None;
        }
        if self
            .locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Acquire)
            .is_ok()
        {
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
    waiters: Mutex<Vec<Waker>>,
}

impl Semaphore {
    pub fn new(permits: usize) -> Self {
        Self {
            permits: AtomicUsize::new(permits),
            waiters: Mutex::new(Vec::new()),
        }
    }

    pub fn acquire(&self) -> SemaphoreAcquire<'_> {
        SemaphoreAcquire {
            semaphore: self,
            waker: None,
        }
    }

    pub fn try_acquire(&self) -> Result<SemaphoreGuard<'_>, SemaphoreTryAcquireError> {
        let mut permits = self.permits.load(Ordering::Acquire);
        while permits > 0 {
            match self.permits.compare_exchange(
                permits,
                permits - 1,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return Ok(SemaphoreGuard(self)),
                Err(observed) => {
                    permits = observed;
                }
            }
        }
        Err(SemaphoreTryAcquireError::NoPermits)
    }
}

fn register_waker(waiters: &mut Vec<Waker>, waker: &Waker) {
    if !waiters.iter().any(|registered| registered.will_wake(waker)) {
        waiters.push(waker.clone());
    }
}

fn remove_waker(waiters: &mut Vec<Waker>, waker: &Waker) {
    if let Some(pos) = waiters
        .iter()
        .position(|registered| registered.will_wake(waker))
    {
        waiters.remove(pos);
    }
}

pub struct SemaphoreGuard<'a>(&'a Semaphore);

impl<'a> Drop for SemaphoreGuard<'a> {
    fn drop(&mut self) {
        self.0.permits.fetch_add(1, Ordering::Release);
        let waker_to_wake = {
            let mut waiters = self.0.waiters.lock();
            waiters.pop()
        };
        if let Some(waker) = waker_to_wake {
            waker.wake();
        }
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
    waker: Option<Waker>,
}

impl<'a> Future for SemaphoreAcquire<'a> {
    type Output = Result<SemaphoreGuard<'a>, SemaphoreAcquireError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        match this.semaphore.try_acquire() {
            Ok(guard) => {
                if let Some(waker) = this.waker.take() {
                    let mut waiters = this.semaphore.waiters.lock();
                    remove_waker(&mut waiters, &waker);
                }
                Poll::Ready(Ok(guard))
            }
            Err(_) => {
                let should_register = match this.waker.as_ref() {
                    Some(registered) => !registered.will_wake(cx.waker()),
                    None => true,
                };
                if should_register {
                    let mut waiters = this.semaphore.waiters.lock();
                    if let Some(previous) = this.waker.replace(cx.waker().clone()) {
                        remove_waker(&mut waiters, &previous);
                    }
                    register_waker(
                        &mut waiters,
                        this.waker.as_ref().expect("registered wakeup"),
                    );
                }
                Poll::Pending
            }
        }
    }
}

impl<'a> Drop for SemaphoreAcquire<'a> {
    fn drop(&mut self) {
        if let Some(waker) = self.waker.take() {
            let mut waiters = self.semaphore.waiters.lock();
            remove_waker(&mut waiters, &waker);
        }
    }
}

pub struct Condvar;

impl Condvar {
    pub fn new() -> Self {
        Self
    }
    pub fn notify_one(&self) {}
    pub fn notify_all(&self) {}
}

pub struct RwLock<T> {
    data: core::cell::UnsafeCell<T>,
    state: AtomicUsize,
}

unsafe impl<T: Send> Send for RwLock<T> {}
unsafe impl<T: Send> Sync for RwLock<T> {}

impl<T> RwLock<T> {
    pub fn new(data: T) -> Self {
        Self {
            data: core::cell::UnsafeCell::new(data),
            state: AtomicUsize::new(0),
        }
    }

    pub fn read(&self) -> RwLockReadGuard<'_, T> {
        loop {
            let state = self.state.load(Ordering::Acquire);
            if state & 1 != 0 {
                core::hint::spin_loop();
                continue;
            }

            if self
                .state
                .compare_exchange_weak(state, state + 2, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
            {
                break;
            }
        }
        RwLockReadGuard { lock: self }
    }

    pub fn try_read(&self) -> Option<RwLockReadGuard<'_, T>> {
        let state = self.state.load(Ordering::Acquire);
        if state & 1 != 0 {
            return None;
        }
        if self
            .state
            .compare_exchange(state, state + 2, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            return None;
        }
        Some(RwLockReadGuard { lock: self })
    }

    pub fn write(&self) -> RwLockWriteGuard<'_, T> {
        loop {
            if self
                .state
                .compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
            {
                break;
            }
            core::hint::spin_loop();
        }
        RwLockWriteGuard { lock: self }
    }

    pub fn try_write(&self) -> Option<RwLockWriteGuard<'_, T>> {
        if self
            .state
            .compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            return None;
        }
        Some(RwLockWriteGuard { lock: self })
    }
}

pub struct RwLockReadGuard<'a, T> {
    lock: &'a RwLock<T>,
}

impl<'a, T> Drop for RwLockReadGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.state.fetch_sub(2, Ordering::Release);
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
        self.lock.state.fetch_and(!1, Ordering::Release);
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
    waiters: Mutex<Vec<Waker>>,
}

impl<T> AsyncMutex<T> {
    pub fn new(data: T) -> Self {
        Self {
            inner: Mutex::new(data),
            waiters: Mutex::new(Vec::new()),
        }
    }

    pub fn lock(&self) -> AsyncMutexLock<'_, T> {
        AsyncMutexLock {
            mutex: self,
            waker: None,
        }
    }
}

pub struct AsyncMutexLock<'a, T> {
    mutex: &'a AsyncMutex<T>,
    waker: Option<Waker>,
}

pub struct AsyncMutexGuard<'a, T> {
    inner: Option<MutexGuard<'a, T>>,
    mutex: &'a AsyncMutex<T>,
}

impl<'a, T> AsyncMutexGuard<'a, T> {
    fn new(guard: MutexGuard<'a, T>, mutex: &'a AsyncMutex<T>) -> Self {
        Self {
            inner: Some(guard),
            mutex,
        }
    }
}

impl<'a, T> core::ops::Deref for AsyncMutexGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner
            .as_ref()
            .expect("locked guard must be present")
            .deref()
    }
}

impl<'a, T> core::ops::DerefMut for AsyncMutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner
            .as_mut()
            .expect("locked guard must be present")
            .deref_mut()
    }
}

impl<'a, T> Drop for AsyncMutexGuard<'a, T> {
    fn drop(&mut self) {
        self.inner = None;
        let waker_to_wake = {
            let mut waiters = self.mutex.waiters.lock();
            waiters.pop()
        };
        if let Some(waker) = waker_to_wake {
            waker.wake();
        }
    }
}

impl<'a, T> Future for AsyncMutexLock<'a, T> {
    type Output = AsyncMutexGuard<'a, T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        match this.mutex.inner.try_lock() {
            Some(guard) => {
                if let Some(waker) = this.waker.take() {
                    let mut waiters = this.mutex.waiters.lock();
                    remove_waker(&mut waiters, &waker);
                }
                Poll::Ready(AsyncMutexGuard::new(guard, this.mutex))
            }
            None => {
                let should_register = match this.waker.as_ref() {
                    Some(registered) => !registered.will_wake(cx.waker()),
                    None => true,
                };
                if should_register {
                    let mut waiters = this.mutex.waiters.lock();
                    if let Some(previous) = this.waker.replace(cx.waker().clone()) {
                        remove_waker(&mut waiters, &previous);
                    }
                    register_waker(
                        &mut waiters,
                        this.waker.as_ref().expect("registered wakeup"),
                    );
                }
                Poll::Pending
            }
        }
    }
}

impl<'a, T> Drop for AsyncMutexLock<'a, T> {
    fn drop(&mut self) {
        if let Some(waker) = self.waker.take() {
            let mut waiters = self.mutex.waiters.lock();
            remove_waker(&mut waiters, &waker);
        }
    }
}
