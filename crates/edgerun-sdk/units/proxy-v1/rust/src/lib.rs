#![no_std]

use edgerun_protocols::proxy::{socks5_reply, socks5_select_no_auth, ProxyError};

edgerun_unit::no_alloc!();
edgerun_unit::metadata!(1);

#[edgerun_unit::export]
unsafe fn socks5_select_no_auth_unit(ptr: i32, len: i32, out_ptr: i32) -> i32 {
    if ptr < 0 || len < 0 || out_ptr < 0 {
        return 1;
    }
    let input = core::slice::from_raw_parts(ptr as *const u8, len as usize);
    let reply = match socks5_select_no_auth(input) {
        Ok(reply) => reply,
        Err(err) => return proxy_error_code(err),
    };
    let out = out_ptr as *mut u8;
    out.add(0).write(reply[0]);
    out.add(1).write(reply[1]);
    0
}

#[edgerun_unit::export]
unsafe fn socks5_reply_build(rep: i32, out_ptr: i32) -> i32 {
    if !(0..=255).contains(&rep) || out_ptr < 0 {
        return 1;
    }
    let reply = socks5_reply(rep as u8);
    let out = out_ptr as *mut u8;
    let mut index = 0usize;
    while index < reply.len() {
        out.add(index).write(reply[index]);
        index += 1;
    }
    reply.len() as i32
}

fn proxy_error_code(err: ProxyError) -> i32 {
    match err {
        ProxyError::Empty => 2,
        ProxyError::InvalidUtf8 => 3,
        ProxyError::BadRequest => 4,
        ProxyError::UnsupportedVersion => 5,
        ProxyError::UnsupportedCommand => 6,
        ProxyError::UnsupportedAddressType => 7,
        ProxyError::NoAcceptableAuth => 8,
    }
}
