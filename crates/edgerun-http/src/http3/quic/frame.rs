//! QUIC frame types (RFC 9000 Section 19)

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
            v if v >= 0x08 && v <= 0x0F => Some(QuicFrameType::Stream),
            _ => None,
        }
    }
}

/// QUIC frame
#[derive(Debug, Clone)]
pub enum QuicFrame {
    /// PADDING frame
    Padding { length: usize },
    /// PING frame
    Ping,
    /// ACK frame
    Ack {
        largest_acknowledged: u64,
        ack_delay: u64,
        ack_range_count: u64,
        first_ack_range: u64,
        /// ACK ranges: (gap, additional_ack) pairs
        ack_ranges: Vec<(u64, u64)>,
    },
    /// ACK_ECN frame (same as ACK but with ECN counts)
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
    /// RESET_STREAM frame
    ResetStream {
        stream_id: u64,
        error_code: u64,
        final_size: u64,
    },
    /// STOP_SENDING frame
    StopSending {
        stream_id: u64,
        error_code: u64,
    },
    /// CRYPTO frame
    Crypto { offset: u64, data: Vec<u8> },
    /// NEW_TOKEN frame
    NewToken { token: Vec<u8> },
    /// STREAM frame
    Stream {
        stream_id: u64,
        offset: u64,
        fin: bool,
        data: Vec<u8>,
    },
    /// MAX_DATA frame
    MaxData { max_data: u64 },
    /// MAX_STREAM_DATA frame
    MaxStreamData { stream_id: u64, max_stream_data: u64 },
    /// MAX_STREAMS frame (bidirectional)
    MaxStreamsBidi { max_streams: u64 },
    /// MAX_STREAMS frame (unidirectional)
    MaxStreamsUni { max_streams: u64 },
    /// DATA_BLOCKED frame
    DataBlocked { max_data: u64 },
    /// STREAM_DATA_BLOCKED frame
    StreamDataBlocked { stream_id: u64, max_stream_data: u64 },
    /// STREAMS_BLOCKED frame (bidirectional)
    StreamsBlockedBidi { max_streams: u64 },
    /// STREAMS_BLOCKED frame (unidirectional)
    StreamsBlockedUni { max_streams: u64 },
    /// NEW_CONNECTION_ID frame
    NewConnectionId {
        sequence_number: u64,
        retire_prior_to: u64,
        connection_id: Vec<u8>,
        stateless_reset_token: [u8; 16],
    },
    /// RETIRE_CONNECTION_ID frame
    RetireConnectionId { sequence_number: u64 },
    /// PATH_CHALLENGE frame
    PathChallenge { data: [u8; 8] },
    /// PATH_RESPONSE frame
    PathResponse { data: [u8; 8] },
    /// CONNECTION_CLOSE frame (transport error)
    ConnectionClose {
        error_code: u64,
        frame_type: u64,
        reason: Vec<u8>,
    },
    /// CONNECTION_CLOSE frame (application error)
    ConnectionCloseApplication {
        error_code: u64,
        reason: Vec<u8>,
    },
    /// HANDSHAKE_DONE frame
    HandshakeDone,
}

impl QuicFrame {
    /// Serialize frame to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            QuicFrame::Padding { length } => vec![0x00; *length],
            QuicFrame::Ping => vec![0x01],
            QuicFrame::HandshakeDone => vec![0x1E],
            QuicFrame::Crypto { offset, data } => {
                let mut output = vec![0x06];
                Self::encode_varint(*offset, &mut output);
                Self::encode_varint(data.len() as u64, &mut output);
                output.extend_from_slice(data);
                output
            }
            QuicFrame::Stream {
                stream_id,
                offset,
                fin,
                data,
            } => {
                let mut type_byte = 0x08u8;
                if *fin {
                    type_byte |= 0x01;
                }
                type_byte |= 0x02; // Offset present
                type_byte |= 0x04; // Length present

                let mut output = vec![type_byte];
                Self::encode_varint(*stream_id, &mut output);
                Self::encode_varint(*offset, &mut output);
                Self::encode_varint(data.len() as u64, &mut output);
                output.extend_from_slice(data);
                output
            }
            QuicFrame::MaxData { max_data } => {
                let mut output = vec![0x10];
                Self::encode_varint(*max_data, &mut output);
                output
            }
            QuicFrame::ConnectionClose {
                error_code,
                frame_type,
                reason,
            } => {
                let mut output = vec![0x1C];
                Self::encode_varint(*error_code, &mut output);
                Self::encode_varint(*frame_type, &mut output);
                Self::encode_varint(reason.len() as u64, &mut output);
                output.extend_from_slice(reason);
                output
            }

            // ACK frame: type=0x02, largest_ack, ack_delay, range_count, first_range, ranges...
            QuicFrame::Ack {
                largest_acknowledged,
                ack_delay,
                ack_range_count,
                first_ack_range,
                ack_ranges,
            } => {
                let mut output = vec![0x02];
                Self::encode_varint(*largest_acknowledged, &mut output);
                Self::encode_varint(*ack_delay, &mut output);
                Self::encode_varint(*ack_range_count, &mut output);
                Self::encode_varint(*first_ack_range, &mut output);
                for (gap, additional) in ack_ranges {
                    Self::encode_varint(*gap, &mut output);
                    Self::encode_varint(*additional, &mut output);
                }
                output
            }
            // ACK_ECN frame: same as ACK + 3 ECN counts
            QuicFrame::AckECN {
                largest_acknowledged,
                ack_delay,
                ack_range_count,
                first_ack_range,
                ack_ranges,
                ect0_count,
                ect1_count,
                ce_count,
            } => {
                let mut output = vec![0x03];
                Self::encode_varint(*largest_acknowledged, &mut output);
                Self::encode_varint(*ack_delay, &mut output);
                Self::encode_varint(*ack_range_count, &mut output);
                Self::encode_varint(*first_ack_range, &mut output);
                for (gap, additional) in ack_ranges {
                    Self::encode_varint(*gap, &mut output);
                    Self::encode_varint(*additional, &mut output);
                }
                Self::encode_varint(*ect0_count, &mut output);
                Self::encode_varint(*ect1_count, &mut output);
                Self::encode_varint(*ce_count, &mut output);
                output
            }
            // RESET_STREAM: type=0x04, stream_id, error_code, final_size
            QuicFrame::ResetStream {
                stream_id,
                error_code,
                final_size,
            } => {
                let mut output = vec![0x04];
                Self::encode_varint(*stream_id, &mut output);
                Self::encode_varint(*error_code, &mut output);
                Self::encode_varint(*final_size, &mut output);
                output
            }
            // STOP_SENDING: type=0x05, stream_id, error_code
            QuicFrame::StopSending {
                stream_id,
                error_code,
            } => {
                let mut output = vec![0x05];
                Self::encode_varint(*stream_id, &mut output);
                Self::encode_varint(*error_code, &mut output);
                output
            }
            // NEW_TOKEN: type=0x07, token
            QuicFrame::NewToken { token } => {
                let mut output = vec![0x07];
                Self::encode_varint(token.len() as u64, &mut output);
                output.extend_from_slice(token);
                output
            }
            // MAX_STREAM_DATA: type=0x11, stream_id, max_stream_data
            QuicFrame::MaxStreamData {
                stream_id,
                max_stream_data,
            } => {
                let mut output = vec![0x11];
                Self::encode_varint(*stream_id, &mut output);
                Self::encode_varint(*max_stream_data, &mut output);
                output
            }
            // MAX_STREAMS (bidi): type=0x12, max_streams
            QuicFrame::MaxStreamsBidi { max_streams } => {
                let mut output = vec![0x12];
                Self::encode_varint(*max_streams, &mut output);
                output
            }
            // MAX_STREAMS (uni): type=0x13, max_streams
            QuicFrame::MaxStreamsUni { max_streams } => {
                let mut output = vec![0x13];
                Self::encode_varint(*max_streams, &mut output);
                output
            }
            // DATA_BLOCKED: type=0x14, max_data
            QuicFrame::DataBlocked { max_data } => {
                let mut output = vec![0x14];
                Self::encode_varint(*max_data, &mut output);
                output
            }
            // STREAM_DATA_BLOCKED: type=0x15, stream_id, max_stream_data
            QuicFrame::StreamDataBlocked {
                stream_id,
                max_stream_data,
            } => {
                let mut output = vec![0x15];
                Self::encode_varint(*stream_id, &mut output);
                Self::encode_varint(*max_stream_data, &mut output);
                output
            }
            // STREAMS_BLOCKED (bidi): type=0x16, max_streams
            QuicFrame::StreamsBlockedBidi { max_streams } => {
                let mut output = vec![0x16];
                Self::encode_varint(*max_streams, &mut output);
                output
            }
            // STREAMS_BLOCKED (uni): type=0x17, max_streams
            QuicFrame::StreamsBlockedUni { max_streams } => {
                let mut output = vec![0x17];
                Self::encode_varint(*max_streams, &mut output);
                output
            }
            // NEW_CONNECTION_ID: type=0x18, seq, retire_prior, len, cid, token
            QuicFrame::NewConnectionId {
                sequence_number,
                retire_prior_to,
                connection_id,
                stateless_reset_token,
            } => {
                let mut output = vec![0x18];
                Self::encode_varint(*sequence_number, &mut output);
                Self::encode_varint(*retire_prior_to, &mut output);
                output.push(connection_id.len() as u8);
                output.extend_from_slice(connection_id);
                output.extend_from_slice(stateless_reset_token);
                output
            }
            // RETIRE_CONNECTION_ID: type=0x19, sequence_number
            QuicFrame::RetireConnectionId { sequence_number } => {
                let mut output = vec![0x19];
                Self::encode_varint(*sequence_number, &mut output);
                output
            }
            // PATH_CHALLENGE: type=0x1A, 8-byte data
            QuicFrame::PathChallenge { data } => {
                let mut output = vec![0x1A];
                output.extend_from_slice(data);
                output
            }
            // PATH_RESPONSE: type=0x1B, 8-byte data
            QuicFrame::PathResponse { data } => {
                let mut output = vec![0x1B];
                output.extend_from_slice(data);
                output
            }
            // CONNECTION_CLOSE (application): type=0x1D, error_code, reason
            QuicFrame::ConnectionCloseApplication { error_code, reason } => {
                let mut output = vec![0x1D];
                Self::encode_varint(*error_code, &mut output);
                Self::encode_varint(reason.len() as u64, &mut output);
                output.extend_from_slice(reason);
                output
            }
        }
    }

    /// Parse frame from bytes
    pub fn from_bytes(data: &[u8]) -> Result<(Self, usize), String> {
        if data.is_empty() {
            return Err("Empty frame data".to_string());
        }

        let frame_type = QuicFrameType::from_u8(data[0])
            .ok_or_else(|| format!("Unknown frame type: 0x{:02x}", data[0]))?;

        match frame_type {
            QuicFrameType::Padding => {
                let mut length = 1;
                while length < data.len() && data[length] == 0x00 {
                    length += 1;
                }
                Ok((QuicFrame::Padding { length }, length))
            }
            QuicFrameType::Ping => Ok((QuicFrame::Ping, 1)),
            QuicFrameType::HandshakeDone => Ok((QuicFrame::HandshakeDone, 1)),
            QuicFrameType::Crypto => {
                let (offset, offset_len) = Self::decode_varint(&data[1..])?;
                let pos = 1 + offset_len;
                let (data_len, data_len_len) = Self::decode_varint(&data[pos..])?;
                let data_start = pos + data_len_len;
                let data_end = data_start + data_len as usize;
                if data_end > data.len() {
                    return Err("Crypto frame data incomplete".to_string());
                }
                let frame_data = data[data_start..data_end].to_vec();
                Ok((
                    QuicFrame::Crypto {
                        offset,
                        data: frame_data,
                    },
                    data_end,
                ))
            }
            QuicFrameType::Stream => {
                let type_byte = data[0];
                let fin = (type_byte & 0x01) != 0;
                let has_offset = (type_byte & 0x02) != 0;
                let has_length = (type_byte & 0x04) != 0;

                let mut pos = 1;
                let (stream_id, n) = Self::decode_varint(&data[pos..])?;
                pos += n;

                let offset = if has_offset {
                    let (v, n) = Self::decode_varint(&data[pos..])?;
                    pos += n;
                    v
                } else {
                    0
                };

                let (data_len, n) = if has_length {
                    let (v, n) = Self::decode_varint(&data[pos..])?;
                    pos += n;
                    (v, n)
                } else {
                    (data.len().saturating_sub(pos) as u64, 0)
                };

                let data_end = pos + data_len as usize;
                let frame_data = data[pos..data_end.min(data.len())].to_vec();

                Ok((
                    QuicFrame::Stream {
                        stream_id,
                        offset,
                        fin,
                        data: frame_data,
                    },
                    data_end.min(data.len()),
                ))
            }
            QuicFrameType::MaxData => {
                let (max_data, n) = Self::decode_varint(&data[1..])?;
                Ok((QuicFrame::MaxData { max_data }, 1 + n))
            }
            QuicFrameType::ConnectionClose => {
                let mut pos = 1;
                let (error_code, n) = Self::decode_varint(&data[pos..])?;
                pos += n;
                let (frame_type_val, n) = Self::decode_varint(&data[pos..])?;
                pos += n;
                let (reason_len, n) = Self::decode_varint(&data[pos..])?;
                pos += n;
                let reason = data[pos..(pos + reason_len as usize).min(data.len())].to_vec();
                pos += reason_len as usize;
                Ok((
                    QuicFrame::ConnectionClose {
                        error_code,
                        frame_type: frame_type_val,
                        reason,
                    },
                    pos,
                ))
            }

            // ACK: type=0x02, largest_ack, ack_delay, range_count, first_range, [ranges...]
            QuicFrameType::Ack => {
                let mut pos = 1;
                let (largest_acknowledged, n) = Self::decode_varint(&data[pos..])?;
                pos += n;
                let (ack_delay, n) = Self::decode_varint(&data[pos..])?;
                pos += n;
                let (ack_range_count, n) = Self::decode_varint(&data[pos..])?;
                pos += n;
                let (first_ack_range, n) = Self::decode_varint(&data[pos..])?;
                pos += n;
                let mut ack_ranges = Vec::new();
                for _ in 0..ack_range_count {
                    if pos >= data.len() {
                        break;
                    }
                    let (gap, n) = Self::decode_varint(&data[pos..])?;
                    pos += n;
                    if pos >= data.len() {
                        break;
                    }
                    let (additional, n) = Self::decode_varint(&data[pos..])?;
                    pos += n;
                    ack_ranges.push((gap, additional));
                }
                Ok((
                    QuicFrame::Ack {
                        largest_acknowledged,
                        ack_delay,
                        ack_range_count,
                        first_ack_range,
                        ack_ranges,
                    },
                    pos,
                ))
            }
            // ACK_ECN: same as ACK + ect0, ect1, ce counts
            QuicFrameType::AckECN => {
                let mut pos = 1;
                let (largest_acknowledged, n) = Self::decode_varint(&data[pos..])?;
                pos += n;
                let (ack_delay, n) = Self::decode_varint(&data[pos..])?;
                pos += n;
                let (ack_range_count, n) = Self::decode_varint(&data[pos..])?;
                pos += n;
                let (first_ack_range, n) = Self::decode_varint(&data[pos..])?;
                pos += n;
                let mut ack_ranges = Vec::new();
                for _ in 0..ack_range_count {
                    if pos >= data.len() { break; }
                    let (gap, n) = Self::decode_varint(&data[pos..])?; pos += n;
                    if pos >= data.len() { break; }
                    let (additional, n) = Self::decode_varint(&data[pos..])?; pos += n;
                    ack_ranges.push((gap, additional));
                }
                let (ect0_count, n) = Self::decode_varint(&data[pos..])?; pos += n;
                let (ect1_count, n) = Self::decode_varint(&data[pos..])?; pos += n;
                let (ce_count, n) = Self::decode_varint(&data[pos..])?; pos += n;
                Ok((
                    QuicFrame::AckECN {
                        largest_acknowledged,
                        ack_delay,
                        ack_range_count,
                        first_ack_range,
                        ack_ranges,
                        ect0_count,
                        ect1_count,
                        ce_count,
                    },
                    pos,
                ))
            }
            // RESET_STREAM: type=0x04, stream_id, error_code, final_size
            QuicFrameType::ResetStream => {
                let mut pos = 1;
                let (stream_id, n) = Self::decode_varint(&data[pos..])?; pos += n;
                let (error_code, n) = Self::decode_varint(&data[pos..])?; pos += n;
                let (final_size, n) = Self::decode_varint(&data[pos..])?; pos += n;
                Ok((QuicFrame::ResetStream { stream_id, error_code, final_size }, pos))
            }
            // STOP_SENDING: type=0x05, stream_id, error_code
            QuicFrameType::StopSending => {
                let mut pos = 1;
                let (stream_id, n) = Self::decode_varint(&data[pos..])?; pos += n;
                let (error_code, n) = Self::decode_varint(&data[pos..])?; pos += n;
                Ok((QuicFrame::StopSending { stream_id, error_code }, pos))
            }
            // NEW_TOKEN: type=0x07, token_len, token
            QuicFrameType::NewToken => {
                let mut pos = 1;
                let (token_len, n) = Self::decode_varint(&data[pos..])?; pos += n;
                let end = (pos + token_len as usize).min(data.len());
                let token = data[pos..end].to_vec();
                pos = end;
                Ok((QuicFrame::NewToken { token }, pos))
            }
            // MAX_STREAM_DATA: type=0x11, stream_id, max_stream_data
            QuicFrameType::MaxStreamData => {
                let mut pos = 1;
                let (stream_id, n) = Self::decode_varint(&data[pos..])?; pos += n;
                let (max_stream_data, n) = Self::decode_varint(&data[pos..])?; pos += n;
                Ok((QuicFrame::MaxStreamData { stream_id, max_stream_data }, pos))
            }
            // MAX_STREAMS (bidi): type=0x12, max_streams
            QuicFrameType::MaxStreamsBidi => {
                let (max_streams, n) = Self::decode_varint(&data[1..])?;
                Ok((QuicFrame::MaxStreamsBidi { max_streams }, 1 + n))
            }
            // MAX_STREAMS (uni): type=0x13, max_streams
            QuicFrameType::MaxStreamsUni => {
                let (max_streams, n) = Self::decode_varint(&data[1..])?;
                Ok((QuicFrame::MaxStreamsUni { max_streams }, 1 + n))
            }
            // DATA_BLOCKED: type=0x14, max_data
            QuicFrameType::DataBlocked => {
                let (max_data, n) = Self::decode_varint(&data[1..])?;
                Ok((QuicFrame::DataBlocked { max_data }, 1 + n))
            }
            // STREAM_DATA_BLOCKED: type=0x15, stream_id, max_stream_data
            QuicFrameType::StreamDataBlocked => {
                let mut pos = 1;
                let (stream_id, n) = Self::decode_varint(&data[pos..])?; pos += n;
                let (max_stream_data, n) = Self::decode_varint(&data[pos..])?; pos += n;
                Ok((QuicFrame::StreamDataBlocked { stream_id, max_stream_data }, pos))
            }
            // STREAMS_BLOCKED (bidi): type=0x16, max_streams
            QuicFrameType::StreamsBlockedBidi => {
                let (max_streams, n) = Self::decode_varint(&data[1..])?;
                Ok((QuicFrame::StreamsBlockedBidi { max_streams }, 1 + n))
            }
            // STREAMS_BLOCKED (uni): type=0x17, max_streams
            QuicFrameType::StreamsBlockedUni => {
                let (max_streams, n) = Self::decode_varint(&data[1..])?;
                Ok((QuicFrame::StreamsBlockedUni { max_streams }, 1 + n))
            }
            // NEW_CONNECTION_ID: type=0x18, seq, retire_prior, len, cid, token
            QuicFrameType::NewConnectionId => {
                let mut pos = 1;
                let (sequence_number, n) = Self::decode_varint(&data[pos..])?; pos += n;
                let (retire_prior_to, n) = Self::decode_varint(&data[pos..])?; pos += n;
                if pos >= data.len() {
                    return Err("NEW_CONNECTION_ID: missing CID length".to_string());
                }
                let cid_len = data[pos] as usize;
                pos += 1;
                if pos + cid_len > data.len() {
                    return Err("NEW_CONNECTION_ID: CID too long".to_string());
                }
                let connection_id = data[pos..pos + cid_len].to_vec();
                pos += cid_len;
                if pos + 16 > data.len() {
                    return Err("NEW_CONNECTION_ID: missing reset token".to_string());
                }
                let mut stateless_reset_token = [0u8; 16];
                stateless_reset_token.copy_from_slice(&data[pos..pos + 16]);
                pos += 16;
                Ok((
                    QuicFrame::NewConnectionId {
                        sequence_number,
                        retire_prior_to,
                        connection_id,
                        stateless_reset_token,
                    },
                    pos,
                ))
            }
            // RETIRE_CONNECTION_ID: type=0x19, sequence_number
            QuicFrameType::RetireConnectionId => {
                let (sequence_number, n) = Self::decode_varint(&data[1..])?;
                Ok((QuicFrame::RetireConnectionId { sequence_number }, 1 + n))
            }
            // PATH_CHALLENGE: type=0x1A, 8 bytes
            QuicFrameType::PathChallenge => {
                if data.len() < 9 {
                    return Err("PATH_CHALLENGE: too short".to_string());
                }
                let mut d = [0u8; 8];
                d.copy_from_slice(&data[1..9]);
                Ok((QuicFrame::PathChallenge { data: d }, 9))
            }
            // PATH_RESPONSE: type=0x1B, 8 bytes
            QuicFrameType::PathResponse => {
                if data.len() < 9 {
                    return Err("PATH_RESPONSE: too short".to_string());
                }
                let mut d = [0u8; 8];
                d.copy_from_slice(&data[1..9]);
                Ok((QuicFrame::PathResponse { data: d }, 9))
            }
            // CONNECTION_CLOSE (application): type=0x1D, error_code, reason_len, reason
            QuicFrameType::ConnectionCloseApplication => {
                let mut pos = 1;
                let (error_code, n) = Self::decode_varint(&data[pos..])?; pos += n;
                let (reason_len, n) = Self::decode_varint(&data[pos..])?; pos += n;
                let end = (pos + reason_len as usize).min(data.len());
                let reason = data[pos..end].to_vec();
                pos = end;
                Ok((
                    QuicFrame::ConnectionCloseApplication { error_code, reason },
                    pos,
                ))
            }
        }
    }

    fn encode_varint(value: u64, output: &mut Vec<u8>) {
        if value < 64 {
            output.push(value as u8);
        } else if value < 16384 {
            output.push(((value >> 8) as u8) | 0x40);
            output.push(value as u8);
        } else if value < 1073741824 {
            let bytes = (value as u32).to_be_bytes();
            // 4-byte varint: prefix 0b10, so first byte = 0x80 | upper 6 bits
            output.push(bytes[0] | 0x80);
            output.push(bytes[1]);
            output.push(bytes[2]);
            output.push(bytes[3]);
        } else {
            let bytes = value.to_be_bytes();
            // 8-byte varint: prefix 0b11, so first byte = 0xC0 | upper 6 bits
            output.push(bytes[0] | 0xC0);
            output.extend_from_slice(&bytes[1..]);
        }
    }

    fn decode_varint(data: &[u8]) -> Result<(u64, usize), String> {
        if data.is_empty() {
            return Err("Empty varint".to_string());
        }
        let first = data[0];
        let len = match first >> 6 {
            0 => 1,
            1 => 2,
            2 => 4,
            3 => 8,
            _ => return Err("Invalid varint".to_string()),
        };
        if data.len() < len {
            return Err("Incomplete varint".to_string());
        }
        let value = match len {
            1 => (first & 0x3F) as u64,
            2 => u16::from_be_bytes([first & 0x3F, data[1]]) as u64,
            4 => {
                let mut b = [first & 0x3F, data[1], data[2], data[3]];
                u32::from_be_bytes(b) as u64
            }
            8 => {
                let mut b: [u8; 8] = data[..8].try_into().unwrap();
                b[0] &= 0x3F;
                u64::from_be_bytes(b)
            }
            _ => unreachable!(),
        };
        Ok((value, len))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ping_frame() {
        let frame = QuicFrame::Ping;
        let bytes = frame.to_bytes();
        assert_eq!(bytes, vec![0x01]);

        let (parsed, len) = QuicFrame::from_bytes(&bytes).unwrap();
        assert!(matches!(parsed, QuicFrame::Ping));
        assert_eq!(len, 1);
    }

    #[test]
    fn test_padding_frame() {
        let frame = QuicFrame::Padding { length: 10 };
        let bytes = frame.to_bytes();
        assert_eq!(bytes.len(), 10);
        assert!(bytes.iter().all(|&b| b == 0));
    }

    #[test]
    fn test_stream_frame() {
        let frame = QuicFrame::Stream {
            stream_id: 0,
            offset: 0,
            fin: true,
            data: b"hello".to_vec(),
        };
        let bytes = frame.to_bytes();
        assert!(!bytes.is_empty());

        let (parsed, _) = QuicFrame::from_bytes(&bytes).unwrap();
        if let QuicFrame::Stream { fin, data, .. } = parsed {
            assert!(fin);
            assert_eq!(data, b"hello");
        } else {
            panic!("Expected Stream frame");
        }
    }

    #[test]
    fn test_crypto_frame() {
        let frame = QuicFrame::Crypto {
            offset: 0,
            data: vec![0x01, 0x02, 0x03],
        };
        let bytes = frame.to_bytes();
        assert_eq!(bytes[0], 0x06);
    }

    #[test]
    fn test_max_data_frame() {
        let frame = QuicFrame::MaxData { max_data: 65535 };
        let bytes = frame.to_bytes();
        assert_eq!(bytes[0], 0x10);
    }

    #[test]
    fn test_ack_frame_roundtrip() {
        let frame = QuicFrame::Ack {
            largest_acknowledged: 100,
            ack_delay: 50,
            ack_range_count: 1,
            first_ack_range: 10,
            ack_ranges: vec![(5, 20)],
        };
        let bytes = frame.to_bytes();
        assert_eq!(bytes[0], 0x02);
        let (parsed, _) = QuicFrame::from_bytes(&bytes).unwrap();
        if let QuicFrame::Ack { largest_acknowledged, .. } = parsed {
            assert_eq!(largest_acknowledged, 100);
        } else {
            panic!("Expected ACK frame");
        }
    }

    #[test]
    fn test_reset_stream_frame_roundtrip() {
        let frame = QuicFrame::ResetStream {
            stream_id: 4,
            error_code: 0,
            final_size: 1024,
        };
        let bytes = frame.to_bytes();
        assert_eq!(bytes[0], 0x04);
        let (parsed, _) = QuicFrame::from_bytes(&bytes).unwrap();
        if let QuicFrame::ResetStream { stream_id, final_size, .. } = parsed {
            assert_eq!(stream_id, 4);
            assert_eq!(final_size, 1024);
        } else {
            panic!("Expected ResetStream frame");
        }
    }

    #[test]
    fn test_stop_sending_frame_roundtrip() {
        let frame = QuicFrame::StopSending {
            stream_id: 8,
            error_code: 42,
        };
        let bytes = frame.to_bytes();
        assert_eq!(bytes[0], 0x05);
        let (parsed, _) = QuicFrame::from_bytes(&bytes).unwrap();
        if let QuicFrame::StopSending { stream_id, error_code } = parsed {
            assert_eq!(stream_id, 8);
            assert_eq!(error_code, 42);
        } else {
            panic!("Expected StopSending frame");
        }
    }

    #[test]
    fn test_new_token_frame_roundtrip() {
        let frame = QuicFrame::NewToken {
            token: vec![0xDE, 0xAD, 0xBE, 0xEF],
        };
        let bytes = frame.to_bytes();
        assert_eq!(bytes[0], 0x07);
        let (parsed, _) = QuicFrame::from_bytes(&bytes).unwrap();
        if let QuicFrame::NewToken { token } = parsed {
            assert_eq!(token, vec![0xDE, 0xAD, 0xBE, 0xEF]);
        } else {
            panic!("Expected NewToken frame");
        }
    }

    #[test]
    fn test_max_stream_data_roundtrip() {
        let frame = QuicFrame::MaxStreamData {
            stream_id: 4,
            max_stream_data: 8192,
        };
        let bytes = frame.to_bytes();
        assert_eq!(bytes[0], 0x11);
        let (parsed, _) = QuicFrame::from_bytes(&bytes).unwrap();
        if let QuicFrame::MaxStreamData { stream_id, max_stream_data } = parsed {
            assert_eq!(stream_id, 4);
            assert_eq!(max_stream_data, 8192);
        } else {
            panic!("Expected MaxStreamData frame");
        }
    }

    #[test]
    fn test_max_streams_bidi_roundtrip() {
        let frame = QuicFrame::MaxStreamsBidi { max_streams: 200 };
        let bytes = frame.to_bytes();
        assert_eq!(bytes[0], 0x12);
        let (parsed, _) = QuicFrame::from_bytes(&bytes).unwrap();
        if let QuicFrame::MaxStreamsBidi { max_streams } = parsed {
            assert_eq!(max_streams, 200);
        } else {
            panic!("Expected MaxStreamsBidi frame");
        }
    }

    #[test]
    fn test_max_streams_uni_roundtrip() {
        let frame = QuicFrame::MaxStreamsUni { max_streams: 100 };
        let bytes = frame.to_bytes();
        assert_eq!(bytes[0], 0x13);
        let (parsed, _) = QuicFrame::from_bytes(&bytes).unwrap();
        if let QuicFrame::MaxStreamsUni { max_streams } = parsed {
            assert_eq!(max_streams, 100);
        } else {
            panic!("Expected MaxStreamsUni frame");
        }
    }

    #[test]
    fn test_data_blocked_roundtrip() {
        let frame = QuicFrame::DataBlocked { max_data: 16384 };
        let bytes = frame.to_bytes();
        assert_eq!(bytes[0], 0x14);
        let (parsed, _) = QuicFrame::from_bytes(&bytes).unwrap();
        if let QuicFrame::DataBlocked { max_data } = parsed {
            assert_eq!(max_data, 16384);
        } else {
            panic!("Expected DataBlocked frame");
        }
    }

    #[test]
    fn test_stream_data_blocked_roundtrip() {
        let frame = QuicFrame::StreamDataBlocked {
            stream_id: 4,
            max_stream_data: 32768,
        };
        let bytes = frame.to_bytes();
        assert_eq!(bytes[0], 0x15);
        let (parsed, _) = QuicFrame::from_bytes(&bytes).unwrap();
        if let QuicFrame::StreamDataBlocked { stream_id, max_stream_data } = parsed {
            assert_eq!(stream_id, 4);
            assert_eq!(max_stream_data, 32768);
        } else {
            panic!("Expected StreamDataBlocked frame");
        }
    }

    #[test]
    fn test_streams_blocked_bidi_roundtrip() {
        let frame = QuicFrame::StreamsBlockedBidi { max_streams: 50 };
        let bytes = frame.to_bytes();
        assert_eq!(bytes[0], 0x16);
        let (parsed, _) = QuicFrame::from_bytes(&bytes).unwrap();
        if let QuicFrame::StreamsBlockedBidi { max_streams } = parsed {
            assert_eq!(max_streams, 50);
        } else {
            panic!("Expected StreamsBlockedBidi frame");
        }
    }

    #[test]
    fn test_streams_blocked_uni_roundtrip() {
        let frame = QuicFrame::StreamsBlockedUni { max_streams: 25 };
        let bytes = frame.to_bytes();
        assert_eq!(bytes[0], 0x17);
        let (parsed, _) = QuicFrame::from_bytes(&bytes).unwrap();
        if let QuicFrame::StreamsBlockedUni { max_streams } = parsed {
            assert_eq!(max_streams, 25);
        } else {
            panic!("Expected StreamsBlockedUni frame");
        }
    }

    #[test]
    fn test_new_connection_id_roundtrip() {
        let frame = QuicFrame::NewConnectionId {
            sequence_number: 1,
            retire_prior_to: 0,
            connection_id: vec![1, 2, 3, 4, 5, 6, 7, 8],
            stateless_reset_token: [0xAA; 16],
        };
        let bytes = frame.to_bytes();
        assert_eq!(bytes[0], 0x18);
        let (parsed, _) = QuicFrame::from_bytes(&bytes).unwrap();
        if let QuicFrame::NewConnectionId {
            sequence_number,
            retire_prior_to,
            connection_id,
            stateless_reset_token,
        } = parsed {
            assert_eq!(sequence_number, 1);
            assert_eq!(retire_prior_to, 0);
            assert_eq!(connection_id, vec![1, 2, 3, 4, 5, 6, 7, 8]);
            assert_eq!(stateless_reset_token, [0xAA; 16]);
        } else {
            panic!("Expected NewConnectionId frame");
        }
    }

    #[test]
    fn test_retire_connection_id_roundtrip() {
        let frame = QuicFrame::RetireConnectionId { sequence_number: 3 };
        let bytes = frame.to_bytes();
        assert_eq!(bytes[0], 0x19);
        let (parsed, _) = QuicFrame::from_bytes(&bytes).unwrap();
        if let QuicFrame::RetireConnectionId { sequence_number } = parsed {
            assert_eq!(sequence_number, 3);
        } else {
            panic!("Expected RetireConnectionId frame");
        }
    }

    #[test]
    fn test_path_challenge_roundtrip() {
        let frame = QuicFrame::PathChallenge {
            data: [0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88],
        };
        let bytes = frame.to_bytes();
        assert_eq!(bytes[0], 0x1A);
        assert_eq!(bytes.len(), 9);
        let (parsed, _) = QuicFrame::from_bytes(&bytes).unwrap();
        if let QuicFrame::PathChallenge { data } = parsed {
            assert_eq!(data, [0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88]);
        } else {
            panic!("Expected PathChallenge frame");
        }
    }

    #[test]
    fn test_path_response_roundtrip() {
        let frame = QuicFrame::PathResponse {
            data: [0xFF, 0xEE, 0xDD, 0xCC, 0xBB, 0xAA, 0x99, 0x88],
        };
        let bytes = frame.to_bytes();
        assert_eq!(bytes[0], 0x1B);
        let (parsed, _) = QuicFrame::from_bytes(&bytes).unwrap();
        if let QuicFrame::PathResponse { data } = parsed {
            assert_eq!(data, [0xFF, 0xEE, 0xDD, 0xCC, 0xBB, 0xAA, 0x99, 0x88]);
        } else {
            panic!("Expected PathResponse frame");
        }
    }

    #[test]
    fn test_connection_close_application_roundtrip() {
        let frame = QuicFrame::ConnectionCloseApplication {
            error_code: 0x0100,
            reason: b"application shutdown".to_vec(),
        };
        let bytes = frame.to_bytes();
        assert_eq!(bytes[0], 0x1D);
        let (parsed, _) = QuicFrame::from_bytes(&bytes).unwrap();
        if let QuicFrame::ConnectionCloseApplication { error_code, reason } = parsed {
            assert_eq!(error_code, 0x0100);
            assert_eq!(reason, b"application shutdown");
        } else {
            panic!("Expected ConnectionCloseApplication frame");
        }
    }

    #[test]
    fn test_ack_ecn_frame_roundtrip() {
        let frame = QuicFrame::AckECN {
            largest_acknowledged: 50,
            ack_delay: 10,
            ack_range_count: 0,
            first_ack_range: 5,
            ack_ranges: vec![],
            ect0_count: 30,
            ect1_count: 5,
            ce_count: 0,
        };
        let bytes = frame.to_bytes();
        assert_eq!(bytes[0], 0x03);
        let (parsed, _) = QuicFrame::from_bytes(&bytes).unwrap();
        if let QuicFrame::AckECN { ect0_count, ect1_count, ce_count, .. } = parsed {
            assert_eq!(ect0_count, 30);
            assert_eq!(ect1_count, 5);
            assert_eq!(ce_count, 0);
        } else {
            panic!("Expected AckECN frame");
        }
    }
}
