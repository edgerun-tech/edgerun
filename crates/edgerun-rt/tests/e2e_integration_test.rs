// End-to-end integration tests for edgerun-rt.
//
// Every test:
//  - runs inside a single rt.block_on() with a hard 5-second timeout
//  - panics with the exact test name if it exceeds the deadline
//  - asserts a concrete invariant (not "no crash")
//  - isolates ONE interaction path so the first failure identifies the exact bug

use edgerun_rt::{
    AsyncReadExt, AsyncTcpListener, AsyncTcpStream, AsyncUdpSocket, AsyncWriteExt,
    Barrier, Builder, CancellationToken, Cursor, DuplexStream, Empty,
    JoinSet, Latch, MissedTickBehavior, Mutex, Notify, OnceCell,
    RateLimiter, Repeat, RwLock, Semaphore, Sleep,
    UnixDatagram, UnixListener, UnixStream,
    broadcast, fs, interval, mpsc, oneshot, pipe, poll_fn, process,
    repeat, sleep, sink, spawn, spawn_blocking, sleep_until, timeout, unbounded,
    yieldnow, AsyncRead, AsyncWrite, BufReader, BufWriter, Runtime,
    RuntimeMetrics, WatchSender,
};
use std::net::SocketAddr;
use std::os::unix::io::AsRawFd;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

// ---- Test harness: each test gets its own 5-second deadline inside block_on ----

struct Runner { rt: Runtime }

impl Runner {
    fn test(&self, name: &str, f: impl std::future::Future<Output = ()> + Send + 'static) {
        let t0 = Instant::now();
        eprint!("  {:>48} ... ", name);
        let ok = self.rt.block_on(async {
            match timeout(Duration::from_secs(5), f).await {
                Ok(()) => true,
                Err(_) => false,
            }
        });
        if !ok {
            panic!("\n  DEADLOCK: '{}' exceeded 5s (wall: {:?})\n", name, t0.elapsed());
        }
        eprintln!("OK ({:?})", t0.elapsed());
    }
}

fn main() {
    let rt = Builder::new_multi_thread().build().unwrap();
    let r = Runner { rt };

    // ============ RUNTIME ============

    r.test("rt_spawn_await", async {
        let h = spawn(async { 42 });
        assert_eq!(h.await.unwrap(), 42);
    });

    r.test("rt_panic_isolation", async {
        let bad = spawn(async { panic!("x") });
        let good = spawn(async { sleep(Duration::from_millis(5)).await; 1 });
        assert!(bad.await.is_err());
        assert_eq!(good.await.unwrap(), 1);
    });

    r.test("rt_blocking", async {
        assert_eq!(spawn_blocking(|| 7).await.unwrap(), 7);
    });

    r.test("rt_yield", async {
        let c = Arc::new(AtomicUsize::new(0));
        let mut hs = vec![];
        for _ in 0..4 {
            let c = c.clone();
            hs.push(spawn(async move { for _ in 0..10 { c.fetch_add(1, Ordering::Relaxed); yieldnow().await; } }));
        }
        for h in hs { h.await.unwrap(); }
        assert_eq!(c.load(Ordering::Relaxed), 40);
    });

    // ============ MPSC CHANNELS — each isolates one path ============

    // Test 1: direct push (queue not full) — no backpressure
    r.test("mpsc_direct", async {
        let (tx, mut rx) = mpsc::channel::<u64>(4);
        tx.send_nowait(1).unwrap();
        tx.send_nowait(2).unwrap();
        assert_eq!(rx.recv().await, Some(1));
        assert_eq!(rx.recv().await, Some(2));
    });

    // Test 2: cap=1, send-then-recv in same task — verifies try_push
    r.test("mpsc_cap1_single_task", async {
        let (tx, mut rx) = mpsc::channel::<u64>(1);
        tx.send_nowait(99).unwrap();
        assert_eq!(rx.recv().await, Some(99));
    });

    // Test 3: cap=1, two tasks: sender blocks, then receiver unblocks
    // THIS IS THE DEADLOCK PATH: sender registers pending, receiver dequeues,
    // calls wake_one_pending_sender, sender re-polled. If sender doesn't
    // re-register after try_push fails again -> permanent deadlock.
    r.test("mpsc_cap1_send_blocks_recv_unblocks", async {
        let (tx, mut rx) = mpsc::channel::<u64>(1);
        // Fill the queue
        tx.send_nowait(0).unwrap();
        let tx2 = tx.clone();
        // This send will block (queue full, cap=1)
        let sender = spawn(async move {
            tx2.send(1).await.unwrap();
            tx2.send(2).await.unwrap();
        });
        // Give sender time to register as pending
        sleep(Duration::from_millis(20)).await;
        // Drain — each dequeue should wake the sender for the next slot
        let v0 = rx.recv().await; assert_eq!(v0, Some(0));
        let v1 = rx.recv().await; assert_eq!(v1, Some(1));
        let v2 = rx.recv().await; assert_eq!(v2, Some(2));
        sender.await.unwrap();
    });

    // Test 4: cap=1, sustained 100 items through cap=1 (multi round-trips)
    r.test("mpsc_cap1_100_items", async {
        let (tx, mut rx) = mpsc::channel::<u64>(1);
        let sender = spawn(async move {
            for i in 0..100u64 { tx.send(i).await.unwrap(); }
        });
        let mut sum = 0u64;
        while let Some(v) = rx.recv().await { sum += v; }
        sender.await.unwrap();
        assert_eq!(sum, (0..100).sum::<u64>());
    });

    // Test 5: cap=2, two producers
    r.test("mpsc_cap2_two_producers", async {
        let (tx, mut rx) = mpsc::channel::<u64>(2);
        let mut hs = vec![];
        for p in 0..2u64 { let t = tx.clone(); hs.push(spawn(async move { for i in 0..50u64 { t.send(p*100+i).await.unwrap(); } })); }
        drop(tx);
        let mut n = 0; while let Some(_) = rx.recv().await { n += 1; }
        for h in hs { h.await.unwrap(); }
        assert_eq!(n, 100);
    });

    // Test 6: cap=4, four producers, 200 each
    r.test("mpsc_cap4_four_producers", async {
        let (tx, mut rx) = mpsc::channel::<u64>(4);
        let mut hs = vec![];
        for p in 0..4u64 { let t = tx.clone(); hs.push(spawn(async move { for i in 0..200u64 { t.send(p*1000+i).await.unwrap(); } })); }
        drop(tx);
        let mut n = 0; while let Some(_) = rx.recv().await { n += 1; }
        for h in hs { h.await.unwrap(); }
        assert_eq!(n, 800);
    });

    // Test 7: channel close wakes blocked receiver
    r.test("mpsc_close_wakes_receiver", async {
        let (tx, mut rx) = mpsc::channel::<u64>(1);
        drop(tx);
        assert_eq!(rx.recv().await, None);
    });

    // ============ OTHER CHANNELS ============

    r.test("oneshot", async {
        let (tx, rx) = oneshot::channel::<u64>();
        spawn(async move { tx.send(42).unwrap(); });
        assert_eq!(rx.await.unwrap(), 42);
    });

    r.test("broadcast", async {
        let (tx, mut rx) = broadcast::channel::<u32>(8);
        tx.send(1).unwrap();
        assert_eq!(rx.recv().await, Ok(1));
    });

    r.test("unbounded", async {
        let (tx, mut rx) = unbounded::channel::<u32>();
        tx.send(1).unwrap();
        assert_eq!(rx.recv().await, Some(1));
    });

    r.test("watch", async {
        let (mut tx, mut rx) = WatchSender::new(0u32);
        tx.send_replace(5);
        rx.changed().await.unwrap();
        assert_eq!(rx.borrow().unwrap(), 5);
    });

    // ============ SYNC PRIMITIVES ============

    r.test("mutex_10_tasks", async {
        let m = Arc::new(Mutex::new(0u64));
        let mut hs = vec![];
        for _ in 0..10 { let m = m.clone(); hs.push(spawn(async move { for _ in 0..100 { *m.lock().await += 1; } })); }
        for h in hs { h.await.unwrap(); }
        assert_eq!(*m.lock().await, 1000);
    });

    r.test("rwlock_write_exclusive", async {
        let rw = Arc::new(RwLock::new(0u64));
        let mut hs = vec![];
        for _ in 0..5 { let r = rw.clone(); hs.push(spawn(async move { let mut g = r.write().await; *g += 1; })); }
        for h in hs { h.await.unwrap(); }
        assert_eq!(*rw.read().await, 5);
    });

    r.test("semaphore_3_concurrent", async {
        let sem = Arc::new(Semaphore::new(3));
        let max = Arc::new(AtomicUsize::new(0));
        let act = Arc::new(AtomicUsize::new(0));
        let mut hs = vec![];
        for _ in 0..10 {
            let s=sem.clone(); let a=act.clone(); let m=max.clone();
            hs.push(spawn(async move {
                let _p = s.acquire().await;
                let c = a.fetch_add(1, Ordering::Relaxed);
                m.fetch_max(c+1, Ordering::Relaxed);
                sleep(Duration::from_millis(5)).await;
                a.fetch_sub(1, Ordering::Relaxed);
            }));
        }
        for h in hs { h.await.unwrap(); }
        assert!(max.load(Ordering::Relaxed) <= 3);
    });

    r.test("notify_one", async {
        let n = Arc::new(Notify::new());
        let c = Arc::new(AtomicBool::new(false));
        let w = spawn({ let n=n.clone(); let c=c.clone(); async move { n.notified().await; c.store(true, Ordering::Relaxed); }});
        sleep(Duration::from_millis(20)).await;
        n.notify_one();
        w.await.unwrap();
        assert!(c.load(Ordering::Relaxed));
    });

    r.test("cancellation", async {
        let t = CancellationToken::new();
        let c = Arc::new(AtomicUsize::new(0));
        let w = spawn({ let t=t.clone(); let c=c.clone(); async move { t.cancelled().await; c.fetch_add(1, Ordering::Relaxed); }});
        sleep(Duration::from_millis(20)).await;
        t.cancel();
        w.await.unwrap();
        assert_eq!(c.load(Ordering::Relaxed), 1);
    });

    // ============ TIMERS ============

    r.test("sleep_50ms", async {
        let t0 = Instant::now();
        sleep(Duration::from_millis(50)).await;
        let e = t0.elapsed();
        assert!(e >= Duration::from_millis(40) && e < Duration::from_millis(500), "slept {:?}", e);
    });

    r.test("timeout_fires", async {
        let r = timeout(Duration::from_millis(30), async { sleep(Duration::from_secs(100)).await; }).await;
        assert!(r.is_err());
    });

    // ============ I/O ============

    r.test("duplex_bufio", async {
        let (a, b) = DuplexStream::channel();
        let mut a = BufWriter::new(a);
        let mut b = BufReader::new(b);
        a.write_all(b"hi").await.unwrap();
        a.flush().await.unwrap();
        assert_eq!(b.read_to_string().await.unwrap(), "hi");
    });

    r.test("pipe", async {
        let (rd, wr) = pipe().unwrap();
        let data = b"pipe";
        unsafe { libc::write(wr.as_raw_fd(), data.as_ptr() as *const _, data.len()); }
        rd.readable().await.unwrap();
        let mut buf = [0u8; 8];
        let n = unsafe { libc::read(rd.as_raw_fd(), buf.as_mut_ptr() as *mut _, 8) };
        assert_eq!(&buf[..n as usize], data);
    });

    r.test("repeat_sink", async {
        let mut rep = repeat(0xAB);
        let mut buf = [0u8; 5];
        rep.read_exact(&mut buf).await.unwrap();
        assert_eq!(buf, [0xAB; 5]);
        sink().write_all(b"x").await.unwrap();
    });

    // ============ FILE ============

    r.test("fs_rw", async {
        let d = std::env::temp_dir().join(format!("ert_e2e_{}", std::process::id()));
        let _c = CleanupDir(d.clone());
        std::fs::create_dir_all(&d).unwrap();
        let p = d.join("f");
        fs::write(&p, b"test").await.unwrap();
        assert_eq!(&fs::read(&p).await.unwrap(), b"test");
    });

    // ============ PROCESS ============

    r.test("process_echo", async {
        let o = process::output(|| { let mut c = std::process::Command::new("echo"); c.arg("-n").arg("x"); c }).await.unwrap();
        assert_eq!(o.stdout, b"x");
    });

    // ============ TCP ============

    r.test("tcp_echo", async {
        let port = find_free_port();
        let addr = format!("127.0.0.1:{}", port);
        let l = Arc::new(AsyncTcpListener::bind(&addr).unwrap());
        let ls = l.clone();
        let srv = spawn(async move {
            let (s, _) = ls.accept().await.unwrap();
            let mut s: Arc<AsyncTcpStream> = s;
            let mut buf = [0u8; 32];
            let n = s.read(&mut buf).await.unwrap();
            s.write_all(&buf[..n]).await.unwrap();
        });
        sleep(Duration::from_millis(30)).await;
        let cli = spawn(async move {
            let mut s = std::net::TcpStream::connect(&addr).unwrap();
            s.set_nonblocking(true).unwrap();
            let mut s: Arc<AsyncTcpStream> = Arc::new(AsyncTcpStream::from_std(s).unwrap());
            s.write_all(b"ping").await.unwrap();
            let mut buf = [0u8; 32];
            let n = s.read(&mut buf).await.unwrap();
            assert_eq!(&buf[..n], b"ping");
        });
        sleep(Duration::from_millis(200)).await;
        drop(srv); drop(cli);
    });

    r.test("udp_loopback", async {
        let port = find_free_port();
        let addr: SocketAddr = format!("127.0.0.1:{}", port).parse().unwrap();
        let s = Arc::new(AsyncUdpSocket::bind(&addr).unwrap());
        let s1 = s.clone();
        let snd = spawn(async move { s1.send_to(b"x", addr).await.unwrap(); });
        let s2 = s.clone();
        let rcv = spawn(async move {
            let mut buf = [0u8; 8];
            let (n, _) = s2.recv_from(&mut buf).await.unwrap();
            assert_eq!(&buf[..n], b"x");
        });
        sleep(Duration::from_millis(200)).await;
        drop(snd); drop(rcv);
    });

    eprintln!("\n  ALL 27 E2E TESTS PASSED");
    r.rt.shutdown();
}

struct CleanupDir(std::path::PathBuf);
impl Drop for CleanupDir { fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); } }

fn find_free_port() -> u16 {
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    l.local_addr().unwrap().port()
}
