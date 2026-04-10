//! Minimal HTTP/2 TLS test server for h2spec conformance testing.
//!
//! Uses edgerun-tls for TLS 1.3 handshake and edgerun-http for HTTP/2 framing.
//!
//! Usage: `cargo run --bin h2spec-server -- --port 8081`
//! Then run: `h2spec -h 127.0.0.1 -p 8081 -k`

use edgerun_http::http2::frame::{
    DataFrame, Frame, HeadersFrame, PingFrame, RstStreamFrame, SettingsFrame, WindowUpdateFrame,
};
use edgerun_http::http2::hpack::{Decoder, Encoder};
use edgerun_http::tls::TlsHttp2Server;
use edgerun_tls::certificate_gen::generate_self_signed;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::process;
use std::thread;

const HTTP2_PREFACE: &[u8] = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";

fn main() {
    let port = parse_args();

    println!("h2spec TLS server listening on 127.0.0.1:{port}");
    println!("Run: h2spec -h 127.0.0.1 -p {port} -k");

    let listener = TcpListener::bind(format!("127.0.0.1:{port}")).unwrap_or_else(|e| {
        eprintln!("Failed to bind: {e}");
        process::exit(1);
    });

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                thread::spawn(move || {
                    match handle_connection(stream) {
                        Ok(()) => {},
                        Err(e) => eprintln!("Connection error: {e:?}"),
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

fn handle_connection(tcp_stream: std::net::TcpStream) -> std::io::Result<()> {
    eprintln!("New connection from {:?}", tcp_stream.peer_addr());
    let cert = generate_self_signed(&["127.0.0.1", "localhost"]);

    // TLS handshake
    eprintln!("Starting TLS handshake...");
    let tls_result = TlsHttp2Server::accept(tcp_stream, &cert);
    let mut tls_stream = match tls_result {
        Ok(server) => {
            eprintln!("TLS handshake completed successfully");
            server.into_inner()
        }
        Err(e) => {
            eprintln!("TLS handshake failed: {e:?}");
            return Err(e);
        }
    };

    // 1. Read client preface (24 bytes)
    let mut preface = [0u8; 24];
    read_exact_tls(&mut tls_stream, &mut preface)?;
    if &preface != HTTP2_PREFACE {
        eprintln!("Invalid preface: {:?}", &preface);
        return Ok(());
    }

    // 2. Read client SETTINGS frame
    let client_settings = read_frame(&mut tls_stream)?;
    if client_settings.frame_type != edgerun_http::http2::frame::FrameType::Settings {
        eprintln!("Expected SETTINGS, got {:?}", client_settings.frame_type);
        return Ok(());
    }
    let settings_frame = SettingsFrame::from_frame(&client_settings)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    // Apply client settings (for now we just acknowledge)
    let _settings = edgerun_http::http2::Settings::from_entries(&settings_frame.entries)
        .unwrap_or_default();

    // 3. Send server preface: SETTINGS + ACK of client SETTINGS
    let server_settings = edgerun_http::http2::Settings::new();
    write_frame(
        &mut tls_stream,
        &SettingsFrame::new(server_settings.to_entries()).to_frame(),
    )?;
    write_frame(&mut tls_stream, &SettingsFrame::ack().to_frame())?;

    // Main frame loop
    let mut encoder = Encoder::new();
    let mut decoder = Decoder::new();
    let mut header_block_buf = Vec::new();
    let mut expecting_continuation = false;
    let mut continuation_stream_id = 0;

    loop {
        let frame = match read_frame(&mut tls_stream) {
            Ok(f) => f,
            Err(_) => break,
        };

        match frame.frame_type {
            edgerun_http::http2::frame::FrameType::Settings => {
                let sf = SettingsFrame::from_frame(&frame)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
                if !sf.ack {
                    // Acknowledge
                    write_frame(&mut tls_stream, &SettingsFrame::ack().to_frame())?;
                }
            }
            edgerun_http::http2::frame::FrameType::Ping => {
                let pf = PingFrame::from_frame(&frame)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
                if !pf.ack {
                    // Send PING ACK
                    let ack = PingFrame::ack(pf.data);
                    write_frame(&mut tls_stream, &ack.to_frame())?;
                }
            }
            edgerun_http::http2::frame::FrameType::WindowUpdate => {
                // Handle window update (just ignore for now)
            }
            edgerun_http::http2::frame::FrameType::RstStream => {
                // Stream reset, ignore
            }
            edgerun_http::http2::frame::FrameType::Headers => {
                let hf = HeadersFrame::from_frame(&frame)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

                if hf.end_stream {
                    // Full headers in one frame
                    let headers = decoder
                        .decode(&hf.header_block)
                        .unwrap_or_default();
                    handle_request(
                        &mut tls_stream,
                        hf.stream_id,
                        &headers,
                        None,
                        &mut encoder,
                    )?;
                } else if frame.flags & edgerun_http::http2::frame::flags::HEADERS_END_HEADERS != 0 {
                    // END_HEADERS set but not END_STREAM — headers complete, no data yet
                    let headers = decoder
                        .decode(&hf.header_block)
                        .unwrap_or_default();
                    header_block_buf.clear();
                    header_block_buf.extend_from_slice(&hf.header_block);
                    continuation_stream_id = hf.stream_id;
                    expecting_continuation = false; // headers are complete, waiting for DATA
                } else {
                    // Need CONTINUATION frames
                    header_block_buf.clear();
                    header_block_buf.extend_from_slice(&hf.header_block);
                    continuation_stream_id = hf.stream_id;
                    expecting_continuation = true;
                }
            }
            edgerun_http::http2::frame::FrameType::Continuation => {
                if expecting_continuation {
                    header_block_buf.extend_from_slice(&frame.payload);
                    if frame.flags & edgerun_http::http2::frame::flags::HEADERS_END_HEADERS != 0 {
                        expecting_continuation = false;
                        let headers = decoder
                            .decode(&header_block_buf)
                            .unwrap_or_default();
                        // Headers complete, but we might still need DATA
                        // For h2spec, most requests end with HEADERS only (GET)
                        // We'll wait for DATA with END_STREAM
                    }
                }
            }
            edgerun_http::http2::frame::FrameType::Data => {
                let df = DataFrame::from_frame(&frame)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

                // Send WINDOW_UPDATE to acknowledge data
                let wu = WindowUpdateFrame::new(df.stream_id, df.data.len() as u32);
                write_frame(&mut tls_stream, &wu.to_frame())?;

                if df.end_stream {
                    // Request complete — but we need headers first
                    // For simplicity, send a default response
                    handle_request(
                        &mut tls_stream,
                        df.stream_id,
                        &[],
                        Some(&df.data),
                        &mut encoder,
                    )?;
                }
            }
            edgerun_http::http2::frame::FrameType::Goaway => {
                break;
            }
            _ => {}
        }
    }

    // Send GOAWAY
    let goaway = edgerun_http::http2::frame::GoawayFrame::new(0, 0, Vec::new());
    let _ = write_frame(&mut tls_stream, &goaway.to_frame());
    Ok(())
}

fn handle_request(
    stream: &mut impl Write,
    stream_id: u32,
    _request_headers: &[(Vec<u8>, Vec<u8>)],
    _body: Option<&[u8]>,
    encoder: &mut Encoder,
) -> std::io::Result<()> {
    // Build response headers
    let response_headers = vec![
        (":status", "200"),
        ("content-type", "text/plain"),
        ("content-length", "2"),
    ];

    let header_block =
        encoder.encode(response_headers.iter().map(|(k, v)| (k.as_bytes(), v.as_bytes())));

    // Send HEADERS with END_STREAM
    let hf = HeadersFrame::new(stream_id, header_block, true);
    write_frame(stream, &hf.to_frame())?;

    Ok(())
}

fn read_exact_tls(stream: &mut impl Read, buf: &mut [u8]) -> std::io::Result<()> {
    let mut pos = 0;
    while pos < buf.len() {
        let n = stream.read(&mut buf[pos..])?;
        if n == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "unexpected EOF",
            ));
        }
        pos += n;
    }
    Ok(())
}

fn read_frame(stream: &mut impl Read) -> std::io::Result<Frame> {
    // Read 9-byte header
    let mut hdr = [0u8; 9];
    read_exact_tls(stream, &mut hdr)?;

    let length = ((hdr[0] as u32) << 16) | ((hdr[1] as u32) << 8) | (hdr[2] as u32);
    let frame_type = hdr[3];
    let flags = hdr[4];
    let stream_id = u32::from_be_bytes([hdr[5], hdr[6], hdr[7], hdr[8]]) & 0x7FFFFFFF;

    // Read payload
    let mut payload = vec![0u8; length as usize];
    if length > 0 {
        read_exact_tls(stream, &mut payload)?;
    }

    // Parse frame type
    let ft = edgerun_http::http2::frame::FrameType::from_u8(frame_type)
        .unwrap_or(edgerun_http::http2::frame::FrameType::Data); // fallback, won't match anyway

    Ok(Frame {
        frame_type: ft,
        flags,
        stream_id,
        payload,
    })
}

fn write_frame(stream: &mut impl Write, frame: &Frame) -> std::io::Result<()> {
    let bytes = frame.to_bytes();
    stream.write_all(&bytes)?;
    stream.flush()
}
