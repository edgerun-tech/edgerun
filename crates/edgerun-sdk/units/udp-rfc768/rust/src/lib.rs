#![no_std]

edgerun_unit::no_alloc!();
edgerun_unit::metadata!(768);

#[edgerun_unit::export]
fn udp_minimum_length() -> i32 {
    8
}

#[edgerun_unit::export]
unsafe fn udp_parse(packet_ptr: i32, packet_len: i32, out_ptr: i32) -> i32 {
    if packet_ptr < 0 || packet_len < 0 || out_ptr < 0 {
        return 1;
    }
    let packet = core::slice::from_raw_parts(packet_ptr as *const u8, packet_len as usize);
    if packet.len() < 8 {
        return 1;
    }
    let src = u16::from_be_bytes([packet[0], packet[1]]) as u32;
    let dst = u16::from_be_bytes([packet[2], packet[3]]) as u32;
    let len = u16::from_be_bytes([packet[4], packet[5]]) as usize;
    let checksum = u16::from_be_bytes([packet[6], packet[7]]) as u32;
    if len < 8 || len > packet.len() {
        return 2;
    }
    let out = out_ptr as *mut u32;
    out.add(0).write_unaligned(src);
    out.add(1).write_unaligned(dst);
    out.add(2).write_unaligned(len as u32);
    out.add(3).write_unaligned(checksum);
    out.add(4).write_unaligned((packet_ptr + 8) as u32);
    out.add(5).write_unaligned((len - 8) as u32);
    0
}
