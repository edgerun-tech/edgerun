use edgerun_rt::Runtime;
use std::net::UdpSocket;

fn main() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    
    rt.block_on(async {
        println!("=== edgerun_rt UDP Test ===");
        
        let client = UdpSocket::bind("127.0.0.1:0").expect("client bind");
        let server = UdpSocket::bind("127.0.0.1:0").expect("server bind");
        
        let client_addr = client.local_addr().unwrap();
        let server_addr = server.local_addr().unwrap();
        
        println!("Client: {}, Server: {}", client_addr, server_addr);
        
        let client = Arc::new(edgerun_rt::AsyncUdpSocket::from_std(client).expect("wrap client"));
        let server = Arc::new(edgerun_rt::AsyncUdpSocket::from_std(server).expect("wrap server"));
        
        let msg = b"Hello QUIC";
        client.send_to(msg, server_addr).await.expect("send");
        println!("Sent");
        
        let mut buf = [0u8; 1024];
        match edgerun_rt::timeout(Duration::from_millis(100), server.recv_from(&mut buf)).await {
            Ok(Ok((n, addr))) => {
                println!("Received {} from {}: {:?}", n, addr, std::str::from_utf8(&buf[..n]));
            }
            Ok(Err(e)) => {
                println!("Recv error: {}", e);
            }
            Err(_) => {
                println!("TIMEOUT - no data received");
            }
        }
        
        println!("=== Test Complete ===");
    });
}