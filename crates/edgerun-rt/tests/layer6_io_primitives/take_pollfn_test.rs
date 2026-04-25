// Test Take and poll_fn with the actual runtime.
use edgerun_rt::{
    poll_fn, spawn, AsyncReadExt, AsyncWriteExt, Cursor, DuplexStream, Runtime, Take,
};
use std::task::{Context, Poll};
use std::time::Duration;

fn main() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async {
        test_take_exact_bytes();
        test_take_less_than_available();
        test_take_more_than_available();
        test_take_zero_limit();
        test_take_remaining();
        test_take_get_ref();
        test_take_into_inner();
        test_take_chain();
        test_take_duplex_stream();
        test_poll_fn_immediate();
        test_poll_fn_pending_then_ready();
        test_poll_fn_counter();
        test_poll_fn_unpin();
        test_poll_fn_with_spawn();
        println!("All Take/poll_fn tests passed!");
    });
}

fn test_take_exact_bytes() {
    println!("  test_take_exact_bytes...");
    let h = spawn(async {
        let cursor = Cursor::new(b"hello world");
        let mut take = cursor.take(5);
        let mut buf = [0u8; 64];
        let n = take.read(&mut buf).await.expect("read failed");
        assert_eq!(&buf[..n], b"hello");
        assert_eq!(take.remaining(), 0);
    });
    std::thread::sleep(Duration::from_millis(50));
    drop(h);
    println!("  test_take_exact_bytes OK");
}

fn test_take_less_than_available() {
    println!("  test_take_less_than_available...");
    let h = spawn(async {
        let cursor = Cursor::new(b"hello world");
        let mut take = cursor.take(3);
        let mut buf = [0u8; 64];
        let n = take.read(&mut buf).await.expect("read failed");
        assert_eq!(n, 3);
        assert_eq!(&buf[..n], b"hel");
        // After reaching limit, should return 0 (EOF)
        let n = take.read(&mut buf).await.expect("read failed");
        assert_eq!(n, 0);
    });
    std::thread::sleep(Duration::from_millis(50));
    drop(h);
    println!("  test_take_less_than_available OK");
}

fn test_take_more_than_available() {
    println!("  test_take_more_than_available...");
    let h = spawn(async {
        let cursor = Cursor::new(b"hi");
        let mut take = cursor.take(100);
        let mut buf = [0u8; 64];
        let n = take.read(&mut buf).await.expect("read failed");
        assert_eq!(n, 2);
        assert_eq!(&buf[..n], b"hi");
        // Inner EOF — Take should also return 0
        let n = take.read(&mut buf).await.expect("read failed");
        assert_eq!(n, 0);
    });
    std::thread::sleep(Duration::from_millis(50));
    drop(h);
    println!("  test_take_more_than_available OK");
}

fn test_take_zero_limit() {
    println!("  test_take_zero_limit...");
    let h = spawn(async {
        let cursor = Cursor::new(b"hello");
        let mut take = cursor.take(0);
        let mut buf = [0u8; 64];
        let n = take.read(&mut buf).await.expect("read failed");
        assert_eq!(n, 0);
        assert_eq!(take.remaining(), 0);
    });
    std::thread::sleep(Duration::from_millis(50));
    drop(h);
    println!("  test_take_zero_limit OK");
}

fn test_take_remaining() {
    println!("  test_take_remaining...");
    let h = spawn(async {
        let cursor = Cursor::new(b"abcdefgh");
        let mut take = cursor.take(4);
        assert_eq!(take.limit(), 4);
        assert_eq!(take.remaining(), 4);

        let mut buf = [0u8; 2];
        let n = take.read(&mut buf).await.expect("read failed");
        assert_eq!(n, 2);
        assert_eq!(take.remaining(), 2);
    });
    std::thread::sleep(Duration::from_millis(50));
    drop(h);
    println!("  test_take_remaining OK");
}

fn test_take_get_ref() {
    println!("  test_take_get_ref...");
    let h = spawn(async {
        let cursor = Cursor::new(b"test");
        let take = cursor.take(4);
        let _ = take.get_ref();
    });
    std::thread::sleep(Duration::from_millis(50));
    drop(h);
    println!("  test_take_get_ref OK");
}

fn test_take_into_inner() {
    println!("  test_take_into_inner...");
    let h = spawn(async {
        let cursor = Cursor::new(b"into_inner test");
        let take = cursor.take(4);
        let inner = take.into_inner();
        // Verify inner is the original cursor
        assert_eq!(inner.position(), 0);
    });
    std::thread::sleep(Duration::from_millis(50));
    drop(h);
    println!("  test_take_into_inner OK");
}

fn test_take_chain() {
    println!("  test_take_chain...");
    let h = spawn(async {
        let c1 = Cursor::new(b"first");
        let c2 = Cursor::new(b"second");
        let mut chained = c1.take(3).chain(c2.take(3));
        let mut buf = [0u8; 64];

        // First read: from first cursor (3 bytes)
        let n = chained.read(&mut buf).await.expect("read failed");
        assert_eq!(&buf[..n], b"fir");

        // Second read: from second cursor (3 bytes)
        let n = chained.read(&mut buf).await.expect("read failed");
        assert_eq!(&buf[..n], b"sec");
    });
    std::thread::sleep(Duration::from_millis(50));
    drop(h);
    println!("  test_take_chain OK");
}

fn test_take_duplex_stream() {
    println!("  test_take_duplex_stream...");
    let h = spawn(async {
        let (mut a, b) = DuplexStream::channel();
        a.write_all(b"limited data here")
            .await
            .expect("write failed");
        let mut take = b.take(7);
        let mut buf = [0u8; 64];
        let n = take.read(&mut buf).await.expect("read failed");
        assert_eq!(&buf[..n], b"limited");
        assert_eq!(take.remaining(), 0);
    });
    std::thread::sleep(Duration::from_millis(100));
    drop(h);
    println!("  test_take_duplex_stream OK");
}

fn test_poll_fn_immediate() {
    println!("  test_poll_fn_immediate...");
    let h = spawn(async {
        let val = poll_fn(|_cx| Poll::Ready(42)).await;
        assert_eq!(val, 42);
    });
    std::thread::sleep(Duration::from_millis(50));
    drop(h);
    println!("  test_poll_fn_immediate OK");
}

fn test_poll_fn_pending_then_ready() {
    println!("  test_poll_fn_pending_then_ready...");
    let h = spawn(async {
        let mut count = 0;
        let val = poll_fn(|cx| {
            count += 1;
            if count == 1 {
                cx.waker().wake_by_ref();
                Poll::Pending
            } else {
                Poll::Ready(count)
            }
        })
        .await;
        assert_eq!(val, 2);
    });
    std::thread::sleep(Duration::from_millis(100));
    drop(h);
    println!("  test_poll_fn_pending_then_ready OK");
}

fn test_poll_fn_counter() {
    println!("  test_poll_fn_counter...");
    let h = spawn(async {
        let mut i = 0;
        let val = poll_fn(|_cx| {
            i += 1;
            if i < 5 {
                Poll::Pending
            } else {
                Poll::Ready(i)
            }
        })
        .await;
        assert_eq!(val, 5);
    });
    std::thread::sleep(Duration::from_millis(100));
    drop(h);
    println!("  test_poll_fn_counter OK");
}

fn test_poll_fn_unpin() {
    println!("  test_poll_fn_unpin...");
    let h = spawn(async {
        let fut = poll_fn(|_cx| Poll::Ready("hello"));
        // Should be Unpin — can be moved after creation.
        let fut2 = fut;
        let val = fut2.await;
        assert_eq!(val, "hello");
    });
    std::thread::sleep(Duration::from_millis(50));
    drop(h);
    println!("  test_poll_fn_unpin OK");
}

fn test_poll_fn_with_spawn() {
    println!("  test_poll_fn_with_spawn...");
    let h = spawn(async {
        let mut state = "initial";
        let result = poll_fn(|cx| match state {
            "initial" => {
                state = "waiting";
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            "waiting" => {
                state = "done";
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            "done" => Poll::Ready("finished"),
            _ => unreachable!(),
        })
        .await;
        assert_eq!(result, "finished");
    });
    std::thread::sleep(Duration::from_millis(100));
    drop(h);
    println!("  test_poll_fn_with_spawn OK");
}
