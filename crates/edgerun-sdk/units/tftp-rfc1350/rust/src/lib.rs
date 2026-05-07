#![no_std]

use edgerun_protocols::tftp::message::TftpOpcode;

edgerun_unit::no_alloc!();
edgerun_unit::metadata!(1350);

#[edgerun_unit::export]
fn tftp_minimum_length() -> i32 {
    2
}

#[edgerun_unit::export]
fn tftp_opcode_valid(opcode: i32) -> i32 {
    if opcode < 0 {
        return 0;
    }
    TftpOpcode::from_u16(opcode as u16).is_some() as i32
}

#[edgerun_unit::export]
unsafe fn tftp_parse(message_ptr: i32, message_len: i32, out_ptr: i32) -> i32 {
    if message_ptr < 0 || message_len < 0 || out_ptr < 0 {
        return 1;
    }
    let message = core::slice::from_raw_parts(message_ptr as *const u8, message_len as usize);
    if message.len() < 2 {
        return 1;
    }

    let opcode = u16::from_be_bytes([message[0], message[1]]);
    let Some(opcode_kind) = TftpOpcode::from_u16(opcode) else {
        return 2;
    };
    if opcode_kind == TftpOpcode::ACK && message.len() != 4 {
        return 3;
    }
    if opcode_kind == TftpOpcode::DATA && message.len() < 4 {
        return 4;
    }

    let out = out_ptr as *mut u32;
    out.add(0).write_unaligned(opcode as u32);
    out.add(1).write_unaligned((message_ptr + 2) as u32);
    out.add(2).write_unaligned((message_len - 2) as u32);
    0
}
