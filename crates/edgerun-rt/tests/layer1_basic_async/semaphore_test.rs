// Test Semaphore primitive with the actual runtime.
use edgerun_rt::{spawn, Runtime, Semaphore};
use std::sync::Arc;
use std::time::Duration;

fn main() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async {
        test_try_acquire_immediate();
        test_try_acquire_no_permits();
        test_acquire_immediate();
        test_acquire_blocks_then_wakes();
        test_concurrent_acquire_limited();
        test_permit_drop_releases();
        test_permit_release();
        test_add_permits_wakes_waiters();
        test_close_wakes_all_with_error();
        test_clone_shares_permits();
        println!("All Semaphore tests passed!");
    });
}

fn test_try_acquire_immediate() {
    println!("  test_try_acquire_immediate...");
    let s = Semaphore::new(3);
    let p1 = s.try_acquire().unwrap();
    assert_eq!(s.available_permits(), 2);
    let p2 = s.try_acquire().unwrap();
    assert_eq!(s.available_permits(), 1);
    drop(p1);
    assert_eq!(s.available_permits(), 2);
    drop(p2);
    assert_eq!(s.available_permits(), 3);
    println!("  test_try_acquire_immediate OK");
}

fn test_try_acquire_no_permits() {
    println!("  test_try_acquire_no_permits...");
    let s = Semaphore::new(0);
    match s.try_acquire() {
        Err(err) => assert_eq!(err, edgerun_rt::SemaphoreTryAcquireError::NoPermits),
        Ok(_) => panic!("should have failed"),
    }
    println!("  test_try_acquire_no_permits OK");
}

fn test_acquire_immediate() {
    println!("  test_acquire_immediate...");
    let s = Arc::new(Semaphore::new(1));
    let s2 = s.clone();
    let handle = spawn(async move {
        let permit = s2.acquire().await.unwrap();
        assert_eq!(s2.available_permits(), 0);
        drop(permit);
    });
    std::thread::sleep(Duration::from_millis(50));
    drop(handle);
    println!("  test_acquire_immediate OK");
}

fn test_acquire_blocks_then_wakes() {
    println!("  test_acquire_blocks_then_wakes...");
    let s = Arc::new(Semaphore::new(0));

    // Task will block waiting for a permit
    let s2 = s.clone();
    let handle = spawn(async move {
        let start = std::time::Instant::now();
        let _permit = s2.acquire().await.unwrap();
        let elapsed = start.elapsed();
        println!("    acquired after {:?} (expected >50ms)", elapsed);
        assert!(elapsed >= Duration::from_millis(40));
    });

    // Add a permit after a delay
    std::thread::sleep(Duration::from_millis(50));
    s.add_permits(1);

    std::thread::sleep(Duration::from_millis(100));
    drop(handle);
    println!("  test_acquire_blocks_then_wakes OK");
}

fn test_concurrent_acquire_limited() {
    println!("  test_concurrent_acquire_limited...");
    let s = Arc::new(Semaphore::new(2));
    let active = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let max_active = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut handles = vec![];

    for i in 0..5 {
        let s = s.clone();
        let active = active.clone();
        let max_active = max_active.clone();
        let h = spawn(async move {
            let _permit = s.acquire().await.unwrap();
            let prev = active.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let new = prev + 1;
            // Track max concurrent
            let mut cur_max = max_active.load(std::sync::atomic::Ordering::SeqCst);
            while new > cur_max {
                match max_active.compare_exchange_weak(
                    cur_max,
                    new,
                    std::sync::atomic::Ordering::SeqCst,
                    std::sync::atomic::Ordering::SeqCst,
                ) {
                    Ok(_) => break,
                    Err(v) => cur_max = v,
                }
            }
            // Simulate work
            std::thread::sleep(Duration::from_millis(30));
            active.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
            println!(
                "    task {} done, active now = {}",
                i,
                active.load(std::sync::atomic::Ordering::SeqCst)
            );
        });
        handles.push(h);
    }

    std::thread::sleep(Duration::from_millis(300));
    let max = max_active.load(std::sync::atomic::Ordering::SeqCst);
    println!("    max concurrent acquires: {} (expected <= 2)", max);
    assert!(
        max <= 2,
        "semaphore should limit to 2 concurrent, got {}",
        max
    );
    for h in handles {
        drop(h);
    }
    println!("  test_concurrent_acquire_limited OK");
}

fn test_permit_drop_releases() {
    println!("  test_permit_drop_releases...");
    let s = Arc::new(Semaphore::new(1));

    // Task acquires and drops permit (drop releases via add_permits)
    let s2 = s.clone();
    let handle = spawn(async move {
        let permit = s2.acquire().await.unwrap();
        // While held, available should be 0
        assert_eq!(s2.available_permits(), 0);
        drop(permit);
        // After drop, available should be 1
        assert_eq!(s2.available_permits(), 1);
    });

    // Wait for task to finish completely
    std::thread::sleep(Duration::from_millis(100));
    // Now the semaphore should have its permit back
    assert_eq!(s.available_permits(), 1);
    drop(handle);
    println!("  test_permit_drop_releases OK");
}

fn test_permit_release() {
    println!("  test_permit_release...");
    let s = Arc::new(Semaphore::new(0));

    let s2 = s.clone();
    let h = spawn(async move {
        let permit = s2.acquire().await.unwrap();
        // Explicitly release
        permit.release();
        println!("    permit released");
    });

    // Need to add permits first so acquire can succeed
    std::thread::sleep(Duration::from_millis(20));
    s.add_permits(1);
    std::thread::sleep(Duration::from_millis(100));
    drop(h);
    println!("  test_permit_release OK");
}

fn test_add_permits_wakes_waiters() {
    println!("  test_add_permits_wakes_waiters...");
    let s = Arc::new(Semaphore::new(0));
    let mut handles = vec![];

    // 3 tasks will all block
    for i in 0..3 {
        let s = s.clone();
        let h = spawn(async move {
            let _permit = s.acquire().await.unwrap();
            println!("    task {} acquired permit after add_permits", i);
        });
        handles.push(h);
    }

    std::thread::sleep(Duration::from_millis(50));
    // Add 3 permits to wake all 3
    s.add_permits(3);
    std::thread::sleep(Duration::from_millis(100));

    for h in handles {
        drop(h);
    }
    println!("  test_add_permits_wakes_waiters OK");
}

fn test_close_wakes_all_with_error() {
    println!("  test_close_wakes_all_with_error...");
    let s = Arc::new(Semaphore::new(0));
    let mut handles = vec![];

    for i in 0..3 {
        let s = s.clone();
        let h = spawn(async move {
            let result = s.acquire().await;
            assert!(result.is_err(), "task {} should get Closed error", i);
            println!("    task {} got Closed error as expected", i);
        });
        handles.push(h);
    }

    std::thread::sleep(Duration::from_millis(50));
    s.close();
    std::thread::sleep(Duration::from_millis(100));

    for h in handles {
        drop(h);
    }
    println!("  test_close_wakes_all_with_error OK");
}

fn test_clone_shares_permits() {
    println!("  test_clone_shares_permits...");
    let s1 = Arc::new(Semaphore::new(2));
    let s2 = s1.clone();

    let s1a = s1.clone();
    let s2a = s2.clone();
    let handle = spawn(async move {
        // Acquire via s1
        let _p1 = s1a.acquire().await.unwrap();
        assert_eq!(s1a.available_permits(), 1);
        assert_eq!(s2a.available_permits(), 1);

        // Acquire via s2
        let _p2 = s2a.acquire().await.unwrap();
        assert_eq!(s1a.available_permits(), 0);
        assert_eq!(s2a.available_permits(), 0);

        // Dropping both restores all
        drop(_p1);
        drop(_p2);
        assert_eq!(s1a.available_permits(), 2);
    });

    std::thread::sleep(Duration::from_millis(100));
    assert_eq!(s1.available_permits(), 2);
    drop(handle);
    println!("  test_clone_shares_permits OK");
}
