//! Unbounded mpsc channel - no backpressure, always succeeds.


extern crate alloc;

use alloc::sync::Arc;
use alloc::vec::Vec;
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

const ACQUIRE: Ordering = Ordering::Acquire;
const RELEASE: Ordering = Ordering::Release;

// ===========================================================================
// Unbounded channel
// ===========================================================================

struct UnboundedInner<T> {
    queue: UnsafeCell<Vec<T>>,
    closed: AtomicBool,
    sender_count: AtomicUsize,
}

impl<T> UnboundedInner<T> {
    fn new() -> Self {
        Self {
            queue: UnsafeCell::new(Vec::new()),
            closed: AtomicBool::new(false),
            sender_count: AtomicUsize::new(1),
        }
    }
}

/// Creates a new unbounded mpsc channel.
pub fn channel<T: Send>() -> (Sender<T>, Receiver<T>) {
    let inner = Arc::new(UnboundedInner::new());
    (Sender { inner: inner.clone() }, Receiver { inner })
}

pub struct Sender<T> {
    inner: Arc<UnboundedInner<T>>,
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        self.inner.sender_count.fetch_add(1, RELEASE);
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        if self.inner.sender_count.fetch_sub(1, ACQUIRE) == 1 {
            self.inner.closed.store(true, RELEASE);
        }
    }
}

impl<T> Sender<T> {
    pub fn send(&self, value: T) -> Result<(), SendError<T>> {
        if self.inner.closed.load(Ordering::Relaxed) {
            return Err(SendError(value));
        }
        unsafe { (*self.inner.queue.get()).push(value) };
        Ok(())
    }

    pub fn len(&self) -> usize {
        unsafe { (*self.inner.queue.get()).len() }
    }

    pub fn is_empty(&self) -> bool {
        unsafe { (*self.inner.queue.get()).is_empty() }
    }

    pub fn is_closed(&self) -> bool {
        self.inner.closed.load(ACQUIRE)
    }
}

pub struct Receiver<T> {
    inner: Arc<UnboundedInner<T>>,
}

impl<T> Receiver<T> {
    pub fn try_recv(&self) -> Option<T> {
        if self.inner.closed.load(ACQUIRE) && unsafe { (*self.inner.queue.get()).is_empty() } {
            None
        } else {
            unsafe { (*self.inner.queue.get()).pop() }
        }
    }

    pub fn blocking_recv(&mut self) -> Option<T> {
        loop {
            if let Some(v) = self.try_recv() {
                return Some(v);
            }
            if self.inner.closed.load(ACQUIRE) {
                return None;
            }
            core::hint::spin_loop();
        }
    }
}

#[derive(Debug)]
pub struct SendError<T>(pub T);