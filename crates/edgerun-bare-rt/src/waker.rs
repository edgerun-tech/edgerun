//! Custom waker implementation.
//!
//! Each waker carries a task ID. When `wake()` is called,
//! the task ID is pushed onto the ready queue.

#![no_std]

use alloc::sync::Arc;
use core::sync::atomic::{AtomicUsize, Ordering};
use core::task::{RawWaker, RawWakerVTable, Waker};

use crate::ready_queue::ReadyQueue;

const Acquire: Ordering = Ordering::Acquire;
const Release: Ordering = Ordering::Release;
const AcqRel: Ordering = Ordering::AcqRel;

// ===========================================================================
// Waker
// ===========================================================================

struct WData {
    id: usize,
    q: Arc<ReadyQueue>,
}

static VTABLE: RawWakerVTable = RawWakerVTable::new(wk_clone, wk_wake, wk_wake_by_ref, wk_drop);

unsafe fn wk_clone(d: *const ()) -> RawWaker {
    Arc::increment_strong_count(d as *const WData);
    RawWaker::new(d, &VTABLE)
}

unsafe fn wk_wake(d: *const ()) {
    let a = Arc::from_raw(d as *const WData);
    a.q.push(a.id);
}

unsafe fn wk_wake_by_ref(d: *const ()) {
    Arc::increment_strong_count(d as *const WData);
    let a = Arc::from_raw(d as *const WData);
    a.q.push(a.id);
}

unsafe fn wk_drop(d: *const ()) {
    let _ = Arc::from_raw(d as *const WData);
}

/// Create a waker that enqueues the given task ID on wake.
pub fn make_waker(id: usize, q: Arc<ReadyQueue>) -> Waker {
    let data = Arc::new(WData { id, q });
    let ptr = Arc::into_raw(data) as *const ();
    unsafe { Waker::from_raw(RawWaker::new(ptr, &VTABLE)) }
}