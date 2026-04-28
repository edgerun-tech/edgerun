use crate::prelude::v1::*;
use edgerun_encoding::byteorder::read_u16_le;

pub(crate) fn handle_notification(data: &[u8]) -> Option<(u16, Vec<u8>)> {
    if data.len() < 3 || data[0] != 0x1b {
        return None;
    }
    Some((read_u16_le(data, 1), data[3..].to_vec()))
}

pub(crate) fn handle_indication(data: &[u8]) -> Option<(u16, Vec<u8>)> {
    if data.len() < 3 || data[0] != 0x1d {
        return None;
    }
    Some((read_u16_le(data, 1), data[3..].to_vec()))
}

pub(crate) fn handle_execute_write_response(data: &[u8]) -> bool {
    data.first() == Some(&0x19)
}

pub(crate) fn parse_read_by_group_response(data: &[u8]) -> Vec<(u16, u16, Vec<u8>)> {
    let mut results = Vec::new();
    if data.len() < 2 || data[0] != 0x11 {
        return results;
    }
    let entry_size = data[1] as usize;
    if entry_size != 6 && entry_size != 20 {
        return results;
    }
    let mut offset = 2;
    while offset + entry_size <= data.len() {
        let start = read_u16_le(data, offset);
        let end = read_u16_le(data, offset + 2);
        let uuid = data[offset + 4..offset + entry_size].to_vec();
        results.push((start, end, uuid));
        offset += entry_size;
    }
    results
}

pub(crate) fn parse_read_by_type_response(data: &[u8]) -> Vec<(u16, Vec<u8>)> {
    let mut results = Vec::new();
    if data.len() < 2 || data[0] != 0x09 {
        return results;
    }
    let entry_size = data[1] as usize;
    if entry_size < 7 {
        return results;
    }
    let mut offset = 2;
    while offset + entry_size <= data.len() {
        let handle = read_u16_le(data, offset);
        let value = data[offset + 2..offset + entry_size].to_vec();
        results.push((handle, value));
        offset += entry_size;
    }
    results
}

pub(crate) fn parse_find_information_response(data: &[u8]) -> Vec<(u16, Vec<u8>)> {
    let mut results = Vec::new();
    if data.len() < 2 || data[0] != 0x05 {
        return results;
    }
    let uuid_size = match data[1] {
        0x01 => 2,
        0x02 => 16,
        _ => return results,
    };
    let entry_size = 2 + uuid_size;
    let mut offset = 2;
    while offset + entry_size <= data.len() {
        let handle = read_u16_le(data, offset);
        let uuid = data[offset + 2..offset + entry_size].to_vec();
        results.push((handle, uuid));
        offset += entry_size;
    }
    results
}

pub(crate) fn parse_error_response(data: &[u8]) -> Option<(u16, u8)> {
    if data.len() < 4 || data[0] != 0x01 {
        return None;
    }
    Some((read_u16_le(data, 1), data[3]))
}

pub(crate) fn parse_mtu_response(data: &[u8]) -> Option<u16> {
    if data.len() < 3 || data[0] != 0x03 {
        return None;
    }
    Some(read_u16_le(data, 1))
}
