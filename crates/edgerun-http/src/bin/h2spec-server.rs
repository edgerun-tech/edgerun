//! Minimal HTTP/2 TLS test server for h2spec conformance testing.
//!
//! Uses edgerun-tls for TLS 1.3 handshake and edgerun-http for HTTP/2 framing.
//!
//! Usage: `cargo run --bin h2spec-server --features tls -- --port 8081`
//! Then run: `h2spec -h 127.0.0.1 -p 8081 -k`

use edgerun_http::http2::frame::{Frame, FrameType};
use edgerun_http::http2::hpack::{Decoder, Encoder};
use edgerun_http::http2::server::{FrameAction, Http2Server};
use edgerun_http::http2::ErrorCode;
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
                    if let Err(e) = handle_connection(stream) {
                        eprintln!("Connection error: {e:?}");
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
        if (args[i] == "--port" || args[i] == "-p") && i + 1 < args.len() {
            if let Ok(port) = args[i + 1].parse::<u16>() {
                return port;
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
    if let Err(e) = read_exact_tls(&mut tls_stream, &mut preface) {
        eprintln!("Failed to read preface: {e}");
        return Ok(());
    }
    if &preface != HTTP2_PREFACE {
        eprintln!("Invalid preface: {:?}", &preface);
        return Ok(());
    }

    // 2. Read client SETTINGS frame
    let settings_frame = match read_frame(&mut tls_stream) {
        Ok((f, _)) => f,
        Err(e) => {
            eprintln!("Failed to read client SETTINGS: {e}");
            return Ok(());
        }
    };
    if settings_frame.frame_type != FrameType::Settings {
        eprintln!("Expected SETTINGS, got {:?}", settings_frame.frame_type);
        return Ok(());
    }

    // 3. Create server and apply client settings
    let mut server = Http2Server::new();
    let settings_typed = match edgerun_http::http2::frame::SettingsFrame::from_frame(&settings_frame) {
        Ok(sf) => sf,
        Err(e) => {
            eprintln!("Failed to parse SETTINGS: {e}");
            write_goaway(&mut tls_stream, 0, ErrorCode::PROTOCOL_ERROR.to_u32(), b"SETTINGS parse error");
            return Ok(());
        }
    };

    match server.apply_client_settings(&settings_typed) {
        FrameAction::WriteFrames(frames) => {
            for frame in &frames {
                write_frame(&mut tls_stream, frame)?;
            }
        }
        FrameAction::Goaway { error_code, debug_data, .. } => {
            write_goaway(&mut tls_stream, 0, error_code, &debug_data);
            return Ok(());
        }
        _ => {}
    }

    // Main frame loop
    let mut encoder = Encoder::new();
    let mut decoder = Decoder::new();
    // Configure decoder with client's header table size setting
    decoder.set_max_table_size(server.client_settings.header_table_size as usize);
    let mut header_block_buf = Vec::new();
    let mut expecting_continuation = false;
    let mut continuation_stream_id = 0u32;

    loop {
        let (frame, raw_type_byte) = match read_frame(&mut tls_stream) {
            Ok(f) => f,
            Err(_) => break,
        };

        // During a CONTINUATION sequence, unknown frame types are a connection error.
        // RFC 7540 §6.2: "Any other frame appearing in the middle of a header block
        // MUST be treated as a connection error of type PROTOCOL_ERROR."
        if expecting_continuation && raw_type_byte >= 0xA {
            write_goaway(
                &mut tls_stream,
                server.last_processed_stream_id,
                ErrorCode::PROTOCOL_ERROR.to_u32(),
                b"Unknown frame type during CONTINUATION sequence",
            );
            break;
        }

        // Handle unknown frame types (RFC 7540 §4.1: MUST ignore outside CONTINUATION)
        if raw_type_byte >= 0xA {
            eprintln!("Ignoring unknown frame type: {raw_type_byte:#x}");
            continue;
        }

        // Check frame size against negotiated max
        let payload_len = frame.payload.len() as u32;
        if payload_len > server.max_frame_size {
            write_goaway(
                &mut tls_stream,
                server.last_processed_stream_id,
                ErrorCode::FRAME_SIZE_ERROR.to_u32(),
                b"Frame too large",
            );
            break;
        }

        // Validate frame semantics
        if let Err(error_code) = frame.validate_semantics() {
            write_goaway(
                &mut tls_stream,
                server.last_processed_stream_id,
                error_code,
                b"Frame semantic violation",
            );
            break;
        }

        // If we're expecting a CONTINUATION, only CONTINUATION frames are allowed.
        // Any other frame type is a connection error (RFC 7540 §6.2).
        if expecting_continuation && frame.frame_type != FrameType::Continuation {
            write_goaway(
                &mut tls_stream,
                server.last_processed_stream_id,
                ErrorCode::PROTOCOL_ERROR.to_u32(),
                b"Expected CONTINUATION frame",
            );
            break;
        }

        let action = match frame.frame_type {
            FrameType::Settings => {
                let action = server.handle_settings(&frame);
                // If settings changed, update decoder
                decoder.set_max_table_size(server.client_settings.header_table_size as usize);
                action
            }
            FrameType::Ping => server.handle_ping(&frame),
            FrameType::WindowUpdate => server.handle_window_update(&frame),
            FrameType::RstStream => server.handle_rst_stream(&frame),
            FrameType::Priority => {
                // PRIORITY frames are always accepted per RFC 7540 §6.3
                let pf = edgerun_http::http2::frame::PriorityFrame::from_frame(&frame)
                    .unwrap_or_else(|_| {
                        if frame.payload.len() >= 5 {
                            let dep_raw = u32::from_be_bytes([
                                frame.payload[0], frame.payload[1], frame.payload[2], frame.payload[3],
                            ]);
                            let exclusive = (dep_raw >> 31) != 0;
                            let stream_dependency = dep_raw & 0x7FFFFFFF;
                            let weight = frame.payload[4].wrapping_add(1);
                            edgerun_http::http2::frame::PriorityFrame::new(
                                frame.stream_id, exclusive, stream_dependency, weight,
                            )
                        } else {
                            edgerun_http::http2::frame::PriorityFrame::new(frame.stream_id, false, 0, 16)
                        }
                    });
                server.handle_priority(&pf)
            }
            FrameType::Headers => server.handle_headers(
                &frame,
                &mut header_block_buf,
                &mut decoder,
                &mut encoder,
                &mut expecting_continuation,
                &mut continuation_stream_id,
            ),
            FrameType::Continuation => server.handle_continuation(
                &frame,
                &mut header_block_buf,
                &mut expecting_continuation,
                &mut continuation_stream_id,
                &mut decoder,
                &mut encoder,
            ),
            FrameType::Data => server.handle_data(&frame, &mut encoder),
            FrameType::Goaway => server.handle_goaway(),
            FrameType::PushPromise => FrameAction::Goaway {
                last_stream_id: server.last_processed_stream_id,
                error_code: ErrorCode::PROTOCOL_ERROR.to_u32(),
                debug_data: b"Client sent PUSH_PROMISE".to_vec(),
            },
        };

        match action {
            FrameAction::None => {}
            FrameAction::WriteFrames(frames) => {
                for f in &frames {
                    if write_frame(&mut tls_stream, f).is_err() {
                        break;
                    }
                }
            }
            FrameAction::Goaway {
                last_stream_id,
                error_code,
                debug_data,
            } => {
                write_goaway(&mut tls_stream, last_stream_id, error_code, &debug_data);
                break;
            }
            FrameAction::CloseConnection => break,
        }
    }

    // Don't send GOAWAY here — if we got here due to read error,
    // the peer already closed their side. Sending more data would
    // trigger RST. If we got here via CloseConnection or client GOAWAY,
    // goaway_sent is already true.

    // Graceful TCP shutdown to avoid RST
    let _ = tls_stream.shutdown(std::net::Shutdown::Write);
    let _ = tls_stream.shutdown(std::net::Shutdown::Both);

    Ok(())
}

fn write_goaway(stream: &mut impl Write, last_stream_id: u32, error_code: u32, debug: &[u8]) {
    let goaway = edgerun_http::http2::frame::GoawayFrame::new(
        last_stream_id,
        error_code,
        debug.to_vec(),
    );
    let _ = write_frame(stream, &goaway.to_frame());
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

fn read_frame(stream: &mut impl Read) -> std::io::Result<(Frame, u8)> {
    let mut hdr = [0u8; 9];
    read_exact_tls(stream, &mut hdr)?;

    let length = ((hdr[0] as u32) << 16) | ((hdr[1] as u32) << 8) | (hdr[2] as u32);
    let raw_type_byte = hdr[3];
    let flags = hdr[4];
    let stream_id = u32::from_be_bytes([hdr[5], hdr[6], hdr[7], hdr[8]]) & 0x7FFFFFFF;

    let mut payload = vec![0u8; length as usize];
    if length > 0 {
        read_exact_tls(stream, &mut payload)?;
    }

    let frame_type = FrameType::from_u8(raw_type_byte).unwrap_or(FrameType::Data);

    Ok((
        Frame {
            frame_type,
            flags,
            stream_id,
            payload,
        },
        raw_type_byte,
    ))
}

fn write_frame(stream: &mut impl Write, frame: &Frame) -> std::io::Result<()> {
    let bytes = frame.to_bytes();
    stream.write_all(&bytes)?;
    stream.flush()
}
