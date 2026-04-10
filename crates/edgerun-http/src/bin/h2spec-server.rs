//! Minimal HTTP/2 TLS test server for h2spec conformance testing.
//!
//! Usage: `cargo run --bin h2spec-server -- --port 8081`
//! Then run: `h2spec -h 127.0.0.1 -p 8081 -k`

use rcgen::{CertificateParams, DistinguishedName, DnType, KeyPair, PKCS_ECDSA_P256_SHA256};
use rustls::pki_types::{CertificateDer, PrivatePkcs8KeyDer};
use rustls::ServerConfig;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::{io, process, thread};

fn main() {
    let port = parse_args();
    let (cert_der, key_der) = generate_cert();

    let cert = CertificateDer::from(cert_der);
    let key: PrivatePkcs8KeyDer<'static> = PrivatePkcs8KeyDer::from(key_der);

    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert], key.into())
        .unwrap();

    println!("h2spec TLS server listening on 127.0.0.1:{port}");
    println!("Run: h2spec -h 127.0.0.1 -p {port} -k");

    let listener = TcpListener::bind(format!("127.0.0.1:{port}"))
        .unwrap_or_else(|e| {
            eprintln!("Failed to bind: {e}");
            process::exit(1);
        });

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let config = Arc::new(config.clone());
                thread::spawn(move || {
                    if let Err(e) = handle_connection(config, stream) {
                        eprintln!("Connection error: {e}");
                    }
                });
            }
            Err(e) => eprintln!("Accept error: {e}"),
        }
    }
}

fn parse_args() -> u16 {
    let args: Vec<String> = std::env::args().collect();
    for i in 0..args.len() {
        if args[i] == "--port" || args[i] == "-p" {
            if let Some(port_str) = args.get(i + 1) {
                if let Ok(port) = port_str.parse::<u16>() {
                    return port;
                }
            }
        }
    }
    8081
}

fn generate_cert() -> (Vec<u8>, Vec<u8>) {
    let mut params = CertificateParams::new(vec!["127.0.0.1".to_string(), "localhost".to_string()]).unwrap();
    params.distinguished_name = DistinguishedName::new();
    params.distinguished_name.push(DnType::CommonName, "localhost");
    params.distinguished_name.push(DnType::OrganizationName, "h2spec-test");

    let key_pair = KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256).unwrap();
    let cert = params.self_signed(&key_pair).unwrap();

    (cert.der().to_vec(), key_pair.serialize_der())
}

fn handle_connection(config: Arc<ServerConfig>, mut stream: TcpStream) -> io::Result<()> {
    let mut conn = rustls::ServerConnection::new(config).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    // Perform TLS handshake
    while conn.is_handshaking() {
        conn.complete_io(&mut stream)?;
    }

    // Now read/write through conn.reader() and conn.writer()
    // Read HTTP/2 connection preface
    let mut preface = [0u8; 24];
    let mut pos = 0;
    while pos < 24 {
        conn.complete_io(&mut stream)?;
        let n = conn.reader().read(&mut preface[pos..])?;
        if n == 0 {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "TLS closed"));
        }
        pos += n;
    }

    if &preface != b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n" {
        return Ok(());
    }

    // Read client SETTINGS
    let mut hdr = [0u8; 9];
    read_tls(&mut conn, &mut stream, &mut hdr)?;
    let len = ((hdr[0] as u32) << 16) | ((hdr[1] as u32) << 8) | (hdr[2] as u32);
    let mut payload = vec![0u8; len as usize];
    if len > 0 { read_tls(&mut conn, &mut stream, &mut payload)?; }

    // Send our SETTINGS + ACK
    let our_settings = build_frame(0x04, 0x00, 0, &[
        0x00, 0x01, 0x00, 0x00, 0x10, 0x00,
        0x00, 0x05, 0x00, 0x00, 0x40, 0x00,
    ]);
    write_tls(&mut conn, &mut stream, &our_settings)?;
    write_tls(&mut conn, &mut stream, &build_frame(0x04, 0x01, 0, &[]))?;

    // Read client SETTINGS
    let mut hdr2 = [0u8; 9];
    if read_tls(&mut conn, &mut stream, &mut hdr2).is_err() { return Ok(()); }
    let len2 = ((hdr2[0] as u32) << 16) | ((hdr2[1] as u32) << 8) | (hdr2[2] as u32);
    let mut payload2 = vec![0u8; len2 as usize];
    if len2 > 0 { read_tls(&mut conn, &mut stream, &mut payload2)?; }

    let mut hdr3 = [0u8; 9];
    if read_tls(&mut conn, &mut stream, &mut hdr3).is_err() { return Ok(()); }

    // Handle frames
    let mut read_buf = Vec::with_capacity(65536);
    loop {
        let mut hdr = [0u8; 9];
        if read_tls(&mut conn, &mut stream, &mut hdr).is_err() { break; }

        let length = ((hdr[0] as u32) << 16) | ((hdr[1] as u32) << 8) | (hdr[2] as u32);
        let frame_type = hdr[3];
        let flags = hdr[4];
        let stream_id = (((hdr[5] as u32) << 24) | ((hdr[6] as u32) << 16) | ((hdr[7] as u32) << 8) | (hdr[8] as u32)) & 0x7FFFFFFF;

        read_buf.resize(length as usize, 0);
        if length > 0 {
            if read_tls(&mut conn, &mut stream, &mut read_buf).is_err() { break; }
        }

        match frame_type {
            0x00 | 0x01 => {
                if flags & 0x1 != 0 {
                    let mut hb = Vec::new();
                    hb.push(0x88);
                    hb.push(0x4C); hb.extend_from_slice(b"content-type"); hb.push(0x0A); hb.extend_from_slice(b"text/plain");
                    hb.push(0x4E); hb.extend_from_slice(b"content-length"); hb.push(b'0');
                    write_tls(&mut conn, &mut stream, &build_frame(0x01, 0x05, stream_id, &hb))?;
                }
            }
            0x04 => {
                if flags & 0x1 == 0 {
                    write_tls(&mut conn, &mut stream, &build_frame(0x04, 0x01, 0, &[]))?;
                }
            }
            0x06 => {
                if flags & 0x1 == 0 && length == 8 {
                    write_tls(&mut conn, &mut stream, &build_frame(0x06, 0x01, 0, &read_buf[..8]))?;
                }
            }
            0x07 => break,
            _ => {}
        }
    }

    let _ = write_tls(&mut conn, &mut stream, &build_frame(0x07, 0x00, 0, &[0, 0, 0, 0, 0, 0, 0, 0]));
    Ok(())
}

fn read_tls(conn: &mut rustls::ServerConnection, stream: &mut TcpStream, buf: &mut [u8]) -> io::Result<()> {
    let mut pos = 0;
    while pos < buf.len() {
        conn.complete_io(stream)?;
        let n = conn.reader().read(&mut buf[pos..])?;
        if n == 0 {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "TLS closed"));
        }
        pos += n;
    }
    Ok(())
}

fn write_tls(conn: &mut rustls::ServerConnection, stream: &mut TcpStream, data: &[u8]) -> io::Result<()> {
    conn.writer().write_all(data)?;
    conn.complete_io(stream)?;
    Ok(())
}

fn build_frame(ft: u8, fl: u8, sid: u32, payload: &[u8]) -> Vec<u8> {
    let len = payload.len();
    let mut buf = Vec::with_capacity(9 + len);
    buf.push((len >> 16) as u8); buf.push((len >> 8) as u8); buf.push(len as u8);
    buf.push(ft); buf.push(fl);
    buf.extend_from_slice(&sid.to_be_bytes());
    buf.extend_from_slice(payload);
    buf
}
