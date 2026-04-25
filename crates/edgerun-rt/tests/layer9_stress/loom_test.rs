// Loom-based concurrency tests for the runtime.
// These tests verify thread-safety under concurrent access patterns
// that are normally hard to reproduce deterministically.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

// ===========================================================================
// Basic Runtime Tests
// ===========================================================================

#[test]
fn loom_smoke_test() {
    loom::model(|| {
        let rt = edgerun_rt::Runtime::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async {
            let handle = edgerun_rt::spawn(async { 1 + 1 });
            assert_eq!(handle.await.unwrap(), 2);
        });
    });
}

#[test]
fn loom_spawn_many() {
    for _ in 0..2 {
        loom::model(|| {
            let rt = edgerun_rt::Runtime::new_multi_thread()
                .enable_all()
                .build()
                .unwrap();

            rt.block_on(async {
                let mut handles = vec![];
                for i in 0..5 {
                    handles.push(edgerun_rt::spawn(async move { i }));
                }
                let mut sum = 0;
                for h in handles {
                    sum += h.await.unwrap();
                }
                assert_eq!(sum, 0 + 1 + 2 + 3 + 4);
            });
        });
    }
}

#[test]
fn loom_join_basic() {
    for _ in 0..2 {
        loom::model(|| {
            let rt = edgerun_rt::Runtime::new_multi_thread()
                .enable_all()
                .build()
                .unwrap();

            rt.block_on(async {
                let h1 = edgerun_rt::spawn(async { 1 });
                let h2 = edgerun_rt::spawn(async { 2 });
                let _ = edgerun_rt::join!(h1, h2);
            });
        });
    }
}

// ===========================================================================
// UDP Tests
// ===========================================================================

#[test]
fn loom_udp_basic() {
    for _ in 0..2 {
        loom::model(|| {
            let rt = edgerun_rt::Runtime::new_multi_thread()
                .enable_all()
                .build()
                .unwrap();

            rt.block_on(async {
                use edgerun_rt::AsyncUdpSocket;

                let socket = AsyncUdpSocket::bind("127.0.0.1:0").unwrap();
                let addr = socket.local_addr().unwrap();

                let sender = edgerun_rt::spawn(async move {
                    let socket = AsyncUdpSocket::bind("127.0.0.1:0").unwrap();
                    socket.send_to(b"test", addr).await.unwrap();
                });

                let receiver = edgerun_rt::spawn(async move {
                    let mut buf = [0u8; 64];
                    let (n, _src) = socket.recv_from(&mut buf).await.unwrap();
                    assert_eq!(&buf[..n], b"test");
                });

                let _ = edgerun_rt::join!(sender, receiver);
            });
        });
    }
}

// ===========================================================================
// Channel Tests
// ===========================================================================

#[test]
fn loom_oneshot_basic() {
    for _ in 0..3 {
        loom::model(|| {
            let rt = edgerun_rt::Runtime::new_multi_thread()
                .enable_all()
                .build()
                .unwrap();

            rt.block_on(async {
                use edgerun_rt::oneshot;

                let (tx, rx) = oneshot::channel::<i32>();

                let sender = edgerun_rt::spawn(async move {
                    tx.send(42).unwrap();
                });

                let receiver = edgerun_rt::spawn(async move { rx.await.unwrap() });

                let _ = edgerun_rt::join!(sender, receiver);
            });
        });
    }
}

#[test]
fn loom_mpsc_basic() {
    for _ in 0..2 {
        loom::model(|| {
            let rt = edgerun_rt::Runtime::new_multi_thread()
                .enable_all()
                .build()
                .unwrap();

            rt.block_on(async {
                let (tx, mut rx) = edgerun_rt::mpsc::channel::<usize>(10);

                let sender = edgerun_rt::spawn(async move {
                    tx.send(42).await.unwrap();
                });

                let received = rx.recv().await.unwrap();
                assert_eq!(received, 42);

                sender.await.ok();
            });
        });
    }
}

#[test]
fn loom_mpsc_multiple_producers() {
    for _ in 0..2 {
        loom::model(|| {
            let rt = edgerun_rt::Runtime::new_multi_thread()
                .enable_all()
                .build()
                .unwrap();

            rt.block_on(async {
                let (tx, mut rx) = edgerun_rt::mpsc::channel::<usize>(10);

                let mut handles = vec![];
                for i in 0..3 {
                    let tx = tx.clone();
                    handles.push(edgerun_rt::spawn(async move {
                        tx.send(i).await.unwrap();
                    }));
                }
                drop(tx);

                let mut sum = 0;
                while let Some(v) = rx.recv().await {
                    sum += v;
                }

                for h in handles {
                    h.await.ok();
                }

                assert_eq!(sum, 0 + 1 + 2);
            });
        });
    }
}

#[test]
fn loom_unbounded_basic() {
    for _ in 0..2 {
        loom::model(|| {
            let rt = edgerun_rt::Runtime::new_multi_thread()
                .enable_all()
                .build()
                .unwrap();

            rt.block_on(async {
                let (tx, rx) = edgerun_rt::unbounded::channel::<i32>();

                tx.send(42).unwrap();

                let received = rx.recv().await.unwrap();
                assert_eq!(received, 42);
            });
        });
    }
}

#[test]
fn loom_watch_basic() {
    for _ in 0..2 {
        loom::model(|| {
            let rt = edgerun_rt::Runtime::new_multi_thread()
                .enable_all()
                .build()
                .unwrap();

            rt.block_on(async {
                let (mut tx, mut rx) = edgerun_rt::WatchSender::new(0usize);

                tx.send_replace(1);
                rx.changed().await.unwrap();
                let val = rx.borrow().unwrap();
                assert_eq!(val, 1);
            });
        });
    }
}

#[test]
fn loom_broadcast_basic() {
    for _ in 0..2 {
        loom::model(|| {
            let rt = edgerun_rt::Runtime::new_multi_thread()
                .enable_all()
                .build()
                .unwrap();

            rt.block_on(async {
                let (tx, mut rx1) = edgerun_rt::broadcast::channel::<i32>(10);

                tx.send(42).unwrap();

                let received = rx1.recv().await.unwrap();
                assert_eq!(received, 42);
            });
        });
    }
}

// ===========================================================================
// Synchronization Primitives Tests
// ===========================================================================

#[test]
fn loom_mutex_basic() {
    loom::model(|| {
        let rt = edgerun_rt::Runtime::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async {
            let m = Arc::new(edgerun_rt::Mutex::new(0u64));
            let m2 = m.clone();

            let handle = edgerun_rt::spawn(async move {
                let mut g = m2.lock().await;
                *g += 1;
            });

            handle.await.ok();

            let g = m.lock().await;
            assert_eq!(*g, 1);
        });
    });
}

#[test]
fn loom_mutex_concurrent_increment() {
    for _ in 0..2 {
        loom::model(|| {
            let rt = edgerun_rt::Runtime::new_multi_thread()
                .enable_all()
                .build()
                .unwrap();

            rt.block_on(async {
                let m = Arc::new(edgerun_rt::Mutex::new(0u64));

                let mut handles = vec![];
                for _ in 0..3 {
                    let m = m.clone();
                    handles.push(edgerun_rt::spawn(async move {
                        let mut g = m.lock().await;
                        *g += 1;
                    }));
                }

                for h in handles {
                    h.await.ok();
                }

                let g = m.lock().await;
                assert_eq!(*g, 3);
            });
        });
    }
}

#[test]
fn loom_rwlock_basic() {
    for _ in 0..2 {
        loom::model(|| {
            let rt = edgerun_rt::Runtime::new_multi_thread()
                .enable_all()
                .build()
                .unwrap();

            rt.block_on(async {
                let rw = Arc::new(edgerun_rt::RwLock::new(0usize));
                let rw2 = rw.clone();

                let handle = edgerun_rt::spawn(async move {
                    let mut g = rw2.write().await;
                    *g += 1;
                });

                handle.await.ok();

                let g = rw.read().await;
                assert_eq!(*g, 1);
            });
        });
    }
}

#[test]
fn loom_notify_basic() {
    for _ in 0..2 {
        loom::model(|| {
            let rt = edgerun_rt::Runtime::new_multi_thread()
                .enable_all()
                .build()
                .unwrap();

            rt.block_on(async {
                let notify = Arc::new(edgerun_rt::Notify::new());
                let notify2 = notify.clone();

                let waiter = edgerun_rt::spawn(async move {
                    notify2.notified().await;
                });

                // Yield to let waiter register
                edgerun_rt::yieldnow().await;
                notify.notify_waiters();

                waiter.await.ok();
            });
        });
    }
}

#[test]
fn loom_notify_multiple() {
    for _ in 0..2 {
        loom::model(|| {
            let rt = edgerun_rt::Runtime::new_multi_thread()
                .enable_all()
                .build()
                .unwrap();

            rt.block_on(async {
                let notify = Arc::new(edgerun_rt::Notify::new());
                let count = Arc::new(AtomicUsize::new(0));

                let mut handles = vec![];
                for _ in 0..3 {
                    let n = notify.clone();
                    let c = count.clone();
                    handles.push(edgerun_rt::spawn(async move {
                        n.notified().await;
                        c.fetch_add(1, Ordering::SeqCst);
                    }));
                }

                // Yield to let all waiters register
                edgerun_rt::yieldnow().await;
                notify.notify_waiters();

                for h in handles {
                    h.await.ok();
                }

                assert_eq!(count.load(Ordering::SeqCst), 3);
            });
        });
    }
}

#[test]
fn loom_cancellation_basic() {
    for _ in 0..2 {
        loom::model(|| {
            let rt = edgerun_rt::Runtime::new_multi_thread()
                .enable_all()
                .build()
                .unwrap();

            rt.block_on(async {
                let token = edgerun_rt::CancellationToken::new();
                let token2 = token.clone();

                let waiter = edgerun_rt::spawn(async move {
                    token2.cancelled().await;
                });

                token.cancel();

                waiter.await.ok();
            });
        });
    }
}

// ===========================================================================
// Timer Tests
// ===========================================================================

#[test]
fn loom_sleep_basic() {
    for _ in 0..2 {
        loom::model(|| {
            let rt = edgerun_rt::Runtime::new_multi_thread()
                .enable_all()
                .build()
                .unwrap();

            rt.block_on(async {
                edgerun_rt::sleep(Duration::from_millis(1)).await;
            });
        });
    }
}

#[test]
fn loom_timeout_success() {
    for _ in 0..2 {
        loom::model(|| {
            let rt = edgerun_rt::Runtime::new_multi_thread()
                .enable_all()
                .build()
                .unwrap();

            rt.block_on(async {
                let result = edgerun_rt::timeout(Duration::from_secs(1), async { 42 }).await;

                assert!(result.is_ok());
                assert_eq!(result.unwrap(), 42);
            });
        });
    }
}

#[test]
fn loom_timeout_fail() {
    for _ in 0..2 {
        loom::model(|| {
            let rt = edgerun_rt::Runtime::new_multi_thread()
                .enable_all()
                .build()
                .unwrap();

            rt.block_on(async {
                let result = edgerun_rt::timeout(Duration::from_millis(1), async {
                    edgerun_rt::sleep(Duration::from_secs(10)).await
                })
                .await;

                assert!(result.is_err());
            });
        });
    }
}
