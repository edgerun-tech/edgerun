// Test Mutex primitive with the actual runtime.
use edgerun_rt::{Mutex, Runtime, spawn};
use std::sync::Arc;
use std::time::Duration;

fn main() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async {
        test_lock_immediate();
        test_lock_modify();
        test_multiple_waiters_fifo();
        test_high_contention();
        test_clone_across_tasks();
        test_into_inner();
        test_default();
        test_heavy_incremental();
        println!("All Mutex tests passed!");
    });
}

fn test_lock_immediate() {
    println!("  test_lock_immediate...");
    let m = Arc::new(Mutex::new(42));
    let h = spawn(async move {
        let g = m.lock().await;
        assert_eq!(*g, 42);
    });
    std::thread::sleep(Duration::from_millis(50));
    drop(h);
    println!("  test_lock_immediate OK");
}

fn test_lock_modify() {
    println!("  test_lock_modify...");
    let m = Arc::new(Mutex::new(0u32));
    let m2 = m.clone();
    let h = spawn(async move {
        let mut g = m2.lock().await;
        *g = 999;
    });
    std::thread::sleep(Duration::from_millis(50));
    drop(h);

    let m3 = m.clone();
    let h2 = spawn(async move {
        let g = m3.lock().await;
        assert_eq!(*g, 999);
    });
    std::thread::sleep(Duration::from_millis(50));
    drop(h2);
    println!("  test_lock_modify OK");
}

fn test_multiple_waiters_fifo() {
    println!("  test_multiple_waiters_fifo...");
    let m = Arc::new(Mutex::new(Vec::<u32>::new()));
    let done = Arc::new(std::sync::atomic::AtomicUsize::new(0));

    // First task grabs the lock
    let m1 = m.clone();
    let dc1 = done.clone();
    let holder = spawn(async move {
        let mut g = m1.lock().await;
        g.push(0);
        std::thread::sleep(Duration::from_millis(50));
        drop(g);
        dc1.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    });

    std::thread::sleep(Duration::from_millis(10));

    // Spawn 3 waiters
    let mut handles = vec![];
    for i in 1..=3 {
        let mi = m.clone();
        let dc = done.clone();
        let h = spawn(async move {
            let mut g = mi.lock().await;
            g.push(i);
            println!("    waiter {} got lock", i);
            dc.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        });
        handles.push(h);
        std::thread::sleep(Duration::from_millis(10));
    }

    // Wait for all tasks
    while done.load(std::sync::atomic::Ordering::SeqCst) < 4 {
        std::thread::sleep(Duration::from_millis(10));
    }

    // Verify final state
    let m4 = m.clone();
    let checker = spawn(async move {
        let g = m4.lock().await;
        assert_eq!(g.len(), 4, "should have 4 entries");
        println!("    final vec: {:?}", *g);
    });
    std::thread::sleep(Duration::from_millis(50));
    drop(checker);
    drop(holder);
    for h in handles {
        drop(h);
    }
    println!("  test_multiple_waiters_fifo OK");
}

fn test_high_contention() {
    println!("  test_high_contention...");
    let m = Arc::new(Mutex::new(0u64));
    let iterations = 50u64;
    let done = Arc::new(std::sync::atomic::AtomicUsize::new(0));

    let mut handles = vec![];
    // 4 tasks each increment the counter
    for _ in 0..4 {
        let mi = m.clone();
        let dc = done.clone();
        let h = spawn(async move {
            for _ in 0..iterations {
                let mut g = mi.lock().await;
                *g += 1;
            }
            dc.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        });
        handles.push(h);
    }

    // Wait for all tasks to complete
    let start = std::time::Instant::now();
    while done.load(std::sync::atomic::Ordering::SeqCst) < 4 {
        std::thread::sleep(Duration::from_millis(10));
        if start.elapsed() > Duration::from_secs(5) {
            let count = done.load(std::sync::atomic::Ordering::SeqCst);
            panic!("timeout: only {} of 4 tasks completed", count);
        }
    }

    // Verify final value
    let mf = m.clone();
    let verifier = spawn(async move {
        let g = mf.lock().await;
        assert_eq!(*g, 4 * iterations, "4 tasks x {} iters = {}", iterations, 4 * iterations);
    });
    std::thread::sleep(Duration::from_millis(50));
    drop(verifier);
    for h in handles {
        drop(h);
    }
    println!("  test_high_contention OK");
}

fn test_clone_across_tasks() {
    println!("  test_clone_across_tasks...");
    let m = Arc::new(Mutex::new("initial"));

    let m2 = m.clone();
    let writer = spawn(async move {
        let mut g = m2.lock().await;
        *g = "updated";
    });

    let m3 = m.clone();
    let reader = spawn(async move {
        let g = m3.lock().await;
        assert_eq!(*g, "updated");
    });

    std::thread::sleep(Duration::from_millis(100));
    drop(writer);
    drop(reader);
    println!("  test_clone_across_tasks OK");
}

fn test_into_inner() {
    println!("  test_into_inner...");
    let m = Mutex::new(vec![1, 2, 3]);
    let v = m.into_inner();
    assert_eq!(v, vec![1, 2, 3]);
    println!("  test_into_inner OK");
}

fn test_default() {
    println!("  test_default...");
    let m = Mutex::<Vec<i32>>::default();
    let v = m.into_inner();
    assert!(v.is_empty());
    println!("  test_default OK");
}

fn test_heavy_incremental() {
    println!("  test_heavy_incremental...");
    let m = Arc::new(Mutex::new(0u64));
    let done = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut handles = vec![];

    // 4 tasks, each doing 25 increments
    for _ in 0..4 {
        let mi = m.clone();
        let dc = done.clone();
        let h = spawn(async move {
            for _ in 0..25 {
                let mut g = mi.lock().await;
                *g += 1;
            }
            dc.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        });
        handles.push(h);
    }

    // Wait for all tasks to complete
    let start = std::time::Instant::now();
    while done.load(std::sync::atomic::Ordering::SeqCst) < 4 {
        std::thread::sleep(Duration::from_millis(10));
        if start.elapsed() > Duration::from_secs(5) {
            let count = done.load(std::sync::atomic::Ordering::SeqCst);
            panic!("timeout: only {} of 4 tasks completed", count);
        }
    }

    let mf = m.clone();
    let verify = spawn(async move {
        let g = mf.lock().await;
        assert_eq!(*g, 100, "4 tasks x 25 increments = 100");
    });
    std::thread::sleep(Duration::from_millis(50));
    drop(verify);
    for h in handles {
        drop(h);
    }
    println!("  test_heavy_incremental OK");
}
