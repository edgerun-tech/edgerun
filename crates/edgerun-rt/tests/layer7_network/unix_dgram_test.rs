// Test UnixDatagram with the actual runtime.
use edgerun_rt::{UnixDatagram, Runtime, AsyncReadExt, AsyncWriteExt};
use std::os::unix::io::AsRawFd;
use std::time::Duration;

fn make_temp_socket_path() -> std::path::PathBuf {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let id = std::process::id();
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    std::env::temp_dir().join(format!("edgerun_rt_dgram_{}_{}.sock", id, n))
}

fn cleanup(path: &std::path::Path) {
    let _ = std::fs::remove_file(path);
}

struct Cleanup<'a>(&'a std::path::Path);
impl Drop for Cleanup<'_> {
    fn drop(&mut self) { cleanup(self.0); }
}

fn main() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async {
        test_bind();
        test_unbound();
        test_send_to_and_recv_from();
        test_connect_send_recv();
        test_async_read_write();
        test_raw_fd();
        test_local_addr();
        println!("All UnixDatagram tests passed!");
    });
}

fn test_bind() {
    println!("  test_bind...");
    let path = make_temp_socket_path();
    let _cleanup = Cleanup(&path.clone());
    let socket = UnixDatagram::bind(&path).expect("bind failed");
    assert!(socket.as_raw_fd() > 0);
    println!("  test_bind OK");
}

fn test_unbound() {
    println!("  test_unbound...");
    let socket = UnixDatagram::unbound().expect("unbound failed");
    assert!(socket.as_raw_fd() > 0);
    println!("  test_unbound OK");
}

fn test_send_to_and_recv_from() {
    println!("  test_send_to_and_recv_from...");
    let path_a = make_temp_socket_path();
    let path_b = make_temp_socket_path();
    let _clean_a = Cleanup(&path_a.clone());
    let _clean_b = Cleanup(&path_b.clone());

    let h = edgerun_rt::spawn(async move {
        let sock_a = UnixDatagram::bind(&path_a).expect("bind a failed");
        let sock_b = UnixDatagram::bind(&path_b).expect("bind b failed");

        // Send from B to A
        let data = b"hello dgram";
        let n = sock_b.poll_send_to(
            &mut std::task::Context::from_waker(&noop_waker()),
            data,
            &path_a,
        );
        assert!(n.is_ready(), "send_to should be ready");
        if let std::task::Poll::Ready(Ok(n)) = n {
            assert_eq!(n, data.len());
        }

        // Wait a bit for data to arrive
        std::thread::sleep(Duration::from_millis(50));

        // Receive on A
        let mut buf = [0u8; 64];
        let n = sock_a.poll_recv(
            &mut std::task::Context::from_waker(&noop_waker()),
            &mut buf,
        );
        if let std::task::Poll::Ready(Ok(n)) = n {
            assert_eq!(&buf[..n], data);
        } else {
            panic!("recv should be ready after send, got {:?}", n);
        }
    });
    std::thread::sleep(Duration::from_millis(300));
    drop(h);
    println!("  test_send_to_and_recv_from OK");
}

fn test_connect_send_recv() {
    println!("  test_connect_send_recv...");
    let path_a = make_temp_socket_path();
    let path_b = make_temp_socket_path();
    let _clean_a = Cleanup(&path_a.clone());
    let _clean_b = Cleanup(&path_b.clone());

    let h = edgerun_rt::spawn(async move {
        let sock_a = UnixDatagram::bind(&path_a).expect("bind a failed");
        let sock_b = UnixDatagram::bind(&path_b).expect("bind b failed");

        // Connect B to A
        sock_b.connect(&path_a).expect("connect failed");

        // Send from B (connected, so no address needed)
        let data = b"connected dgram";
        let n = sock_b.poll_send(
            &mut std::task::Context::from_waker(&noop_waker()),
            data,
        );
        assert!(n.is_ready());

        std::thread::sleep(Duration::from_millis(50));

        // Receive on A
        let mut buf = [0u8; 64];
        let n = sock_a.poll_recv(
            &mut std::task::Context::from_waker(&noop_waker()),
            &mut buf,
        );
        if let std::task::Poll::Ready(Ok(n)) = n {
            assert_eq!(&buf[..n], data);
        } else {
            panic!("recv should be ready, got {:?}", n);
        }
    });
    std::thread::sleep(Duration::from_millis(300));
    drop(h);
    println!("  test_connect_send_recv OK");
}

fn test_async_read_write() {
    println!("  test_async_read_write...");
    let path_a = make_temp_socket_path();
    let path_b = make_temp_socket_path();
    let _clean_a = Cleanup(&path_a.clone());
    let _clean_b = Cleanup(&path_b.clone());

    let h = edgerun_rt::spawn(async move {
        let mut sock_a = UnixDatagram::bind(&path_a).expect("bind a failed");
        let mut sock_b = UnixDatagram::bind(&path_b).expect("bind b failed");

        sock_b.connect(&path_a).expect("connect failed");

        // Use AsyncWrite
        sock_b.write_all(b"async dgram").await.expect("write failed");

        std::thread::sleep(Duration::from_millis(50));

        // Use AsyncRead
        let mut buf = [0u8; 64];
        let n = sock_a.read(&mut buf).await.expect("read failed");
        assert_eq!(&buf[..n], b"async dgram");
    });
    std::thread::sleep(Duration::from_millis(300));
    drop(h);
    println!("  test_async_read_write OK");
}

fn test_raw_fd() {
    println!("  test_raw_fd...");
    let path = make_temp_socket_path();
    let _cleanup = Cleanup(&path.clone());
    let socket = UnixDatagram::bind(&path).expect("bind failed");
    let fd = socket.as_raw_fd();
    assert!(fd > 0);
    println!("  test_raw_fd OK (fd={})", fd);
}

fn test_local_addr() {
    println!("  test_local_addr...");
    let path = make_temp_socket_path();
    let _cleanup = Cleanup(&path.clone());
    let socket = UnixDatagram::bind(&path).expect("bind failed");
    let addr = socket.local_addr().expect("local_addr failed");
    assert!(addr.as_pathname().is_some(), "should have a pathname");
    println!("  test_local_addr OK (addr={:?})", addr.as_pathname());
}

fn noop_waker() -> std::task::Waker {
    static VTABLE: std::task::RawWakerVTable =
        std::task::RawWakerVTable::new(clone_noop, wake_noop, wake_noop, drop_noop);
    const fn clone_noop(_: *const ()) -> std::task::RawWaker {
        std::task::RawWaker::new(std::ptr::null(), &VTABLE)
    }
    const fn wake_noop(_: *const ()) {}
    const fn drop_noop(_: *const ()) {}
    unsafe { std::task::Waker::from_raw(std::task::RawWaker::new(std::ptr::null(), &VTABLE)) }
}
