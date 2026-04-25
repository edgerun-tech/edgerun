// Test yieldnow with the actual runtime.
use edgerun_rt::{spawn, yieldnow, Runtime, YieldNow};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

fn main() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async {
        test_yield_basic();
        test_yield_type();
        test_yield_does_not_deadlock();
        test_yield_allows_other_work();
        println!("All yieldnow tests passed!");
    });
}

fn test_yield_basic() {
    println!("  test_yield_basic...");
    let counter = Arc::new(AtomicUsize::new(0));
    let c = counter.clone();
    let h = spawn(async move {
        yieldnow().await;
        c.fetch_add(1, Ordering::SeqCst);
    });
    std::thread::sleep(Duration::from_millis(50));
    assert_eq!(counter.load(Ordering::SeqCst), 1);
    drop(h);
    println!("  test_yield_basic OK");
}

fn test_yield_type() {
    println!("  test_yield_type...");
    let _fut: YieldNow = yieldnow();
    println!("  test_yield_type OK");
}

fn test_yield_does_not_deadlock() {
    println!("  test_yield_does_not_deadlock...");
    let counter = Arc::new(AtomicUsize::new(0));
    let c = counter.clone();

    let h = spawn(async move {
        yieldnow().await;
        edgerun_rt::sleep(Duration::from_millis(10)).await;
        yieldnow().await;
        edgerun_rt::sleep(Duration::from_millis(10)).await;
        c.fetch_add(1, Ordering::SeqCst);
    });

    std::thread::sleep(Duration::from_millis(500));
    assert_eq!(
        counter.load(Ordering::SeqCst),
        1,
        "task should not deadlock"
    );
    drop(h);
    println!("  test_yield_does_not_deadlock OK");
}

fn test_yield_allows_other_work() {
    println!("  test_yield_allows_other_work...");
    let c1 = Arc::new(AtomicUsize::new(0));
    let c2 = Arc::new(AtomicUsize::new(0));

    // Task 1 yields
    let c1a = c1.clone();
    let t1 = spawn(async move {
        yieldnow().await;
        edgerun_rt::sleep(Duration::from_millis(50)).await;
        c1a.fetch_add(1, Ordering::SeqCst);
    });

    // Task 2 doesn't yield - it should make progress independently
    let c2a = c2.clone();
    let t2 = spawn(async move {
        edgerun_rt::sleep(Duration::from_millis(10)).await;
        c2a.fetch_add(1, Ordering::SeqCst);
    });

    std::thread::sleep(Duration::from_millis(500));
    assert_eq!(c1.load(Ordering::SeqCst), 1, "task 1 should complete");
    assert_eq!(c2.load(Ordering::SeqCst), 1, "task 2 should complete");
    drop(t1);
    drop(t2);
    println!("  test_yield_allows_other_work OK");
}
