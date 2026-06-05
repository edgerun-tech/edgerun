//! Async HTTP/2 client with stream multiplexing.
//!
//! Provides [`AsyncClient`] for making HTTP/2 requests over a single
//! persistent TCP connection. Multiple requests are multiplexed concurrently
//! as independent HTTP/2 streams.

use crate::http::runtime::time::{Duration, Instant};
use alloc::collections::BTreeMap as HashMap;
use alloc::format;
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;

use crate::http::runtime::{
    AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, JoinHandle, mpsc, select, sleep, spawn,
};
use crate::rt::channel as response_channel;

use crate::http::http2::flow_control::FlowControlManager;
use crate::http::http2::frame::{
    DataFrame, Frame, FrameType, GoawayFrame, HeadersFrame, PingFrame, RstStreamFrame,
    SettingsFrame, WindowUpdateFrame, flags,
};
use crate::http::http2::hpack::HpackContext;
use crate::http::http2::settings::Settings;
use crate::http::http2::stream::{StreamManager, StreamState};
use crate::http::http2::{CONNECTION_PREFACE, ErrorCode, Http2Error, Result};
use crate::http::{HeaderMap, StatusCode};

/// Maximum body size per stream before we error (100 MB).
const MAX_BODY_SIZE: usize = 100 * 1024 * 1024;

/// Idle timeout — if no frames received for this duration, send a PING.
const IDLE_TIMEOUT: Duration = Duration::from_secs(30);

/// PING timeout — if no response after this duration, kill the connection.
const PING_TIMEOUT: Duration = Duration::from_secs(10);

// ===========================================================================
// Public types
// ===========================================================================

/// HTTP/2 response received from the server.
#[derive(Debug)]
pub struct HttpResponse {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Vec<u8>,
}

/// Pending request awaiting response.
pub struct PendingRequest {
    stream_id: u32,
    response_rx: response_channel::Receiver<Result<HttpResponse>>,
    body_rx: mpsc::Receiver<Vec<u8>>,
}

impl PendingRequest {
    /// Wait for the response headers. Returns status + headers.
    pub async fn await_response(self) -> Result<(StatusCode, HeaderMap)> {
        let resp = self.response_rx.await.map_err(|_| {
            Http2Error::Io(crate::http::runtime::io::Error::new(
                crate::http::runtime::io::ErrorKind::ConnectionReset,
                "response channel closed",
            ))
        })??;
        Ok((resp.status, resp.headers))
    }

    /// Collect the full response body into a Vec.
    pub async fn collect_body(self) -> Result<Vec<u8>> {
        let mut body = Vec::new();
        let mut total = 0usize;
        let mut rx = self.body_rx;
        while let Some(chunk) = rx.recv().await {
            total += chunk.len();
            if total > MAX_BODY_SIZE {
                return Err(Http2Error::FlowControl("response body too large".into()));
            }
            body.extend_from_slice(&chunk);
        }
        Ok(body)
    }

    /// Convenience: await response headers + collect body in one call.
    pub async fn into_full_response(self) -> Result<HttpResponse> {
        let Self {
            response_rx,
            body_rx,
            ..
        } = self;
        let resp = response_rx.await.map_err(|_| {
            Http2Error::Io(crate::http::runtime::io::Error::new(
                crate::http::runtime::io::ErrorKind::ConnectionReset,
                "response channel closed",
            ))
        })??;
        let mut body = Vec::new();
        let mut total = 0usize;
        let mut rx = body_rx;
        while let Some(chunk) = rx.recv().await {
            total += chunk.len();
            if total > MAX_BODY_SIZE {
                return Err(Http2Error::FlowControl("response body too large".into()));
            }
            body.extend_from_slice(&chunk);
        }
        Ok(HttpResponse {
            status: resp.status,
            headers: resp.headers,
            body,
        })
    }
}

// ===========================================================================
// AsyncClient
// ===========================================================================

/// Async HTTP/2 client over an established read/write stream.
///
/// The client spawns a background connection task that handles frame I/O,
/// HPACK encoding/decoding, flow control, and stream multiplexing.
///
/// # Example
/// ```text
/// let client = AsyncClient::new(tls_stream).await?;
/// let mut headers = HeaderMap::new();
/// headers.insert(":method", "GET")?;
/// headers.insert(":scheme", "https")?;
/// headers.insert(":authority", "example.com")?;
/// headers.insert(":path", "/api/data")?;
/// let pending = client.request(&headers, None).await?;
/// let resp = pending.into_full_response().await?;
/// ```
pub struct AsyncClient {
    frame_tx: mpsc::Sender<OutgoingFrame>,
    streams: Arc<crate::http::runtime::sync::Mutex<StreamStateInner>>,
    _task: JoinHandle<()>,
    next_stream_id: u32,
    /// Max frame size for outgoing DATA frames.
    max_frame_size: u32,
}

impl AsyncClient {
    /// Create a new HTTP/2 client over the given async read/write stream.
    pub async fn new<S>(mut stream: S) -> Result<Self>
    where
        S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    {
        // Send client connection preface + SETTINGS
        stream
            .write_all(CONNECTION_PREFACE)
            .await
            .map_err(crate::http::runtime::bare_io)?;

        let client_settings = Settings::new();
        let settings_frame = SettingsFrame::new(client_settings.to_entries());
        stream
            .write_all(&settings_frame.to_frame().to_bytes())
            .await
            .map_err(crate::http::runtime::bare_io)?;
        stream
            .flush()
            .await
            .map_err(crate::http::runtime::bare_io)?;

        // Read server SETTINGS frame per RFC 9113 §3.4
        let (server_settings_frame, _) =
            read_frame_async(&mut stream, client_settings.max_frame_size).await?;
        if server_settings_frame.frame_type != FrameType::Settings {
            return Err(Http2Error::ProtocolViolation(
                "Expected server SETTINGS frame".into(),
            ));
        }
        let server_settings =
            Settings::from_entries(&SettingsFrame::from_frame(&server_settings_frame)?.entries)?;
        let max_frame_size = server_settings.max_frame_size;

        // Send SETTINGS ACK
        let ack_frame = SettingsFrame::ack();
        stream
            .write_all(&ack_frame.to_frame().to_bytes())
            .await
            .map_err(crate::http::runtime::bare_io)?;
        stream
            .flush()
            .await
            .map_err(crate::http::runtime::bare_io)?;

        let (frame_tx, frame_rx) = mpsc::channel::<OutgoingFrame>(64);
        let streams = Arc::new(crate::http::runtime::sync::Mutex::new(
            StreamStateInner::new(client_settings, server_settings),
        ));

        let task = spawn(connection_task(stream, frame_rx, Arc::clone(&streams)));

        Ok(AsyncClient {
            frame_tx,
            streams,
            _task: task,
            next_stream_id: 1,
            max_frame_size,
        })
    }

    /// Send a request and return a [`PendingRequest`] that resolves to the response.
    ///
    /// If `body` is `Some`, the body is sent as DATA frame(s) after HEADERS.
    /// For streaming uploads, use [`Self::request_stream`] instead.
    pub async fn request(
        &self,
        headers: &[(Vec<u8>, Vec<u8>)],
        body: Option<Vec<u8>>,
    ) -> Result<PendingRequest> {
        let stream_id = {
            let mut state = self.streams.lock().unwrap();
            let id = state.next_stream_id();
            state.create_stream(id)?;
            id
        };

        let (response_tx, response_rx) = response_channel::channel::<Result<HttpResponse>>();
        let (body_tx, body_rx) = mpsc::channel::<Vec<u8>>(64);

        {
            let mut state = self.streams.lock().unwrap();
            state.register_stream_callbacks(stream_id, response_tx, body_tx);
        }

        // END_STREAM = true when body is empty (not when body is None - that means different thing)
        let end_stream = body.as_ref().is_none_or(|b| b.is_empty());
        let msg = OutgoingFrame::SendHeaders {
            stream_id,
            headers: headers.to_vec(),
            end_stream,
        };
        self.frame_tx.send(msg).await.map_err(|_| {
            Http2Error::Io(crate::http::runtime::io::Error::new(
                crate::http::runtime::io::ErrorKind::BrokenPipe,
                "connection closed",
            ))
        })?;

        if let Some(body_data) = body {
            self._send_data_frames(stream_id, body_data).await?;
        } else if !end_stream {
            let msg = OutgoingFrame::SendData {
                stream_id,
                data: Vec::new(),
                end_stream: true,
            };
            self.frame_tx.send(msg).await.map_err(|_| {
                Http2Error::Io(crate::http::runtime::io::Error::new(
                    crate::http::runtime::io::ErrorKind::BrokenPipe,
                    "connection closed",
                ))
            })?;
        }

        Ok(PendingRequest {
            stream_id,
            response_rx,
            body_rx,
        })
    }

    /// Send a request with a streaming upload body.
    ///
    /// The caller provides an `mpsc::Receiver` of body chunks. After the last
    /// chunk is sent, the caller should drop the sender to signal end-of-stream.
    ///
    /// # Example
    /// ```text
    /// let (body_tx, body_rx) = mpsc::channel(8);
    /// let pending = client.request_stream(&headers, body_rx).await?;
    /// // Send body chunks concurrently:
    /// body_tx.send(chunk1).await?;
    /// body_tx.send(chunk2).await?;
    /// drop(body_tx); // signals end-of-stream
    /// let resp = pending.into_full_response().await?;
    /// ```
    pub async fn request_stream(
        &self,
        headers: &[(Vec<u8>, Vec<u8>)],
        body_rx: mpsc::Receiver<Vec<u8>>,
    ) -> Result<PendingRequest> {
        let stream_id = {
            let mut state = self.streams.lock().unwrap();
            let id = state.next_stream_id();
            state.create_stream(id)?;
            id
        };

        let (response_tx, response_rx) = response_channel::channel::<Result<HttpResponse>>();
        let (resp_body_tx, resp_body_rx) = mpsc::channel::<Vec<u8>>(16);

        {
            let mut state = self.streams.lock().unwrap();
            state.register_stream_callbacks(stream_id, response_tx, resp_body_tx);
        }

        // Send HEADERS without END_STREAM
        let msg = OutgoingFrame::SendHeaders {
            stream_id,
            headers: headers.to_vec(),
            end_stream: false,
        };
        self.frame_tx.send(msg).await.map_err(|_| {
            Http2Error::Io(crate::http::runtime::io::Error::new(
                crate::http::runtime::io::ErrorKind::BrokenPipe,
                "connection closed",
            ))
        })?;

        // Spawn a task to forward body chunks to the connection task
        let frame_tx = self.frame_tx.clone();
        let max_frame = self.max_frame_size as usize;
        spawn(async move {
            let mut rx = body_rx;
            while let Some(chunk) = rx.recv().await {
                // Split large chunks to fit max frame size.
                // END_STREAM is NOT set here — more chunks may follow.
                let mut offset = 0;
                while offset < chunk.len() {
                    let end = (offset + max_frame).min(chunk.len());
                    let msg = OutgoingFrame::SendData {
                        stream_id,
                        data: chunk[offset..end].to_vec(),
                        end_stream: false, // only set after rx is exhausted
                    };
                    if frame_tx.send(msg).await.is_err() {
                        return;
                    }
                    offset = end;
                }
            }
            // body_rx sender dropped — send final empty DATA with END_STREAM
            let msg = OutgoingFrame::SendData {
                stream_id,
                data: Vec::new(),
                end_stream: true,
            };
            let _ = frame_tx.send(msg).await;
        });

        Ok(PendingRequest {
            stream_id,
            response_rx,
            body_rx: resp_body_rx,
        })
    }

    /// Internal: split body into DATA frames and send them.
    async fn _send_data_frames(&self, stream_id: u32, body_data: Vec<u8>) -> Result<()> {
        let max_frame = self.max_frame_size as usize;
        let mut offset = 0;
        while offset < body_data.len() {
            let chunk_end = (offset + max_frame).min(body_data.len());
            let chunk = body_data[offset..chunk_end].to_vec();
            let is_last = chunk_end >= body_data.len();
            let msg = OutgoingFrame::SendData {
                stream_id,
                data: chunk,
                end_stream: is_last,
            };
            self.frame_tx.send(msg).await.map_err(|_| {
                Http2Error::Io(crate::http::runtime::io::Error::new(
                    crate::http::runtime::io::ErrorKind::BrokenPipe,
                    "connection closed",
                ))
            })?;
            offset = chunk_end;
        }
        Ok(())
    }

    /// Send a PING to measure connection latency.
    pub async fn ping(&self) -> Result<u64> {
        let (tx, rx) = response_channel::channel();
        let msg = OutgoingFrame::Ping { reply_tx: tx };
        self.frame_tx.send(msg).await.map_err(|_| {
            Http2Error::Io(crate::http::runtime::io::Error::new(
                crate::http::runtime::io::ErrorKind::BrokenPipe,
                "connection closed",
            ))
        })?;
        rx.await.map_err(|_| {
            Http2Error::Io(crate::http::runtime::io::Error::new(
                crate::http::runtime::io::ErrorKind::ConnectionReset,
                "ping channel closed",
            ))
        })?
    }

    /// Gracefully close the connection.
    pub async fn close(&self) -> Result<()> {
        let msg = OutgoingFrame::Goaway {
            error_code: ErrorCode::NO_ERROR.to_u32(),
            debug_data: Vec::new(),
        };
        let _ = self.frame_tx.send(msg).await;
        Ok(())
    }
}

// ===========================================================================
// Internal types
// =========================================================================//

enum OutgoingFrame {
    SendHeaders {
        stream_id: u32,
        headers: Vec<(Vec<u8>, Vec<u8>)>,
        end_stream: bool,
    },
    SendData {
        stream_id: u32,
        data: Vec<u8>,
        end_stream: bool,
    },
    Ping {
        reply_tx: response_channel::Sender<Result<u64>>,
    },
    Goaway {
        error_code: u32,
        debug_data: Vec<u8>,
    },
}

enum ConnectionEvent {
    IncomingFrame(Frame),
    OutgoingMessage(OutgoingFrame),
    KeepaliveTick,
}

struct StreamStateInner {
    hpack: HpackContext,
    flow: FlowControlManager,
    streams: StreamManager,
    client_settings: Settings,
    remote_settings: Settings,
    max_frame_size: u32,
    stream_callbacks: HashMap<u32, StreamCallbacks>,
    next_stream: u32,
}

struct StreamCallbacks {
    response_tx: Option<response_channel::Sender<Result<HttpResponse>>>,
    body_tx: Option<mpsc::Sender<Vec<u8>>>,
    got_response_headers: bool,
}

impl StreamStateInner {
    fn new(client_settings: Settings, remote_settings: Settings) -> Self {
        let max_frame_size = remote_settings.max_frame_size;
        StreamStateInner {
            hpack: HpackContext::new(),
            flow: FlowControlManager::new(remote_settings.initial_window_size),
            streams: StreamManager::new(remote_settings.initial_window_size),
            client_settings,
            remote_settings,
            max_frame_size,
            stream_callbacks: HashMap::new(),
            next_stream: 1,
        }
    }

    fn next_stream_id(&mut self) -> u32 {
        let id = self.next_stream;
        self.next_stream += 2;
        id
    }

    fn create_stream(&mut self, id: u32) -> Result<()> {
        self.streams.create_client_stream()?;
        if let Some(s) = self.streams.get_stream_mut(id) {
            s.open()?;
        }
        Ok(())
    }

    fn register_stream_callbacks(
        &mut self,
        stream_id: u32,
        response_tx: response_channel::Sender<Result<HttpResponse>>,
        body_tx: mpsc::Sender<Vec<u8>>,
    ) {
        self.stream_callbacks.insert(
            stream_id,
            StreamCallbacks {
                response_tx: Some(response_tx),
                body_tx: Some(body_tx),
                got_response_headers: false,
            },
        );
    }
}

// ===========================================================================
// Background connection task — event-driven with keepalive timer
// ===========================================================================

async fn connection_task<S>(
    mut stream: S,
    mut frame_rx: mpsc::Receiver<OutgoingFrame>,
    state: Arc<crate::http::runtime::sync::Mutex<StreamStateInner>>,
) where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let max_frame_size = {
        let s = state.lock().unwrap();
        s.remote_settings.max_frame_size
    };

    let mut continuation_state: Option<(u32, Vec<u8>, bool)> = None;
    let mut last_activity = Instant::now();
    // Track keepalive PING: (deadline, sent_ping_id)
    let mut ping_deadline: Option<(Instant, u64)> = None;

    loop {
        // Decide which futures to include in select
        let next_keepalive = last_activity + IDLE_TIMEOUT;
        let has_pending_ping = ping_deadline.is_some();

        let event = if has_pending_ping {
            // PING in flight — only wait for incoming frames or client messages
            // (PING timeout checked after each event)
            select!(
                async {
                    match read_frame_async(&mut stream, max_frame_size).await {
                        Ok((frame, _len)) => ConnectionEvent::IncomingFrame(frame),
                        Err(e) => {
                            edgerun_log::debug!("HTTP/2 frame read error: {:?}", e);
                            ConnectionEvent::OutgoingMessage(OutgoingFrame::Goaway {
                                error_code: ErrorCode::INTERNAL_ERROR.to_u32(),
                                debug_data: Vec::new(),
                            })
                        }
                    }
                },
                async {
                    match frame_rx.recv().await {
                        Some(m) => ConnectionEvent::OutgoingMessage(m),
                        None => ConnectionEvent::OutgoingMessage(OutgoingFrame::Goaway {
                            error_code: ErrorCode::NO_ERROR.to_u32(),
                            debug_data: b"client dropped".to_vec(),
                        }),
                    }
                },
            )
        } else {
            // No pending PING — race read, channel, and keepalive timer
            let keepalive = sleep(IDLE_TIMEOUT);
            select!(
                async {
                    match read_frame_async(&mut stream, max_frame_size).await {
                        Ok((frame, _len)) => ConnectionEvent::IncomingFrame(frame),
                        Err(e) => {
                            edgerun_log::debug!("HTTP/2 frame read error: {:?}", e);
                            ConnectionEvent::OutgoingMessage(OutgoingFrame::Goaway {
                                error_code: ErrorCode::INTERNAL_ERROR.to_u32(),
                                debug_data: Vec::new(),
                            })
                        }
                    }
                },
                async {
                    select!(
                        async {
                            match frame_rx.recv().await {
                                Some(m) => ConnectionEvent::OutgoingMessage(m),
                                None => ConnectionEvent::OutgoingMessage(OutgoingFrame::Goaway {
                                    error_code: ErrorCode::NO_ERROR.to_u32(),
                                    debug_data: b"client dropped".to_vec(),
                                }),
                            }
                        },
                        async {
                            keepalive.await;
                            ConnectionEvent::KeepaliveTick
                        },
                    )
                },
            )
        };

        match event {
            ConnectionEvent::IncomingFrame(frame) => {
                last_activity = Instant::now();

                // Check if this is a PING ACK matching our keepalive
                if frame.frame_type == FrameType::Ping
                    && frame.flags & flags::PING_ACK != 0
                    && frame.payload.len() == 8
                {
                    let ack_id = u64::from_be_bytes(frame.payload[..8].try_into().unwrap());
                    if let Some((_, sent_id)) = ping_deadline {
                        if ack_id == sent_id {
                            ping_deadline = None;
                        }
                    }
                }

                // Any frame resets pending PING (server is alive)
                if let Some((deadline, _)) = ping_deadline {
                    if Instant::now() <= deadline {
                        ping_deadline = None;
                    }
                }

                if let Err(e) = process_incoming_frame(
                    &mut stream,
                    &state,
                    &frame,
                    &mut continuation_state,
                    max_frame_size,
                )
                .await
                {
                    edgerun_log::warn!("HTTP/2 connection task error: {:?}", e);
                    let _ = write_frame_async(
                        &mut stream,
                        &Frame {
                            frame_type: FrameType::Goaway,
                            flags: 0,
                            stream_id: 0,
                            payload: {
                                let mut p = Vec::with_capacity(8);
                                p.extend_from_slice(&0u32.to_be_bytes());
                                p.extend_from_slice(
                                    &ErrorCode::INTERNAL_ERROR.to_u32().to_be_bytes(),
                                );
                                p
                            },
                        },
                    )
                    .await;
                    break;
                }
            }
            ConnectionEvent::OutgoingMessage(msg) => match msg {
                OutgoingFrame::SendHeaders {
                    stream_id,
                    headers,
                    end_stream,
                } => {
                    let (encoded,) = {
                        let mut s = state.lock().unwrap();
                        let enc = s
                            .hpack
                            .encode_header_block(headers.iter().map(|(n, v)| (&n[..], &v[..])))?;
                        (enc,)
                    };
                    let hdr_frame = HeadersFrame::new(stream_id, encoded, end_stream);
                    let frame_bytes = hdr_frame.to_frame().to_bytes();
                    if let Err(e) = stream.write_all(&frame_bytes).await {
                        edgerun_log::warn!("Error writing HEADERS: {:?}", e);
                        break;
                    }
                    let _ = stream.flush().await;
                }
                OutgoingFrame::SendData {
                    stream_id,
                    data,
                    end_stream,
                } => {
                    let data_len = data.len() as u32;
                    let frame_bytes = {
                        let data_frame = DataFrame::new(stream_id, data, end_stream);
                        data_frame.to_frame().to_bytes()
                    };

                    let flow_ok = {
                        let mut s = state.lock().unwrap();
                        s.flow.consume_connection(data_len).is_ok()
                            && s.flow.consume_stream(stream_id, data_len).is_ok()
                    };
                    if !flow_ok {
                        edgerun_log::warn!("Flow control error for stream {}", stream_id);
                        continue;
                    }

                    if let Err(e) = stream.write_all(&frame_bytes).await {
                        edgerun_log::warn!("Error writing DATA: {:?}", e);
                        break;
                    }
                    let _ = stream.flush().await;

                    let wu_conn = WindowUpdateFrame::new(0, data_len);
                    let _ = write_frame_async(&mut stream, &wu_conn.to_frame()).await;
                    let wu_stream = WindowUpdateFrame::new(stream_id, data_len);
                    let _ = write_frame_async(&mut stream, &wu_stream.to_frame()).await;
                }
                OutgoingFrame::Ping { reply_tx } => {
                    let ping_id = crate::http::runtime::time::SystemTime::now()
                        .duration_since(crate::http::runtime::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                    let data = ping_id.to_be_bytes();
                    let ping_frame = PingFrame::new(data);
                    let frame_bytes = ping_frame.to_frame().to_bytes();
                    if let Err(e) = stream.write_all(&frame_bytes).await {
                        let _ =
                            reply_tx.send(Err(Http2Error::Io(crate::http::runtime::bare_io(e))));
                        break;
                    }
                    let _ = stream.flush().await;
                    let _ = reply_tx.send(Ok(ping_id));
                }
                OutgoingFrame::Goaway {
                    error_code,
                    debug_data,
                } => {
                    let last_stream = {
                        let s = state.lock().unwrap();
                        s.next_stream.saturating_sub(2)
                    };
                    let goaway = GoawayFrame::new(last_stream, error_code, debug_data);
                    let frame_bytes = goaway.to_frame().to_bytes();
                    let _ = stream.write_all(&frame_bytes).await;
                    let _ = stream.flush().await;
                    break;
                }
            },
            ConnectionEvent::KeepaliveTick => {
                let ping_id = crate::http::runtime::time::SystemTime::now()
                    .duration_since(crate::http::runtime::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let data = ping_id.to_be_bytes();
                let ping_frame = PingFrame::new(data);
                let frame_bytes = ping_frame.to_frame().to_bytes();
                if stream.write_all(&frame_bytes).await.is_err() {
                    break;
                }
                let _ = stream.flush().await;
                ping_deadline = Some((Instant::now() + PING_TIMEOUT, ping_id));
            }
        }

        // Check PING timeout — if deadline passed with no response, connection is dead
        if let Some((deadline, _)) = ping_deadline {
            if Instant::now() > deadline {
                edgerun_log::warn!("HTTP/2 keepalive PING timeout — closing connection");
                break;
            }
        }
    }

    // Close all pending stream callbacks
    let mut s = state.lock().unwrap();
    for (_, cb) in core::mem::take(&mut s.stream_callbacks) {
        if let Some(tx) = cb.response_tx {
            let _ = tx.send(Err(Http2Error::Io(crate::http::runtime::io::Error::new(
                crate::http::runtime::io::ErrorKind::ConnectionReset,
                "connection closed",
            ))));
        }
        drop(cb.body_tx);
    }
}

/// Process an incoming frame.
async fn process_incoming_frame<S>(
    stream: &mut S,
    state: &Arc<crate::http::runtime::sync::Mutex<StreamStateInner>>,
    frame: &Frame,
    continuation_state: &mut Option<(u32, Vec<u8>, bool)>,
    max_frame_size: u32,
) -> Result<()>
where
    S: AsyncWrite + Unpin,
{
    if let Err(ec) = frame.validate_semantics() {
        return Err(Http2Error::ConnectionError {
            error_code: ErrorCode::from_u32(ec),
            reason: "semantic violation".into(),
        });
    }

    match frame.frame_type {
        FrameType::Settings => {
            let is_ack = frame.flags & flags::SETTINGS_ACK != 0;
            if !is_ack {
                let settings_frame = SettingsFrame::from_frame(frame)?;
                let (header_table_size, max_frame) = {
                    let mut s = state.lock().unwrap();
                    s.client_settings = Settings::from_entries(&settings_frame.entries)?;
                    let hts = s.client_settings.header_table_size as usize;
                    let mf = s.client_settings.max_frame_size;
                    (hts, mf)
                };
                {
                    let mut s = state.lock().unwrap();
                    s.max_frame_size = max_frame;
                    s.hpack.set_max_table_size(header_table_size);
                }
                let ack = SettingsFrame::ack();
                write_frame_async(stream, &ack.to_frame()).await?;
            }
        }

        FrameType::Headers => {
            let hdr_frame = HeadersFrame::from_frame(frame)?;
            let end_headers = frame.flags & flags::HEADERS_END_HEADERS != 0;
            let end_stream = hdr_frame.end_stream;

            if !end_headers {
                *continuation_state = Some((
                    hdr_frame.stream_id,
                    hdr_frame.header_block.clone(),
                    end_stream,
                ));
                return Ok(());
            }

            let headers = {
                let mut s = state.lock().unwrap();
                s.hpack.decode_header_block(&hdr_frame.header_block)?
            };

            handle_response_headers(state, hdr_frame.stream_id, headers, end_stream).await?;
        }

        FrameType::Continuation => {
            if let Some((stream_id, block, headers_end_stream)) = continuation_state {
                let block_clone = {
                    let mut b = block.clone();
                    b.extend_from_slice(&frame.payload);
                    b
                };

                let end_headers = frame.flags & flags::HEADERS_END_HEADERS != 0;
                if end_headers {
                    let sid = *stream_id;
                    let end_stream = *headers_end_stream;
                    *continuation_state = None;

                    let headers = {
                        let mut s = state.lock().unwrap();
                        s.hpack.decode_header_block(&block_clone)?
                    };

                    handle_response_headers(state, sid, headers, end_stream).await?;
                }
            }
        }

        FrameType::Data => {
            let end_stream = frame.flags & flags::DATA_END_STREAM != 0;
            let sid = frame.stream_id;
            let data_len = frame.payload.len() as u32;

            let wu_conn = WindowUpdateFrame::new(0, data_len);
            let _ = write_frame_async(stream, &wu_conn.to_frame()).await;
            let wu_stream = WindowUpdateFrame::new(sid, data_len);
            let _ = write_frame_async(stream, &wu_stream.to_frame()).await;

            {
                let mut s = state.lock().unwrap();
                if let Some(cb) = s.stream_callbacks.get_mut(&sid) {
                    if !frame.payload.is_empty() {
                        if let Some(ref tx) = cb.body_tx {
                            if tx.try_send(frame.payload.clone()).is_err() {
                                // Channel full — consumer too slow or dropped.
                                edgerun_log::warn!(
                                    "HTTP/2 body channel full for stream {sid} — closing body"
                                );
                                cb.body_tx = None;
                            }
                        }
                    }
                    if end_stream {
                        cb.body_tx = None;
                        if !cb.got_response_headers {
                            let resp = HttpResponse {
                                status: StatusCode::new(200)
                                    .unwrap_or_else(|_| StatusCode::new(200).unwrap()),
                                headers: HeaderMap::new(),
                                body: Vec::new(),
                            };
                            if let Some(tx) = cb.response_tx.take() {
                                let _ = tx.send(Ok(resp));
                            }
                        }
                    }
                }
            }
        }

        FrameType::RstStream => {
            let rst = RstStreamFrame::from_frame(frame)?;
            let sid = rst.stream_id;
            let mut s = state.lock().unwrap();
            if let Some(cb) = s.stream_callbacks.remove(&sid) {
                if let Some(tx) = cb.response_tx {
                    let _ = tx.send(Err(Http2Error::StreamError {
                        stream_id: sid,
                        error_code: ErrorCode::from_u32(rst.error_code),
                    }));
                }
            }
            if let Some(s) = s.streams.get_stream_mut(sid) {
                s.close();
            }
        }

        FrameType::Goaway => {
            edgerun_log::debug!("HTTP/2 GOAWAY received");
            return Err(Http2Error::Io(crate::http::runtime::io::Error::new(
                crate::http::runtime::io::ErrorKind::ConnectionReset,
                "GOAWAY received",
            )));
        }

        FrameType::Ping => {
            let ping = PingFrame::from_frame(frame)?;
            if !ping.ack {
                let ack = PingFrame::ack(ping.data);
                let _ = write_frame_async(stream, &ack.to_frame()).await;
            }
        }

        FrameType::WindowUpdate => {
            let wu = WindowUpdateFrame::from_frame(frame)?;
            let mut s = state.lock().unwrap();
            if wu.stream_id == 0 {
                let _ = s.flow.increment_connection(wu.window_increment);
            } else {
                let _ = s.flow.increment_stream(wu.stream_id, wu.window_increment);
            }
        }

        FrameType::PushPromise | FrameType::Priority => {
            // Ignore — client doesn't support push
        }
        // RFC 9113 §4.1: unknown frame types MUST be ignored
        FrameType::Extension => {}
    }

    Ok(())
}

async fn handle_response_headers(
    state: &Arc<crate::http::runtime::sync::Mutex<StreamStateInner>>,
    stream_id: u32,
    headers: Vec<(Vec<u8>, Vec<u8>)>,
    end_stream: bool,
) -> Result<()> {
    let mut s = state.lock().unwrap();
    if let Some(cb) = s.stream_callbacks.get_mut(&stream_id) {
        let mut status_code = 200u16;
        let mut resp_headers = HeaderMap::new();

        for (name, value) in &headers {
            if name == b":status" {
                if let Ok(s) = core::str::from_utf8(value) {
                    if let Ok(code) = s.parse::<u16>() {
                        status_code = code;
                    }
                }
            } else {
                if let (Ok(n), Ok(v)) = (core::str::from_utf8(name), core::str::from_utf8(value)) {
                    let _ = resp_headers.insert(n, v);
                }
            }
        }

        let status = StatusCode::new(status_code).unwrap_or_else(|_| StatusCode::new(200).unwrap());
        cb.got_response_headers = true;

        if end_stream {
            let resp = HttpResponse {
                status,
                headers: resp_headers,
                body: Vec::new(),
            };
            if let Some(tx) = cb.response_tx.take() {
                let _ = tx.send(Ok(resp));
            }
            cb.body_tx = None;
        } else {
            let resp = HttpResponse {
                status,
                headers: resp_headers,
                body: Vec::new(),
            };
            if let Some(tx) = cb.response_tx.take() {
                let _ = tx.send(Ok(resp));
            }
        }
    }
    Ok(())
}

// ===========================================================================
// Async frame I/O helpers
// ===========================================================================

async fn read_frame_async<S>(
    stream: &mut S,
    max_frame_size: u32,
) -> crate::http::runtime::io::Result<(Frame, usize)>
where
    S: AsyncRead + Unpin,
{
    let mut header = [0u8; 9];
    stream
        .read_exact(&mut header)
        .await
        .map_err(crate::http::runtime::bare_io)?;

    let length = ((header[0] as u32) << 16) | ((header[1] as u32) << 8) | (header[2] as u32);

    if length > max_frame_size {
        return Err(crate::http::runtime::io::Error::new(
            crate::http::runtime::io::ErrorKind::InvalidData,
            format!("frame size {} exceeds max {}", length, max_frame_size),
        ));
    }

    let mut payload = vec![0u8; length as usize];
    if length > 0 {
        stream
            .read_exact(&mut payload)
            .await
            .map_err(crate::http::runtime::bare_io)?;
    }

    let mut frame_bytes = Vec::with_capacity(9 + payload.len());
    frame_bytes.extend_from_slice(&header);
    frame_bytes.extend_from_slice(&payload);

    Frame::from_bytes(&frame_bytes, max_frame_size).map_err(|e| {
        crate::http::runtime::io::Error::new(
            crate::http::runtime::io::ErrorKind::InvalidData,
            format!("{:?}", e),
        )
    })
}

async fn write_frame_async<S>(stream: &mut S, frame: &Frame) -> crate::http::runtime::io::Result<()>
where
    S: AsyncWrite + Unpin,
{
    let bytes = frame.to_bytes();
    stream
        .write_all(&bytes)
        .await
        .map_err(crate::http::runtime::bare_io)?;
    stream.flush().await.map_err(crate::http::runtime::bare_io)
}
