use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use edgerun_encoding::byteorder::{read_u16_le, read_u32_le};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GattUuid(pub String);

impl GattUuid {
    pub fn from_16(uuid: u16) -> Self {
        Self(format!("{:04x}", uuid))
    }

    pub fn as_16(&self) -> Option<u16> {
        let cleaned = self.0.replace('-', "").to_lowercase();
        if cleaned.len() == 4 {
            edgerun_encoding::hex::parse_hex_int(&cleaned)
        } else if cleaned.len() == 32 && cleaned.starts_with("0000") {
            edgerun_encoding::hex::parse_hex_int(&cleaned[4..8])
        } else {
            None
        }
    }

    pub fn from_hex(hex: &str) -> Option<Self> {
        let cleaned = hex.replace('-', "").to_lowercase();
        if matches!(cleaned.len(), 4 | 32) && edgerun_encoding::hex::hex_to_bytes(&cleaned).is_ok()
        {
            Some(Self(cleaned))
        } else {
            None
        }
    }

    pub fn as_128_bits(&self) -> Option<String> {
        let cleaned = self.0.replace('-', "").to_lowercase();
        if cleaned.len() == 4 {
            Some(format!("0000{}00001000800000805f9b34fb", cleaned))
        } else if cleaned.len() == 32 {
            Some(format!(
                "{}-{}-{}-{}-{}",
                &cleaned[0..8],
                &cleaned[8..12],
                &cleaned[12..16],
                &cleaned[16..20],
                &cleaned[20..32]
            ))
        } else {
            None
        }
    }

    pub fn matches_16(&self, uuid: u16) -> bool {
        if let Some(val) = self.as_16() {
            val == uuid
        } else {
            let cleaned = self.0.replace('-', "").to_lowercase();
            cleaned.ends_with(&format!("{:04x}", uuid))
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GattAttributeType {
    PrimaryService,
    SecondaryService,
    Include,
    Characteristic,
    CharacteristicExtended,
    Descriptor,
    Unknown(u16),
}

impl GattAttributeType {
    pub fn from_uuid(uuid: u16) -> Self {
        match uuid {
            0x2800 => Self::PrimaryService,
            0x2801 => Self::SecondaryService,
            0x2802 => Self::Include,
            0x2803 => Self::Characteristic,
            0x2900..=0x29ff => Self::Descriptor,
            _ => Self::Unknown(uuid),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GattProperty {
    Broadcast,
    Read,
    WriteNoResponse,
    Write,
    Notify,
    Indicate,
    SignedWrite,
    Extended,
}

impl GattProperty {
    pub fn from_bits(bits: u8) -> Vec<Self> {
        let mut props = Vec::new();
        if bits & 0x01 != 0 {
            props.push(Self::Broadcast);
        }
        if bits & 0x02 != 0 {
            props.push(Self::Read);
        }
        if bits & 0x04 != 0 {
            props.push(Self::WriteNoResponse);
        }
        if bits & 0x08 != 0 {
            props.push(Self::Write);
        }
        if bits & 0x10 != 0 {
            props.push(Self::Notify);
        }
        if bits & 0x20 != 0 {
            props.push(Self::Indicate);
        }
        if bits & 0x40 != 0 {
            props.push(Self::SignedWrite);
        }
        if bits & 0x80 != 0 {
            props.push(Self::Extended);
        }
        props
    }

    pub fn has_read(&self) -> bool {
        matches!(self, Self::Read)
    }

    pub fn has_write(&self) -> bool {
        matches!(self, Self::Write | Self::WriteNoResponse)
    }

    pub fn has_notify(&self) -> bool {
        matches!(self, Self::Notify | Self::Indicate)
    }
}

pub const UUID_CLIENT_CHARACTERISTIC_CONFIGURATION: u16 = 0x2902;
pub const UUID_SERVER_CHARACTERISTIC_CONFIGURATION: u16 = 0x2903;
pub const UUID_CHARACTERISTIC_USER_DESCRIPTION: u16 = 0x2901;
pub const UUID_GATT_CHARACTERISTIC_EXTENDED_PROPERTIES: u16 = 0x2900;

pub fn format_gatt_uuid(bytes: &[u8]) -> String {
    match bytes.len() {
        2 => format!("{:04x}", read_u16_le(bytes, 0)),
        4 => format!("{:04x}-0000-1000-8000-00805f9b34fb", read_u32_le(bytes, 0)),
        16 => {
            let mut b = bytes.to_vec();
            b.reverse();
            let hex = b.iter().map(|b| format!("{:02x}", b)).collect::<String>();
            format!(
                "{}-{}-{}-{}-{}",
                &hex[0..8],
                &hex[8..12],
                &hex[12..16],
                &hex[16..20],
                &hex[20..32]
            )
        }
        _ => bytes
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>(),
    }
}

pub fn parse_gatt_uuid(uuid_str: &str) -> Option<GattUuid> {
    let cleaned = uuid_str.replace('-', "").to_lowercase();
    if cleaned.len() == 4 && cleaned.chars().all(|c| c.is_ascii_hexdigit()) {
        Some(GattUuid(cleaned))
    } else if cleaned.len() == 8 && cleaned.starts_with("0000") {
        Some(GattUuid(cleaned[4..].to_string()))
    } else if cleaned.len() == 32 && cleaned.chars().all(|c| c.is_ascii_hexdigit()) {
        Some(GattUuid(cleaned))
    } else {
        None
    }
}

pub fn parse_bdaddr_string(addr: &str) -> Option<[u8; 6]> {
    edgerun_encoding::hex::parse_mac(addr)
}

pub fn format_bdaddr_hex(bytes: &[u8]) -> String {
    edgerun_encoding::hex::format_mac_bytes(bytes).unwrap_or_default()
}

pub fn reverse_bdaddr(addr: &str) -> Option<[u8; 6]> {
    parse_bdaddr_string(addr).map(|mut a| {
        a.reverse();
        a
    })
}

pub fn handle_notification(data: &[u8]) -> Option<(u16, Vec<u8>)> {
    if data.len() < 3 || data[0] != 0x1b {
        return None;
    }
    Some((read_u16_le(data, 1), data[3..].to_vec()))
}

pub fn handle_indication(data: &[u8]) -> Option<(u16, Vec<u8>)> {
    if data.len() < 3 || data[0] != 0x1d {
        return None;
    }
    Some((read_u16_le(data, 1), data[3..].to_vec()))
}

pub fn handle_execute_write_response(data: &[u8]) -> bool {
    data.first() == Some(&0x19)
}

pub fn parse_read_by_group_response(data: &[u8]) -> Vec<(u16, u16, Vec<u8>)> {
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

pub fn parse_read_by_type_response(data: &[u8]) -> Vec<(u16, Vec<u8>)> {
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

pub fn parse_find_information_response(data: &[u8]) -> Vec<(u16, Vec<u8>)> {
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

pub fn parse_error_response(data: &[u8]) -> Option<(u16, u8)> {
    if data.len() < 4 || data[0] != 0x01 {
        return None;
    }
    Some((read_u16_le(data, 1), data[3]))
}

pub fn parse_mtu_response(data: &[u8]) -> Option<u16> {
    if data.len() < 3 || data[0] != 0x03 {
        return None;
    }
    Some(read_u16_le(data, 1))
}

pub fn parse_connection_complete(data: &[u8]) -> Option<(u8, u16)> {
    for i in 0..data.len() {
        if data[i] == 0x01 && i + 18 <= data.len() {
            return Some((data[i + 1], read_u16_le(data, i + 2)));
        }
        if data[i] == 0x13 && i + 11 <= data.len() {
            return Some((data[i + 1], read_u16_le(data, i + 2)));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn gatt_uuid_from_16() {
        let uuid = GattUuid::from_16(0x180D);
        assert_eq!(uuid.0, "180d");
        assert_eq!(uuid.as_16(), Some(0x180D));
    }

    #[test]
    fn format_128bit_uuid() {
        let bytes = [
            0xfb, 0x34, 0x9b, 0x5f, 0x80, 0x00, 0x00, 0x80, 0x00, 0x10, 0x00, 0x00, 0x02, 0xff,
            0x00, 0x00,
        ];
        assert_eq!(
            format_gatt_uuid(&bytes),
            "0000ff02-0000-1000-8000-00805f9b34fb"
        );
    }

    #[test]
    fn parses_att_responses() {
        assert_eq!(
            parse_error_response(&[0x01, 0x0A, 0x00, 0x0A]),
            Some((0x000A, 0x0A))
        );
        assert_eq!(
            parse_find_information_response(&[0x05, 0x01, 0x03, 0x00, 0x03, 0x28]),
            vec![(0x0003, vec![0x03, 0x28])]
        );
        assert_eq!(
            parse_read_by_group_response(&[0x11, 0x06, 0x01, 0x00, 0x08, 0x00, 0x00, 0x28]),
            vec![(0x0001, 0x0008, vec![0x00, 0x28])]
        );
    }

    #[test]
    fn maps_bdaddr() {
        assert_eq!(
            parse_bdaddr_string("AA:BB:CC:DD:EE:FF"),
            Some([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF])
        );
        assert_eq!(
            reverse_bdaddr("AA:BB:CC:DD:EE:FF"),
            Some([0xFF, 0xEE, 0xDD, 0xCC, 0xBB, 0xAA])
        );
        assert_eq!(
            format_bdaddr_hex(&[0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]),
            "aa:bb:cc:dd:ee:ff"
        );
    }
}
