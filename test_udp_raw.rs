use std::net::UdpSocket;
use std::time::Duration;

fn main() {
    println!("=== Raw UDP Test ===");
    
    let client = UdpSocket::bind("127.0.0.1:0").expect("client bind");
    let server = UdpSocket::bind("127.0.0.1:0").expect("server bind");
    
    let client_addr = client.local_addr().unwrap();
    let server_addr = server.local_addr().unwrap();
    
    println!("Client: {}, Server: {}", client_addr, server_addr);
    
    client.connect(server_addr).expect("client connect");
    
    let msg = b"Hello QUIC";
    client.send(msg).expect("send");
    println!("Sent: {:?}", std::str::from_utf8(msg));
    
    server.set_nonblocking(true).expect("set nonblocking");
    
    let mut buf = [0u8; 1024];
    match server.recv_from(&mut buf) {
        Ok((n, addr)) => {
            println!("Received {} from {}: {:?}", n, addr, std::str::from_utf8(&buf[..n]));
        }
        Err(e) => {
            println!("No data yet: {}", e);
        }
    }
    
    println!("=== Test Complete ===");
}