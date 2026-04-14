// Stress and race condition tests for channels and sync primitives.
// These tests verify correctness under concurrent access with realistic
// contention patterns.
use edgerun_rt::{Runtime, mpsc, unbounded, Notify, Semaphore, Mutex, Barrier, CancellationToken, WatchSender};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

// ===========================================================================
// mpsc stress tests
// ===========================================================================

#[test]
fn mpsc_stress_many_producers() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async {
        const PRODUCERS: usize = 5;
        const ITEMS_PER_PRODUCER: usize = 200;
        const TOTAL: usize = PRODUCERS * ITEMS_PER_PRODUCER;

        let (tx, mut rx) = mpsc::channel(32);
        let tx = Arc::new(tx);

        let mut handles = vec![];
        for p in 0..PRODUCERS {
            let tx = tx.clone();
            handles.push(edgerun_rt::spawn(async move {
                for i in 0..ITEMS_PER_PRODUCER {
                    tx.send((p, i)).await.expect("send should succeed");
                }
            }));
        }

        let mut received = 0;
        while received < TOTAL {
            rx.recv().await.expect("should receive");
            received += 1;
        }

        assert_eq!(received, TOTAL);
        for h in handles {
            h.await.expect("producer should complete");
        }
    });
}

#[test]
fn mpsc_stress_backpressure() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async {
        let (tx, mut rx) = mpsc::channel(1);
        let sent = Arc::new(AtomicUsize::new(0));

        let s = sent.clone();
        let producer = edgerun_rt::spawn(async move {
            for i in 0..50 {
                tx.send(i).await.expect("send should succeed");
                s.fetch_add(1, Ordering::SeqCst);
            }
        });

        let mut total = 0;
        for _ in 0..50 {
            let item = rx.recv().await.expect("should receive");
            total += item;
        }
        assert_eq!(total, (0..50).sum::<usize>());

        producer.await.expect("producer should complete");
        assert_eq!(sent.load(Ordering::SeqCst), 50);
    });
}

// ===========================================================================
// unbounded stress tests
// ===========================================================================

#[test]
fn unbounded_stress_concurrent_send_recv() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async {
        const SENDERS: usize = 5;
        const ITEMS: usize = 500;

        let (tx, mut rx) = unbounded::channel::<usize>();
        let tx = Arc::new(tx);

        let mut handles = vec![];
        for _ in 0..SENDERS {
            let tx = tx.clone();
            handles.push(edgerun_rt::spawn(async move {
                for i in 0..ITEMS {
                    tx.send(i).expect("unbounded send should never fail");
                }
            }));
        }

        let mut received = 0;
        let expected = SENDERS * ITEMS;
        while received < expected {
            rx.recv().await.expect("should receive");
            received += 1;
        }

        assert_eq!(received, expected);
        for h in handles {
            h.await.expect("sender should complete");
        }
    });
}

// ===========================================================================
// watch stress tests
// ===========================================================================

#[test]
fn watch_stress_rapid_updates() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async {
        let (mut tx, mut rx) = WatchSender::new(0usize);

        let sender = edgerun_rt::spawn(async move {
            for i in 1..=200 {
                tx.send_replace(i);
                if i % 20 == 0 {
                    edgerun_rt::yieldnow().await;
                }
            }
        });

        let mut last = 0;
        for _ in 0..50 {
            rx.changed().await.expect("changed should succeed");
            let val = rx.borrow().expect("should borrow");
            assert!(val >= last, "values should be non-decreasing");
            last = val;
        }

        sender.await.expect("sender should complete");
    });
}

// ===========================================================================
// Notify stress tests
// ===========================================================================

#[test]
fn notify_stress_concurrent_notify() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async {
        const WAITERS: usize = 20;
        let notify = Arc::new(Notify::new());
        let count = Arc::new(AtomicUsize::new(0));

        let mut handles = vec![];
        for _ in 0..WAITERS {
            let n = notify.clone();
            let c = count.clone();
            handles.push(edgerun_rt::spawn(async move {
                n.notified().await;
                c.fetch_add(1, Ordering::SeqCst);
            }));
        }

        edgerun_rt::sleep(Duration::from_millis(30)).await;
        notify.notify_waiters();

        edgerun_rt::sleep(Duration::from_millis(50)).await;
        assert_eq!(count.load(Ordering::SeqCst), WAITERS);
        for h in handles {
            h.await.expect("waiter should complete");
        }
    });
}

// ===========================================================================
// Semaphore stress tests
// ===========================================================================

#[test]
fn semaphore_stress_concurrent_acquire() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async {
        const PERMITS: usize = 5;
        const WORKERS: usize = 20;
        let sem = Arc::new(Semaphore::new(PERMITS));
        let active = Arc::new(AtomicUsize::new(0));
        let max_active = Arc::new(AtomicUsize::new(0));

        let mut handles = vec![];
        for _ in 0..WORKERS {
            let sem = sem.clone();
            let a = active.clone();
            let m = max_active.clone();
            handles.push(edgerun_rt::spawn(async move {
                let _permit = sem.acquire().await;
                let prev = a.fetch_add(1, Ordering::SeqCst);
                m.fetch_max(prev + 1, Ordering::SeqCst);
                edgerun_rt::sleep(Duration::from_millis(1)).await;
                a.fetch_sub(1, Ordering::SeqCst);
            }));
        }

        for h in handles {
            h.await.expect("worker should complete");
        }

        let peak = max_active.load(Ordering::SeqCst);
        assert!(peak <= PERMITS, "peak active {} exceeded permits {}", peak, PERMITS);
    });
}

// ===========================================================================
// Mutex stress tests
// ===========================================================================

#[test]
fn mutex_stress_concurrent_increment() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async {
        const WORKERS: usize = 10;
        const INCREMENTS: usize = 200;
        let m = Arc::new(Mutex::new(0u64));

        let mut handles = vec![];
        for _ in 0..WORKERS {
            let m = m.clone();
            handles.push(edgerun_rt::spawn(async move {
                for _ in 0..INCREMENTS {
                    let mut g = m.lock().await;
                    *g += 1;
                }
            }));
        }

        for h in handles {
            h.await.expect("worker should complete");
        }

        let g = m.lock().await;
        assert_eq!(*g, (WORKERS * INCREMENTS) as u64);
    });
}

// ===========================================================================
// Barrier stress tests
// ===========================================================================

#[test]
fn barrier_stress_many_waiters() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async {
        const WAITERS: usize = 50;
        let barrier = Arc::new(Barrier::new(WAITERS));
        let reached = Arc::new(AtomicUsize::new(0));

        let mut handles = vec![];
        for _ in 0..WAITERS {
            let b = barrier.clone();
            let r = reached.clone();
            handles.push(edgerun_rt::spawn(async move {
                r.fetch_add(1, Ordering::SeqCst);
                b.wait().await;
            }));
        }

        while reached.load(Ordering::SeqCst) < WAITERS {
            edgerun_rt::yieldnow().await;
        }

        for h in handles {
            h.await.expect("waiter should complete");
        }
    });
}

// ===========================================================================
// CancellationToken stress tests
// ===========================================================================

#[test]
fn cancellation_stress_many_waiters() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async {
        const WAITERS: usize = 100;
        let token = CancellationToken::new();
        let count = Arc::new(AtomicUsize::new(0));

        let mut handles = vec![];
        for _ in 0..WAITERS {
            let t = token.clone();
            let c = count.clone();
            handles.push(edgerun_rt::spawn(async move {
                t.cancelled().await;
                c.fetch_add(1, Ordering::SeqCst);
            }));
        }

        edgerun_rt::sleep(Duration::from_millis(10)).await;
        token.cancel();

        edgerun_rt::sleep(Duration::from_millis(30)).await;
        assert_eq!(count.load(Ordering::SeqCst), WAITERS);
        for h in handles {
            h.await.expect("waiter should complete");
        }
    });
}

// ===========================================================================
// Cross-primitive stress test: channel + cancellation
// ===========================================================================

#[test]
fn stress_pipeline_with_cancellation() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async {
        const ITEMS: usize = 200;
        let (tx, mut rx) = mpsc::channel(16);
        let token = CancellationToken::new();

        // Producer.
        let producer_token = token.clone();
        let tx2 = tx.clone();
        let producer = edgerun_rt::spawn(async move {
            for i in 0..ITEMS {
                if producer_token.is_cancelled() {
                    break;
                }
                tx2.send(i).await.expect("send should succeed");
            }
        });

        // Consumer.
        let consumer = edgerun_rt::spawn(async move {
            let mut sum = 0usize;
            let mut count = 0;
            loop {
                match rx.recv().await {
                    Some(v) => { sum += v; count += 1; }
                    None => break,
                }
            }
            (sum, count)
        });

        edgerun_rt::sleep(Duration::from_millis(100)).await;
        token.cancel();
        producer.await.expect("producer should complete");
        drop(tx);
        let (sum, count) = consumer.await.expect("consumer should complete");

        assert!(count <= ITEMS);
        let expected_sum = (0..count).sum::<usize>();
        assert_eq!(sum, expected_sum);
    });
}
