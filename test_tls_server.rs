//! Standalone test to diagnose edgerun-tls server hang
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;
use std::time::Duration;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("port").port();
    eprintln!("Listening on port {port}");

    let server_thread = thread::spawn(move || {
        eprintln!("Server: waiting for connection...");
        let (stream, addr) = listener.accept().expect("accept");
        stream.set_read_timeout(Some(Duration::from_secs(5))).ok();
        stream.set_write_timeout(Some(Duration::from_secs(5))).ok();
        eprintln!("Server: accepted from {addr:?}");

        // Read raw bytes to see if client sent anything
        let mut buf = [0u8; 1024];
        match stream.peek(&mut buf) {
            Ok(n) => eprintln!("Server: peeked {n} bytes: {:02x?}", &buf[..n.min(32)]),
            Err(e) => eprintln!("Server: peek error: {e}"),
        }

        let cert = edgerun_tls::certificate_gen::generate_self_signed(&["localhost"]);
        eprintln!("Server: generated cert, cert_der len = {}", cert.cert_der.len());

        eprintln!("Server: starting TLS handshake...");
        match edgerun_tls::server::TlsServerStream::accept(stream, &cert) {
            Ok(mut tls) => {
                eprintln!("Server: TLS handshake OK!");
                let mut buf = [0u8; 1024];
                match tls.read(&mut buf) {
                    Ok(n) => eprintln!("Server: read {n} bytes: {:?}", String::from_utf8_lossy(&buf[..n])),
                    Err(e) => eprintln!("Server: read error: {e:?}"),
                }
            }
            Err(e) => eprintln!("Server: TLS handshake FAILED: {e:?}"),
        }
    });

    // Give server time to start
    thread::sleep(Duration::from_millis(200));

    eprintln!("Client: connecting...");
    let tcp = std::net::TcpStream::connect(format!("127.0.0.1:{port}")).expect("connect");
    eprintln!("Client: connected");

    eprintln!("Client: starting TLS handshake...");
    match edgerun_tls::TlsStream::client(tcp, "localhost") {
        Ok(mut tls) => {
            eprintln!("Client: TLS handshake OK!");
            tls.write_all(b"HELLO").expect("write");
            tls.flush().expect("flush");
            thread::sleep(Duration::from_millis(500));
        }
        Err(e) => eprintln!("Client: TLS handshake FAILED: {e:?}"),
    }

    eprintln!("Client: done, waiting for server thread...");
    let _ = server_thread.join();
    eprintln!("All done!");
}
