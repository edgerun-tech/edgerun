//! Synchronization primitives - no_std implementations.

#![no_std]

extern crate alloc;

use alloc::boxed::Box;
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicUsize, AtomicBool, Ordering};

const Acquire: Ordering = Ordering::Acquire;
const Release: Ordering = Ordering::Release;
const AcqRel: Ordering = Ordering::AcqRel;
const Relaxed: Ordering = Ordering::Relaxed;

// ===========================================================================
// Mutex
// ===========================================================================

pub struct Mutex<T: ?Sized> {
    state: AtomicBool,
    data: UnsafeCell<T>,
}

unsafe impl<T: ?Sized + Send> Send for Mutex<T> {}
unsafe impl<T: ?Sized + Send> Sync for Mutex<T> {}

impl<T> Mutex<T> {
    pub const fn new(data: T) -> Self {
        Self {
            state: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }
}

impl<T: ?Sized> Mutex<T> {
    pub fn lock(&self) -> MutexGuard<'_, T> {
        while self.state.load(Acquire) {
            core::hint::spin_loop();
        }
        self.state.store(true, Release);
        MutexGuard { mutex: self }
    }

    pub fn try_lock(&self) -> Option<MutexGuard<'_, T>> {
        if self.state.load(Relaxed) == false
            && self.state.compare_exchange(false, true, Acquire, Release).is_ok()
        {
            Some(MutexGuard { mutex: self })
        } else {
            None
        }
    }
}

pub struct MutexGuard<'a, T: ?Sized> {
    mutex: &'a Mutex<T>,
}

impl<T: ?Sized> Drop for MutexGuard<'_, T> {
    fn drop(&mut self) {
        self.mutex.state.store(false, Release);
    }
}

impl<T: ?Sized> core::ops::Deref for MutexGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.mutex.data.get() }
    }
}

impl<T: ?Sized> core::ops::DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.mutex.data.get() }
    }
}

// ===========================================================================
// RwLock
// ===========================================================================

const WRITE_LOCKED: usize = usize::MAX;

pub struct RwLock<T: ?Sized> {
    state: AtomicUsize,
    data: UnsafeCell<T>,
}

unsafe impl<T: ?Sized + Send> Send for RwLock<T> {}
unsafe impl<T: ?Sized + Send + Sync> Sync for RwLock<T> {}

impl<T> RwLock<T> {
    pub const fn new(data: T) -> Self {
        Self {
            state: AtomicUsize::new(0),
            data: UnsafeCell::new(data),
        }
    }
}

impl<T: ?Sized> RwLock<T> {
    pub fn read(&self) -> RwLockReadGuard<'_, T> {
        loop {
            let s = self.state.load(Relaxed);
            if s < WRITE_LOCKED {
                if self.state.compare_exchange(s, s + 1, Acquire, Relaxed).is_ok() {
                    return RwLockReadGuard { lock: self };
                }
            }
            core::hint::spin_loop();
        }
    }

    pub fn try_read(&self) -> Option<RwLockReadGuard<'_, T>> {
        let s = self.state.load(Relaxed);
        if s < WRITE_LOCKED && self.state.compare_exchange(s, s + 1, Acquire, Relaxed).is_ok() {
            Some(RwLockReadGuard { lock: self })
        } else {
            None
        }
    }

    pub fn write(&self) -> RwLockWriteGuard<'_, T> {
        loop {
            if self.state.compare_exchange(0, WRITE_LOCKED, Acquire, Relaxed).is_ok() {
                return RwLockWriteGuard { lock: self };
            }
            core::hint::spin_loop();
        }
    }
}

pub struct RwLockReadGuard<'a, T: ?Sized> {
    lock: &'a RwLock<T>,
}

impl<T: ?Sized> Drop for RwLockReadGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.state.fetch_sub(1, Release);
    }
}

impl<T: ?Sized> core::ops::Deref for RwLockReadGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}

pub struct RwLockWriteGuard<'a, T: ?Sized> {
    lock: &'a RwLock<T>,
}

impl<T: ?Sized> Drop for RwLockWriteGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.state.store(0, Release);
    }
}

impl<T: ?Sized> core::ops::Deref for RwLockWriteGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}

impl<T: ?Sized> core::ops::DerefMut for RwLockWriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.data.get() }
    }
}

// ===========================================================================
// Condvar
// ===========================================================================

struct Waiter {
    signaled: AtomicBool,
    next: usize,
}

impl Waiter {
    const fn new() -> Self {
        Self {
            signaled: AtomicBool::new(false),
            next: 0,
        }
    }
}

pub struct Condvar {
    head: AtomicUsize,
}

impl Condvar {
    pub const fn new() -> Self {
        Self {
            head: AtomicUsize::new(0),
        }
    }

    pub fn wait<T>(&self, guard: &mut MutexGuard<'_, T>) {
        let waiter = Box::into_raw(Box::new(Waiter::new())) as usize;
        loop {
            let next = self.head.load(Relaxed);
            unsafe { (*(waiter as *mut Waiter)).next = next };
            if self.head.compare_exchange(next, waiter, AcqRel, Relaxed).is_ok() {
                break;
            }
        }
        while !unsafe { (*(waiter as *mut Waiter)).signaled.load(Acquire) } {
            core::hint::spin_loop();
        }
        guard.mutex.lock();
    }

    pub fn notify_one(&self) {
        loop {
            let ptr = self.head.load(Relaxed);
            if ptr == 0 { return; }
            let next = unsafe { (*(ptr as *mut Waiter)).next };
            if self.head.compare_exchange(ptr, next, AcqRel, Relaxed).is_ok() {
                unsafe { (*(ptr as *mut Waiter)).signaled.store(true, Release) };
                return;
            }
        }
    }

    pub fn notify_all(&self) {
        loop {
            let ptr = self.head.swap(0, AcqRel);
            if ptr == 0 { return; }
            let mut current = ptr;
            while current != 0 {
                let next = unsafe { (*(current as *mut Waiter)).next };
                unsafe { (*(current as *mut Waiter)).signaled.store(true, Release) };
                current = next;
            }
        }
    }
}