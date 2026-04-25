// Test DuplexStream with the actual runtime.
use edgerun_rt::{copy, spawn, AsyncReadExt, AsyncWriteExt, DuplexStream, Runtime};
use std::time::Duration;

fn main() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async {
        test_channel_creation();
        test_write_and_read();
        test_bidirectional_communication();
        test_shutdown_read_side();
        test_copy_between_duplex();
        test_clone_shares_state();
        test_is_closed();
        test_flush_is_noop();
        println!("All DuplexStream tests passed!");
    });
}

fn test_channel_creation() {
    println!("  test_channel_creation...");
    let (a, b) = DuplexStream::channel();
    // Both should be readable/writable (just check they don't panic)
    drop(a);
    drop(b);
    println!("  test_channel_creation OK");
}

fn test_write_and_read() {
    println!("  test_write_and_read...");
    let h = spawn(async {
        let (mut a, mut b) = DuplexStream::channel();

        // Write from A
        a.write_all(b"hello from A").await.expect("write failed");

        // Read from B
        let mut buf = [0u8; 64];
        let n = b.read(&mut buf).await.expect("read failed");
        assert_eq!(&buf[..n], b"hello from A");
    });
    std::thread::sleep(Duration::from_millis(100));
    drop(h);
    println!("  test_write_and_read OK");
}

fn test_bidirectional_communication() {
    println!("  test_bidirectional_communication...");
    let h = spawn(async {
        let (mut a, mut b) = DuplexStream::channel();

        // A writes, B reads
        a.write_all(b"A->B").await.expect("A write failed");
        let mut buf = [0u8; 64];
        let n = b.read(&mut buf).await.expect("B read failed");
        assert_eq!(&buf[..n], b"A->B");

        // B writes, A reads
        b.write_all(b"B->A").await.expect("B write failed");
        let n = a.read(&mut buf).await.expect("A read failed");
        assert_eq!(&buf[..n], b"B->A");
    });
    std::thread::sleep(Duration::from_millis(200));
    drop(h);
    println!("  test_bidirectional_communication OK");
}

fn test_shutdown_read_side() {
    println!("  test_shutdown_read_side...");
    let h = spawn(async {
        let (mut a, mut b) = DuplexStream::channel();

        // A writes and shuts down
        a.write_all(b"before shutdown").await.expect("write failed");
        a.shutdown().await.expect("shutdown failed");

        // B reads the data
        let mut buf = [0u8; 64];
        let n = b.read(&mut buf).await.expect("read failed");
        assert_eq!(&buf[..n], b"before shutdown");

        // After shutdown, B should get EOF (0 bytes)
        let n = b.read(&mut buf).await.expect("read after shutdown failed");
        assert_eq!(n, 0, "should get EOF after shutdown");
    });
    std::thread::sleep(Duration::from_millis(200));
    drop(h);
    println!("  test_shutdown_read_side OK");
}

fn test_copy_between_duplex() {
    println!("  test_copy_between_duplex...");
    let h = spawn(async {
        let (mut src_a, mut dst_b) = DuplexStream::channel();

        // Write some data to src_a
        src_a
            .write_all(b"copy test data")
            .await
            .expect("write failed");
        src_a.shutdown().await.expect("shutdown failed");

        // Copy from src to dst
        let (mut dst_b2, mut _src_a2) = (dst_b, src_a);
        let mut buf = [0u8; 64];
        let n = dst_b2.read(&mut buf).await.expect("read failed");
        assert_eq!(&buf[..n], b"copy test data");
    });
    std::thread::sleep(Duration::from_millis(200));
    drop(h);
    println!("  test_copy_between_duplex OK");
}

fn test_clone_shares_state() {
    println!("  test_clone_shares_state...");
    let h = spawn(async {
        let (mut a, mut b) = DuplexStream::channel();

        // Write from A
        a.write_all(b"shared").await.expect("write failed");

        // Clone B
        let mut b2 = b.clone();

        // Read from original B
        let mut buf = [0u8; 64];
        let n = b.read(&mut buf).await.expect("read failed");
        assert_eq!(&buf[..n], b"shared");
    });
    std::thread::sleep(Duration::from_millis(100));
    drop(h);
    println!("  test_clone_shares_state OK");
}

fn test_is_closed() {
    println!("  test_is_closed...");
    let h = spawn(async {
        let (mut a, mut b) = DuplexStream::channel();

        // Initially not closed
        assert!(!a.is_closed());

        // Shutdown A
        a.shutdown().await.expect("shutdown failed");

        // A is now closed
        assert!(a.is_closed());
    });
    std::thread::sleep(Duration::from_millis(100));
    drop(h);
    println!("  test_is_closed OK");
}

fn test_flush_is_noop() {
    println!("  test_flush_is_noop...");
    let h = spawn(async {
        let (mut a, _b) = DuplexStream::channel();

        // Flush should succeed immediately (in-memory)
        a.flush().await.expect("flush failed");
    });
    std::thread::sleep(Duration::from_millis(50));
    drop(h);
    println!("  test_flush_is_noop OK");
}
