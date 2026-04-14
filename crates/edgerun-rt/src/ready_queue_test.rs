//! Unit tests for ready_queue.rs — tests the Condvar-based task queue
//! directly without the runtime.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;
use crate::ready_queue::ReadyQueue;

#[test]
fn push_and_pop_single() {
    let q = ReadyQueue::new();
    q.push(1);
    assert_eq!(q.pop(), Some(1));
}

#[test]
fn push_and_pop_fifo_order() {
    let q = ReadyQueue::new();
    q.push(1);
    q.push(2);
    q.push(3);
    assert_eq!(q.pop(), Some(1));
    assert_eq!(q.pop(), Some(2));
    assert_eq!(q.pop(), Some(3));
}

#[test]
fn len_reflects_queued_tasks() {
    let q = ReadyQueue::new();
    assert_eq!(q.len(), 0);
    q.push(1);
    q.push(2);
    assert_eq!(q.len(), 2);
    q.pop();
    assert_eq!(q.len(), 1);
    q.pop();
    assert_eq!(q.len(), 0);
}

#[test]
fn pop_returns_none_after_shutdown() {
    let q = ReadyQueue::new();
    q.shutdown();
    assert_eq!(q.pop(), None);
}

#[test]
fn pop_blocks_until_push() {
    let q = Arc::new(ReadyQueue::new());
    let q2 = q.clone();

    let handle = thread::spawn(move || {
        thread::sleep(Duration::from_millis(50));
        q2.push(42);
    });

    let result = q.pop();
    assert_eq!(result, Some(42));
    handle.join().unwrap();
}

#[test]
fn pop_unblocks_on_shutdown() {
    let q = Arc::new(ReadyQueue::new());
    let q2 = q.clone();

    let handle = thread::spawn(move || {
        thread::sleep(Duration::from_millis(50));
        q2.shutdown();
    });

    let result = q.pop();
    assert_eq!(result, None);
    handle.join().unwrap();
}

#[test]
fn concurrent_pushers() {
    let q = Arc::new(ReadyQueue::new());
    let mut handles = vec![];

    for i in 0..10 {
        let q2 = q.clone();
        handles.push(thread::spawn(move || {
            q2.push(i);
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    assert_eq!(q.len(), 10);
    let mut results = vec![];
    for _ in 0..10 {
        results.push(q.pop().unwrap());
    }
    // All 10 values should be present (order may vary due to scheduling).
    results.sort();
    assert_eq!(results, (0..10).collect::<Vec<_>>());
}

#[test]
fn push_after_shutdown_still_enqueues() {
    let q = ReadyQueue::new();
    q.shutdown();
    // Push after shutdown still enqueues items.
    q.push(99);
    assert_eq!(q.len(), 1);
    // pop returns the item first (queue drains existing items),
    // then subsequent pop returns None because done flag is set.
    assert_eq!(q.pop(), Some(99));
    // Now the queue is empty and done=true, so pop returns None.
    assert_eq!(q.pop(), None);
}

#[test]
fn multiple_consumers_drain_queue() {
    let q = Arc::new(ReadyQueue::new());
    const N: usize = 100;

    // Push 100 items.
    for i in 0..N {
        q.push(i);
    }

    // 4 consumers race to pop.
    let mut handles = vec![];
    let received = Arc::new(AtomicUsize::new(0));
    for _ in 0..4 {
        let q2 = q.clone();
        let rcvd = received.clone();
        handles.push(thread::spawn(move || {
            let mut local_count = 0;
            while let Some(_) = q2.pop() {
                local_count += 1;
                rcvd.fetch_add(1, Ordering::Relaxed);
            }
            local_count
        }));
    }

    // Shutdown to unblock consumers.
    q.shutdown();

    let counts: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    let total: usize = counts.iter().sum();
    assert_eq!(total, N, "all items should have been consumed");
}

#[test]
fn shutdown_is_idempotent() {
    let q = ReadyQueue::new();
    q.shutdown();
    q.shutdown();
    q.shutdown();
    assert_eq!(q.pop(), None);
}
