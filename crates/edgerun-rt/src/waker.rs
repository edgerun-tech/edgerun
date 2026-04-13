//! Custom waker implementation.
//!
//! Each waker carries a task ID. When `wake()` is called, the task ID
//! is pushed onto the ready queue so a worker thread can poll it.
//!
//! ## Reference counting invariant
//! `make_waker()` creates a `RawWaker` with strong count = 1 (Arc::into_raw).
//! - `wake`: consumes `self` → `Arc::from_raw` (decrements to 0, frees). No increment.
//! - `clone`: returns a new `RawWaker` → `Arc::increment_strong_count` (count +1).
//! - `wake_by_ref`: `Arc::increment_strong_count` + `Arc::from_raw` (clone, then drop clone).
//! - `drop`: `Arc::from_raw` (decrements).

use std::sync::Arc;
use std::task::{RawWaker, RawWakerVTable, Waker};

use crate::ready_queue::ReadyQueue;

struct WData {
    id: usize,
    q: Arc<ReadyQueue>,
}

static VTABLE: RawWakerVTable =
    RawWakerVTable::new(wk_clone, wk_wake, wk_wake_by_ref, wk_drop);

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
pub(crate) fn make_waker(id: usize, q: Arc<ReadyQueue>) -> Waker {
    let data = Arc::new(WData { id, q });
    let ptr = Arc::into_raw(data) as *const ();
    unsafe { Waker::from_raw(RawWaker::new(ptr, &VTABLE)) }
}
