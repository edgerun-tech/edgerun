//! HTTP/2 connection management

use super::flow_control::FlowControlManager;
use super::frame::{
    DataFrame, Frame, FrameType, GoawayFrame, HeadersFrame, PingFrame, RstStreamFrame,
    SettingsFrame, WindowUpdateFrame,
};
use super::stream::{Stream, StreamManager};
use super::{ErrorCode, Result};
use super::{Settings, CONNECTION_PREFACE};
use std::collections::HashMap;
use std::io::{self, Read, Write};

/// HTTP/2 connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConnectionState {
    /// Preface not yet sent/received
    Init,
    /// Preface sent, waiting for acknowledgment
    PrefaceSent,
    /// Connection established
    Established,
    /// GOAWAY sent, connection closing
    Closing,
    /// Connection closed
    Closed,
}

/// HTTP/2 connection
pub struct Connection<S> {
    /// Underlying stream (TcpStream or similar)
    stream: S,
    /// Connection state
    state: ConnectionState,
    /// Local settings
    local_settings: Settings,
    /// Remote settings (acknowledged)
    remote_settings: Settings,
    /// Pending settings (not yet acknowledged)
    pending_settings: bool,
    /// Stream manager
    streams: StreamManager,
    /// Flow control manager
    flow_control: FlowControlManager,
    /// Next ping ID
    ping_id: u64,
    /// Pending ping callbacks
    pending_pings: HashMap<u64, bool>,
    /// Last stream ID received
    last_stream_received: u32,
    /// Read buffer
    read_buffer: Vec<u8>,
    /// Write buffer
    write_buffer: Vec<u8>,
}

impl<S: Read + Write> Connection<S> {
    /// Create a new HTTP/2 client connection
    pub fn client(stream: S) -> Result<Self> {
        let mut conn = Connection {
            stream,
            state: ConnectionState::Init,
            local_settings: Settings::new(),
            remote_settings: Settings::new(),
            pending_settings: false,
            streams: StreamManager::new(65535),
            flow_control: FlowControlManager::new(65535),
            ping_id: 0,
            pending_pings: HashMap::new(),
            last_stream_received: 0,
            read_buffer: Vec::new(),
            write_buffer: Vec::new(),
        };

        // Send connection preface
        conn.send_preface()?;

        Ok(conn)
    }

    /// Create a new HTTP/2 server connection
    pub fn server(stream: S) -> Result<Self> {
        let conn = Connection {
            stream,
            state: ConnectionState::Init,
            local_settings: Settings::new(),
            remote_settings: Settings::new(),
            pending_settings: false,
            streams: StreamManager::new(65535),
            flow_control: FlowControlManager::new(65535),
            ping_id: 0,
            pending_pings: HashMap::new(),
            last_stream_received: 0,
            read_buffer: Vec::new(),
            write_buffer: Vec::new(),
        };

        Ok(conn)
    }

    /// Send connection preface
    fn send_preface(&mut self) -> Result<()> {
        // Send client connection preface
        self.stream.write_all(CONNECTION_PREFACE)?;

        // Send initial SETTINGS
        let settings_frame = SettingsFrame::new(self.local_settings.to_entries());
        let frame_bytes = settings_frame.to_frame().to_bytes();
        self.stream.write_all(&frame_bytes)?;
        self.stream.flush()?;

        self.state = ConnectionState::PrefaceSent;
        self.pending_settings = true;

        Ok(())
    }

    /// Read and process the next frame
    pub fn poll(&mut self) -> Result<Option<Frame>> {
        // Read frame header
        let mut header = [0u8; 9];
        self.stream.read_exact(&mut header)?;

        // Extend read buffer
        self.read_buffer.extend_from_slice(&header);

        // Parse frame length
        let length = ((header[0] as u32) << 16) | ((header[1] as u32) << 8) | (header[2] as u32);

        // Read frame payload
        let mut payload = vec![0u8; length as usize];
        if length > 0 {
            self.stream.read_exact(&mut payload)?;
            self.read_buffer.extend_from_slice(&payload);
        }

        // Parse frame
        let (frame, _) = Frame::from_bytes(&self.read_buffer)?;

        // Clear read buffer
        self.read_buffer.clear();

        // Process frame
        self.process_frame(&frame)?;

        Ok(Some(frame))
    }

    /// Process a received frame
    fn process_frame(&mut self, frame: &Frame) -> Result<()> {
        match frame.frame_type {
            FrameType::Settings => {
                let settings_frame = SettingsFrame::from_frame(frame)?;
                self.process_settings(&settings_frame)?;
            }
            FrameType::Ping => {
                let ping_frame = PingFrame::from_frame(frame)?;
                self.process_ping(&ping_frame)?;
            }
            FrameType::Goaway => {
                let goaway_frame = GoawayFrame::from_frame(frame)?;
                self.process_goaway(&goaway_frame)?;
            }
            FrameType::WindowUpdate => {
                let wu_frame = WindowUpdateFrame::from_frame(frame)?;
                self.process_window_update(&wu_frame)?;
            }
            FrameType::Headers => {
                let headers_frame = HeadersFrame::from_frame(frame)?;
                self.last_stream_received = frame.stream_id;
                self.process_headers(&headers_frame)?;
            }
            FrameType::Data => {
                let data_frame = DataFrame::from_frame(frame)?;
                self.last_stream_received = frame.stream_id;
                self.process_data(&data_frame)?;
            }
            FrameType::RstStream => {
                let rst_frame = RstStreamFrame::from_frame(frame)?;
                self.process_rst_stream(&rst_frame)?;
            }
            FrameType::Priority | FrameType::PushPromise | FrameType::Continuation => {
                // TODO: Implement these frame types
            }
        }

        Ok(())
    }

    /// Process SETTINGS frame
    fn process_settings(&mut self, settings_frame: &SettingsFrame) -> Result<()> {
        if settings_frame.ack {
            // Remote acknowledged our settings
            self.pending_settings = false;
        } else {
            // Apply remote settings
            self.remote_settings = Settings::from_entries(&settings_frame.entries)?;
            self.remote_settings.validate()?;

            // Send SETTINGS ack
            let ack_frame = SettingsFrame::ack();
            let frame_bytes = ack_frame.to_frame().to_bytes();
            self.stream.write_all(&frame_bytes)?;
        }

        Ok(())
    }

    /// Process PING frame
    fn process_ping(&mut self, ping_frame: &PingFrame) -> Result<()> {
        if ping_frame.ack {
            // Ping acknowledgment
            let ping_id = u64::from_be_bytes(ping_frame.data);
            self.pending_pings.remove(&ping_id);
        } else {
            // Send PING ACK
            let ack = PingFrame::ack(ping_frame.data);
            let frame_bytes = ack.to_frame().to_bytes();
            self.stream.write_all(&frame_bytes)?;
        }

        Ok(())
    }

    /// Process GOAWAY frame
    fn process_goaway(&mut self, goaway_frame: &GoawayFrame) -> Result<()> {
        self.state = ConnectionState::Closing;

        // Close streams higher than last_stream_received
        let last_stream = goaway_frame.last_stream_id;
        for stream_id in self.streams.active_stream_ids() {
            if stream_id > last_stream {
                if let Some(stream) = self.streams.get_stream_mut(stream_id) {
                    stream.close();
                }
            }
        }

        Ok(())
    }

    /// Process WINDOW_UPDATE frame
    fn process_window_update(&mut self, wu_frame: &WindowUpdateFrame) -> Result<()> {
        if wu_frame.stream_id == 0 {
            // Connection-level window update
            self.flow_control
                .increment_connection(wu_frame.window_increment)?;
        } else {
            // Stream-level window update
            self.flow_control
                .increment_stream(wu_frame.stream_id, wu_frame.window_increment)?;
        }

        Ok(())
    }

    /// Process HEADERS frame
    fn process_headers(&mut self, headers_frame: &HeadersFrame) -> Result<()> {
        // Get or create stream
        let stream = self
            .streams
            .get_or_create_stream(headers_frame.stream_id)?;

        // Transition stream state
        if stream.state == super::stream::StreamState::Idle {
            stream.open()?;
        }

        if headers_frame.end_stream {
            stream.half_close_remote()?;
        }

        Ok(())
    }

    /// Process DATA frame
    fn process_data(&mut self, data_frame: &DataFrame) -> Result<()> {
        // Update flow control
        let data_len = data_frame.data.len() as u32;
        self.flow_control.consume_connection(data_len)?;
        self.flow_control
            .consume_stream(data_frame.stream_id, data_len)?;

        // Queue data to stream
        if let Some(stream) = self.streams.get_stream_mut(data_frame.stream_id) {
            stream.queue_received(data_frame.data.clone());

            if data_frame.end_stream {
                stream.half_close_remote()?;
            }
        }

        // Send WINDOW_UPDATE to replenish window
        let wu_frame = WindowUpdateFrame::new(data_frame.stream_id, data_len);
        let frame_bytes = wu_frame.to_frame().to_bytes();
        self.stream.write_all(&frame_bytes)?;

        Ok(())
    }

    /// Process RST_STREAM frame
    fn process_rst_stream(&mut self, rst_frame: &RstStreamFrame) -> Result<()> {
        if let Some(stream) = self.streams.get_stream_mut(rst_frame.stream_id) {
            stream.close();
        }

        Ok(())
    }

    /// Send a request on a new stream
    pub fn send_request(
        &mut self,
        headers: Vec<u8>,
        body: Option<Vec<u8>>,
    ) -> Result<u32> {
        // Create new stream
        let stream_id = self.streams.create_client_stream()?;

        // Send HEADERS frame
        let end_stream = body.is_none();
        let headers_frame = HeadersFrame::new(stream_id, headers, end_stream);
        let frame_bytes = headers_frame.to_frame().to_bytes();
        self.stream.write_all(&frame_bytes)?;

        // Send DATA frame if body present
        if let Some(body_data) = body {
            let data_frame = DataFrame::new(stream_id, body_data, true);
            let frame_bytes = data_frame.to_frame().to_bytes();
            self.stream.write_all(&frame_bytes)?;
        }

        self.stream.flush()?;

        Ok(stream_id)
    }

    /// Send a response on an existing stream
    pub fn send_response(
        &mut self,
        stream_id: u32,
        headers: Vec<u8>,
        body: Option<Vec<u8>>,
    ) -> Result<()> {
        // Send HEADERS frame
        let end_stream = body.is_none();
        let headers_frame = HeadersFrame::new(stream_id, headers, end_stream);
        let frame_bytes = headers_frame.to_frame().to_bytes();
        self.stream.write_all(&frame_bytes)?;

        // Send DATA frame if body present
        if let Some(body_data) = body {
            let data_frame = DataFrame::new(stream_id, body_data, true);
            let frame_bytes = data_frame.to_frame().to_bytes();
            self.stream.write_all(&frame_bytes)?;
        }

        self.stream.flush()?;

        Ok(())
    }

    /// Send a PING
    pub fn ping(&mut self) -> Result<u64> {
        self.ping_id += 1;
        let data = self.ping_id.to_be_bytes();
        let ping_frame = PingFrame::new(data);
        let frame_bytes = ping_frame.to_frame().to_bytes();
        self.stream.write_all(&frame_bytes)?;
        self.stream.flush()?;

        self.pending_pings.insert(self.ping_id, false);

        Ok(self.ping_id)
    }

    /// Send GOAWAY and close connection
    pub fn goaway(&mut self, error_code: ErrorCode, debug_data: Vec<u8>) -> Result<()> {
        let last_stream = self.last_stream_received;
        let goaway_frame = GoawayFrame::new(last_stream, error_code.to_u32(), debug_data);
        let frame_bytes = goaway_frame.to_frame().to_bytes();
        self.stream.write_all(&frame_bytes)?;
        self.stream.flush()?;

        self.state = ConnectionState::Closing;

        Ok(())
    }

    /// Close the connection gracefully
    pub fn close(&mut self) -> Result<()> {
        self.goaway(ErrorCode::NO_ERROR, Vec::new())
    }

    /// Check if connection is established
    pub fn is_established(&self) -> bool {
        self.state == ConnectionState::Established
    }

    /// Check if connection is closed
    pub fn is_closed(&self) -> bool {
        self.state == ConnectionState::Closed
    }

    /// Get local settings
    pub fn local_settings(&self) -> &Settings {
        &self.local_settings
    }

    /// Get remote settings
    pub fn remote_settings(&self) -> &Settings {
        &self.remote_settings
    }

    /// Get stream by ID
    pub fn get_stream(&self, stream_id: u32) -> Option<&Stream> {
        self.streams.get_stream(stream_id)
    }

    /// Get mutable stream by ID
    pub fn get_stream_mut(&mut self, stream_id: u32) -> Option<&mut Stream> {
        self.streams.get_stream_mut(stream_id)
    }

    /// Get active stream count
    pub fn active_stream_count(&self) -> usize {
        self.streams.active_count()
    }
}

impl<S: Read + Write> Read for Connection<S> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        // Read next frame and extract data
        let frame = self.poll().map_err(io::Error::from)?;

        match frame {
            Some(frame) => {
                if frame.frame_type == FrameType::Data {
                    let data_frame = DataFrame::from_frame(&frame).map_err(io::Error::from)?;
                    let len = data_frame.data.len().min(buf.len());
                    buf[..len].copy_from_slice(&data_frame.data[..len]);
                    Ok(len)
                } else {
                    // Not a DATA frame, try again
                    self.read(buf)
                }
            }
            None => Ok(0),
        }
    }
}

impl<S: Read + Write> Write for Connection<S> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // Write as DATA frame on stream 1 (default)
        let data_frame = DataFrame::new(1, buf.to_vec(), false);
        let frame_bytes = data_frame.to_frame().to_bytes();
        self.stream.write_all(&frame_bytes)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.stream.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_connection_client_new() {
        // Create a mock stream that accepts the preface
        let mock = Cursor::new(Vec::new());
        let result = Connection::client(mock);
        assert!(result.is_ok());
    }

    #[test]
    fn test_connection_ping() {
        let mock = Cursor::new(Vec::new());
        let mut conn = Connection::client(mock).unwrap();
        let ping_id = conn.ping();
        assert!(ping_id.is_ok());
        assert_eq!(conn.pending_pings.len(), 1);
    }

    #[test]
    fn test_connection_state() {
        let mock = Cursor::new(Vec::new());
        let conn = Connection::client(mock).unwrap();
        assert_eq!(conn.state, ConnectionState::PrefaceSent);
        assert!(!conn.is_closed());
    }

    #[test]
    fn test_connection_settings() {
        let mock = Cursor::new(Vec::new());
        let conn = Connection::client(mock).unwrap();

        assert_eq!(conn.local_settings().max_frame_size, 16384);
        assert_eq!(conn.remote_settings().initial_window_size, 65535);
    }
}
