// Test AsyncUdpSocket with the actual runtime.
use edgerun_rt::{spawn, AsyncUdpSocket, Runtime};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

fn main() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async {
        test_bind_and_local_addr();
        test_send_to_recv_from();
        test_multiple_senders_single_receiver();
        test_clone_shares_fd();
        test_large_datagram();
        test_connected_send_recv();
        test_socket_options();
        println!("All UDP tests passed!");
    });
}

fn find_free_port() -> u16 {
    let socket = std::net::UdpSocket::bind("127.0.0.1:0").expect("bind failed");
    socket.local_addr().unwrap().port()
}

fn test_bind_and_local_addr() {
    println!("  test_bind_and_local_addr...");
    let port = find_free_port();
    let addr = format!("127.0.0.1:{}", port);
    let socket = AsyncUdpSocket::bind(&addr).expect("bind failed");
    let local = socket.local_addr().expect("local_addr failed");
    assert_eq!(local.port(), port);
    assert_eq!(local.ip().to_string(), "127.0.0.1");
    println!("  test_bind_and_local_addr OK");
}

fn test_send_to_recv_from() {
    println!("  test_send_to_recv_from...");
    let port_a = find_free_port();
    let port_b = find_free_port();
    let addr_a = format!("127.0.0.1:{}", port_a);
    let addr_b = format!("127.0.0.1:{}", port_b);
    let target_b: SocketAddr = addr_b.parse().unwrap();

    let socket_a = Arc::new(AsyncUdpSocket::bind(&addr_a).expect("bind failed"));
    let socket_b = Arc::new(AsyncUdpSocket::bind(&addr_b).expect("bind failed"));

    let send_a = socket_a.clone();
    let sender = spawn(async move {
        let n = send_a
            .send_to(b"hello udp", target_b)
            .await
            .expect("send_to failed");
        assert_eq!(n, 9);
        println!("    A sent {} bytes to B", n);
    });

    let recv_b = socket_b.clone();
    let receiver = spawn(async move {
        let mut buf = [0u8; 64];
        let (n, src_addr) = recv_b.recv_from(&mut buf).await.expect("recv_from failed");
        assert_eq!(&buf[..n], b"hello udp");
        assert_eq!(src_addr.ip().to_string(), "127.0.0.1");
        assert_eq!(src_addr.port(), port_a);
        println!("    B received {} bytes from {}", n, src_addr);
    });

    std::thread::sleep(Duration::from_millis(200));
    drop(sender);
    drop(receiver);
    println!("  test_send_to_recv_from OK");
}

fn test_multiple_senders_single_receiver() {
    println!("  test_multiple_senders_single_receiver...");
    let port_rx = find_free_port();
    let addr_rx = format!("127.0.0.1:{}", port_rx);

    let socket_rx = Arc::new(AsyncUdpSocket::bind(&addr_rx).expect("bind failed"));
    let mut sender_handles = vec![];

    for i in 1..=3 {
        let port_tx = find_free_port();
        let addr_tx = format!("127.0.0.1:{}", port_tx);
        let target_rx: SocketAddr = addr_rx.parse().unwrap();
        let msg = format!("msg from sender {}", i);

        let h = spawn(async move {
            let socket = AsyncUdpSocket::bind(&addr_tx).expect("bind failed");
            let n = socket
                .send_to(msg.as_bytes(), target_rx)
                .await
                .expect("send_to failed");
            println!("    sender {} sent {} bytes", i, n);
        });
        sender_handles.push(h);
    }

    let rx = socket_rx.clone();
    let receiver = spawn(async move {
        let mut received = 0;
        let mut buf = [0u8; 64];
        while received < 3 {
            let (n, _src) = rx.recv_from(&mut buf).await.expect("recv_from failed");
            let msg = String::from_utf8_lossy(&buf[..n]);
            println!("    rx received: {}", msg);
            received += 1;
        }
        assert_eq!(received, 3);
    });

    std::thread::sleep(Duration::from_millis(300));
    drop(receiver);
    for h in sender_handles {
        drop(h);
    }
    println!("  test_multiple_senders_single_receiver OK");
}

fn test_clone_shares_fd() {
    println!("  test_clone_shares_fd...");
    let port = find_free_port();
    let addr = format!("127.0.0.1:{}", port);
    let socket1 = Arc::new(AsyncUdpSocket::bind(&addr).expect("bind failed"));
    let socket2 = socket1.clone();

    let port_peer = find_free_port();
    let addr_peer = format!("127.0.0.1:{}", port_peer);
    let target: SocketAddr = addr.parse().unwrap();

    let peer = Arc::new(AsyncUdpSocket::bind(&addr_peer).expect("peer bind failed"));
    let peer2 = peer.clone();
    let sender = spawn(async move {
        peer2
            .send_to(b"clone test", target)
            .await
            .expect("peer send failed");
    });

    let recv = socket2.clone();
    let receiver = spawn(async move {
        let mut buf = [0u8; 64];
        let (n, _src) = recv.recv_from(&mut buf).await.expect("recv_from failed");
        assert_eq!(&buf[..n], b"clone test");
        println!("    received via clone, {} bytes", n);
    });

    std::thread::sleep(Duration::from_millis(200));
    drop(sender);
    drop(receiver);
    drop(socket1);
    drop(socket2);
    drop(peer);
    println!("  test_clone_shares_fd OK");
}

fn test_large_datagram() {
    println!("  test_large_datagram...");
    let port_a = find_free_port();
    let port_b = find_free_port();
    let addr_a = format!("127.0.0.1:{}", port_a);
    let addr_b = format!("127.0.0.1:{}", port_b);
    let target_b: SocketAddr = addr_b.parse().unwrap();

    let socket_a = Arc::new(AsyncUdpSocket::bind(&addr_a).expect("bind failed"));
    let socket_b = Arc::new(AsyncUdpSocket::bind(&addr_b).expect("bind failed"));

    let data = vec![0xCDu8; 4000];
    let send_a = socket_a.clone();
    let sender = spawn(async move {
        let n = send_a
            .send_to(&data, target_b)
            .await
            .expect("send_to failed");
        assert_eq!(n, 4000);
        println!("    sent {} bytes", n);
    });

    let recv_b = socket_b.clone();
    let receiver = spawn(async move {
        let mut buf = [0u8; 8192];
        let (n, _src) = recv_b.recv_from(&mut buf).await.expect("recv_from failed");
        assert_eq!(n, 4000);
        assert_eq!(buf[0], 0xCD);
        assert_eq!(buf[3999], 0xCD);
        println!("    received {} bytes", n);
    });

    std::thread::sleep(Duration::from_millis(200));
    drop(sender);
    drop(receiver);
    println!("  test_large_datagram OK");
}

fn test_connected_send_recv() {
    println!("  test_connected_send_recv...");
    let port_a = find_free_port();
    let port_b = find_free_port();
    let addr_a = format!("127.0.0.1:{}", port_a);
    let addr_b = format!("127.0.0.1:{}", port_b);
    let target_b: SocketAddr = addr_b.parse().unwrap();

    let socket_a = Arc::new(AsyncUdpSocket::bind(&addr_a).expect("bind failed"));
    let socket_b = Arc::new(AsyncUdpSocket::bind(&addr_b).expect("bind failed"));

    // Connect A to B
    socket_a.connect(target_b).expect("connect failed");

    // A sends using send_to (still works even when connected)
    let send_a = socket_a.clone();
    let sender = spawn(async move {
        let n = send_a
            .send_to(b"connected udp", target_b)
            .await
            .expect("send_to failed");
        println!("    A sent {} bytes (connected)", n);
    });

    // B receives
    let recv_b = socket_b.clone();
    let receiver = spawn(async move {
        let mut buf = [0u8; 64];
        let (n, src) = recv_b.recv_from(&mut buf).await.expect("recv_from failed");
        assert_eq!(&buf[..n], b"connected udp");
        assert_eq!(src.port(), port_a);
        println!("    B received {} bytes (connected)", n);
    });

    std::thread::sleep(Duration::from_millis(200));
    drop(sender);
    drop(receiver);
    println!("  test_connected_send_recv OK");
}

fn test_socket_options() {
    println!("  test_socket_options...");
    let port = find_free_port();
    let addr = format!("127.0.0.1:{}", port);
    let socket = AsyncUdpSocket::bind(&addr).expect("bind failed");

    // Test broadcast
    let broadcast = socket.broadcast().expect("broadcast get failed");
    assert!(!broadcast, "broadcast should be false by default");
    socket.set_broadcast(true).expect("broadcast set failed");
    let broadcast = socket.broadcast().expect("broadcast get after failed");
    assert!(broadcast, "broadcast should be true after setting");

    // Test TTL
    let ttl = socket.ttl().expect("ttl get failed");
    assert!(ttl > 0, "ttl should be > 0, got {}", ttl);
    socket.set_ttl(128).expect("ttl set failed");
    let ttl = socket.ttl().expect("ttl get after failed");
    assert_eq!(ttl, 128, "ttl should be 128 after setting");

    // Test peer_addr on unconnected socket
    let peer = socket.peer_addr();
    assert!(peer.is_err(), "peer_addr should fail on unconnected socket");

    // Test local_addr
    let local = socket.local_addr().expect("local_addr failed");
    assert_eq!(local.port(), port);

    println!("  test_socket_options OK");
}
