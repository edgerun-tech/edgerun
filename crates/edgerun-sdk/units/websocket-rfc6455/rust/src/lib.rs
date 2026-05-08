#![no_std]

use edgerun_protocols::websocket::{
    decode_frame_prefix, decode_payload_len, handshake_complete, WebSocketError,
    WebSocketFramePrefix,
};

edgerun_unit::no_alloc!();
edgerun_unit::metadata!(6455);

#[edgerun_unit::export]
unsafe fn websocket_handshake_complete(ptr: i32, len: i32) -> i32 {
    if ptr < 0 || len < 0 {
        return 0;
    }
    let input = core::slice::from_raw_parts(ptr as *const u8, len as usize);
    handshake_complete(input) as i32
}

#[edgerun_unit::export]
unsafe fn websocket_frame_prefix_decode(header_ptr: i32, header_len: i32, out_ptr: i32) -> i32 {
    if header_ptr < 0 || header_len < 0 || out_ptr < 0 {
        return 1;
    }
    let header = core::slice::from_raw_parts(header_ptr as *const u8, header_len as usize);
    let prefix = match decode_frame_prefix(header) {
        Ok(prefix) => prefix,
        Err(err) => return websocket_error_code(err),
    };
    let out = out_ptr as *mut u32;
    out.add(0).write_unaligned(prefix.fin as u32);
    out.add(1).write_unaligned(prefix.opcode as u32);
    out.add(2).write_unaligned(prefix.masked as u32);
    out.add(3).write_unaligned(prefix.payload_len_code as u32);
    out.add(4)
        .write_unaligned(prefix.extended_len_bytes() as u32);
    0
}

#[edgerun_unit::export]
unsafe fn websocket_payload_len_decode(
    payload_len_code: i32,
    extended_ptr: i32,
    extended_len: i32,
    max_len: i32,
    out_ptr: i32,
) -> i32 {
    if !(0..=127).contains(&payload_len_code)
        || extended_ptr < 0
        || extended_len < 0
        || max_len < 0
        || out_ptr < 0
    {
        return 1;
    }
    let extended = core::slice::from_raw_parts(extended_ptr as *const u8, extended_len as usize);
    let prefix = WebSocketFramePrefix {
        fin: true,
        opcode: 2,
        masked: false,
        payload_len_code: payload_len_code as u8,
    };
    let payload_len = match decode_payload_len(prefix, extended, max_len as usize) {
        Ok(payload_len) => payload_len,
        Err(err) => return websocket_error_code(err),
    };
    (out_ptr as *mut u32).write_unaligned(payload_len as u32);
    0
}

fn websocket_error_code(err: WebSocketError) -> i32 {
    match err {
        WebSocketError::TruncatedHeader => 2,
        WebSocketError::Fragmented => 3,
        WebSocketError::UnmaskedClientFrame => 4,
        WebSocketError::MissingKey => 5,
        WebSocketError::NotUpgrade => 6,
        WebSocketError::HandshakeTooLarge => 7,
        WebSocketError::InvalidUtf8 => 8,
        WebSocketError::ControlPayloadTooLarge => 9,
        WebSocketError::UnsupportedOpcode => 10,
        WebSocketError::MessageTooLarge => 11,
    }
}
