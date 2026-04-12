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
    /// Remote HTTP/3 settings
    #[allow(dead_code)]
    remote_settings: Http3Settings,
    /// Streams
    streams: HashMap<u64, Http3Stream>,
    /// Next bidirectional stream ID
    next_bidi_stream_id: u64,
    /// Next unidirectional stream ID
    next_uni_stream_id: u64,
    /// Max push ID
    #[allow(dead_code)]
    max_push_id: u64,
    /// Server name (for SNI)
    #[allow(dead_code)]
    server_name: String,
    /// Control stream ID
    control_stream_id: Option<u64>,
    /// Buffered CRYPTO data to send
    pending_crypto: Vec<u8>,
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
        };

        // Send server connection preface:
        // 1. Control stream with SETTINGS
        // 2. QPACK encoder stream
        // 3. QPACK decoder stream
        conn.send_server_preface()?;

        Ok(conn)
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

    /// Poll for incoming frames on the given stream.
    ///
    /// Returns the parsed [`Http3Frame`] if one was received, `None` if no data
    /// is available, or an error if the frame is malformed.
    pub fn poll_stream(&mut self, stream_id: u64) -> Result<Option<Http3Frame>> {
        // Try to receive data from QUIC
        match self.quic.recv_stream_data() {
            Ok(Some((recv_stream_id, data, _fin))) => {
                // Verify the data belongs to the requested stream
                if recv_stream_id != stream_id {
                    // Return data to buffer for later (simplified: just report None)
                    return Ok(None);
                }
                if data.is_empty() {
                    return Ok(None);
                }
                // Parse as HTTP/3 frame
                Http3Frame::from_bytes(&data)
                    .map(|(frame, _consumed)| Some(frame))
                    .map_err(|e| Http3Error::ProtocolViolation(format!("Malformed HTTP/3 frame: {:?}", e)))
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
        // Manually craft a minimal :status: 200 encoding using static table index 28
        // Indexed Header Field: 1 1 S----- → 0xC0 | index
        let encoded = vec![0xC0 | 28];

        let mut decoder = QpackDecoder::new();
        let (status, _headers) =
            Http3Connection::decode_response_header(&encoded, &mut decoder).unwrap();

        assert_eq!(status.as_u16(), 200);
    }

    #[test]
    fn test_qpack_decode_static_request() {
        // Manually craft minimal request headers using static table:
        // :method: GET → index 18, 0xC0 | 18
        // :scheme: https → index 59, 0xC0 | 59
        // :path: / → index 2, 0xC0 | 2
        let encoded = vec![0xC0 | 18, 0xC0 | 59, 0xC0 | 2];

        let mut decoder = QpackDecoder::new();
        let (method, _uri, _headers) =
            Http3Connection::decode_request(&encoded, &mut decoder).unwrap();

        assert_eq!(method, Method::GET);
        // URI will be "https:/" without authority — just verify the method is correct
    }
}
