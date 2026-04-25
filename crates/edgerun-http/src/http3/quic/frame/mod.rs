//! QUIC frame types (RFC 9000 Section 19)

mod decode;
mod encode;
#[cfg(test)]
mod tests;
mod varint;

/// QUIC frame types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuicFrameType {
    Padding = 0x00,
    Ping = 0x01,
    Ack = 0x02,
    AckECN = 0x03,
    ResetStream = 0x04,
    StopSending = 0x05,
    Crypto = 0x06,
    NewToken = 0x07,
    Stream = 0x08,
    MaxData = 0x10,
    MaxStreamData = 0x11,
    MaxStreamsBidi = 0x12,
    MaxStreamsUni = 0x13,
    DataBlocked = 0x14,
    StreamDataBlocked = 0x15,
    StreamsBlockedBidi = 0x16,
    StreamsBlockedUni = 0x17,
    NewConnectionId = 0x18,
    RetireConnectionId = 0x19,
    PathChallenge = 0x1A,
    PathResponse = 0x1B,
    ConnectionClose = 0x1C,
    ConnectionCloseApplication = 0x1D,
    HandshakeDone = 0x1E,
}

impl QuicFrameType {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x00 => Some(QuicFrameType::Padding),
            0x01 => Some(QuicFrameType::Ping),
            0x02 => Some(QuicFrameType::Ack),
            0x03 => Some(QuicFrameType::AckECN),
            0x04 => Some(QuicFrameType::ResetStream),
            0x05 => Some(QuicFrameType::StopSending),
            0x06 => Some(QuicFrameType::Crypto),
            0x07 => Some(QuicFrameType::NewToken),
            0x10 => Some(QuicFrameType::MaxData),
            0x11 => Some(QuicFrameType::MaxStreamData),
            0x12 => Some(QuicFrameType::MaxStreamsBidi),
            0x13 => Some(QuicFrameType::MaxStreamsUni),
            0x14 => Some(QuicFrameType::DataBlocked),
            0x15 => Some(QuicFrameType::StreamDataBlocked),
            0x16 => Some(QuicFrameType::StreamsBlockedBidi),
            0x17 => Some(QuicFrameType::StreamsBlockedUni),
            0x18 => Some(QuicFrameType::NewConnectionId),
            0x19 => Some(QuicFrameType::RetireConnectionId),
            0x1A => Some(QuicFrameType::PathChallenge),
            0x1B => Some(QuicFrameType::PathResponse),
            0x1C => Some(QuicFrameType::ConnectionClose),
            0x1D => Some(QuicFrameType::ConnectionCloseApplication),
            0x1E => Some(QuicFrameType::HandshakeDone),
            v if (0x08..=0x0F).contains(&v) => Some(QuicFrameType::Stream),
            _ => None,
        }
    }
}

/// QUIC frame
#[derive(Debug, Clone)]
pub enum QuicFrame {
    Padding {
        length: usize,
    },
    Ping,
    Ack {
        largest_acknowledged: u64,
        ack_delay: u64,
        ack_range_count: u64,
        first_ack_range: u64,
        ack_ranges: Vec<(u64, u64)>,
    },
    AckECN {
        largest_acknowledged: u64,
        ack_delay: u64,
        ack_range_count: u64,
        first_ack_range: u64,
        ack_ranges: Vec<(u64, u64)>,
        ect0_count: u64,
        ect1_count: u64,
        ce_count: u64,
    },
    ResetStream {
        stream_id: u64,
        error_code: u64,
        final_size: u64,
    },
    StopSending {
        stream_id: u64,
        error_code: u64,
    },
    Crypto {
        offset: u64,
        data: Vec<u8>,
    },
    NewToken {
        token: Vec<u8>,
    },
    Stream {
        stream_id: u64,
        offset: u64,
        fin: bool,
        data: Vec<u8>,
    },
    MaxData {
        max_data: u64,
    },
    MaxStreamData {
        stream_id: u64,
        max_stream_data: u64,
    },
    MaxStreamsBidi {
        max_streams: u64,
    },
    MaxStreamsUni {
        max_streams: u64,
    },
    DataBlocked {
        max_data: u64,
    },
    StreamDataBlocked {
        stream_id: u64,
        max_stream_data: u64,
    },
    StreamsBlockedBidi {
        max_streams: u64,
    },
    StreamsBlockedUni {
        max_streams: u64,
    },
    NewConnectionId {
        sequence_number: u64,
        retire_prior_to: u64,
        connection_id: Vec<u8>,
        stateless_reset_token: [u8; 16],
    },
    RetireConnectionId {
        sequence_number: u64,
    },
    PathChallenge {
        data: [u8; 8],
    },
    PathResponse {
        data: [u8; 8],
    },
    ConnectionClose {
        error_code: u64,
        frame_type: u64,
        reason: Vec<u8>,
    },
    ConnectionCloseApplication {
        error_code: u64,
        reason: Vec<u8>,
    },
    HandshakeDone,
}

impl QuicFrame {
    /// Serialize frame to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        encode::to_bytes(self)
    }

    /// Parse frame from bytes
    pub fn from_bytes(data: &[u8]) -> Result<(Self, usize), String> {
        decode::from_bytes(data)
    }
}
