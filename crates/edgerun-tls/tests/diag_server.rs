//! Diagnostic server: dumps raw TLS bytes received from external clients.
//! Run: cargo test -p edgerun-tls --test diag_server -- --ignored --nocapture
//! Then in another terminal: openssl s_client -connect 127.0.0.1:9876 -tls1_3

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

static SERVER_READY: AtomicBool = AtomicBool::new(false);

/// Read raw bytes from a stream, printing hex dump for first 512 bytes.
fn dump_bytes(label: &str, stream: &mut TcpStream) -> Result<Vec<u8>, String> {
    let mut buf = [0u8; 1024];
    stream.set_read_timeout(Some(Duration::from_secs(3))).ok();
    let n = stream.read(&mut buf).map_err(|e| format!("read error: {}", e))?;
    if n == 0 {
        return Err("EOF".to_string());
    }
    let data = &buf[..n];
    println!("[{}] {} bytes:", label, n);
    for chunk in data.chunks(16) {
        let hex: String = chunk.iter().map(|b| format!("{:02x} ", b)).collect();
        let ascii: String = chunk.iter().map(|&b| if b >= 32 && b < 127 { b as char } else { '.' }).collect();
        println!("  {:04x}  {:48}  |{}|", 0, hex, ascii);
    }
    Ok(data.to_vec())
}

#[test]
#[ignore = "run manually with: openssl s_client -connect 127.0.0.1:9876 -tls1_3"]
fn diagnose_tls_handshake_with_openssl() {
    let listener = TcpListener::bind("127.0.0.1:9876").expect("bind");
    SERVER_READY.store(true, Ordering::SeqCst);
    println!("Diagnostic server listening on 127.0.0.1:9876");
    println!("Run: openssl s_client -connect 127.0.0.1:9876 -tls1_3");

    let (mut stream, addr) = listener.accept().expect("accept");
    println!("\n=== Connection from {:?} ===", addr);
    // Set blocking + no timeout so read_exact works correctly
    let sock = socket2::SockRef::from(&stream);
    sock.set_nonblocking(false).expect("set blocking");
    sock.set_read_timeout(None).ok();
    sock.set_write_timeout(None).ok();

    // Read TLS record header (5 bytes)
    let mut hdr = [0u8; 5];
    // Remove the timeout we set earlier — don't override the socket settings
    match stream.read_exact(&mut hdr) {
        Ok(()) => {
            println!("\nTLS Record Header:");
            println!("  content_type: 0x{:02x} ({})", hdr[0], match hdr[0] {
                20 => "ChangeCipherSpec",
                21 => "Alert",
                22 => "Handshake",
                23 => "ApplicationData",
                _ => "Unknown",
            });
            println!("  version: 0x{:02x}{:02x}", hdr[1], hdr[2]);
            let length = u16::from_be_bytes([hdr[3], hdr[4]]) as usize;
            println!("  length: {}", length);

            // Read handshake fragment
            let mut frag = vec![0u8; length];
            stream.read_exact(&mut frag).ok();
            println!("\nHandshake fragment (first 128 bytes):");
            for chunk in frag[..length.min(128)].chunks(16) {
                let hex: String = chunk.iter().map(|b| format!("{:02x} ", b)).collect();
                println!("  {}", hex);
            }

            // Parse handshake message type and length
            if !frag.is_empty() {
                let hs_type = frag[0];
                let hs_len = u32::from_be_bytes([0, frag[1], frag[2], frag[3]]) as usize;
                println!("\nHandshake message:");
                println!("  type: {} ({})", hs_type, match hs_type {
                    1 => "ClientHello",
                    2 => "ServerHello",
                    8 => "EncryptedExtensions",
                    11 => "Certificate",
                    15 => "CertificateVerify",
                    20 => "Finished",
                    _ => "Unknown",
                });
                println!("  length: {}", hs_len);
                if hs_type == 1 && frag.len() >= 40 {
                    // Parse ClientHello fields
                    let legacy_version = u16::from_be_bytes([frag[4], frag[5]]);
                    println!("  legacy_version: 0x{:04x}", legacy_version);
                    println!("  random: {:02x?}", &frag[6..38]);
                    let session_id_len = frag[38] as usize;
                    println!("  session_id_len: {}", session_id_len);
                    let cs_offset = 39 + session_id_len;
                    if cs_offset + 2 <= frag.len() {
                        let cs_len = u16::from_be_bytes([frag[cs_offset], frag[cs_offset + 1]]) as usize;
                        println!("  cipher_suites_len: {}", cs_len);
                        let mut pos = cs_offset + 2;
                        while pos + 1 < cs_offset + 2 + cs_len {
                            let cs = u16::from_be_bytes([frag[pos], frag[pos + 1]]);
                            println!("    cipher: 0x{:04x}", cs);
                            pos += 2;
                        }
                        let comp_offset = cs_offset + 2 + cs_len;
                        if comp_offset < frag.len() {
                            let comp_len = frag[comp_offset] as usize;
                            println!("  compression_len: {}", comp_len);
                            let ext_offset = comp_offset + 1 + comp_len;
                            if ext_offset + 2 <= frag.len() {
                                let ext_len = u16::from_be_bytes([frag[ext_offset], frag[ext_offset + 1]]) as usize;
                                println!("  extensions_len: {}", ext_len);
                                // Parse extensions
                                let mut ep = ext_offset + 2;
                                while ep + 4 <= ext_offset + 2 + ext_len {
                                    let ext_type = u16::from_be_bytes([frag[ep], frag[ep + 1]]);
                                    let ext_data_len = u16::from_be_bytes([frag[ep + 2], frag[ep + 3]]) as usize;
                                    let ext_name = match ext_type {
                                        0 => "server_name (SNI)",
                                        10 => "supported_groups",
                                        11 => "ec_point_formats",
                                        13 => "signature_algorithms",
                                        16 => "application_layer_protocol_negotiation",
                                        23 => "extended_master_secret",
                                        35 => "session_ticket",
                                        43 => "supported_versions",
                                        45 => "psk_key_exchange_modes",
                                        49 => "post_handshake_auth",
                                        51 => "key_share",
                                        _ => "unknown",
                                    };
                                    println!("    extension {}: {} (len={})", ext_type, ext_name, ext_data_len);
                                    ep += 4 + ext_data_len;
                                }
                            }
                        }
                    }
                }
            }
        }
        Err(e) => {
            println!("Read error: {}", e);
        }
    }

    // Try to use edgerun-tls server
    println!("\n--- Attempting edgerun-tls server handshake ---");
    let cert = edgerun_tls::certificate_gen::generate_self_signed(&["localhost"]).unwrap();
    let result = edgerun_tls::server::TlsServerStream::accept(stream, &cert);
    match result {
        Ok(mut tls) => {
            println!("TLS handshake SUCCESS!");
            let mut buf = [0u8; 64];
            match tls.read(&mut buf) {
                Ok(n) => println!("Read {} bytes: {:?}", n, &buf[..n]),
                Err(e) => println!("Read after handshake error: {:?}", e),
            }
        }
        Err(e) => {
            println!("TLS handshake FAILED: {:?}", e);
        }
    }
}

/// Simple loopback test — just verify our client can talk to our server.
#[test]
fn verify_internal_loopback() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();

    let server_thread = thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let cert = edgerun_tls::certificate_gen::generate_self_signed(&["localhost"]).unwrap();
        let mut tls = edgerun_tls::server::TlsServerStream::accept(stream, &cert)
            .expect("server handshake failed");
        let mut buf = [0u8; 1024];
        let n = tls.read(&mut buf).expect("read failed");
        String::from_utf8_lossy(&buf[..n]).to_string()
    });

    thread::sleep(Duration::from_millis(100));
    let tcp = TcpStream::connect(format!("127.0.0.1:{}", port)).unwrap();
    let mut tls = edgerun_tls::TlsStream::client(tcp, "localhost")
        .expect("client handshake failed");
    tls.write_all(b"HELLO FROM CLIENT").unwrap();
    tls.flush().unwrap();

    let received = server_thread.join().unwrap();
    assert_eq!(received, "HELLO FROM CLIENT");
}
