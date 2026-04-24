//! HTTP/3 connection over QUIC

use super::Http3Error;
use std::collections::HashMap;
use std::str::FromStr;

use super::http3::frame::{Http3Frame, Http3FrameType};
use super::http3::settings::Http3Settings;
use super::http3::stream::{Http3Stream, Http3StreamType};
use super::http3::stream_types;
use super::error_codes;
use super::qpack::{QpackDecoder, QpackEncoder};
use std::collections::hash_map::Entry;
use super::quic::QuicConnection as QuicConn;
use super::Result;
use crate::header::HeaderMap;
use crate::method::Method;
use crate::status::StatusCode;
use crate::uri::Uri;

/// HTTP/3 connection
pub struct Http3Connection {
    /// Underlying QUIC connection
    quic: QuicConn,
    /// QPACK encoder
    qpack_encoder: QpackEncoder,
    /// QPACK decoder
    qpack_decoder: QpackDecoder,
    /// Local HTTP/3 settings
    local_settings: Http3Settings,
    /// Remote HTTP/3 settings (populated from peer's SETTINGS frame)
    remote_settings: Http3Settings,
    /// Streams
    streams: HashMap<u64, Http3Stream>,
    /// Next bidirectional stream ID
    next_bidi_stream_id: u64,
    /// Next unidirectional stream ID
    next_uni_stream_id: u64,
    /// Max push ID (next push ID to assign)
    max_push_id: u64,
    /// Server name (for SNI)
    server_name: String,
    /// Control stream ID
    control_stream_id: Option<u64>,
    /// Buffered CRYPTO data to send
    pending_crypto: Vec<u8>,
    /// Receive buffers for streams (leftover bytes after HTTP/3 frame parsing)
    recv_buffers: HashMap<u64, Vec<u8>>,
    /// Tracks which streams have already received HEADERS frames
    /// Key = stream_id, Value = true if HEADERS received
    stream_headers_received: HashMap<u64, bool>,
    /// Stream priorities (RFC 9218)
    /// Key = stream_id, Value = (urgency, incremental)
    stream_priorities: HashMap<u64, (u8, bool)>,
    /// Connection migration state (RFC 9000 §9)
    /// The current active path (source and destination addresses)
    active_path: Option<(std::net::SocketAddr, std::net::SocketAddr)>,
    /// GOAWAY: the highest stream ID the peer will accept (RFC 9114 §5.2)
    /// None = no GOAWAY received, Some(id) = no new streams > id
    going_away: Option<u64>,
    /// GOAWAY: the highest stream ID we will accept (sent to peer)
    sent_goaway_id: u64,
    /// QPACK encoder stream ID (unidirectional, type 0x02)
    qpack_encoder_stream_id: Option<u64>,
    /// QPACK decoder stream ID (unidirectional, type 0x03)
    qpack_decoder_stream_id: Option<u64>,
    /// Known unidirectional stream types (for dispatching incoming uni streams)
    /// Key = stream_id, Value = stream type varint
    known_uni_stream_types: HashMap<u64, u64>,
    /// Max push ID received from peer (client-side: server's push limit)
    max_push_id_received: u64,
    /// Max bidirectional streams allowed by peer (from QUIC transport params)
    max_bidi_streams: u64,
    /// Max unidirectional streams allowed by peer (from QUIC transport params)
    max_uni_streams: u64,
}

impl Http3Connection {
    /// Get mutable access to the underlying QUIC connection (for testing).
    pub(crate) fn quic_mut(&mut self) -> &mut QuicConn {
        &mut self.quic
    }

    /// Get mutable access to known_uni_stream_types (for testing).
    pub(crate) fn known_uni_stream_types_mut(&mut self) -> &mut std::collections::HashMap<u64, u64> {
        &mut self.known_uni_stream_types
    }

    /// Get sent GOAWAY ID (for testing).
    pub(crate) fn sent_goaway_id(&self) -> Option<u64> {
        if self.sent_goaway_id == u64::MAX {
            None
        } else {
            Some(self.sent_goaway_id)
        }
    }

    /// Get received GOAWAY ID (for testing).
    pub(crate) fn received_goaway_id(&self) -> Option<u64> {
        self.going_away
    }

    /// Create a new HTTP/3 client connection.
    ///
    /// Resolves the server hostname, establishes a QUIC connection with
    /// TLS 1.3 handshake, and sends the HTTP/3 connection preface.
    pub async fn connect(server_name: &str) -> Result<Self> {
        let quic = QuicConn::connect(server_name).await?;

        let mut conn = Http3Connection {
            quic,
            qpack_encoder: QpackEncoder::new(),
            qpack_decoder: QpackDecoder::new(),
            local_settings: Http3Settings::new(),
            remote_settings: Http3Settings::new(),
            streams: HashMap::new(),
            next_bidi_stream_id: 0,
            next_uni_stream_id: 2,
            max_push_id: 0,
            server_name: server_name.to_string(),
            control_stream_id: None,
            pending_crypto: Vec::new(),
            recv_buffers: HashMap::new(),
            stream_headers_received: HashMap::new(),
            stream_priorities: HashMap::new(),
            active_path: None,
            going_away: None,
            sent_goaway_id: u64::MAX,
            qpack_encoder_stream_id: None,
            qpack_decoder_stream_id: None,
            known_uni_stream_types: HashMap::new(),
            max_push_id_received: 0,
            max_bidi_streams: 100,
            max_uni_streams: 100,
        };

        conn.send_connection_preface().await?;

        Ok(conn)
    }

    /// Create a server-side HTTP/3 connection from an established QUIC connection.
    ///
    /// Sends the HTTP/3 connection preface: control stream + QPACK encoder/decoder streams.
    pub async fn from_server(quic: QuicConn) -> super::Result<Self> {
        let mut conn = Http3Connection {
            quic,
            qpack_encoder: QpackEncoder::new(),
            qpack_decoder: QpackDecoder::new(),
            local_settings: Http3Settings::new(),
            remote_settings: Http3Settings::new(),
            streams: HashMap::new(),
            next_bidi_stream_id: 1,
            next_uni_stream_id: 3,
            max_push_id: 0,
            server_name: String::new(),
            control_stream_id: None,
            pending_crypto: Vec::new(),
            recv_buffers: HashMap::new(),
            stream_headers_received: HashMap::new(),
            stream_priorities: HashMap::new(),
            active_path: None,
            going_away: None,
            sent_goaway_id: u64::MAX,
            qpack_encoder_stream_id: None,
            qpack_decoder_stream_id: None,
            known_uni_stream_types: HashMap::new(),
            max_push_id_received: 0,
            max_bidi_streams: 100,
            max_uni_streams: 100,
        };

        conn.send_connection_preface().await?;

        Ok(conn)
    }

    /// Create an HTTP/3 connection from a pre-established QUIC connection (for testing).
    ///
    /// The QUIC connection should already have application traffic keys set up.
    /// No connection preface is sent.
    pub fn from_mock(quic: QuicConn) -> Self {
        Http3Connection {
            quic,
            qpack_encoder: QpackEncoder::new(),
            qpack_decoder: QpackDecoder::new(),
            local_settings: Http3Settings::new(),
            remote_settings: Http3Settings::new(),
            streams: HashMap::new(),
            next_bidi_stream_id: 1,
            next_uni_stream_id: 3,
            max_push_id: 0,
            server_name: String::new(),
            control_stream_id: None,
            pending_crypto: Vec::new(),
            recv_buffers: HashMap::new(),
            stream_headers_received: HashMap::new(),
            stream_priorities: HashMap::new(),
            active_path: None,
            going_away: None,
            sent_goaway_id: u64::MAX,
            qpack_encoder_stream_id: None,
            qpack_decoder_stream_id: None,
            known_uni_stream_types: HashMap::new(),
            max_push_id_received: 0,
            max_bidi_streams: 100,
            max_uni_streams: 100,
        }
    }

    /// Send the HTTP/3 connection preface (RFC 9114 §6.2.1).
    ///
    /// Creates three unidirectional streams:
    /// - Control stream (type 0x00) with SETTINGS
    /// - QPACK encoder stream (type 0x02)
    /// - QPACK decoder stream (type 0x03)
    async fn send_connection_preface(&mut self) -> Result<()> {
        let control_stream_id = self.next_uni_stream_id;
        self.next_uni_stream_id += 4;

        let mut stream_data = Vec::new();
        Self::encode_varint(stream_types::CONTROL, &mut stream_data);

        let settings_frame = Http3Frame::Settings {
            entries: self.local_settings.to_entries(),
        };
        stream_data.extend_from_slice(&settings_frame.to_bytes());

        self.quic.send_stream_data(control_stream_id, &stream_data, false).await
            .map_err(|e| Http3Error::QuicError(e))?;
        self.control_stream_id = Some(control_stream_id);

        let encoder_stream_id = self.next_uni_stream_id;
        self.next_uni_stream_id += 4;
        let mut encoder_data = Vec::new();
        Self::encode_varint(stream_types::QPACK_ENCODER, &mut encoder_data);
        if self.qpack_encoder.insert_count() == 0 && self.local_settings.max_table_capacity > 0 {
            let cap = self.local_settings.max_table_capacity as usize;
            if cap <= 31 {
                encoder_data.push((0x20 | (cap & 0x1F)) as u8);
            } else {
                encoder_data.push((0x20 | (cap & 0x1F)) as u8);
                let mut remaining = cap >> 5;
                while remaining > 127 {
                    encoder_data.push((remaining & 0x7F | 0x80) as u8);
                    remaining >>= 7;
                }
                encoder_data.push(remaining as u8);
            }
        }
        self.quic.send_stream_data(encoder_stream_id, &encoder_data, false).await
            .map_err(|e| Http3Error::QuicError(e))?;
        self.qpack_encoder_stream_id = Some(encoder_stream_id);

        let decoder_stream_id = self.next_uni_stream_id;
        self.next_uni_stream_id += 4;
        let mut decoder_data = Vec::new();
        Self::encode_varint(stream_types::QPACK_DECODER, &mut decoder_data);
        decoder_data.push(0x20);
        self.quic.send_stream_data(decoder_stream_id, &decoder_data, false).await
            .map_err(|e| Http3Error::QuicError(e))?;
        self.qpack_decoder_stream_id = Some(decoder_stream_id);

        Ok(())
    }

    /// Get the server name (SNI) for this connection.
    pub fn server_name(&self) -> &str {
        &self.server_name
    }

    /// Get the next push ID that will be assigned.
    pub fn next_push_id(&self) -> u64 {
        self.max_push_id
    }

    /// Get the remote peer settings.
    pub fn remote_settings(&self) -> &Http3Settings {
        &self.remote_settings
    }

    // ------------------------------------------------------------------
    // QPACK configuration
    // ------------------------------------------------------------------

    /// Disable QPACK dynamic table encoding.
    ///
    /// Use this when the encoder and decoder are not synchronized
    /// (e.g., in tests without encoder/decoder streams).
    /// Only static table references and literals without indexing are used.
    pub fn disable_dynamic_table(&mut self) {
        self.qpack_encoder.set_max_capacity(0);
    }

    // ------------------------------------------------------------------
    // Server-side request lifecycle
    // ------------------------------------------------------------------

    /// Accept the next incoming HTTP/3 request (server-side).
    ///
    /// Receives data from the QUIC connection, parses HTTP/3 frames,
    /// decodes QPACK headers, and returns `(stream_id, method, uri, headers)`.
    ///
    /// After calling this, use `recv_request_body(stream_id)` to collect
    /// the request body, then `send_response(stream_id, ...)` to reply.
    ///
    /// Returns `None` if no data is available yet.
    pub async fn accept_request(
            &mut self,
        ) -> Result<Option<(u64, Method, Uri, HeaderMap)>> {
            loop {
                let (stream_id, frame) = match self.poll_stream_with_buffer().await? {
                    Some((sid, frame)) => (sid, frame),
                    None => return Ok(None),
                };
    
                match frame {
                    Http3Frame::Headers { header_block } => {
                        // Decode the request headers
                        let (method, uri, headers) =
                            Self::decode_request(&header_block, &mut self.qpack_decoder)?;
    
                        // Mark that this stream has received HEADERS
                        self.stream_headers_received.insert(stream_id, true);
    
                        return Ok(Some((stream_id, method, uri, headers)));
                    }
                    Http3Frame::Data { payload } => {
                        // Data before headers — buffer it for later
                        let buf = self.recv_buffers.entry(stream_id).or_insert_with(Vec::new);
                        buf.extend_from_slice(&payload);
                    }
                    _ => {
                        // Ignore non-data frames during request acceptance
                    }
                }
            }
        }

    /// Receive the request body for the given stream.
    ///
    /// Collects all DATA frames on this stream until no more data is available.
    /// Returns buffered data first (from frames received before HEADERS),
    /// then polls for additional DATA frames.
    ///
    /// Returns the body bytes, or `None` if no body data is available yet.
    pub async fn recv_request_body(&mut self, stream_id: u64) -> Result<Option<Vec<u8>>> {
            let mut body = Vec::new();
    
            // First, return any buffered data (from DATA frames received before HEADERS)
            if let Entry::Occupied(mut entry) = self.recv_buffers.entry(stream_id) {
                let buf = entry.get_mut();
                if !buf.is_empty() {
                    body.append(buf);
                    entry.remove();
                }
            }
    
            // Then collect any additional DATA frames
            loop {
                match self.poll_stream(stream_id).await? {
                    Some(Http3Frame::Data { payload }) => {
                        body.extend_from_slice(&payload);
                    }
                    Some(_) => {} // Ignore other frames
                    None => break,
                }
            }
    
            if body.is_empty() {
                Ok(None)
            } else {
                Ok(Some(body))
            }
        }

    /// Poll for an HTTP/3 frame on any stream, using per-stream receive buffers.
    ///
    /// If a stream has buffered data, tries to parse a frame from it first.
    /// Otherwise, reads from the QUIC connection.
    async fn poll_stream_with_buffer(&mut self) -> Result<Option<(u64, Http3Frame)>> {
        // First, check existing buffers for parseable frames
        // Collect stream IDs and clone their buffer data to avoid borrow conflicts
        let stream_ids: Vec<u64> = self.recv_buffers.keys().copied().collect();

        for stream_id in &stream_ids {
            let buf = match self.recv_buffers.get(stream_id) {
                Some(b) if !b.is_empty() => b.clone(),
                _ => continue,
            };

            match self.try_parse_frame_for_stream(*stream_id, &buf) {
                Ok(Some((frame, consumed))) => {
                    // Remove consumed bytes from buffer
                    if let Some(buf) = self.recv_buffers.get_mut(stream_id) {
                        buf.drain(..consumed);
                        if buf.is_empty() {
                            self.recv_buffers.remove(stream_id);
                        }
                    }
                    return Ok(Some((*stream_id, frame)));
                }
                Ok(None) => {
                    // Not enough data for a complete frame yet
                }
                Err(_) => {
                    let _ = self.quic.send_reset_stream(*stream_id, error_codes::H3_FRAME_ERROR).await;
                    self.recv_buffers.remove(stream_id);
                }
            }
        }

        // Read from QUIC connection
        match self.quic.recv_stream_data().await {
            Ok(Some((stream_id, data, fin))) => {
                // Handle stream closure
                if fin {
                    self.on_stream_closed(stream_id);
                    return Ok(None);
                }

                if data.is_empty() {
                    return Ok(None);
                }

                // For unidirectional streams with no known type, read the type varint first
                if stream_id % 4 >= 2 && !self.known_uni_stream_types.contains_key(&stream_id) {
                    let buf = self.recv_buffers.entry(stream_id).or_insert_with(Vec::new);
                    buf.extend_from_slice(&data);
                    return self.try_read_stream_type(stream_id);
                }

                // Dispatch based on stream type
                let stream_type = if stream_id % 4 >= 2 {
                    self.known_uni_stream_types.get(&stream_id).copied()
                } else {
                    None // Bidirectional — request/response stream
                };

                match stream_type {
                    Some(stream_types::CONTROL) => {
                        // Control stream: validate and dispatch frames
                        self.buffer_and_parse_control(stream_id, data)
                    }
                    Some(stream_types::QPACK_ENCODER) => {
                        // QPACK encoder stream: feed to decoder
                        self.qpack_decoder.on_encoder_stream(&data).ok();
                        Ok(None)
                    }
                    Some(stream_types::QPACK_DECODER) => {
                        // QPACK decoder stream: feed to encoder
                        if let Ok((push_id, _)) = Http3Frame::decode_varint(&data) {
                            self.qpack_encoder.set_known_received_count(push_id);
                        }
                        Ok(None)
                    }
                    Some(stream_types::PUSH) => {
                        // Push stream: parse as HTTP/3 frames
                        self.buffer_and_parse_frame(stream_id, data)
                    }
                    Some(_) => {
                        // Known uni-directional stream type not matching control/qpack/push
                        // Treat as unknown — buffer and parse frames
                        self.buffer_and_parse_frame(stream_id, data)
                    }
                    None => {
                        // Bidirectional stream or known request/response stream —
                        // parse as HTTP/3 frames (HEADERS, DATA, etc.)
                        self.buffer_and_parse_frame(stream_id, data)
                    }
                }
            }
            Ok(None) => Ok(None),
            Err(e) => Err(Http3Error::QuicError(e)),
        }
    }

    /// Try to read the stream type varint from a new unidirectional stream.
    fn try_read_stream_type(&mut self, stream_id: u64) -> Result<Option<(u64, Http3Frame)>> {
        let buf = self.recv_buffers.get(&stream_id).cloned().unwrap_or_default();
        if buf.is_empty() {
            return Ok(None);
        }

        match Http3Frame::decode_varint(&buf) {
            Ok((stream_type, varint_len)) => {
                if buf.len() < varint_len {
                    return Ok(None); // Incomplete varint
                }
                // Remove the type varint from the buffer
                if let Some(buf) = self.recv_buffers.get_mut(&stream_id) {
                    buf.drain(..varint_len);
                }
                self.known_uni_stream_types.insert(stream_id, stream_type);

                // Parse any remaining data as frames for this stream type
                if let Some(buf) = self.recv_buffers.get(&stream_id).cloned() {
                    if !buf.is_empty() {
                        match stream_type {
                            stream_types::CONTROL => {
                                return self.buffer_and_parse_control(stream_id, buf);
                            }
                            stream_types::QPACK_ENCODER => {
                                self.qpack_decoder.on_encoder_stream(&buf).ok();
                            }
                            stream_types::QPACK_DECODER => {
                                if let Ok((push_id, _)) = Http3Frame::decode_varint(&buf) {
                                    self.qpack_encoder.set_known_received_count(push_id);
                                }
                            }
                            stream_types::PUSH => {
                                return self.buffer_and_parse_frame(stream_id, buf);
                            }
                            _ => {} // Unknown type — discard
                        }
                    }
                }
                Ok(None)
            }
            Err(_) => Ok(None), // Incomplete varint — wait for more data
        }
    }

    /// Buffer data on the control stream and parse frames with validation.
    fn buffer_and_parse_control(&mut self, stream_id: u64, data: Vec<u8>) -> Result<Option<(u64, Http3Frame)>> {
        let buf = self.recv_buffers.entry(stream_id).or_insert_with(Vec::new);
        buf.extend_from_slice(&data);

        // Parse and extract goaway_id before borrowing self again
        let (frame_opt, goaway_id_opt) = match Http3Frame::from_bytes(buf) {
            Ok((frame, consumed)) => {
                // Validate frame type for control stream
                if let Err(e) = Self::validate_control_stream_frame(frame.frame_type()) {
                    return Err(e);
                }

                let goaway_id = if let Http3Frame::Goaway { stream_id: gid } = &frame {
                    Some(*gid)
                } else {
                    None
                };

                buf.drain(..consumed);
                if buf.is_empty() {
                    self.recv_buffers.remove(&stream_id);
                }
                (Some(frame), goaway_id)
            }
            Err(_) => (None, None),
        };

        // Dispatch GOAWAY after releasing the buffer borrow
        if let Some(gid) = goaway_id_opt {
            self.process_goaway(gid);
        }

        Ok(frame_opt.map(|f| (stream_id, f)))
    }

    /// Buffer data and try to parse an HTTP/3 frame.
    fn buffer_and_parse_frame(&mut self, stream_id: u64, data: Vec<u8>) -> Result<Option<(u64, Http3Frame)>> {
        let buf = self.recv_buffers.entry(stream_id).or_insert_with(Vec::new);
        buf.extend_from_slice(&data);

        match Http3Frame::from_bytes(buf) {
            Ok((frame, consumed)) => {
                buf.drain(..consumed);
                if buf.is_empty() {
                    self.recv_buffers.remove(&stream_id);
                }
                Ok(Some((stream_id, frame)))
            }
            Err(_) => Ok(None),
        }
    }

    /// Try to parse an HTTP/3 frame from the buffer for a known stream type.
    fn try_parse_frame_for_stream(
        &mut self,
        stream_id: u64,
        buf: &[u8],
    ) -> Result<Option<(Http3Frame, usize)>> {
        // For control streams, validate frame types
        if self.control_stream_id == Some(stream_id) {
            match Http3Frame::from_bytes(buf) {
                Ok((frame, consumed)) => {
                    Self::validate_control_stream_frame(frame.frame_type())?;
                    let goaway_id = if let Http3Frame::Goaway { stream_id: gid } = &frame {
                        Some(*gid)
                    } else {
                        None
                    };
                    if let Some(gid) = goaway_id {
                        self.process_goaway(gid);
                    }
                    Ok(Some((frame, consumed)))
                }
                Err(_) => Ok(None),
            }
        } else {
            match Http3Frame::from_bytes(buf) {
                Ok((frame, consumed)) => Ok(Some((frame, consumed))),
                Err(_) => Ok(None),
            }
        }
    }

    // ------------------------------------------------------------------
    // Typed request/response API (QPACK encode/decode)
    // ------------------------------------------------------------------

    /// Send an HTTP/3 request using typed parameters.
    ///
    /// Encodes the method, URI, and headers via QPACK, sends HEADERS + DATA
    /// on a new bidirectional stream, and returns the stream ID.
    pub async fn send_request(
        &mut self,
        method: &Method,
        uri: &Uri,
        headers: &HeaderMap,
        body: Option<Vec<u8>>,
    ) -> Result<u64> {
        let header_block = Self::encode_request(method, uri, headers, &mut self.qpack_encoder)?;
        self.send_request_raw(header_block, body).await
    }

    /// Send an HTTP/3 response using typed parameters.
    ///
    /// Encodes the status and headers via QPACK, sends HEADERS + DATA on the
    /// given stream ID.
    pub async fn send_response(
        &mut self,
        stream_id: u64,
        status: StatusCode,
        headers: &HeaderMap,
        body: Option<Vec<u8>>,
    ) -> Result<()> {
        let header_block = Self::encode_response(status, headers, &mut self.qpack_encoder)?;
        self.send_response_raw(stream_id, header_block, body).await
    }

    /// Receive and decode an HTTP/3 response from the given stream.
    ///
    /// Polls the stream for HEADERS + DATA frames, decodes the QPACK header
    /// block, and returns `(status, headers, body)`.
    ///
    /// Returns `None` if no data is available yet.
    pub async fn recv_response(
        &mut self,
        stream_id: u64,
    ) -> Result<Option<(StatusCode, HeaderMap, Vec<u8>)>> {
        let mut headers_received = false;
        let mut body = Vec::new();

        let frame = match self.poll_stream(stream_id).await? {
            Some(f) => f,
            None => return Ok(None),
        };

        let (status, headers) = match frame {
            Http3Frame::Headers { header_block } => {
                let (s, h) = Self::decode_response_header(&header_block, &mut self.qpack_decoder)?;
                headers_received = true;
                (s, h)
            }
            Http3Frame::Data { payload } => {
                body.extend_from_slice(&payload);
                return Ok(Some((
                    StatusCode::new(200).map_err(|e| Http3Error::ProtocolViolation(e))?,
                    HeaderMap::new(),
                    body,
                )));
            }
            _ => {
                return Err(Http3Error::ProtocolViolation(
                    "Expected HEADERS frame for response".to_string(),
                ));
            }
        };

        loop {
            match self.poll_stream(stream_id).await? {
                Some(Http3Frame::Data { payload }) => {
                    body.extend_from_slice(&payload);
                }
                Some(_) => {}
                None => break,
            }
        }

        if headers_received {
            Ok(Some((status, headers, body)))
        } else {
            Ok(None)
        }
    }

    /// Accept the next incoming HTTP/3 stream (server-side).
    ///
    /// Polls the QUIC connection for the first available STREAM frame and
    /// returns `(stream_id, frame)`. The caller can then use `recv_request()`
    /// or `recv_response()` with the returned stream ID.
    ///
    /// Returns `None` if no data is available yet.
    pub async fn accept_stream(&mut self) -> Result<Option<(u64, Http3Frame)>> {
            match self.poll_stream_any().await? {
                Some((stream_id, frame)) => Ok(Some((stream_id, frame))),
                None => Ok(None),
            }
        }

    /// Poll for incoming frames on any stream.
    ///
    /// Returns the parsed [`Http3Frame`] and the stream ID it belongs to.
    async fn poll_stream_any(&mut self) -> Result<Option<(u64, Http3Frame)>> {
        match self.quic.recv_stream_data().await {
            Ok(Some((stream_id, data, _fin))) => {
                if data.is_empty() {
                    return Ok(None);
                }
                Http3Frame::from_bytes(&data)
                    .map(|(frame, _consumed)| Some((stream_id, frame)))
                    .map_err(|e| Http3Error::ProtocolViolation(format!("Malformed HTTP/3 frame: {:?}", e)))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(Http3Error::QuicError(e)),
        }
    }

    /// Receive and decode an HTTP/3 request from the given stream.
    ///
    /// Polls the stream for HEADERS + DATA frames, decodes the QPACK header
    /// block, and returns `(method, uri, headers, body)`.
    ///
    /// Returns `None` if no data is available yet.
    pub async fn recv_request(
            &mut self,
            stream_id: u64,
        ) -> Result<Option<(Method, Uri, HeaderMap, Vec<u8>)>> {
            let frame = match self.poll_stream(stream_id).await? {
                Some(f) => f,
                None => return Ok(None),
            };
    
            let (method, uri, headers) = match frame {
                Http3Frame::Headers { header_block } => {
                    Self::decode_request(&header_block, &mut self.qpack_decoder)?
                }
                _ => {
                    return Err(Http3Error::ProtocolViolation(
                        "Expected HEADERS frame for request".to_string(),
                    ));
                }
            };
    
            // Collect body
            let mut body = Vec::new();
            loop {
                match self.poll_stream(stream_id).await? {
                    Some(Http3Frame::Data { payload }) => {
                        body.extend_from_slice(&payload);
                    }
                    Some(_) => {}
                    None => break,
                }
            }
    
            Ok(Some((method, uri, headers, body)))
        }

    // ------------------------------------------------------------------
    // QPACK encode/decode helpers
    // ------------------------------------------------------------------

    /// Encode a request into a QPACK header block.
    pub(crate) fn encode_request(
        method: &Method,
        uri: &Uri,
        headers: &HeaderMap,
        encoder: &mut QpackEncoder,
    ) -> Result<Vec<u8>> {
        // Collect pseudo-headers as owned Strings (needed for lifetime reasons)
        let mut h3_headers: Vec<(String, String)> = Vec::new();

        // :method
        h3_headers.push((":method".into(), method.as_str().into()));

        // :scheme
        let scheme = match uri.scheme() {
            crate::uri::Scheme::Http => "http",
            crate::uri::Scheme::Https => "https",
            crate::uri::Scheme::Other(s) => s.as_str(),
        };
        h3_headers.push((":scheme".into(), scheme.into()));

        // :authority (host:port)
        if let Some(host) = uri.host() {
            let authority = if let Some(port) = uri.port() {
                format!("{}:{}", host, port)
            } else {
                host.to_string()
            };
            h3_headers.push((":authority".into(), authority));
        }

        // :path
        let path = uri.path();
        let path = if let Some(query) = uri.query() {
            format!("{}?{}", path, query)
        } else {
            path.to_string()
        };
        h3_headers.push((":path".into(), path));

        // Regular headers
        for (name, value) in headers.iter() {
            h3_headers.push((name.as_str().into(), value.as_str().into()));
        }

        // Convert to &[(&str, &str)] for encoder
        let refs: Vec<(&str, &str)> = h3_headers.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();

        let (header_block, _encoder_instructions) = encoder.encode(&refs)
            .map_err(|e| Http3Error::QpackError(e.to_string()))?;
        Ok(header_block)
    }

    /// Encode a CONNECT request into a QPACK header block.
    ///
    /// CONNECT requests use `:method: CONNECT` and `:authority` (target host).
    /// For extended CONNECT (RFC 9114 §4.4), an optional `:protocol` can be set
    /// to `websocket` for WebSocket-over-HTTP/3 tunneling.
    pub(crate) fn encode_connect_request(
        authority: &str,
        protocol: Option<&str>,
        headers: &HeaderMap,
        encoder: &mut QpackEncoder,
    ) -> Result<Vec<u8>> {
        let mut h3_headers: Vec<(String, String)> = Vec::new();

        // :method: CONNECT
        h3_headers.push((":method".into(), "CONNECT".into()));
        // :authority
        h3_headers.push((":authority".into(), authority.into()));

        // Optional :protocol for extended CONNECT (WebSocket, WebTransport)
        if let Some(proto) = protocol {
            h3_headers.push((":protocol".into(), proto.into()));
        }

        // Regular headers
        for (name, value) in headers.iter() {
            h3_headers.push((name.as_str().into(), value.as_str().into()));
        }

        let refs: Vec<(&str, &str)> = h3_headers.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();

        let (header_block, _encoder_instructions) = encoder.encode(&refs)
            .map_err(|e| Http3Error::QpackError(e.to_string()))?;
        Ok(header_block)
    }

    /// Send a CONNECT request to establish a tunnel (RFC 9114 §4.4).
    ///
    /// For HTTP/3, CONNECT establishes a bidirectional tunnel stream.
    /// Returns the stream ID for the tunnel. Use `send_stream_data` on the
    /// returned stream ID to send raw tunnel data.
    ///
    /// # Extended CONNECT (WebSocket)
    /// Set `protocol` to `Some("websocket")` for WebSocket-over-HTTP/3 tunneling
    /// (RFC 9114 §4.4, RFC 9220).
    pub async fn send_connect(
        &mut self,
        authority: &str,
        protocol: Option<&str>,
        extra_headers: &HeaderMap,
    ) -> Result<u64> {
        let header_block = Self::encode_connect_request(
            authority,
            protocol,
            extra_headers,
            &mut self.qpack_encoder,
        )?;

        // CONNECT uses a bidirectional stream, no body initially
        self.send_request_raw(header_block, None).await
    }

    /// Encode a response into a QPACK header block.
    pub(crate) fn encode_response(
        status: StatusCode,
        headers: &HeaderMap,
        encoder: &mut QpackEncoder,
    ) -> Result<Vec<u8>> {
        let mut h3_headers: Vec<(String, String)> = Vec::new();

        // :status
        let status_str = status.as_str();
        h3_headers.push((":status".into(), status_str.into_owned()));

        // Regular headers
        for (name, value) in headers.iter() {
            h3_headers.push((name.as_str().into(), value.as_str().into()));
        }

        let refs: Vec<(&str, &str)> = h3_headers.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();

        let (header_block, _encoder_instructions) = encoder.encode(&refs)
            .map_err(|e| Http3Error::QpackError(e.to_string()))?;
        Ok(header_block)
    }

    /// Decode a request header block into (method, uri, headers).
    pub(crate) fn decode_request(
        header_block: &[u8],
        decoder: &mut QpackDecoder,
    ) -> Result<(Method, Uri, HeaderMap)> {
        let headers = decoder.decode(header_block)
            .map_err(|e| Http3Error::QpackError(e.to_string()))?;

        let mut method = None;
        let mut scheme = None;
        let mut authority = None;
        let mut path = None;
        let mut regular_headers = HeaderMap::new();

        for (name, value) in headers {
            match name.as_str() {
                ":method" => {
                    method = Some(Method::from_str(&value)
                        .map_err(|e| Http3Error::ProtocolViolation(e))?);
                }
                ":scheme" => scheme = Some(value),
                ":authority" => authority = Some(value),
                ":path" => path = Some(value),
                _ => {
                    let _ = regular_headers.insert(&name, &value);
                }
            }
        }

        let method = method.ok_or_else(|| {
            Http3Error::ProtocolViolation("Missing :method pseudo-header".to_string())
        })?;
        let path = path.ok_or_else(|| {
            Http3Error::ProtocolViolation("Missing :path pseudo-header".to_string())
        })?;

        // Build URI
        let uri_str = if let Some(s) = scheme {
            if let Some(a) = authority {
                format!("{}://{}{}", s, a, path)
            } else {
                // Scheme but no authority — add a dummy authority for parsing
                format!("{}://{}", s, path.trim_start_matches('/'))
            }
        } else if let Some(a) = authority {
            // Origin-form with authority but no scheme (unusual but valid)
            format!("http://{}{}", a, path)
        } else {
            // Origin-form path only — use a dummy scheme+authority for parsing
            format!("http://localhost{}", path)
        };

        let uri = Uri::parse(&uri_str)
            .map_err(|e| Http3Error::ProtocolViolation(e))?;

        Ok((method, uri, regular_headers))
    }

    /// Decode a response header block into (status, headers).
    pub(crate) fn decode_response_header(
        header_block: &[u8],
        decoder: &mut QpackDecoder,
    ) -> Result<(StatusCode, HeaderMap)> {
        let headers = decoder.decode(header_block)
            .map_err(|e| Http3Error::QpackError(e.to_string()))?;

        let mut status = None;
        let mut regular_headers = HeaderMap::new();

        for (name, value) in headers {
            if name == ":status" {
                status = Some(StatusCode::new(
                    value.parse::<u16>()
                        .map_err(|e| Http3Error::ProtocolViolation(format!("Invalid status: {}", e)))?
                ).map_err(|e| Http3Error::ProtocolViolation(e))?);
            } else {
                let _ = regular_headers.insert(&name, &value);
            }
        }

        let status = status.ok_or_else(|| {
            Http3Error::ProtocolViolation("Missing :status pseudo-header".to_string())
        })?;

        Ok((status, regular_headers))
    }

    // ------------------------------------------------------------------
    // Raw (pre-encoded) send methods — used by typed methods above
    // ------------------------------------------------------------------

    /// Send a request with a pre-encoded QPACK header block.
    pub async fn send_request_raw(
        &mut self,
        header_block: Vec<u8>,
        body: Option<Vec<u8>>,
    ) -> Result<u64> {
        if let Some(max_id) = self.going_away {
            if self.next_bidi_stream_id > max_id {
                return Err(Http3Error::FrameUnexpected(format!(
                    "GOAWAY received: cannot create stream {} (max allowed: {})",
                    self.next_bidi_stream_id, max_id
                )));
            }
        }

        if !self.can_create_bidi_stream() {
            return Err(Http3Error::ProtocolViolation(format!(
                "Bidirectional stream limit reached ({}/{})",
                self.next_bidi_stream_id / 4, self.max_bidi_streams
            )));
        }

        let stream_id = self.next_bidi_stream_id;
        self.next_bidi_stream_id += 4;

        let stream = Http3Stream::new(stream_id, Http3StreamType::Request);
        self.streams.insert(stream_id, stream);

        let headers_frame = Http3Frame::Headers { header_block };
        let frame_data = headers_frame.to_bytes();

        self.quic
            .send_stream_data(stream_id, &frame_data, body.is_none()).await
            .map_err(|e| format!("Failed to send headers: {}", e))?;

        if let Some(body_data) = body {
            let data_frame = Http3Frame::Data { payload: body_data };
            let data_bytes = data_frame.to_bytes();
            self.quic
                .send_stream_data(stream_id, &data_bytes, true).await
                .map_err(|e| format!("Failed to send data: {}", e))?;
        }

        Ok(stream_id)
    }

    /// Send an HTTP/3 response with a pre-encoded QPACK header block.
    pub async fn send_response_raw(
        &mut self,
        stream_id: u64,
        header_block: Vec<u8>,
        body: Option<Vec<u8>>,
    ) -> Result<()> {
        let headers_frame = Http3Frame::Headers { header_block };
        let frame_data = headers_frame.to_bytes();

        self.quic
            .send_stream_data(stream_id, &frame_data, body.is_none()).await
            .map_err(|e| format!("Failed to send headers: {}", e))?;

        if let Some(body_data) = body {
            let data_frame = Http3Frame::Data { payload: body_data };
            let data_bytes = data_frame.to_bytes();
            self.quic
                .send_stream_data(stream_id, &data_bytes, true).await
                .map_err(|e| format!("Failed to send data: {}", e))?;
        }

        if let Some(stream) = self.streams.get_mut(&stream_id) {
            stream.half_close_local();
        }

        Ok(())
    }

    /// Send HTTP/3 response trailers (RFC 9114 §4.2).
    ///
    /// Trailers are additional headers sent after the response body.
    /// The stream remains open after sending trailers (FIN not set).
    pub async fn send_response_trailers(
        &mut self,
        stream_id: u64,
        trailer_block: Vec<u8>,
    ) -> super::Result<()> {
        // Trailers are sent as a HEADERS frame after DATA
        // The server must have already sent the initial HEADERS + DATA
        let headers_frame = Http3Frame::Headers { header_block: trailer_block };
        let frame_data = headers_frame.to_bytes();

        self.quic
            .send_stream_data(stream_id, &frame_data, false).await
            .map_err(|e| Http3Error::QuicError(e))
    }

    /// Receive HTTP/3 response trailers from the given stream.
    ///
    /// After receiving the response body, call this to check for trailers.
    /// Returns `None` if no trailer HEADERS frame is available.
    pub async fn recv_response_trailers(
        &mut self,
        stream_id: u64,
    ) -> Result<Option<HeaderMap>> {
        // Poll for a HEADERS frame on the stream (trailers)
        match self.poll_stream(stream_id).await? {
            Some(Http3Frame::Headers { header_block }) => {
                let headers = self.qpack_decoder.decode(&header_block)
                    .map_err(|e| Http3Error::QpackError(e.to_string()))?;
                let mut trailer_map = HeaderMap::new();
                for (name, value) in headers {
                    let _ = trailer_map.insert(&name, &value);
                }
                Ok(Some(trailer_map))
            }
            Some(_) => Ok(None), // Non-HEADERS frame — not a trailer
            None => Ok(None),
        }
    }

    /// Poll for incoming frames on the given stream.
    ///
    /// Returns the parsed [`Http3Frame`] if one was received, `None` if no data
    /// is available, or an error if the frame is malformed.
    pub async fn poll_stream(&mut self, stream_id: u64) -> Result<Option<Http3Frame>> {
        if let Entry::Occupied(mut entry) = self.recv_buffers.entry(stream_id) {
            let buf = entry.get_mut();
            if !buf.is_empty() {
                match Http3Frame::from_bytes(buf) {
                    Ok((frame, consumed)) => {
                        buf.drain(..consumed);
                        if buf.is_empty() {
                            entry.remove();
                        }
                        return Ok(Some(frame));
                    }
                    Err(_) => {}
                }
            }
        }

        match self.quic.recv_stream_data().await {
            Ok(Some((recv_stream_id, data, _fin))) => {
                let buf = self.recv_buffers.entry(recv_stream_id).or_insert_with(Vec::new);
                buf.extend_from_slice(&data);

                if recv_stream_id == stream_id {
                    match Http3Frame::from_bytes(buf) {
                        Ok((frame, consumed)) => {
                            buf.drain(..consumed);
                            if buf.is_empty() {
                                self.recv_buffers.remove(&stream_id);
                            }
                            return Ok(Some(frame));
                        }
                        Err(_) => return Ok(None),
                    }
                }

                Ok(None)
            }
            Ok(None) => Ok(None),
            Err(e) => Err(Http3Error::QuicError(e)),
        }
    }

    /// Send GOAWAY
    pub async fn goaway(&mut self, stream_id: u64) -> Result<()> {
        self.sent_goaway_id = stream_id;
        let frame = Http3Frame::Goaway { stream_id };
        let frame_data = frame.to_bytes();

        let control_id = self.control_stream_id
            .ok_or_else(|| "No control stream established".to_string())?;

        self.quic
            .send_stream_data(control_id, &frame_data, false).await
            .map_err(|e| Http3Error::QuicError(e))?;

        Ok(())
    }

    /// Get stream by ID
    pub fn get_stream(&self, stream_id: u64) -> Option<&Http3Stream> {
        self.streams.get(&stream_id)
    }

    /// Get QPACK encoder
    pub fn qpack_encoder_mut(&mut self) -> &mut QpackEncoder {
        &mut self.qpack_encoder
    }

    /// Get QPACK decoder
    pub fn qpack_decoder_mut(&mut self) -> &mut QpackDecoder {
        &mut self.qpack_decoder
    }

    fn encode_varint(value: u64, output: &mut Vec<u8>) {
        edgerun_encoding::quic_varint::encode_varint(value, output)
    }

    // ------------------------------------------------------------------
    // Server Push (RFC 9114 §4.4, §7.5-7.6)
    // ------------------------------------------------------------------

    /// Send a PUSH_PROMISE frame to the client (server push).
    ///
    /// The server pushes a response for a request stream by sending a
    /// PUSH_PROMISE with the push stream ID and the promised request headers.
    /// Also creates the push stream and sends the stream type varint prefix.
    ///
    /// Returns the push stream ID that will carry the pushed response.
    /// After calling this, use `quic.send_stream_data(push_stream_id, ...)`
    /// to send HEADERS + DATA frames on the push stream.
    pub async fn send_push_promise(
        &mut self,
        request_stream_id: u64,
        promised_headers: Vec<u8>,
    ) -> Result<u64> {
        let push_id = self.max_push_id;
        let push_stream_id = self.next_uni_stream_id;
        self.next_uni_stream_id += 4;
        self.max_push_id += 1;

        // Check MAX_PUSH_ID limit
        if push_id > self.max_push_id_received {
            return Err(Http3Error::ProtocolViolation(format!(
                "Push ID {} exceeds peer's MAX_PUSH_ID limit ({})",
                push_id, self.max_push_id_received
            )));
        }

        // Send PUSH_PROMISE on the request stream
        let push_promise = Http3Frame::PushPromise {
            push_id,
            header_block: promised_headers.clone(),
        };
        let frame_data = push_promise.to_bytes();

        self.quic
            .send_stream_data(request_stream_id, &frame_data, false).await
            .map_err(|e| Http3Error::QuicError(e))?;

        // Create the push stream with type varint prefix (RFC 9114 §6.2.4)
        let mut push_stream_data = Vec::new();
        Self::encode_varint(stream_types::PUSH, &mut push_stream_data);

        self.quic
            .send_stream_data(push_stream_id, &push_stream_data, false).await
            .map_err(|e| Http3Error::QuicError(e))?;

        // Track this push stream
        self.known_uni_stream_types.insert(push_stream_id, stream_types::PUSH);

        Ok(push_stream_id)
    }

    /// Send HEADERS + DATA frames on a push stream.
    ///
    /// After `send_push_promise()` creates the push stream, call this to
    /// send the pushed response headers and body.
    pub async fn send_push_data(
        &mut self,
        push_stream_id: u64,
        push_headers: Vec<u8>,
        push_body: Vec<u8>,
        fin: bool,
    ) -> Result<()> {
        // Send HEADERS frame on push stream
        let headers_frame = Http3Frame::Headers { header_block: push_headers };
        let headers_data = headers_frame.to_bytes();
        self.quic
            .send_stream_data(push_stream_id, &headers_data, false).await
            .map_err(|e| Http3Error::QuicError(e))?;

        // Send DATA frame on push stream
        if !push_body.is_empty() {
            let data_frame = Http3Frame::Data { payload: push_body };
            let data_data = data_frame.to_bytes();
            self.quic
                .send_stream_data(push_stream_id, &data_data, fin).await
                .map_err(|e| Http3Error::QuicError(e))?;
        } else if fin {
            // No body, but close the stream
            self.quic
                .send_stream_data(push_stream_id, &[], true).await
                .map_err(|e| Http3Error::QuicError(e))?;
        }

        Ok(())
    }

    /// Send a MAX_PUSH_ID frame to allow the server to push more responses.
    pub async fn send_max_push_id(&mut self, push_id: u64) -> Result<()> {
        let frame = Http3Frame::MaxPushId { push_id };
        let frame_data = frame.to_bytes();

        let control_id = self.control_stream_id
            .ok_or_else(|| "No control stream established".to_string())?;

        self.quic
            .send_stream_data(control_id, &frame_data, false).await
            .map_err(|e| format!("Failed to send MAX_PUSH_ID: {}", e))?;

        Ok(())
    }

    /// Cancel a server push stream.
    pub async fn cancel_push(&mut self, push_id: u64) -> Result<()> {
        let frame = Http3Frame::CancelPush { push_id };
        let frame_data = frame.to_bytes();

        let control_id = self.control_stream_id
            .ok_or_else(|| "No control stream established".to_string())?;

        self.quic
            .send_stream_data(control_id, &frame_data, false).await
            .map_err(|e| format!("Failed to send CANCEL_PUSH: {}", e))?;

        Ok(())
    }

    // ------------------------------------------------------------------
    // Server Push Stream Reading (RFC 9114 §4.4, §7.2)
    // ------------------------------------------------------------------

    /// Accept an incoming push stream from the server.
    ///
    /// When the server opens a unidirectional push stream (type 0x01),
    /// this reads the push ID varint prefix and then the HEADERS frame.
    ///
    /// Returns (push_id, decoded_headers) if a complete push is available.
    pub async fn accept_push_stream(
        &mut self,
        _expected_stream_id: u64,
    ) -> Result<Option<(u64, Vec<(String, String)>)>> {
        let result = match self.quic.recv_stream_data().await? {
            Some(d) => d,
            None => return Ok(None),
        };
        let (_sid, data, _fin) = result;

        if data.is_empty() {
            return Ok(None);
        }

        // Decode push ID varint
        let (push_id, varint_len) = Http3Frame::decode_varint(&data)
            .map_err(|e| Http3Error::ProtocolViolation(format!(
                "Invalid push ID varint: {}", e
            )))?;

        if data.len() < varint_len {
            return Ok(None);
        }

        // Parse HEADERS frame after the push ID
        let frame_data = &data[varint_len..];
        match Http3Frame::from_bytes(frame_data)? {
            (Http3Frame::Headers { header_block }, _consumed) => {
                let headers = self.qpack_decoder.decode(&header_block)
                    .map_err(|e| Http3Error::QpackError(e.to_string()))?;
                Ok(Some((push_id, headers)))
            }
            _ => Ok(None),
        }
    }

    /// Read DATA frames from an accepted push stream.
    ///
    /// After `accept_push_stream()`, call this repeatedly to receive
    /// the pushed response body.
    pub async fn read_push_body(
        &mut self,
        stream_id: u64,
    ) -> Result<Option<Vec<u8>>> {
        match self.poll_stream(stream_id).await? {
            Some(Http3Frame::Data { payload }) => Ok(Some(payload)),
            _ => Ok(None),
        }
    }

    /// Check if a unidirectional stream is a known push stream.
    pub fn is_push_stream(&self, stream_id: u64) -> bool {
        // Check if we've tracked this as a push stream
        self.known_uni_stream_types.get(&stream_id) == Some(&stream_types::PUSH)
    }

    // ------------------------------------------------------------------
    // HTTP/3 Priority (RFC 9218)
    // ------------------------------------------------------------------

    /// Set the priority for a request stream (RFC 9218 §4).
    ///
    /// Priority determines the scheduling order of responses.
    /// The priority field is sent on the request stream before the request body.
    pub async fn set_stream_priority(
        &mut self,
        stream_id: u64,
        urgency: u8,
        incremental: bool,
    ) -> Result<()> {
        // Priority frame format (RFC 9218 §4):
        // 0x00 = Priority field ID
        // urgency (0-7), incremental (1 bit)
        let priority_byte = (urgency.min(7) << 1) | (incremental as u8);
        let priority_data = vec![0x00, priority_byte];

        self.quic
            .send_stream_data(stream_id, &priority_data, false).await
            .map_err(|e| Http3Error::QuicError(e))
    }

    /// Get the priority for a received stream.
    ///
    /// Returns (urgency, incremental) if a priority frame was received.
    pub fn get_stream_priority(&self, stream_id: u64) -> Option<(u8, bool)> {
        self.stream_priorities.get(&stream_id).copied()
    }

    /// Schedule streams by priority for transmission.
    ///
    /// Returns stream IDs sorted by urgency (lower = more urgent),
    /// with incremental streams interleaved.
    pub fn schedule_by_priority(&self) -> Vec<u64> {
        let mut streams: Vec<(u64, u8, bool)> = self.stream_priorities.iter()
            .map(|(id, (u, i))| (*id, *u, *i))
            .collect();

        // Sort by urgency (lower first), then incremental (non-incremental first)
        streams.sort_by_key(|(_, u, i)| (*u, *i as u8));

        streams.into_iter().map(|(id, _, _)| id).collect()
    }

    // ------------------------------------------------------------------
    // GOAWAY Receive Processing (RFC 9114 §5.2)
    // ------------------------------------------------------------------

    /// Process an incoming GOAWAY frame (RFC 9114 §5.2).
    ///
    /// After receiving GOAWAY, the connection rejects new bidirectional
    /// streams with IDs greater than the goaway stream ID.
    pub fn process_goaway(&mut self, stream_id: u64) {
        self.going_away = Some(stream_id);
    }

    /// Check if the peer has sent GOAWAY and we should stop creating new streams.
    pub fn is_going_away(&self) -> bool {
        self.going_away.is_some()
    }

    /// Check if a given bidirectional stream ID is allowed after GOAWAY.
    pub fn is_stream_id_allowed(&self, stream_id: u64) -> bool {
        match self.going_away {
            Some(max_id) => stream_id <= max_id,
            None => true,
        }
    }

    // ------------------------------------------------------------------
    // Stream Creation Limits (RFC 9000 §4.6-4.7)
    // ------------------------------------------------------------------

    /// Check if we can create a new bidirectional stream.
    pub fn can_create_bidi_stream(&self) -> bool {
        let streams_created = self.next_bidi_stream_id / 4;
        let max_allowed = self.max_bidi_streams;
        streams_created < max_allowed
    }

    /// Check if we can create a new unidirectional stream.
    pub fn can_create_uni_stream(&self) -> bool {
        let streams_created = self.next_uni_stream_id / 4;
        let max_allowed = self.max_uni_streams;
        streams_created < max_allowed
    }

    // ------------------------------------------------------------------
    // Control Stream Validation (RFC 9114 §6.2.1)
    // ------------------------------------------------------------------

    /// Validate that a frame is legal on the control stream.
    ///
    /// Control stream only allows: SETTINGS (0x04), GOAWAY (0x07),
    /// MAX_PUSH_ID (0x05), CANCEL_PUSH (0x03).
    /// DATA, HEADERS, and other frame types are protocol errors.
    pub fn validate_control_stream_frame(frame_type: Http3FrameType) -> Result<()> {
        match frame_type {
            Http3FrameType::Settings
            | Http3FrameType::Goaway
            | Http3FrameType::MaxPushId
            | Http3FrameType::CancelPush => Ok(()),
            _ => Err(Http3Error::FrameUnexpected(format!(
                "Frame {:?} not allowed on control stream",
                frame_type
            ))),
        }
    }

    /// Validate that a frame is legal for the given stream type.
    ///
    /// - Control stream: only SETTINGS, GOAWAY, MAX_PUSH_ID, CANCEL_PUSH
    /// - Push stream: only DATA, HEADERS (after push_id varint)
    /// - QPACK encoder stream: only QPACK encoder instructions
    /// - QPACK decoder stream: only QPACK decoder instructions
    /// - Request/response streams: DATA, HEADERS
    pub fn validate_frame_on_stream_type(
        stream_id: u64,
        frame_type: Http3FrameType,
        stream_type: StreamType,
    ) -> Result<()> {
        match stream_type {
            StreamType::Control => Self::validate_control_stream_frame(frame_type),
            StreamType::Push => {
                match frame_type {
                    Http3FrameType::Data | Http3FrameType::Headers => Ok(()),
                    _ => Err(Http3Error::FrameUnexpected(format!(
                        "Frame {:?} not allowed on push stream {}",
                        frame_type, stream_id
                    ))),
                }
            }
            StreamType::QpackEncoder | StreamType::QpackDecoder => {
                // QPACK streams use their own instruction format, not HTTP/3 frames
                Err(Http3Error::FrameUnexpected(format!(
                    "HTTP/3 frames not expected on QPACK stream {}",
                    stream_id
                )))
            }
            StreamType::Request => {
                match frame_type {
                    Http3FrameType::Data | Http3FrameType::Headers => Ok(()),
                    _ => Err(Http3Error::FrameUnexpected(format!(
                        "Frame {:?} not allowed on request stream {}",
                        frame_type, stream_id
                    ))),
                }
            }
        }
    }

    /// Determine the stream type from a stream ID.
    fn stream_type_for_id(stream_id: u64) -> StreamType {
        // Unidirectional streams: stream_id % 4 == 2 or 3
        match stream_id % 4 {
            0 | 1 => StreamType::Request, // Bidirectional
            2 => StreamType::Control, // First uni stream (usually control)
            3 => StreamType::Push, // Server-initiated (could be push, QPACK, etc.)
            _ => StreamType::Request,
        }
    }

    // ------------------------------------------------------------------
    // Stream Closure Handling
    // ------------------------------------------------------------------

    /// Called when a stream is closed (FIN received or reset).
    ///
    /// Cleans up stream state and detects critical stream closure.
    pub fn on_stream_closed(&mut self, stream_id: u64) {
        // Remove from active streams
        self.streams.remove(&stream_id);

        // Remove from receive buffers
        self.recv_buffers.remove(&stream_id);

        // Remove from headers received tracking
        self.stream_headers_received.remove(&stream_id);

        // Remove from priorities
        self.stream_priorities.remove(&stream_id);

        // Check if this was the control stream — critical!
        if self.control_stream_id == Some(stream_id) {
            // Control stream closed — connection is dead per RFC 9114 §6.2.1
            self.control_stream_id = None;
        }

        // QPACK stream closure — fatal per RFC 9114 §6.2.2/6.2.3
        if self.qpack_encoder_stream_id == Some(stream_id) {
            self.qpack_encoder_stream_id = None;
        }
        if self.qpack_decoder_stream_id == Some(stream_id) {
            self.qpack_decoder_stream_id = None;
        }
    }

    /// Check if a critical stream (control, QPACK) has been closed.
    pub fn is_critical_stream_closed(&self) -> bool {
        // Control stream was never created or was closed
        (self.control_stream_id.is_none() && self.quic.is_established())
            || self.qpack_encoder_stream_id.is_none()
            || self.qpack_decoder_stream_id.is_none()
    }

    // ------------------------------------------------------------------
    // RESET_STREAM / STOP_SENDING (RFC 9000 §4.5-4.6)
    // ------------------------------------------------------------------

    /// Send RESET_STREAM to abort a stream (RFC 9000 §4.5).
    ///
    /// Tells the peer to stop sending on this stream and discard pending data.
    pub async fn reset_stream(&mut self, stream_id: u64, error_code: u64) -> Result<()> {
        self.quic
            .send_reset_stream(stream_id, error_code).await
            .map_err(|e| Http3Error::QuicError(e))?;
        self.on_stream_closed(stream_id);
        Ok(())
    }

    /// Send STOP_SENDING to tell peer to stop sending on a stream (RFC 9000 §4.6).
    ///
    /// We don't want to receive more data on this stream.
    pub async fn stop_sending(&mut self, stream_id: u64, error_code: u64) -> Result<()> {
        self.quic
            .send_stop_sending(stream_id, error_code).await
            .map_err(|e| Http3Error::QuicError(e))
    }
}

/// Stream type identifiers for HTTP/3.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamType {
    /// Control stream (SETTINGS, GOAWAY, etc.)
    Control,
    /// Push stream (server-pushed response)
    Push,
    /// QPACK encoder stream
    QpackEncoder,
    /// QPACK decoder stream
    QpackDecoder,
    /// Request/response bidirectional stream
    Request,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_id_allocation() {
        let mut conn = Http3Connection {
            quic: QuicConn::dummy(),
            qpack_encoder: QpackEncoder::new(),
            qpack_decoder: QpackDecoder::new(),
            local_settings: Http3Settings::new(),
            remote_settings: Http3Settings::new(),
            streams: HashMap::new(),
            next_bidi_stream_id: 0,
            next_uni_stream_id: 2,
            max_push_id: 0,
            server_name: "example.com".to_string(),
            control_stream_id: None,
            pending_crypto: Vec::new(),
            recv_buffers: HashMap::new(),
            stream_headers_received: HashMap::new(),
            stream_priorities: HashMap::new(),
            active_path: None,
            going_away: None,
            sent_goaway_id: u64::MAX,
            qpack_encoder_stream_id: None,
            qpack_decoder_stream_id: None,
            known_uni_stream_types: HashMap::new(),
            max_push_id_received: 0,
            max_bidi_streams: 100,
            max_uni_streams: 100,
        };

        assert_eq!(conn.next_bidi_stream_id, 0);
        assert_eq!(conn.next_uni_stream_id, 2);

        // After incrementing
        conn.next_bidi_stream_id += 4;
        assert_eq!(conn.next_bidi_stream_id, 4);
    }

    #[test]
    fn test_qpack_encode_request() {
        // Encoding works — produces non-empty output with correct structure
        let method = Method::GET;
        let uri = Uri::parse("https://example.com/").unwrap();
        let headers = HeaderMap::new();

        let mut encoder = QpackEncoder::new();
        let encoded = Http3Connection::encode_request(&method, &uri, &headers, &mut encoder).unwrap();

        // Should produce bytes
        assert!(!encoded.is_empty());
        // At minimum the first byte should encode :method
        assert!(encoded.len() >= 3);
    }

    #[test]
    fn test_qpack_encode_response() {
        // :status: 200 is in the static table (index 28)
        let status = StatusCode::new(200).unwrap();
        let headers = HeaderMap::new();

        let mut encoder = QpackEncoder::new();
        let encoded = Http3Connection::encode_response(status, &headers, &mut encoder).unwrap();

        // :status: 200 is indexed static reference → single byte
        assert!(!encoded.is_empty());
    }

    #[test]
    fn test_qpack_decode_static_response() {
        // Encode :status: 200 via encoder, then decode
        let mut encoder = QpackEncoder::new();
        let (encoded, _) = encoder.encode(&[(":status", "200")]).unwrap();

        let mut decoder = QpackDecoder::new();
        let (status, _headers) =
            Http3Connection::decode_response_header(&encoded, &mut decoder).unwrap();

        assert_eq!(status.as_u16(), 200);
    }

    #[test]
    fn test_qpack_decode_static_request() {
        // Encode minimal request headers via encoder, then decode
        let mut encoder = QpackEncoder::new();
        let (encoded, _) = encoder.encode(&[
            (":method", "GET"),
            (":scheme", "https"),
            (":path", "/"),
        ]).unwrap();

        let mut decoder = QpackDecoder::new();
        let (method, _uri, _headers) =
            Http3Connection::decode_request(&encoded, &mut decoder).unwrap();

        assert_eq!(method, Method::GET);
    }

    #[test]
    fn test_validate_control_stream_frame_data_rejected() {
        // DATA frames are NOT allowed on control stream
        let result = Http3Connection::validate_control_stream_frame(
            crate::http3::http3::frame::Http3FrameType::Data
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_control_stream_frame_settings_allowed() {
        // SETTINGS is the first frame on control stream
        let result = Http3Connection::validate_control_stream_frame(
            crate::http3::http3::frame::Http3FrameType::Settings
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_control_stream_frame_goaway_allowed() {
        // GOAWAY is allowed on control stream
        let result = Http3Connection::validate_control_stream_frame(
            crate::http3::http3::frame::Http3FrameType::Goaway
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_process_goaway_sets_going_away() {
        let mut conn = Http3Connection::from_mock(QuicConn::dummy());
        assert!(conn.going_away.is_none());

        // Process GOAWAY with stream ID 4
        conn.process_goaway(4);

        assert_eq!(conn.going_away, Some(4));
    }

    #[test]
    fn test_can_create_bidi_stream_within_limits() {
        let conn = Http3Connection::from_mock(QuicConn::dummy());
        // Default max_bidi_streams is 100, so first streams should be allowed
        assert!(conn.can_create_bidi_stream());
    }

    #[test]
    fn test_goaway_sets_going_away() {
        let mut conn = Http3Connection::from_mock(QuicConn::dummy());
        assert!(conn.going_away.is_none());

        // Process GOAWAY with stream ID 4
        conn.process_goaway(4);

        assert_eq!(conn.going_away, Some(4));
    }

    #[test]
    fn test_is_push_stream() {
        let mut conn = Http3Connection::from_mock(QuicConn::dummy());

        // No push stream tracked yet
        assert!(!conn.is_push_stream(7));

        // After send_push_promise creates a push stream at ID 7
        // We can't easily test full flow without QUIC, but we can check the map logic
        conn.known_uni_stream_types.insert(7, super::super::http3::stream_types::PUSH);
        assert!(conn.is_push_stream(7));

        // Non-push uni streams
        conn.known_uni_stream_types.insert(6, super::super::http3::stream_types::CONTROL);
        assert!(!conn.is_push_stream(6));
    }

    #[test]
    fn test_varint_encode_decode_roundtrip() {
        let values = [0u64, 1, 63, 64, 16383, 16384, 1073741823, 1073741824, u64::MAX / 4];
        for value in values {
            let mut encoded = Vec::new();
            Http3Connection::encode_varint(value, &mut encoded);

            let (decoded, len) = Http3Frame::decode_varint(&encoded).unwrap();
            assert_eq!(decoded, value);
            assert_eq!(len, encoded.len());
        }
    }

    #[test]
    fn test_frame_to_bytes_and_back() {
        // SETTINGS frame roundtrip
        let original = Http3Frame::Settings {
            entries: vec![(0x06, 100), (0x07, 128)],
        };
        let bytes = original.to_bytes();
        let (parsed, consumed) = Http3Frame::from_bytes(&bytes).unwrap();
        assert_eq!(consumed, bytes.len());

        if let Http3Frame::Settings { entries } = parsed {
            assert_eq!(entries.len(), 2);
            assert_eq!(entries[0], (0x06, 100));
            assert_eq!(entries[1], (0x07, 128));
        } else {
            panic!("Expected SETTINGS frame");
        }
    }

    #[test]
    fn test_frame_push_promise_roundtrip() {
        let original = Http3Frame::PushPromise {
            push_id: 42,
            header_block: vec![0x00, 0x01, 0x02],
        };
        let bytes = original.to_bytes();
        let (parsed, consumed) = Http3Frame::from_bytes(&bytes).unwrap();
        assert_eq!(consumed, bytes.len());

        if let Http3Frame::PushPromise { push_id, header_block } = parsed {
            assert_eq!(push_id, 42);
            assert_eq!(header_block, vec![0x00, 0x01, 0x02]);
        } else {
            panic!("Expected PushPromise frame");
        }
    }

    #[test]
    fn test_is_push_stream_returns_false_for_bidi_streams() {
        let conn = Http3Connection::from_mock(QuicConn::dummy());
        // Bidirectional streams are 0, 4, 8... (stream_id % 4 == 0)
        assert!(!conn.is_push_stream(0));
        assert!(!conn.is_push_stream(4));
        // Even without explicit tracking, bidi streams should not be push streams
    }
}
