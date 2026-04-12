// Test TCP (AsyncTcpStream, AsyncTcpListener) with the actual runtime.
use edgerun_rt::{
    AsyncTcpListener, AsyncTcpStream, AsyncReadExt, AsyncWriteExt,
    Runtime, spawn,
};
use std::sync::Arc;
use std::time::Duration;

fn main() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async {
        test_bind_and_accept();
        test_echo_server();
        test_multiple_connections();
        test_split_read_write_halves();
        test_shutdown_write();
        test_local_and_peer_addr();
        test_connect_to_listening_port();
        test_listener_local_addr();
        test_large_data_transfer();
        test_write_after_shutdown();
        println!("All TCP tests passed!");
    });
}

fn test_bind_and_accept() {
    println!("  test_bind_and_accept...");
    let port = find_free_port();
    let addr = format!("127.0.0.1:{}", port);

    let listener = Arc::new(AsyncTcpListener::bind(&addr).expect("bind failed"));
    let local = listener.local_addr().expect("local_addr failed");
    assert_eq!(local.port(), port);

    // Server accepts one connection
    let srv = listener.clone();
    let server = spawn(async move {
        let (stream, _addr) = srv.accept().await.expect("accept failed");
        drop(stream);
    });

    // Client connects from a separate thread using std TcpStream
    std::thread::sleep(Duration::from_millis(20));
    let client_thread = std::thread::spawn(move || {
        let _stream = std::net::TcpStream::connect(&addr).expect("client connect");
    });

    std::thread::sleep(Duration::from_millis(200));
    drop(server);
    client_thread.join().unwrap();
    println!("  test_bind_and_accept OK");
}

fn test_echo_server() {
    println!("  test_echo_server...");
    let port = find_free_port();
    let addr = format!("127.0.0.1:{}", port);

    let listener = Arc::new(AsyncTcpListener::bind(&addr).expect("bind failed"));

    let srv = listener.clone();
    let server = spawn(async move {
        let (stream, _addr) = srv.accept().await.expect("accept failed");
        let mut stream: Arc<AsyncTcpStream> = stream;
        let mut buf = [0u8; 64];
        let n = stream.read(&mut buf).await.expect("read failed");
        stream.write_all(&buf[..n]).await.expect("write_all failed");
    });

    std::thread::sleep(Duration::from_millis(20));

    let client = spawn(async move {
        let mut stream = std::net::TcpStream::connect(&addr).expect("client connect");
        stream.set_nonblocking(true).unwrap();
        let mut stream: Arc<AsyncTcpStream> = Arc::new(AsyncTcpStream::from_std(stream).unwrap());
        stream.write_all(b"hello echo").await.expect("write failed");

        let mut buf = [0u8; 64];
        let n = stream.read(&mut buf).await.expect("read failed");
        assert_eq!(&buf[..n], b"hello echo");
    });

    std::thread::sleep(Duration::from_millis(200));
    drop(server);
    drop(client);
    println!("  test_echo_server OK");
}

fn test_multiple_connections() {
    println!("  test_multiple_connections...");
    let port = find_free_port();
    let addr = format!("127.0.0.1:{}", port);

    let listener = Arc::new(AsyncTcpListener::bind(&addr).expect("bind failed"));

    let srv = listener.clone();
    let server = spawn(async move {
        for i in 0..3 {
            let (stream, _addr) = srv.accept().await.expect("accept failed");
            let mut stream: Arc<AsyncTcpStream> = stream;
            let mut buf = [0u8; 32];
            let n = stream.read(&mut buf).await.expect("read failed");
            stream.write_all(&buf[..n]).await.expect("write_all failed");
            println!("    server echoed connection {}", i + 1);
        }
    });

    std::thread::sleep(Duration::from_millis(20));

    let mut client_handles = vec![];
    for i in 1..=3 {
        let addr = addr.clone();
        let msg = format!("msg{}", i);
        let h = spawn(async move {
            let mut stream = std::net::TcpStream::connect(&addr).expect("client connect");
            stream.set_nonblocking(true).unwrap();
            let mut stream: Arc<AsyncTcpStream> = Arc::new(AsyncTcpStream::from_std(stream).unwrap());
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

fn test_split_read_write_halves() {
    println!("  test_split_read_write_halves...");
    let port = find_free_port();
    let addr = format!("127.0.0.1:{}", port);

    let listener = Arc::new(AsyncTcpListener::bind(&addr).expect("bind failed"));

    let srv = listener.clone();
    let server = spawn(async move {
        let (stream, _addr) = srv.accept().await.expect("accept failed");
        let stream: Arc<AsyncTcpStream> = stream;
        let (mut read_half, mut write_half) = stream.split();

        let mut buf = [0u8; 32];
        let n = read_half.read(&mut buf).await.expect("read failed");
        write_half.write_all(&buf[..n]).await.expect("write failed");
    });

    std::thread::sleep(Duration::from_millis(20));

    let client = spawn(async move {
        let mut stream = std::net::TcpStream::connect(&addr).expect("client connect");
        stream.set_nonblocking(true).unwrap();
        let mut stream: Arc<AsyncTcpStream> = Arc::new(AsyncTcpStream::from_std(stream).unwrap());
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
    let port = find_free_port();
    let addr = format!("127.0.0.1:{}", port);

    let listener = Arc::new(AsyncTcpListener::bind(&addr).expect("bind failed"));

    let srv = listener.clone();
    let server = spawn(async move {
        let (stream, _addr) = srv.accept().await.expect("accept failed");
        let mut stream: Arc<AsyncTcpStream> = stream;
        stream.write_all(b"final message").await.expect("write failed");
        stream.shutdown_write().expect("shutdown_write failed");
    });

    std::thread::sleep(Duration::from_millis(20));

    let client = spawn(async move {
        let mut stream = std::net::TcpStream::connect(&addr).expect("client connect");
        stream.set_nonblocking(true).unwrap();
        let mut stream: Arc<AsyncTcpStream> = Arc::new(AsyncTcpStream::from_std(stream).unwrap());
        let mut buf = [0u8; 64];
        let n = stream.read(&mut buf).await.expect("read failed");
        assert_eq!(&buf[..n], b"final message");
        // After server shutdown_write, read should return 0 (EOF)
        let n = stream.read(&mut buf).await.expect("read after shutdown");
        assert_eq!(n, 0, "should get EOF after server shutdown_write");
    });

    std::thread::sleep(Duration::from_millis(200));
    drop(server);
    drop(client);
    println!("  test_shutdown_write OK");
}

fn test_local_and_peer_addr() {
    println!("  test_local_and_peer_addr...");
    let port = find_free_port();
    let addr = format!("127.0.0.1:{}", port);

    let listener = Arc::new(AsyncTcpListener::bind(&addr).expect("bind failed"));

    let srv = listener.clone();
    let server = spawn(async move {
        let (stream, _addr) = srv.accept().await.expect("accept failed");
        let stream: Arc<AsyncTcpStream> = stream;

        let peer = stream.peer_addr().expect("peer_addr failed");
        let local = stream.local_addr().expect("local_addr failed");
        assert_eq!(local.port(), port);
        assert_eq!(peer.ip().to_string(), "127.0.0.1");
        println!("    peer={}, local={}", peer, local);
    });

    std::thread::sleep(Duration::from_millis(20));

    let client = spawn(async move {
        let mut stream = std::net::TcpStream::connect(&addr).expect("client connect");
        stream.set_nonblocking(true).unwrap();
        let mut stream: Arc<AsyncTcpStream> = Arc::new(AsyncTcpStream::from_std(stream).unwrap());
        stream.local_addr().expect("client local_addr");
    });

    std::thread::sleep(Duration::from_millis(200));
    drop(server);
    drop(client);
    println!("  test_local_and_peer_addr OK");
}

fn test_connect_to_listening_port() {
    println!("  test_connect_to_listening_port...");
    let port = find_free_port();
    let addr = format!("127.0.0.1:{}", port);

    let listener = Arc::new(AsyncTcpListener::bind(&addr).expect("bind failed"));

    let srv = listener.clone();
    let server = spawn(async move {
        let (stream, _addr) = srv.accept().await.expect("accept failed");
        drop(stream);
    });

    std::thread::sleep(Duration::from_millis(20));

    let client_thread = std::thread::spawn(move || {
        let _stream = std::net::TcpStream::connect(&addr).expect("connect failed");
    });

    std::thread::sleep(Duration::from_millis(200));
    drop(server);
    client_thread.join().unwrap();
    println!("  test_connect_to_listening_port OK");
}

fn test_listener_local_addr() {
    println!("  test_listener_local_addr...");
    let port = find_free_port();
    let addr = format!("127.0.0.1:{}", port);

    let listener = AsyncTcpListener::bind(&addr).expect("bind failed");
    let local = listener.local_addr().expect("local_addr failed");
    assert_eq!(local.port(), port);
    assert_eq!(local.ip().to_string(), "127.0.0.1");
    println!("  test_listener_local_addr OK");
}

fn test_large_data_transfer() {
    println!("  test_large_data_transfer...");
    let port = find_free_port();
    let addr = format!("127.0.0.1:{}", port);
    let data_len = 10000usize;

    let listener = Arc::new(AsyncTcpListener::bind(&addr).expect("bind failed"));

    let srv = listener.clone();
    let server = spawn(async move {
        let (stream, _addr) = srv.accept().await.expect("accept failed");
        let mut stream: Arc<AsyncTcpStream> = stream;
        let mut received = Vec::new();
        let mut buf = [0u8; 1024];
        loop {
            let n = stream.read(&mut buf).await.expect("read failed");
            if n == 0 { break; }
            received.extend_from_slice(&buf[..n]);
            if received.len() >= data_len { break; }
        }
        assert_eq!(received.len(), data_len);
        assert_eq!(received[0], 0xAB);
        assert_eq!(received[9999], 0xAB);
        println!("    server received {} bytes", received.len());
    });

    std::thread::sleep(Duration::from_millis(20));

    let data = vec![0xABu8; data_len];
    let client = spawn(async move {
        let mut stream = std::net::TcpStream::connect(&addr).expect("client connect");
        stream.set_nonblocking(true).unwrap();
        let mut stream: Arc<AsyncTcpStream> = Arc::new(AsyncTcpStream::from_std(stream).unwrap());
        stream.write_all(&data).await.expect("write_all failed");
        drop(stream);
    });

    std::thread::sleep(Duration::from_millis(300));
    drop(server);
    drop(client);
    println!("  test_large_data_transfer OK");
}

fn test_write_after_shutdown() {
    println!("  test_write_after_shutdown...");
    let port = find_free_port();
    let addr = format!("127.0.0.1:{}", port);

    let listener = Arc::new(AsyncTcpListener::bind(&addr).expect("bind failed"));

    let srv = listener.clone();
    let server = spawn(async move {
        let (stream, _addr) = srv.accept().await.expect("accept failed");
        let mut stream: Arc<AsyncTcpStream> = stream;
        stream.shutdown_write().expect("shutdown_write failed");
        let result = stream.write_all(b"test").await;
        assert!(result.is_err(), "write after shutdown_write should fail");
    });

    std::thread::sleep(Duration::from_millis(20));

    let client = spawn(async move {
        let mut stream = std::net::TcpStream::connect(&addr).expect("client connect");
        stream.set_nonblocking(true).unwrap();
        let mut stream: Arc<AsyncTcpStream> = Arc::new(AsyncTcpStream::from_std(stream).unwrap());
        std::thread::sleep(Duration::from_millis(100));
    });

    std::thread::sleep(Duration::from_millis(200));
    drop(server);
    drop(client);
    println!("  test_write_after_shutdown OK");
}

fn find_free_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind failed");
    listener.local_addr().unwrap().port()
}
