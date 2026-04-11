#![allow(dead_code)]
use edgerun_fingerprint::{
    validate_enroll_request, FingerprintCapture, FingerprintCapturePurpose,
    FingerprintEnrollProgress, FingerprintEnrollRequest, FingerprintEnrollmentSession,
    FingerprintError, FingerprintReader, FingerprintReaderInfo, FingerprintTemplateRecord,
    FingerprintVerification, FingerprintVerifyRequest,
};
use std::fs;
use std::fs::File;
use std::io;
use std::os::fd::AsRawFd;
use std::os::raw::{c_int, c_ulong, c_void};
use std::path::{Path, PathBuf};

pub(crate) const GOODIX_VENDOR_ID: u16 = 0x27c6;
pub(crate) const GOODIX_FRAMEWORK_13_PRODUCT_ID: u16 = 0x609c;

const USB_DIR_IN: u8 = 0x80;
const USB_TYPE_STANDARD: u8 = 0x00 << 5;
const USB_RECIP_DEVICE: u8 = 0x00;
const USB_REQ_GET_DESCRIPTOR: u8 = 0x06;
const USB_REQ_GET_CONFIGURATION: u8 = 0x08;
const USB_DT_DEVICE: u8 = 0x01;
const USB_DT_CONFIG: u8 = 0x02;
const USB_DT_STRING: u8 = 0x03;
const GOODIX_PACKAGE_CRC_SIZE: usize = 4;
const GOODIX_PACKAGE_HEADER_SIZE: usize = 8;
const GOODIX_RESPONSE_ACK_CMD: u8 = 0xaa;
pub(crate) const GOODIX_CMD_GET_VERSION: u8 = 0xd0;
pub(crate) const GOODIX_CMD_UPDATE_CONFIG: u8 = 0xc0;
pub(crate) const GOODIX_CMD_CAPTURE_DATA: u8 = 0xa2;
pub(crate) const GOODIX_CMD_IDENTIFY: u8 = 0xa5;
pub(crate) const GOODIX_CMD_ENROLL_INIT: u8 = 0xa1;
pub(crate) const GOODIX_CMD_ENROLL: u8 = 0xa0;
pub(crate) const GOODIX_CMD_CHECK_DUPLICATE: u8 = 0xa3;
pub(crate) const GOODIX_CMD_COMMIT_ENROLLMENT: u8 = 0xa4;
pub(crate) const GOODIX_CMD_GET_FINGERLIST: u8 = 0xa6;
pub(crate) const GOODIX_CMD_FINGER_MODE: u8 = 0xb0;
pub(crate) const GOODIX_CMD_POWER_BUTTON_SHIELD: u8 = 0xe0;
pub(crate) const GOODIX_CMD_DELETE_TEMPLATE: u8 = 0xa7;
pub(crate) const GOODIX_SUBCMD_DEFAULT: u8 = 0x00;
pub(crate) const GOODIX_SUBCMD_DELETE_ALL: u8 = 0x01;
pub(crate) const GOODIX_SUBCMD_WRITE_CFG_TO_FLASH: u8 = 0x01;
pub(crate) const GOODIX_SUBCMD_GET_FINGER_MODE: u8 = 0x00;
pub(crate) const GOODIX_SUBCMD_SET_FINGER_DOWN: u8 = 0x01;
pub(crate) const GOODIX_SUBCMD_SET_FINGER_UP: u8 = 0x02;
pub(crate) const GOODIX_SUBCMD_PWR_BTN_SHIELD_OFF: u8 = 0x00;
pub(crate) const GOODIX_SUBCMD_PWR_BTN_SHIELD_ON: u8 = 0x01;
const GOODIX_TIMEOUT_SEND_MS: u32 = 1000;
const GOODIX_TIMEOUT_ACK_MS: u32 = 2000;
const GOODIX_TIMEOUT_DATA_MS: u32 = 5000;
pub(crate) const GOODIX_SUCCESS: u8 = 0x00;
pub(crate) const GOODIX_FAILED: u8 = 0x80;
pub(crate) const GOODIX_ERROR_FINGER_ID_NOEXIST: u8 = 0x9c;
pub(crate) const GOODIX_ERROR_TEMPLATE_INCOMPLETE: u8 = 0xb8;
pub(crate) const GOODIX_ERROR_WAIT_FINGER_UP_TIMEOUT: u8 = 0xc7;
pub(crate) const GOODIX_ERROR_NO_AVAILABLE_SPACE: u8 = 0x8f;
const GOODIX_MAX_STORED_PRINTS: u8 = 20;
const GOODIX_SENSOR_CONFIG_SIZE: usize = 128;
const GOODIX_SENSOR_CONFIG_BODY_SIZE: usize = 26;
const GOODIX_SENSOR_CONFIG_RESERVED_SIZE: usize = 98;
const GOODIX_DEFAULT_SENSOR_CONFIG_BODY: [u8; GOODIX_SENSOR_CONFIG_BODY_SIZE] = [
    0x00, 0x00, 0x64, 0x50, 0x0f, 0x41, 0x08, 0x0a, 0x18, 0x00, 0x00, 0x23, 0x00, 0x00, 0x01, 0x01,
    0x00, 0x01, 0x01, 0x01, 0x01, 0x00, 0x01, 0x01, 0x05, 0x05,
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GoodixVersionInfo {
    pub format: [u8; 2],
    pub fwtype: [u8; 8],
    pub fwversion: [u8; 8],
    pub customer: [u8; 8],
    pub mcu: [u8; 8],
    pub sensor: [u8; 8],
    pub algversion: [u8; 8],
    pub interface: [u8; 8],
    pub protocol: [u8; 8],
    pub flash_version: [u8; 8],
    pub reserved: [u8; 38],
}

impl GoodixVersionInfo {
    pub fn firmware_type_string(&self) -> String {
        fixed_c_string(&self.fwtype)
    }
    pub fn firmware_version_string(&self) -> String {
        fixed_c_string(&self.fwversion)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GoodixPacketHeader {
    pub cmd0: u8,
    pub cmd1: u8,
    pub package_num: u8,
    pub reserved: u8,
    pub payload_len: u16,
    pub crc8: u8,
    pub rev_crc8: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GoodixPacket {
    pub header: GoodixPacketHeader,
    pub payload: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GoodixAck {
    pub result: u8,
    pub ack_cmd: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GoodixTemplate {
    pub template_type: u8,
    pub finger_index: u8,
    pub account_id: [u8; 32],
    pub template_id: [u8; 32],
    pub payload: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GoodixCaptureResponse {
    pub result: u8,
    pub image_quality: Option<u8>,
    pub image_coverage: Option<u8>,
}

impl GoodixCaptureResponse {
    pub fn is_success(&self) -> bool {
        self.result < GOODIX_FAILED
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GoodixIdentifyResult {
    pub matched: bool,
    pub result: u8,
    pub reject_detail: Option<u16>,
    pub score: Option<u32>,
    pub study: Option<u8>,
    pub template: Option<GoodixTemplate>,
}

impl GoodixIdentifyResult {
    pub fn is_success(&self) -> bool {
        self.result < GOODIX_FAILED || self.matched
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GoodixEnrollInitResult {
    pub result: u8,
    pub template_id: Option<[u8; 32]>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GoodixEnrollUpdateResult {
    pub rollback: bool,
    pub overlay: u8,
    pub preoverlay: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GoodixDuplicateCheckResult {
    pub duplicate: bool,
    pub template: Option<GoodixTemplate>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GoodixEnrollmentProgress {
    pub capture: GoodixCaptureResponse,
    pub update: GoodixEnrollUpdateResult,
    pub duplicate: GoodixDuplicateCheckResult,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GoodixSimpleResult {
    pub result: u8,
}

impl GoodixSimpleResult {
    pub fn is_success(&self) -> bool {
        self.result < GOODIX_FAILED
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GoodixFingerModeStatus {
    pub status: u8,
}

impl GoodixFingerModeStatus {
    pub fn is_success(&self) -> bool {
        self.status == GOODIX_SUCCESS
    }

    pub fn is_wait_finger_up_timeout(&self) -> bool {
        self.status == GOODIX_ERROR_WAIT_FINGER_UP_TIMEOUT
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GoodixFingerConfig {
    pub status: u8,
    pub max_stored_prints: u8,
}

impl GoodixFingerConfig {
    pub fn is_success(&self) -> bool {
        self.status < GOODIX_FAILED
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GoodixSensorConfig {
    pub config_body: [u8; GOODIX_SENSOR_CONFIG_BODY_SIZE],
    pub reserved: [u8; GOODIX_SENSOR_CONFIG_RESERVED_SIZE],
    pub crc32: [u8; 4],
}

impl GoodixSensorConfig {
    pub fn into_bytes(self) -> [u8; GOODIX_SENSOR_CONFIG_SIZE] {
        let mut out = [0u8; GOODIX_SENSOR_CONFIG_SIZE];
        out[..GOODIX_SENSOR_CONFIG_BODY_SIZE].copy_from_slice(&self.config_body);
        out[GOODIX_SENSOR_CONFIG_BODY_SIZE
            ..GOODIX_SENSOR_CONFIG_BODY_SIZE + GOODIX_SENSOR_CONFIG_RESERVED_SIZE]
            .copy_from_slice(&self.reserved);
        out[GOODIX_SENSOR_CONFIG_BODY_SIZE + GOODIX_SENSOR_CONFIG_RESERVED_SIZE..]
            .copy_from_slice(&self.crc32);
        out
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

const IOC_NRBITS: u32 = 8;
const IOC_TYPEBITS: u32 = 8;
const IOC_SIZEBITS: u32 = 14;
const IOC_NRSHIFT: u32 = 0;
const IOC_TYPESHIFT: u32 = IOC_NRSHIFT + IOC_NRBITS;
const IOC_SIZESHIFT: u32 = IOC_TYPESHIFT + IOC_TYPEBITS;
const IOC_DIRSHIFT: u32 = IOC_SIZESHIFT + IOC_SIZEBITS;
const IOC_WRITE: u32 = 1;
const IOC_READ: u32 = 2;
const USBDEVFS_TYPE: u8 = b'U';
const USBDEVFS_CONTROL: c_ulong = iowr::<UsbdevfsCtrlTransfer>(USBDEVFS_TYPE, 0) as c_ulong;
const USBDEVFS_BULK: c_ulong = iowr::<UsbdevfsBulkTransfer>(USBDEVFS_TYPE, 2) as c_ulong;
const USBDEVFS_CLAIMINTERFACE: c_ulong = iow::<CUInt>(USBDEVFS_TYPE, 15) as c_ulong;
const USBDEVFS_RELEASEINTERFACE: c_ulong = iow::<CUInt>(USBDEVFS_TYPE, 16) as c_ulong;

type CUInt = u32;

const fn ioc(dir: u32, ty: u8, nr: u8, size: usize) -> u32 {
    (dir << IOC_DIRSHIFT)
        | ((ty as u32) << IOC_TYPESHIFT)
        | ((nr as u32) << IOC_NRSHIFT)
        | ((size as u32) << IOC_SIZESHIFT)
}
const fn ior<T>(ty: u8, nr: u8) -> u32 {
    ioc(IOC_READ, ty, nr, core::mem::size_of::<T>())
}
const fn iow<T>(ty: u8, nr: u8) -> u32 {
    ioc(IOC_WRITE, ty, nr, core::mem::size_of::<T>())
}
const fn iowr<T>(ty: u8, nr: u8) -> u32 {
    ioc(IOC_READ | IOC_WRITE, ty, nr, core::mem::size_of::<T>())
}

unsafe extern "C" {
    fn ioctl(fd: c_int, request: c_ulong, ...) -> c_int;
}

#[repr(C)]
struct UsbdevfsCtrlTransfer {
    b_request_type: u8,
    b_request: u8,
    w_value: u16,
    w_index: u16,
    w_length: u16,
    timeout: u32,
    data: *mut c_void,
}

#[repr(C)]
struct UsbdevfsBulkTransfer {
    ep: u32,
    len: u32,
    timeout: u32,
    data: *mut c_void,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GoodixUsbDevice {
    pub bus_number: u8,
    pub device_number: u8,
    pub vendor_id: u16,
    pub product_id: u16,
    pub devnode: PathBuf,
    pub sysfs_path: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GoodixUsbInterface {
    pub number: u8,
    pub alt_setting: u8,
    pub endpoints: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GoodixUsbTransportInfo {
    pub interface_number: u8,
    pub bulk_in_endpoint: Option<u8>,
    pub bulk_out_endpoint: Option<u8>,
    pub interrupt_in_endpoint: Option<u8>,
}

#[derive(Debug)]
pub enum GoodixFingerprintError {
    Io(io::Error),
    Parse(String),
    UnsupportedDevice { vendor_id: u16, product_id: u16 },
    MissingUsbInterface,
    MissingUsbEndpoint(&'static str),
    Fingerprint(FingerprintError),
}

impl core::fmt::Display for GoodixFingerprintError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "I/O error: {err}"),
            Self::Parse(msg) => f.write_str(msg),
            Self::UnsupportedDevice {
                vendor_id,
                product_id,
            } => {
                write!(
                    f,
                    "unsupported Goodix fingerprint device {vendor_id:04x}:{product_id:04x}"
                )
            }
            Self::MissingUsbInterface => f.write_str("missing Goodix USB interface"),
            Self::MissingUsbEndpoint(kind) => write!(f, "missing Goodix USB endpoint: {kind}"),
            Self::Fingerprint(err) => write!(f, "fingerprint error: {err}"),
        }
    }
}

impl std::error::Error for GoodixFingerprintError {}

impl From<io::Error> for GoodixFingerprintError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<FingerprintError> for GoodixFingerprintError {
    fn from(value: FingerprintError) -> Self {
        Self::Fingerprint(value)
    }
}

fn le_u16(bytes: &[u8]) -> Result<u16, GoodixFingerprintError> {
    if bytes.len() < 2 {
        return Err(GoodixFingerprintError::Parse(
            "short little-endian u16 field".into(),
        ));
    }
    Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
}

pub fn parse_usb_device_descriptor(
    bytes: &[u8],
) -> Result<UsbDeviceDescriptor, GoodixFingerprintError> {
    if bytes.len() < 18 {
        return Err(GoodixFingerprintError::Parse(
            "short USB device descriptor".into(),
        ));
    }
    if bytes[0] != 18 || bytes[1] != USB_DT_DEVICE {
        return Err(GoodixFingerprintError::Parse(
            "invalid USB device descriptor header".into(),
        ));
    }
    Ok(UsbDeviceDescriptor {
        usb_version_bcd: le_u16(&bytes[2..4])?,
        device_class: bytes[4],
        device_subclass: bytes[5],
        device_protocol: bytes[6],
        max_packet_size0: bytes[7],
        vendor_id: le_u16(&bytes[8..10])?,
        product_id: le_u16(&bytes[10..12])?,
        device_version_bcd: le_u16(&bytes[12..14])?,
        manufacturer_index: bytes[14],
        product_index: bytes[15],
        serial_number_index: bytes[16],
        num_configurations: bytes[17],
    })
}

pub fn parse_usb_configuration_descriptor(
    bytes: &[u8],
) -> Result<UsbConfigurationDescriptor, GoodixFingerprintError> {
    if bytes.len() < 9 {
        return Err(GoodixFingerprintError::Parse(
            "short USB configuration descriptor".into(),
        ));
    }
    if bytes[1] != USB_DT_CONFIG {
        return Err(GoodixFingerprintError::Parse(
            "invalid USB configuration descriptor header".into(),
        ));
    }
    let total_length = le_u16(&bytes[2..4])?;
    if total_length < 9 {
        return Err(GoodixFingerprintError::Parse(
            "USB configuration total length too small".into(),
        ));
    }
    if bytes.len() < total_length as usize {
        return Err(GoodixFingerprintError::Parse(
            "short USB configuration descriptor payload".into(),
        ));
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

fn parse_usb_utf16le_string_descriptor(bytes: &[u8]) -> Result<String, GoodixFingerprintError> {
    if bytes.len() < 2 || bytes[1] != USB_DT_STRING || bytes[0] as usize > bytes.len() {
        return Err(GoodixFingerprintError::Parse(
            "invalid USB string descriptor header".into(),
        ));
    }
    let declared_len = bytes[0] as usize;
    if declared_len < 2 || !(declared_len - 2).is_multiple_of(2) {
        return Err(GoodixFingerprintError::Parse(
            "invalid USB string descriptor length".into(),
        ));
    }
    let mut units = Vec::new();
    for chunk in bytes[2..declared_len].chunks_exact(2) {
        units.push(u16::from_le_bytes([chunk[0], chunk[1]]));
    }
    String::from_utf16(&units)
        .map_err(|_| GoodixFingerprintError::Parse("invalid UTF-16LE USB string descriptor".into()))
}

fn parse_usb_language_ids(bytes: &[u8]) -> Result<Vec<u16>, GoodixFingerprintError> {
    if bytes.len() < 2 || bytes[1] != USB_DT_STRING || bytes[0] as usize > bytes.len() {
        return Err(GoodixFingerprintError::Parse(
            "invalid USB language descriptor header".into(),
        ));
    }
    let declared_len = bytes[0] as usize;
    if declared_len < 4 || !(declared_len - 2).is_multiple_of(2) {
        return Err(GoodixFingerprintError::Parse(
            "invalid USB language descriptor length".into(),
        ));
    }
    let mut out = Vec::new();
    for chunk in bytes[2..declared_len].chunks_exact(2) {
        out.push(u16::from_le_bytes([chunk[0], chunk[1]]));
    }
    Ok(out)
}

fn fixed_c_string(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).to_string()
}

fn hex_string(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use core::fmt::Write as _;
        let _ = write!(&mut out, "{b:02x}");
    }
    out
}

fn decode_hex_32(s: &str) -> Option<[u8; 32]> {
    if s.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for i in 0..32 {
        out[i] = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(out)
}

fn decode_template_id_hex(s: &str) -> Result<[u8; 32], FingerprintError> {
    decode_hex_32(s).ok_or(FingerprintError::InvalidRequest(
        "template_id must be 64 hex chars",
    ))
}

fn capture_quality_from_raw(value: u8) -> edgerun_fingerprint::FingerprintCaptureQuality {
    match value {
        0..=24 => edgerun_fingerprint::FingerprintCaptureQuality::Poor,
        25..=49 => edgerun_fingerprint::FingerprintCaptureQuality::Fair,
        50..=79 => edgerun_fingerprint::FingerprintCaptureQuality::Good,
        _ => edgerun_fingerprint::FingerprintCaptureQuality::Excellent,
    }
}

fn ensure_goodix_ok(result: u8, context: &'static str) -> Result<(), GoodixFingerprintError> {
    if result == GOODIX_SUCCESS {
        return Ok(());
    }
    if result >= GOODIX_FAILED {
        // Classify known error codes for better diagnostics
        let err_msg = match result {
            GOODIX_ERROR_FINGER_ID_NOEXIST => {
                "finger ID does not exist"
            }
            GOODIX_ERROR_TEMPLATE_INCOMPLETE => {
                "template incomplete — try re-enrolling"
            }
            GOODIX_ERROR_WAIT_FINGER_UP_TIMEOUT => {
                "timeout waiting for finger lift"
            }
            GOODIX_ERROR_NO_AVAILABLE_SPACE => {
                "no available storage space — delete some templates"
            }
            _ => "unknown error",
        };
        return Err(GoodixFingerprintError::Parse(format!(
            "Goodix {context} failed: 0x{result:02x} ({err_msg})"
        )));
    }
    Ok(())
}

fn goodix_crc8(bytes: &[u8]) -> u8 {
    let mut crc: u32 = 0;
    for &b in bytes {
        crc ^= u32::from(b) << 8;
        for _ in 0..8 {
            if crc & 0x8000 != 0 {
                crc ^= 0x1070 << 3;
            }
            crc <<= 1;
        }
    }
    !((crc >> 8) as u8)
}

fn reflect(mut data: u32, n_bits: u8) -> u32 {
    let mut reflection = 0u32;
    for bit in 0..n_bits {
        if data & 0x01 != 0 {
            reflection |= 1 << ((n_bits - 1) - bit);
        }
        data >>= 1;
    }
    reflection
}

fn goodix_crc32(bytes: &[u8]) -> [u8; 4] {
    const POLY: u32 = 0x04C11DB7;
    let mut crc = 0xFFFF_FFFFu32;
    for &message_byte in bytes {
        let data = reflect(u32::from(message_byte), 8) ^ (crc >> 24);
        crc ^= data << 24;
        for _ in 0..8 {
            if crc & 0x8000_0000 != 0 {
                crc = (crc << 1) ^ POLY;
            } else {
                crc <<= 1;
            }
        }
    }
    let final_crc = reflect(crc, 32) ^ 0xFFFF_FFFF;
    final_crc.to_le_bytes()
}

pub fn build_goodix_package(cmd0: u8, cmd1: u8, payload: &[u8]) -> Vec<u8> {
    let mut out =
        Vec::with_capacity(GOODIX_PACKAGE_HEADER_SIZE + payload.len() + GOODIX_PACKAGE_CRC_SIZE);
    let payload_plus_crc = (payload.len() + GOODIX_PACKAGE_CRC_SIZE) as u16;
    let mut header = [0u8; GOODIX_PACKAGE_HEADER_SIZE];
    header[0] = cmd0;
    header[1] = cmd1;
    header[2] = 0;
    header[3] = 0;
    header[4..6].copy_from_slice(&payload_plus_crc.to_le_bytes());
    header[6] = goodix_crc8(&header[..6]);
    header[7] = !header[6];
    out.extend_from_slice(&header);
    out.extend_from_slice(payload);
    out.extend_from_slice(&goodix_crc32(&out));
    out
}

pub fn parse_goodix_packet(bytes: &[u8]) -> Result<GoodixPacket, GoodixFingerprintError> {
    if bytes.len() < GOODIX_PACKAGE_HEADER_SIZE + GOODIX_PACKAGE_CRC_SIZE {
        return Err(GoodixFingerprintError::Parse("short Goodix packet".into()));
    }
    let crc8 = goodix_crc8(&bytes[..6]);
    if bytes[6] != crc8 || bytes[7] != !crc8 {
        return Err(GoodixFingerprintError::Parse(
            "invalid Goodix header CRC8".into(),
        ));
    }
    let len_with_crc = le_u16(&bytes[4..6])? as usize;
    if bytes.len() < GOODIX_PACKAGE_HEADER_SIZE + len_with_crc {
        return Err(GoodixFingerprintError::Parse(
            "short Goodix packet payload".into(),
        ));
    }
    let payload_len = len_with_crc
        .checked_sub(GOODIX_PACKAGE_CRC_SIZE)
        .ok_or_else(|| GoodixFingerprintError::Parse("invalid Goodix payload length".into()))?;
    let end = GOODIX_PACKAGE_HEADER_SIZE + payload_len;
    let expected_crc = goodix_crc32(&bytes[..end]);
    let actual_crc: [u8; 4] = bytes[end..end + 4].try_into().unwrap();
    if expected_crc != actual_crc {
        return Err(GoodixFingerprintError::Parse(
            "invalid Goodix packet CRC32".into(),
        ));
    }
    Ok(GoodixPacket {
        header: GoodixPacketHeader {
            cmd0: bytes[0],
            cmd1: bytes[1],
            package_num: bytes[2],
            reserved: bytes[3],
            payload_len: payload_len as u16,
            crc8: bytes[6],
            rev_crc8: bytes[7],
        },
        payload: bytes[GOODIX_PACKAGE_HEADER_SIZE..end].to_vec(),
    })
}

pub fn parse_goodix_ack(packet: &GoodixPacket) -> Result<GoodixAck, GoodixFingerprintError> {
    if packet.header.cmd0 != GOODIX_RESPONSE_ACK_CMD || packet.payload.len() < 2 {
        return Err(GoodixFingerprintError::Parse(
            "invalid Goodix ack packet".into(),
        ));
    }
    Ok(GoodixAck {
        result: packet.payload[0],
        ack_cmd: packet.payload[1],
    })
}

fn parse_goodix_version_info(payload: &[u8]) -> Result<GoodixVersionInfo, GoodixFingerprintError> {
    if payload.len() < 1 + 112 {
        return Err(GoodixFingerprintError::Parse(
            "short Goodix version payload".into(),
        ));
    }
    let body = &payload[1..113];
    Ok(GoodixVersionInfo {
        format: body[0..2].try_into().unwrap(),
        fwtype: body[2..10].try_into().unwrap(),
        fwversion: body[10..18].try_into().unwrap(),
        customer: body[18..26].try_into().unwrap(),
        mcu: body[26..34].try_into().unwrap(),
        sensor: body[34..42].try_into().unwrap(),
        algversion: body[42..50].try_into().unwrap(),
        interface: body[50..58].try_into().unwrap(),
        protocol: body[58..66].try_into().unwrap(),
        flash_version: body[66..74].try_into().unwrap(),
        reserved: body[74..112].try_into().unwrap(),
    })
}

fn parse_goodix_template(bytes: &[u8]) -> Result<GoodixTemplate, GoodixFingerprintError> {
    if bytes.len() < 68 + 1 + 2 {
        return Err(GoodixFingerprintError::Parse(
            "short Goodix template payload".into(),
        ));
    }
    if bytes[0] != 67 {
        return Err(GoodixFingerprintError::Parse(
            "invalid Goodix template marker".into(),
        ));
    }
    let payload_size = bytes[68] as usize;
    if payload_size > 56 || bytes.len() < 69 + payload_size {
        return Err(GoodixFingerprintError::Parse(
            "invalid Goodix template data size".into(),
        ));
    }
    Ok(GoodixTemplate {
        template_type: bytes[1],
        finger_index: bytes[2],
        account_id: bytes[4..36].try_into().unwrap(),
        template_id: bytes[36..68].try_into().unwrap(),
        payload: bytes[69..69 + payload_size].to_vec(),
    })
}

fn build_goodix_finger_id(
    template_id: &[u8; 32],
    user_id: &[u8],
) -> Result<Vec<u8>, GoodixFingerprintError> {
    if user_id.len() > 100 || user_id.len() > 56 {
        return Err(GoodixFingerprintError::Parse(
            "Goodix user id too long".into(),
        ));
    }
    let total_len = 70 + user_id.len();
    let mut out = vec![0u8; total_len + 2];
    let len_le = (total_len as u16).to_le_bytes();
    out[0..2].copy_from_slice(&len_le);
    out[2] = 67;
    out[3] = 1;
    out[4] = 1;
    out[5] = 0;
    out[38..70].copy_from_slice(template_id);
    out[70] = user_id.len() as u8;
    out[71..71 + user_id.len()].copy_from_slice(user_id);
    out[71 + user_id.len()] = 0;
    Ok(out)
}

fn parse_goodix_simple_result(
    payload: &[u8],
) -> Result<GoodixSimpleResult, GoodixFingerprintError> {
    if payload.is_empty() {
        return Err(GoodixFingerprintError::Parse(
            "short Goodix result payload".into(),
        ));
    }
    Ok(GoodixSimpleResult { result: payload[0] })
}

fn parse_goodix_finger_mode_status(
    payload: &[u8],
) -> Result<GoodixFingerModeStatus, GoodixFingerprintError> {
    if payload.is_empty() {
        return Err(GoodixFingerprintError::Parse(
            "short Goodix finger-mode payload".into(),
        ));
    }
    Ok(GoodixFingerModeStatus { status: payload[0] })
}

fn parse_goodix_finger_config(
    payload: &[u8],
) -> Result<GoodixFingerConfig, GoodixFingerprintError> {
    if payload.is_empty() {
        return Err(GoodixFingerprintError::Parse(
            "short Goodix finger-config payload".into(),
        ));
    }
    Ok(GoodixFingerConfig {
        status: payload[0],
        max_stored_prints: payload.get(2).copied().unwrap_or(GOODIX_MAX_STORED_PRINTS),
    })
}

pub fn build_default_goodix_sensor_config() -> GoodixSensorConfig {
    let mut reserved = [0u8; GOODIX_SENSOR_CONFIG_RESERVED_SIZE];
    reserved[0] = 1;
    let mut prefix = [0u8; GOODIX_SENSOR_CONFIG_BODY_SIZE + GOODIX_SENSOR_CONFIG_RESERVED_SIZE];
    prefix[..GOODIX_SENSOR_CONFIG_BODY_SIZE].copy_from_slice(&GOODIX_DEFAULT_SENSOR_CONFIG_BODY);
    prefix[GOODIX_SENSOR_CONFIG_BODY_SIZE..].copy_from_slice(&reserved);
    let crc32 = goodix_crc32(&prefix);
    GoodixSensorConfig {
        config_body: GOODIX_DEFAULT_SENSOR_CONFIG_BODY,
        reserved,
        crc32,
    }
}

fn parse_goodix_enroll_init(
    payload: &[u8],
) -> Result<GoodixEnrollInitResult, GoodixFingerprintError> {
    if payload.is_empty() {
        return Err(GoodixFingerprintError::Parse(
            "short Goodix enroll-init payload".into(),
        ));
    }
    let result = payload[0];
    let template_id = if result == 0 {
        if payload.len() < 33 {
            return Err(GoodixFingerprintError::Parse(
                "short Goodix enroll-init template id".into(),
            ));
        }
        Some(payload[1..33].try_into().unwrap())
    } else {
        None
    };
    Ok(GoodixEnrollInitResult {
        result,
        template_id,
    })
}

fn parse_goodix_enroll_update(
    payload: &[u8],
) -> Result<GoodixEnrollUpdateResult, GoodixFingerprintError> {
    if payload.len() < 3 {
        return Err(GoodixFingerprintError::Parse(
            "short Goodix enroll-update payload".into(),
        ));
    }
    Ok(GoodixEnrollUpdateResult {
        rollback: payload[0] >= 0x80,
        overlay: payload[1],
        preoverlay: payload[2],
    })
}

fn parse_goodix_duplicate_check(
    payload: &[u8],
) -> Result<GoodixDuplicateCheckResult, GoodixFingerprintError> {
    if payload.is_empty() {
        return Err(GoodixFingerprintError::Parse(
            "short Goodix duplicate-check payload".into(),
        ));
    }
    let duplicate = payload[0] != 0;
    if !duplicate {
        return Ok(GoodixDuplicateCheckResult {
            duplicate: false,
            template: None,
        });
    }
    if payload.len() < 3 {
        return Err(GoodixFingerprintError::Parse(
            "short Goodix duplicate-check template length".into(),
        ));
    }
    let template_len = le_u16(&payload[1..3])? as usize;
    if payload.len() < 3 + template_len {
        return Err(GoodixFingerprintError::Parse(
            "short Goodix duplicate-check template".into(),
        ));
    }
    let template = parse_goodix_template(&payload[3..3 + template_len])?;
    Ok(GoodixDuplicateCheckResult {
        duplicate: true,
        template: Some(template),
    })
}

fn parse_goodix_capture_response(
    payload: &[u8],
) -> Result<GoodixCaptureResponse, GoodixFingerprintError> {
    if payload.is_empty() {
        return Err(GoodixFingerprintError::Parse(
            "short Goodix capture payload".into(),
        ));
    }
    let result = payload[0];
    if payload.len() >= 3 {
        Ok(GoodixCaptureResponse {
            result,
            image_quality: Some(payload[1]),
            image_coverage: Some(payload[2]),
        })
    } else {
        Ok(GoodixCaptureResponse {
            result,
            image_quality: None,
            image_coverage: None,
        })
    }
}

fn parse_goodix_identify_result(
    payload: &[u8],
) -> Result<GoodixIdentifyResult, GoodixFingerprintError> {
    if payload.is_empty() {
        return Err(GoodixFingerprintError::Parse(
            "short Goodix identify payload".into(),
        ));
    }
    let matched = payload[0] == 0;
    if !matched {
        return Ok(GoodixIdentifyResult {
            matched: false,
            result: payload[0],
            reject_detail: None,
            score: None,
            study: None,
            template: None,
        });
    }
    if payload.len() < 10 {
        return Err(GoodixFingerprintError::Parse(
            "short Goodix identify match payload".into(),
        ));
    }
    let reject_detail = le_u16(&payload[1..3])?;
    let score = u32::from_le_bytes(payload[3..7].try_into().unwrap());
    let study = payload[7];
    let template_len = le_u16(&payload[8..10])? as usize;
    if payload.len() < 10 + template_len {
        return Err(GoodixFingerprintError::Parse(
            "short Goodix identify template payload".into(),
        ));
    }
    let template = parse_goodix_template(&payload[10..10 + template_len])?;
    Ok(GoodixIdentifyResult {
        matched: true,
        result: payload[0],
        reject_detail: Some(reject_detail),
        score: Some(score),
        study: Some(study),
        template: Some(template),
    })
}

fn parse_goodix_finger_list(payload: &[u8]) -> Result<Vec<GoodixTemplate>, GoodixFingerprintError> {
    if payload.is_empty() {
        return Err(GoodixFingerprintError::Parse(
            "short Goodix finger-list payload".into(),
        ));
    }
    if payload[0] >= 0x80 {
        return Err(GoodixFingerprintError::Parse(format!(
            "Goodix finger-list failed: 0x{:02x}",
            payload[0]
        )));
    }
    if payload.len() < 2 {
        return Err(GoodixFingerprintError::Parse(
            "short Goodix finger-list count".into(),
        ));
    }
    let count = payload[1] as usize;
    if count > GOODIX_MAX_STORED_PRINTS as usize {
        return Err(GoodixFingerprintError::Parse(
            "Goodix finger-list count exceeds supported maximum".into(),
        ));
    }
    let mut offset = 2usize;
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        if payload.len() < offset + 2 {
            return Err(GoodixFingerprintError::Parse(
                "short Goodix finger-list entry length".into(),
            ));
        }
        let entry_len = le_u16(&payload[offset..offset + 2])? as usize;
        offset += 2;
        if payload.len() < offset + entry_len {
            return Err(GoodixFingerprintError::Parse(
                "short Goodix finger-list entry".into(),
            ));
        }
        out.push(parse_goodix_template(&payload[offset..offset + entry_len])?);
        offset += entry_len;
    }
    Ok(out)
}

pub struct GoodixUsbTransport {
    file: File,
    interface_number: u8,
}

impl GoodixUsbTransport {
    pub fn claim(file: File, interface_number: u8) -> Result<Self, GoodixFingerprintError> {
        let iface: CUInt = interface_number.into();
        let rc = unsafe { ioctl(file.as_raw_fd(), USBDEVFS_CLAIMINTERFACE, &iface) };
        if rc < 0 {
            return Err(GoodixFingerprintError::Io(io::Error::last_os_error()));
        }
        Ok(Self {
            file,
            interface_number,
        })
    }

    pub fn interface_number(&self) -> u8 {
        self.interface_number
    }

    pub fn get_device_descriptor(&mut self) -> Result<UsbDeviceDescriptor, GoodixFingerprintError> {
        let mut data = [0u8; 18];
        let got = self.control_transfer(
            USB_DIR_IN | USB_TYPE_STANDARD | USB_RECIP_DEVICE,
            USB_REQ_GET_DESCRIPTOR,
            u16::from(USB_DT_DEVICE) << 8,
            0,
            &mut data,
            1000,
        )?;
        parse_usb_device_descriptor(&data[..got])
    }

    pub fn get_configuration_value(&mut self) -> Result<u8, GoodixFingerprintError> {
        let mut data = [0u8; 1];
        let got = self.control_transfer(
            USB_DIR_IN | USB_TYPE_STANDARD | USB_RECIP_DEVICE,
            USB_REQ_GET_CONFIGURATION,
            0,
            0,
            &mut data,
            1000,
        )?;
        if got != 1 {
            return Err(GoodixFingerprintError::Parse(
                "unexpected GET_CONFIGURATION response length".into(),
            ));
        }
        Ok(data[0])
    }

    pub fn get_configuration_descriptor(
        &mut self,
        index: u8,
    ) -> Result<UsbConfigurationDescriptor, GoodixFingerprintError> {
        let mut head = [0u8; 9];
        let got = self.control_transfer(
            USB_DIR_IN | USB_TYPE_STANDARD | USB_RECIP_DEVICE,
            USB_REQ_GET_DESCRIPTOR,
            (u16::from(USB_DT_CONFIG) << 8) | u16::from(index),
            0,
            &mut head,
            1000,
        )?;
        if got < 9 {
            return Err(GoodixFingerprintError::Parse(
                "short USB configuration descriptor".into(),
            ));
        }
        if head[1] != USB_DT_CONFIG {
            return Err(GoodixFingerprintError::Parse(
                "invalid USB configuration descriptor header".into(),
            ));
        }
        let total_length = le_u16(&head[2..4])? as usize;
        if total_length < 9 {
            return Err(GoodixFingerprintError::Parse(
                "USB configuration total length too small".into(),
            ));
        }
        let mut full = vec![0u8; total_length];
        let got = self.control_transfer(
            USB_DIR_IN | USB_TYPE_STANDARD | USB_RECIP_DEVICE,
            USB_REQ_GET_DESCRIPTOR,
            (u16::from(USB_DT_CONFIG) << 8) | u16::from(index),
            0,
            &mut full,
            1000,
        )?;
        parse_usb_configuration_descriptor(&full[..got])
    }

    pub fn get_string_descriptor(
        &mut self,
        index: u8,
        language_id: u16,
    ) -> Result<String, GoodixFingerprintError> {
        let mut data = [0u8; 255];
        let got = self.control_transfer(
            USB_DIR_IN | USB_TYPE_STANDARD | USB_RECIP_DEVICE,
            USB_REQ_GET_DESCRIPTOR,
            (u16::from(USB_DT_STRING) << 8) | u16::from(index),
            language_id,
            &mut data,
            1000,
        )?;
        parse_usb_utf16le_string_descriptor(&data[..got])
    }

    pub fn get_supported_language_ids(&mut self) -> Result<Vec<u16>, GoodixFingerprintError> {
        let mut data = [0u8; 255];
        let got = self.control_transfer(
            USB_DIR_IN | USB_TYPE_STANDARD | USB_RECIP_DEVICE,
            USB_REQ_GET_DESCRIPTOR,
            u16::from(USB_DT_STRING) << 8,
            0,
            &mut data,
            1000,
        )?;
        parse_usb_language_ids(&data[..got])
    }

    pub fn exchange_goodix_command(
        &mut self,
        info: &GoodixUsbTransportInfo,
        cmd0: u8,
        cmd1: u8,
        payload: &[u8],
    ) -> Result<GoodixPacket, GoodixFingerprintError> {
        let bulk_out = info
            .bulk_out_endpoint
            .ok_or(GoodixFingerprintError::MissingUsbEndpoint("bulk-out"))?;
        let bulk_in = info
            .bulk_in_endpoint
            .ok_or(GoodixFingerprintError::MissingUsbEndpoint("bulk-in"))?;
        let mut request = build_goodix_package(cmd0, cmd1, payload);
        let sent = self.bulk_transfer(bulk_out, &mut request, GOODIX_TIMEOUT_SEND_MS)?;
        if sent != request.len() {
            return Err(GoodixFingerprintError::Parse(
                "short Goodix command write".into(),
            ));
        }

        let mut ack_buf = vec![0u8; 2048];
        let ack_len = self.bulk_transfer(bulk_in, &mut ack_buf, GOODIX_TIMEOUT_ACK_MS)?;
        let ack = parse_goodix_packet(&ack_buf[..ack_len])?;
        let parsed_ack = parse_goodix_ack(&ack)?;
        if parsed_ack.ack_cmd != cmd0 {
            return Err(GoodixFingerprintError::Parse(
                "unexpected Goodix ack command".into(),
            ));
        }
        if parsed_ack.result >= 0x80 {
            return Err(GoodixFingerprintError::Parse(format!(
                "Goodix command ack failed: 0x{:02x}",
                parsed_ack.result
            )));
        }

        let mut data_buf = vec![0u8; 2048];
        let data_len = self.bulk_transfer(bulk_in, &mut data_buf, GOODIX_TIMEOUT_DATA_MS)?;
        let packet = parse_goodix_packet(&data_buf[..data_len])?;
        if packet.header.cmd0 != cmd0 || packet.header.cmd1 != cmd1 {
            return Err(GoodixFingerprintError::Parse(
                "unexpected Goodix response packet".into(),
            ));
        }
        Ok(packet)
    }

    pub fn get_goodix_version(
        &mut self,
        info: &GoodixUsbTransportInfo,
    ) -> Result<GoodixVersionInfo, GoodixFingerprintError> {
        let packet = self.exchange_goodix_command(info, GOODIX_CMD_GET_VERSION, 0x00, &[0])?;
        if packet.payload.first().copied().unwrap_or(0x80) >= 0x80 {
            return Err(GoodixFingerprintError::Parse(
                "Goodix get-version failed".into(),
            ));
        }
        parse_goodix_version_info(&packet.payload)
    }

    pub fn update_config(
        &mut self,
        info: &GoodixUsbTransportInfo,
        config: &GoodixSensorConfig,
        write_to_flash: bool,
    ) -> Result<GoodixFingerConfig, GoodixFingerprintError> {
        let subcmd = if write_to_flash {
            GOODIX_SUBCMD_WRITE_CFG_TO_FLASH
        } else {
            GOODIX_SUBCMD_DEFAULT
        };
        let packet = self.exchange_goodix_command(
            info,
            GOODIX_CMD_UPDATE_CONFIG,
            subcmd,
            &config.clone().into_bytes(),
        )?;
        parse_goodix_finger_config(&packet.payload)
    }

    pub fn get_finger_list(
        &mut self,
        info: &GoodixUsbTransportInfo,
    ) -> Result<Vec<GoodixTemplate>, GoodixFingerprintError> {
        let packet = self.exchange_goodix_command(info, GOODIX_CMD_GET_FINGERLIST, 0x00, &[0])?;
        parse_goodix_finger_list(&packet.payload)
    }

    pub fn capture_data(
        &mut self,
        info: &GoodixUsbTransportInfo,
        mode: [u8; 3],
    ) -> Result<GoodixCaptureResponse, GoodixFingerprintError> {
        let packet = self.exchange_goodix_command(info, GOODIX_CMD_CAPTURE_DATA, 0x00, &mode)?;
        parse_goodix_capture_response(&packet.payload)
    }

    pub fn identify(
        &mut self,
        info: &GoodixUsbTransportInfo,
        nonce: &[u8; 32],
    ) -> Result<GoodixIdentifyResult, GoodixFingerprintError> {
        let packet = self.exchange_goodix_command(info, GOODIX_CMD_IDENTIFY, 0x00, nonce)?;
        parse_goodix_identify_result(&packet.payload)
    }

    pub fn enroll_init(
        &mut self,
        info: &GoodixUsbTransportInfo,
    ) -> Result<GoodixEnrollInitResult, GoodixFingerprintError> {
        let packet = self.exchange_goodix_command(info, GOODIX_CMD_ENROLL_INIT, 0x00, &[0])?;
        parse_goodix_enroll_init(&packet.payload)
    }

    pub fn enroll_update(
        &mut self,
        info: &GoodixUsbTransportInfo,
        mode: [u8; 3],
    ) -> Result<GoodixEnrollUpdateResult, GoodixFingerprintError> {
        let packet = self.exchange_goodix_command(info, GOODIX_CMD_ENROLL, 0x00, &mode)?;
        parse_goodix_enroll_update(&packet.payload)
    }

    pub fn check_duplicate(
        &mut self,
        info: &GoodixUsbTransportInfo,
        mode: [u8; 3],
    ) -> Result<GoodixDuplicateCheckResult, GoodixFingerprintError> {
        let packet = self.exchange_goodix_command(info, GOODIX_CMD_CHECK_DUPLICATE, 0x00, &mode)?;
        parse_goodix_duplicate_check(&packet.payload)
    }

    pub fn commit_enrollment(
        &mut self,
        info: &GoodixUsbTransportInfo,
        template_id: &[u8; 32],
        user_id: &[u8],
    ) -> Result<GoodixSimpleResult, GoodixFingerprintError> {
        let payload = build_goodix_finger_id(template_id, user_id)?;
        let packet =
            self.exchange_goodix_command(info, GOODIX_CMD_COMMIT_ENROLLMENT, 0x00, &payload)?;
        parse_goodix_simple_result(&packet.payload)
    }

    pub fn delete_template(
        &mut self,
        info: &GoodixUsbTransportInfo,
        template_id: &[u8; 32],
        user_id: &[u8],
    ) -> Result<GoodixSimpleResult, GoodixFingerprintError> {
        let payload = build_goodix_finger_id(template_id, user_id)?;
        let packet =
            self.exchange_goodix_command(info, GOODIX_CMD_DELETE_TEMPLATE, 0x00, &payload)?;
        parse_goodix_simple_result(&packet.payload)
    }

    pub fn delete_all_templates(
        &mut self,
        info: &GoodixUsbTransportInfo,
    ) -> Result<GoodixSimpleResult, GoodixFingerprintError> {
        let packet = self.exchange_goodix_command(
            info,
            GOODIX_CMD_DELETE_TEMPLATE,
            GOODIX_SUBCMD_DELETE_ALL,
            &[],
        )?;
        parse_goodix_simple_result(&packet.payload)
    }

    pub fn get_finger_mode(
        &mut self,
        info: &GoodixUsbTransportInfo,
    ) -> Result<GoodixFingerModeStatus, GoodixFingerprintError> {
        let packet = self.exchange_goodix_command(
            info,
            GOODIX_CMD_FINGER_MODE,
            GOODIX_SUBCMD_GET_FINGER_MODE,
            &[0],
        )?;
        parse_goodix_finger_mode_status(&packet.payload)
    }

    pub fn set_finger_down_mode(
        &mut self,
        info: &GoodixUsbTransportInfo,
    ) -> Result<GoodixFingerModeStatus, GoodixFingerprintError> {
        let packet = self.exchange_goodix_command(
            info,
            GOODIX_CMD_FINGER_MODE,
            GOODIX_SUBCMD_SET_FINGER_DOWN,
            &[0],
        )?;
        parse_goodix_finger_mode_status(&packet.payload)
    }

    pub fn set_finger_up_mode(
        &mut self,
        info: &GoodixUsbTransportInfo,
    ) -> Result<GoodixFingerModeStatus, GoodixFingerprintError> {
        let packet = self.exchange_goodix_command(
            info,
            GOODIX_CMD_FINGER_MODE,
            GOODIX_SUBCMD_SET_FINGER_UP,
            &[0],
        )?;
        parse_goodix_finger_mode_status(&packet.payload)
    }

    pub fn set_power_button_shield(
        &mut self,
        info: &GoodixUsbTransportInfo,
        enabled: bool,
    ) -> Result<GoodixSimpleResult, GoodixFingerprintError> {
        let subcmd = if enabled {
            GOODIX_SUBCMD_PWR_BTN_SHIELD_ON
        } else {
            GOODIX_SUBCMD_PWR_BTN_SHIELD_OFF
        };
        let packet =
            self.exchange_goodix_command(info, GOODIX_CMD_POWER_BUTTON_SHIELD, subcmd, &[])?;
        parse_goodix_simple_result(&packet.payload)
    }

    pub fn control_transfer(
        &mut self,
        request_type: u8,
        request: u8,
        value: u16,
        index: u16,
        data: &mut [u8],
        timeout_ms: u32,
    ) -> Result<usize, GoodixFingerprintError> {
        let mut transfer = UsbdevfsCtrlTransfer {
            b_request_type: request_type,
            b_request: request,
            w_value: value,
            w_index: index,
            w_length: data.len() as u16,
            timeout: timeout_ms,
            data: data.as_mut_ptr().cast(),
        };
        let rc = unsafe { ioctl(self.file.as_raw_fd(), USBDEVFS_CONTROL, &mut transfer) };
        if rc < 0 {
            return Err(GoodixFingerprintError::Io(io::Error::last_os_error()));
        }
        Ok(rc as usize)
    }

    pub fn bulk_transfer(
        &mut self,
        endpoint: u8,
        data: &mut [u8],
        timeout_ms: u32,
    ) -> Result<usize, GoodixFingerprintError> {
        let mut transfer = UsbdevfsBulkTransfer {
            ep: endpoint.into(),
            len: data.len() as u32,
            timeout: timeout_ms,
            data: data.as_mut_ptr().cast(),
        };
        let rc = unsafe { ioctl(self.file.as_raw_fd(), USBDEVFS_BULK, &mut transfer) };
        if rc < 0 {
            return Err(GoodixFingerprintError::Io(io::Error::last_os_error()));
        }
        Ok(rc as usize)
    }
}

impl Drop for GoodixUsbTransport {
    fn drop(&mut self) {
        let iface: CUInt = self.interface_number.into();
        unsafe {
            let _ = ioctl(self.file.as_raw_fd(), USBDEVFS_RELEASEINTERFACE, &iface);
        }
    }
}

pub fn is_supported_goodix_fingerprint_device(vendor_id: u16, product_id: u16) -> bool {
    vendor_id == GOODIX_VENDOR_ID && product_id == GOODIX_FRAMEWORK_13_PRODUCT_ID
}

pub fn discover_supported_devices() -> Result<Vec<GoodixUsbDevice>, GoodixFingerprintError> {
    discover_supported_devices_in(Path::new("/sys/bus/usb/devices"), Path::new("/dev/bus/usb"))
}

pub fn discover_supported_devices_in(
    sysfs_root: &Path,
    usb_bus_root: &Path,
) -> Result<Vec<GoodixUsbDevice>, GoodixFingerprintError> {
    let mut devices = Vec::new();
    if !sysfs_root.exists() {
        return Ok(devices);
    }
    for entry in fs::read_dir(sysfs_root)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let vendor_id = match read_hex_u16(path.join("idVendor")) {
            Ok(v) => v,
            Err(GoodixFingerprintError::Io(err)) if err.kind() == io::ErrorKind::NotFound => {
                continue;
            }
            Err(err) => return Err(err),
        };
        let product_id = match read_hex_u16(path.join("idProduct")) {
            Ok(v) => v,
            Err(GoodixFingerprintError::Io(err)) if err.kind() == io::ErrorKind::NotFound => {
                continue;
            }
            Err(err) => return Err(err),
        };
        if !is_supported_goodix_fingerprint_device(vendor_id, product_id) {
            continue;
        }
        let bus_number = read_dec_u8(path.join("busnum"))?;
        let device_number = read_dec_u8(path.join("devnum"))?;
        let devnode = usb_bus_root
            .join(format!("{bus_number:03}"))
            .join(format!("{device_number:03}"));
        devices.push(GoodixUsbDevice {
            bus_number,
            device_number,
            vendor_id,
            product_id,
            devnode,
            sysfs_path: path,
        });
    }
    devices.sort_by_key(|d| (d.bus_number, d.device_number));
    Ok(devices)
}

fn read_file_trimmed(path: PathBuf) -> Result<String, GoodixFingerprintError> {
    Ok(fs::read_to_string(path)?.trim().to_string())
}

fn read_hex_u16(path: PathBuf) -> Result<u16, GoodixFingerprintError> {
    let text = read_file_trimmed(path)?;
    u16::from_str_radix(text.trim_start_matches("0x"), 16)
        .map_err(|_| GoodixFingerprintError::Parse(format!("invalid hex device attribute: {text}")))
}

fn read_dec_u8(path: PathBuf) -> Result<u8, GoodixFingerprintError> {
    let text = read_file_trimmed(path)?;
    text.parse::<u8>().map_err(|_| {
        GoodixFingerprintError::Parse(format!("invalid decimal device attribute: {text}"))
    })
}

fn read_interface_number(path: &Path) -> Result<u8, GoodixFingerprintError> {
    read_dec_u8(path.join("bInterfaceNumber"))
}

fn read_alt_setting(path: &Path) -> Result<u8, GoodixFingerprintError> {
    read_dec_u8(path.join("bAlternateSetting"))
}

fn parse_endpoint_name(name: &str) -> Option<u8> {
    let suffix = name.strip_prefix("ep_")?;
    u8::from_str_radix(suffix, 16).ok()
}

fn discover_interface_endpoints(interface_path: &Path) -> Result<Vec<u8>, GoodixFingerprintError> {
    let mut endpoints = Vec::new();
    for entry in fs::read_dir(interface_path)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if let Some(ep) = parse_endpoint_name(&name) {
            endpoints.push(ep);
        }
    }
    endpoints.sort_unstable();
    Ok(endpoints)
}

pub fn discover_interfaces(
    device: &GoodixUsbDevice,
) -> Result<Vec<GoodixUsbInterface>, GoodixFingerprintError> {
    let Some(sysfs_name) = device.sysfs_path.file_name() else {
        return Err(GoodixFingerprintError::MissingUsbInterface);
    };
    let prefix = format!("{}:", sysfs_name.to_string_lossy());
    let parent = device
        .sysfs_path
        .parent()
        .ok_or(GoodixFingerprintError::MissingUsbInterface)?;
    let mut interfaces = Vec::new();
    for entry in fs::read_dir(parent)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.starts_with(&prefix) {
            continue;
        }
        let number = match read_interface_number(&path) {
            Ok(v) => v,
            Err(GoodixFingerprintError::Io(err)) if err.kind() == io::ErrorKind::NotFound => {
                continue
            }
            Err(err) => return Err(err),
        };
        let alt_setting = match read_alt_setting(&path) {
            Ok(v) => v,
            Err(GoodixFingerprintError::Io(err)) if err.kind() == io::ErrorKind::NotFound => 0,
            Err(err) => return Err(err),
        };
        let endpoints = discover_interface_endpoints(&path)?;
        interfaces.push(GoodixUsbInterface {
            number,
            alt_setting,
            endpoints,
        });
    }
    interfaces.sort_by_key(|i| (i.number, i.alt_setting));
    Ok(interfaces)
}

pub fn select_transport_interface(
    interfaces: &[GoodixUsbInterface],
) -> Result<GoodixUsbTransportInfo, GoodixFingerprintError> {
    let interface = interfaces
        .iter()
        .find(|iface| iface.endpoints.iter().any(|ep| ep & 0x80 == 0x80))
        .ok_or(GoodixFingerprintError::MissingUsbInterface)?;
    let mut in_eps = interface
        .endpoints
        .iter()
        .copied()
        .filter(|ep| ep & 0x80 == 0x80)
        .collect::<Vec<_>>();
    let mut out_eps = interface
        .endpoints
        .iter()
        .copied()
        .filter(|ep| ep & 0x80 == 0)
        .collect::<Vec<_>>();
    in_eps.sort_unstable();
    out_eps.sort_unstable();
    Ok(GoodixUsbTransportInfo {
        interface_number: interface.number,
        bulk_in_endpoint: in_eps.first().copied(),
        bulk_out_endpoint: out_eps.first().copied(),
        interrupt_in_endpoint: in_eps.get(1).copied(),
    })
}

pub struct GoodixFingerprintReader {
    device: GoodixUsbDevice,
}

impl GoodixFingerprintReader {
    pub fn new(device: GoodixUsbDevice) -> Result<Self, GoodixFingerprintError> {
        if !is_supported_goodix_fingerprint_device(device.vendor_id, device.product_id) {
            return Err(GoodixFingerprintError::UnsupportedDevice {
                vendor_id: device.vendor_id,
                product_id: device.product_id,
            });
        }
        Ok(Self { device })
    }

    pub fn device(&self) -> &GoodixUsbDevice {
        &self.device
    }

    pub fn open_transport(&self) -> Result<File, GoodixFingerprintError> {
        Ok(std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&self.device.devnode)?)
    }

    pub fn transport_info(&self) -> Result<GoodixUsbTransportInfo, GoodixFingerprintError> {
        let interfaces = discover_interfaces(&self.device)?;
        select_transport_interface(&interfaces)
    }

    pub fn open_claimed_transport(&self) -> Result<GoodixUsbTransport, GoodixFingerprintError> {
        let info = self.transport_info()?;
        let file = self.open_transport()?;
        GoodixUsbTransport::claim(file, info.interface_number)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GoodixDeviceStrings {
    pub manufacturer: Option<String>,
    pub product: Option<String>,
    pub serial_number: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GoodixSupportedFeatures {
    pub match_on_sensor: bool,
    pub persistent_templates: bool,
    pub template_listing: bool,
    pub template_delete: bool,
    pub delete_all_templates: bool,
    pub update_config: bool,
    pub capture: bool,
    pub identify: bool,
    pub enroll_init: bool,
    pub enroll_update: bool,
    pub duplicate_check: bool,
    pub commit_enrollment: bool,
    pub finger_mode_query: bool,
    pub finger_down_mode: bool,
    pub finger_up_mode: bool,
    pub power_button_shield: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GoodixDeviceProbe {
    pub transport: GoodixUsbTransportInfo,
    pub device_descriptor: UsbDeviceDescriptor,
    pub active_configuration: u8,
    pub configuration_descriptor: UsbConfigurationDescriptor,
    pub strings: GoodixDeviceStrings,
    pub version_info: Option<GoodixVersionInfo>,
    pub features: GoodixSupportedFeatures,
}

impl GoodixFingerprintReader {
    fn read_device_strings(
        usb: &mut GoodixUsbTransport,
        descriptor: &UsbDeviceDescriptor,
    ) -> GoodixDeviceStrings {
        let lang = usb
            .get_supported_language_ids()
            .ok()
            .and_then(|langs| langs.into_iter().next())
            .unwrap_or(0x0409);
        let manufacturer = if descriptor.manufacturer_index != 0 {
            usb.get_string_descriptor(descriptor.manufacturer_index, lang)
                .ok()
        } else {
            None
        };
        let product = if descriptor.product_index != 0 {
            usb.get_string_descriptor(descriptor.product_index, lang)
                .ok()
        } else {
            None
        };
        let serial_number = if descriptor.serial_number_index != 0 {
            usb.get_string_descriptor(descriptor.serial_number_index, lang)
                .ok()
        } else {
            None
        };
        GoodixDeviceStrings {
            manufacturer,
            product,
            serial_number,
        }
    }

    pub fn supported_features(&self) -> GoodixSupportedFeatures {
        GoodixSupportedFeatures {
            match_on_sensor: true,
            persistent_templates: true,
            template_listing: true,
            template_delete: true,
            delete_all_templates: true,
            update_config: true,
            capture: true,
            identify: true,
            enroll_init: true,
            enroll_update: true,
            duplicate_check: true,
            commit_enrollment: true,
            finger_mode_query: true,
            finger_down_mode: true,
            finger_up_mode: true,
            power_button_shield: true,
        }
    }

    pub fn probe_device(&self) -> Result<GoodixDeviceProbe, GoodixFingerprintError> {
        let transport = self.transport_info()?;
        let mut usb = self.open_claimed_transport()?;
        let device_descriptor = usb.get_device_descriptor()?;
        let active_configuration = usb.get_configuration_value()?;
        let configuration_descriptor = usb.get_configuration_descriptor(0)?;
        let strings = Self::read_device_strings(&mut usb, &device_descriptor);
        let version_info = usb.get_goodix_version(&transport).ok();
        Ok(GoodixDeviceProbe {
            transport,
            device_descriptor,
            active_configuration,
            configuration_descriptor,
            strings,
            version_info,
            features: self.supported_features(),
        })
    }
}

impl GoodixFingerprintReader {
    pub fn default_capture_mode(&self) -> [u8; 3] {
        [0x01, 0x00, 0x23]
    }

    pub fn default_enroll_mode(&self) -> [u8; 3] {
        [0x01, 0x64, 0x50]
    }

    pub fn begin_enrollment_template(&self) -> Result<Option<[u8; 32]>, GoodixFingerprintError> {
        let transport = self.transport_info()?;
        let mut usb = self.open_claimed_transport()?;
        let init = usb.enroll_init(&transport)?;
        ensure_goodix_ok(init.result, "enroll-init")?;
        Ok(init.template_id)
    }

    pub fn commit_template_id(
        &self,
        template_id: &[u8; 32],
        label: &str,
    ) -> Result<(), GoodixFingerprintError> {
        let transport = self.transport_info()?;
        let mut usb = self.open_claimed_transport()?;
        let result = usb.commit_enrollment(&transport, template_id, label.as_bytes())?;
        if result.result >= 0x80 {
            return Err(GoodixFingerprintError::Parse(format!(
                "Goodix commit failed: 0x{:02x}",
                result.result
            )));
        }
        Ok(())
    }

    pub fn delete_template_id(
        &self,
        template_id: &[u8; 32],
        label: &[u8],
    ) -> Result<(), GoodixFingerprintError> {
        let transport = self.transport_info()?;
        let mut usb = self.open_claimed_transport()?;
        let result = usb.delete_template(&transport, template_id, label)?;
        if result.result >= 0x80 {
            return Err(GoodixFingerprintError::Parse(format!(
                "Goodix delete failed: 0x{:02x}",
                result.result
            )));
        }
        Ok(())
    }

    pub fn verify_live_finger(&self) -> Result<GoodixIdentifyResult, GoodixFingerprintError> {
        let transport = self.transport_info()?;
        let mut usb = self.open_claimed_transport()?;
        let capture = usb.capture_data(&transport, self.default_capture_mode())?;
        ensure_goodix_ok(capture.result, "capture")?;
        let nonce = [0u8; 32];
        usb.identify(&transport, &nonce)
    }

    pub fn enroll_live_sample(&self) -> Result<GoodixEnrollmentProgress, GoodixFingerprintError> {
        let transport = self.transport_info()?;
        let mut usb = self.open_claimed_transport()?;
        let capture = usb.capture_data(&transport, [0, 0, 0])?;
        ensure_goodix_ok(capture.result, "capture")?;
        let update = usb.enroll_update(&transport, self.default_enroll_mode())?;
        let duplicate = usb.check_duplicate(&transport, [0, 0, 0])?;
        Ok(GoodixEnrollmentProgress {
            capture,
            update,
            duplicate,
        })
    }

    pub fn commit_new_template(
        &self,
        label: &str,
    ) -> Result<Option<String>, GoodixFingerprintError> {
        let Some(template_id) = self.begin_enrollment_template()? else {
            return Ok(None);
        };
        self.commit_template_id(&template_id, label)?;
        Ok(Some(hex_string(&template_id)))
    }

    pub fn delete_all_templates_live(&self) -> Result<(), GoodixFingerprintError> {
        let transport = self.transport_info()?;
        let mut usb = self.open_claimed_transport()?;
        let result = usb.delete_all_templates(&transport)?;
        if result.result >= 0x80 {
            return Err(GoodixFingerprintError::Parse(format!(
                "Goodix delete-all failed: 0x{:02x}",
                result.result
            )));
        }
        Ok(())
    }

    pub fn get_finger_mode_live(&self) -> Result<GoodixFingerModeStatus, GoodixFingerprintError> {
        let transport = self.transport_info()?;
        let mut usb = self.open_claimed_transport()?;
        usb.get_finger_mode(&transport)
    }

    pub fn set_finger_down_mode_live(
        &self,
    ) -> Result<GoodixFingerModeStatus, GoodixFingerprintError> {
        let transport = self.transport_info()?;
        let mut usb = self.open_claimed_transport()?;
        usb.set_finger_down_mode(&transport)
    }

    pub fn update_default_config_live(
        &self,
        write_to_flash: bool,
    ) -> Result<GoodixFingerConfig, GoodixFingerprintError> {
        let transport = self.transport_info()?;
        let mut usb = self.open_claimed_transport()?;
        usb.update_config(
            &transport,
            &build_default_goodix_sensor_config(),
            write_to_flash,
        )
    }

    pub fn set_power_button_shield_live(
        &self,
        enabled: bool,
    ) -> Result<(), GoodixFingerprintError> {
        let transport = self.transport_info()?;
        let mut usb = self.open_claimed_transport()?;
        let result = usb.set_power_button_shield(&transport, enabled)?;
        if result.result >= 0x80 {
            return Err(GoodixFingerprintError::Parse(format!(
                "Goodix power-button shield failed: 0x{:02x}",
                result.result
            )));
        }
        Ok(())
    }
}

impl FingerprintReader for GoodixFingerprintReader {
    fn reader_info(&self) -> Result<FingerprintReaderInfo, FingerprintError> {
        let features = self.supported_features();
        Ok(FingerprintReaderInfo {
            provider: "goodix-usb".into(),
            reader_name: "Goodix Fingerprint USB Device".into(),
            supports_match_on_sensor: features.match_on_sensor,
            supports_persistent_templates: features.persistent_templates,
            hardware_protected_match: true,
            max_templates: Some(20),
            usb_vendor_id: Some(self.device.vendor_id),
            usb_product_id: Some(self.device.product_id),
        })
    }

    fn list_templates(&self) -> Result<Vec<FingerprintTemplateRecord>, FingerprintError> {
        let transport = self
            .transport_info()
            .map_err(|e| FingerprintError::Provider(e.to_string()))?;
        let mut usb = self
            .open_claimed_transport()
            .map_err(|e| FingerprintError::Provider(e.to_string()))?;
        let templates = usb
            .get_finger_list(&transport)
            .map_err(|e| FingerprintError::Provider(e.to_string()))?;
        Ok(templates
            .into_iter()
            .map(|template| FingerprintTemplateRecord {
                template_id: hex_string(&template.template_id),
                label: fixed_c_string(&template.payload),
                enrolled_at_unix_ms: 0,
                last_verified_unix_ms: None,
            })
            .collect())
    }

    fn capture(
        &mut self,
        purpose: FingerprintCapturePurpose,
        _timeout_ms: u32,
    ) -> Result<FingerprintCapture, FingerprintError> {
        let transport = self
            .transport_info()
            .map_err(|e| FingerprintError::Provider(e.to_string()))?;
        let mut usb = self
            .open_claimed_transport()
            .map_err(|e| FingerprintError::Provider(e.to_string()))?;
        let mode = match purpose {
            FingerprintCapturePurpose::Enrollment => [0, 0, 0],
            FingerprintCapturePurpose::Verification => self.default_capture_mode(),
        };
        let resp = usb
            .capture_data(&transport, mode)
            .map_err(|e| FingerprintError::Provider(e.to_string()))?;
        let quality = capture_quality_from_raw(resp.image_quality.unwrap_or(0));
        Ok(FingerprintCapture {
            bytes: Vec::new(),
            quality,
            state: edgerun_fingerprint::default_fingerprint_biometric_state(false, true, true),
        })
    }

    fn begin_enrollment(
        &mut self,
        request: &FingerprintEnrollRequest,
    ) -> Result<FingerprintEnrollmentSession, FingerprintError> {
        validate_enroll_request(request)?;
        let transport = self
            .transport_info()
            .map_err(|e| FingerprintError::Provider(e.to_string()))?;
        let mut usb = self
            .open_claimed_transport()
            .map_err(|e| FingerprintError::Provider(e.to_string()))?;
        let init = usb
            .enroll_init(&transport)
            .map_err(|e| FingerprintError::Provider(e.to_string()))?;
        ensure_goodix_ok(init.result, "enroll-init")
            .map_err(|e| FingerprintError::Provider(e.to_string()))?;
        let session_id = init
            .template_id
            .map(|t| hex_string(&t))
            .unwrap_or_else(|| "pending".into());
        Ok(FingerprintEnrollmentSession {
            session_id,
            label: request.label.clone(),
            samples_required: request.samples_required,
            samples_collected: 0,
            require_hardware_match: request.require_hardware_match,
        })
    }

    fn enroll_step(
        &mut self,
        session_id: &str,
        _capture: &FingerprintCapture,
    ) -> Result<FingerprintEnrollProgress, FingerprintError> {
        let progress = self
            .enroll_live_sample()
            .map_err(|e| FingerprintError::Provider(e.to_string()))?;
        let samples_collected = if progress.duplicate.duplicate { 1 } else { 1 };
        Ok(FingerprintEnrollProgress {
            session: FingerprintEnrollmentSession {
                session_id: session_id.to_string(),
                label: String::new(),
                samples_required: 1,
                samples_collected,
                require_hardware_match: true,
            },
            complete: progress.duplicate.duplicate || progress.update.overlay >= 100,
            template_id: progress
                .duplicate
                .template
                .as_ref()
                .map(|t| hex_string(&t.template_id))
                .or_else(|| decode_hex_32(session_id).map(|_| session_id.to_string())),
            last_quality: capture_quality_from_raw(progress.capture.image_quality.unwrap_or(0)),
        })
    }

    fn finish_enrollment(
        &mut self,
        session_id: &str,
    ) -> Result<FingerprintTemplateRecord, FingerprintError> {
        let template_id = decode_template_id_hex(session_id)?;
        self.commit_template_id(&template_id, "")
            .map_err(|e| FingerprintError::Provider(e.to_string()))?;
        Ok(FingerprintTemplateRecord {
            template_id: session_id.to_string(),
            label: String::new(),
            enrolled_at_unix_ms: 0,
            last_verified_unix_ms: None,
        })
    }

    fn verify_capture(
        &mut self,
        _request: &FingerprintVerifyRequest,
        _capture: &FingerprintCapture,
    ) -> Result<FingerprintVerification, FingerprintError> {
        let result = self
            .verify_live_finger()
            .map_err(|e| FingerprintError::Provider(e.to_string()))?;
        Ok(FingerprintVerification {
            matched: result.matched,
            template_id: result.template.as_ref().map(|t| hex_string(&t.template_id)),
            state: edgerun_fingerprint::default_fingerprint_biometric_state(
                result.matched,
                true,
                true,
            ),
            false_accept_rate_per_million: None,
        })
    }

    fn delete_template(&mut self, template_id: &str) -> Result<(), FingerprintError> {
        let tid = decode_template_id_hex(template_id)?;
        self.delete_template_id(&tid, b"")
            .map_err(|e| FingerprintError::Provider(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgerun_linux_sysfs::temp_root;

    #[test]
    fn supported_device_detection_matches_framework_13_reader() {
        assert!(is_supported_goodix_fingerprint_device(0x27c6, 0x609c));
        assert!(!is_supported_goodix_fingerprint_device(0x27c6, 0x638c));
    }

    #[test]
    fn discovery_finds_supported_sysfs_entries() {
        let root = temp_root("goodix-discovery");
        let sysfs = root.join("sys");
        let devbus = root.join("dev/bus/usb/001");
        fs::create_dir_all(&devbus).unwrap();

        let d1 = sysfs.join("1-3");
        fs::create_dir_all(&d1).unwrap();
        fs::write(d1.join("idVendor"), "27c6\n").unwrap();
        fs::write(d1.join("idProduct"), "609c\n").unwrap();
        fs::write(d1.join("busnum"), "1\n").unwrap();
        fs::write(d1.join("devnum"), "2\n").unwrap();

        let d2 = sysfs.join("1-4");
        fs::create_dir_all(&d2).unwrap();
        fs::write(d2.join("idVendor"), "1234\n").unwrap();
        fs::write(d2.join("idProduct"), "5678\n").unwrap();
        fs::write(d2.join("busnum"), "1\n").unwrap();
        fs::write(d2.join("devnum"), "9\n").unwrap();

        let devices = discover_supported_devices_in(&sysfs, &root.join("dev/bus/usb")).unwrap();
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].vendor_id, 0x27c6);
        assert_eq!(devices[0].product_id, 0x609c);
        assert_eq!(devices[0].devnode, root.join("dev/bus/usb/001/002"));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn interface_discovery_and_transport_selection_work() {
        let root = temp_root("goodix-interfaces");
        let dev = GoodixUsbDevice {
            bus_number: 1,
            device_number: 2,
            vendor_id: 0x27c6,
            product_id: 0x609c,
            devnode: root.join("dev/bus/usb/001/002"),
            sysfs_path: root.join("sys/1-3"),
        };
        fs::create_dir_all(&dev.sysfs_path).unwrap();
        let iface = root.join("sys/1-3:1.0");
        fs::create_dir_all(&iface).unwrap();
        fs::write(iface.join("bInterfaceNumber"), "0\n").unwrap();
        fs::write(iface.join("bAlternateSetting"), "0\n").unwrap();
        fs::create_dir_all(iface.join("ep_81")).unwrap();
        fs::create_dir_all(iface.join("ep_02")).unwrap();
        fs::create_dir_all(iface.join("ep_83")).unwrap();

        let interfaces = discover_interfaces(&dev).unwrap();
        assert_eq!(interfaces.len(), 1);
        assert_eq!(interfaces[0].endpoints, vec![0x02, 0x81, 0x83]);

        let transport = select_transport_interface(&interfaces).unwrap();
        assert_eq!(transport.interface_number, 0);
        assert_eq!(transport.bulk_in_endpoint, Some(0x81));
        assert_eq!(transport.bulk_out_endpoint, Some(0x02));
        assert_eq!(transport.interrupt_in_endpoint, Some(0x83));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn parse_usb_device_descriptor_works() {
        let raw = [
            18, 1, 0x10, 0x02, 0xff, 0, 0, 64, 0xc6, 0x27, 0x9c, 0x60, 0x00, 0x01, 1, 2, 3, 1,
        ];
        let desc = parse_usb_device_descriptor(&raw).unwrap();
        assert_eq!(desc.vendor_id, 0x27c6);
        assert_eq!(desc.product_id, 0x609c);
        assert_eq!(desc.max_packet_size0, 64);
    }

    #[test]
    fn parse_usb_configuration_descriptor_works() {
        let raw = [9, 2, 16, 0, 1, 1, 0, 0x80, 50, 7, 5, 0x81, 2, 64, 0, 0];
        let desc = parse_usb_configuration_descriptor(&raw).unwrap();
        assert_eq!(desc.total_length, 16);
        assert_eq!(desc.num_interfaces, 1);
        assert_eq!(desc.attributes, 0x80);
    }

    #[test]
    fn parse_usb_string_descriptor_works() {
        let raw = [12, 3, b'G', 0, b'o', 0, b'o', 0, b'd', 0, b'i', 0];
        let s = parse_usb_utf16le_string_descriptor(&raw).unwrap();
        assert_eq!(s, "Goodi");
    }

    #[test]
    fn build_and_parse_goodix_packet_roundtrip() {
        let packet = build_goodix_package(GOODIX_CMD_GET_VERSION, 0x00, &[0x11, 0x22]);
        let parsed = parse_goodix_packet(&packet).unwrap();
        assert_eq!(parsed.header.cmd0, GOODIX_CMD_GET_VERSION);
        assert_eq!(parsed.payload, vec![0x11, 0x22]);
    }

    #[test]
    fn parse_goodix_version_info_works() {
        let mut payload = vec![0u8; 117];
        payload[0] = 0;
        payload[1..3].copy_from_slice(&[1, 0]);
        payload[3..6].copy_from_slice(b"APP");
        payload[11..16].copy_from_slice(b"1.0.0");
        let info = parse_goodix_version_info(&payload).unwrap();
        assert_eq!(info.firmware_type_string(), "APP");
        assert_eq!(info.firmware_version_string(), "1.0.0");
    }

    #[test]
    fn parse_goodix_template_works() {
        let mut raw = vec![0u8; 69 + 5 + 2];
        raw[0] = 67;
        raw[1] = 2;
        raw[2] = 7;
        raw[4..8].copy_from_slice(b"acct");
        raw[36..40].copy_from_slice(&[0x11, 0x22, 0x33, 0x44]);
        raw[68] = 5;
        raw[69..74].copy_from_slice(b"user1");
        let template = parse_goodix_template(&raw).unwrap();
        assert_eq!(template.template_type, 2);
        assert_eq!(template.finger_index, 7);
        assert_eq!(template.payload, b"user1");
    }

    #[test]
    fn parse_goodix_finger_list_works() {
        let mut entry = vec![0u8; 69 + 4 + 2];
        entry[0] = 67;
        entry[1] = 1;
        entry[2] = 3;
        entry[68] = 4;
        entry[69..73].copy_from_slice(b"ken1");
        let mut payload = vec![0, 1];
        payload.extend_from_slice(&(entry.len() as u16).to_le_bytes());
        payload.extend_from_slice(&entry);
        let templates = parse_goodix_finger_list(&payload).unwrap();
        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].finger_index, 3);
        assert_eq!(templates[0].payload, b"ken1");
    }

    #[test]
    fn parse_goodix_capture_response_works() {
        let resp = parse_goodix_capture_response(&[0x00, 90, 70]).unwrap();
        assert_eq!(resp.result, 0);
        assert_eq!(resp.image_quality, Some(90));
        assert_eq!(resp.image_coverage, Some(70));
    }

    #[test]
    fn parse_goodix_identify_match_works() {
        let mut template = vec![0u8; 69 + 4 + 2];
        template[0] = 67;
        template[1] = 1;
        template[2] = 5;
        template[68] = 4;
        template[69..73].copy_from_slice(b"user");
        let mut payload = vec![0x00];
        payload.extend_from_slice(&0x1234u16.to_le_bytes());
        payload.extend_from_slice(&77u32.to_le_bytes());
        payload.push(9);
        payload.extend_from_slice(&(template.len() as u16).to_le_bytes());
        payload.extend_from_slice(&template);
        let result = parse_goodix_identify_result(&payload).unwrap();
        assert!(result.matched);
        assert_eq!(result.reject_detail, Some(0x1234));
        assert_eq!(result.score, Some(77));
        assert_eq!(result.study, Some(9));
        assert_eq!(result.template.unwrap().finger_index, 5);
    }

    #[test]
    fn parse_goodix_identify_no_match_works() {
        let result = parse_goodix_identify_result(&[0x80]).unwrap();
        assert!(!result.matched);
        assert_eq!(result.result, 0x80);
        assert!(result.template.is_none());
    }

    #[test]
    fn parse_goodix_enroll_init_works() {
        let mut payload = vec![0u8; 33];
        payload[0] = 0;
        payload[1..5].copy_from_slice(&[1, 2, 3, 4]);
        let init = parse_goodix_enroll_init(&payload).unwrap();
        assert_eq!(init.result, 0);
        assert_eq!(init.template_id.unwrap()[..4], [1, 2, 3, 4]);
    }

    #[test]
    fn parse_goodix_enroll_update_works() {
        let update = parse_goodix_enroll_update(&[0x81, 45, 67]).unwrap();
        assert!(update.rollback);
        assert_eq!(update.overlay, 45);
        assert_eq!(update.preoverlay, 67);
    }

    #[test]
    fn parse_goodix_duplicate_check_works() {
        let mut template = vec![0u8; 69 + 4 + 2];
        template[0] = 67;
        template[2] = 8;
        template[68] = 4;
        template[69..73].copy_from_slice(b"dupe");
        let mut payload = vec![1];
        payload.extend_from_slice(&(template.len() as u16).to_le_bytes());
        payload.extend_from_slice(&template);
        let dup = parse_goodix_duplicate_check(&payload).unwrap();
        assert!(dup.duplicate);
        assert_eq!(dup.template.unwrap().finger_index, 8);
    }

    #[test]
    fn parse_goodix_identify_rejects_short_match_payload() {
        assert!(parse_goodix_identify_result(&[0x00, 0x34]).is_err());
    }

    #[test]
    fn parse_goodix_finger_list_rejects_count_over_maximum() {
        let payload = [0x00, GOODIX_MAX_STORED_PRINTS + 1];
        assert!(parse_goodix_finger_list(&payload).is_err());
    }

    #[test]
    fn parse_goodix_template_rejects_oversized_label() {
        let mut raw = vec![0u8; 69 + 57 + 2];
        raw[0] = 67;
        raw[68] = 57;
        assert!(parse_goodix_template(&raw).is_err());
    }

    #[test]
    fn build_goodix_finger_id_rejects_user_id_too_long() {
        let tid = [0x22u8; 32];
        let user = vec![0x41; 57];
        assert!(build_goodix_finger_id(&tid, &user).is_err());
    }

    #[test]
    fn build_goodix_finger_id_works() {
        let tid = [0x11u8; 32];
        let fid = build_goodix_finger_id(&tid, b"ken").unwrap();
        assert_eq!(fid[2], 67);
        assert_eq!(fid[3], 1);
        assert_eq!(fid[4], 1);
        assert_eq!(fid[38..70], [0x11; 32]);
        assert_eq!(fid[70], 3);
        assert_eq!(&fid[71..74], b"ken");
    }

    #[test]
    fn parse_goodix_simple_result_works() {
        let result = parse_goodix_simple_result(&[0]).unwrap();
        assert_eq!(result.result, 0);
    }

    #[test]
    fn parse_goodix_finger_mode_status_works() {
        let status = parse_goodix_finger_mode_status(&[0xc7]).unwrap();
        assert_eq!(status.status, 0xc7);
    }

    #[test]
    fn decode_hex_32_roundtrip_works() {
        let tid = [0xabu8; 32];
        let hex = hex_string(&tid);
        assert_eq!(decode_hex_32(&hex).unwrap(), tid);
    }

    #[test]
    fn parse_goodix_simple_result_rejects_empty() {
        assert!(parse_goodix_simple_result(&[]).is_err());
    }

    #[test]
    fn parse_goodix_finger_mode_status_rejects_empty() {
        assert!(parse_goodix_finger_mode_status(&[]).is_err());
    }

    #[test]
    fn parse_usb_language_ids_works() {
        let raw = [6, 3, 0x09, 0x04, 0x11, 0x04];
        let langs = parse_usb_language_ids(&raw).unwrap();
        assert_eq!(langs, vec![0x0409, 0x0411]);
    }

    #[test]
    fn parse_usb_language_ids_rejects_short_payload() {
        assert!(parse_usb_language_ids(&[2, 3]).is_err());
    }

    #[test]
    fn parse_goodix_ack_works() {
        let packet = GoodixPacket {
            header: GoodixPacketHeader {
                cmd0: GOODIX_RESPONSE_ACK_CMD,
                cmd1: 0,
                package_num: 0,
                reserved: 0,
                payload_len: 2,
                crc8: 0,
                rev_crc8: 0,
            },
            payload: vec![0, GOODIX_CMD_GET_VERSION],
        };
        let ack = parse_goodix_ack(&packet).unwrap();
        assert_eq!(ack.result, 0);
        assert_eq!(ack.ack_cmd, GOODIX_CMD_GET_VERSION);
    }

    #[test]
    fn parse_goodix_ack_rejects_non_ack_packet() {
        let packet = GoodixPacket {
            header: GoodixPacketHeader {
                cmd0: GOODIX_CMD_GET_VERSION,
                cmd1: 0,
                package_num: 0,
                reserved: 0,
                payload_len: 2,
                crc8: 0,
                rev_crc8: 0,
            },
            payload: vec![0, GOODIX_CMD_GET_VERSION],
        };
        assert!(parse_goodix_ack(&packet).is_err());
    }

    #[test]
    fn parse_goodix_packet_rejects_bad_crc() {
        let mut packet = build_goodix_package(GOODIX_CMD_GET_VERSION, 0x00, &[0x11, 0x22]);
        let last = packet.len() - 1;
        packet[last] ^= 0xff;
        assert!(parse_goodix_packet(&packet).is_err());
    }

    #[test]
    fn decode_hex_32_rejects_wrong_length() {
        assert!(decode_hex_32("abcd").is_none());
    }

    #[test]
    fn decode_template_id_hex_rejects_invalid_ids() {
        assert_eq!(
            decode_template_id_hex("abcd").unwrap_err(),
            FingerprintError::InvalidRequest("template_id must be 64 hex chars")
        );
    }

    #[test]
    fn capture_quality_from_raw_maps_expected_ranges() {
        assert_eq!(
            capture_quality_from_raw(0),
            edgerun_fingerprint::FingerprintCaptureQuality::Poor
        );
        assert_eq!(
            capture_quality_from_raw(25),
            edgerun_fingerprint::FingerprintCaptureQuality::Fair
        );
        assert_eq!(
            capture_quality_from_raw(50),
            edgerun_fingerprint::FingerprintCaptureQuality::Good
        );
        assert_eq!(
            capture_quality_from_raw(80),
            edgerun_fingerprint::FingerprintCaptureQuality::Excellent
        );
    }

    #[test]
    fn ensure_goodix_ok_rejects_failure_codes() {
        assert!(ensure_goodix_ok(GOODIX_SUCCESS, "capture").is_ok());
        assert!(ensure_goodix_ok(GOODIX_FAILED, "capture").is_err());
        assert!(ensure_goodix_ok(GOODIX_ERROR_NO_AVAILABLE_SPACE, "capture").is_err());
    }

    #[test]
    fn goodix_result_helpers_work() {
        let simple = GoodixSimpleResult {
            result: GOODIX_SUCCESS,
        };
        assert!(simple.is_success());
        let config = GoodixFingerConfig {
            status: GOODIX_SUCCESS,
            max_stored_prints: 20,
        };
        assert!(config.is_success());
        let capture = GoodixCaptureResponse {
            result: GOODIX_SUCCESS,
            image_quality: Some(80),
            image_coverage: Some(70),
        };
        assert!(capture.is_success());
        let finger = GoodixFingerModeStatus {
            status: GOODIX_ERROR_WAIT_FINGER_UP_TIMEOUT,
        };
        assert!(finger.is_wait_finger_up_timeout());
        assert!(!finger.is_success());
        let identify = GoodixIdentifyResult {
            matched: true,
            result: GOODIX_SUCCESS,
            reject_detail: None,
            score: None,
            study: None,
            template: None,
        };
        assert!(identify.is_success());
    }

    #[test]
    fn supported_features_report_expected_surface() {
        let reader = GoodixFingerprintReader::new(GoodixUsbDevice {
            bus_number: 1,
            device_number: 2,
            vendor_id: 0x27c6,
            product_id: 0x609c,
            devnode: PathBuf::from("/dev/bus/usb/001/002"),
            sysfs_path: PathBuf::from("/sys/bus/usb/devices/1-3"),
        })
        .unwrap();
        let features = reader.supported_features();
        assert!(features.match_on_sensor);
        assert!(features.template_listing);
        assert!(features.commit_enrollment);
        assert!(features.update_config);
        assert!(features.finger_mode_query);
        assert!(features.finger_down_mode);
        assert!(features.power_button_shield);
    }

    #[test]
    fn reader_info_reports_goodix_reader_shape() {
        let reader = GoodixFingerprintReader::new(GoodixUsbDevice {
            bus_number: 1,
            device_number: 2,
            vendor_id: 0x27c6,
            product_id: 0x609c,
            devnode: PathBuf::from("/dev/bus/usb/001/002"),
            sysfs_path: PathBuf::from("/sys/bus/usb/devices/1-3"),
        })
        .unwrap();
        let info = reader.reader_info().unwrap();
        assert_eq!(info.provider, "goodix-usb");
        assert!(info.supports_match_on_sensor);
        assert!(info.hardware_protected_match);
        assert_eq!(info.max_templates, Some(20));
    }

    #[test]
    fn parse_goodix_finger_config_works() {
        let cfg = parse_goodix_finger_config(&[GOODIX_SUCCESS, 0x00, 15]).unwrap();
        assert_eq!(cfg.status, GOODIX_SUCCESS);
        assert_eq!(cfg.max_stored_prints, 15);
    }

    #[test]
    fn parse_goodix_finger_config_defaults_capacity_for_old_firmware() {
        let cfg = parse_goodix_finger_config(&[GOODIX_SUCCESS]).unwrap();
        assert_eq!(cfg.max_stored_prints, 20);
    }

    #[test]
    fn build_default_goodix_sensor_config_matches_expected_shape() {
        let config = build_default_goodix_sensor_config();
        assert_eq!(config.config_body, GOODIX_DEFAULT_SENSOR_CONFIG_BODY);
        assert_eq!(config.reserved[0], 1);
        assert!(config.reserved[1..].iter().all(|b| *b == 0));
        let bytes = config.clone().into_bytes();
        assert_eq!(bytes.len(), GOODIX_SENSOR_CONFIG_SIZE);
        assert_eq!(
            &bytes[..GOODIX_SENSOR_CONFIG_BODY_SIZE],
            &GOODIX_DEFAULT_SENSOR_CONFIG_BODY
        );
        let expected_crc = goodix_crc32(&bytes[..GOODIX_SENSOR_CONFIG_SIZE - 4]);
        assert_eq!(config.crc32, expected_crc);
    }
}
