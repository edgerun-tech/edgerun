// Test mpsc (bounded multi-producer single-consumer) channel with the actual runtime.
use edgerun_rt::mpsc::{self, TryRecvError};
use edgerun_rt::{Runtime, spawn};
use std::time::Duration;

fn main() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async {
        test_send_nowait_recv();
        test_async_send_recv();
        test_backpressure_blocking();
        test_async_send_with_backpressure();
        test_multiple_senders();
        test_sender_drop_closes_channel();
        test_receiver_drop_closes_senders();
        test_try_recv_empty_and_disconnected();
        test_capacity_one();
        test_async_recv_then_close();
        test_heavy_producer_consumer();
        println!("All mpsc tests passed!");
    });
}

fn test_send_nowait_recv() {
    println!("  test_send_nowait_recv...");
    let (tx, mut rx) = mpsc::channel::<i32>(10);
    tx.send_nowait(42).unwrap();
    tx.send_nowait(100).unwrap();
    assert_eq!(rx.blocking_recv(), Some(42));
    assert_eq!(rx.blocking_recv(), Some(100));
    println!("  test_send_nowait_recv OK");
}

fn test_async_send_recv() {
    println!("  test_async_send_recv...");
    let (tx, rx) = mpsc::channel::<&str>(10);

    let sender = spawn(async move {
        tx.send("hello").await.unwrap();
        tx.send("world").await.unwrap();
    });

    let receiver = spawn(async move {
        let val1 = rx.recv().await;
        assert_eq!(val1, Some("hello"));
        let val2 = rx.recv().await;
        assert_eq!(val2, Some("world"));
    });

    std::thread::sleep(Duration::from_millis(100));
    drop(sender);
    drop(receiver);
    println!("  test_async_send_recv OK");
}

fn test_backpressure_blocking() {
    println!("  test_backpressure_blocking...");
    let (tx, rx) = mpsc::channel::<i32>(2);

    assert!(tx.send_nowait(1).is_ok());
    assert!(tx.send_nowait(2).is_ok());
    // Channel is full (capacity 2)
    let err = tx.send_nowait(3);
    assert!(err.is_err(), "send_nowait should fail when full");

    // Drain one slot
    let mut rx = rx;
    assert_eq!(rx.blocking_recv(), Some(1));
    // Now there's room
    assert!(tx.send_nowait(3).is_ok());
    assert_eq!(rx.blocking_recv(), Some(2));
    assert_eq!(rx.blocking_recv(), Some(3));
    println!("  test_backpressure_blocking OK");
}

fn test_async_send_with_backpressure() {
    println!("  test_async_send_with_backpressure...");
    let (tx, rx) = mpsc::channel::<i32>(1);
    let mut handles = vec![];

    // 3 senders try to send, but capacity is only 1
    for i in 1..=3 {
        let tx = tx.clone();
        let h = spawn(async move {
            let start = std::time::Instant::now();
            tx.send(i).await.unwrap();
            let elapsed = start.elapsed();
            println!("    sender {} enqueued after {:?}", i, elapsed);
        });
        handles.push(h);
    }

    // Drop the original sender so only the clones remain
    drop(tx);

    // Drain items one at a time
    let rx_handle = spawn(async move {
        for expected in 1..=3 {
            let val = rx.recv().await.unwrap();
            assert_eq!(val, expected);
            println!("    received {}", val);
        }
        // After all sends, channel should close
        let val = rx.recv().await;
        assert_eq!(val, None, "channel should be closed after all senders dropped");
    });

    std::thread::sleep(Duration::from_millis(300));
    for h in handles {
        drop(h);
    }
    drop(rx_handle);
    println!("  test_async_send_with_backpressure OK");
}

fn test_multiple_senders() {
    println!("  test_multiple_senders...");
    let (tx, rx) = mpsc::channel::<i32>(100);
    let mut handles = vec![];

    // 5 senders each send 10 items
    for s in 0..5 {
        let tx = tx.clone();
        let h = spawn(async move {
            for i in 0..10 {
                tx.send(s * 10 + i).await.unwrap();
            }
        });
        handles.push(h);
    }

    drop(tx); // drop original so only clones remain

    // Collect all items
    let collector = spawn(async move {
        let mut received = Vec::new();
        while let Some(val) = rx.recv().await {
            received.push(val);
        }
        assert_eq!(received.len(), 50, "should receive all 50 items");
    });

    std::thread::sleep(Duration::from_millis(300));
    for h in handles {
        drop(h);
    }
    drop(collector);
    println!("  test_multiple_senders OK");
}

fn test_sender_drop_closes_channel() {
    println!("  test_sender_drop_closes_channel...");
    let (tx, rx) = mpsc::channel::<i32>(10);
    tx.send_nowait(1).unwrap();
    tx.send_nowait(2).unwrap();
    drop(tx);

    // Receiver should drain existing items then get None
    let mut rx = rx;
    assert_eq!(rx.blocking_recv(), Some(1));
    assert_eq!(rx.blocking_recv(), Some(2));
    assert_eq!(rx.blocking_recv(), None, "channel closed after sender drop");
    println!("  test_sender_drop_closes_channel OK");
}

fn test_receiver_drop_closes_senders() {
    println!("  test_receiver_drop_closes_senders...");
    let (tx, rx) = mpsc::channel::<i32>(1);

    // Fill the channel
    assert!(tx.send_nowait(99).is_ok());

    // Drop receiver — should close the channel and wake pending senders
    drop(rx);

    // Now send should fail because channel is closed
    let err = tx.send_nowait(1);
    assert!(err.is_err(), "send should fail after receiver dropped");
    println!("  test_receiver_drop_closes_senders OK");
}

fn test_try_recv_empty_and_disconnected() {
    println!("  test_try_recv_empty_and_disconnected...");
    let (tx, rx) = mpsc::channel::<i32>(10);

    // Empty channel
    assert_eq!(rx.try_recv(), Err(TryRecvError::Empty));

    // After sender drops
    drop(tx);
    assert_eq!(rx.try_recv(), Err(TryRecvError::Disconnected));
    println!("  test_try_recv_empty_and_disconnected OK");
}

fn test_capacity_one() {
    println!("  test_capacity_one...");
    let (tx, rx) = mpsc::channel::<&str>(1);

    let sender = spawn(async move {
        tx.send("a").await.unwrap();
        tx.send("b").await.unwrap();
        tx.send("c").await.unwrap();
    });

    let receiver = spawn(async move {
        let mut results = Vec::new();
        while let Some(val) = rx.recv().await {
            results.push(val);
        }
        assert_eq!(results, vec!["a", "b", "c"]);
    });

    std::thread::sleep(Duration::from_millis(200));
    drop(sender);
    drop(receiver);
    println!("  test_capacity_one OK");
}

fn test_async_recv_then_close() {
    println!("  test_async_recv_then_close...");
    let (tx, rx) = mpsc::channel::<i32>(5);

    // Start a task that waits for items
    let receiver_task = spawn(async move {
        let mut count = 0;
        while let Some(val) = rx.recv().await {
            count += 1;
            println!("    received item {} (val={})", count, val);
        }
        println!("    channel closed after {} items", count);
        assert_eq!(count, 3);
    });

    // Send 3 items with delays
    std::thread::sleep(Duration::from_millis(20));
    tx.send_nowait(10).unwrap();
    std::thread::sleep(Duration::from_millis(20));
    tx.send_nowait(20).unwrap();
    std::thread::sleep(Duration::from_millis(20));
    tx.send_nowait(30).unwrap();

    // Close the channel
    drop(tx);
    std::thread::sleep(Duration::from_millis(100));
    drop(receiver_task);
    println!("  test_async_recv_then_close OK");
}

fn test_heavy_producer_consumer() {
    println!("  test_heavy_producer_consumer...");
    let (tx, rx) = mpsc::channel::<u64>(10);
    let num_items = 100u64;

    // 4 producers
    let mut producer_handles = vec![];
    for p in 0..4 {
        let tx = tx.clone();
        let h = spawn(async move {
            for i in 0..num_items {
                tx.send(p * num_items + i).await.unwrap();
            }
        });
        producer_handles.push(h);
    }
    drop(tx);

    // Single consumer
    let consumer = spawn(async move {
        let mut count = 0u64;
        while let Some(_val) = rx.recv().await {
            count += 1;
        }
        let expected_total = 4 * num_items;
        assert_eq!(count, expected_total, "should receive all {} items", expected_total);
    });

    std::thread::sleep(Duration::from_millis(1000));
    for h in producer_handles {
        drop(h);
    }
    drop(consumer);
    println!("  test_heavy_producer_consumer OK");
}
