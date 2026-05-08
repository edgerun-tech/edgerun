#![no_std]

use edgerun_protocols::ethernet_ipv4::{checksum, ip_checksum, parse_packet};

edgerun_unit::no_alloc!();
edgerun_unit::metadata!(1);

#[edgerun_unit::export]
unsafe fn ethernet_ipv4_checksum(ptr: i32, len: i32) -> i32 {
    if ptr < 0 || len < 0 {
        return -1;
    }
    let input = core::slice::from_raw_parts(ptr as *const u8, len as usize);
    checksum(input) as i32
}

#[edgerun_unit::export]
unsafe fn ethernet_ipv4_ip_checksum(ptr: i32, len: i32) -> i32 {
    if ptr < 0 || len < 0 {
        return -1;
    }
    let input = core::slice::from_raw_parts(ptr as *const u8, len as usize);
    ip_checksum(input) as i32
}

#[edgerun_unit::export]
unsafe fn ethernet_ipv4_parse_packet(ptr: i32, len: i32, out_ptr: i32) -> i32 {
    if ptr < 0 || len < 0 || out_ptr < 0 {
        return 1;
    }
    let input = core::slice::from_raw_parts(ptr as *const u8, len as usize);
    let Some((eth, ip)) = parse_packet(input) else {
        return 2;
    };
    let out = out_ptr as *mut u32;
    out.add(0).write_unaligned(eth.ethertype as u32);
    out.add(1).write_unaligned(ip.ver_ihl as u32);
    out.add(2).write_unaligned(ip.tos as u32);
    out.add(3).write_unaligned(ip.len as u32);
    out.add(4).write_unaligned(ip.ttl as u32);
    out.add(5).write_unaligned(ip.proto as u32);
    out.add(6).write_unaligned(ip.checksum as u32);
    out.add(7).write_unaligned(u32::from_be_bytes(ip.src));
    out.add(8).write_unaligned(u32::from_be_bytes(ip.dst));
    0
}
