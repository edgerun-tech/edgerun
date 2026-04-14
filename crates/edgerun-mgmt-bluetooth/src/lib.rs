use edgerun_bluetooth::{
    BluetoothAddressKind, BluetoothBeaconObservation, BluetoothConnectionInfo,
    BluetoothConnectionProvider, BluetoothLinkKind, BluetoothProfile, BluetoothScanResult,
    BluetoothScanner, BluetoothTransportKind, default_bluetooth_descriptor,
};
use edgerun_capabilities::{CapabilityDescriptor, CapabilityError, CapabilityProvider};
use std::io;
use std::mem::size_of;
use std::os::fd::RawFd;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{fs, str};

const AF_BLUETOOTH: i32 = 31;
const SOCK_RAW: i32 = 3;
const BTPROTO_HCI: i32 = 1;
const HCI_CHANNEL_CONTROL: u16 = 3;
const HCI_DEV_NONE: u16 = 0xffff;

const MGMT_OP_READ_VERSION: u16 = 0x0001;
const MGMT_OP_READ_INDEX_LIST: u16 = 0x0003;
const MGMT_OP_READ_INFO: u16 = 0x0004;
const MGMT_OP_SET_POWERED: u16 = 0x0005;
const MGMT_OP_SET_DISCOVERABLE: u16 = 0x0006;
const MGMT_OP_SET_CONNECTABLE: u16 = 0x0007;
const MGMT_OP_SET_PAIRABLE: u16 = 0x0009;
const MGMT_OP_SET_LOCAL_NAME: u16 = 0x000f;
const MGMT_OP_GET_CONNECTIONS: u16 = 0x0015;
const MGMT_OP_CONNECT: u16 = 0x000e;
const MGMT_OP_DISCONNECT: u16 = 0x0014;
const MGMT_OP_PAIR_DEVICE: u16 = 0x0019;
const MGMT_OP_CANCEL_PAIRING: u16 = 0x001b;
const MGMT_OP_START_DISCOVERY: u16 = 0x0023;
const MGMT_OP_STOP_DISCOVERY: u16 = 0x0024;

const MGMT_EV_CMD_COMPLETE: u16 = 0x0001;
const MGMT_EV_CMD_STATUS: u16 = 0x0002;
const MGMT_EV_NEW_SETTINGS: u16 = 0x0006;
const MGMT_EV_CLASS_OF_DEVICE_CHANGED: u16 = 0x0007;
const MGMT_EV_LOCAL_NAME_CHANGED: u16 = 0x0008;
const MGMT_EV_DEVICE_CONNECTED: u16 = 0x000b;
const MGMT_EV_DEVICE_DISCONNECTED: u16 = 0x000c;
const MGMT_EV_CONNECT_FAILED: u16 = 0x000d;
const MGMT_EV_DEVICE_FOUND: u16 = 0x0012;
const MGMT_EV_DISCOVERING: u16 = 0x0013;

const MGMT_ADDR_BREDR: u8 = 0x01;
const MGMT_ADDR_LE_PUBLIC: u8 = 0x02;
const MGMT_ADDR_LE_RANDOM: u8 = 0x04;
const MGMT_ADDR_LE_ANY: u8 = MGMT_ADDR_LE_PUBLIC | MGMT_ADDR_LE_RANDOM;
const MGMT_ADDR_ALL: u8 = MGMT_ADDR_BREDR | MGMT_ADDR_LE_ANY;

#[repr(C)]
#[derive(Clone, Copy)]
struct SockAddrHci {
    hci_family: u16,
    hci_dev: u16,
    hci_channel: u16,
}

unsafe extern "C" {
    fn socket(domain: i32, ty: i32, protocol: i32) -> i32;
    fn bind(fd: i32, addr: *const core::ffi::c_void, len: u32) -> i32;
    fn send(fd: i32, buf: *const core::ffi::c_void, len: usize, flags: i32) -> isize;
    fn recv(fd: i32, buf: *mut core::ffi::c_void, len: usize, flags: i32) -> isize;
    fn poll(fds: *mut PollFd, nfds: usize, timeout: i32) -> i32;
    fn close(fd: i32) -> i32;
}

#[repr(C)]
#[derive(Clone, Copy)]
struct PollFd {
    fd: i32,
    events: i16,
    revents: i16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MgmtDiscoveryTransport {
    Classic,
    LowEnergy,
    Interleaved,
}

impl MgmtDiscoveryTransport {
    pub fn address_mask(self) -> u8 {
        match self {
            Self::Classic => MGMT_ADDR_BREDR,
            Self::LowEnergy => MGMT_ADDR_LE_ANY,
            Self::Interleaved => MGMT_ADDR_ALL,
        }
    }
}

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
pub struct MgmtBluetoothBackend {
    pub controller: MgmtControllerInfo,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BluezDeviceRecord {
    pub controller_address: String,
    pub device_address: String,
    pub address_kind: BluetoothAddressKind,
    pub local_name: Option<String>,
    pub transport_kind: BluetoothTransportKind,
    pub service_uuids: Vec<String>,
    pub profiles: Vec<BluetoothProfile>,
    pub trusted: Option<bool>,
    pub paired: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MgmtControllerEvent {
    SettingsChanged {
        index: u16,
        settings_raw: u32,
        settings: MgmtControllerSettings,
    },
    ClassOfDeviceChanged {
        index: u16,
        class_of_device: u32,
    },
    LocalNameChanged {
        index: u16,
        name: String,
        short_name: String,
    },
    DeviceConnected {
        index: u16,
        device_id: String,
        transport_kind: BluetoothTransportKind,
        address_kind: BluetoothAddressKind,
        flags: u32,
        local_name: Option<String>,
        service_uuids: Vec<String>,
        profiles: Vec<BluetoothProfile>,
    },
    DeviceDisconnected {
        index: u16,
        device_id: String,
        transport_kind: BluetoothTransportKind,
        address_kind: BluetoothAddressKind,
        reason: u8,
    },
    ConnectFailed {
        index: u16,
        device_id: String,
        transport_kind: BluetoothTransportKind,
        address_kind: BluetoothAddressKind,
        status: u8,
    },
    DiscoveringChanged {
        index: u16,
        address_mask: u8,
        discovering: bool,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct MgmtEvent {
    opcode: u16,
    index: u16,
    payload: Vec<u8>,
}

struct MgmtSocket(RawFd);

impl MgmtSocket {
    fn open() -> Result<Self, CapabilityError> {
        let fd = unsafe { socket(AF_BLUETOOTH, SOCK_RAW, BTPROTO_HCI) };
        if fd < 0 {
            return Err(last_socket_error(
                "failed to open Bluetooth management socket",
            ));
        }
        let addr = SockAddrHci {
            hci_family: AF_BLUETOOTH as u16,
            hci_dev: HCI_DEV_NONE,
            hci_channel: HCI_CHANNEL_CONTROL,
        };
        let rc = unsafe {
            bind(
                fd,
                (&addr as *const SockAddrHci).cast(),
                size_of::<SockAddrHci>() as u32,
            )
        };
        if rc < 0 {
            let err = last_socket_error("failed to bind Bluetooth management socket");
            unsafe { close(fd) };
            return Err(err);
        }
        Ok(Self(fd))
    }

    fn send_command(&self, opcode: u16, index: u16, payload: &[u8]) -> Result<(), CapabilityError> {
        let packet = build_mgmt_packet(opcode, index, payload);
        let rc = unsafe { send(self.0, packet.as_ptr().cast(), packet.len(), 0) };
        if rc < 0 {
            return Err(last_socket_error("failed to send management command"));
        }
        Ok(())
    }

    fn recv_event(&self, timeout_ms: i32) -> Result<Option<MgmtEvent>, CapabilityError> {
        let mut pollfd = PollFd {
            fd: self.0,
            events: 0x0001,
            revents: 0,
        };
        let poll_rc = unsafe { poll(&mut pollfd, 1, timeout_ms) };
        if poll_rc < 0 {
            return Err(last_socket_error("failed to poll management socket"));
        }
        if poll_rc == 0 {
            return Ok(None);
        }
        let mut buf = vec![0u8; 4096];
        let read = unsafe { recv(self.0, buf.as_mut_ptr().cast(), buf.len(), 0) };
        if read < 0 {
            return Err(last_socket_error("failed to receive management event"));
        }
        buf.truncate(read as usize);
        parse_mgmt_event(&buf).map(Some)
    }

    fn command(&self, opcode: u16, index: u16, payload: &[u8]) -> Result<Vec<u8>, CapabilityError> {
        self.send_command(opcode, index, payload)?;
        wait_for_command_result(self, opcode, index, 2_000)
    }
}

impl Drop for MgmtSocket {
    fn drop(&mut self) {
        unsafe { close(self.0) };
    }
}

fn last_socket_error(context: &str) -> CapabilityError {
    let err = io::Error::last_os_error();
    match err.kind() {
        io::ErrorKind::PermissionDenied => CapabilityError::PermissionDenied(
            "bluetooth management operation requires elevated permission",
        ),
        _ => CapabilityError::Provider(format!("{context}: {err}")),
    }
}

fn system_time_unix_ms() -> Result<i64, CapabilityError> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| CapabilityError::Provider(e.to_string()))?;
    Ok(duration.as_millis() as i64)
}

fn build_mgmt_packet(opcode: u16, index: u16, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(6 + payload.len());
    out.extend_from_slice(&opcode.to_le_bytes());
    out.extend_from_slice(&index.to_le_bytes());
    out.extend_from_slice(&(payload.len() as u16).to_le_bytes());
    out.extend_from_slice(payload);
    out
}

fn parse_mgmt_event(packet: &[u8]) -> Result<MgmtEvent, CapabilityError> {
    if packet.len() < 6 {
        return Err(CapabilityError::Provider("short management event".into()));
    }
    let opcode = u16::from_le_bytes([packet[0], packet[1]]);
    let index = u16::from_le_bytes([packet[2], packet[3]]);
    let len = u16::from_le_bytes([packet[4], packet[5]]) as usize;
    if packet.len() < 6 + len {
        return Err(CapabilityError::Provider(
            "truncated management event".into(),
        ));
    }
    Ok(MgmtEvent {
        opcode,
        index,
        payload: packet[6..6 + len].to_vec(),
    })
}

fn mgmt_status_name(status: u8) -> &'static str {
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

fn wait_for_command_result(
    socket: &MgmtSocket,
    expected_opcode: u16,
    expected_index: u16,
    timeout_ms: i32,
) -> Result<Vec<u8>, CapabilityError> {
    let mut remaining = timeout_ms;
    while remaining > 0 {
        let poll_ms = remaining.min(250);
        let Some(event) = socket.recv_event(poll_ms)? else {
            remaining -= poll_ms;
            continue;
        };
        if event.opcode == MGMT_EV_CMD_COMPLETE && event.payload.len() >= 3 {
            let cmd_opcode = u16::from_le_bytes([event.payload[0], event.payload[1]]);
            let status = event.payload[2];
            if cmd_opcode == expected_opcode && event.index == expected_index {
                if status == 0 {
                    return Ok(event.payload[3..].to_vec());
                }
                return Err(CapabilityError::Provider(format!(
                    "mgmt command 0x{cmd_opcode:04x} failed with status 0x{status:02x} ({})",
                    mgmt_status_name(status)
                )));
            }
        }
        if event.opcode == MGMT_EV_CMD_STATUS && event.payload.len() >= 3 {
            let cmd_opcode = u16::from_le_bytes([event.payload[0], event.payload[1]]);
            let status = event.payload[2];
            if cmd_opcode == expected_opcode && event.index == expected_index {
                if status == 0 {
                    continue;
                }
                return Err(CapabilityError::Provider(format!(
                    "mgmt command 0x{cmd_opcode:04x} status 0x{status:02x} ({})",
                    mgmt_status_name(status)
                )));
            }
        }
    }
    Err(CapabilityError::Provider(format!(
        "timeout waiting for mgmt command 0x{expected_opcode:04x}"
    )))
}

fn format_bdaddr_le(bytes: &[u8]) -> String {
    bytes
        .iter()
        .rev()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(":")
}

/// Parse a BDADDR string (e.g. "AA:BB:CC:DD:EE:FF") into little-endian bytes
/// as used by the kernel mgmt protocol.
fn parse_bdaddr(addr: &str) -> Result<[u8; 6], CapabilityError> {
    edgerun_encoding::hex::parse_bdaddr(addr)
        .ok_or_else(|| CapabilityError::Provider(format!("invalid BDADDR format: {}", addr).into()))
}

fn parse_settings(bits: u32) -> MgmtControllerSettings {
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

fn parse_controller_info(
    index: u16,
    payload: &[u8],
) -> Result<MgmtControllerInfo, CapabilityError> {
    if payload.len() < 6 + 1 + 2 + 4 + 4 + 3 + 249 + 11 {
        return Err(CapabilityError::Provider(
            "short mgmt controller info payload".into(),
        ));
    }
    let address = format_bdaddr_le(&payload[0..6]);
    let bluetooth_version = payload[6];
    let manufacturer = u16::from_le_bytes([payload[7], payload[8]]);
    let supported_settings_raw =
        u32::from_le_bytes([payload[9], payload[10], payload[11], payload[12]]);
    let current_settings_raw =
        u32::from_le_bytes([payload[13], payload[14], payload[15], payload[16]]);
    let class_of_device =
        u32::from(payload[17]) | (u32::from(payload[18]) << 8) | (u32::from(payload[19]) << 16);
    let name = edgerun_encoding::cstring::decode_c_string(&payload[20..269]).unwrap_or_default();
    let short_name = edgerun_encoding::cstring::decode_c_string(&payload[269..280]).unwrap_or_default();
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

fn bool_from_ini(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

fn address_kind_from_str(value: &str) -> BluetoothAddressKind {
    match value.trim().to_ascii_lowercase().as_str() {
        "public" => BluetoothAddressKind::Public,
        "random" => BluetoothAddressKind::Random,
        _ => BluetoothAddressKind::Unknown,
    }
}

fn transport_from_technologies(value: &str) -> BluetoothTransportKind {
    let lower = value.to_ascii_lowercase();
    let bredr = lower.contains("br/edr");
    let le = lower.contains("le");
    match (bredr, le) {
        (true, true) => BluetoothTransportKind::DualMode,
        (true, false) => BluetoothTransportKind::Classic,
        (false, true) => BluetoothTransportKind::LowEnergy,
        _ => BluetoothTransportKind::Unknown,
    }
}

fn parse_service_uuids(value: &str) -> Vec<String> {
    value
        .split([';', ','])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .filter_map(normalize_service_uuid)
        .collect()
}

fn is_hex(s: &str) -> bool {
    s.chars().all(|c| c.is_ascii_hexdigit())
}

fn normalize_service_uuid(value: &str) -> Option<String> {
    let raw = value
        .trim()
        .trim_start_matches('{')
        .trim_end_matches('}')
        .trim_start_matches("0x")
        .trim_start_matches("0X")
        .to_ascii_lowercase();
    let compact = raw.replace('-', "");
    if !is_hex(&compact) {
        return None;
    }
    match compact.len() {
        4 => Some(format_16bit_service_uuid(
            u16::from_str_radix(&compact, 16).ok()?,
        )),
        8 => Some(format_32bit_service_uuid(
            u32::from_str_radix(&compact, 16).ok()?,
        )),
        32 => Some(format!(
            "{}-{}-{}-{}-{}",
            &compact[0..8],
            &compact[8..12],
            &compact[12..16],
            &compact[16..20],
            &compact[20..32]
        )),
        _ => None,
    }
}

fn parse_uuid16_from_service_uuid(raw: &str) -> Option<u16> {
    let compact = raw
        .trim()
        .trim_start_matches('{')
        .trim_end_matches('}')
        .trim_start_matches("0x")
        .trim_start_matches("0X")
        .replace('-', "");
    let compact = compact.to_ascii_lowercase();
    match compact.len() {
        4 if is_hex(&compact) => u16::from_str_radix(&compact, 16).ok(),
        8 if is_hex(&compact) && compact.starts_with("0000") => {
            u16::from_str_radix(&compact[4..], 16).ok()
        }
        32 if is_hex(&compact)
            && compact.starts_with("0000")
            && compact.ends_with("00001000800000805f9b34fb") =>
        {
            u16::from_str_radix(&compact[4..8], 16).ok()
        }
        _ => None,
    }
}

fn profiles_from_services(services: &[String]) -> Vec<BluetoothProfile> {
    let mut profiles = Vec::new();
    for svc in services {
        if let Some(uuid) = parse_uuid16_from_service_uuid(svc) {
            profiles.extend(infer_profiles_from_uuid16(uuid));
        }
    }
    dedup_vec(&mut profiles);
    profiles
}

fn dedup_vec<T: PartialEq>(values: &mut Vec<T>) {
    let mut out = Vec::new();
    for value in values.drain(..) {
        if !out.contains(&value) {
            out.push(value);
        }
    }
    *values = out;
}

fn parse_ini_map(
    text: &str,
) -> std::collections::BTreeMap<String, std::collections::BTreeMap<String, String>> {
    let mut out: std::collections::BTreeMap<String, std::collections::BTreeMap<String, String>> =
        std::collections::BTreeMap::new();
    let mut current = String::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            current = line[1..line.len() - 1].to_string();
            out.entry(current.clone()).or_default();
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            out.entry(current.clone())
                .or_default()
                .insert(k.trim().to_string(), v.trim().to_string());
        }
    }
    out
}

fn parse_bluez_device_record(
    controller_address: &str,
    device_address: &str,
    text: &str,
) -> BluezDeviceRecord {
    let ini = parse_ini_map(text);
    let general = ini.get("General");
    let local_name = general.and_then(|m| m.get("Name")).cloned();
    let address_kind = general
        .and_then(|m| m.get("AddressType"))
        .map(|s| address_kind_from_str(s))
        .unwrap_or(BluetoothAddressKind::Unknown);
    let transport_kind = general
        .and_then(|m| m.get("SupportedTechnologies"))
        .map(|s| transport_from_technologies(s))
        .unwrap_or(BluetoothTransportKind::Unknown);
    let service_uuids = general
        .and_then(|m| m.get("Services"))
        .map(|s| parse_service_uuids(s))
        .unwrap_or_default();
    let profiles = profiles_from_services(&service_uuids);
    let trusted = general
        .and_then(|m| m.get("Trusted"))
        .and_then(|s| bool_from_ini(s));
    let paired = Some(
        ini.contains_key("LinkKey")
            || ini.contains_key("LongTermKey")
            || ini.contains_key("PeripheralLongTermKey")
            || ini.contains_key("SlaveLongTermKey"),
    );
    BluezDeviceRecord {
        controller_address: controller_address.to_string(),
        device_address: device_address.to_string(),
        address_kind,
        local_name,
        transport_kind,
        service_uuids,
        profiles,
        trusted,
        paired,
    }
}

fn bluez_controller_dir(controller_address: &str) -> PathBuf {
    Path::new("/var/lib/bluetooth").join(controller_address)
}

pub fn read_bluez_device_record(
    controller_address: &str,
    device_address: &str,
) -> Result<Option<BluezDeviceRecord>, CapabilityError> {
    let base = bluez_controller_dir(controller_address);
    let info_path = base.join(device_address).join("info");
    let cache_path = base.join("cache").join(device_address);
    let source = if info_path.exists() {
        info_path
    } else if cache_path.exists() {
        cache_path
    } else {
        return Ok(None);
    };
    let text = fs::read_to_string(&source).map_err(|e| CapabilityError::Provider(e.to_string()))?;
    Ok(Some(parse_bluez_device_record(
        controller_address,
        device_address,
        &text,
    )))
}

pub fn list_bluez_known_devices(
    controller_address: &str,
) -> Result<Vec<BluezDeviceRecord>, CapabilityError> {
    let base = bluez_controller_dir(controller_address);
    if !base.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    let entries = fs::read_dir(&base).map_err(|e| CapabilityError::Provider(e.to_string()))?;
    for entry in entries {
        let entry = entry.map_err(|e| CapabilityError::Provider(e.to_string()))?;
        let name = entry.file_name().to_string_lossy().to_string();
        let path = entry.path();
        if path.is_dir() && name == "cache" {
            for child in fs::read_dir(path).map_err(|e| CapabilityError::Provider(e.to_string()))? {
                let child = child.map_err(|e| CapabilityError::Provider(e.to_string()))?;
                let addr = child.file_name().to_string_lossy().to_string();
                if let Some(record) = read_bluez_device_record(controller_address, &addr)? {
                    out.push(record);
                }
            }
        } else if path.is_dir() && name.contains(':') {
            if let Some(record) = read_bluez_device_record(controller_address, &name)? {
                out.push(record);
            }
        }
    }
    out.sort_by(|a, b| a.device_address.cmp(&b.device_address));
    out.dedup_by(|a, b| a.device_address == b.device_address);
    Ok(out)
}

fn infer_profiles_from_uuid16(uuid: u16) -> Vec<BluetoothProfile> {
    match uuid {
        0x1108 => vec![BluetoothProfile::Headset],
        0x111e | 0x111f => vec![BluetoothProfile::HandsFree],
        0x110a => vec![BluetoothProfile::AudioSource],
        0x110b => vec![BluetoothProfile::AudioSink],
        0x110c | 0x110d => vec![BluetoothProfile::AudioSink],
        0x1124 => vec![BluetoothProfile::Hid],
        0x180d => vec![BluetoothProfile::HeartRate],
        0x180f => vec![BluetoothProfile::BatteryService],
        0x184f => vec![BluetoothProfile::HearingAid],
        _ => Vec::new(),
    }
}

#[derive(Default)]
struct ParsedDiscoveryData {
    local_name: Option<String>,
    service_uuids: Vec<String>,
    tx_power_dbm: Option<i16>,
    profiles: Vec<BluetoothProfile>,
}

fn format_16bit_service_uuid(uuid: u16) -> String {
    format!("0000{uuid:04x}-0000-1000-8000-00805f9b34fb")
}

fn format_32bit_service_uuid(uuid: u32) -> String {
    format!("{uuid:08x}-0000-1000-8000-00805f9b34fb")
}

fn format_128bit_service_uuid(bytes_le: &[u8; 16]) -> String {
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes_le[0],
        bytes_le[1],
        bytes_le[2],
        bytes_le[3],
        bytes_le[4],
        bytes_le[5],
        bytes_le[6],
        bytes_le[7],
        bytes_le[8],
        bytes_le[9],
        bytes_le[10],
        bytes_le[11],
        bytes_le[12],
        bytes_le[13],
        bytes_le[14],
        bytes_le[15]
    )
}

fn parse_eir_or_ad_data(data: &[u8]) -> ParsedDiscoveryData {
    let mut out = ParsedDiscoveryData::default();
    let mut offset = 0usize;
    while offset < data.len() {
        let len = data[offset] as usize;
        if len == 0 || offset + 1 + len > data.len() {
            break;
        }
        let ty = data[offset + 1];
        let value = &data[offset + 2..offset + 1 + len];
        match ty {
            0x08
                if out.local_name.is_none() => {
                    out.local_name = Some(String::from_utf8_lossy(value).to_string());
                }
            0x09
                if !value.is_empty() => {
                    out.local_name = Some(String::from_utf8_lossy(value).to_string());
                }
            0x02 | 0x03 => {
                for chunk in value.chunks_exact(2) {
                    parse_uuid16_in_discovery_payload(
                        &mut out.service_uuids,
                        &mut out.profiles,
                        u16::from_le_bytes([chunk[0], chunk[1]]),
                    );
                }
            }
            0x04 | 0x05 => {
                for chunk in value.chunks_exact(4) {
                    let uuid = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                    out.service_uuids.push(format_32bit_service_uuid(uuid));
                    if let Some(profile_uuid) = map_profile_from_32bit_uuid(uuid) {
                        out.profiles.extend(profile_uuid);
                    }
                }
            }
            0x06 | 0x07 => {
                for chunk in value.chunks_exact(16) {
                    let mut b = [0u8; 16];
                    b.copy_from_slice(chunk);
                    b.reverse();
                    out.service_uuids.push(format_128bit_service_uuid(&b));
                }
            }
            0x14 => {
                for chunk in value.chunks_exact(2) {
                    parse_uuid16_in_discovery_payload(
                        &mut out.service_uuids,
                        &mut out.profiles,
                        u16::from_le_bytes([chunk[0], chunk[1]]),
                    );
                }
            }
            0x16
                if value.len() >= 2 => {
                    let uuid = u16::from_le_bytes([value[0], value[1]]);
                    parse_uuid16_in_discovery_payload(
                        &mut out.service_uuids,
                        &mut out.profiles,
                        uuid,
                    );
                }
            0x20
                if value.len() >= 4 => {
                    let uuid = u32::from_le_bytes([value[0], value[1], value[2], value[3]]);
                    out.service_uuids.push(format_32bit_service_uuid(uuid));
                    if let Some(profile_uuid) = map_profile_from_32bit_uuid(uuid) {
                        out.profiles.extend(profile_uuid);
                    }
                }
            0x21
                if value.len() >= 16 => {
                    let mut b = [0u8; 16];
                    b.copy_from_slice(&value[..16]);
                    b.reverse();
                    let value = format_128bit_service_uuid(&b);
                    out.service_uuids.push(value);
                }
            0x15 => {
                for chunk in value.chunks_exact(16) {
                    let mut b = [0u8; 16];
                    b.copy_from_slice(chunk);
                    b.reverse();
                    out.service_uuids.push(format_128bit_service_uuid(&b));
                }
            }
            0x0a => {
                if let Some(v) = value.first() {
                    out.tx_power_dbm = Some(*v as i8 as i16);
                }
            }
            _ => {}
        }
        offset += len + 1;
    }
    dedup_vec(&mut out.service_uuids);
    dedup_vec(&mut out.profiles);
    out
}

fn parse_uuid16_in_discovery_payload(
    service_uuids: &mut Vec<String>,
    profiles: &mut Vec<BluetoothProfile>,
    uuid: u16,
) {
    service_uuids.push(format_16bit_service_uuid(uuid));
    profiles.extend(infer_profiles_from_uuid16(uuid));
}

fn map_profile_from_32bit_uuid(uuid: u32) -> Option<Vec<BluetoothProfile>> {
    if uuid <= u16::MAX as u32 {
        Some(infer_profiles_from_uuid16(uuid as u16))
    } else {
        None
    }
}

fn map_addr_type(value: u8) -> (BluetoothTransportKind, BluetoothAddressKind) {
    match value {
        0 => (
            BluetoothTransportKind::Classic,
            BluetoothAddressKind::Public,
        ),
        1 => (
            BluetoothTransportKind::LowEnergy,
            BluetoothAddressKind::Public,
        ),
        2 => (
            BluetoothTransportKind::LowEnergy,
            BluetoothAddressKind::Random,
        ),
        _ => (
            BluetoothTransportKind::Unknown,
            BluetoothAddressKind::Unknown,
        ),
    }
}

fn parse_mgmt_controller_event(event: &MgmtEvent) -> Option<MgmtControllerEvent> {
    match event.opcode {
        MGMT_EV_NEW_SETTINGS if event.payload.len() >= 4 => {
            let settings_raw = u32::from_le_bytes([
                event.payload[0],
                event.payload[1],
                event.payload[2],
                event.payload[3],
            ]);
            Some(MgmtControllerEvent::SettingsChanged {
                index: event.index,
                settings_raw,
                settings: parse_settings(settings_raw),
            })
        }
        MGMT_EV_CLASS_OF_DEVICE_CHANGED if event.payload.len() >= 3 => {
            let class_of_device = u32::from(event.payload[0])
                | (u32::from(event.payload[1]) << 8)
                | (u32::from(event.payload[2]) << 16);
            Some(MgmtControllerEvent::ClassOfDeviceChanged {
                index: event.index,
                class_of_device,
            })
        }
        MGMT_EV_LOCAL_NAME_CHANGED if event.payload.len() >= 260 => {
            let name = edgerun_encoding::cstring::decode_c_string(&event.payload[0..249]).unwrap_or_default();
            let short_name = edgerun_encoding::cstring::decode_c_string(&event.payload[249..260]).unwrap_or_default();
            Some(MgmtControllerEvent::LocalNameChanged {
                index: event.index,
                name,
                short_name,
            })
        }
        MGMT_EV_DEVICE_CONNECTED if event.payload.len() >= 13 => {
            let device_id = format_bdaddr_le(&event.payload[0..6]);
            let addr_type = event.payload[6];
            let flags = u32::from_le_bytes([
                event.payload[7],
                event.payload[8],
                event.payload[9],
                event.payload[10],
            ]);
            let eir_len = u16::from_le_bytes([event.payload[11], event.payload[12]]) as usize;
            let eir = if event.payload.len() >= 13 + eir_len {
                event.payload[13..13 + eir_len].to_vec()
            } else {
                Vec::new()
            };
            let parsed = parse_eir_or_ad_data(&eir);
            let (transport_kind, address_kind) = map_addr_type(addr_type);
            Some(MgmtControllerEvent::DeviceConnected {
                index: event.index,
                device_id,
                transport_kind,
                address_kind,
                flags,
                local_name: parsed.local_name,
                service_uuids: parsed.service_uuids,
                profiles: parsed.profiles,
            })
        }
        MGMT_EV_DEVICE_DISCONNECTED if event.payload.len() >= 8 => {
            let device_id = format_bdaddr_le(&event.payload[0..6]);
            let addr_type = event.payload[6];
            let reason = event.payload[7];
            let (transport_kind, address_kind) = map_addr_type(addr_type);
            Some(MgmtControllerEvent::DeviceDisconnected {
                index: event.index,
                device_id,
                transport_kind,
                address_kind,
                reason,
            })
        }
        MGMT_EV_CONNECT_FAILED if event.payload.len() >= 8 => {
            let device_id = format_bdaddr_le(&event.payload[0..6]);
            let addr_type = event.payload[6];
            let status = event.payload[7];
            let (transport_kind, address_kind) = map_addr_type(addr_type);
            Some(MgmtControllerEvent::ConnectFailed {
                index: event.index,
                device_id,
                transport_kind,
                address_kind,
                status,
            })
        }
        MGMT_EV_DISCOVERING if event.payload.len() >= 2 => {
            Some(MgmtControllerEvent::DiscoveringChanged {
                index: event.index,
                address_mask: event.payload[0],
                discovering: event.payload[1] != 0,
            })
        }
        _ => None,
    }
}

pub fn read_management_version() -> Result<MgmtVersionInfo, CapabilityError> {
    let socket = MgmtSocket::open()?;
    let payload = socket.command(MGMT_OP_READ_VERSION, HCI_DEV_NONE, &[])?;
    if payload.len() < 3 {
        return Err(CapabilityError::Provider(
            "short mgmt version payload".into(),
        ));
    }
    Ok(MgmtVersionInfo {
        version: payload[0],
        revision: u16::from_le_bytes([payload[1], payload[2]]),
    })
}

pub fn read_controller_indices() -> Result<Vec<u16>, CapabilityError> {
    let socket = MgmtSocket::open()?;
    let payload = socket.command(MGMT_OP_READ_INDEX_LIST, HCI_DEV_NONE, &[])?;
    if payload.len() < 2 {
        return Err(CapabilityError::Provider(
            "short mgmt index list payload".into(),
        ));
    }
    let count = u16::from_le_bytes([payload[0], payload[1]]) as usize;
    let mut out = Vec::new();
    for chunk in payload[2..].chunks_exact(2).take(count) {
        out.push(u16::from_le_bytes([chunk[0], chunk[1]]));
    }
    Ok(out)
}

pub fn read_controller_info(index: u16) -> Result<MgmtControllerInfo, CapabilityError> {
    let socket = MgmtSocket::open()?;
    let payload = socket.command(MGMT_OP_READ_INFO, index, &[])?;
    parse_controller_info(index, &payload)
}

pub fn discover_controllers() -> Result<Vec<MgmtControllerInfo>, CapabilityError> {
    let mut out = Vec::new();
    for index in read_controller_indices()? {
        out.push(read_controller_info(index)?);
    }
    out.sort_by_key(|c| c.index);
    Ok(out)
}

pub fn set_controller_powered(
    index: u16,
    powered: bool,
) -> Result<MgmtControllerInfo, CapabilityError> {
    let socket = MgmtSocket::open()?;
    let payload = socket.command(MGMT_OP_SET_POWERED, index, &[powered as u8])?;
    if payload.len() < 4 {
        return Err(CapabilityError::Provider(
            "short set-powered payload".into(),
        ));
    }
    let settings_raw = u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
    let mut info = read_controller_info(index)?;
    info.current_settings_raw = settings_raw;
    info.current_settings = parse_settings(settings_raw);
    Ok(info)
}

pub fn set_controller_connectable(
    index: u16,
    connectable: bool,
) -> Result<MgmtControllerInfo, CapabilityError> {
    let socket = MgmtSocket::open()?;
    let payload = socket.command(MGMT_OP_SET_CONNECTABLE, index, &[connectable as u8])?;
    if payload.len() < 4 {
        return Err(CapabilityError::Provider(
            "short set-connectable payload".into(),
        ));
    }
    let settings_raw = u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
    let mut info = read_controller_info(index)?;
    info.current_settings_raw = settings_raw;
    info.current_settings = parse_settings(settings_raw);
    Ok(info)
}

pub fn set_controller_discoverable(
    index: u16,
    discoverable: bool,
    timeout_secs: u16,
) -> Result<MgmtControllerInfo, CapabilityError> {
    let socket = MgmtSocket::open()?;
    let mut params = Vec::with_capacity(3);
    params.push(discoverable as u8);
    params.extend_from_slice(&timeout_secs.to_le_bytes());
    let payload = socket.command(MGMT_OP_SET_DISCOVERABLE, index, &params)?;
    if payload.len() < 4 {
        return Err(CapabilityError::Provider(
            "short set-discoverable payload".into(),
        ));
    }
    let settings_raw = u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
    let mut info = read_controller_info(index)?;
    info.current_settings_raw = settings_raw;
    info.current_settings = parse_settings(settings_raw);
    Ok(info)
}

pub fn set_controller_pairable(
    index: u16,
    pairable: bool,
) -> Result<MgmtControllerInfo, CapabilityError> {
    let socket = MgmtSocket::open()?;
    let payload = socket.command(MGMT_OP_SET_PAIRABLE, index, &[pairable as u8])?;
    if payload.len() < 4 {
        return Err(CapabilityError::Provider(
            "short set-pairable payload".into(),
        ));
    }
    let settings_raw = u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
    let mut info = read_controller_info(index)?;
    info.current_settings_raw = settings_raw;
    info.current_settings = parse_settings(settings_raw);
    Ok(info)
}

pub fn set_controller_local_name(
    index: u16,
    name: &str,
    short_name: &str,
) -> Result<MgmtControllerInfo, CapabilityError> {
    fn encode_fixed_nul(s: &str, len: usize) -> Result<Vec<u8>, CapabilityError> {
        if s.as_bytes().contains(&0) {
            return Err(CapabilityError::Provider(
                "local name must not contain interior NUL".into(),
            ));
        }
        if s.len() + 1 > len {
            return Err(CapabilityError::Provider(format!(
                "local name field too long for fixed buffer of {len}"
            )));
        }
        let mut out = vec![0u8; len];
        out[..s.len()].copy_from_slice(s.as_bytes());
        Ok(out)
    }

    let socket = MgmtSocket::open()?;
    let mut params = encode_fixed_nul(name, 249)?;
    params.extend_from_slice(&encode_fixed_nul(short_name, 11)?);
    let payload = socket.command(MGMT_OP_SET_LOCAL_NAME, index, &params)?;
    if payload.len() < 260 {
        return Err(CapabilityError::Provider(
            "short set-local-name payload".into(),
        ));
    }
    let mut info = read_controller_info(index)?;
    info.name = edgerun_encoding::cstring::decode_c_string(&payload[0..249]).unwrap_or_default();
    info.short_name = edgerun_encoding::cstring::decode_c_string(&payload[249..260]).unwrap_or_default();
    Ok(info)
}

impl MgmtBluetoothBackend {
    pub fn monitor_events(
        &self,
        timeout_ms: u32,
    ) -> Result<Vec<MgmtControllerEvent>, CapabilityError> {
        let socket = MgmtSocket::open()?;
        let start = system_time_unix_ms()?;
        let mut remaining = timeout_ms as i32;
        let mut out = Vec::new();
        while remaining > 0 {
            let poll_ms = remaining.min(250);
            let Some(event) = socket.recv_event(poll_ms)? else {
                remaining -= poll_ms;
                continue;
            };
            if event.index != self.controller.index {
                remaining = timeout_ms as i32 - (system_time_unix_ms()? - start) as i32;
                continue;
            }
            if let Some(parsed) = parse_mgmt_controller_event(&event) {
                out.push(parsed);
            }
            remaining = timeout_ms as i32 - (system_time_unix_ms()? - start) as i32;
        }
        Ok(out)
    }

    pub fn discover_nearby(
        &self,
        transport: MgmtDiscoveryTransport,
        timeout_ms: u32,
    ) -> Result<BluetoothScanResult, CapabilityError> {
        let socket = MgmtSocket::open()?;
        let mask = transport.address_mask();
        socket.send_command(MGMT_OP_START_DISCOVERY, self.controller.index, &[mask])?;
        if let Err(err) = wait_for_command_result(
            &socket,
            MGMT_OP_START_DISCOVERY,
            self.controller.index,
            2_000,
        ) {
            if err.to_string().contains("status 0x0a") {
                let _ = socket.send_command(MGMT_OP_STOP_DISCOVERY, self.controller.index, &[mask]);
                let _ = wait_for_command_result(
                    &socket,
                    MGMT_OP_STOP_DISCOVERY,
                    self.controller.index,
                    500,
                );
                socket.send_command(MGMT_OP_START_DISCOVERY, self.controller.index, &[mask])?;
                if let Err(err2) = wait_for_command_result(
                    &socket,
                    MGMT_OP_START_DISCOVERY,
                    self.controller.index,
                    2_000,
                ) {
                    if err2.to_string().contains("status 0x0a") {
                        return self.discover_cached_or_known();
                    }
                    return Err(err2);
                }
            } else {
                return Err(err);
            }
        }

        let start = system_time_unix_ms()?;
        let mut remaining = timeout_ms as i32;
        let mut observations = Vec::new();
        while remaining > 0 {
            let poll_ms = remaining.min(250);
            let Some(event) = socket.recv_event(poll_ms)? else {
                remaining -= poll_ms;
                continue;
            };
            if event.index != self.controller.index {
                remaining -= poll_ms;
                continue;
            }
            if event.opcode == MGMT_EV_DEVICE_FOUND && event.payload.len() >= 14 {
                let address = format_bdaddr_le(&event.payload[0..6]);
                let addr_type = event.payload[6];
                let rssi = event.payload[7] as i8 as i16;
                let eir_len = u16::from_le_bytes([event.payload[12], event.payload[13]]) as usize;
                if event.payload.len() >= 14 + eir_len {
                    let eir = event.payload[14..14 + eir_len].to_vec();
                    let parsed = parse_eir_or_ad_data(&eir);
                    let (transport_kind, address_kind) = map_addr_type(addr_type);
                    observations.push(BluetoothBeaconObservation {
                        device_id: address,
                        transport_kind,
                        address_kind,
                        rssi_dbm: rssi,
                        tx_power_dbm: parsed.tx_power_dbm,
                        local_name: parsed.local_name,
                        service_uuids: parsed.service_uuids,
                        profiles: parsed.profiles,
                        classic_device_class: None,
                        advertisement_data: eir,
                        captured_at_unix_ms: system_time_unix_ms()?,
                    });
                }
            }
            if event.opcode == MGMT_EV_DISCOVERING && event.payload.len() >= 2 {
                let discovering = event.payload[1];
                if discovering == 0 {
                    break;
                }
            }
            remaining = timeout_ms as i32 - (system_time_unix_ms()? - start) as i32;
        }
        let _ = socket.send_command(MGMT_OP_STOP_DISCOVERY, self.controller.index, &[mask]);
        let _ =
            wait_for_command_result(&socket, MGMT_OP_STOP_DISCOVERY, self.controller.index, 500);
        Ok(BluetoothScanResult { observations })
    }

    pub fn known_devices(&self) -> Result<Vec<BluezDeviceRecord>, CapabilityError> {
        list_bluez_known_devices(&self.controller.address)
    }

    pub fn discover_cached_or_known(&self) -> Result<BluetoothScanResult, CapabilityError> {
        let known = self.known_devices()?;
        let observations = known
            .into_iter()
            .map(|dev| BluetoothBeaconObservation {
                device_id: dev.device_address,
                transport_kind: dev.transport_kind,
                address_kind: dev.address_kind,
                rssi_dbm: 0,
                tx_power_dbm: None,
                local_name: dev.local_name,
                service_uuids: dev.service_uuids,
                profiles: dev.profiles,
                classic_device_class: None,
                advertisement_data: Vec::new(),
                captured_at_unix_ms: 0,
            })
            .collect();
        Ok(BluetoothScanResult { observations })
    }

    pub fn list_connections(&self) -> Result<Vec<BluetoothConnectionInfo>, CapabilityError> {
        let socket = MgmtSocket::open()?;
        let payload = socket.command(MGMT_OP_GET_CONNECTIONS, self.controller.index, &[])?;
        if payload.len() < 2 {
            return Err(CapabilityError::Provider(
                "short get-connections payload".into(),
            ));
        }
        let count = u16::from_le_bytes([payload[0], payload[1]]) as usize;
        let mut out = Vec::new();
        let mut offset = 2usize;
        for _ in 0..count {
            if offset + 7 > payload.len() {
                break;
            }
            let address = format_bdaddr_le(&payload[offset..offset + 6]);
            let addr_type = payload[offset + 6];
            let (transport_kind, _) = map_addr_type(addr_type);
            let link_kind = match transport_kind {
                BluetoothTransportKind::Classic => BluetoothLinkKind::Acl,
                BluetoothTransportKind::LowEnergy => BluetoothLinkKind::Unknown,
                _ => BluetoothLinkKind::Unknown,
            };
            let known = read_bluez_device_record(&self.controller.address, &address)
                .ok()
                .flatten();
            out.push(BluetoothConnectionInfo {
                device_id: address,
                transport_kind,
                address_kind: match addr_type {
                    1 => BluetoothAddressKind::Public,
                    2 => BluetoothAddressKind::Random,
                    _ => BluetoothAddressKind::Unknown,
                },
                link_kind,
                outbound: true,
                state: 0,
                local_name: known.as_ref().and_then(|d| d.local_name.clone()),
                service_uuids: known
                    .as_ref()
                    .map(|d| d.service_uuids.clone())
                    .unwrap_or_default(),
                profiles: known
                    .as_ref()
                    .map(|d| d.profiles.clone())
                    .unwrap_or_default(),
                trusted: known.as_ref().and_then(|d| d.trusted),
                paired: known.as_ref().and_then(|d| d.paired),
            });
            offset += 7;
        }
        Ok(out)
    }

    /// Connect to a remote Bluetooth device.
    ///
    /// Uses the kernel mgmt `MGMT_OP_CONNECT` command. The `addr_type` parameter
    /// specifies the address type: 0x01=BR/EDR, 0x02=LE Public, 0x04=LE Random.
    /// Use 0x00 to auto-detect.
    pub fn connect_device(&self, bdaddr: &str, addr_type: u8) -> Result<(), CapabilityError> {
        let addr_bytes = parse_bdaddr(bdaddr)?;
        let auto_type = if addr_type != 0 { addr_type } else { MGMT_ADDR_ALL };

        // Payload: 6-byte BD_ADDR + 1-byte address type
        let mut payload = Vec::with_capacity(7);
        payload.extend_from_slice(&addr_bytes);
        payload.push(auto_type);

        let socket = MgmtSocket::open()?;
        let _result = socket.command(MGMT_OP_CONNECT, self.controller.index, &payload)?;
        Ok(())
    }

    /// Disconnect from a remote Bluetooth device.
    pub fn disconnect_device(&self, bdaddr: &str, addr_type: u8) -> Result<(), CapabilityError> {
        let addr_bytes = parse_bdaddr(bdaddr)?;
        let auto_type = if addr_type != 0 { addr_type } else { MGMT_ADDR_ALL };

        let mut payload = Vec::with_capacity(7);
        payload.extend_from_slice(&addr_bytes);
        payload.push(auto_type);

        let socket = MgmtSocket::open()?;
        let _result = socket.command(MGMT_OP_DISCONNECT, self.controller.index, &payload)?;
        Ok(())
    }

    /// Initiate pairing with a remote device.
    /// Returns immediately; pairing completion is reported via `MGMT_EV_DEVICE_CONNECTED`
    /// or `MGMT_EV_CONNECT_FAILED` events.
    pub fn pair_device(&self, bdaddr: &str, addr_type: u8) -> Result<(), CapabilityError> {
        let addr_bytes = parse_bdaddr(bdaddr)?;
        let auto_type = if addr_type != 0 { addr_type } else { MGMT_ADDR_ALL };

        let mut payload = Vec::with_capacity(7);
        payload.extend_from_slice(&addr_bytes);
        payload.push(auto_type);

        let socket = MgmtSocket::open()?;
        let _result = socket.command(MGMT_OP_PAIR_DEVICE, self.controller.index, &payload)?;
        Ok(())
    }

    /// Cancel an ongoing pairing attempt.
    pub fn cancel_pairing(&self, bdaddr: &str, addr_type: u8) -> Result<(), CapabilityError> {
        let addr_bytes = parse_bdaddr(bdaddr)?;
        let auto_type = if addr_type != 0 { addr_type } else { MGMT_ADDR_ALL };

        let mut payload = Vec::with_capacity(7);
        payload.extend_from_slice(&addr_bytes);
        payload.push(auto_type);

        let socket = MgmtSocket::open()?;
        let _result = socket.command(MGMT_OP_CANCEL_PAIRING, self.controller.index, &payload)?;
        Ok(())
    }
}

impl CapabilityProvider for MgmtBluetoothBackend {
    fn descriptor(&self) -> CapabilityDescriptor {
        default_bluetooth_descriptor("linux-bt-mgmt", &format!("hci{}", self.controller.index))
    }
}

impl BluetoothScanner for MgmtBluetoothBackend {
    fn scan_nearby(&self) -> Result<BluetoothScanResult, CapabilityError> {
        self.discover_nearby(MgmtDiscoveryTransport::Interleaved, 1_500)
    }
}

impl BluetoothConnectionProvider for MgmtBluetoothBackend {
    fn list_connections(&self) -> Result<Vec<BluetoothConnectionInfo>, CapabilityError> {
        MgmtBluetoothBackend::list_connections(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_mgmt_packet() {
        let p = build_mgmt_packet(MGMT_OP_SET_POWERED, 2, &[1]);
        assert_eq!(&p[..2], &MGMT_OP_SET_POWERED.to_le_bytes());
        assert_eq!(&p[2..4], &2u16.to_le_bytes());
        assert_eq!(&p[4..6], &1u16.to_le_bytes());
        assert_eq!(p[6], 1);
    }

    #[test]
    fn parses_mgmt_event_header() {
        let ev = parse_mgmt_event(&[1, 0, 2, 0, 3, 0, 9, 8, 7]).unwrap();
        assert_eq!(ev.opcode, 1);
        assert_eq!(ev.index, 2);
        assert_eq!(ev.payload, vec![9, 8, 7]);
    }

    #[test]
    fn parses_settings_bits() {
        let settings = parse_settings((1 << 0) | (1 << 7) | (1 << 9));
        assert!(settings.powered);
        assert!(settings.bredr);
        assert!(settings.low_energy);
        assert!(!settings.discoverable);
    }

    #[test]
    fn parses_controller_info_payload() {
        let mut payload = vec![0u8; 280];
        payload[0..6].copy_from_slice(&[0xC1, 0xED, 0x73, 0xF4, 0x41, 0xA8]);
        payload[6] = 0x0c;
        payload[7..9].copy_from_slice(&0x1234u16.to_le_bytes());
        payload[9..13].copy_from_slice(&0x00000281u32.to_le_bytes());
        payload[13..17].copy_from_slice(&0x00000081u32.to_le_bytes());
        payload[17..20].copy_from_slice(&[0x04, 0x24, 0x08]);
        payload[20..24].copy_from_slice(b"hci0");
        payload[269..274].copy_from_slice(b"bt0\0\0");
        let info = parse_controller_info(0, &payload).unwrap();
        assert_eq!(info.address, "A8:41:F4:73:ED:C1");
        assert!(info.supported_settings.low_energy);
        assert!(info.current_settings.powered);
        assert_eq!(info.name, "hci0");
        assert_eq!(info.short_name, "bt0");
    }

    #[test]
    fn parses_eir_name_and_uuid() {
        let parsed = parse_eir_or_ad_data(&[3, 0x03, 0x0f, 0x18, 5, 0x09, b'T', b'e', b's', b't']);
        assert_eq!(parsed.local_name.as_deref(), Some("Test"));
        assert!(
            parsed
                .service_uuids
                .contains(&"0000180f-0000-1000-8000-00805f9b34fb".to_string())
        );
        assert!(parsed.profiles.contains(&BluetoothProfile::BatteryService));
    }

    #[test]
    fn parse_complete_name_overrides_shortened_name() {
        let parsed = parse_eir_or_ad_data(&[
            4, 0x08, b's', b'h', b'o', 9, 0x09, b'f', b'u', b'l', b'l', b'n', b'a', b'm', b'e',
        ]);
        assert_eq!(parsed.local_name.as_deref(), Some("fullname"));
    }

    #[test]
    fn parses_32bit_service_uuid_from_eir_data() {
        let parsed = parse_eir_or_ad_data(&[
            5, 0x05, 0x6f, 0x00, 0x00, 0x00, 3, 0x09, b'T', b'e', b's', b't',
        ]);
        assert!(
            parsed
                .service_uuids
                .contains(&"0000006f-0000-1000-8000-00805f9b34fb".to_string())
        );
    }

    #[test]
    fn parses_service_uuid_list_with_commas() {
        let uuids = parse_service_uuids(
            "0000180f-0000-1000-8000-00805f9b34fb,0000180d-0000-1000-8000-00805f9b34fb",
        );
        assert_eq!(uuids.len(), 2);
        assert_eq!(uuids[0], "0000180f-0000-1000-8000-00805f9b34fb");
        assert_eq!(uuids[1], "0000180d-0000-1000-8000-00805f9b34fb");
    }

    #[test]
    fn normalizes_service_uuids_in_info_records() {
        let text = "[General]\nName=Headset\nAddressType=public\nSupportedTechnologies=BR/EDR;LE;\nServices={0000180f-0000-1000-8000-00805f9b34fb}\n\n";
        let record = parse_bluez_device_record("AA:BB:CC:DD:EE:FF", "11:22:33:44:55:66", text);
        assert_eq!(
            record.service_uuids,
            vec!["0000180f-0000-1000-8000-00805f9b34fb".to_string()]
        );
        assert!(record.profiles.contains(&BluetoothProfile::BatteryService));
    }

    #[test]
    fn parses_service_data_16bit_profile() {
        let parsed = parse_eir_or_ad_data(&[4, 0x16, 0x0f, 0x18, 0x01, 0x02]);
        assert!(
            parsed
                .service_uuids
                .contains(&"0000180f-0000-1000-8000-00805f9b34fb".to_string())
        );
        assert!(parsed.profiles.contains(&BluetoothProfile::BatteryService));
    }

    #[test]
    fn parses_16bit_service_uuid_solicitation() {
        let parsed = parse_eir_or_ad_data(&[3, 0x14, 0x0f, 0x18]);
        assert!(
            parsed
                .service_uuids
                .contains(&"0000180f-0000-1000-8000-00805f9b34fb".to_string())
        );
        assert!(parsed.profiles.contains(&BluetoothProfile::BatteryService));
    }

    #[test]
    fn parses_128bit_advertising_uuid_list() {
        let parsed = parse_eir_or_ad_data(&[
            17, 0x21, 0xff, 0xee, 0xdd, 0xcc, 0xbb, 0xaa, 0x99, 0x88, 0x77, 0x66, 0x55, 0x44, 0x33,
            0x22, 0x11, 0x00,
        ]);
        assert!(
            parsed
                .service_uuids
                .contains(&"00112233-4455-6677-8899-aabbccddeeff".to_string())
        );
    }

    #[test]
    fn parses_service_uuids_from_info_record_variants() {
        let text = "[General]\nServices=180f;{180D};0000180f\n";
        let record = parse_bluez_device_record("AA:BB:CC:DD:EE:FF", "11:22:33:44:55:66", text);
        assert!(
            record
                .service_uuids
                .contains(&"0000180f-0000-1000-8000-00805f9b34fb".to_string())
        );
        assert!(record.profiles.contains(&BluetoothProfile::HeartRate));
        assert!(record.profiles.contains(&BluetoothProfile::BatteryService));
    }

    #[test]
    fn does_not_infer_profile_from_non_base_32bit_uuid() {
        let parsed = parse_uuid16_from_service_uuid("12345678");
        assert!(parsed.is_none());
    }

    #[test]
    fn infers_profiles_from_110b_service_uuid() {
        let parsed = parse_uuid16_from_service_uuid("110b").expect("parseable");
        assert_eq!(
            infer_profiles_from_uuid16(parsed),
            vec![BluetoothProfile::AudioSink]
        );
    }

    #[test]
    fn normalizes_0x_prefixed_service_uuids() {
        let text = "[General]\nServices=0x180f;0X180A\n";
        let record = parse_bluez_device_record("AA:BB:CC:DD:EE:FF", "11:22:33:44:55:66", text);
        assert!(
            record
                .service_uuids
                .contains(&"0000180f-0000-1000-8000-00805f9b34fb".to_string())
        );
        assert!(
            record
                .service_uuids
                .contains(&"0000180a-0000-1000-8000-00805f9b34fb".to_string())
        );
        assert!(record.profiles.contains(&BluetoothProfile::BatteryService));
    }

    #[test]
    fn parses_ini_bool_values() {
        assert_eq!(bool_from_ini("true"), Some(true));
        assert_eq!(bool_from_ini("0"), Some(false));
        assert_eq!(bool_from_ini("yes"), Some(true));
        assert_eq!(bool_from_ini("OFF"), Some(false));
        assert_eq!(bool_from_ini("n/a"), None);
    }

    #[test]
    fn maps_discovery_transport_masks() {
        assert_eq!(
            MgmtDiscoveryTransport::Classic.address_mask(),
            MGMT_ADDR_BREDR
        );
        assert_eq!(
            MgmtDiscoveryTransport::LowEnergy.address_mask(),
            MGMT_ADDR_LE_ANY
        );
        assert_eq!(
            MgmtDiscoveryTransport::Interleaved.address_mask(),
            MGMT_ADDR_ALL
        );
    }

    #[test]
    fn parses_bluez_info_record() {
        let text = "[General]\nName=Headset\nAddressType=public\nSupportedTechnologies=BR/EDR;LE;\nTrusted=true\nServices=00001108-0000-1000-8000-00805f9b34fb;0000180f-0000-1000-8000-00805f9b34fb;\n\n[LinkKey]\nKey=00\n";
        let record = parse_bluez_device_record("AA:BB:CC:DD:EE:FF", "11:22:33:44:55:66", text);
        assert_eq!(record.local_name.as_deref(), Some("Headset"));
        assert_eq!(record.transport_kind, BluetoothTransportKind::DualMode);
        assert!(record.profiles.contains(&BluetoothProfile::Headset));
        assert!(record.profiles.contains(&BluetoothProfile::BatteryService));
        assert_eq!(record.trusted, Some(true));
        assert_eq!(record.paired, Some(true));
    }

    #[test]
    fn parses_new_settings_event() {
        let event = MgmtEvent {
            opcode: MGMT_EV_NEW_SETTINGS,
            index: 2,
            payload: (1u32 | (1u32 << 9)).to_le_bytes().to_vec(),
        };
        let parsed = parse_mgmt_controller_event(&event).unwrap();
        match parsed {
            MgmtControllerEvent::SettingsChanged {
                index, settings, ..
            } => {
                assert_eq!(index, 2);
                assert!(settings.powered);
                assert!(settings.low_energy);
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[test]
    fn parses_device_connected_event() {
        let mut payload = vec![0xC6, 0xDA, 0x6F, 0x2C, 0x3A, 0xD4, 0x00];
        payload.extend_from_slice(&1u32.to_le_bytes());
        let eir = vec![5, 0x09, b'T', b'e', b's', b't'];
        payload.extend_from_slice(&(eir.len() as u16).to_le_bytes());
        payload.extend_from_slice(&eir);
        let event = MgmtEvent {
            opcode: MGMT_EV_DEVICE_CONNECTED,
            index: 0,
            payload,
        };
        let parsed = parse_mgmt_controller_event(&event).unwrap();
        match parsed {
            MgmtControllerEvent::DeviceConnected {
                device_id,
                local_name,
                transport_kind,
                ..
            } => {
                assert_eq!(device_id, "D4:3A:2C:6F:DA:C6");
                assert_eq!(local_name.as_deref(), Some("Test"));
                assert_eq!(transport_kind, BluetoothTransportKind::Classic);
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[test]
    fn status_name_decodes_busy() {
        assert_eq!(mgmt_status_name(0x0a), "busy");
        assert_eq!(mgmt_status_name(0x0f), "not-powered");
    }

    #[test]
    fn parses_class_and_name_events() {
        let class_event = MgmtEvent {
            opcode: MGMT_EV_CLASS_OF_DEVICE_CHANGED,
            index: 0,
            payload: vec![0x04, 0x01, 0x7c],
        };
        match parse_mgmt_controller_event(&class_event).unwrap() {
            MgmtControllerEvent::ClassOfDeviceChanged {
                class_of_device, ..
            } => assert_eq!(class_of_device, 0x7c0104),
            other => panic!("unexpected event: {other:?}"),
        }

        let mut payload = vec![0u8; 260];
        payload[..4].copy_from_slice(b"test");
        payload[249..252].copy_from_slice(b"bt0");
        let name_event = MgmtEvent {
            opcode: MGMT_EV_LOCAL_NAME_CHANGED,
            index: 0,
            payload,
        };
        match parse_mgmt_controller_event(&name_event).unwrap() {
            MgmtControllerEvent::LocalNameChanged {
                name, short_name, ..
            } => {
                assert_eq!(name, "test");
                assert_eq!(short_name, "bt0");
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    // --- MgmtDiscoveryTransport ---

    #[test]
    fn discovery_transport_address_masks() {
        assert_eq!(MgmtDiscoveryTransport::Classic.address_mask(), MGMT_ADDR_BREDR);
        assert_eq!(MgmtDiscoveryTransport::LowEnergy.address_mask(), MGMT_ADDR_LE_ANY);
        assert_eq!(MgmtDiscoveryTransport::Interleaved.address_mask(), MGMT_ADDR_ALL);
    }

    #[test]
    fn discovery_transport_clone() {
        let t = MgmtDiscoveryTransport::LowEnergy;
        assert_eq!(t.clone(), t);
    }

    // --- MgmtVersionInfo ---

    #[test]
    fn version_info_fields() {
        let v = MgmtVersionInfo { version: 1, revision: 42 };
        assert_eq!(v.version, 1);
        assert_eq!(v.revision, 42);
    }

    #[test]
    fn version_info_clone_and_eq() {
        let v = MgmtVersionInfo { version: 2, revision: 100 };
        assert_eq!(v.clone(), v);
    }

    // --- MgmtControllerSettings ---

    #[test]
    fn settings_all_false() {
        let s = MgmtControllerSettings {
            powered: false, connectable: false, fast_connectable: false,
            discoverable: false, pairable: false, link_security: false,
            secure_simple_pairing: false, bredr: false, high_speed: false,
            low_energy: false,
        };
        assert!(!s.powered);
        assert!(!s.low_energy);
    }

    #[test]
    fn settings_all_true() {
        let s = MgmtControllerSettings {
            powered: true, connectable: true, fast_connectable: true,
            discoverable: true, pairable: true, link_security: true,
            secure_simple_pairing: true, bredr: true, high_speed: true,
            low_energy: true,
        };
        assert!(s.powered);
        assert!(s.connectable);
        assert!(s.low_energy);
    }

    #[test]
    fn settings_clone_and_eq() {
        let s = MgmtControllerSettings {
            powered: true, connectable: false, fast_connectable: false,
            discoverable: false, pairable: false, link_security: false,
            secure_simple_pairing: false, bredr: false, high_speed: false,
            low_energy: false,
        };
        assert_eq!(s.clone(), s);
    }

    // --- MgmtControllerInfo ---

    #[test]
    fn controller_info_construction() {
        let settings = MgmtControllerSettings {
            powered: true, connectable: true, fast_connectable: false,
            discoverable: false, pairable: false, link_security: true,
            secure_simple_pairing: true, bredr: true, high_speed: false,
            low_energy: true,
        };
        let info = MgmtControllerInfo {
            index: 0,
            address: "AA:BB:CC:DD:EE:FF".to_string(),
            bluetooth_version: 5,
            manufacturer: 0x003f,
            supported_settings_raw: 0x3ff,
            current_settings_raw: 0x207,
            supported_settings: settings.clone(),
            current_settings: settings,
            class_of_device: 0,
            name: "my-controller".to_string(),
            short_name: "my-ctrl".to_string(),
        };
        assert_eq!(info.index, 0);
        assert_eq!(info.bluetooth_version, 5);
        assert!(info.current_settings.powered);
    }

    #[test]
    fn controller_info_clone() {
        let s = MgmtControllerSettings {
            powered: false, connectable: false, fast_connectable: false,
            discoverable: false, pairable: false, link_security: false,
            secure_simple_pairing: false, bredr: false, high_speed: false,
            low_energy: false,
        };
        let info = MgmtControllerInfo {
            index: 1, address: "00:00:00:00:00:01".to_string(),
            bluetooth_version: 4, manufacturer: 0,
            supported_settings_raw: 0, current_settings_raw: 0,
            supported_settings: s.clone(), current_settings: s,
            class_of_device: 0, name: "".to_string(), short_name: "".to_string(),
        };
        assert_eq!(info.clone(), info);
    }

    // --- MgmtBluetoothBackend ---

    #[test]
    fn mgmt_bluetooth_backend_construction() {
        let s = MgmtControllerSettings {
            powered: false, connectable: false, fast_connectable: false,
            discoverable: false, pairable: false, link_security: false,
            secure_simple_pairing: false, bredr: false, high_speed: false,
            low_energy: false,
        };
        let controller = MgmtControllerInfo {
            index: 0, address: "00:00:00:00:00:00".to_string(),
            bluetooth_version: 5, manufacturer: 0,
            supported_settings_raw: 0, current_settings_raw: 0,
            supported_settings: s.clone(), current_settings: s,
            class_of_device: 0, name: "".to_string(), short_name: "".to_string(),
        };
        let backend = MgmtBluetoothBackend { controller };
        assert_eq!(backend.controller.index, 0);
    }

    // --- BluezDeviceRecord ---

    #[test]
    fn bluez_device_record_empty_services() {
        let record = BluezDeviceRecord {
            controller_address: "AA:BB".to_string(),
            device_address: "CC:DD".to_string(),
            address_kind: BluetoothAddressKind::Unknown,
            local_name: None,
            transport_kind: BluetoothTransportKind::Unknown,
            service_uuids: vec![],
            profiles: vec![],
            trusted: None,
            paired: Some(false),
        };
        assert!(record.service_uuids.is_empty());
        assert!(record.profiles.is_empty());
        assert_eq!(record.paired, Some(false));
    }

    #[test]
    fn bluez_device_record_clone() {
        let record = BluezDeviceRecord {
            controller_address: "A".to_string(),
            device_address: "B".to_string(),
            address_kind: BluetoothAddressKind::Public,
            local_name: Some("Device".to_string()),
            transport_kind: BluetoothTransportKind::LowEnergy,
            service_uuids: vec!["uuid1".to_string()],
            profiles: vec![BluetoothProfile::HeartRate],
            trusted: Some(true),
            paired: Some(true),
        };
        assert_eq!(record.clone(), record);
    }

    // --- MgmtControllerEvent variants ---

    #[test]
    fn event_device_disconnected() {
        let event = MgmtControllerEvent::DeviceDisconnected {
            index: 0,
            device_id: "AA:BB:CC".to_string(),
            transport_kind: BluetoothTransportKind::LowEnergy,
            address_kind: BluetoothAddressKind::Random,
            reason: 0x16,
        };
        match event {
            MgmtControllerEvent::DeviceDisconnected { reason, .. } => {
                assert_eq!(reason, 0x16);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn event_connect_failed() {
        let event = MgmtControllerEvent::ConnectFailed {
            index: 1,
            device_id: "DD:EE:FF".to_string(),
            transport_kind: BluetoothTransportKind::Classic,
            address_kind: BluetoothAddressKind::Public,
            status: 0x03,
        };
        match event {
            MgmtControllerEvent::ConnectFailed { status, .. } => {
                assert_eq!(status, 0x03);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn event_discovering_changed() {
        let event = MgmtControllerEvent::DiscoveringChanged {
            index: 0,
            address_mask: MGMT_ADDR_LE_ANY,
            discovering: true,
        };
        match event {
            MgmtControllerEvent::DiscoveringChanged { discovering, address_mask, .. } => {
                assert!(discovering);
                assert_eq!(address_mask, MGMT_ADDR_LE_ANY);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn event_clone_and_debug() {
        let event = MgmtControllerEvent::SettingsChanged {
            index: 0,
            settings_raw: 0,
            settings: MgmtControllerSettings {
                powered: false, connectable: false, fast_connectable: false,
                discoverable: false, pairable: false, link_security: false,
                secure_simple_pairing: false, bredr: false, high_speed: false,
                low_energy: false,
            },
        };
        let cloned = event.clone();
        assert_eq!(cloned, event);
        let debug = format!("{:?}", event);
        assert!(debug.contains("SettingsChanged"));
    }

    // --- mgmt_status_name ---

    #[test]
    fn status_name_all_known_values() {
        assert_eq!(mgmt_status_name(0x00), "success");
        assert_eq!(mgmt_status_name(0x01), "unknown-command");
        assert_eq!(mgmt_status_name(0x02), "not-connected");
        assert_eq!(mgmt_status_name(0x03), "failed");
        assert_eq!(mgmt_status_name(0x04), "connect-failed");
        assert_eq!(mgmt_status_name(0x05), "authentication-failed");
        assert_eq!(mgmt_status_name(0x06), "not-paired");
        assert_eq!(mgmt_status_name(0x07), "no-resources");
        assert_eq!(mgmt_status_name(0x08), "timeout");
        assert_eq!(mgmt_status_name(0x09), "already-connected");
        assert_eq!(mgmt_status_name(0x0a), "busy");
        assert_eq!(mgmt_status_name(0x0b), "rejected");
        assert_eq!(mgmt_status_name(0x0c), "not-supported");
        assert_eq!(mgmt_status_name(0x0d), "invalid-parameters");
        assert_eq!(mgmt_status_name(0x0e), "disconnected");
        assert_eq!(mgmt_status_name(0x0f), "not-powered");
        assert_eq!(mgmt_status_name(0x10), "cancelled");
        assert_eq!(mgmt_status_name(0x11), "invalid-index");
        assert_eq!(mgmt_status_name(0x12), "rfkilled");
        assert_eq!(mgmt_status_name(0xff), "unknown-status");
    }

    // --- format_bdaddr_le ---

    #[test]
    fn format_bdaddr_le_reverses_bytes() {
        let bytes = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06];
        assert_eq!(format_bdaddr_le(&bytes), "06:05:04:03:02:01");
    }

    #[test]
    fn format_bdaddr_le_zero_address() {
        let bytes = [0u8; 6];
        assert_eq!(format_bdaddr_le(&bytes), "00:00:00:00:00:00");
    }

    // --- parse_settings ---

    #[test]
    fn parse_settings_zero() {
        let s = parse_settings(0);
        assert!(!s.powered);
        assert!(!s.connectable);
        assert!(!s.discoverable);
        assert!(!s.pairable);
        assert!(!s.link_security);
        assert!(!s.secure_simple_pairing);
        assert!(!s.bredr);
        assert!(!s.high_speed);
        assert!(!s.low_energy);
    }

    #[test]
    fn parse_settings_all_bits_set() {
        let s = parse_settings(u32::MAX);
        assert!(s.powered);
        assert!(s.connectable);
        assert!(s.fast_connectable);
        assert!(s.discoverable);
        assert!(s.pairable);
        assert!(s.link_security);
        assert!(s.secure_simple_pairing);
        assert!(s.bredr);
        assert!(s.high_speed);
        assert!(s.low_energy);
    }

    #[test]
    fn parse_settings_individual_bits() {
        assert!(parse_settings(1 << 0).powered);
        assert!(parse_settings(1 << 1).connectable);
        assert!(parse_settings(1 << 2).fast_connectable);
        assert!(parse_settings(1 << 3).discoverable);
        assert!(parse_settings(1 << 4).pairable);
        assert!(parse_settings(1 << 5).link_security);
        assert!(parse_settings(1 << 6).secure_simple_pairing);
        assert!(parse_settings(1 << 7).bredr);
        assert!(parse_settings(1 << 8).high_speed);
        assert!(parse_settings(1 << 9).low_energy);
    }

    // --- parse_controller_info ---

    #[test]
    fn parse_controller_info_too_short() {
        let result = parse_controller_info(0, &[0u8; 10]);
        assert!(result.is_err());
    }

    #[test]
    fn parse_controller_info_valid_payload() {
        let mut payload = vec![0u8; 280];
        // address (6 bytes)
        payload[0..6].copy_from_slice(&[0x01, 0x02, 0x03, 0x04, 0x05, 0x06]);
        payload[6] = 5; // bluetooth_version
        payload[7] = 0x3f; payload[8] = 0x00; // manufacturer
        payload[9..13].copy_from_slice(&0x3ffu32.to_le_bytes()); // supported_settings
        // current_settings: bits 0,1,9 = powered, connectable, low_energy (0x203)
        payload[13..17].copy_from_slice(&0x203u32.to_le_bytes());
        // class_of_device is 3 bytes at offset 17-19
        payload[17] = 0x04; payload[18] = 0x01; payload[19] = 0x7c;
        payload[20..24].copy_from_slice(b"name");
        payload[269..272].copy_from_slice(b"sn\0");
        let info = parse_controller_info(0, &payload).unwrap();
        assert_eq!(info.address, "06:05:04:03:02:01");
        assert_eq!(info.bluetooth_version, 5);
        assert_eq!(info.manufacturer, 0x003f);
        assert!(info.current_settings.powered);
        assert!(info.current_settings.connectable);
        assert!(info.current_settings.low_energy);
        assert_eq!(info.class_of_device, 0x7c0104);
        assert_eq!(info.name, "name");
        assert_eq!(info.short_name, "sn");
    }

    // --- bool_from_ini ---

    #[test]
    fn bool_from_ini_true_values() {
        assert_eq!(bool_from_ini("1"), Some(true));
        assert_eq!(bool_from_ini("true"), Some(true));
        assert_eq!(bool_from_ini("yes"), Some(true));
        assert_eq!(bool_from_ini("on"), Some(true));
        assert_eq!(bool_from_ini(" TRUE "), Some(true));
    }

    #[test]
    fn bool_from_ini_false_values() {
        assert_eq!(bool_from_ini("0"), Some(false));
        assert_eq!(bool_from_ini("false"), Some(false));
        assert_eq!(bool_from_ini("no"), Some(false));
        assert_eq!(bool_from_ini("off"), Some(false));
        assert_eq!(bool_from_ini(" FALSE "), Some(false));
    }

    #[test]
    fn bool_from_ini_invalid_values() {
        assert_eq!(bool_from_ini("maybe"), None);
        assert_eq!(bool_from_ini(""), None);
        assert_eq!(bool_from_ini("2"), None);
    }

    // --- address_kind_from_str ---

    #[test]
    fn address_kind_from_public() {
        assert_eq!(address_kind_from_str("public"), BluetoothAddressKind::Public);
    }

    #[test]
    fn address_kind_from_random() {
        assert_eq!(address_kind_from_str("random"), BluetoothAddressKind::Random);
    }

    #[test]
    fn address_kind_from_unknown() {
        assert_eq!(address_kind_from_str("foo"), BluetoothAddressKind::Unknown);
        assert_eq!(address_kind_from_str(""), BluetoothAddressKind::Unknown);
    }

    // --- transport_from_technologies ---

    #[test]
    fn transport_from_technologies_classic() {
        assert_eq!(
            transport_from_technologies("BR/EDR"),
            BluetoothTransportKind::Classic
        );
    }

    #[test]
    fn transport_from_technologies_le() {
        assert_eq!(
            transport_from_technologies("LE"),
            BluetoothTransportKind::LowEnergy
        );
    }

    #[test]
    fn transport_from_technologies_dual() {
        assert_eq!(
            transport_from_technologies("BR/EDR;LE;"),
            BluetoothTransportKind::DualMode
        );
    }

    #[test]
    fn transport_from_technologies_unknown() {
        assert_eq!(
            transport_from_technologies("UNKNOWN"),
            BluetoothTransportKind::Unknown
        );
    }

    #[test]
    fn transport_from_technologies_case_insensitive() {
        assert_eq!(
            transport_from_technologies("br/edr;le"),
            BluetoothTransportKind::DualMode
        );
    }

    // --- parse_service_uuids ---

    #[test]
    fn parse_service_uuids_empty() {
        let result = parse_service_uuids("");
        assert!(result.is_empty());
    }

    #[test]
    fn parse_service_uuids_semicolon_separated() {
        let result = parse_service_uuids("0000180f-0000-1000-8000-00805f9b34fb;0000180d-0000-1000-8000-00805f9b34fb;");
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn parse_service_uuids_comma_separated() {
        let result = parse_service_uuids("180f,180d");
        assert_eq!(result.len(), 2);
    }

    // --- is_hex ---

    #[test]
    fn is_hex_valid() {
        assert!(is_hex("0123456789abcdef"));
        assert!(is_hex("AABBCCDD"));
        assert!(is_hex(""));
    }

    #[test]
    fn is_hex_invalid() {
        assert!(!is_hex("hello"));
        assert!(!is_hex("12g4"));
    }

    // --- normalize_service_uuid ---

    #[test]
    fn normalize_16bit_uuid() {
        let result = normalize_service_uuid("180f").unwrap();
        assert_eq!(result, "0000180f-0000-1000-8000-00805f9b34fb");
    }

    #[test]
    fn normalize_16bit_with_0x_prefix() {
        let result = normalize_service_uuid("0x180d").unwrap();
        assert_eq!(result, "0000180d-0000-1000-8000-00805f9b34fb");
    }

    #[test]
    fn normalize_32bit_uuid() {
        let result = normalize_service_uuid("0000180f").unwrap();
        assert_eq!(result, "0000180f-0000-1000-8000-00805f9b34fb");
    }

    #[test]
    fn normalize_128bit_uuid_with_braces() {
        let input = "{0000180f-0000-1000-8000-00805f9b34fb}";
        let result = normalize_service_uuid(input).unwrap();
        assert_eq!(result, "0000180f-0000-1000-8000-00805f9b34fb");
    }

    #[test]
    fn normalize_invalid_uuid_returns_none() {
        assert!(normalize_service_uuid("not-hex").is_none());
        assert!(normalize_service_uuid("xyz").is_none());
    }

    #[test]
    fn normalize_wrong_length_uuid_returns_none() {
        assert!(normalize_service_uuid("123").is_none());
        assert!(normalize_service_uuid("1234567").is_none());
    }

    // --- parse_uuid16_from_service_uuid ---

    #[test]
    fn parse_uuid16_direct_4chars() {
        assert_eq!(parse_uuid16_from_service_uuid("180f"), Some(0x180f));
    }

    #[test]
    fn parse_uuid16_from_full_128bit() {
        assert_eq!(
            parse_uuid16_from_service_uuid("0000180f-0000-1000-8000-00805f9b34fb"),
            Some(0x180f)
        );
    }

    #[test]
    fn parse_uuid16_from_8char() {
        assert_eq!(parse_uuid16_from_service_uuid("0000180d"), Some(0x180d));
    }

    #[test]
    fn parse_uuid16_invalid_returns_none() {
        assert!(parse_uuid16_from_service_uuid("xyzw").is_none());
    }

    // --- profiles_from_services ---

    #[test]
    fn profiles_from_known_uuids() {
        let uuids = vec![
            "00001108-0000-1000-8000-00805f9b34fb".to_string(), // Headset
            "0000180d-0000-1000-8000-00805f9b34fb".to_string(), // HeartRate
        ];
        let profiles = profiles_from_services(&uuids);
        assert!(profiles.contains(&BluetoothProfile::Headset));
        assert!(profiles.contains(&BluetoothProfile::HeartRate));
    }

    #[test]
    fn profiles_from_unknown_uuid() {
        let uuids = vec!["00009999-0000-1000-8000-00805f9b34fb".to_string()];
        let profiles = profiles_from_services(&uuids);
        assert!(profiles.is_empty());
    }

    #[test]
    fn profiles_deduplication() {
        // Two uuids that map to AudioSink
        let uuids = vec![
            "0000110b-0000-1000-8000-00805f9b34fb".to_string(),
            "0000110c-0000-1000-8000-00805f9b34fb".to_string(),
        ];
        let profiles = profiles_from_services(&uuids);
        let count = profiles.iter().filter(|&&p| p == BluetoothProfile::AudioSink).count();
        assert_eq!(count, 1);
    }

    // --- parse_ini_map ---

    #[test]
    fn parse_ini_map_basic() {
        let text = "[Section1]\nkey1=value1\n\n[Section2]\nkey2=value2\n";
        let map = parse_ini_map(text);
        assert_eq!(map.get("Section1").and_then(|m| m.get("key1")), Some(&"value1".to_string()));
        assert_eq!(map.get("Section2").and_then(|m| m.get("key2")), Some(&"value2".to_string()));
    }

    #[test]
    fn parse_ini_map_skips_comments() {
        let text = "# comment\n; comment\n[section]\nkey=val\n";
        let map = parse_ini_map(text);
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn parse_ini_map_empty() {
        let map = parse_ini_map("");
        assert!(map.is_empty());
    }

    // --- parse_bluez_device_record ---

    #[test]
    fn parses_bluez_record_with_linkkey() {
        let text = "[General]\nName=Test\nAddressType=random\nSupportedTechnologies=LE;\nServices=0000180f-0000-1000-8000-00805f9b34fb;\n\n[LongTermKey]\nKey=00\n";
        let record = parse_bluez_device_record("AA:BB", "CC:DD", text);
        assert_eq!(record.device_address, "CC:DD");
        assert_eq!(record.address_kind, BluetoothAddressKind::Random);
        assert_eq!(record.transport_kind, BluetoothTransportKind::LowEnergy);
        assert_eq!(record.local_name.as_deref(), Some("Test"));
        assert!(record.profiles.contains(&BluetoothProfile::BatteryService));
        assert_eq!(record.paired, Some(true));
    }

    #[test]
    fn parses_bluez_record_without_pairing_keys() {
        let text = "[General]\nAddressType=public\nSupportedTechnologies=BR/EDR;\n";
        let record = parse_bluez_device_record("AA", "BB", text);
        assert_eq!(record.paired, Some(false));
    }

    // --- bluez_controller_dir ---

    #[test]
    fn bluez_controller_dir_path() {
        let path = bluez_controller_dir("AA:BB:CC:DD:EE:FF");
        assert_eq!(path, Path::new("/var/lib/bluetooth").join("AA:BB:CC:DD:EE:FF"));
    }

    // --- build_mgmt_packet ---

    #[test]
    fn build_mgmt_packet_empty_payload() {
        let packet = build_mgmt_packet(0x0001, 0x0000, &[]);
        assert_eq!(packet, vec![0x01, 0x00, 0x00, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn build_mgmt_packet_with_payload() {
        let packet = build_mgmt_packet(0x0023, 0x0000, &[0x01]);
        assert_eq!(packet, vec![0x23, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01]);
    }

    // --- parse_mgmt_event ---

    #[test]
    fn parse_mgmt_event_too_short() {
        let result = parse_mgmt_event(&[0x00, 0x01]);
        assert!(result.is_err());
    }

    #[test]
    fn parse_mgmt_event_truncated_payload() {
        // header says payload is 10 bytes, but only 2 bytes follow
        let packet = [0x01, 0x00, 0x00, 0x00, 0x0a, 0x00, 0x01, 0x02];
        let result = parse_mgmt_event(&packet);
        assert!(result.is_err());
    }

    #[test]
    fn parse_mgmt_event_valid() {
        let packet = [0x01, 0x00, 0x02, 0x00, 0x02, 0x00, 0xaa, 0xbb];
        let event = parse_mgmt_event(&packet).unwrap();
        assert_eq!(event.opcode, 0x0001);
        assert_eq!(event.index, 0x0002);
        assert_eq!(event.payload, vec![0xaa, 0xbb]);
    }

    // --- format_16bit_service_uuid ---

    #[test]
    fn format_16bit_uuid() {
        assert_eq!(
            format_16bit_service_uuid(0x180f),
            "0000180f-0000-1000-8000-00805f9b34fb"
        );
    }

    // --- format_32bit_service_uuid ---

    #[test]
    fn format_32bit_uuid() {
        assert_eq!(
            format_32bit_service_uuid(0x0000180f),
            "0000180f-0000-1000-8000-00805f9b34fb"
        );
    }

    // --- parse_eir_or_ad_data ---

    #[test]
    fn parse_eir_short_name() {
        let data = [0x05, 0x08, b'T', b'e', b's', b't']; // short name
        let parsed = parse_eir_or_ad_data(&data);
        assert_eq!(parsed.local_name.as_deref(), Some("Test"));
    }

    #[test]
    fn parse_eir_complete_name() {
        let data = [0x05, 0x09, b'T', b'e', b's', b't']; // complete name
        let parsed = parse_eir_or_ad_data(&data);
        assert_eq!(parsed.local_name.as_deref(), Some("Test"));
    }

    #[test]
    fn parse_eir_complete_name_overrides_short() {
        let data = [
            0x03, 0x08, b'A', b'B',  // short name
            0x05, 0x09, b'T', b'e', b's', b't', // complete name
        ];
        let parsed = parse_eir_or_ad_data(&data);
        assert_eq!(parsed.local_name.as_deref(), Some("Test"));
    }

    #[test]
    fn parse_eir_16bit_uuids() {
        let data = [0x05, 0x02, 0x0f, 0x18, 0x0d, 0x18]; // incomplete list
        let parsed = parse_eir_or_ad_data(&data);
        assert_eq!(parsed.service_uuids.len(), 2);
        assert!(parsed.profiles.contains(&BluetoothProfile::BatteryService));
        assert!(parsed.profiles.contains(&BluetoothProfile::HeartRate));
    }

    #[test]
    fn parse_eir_tx_power() {
        let data = [0x02, 0x0a, 0x04]; // tx power level = 4
        let parsed = parse_eir_or_ad_data(&data);
        assert_eq!(parsed.tx_power_dbm, Some(4));
    }

    #[test]
    fn parse_eir_empty_data() {
        let parsed = parse_eir_or_ad_data(&[]);
        assert!(parsed.local_name.is_none());
        assert!(parsed.service_uuids.is_empty());
        assert!(parsed.tx_power_dbm.is_none());
    }

    // --- infer_profiles_from_uuid16 ---

    #[test]
    fn infer_profiles_headset() {
        assert_eq!(infer_profiles_from_uuid16(0x1108), vec![BluetoothProfile::Headset]);
    }

    #[test]
    fn infer_profiles_handsfree() {
        let p = infer_profiles_from_uuid16(0x111e);
        assert!(p.contains(&BluetoothProfile::HandsFree));
    }

    #[test]
    fn infer_profiles_audio_source() {
        assert_eq!(infer_profiles_from_uuid16(0x110a), vec![BluetoothProfile::AudioSource]);
    }

    #[test]
    fn infer_profiles_hid() {
        assert_eq!(infer_profiles_from_uuid16(0x1124), vec![BluetoothProfile::Hid]);
    }

    #[test]
    fn infer_profiles_unknown() {
        assert!(infer_profiles_from_uuid16(0x9999).is_empty());
    }

    // --- parse_bluez_device_record edge cases ---

    #[test]
    fn parses_bluez_record_cache_file() {
        // Cache files have the same ini format but no pairing keys
        let text = "[General]\nName=Cached\nAddressType=random\nSupportedTechnologies=LE;\n";
        let record = parse_bluez_device_record("AA", "BB", text);
        assert_eq!(record.local_name.as_deref(), Some("Cached"));
        assert_eq!(record.paired, Some(false));
    }

    // --- parses_device_disconnected_event ---

    #[test]
    fn parses_device_disconnected_event() {
        let payload = vec![0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x02, 0x16]; // addr + LE random + reason
        let event = MgmtEvent {
            opcode: MGMT_EV_DEVICE_DISCONNECTED,
            index: 0,
            payload,
        };
        let parsed = parse_mgmt_controller_event(&event).unwrap();
        match parsed {
            MgmtControllerEvent::DeviceDisconnected {
                device_id, reason, ..
            } => {
                assert_eq!(device_id, "FF:EE:DD:CC:BB:AA");
                assert_eq!(reason, 0x16);
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }
}
