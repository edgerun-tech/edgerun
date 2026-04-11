//! HTTP/3 connection over QUIC

use std::collections::HashMap;
use std::net::UdpSocket;

use super::http3::frame::Http3Frame;
use super::http3::settings::Http3Settings;
use super::http3::stream::{Http3Stream, Http3StreamType};
use super::http3::stream_types;
use super::qpack::{QpackDecoder, QpackEncoder};
use super::quic::QuicConnection as QuicConn;
use super::Result;

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

    /// Send an HTTP/3 request
    pub fn send_request(
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

    /// Send an HTTP/3 response
    pub fn send_response(
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
}
