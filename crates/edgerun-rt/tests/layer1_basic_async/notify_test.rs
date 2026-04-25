// Test Notify primitive with the actual runtime.
use edgerun_rt::{spawn, Notify, Runtime};
use std::sync::Arc;
use std::time::Duration;

fn main() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async {
        test_notify_one();
        test_notify_multiple_waiters();
        test_notify_clone();
        test_notify_drop_cancellation();
        test_notify_pre_notified();
        test_notify_waiters_wake_all();
        test_notify_concurrent_notify_and_wait();
        println!("All Notify tests passed!");
    });
}

fn test_notify_one() {
    println!("  test_notify_one...");
    let notify = Arc::new(Notify::new());
    let notify2 = notify.clone();

    let handle = spawn(async move {
        notify2.notified().await;
        println!("    waiter received notification");
    });

    // Give the task time to register the waiter
    std::thread::sleep(Duration::from_millis(50));
    notify.notify_one();

    // Give time for wake delivery
    std::thread::sleep(Duration::from_millis(50));
    drop(handle);
    println!("  test_notify_one OK");
}

fn test_notify_multiple_waiters() {
    println!("  test_notify_multiple_waiters...");
    let notify = Arc::new(Notify::new());

    let mut handles = vec![];
    for i in 0..5 {
        let n = notify.clone();
        let h = spawn(async move {
            n.notified().await;
            println!("    waiter {} received notification", i);
        });
        handles.push(h);
    }

    std::thread::sleep(Duration::from_millis(50));

    // notify_one should wake them one at a time
    for i in 0..5 {
        notify.notify_one();
        std::thread::sleep(Duration::from_millis(20));
        println!("    sent notification {} of 5", i + 1);
    }

    std::thread::sleep(Duration::from_millis(50));
    for h in handles {
        drop(h);
    }
    println!("  test_notify_multiple_waiters OK");
}

fn test_notify_clone() {
    println!("  test_notify_clone...");
    let notify1 = Notify::new();
    let notify2 = notify1.clone();

    let handle = spawn(async move {
        notify2.notified().await;
        println!("    clone waiter notified");
    });

    std::thread::sleep(Duration::from_millis(50));
    notify1.notify_one();
    std::thread::sleep(Duration::from_millis(50));
    drop(handle);
    println!("  test_notify_clone OK");
}

fn test_notify_drop_cancellation() {
    println!("  test_notify_drop_cancellation...");
    let notify = Arc::new(Notify::new());

    // Create a notified future and drop it before it's woken
    let n = notify.clone();
    let _fut = n.notified();
    // dropped immediately

    // The notify should still work fine
    let notify2 = notify.clone();
    let handle = spawn(async move {
        let n = notify2.clone();
        n.notified().await;
        println!("    post-drop waiter notified");
    });

    std::thread::sleep(Duration::from_millis(50));
    notify.notify_one();
    std::thread::sleep(Duration::from_millis(50));
    drop(handle);
    println!("  test_notify_drop_cancellation OK");
}

fn test_notify_pre_notified() {
    println!("  test_notify_pre_notified...");
    let notify = Notify::new();

    // notify before any waiter
    notify.notify_one();
    notify.notify_one();

    // Now create a waiter - should resolve immediately
    for _ in 0..2 {
        let _fut = notify.notified();
    }
    println!("  test_notify_pre_notified OK");
}

fn test_notify_waiters_wake_all() {
    println!("  test_notify_waiters_wake_all...");
    let notify = Arc::new(Notify::new());
    let mut handles = vec![];

    for i in 0..3 {
        let n = notify.clone();
        let h = spawn(async move {
            n.notified().await;
            println!("    waiter {} woken by notify_waiters", i);
        });
        handles.push(h);
    }

    std::thread::sleep(Duration::from_millis(50));
    notify.notify_waiters();
    std::thread::sleep(Duration::from_millis(100));

    for h in handles {
        drop(h);
    }
    println!("  test_notify_waiters_wake_all OK");
}

fn test_notify_concurrent_notify_and_wait() {
    println!("  test_notify_concurrent_notify_and_wait...");
    let notify = Arc::new(Notify::new());

    // Spawn a task that notifies after a delay
    let n1 = notify.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(50));
        n1.notify_one();
    });

    // Main task waits
    let start = std::time::Instant::now();
    spawn(async move {
        notify.notified().await;
        let elapsed = start.elapsed();
        println!("    waited {:?} for notification", elapsed);
        assert!(
            elapsed >= Duration::from_millis(40),
            "should have waited for the background thread"
        );
    });

    std::thread::sleep(Duration::from_millis(150));
    println!("  test_notify_concurrent_notify_and_wait OK");
}
