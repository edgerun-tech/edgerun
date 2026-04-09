//! QUIC transport layer (loss recovery, congestion control, flow control)

use super::frame::QuicFrame;
use super::packet::QuicPacket;
use super::{ConnectionId, TransportParameters};

/// QUIC transport connection state
pub struct QuicTransport {
    /// Local connection ID
    pub local_cid: ConnectionId,
    /// Remote connection ID
    pub remote_cid: ConnectionId,
    /// Transport parameters
    pub params: TransportParameters,
    /// Next packet number
    next_packet_number: u64,
    /// Connection-level flow control
    pub max_data: u64,
    /// Stream-level flow control
    pub max_stream_data: u64,
    /// Idle timeout timer
    last_activity: std::time::Instant,
    /// Path MTU
    mtu: usize,
    /// RTT estimate
    rtt_estimate: std::time::Duration,
}

impl QuicTransport {
    /// Create new transport
    pub fn new(local_cid: ConnectionId, remote_cid: ConnectionId) -> Self {
        QuicTransport {
            local_cid,
            remote_cid,
            params: TransportParameters::default(),
            next_packet_number: 0,
            max_data: 65535,
            max_stream_data: 65535,
            last_activity: std::time::Instant::now(),
            mtu: 1200,
            rtt_estimate: std::time::Duration::from_millis(100),
        }
    }

    /// Get next packet number
    pub fn next_packet_number(&mut self) -> u64 {
        let pn = self.next_packet_number;
        self.next_packet_number += 1;
        pn
    }

    /// Update last activity time
    pub fn update_activity(&mut self) {
        self.last_activity = std::time::Instant::now();
    }

    /// Check if connection is idle
    pub fn is_idle(&self, timeout: std::time::Duration) -> bool {
        self.last_activity.elapsed() > timeout
    }

    /// Create STREAM frame data
    pub fn create_stream_frame(
        &mut self,
        stream_id: u64,
        data: Vec<u8>,
        fin: bool,
    ) -> QuicFrame {
        QuicFrame::Stream {
            stream_id,
            offset: 0,
            fin,
            data,
        }
    }

    /// Create MAX_DATA frame
    pub fn create_max_data_frame(&self) -> QuicFrame {
        QuicFrame::MaxData {
            max_data: self.max_data,
        }
    }

    /// Process received frame
    pub fn process_frame(&mut self, frame: &QuicFrame) {
        match frame {
            QuicFrame::MaxData { max_data } => {
                self.max_data = *max_data;
            }
            QuicFrame::MaxStreamData {
                max_stream_data, ..
            } => {
                self.max_stream_data = *max_stream_data;
            }
            QuicFrame::ConnectionClose {
                error_code, reason, ..
            } => {
                // Connection closed by peer
                let _ = (error_code, reason);
            }
            _ => {}
        }
        self.update_activity();
    }

    /// Get RTT estimate
    pub fn rtt(&self) -> std::time::Duration {
        self.rtt_estimate
    }

    /// Get current MTU
    pub fn mtu(&self) -> usize {
        self.mtu
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transport_new() {
        let local = ConnectionId::random();
        let remote = ConnectionId::random();
        let transport = QuicTransport::new(local, remote);
        assert_eq!(transport.next_packet_number, 0);
        assert_eq!(transport.max_data, 65535);
    }

    #[test]
    fn test_packet_number_increment() {
        let local = ConnectionId::random();
        let remote = ConnectionId::random();
        let mut transport = QuicTransport::new(local, remote);
        assert_eq!(transport.next_packet_number(), 0);
        assert_eq!(transport.next_packet_number(), 1);
        assert_eq!(transport.next_packet_number(), 2);
    }

    #[test]
    fn test_idle_timeout() {
        let local = ConnectionId::random();
        let remote = ConnectionId::random();
        let transport = QuicTransport::new(local, remote);
        assert!(!transport.is_idle(std::time::Duration::from_secs(60)));
    }

    #[test]
    fn test_stream_frame_creation() {
        let local = ConnectionId::random();
        let remote = ConnectionId::random();
        let mut transport = QuicTransport::new(local, remote);
        let frame = transport.create_stream_frame(0, b"hello".to_vec(), false);
        if let QuicFrame::Stream { data, fin, .. } = frame {
            assert_eq!(data, b"hello");
            assert!(!fin);
        } else {
            panic!("Expected Stream frame");
        }
    }
}
