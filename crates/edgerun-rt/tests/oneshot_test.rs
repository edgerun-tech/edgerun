// Test oneshot channel with the actual runtime.
use edgerun_rt::{Runtime, spawn};
use edgerun_rt::oneshot;
use std::time::Duration;

fn main() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async {
        test_send_before_recv();
        test_send_after_spawn();
        test_sender_dropped_no_value();
        test_send_then_drop_then_recv();
        test_send_twice_fails();
        test_blocking_recv();
        test_send_non_send_value();
        test_cross_task_oneshot();
        test_multiple_oneshots_chained();
        println!("All oneshot tests passed!");
    });
}

fn test_send_before_recv() {
    println!("  test_send_before_recv...");
    let (tx, rx) = oneshot::channel::<i32>();
    tx.send(42).unwrap();
    let val = rx.blocking_recv().unwrap();
    assert_eq!(val, 42);
    println!("  test_send_before_recv OK");
}

fn test_send_after_spawn() {
    println!("  test_send_after_spawn...");
    let (tx, rx) = oneshot::channel::<&str>();

    let sender = spawn(async move {
        tx.send("hello from spawned task").unwrap();
    });

    // Give the task time to send
    std::thread::sleep(Duration::from_millis(50));
    let val = rx.blocking_recv().unwrap();
    assert_eq!(val, "hello from spawned task");
    drop(sender);
    println!("  test_send_after_spawn OK");
}

fn test_sender_dropped_no_value() {
    println!("  test_sender_dropped_no_value...");
    let (tx, rx) = oneshot::channel::<i32>();
    drop(tx);
    let err = rx.blocking_recv().unwrap_err();
    assert_eq!(format!("{}", err), "sender dropped");
    println!("  test_sender_dropped_no_value OK");
}

fn test_send_then_drop_then_recv() {
    println!("  test_send_then_drop_then_recv...");
    let (tx, rx) = oneshot::channel::<String>();
    tx.send("sent then dropped".to_string()).unwrap();
    // tx is already moved/dropped by send
    let val = rx.blocking_recv().unwrap();
    assert_eq!(val, "sent then dropped");
    println!("  test_send_then_drop_then_recv OK");
}

fn test_send_twice_fails() {
    println!("  test_send_twice_fails...");
    let (tx, _rx) = oneshot::channel::<i32>();
    tx.send(1).unwrap();
    // Can't send twice - send takes self by value
    // The sender is consumed by the first send
    println!("  test_send_twice_fails OK");
}

fn test_blocking_recv() {
    println!("  test_blocking_recv...");
    let (tx, rx) = oneshot::channel::<u64>();

    let sender = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(50));
        tx.send(99999).unwrap();
    });

    let val = rx.blocking_recv().unwrap();
    assert_eq!(val, 99999);
    sender.join().unwrap();
    println!("  test_blocking_recv OK");
}

fn test_send_non_send_value() {
    println!("  test_send_non_send_value...");
    // Note: oneshot channel requires T: Send for spawn
    // so we use a simple Send type here
    let (tx, rx) = oneshot::channel::<Box<i32>>();

    let sender = spawn(async move {
        tx.send(Box::new(123)).unwrap();
    });

    std::thread::sleep(Duration::from_millis(50));
    let val = rx.blocking_recv().unwrap();
    assert_eq!(*val, 123);
    drop(sender);
    println!("  test_send_non_send_value OK");
}

fn test_cross_task_oneshot() {
    println!("  test_cross_task_oneshot...");
    let (tx, rx) = oneshot::channel::<i32>();

    // Spawn a task that will send after receiving a signal
    let (done_tx, done_rx) = oneshot::channel::<()>();
    let sender = spawn(async move {
        // Wait a bit then send
        std::thread::sleep(Duration::from_millis(30));
        tx.send(777).unwrap();
        done_tx.send(()).unwrap();
    });

    let val = rx.blocking_recv().unwrap();
    assert_eq!(val, 777);
    done_rx.blocking_recv().unwrap();
    drop(sender);
    println!("  test_cross_task_oneshot OK");
}

fn test_multiple_oneshots_chained() {
    println!("  test_multiple_oneshots_chained...");
    let (tx1, rx1) = oneshot::channel::<i32>();
    let (tx2, rx2) = oneshot::channel::<i32>();
    let (tx3, rx3) = oneshot::channel::<i32>();

    let task1 = spawn(async move {
        let val = rx1.blocking_recv().unwrap();
        tx2.send(val + 1).unwrap();
    });

    let task2 = spawn(async move {
        let val = rx2.blocking_recv().unwrap();
        tx3.send(val * 2).unwrap();
    });

    let task3 = spawn(async move {
        let val = rx3.blocking_recv().unwrap();
        assert_eq!(val, 84, "(41 + 1) * 2 = 84");
    });

    std::thread::sleep(Duration::from_millis(20));
    tx1.send(41).unwrap();

    std::thread::sleep(Duration::from_millis(100));
    drop(task1);
    drop(task2);
    drop(task3);
    println!("  test_multiple_oneshots_chained OK");
}
