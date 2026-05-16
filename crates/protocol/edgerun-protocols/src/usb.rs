//! USB standard descriptor codecs.
//!
//! This module owns USB descriptor bytes only. Device discovery, sysfs, usbfs,
//! endpoint transfer, and authorization belong to the runtime/adapter side.

use crate::prelude::*;
use edgerun_encoding::byteorder::read_u16_le;

pub const USB_DT_DEVICE: u8 = 0x01;
pub const USB_DT_CONFIG: u8 = 0x02;
pub const USB_DT_STRING: u8 = 0x03;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UsbDescriptorError {
    ShortDeviceDescriptor,
    InvalidDeviceDescriptorHeader,
    ShortConfigurationDescriptor,
    InvalidConfigurationDescriptorHeader,
    ConfigurationTotalLengthTooSmall,
    ShortConfigurationDescriptorPayload,
    InvalidStringDescriptorHeader,
    InvalidStringDescriptorLength,
    InvalidUtf16StringDescriptor,
}

impl core::fmt::Display for UsbDescriptorError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::ShortDeviceDescriptor => "short USB device descriptor",
            Self::InvalidDeviceDescriptorHeader => "invalid USB device descriptor header",
            Self::ShortConfigurationDescriptor => "short USB configuration descriptor",
            Self::InvalidConfigurationDescriptorHeader => {
                "invalid USB configuration descriptor header"
            }
            Self::ConfigurationTotalLengthTooSmall => "USB configuration total length too small",
            Self::ShortConfigurationDescriptorPayload => {
                "short USB configuration descriptor payload"
            }
            Self::InvalidStringDescriptorHeader => "invalid USB string descriptor header",
            Self::InvalidStringDescriptorLength => "invalid USB string descriptor length",
            Self::InvalidUtf16StringDescriptor => "invalid UTF-16LE USB string descriptor",
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UsbDeviceDescriptor {
    pub usb_version_bcd: u16,
    pub device_class: u8,
    pub device_subclass: u8,
    pub device_protocol: u8,
    pub max_packet_size0: u8,
    pub vendor_id: u16,
    pub product_id: u16,
    pub device_version_bcd: u16,
    pub manufacturer_index: u8,
    pub product_index: u8,
    pub serial_number_index: u8,
    pub num_configurations: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UsbConfigurationDescriptor {
    pub total_length: u16,
    pub num_interfaces: u8,
    pub configuration_value: u8,
    pub configuration_index: u8,
    pub attributes: u8,
    pub max_power_2ma: u8,
    pub extra_descriptors: Vec<u8>,
}

pub fn parse_usb_device_descriptor(
    bytes: &[u8],
) -> Result<UsbDeviceDescriptor, UsbDescriptorError> {
    if bytes.len() < 18 {
        return Err(UsbDescriptorError::ShortDeviceDescriptor);
    }
    if bytes[0] != 18 || bytes[1] != USB_DT_DEVICE {
        return Err(UsbDescriptorError::InvalidDeviceDescriptorHeader);
    }
    Ok(UsbDeviceDescriptor {
        usb_version_bcd: read_u16_le(bytes, 2),
        device_class: bytes[4],
        device_subclass: bytes[5],
        device_protocol: bytes[6],
        max_packet_size0: bytes[7],
        vendor_id: read_u16_le(bytes, 8),
        product_id: read_u16_le(bytes, 10),
        device_version_bcd: read_u16_le(bytes, 12),
        manufacturer_index: bytes[14],
        product_index: bytes[15],
        serial_number_index: bytes[16],
        num_configurations: bytes[17],
    })
}

pub fn parse_usb_configuration_descriptor(
    bytes: &[u8],
) -> Result<UsbConfigurationDescriptor, UsbDescriptorError> {
    if bytes.len() < 9 {
        return Err(UsbDescriptorError::ShortConfigurationDescriptor);
    }
    if bytes[1] != USB_DT_CONFIG {
        return Err(UsbDescriptorError::InvalidConfigurationDescriptorHeader);
    }
    let total_length = read_u16_le(bytes, 2);
    if total_length < 9 {
        return Err(UsbDescriptorError::ConfigurationTotalLengthTooSmall);
    }
    if bytes.len() < total_length as usize {
        return Err(UsbDescriptorError::ShortConfigurationDescriptorPayload);
    }
    Ok(UsbConfigurationDescriptor {
        total_length,
        num_interfaces: bytes[4],
        configuration_value: bytes[5],
        configuration_index: bytes[6],
        attributes: bytes[7],
        max_power_2ma: bytes[8],
        extra_descriptors: bytes[9..total_length as usize].to_vec(),
    })
}

pub fn parse_usb_utf16le_string_descriptor(bytes: &[u8]) -> Result<String, UsbDescriptorError> {
    if bytes.len() < 2 || bytes[1] != USB_DT_STRING || bytes[0] as usize > bytes.len() {
        return Err(UsbDescriptorError::InvalidStringDescriptorHeader);
    }
    let declared_len = bytes[0] as usize;
    if declared_len < 2 || !(declared_len - 2).is_multiple_of(2) {
        return Err(UsbDescriptorError::InvalidStringDescriptorLength);
    }
    let mut units = Vec::new();
    for chunk in bytes[2..declared_len].chunks_exact(2) {
        units.push(read_u16_le(chunk, 0));
    }
    String::from_utf16(&units).map_err(|_| UsbDescriptorError::InvalidUtf16StringDescriptor)
}

pub fn parse_usb_language_ids(bytes: &[u8]) -> Result<Vec<u16>, UsbDescriptorError> {
    if bytes.len() < 2 || bytes[1] != USB_DT_STRING || bytes[0] as usize > bytes.len() {
        return Err(UsbDescriptorError::InvalidStringDescriptorHeader);
    }
    let declared_len = bytes[0] as usize;
    if declared_len < 4 || !(declared_len - 2).is_multiple_of(2) {
        return Err(UsbDescriptorError::InvalidStringDescriptorLength);
    }
    let mut out = Vec::new();
    for chunk in bytes[2..declared_len].chunks_exact(2) {
        out.push(read_u16_le(chunk, 0));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn parses_device_descriptor() {
        let raw = [
            18, 1, 0, 2, 0xff, 0, 0, 64, 0xc6, 0x27, 0x9c, 0x60, 0, 1, 1, 2, 3, 1,
        ];
        let desc = parse_usb_device_descriptor(&raw).unwrap();
        assert_eq!(desc.vendor_id, 0x27c6);
        assert_eq!(desc.product_id, 0x609c);
        assert_eq!(desc.max_packet_size0, 64);
    }

    #[test]
    fn parses_configuration_descriptor() {
        let raw = [9, 2, 9, 0, 1, 1, 0, 0x80, 50];
        let desc = parse_usb_configuration_descriptor(&raw).unwrap();
        assert_eq!(desc.total_length, 9);
        assert_eq!(desc.num_interfaces, 1);
    }

    #[test]
    fn parses_string_descriptor() {
        let raw = [8, 3, b'T', 0, b'e', 0, b's', 0];
        let value = parse_usb_utf16le_string_descriptor(&raw).unwrap();
        assert_eq!(value, "Tes");
    }

    #[test]
    fn parses_language_ids() {
        let raw = [4, 3, 0x09, 0x04];
        assert_eq!(parse_usb_language_ids(&raw).unwrap(), vec![0x0409]);
    }
}
