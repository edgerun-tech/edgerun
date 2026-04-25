// Test unbounded channel with the actual runtime.
use edgerun_rt::unbounded::{self, TryRecvError};
use edgerun_rt::{spawn, Runtime};
use std::time::Duration;

fn main() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async {
        test_send_recv_basic();
        test_async_recv();
        test_multiple_senders();
        test_sender_drop_closes();
        test_receiver_drop_closes();
        test_try_recv_empty_and_disconnected();
        test_len_and_is_empty();
        test_is_closed();
        test_send_after_close_fails();
        test_blocking_recv();
        test_async_recv_then_close();
        test_heavy_unbounded_throughput();
        println!("All unbounded tests passed!");
    });
}

fn test_send_recv_basic() {
    println!("  test_send_recv_basic...");
    let (tx, rx) = unbounded::channel::<i32>();
    tx.send(42).unwrap();
    tx.send(100).unwrap();
    drop(tx); // Close the channel so receiver gets None
    let mut rx = rx;
    assert_eq!(rx.blocking_recv(), Some(42));
    assert_eq!(rx.blocking_recv(), Some(100));
    assert_eq!(rx.blocking_recv(), None);
    println!("  test_send_recv_basic OK");
}

fn test_async_recv() {
    println!("  test_async_recv...");
    let (tx, rx) = unbounded::channel::<&str>();

    let sender = spawn(async move {
        tx.send("hello").unwrap();
        tx.send("world").unwrap();
    });

    let receiver = spawn(async move {
        let v1 = rx.recv().await;
        assert_eq!(v1, Some("hello"));
        let v2 = rx.recv().await;
        assert_eq!(v2, Some("world"));
        let v3 = rx.recv().await;
        assert_eq!(v3, None);
    });

    std::thread::sleep(Duration::from_millis(100));
    drop(sender);
    drop(receiver);
    println!("  test_async_recv OK");
}

fn test_multiple_senders() {
    println!("  test_multiple_senders...");
    let (tx, rx) = unbounded::channel::<i32>();
    let mut handles = vec![];

    for s in 0..5 {
        let tx = tx.clone();
        let h = spawn(async move {
            for i in 0..10 {
                tx.send(s * 10 + i).unwrap();
            }
        });
        handles.push(h);
    }
    drop(tx);

    let collector = spawn(async move {
        let mut received = Vec::new();
        while let Some(val) = rx.recv().await {
            received.push(val);
        }
        assert_eq!(received.len(), 50, "should receive all 50 items");
    });

    std::thread::sleep(Duration::from_millis(200));
    for h in handles {
        drop(h);
    }
    drop(collector);
    println!("  test_multiple_senders OK");
}

fn test_sender_drop_closes() {
    println!("  test_sender_drop_closes...");
    let (tx, rx) = unbounded::channel::<i32>();
    tx.send(1).unwrap();
    tx.send(2).unwrap();
    drop(tx);

    let mut rx = rx;
    assert_eq!(rx.blocking_recv(), Some(1));
    assert_eq!(rx.blocking_recv(), Some(2));
    assert_eq!(rx.blocking_recv(), None);
    println!("  test_sender_drop_closes OK");
}

fn test_receiver_drop_closes() {
    println!("  test_receiver_drop_closes...");
    let (tx, rx) = unbounded::channel::<i32>();
    drop(rx);

    // Send should fail after receiver dropped
    let result = tx.send(99);
    assert!(result.is_err(), "send should fail after receiver dropped");
    println!("  test_receiver_drop_closes OK");
}

fn test_try_recv_empty_and_disconnected() {
    println!("  test_try_recv_empty_and_disconnected...");
    let (tx, rx) = unbounded::channel::<i32>();

    assert_eq!(rx.try_recv(), Err(TryRecvError::Empty));

    drop(tx);
    assert_eq!(rx.try_recv(), Err(TryRecvError::Disconnected));
    println!("  test_try_recv_empty_and_disconnected OK");
}

fn test_len_and_is_empty() {
    println!("  test_len_and_is_empty...");
    let (tx, _rx) = unbounded::channel::<i32>();
    assert!(tx.is_empty());
    assert_eq!(tx.len(), 0);

    tx.send(1).unwrap();
    tx.send(2).unwrap();
    tx.send(3).unwrap();
    assert!(!tx.is_empty());
    assert_eq!(tx.len(), 3);
    println!("  test_len_and_is_empty OK");
}

fn test_is_closed() {
    println!("  test_is_closed...");
    let (tx, rx) = unbounded::channel::<i32>();
    assert!(!tx.is_closed());

    drop(rx);
    assert!(tx.is_closed());
    println!("  test_is_closed OK");
}

fn test_send_after_close_fails() {
    println!("  test_send_after_close_fails...");
    let (tx, rx) = unbounded::channel::<i32>();
    drop(rx);
    assert!(tx.is_closed());

    let result = tx.send(999);
    assert!(result.is_err(), "send after close should fail");
    println!("  test_send_after_close_fails OK");
}

fn test_blocking_recv() {
    println!("  test_blocking_recv...");
    let (tx, mut rx) = unbounded::channel::<u64>();

    let sender = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(50));
        tx.send(12345).unwrap();
    });

    let val = rx.blocking_recv().unwrap();
    assert_eq!(val, 12345);
    sender.join().unwrap();
    println!("  test_blocking_recv OK");
}

fn test_async_recv_then_close() {
    println!("  test_async_recv_then_close...");
    let (tx, rx) = unbounded::channel::<i32>();

    let receiver = spawn(async move {
        let mut count = 0;
        while let Some(_val) = rx.recv().await {
            count += 1;
        }
        assert_eq!(count, 3, "should receive 3 items");
    });

    std::thread::sleep(Duration::from_millis(20));
    tx.send(10).unwrap();
    tx.send(20).unwrap();
    tx.send(30).unwrap();
    drop(tx);

    std::thread::sleep(Duration::from_millis(100));
    drop(receiver);
    println!("  test_async_recv_then_close OK");
}

fn test_heavy_unbounded_throughput() {
    println!("  test_heavy_unbounded_throughput...");
    let (tx, rx) = unbounded::channel::<u64>();
    let num_items = 1000u64;

    let mut producer_handles = vec![];
    for p in 0..4 {
        let tx = tx.clone();
        let h = spawn(async move {
            for i in 0..num_items {
                tx.send(p * num_items + i).unwrap();
            }
        });
        producer_handles.push(h);
    }
    drop(tx);

    let consumer = spawn(async move {
        let mut count = 0u64;
        while let Some(_val) = rx.recv().await {
            count += 1;
        }
        let expected = 4 * num_items;
        assert_eq!(count, expected, "should receive all {} items", expected);
    });

    std::thread::sleep(Duration::from_millis(1000));
    for h in producer_handles {
        drop(h);
    }
    drop(consumer);
    println!("  test_heavy_unbounded_throughput OK");
}
