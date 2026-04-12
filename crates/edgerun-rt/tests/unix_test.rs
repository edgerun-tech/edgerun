// Test Unix domain sockets with the actual runtime.
use edgerun_rt::{UnixListener, UnixStream, AsyncReadExt, AsyncWriteExt, Runtime, spawn};
use std::os::fd::AsRawFd;
use std::sync::Arc;
use std::time::Duration;

fn make_temp_socket_path() -> std::path::PathBuf {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let id = std::process::id();
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    std::env::temp_dir().join(format!("edgerun_rt_test_{}_{}.sock", id, n))
}

fn cleanup_socket(path: &std::path::Path) {
    let _ = std::fs::remove_file(path);
}

fn main() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async {
        test_bind_and_accept();
        test_echo_server();
        test_split_read_write_halves();
        test_shutdown_write();
        test_multiple_connections();
        test_clone_stream();
        println!("All Unix socket tests passed!");
    });
}

fn test_bind_and_accept() {
    println!("  test_bind_and_accept...");
    let path = make_temp_socket_path();
    let _cleanup = Cleanup(&path);

    let listener = Arc::new(UnixListener::bind(&path).expect("bind failed"));

    let srv = listener.clone();
    let server = spawn(async move {
        let stream = srv.accept().await.expect("accept failed");
        assert!(Arc::strong_count(&stream) >= 1);
    });

    std::thread::sleep(Duration::from_millis(20));
    let _client = std::os::unix::net::UnixStream::connect(&path).expect("client connect");

    std::thread::sleep(Duration::from_millis(200));
    drop(server);
    println!("  test_bind_and_accept OK");
}

fn test_echo_server() {
    println!("  test_echo_server...");
    let path = make_temp_socket_path();
    let _cleanup = Cleanup(&path);

    let listener = Arc::new(UnixListener::bind(&path).expect("bind failed"));

    let srv = listener.clone();
    let server = spawn(async move {
        let stream = srv.accept().await.expect("accept failed");
        let mut stream: Arc<UnixStream> = stream;
        let mut buf = [0u8; 64];
        let n = stream.read(&mut buf).await.expect("read failed");
        stream.write_all(&buf[..n]).await.expect("write_all failed");
    });

    std::thread::sleep(Duration::from_millis(20));

    let client_path = path.clone();
    let client = spawn(async move {
        let std_stream = std::os::unix::net::UnixStream::connect(&client_path).expect("client connect");
        std_stream.set_nonblocking(true).unwrap();
        let fd = unsafe { libc::dup(std_stream.as_raw_fd()) };
        std::mem::forget(std_stream);
        let mut stream = UnixStream::from_fd(fd);
        stream.write_all(b"hello unix").await.expect("write failed");
        let mut buf = [0u8; 64];
        let n = stream.read(&mut buf).await.expect("read failed");
        assert_eq!(&buf[..n], b"hello unix");
    });

    std::thread::sleep(Duration::from_millis(300));
    drop(server);
    drop(client);
    println!("  test_echo_server OK");
}

fn test_split_read_write_halves() {
    println!("  test_split_read_write_halves...");
    let path = make_temp_socket_path();
    let _cleanup = Cleanup(&path);

    let listener = Arc::new(UnixListener::bind(&path).expect("bind failed"));

    let srv = listener.clone();
    let server = spawn(async move {
        let stream = srv.accept().await.expect("accept failed");
        let stream: Arc<UnixStream> = stream;
        let (mut read_half, mut write_half) = stream.split();

        let mut buf = [0u8; 32];
        let n = read_half.read(&mut buf).await.expect("read failed");
        write_half.write_all(&buf[..n]).await.expect("write failed");
    });

    std::thread::sleep(Duration::from_millis(20));

    let client_path = path.clone();
    let client = spawn(async move {
        let std_stream = std::os::unix::net::UnixStream::connect(&client_path).expect("client connect");
        std_stream.set_nonblocking(true).unwrap();
        let fd = unsafe { libc::dup(std_stream.as_raw_fd()) };
        std::mem::forget(std_stream);
        let mut stream = UnixStream::from_fd(fd);
        stream.write_all(b"split test").await.expect("write failed");
        let mut buf = [0u8; 32];
        let n = stream.read(&mut buf).await.expect("read failed");
        assert_eq!(&buf[..n], b"split test");
    });

    std::thread::sleep(Duration::from_millis(200));
    drop(server);
    drop(client);
    println!("  test_split_read_write_halves OK");
}

fn test_shutdown_write() {
    println!("  test_shutdown_write...");
    let path = make_temp_socket_path();
    let _cleanup = Cleanup(&path);

    let listener = Arc::new(UnixListener::bind(&path).expect("bind failed"));

    let srv = listener.clone();
    let server = spawn(async move {
        let stream = srv.accept().await.expect("accept failed");
        let mut stream: Arc<UnixStream> = stream;
        stream.write_all(b"final").await.expect("write failed");
        stream.shutdown_write().expect("shutdown_write failed");
    });

    std::thread::sleep(Duration::from_millis(20));

    let client_path = path.clone();
    let client = spawn(async move {
        let std_stream = std::os::unix::net::UnixStream::connect(&client_path).expect("client connect");
        std_stream.set_nonblocking(true).unwrap();
        let fd = unsafe { libc::dup(std_stream.as_raw_fd()) };
        std::mem::forget(std_stream);
        let mut stream = UnixStream::from_fd(fd);
        let mut buf = [0u8; 64];
        let n = stream.read(&mut buf).await.expect("read failed");
        assert_eq!(&buf[..n], b"final");
        let n = stream.read(&mut buf).await.expect("read after shutdown");
        assert_eq!(n, 0, "should get EOF after server shutdown_write");
    });

    std::thread::sleep(Duration::from_millis(200));
    drop(server);
    drop(client);
    println!("  test_shutdown_write OK");
}

fn test_multiple_connections() {
    println!("  test_multiple_connections...");
    let path = make_temp_socket_path();
    let _cleanup = Cleanup(&path);

    let listener = Arc::new(UnixListener::bind(&path).expect("bind failed"));

    let srv = listener.clone();
    let server = spawn(async move {
        for _ in 0..3 {
            let stream = srv.accept().await.expect("accept failed");
            let mut stream: Arc<UnixStream> = stream;
            let mut buf = [0u8; 32];
            let n = stream.read(&mut buf).await.expect("read failed");
            stream.write_all(&buf[..n]).await.expect("write_all failed");
        }
    });

    std::thread::sleep(Duration::from_millis(20));

    let mut client_handles = vec![];
    for i in 1..=3 {
        let p = path.clone();
        let msg = format!("msg{}", i);
        let h = spawn(async move {
            let std_stream = std::os::unix::net::UnixStream::connect(&p).expect("client connect");
            std_stream.set_nonblocking(true).unwrap();
            let fd = unsafe { libc::dup(std_stream.as_raw_fd()) };
            std::mem::forget(std_stream);
            let mut stream = UnixStream::from_fd(fd);
            stream.write_all(msg.as_bytes()).await.expect("write failed");
            let mut buf = [0u8; 32];
            let n = stream.read(&mut buf).await.expect("read failed");
            assert_eq!(&buf[..n], msg.as_bytes());
        });
        client_handles.push(h);
        std::thread::sleep(Duration::from_millis(20));
    }

    std::thread::sleep(Duration::from_millis(300));
    drop(server);
    for h in client_handles {
        drop(h);
    }
    println!("  test_multiple_connections OK");
}

fn test_clone_stream() {
    println!("  test_clone_stream...");
    let path = make_temp_socket_path();
    let _cleanup = Cleanup(&path);

    let listener = Arc::new(UnixListener::bind(&path).expect("bind failed"));

    let srv = listener.clone();
    let server = spawn(async move {
        let stream = srv.accept().await.expect("accept failed");
        let mut stream: Arc<UnixStream> = stream;
        let mut clone = stream.clone();
        assert!(Arc::strong_count(&stream) >= 2);

        let mut buf = [0u8; 32];
        let n = clone.read(&mut buf).await.expect("read via clone failed");
        stream.write_all(&buf[..n]).await.expect("write back failed");
    });

    std::thread::sleep(Duration::from_millis(20));

    let client_path = path.clone();
    let client = spawn(async move {
        let std_stream = std::os::unix::net::UnixStream::connect(&client_path).expect("client connect");
        std_stream.set_nonblocking(true).unwrap();
        let fd = unsafe { libc::dup(std_stream.as_raw_fd()) };
        std::mem::forget(std_stream);
        let mut stream = UnixStream::from_fd(fd);
        stream.write_all(b"clone").await.expect("write failed");
        let mut buf = [0u8; 32];
        let n = stream.read(&mut buf).await.expect("read failed");
        assert_eq!(&buf[..n], b"clone");
    });

    std::thread::sleep(Duration::from_millis(200));
    drop(server);
    drop(client);
    println!("  test_clone_stream OK");
}

struct Cleanup<'a>(&'a std::path::Path);
impl Drop for Cleanup<'_> {
    fn drop(&mut self) {
        cleanup_socket(self.0);
    }
}
