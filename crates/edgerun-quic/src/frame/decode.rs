use super::varint::decode_varint_string as decode_varint;
use super::QuicFrame;
use super::QuicFrameType;
use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};

pub fn from_bytes(data: &[u8]) -> Result<(QuicFrame, usize), String> {
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
            let (offset, offset_len) = decode_varint(&data[1..])?;
            let pos = 1 + offset_len;
            let (data_len, data_len_len) = decode_varint(&data[pos..])?;
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
            let (stream_id, n) = decode_varint(&data[pos..])?;
            pos += n;

            let offset = if has_offset {
                let (v, n) = decode_varint(&data[pos..])?;
                pos += n;
                v
            } else {
                0
            };

            let (data_len, _n) = if has_length {
                let (v, n) = decode_varint(&data[pos..])?;
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
            let (max_data, n) = decode_varint(&data[1..])?;
            Ok((QuicFrame::MaxData { max_data }, 1 + n))
        }
        QuicFrameType::ConnectionClose => {
            let mut pos = 1;
            let (error_code, n) = decode_varint(&data[pos..])?;
            pos += n;
            let (frame_type_val, n) = decode_varint(&data[pos..])?;
            pos += n;
            let (reason_len, n) = decode_varint(&data[pos..])?;
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
        QuicFrameType::Ack => {
            let mut pos = 1;
            let (largest_acknowledged, n) = decode_varint(&data[pos..])?;
            pos += n;
            let (ack_delay, n) = decode_varint(&data[pos..])?;
            pos += n;
            let (ack_range_count, n) = decode_varint(&data[pos..])?;
            pos += n;
            let (first_ack_range, n) = decode_varint(&data[pos..])?;
            pos += n;
            let mut ack_ranges = Vec::new();
            for _ in 0..ack_range_count {
                if pos >= data.len() {
                    break;
                }
                let (gap, n) = decode_varint(&data[pos..])?;
                pos += n;
                if pos >= data.len() {
                    break;
                }
                let (additional, n) = decode_varint(&data[pos..])?;
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
        QuicFrameType::AckECN => {
            let mut pos = 1;
            let (largest_acknowledged, n) = decode_varint(&data[pos..])?;
            pos += n;
            let (ack_delay, n) = decode_varint(&data[pos..])?;
            pos += n;
            let (ack_range_count, n) = decode_varint(&data[pos..])?;
            pos += n;
            let (first_ack_range, n) = decode_varint(&data[pos..])?;
            pos += n;
            let mut ack_ranges = Vec::new();
            for _ in 0..ack_range_count {
                if pos >= data.len() {
                    break;
                }
                let (gap, n) = decode_varint(&data[pos..])?;
                pos += n;
                if pos >= data.len() {
                    break;
                }
                let (additional, n) = decode_varint(&data[pos..])?;
                pos += n;
                ack_ranges.push((gap, additional));
            }
            let (ect0_count, n) = decode_varint(&data[pos..])?;
            pos += n;
            let (ect1_count, n) = decode_varint(&data[pos..])?;
            pos += n;
            let (ce_count, n) = decode_varint(&data[pos..])?;
            pos += n;
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
        QuicFrameType::ResetStream => {
            let mut pos = 1;
            let (stream_id, n) = decode_varint(&data[pos..])?;
            pos += n;
            let (error_code, n) = decode_varint(&data[pos..])?;
            pos += n;
            let (final_size, n) = decode_varint(&data[pos..])?;
            pos += n;
            Ok((
                QuicFrame::ResetStream {
                    stream_id,
                    error_code,
                    final_size,
                },
                pos,
            ))
        }
        QuicFrameType::StopSending => {
            let mut pos = 1;
            let (stream_id, n) = decode_varint(&data[pos..])?;
            pos += n;
            let (error_code, n) = decode_varint(&data[pos..])?;
            pos += n;
            Ok((
                QuicFrame::StopSending {
                    stream_id,
                    error_code,
                },
                pos,
            ))
        }
        QuicFrameType::NewToken => {
            let mut pos = 1;
            let (token_len, n) = decode_varint(&data[pos..])?;
            pos += n;
            let end = (pos + token_len as usize).min(data.len());
            let token = data[pos..end].to_vec();
            pos = end;
            Ok((QuicFrame::NewToken { token }, pos))
        }
        QuicFrameType::MaxStreamData => {
            let mut pos = 1;
            let (stream_id, n) = decode_varint(&data[pos..])?;
            pos += n;
            let (max_stream_data, n) = decode_varint(&data[pos..])?;
            pos += n;
            Ok((
                QuicFrame::MaxStreamData {
                    stream_id,
                    max_stream_data,
                },
                pos,
            ))
        }
        QuicFrameType::MaxStreamsBidi => {
            let (max_streams, n) = decode_varint(&data[1..])?;
            Ok((QuicFrame::MaxStreamsBidi { max_streams }, 1 + n))
        }
        QuicFrameType::MaxStreamsUni => {
            let (max_streams, n) = decode_varint(&data[1..])?;
            Ok((QuicFrame::MaxStreamsUni { max_streams }, 1 + n))
        }
        QuicFrameType::DataBlocked => {
            let (max_data, n) = decode_varint(&data[1..])?;
            Ok((QuicFrame::DataBlocked { max_data }, 1 + n))
        }
        QuicFrameType::StreamDataBlocked => {
            let mut pos = 1;
            let (stream_id, n) = decode_varint(&data[pos..])?;
            pos += n;
            let (max_stream_data, n) = decode_varint(&data[pos..])?;
            pos += n;
            Ok((
                QuicFrame::StreamDataBlocked {
                    stream_id,
                    max_stream_data,
                },
                pos,
            ))
        }
        QuicFrameType::StreamsBlockedBidi => {
            let (max_streams, n) = decode_varint(&data[1..])?;
            Ok((QuicFrame::StreamsBlockedBidi { max_streams }, 1 + n))
        }
        QuicFrameType::StreamsBlockedUni => {
            let (max_streams, n) = decode_varint(&data[1..])?;
            Ok((QuicFrame::StreamsBlockedUni { max_streams }, 1 + n))
        }
        QuicFrameType::NewConnectionId => {
            let mut pos = 1;
            let (sequence_number, n) = decode_varint(&data[pos..])?;
            pos += n;
            let (retire_prior_to, n) = decode_varint(&data[pos..])?;
            pos += n;
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
        QuicFrameType::RetireConnectionId => {
            let (sequence_number, n) = decode_varint(&data[1..])?;
            Ok((QuicFrame::RetireConnectionId { sequence_number }, 1 + n))
        }
        QuicFrameType::PathChallenge => {
            if data.len() < 9 {
                return Err("PATH_CHALLENGE: too short".to_string());
            }
            let mut d = [0u8; 8];
            d.copy_from_slice(&data[1..9]);
            Ok((QuicFrame::PathChallenge { data: d }, 9))
        }
        QuicFrameType::PathResponse => {
            if data.len() < 9 {
                return Err("PATH_RESPONSE: too short".to_string());
            }
            let mut d = [0u8; 8];
            d.copy_from_slice(&data[1..9]);
            Ok((QuicFrame::PathResponse { data: d }, 9))
        }
        QuicFrameType::ConnectionCloseApplication => {
            let mut pos = 1;
            let (error_code, n) = decode_varint(&data[pos..])?;
            pos += n;
            let (reason_len, n) = decode_varint(&data[pos..])?;
            pos += n;
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
