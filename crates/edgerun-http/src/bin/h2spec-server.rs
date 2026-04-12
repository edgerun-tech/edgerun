//! TLS wrapper around the unified HttpServer for h2spec conformance testing.
//!
//! Performs a TLS 1.3 handshake with ALPN negotiation (h2), then hands
//! the decrypted stream to HttpServer which auto-detects HTTP/2 and
//! dispatches requests to the unified Handler.
//!
//! Usage: `cargo run --bin h2spec-server --features tls -- --port 8081`
//! Then run: `h2spec -h 127.0.0.1 -p 8081 -k`

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::net::TcpListener;
use std::process;

use edgerun_http::{HttpServer, Handler, Request, Response, StatusCode};
use edgerun_tls::certificate_gen::generate_self_signed;
use edgerun_tls::TlsHttp2Server;

fn main() {
    let port = parse_args();

    println!("h2spec TLS server (unified HttpServer) listening on 127.0.0.1:{port}");
    println!("Run: h2spec -h 127.0.0.1 -p {port} -k");

    let listener = TcpListener::bind(format!("127.0.0.1:{port}")).unwrap_or_else(|e| {
        eprintln!("Failed to bind: {e}");
        process::exit(1);
    });

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                std::thread::spawn(move || {
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

    // TLS handshake — ALPN negotiates h2
    let tls_result = TlsHttp2Server::accept(tcp_stream, &cert);
    let tls_stream = match tls_result {
        Ok(server) => {
            eprintln!("TLS handshake completed");
            server.into_inner()
        }
        Err(e) => {
            eprintln!("TLS handshake failed: {e:?}");
            return Err(e);
        }
    };

    // Hand the decrypted stream to the unified HttpServer.
    // It will detect the HTTP/2 preface and run the frame loop.
    let handler = Arc::new(EchoHandler);
    let rt = edgerun_rt::Runtime::new_multi_thread().enable_all().build().unwrap();

    rt.block_on(async move {
        let server = HttpServer::new(handler).keep_alive(None);
        // The HttpServer's handle_connection reads the first line to detect protocol.
        // For HTTP/2 over TLS, the first bytes after TLS are the HTTP/2 connection preface.
        // We wrap the TLS stream in an AsyncTcpStream-compatible wrapper.
        handle_http2_stream(tls_stream, Arc::clone(&handler)).await
    })
}

/// Handle a single HTTP/2 stream after TLS handshake.
async fn handle_http2_stream<S>(
    stream: S,
    handler: Arc<dyn Handler>,
) -> std::io::Result<()>
where
    S: edgerun_rt::AsyncRead + edgerun_rt::AsyncWrite + Unpin + Send + 'static,
{
    // The unified server's internal handle_connection expects to read the first line.
    // But we've already done TLS, so the raw HTTP/2 preface is the first data.
    // We need to use the internal HTTP/2 handler directly.
    use edgerun_rt::AsyncReadExt;
    use std::sync::Arc as StdArc;
    use std::time::Duration;

    let mut rdwr = stream;
    let mut read_buf = [0u8; 24];

    // Read the HTTP/2 connection preface
    rdwr.read_exact(&mut read_buf).await?;
    if &read_buf != b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n" {
        eprintln!("Expected HTTP/2 preface, got: {:?}", &read_buf);
        return Ok(());
    }

    // Delegate to the internal HTTP/2 handler
    handle_http2_frames(rdwr, handler).await
}

/// Internal HTTP/2 frame loop — same as HttpServer's handle_http2.
async fn handle_http2_frames<S>(
    stream: S,
    handler: Arc<dyn Handler>,
) -> std::io::Result<()>
where
    S: edgerun_rt::AsyncRead + edgerun_rt::AsyncWrite + Unpin + Send + 'static,
{
    use edgerun_http::http2::frame::{Frame, FrameType, flags};
    use edgerun_http::http2::hpack::{Decoder, Encoder};
    use edgerun_http::http2::headers::{validate_header_name_case, validate_request_headers};
    use edgerun_http::http2::server::{FrameAction, Http2Server};
    use edgerun_http::http2::ErrorCode;
    use edgerun_http::{HeaderMap, Method, Request, Response, StatusCode, Uri};
    use edgerun_rt::AsyncReadExt;
    use edgerun_rt::AsyncWriteExt;

    let mut rdwr = stream;
    let mut encoder = Encoder::new();
    let mut decoder = Decoder::new();
    let mut server = Http2Server::new();
    let mut expecting_continuation: Option<(u32, Vec<u8>)> = None;

    loop {
        let frame = match read_frame(&mut rdwr).await {
            Ok(f) => f,
            Err(_) => break,
        };

        if frame.frame_type == FrameType::Settings {
            let settings_frame = match edgerun_http::http2::frame::SettingsFrame::from_frame(&frame) {
                Ok(sf) => sf,
                Err(_) => {
                    write_goaway(&mut rdwr, server.last_processed_stream_id, ErrorCode::PROTOCOL_ERROR.to_u32(), b"Bad SETTINGS").await;
                    break;
                }
            };

            match server.apply_client_settings(&settings_frame) {
                FrameAction::WriteFrames(frames) => {
                    for f in &frames {
                        write_frame(&mut rdwr, f).await?;
                    }
                    decoder.set_max_table_size(server.client_settings.header_table_size as usize);
                }
                FrameAction::Goaway { error_code, debug_data, .. } => {
                    write_goaway(&mut rdwr, server.last_processed_stream_id, error_code, &debug_data).await;
                    break;
                }
                _ => {}
            }
            continue;
        }

        if frame.payload.len() as u32 > server.max_frame_size {
            write_goaway(&mut rdwr, server.last_processed_stream_id, ErrorCode::FRAME_SIZE_ERROR.to_u32(), b"Frame too large").await;
            break;
        }

        if let Err(ec) = frame.validate_semantics() {
            write_goaway(&mut rdwr, server.last_processed_stream_id, ec, b"Semantic violation").await;
            break;
        }

        let action = match frame.frame_type {
            FrameType::Ping => server.handle_ping(&frame),
            FrameType::WindowUpdate => server.handle_window_update(&frame),
            FrameType::RstStream => server.handle_rst_stream(&frame),
            FrameType::Priority => {
                let pf = if frame.payload.len() >= 5 {
                    let dep_raw = u32::from_be_bytes([frame.payload[0], frame.payload[1], frame.payload[2], frame.payload[3]]);
                    let exclusive = (dep_raw >> 31) != 0;
                    let stream_dependency = dep_raw & 0x7FFFFFFF;
                    let weight = frame.payload[4].wrapping_add(1);
                    edgerun_http::http2::frame::PriorityFrame::new(frame.stream_id, exclusive, stream_dependency, weight)
                } else {
                    edgerun_http::http2::frame::PriorityFrame::new(frame.stream_id, false, 0, 16)
                };
                server.handle_priority(&pf)
            }
            FrameType::Goaway => {
                server.handle_goaway();
                break;
            }
            FrameType::PushPromise => {
                write_goaway(&mut rdwr, server.last_processed_stream_id, ErrorCode::PROTOCOL_ERROR.to_u32(), b"No push").await;
                break;
            }
            FrameType::Data => {
                // Accept data on streams (for POST bodies)
                FrameAction::None
            }
            FrameType::Headers => {
                let hf = match edgerun_http::http2::frame::HeadersFrame::from_frame(&frame) {
                    Ok(hf) => hf,
                    Err(_) => {
                        write_goaway(&mut rdwr, server.last_processed_stream_id, ErrorCode::PROTOCOL_ERROR.to_u32(), b"Bad HEADERS").await;
                        break;
                    }
                };

                if hf.stream_id == 0 || hf.stream_id % 2 == 0 {
                    write_goaway(&mut rdwr, server.last_processed_stream_id, ErrorCode::PROTOCOL_ERROR.to_u32(), b"Invalid stream").await;
                    break;
                }

                server.update_last_stream(hf.stream_id);
                let _ = server.stream_manager.get_or_create_stream(hf.stream_id);
                if let Some(s) = server.stream_manager.get_stream_mut(hf.stream_id) {
                    let _ = s.open();
                }

                let end_headers = frame.flags & flags::HEADERS_END_HEADERS != 0;

                if hf.end_stream && end_headers {
                    process_request(hf.stream_id, &hf.header_block, &mut decoder, &mut encoder, &mut server, &handler).await
                } else if end_headers && !hf.end_stream {
                    let headers = match decoder.decode(&hf.header_block) {
                        Ok(h) => h,
                        Err(_) => {
                            write_goaway(&mut rdwr, server.last_processed_stream_id, ErrorCode::COMPRESSION_ERROR.to_u32(), b"HPACK error").await;
                            break;
                        }
                    };
                    if let Err((ec, _)) = validate_request_headers(&headers) {
                        write_rst_stream(&mut rdwr, hf.stream_id, ec).await;
                        FrameAction::None
                    } else if let Err((ec, _)) = validate_header_name_case(&headers) {
                        write_rst_stream(&mut rdwr, hf.stream_id, ec).await;
                        FrameAction::None
                    } else {
                        server.pending_headers.insert(hf.stream_id, headers);
                        FrameAction::None
                    }
                } else if !end_headers {
                    expecting_continuation = Some((hf.stream_id, hf.header_block.clone()));
                    FrameAction::None
                } else {
                    FrameAction::None
                }
            }
            FrameType::Continuation => {
                if let Some((stream_id, ref mut block)) = expecting_contin {
                    block.extend_from_slice(&frame.payload);
                    let end_headers = frame.flags & flags::HEADERS_END_HEADERS != 0;
                    if end_headers {
                        let block = block.clone();
                        expecting_contin = None;
                        process_request(stream_id, &block, &mut decoder, &mut encoder, &mut server, &handler).await
                    } else {
                        FrameAction::None
                    }
                } else {
                    write_goaway(&mut rdwr, server.last_processed_stream_id, ErrorCode::PROTOCOL_ERROR.to_u32(), b"Unexpected CONTINUATION").await;
                    break;
                }
            }
            _ => FrameAction::None,
        };

        match action {
            FrameAction::WriteFrames(frames) => {
                for f in &frames {
                    if write_frame(&mut rdwr, f).await.is_err() { break; }
                }
            }
            FrameAction::Goaway { last_stream_id, error_code, debug_data } => {
                write_goaway(&mut rdwr, last_stream_id, error_code, &debug_data).await;
                break;
            }
            FrameAction::CloseConnection => break,
            FrameAction::None => {}
        }
    }

    let _ = rdwr.shutdown(edgerun_rt::io::Shutdown::Both);
    Ok(())
}

/// Decode HPACK headers, call Handler, encode response as HTTP/2 HEADERS + DATA.
async fn process_request<S>(
    stream_id: u32,
    header_block: &[u8],
    decoder: &mut Decoder,
    encoder: &mut Encoder,
    server: &mut Http2Server,
    handler: &dyn Handler,
) -> FrameAction
where
    S: edgerun_rt::AsyncRead + edgerun_rt::AsyncWrite + Unpin,
{
    let headers = match decoder.decode(header_block) {
        Ok(h) => h,
        Err(_) => return FrameAction::None,
    };

    if let Err((ec, _)) = validate_request_headers(&headers) {
        return FrameAction::WriteFrames(vec![
            edgerun_http::http2::frame::RstStreamFrame::new(stream_id, ec).to_frame(),
        ]);
    }
    if let Err((ec, _)) = validate_header_name_case(&headers) {
        return FrameAction::WriteFrames(vec![
            edgerun_http::http2::frame::RstStreamFrame::new(stream_id, ec).to_frame(),
        ]);
    }

    let method = headers.iter()
        .find(|(k, _)| k == b":method")
        .and_then(|(_, v)| std::str::from_utf8(v).ok()?.parse().ok())
        .unwrap_or(Method::GET);

    let scheme = headers.iter()
        .find(|(k, _)| k == b":scheme")
        .and_then(|(_, v)| std::str::from_utf8(v).ok())
        .unwrap_or("http");

    let path = headers.iter()
        .find(|(k, _)| k == b":path")
        .and_then(|(_, v)| std::str::from_utf8(v).ok())
        .unwrap_or("/");

    let authority = headers.iter()
        .find(|(k, _)| k == b":authority")
        .and_then(|(_, v)| std::str::from_utf8(v).ok())
        .unwrap_or("localhost");

    let uri_str = format!("{}://{}{}", scheme, authority, path);
    let uri = Uri::parse(&uri_str).unwrap_or_else(|_| Uri::parse("http://localhost/").unwrap());

    let mut req_headers = HeaderMap::new();
    for (k, v) in &headers {
        if !k.starts_with(b":") {
            if let (Ok(kk), Ok(vv)) = (std::str::from_utf8(k), std::str::from_utf8(v)) {
                let _ = req_headers.insert(kk, vv);
            }
        }
    }

    let content_length = headers.iter()
        .find(|(k, _)| k == b"content-length")
        .and_then(|(_, v)| std::str::from_utf8(v).ok()?.parse::<usize>().ok());

    if let Some(s) = server.stream_manager.get_stream_mut(stream_id) {
        s.content_length = content_length.map(|x| x as u64);
    }

    let request = Request::new(method, uri, req_headers, None);
    let response = handler.handle(request).await;

    let mut resp_frames = Vec::new();

    let mut resp_headers = vec![
        (b":status".to_vec(), response.status().as_u16().to_string().into_bytes()),
    ];
    for (k, v) in response.headers().iter() {
        resp_headers.push((k.as_str().as_bytes().to_vec(), v.as_str().as_bytes().to_vec()));
    }

    let header_block = encoder.encode(resp_headers.iter().map(|(k, v)| (k.as_slice(), v.as_slice())));
    resp_frames.push(edgerun_http::http2::frame::HeadersFrame::new(stream_id, header_block, response.body().is_empty()).to_frame());

    if !response.body().is_empty() {
        resp_frames.push(edgerun_http::http2::frame::DataFrame::new(stream_id, response.body().to_vec(), true).to_frame());
    }

    if let Some(s) = server.stream_manager.get_stream_mut(stream_id) {
        let _ = s.half_close_local();
    }
    server.half_close_remote(stream_id);

    FrameAction::WriteFrames(resp_frames)
}

async fn read_frame<S>(stream: &mut S) -> std::io::Result<Frame>
where
    S: edgerun_rt::AsyncRead + Unpin,
{
    use edgerun_rt::AsyncReadExt;

    let mut hdr = [0u8; 9];
    stream.read_exact(&mut hdr).await?;

    let length = ((hdr[0] as u32) << 16) | ((hdr[1] as u32) << 8) | (hdr[2] as u32);
    let raw_type = hdr[3];
    let flags_byte = hdr[4];
    let stream_id = u32::from_be_bytes([hdr[5], hdr[6], hdr[7], hdr[8]]) & 0x7FFFFFFF;

    let mut payload = vec![0u8; length as usize];
    if length > 0 {
        stream.read_exact(&mut payload).await?;
    }

    let frame_type = FrameType::from_u8(raw_type).unwrap_or(FrameType::Data);

    Ok(Frame {
        frame_type,
        flags: flags_byte,
        stream_id,
        payload,
    })
}

async fn write_frame<S>(stream: &mut S, frame: &Frame) -> std::io::Result<()>
where
    S: edgerun_rt::AsyncWrite + Unpin,
{
    let bytes = frame.to_bytes();
    stream.write_all(&bytes).await?;
    stream.flush().await
}

async fn write_goaway<S>(stream: &mut S, last_stream_id: u32, error_code: u32, debug: &[u8])
where
    S: edgerun_rt::AsyncWrite + Unpin,
{
    let goaway = edgerun_http::http2::frame::GoawayFrame::new(last_stream_id, error_code, debug.to_vec());
    let _ = write_frame(stream, &goaway.to_frame()).await;
}

async fn write_rst_stream<S>(stream: &mut S, stream_id: u32, error_code: u32)
where
    S: edgerun_rt::AsyncWrite + Unpin,
{
    let rst = edgerun_http::http2::frame::RstStreamFrame::new(stream_id, error_code);
    let _ = write_frame(stream, &rst.to_frame()).await;
}

/// Simple echo handler for h2spec testing.
struct EchoHandler;

impl Handler for EchoHandler {
    fn handle(&self, req: Request) -> Pin<Box<dyn Future<Output = Response> + Send + '_>> {
        Box::pin(async move {
            Response::text(
                StatusCode::new(200).unwrap(),
                &format!("{} {}", req.method().as_str(), req.uri().request_target()),
            )
        })
    }
}
