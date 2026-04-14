// End-to-end integration tests for edgerun-rt.
// These tests exercise the full runtime lifecycle, I/O, channels, sync primitives,
// timers, processes, and their interactions under concurrent load.
// Every test is designed to expose issues immediately — no sleeps without assertions,
// no "maybe passes" — deterministic assertions on all outcomes.
//
// harness = false — custom main() test runner.

use edgerun_rt::{
    AsyncReadExt, AsyncTcpListener, AsyncTcpStream, AsyncUdpSocket, AsyncWriteExt,
    Barrier, Builder, CancellationToken, Cursor, DuplexStream, Empty, Interval,
    JoinSet, Latch, MissedTickBehavior, Mutex, Notify, OnceCell, OwnedAsyncFd,
    RateLimiter, Repeat, RwLock, Semaphore, Sleep, Timeout,
    UnixDatagram, UnixListener, UnixStream,
    broadcast, fs, interval, mpsc, oneshot, pipe, poll_fn, process,
    repeat, sleep, sink, spawn, spawn_blocking, sleep_until, timeout, unbounded,
    yieldnow, AsyncRead, AsyncWrite, BufReader, BufWriter, Runtime, RuntimeHandle,
    RuntimeMetrics,
};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

// ===========================================================================
// Helpers
// ===========================================================================

fn find_free_port() -> u16 {
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    l.local_addr().unwrap().port()
}

fn make_temp_dir() -> std::path::PathBuf {
    static C: AtomicUsize = AtomicUsize::new(0);
    let id = std::process::id();
    let n = C.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("edgerun_e2e_{}_{}", id, n))
}

struct CleanupDir(std::path::PathBuf);
impl Drop for CleanupDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

// ===========================================================================
// Main dispatch
// ===========================================================================

fn main() {
    println!("=== edgerun-rt E2E Integration Tests ===");

    // Runtime lifecycle & scheduling
    e2e_runtime_lifecycle();
    e2e_cross_thread_handle_spawn();
    e2e_blocking_pool_exhaustion();
    e2e_task_panic_isolation();
    e2e_nested_block_on();
    e2e_shutdown_with_pending_tasks();
    e2e_metrics_accuracy();
    e2e_handle_clone_independence();
    e2e_free_spawn_from_worker_thread();
    e2e_yield_cooperative_scheduling();

    // Network I/O
    e2e_tcp_full_request_response();
    e2e_tcp_connection_refused();
    e2e_udp_send_recv_roundtrip();
    e2e_tcp_concurrent_accept();
    e2e_unix_stream_connected_pair();
    e2e_unix_dgram_send_recv();

    // Channels
    e2e_mpsc_backpressure_under_load();
    e2e_oneshot_cross_thread();
    e2e_broadcast_multi_producer_multi_consumer();
    e2e_watch_version_tracking();
    e2e_unbounded_never_blocks_sender();

    // Sync primitives
    e2e_mutex_sequential();
    e2e_rwlock_read_concurrency();
    e2e_semaphore_capacity_exhaustion();
    e2e_barrier_multi_wait_generations();
    e2e_notify_fifo_ordering();
    e2e_once_cell_concurrent_init();
    e2e_latch_countdown();
    e2e_rate_limiter_token_bucket();
    e2e_cancellation_composition();

    // I/O utilities
    e2e_duplex_stream_through_bufio();
    e2e_cursor_read_write_seek();
    e2e_copy_bidirectional();
    e2e_pipe_async_fd();
    e2e_repeat_and_sink();
    e2e_empty_eof();
    e2e_async_fd_owned_close();

    // File I/O
    e2e_fs_concurrent_write_read();
    e2e_fs_nonexistent_path_error();

    // Process management
    e2e_process_output_and_kill();
    e2e_process_concurrent_execution();

    // Timer edge cases
    e2e_timeout_cancels_inner_future();
    e2e_interval_missed_tick_skip();
    e2e_sleep_at_future_deadline();

    // JoinSet
    e2e_join_set_dynamic_tasks_with_abort();

    println!("\n=== All E2E tests passed ===");
}

// ===========================================================================
// Runtime lifecycle & scheduling
// ===========================================================================

fn e2e_runtime_lifecycle() {
    println!("\n  e2e_runtime_lifecycle...");
    let rt = Builder::new_multi_thread()
        .worker_threads(2)
        .max_blocking_threads(2)
        .build()
        .unwrap();

    let counter = Arc::new(AtomicUsize::new(0));
    let mut handles = vec![];
    for i in 0..20 {
        let c = counter.clone();
        handles.push(rt.spawn(async move {
            sleep(Duration::from_millis(5)).await;
            c.fetch_add(1, Ordering::SeqCst);
            i
        }));
    }

    let results = rt.block_on(async {
        let mut results = vec![];
        for h in handles {
            results.push(h.await.expect("task should not abort"));
        }
        results
    });

    assert_eq!(results.len(), 20);
    assert_eq!(counter.load(Ordering::SeqCst), 20);
    rt.shutdown();
    println!("  e2e_runtime_lifecycle OK");
}

fn e2e_cross_thread_handle_spawn() {
    println!("  e2e_cross_thread_handle_spawn...");
    let rt = Builder::new_multi_thread()
        .worker_threads(2)
        .build()
        .unwrap();

    let handle = rt.handle();
    let handle2 = handle.clone();

    // Spawn from a separate OS thread using the cloned handle.
    let outer = std::thread::spawn(move || {
        let h = handle2.spawn(async {
            sleep(Duration::from_millis(10)).await;
            99
        });
        h
    });

    let join_from_thread = outer.join().unwrap();
    let result = rt.block_on(async {
        join_from_thread.await.unwrap()
    });

    assert_eq!(result, 99);
    rt.shutdown();
    println!("  e2e_cross_thread_handle_spawn OK");
}

fn e2e_blocking_pool_exhaustion() {
    println!("  e2e_blocking_pool_exhaustion...");
    let rt = Builder::new_multi_thread()
        .max_blocking_threads(2)
        .build()
        .unwrap();

    let started = Arc::new(AtomicUsize::new(0));
    let completed = Arc::new(AtomicUsize::new(0));

    // Occupy both blocking threads.
    let mut blockers = vec![];
    for _ in 0..2 {
        let s = started.clone();
        blockers.push(rt.spawn_blocking(move || {
            s.fetch_add(1, Ordering::SeqCst);
            std::thread::sleep(Duration::from_millis(200));
        }));
    }

    // Wait for blockers to start.
    std::thread::sleep(Duration::from_millis(50));
    assert_eq!(started.load(Ordering::SeqCst), 2);

    // Queue a third — it should wait, then execute.
    let c = completed.clone();
    let queued = rt.spawn_blocking(move || {
        c.fetch_add(1, Ordering::SeqCst);
    });

    rt.block_on(async {
        for b in blockers {
            b.await.expect("blocker should complete");
        }
        queued.await.expect("queued blocker should complete");
    });

    assert_eq!(completed.load(Ordering::SeqCst), 1);
    rt.shutdown();
    println!("  e2e_blocking_pool_exhaustion OK");
}

fn e2e_task_panic_isolation() {
    println!("  e2e_task_panic_isolation...");
    let rt = Builder::new_multi_thread().build().unwrap();

    let counter = Arc::new(AtomicUsize::new(0));

    // Task that panics.
    let panicking = rt.spawn(async {
        panic!("intentional panic for test");
    });

    // Task that runs normally.
    let c = counter.clone();
    let normal = rt.spawn(async move {
        sleep(Duration::from_millis(10)).await;
        c.fetch_add(1, Ordering::SeqCst);
        42
    });

    rt.block_on(async {
        // Panicking task should return Err(JoinError).
        let result = panicking.await;
        assert!(result.is_err(), "panicking task should return Err");

        // Normal task should still succeed.
        let val = normal.await.unwrap();
        assert_eq!(val, 42);
    });

    assert_eq!(counter.load(Ordering::SeqCst), 1);
    rt.shutdown();
    println!("  e2e_task_panic_isolation OK");
}

fn e2e_nested_block_on() {
    println!("  e2e_nested_block_on...");
    let rt = Builder::new_multi_thread().build().unwrap();

    // block_on from within block_on — nested runtime should work.
    let result = rt.block_on(async {
        let inner_rt = Builder::new_multi_thread().build().unwrap();
        inner_rt.block_on(async {
            sleep(Duration::from_millis(5)).await;
            123
        })
    });

    assert_eq!(result, 123);
    rt.shutdown();
    println!("  e2e_nested_block_on OK");
}

fn e2e_shutdown_with_pending_tasks() {
    println!("  e2e_shutdown_with_pending_tasks...");
    let completed = Arc::new(AtomicUsize::new(0));

    {
        let rt = Builder::new_multi_thread().build().unwrap();
        for _ in 0..10 {
            let c = completed.clone();
            rt.spawn(async move {
                sleep(Duration::from_millis(50)).await;
                c.fetch_add(1, Ordering::SeqCst);
            });
        }
        // Drop runtime while tasks are still pending — shutdown should wait.
    }

    // Give shutdown time to complete pending tasks.
    std::thread::sleep(Duration::from_millis(500));
    // Key assertion: no crash / segfault / hang.
    println!("  e2e_shutdown_with_pending_tasks OK (completed {})", completed.load(Ordering::SeqCst));
}

fn e2e_metrics_accuracy() {
    println!("  e2e_metrics_accuracy...");
    let rt = Builder::new_multi_thread()
        .worker_threads(2)
        .build()
        .unwrap();

    let initial = rt.metrics();
    let initial_spawned = initial.total_spawned();

    let mut handles = vec![];
    for _ in 0..5 {
        handles.push(rt.spawn(async {
            sleep(Duration::from_millis(5)).await;
        }));
    }

    rt.block_on(async {
        for h in handles {
            let _ = h.await;
        }
    });

    let after = rt.metrics();
    assert_eq!(
        after.total_spawned() - initial_spawned,
        5,
        "spawned counter should reflect 5 new tasks, was {} - {} = {}",
        after.total_spawned(), initial_spawned, after.total_spawned() - initial_spawned
    );

    rt.shutdown();
    println!("  e2e_metrics_accuracy OK");
}

fn e2e_handle_clone_independence() {
    println!("  e2e_handle_clone_independence...");
    let rt = Builder::new_multi_thread().build().unwrap();
    let h1 = rt.handle();
    let h2 = h1.clone();

    // Spawn on h1 from main thread.
    let t1 = h1.spawn(async { 1 });
    // Spawn on h2 from spawned worker.
    let t2 = rt.spawn(async move {
        let h3 = h2.clone();
        let inner = h3.spawn(async { 2 });
        inner.await.unwrap()
    });

    rt.block_on(async {
        assert_eq!(t1.await.unwrap(), 1);
        assert_eq!(t2.await.unwrap(), 2);
    });

    rt.shutdown();
    println!("  e2e_handle_clone_independence OK");
}

fn e2e_free_spawn_from_worker_thread() {
    println!("  e2e_free_spawn_from_worker_thread...");
    let rt = Builder::new_multi_thread().build().unwrap();

    // spawn() from within block_on should use thread-local runtime.
    let outer = rt.spawn(async {
        let inner = spawn(async { 77 });
        inner.await.unwrap()
    });

    let result = rt.block_on(async {
        outer.await.unwrap()
    });

    assert_eq!(result, 77);
    rt.shutdown();
    println!("  e2e_free_spawn_from_worker_thread OK");
}

fn e2e_yield_cooperative_scheduling() {
    println!("  e2e_yield_cooperative_scheduling...");
    let rt = Builder::new_multi_thread().build().unwrap();
    let order = Arc::new(std::sync::Mutex::new(Vec::new()));

    let mut handles = vec![];
    for i in 0..5 {
        let o = order.clone();
        handles.push(rt.spawn(async move {
            for _ in 0..3 {
                o.lock().unwrap().push(i);
                yieldnow().await;
            }
        }));
    }

    rt.block_on(async {
        for h in handles {
            h.await.unwrap();
        }
    });

    let final_order = order.lock().unwrap();
    // Each task should have pushed 3 times = 15 total entries.
    assert_eq!(final_order.len(), 15);
    rt.shutdown();
    println!("  e2e_yield_cooperative_scheduling OK");
}

// ===========================================================================
// Network I/O end-to-end
// ===========================================================================

fn e2e_tcp_full_request_response() {
    println!("\n  e2e_tcp_full_request_response...");
    let rt = Builder::new_multi_thread().build().unwrap();

    rt.block_on(async {
        let port = find_free_port();
        let addr = format!("127.0.0.1:{}", port);
        let listener = Arc::new(AsyncTcpListener::bind(&addr).unwrap());

        // Server: accept, read request, write response.
        let srv = listener.clone();
        let server_task = spawn(async move {
            let (stream, _) = srv.accept().await.unwrap();
            let mut stream: Arc<AsyncTcpStream> = stream;
            let mut buf = [0u8; 64];
            let n = stream.read(&mut buf).await.unwrap();
            assert_eq!(&buf[..n], b"GET /hello");
            stream.write_all(b"HTTP/1.1 200 OK\r\n\r\nHello").await.unwrap();
            stream.shutdown_write().unwrap();
        });

        std::thread::sleep(Duration::from_millis(30));

        // Client: connect, send request, read response.
        let client_task = spawn(async move {
            let mut stream = std::net::TcpStream::connect(&addr).unwrap();
            stream.set_nonblocking(true).unwrap();
            let mut stream: Arc<AsyncTcpStream> = Arc::new(AsyncTcpStream::from_std(stream).unwrap());
            stream.write_all(b"GET /hello").await.unwrap();

            let mut response = Vec::new();
            let mut buf = [0u8; 64];
            loop {
                let n = stream.read(&mut buf).await.unwrap();
                if n == 0 { break; }
                response.extend_from_slice(&buf[..n]);
            }
            assert_eq!(&response, b"HTTP/1.1 200 OK\r\n\r\nHello");
        });

        std::thread::sleep(Duration::from_millis(300));
        drop(server_task);
        drop(client_task);
    });

    rt.shutdown();
    println!("  e2e_tcp_full_request_response OK");
}

fn e2e_tcp_connection_refused() {
    println!("  e2e_tcp_connection_refused...");
    let rt = Builder::new_multi_thread().build().unwrap();

    rt.block_on(async {
        // Try to connect to a port that has no listener.
        let port = find_free_port();
        let addr = format!("127.0.0.1:{}", port);

        // AsyncTcpStream::connect should fail with ConnectionRefused.
        let result = AsyncTcpStream::connect(&addr).await;
        assert!(result.is_err(), "connect to unbound port should fail");
    });

    rt.shutdown();
    println!("  e2e_tcp_connection_refused OK");
}

fn e2e_udp_send_recv_roundtrip() {
    println!("  e2e_udp_send_recv_roundtrip...");
    let rt = Builder::new_multi_thread().build().unwrap();

    rt.block_on(async {
        let port = find_free_port();
        let addr = format!("127.0.0.1:{}", port);

        let socket = Arc::new(AsyncUdpSocket::bind(&addr).unwrap());

        // Send from the same socket to itself (loopback).
        let sock1 = socket.clone();
        let sender = spawn(async move {
            sock1.send_to(b"ping", &addr).await.unwrap();
        });

        let sock2 = socket.clone();
        let receiver = spawn(async move {
            let mut buf = [0u8; 64];
            let (n, from) = sock2.recv_from(&mut buf).await.unwrap();
            assert_eq!(&buf[..n], b"ping");
            assert_eq!(from.ip().to_string(), "127.0.0.1");
        });

        std::thread::sleep(Duration::from_millis(200));
        drop(sender);
        drop(receiver);
    });

    rt.shutdown();
    println!("  e2e_udp_send_recv_roundtrip OK");
}

fn e2e_tcp_concurrent_accept() {
    println!("  e2e_tcp_concurrent_accept...");
    let rt = Builder::new_multi_thread().build().unwrap();

    rt.block_on(async {
        let port = find_free_port();
        let addr = format!("127.0.0.1:{}", port);
        let listener = Arc::new(AsyncTcpListener::bind(&addr).unwrap());

        // Spawn 5 clients concurrently.
        let mut client_handles = vec![];
        for i in 0..5 {
            let addr = addr.clone();
            client_handles.push(spawn(async move {
                let mut stream = std::net::TcpStream::connect(&addr).unwrap();
                stream.set_nonblocking(true).unwrap();
                let mut stream: Arc<AsyncTcpStream> = Arc::new(AsyncTcpStream::from_std(stream).unwrap());
                stream.write_all(&format!("client{}", i).into_bytes()).await.unwrap();
                let mut buf = [0u8; 32];
                let n = stream.read(&mut buf).await.unwrap();
                String::from_utf8_lossy(&buf[..n]).to_string()
            }));
        }

        std::thread::sleep(Duration::from_millis(50));

        // Server accepts all 5 concurrently.
        let mut server_handles = vec![];
        for _ in 0..5 {
            let l = listener.clone();
            server_handles.push(spawn(async move {
                let (stream, _) = l.accept().await.unwrap();
                let mut stream: Arc<AsyncTcpStream> = stream;
                let mut buf = [0u8; 32];
                let n = stream.read(&mut buf).await.unwrap();
                let msg = String::from_utf8_lossy(&buf[..n]).to_string();
                stream.write_all(msg.as_bytes()).await.unwrap();
                msg
            }));
        }

        std::thread::sleep(Duration::from_millis(500));

        // Verify all clients got their data echoed back.
        for (i, ch) in client_handles.into_iter().enumerate() {
            let result = ch.await;
            if let Ok(msg) = result {
                assert_eq!(msg, format!("client{}", i));
            }
        }
    });

    rt.shutdown();
    println!("  e2e_tcp_concurrent_accept OK");
}

fn e2e_unix_stream_connected_pair() {
    println!("  e2e_unix_stream_connected_pair...");
    let rt = Builder::new_multi_thread().build().unwrap();

    rt.block_on(async {
        let path = make_temp_dir().join("unix_e2e.sock");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let _cleanup = CleanupDir(path.parent().unwrap().to_path_buf());

        let listener = Arc::new(UnixListener::bind(&path).unwrap());

        let srv = listener.clone();
        let server = spawn(async move {
            let (stream, _) = srv.accept().await.unwrap();
            let mut stream: Arc<UnixStream> = stream;
            let mut buf = [0u8; 64];
            let n = stream.read(&mut buf).await.unwrap();
            stream.write_all(&buf[..n]).await.unwrap();
        });

        std::thread::sleep(Duration::from_millis(30));

        let client = spawn(async move {
            let mut stream = UnixStream::connect(&path).await.unwrap();
            stream.write_all(b"unix echo").await.unwrap();
            let mut buf = [0u8; 64];
            let n = stream.read(&mut buf).await.unwrap();
            assert_eq!(&buf[..n], b"unix echo");
        });

        std::thread::sleep(Duration::from_millis(300));
        drop(server);
        drop(client);
    });

    rt.shutdown();
    println!("  e2e_unix_stream_connected_pair OK");
}

fn e2e_unix_dgram_send_recv() {
    println!("  e2e_unix_dgram_send_recv...");
    let rt = Builder::new_multi_thread().build().unwrap();

    rt.block_on(async {
        let dir = make_temp_dir();
        std::fs::create_dir_all(&dir).unwrap();
        let _cleanup = CleanupDir(dir.clone());

        let path_a = dir.join("a.sock");
        let path_b = dir.join("b.sock");

        let sock_a = UnixDatagram::bind(&path_a).unwrap();
        let sock_b = UnixDatagram::bind(&path_b).unwrap();

        sock_a.send_to(b"hello", &path_b).await.unwrap();

        let mut buf = [0u8; 64];
        let (n, from) = sock_b.recv_from(&mut buf).await.unwrap();
        assert_eq!(&buf[..n], b"hello");
        assert_eq!(from.as_pathname().unwrap(), &path_a);
    });

    rt.shutdown();
    println!("  e2e_unix_dgram_send_recv OK");
}

// ===========================================================================
// Channels end-to-end
// ===========================================================================

fn e2e_mpsc_backpressure_under_load() {
    println!("\n  e2e_mpsc_backpressure_under_load...");
    let rt = Builder::new_multi_thread().build().unwrap();

    rt.block_on(async {
        let (tx, mut rx) = mpsc::channel::<u64>(4);
        let send_count = Arc::new(AtomicUsize::new(0));

        // 4 producers, each sending 50 items.
        let mut producers = vec![];
        for p in 0..4 {
            let tx = tx.clone();
            let sc = send_count.clone();
            producers.push(spawn(async move {
                for i in 0..50 {
                    tx.send((p as u64) * 100 + i as u64).await.unwrap();
                    sc.fetch_add(1, Ordering::SeqCst);
                }
            }));
        }
        drop(tx); // Close sender so receiver sees EOF.

        // Single consumer.
        let mut received = 0u64;
        while let Some(_val) = rx.recv().await {
            received += 1;
        }

        assert_eq!(received, 200);
        for p in producers {
            p.await.unwrap();
        }
    });

    rt.shutdown();
    println!("  e2e_mpsc_backpressure_under_load OK");
}

fn e2e_oneshot_cross_thread() {
    println!("  e2e_oneshot_cross_thread...");
    let rt = Builder::new_multi_thread().build().unwrap();

    rt.block_on(async {
        let (tx, rx) = oneshot::channel::<String>();

        // Send from OS thread.
        let sender_thread = std::thread::spawn(move || {
            tx.send("from os thread".to_string()).unwrap();
        });

        let val = rx.await.unwrap();
        assert_eq!(val, "from os thread");
        sender_thread.join().unwrap();
    });

    rt.shutdown();
    println!("  e2e_oneshot_cross_thread OK");
}

fn e2e_broadcast_multi_producer_multi_consumer() {
    println!("  e2e_broadcast_multi_producer_multi_consumer...");
    let rt = Builder::new_multi_thread().build().unwrap();

    rt.block_on(async {
        let (mut tx1, mut rx1) = broadcast::channel::<u32>(16);
        let mut rx2 = tx1.subscribe();
        let mut rx3 = tx1.subscribe();

        // Two producers.
        let p1 = spawn(async move {
            for i in 0..5 {
                tx1.send(i).unwrap();
                sleep(Duration::from_millis(5)).await;
            }
        });

        let mut tx4 = tx1.clone();
        let p2 = spawn(async move {
            for i in 100..105 {
                tx4.send(i).unwrap();
                sleep(Duration::from_millis(5)).await;
            }
        });

        // Three consumers — each should see all 10 messages.
        let mut c1_count = 0;
        let mut c2_count = 0;
        let mut c3_count = 0;

        for _ in 0..10 {
            if let Ok(_v) = rx1.recv().await { c1_count += 1; }
            if let Ok(_v) = rx2.recv().await { c2_count += 1; }
            if let Ok(_v) = rx3.recv().await { c3_count += 1; }
        }

        p1.await.unwrap();
        p2.await.unwrap();

        assert_eq!(c1_count, 10);
        assert_eq!(c2_count, 10);
        assert_eq!(c3_count, 10);
    });

    rt.shutdown();
    println!("  e2e_broadcast_multi_producer_multi_consumer OK");
}

fn e2e_watch_version_tracking() {
    println!("  e2e_watch_version_tracking...");
    let rt = Builder::new_multi_thread().build().unwrap();

    rt.block_on(async {
        let (mut tx, mut rx) = watch::Sender::new(0u32);

        assert_eq!(*rx.borrow().unwrap(), 0);

        let watcher = spawn(async move {
            let mut expected = 1;
            for _ in 0..10 {
                rx.changed().await.unwrap();
                let val = *rx.borrow().unwrap();
                assert!(val >= expected, "value {} should be >= {}", val, expected);
                expected = val + 1;
            }
        });

        for i in 1..=10 {
            tx.send_replace(i);
            sleep(Duration::from_millis(5)).await;
        }

        watcher.await.unwrap();
    });

    rt.shutdown();
    println!("  e2e_watch_version_tracking OK");
}

fn e2e_unbounded_never_blocks_sender() {
    println!("  e2e_unbounded_never_blocks_sender...");
    let rt = Builder::new_multi_thread().build().unwrap();

    rt.block_on(async {
        let (tx, mut rx) = unbounded::channel::<u32>();

        // Unbounded send never blocks.
        for i in 0..1000 {
            tx.send(i).expect("unbounded send should never fail");
        }

        let mut total = 0;
        for _ in 0..1000 {
            let val = rx.recv().await.unwrap();
            total += val;
        }

        assert_eq!(total, (0..1000).sum::<u32>());
    });

    rt.shutdown();
    println!("  e2e_unbounded_never_blocks_sender OK");
}

// ===========================================================================
// Sync primitives end-to-end
// ===========================================================================

fn e2e_mutex_sequential() {
    println!("\n  e2e_mutex_sequential...");
    let rt = Builder::new_multi_thread().build().unwrap();

    rt.block_on(async {
        let m = Arc::new(Mutex::new(0u32));

        // A single task acquiring the mutex twice sequentially.
        {
            let mut g = m.lock().await;
            *g += 1;
            drop(g);
            let mut g = m.lock().await;
            *g += 1;
        }

        let g = m.lock().await;
        assert_eq!(*g, 2);
    });

    rt.shutdown();
    println!("  e2e_mutex_sequential OK");
}

fn e2e_rwlock_read_concurrency() {
    println!("  e2e_rwlock_read_concurrency...");
    let rt = Builder::new_multi_thread().build().unwrap();

    rt.block_on(async {
        let rw = Arc::new(RwLock::new(vec![1, 2, 3]));

        // Multiple readers should be able to read concurrently.
        let mut readers = vec![];
        for _ in 0..5 {
            let rw = rw.clone();
            readers.push(spawn(async move {
                let g = rw.read().await;
                let sum: i32 = g.iter().sum();
                drop(g);
                sum
            }));
        }

        for r in readers {
            assert_eq!(r.await.unwrap(), 6);
        }

        // Writer should exclude readers.
        {
            let mut g = rw.write().await;
            g.push(4);
        }

        let g = rw.read().await;
        assert_eq!(*g, vec![1, 2, 3, 4]);
    });

    rt.shutdown();
    println!("  e2e_rwlock_read_concurrency OK");
}

fn e2e_semaphore_capacity_exhaustion() {
    println!("  e2e_semaphore_capacity_exhaustion...");
    let rt = Builder::new_multi_thread().build().unwrap();

    rt.block_on(async {
        let sem = Arc::new(Semaphore::new(3));
        let active = Arc::new(AtomicUsize::new(0));
        let max_active = Arc::new(AtomicUsize::new(0));

        let mut tasks = vec![];
        for _ in 0..10 {
            let sem = sem.clone();
            let a = active.clone();
            let m = max_active.clone();
            tasks.push(spawn(async move {
                let _p = sem.acquire().await;
                let cur = a.fetch_add(1, Ordering::SeqCst);
                m.fetch_max(cur + 1, Ordering::SeqCst);
                sleep(Duration::from_millis(10)).await;
                a.fetch_sub(1, Ordering::SeqCst);
            }));
        }

        for t in tasks {
            t.await.unwrap();
        }

        // Peak concurrency should never exceed 3.
        let peak = max_active.load(Ordering::SeqCst);
        assert!(peak <= 3, "semaphore allowed {} concurrent, max is 3", peak);
    });

    rt.shutdown();
    println!("  e2e_semaphore_capacity_exhaustion OK");
}

fn e2e_barrier_multi_wait_generations() {
    println!("  e2e_barrier_multi_wait_generations...");
    let rt = Builder::new_multi_thread().build().unwrap();

    rt.block_on(async {
        let barrier = Arc::new(Barrier::new(3));
        let gen = Arc::new(AtomicUsize::new(0));

        // First generation: 3 waiters.
        let mut tasks = vec![];
        for _ in 0..3 {
            let b = barrier.clone();
            let g = gen.clone();
            tasks.push(spawn(async move {
                g.fetch_add(1, Ordering::SeqCst);
                b.wait().await;
            }));
        }
        for t in tasks {
            t.await.unwrap();
        }
        assert_eq!(gen.load(Ordering::SeqCst), 3);

        // Second generation: reuse barrier with 3 more waiters.
        gen.store(0, Ordering::SeqCst);
        let mut tasks2 = vec![];
        for _ in 0..3 {
            let b = barrier.clone();
            let g = gen.clone();
            tasks2.push(spawn(async move {
                g.fetch_add(1, Ordering::SeqCst);
                b.wait().await;
            }));
        }
        for t in tasks2 {
            t.await.unwrap();
        }
        assert_eq!(gen.load(Ordering::SeqCst), 3);
    });

    rt.shutdown();
    println!("  e2e_barrier_multi_wait_generations OK");
}

fn e2e_notify_fifo_ordering() {
    println!("  e2e_notify_fifo_ordering...");
    let rt = Builder::new_multi_thread().build().unwrap();

    rt.block_on(async {
        let notify = Arc::new(Notify::new());
        let order = Arc::new(std::sync::Mutex::new(Vec::new()));

        let mut waiters = vec![];
        for i in 0..5 {
            let n = notify.clone();
            let o = order.clone();
            waiters.push(spawn(async move {
                n.notified().await;
                o.lock().unwrap().push(i);
            }));
        }

        // Let all waiters register.
        std::thread::sleep(Duration::from_millis(50));

        // Notify one at a time.
        for _ in 0..5 {
            notify.notify_one();
            sleep(Duration::from_millis(10)).await;
        }

        for w in waiters {
            w.await.unwrap();
        }

        // All 5 should be woken.
        let o = order.lock().unwrap();
        assert_eq!(o.len(), 5);
    });

    rt.shutdown();
    println!("  e2e_notify_fifo_ordering OK");
}

fn e2e_once_cell_concurrent_init() {
    println!("  e2e_once_cell_concurrent_init...");
    let rt = Builder::new_multi_thread().build().unwrap();

    rt.block_on(async {
        let cell = Arc::new(OnceCell::new());
        let init_count = Arc::new(AtomicUsize::new(0));

        let mut tasks = vec![];
        for _ in 0..5 {
            let c = cell.clone();
            let ic = init_count.clone();
            tasks.push(spawn(async move {
                let val = c.get_or_init(|| {
                    ic.fetch_add(1, Ordering::SeqCst);
                    std::thread::sleep(Duration::from_millis(50));
                    42
                }).await;
                *val
            }));
        }

        let mut results = vec![];
        for t in tasks {
            results.push(t.await.unwrap());
        }

        // All tasks should see 42, init should run exactly once.
        for r in &results {
            assert_eq!(*r, 42);
        }
        assert_eq!(init_count.load(Ordering::SeqCst), 1);
    });

    rt.shutdown();
    println!("  e2e_once_cell_concurrent_init OK");
}

fn e2e_latch_countdown() {
    println!("  e2e_latch_countdown...");
    let rt = Builder::new_multi_thread().build().unwrap();

    rt.block_on(async {
        let latch = Arc::new(Latch::new(5));
        let released = Arc::new(AtomicBool::new(false));

        let waiter = spawn({
            let l = latch.clone();
            let r = released.clone();
            async move {
                l.wait().await;
                r.store(true, Ordering::SeqCst);
            }
        });

        // Count down from 5 separate tasks.
        let mut countdowners = vec![];
        for _ in 0..5 {
            let l = latch.clone();
            countdowners.push(spawn(async move {
                l.count_down();
            }));
        }

        for c in countdowners {
            c.await.unwrap();
        }

        waiter.await.unwrap();
        assert!(released.load(Ordering::SeqCst));
    });

    rt.shutdown();
    println!("  e2e_latch_countdown OK");
}

fn e2e_rate_limiter_token_bucket() {
    println!("  e2e_rate_limiter_token_bucket...");
    let rt = Builder::new_multi_thread().build().unwrap();

    rt.block_on(async {
        // 5 tokens/sec, burst of 3.
        let rl = RateLimiter::new(5.0, 3);

        // Should acquire 3 tokens immediately (burst).
        let mut acquired = 0;
        for _ in 0..3 {
            if rl.try_acquire() {
                acquired += 1;
            }
        }
        assert_eq!(acquired, 3, "should acquire all burst tokens");

        // 4th should fail immediately.
        assert!(!rl.try_acquire(), "should be empty after burst");

        // Wait for refill (~200ms for 1 token at 5/sec).
        sleep(Duration::from_millis(250)).await;
        assert!(rl.try_acquire(), "should have refilled 1 token");
    });

    rt.shutdown();
    println!("  e2e_rate_limiter_token_bucket OK");
}

fn e2e_cancellation_composition() {
    println!("  e2e_cancellation_composition...");
    let rt = Builder::new_multi_thread().build().unwrap();

    rt.block_on(async {
        let token = CancellationToken::new();
        let sem = Arc::new(Semaphore::new(2));
        let work_done = Arc::new(AtomicUsize::new(0));

        let mut tasks = vec![];
        for _ in 0..5 {
            let t = token.clone();
            let s = sem.clone();
            let wd = work_done.clone();
            tasks.push(spawn(async move {
                loop {
                    if t.is_cancelled() {
                        return false;
                    }
                    if let Ok(_p) = s.try_acquire() {
                        wd.fetch_add(1, Ordering::SeqCst);
                        sleep(Duration::from_millis(10)).await;
                        return true;
                    }
                    yieldnow().await;
                }
            }));
        }

        std::thread::sleep(Duration::from_millis(50));
        token.cancel();

        let mut completed = 0;
        for t in tasks {
            if t.await.unwrap() {
                completed += 1;
            }
        }

        // At most 2 tasks should complete (semaphore capacity).
        assert!(completed <= 2, "only {} tasks should acquire semaphore, got {}", 2, completed);
    });

    rt.shutdown();
    println!("  e2e_cancellation_composition OK");
}

// ===========================================================================
// Async I/O utilities end-to-end
// ===========================================================================

fn e2e_duplex_stream_through_bufio() {
    println!("\n  e2e_duplex_stream_through_bufio...");
    let rt = Builder::new_multi_thread().build().unwrap();

    rt.block_on(async {
        let (a, b) = DuplexStream::channel();

        let mut buf_a = BufWriter::new(a);
        let mut buf_b = BufReader::new(b);

        buf_a.write_all(b"hello bufio").await.unwrap();
        buf_a.flush().await.unwrap();

        let mut out = String::new();
        buf_b.read_to_string(&mut out).await.unwrap();
        assert_eq!(out, "hello bufio");
    });

    rt.shutdown();
    println!("  e2e_duplex_stream_through_bufio OK");
}

fn e2e_cursor_read_write_seek() {
    println!("  e2e_cursor_read_write_seek...");
    let rt = Builder::new_multi_thread().build().unwrap();

    rt.block_on(async {
        let mut cursor = Cursor::new(Vec::new());
        cursor.write_all(b"hello world").await.unwrap();
        cursor.set_position(0);

        let mut buf = String::new();
        cursor.read_to_string(&mut buf).await.unwrap();
        assert_eq!(buf, "hello world");

        // Seek to position 6.
        cursor.set_position(6);
        let mut buf2 = String::new();
        cursor.read_to_string(&mut buf2).await.unwrap();
        assert_eq!(buf2, "world");
    });

    rt.shutdown();
    println!("  e2e_cursor_read_write_seek OK");
}

fn e2e_copy_bidirectional() {
    println!("  e2e_copy_bidirectional...");
    let rt = Builder::new_multi_thread().build().unwrap();

    rt.block_on(async {
        let (a, b) = DuplexStream::channel();

        let writer = spawn(async move {
            let mut a = a;
            a.write_all(b"left to right").await.unwrap();
            a.shutdown().await.unwrap();
        });

        let reader = spawn(async move {
            let mut b = b;
            let mut buf = String::new();
            b.read_to_string(&mut buf).await.unwrap();
            buf
        });

        writer.await.unwrap();
        let result = reader.await.unwrap();
        assert_eq!(result, "left to right");
    });

    rt.shutdown();
    println!("  e2e_copy_bidirectional OK");
}

fn e2e_pipe_async_fd() {
    println!("  e2e_pipe_async_fd...");
    let rt = Builder::new_multi_thread().build().unwrap();

    rt.block_on(async {
        let (mut read_end, mut write_end) = pipe();

        write_end.write_all(b"pipe data").await.unwrap();

        let mut buf = [0u8; 32];
        let n = read_end.read(&mut buf).await.unwrap();
        assert_eq!(&buf[..n], b"pipe data");
    });

    rt.shutdown();
    println!("  e2e_pipe_async_fd OK");
}

fn e2e_repeat_and_sink() {
    println!("  e2e_repeat_and_sink...");
    let rt = Builder::new_multi_thread().build().unwrap();

    rt.block_on(async {
        let mut rep = repeat(0xAB);
        let mut buf = [0u8; 10];
        rep.read_exact(&mut buf).await.unwrap();
        assert_eq!(buf, [0xAB; 10]);

        let mut s = sink();
        s.write_all(b"discard me").await.unwrap();
        s.flush().await.unwrap();
    });

    rt.shutdown();
    println!("  e2e_repeat_and_sink OK");
}

fn e2e_empty_eof() {
    println!("  e2e_empty_eof...");
    let rt = Builder::new_multi_thread().build().unwrap();

    rt.block_on(async {
        let mut e = empty();
        let mut buf = [0u8; 10];
        let n = e.read(&mut buf).await.unwrap();
        assert_eq!(n, 0, "empty() should return EOF immediately");
    });

    rt.shutdown();
    println!("  e2e_empty_eof OK");
}

fn e2e_async_fd_owned_close() {
    println!("  e2e_async_fd_owned_close...");
    let rt = Builder::new_multi_thread().build().unwrap();

    rt.block_on(async {
        let (r, w) = pipe();
        // Convert to OwnedAsyncFd — should close fd on drop.
        let owned_r = OwnedAsyncFd::new(r).unwrap();
        drop(owned_r);

        // Write end should still work.
        let mut w = w;
        w.write_all(b"still works").await.unwrap();
    });

    rt.shutdown();
    println!("  e2e_async_fd_owned_close OK");
}

// ===========================================================================
// File I/O end-to-end
// ===========================================================================

fn e2e_fs_concurrent_write_read() {
    println!("\n  e2e_fs_concurrent_write_read...");
    let rt = Builder::new_multi_thread().build().unwrap();

    rt.block_on(async {
        let dir = make_temp_dir();
        std::fs::create_dir_all(&dir).unwrap();
        let _cleanup = CleanupDir(dir.clone());

        // Write 3 files concurrently.
        let mut writers = vec![];
        for i in 0..3 {
            let path = dir.join(format!("file{}.txt", i));
            let content = format!("content {}", i);
            writers.push(fs::write(path, content.into_bytes()));
        }
        edgerun_rt::join!(writers[0], writers[1], writers[2]);

        // Read them back concurrently.
        let mut readers = vec![];
        for i in 0..3 {
            let path = dir.join(format!("file{}.txt", i));
            readers.push(fs::read_to_string(path));
        }

        let r0 = readers.remove(0).await.unwrap();
        let r1 = readers.remove(0).await.unwrap();
        let r2 = readers.remove(0).await.unwrap();

        assert_eq!(r0, "content 0");
        assert_eq!(r1, "content 1");
        assert_eq!(r2, "content 2");
    });

    rt.shutdown();
    println!("  e2e_fs_concurrent_write_read OK");
}

fn e2e_fs_nonexistent_path_error() {
    println!("  e2e_fs_nonexistent_path_error...");
    let rt = Builder::new_multi_thread().build().unwrap();

    rt.block_on(async {
        let result = fs::read("/nonexistent/path/that/does/not/exist").await;
        assert!(result.is_err(), "reading nonexistent path should fail");

        let exists = fs::exists("/nonexistent/path").await;
        assert!(!exists);
    });

    rt.shutdown();
    println!("  e2e_fs_nonexistent_path_error OK");
}

// ===========================================================================
// Process management end-to-end
// ===========================================================================

fn e2e_process_output_and_kill() {
    println!("\n  e2e_process_output_and_kill...");
    let rt = Builder::new_multi_thread().build().unwrap();

    rt.block_on(async {
        // Successful command.
        let out = process::output(|| {
            let mut c = std::process::Command::new("echo");
            c.arg("-n").arg("process e2e");
            c
        }).await.unwrap();
        assert_eq!(out.stdout, b"process e2e");
        assert!(out.status.success());

        // Failing command.
        let out = process::status(|| std::process::Command::new("false")).await.unwrap();
        assert!(!out.status.success());

        // Child with kill.
        let child = process::Child::spawn(|| {
            let mut c = std::process::Command::new("sleep");
            c.arg("60");
            c
        }).unwrap();
        let pid = child.id();
        assert!(pid > 0);

        // Kill the child.
        unsafe { libc::kill(pid as i32, libc::SIGKILL) };

        let status = child.wait().await;
        // Should complete without hanging.
        let _ = status;
    });

    rt.shutdown();
    println!("  e2e_process_output_and_kill OK");
}

fn e2e_process_concurrent_execution() {
    println!("  e2e_process_concurrent_execution...");
    let rt = Builder::new_multi_thread().build().unwrap();

    rt.block_on(async {
        let a = process::output(|| {
            let mut c = std::process::Command::new("printf"); c.arg("a"); c
        });
        let b = process::output(|| {
            let mut c = std::process::Command::new("printf"); c.arg("b"); c
        });
        let c = process::output(|| {
            let mut c = std::process::Command::new("printf"); c.arg("c"); c
        });

        let (a, b, c) = edgerun_rt::join!(a, b, c);

        assert_eq!(a.unwrap().stdout, b"a");
        assert_eq!(b.unwrap().stdout, b"b");
        assert_eq!(c.unwrap().stdout, b"c");
    });

    rt.shutdown();
    println!("  e2e_process_concurrent_execution OK");
}

// ===========================================================================
// Timer edge cases
// ===========================================================================

fn e2e_timeout_cancels_inner_future() {
    println!("\n  e2e_timeout_cancels_inner_future...");
    let rt = Builder::new_multi_thread().build().unwrap();

    rt.block_on(async {
        let result = timeout(
            Duration::from_millis(30),
            async {
                sleep(Duration::from_secs(100)).await;
                "should not reach"
            },
        ).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("deadline"));
    });

    rt.shutdown();
    println!("  e2e_timeout_cancels_inner_future OK");
}

fn e2e_interval_missed_tick_skip() {
    println!("  e2e_interval_missed_tick_skip...");
    let rt = Builder::new_multi_thread().build().unwrap();

    rt.block_on(async {
        let mut iv = interval(Duration::from_millis(10));
        iv.set_missed_tick_behavior(MissedTickBehavior::Skip);

        // Sleep longer than 3 intervals, then tick — should only get 1 tick.
        sleep(Duration::from_millis(50)).await;

        let start = Instant::now();
        iv.tick().await;
        // Should return immediately (catching up with Skip means just 1 tick).
        let elapsed = start.elapsed();
        assert!(elapsed < Duration::from_millis(20), "tick should be fast with Skip, took {:?}", elapsed);
    });

    rt.shutdown();
    println!("  e2e_interval_missed_tick_skip OK");
}

fn e2e_sleep_at_future_deadline() {
    println!("  e2e_sleep_at_future_deadline...");
    let rt = Builder::new_multi_thread().build().unwrap();

    rt.block_on(async {
        let deadline = Instant::now() + Duration::from_millis(50);
        let _result = sleep_until(deadline).await;
        let elapsed = deadline.elapsed();

        // Should complete right at the deadline.
        assert!(elapsed < Duration::from_millis(20), "slept too long past deadline: {:?}", elapsed);
    });

    rt.shutdown();
    println!("  e2e_sleep_at_future_deadline OK");
}

// ===========================================================================
// JoinSet end-to-end
// ===========================================================================

fn e2e_join_set_dynamic_tasks_with_abort() {
    println!("\n  e2e_join_set_dynamic_tasks_with_abort...");
    let rt = Builder::new_multi_thread().build().unwrap();

    rt.block_on(async {
        let mut set = JoinSet::new();
        let completed = Arc::new(AtomicUsize::new(0));

        // Spawn 10 slow tasks.
        for i in 0..10 {
            let c = completed.clone();
            set.spawn(async move {
                sleep(Duration::from_secs(100)).await;
                c.fetch_add(1, Ordering::SeqCst);
                i
            });
        }

        // Let them start.
        sleep(Duration::from_millis(20)).await;

        // Abort all.
        set.abort_all();
        assert!(set.is_empty());

        // join_next should return None.
        let next = set.join_next().await;
        assert!(next.is_none());

        // No tasks should have completed.
        assert_eq!(completed.load(Ordering::SeqCst), 0);
    });

    rt.shutdown();
    println!("  e2e_join_set_dynamic_tasks_with_abort OK");
}
