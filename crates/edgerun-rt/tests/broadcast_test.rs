// Test broadcast channel with the actual runtime.
use edgerun_rt::broadcast::{self, TryRecvError};
use edgerun_rt::{Runtime, spawn};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

fn main() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async {
        test_send_recv_basic();
        test_multiple_receivers();
        test_resubscribe();
        test_try_recv_empty_and_closed();
        test_blocking_recv();
        test_async_recv();
        test_close_wakes_receivers();
        test_multiple_senders();
        test_capacity_one();
        test_receiver_falls_behind();
        println!("All broadcast tests passed!");
    });
}

fn test_send_recv_basic() {
    println!("  test_send_recv_basic...");
    let (tx, mut rx) = broadcast::channel::<i32>(16);
    tx.send(42).unwrap();
    tx.send(100).unwrap();
    drop(tx);

    assert_eq!(rx.try_recv(), Ok(42));
    assert_eq!(rx.try_recv(), Ok(100));
    assert_eq!(rx.try_recv(), Err(TryRecvError::Closed));
    println!("  test_send_recv_basic OK");
}

fn test_multiple_receivers() {
    println!("  test_multiple_receivers...");
    let (tx, mut rx1) = broadcast::channel::<&str>(16);

    // Send before resubscribe so rx2/rx3 miss it
    tx.send("before").unwrap();

    let mut rx2 = rx1.resubscribe();
    let mut rx3 = rx1.resubscribe();

    tx.send("after").unwrap();

    // rx1 sees both messages
    assert_eq!(rx1.try_recv(), Ok("before"));
    assert_eq!(rx1.try_recv(), Ok("after"));

    // rx2, rx3 only see "after" (they resubscribed after "before")
    assert_eq!(rx2.try_recv(), Ok("after"));
    assert_eq!(rx3.try_recv(), Ok("after"));
    println!("  test_multiple_receivers OK");
}

fn test_resubscribe() {
    println!("  test_resubscribe...");
    let (tx, mut rx) = broadcast::channel::<i32>(16);
    tx.send(1).unwrap();
    tx.send(2).unwrap();

    // Resubscribe starts from current end, missing past messages
    let mut rx2 = rx.resubscribe();
    tx.send(3).unwrap();

    // Original receiver sees 1, 2, 3
    assert_eq!(rx.try_recv(), Ok(1));
    assert_eq!(rx.try_recv(), Ok(2));
    assert_eq!(rx.try_recv(), Ok(3));

    // Resubscribed receiver only sees 3
    assert_eq!(rx2.try_recv(), Ok(3));
    println!("  test_resubscribe OK");
}

fn test_try_recv_empty_and_closed() {
    println!("  test_try_recv_empty_and_closed...");
    let (tx, mut rx) = broadcast::channel::<i32>(16);
    assert_eq!(rx.try_recv(), Err(TryRecvError::Empty));
    drop(tx);
    assert_eq!(rx.try_recv(), Err(TryRecvError::Closed));
    println!("  test_try_recv_empty_and_closed OK");
}

fn test_blocking_recv() {
    println!("  test_blocking_recv...");
    let (tx, mut rx) = broadcast::channel::<u64>(16);

    let sender = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(50));
        tx.send(12345).unwrap();
    });

    let val = rx.blocking_recv().unwrap();
    assert_eq!(val, 12345);
    sender.join().unwrap();
    println!("  test_blocking_recv OK");
}

fn test_async_recv() {
    println!("  test_async_recv...");
    let (tx, mut rx) = broadcast::channel::<&str>(16);

    let sender = spawn(async move {
        tx.send("async1").unwrap();
        tx.send("async2").unwrap();
    });

    let receiver = spawn(async move {
        let v1 = rx.recv().await.unwrap();
        assert_eq!(v1, "async1");
        let v2 = rx.recv().await.unwrap();
        assert_eq!(v2, "async2");
    });

    std::thread::sleep(Duration::from_millis(100));
    drop(sender);
    drop(receiver);
    println!("  test_async_recv OK");
}

fn test_close_wakes_receivers() {
    println!("  test_close_wakes_receivers...");
    let (mut tx, mut rx) = broadcast::channel::<i32>(16);
    let mut rx2 = rx.resubscribe();
    let count = Arc::new(AtomicUsize::new(0));

    let c1 = count.clone();
    let r1 = spawn(async move {
        let result = rx.recv().await;
        if result.is_err() {
            c1.fetch_add(1, Ordering::SeqCst);
        }
    });

    let c2 = count.clone();
    let r2 = spawn(async move {
        let result = rx2.recv().await;
        if result.is_err() {
            c2.fetch_add(1, Ordering::SeqCst);
        }
    });

    std::thread::sleep(Duration::from_millis(20));
    tx.close();
    std::thread::sleep(Duration::from_millis(100));

    assert_eq!(count.load(Ordering::SeqCst), 2, "both receivers should get closed");
    drop(r1);
    drop(r2);
    println!("  test_close_wakes_receivers OK");
}

fn test_multiple_senders() {
    println!("  test_multiple_senders...");
    let (tx1, mut rx) = broadcast::channel::<i32>(16);
    let mut tx2 = tx1.clone();

    tx1.send(1).unwrap();
    tx2.send(2).unwrap();

    assert_eq!(rx.try_recv(), Ok(1));
    assert_eq!(rx.try_recv(), Ok(2));
    println!("  test_multiple_senders OK");
}

fn test_capacity_one() {
    println!("  test_capacity_one...");
    let (tx, mut rx) = broadcast::channel::<&str>(1);
    tx.send("a").unwrap();
    tx.send("b").unwrap(); // overwrites "a"
    tx.send("c").unwrap(); // overwrites "b"

    // Receiver fell behind by 2 messages ("a", "b" were dropped).
    assert_eq!(rx.try_recv(), Err(broadcast::TryRecvError::Lagged(2)));
    println!("  test_capacity_one OK");
}

fn test_receiver_falls_behind() {
    println!("  test_receiver_falls_behind...");
    let (tx, mut rx) = broadcast::channel::<i32>(2);
    tx.send(1).unwrap();
    tx.send(2).unwrap();
    tx.send(3).unwrap(); // buffer full, drops 1

    // Receiver starts at index 0, but message 1 was dropped.
    // start = produced(3) - buf.len()(2) = 1, so next(0) < start(1) => lagged by 1.
    assert_eq!(rx.try_recv(), Err(broadcast::TryRecvError::Lagged(1)));
    println!("  test_receiver_falls_behind OK");
}
