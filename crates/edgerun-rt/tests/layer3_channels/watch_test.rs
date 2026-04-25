// Test watch channel with the actual runtime.
use edgerun_rt::{spawn, Runtime, WatchSender};
use std::time::Duration;

fn main() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async {
        test_initial_value();
        test_send_replace_updates();
        test_has_changed();
        test_close_marks_receiver();
        test_clone_sender();
        test_borrow_clone();
        test_version_tracking();
        test_changed_ready();
        test_changed_pending_then_update();
        test_close_wakes_waiters();
        test_cross_task_watch_multi_update();
        test_cross_task_watch_close();
        println!("All watch tests passed!");
    });
}

fn test_initial_value() {
    println!("  test_initial_value...");
    let (tx, rx) = WatchSender::new(42i32);
    assert_eq!(*tx.borrow().unwrap(), 42);
    assert_eq!(rx.borrow(), Ok(42));
    // has_changed is true initially since version=1 > last_seen=0
    assert!(rx.has_changed());
    println!("  test_initial_value OK");
}

fn test_send_replace_updates() {
    println!("  test_send_replace_updates...");
    let (mut tx, mut rx) = WatchSender::new("hello");
    tx.send_replace("world");
    assert_eq!(rx.borrow(), Ok("world"));
    assert!(rx.has_changed());
    println!("  test_send_replace_updates OK");
}

fn test_has_changed() {
    println!("  test_has_changed...");
    let (mut tx, mut rx) = WatchSender::new(0u64);
    // Initial: version=1, last_seen=0, so has_changed is true
    assert!(rx.has_changed());
    tx.send_replace(1);
    // After another replace, version increments further
    assert!(rx.has_changed());
    println!("  test_has_changed OK");
}

fn test_close_marks_receiver() {
    println!("  test_close_marks_receiver...");
    let (tx, rx) = WatchSender::new("value");
    tx.close();
    assert!(rx.borrow().is_err(), "borrow after close should fail");
    println!("  test_close_marks_receiver OK");
}

fn test_clone_sender() {
    println!("  test_clone_sender...");
    let (mut tx1, mut rx) = WatchSender::new(100i32);
    let mut tx2 = tx1.clone();

    tx1.send_replace(200);
    assert_eq!(rx.borrow(), Ok(200));

    tx2.send_replace(300);
    assert_eq!(rx.borrow(), Ok(300));
    println!("  test_clone_sender OK");
}

fn test_borrow_clone() {
    println!("  test_borrow_clone...");
    let (mut tx, _rx) = WatchSender::new(vec![1, 2, 3]);
    let cloned = tx.borrow_clone().unwrap();
    assert_eq!(cloned, vec![1, 2, 3]);

    tx.send_replace(vec![4, 5]);
    let cloned2 = tx.borrow_clone().unwrap();
    assert_eq!(cloned2, vec![4, 5]);
    println!("  test_borrow_clone OK");
}

fn test_version_tracking() {
    println!("  test_version_tracking...");
    let (mut tx, mut rx) = WatchSender::new(0u64);
    assert_eq!(rx.version(), 1);
    // has_changed is true initially (version=1 > last_seen=0)
    assert!(rx.has_changed());

    tx.send_replace(1);
    assert_eq!(rx.version(), 2);

    tx.send_replace(2);
    assert_eq!(rx.version(), 3);

    tx.send_replace(3);
    assert_eq!(rx.version(), 4);
    println!("  test_version_tracking OK");
}

fn test_changed_ready() {
    println!("  test_changed_ready...");
    let (mut tx, mut rx) = WatchSender::new(0u32);
    tx.send_replace(1);

    // Receiver has pending change, so changed() resolves immediately
    let h = spawn(async move {
        rx.changed().await.unwrap();
    });

    std::thread::sleep(Duration::from_millis(50));
    drop(h);
    println!("  test_changed_ready OK");
}

fn test_changed_pending_then_update() {
    println!("  test_changed_pending_then_update...");
    let (mut tx, mut rx) = WatchSender::new("v0");

    // Receiver will block waiting for a change
    let h = spawn(async move {
        rx.changed().await.unwrap();
    });

    std::thread::sleep(Duration::from_millis(20));
    tx.send_replace("v1");

    std::thread::sleep(Duration::from_millis(50));
    drop(h);
    println!("  test_changed_pending_then_update OK");
}

fn test_close_wakes_waiters() {
    println!("  test_close_wakes_waiters...");
    let (mut tx, rx) = WatchSender::new(0u32);

    let mut rx = rx;
    let closer = spawn(async move {
        // Consume initial version
        rx.changed().await.unwrap();
        // Now wait for close
        let result = rx.changed().await;
        assert!(result.is_err(), "close should cause error");
    });

    std::thread::sleep(Duration::from_millis(20));
    tx.close();

    std::thread::sleep(Duration::from_millis(50));
    drop(closer);
    println!("  test_close_wakes_waiters OK");
}

fn test_cross_task_watch_multi_update() {
    println!("  test_cross_task_watch_multi_update...");
    let (mut tx, rx) = WatchSender::new("initial".to_string());

    let watcher = spawn(async move {
        let mut rx = rx;
        // Consume initial version
        rx.changed().await.unwrap();
        for expected in &["update1", "update2", "update3"] {
            rx.changed().await.expect("watcher changed failed");
            let val = rx.borrow().expect("watcher borrow failed");
            assert_eq!(val.as_str(), *expected, "saw unexpected value: {}", val);
        }
    });

    std::thread::sleep(Duration::from_millis(20));

    for val in &["update1", "update2", "update3"] {
        tx.send_replace(val.to_string());
        std::thread::sleep(Duration::from_millis(30));
    }

    std::thread::sleep(Duration::from_millis(100));
    drop(watcher);
    println!("  test_cross_task_watch_multi_update OK");
}

fn test_cross_task_watch_close() {
    println!("  test_cross_task_watch_close...");
    let (mut tx, rx) = WatchSender::new(0u32);

    let watcher = spawn(async move {
        let mut rx = rx;
        // Consume initial version
        rx.changed().await.unwrap();
        // Now wait for close
        let result = rx.changed().await;
        assert!(result.is_err(), "close should return error");
    });

    std::thread::sleep(Duration::from_millis(20));
    tx.close();

    std::thread::sleep(Duration::from_millis(50));
    drop(watcher);
    println!("  test_cross_task_watch_close OK");
}
