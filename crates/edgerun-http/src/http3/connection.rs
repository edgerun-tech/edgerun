//! HTTP/3 connection over QUIC

use super::Http3Error;
use std::collections::HashMap;
use std::net::UdpSocket;
use std::str::FromStr;

use super::http3::frame::Http3Frame;
use super::http3::settings::Http3Settings;
use super::http3::stream::{Http3Stream, Http3StreamType};
use super::http3::stream_types;
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
}

impl Http3Connection {
    /// Create a new HTTP/3 client connection
    pub fn connect(socket: UdpSocket, server_name: &str) -> Result<Self> {
        let quic = QuicConn::client(socket, server_name)?;

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
        };

        // Send connection preface: create control stream and send SETTINGS
        conn.send_connection_preface()?;

        Ok(conn)
    }

    /// Create a server-side HTTP/3 connection from an established QUIC connection.
    ///
    /// Sends the HTTP/3 connection preface: control stream + QPACK encoder/decoder streams.
    pub fn from_server(quic: QuicConn) -> super::Result<Self> {
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
        };

        // Send server connection preface:
        // 1. Control stream with SETTINGS
        // 2. QPACK encoder stream
        // 3. QPACK decoder stream
        conn.send_server_preface()?;

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
        }
    }

    /// Send the server-side connection preface (RFC 9114 §6.2.1).
    ///
    /// Creates three unidirectional streams:
    /// - Control stream (type 0x00) with SETTINGS
    /// - QPACK encoder stream (type 0x02)
    /// - QPACK decoder stream (type 0x03)
    fn send_server_preface(&mut self) -> super::Result<()> {
        // 1. Control stream (stream ID 3 for server-initiated uni)
        let control_stream_id = self.next_uni_stream_id;
        self.next_uni_stream_id += 4;

        let mut stream_data = Vec::new();
        Self::encode_varint(stream_types::CONTROL, &mut stream_data);

        let settings_frame = Http3Frame::Settings {
            entries: self.local_settings.to_entries(),
        };
        stream_data.extend_from_slice(&settings_frame.to_bytes());

        self.quic.send_stream_data(control_stream_id, &stream_data, false)
            .map_err(|e| format!("Failed to send control stream: {}", e))?;
        self.control_stream_id = Some(control_stream_id);

        // 2. QPACK encoder stream (stream ID 7)
        let encoder_stream_id = self.next_uni_stream_id;
        self.next_uni_stream_id += 4;

        let mut encoder_data = Vec::new();
        Self::encode_varint(stream_types::QPACK_ENCODER, &mut encoder_data);
        // No additional data needed — default settings are fine

        self.quic.send_stream_data(encoder_stream_id, &encoder_data, false)
            .map_err(|e| format!("Failed to send QPACK encoder stream: {}", e))?;

        // 3. QPACK decoder stream (stream ID 11)
        let decoder_stream_id = self.next_uni_stream_id;
        self.next_uni_stream_id += 4;

        let mut decoder_data = Vec::new();
        Self::encode_varint(stream_types::QPACK_DECODER, &mut decoder_data);
        // Send Set Max Table Capacity with 0 (no dynamic table for now)
        // Instruction: 0x20 | (max_table_capacity >> 8) ... (simplified: just 0)
        decoder_data.push(0x20); // Set Max Table Capacity = 0

        self.quic.send_stream_data(decoder_stream_id, &decoder_data, false)
            .map_err(|e| format!("Failed to send QPACK decoder stream: {}", e))?;

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


    /// Send connection preface (control stream + SETTINGS)
    fn send_connection_preface(&mut self) -> Result<()> {
        // Create control stream (unidirectional)
        let control_stream_id = self.next_uni_stream_id;
        self.next_uni_stream_id += 4;

        // Stream type indicator: 0x00 = control stream
        let mut stream_data = Vec::new();
        Self::encode_varint(stream_types::CONTROL, &mut stream_data);

        // SETTINGS frame
        let settings_frame = Http3Frame::Settings {
            entries: self.local_settings.to_entries(),
        };
        stream_data.extend_from_slice(&settings_frame.to_bytes());

        // Send on QUIC unidirectional stream
        self.quic.send_stream_data(control_stream_id, &stream_data, false)
            .map_err(|e| format!("Failed to send control stream: {}", e))?;

        self.control_stream_id = Some(control_stream_id);

        Ok(())
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
    pub fn accept_request(
        &mut self,
    ) -> Result<Option<(u64, Method, Uri, HeaderMap)>> {
        loop {
            let (stream_id, frame) = match self.poll_stream_with_buffer()? {
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
    pub fn recv_request_body(&mut self, stream_id: u64) -> Result<Option<Vec<u8>>> {
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
            match self.poll_stream(stream_id)? {
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
    fn poll_stream_with_buffer(&mut self) -> Result<Option<(u64, Http3Frame)>> {
        // First, check existing buffers for parseable frames
        let mut consumed_streams = Vec::new();
        for (&stream_id, buf) in &self.recv_buffers {
            if buf.is_empty() {
                consumed_streams.push(stream_id);
                continue;
            }
            match Http3Frame::from_bytes(buf) {
                Ok((frame, consumed)) => {
                    // Remove consumed bytes from buffer
                    let buf = self.recv_buffers.get_mut(&stream_id).unwrap();
                    buf.drain(..consumed);
                    if buf.is_empty() {
                        consumed_streams.push(stream_id);
                    }
                    return Ok(Some((stream_id, frame)));
                }
                Err(_) => {
                    // Not enough data for a complete frame yet
                }
            }
        }

        // Clean up empty buffers
        for stream_id in consumed_streams {
            self.recv_buffers.remove(&stream_id);
        }

        // Read from QUIC connection
        match self.quic.recv_stream_data() {
            Ok(Some((stream_id, data, _fin))) => {
                if data.is_empty() {
                    return Ok(None);
                }

                // Try to parse an HTTP/3 frame from this data
                match Http3Frame::from_bytes(&data) {
                    Ok((frame, consumed)) => {
                        // Buffer any leftover bytes
                        if consumed < data.len() {
                            let buf = self.recv_buffers.entry(stream_id).or_insert_with(Vec::new);
                            buf.extend_from_slice(&data[consumed..]);
                        }
                        Ok(Some((stream_id, frame)))
                    }
                    Err(_) => {
                        // Not a complete HTTP/3 frame — buffer for later
                        let buf = self.recv_buffers.entry(stream_id).or_insert_with(Vec::new);
                        buf.extend_from_slice(&data);
                        Ok(None)
                    }
                }
            }
            Ok(None) => Ok(None),
            Err(e) => Err(Http3Error::QuicError(e)),
        }
    }

    // ------------------------------------------------------------------
    // Typed request/response API (QPACK encode/decode)
    // ------------------------------------------------------------------

    /// Send an HTTP/3 request using typed parameters.
    ///
    /// Encodes the method, URI, and headers via QPACK, sends HEADERS + DATA
    /// on a new bidirectional stream, and returns the stream ID.
    ///
    /// The stream ID can be used with `poll_stream()` to receive the response.
    pub fn send_request(
        &mut self,
        method: &Method,
        uri: &Uri,
        headers: &HeaderMap,
        body: Option<Vec<u8>>,
    ) -> Result<u64> {
        let header_block = Self::encode_request(method, uri, headers, &mut self.qpack_encoder)?;
        self.send_request_raw(header_block, body)
    }

    /// Send an HTTP/3 response using typed parameters.
    ///
    /// Encodes the status and headers via QPACK, sends HEADERS + DATA on the
    /// given stream ID.
    pub fn send_response(
        &mut self,
        stream_id: u64,
        status: StatusCode,
        headers: &HeaderMap,
        body: Option<Vec<u8>>,
    ) -> Result<()> {
        let header_block = Self::encode_response(status, headers, &mut self.qpack_encoder)?;
        self.send_response_raw(stream_id, header_block, body)
    }

    /// Receive and decode an HTTP/3 response from the given stream.
    ///
    /// Polls the stream for HEADERS + DATA frames, decodes the QPACK header
    /// block, and returns `(status, headers, body)`.
    ///
    /// Returns `None` if no data is available yet.
    pub fn recv_response(
        &mut self,
        stream_id: u64,
    ) -> Result<Option<(StatusCode, HeaderMap, Vec<u8>)>> {
        // Poll for frames on the stream
        let mut headers_received = false;
        let mut body = Vec::new();

        // First, get the HEADERS frame
        let frame = match self.poll_stream(stream_id)? {
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
                // Body without headers — unusual but handle it
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

        // Then collect any DATA frames
        loop {
            match self.poll_stream(stream_id)? {
                Some(Http3Frame::Data { payload }) => {
                    body.extend_from_slice(&payload);
                }
                Some(_) => {} // Ignore other frames
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
    pub fn accept_stream(&mut self) -> Result<Option<(u64, Http3Frame)>> {
        match self.poll_stream_any()? {
            Some((stream_id, frame)) => Ok(Some((stream_id, frame))),
            None => Ok(None),
        }
    }

    /// Poll for incoming frames on any stream.
    ///
    /// Returns the parsed [`Http3Frame`] and the stream ID it belongs to.
    fn poll_stream_any(&mut self) -> Result<Option<(u64, Http3Frame)>> {
        match self.quic.recv_stream_data() {
            Ok(Some((stream_id, data, _fin))) => {
                if data.is_empty() {
                    return Ok(None);
                }
                // Parse as HTTP/3 frame
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
    pub fn recv_request(
        &mut self,
        stream_id: u64,
    ) -> Result<Option<(Method, Uri, HeaderMap, Vec<u8>)>> {
        let frame = match self.poll_stream(stream_id)? {
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
            match self.poll_stream(stream_id)? {
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

        encoder.encode(&refs)
            .map_err(|e| Http3Error::QpackError(e.to_string()))
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

        encoder.encode(&refs)
            .map_err(|e| Http3Error::QpackError(e.to_string()))
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
    pub fn send_request_raw(
        &mut self,
        header_block: Vec<u8>,
        body: Option<Vec<u8>>,
    ) -> Result<u64> {
        let stream_id = self.next_bidi_stream_id;
        self.next_bidi_stream_id += 4;

        // Create stream
        let stream = Http3Stream::new(stream_id, Http3StreamType::Request);
        self.streams.insert(stream_id, stream);

        // Send HEADERS frame
        let headers_frame = Http3Frame::Headers { header_block };
        let frame_data = headers_frame.to_bytes();

        self.quic
            .send_stream_data(stream_id, &frame_data, body.is_none())
            .map_err(|e| format!("Failed to send headers: {}", e))?;

        // Send DATA frame if body present
        if let Some(body_data) = body {
            let data_frame = Http3Frame::Data {
                payload: body_data,
            };
            let data_bytes = data_frame.to_bytes();
            self.quic
                .send_stream_data(stream_id, &data_bytes, true)
                .map_err(|e| format!("Failed to send data: {}", e))?;
        }

        Ok(stream_id)
    }

    /// Send an HTTP/3 response with a pre-encoded QPACK header block.
    pub fn send_response_raw(
        &mut self,
        stream_id: u64,
        header_block: Vec<u8>,
        body: Option<Vec<u8>>,
    ) -> Result<()> {
        let headers_frame = Http3Frame::Headers { header_block };
        let frame_data = headers_frame.to_bytes();

        self.quic
            .send_stream_data(stream_id, &frame_data, body.is_none())
            .map_err(|e| format!("Failed to send headers: {}", e))?;

        if let Some(body_data) = body {
            let data_frame = Http3Frame::Data {
                payload: body_data,
            };
            let data_bytes = data_frame.to_bytes();
            self.quic
                .send_stream_data(stream_id, &data_bytes, true)
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
    pub fn send_response_trailers(
        &mut self,
        stream_id: u64,
        trailer_block: Vec<u8>,
    ) -> super::Result<()> {
        // Trailers are sent as a HEADERS frame after DATA
        // The server must have already sent the initial HEADERS + DATA
        let headers_frame = Http3Frame::Headers { header_block: trailer_block };
        let frame_data = headers_frame.to_bytes();

        self.quic
            .send_stream_data(stream_id, &frame_data, false)
            .map_err(|e| Http3Error::QuicError(e))
    }

    /// Receive HTTP/3 response trailers from the given stream.
    ///
    /// After receiving the response body, call this to check for trailers.
    /// Returns `None` if no trailer HEADERS frame is available.
    pub fn recv_response_trailers(
        &mut self,
        stream_id: u64,
    ) -> Result<Option<HeaderMap>> {
        // Poll for a HEADERS frame on the stream (trailers)
        match self.poll_stream(stream_id)? {
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
    pub fn poll_stream(&mut self, stream_id: u64) -> Result<Option<Http3Frame>> {
        // First check buffer for this stream
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
                    Err(_) => {
                        // Not enough data in buffer — fall through to read more
                    }
                }
            }
        }

        // Try to receive data from QUIC
        match self.quic.recv_stream_data() {
            Ok(Some((recv_stream_id, data, _fin))) => {
                // Buffer the data for this stream
                let buf = self.recv_buffers.entry(recv_stream_id).or_insert_with(Vec::new);
                buf.extend_from_slice(&data);

                // If it's for the requested stream, try to parse
                if recv_stream_id == stream_id {
                    match Http3Frame::from_bytes(buf) {
                        Ok((frame, consumed)) => {
                            buf.drain(..consumed);
                            if buf.is_empty() {
                                self.recv_buffers.remove(&stream_id);
                            }
                            return Ok(Some(frame));
                        }
                        Err(_) => {
                            // Not enough data yet — return None, data stays in buffer
                            return Ok(None);
                        }
                    }
                }

                // Data for a different stream — return None, data stays buffered
                Ok(None)
            }
            Ok(None) => Ok(None),
            Err(e) => Err(Http3Error::QuicError(e)),
        }
    }

    /// Send GOAWAY
    pub fn goaway(&mut self, stream_id: u64) -> Result<()> {
        let frame = Http3Frame::Goaway { stream_id };
        let frame_data = frame.to_bytes();

        let control_id = self.control_stream_id
            .ok_or_else(|| "No control stream established".to_string())?;

        self.quic
            .send_stream_data(control_id, &frame_data, false)
            .map_err(|e| format!("Failed to send GOAWAY: {}", e))?;

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
        if value < 64 {
            output.push(value as u8);
        } else if value < 16384 {
            output.push(((value >> 8) as u8) | 0x40);
            output.push(value as u8);
        } else if value < 1073741824 {
            let bytes = (value as u32).to_be_bytes();
            output.push(bytes[0] | 0x80);
            output.push(bytes[1]);
            output.push(bytes[2]);
            output.push(bytes[3]);
        } else {
            let bytes = value.to_be_bytes();
            output.push(bytes[0] | 0xC0);
            output.extend_from_slice(&bytes[1..]);
        }
    }

    // ------------------------------------------------------------------
    // Server Push (RFC 9114 §4.4, §7.5-7.6)
    // ------------------------------------------------------------------

    /// Send a PUSH_PROMISE frame to the client (server push).
    ///
    /// The server pushes a response for a request stream by sending a
    /// PUSH_PROMISE with the push stream ID and the promised request headers.
    ///
    /// Returns the push stream ID that will carry the pushed response.
    pub fn send_push_promise(
        &mut self,
        request_stream_id: u64,
        promised_headers: Vec<u8>,
    ) -> Result<u64> {
        let push_id = self.max_push_id;
        let push_stream_id = self.next_uni_stream_id;
        self.next_uni_stream_id += 4;
        self.max_push_id += 1;

        // Send PUSH_PROMISE on the request stream
        let push_promise = Http3Frame::PushPromise {
            push_id,
            header_block: promised_headers,
        };
        let frame_data = push_promise.to_bytes();

        self.quic
            .send_stream_data(request_stream_id, &frame_data, false)
            .map_err(|e| format!("Failed to send PUSH_PROMISE: {}", e))?;

        Ok(push_stream_id)
    }

    /// Send a MAX_PUSH_ID frame to allow the server to push more responses.
    pub fn send_max_push_id(&mut self, push_id: u64) -> Result<()> {
        let frame = Http3Frame::MaxPushId { push_id };
        let frame_data = frame.to_bytes();

        let control_id = self.control_stream_id
            .ok_or_else(|| "No control stream established".to_string())?;

        self.quic
            .send_stream_data(control_id, &frame_data, false)
            .map_err(|e| format!("Failed to send MAX_PUSH_ID: {}", e))?;

        Ok(())
    }

    /// Cancel a server push stream.
    pub fn cancel_push(&mut self, push_id: u64) -> Result<()> {
        let frame = Http3Frame::CancelPush { push_id };
        let frame_data = frame.to_bytes();

        let control_id = self.control_stream_id
            .ok_or_else(|| "No control stream established".to_string())?;

        self.quic
            .send_stream_data(control_id, &frame_data, false)
            .map_err(|e| format!("Failed to send CANCEL_PUSH: {}", e))?;

        Ok(())
    }

    // ------------------------------------------------------------------
    // Server Push Stream Reading (RFC 9114 §4.4, §7.2)
    // ------------------------------------------------------------------

    /// Accept an incoming push stream from the server.
    ///
    /// When the server opens a unidirectional stream with type 0x1 (push),
    /// call this to begin reading the pushed response.
    /// The push stream starts with a push ID (varint) followed by HTTP/3 frames.
    pub fn accept_push_stream(
        &mut self,
        stream_id: u64,
    ) -> Result<Option<(u64, Vec<(String, String)>)>> {
        // Push streams start with a varint push_id followed by HEADERS + DATA
        // We read the raw bytes and parse the push ID manually
        match self.poll_stream(stream_id)? {
            Some(Http3Frame::Headers { header_block }) => {
                let headers = self.qpack_decoder.decode(&header_block)
                    .map_err(|e| Http3Error::QpackError(e.to_string()))?;
                // Use stream_id as push_id proxy (in a full impl, read the varint prefix)
                Ok(Some((stream_id, headers)))
            }
            _ => Ok(None),
        }
    }

    /// Read DATA frames from an accepted push stream.
    ///
    /// After `accept_push_stream()`, call this repeatedly to receive
    /// the pushed response body.
    pub fn read_push_body(
        &mut self,
        stream_id: u64,
    ) -> Result<Option<Vec<u8>>> {
        match self.poll_stream(stream_id)? {
            Some(Http3Frame::Data { payload }) => Ok(Some(payload)),
            _ => Ok(None),
        }
    }

    /// Check if a unidirectional stream is a known push stream.
    pub fn is_push_stream(&self, stream_id: u64) -> bool {
        // Push streams are unidirectional (stream_id % 4 == 3)
        // and were advertised via PUSH_PROMISE
        stream_id % 4 == 3
    }

    // ------------------------------------------------------------------
    // HTTP/3 Priority (RFC 9218)
    // ------------------------------------------------------------------

    /// Set the priority for a request stream (RFC 9218 §4).
    ///
    /// Priority determines the scheduling order of responses.
    /// The priority field is sent on the request stream before the request body.
    pub fn set_stream_priority(
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
            .send_stream_data(stream_id, &priority_data, false)
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
        let encoded = encoder.encode(&[(":status", "200")]).unwrap();

        let mut decoder = QpackDecoder::new();
        let (status, _headers) =
            Http3Connection::decode_response_header(&encoded, &mut decoder).unwrap();

        assert_eq!(status.as_u16(), 200);
    }

    #[test]
    fn test_qpack_decode_static_request() {
        // Encode minimal request headers via encoder, then decode
        let mut encoder = QpackEncoder::new();
        let encoded = encoder.encode(&[
            (":method", "GET"),
            (":scheme", "https"),
            (":path", "/"),
        ]).unwrap();

        let mut decoder = QpackDecoder::new();
        let (method, _uri, _headers) =
            Http3Connection::decode_request(&encoded, &mut decoder).unwrap();

        assert_eq!(method, Method::GET);
    }
}
