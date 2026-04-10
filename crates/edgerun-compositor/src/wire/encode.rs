//! Encode Wayland messages to bytes.

use super::{Message, align4};

/// Encode a message for sending to a client.
///
/// Returns the raw bytes (no file descriptors — use [`send_with_fds`] for fd passing).
pub fn encode(msg: &Message) -> Vec<u8> {
    let mut out = Vec::with_capacity(msg.size as usize);

    // Header
    out.extend_from_slice(&msg.sender_id.to_le_bytes());
    out.extend_from_slice(&msg.opcode.to_le_bytes());
    out.extend_from_slice(&msg.size.to_le_bytes());

    // Raw argument payload (already pre-encoded)
    out.extend_from_slice(&msg.args);

    out
}

/// Build a message with no arguments.
pub fn message_empty(sender_id: u32, opcode: u16) -> Message {
    Message {
        sender_id,
        opcode,
        size: 8, // header only
        args: Vec::new(),
        fds: Vec::new(),
    }
}

/// Build a message with a single uint argument.
pub fn message_uint(sender_id: u32, opcode: u16, value: u32) -> Message {
    Message {
        sender_id,
        opcode,
        size: 12, // 8 + 4
        args: value.to_le_bytes().to_vec(),
        fds: Vec::new(),
    }
}

/// Build a message with two uint arguments.
pub fn message_uint2(sender_id: u32, opcode: u16, a: u32, b: u32) -> Message {
    let mut args = Vec::with_capacity(8);
    args.extend_from_slice(&a.to_le_bytes());
    args.extend_from_slice(&b.to_le_bytes());
    Message {
        sender_id,
        opcode,
        size: 16,
        args,
        fds: Vec::new(),
    }
}

/// Build a message with three uint arguments.
pub fn message_uint3(sender_id: u32, opcode: u16, a: u32, b: u32, c: u32) -> Message {
    let mut args = Vec::with_capacity(12);
    args.extend_from_slice(&a.to_le_bytes());
    args.extend_from_slice(&b.to_le_bytes());
    args.extend_from_slice(&c.to_le_bytes());
    Message {
        sender_id,
        opcode,
        size: 20,
        args,
        fds: Vec::new(),
    }
}

/// Build a message with a uint and a string argument.
pub fn message_uint_string(sender_id: u32, opcode: u16, value: u32, s: &str) -> Message {
    let mut args = Vec::new();
    args.extend_from_slice(&value.to_le_bytes());
    encode_string(&mut args, s);
    Message {
        sender_id,
        opcode,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}

/// Build a message with a uint and a fixed-point argument.
pub fn message_uint_fixed(
    sender_id: u32,
    opcode: u16,
    value: u32,
    fixed: i32,
) -> Message {
    let mut args = Vec::with_capacity(8);
    args.extend_from_slice(&value.to_le_bytes());
    args.extend_from_slice(&fixed.to_le_bytes());
    Message {
        sender_id,
        opcode,
        size: 16,
        args,
        fds: Vec::new(),
    }
}

/// Build a message with a single fd (caller must use [`send_with_fds`]).
pub fn message_fd(sender_id: u32, opcode: u16, fd: i32) -> Message {
    Message {
        sender_id,
        opcode,
        size: 12, // 8 + 4
        args: [0u8; 4].to_vec(), // fd placeholder (actual fd sent via SCM_RIGHTS)
        fds: vec![fd],
    }
}

/// Build a message with uint + uint + string.
pub fn message_uint2_string(sender_id: u32, opcode: u16, a: u32, b: u32, s: &str) -> Message {
    let mut args = Vec::new();
    args.extend_from_slice(&a.to_le_bytes());
    args.extend_from_slice(&b.to_le_bytes());
    encode_string(&mut args, s);
    Message {
        sender_id,
        opcode,
        size: (8 + args.len()) as u16,
        args,
        fds: Vec::new(),
    }
}

/// Encode a Wayland string argument (length-prefixed, null-terminated, 4-byte aligned).
pub fn encode_string(buf: &mut Vec<u8>, s: &str) {
    let len = s.len() + 1; // +1 for null terminator
    let aligned = align4(len);
    buf.extend_from_slice(&(len as u32).to_le_bytes());
    buf.extend_from_slice(s.as_bytes());
    buf.push(0); // null terminator
    // Pad to 4-byte boundary
    for _ in 0..(aligned - len) {
        buf.push(0);
    }
}

/// Encode an array argument (length-prefixed, 4-byte aligned).
pub fn encode_array(buf: &mut Vec<u8>, data: &[u8]) {
    let len = data.len();
    let aligned = align4(len);
    buf.extend_from_slice(&(len as u32).to_le_bytes());
    buf.extend_from_slice(data);
    for _ in 0..(aligned - len) {
        buf.push(0);
    }
}
