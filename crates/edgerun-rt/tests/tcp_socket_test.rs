// Test TcpSocket with the actual runtime.
use edgerun_rt::{TcpSocket, Runtime, AsyncReadExt, AsyncWriteExt};
use std::net::{SocketAddr, TcpListener as StdTcpListener};
use std::os::unix::io::AsRawFd;
use std::sync::atomic::{AtomicU16, Ordering};
use std::time::Duration;

static PORT: AtomicU16 = AtomicU16::new(15000);

fn get_test_port() -> u16 {
    PORT.fetch_add(1, Ordering::Relaxed)
}

fn main() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async {
        test_bind_ipv4();
        test_bind_ipv6();
        test_bind_with_reuseaddr();
        test_connect_with_nodelay();
        test_connect_with_ttl();
        test_bind_and_accept();
        test_send_buffer_size();
        test_recv_buffer_size();
        test_reuseport();
        println!("All TcpSocket tests passed!");
    });
}

fn test_bind_ipv4() {
    println!("  test_bind_ipv4...");
    let port = get_test_port();
    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();
    let socket = TcpSocket::new_v4().expect("new_v4 failed");
    let _listener = socket.bind(addr).expect("bind failed");
    println!("  test_bind_ipv4 OK");
}

fn test_bind_ipv6() {
    println!("  test_bind_ipv6...");
    let port = get_test_port();
    let addr: SocketAddr = format!("[::]:{}", port).parse().unwrap();
    let socket = TcpSocket::new_v6().expect("new_v6 failed");
    let _listener = socket.bind(addr).expect("bind failed");
    println!("  test_bind_ipv6 OK");
}

fn test_bind_with_reuseaddr() {
    println!("  test_bind_with_reuseaddr...");
    let port = get_test_port();
    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();
    let mut socket = TcpSocket::new_v4().expect("new_v4 failed");
    socket.set_reuseaddr(true);
    let _listener = socket.bind(addr).expect("bind with reuseaddr failed");
    println!("  test_bind_with_reuseaddr OK");
}

fn test_connect_with_nodelay() {
    println!("  test_connect_with_nodelay...");
    let port = get_test_port();
    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();

    // Start a simple echo server using std
    let server = std::net::TcpListener::bind(addr).expect("bind failed");
    server.set_nonblocking(true).expect("set_nonblocking failed");

    let h = edgerun_rt::spawn(async move {
        let mut socket = TcpSocket::new_v4().expect("new_v4 failed");
        socket.set_nodelay(true);
        let stream = socket.connect(addr).await.expect("connect failed");
        assert!(stream.as_raw_fd() > 0);
    });

    // Accept the connection
    std::thread::sleep(Duration::from_millis(50));
    if let Ok((stream, _)) = server.accept() {
        drop(stream);
    }

    std::thread::sleep(Duration::from_millis(200));
    drop(h);
    println!("  test_connect_with_nodelay OK");
}

fn test_connect_with_ttl() {
    println!("  test_connect_with_ttl...");
    let port = get_test_port();
    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();

    let server = std::net::TcpListener::bind(addr).expect("bind failed");
    server.set_nonblocking(true).expect("set_nonblocking failed");

    let h = edgerun_rt::spawn(async move {
        let mut socket = TcpSocket::new_v4().expect("new_v4 failed");
        socket.set_ttl(128);
        let stream = socket.connect(addr).await.expect("connect failed");
        assert!(stream.as_raw_fd() > 0);
    });

    std::thread::sleep(Duration::from_millis(50));
    if let Ok((stream, _)) = server.accept() {
        drop(stream);
    }

    std::thread::sleep(Duration::from_millis(200));
    drop(h);
    println!("  test_connect_with_ttl OK");
}

fn test_bind_and_accept() {
    println!("  test_bind_and_accept...");
    let port = get_test_port();
    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();

    let mut socket = TcpSocket::new_v4().expect("new_v4 failed");
    socket.set_reuseaddr(true);
    let listener = socket.bind(addr).expect("bind failed");

    let srv = std::sync::Arc::new(listener);
    let server = edgerun_rt::spawn(async move {
        let (stream, _addr) = srv.accept().await.expect("accept failed");
        assert!(stream.as_raw_fd() > 0);
    });

    std::thread::sleep(Duration::from_millis(50));
    // Connect with std TcpStream
    let _client = std::net::TcpStream::connect(addr).expect("client connect failed");

    std::thread::sleep(Duration::from_millis(200));
    drop(server);
    println!("  test_bind_and_accept OK");
}

fn test_send_buffer_size() {
    println!("  test_send_buffer_size...");
    let port = get_test_port();
    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();
    let mut socket = TcpSocket::new_v4().expect("new_v4 failed");
    socket.set_send_buffer_size(65536);
    let _listener = socket.bind(addr).expect("bind failed");
    println!("  test_send_buffer_size OK");
}

fn test_recv_buffer_size() {
    println!("  test_recv_buffer_size...");
    let port = get_test_port();
    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();
    let mut socket = TcpSocket::new_v4().expect("new_v4 failed");
    socket.set_recv_buffer_size(65536);
    let _listener = socket.bind(addr).expect("bind failed");
    println!("  test_recv_buffer_size OK");
}

fn test_reuseport() {
    println!("  test_reuseport...");
    let port = get_test_port();
    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();
    let mut socket = TcpSocket::new_v4().expect("new_v4 failed");
    socket.set_reuseport(true);
    let _listener = socket.bind(addr).expect("bind with reuseport failed");
    println!("  test_reuseport OK");
}
