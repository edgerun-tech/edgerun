//! Synchronization primitives — drop-in replacements for parking_lot.
//!
//! Provides `Mutex`, `MutexGuard`, `RwLock`, `RwLockReadGuard`,
//! `RwLockWriteGuard`, and `Condvar` with the same API surface as
//! parking_lot (no poisoning, no `LockResult` wrappers).
//!
//! ## Design
//! - `MutexGuard` stores a raw pointer to the parent `Mutex` so that
//!   `Condvar::wait(&mut guard)` can re-lock after being notified.
//! - All types are thin wrappers over `std::sync` equivalents.
//! - Poisoned locks are automatically recovered (parking_lot semantics).

use std::marker::PhantomData;
use std::ptr::{self, NonNull};
use std::sync::PoisonError;
use std::sync::{
    Condvar as StdCondvar, Mutex as StdMutex, MutexGuard as StdMutexGuard,
    RwLock as StdRwLock, RwLockReadGuard as StdRwLockReadGuard,
    RwLockWriteGuard as StdRwLockWriteGuard,
};
use std::time::Duration;

// ===========================================================================
// Mutex
// ===========================================================================

/// A mutual-exclusion lock.
///
/// Unlike `std::sync::Mutex`, `lock()` returns a guard directly — no
/// `LockResult` wrapper. Poisoned locks are automatically recovered
/// (matching parking_lot behavior).
pub struct Mutex<T: ?Sized> {
    inner: StdMutex<T>,
}

impl<T> Mutex<T> {
    /// Creates a new mutex in an unlocked state ready for use.
    #[inline]
    pub const fn new(data: T) -> Self {
        Self {
            inner: StdMutex::new(data),
        }
    }
}

impl<T: ?Sized> Mutex<T> {
    /// Acquires the mutex, blocking the current thread until it is able to do so.
    ///
    /// If a concurrent thread panicked while holding the lock, the poisoned
    /// state is recovered automatically (parking_lot semantics).
    #[inline]
    pub fn lock(&self) -> MutexGuard<'_, T> {
        let guard = match self.inner.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        MutexGuard {
            mutex: NonNull::from(self),
            guard,
            _marker: PhantomData,
        }
    }

    /// Attempts to acquire the mutex in a non-blocking fashion.
    ///
    /// Returns `Some(guard)` on success, `None` if the mutex is already locked.
    #[inline]
    pub fn try_lock(&self) -> Option<MutexGuard<'_, T>> {
        self.inner.try_lock().ok().map(|guard| MutexGuard {
            mutex: NonNull::from(self),
            guard,
            _marker: PhantomData,
        })
    }

    /// Returns a mutable reference to the underlying data.
    ///
    /// This call borrows the `Mutex` mutably — no locking required.
    #[inline]
    pub fn get_mut(&mut self) -> &mut T {
        self.inner.get_mut().unwrap_or_else(|e| e.into_inner())
    }

    /// Consumes the `Mutex`, returning the underlying data.
    #[inline]
    pub fn into_inner(self) -> T
    where
        T: Sized,
    {
        self.inner.into_inner().unwrap_or_else(|e| e.into_inner())
    }

    /// Returns a reference to the underlying `std::sync::Mutex`.
    /// Used internally by `Condvar::wait`.
    pub(crate) fn std_mutex(&self) -> &StdMutex<T> {
        &self.inner
    }
}

impl<T: Default> Default for Mutex<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T: ?Sized + std::fmt::Debug> std::fmt::Debug for Mutex<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.inner, f)
    }
}

unsafe impl<T: ?Sized + Send> Send for Mutex<T> {}
unsafe impl<T: ?Sized + Send> Sync for Mutex<T> {}

/// A guard that holds exclusive access to a [`Mutex`].
///
/// Stores a pointer to the parent mutex so that `Condvar::wait` can
/// re-lock after being notified.
pub struct MutexGuard<'a, T: ?Sized> {
    mutex: NonNull<Mutex<T>>,
    guard: StdMutexGuard<'a, T>,
    _marker: PhantomData<&'a Mutex<T>>,
}

impl<T: ?Sized> MutexGuard<'_, T> {
    /// Returns a reference to the parent `Mutex`.
    pub fn mutex(&self) -> &Mutex<T> {
        unsafe { self.mutex.as_ref() }
    }
}

impl<T: ?Sized> std::ops::Deref for MutexGuard<'_, T> {
    type Target = T;
    #[inline]
    fn deref(&self) -> &T {
        self.guard.deref()
    }
}

impl<T: ?Sized> std::ops::DerefMut for MutexGuard<'_, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut T {
        self.guard.deref_mut()
    }
}

impl<T: ?Sized + std::fmt::Debug> std::fmt::Debug for MutexGuard<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.guard, f)
    }
}

// ===========================================================================
// RwLock
// ===========================================================================

/// A reader-writer lock.
///
/// Unlike `std::sync::RwLock`, `read()` and `write()` return guards
/// directly — no `LockResult` wrappers. Poisoned locks are recovered.
pub struct RwLock<T: ?Sized> {
    inner: StdRwLock<T>,
}

impl<T> RwLock<T> {
    /// Creates a new rwlock in an unlocked state ready for use.
    #[inline]
    pub const fn new(data: T) -> Self {
        Self {
            inner: StdRwLock::new(data),
        }
    }
}

impl<T: ?Sized> RwLock<T> {
    /// Acquires a read lock, blocking the current thread until it can be acquired.
    #[inline]
    pub fn read(&self) -> RwLockReadGuard<'_, T> {
        let guard = match self.inner.read() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        RwLockReadGuard { guard }
    }

    /// Attempts to acquire a read lock non-blocking.
    #[inline]
    pub fn try_read(&self) -> Option<RwLockReadGuard<'_, T>> {
        self.inner.try_read().ok().map(|guard| RwLockReadGuard { guard })
    }

    /// Acquires a write lock, blocking the current thread until it can be acquired.
    #[inline]
    pub fn write(&self) -> RwLockWriteGuard<'_, T> {
        let guard = match self.inner.write() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        RwLockWriteGuard { guard }
    }

    /// Attempts to acquire a write lock non-blocking.
    #[inline]
    pub fn try_write(&self) -> Option<RwLockWriteGuard<'_, T>> {
        self.inner.try_write().ok().map(|guard| RwLockWriteGuard { guard })
    }

    /// Consumes this `RwLock`, returning the underlying data.
    #[inline]
    pub fn into_inner(self) -> T
    where
        T: Sized,
    {
        match self.inner.into_inner() {
            Ok(val) => val,
            Err(poisoned) => poisoned.into_inner(),
        }
    }
}

impl<T: Default> Default for RwLock<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T: ?Sized + std::fmt::Debug> std::fmt::Debug for RwLock<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.inner, f)
    }
}

unsafe impl<T: ?Sized + Send> Send for RwLock<T> {}
unsafe impl<T: ?Sized + Send + Sync> Sync for RwLock<T> {}

/// A guard that holds shared read access to a [`RwLock`].
pub struct RwLockReadGuard<'a, T: ?Sized> {
    guard: StdRwLockReadGuard<'a, T>,
}

impl<T: ?Sized> std::ops::Deref for RwLockReadGuard<'_, T> {
    type Target = T;
    #[inline]
    fn deref(&self) -> &T {
        self.guard.deref()
    }
}

impl<T: ?Sized + std::fmt::Debug> std::fmt::Debug for RwLockReadGuard<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.guard, f)
    }
}

/// A guard that holds exclusive write access to a [`RwLock`].
pub struct RwLockWriteGuard<'a, T: ?Sized> {
    guard: StdRwLockWriteGuard<'a, T>,
}

impl<T: ?Sized> std::ops::Deref for RwLockWriteGuard<'_, T> {
    type Target = T;
    #[inline]
    fn deref(&self) -> &T {
        self.guard.deref()
    }
}

impl<T: ?Sized> std::ops::DerefMut for RwLockWriteGuard<'_, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut T {
        self.guard.deref_mut()
    }
}

impl<T: ?Sized + std::fmt::Debug> std::fmt::Debug for RwLockWriteGuard<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.guard, f)
    }
}

// ===========================================================================
// Condvar
// ===========================================================================

/// A condition variable.
///
/// Compatible with our [`Mutex`]. Provides `wait`, `wait_for`, `notify_one`,
/// and `notify_all`.
pub struct Condvar(StdCondvar);

impl Condvar {
    /// Creates a new condition variable.
    #[inline]
    pub const fn new() -> Self {
        Self(StdCondvar::new())
    }

    /// Blocks the current thread until this condition variable receives a notification.
    ///
    /// The mutex is atomically released and the thread is blocked.
    /// When woken, the mutex is reacquired.
    #[inline]
    pub fn wait<T>(&self, guard: &mut MutexGuard<'_, T>) {
        let old_guard = unsafe { ptr::read(&guard.guard) };
        let new_guard = self.0.wait(old_guard).unwrap_or_else(|e| {
            e.into_inner()
        });
        unsafe { ptr::write(&mut guard.guard, new_guard) };
    }

    /// Waits for a notification, or returns if the timeout has elapsed.
    ///
    /// Returns `true` if notified, `false` if the timeout elapsed.
    #[inline]
    pub fn wait_for<T>(&self, guard: &mut MutexGuard<'_, T>, timeout: Duration) -> bool {
        let old_guard = unsafe { ptr::read(&guard.guard) };
        let (new_guard, timeout_result) = self.0.wait_timeout(old_guard, timeout)
            .unwrap_or_else(|e| e.into_inner());
        unsafe { ptr::write(&mut guard.guard, new_guard) };
        !timeout_result.timed_out()
    }

    /// Wakes one blocked thread waiting on this condition variable.
    #[inline]
    pub fn notify_one(&self) {
        self.0.notify_one();
    }

    /// Wakes all blocked threads waiting on this condition variable.
    #[inline]
    pub fn notify_all(&self) {
        self.0.notify_all();
    }
}

impl Default for Condvar {
    fn default() -> Self {
        Self::new()
    }
}
