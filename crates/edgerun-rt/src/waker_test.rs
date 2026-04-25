//! Unit tests for waker.rs — tests the RawWakerVTable implementation
//! directly without the runtime.

use crate::ready_queue::ReadyQueue;
use crate::waker::make_waker;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[test]
fn waker_wake_consumes() {
    let q = Arc::new(ReadyQueue::new());
    let w = make_waker(42, q.clone());
    assert_eq!(q.len(), 0);
    w.wake();
    assert_eq!(q.len(), 1);
    assert_eq!(q.pop(), Some(42));
}

#[test]
fn waker_wake_by_ref_does_not_consume() {
    let q = Arc::new(ReadyQueue::new());
    let w = make_waker(7, q.clone());
    assert_eq!(q.len(), 0);
    w.wake_by_ref();
    assert_eq!(q.len(), 1);
    assert_eq!(q.pop(), Some(7));
    // Waker still usable.
    w.wake();
    assert_eq!(q.len(), 1);
    assert_eq!(q.pop(), Some(7));
}

#[test]
fn waker_clone_produces_independent_waker() {
    let q = Arc::new(ReadyQueue::new());
    let w1 = make_waker(99, q.clone());
    let w2 = w1.clone();
    // Both should enqueue the same task ID.
    w1.wake();
    w2.wake();
    assert_eq!(q.pop(), Some(99));
    assert_eq!(q.pop(), Some(99));
}

#[test]
fn waker_drop_does_not_enqueue() {
    let q = Arc::new(ReadyQueue::new());
    {
        let _w = make_waker(1, q.clone());
    }
    assert_eq!(q.len(), 0);
}

#[test]
fn waker_multiple_clones_share_same_id() {
    let q = Arc::new(ReadyQueue::new());
    let w = make_waker(55, q.clone());
    let clones: Vec<_> = (0..5).map(|_| w.clone()).collect();
    // Drop original.
    drop(w);
    // All clones should still work and enqueue 55.
    for c in clones {
        c.wake();
    }
    for _ in 0..5 {
        assert_eq!(q.pop(), Some(55));
    }
}

#[test]
fn waker_drop_after_clone_does_not_double_free() {
    // This test ensures reference counting is correct —
    // no segfault or double free when dropping clones.
    let q = Arc::new(ReadyQueue::new());
    let w = make_waker(1, q.clone());
    {
        let c1 = w.clone();
        let c2 = c1.clone();
        c1.wake();
        drop(c2);
    }
    // Original still valid.
    w.wake_by_ref();
    assert_eq!(q.pop(), Some(1));
    assert_eq!(q.pop(), Some(1));
}

#[test]
fn waker_correct_task_id() {
    let q = Arc::new(ReadyQueue::new());
    for id in [0, 1, 100, usize::MAX] {
        let w = make_waker(id, q.clone());
        w.wake();
        assert_eq!(q.pop(), Some(id));
    }
}
