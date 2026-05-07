use alloc::string::{String, ToString};
use alloc::vec::Vec;
use edgerun_encoding::byteorder::{read_u16_le, read_u32_le};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MgmtVersionInfo {
    pub version: u8,
    pub revision: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MgmtControllerSettings {
    pub powered: bool,
    pub connectable: bool,
    pub fast_connectable: bool,
    pub discoverable: bool,
    pub pairable: bool,
    pub link_security: bool,
    pub secure_simple_pairing: bool,
    pub bredr: bool,
    pub high_speed: bool,
    pub low_energy: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MgmtControllerInfo {
    pub index: u16,
    pub address: String,
    pub bluetooth_version: u8,
    pub manufacturer: u16,
    pub supported_settings_raw: u32,
    pub current_settings_raw: u32,
    pub supported_settings: MgmtControllerSettings,
    pub current_settings: MgmtControllerSettings,
    pub class_of_device: u32,
    pub name: String,
    pub short_name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MgmtEvent {
    pub opcode: u16,
    pub index: u16,
    pub payload: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BluetoothMgmtError {
    ShortEvent,
    TruncatedEvent,
    ShortVersionPayload,
    ShortIndexListPayload,
    ShortControllerInfoPayload,
    InvalidBdaddr,
}

pub type Result<T> = core::result::Result<T, BluetoothMgmtError>;

pub fn build_mgmt_packet(opcode: u16, index: u16, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(6 + payload.len());
    out.extend_from_slice(&opcode.to_le_bytes());
    out.extend_from_slice(&index.to_le_bytes());
    out.extend_from_slice(&(payload.len() as u16).to_le_bytes());
    out.extend_from_slice(payload);
    out
}

pub fn parse_mgmt_event(packet: &[u8]) -> Result<MgmtEvent> {
    if packet.len() < 6 {
        return Err(BluetoothMgmtError::ShortEvent);
    }
    let opcode = read_u16_le(packet, 0);
    let index = read_u16_le(packet, 2);
    let len = read_u16_le(packet, 4) as usize;
    if packet.len() < 6 + len {
        return Err(BluetoothMgmtError::TruncatedEvent);
    }
    Ok(MgmtEvent {
        opcode,
        index,
        payload: packet[6..6 + len].to_vec(),
    })
}

pub fn mgmt_status_name(status: u8) -> &'static str {
    match status {
        0x00 => "success",
        0x01 => "unknown-command",
        0x02 => "not-connected",
        0x03 => "failed",
        0x04 => "connect-failed",
        0x05 => "authentication-failed",
        0x06 => "not-paired",
        0x07 => "no-resources",
        0x08 => "timeout",
        0x09 => "already-connected",
        0x0a => "busy",
        0x0b => "rejected",
        0x0c => "not-supported",
        0x0d => "invalid-parameters",
        0x0e => "disconnected",
        0x0f => "not-powered",
        0x10 => "cancelled",
        0x11 => "invalid-index",
        0x12 => "rfkilled",
        _ => "unknown-status",
    }
}

pub fn format_bdaddr_le(bytes: &[u8]) -> String {
    edgerun_encoding::hex::format_bdaddr_le(bytes)
        .map(|addr| addr.to_ascii_uppercase())
        .unwrap_or_default()
}

pub fn parse_bdaddr(addr: &str) -> Option<[u8; 6]> {
    edgerun_encoding::hex::parse_bdaddr(addr)
}

pub fn parse_settings(bits: u32) -> MgmtControllerSettings {
    MgmtControllerSettings {
        powered: bits & (1 << 0) != 0,
        connectable: bits & (1 << 1) != 0,
        fast_connectable: bits & (1 << 2) != 0,
        discoverable: bits & (1 << 3) != 0,
        pairable: bits & (1 << 4) != 0,
        link_security: bits & (1 << 5) != 0,
        secure_simple_pairing: bits & (1 << 6) != 0,
        bredr: bits & (1 << 7) != 0,
        high_speed: bits & (1 << 8) != 0,
        low_energy: bits & (1 << 9) != 0,
    }
}

pub fn parse_controller_info(index: u16, payload: &[u8]) -> Result<MgmtControllerInfo> {
    if payload.len() < 6 + 1 + 2 + 4 + 4 + 3 + 249 + 11 {
        return Err(BluetoothMgmtError::ShortControllerInfoPayload);
    }
    let address = format_bdaddr_le(&payload[0..6]);
    let bluetooth_version = payload[6];
    let manufacturer = read_u16_le(payload, 7);
    let supported_settings_raw = read_u32_le(payload, 9);
    let current_settings_raw = read_u32_le(payload, 13);
    let class_of_device =
        u32::from(payload[17]) | (u32::from(payload[18]) << 8) | (u32::from(payload[19]) << 16);
    let name = edgerun_encoding::cstring::decode_c_string(&payload[20..269]).unwrap_or_default();
    let short_name =
        edgerun_encoding::cstring::decode_c_string(&payload[269..280]).unwrap_or_default();
    Ok(MgmtControllerInfo {
        index,
        address,
        bluetooth_version,
        manufacturer,
        supported_settings_raw,
        current_settings_raw,
        supported_settings: parse_settings(supported_settings_raw),
        current_settings: parse_settings(current_settings_raw),
        class_of_device,
        name,
        short_name,
    })
}

pub fn parse_version_payload(payload: &[u8]) -> Result<MgmtVersionInfo> {
    if payload.len() < 3 {
        return Err(BluetoothMgmtError::ShortVersionPayload);
    }
    Ok(MgmtVersionInfo {
        version: payload[0],
        revision: read_u16_le(payload, 1),
    })
}

pub fn parse_index_list_payload(payload: &[u8]) -> Result<Vec<u16>> {
    if payload.len() < 2 {
        return Err(BluetoothMgmtError::ShortIndexListPayload);
    }
    let count = read_u16_le(payload, 0) as usize;
    let mut out = Vec::new();
    for chunk in payload[2..].chunks_exact(2).take(count) {
        out.push(read_u16_le(chunk, 0));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn builds_packet() {
        assert_eq!(
            build_mgmt_packet(0x0023, 0, &[1]),
            vec![0x23, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01]
        );
    }

    #[test]
    fn parses_event() {
        let event = parse_mgmt_event(&[0x01, 0x00, 0x02, 0x00, 0x02, 0x00, 0xaa, 0xbb]).unwrap();
        assert_eq!(event.opcode, 0x0001);
        assert_eq!(event.index, 0x0002);
        assert_eq!(event.payload, vec![0xaa, 0xbb]);
        assert_eq!(
            parse_mgmt_event(&[0x00, 0x01]),
            Err(BluetoothMgmtError::ShortEvent)
        );
    }

    #[test]
    fn parses_controller_info() {
        let mut payload = vec![0u8; 280];
        payload[0..6].copy_from_slice(&[1, 2, 3, 4, 5, 6]);
        payload[6] = 9;
        payload[7..9].copy_from_slice(&0x1234u16.to_le_bytes());
        payload[9..13].copy_from_slice(&0x201u32.to_le_bytes());
        payload[13..17].copy_from_slice(&0x003u32.to_le_bytes());
        payload[17..20].copy_from_slice(&[0x0c, 0x02, 0x5a]);
        payload[20..24].copy_from_slice(b"dev\0");
        payload[269..272].copy_from_slice(b"d\0\0");
        let info = parse_controller_info(7, &payload).unwrap();
        assert_eq!(info.index, 7);
        assert_eq!(info.address, "06:05:04:03:02:01");
        assert_eq!(info.bluetooth_version, 9);
        assert_eq!(info.manufacturer, 0x1234);
        assert!(info.supported_settings.powered);
        assert!(info.supported_settings.low_energy);
        assert!(info.current_settings.powered);
        assert!(info.current_settings.connectable);
        assert_eq!(info.name, "dev");
        assert_eq!(info.short_name, "d");
    }

    #[test]
    fn parses_index_and_version_payloads() {
        assert_eq!(
            parse_version_payload(&[12, 0x34, 0x12]).unwrap(),
            MgmtVersionInfo {
                version: 12,
                revision: 0x1234
            }
        );
        assert_eq!(
            parse_index_list_payload(&[2, 0, 1, 0, 3, 0]).unwrap(),
            vec![1, 3]
        );
    }
}
