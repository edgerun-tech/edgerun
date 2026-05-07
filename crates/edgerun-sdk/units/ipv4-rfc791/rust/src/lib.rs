#![no_std]

edgerun_unit::no_alloc!();
edgerun_unit::metadata!(791);

#[edgerun_unit::export]
fn ipv4_minimum_length() -> i32 {
    20
}

#[edgerun_unit::export]
unsafe fn ipv4_parse(packet_ptr: i32, packet_len: i32, out_ptr: i32) -> i32 {
    if packet_ptr < 0 || packet_len < 0 || out_ptr < 0 {
        return 1;
    }
    let packet = core::slice::from_raw_parts(packet_ptr as *const u8, packet_len as usize);
    if packet.len() < 20 {
        return 1;
    }
    let version = packet[0] >> 4;
    let ihl = packet[0] & 0x0f;
    if version != 4 || ihl < 5 {
        return 2;
    }
    let header_len = ihl as usize * 4;
    if packet.len() < header_len {
        return 1;
    }
    let total_len = u16::from_be_bytes([packet[2], packet[3]]) as usize;
    if total_len < header_len || total_len > packet.len() {
        return 3;
    }
    let protocol = packet[9] as u32;
    let src = u32::from_be_bytes([packet[12], packet[13], packet[14], packet[15]]);
    let dst = u32::from_be_bytes([packet[16], packet[17], packet[18], packet[19]]);
    let out = out_ptr as *mut u32;
    out.add(0).write_unaligned(version as u32);
    out.add(1).write_unaligned(header_len as u32);
    out.add(2).write_unaligned(total_len as u32);
    out.add(3).write_unaligned(protocol);
    out.add(4).write_unaligned(src);
    out.add(5).write_unaligned(dst);
    out.add(6)
        .write_unaligned((packet_ptr + header_len as i32) as u32);
    out.add(7).write_unaligned((total_len - header_len) as u32);
    0
}
