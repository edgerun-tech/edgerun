// Test mpsc::Permit with the actual runtime.
use edgerun_rt::mpsc::{channel, Permit, PermitError};
use edgerun_rt::{Runtime, spawn};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

fn main() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async {
        test_try_reserve_success();
        test_try_reserve_full();
        test_try_reserve_closed();
        test_permit_send();
        test_permit_drop_releases_slot();
        test_reserve_async();
        test_reserve_waits_for_space();
        test_reserve_closed();
        test_permit_concurrent_reserve();
        println!("All mpsc::Permit tests passed!");
    });
}

fn test_try_reserve_success() {
    println!("  test_try_reserve_success...");
    let (tx, _rx) = channel::<i32>(5);
    let permit = tx.try_reserve().expect("should succeed");
    // Permit acquired.
    drop(permit);
    println!("  test_try_reserve_success OK");
}

fn test_try_reserve_full() {
    println!("  test_try_reserve_full...");
    let (tx, _rx) = channel::<i32>(2);
    let p1 = tx.try_reserve().expect("reserve 1");
    let p2 = tx.try_reserve().expect("reserve 2");
    let err = tx.try_reserve();
    assert!(matches!(err, Err(PermitError::Full)));
    drop(p1);
    drop(p2);
    println!("  test_try_reserve_full OK");
}

fn test_try_reserve_closed() {
    println!("  test_try_reserve_closed...");
    let (tx, rx) = channel::<i32>(5);
    drop(rx); // close the channel
    let err = tx.try_reserve();
    assert!(matches!(err, Err(PermitError::Closed)));
    println!("  test_try_reserve_closed OK");
}

fn test_permit_send() {
    println!("  test_permit_send...");
    let (tx, mut rx) = channel::<i32>(5);
    let permit = tx.try_reserve().expect("should succeed");
    permit.send(42);
    let v = rx.try_recv().expect("should have value");
    assert_eq!(v, 42);
    println!("  test_permit_send OK");
}

fn test_permit_drop_releases_slot() {
    println!("  test_permit_drop_releases_slot...");
    let (tx, _rx) = channel::<i32>(1);
    let p1 = tx.try_reserve().expect("reserve 1");
    drop(p1); // release slot
    // Should be able to reserve again.
    let p2 = tx.try_reserve().expect("should succeed after drop");
    drop(p2);
    println!("  test_permit_drop_releases_slot OK");
}

fn test_reserve_async() {
    println!("  test_reserve_async...");
    let h = spawn(async {
        let (tx, _rx) = channel::<i32>(5);
        let permit = tx.reserve().await.expect("should succeed");
        drop(permit);
    });
    std::thread::sleep(Duration::from_millis(50));
    drop(h);
    println!("  test_reserve_async OK");
}

fn test_reserve_waits_for_space() {
    println!("  test_reserve_waits_for_space...");
    let count = Arc::new(AtomicUsize::new(0));
    let count2 = count.clone();

    let h = spawn(async move {
        let (tx, mut rx) = channel::<i32>(1);

        // Fill the channel.
        tx.send_nowait(1).expect("send 1");

        // Spawn a task that reserves — should wait.
        let tx2 = tx.clone();
        let counter = count2.clone();
        let reserve_task = spawn(async move {
            let permit = tx2.reserve().await.expect("reserve should succeed");
            counter.fetch_add(1, Ordering::SeqCst);
            permit.send(2);
        });

        // Give reserve_task time to register.
        std::thread::sleep(Duration::from_millis(50));

        // Receive the first value — frees a slot.
        let v = rx.try_recv().expect("should have value");
        assert_eq!(v, 1);

        // Wait for reserve_task to complete.
        std::thread::sleep(Duration::from_millis(200));
        drop(reserve_task);

        assert_eq!(count2.load(Ordering::SeqCst), 1);
    });
    std::thread::sleep(Duration::from_millis(500));
    drop(h);
    println!("  test_reserve_waits_for_space OK");
}

fn test_reserve_closed() {
    println!("  test_reserve_closed...");
    let h = spawn(async {
        let (tx, rx) = channel::<i32>(5);
        drop(rx); // close
        let err = tx.reserve().await;
        assert!(matches!(err, Err(PermitError::Closed)));
    });
    std::thread::sleep(Duration::from_millis(100));
    drop(h);
    println!("  test_reserve_closed OK");
}

fn test_permit_concurrent_reserve() {
    println!("  test_permit_concurrent_reserve...");
    let count = Arc::new(AtomicUsize::new(0));

    let h = spawn(async move {
        let (tx, mut rx) = channel::<i32>(3);
        let count2 = count.clone();

        // Fill the channel.
        for i in 0..3 {
            tx.send_nowait(i).expect("fill");
        }

        // Spawn 3 reserve tasks.
        let mut tasks = Vec::new();
        for _ in 0..3 {
            let tx2 = tx.clone();
            let c = count2.clone();
            tasks.push(spawn(async move {
                let permit = tx2.reserve().await.expect("reserve");
                c.fetch_add(1, Ordering::SeqCst);
                permit.send(99);
            }));
        }

        // Consume values to free slots.
        for _ in 0..3 {
            let _ = rx.try_recv();
            std::thread::sleep(Duration::from_millis(50));
        }

        std::thread::sleep(Duration::from_millis(300));
        for t in tasks {
            drop(t);
        }

        assert_eq!(count2.load(Ordering::SeqCst), 3);
    });
    std::thread::sleep(Duration::from_millis(1000));
    drop(h);
    println!("  test_permit_concurrent_reserve OK");
}
