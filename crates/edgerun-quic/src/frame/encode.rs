use super::varint::encode_varint;
use super::QuicFrame;
use alloc::{vec, vec::Vec};

pub fn to_bytes(frame: &QuicFrame) -> Vec<u8> {
    match frame {
        QuicFrame::Padding { length } => vec![0x00; *length],
        QuicFrame::Ping => vec![0x01],
        QuicFrame::HandshakeDone => vec![0x1E],
        QuicFrame::Crypto { offset, data } => {
            let mut output = vec![0x06];
            encode_varint(*offset, &mut output);
            encode_varint(data.len() as u64, &mut output);
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
            encode_varint(*stream_id, &mut output);
            encode_varint(*offset, &mut output);
            encode_varint(data.len() as u64, &mut output);
            output.extend_from_slice(data);
            output
        }
        QuicFrame::MaxData { max_data } => {
            let mut output = vec![0x10];
            encode_varint(*max_data, &mut output);
            output
        }
        QuicFrame::ConnectionClose {
            error_code,
            frame_type,
            reason,
        } => {
            let mut output = vec![0x1C];
            encode_varint(*error_code, &mut output);
            encode_varint(*frame_type, &mut output);
            encode_varint(reason.len() as u64, &mut output);
            output.extend_from_slice(reason);
            output
        }
        QuicFrame::Ack {
            largest_acknowledged,
            ack_delay,
            ack_range_count,
            first_ack_range,
            ack_ranges,
        } => {
            let mut output = vec![0x02];
            encode_varint(*largest_acknowledged, &mut output);
            encode_varint(*ack_delay, &mut output);
            encode_varint(*ack_range_count, &mut output);
            encode_varint(*first_ack_range, &mut output);
            for (gap, additional) in ack_ranges {
                encode_varint(*gap, &mut output);
                encode_varint(*additional, &mut output);
            }
            output
        }
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
            encode_varint(*largest_acknowledged, &mut output);
            encode_varint(*ack_delay, &mut output);
            encode_varint(*ack_range_count, &mut output);
            encode_varint(*first_ack_range, &mut output);
            for (gap, additional) in ack_ranges {
                encode_varint(*gap, &mut output);
                encode_varint(*additional, &mut output);
            }
            encode_varint(*ect0_count, &mut output);
            encode_varint(*ect1_count, &mut output);
            encode_varint(*ce_count, &mut output);
            output
        }
        QuicFrame::ResetStream {
            stream_id,
            error_code,
            final_size,
        } => {
            let mut output = vec![0x04];
            encode_varint(*stream_id, &mut output);
            encode_varint(*error_code, &mut output);
            encode_varint(*final_size, &mut output);
            output
        }
        QuicFrame::StopSending {
            stream_id,
            error_code,
        } => {
            let mut output = vec![0x05];
            encode_varint(*stream_id, &mut output);
            encode_varint(*error_code, &mut output);
            output
        }
        QuicFrame::NewToken { token } => {
            let mut output = vec![0x07];
            encode_varint(token.len() as u64, &mut output);
            output.extend_from_slice(token);
            output
        }
        QuicFrame::MaxStreamData {
            stream_id,
            max_stream_data,
        } => {
            let mut output = vec![0x11];
            encode_varint(*stream_id, &mut output);
            encode_varint(*max_stream_data, &mut output);
            output
        }
        QuicFrame::MaxStreamsBidi { max_streams } => {
            let mut output = vec![0x12];
            encode_varint(*max_streams, &mut output);
            output
        }
        QuicFrame::MaxStreamsUni { max_streams } => {
            let mut output = vec![0x13];
            encode_varint(*max_streams, &mut output);
            output
        }
        QuicFrame::DataBlocked { max_data } => {
            let mut output = vec![0x14];
            encode_varint(*max_data, &mut output);
            output
        }
        QuicFrame::StreamDataBlocked {
            stream_id,
            max_stream_data,
        } => {
            let mut output = vec![0x15];
            encode_varint(*stream_id, &mut output);
            encode_varint(*max_stream_data, &mut output);
            output
        }
        QuicFrame::StreamsBlockedBidi { max_streams } => {
            let mut output = vec![0x16];
            encode_varint(*max_streams, &mut output);
            output
        }
        QuicFrame::StreamsBlockedUni { max_streams } => {
            let mut output = vec![0x17];
            encode_varint(*max_streams, &mut output);
            output
        }
        QuicFrame::NewConnectionId {
            sequence_number,
            retire_prior_to,
            connection_id,
            stateless_reset_token,
        } => {
            let mut output = vec![0x18];
            encode_varint(*sequence_number, &mut output);
            encode_varint(*retire_prior_to, &mut output);
            output.push(connection_id.len() as u8);
            output.extend_from_slice(connection_id);
            output.extend_from_slice(stateless_reset_token);
            output
        }
        QuicFrame::RetireConnectionId { sequence_number } => {
            let mut output = vec![0x19];
            encode_varint(*sequence_number, &mut output);
            output
        }
        QuicFrame::PathChallenge { data } => {
            let mut output = vec![0x1A];
            output.extend_from_slice(data);
            output
        }
        QuicFrame::PathResponse { data } => {
            let mut output = vec![0x1B];
            output.extend_from_slice(data);
            output
        }
        QuicFrame::ConnectionCloseApplication { error_code, reason } => {
            let mut output = vec![0x1D];
            encode_varint(*error_code, &mut output);
            encode_varint(reason.len() as u64, &mut output);
            output.extend_from_slice(reason);
            output
        }
    }
}
