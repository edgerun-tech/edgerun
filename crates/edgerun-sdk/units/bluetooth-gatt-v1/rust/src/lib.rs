#![no_std]

use edgerun_protocols::bluetooth_gatt::{
    handle_execute_write_response, parse_connection_complete, parse_error_response,
    parse_mtu_response,
};

edgerun_unit::no_alloc!();
edgerun_unit::metadata!(1);

#[edgerun_unit::export]
unsafe fn bluetooth_gatt_att_error_parse(ptr: i32, len: i32, out_ptr: i32) -> i32 {
    if ptr < 0 || len < 0 || out_ptr < 0 {
        return 1;
    }
    let input = core::slice::from_raw_parts(ptr as *const u8, len as usize);
    let Some((handle, code)) = parse_error_response(input) else {
        return 2;
    };
    let out = out_ptr as *mut u32;
    out.add(0).write_unaligned(handle as u32);
    out.add(1).write_unaligned(code as u32);
    0
}

#[edgerun_unit::export]
unsafe fn bluetooth_gatt_att_mtu_parse(ptr: i32, len: i32) -> i32 {
    if ptr < 0 || len < 0 {
        return -1;
    }
    let input = core::slice::from_raw_parts(ptr as *const u8, len as usize);
    parse_mtu_response(input).map(i32::from).unwrap_or(-2)
}

#[edgerun_unit::export]
unsafe fn bluetooth_gatt_execute_write_response(ptr: i32, len: i32) -> i32 {
    if ptr < 0 || len < 0 {
        return 0;
    }
    let input = core::slice::from_raw_parts(ptr as *const u8, len as usize);
    handle_execute_write_response(input) as i32
}

#[edgerun_unit::export]
unsafe fn bluetooth_gatt_connection_complete_parse(ptr: i32, len: i32, out_ptr: i32) -> i32 {
    if ptr < 0 || len < 0 || out_ptr < 0 {
        return 1;
    }
    let input = core::slice::from_raw_parts(ptr as *const u8, len as usize);
    let Some((status, handle)) = parse_connection_complete(input) else {
        return 2;
    };
    let out = out_ptr as *mut u32;
    out.add(0).write_unaligned(status as u32);
    out.add(1).write_unaligned(handle as u32);
    0
}
