//! HTTP/2 stream management

use super::Result;
use std::collections::VecDeque;

/// Stream states (RFC 7540 Section 5.1)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamState {
    /// Initial state
    Idle,
    /// Reserved (local) - after sending PUSH_PROMISE or HEADERS with priority
    ReservedLocal,
    /// Reserved (remote) - after receiving PUSH_PROMISE
    ReservedRemote,
    /// Open - after sending/receiving HEADERS
    Open,
    /// Half-closed (local) - after sending END_STREAM
    HalfClosedLocal,
    /// Half-closed (remote) - after receiving END_STREAM
    HalfClosedRemote,
    /// Closed
    Closed,
}

/// Stream priority information
#[derive(Debug, Clone)]
pub struct Priority {
    /// Stream dependency (0 means no dependency)
    pub stream_dependency: u32,
    /// Weight (1-256)
    pub weight: u8,
    /// Exclusive flag
    pub exclusive: bool,
}

impl Default for Priority {
    fn default() -> Self {
        Priority {
            stream_dependency: 0,
            weight: 16, // Default weight
            exclusive: false,
        }
    }
}

/// HTTP/2 stream
pub struct Stream {
    /// Stream ID
    pub id: u32,
    /// Current state
    pub state: StreamState,
    /// Local flow control window
    pub local_window: i64,
    /// Remote flow control window
    pub remote_window: i64,
    /// Priority information
    pub priority: Priority,
    /// Buffered data waiting to be sent
    pub send_buffer: VecDeque<Vec<u8>>,
    /// Received data not yet consumed
    pub recv_buffer: VecDeque<Vec<u8>>,
    /// Content length (if known)
    pub content_length: Option<u64>,
    /// Bytes sent
    pub bytes_sent: u64,
    /// Bytes received
    pub bytes_received: u64,
    /// End-of-stream sent
    pub end_stream_sent: bool,
    /// End-of-stream received
    pub end_stream_received: bool,
}

impl Stream {
    /// Create a new stream
    pub fn new(id: u32, initial_window_size: u32) -> Self {
        Stream {
            id,
            state: StreamState::Idle,
            local_window: initial_window_size as i64,
            remote_window: initial_window_size as i64,
            priority: Priority::default(),
            send_buffer: VecDeque::new(),
            recv_buffer: VecDeque::new(),
            content_length: None,
            bytes_sent: 0,
            bytes_received: 0,
            end_stream_sent: false,
            end_stream_received: false,
        }
    }

    /// Check if stream is client-initiated (odd ID)
    pub fn is_client_initiated(&self) -> bool {
        self.id % 2 == 1
    }

    /// Check if stream is server-initiated (even ID, push)
    pub fn is_server_initiated(&self) -> bool {
        self.id.is_multiple_of(2) && self.id > 0
    }

    /// Transition to Open state
    pub fn open(&mut self) -> Result<()> {
        match self.state {
            StreamState::Idle | StreamState::ReservedLocal | StreamState::ReservedRemote => {
                self.state = StreamState::Open;
                Ok(())
            }
            _ => Err(super::Http2Error::ProtocolViolation(format!(
                "Cannot open stream in {:?} state",
                self.state
            ))),
        }
    }

    /// Mark local half-closed (sent END_STREAM)
    pub fn half_close_local(&mut self) -> Result<()> {
        match self.state {
            StreamState::Open | StreamState::HalfClosedRemote => {
                self.state = if self.state == StreamState::HalfClosedRemote {
                    StreamState::Closed
                } else {
                    StreamState::HalfClosedLocal
                };
                self.end_stream_sent = true;
                Ok(())
            }
            _ => Err(super::Http2Error::ProtocolViolation(format!(
                "Cannot half-close local in {:?} state",
                self.state
            ))),
        }
    }

    /// Mark remote half-closed (received END_STREAM)
    pub fn half_close_remote(&mut self) -> Result<()> {
        match self.state {
            StreamState::Open | StreamState::HalfClosedLocal => {
                self.state = if self.state == StreamState::HalfClosedLocal {
                    StreamState::Closed
                } else {
                    StreamState::HalfClosedRemote
                };
                self.end_stream_received = true;
                Ok(())
            }
            _ => Err(super::Http2Error::ProtocolViolation(format!(
                "Cannot half-close remote in {:?} state",
                self.state
            ))),
        }
    }

    /// Close the stream
    pub fn close(&mut self) {
        self.state = StreamState::Closed;
    }

    /// Check if stream is closed
    pub fn is_closed(&self) -> bool {
        self.state == StreamState::Closed
    }

    /// Check if stream is active (not closed)
    pub fn is_active(&self) -> bool {
        !self.is_closed()
    }

    /// Queue data for sending
    pub fn queue_data(&mut self, data: Vec<u8>) {
        self.send_buffer.push_back(data);
    }

    /// Get next chunk of data to send
    pub fn pop_data(&mut self) -> Option<Vec<u8>> {
        let data = self.send_buffer.pop_front();
        if let Some(ref d) = data {
            self.bytes_sent += d.len() as u64;
        }
        data
    }

    /// Queue received data
    pub fn queue_received(&mut self, data: Vec<u8>) {
        self.bytes_received += data.len() as u64;
        self.recv_buffer.push_back(data);
    }

    /// Get next chunk of received data
    pub fn pop_received(&mut self) -> Option<Vec<u8>> {
        self.recv_buffer.pop_front()
    }

    /// Check if all data has been received
    pub fn is_recv_complete(&self) -> bool {
        self.end_stream_received
            && self.content_length.is_none_or(|cl| {
                self.bytes_received >= cl
            })
    }

    /// Check if all data has been sent
    pub fn is_send_complete(&self) -> bool {
        self.end_stream_sent && self.send_buffer.is_empty()
    }
}

/// Stream manager for multiplexing
pub struct StreamManager {
    /// Active streams
    streams: std::collections::HashMap<u32, Stream>,
    /// Maximum concurrent streams
    max_concurrent_streams: Option<u32>,
    /// Next client-initiated stream ID
    next_client_stream: u32,
    /// Next server-initiated stream ID
    next_server_stream: u32,
    /// Initial window size for new streams
    initial_window_size: u32,
}

impl StreamManager {
    /// Create a new stream manager
    pub fn new(initial_window_size: u32) -> Self {
        StreamManager {
            streams: std::collections::HashMap::new(),
            max_concurrent_streams: None,
            next_client_stream: 1, // Client streams are odd: 1, 3, 5, ...
            next_server_stream: 2, // Server streams are even: 2, 4, 6, ...
            initial_window_size,
        }
    }

    /// Create a new client-initiated stream
    pub fn create_client_stream(&mut self) -> Result<u32> {
        // Check max concurrent streams
        if let Some(max) = self.max_concurrent_streams {
            let active_count = self.streams.values().filter(|s| s.is_active()).count() as u32;
            if active_count >= max {
                return Err(super::Http2Error::ProtocolViolation(
                    "Max concurrent streams reached".to_string(),
                ));
            }
        }

        let stream_id = self.next_client_stream;
        let stream = Stream::new(stream_id, self.initial_window_size);
        self.streams.insert(stream_id, stream);

        // Next client stream ID (skip by 2 for odd numbers)
        self.next_client_stream = self.next_client_stream.saturating_add(2);

        Ok(stream_id)
    }


    /// Create a server-initiated stream (for PUSH_PROMISE)
    pub fn create_server_stream(&mut self, stream_id: u32) -> Result<u32> {
        // Server streams must be even
        if !stream_id.is_multiple_of(2) {
            return Err(super::Http2Error::ProtocolViolation(
                "Server stream ID must be even".to_string(),
            ));
        }

        let stream = Stream::new(stream_id, self.initial_window_size);
        self.streams.insert(stream_id, stream);

        // Advance next_server_stream past this one
        if stream_id >= self.next_server_stream {
            self.next_server_stream = stream_id.saturating_add(2);
        }

        Ok(stream_id)
    }



    /// Get or create a stream for received headers
    pub fn get_or_create_stream(&mut self, stream_id: u32) -> Result<&mut Stream> {
        if !self.streams.contains_key(&stream_id) {
            let stream = Stream::new(stream_id, self.initial_window_size);
            self.streams.insert(stream_id, stream);
        }

        self.streams
            .get_mut(&stream_id)
            .ok_or_else(|| super::Http2Error::ProtocolViolation("Stream not found".to_string()))
    }

    /// Get a stream by ID
    pub fn get_stream(&self, stream_id: u32) -> Option<&Stream> {
        self.streams.get(&stream_id)
    }

    /// Get a mutable stream by ID
    pub fn get_stream_mut(&mut self, stream_id: u32) -> Option<&mut Stream> {
        self.streams.get_mut(&stream_id)
    }

    /// Remove a closed stream
    pub fn remove_stream(&mut self, stream_id: u32) {
        self.streams.remove(&stream_id);
    }

    /// Clean up closed streams
    pub fn cleanup_closed(&mut self) {
        self.streams.retain(|_, s| !s.is_closed());
    }

    /// Get the number of active streams
    pub fn active_count(&self) -> usize {
        self.streams.values().filter(|s| s.is_active()).count()
    }

    /// Get all active stream IDs
    pub fn active_stream_ids(&self) -> Vec<u32> {
        self.streams
            .iter()
            .filter(|(_, s)| s.is_active())
            .map(|(id, _)| *id)
            .collect()
    }

    /// Set max concurrent streams
    pub fn set_max_concurrent_streams(&mut self, max: u32) {
        self.max_concurrent_streams = Some(max);
    }

    /// Update initial window size for all streams
    pub fn update_initial_window_size(&mut self, new_size: u32) {
        let delta = new_size as i64 - self.initial_window_size as i64;
        self.initial_window_size = new_size;
        for stream in self.streams.values_mut() {
            stream.local_window += delta;
            stream.remote_window += delta;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_new() {
        let stream = Stream::new(1, 65535);
        assert_eq!(stream.id, 1);
        assert_eq!(stream.state, StreamState::Idle);
        assert_eq!(stream.local_window, 65535);
        assert!(stream.is_client_initiated());
    }

    #[test]
    fn test_stream_state_transitions() {
        let mut stream = Stream::new(1, 65535);

        // Idle -> Open
        stream.open().unwrap();
        assert_eq!(stream.state, StreamState::Open);

        // Open -> HalfClosedLocal
        stream.half_close_local().unwrap();
        assert_eq!(stream.state, StreamState::HalfClosedLocal);

        // HalfClosedLocal -> Closed
        stream.half_close_remote().unwrap();
        assert_eq!(stream.state, StreamState::Closed);
    }

    #[test]
    fn test_stream_data_queue() {
        let mut stream = Stream::new(1, 65535);
        stream.queue_data(b"hello".to_vec());
        stream.queue_data(b"world".to_vec());

        assert_eq!(stream.send_buffer.len(), 2);

        let data = stream.pop_data().unwrap();
        assert_eq!(data, b"hello");

        let data = stream.pop_data().unwrap();
        assert_eq!(data, b"world");

        assert!(stream.pop_data().is_none());
    }

    #[test]
    fn test_stream_manager_create() {
        let mut manager = StreamManager::new(65535);

        let id1 = manager.create_client_stream().unwrap();
        assert_eq!(id1, 1);

        let id2 = manager.create_client_stream().unwrap();
        assert_eq!(id2, 3);

        assert_eq!(manager.active_count(), 2);
    }

    #[test]
    fn test_stream_manager_max_concurrent() {
        let mut manager = StreamManager::new(65535);
        manager.set_max_concurrent_streams(2);

        assert!(manager.create_client_stream().is_ok());
        assert!(manager.create_client_stream().is_ok());
        assert!(manager.create_client_stream().is_err());
    }

    #[test]
    fn test_stream_manager_cleanup() {
        let mut manager = StreamManager::new(65535);

        let id = manager.create_client_stream().unwrap();
        assert_eq!(manager.active_count(), 1);

        if let Some(stream) = manager.get_stream_mut(id) {
            stream.close();
        }

        manager.cleanup_closed();
        assert_eq!(manager.active_count(), 0);
    }

    #[test]
    fn test_create_server_stream() {
        let mut manager = StreamManager::new(65535);

        // Create a server-initiated stream (PUSH_PROMISE)
        let id = manager.create_server_stream(4).unwrap();
        assert_eq!(id, 4);
        assert!(manager.get_stream(4).is_some());

        // Odd stream ID should fail
        assert!(manager.create_server_stream(3).is_err());

        // Client streams still work
        let client = manager.create_client_stream().unwrap();
        assert_eq!(client, 1);
    }
}
