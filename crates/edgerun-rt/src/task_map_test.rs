//! Unit tests for task_map.rs — tests the task ID to poll function map
//! directly without the runtime.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll, Waker};
use crate::task_map::TaskMap;

static NOOP_WAKER: std::sync::LazyLock<Waker> =
    std::sync::LazyLock::new(|| {
        static VTABLE: std::task::RawWakerVTable =
            std::task::RawWakerVTable::new(clone_noop, wake_noop, wake_noop, drop_noop);
        const fn clone_noop(_: *const ()) -> std::task::RawWaker {
            std::task::RawWaker::new(std::ptr::null(), &VTABLE)
        }
        const fn wake_noop(_: *const ()) {}
        const fn drop_noop(_: *const ()) {}
        unsafe {
            Waker::from_raw(std::task::RawWaker::new(
                std::ptr::null(),
                &VTABLE,
            ))
        }
    });

fn cx() -> Context<'static> {
    Context::from_waker(&NOOP_WAKER)
}

#[test]
fn insert_and_take_for_poll() {
    let map = TaskMap::new();
    let id = map.insert(Box::new(|_cx| true));
    assert_eq!(map.len(), 1);
    let mut poll_fn = map.take_for_poll(id).expect("task should exist");
    assert_eq!(map.len(), 0);
    // Verify the poll fn works.
    assert!(poll_fn(&mut cx()));
}

#[test]
fn take_for_poll_missing_id() {
    let map = TaskMap::new();
    assert!(map.take_for_poll(999).is_none());
}

#[test]
fn reinsert_restores_task() {
    let map = TaskMap::new();
    let id = map.insert(Box::new(|_cx| false));
    let mut poll_fn = map.take_for_poll(id).expect("should exist");
    assert_eq!(map.len(), 0);
    assert!(!poll_fn(&mut cx()));
    map.reinsert(id, poll_fn);
    assert_eq!(map.len(), 1);
    assert!(map.take_for_poll(id).is_some());
}

#[test]
fn next_id_is_monotonic() {
    let map = TaskMap::new();
    let id1 = map.next_id();
    let id2 = map.next_id();
    let id3 = map.next_id();
    assert!(id1 < id2);
    assert!(id2 < id3);
}

#[test]
fn insert_with_id_uses_provided_id() {
    let map = TaskMap::new();
    let reserved = map.next_id();
    map.insert_with_id(reserved, Box::new(|_cx| true));
    assert!(map.take_for_poll(reserved).is_some());
}

#[test]
fn multiple_tasks_independent() {
    let map = TaskMap::new();
    let id1 = map.insert(Box::new(|_cx| true));
    let id2 = map.insert(Box::new(|_cx| false));
    let id3 = map.insert(Box::new(|_cx| true));
    assert_eq!(map.len(), 3);

    // Take them all.
    let mut f1 = map.take_for_poll(id1).unwrap();
    let mut f2 = map.take_for_poll(id2).unwrap();
    let mut f3 = map.take_for_poll(id3).unwrap();
    assert_eq!(map.len(), 0);

    assert!(f1(&mut cx()));
    assert!(!f2(&mut cx()));
    assert!(f3(&mut cx()));
}

#[test]
fn poll_fn_can_mutate_state() {
    // Verify the poll fn can hold mutable state via Arc/AtomicUsize.
    let counter = Arc::new(AtomicUsize::new(0));
    let c = counter.clone();
    let map = TaskMap::new();
    let id = map.insert(Box::new(move |_cx| {
        c.fetch_add(1, Ordering::SeqCst);
        c.load(Ordering::SeqCst) < 3
    }));

    let mut f = map.take_for_poll(id).unwrap();
    assert!(f(&mut cx())); // 1 < 3
    map.reinsert(id, f);

    let mut f = map.take_for_poll(id).unwrap();
    assert!(f(&mut cx())); // 2 < 3
    map.reinsert(id, f);

    let mut f = map.take_for_poll(id).unwrap();
    assert!(!f(&mut cx())); // 3 < 3 is false
    // Don't reinsert — task is done.
    assert_eq!(map.len(), 0);
    assert_eq!(counter.load(Ordering::SeqCst), 3);
}

#[test]
fn len_reflects_insertions_and_removals() {
    let map = TaskMap::new();
    assert_eq!(map.len(), 0);
    let id1 = map.insert(Box::new(|_cx| true));
    assert_eq!(map.len(), 1);
    let id2 = map.insert(Box::new(|_cx| true));
    assert_eq!(map.len(), 2);
    map.take_for_poll(id1);
    assert_eq!(map.len(), 1);
    map.take_for_poll(id2);
    assert_eq!(map.len(), 0);
}
