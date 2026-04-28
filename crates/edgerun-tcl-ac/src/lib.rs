#![no_std]

extern crate alloc;
#[cfg(not(target_os = "none"))]
extern crate std;

use aes::cipher::{generic_array::GenericArray, BlockDecrypt, BlockEncrypt, KeyInit};
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use core::option::Option::{None, Some};
use core::result::Result::{Err, Ok};
#[cfg(target_os = "none")]
use edgerun_bluetooth_gatt::sync::RwLock;
use edgerun_bluetooth_gatt::{format_gatt_uuid, AttProtocol, GattError, L2capSocket};
use edgerun_capabilities::{CapabilityError, CapabilityProvider};
use edgerun_crypto::{hmac_sha256, sha256, OsRng, RngCore};
#[cfg(not(target_os = "none"))]
use std::sync::RwLock;

pub const TCL_SERVICE_UUID: &str = "0000f900-0000-1000-8000-00805f9b34fb";
pub const TCL_WRITE_CHAR_UUID: &str = "0000f901-0000-1000-8000-00805f9b34fb";
pub const TCL_INDICATE_CHAR_UUID: &str = "0000f902-0000-1000-8000-00805f9b34fb";
pub const TCL_LEGACY_SERVICE_UUID: &str = "0000f100-0000-1000-8000-00805f9b34fb";
pub const TCL_LEGACY_WRITE_CHAR_UUID: &str = "0000f101-0000-1000-8000-00805f9b34fb";
pub const TCL_LEGACY_INDICATE_CHAR_UUID: &str = "0000f102-0000-1000-8000-00805f9b34fb";

const BASE_KEY: &[u8; 16] = b"p7#z9@L2!c5%v1&k";
const PRESET_IV: &[u8; 16] = b"GjVEI7lQ382O7Ua0";
const PROTOCOL_HEAD: u8 = 0xBB;
const PROTOCOL_HEADER_LEN: usize = 37;

const CMD_SEND_APP_RANDOM: u8 = 16;
const CMD_SEND_DEVICE_RANDOM: u8 = 17;
const CMD_SEND_WIFI_INFO: u8 = 18;
const CMD_STATUS_REPORT_RESPONSE: u8 = 21;
const CMD_GET_DEVICE_INFO: u8 = 22;
const CMD_GET_DEVICE_INFO_RESPONSE: u8 = 23;

#[cfg(not(target_os = "none"))]
fn sleep_ms(ms: u64) {
    std::thread::sleep(std::time::Duration::from_millis(ms));
}

#[cfg(target_os = "none")]
fn sleep_ms(_ms: u64) {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AcMode {
    Cool,
    Heat,
    Auto,
    Dry,
    Fan,
    Eco,
    Unknown,
}

impl AcMode {
    pub fn from_u8(val: u8) -> Self {
        match val {
            0 => Self::Cool,
            1 => Self::Heat,
            2 => Self::Auto,
            3 => Self::Dry,
            4 => Self::Fan,
            5 => Self::Eco,
            _ => Self::Unknown,
        }
    }

    pub fn to_u8(&self) -> u8 {
        match self {
            Self::Cool => 0,
            Self::Heat => 1,
            Self::Auto => 2,
            Self::Dry => 3,
            Self::Fan => 4,
            Self::Eco => 5,
            Self::Unknown => 0xff,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FanSpeed {
    Auto,
    Low,
    Medium,
    High,
    Turbo,
    Quiet,
    Unknown,
}

impl FanSpeed {
    pub fn from_u8(val: u8) -> Self {
        match val {
            0 => Self::Auto,
            1 => Self::Low,
            2 => Self::Medium,
            3 => Self::High,
            4 => Self::Turbo,
            5 => Self::Quiet,
            _ => Self::Unknown,
        }
    }

    pub fn to_u8(&self) -> u8 {
        match self {
            Self::Auto => 0,
            Self::Low => 1,
            Self::Medium => 2,
            Self::High => 3,
            Self::Turbo => 4,
            Self::Quiet => 5,
            Self::Unknown => 0xff,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindDirection {
    Fixed,
    Swing,
    Auto,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AcState {
    pub power: bool,
    pub temperature: i8,
    pub mode: AcMode,
    pub fan_speed: FanSpeed,
    pub wind_direction: WindDirection,
    pub eco_mode: bool,
    pub turbo_mode: bool,
    pub quiet_mode: bool,
    pub light_enabled: bool,
}

impl Default for AcState {
    fn default() -> Self {
        Self {
            power: false,
            temperature: 24,
            mode: AcMode::Cool,
            fan_speed: FanSpeed::Auto,
            wind_direction: WindDirection::Fixed,
            eco_mode: false,
            turbo_mode: false,
            quiet_mode: false,
            light_enabled: true,
        }
    }
}

struct SessionKeys {
    session_key: [u8; 16],
}

pub struct TclAcClient {
    device_addr: RwLock<Option<String>>,
    connected: RwLock<bool>,
    socket: RwLock<Option<L2capSocket>>,
    service_handle: RwLock<Option<u16>>,
    write_char_handle: RwLock<Option<u16>>,
    indicate_char_handle: RwLock<Option<u16>>,
    mtu: RwLock<u16>,
    session: RwLock<Option<SessionKeys>>,
    encrypted: RwLock<bool>,
}

impl TclAcClient {
    pub fn new() -> Self {
        Self {
            device_addr: RwLock::new(None),
            connected: RwLock::new(false),
            socket: RwLock::new(None),
            service_handle: RwLock::new(None),
            write_char_handle: RwLock::new(None),
            indicate_char_handle: RwLock::new(None),
            mtu: RwLock::new(512),
            session: RwLock::new(None),
            encrypted: RwLock::new(false),
        }
    }

    pub fn connect(&self, device_addr: &str) -> Result<(), CapabilityError> {
        if device_addr.is_empty() {
            return Err(CapabilityError::Provider("device address required".into()));
        }

        let socket = match Self::connect_socket(device_addr, 0x01) {
            Ok(socket) => socket,
            Err(public_err) => Self::connect_socket(device_addr, 0x02).map_err(|random_err| {
                CapabilityError::Provider(format!(
                    "failed to connect as public ({}) or random ({}) BLE address",
                    public_err, random_err
                ))
            })?,
        };

        *self.device_addr.write().unwrap() = Some(device_addr.to_string());
        *self.connected.write().unwrap() = true;
        *self.socket.write().unwrap() = Some(socket);

        if let Ok(mut proto) = self.create_protocol() {
            if let Ok(mtu) = proto.exchange_mtu(512) {
                *self.mtu.write().unwrap() = mtu;
            }
        }

        self.discover_tcl_service()?;
        self.enable_indications()?;
        if *self.encrypted.read().unwrap() {
            self.perform_key_exchange()?;
        }

        Ok(())
    }

    fn connect_socket(device_addr: &str, addr_type: u8) -> Result<L2capSocket, GattError> {
        let socket = L2capSocket::new()?;
        socket.connect_device(device_addr, addr_type)?;
        Ok(socket)
    }

    fn discover_tcl_service(&self) -> Result<(), CapabilityError> {
        let mut proto = self.create_protocol()?;

        let primary_service_type: [u8; 2] = [0x00, 0x28];
        let data = proto
            .read_by_group_type(0x0001, 0xFFFF, &primary_service_type)
            .map_err(|e| {
                CapabilityError::Provider(format!("failed to discover services: {}", e))
            })?;

        let services = proto.parse_read_by_group_response(&data);

        for (start, end, uuid) in services {
            if uuid_matches(&uuid, TCL_SERVICE_UUID) {
                *self.encrypted.write().unwrap() = true;
                *self.service_handle.write().unwrap() = Some(start);
                self.discover_characteristics_in_range(start, end)?;
                return Ok(());
            }
            if uuid_matches(&uuid, TCL_LEGACY_SERVICE_UUID) {
                *self.encrypted.write().unwrap() = false;
                *self.service_handle.write().unwrap() = Some(start);
                self.discover_characteristics_in_range(start, end)?;
                return Ok(());
            }
        }

        *self.service_handle.write().unwrap() = None;
        Ok(())
    }

    fn discover_characteristics_in_range(
        &self,
        start: u16,
        end: u16,
    ) -> Result<(), CapabilityError> {
        let mut proto = self.create_protocol()?;

        let char_type: [u8; 2] = [0x03, 0x28];
        let data = proto.read_by_type(start, end, &char_type).map_err(|e| {
            CapabilityError::Provider(format!("failed to discover characteristics: {}", e))
        })?;

        let chars = proto.parse_read_by_type_response(&data);

        for (handle, value) in chars {
            if value.len() < 5 {
                continue;
            }
            let value_handle = u16::from_le_bytes([value[1], value[2]]);
            let uuid = value[3..].to_vec();

            if uuid_matches(&uuid, TCL_WRITE_CHAR_UUID)
                || uuid_matches(&uuid, TCL_LEGACY_WRITE_CHAR_UUID)
            {
                *self.write_char_handle.write().unwrap() = Some(value_handle);
            } else if uuid_matches(&uuid, TCL_INDICATE_CHAR_UUID)
                || uuid_matches(&uuid, TCL_LEGACY_INDICATE_CHAR_UUID)
            {
                *self.indicate_char_handle.write().unwrap() = Some(value_handle);
            }
        }

        Ok(())
    }

    fn enable_indications(&self) -> Result<(), CapabilityError> {
        self.service_handle
            .read()
            .unwrap()
            .ok_or_else(|| CapabilityError::Provider("TCL service not found".into()))?;
        let indicate_handle =
            self.indicate_char_handle.read().unwrap().ok_or_else(|| {
                CapabilityError::Provider("indicate characteristic not found".into())
            })?;
        let mut proto = self.create_protocol()?;
        let data = proto
            .find_information(indicate_handle + 1, 0xffff)
            .map_err(|e| {
                CapabilityError::Provider(format!("failed to discover descriptors: {}", e))
            })?;
        for (handle, uuid) in proto.parse_find_information_response(&data) {
            if format_gatt_uuid(&uuid) == "2902" {
                proto
                    .write_value(handle, &[0x02, 0x00], true)
                    .map_err(|e| {
                        CapabilityError::Provider(format!("failed to enable indications: {}", e))
                    })?;
                return Ok(());
            }
        }
        Err(CapabilityError::Provider(
            "client characteristic configuration descriptor not found".into(),
        ))
    }

    fn create_protocol(&self) -> Result<AttProtocol, CapabilityError> {
        let socket_guard = self.socket.read().unwrap();
        let socket = socket_guard
            .as_ref()
            .ok_or_else(|| CapabilityError::Provider("not connected".into()))?;
        let mut proto = AttProtocol::new(socket.clone());
        let mtu = *self.mtu.read().unwrap();
        proto.set_mtu(mtu);
        Ok(proto)
    }

    fn create_protocol_gatt(&self) -> Result<AttProtocol, GattError> {
        let socket_guard = self.socket.read().unwrap();
        let socket = socket_guard.as_ref().ok_or(GattError::NotConnected)?;
        let mut proto = AttProtocol::new(socket.clone());
        let mtu = *self.mtu.read().unwrap();
        proto.set_mtu(mtu);
        Ok(proto)
    }

    fn derive_session_key(&self, local_random: &[u8; 32], remote_random: &[u8; 32]) -> [u8; 16] {
        let mut combined = [0u8; 64];
        combined[..32].copy_from_slice(local_random);
        combined[32..].copy_from_slice(remote_random);

        let prk = hmac_sha256(&combined, BASE_KEY);
        let mut info = Vec::new();
        info.push(1);
        let okm = hmac_sha256(&prk, &info);
        let mut key = [0u8; 16];
        key.copy_from_slice(&okm[..16]);
        key
    }

    fn perform_key_exchange(&self) -> Result<(), CapabilityError> {
        let write_handle = {
            let guard = self.write_char_handle.read().unwrap();
            guard
                .ok_or_else(|| CapabilityError::Provider("write characteristic not found".into()))?
        };

        let mut local_random: [u8; 32] = [0; 32];
        OsRng.fill_bytes(&mut local_random);

        let encrypted_random = self.encrypt_payload(&local_random);
        let send_data = self.build_protocol_packet(CMD_SEND_APP_RANDOM, &encrypted_random);

        let mut proto = self.create_protocol()?;
        proto.write_cmd(write_handle, &send_data)?;

        let response = self.wait_for_protocol_packet(3000)?;

        let parsed = self
            .parse_protocol_packet(&response)
            .ok_or_else(|| CapabilityError::Provider("invalid key exchange response".into()))?;

        if parsed.0 != CMD_SEND_DEVICE_RANDOM {
            return Err(CapabilityError::Provider(format!(
                "unexpected response cmd: {}",
                parsed.0
            )));
        }

        let decrypted = self
            .decrypt_payload(&parsed.1)
            .ok_or_else(|| CapabilityError::Provider("failed to decrypt remote random".into()))?;

        if decrypted.len() != 32 {
            return Err(CapabilityError::Provider(
                "invalid remote random length".into(),
            ));
        }

        let mut remote_random = [0u8; 32];
        remote_random.copy_from_slice(&decrypted[..32]);

        let session_key = self.derive_session_key(&local_random, &remote_random);

        *self.session.write().unwrap() = Some(SessionKeys { session_key });

        Ok(())
    }

    fn encrypt_payload(&self, payload: &[u8]) -> Vec<u8> {
        let session = self.session.read().unwrap();
        let key = session.as_ref().map(|s| &s.session_key).unwrap_or(BASE_KEY);

        aes128_cbc_encrypt_pkcs7(key, PRESET_IV, payload)
    }

    fn decrypt_payload(&self, payload: &[u8]) -> Option<Vec<u8>> {
        let session = self.session.read().unwrap();
        let key = session.as_ref().map(|s| &s.session_key).unwrap_or(BASE_KEY);

        aes128_cbc_decrypt_pkcs7(key, PRESET_IV, payload)
    }

    fn build_protocol_packet(&self, cmd: u8, payload: &[u8]) -> Vec<u8> {
        let payload_len = payload.len();
        let total_len = payload_len + PROTOCOL_HEADER_LEN;

        let mut packet = vec![0u8; total_len];
        packet[0] = PROTOCOL_HEAD;
        packet[1] = cmd;
        packet[2] = ((payload_len >> 8) & 0xFF) as u8;
        packet[3] = (payload_len & 0xFF) as u8;
        packet[4..4 + payload_len].copy_from_slice(payload);

        let sha = sha256(payload);
        packet[4 + payload_len..4 + payload_len + 32].copy_from_slice(&sha);

        let crc = Self::calculate_crc8(&packet[..total_len - 1]);
        packet[total_len - 1] = crc;

        packet
    }

    fn parse_protocol_packet(&self, data: &[u8]) -> Option<(u8, Vec<u8>)> {
        if data.len() < PROTOCOL_HEADER_LEN || data[0] != PROTOCOL_HEAD {
            return None;
        }

        let cmd = data[1];
        let payload_len = ((data[2] as usize) << 8) | (data[3] as usize);
        let expected_total = payload_len + PROTOCOL_HEADER_LEN;

        if data.len() != expected_total {
            return None;
        }

        let mut payload = vec![0u8; payload_len];
        payload.copy_from_slice(&data[4..4 + payload_len]);

        let crc = Self::calculate_crc8(&data[..data.len() - 1]);
        if crc != data[data.len() - 1] {
            return None;
        }
        let sha = sha256(&payload);
        if sha.as_slice() != &data[4 + payload_len..4 + payload_len + 32] {
            return None;
        }

        Some((cmd, payload))
    }

    fn wait_for_protocol_packet(&self, timeout_ms: i32) -> Result<Vec<u8>, CapabilityError> {
        let socket_guard = self.socket.read().unwrap();
        let socket = socket_guard
            .as_ref()
            .ok_or_else(|| CapabilityError::Provider("not connected".into()))?;
        let indicate_handle =
            self.indicate_char_handle.read().unwrap().ok_or_else(|| {
                CapabilityError::Provider("indicate characteristic not found".into())
            })?;
        let mut proto = self.create_protocol()?;
        let start = now_ms();
        let mut payload = Vec::new();
        loop {
            let elapsed = now_ms().saturating_sub(start);
            if elapsed >= timeout_ms as u64 {
                return Err(CapabilityError::Provider(
                    "timed out waiting for indication".into(),
                ));
            }
            let mut buf = vec![0u8; *self.mtu.read().unwrap() as usize];
            let remaining = (timeout_ms as u64 - elapsed).min(500) as i32;
            let n = match socket.recv_data(&mut buf, remaining) {
                Ok(n) => n,
                Err(GattError::Timeout(_)) => continue,
                Err(e) => {
                    return Err(CapabilityError::Provider(format!(
                        "failed reading indication: {}",
                        e
                    )));
                }
            };
            buf.truncate(n);
            let indicated = proto
                .handle_indication(&buf)
                .or_else(|| proto.handle_notification(&buf));
            let Some((handle, value)) = indicated else {
                continue;
            };
            if handle != indicate_handle {
                continue;
            }
            if value.first() == Some(&PROTOCOL_HEAD) {
                payload.clear();
            }
            payload.extend_from_slice(&value);
            if payload.len() >= 4 && payload[0] == PROTOCOL_HEAD {
                let expected =
                    (((payload[2] as usize) << 8) | payload[3] as usize) + PROTOCOL_HEADER_LEN;
                if payload.len() == expected {
                    return Ok(payload);
                }
            }
        }
    }

    fn calculate_crc8(data: &[u8]) -> u8 {
        let mut crc: u8 = 0;
        for &byte in data {
            crc ^= byte;
            for _ in 0..8 {
                crc = if crc & 0x80 != 0 {
                    (crc << 1) ^ 0x07
                } else {
                    crc << 1
                };
            }
        }
        crc
    }

    pub fn disconnect(&self) -> Result<(), CapabilityError> {
        *self.connected.write().unwrap() = false;
        *self.socket.write().unwrap() = None;
        *self.device_addr.write().unwrap() = None;
        *self.service_handle.write().unwrap() = None;
        *self.write_char_handle.write().unwrap() = None;
        *self.indicate_char_handle.write().unwrap() = None;
        *self.session.write().unwrap() = None;
        *self.encrypted.write().unwrap() = false;
        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        *self.connected.read().unwrap()
    }

    pub fn send_encrypted_command(&self, cmd: u8, payload: &[u8]) -> Result<(), CapabilityError> {
        let write_handle = {
            let guard = self.write_char_handle.read().unwrap();
            guard
                .ok_or_else(|| CapabilityError::Provider("write characteristic not found".into()))?
        };

        let encrypted = self.encrypt_payload(payload);
        let packet = self.build_protocol_packet(cmd, &encrypted);

        let mut proto = self.create_protocol_gatt()?;
        proto.write_cmd(write_handle, &packet).map_err(Into::into)
    }

    pub fn send_raw_command(&self, payload: &[u8]) -> Result<(), CapabilityError> {
        let write_handle = {
            let guard = self.write_char_handle.read().unwrap();
            guard
                .ok_or_else(|| CapabilityError::Provider("write characteristic not found".into()))?
        };

        let mut proto = self.create_protocol_gatt()?;
        let max_write = proto.mtu().saturating_sub(3) as usize;
        if payload.len() <= max_write {
            return proto
                .write_value(write_handle, payload, true)
                .map_err(Into::into);
        }

        let chunk_len = max_write;
        if chunk_len == 0 {
            return Err(CapabilityError::Provider(
                "invalid ATT MTU for split write".into(),
            ));
        }
        for chunk in payload.chunks(chunk_len) {
            proto.write_value(write_handle, chunk, true)?;
            sleep_ms(10);
        }
        Ok(())
    }

    fn wait_for_raw_value(&self, timeout_ms: i32) -> Result<Vec<u8>, CapabilityError> {
        let socket_guard = self.socket.read().unwrap();
        let socket = socket_guard
            .as_ref()
            .ok_or_else(|| CapabilityError::Provider("not connected".into()))?;
        let indicate_handle =
            self.indicate_char_handle.read().unwrap().ok_or_else(|| {
                CapabilityError::Provider("indicate characteristic not found".into())
            })?;
        let proto = self.create_protocol()?;
        let start = now_ms();
        loop {
            let elapsed = now_ms().saturating_sub(start);
            if elapsed >= timeout_ms as u64 {
                return Err(CapabilityError::Provider(
                    "timed out waiting for indication".into(),
                ));
            }
            let mut buf = vec![0u8; *self.mtu.read().unwrap() as usize];
            let remaining = (timeout_ms as u64 - elapsed).min(500) as i32;
            let n = match socket.recv_data(&mut buf, remaining) {
                Ok(n) => n,
                Err(GattError::Timeout(_)) => continue,
                Err(e) => {
                    return Err(CapabilityError::Provider(format!(
                        "failed reading indication: {}",
                        e
                    )));
                }
            };
            buf.truncate(n);
            let indicated = proto
                .handle_indication(&buf)
                .or_else(|| proto.handle_notification(&buf));
            let Some((handle, value)) = indicated else {
                continue;
            };
            if handle == indicate_handle {
                return Ok(value);
            }
        }
    }

    pub fn get_device_info(&self) -> Result<String, CapabilityError> {
        if !*self.encrypted.read().unwrap() {
            self.send_raw_command(
                br#"{"msgId":"123","version":"1","method":"getDeviceInfo","params":{"code":0}}"#,
            )?;
            let response = self.wait_for_raw_value(3000)?;
            return String::from_utf8(response)
                .map_err(|e| CapabilityError::Provider(format!("invalid UTF-8: {}", e)));
        }

        self.send_encrypted_command(CMD_GET_DEVICE_INFO, br#"{"command":"get_wifi_status"}"#)?;

        let response = self.wait_for_protocol_packet(3000)?;

        let parsed = self
            .parse_protocol_packet(&response)
            .ok_or_else(|| CapabilityError::Provider("invalid device info response".into()))?;

        if parsed.0 != CMD_GET_DEVICE_INFO_RESPONSE {
            return Err(CapabilityError::Provider(format!(
                "unexpected response cmd: {}",
                parsed.0
            )));
        }

        let decrypted = self
            .decrypt_payload(&parsed.1)
            .ok_or_else(|| CapabilityError::Provider("failed to decrypt device info".into()))?;

        String::from_utf8(decrypted)
            .map_err(|e| CapabilityError::Provider(format!("invalid UTF-8: {}", e)))
    }

    pub fn provision_wifi(
        &self,
        ssid: &str,
        password: &str,
        bind_code: &str,
        tenant_id: Option<&str>,
        new_product_key: Option<&str>,
    ) -> Result<Option<String>, CapabilityError> {
        let payload = self.build_legacy_provision_payload(
            ssid,
            password,
            bind_code,
            tenant_id,
            new_product_key,
        );
        self.send_raw_command(payload.as_bytes())?;

        match self.wait_for_raw_value(15_000) {
            Ok(response) => String::from_utf8(response)
                .map(Some)
                .map_err(|e| CapabilityError::Provider(format!("invalid UTF-8: {}", e))),
            Err(CapabilityError::Provider(msg)) if msg == "timed out waiting for indication" => {
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }

    pub fn provision_wifi_responses(
        &self,
        ssid: &str,
        password: &str,
        bind_code: &str,
        tenant_id: Option<&str>,
        new_product_key: Option<&str>,
        timeout_ms: i32,
    ) -> Result<Vec<String>, CapabilityError> {
        let payload = self.build_legacy_provision_payload(
            ssid,
            password,
            bind_code,
            tenant_id,
            new_product_key,
        );
        self.send_raw_command(payload.as_bytes())?;

        let start = now_ms();
        let mut responses = Vec::new();
        loop {
            let elapsed = now_ms().saturating_sub(start);
            if elapsed >= timeout_ms as u64 {
                return Ok(responses);
            }
            let remaining = (timeout_ms as u64 - elapsed).min(5_000) as i32;
            match self.wait_for_raw_value(remaining) {
                Ok(response) => responses.push(
                    String::from_utf8(response)
                        .map_err(|e| CapabilityError::Provider(format!("invalid UTF-8: {}", e)))?,
                ),
                Err(CapabilityError::Provider(msg))
                    if msg == "timed out waiting for indication" =>
                {
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
    }

    pub fn provision_wifi_with_commission_responses(
        &self,
        ssid: &str,
        password: &str,
        bind_code: &str,
        server_host: Option<&str>,
        server_host_v2: Option<&str>,
        tenant_id: Option<&str>,
        new_product_key: Option<&str>,
        timeout_ms: i32,
    ) -> Result<Vec<String>, CapabilityError> {
        let payload = self.build_legacy_provision_payload_with_hosts(
            ssid,
            password,
            bind_code,
            server_host,
            server_host_v2,
            tenant_id,
            new_product_key,
        );
        self.send_raw_command(payload.as_bytes())?;

        let start = now_ms();
        let mut responses = Vec::new();
        loop {
            let elapsed = now_ms().saturating_sub(start);
            if elapsed >= timeout_ms as u64 {
                return Ok(responses);
            }
            let remaining = (timeout_ms as u64 - elapsed).min(5_000) as i32;
            match self.wait_for_raw_value(remaining) {
                Ok(response) => responses.push(
                    String::from_utf8(response)
                        .map_err(|e| CapabilityError::Provider(format!("invalid UTF-8: {}", e)))?,
                ),
                Err(CapabilityError::Provider(msg))
                    if msg == "timed out waiting for indication" =>
                {
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
    }

    fn build_legacy_provision_payload(
        &self,
        ssid: &str,
        password: &str,
        bind_code: &str,
        tenant_id: Option<&str>,
        new_product_key: Option<&str>,
    ) -> String {
        self.build_legacy_provision_payload_with_hosts(
            ssid,
            password,
            bind_code,
            Some("prod-center.aws.tcljd.com"),
            Some("prod-center.aws.tcljd.com"),
            tenant_id,
            new_product_key,
        )
    }

    fn build_legacy_provision_payload_with_hosts(
        &self,
        ssid: &str,
        password: &str,
        bind_code: &str,
        server_host: Option<&str>,
        server_host_v2: Option<&str>,
        tenant_id: Option<&str>,
        new_product_key: Option<&str>,
    ) -> String {
        let msg_id = ((now_ms() / 1000) % 900 + 100).to_string();
        let mut params = Vec::new();
        push_json_field(&mut params, "bindCode", bind_code);
        push_json_field(&mut params, "ssid", ssid);
        push_json_field(&mut params, "password", password);
        params.push(format!("\"timestamp\":{}", now_ms() / 1000));
        params.push("\"timezone\":7".to_string());
        push_json_field(&mut params, "timearea", "Asia/Bangkok");
        params.push("\"serverPort\":443".to_string());
        push_json_field(&mut params, "cloudType", "AWS");
        push_json_field(&mut params, "caType", "release");
        if let Some(value) = server_host.filter(|s| !s.is_empty()) {
            push_json_field(&mut params, "serverHost", normalize_commission_host(value));
        }
        if let Some(value) = server_host_v2.filter(|s| !s.is_empty()) {
            push_json_field(
                &mut params,
                "serverHostV2",
                normalize_commission_host(value),
            );
        }
        if let Some(value) = tenant_id.filter(|s| !s.is_empty()) {
            push_json_field(&mut params, "tenantId", value);
            push_json_field(&mut params, "stationId", value);
        }
        if let Some(value) = new_product_key.filter(|s| !s.is_empty()) {
            push_json_field(&mut params, "newProductKey", value);
        }

        format!(
            "{{\"msgId\":\"{}\",\"method\":\"setReq\",\"version\":\"1\",\"params\":{{{}}}}}",
            msg_id,
            params.join(",")
        )
    }

    pub fn report_status_ack(&self) -> Result<(), CapabilityError> {
        self.send_encrypted_command(CMD_STATUS_REPORT_RESPONSE, b"reportStatusAck")
    }

    pub fn read_state(&self) -> Result<AcState, CapabilityError> {
        let char_handle = {
            let guard = self.indicate_char_handle.read().unwrap();
            *guard
        };

        if char_handle.is_none() {
            return Err(CapabilityError::Provider(
                "characteristic not discovered".into(),
            ));
        }

        let mut proto = self.create_protocol()?;
        let data = proto.read_value(char_handle.unwrap())?;

        let parsed = self.parse_protocol_packet(&data);

        if let Some((cmd, encrypted_payload)) = parsed {
            if cmd == CMD_STATUS_REPORT_RESPONSE || cmd == CMD_GET_DEVICE_INFO_RESPONSE {
                if let Some(decrypted) = self.decrypt_payload(&encrypted_payload) {
                    return Ok(self.parse_state_from_response(&decrypted));
                }
            }
        }

        Ok(self.parse_state_from_response(&data))
    }

    fn parse_state_from_response(&self, data: &[u8]) -> AcState {
        let mut state = AcState::default();

        if data.is_empty() {
            return state;
        }

        if !data.is_empty() {
            state.power = data[0] == 0x01;
        }
        if data.len() >= 2 {
            state.temperature = data[1] as i8;
        }
        if data.len() >= 3 {
            state.mode = AcMode::from_u8(data[2]);
        }
        if data.len() >= 4 {
            state.fan_speed = FanSpeed::from_u8(data[3]);
        }
        if data.len() >= 5 {
            state.wind_direction = match data[4] {
                0 => WindDirection::Fixed,
                1 => WindDirection::Swing,
                _ => WindDirection::Auto,
            };
        }
        if data.len() >= 6 {
            state.eco_mode = (data[5] & 0x01) != 0;
            state.turbo_mode = (data[5] & 0x02) != 0;
            state.quiet_mode = (data[5] & 0x04) != 0;
        }

        state
    }

    pub fn set_power(&self, on: bool) -> Result<(), CapabilityError> {
        let payload = vec![0x01, if on { 0x01 } else { 0x00 }];
        self.send_encrypted_command(CMD_SEND_WIFI_INFO, &payload)
    }

    pub fn set_temperature(&self, temp: i8) -> Result<(), CapabilityError> {
        let temp = temp.clamp(16, 31);
        let payload = vec![0x02, temp as u8];
        self.send_encrypted_command(CMD_SEND_WIFI_INFO, &payload)
    }

    pub fn set_mode(&self, mode: AcMode) -> Result<(), CapabilityError> {
        let payload = vec![0x03, mode.to_u8()];
        self.send_encrypted_command(CMD_SEND_WIFI_INFO, &payload)
    }

    pub fn set_fan_speed(&self, speed: FanSpeed) -> Result<(), CapabilityError> {
        let payload = vec![0x04, speed.to_u8()];
        self.send_encrypted_command(CMD_SEND_WIFI_INFO, &payload)
    }

    pub fn set_wind_swing(&self, enable: bool) -> Result<(), CapabilityError> {
        let payload = vec![0x05, if enable { 0x01 } else { 0x00 }];
        self.send_encrypted_command(CMD_SEND_WIFI_INFO, &payload)
    }

    pub fn set_eco(&self, enable: bool) -> Result<(), CapabilityError> {
        let payload = vec![0x06, if enable { 0x01 } else { 0x00 }];
        self.send_encrypted_command(CMD_SEND_WIFI_INFO, &payload)
    }

    pub fn set_turbo(&self, enable: bool) -> Result<(), CapabilityError> {
        let payload = vec![0x07, if enable { 0x01 } else { 0x00 }];
        self.send_encrypted_command(CMD_SEND_WIFI_INFO, &payload)
    }

    pub fn set_quiet(&self, enable: bool) -> Result<(), CapabilityError> {
        let payload = vec![0x08, if enable { 0x01 } else { 0x00 }];
        self.send_encrypted_command(CMD_SEND_WIFI_INFO, &payload)
    }

    pub fn full_control(&self, state: &AcState) -> Result<(), CapabilityError> {
        let mut payload = vec![
            0x10,
            if state.power { 0x01 } else { 0x00 },
            state.temperature as u8,
            state.mode.to_u8(),
            state.fan_speed.to_u8(),
        ];

        let mut flags = 0u8;
        if state.eco_mode {
            flags |= 0x01;
        }
        if state.turbo_mode {
            flags |= 0x02;
        }
        if state.quiet_mode {
            flags |= 0x04;
        }

        payload.push(flags);
        payload.push(match state.wind_direction {
            WindDirection::Swing => 0x01,
            WindDirection::Fixed => 0x00,
            WindDirection::Auto => 0x02,
            WindDirection::Unknown => 0x00,
        });

        self.send_encrypted_command(CMD_SEND_WIFI_INFO, &payload)
    }

    pub fn device_address(&self) -> Option<String> {
        self.device_addr.read().unwrap().clone()
    }

    pub fn service_handle(&self) -> Option<u16> {
        *self.service_handle.read().unwrap()
    }

    pub fn characteristic_handle(&self) -> Option<u16> {
        *self.write_char_handle.read().unwrap()
    }

    pub fn negotiated_mtu(&self) -> u16 {
        *self.mtu.read().unwrap()
    }

    pub fn uses_legacy_provisioning(&self) -> bool {
        !*self.encrypted.read().unwrap()
    }
}

fn uuid_matches(raw: &[u8], expected: &str) -> bool {
    let actual = format_gatt_uuid(raw).replace('-', "").to_lowercase();
    let expected = expected.replace('-', "").to_lowercase();
    actual == expected || (actual.len() == 4 && expected.starts_with(&format!("0000{}", actual)))
}

fn push_json_field(fields: &mut Vec<String>, key: &str, value: &str) {
    fields.push(format!("\"{}\":\"{}\"", key, json_escape(value)));
}

fn normalize_commission_host(value: &str) -> &str {
    let value = value
        .strip_prefix("https://")
        .or_else(|| value.strip_prefix("http://"))
        .unwrap_or(value);
    value.strip_suffix(":443").unwrap_or(value)
}

fn json_escape(value: &str) -> String {
    let mut escaped = String::new();
    for ch in value.chars() {
        match ch {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            c => escaped.push(c),
        }
    }
    escaped
}

#[cfg(not(target_os = "none"))]
fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(target_os = "none")]
fn now_ms() -> u64 {
    0
}

fn aes128_cbc_encrypt_pkcs7(key: &[u8; 16], iv: &[u8; 16], payload: &[u8]) -> Vec<u8> {
    let cipher = aes::Aes128::new(GenericArray::from_slice(key));
    let pad_len = 16 - (payload.len() % 16);
    let mut out = Vec::with_capacity(payload.len() + pad_len);
    out.extend_from_slice(payload);
    out.extend(core::iter::repeat(pad_len as u8).take(pad_len));

    let mut previous = *iv;
    for block in out.chunks_exact_mut(16) {
        for i in 0..16 {
            block[i] ^= previous[i];
        }
        cipher.encrypt_block(GenericArray::from_mut_slice(block));
        previous.copy_from_slice(block);
    }

    out
}

fn aes128_cbc_decrypt_pkcs7(key: &[u8; 16], iv: &[u8; 16], payload: &[u8]) -> Option<Vec<u8>> {
    if payload.is_empty() || payload.len() % 16 != 0 {
        return None;
    }

    let cipher = aes::Aes128::new(GenericArray::from_slice(key));
    let mut out = Vec::with_capacity(payload.len());
    let mut previous = *iv;

    for block in payload.chunks_exact(16) {
        let mut decrypted = [0u8; 16];
        decrypted.copy_from_slice(block);
        cipher.decrypt_block(GenericArray::from_mut_slice(&mut decrypted));
        for i in 0..16 {
            decrypted[i] ^= previous[i];
        }
        out.extend_from_slice(&decrypted);
        previous.copy_from_slice(block);
    }

    let pad_len = *out.last()? as usize;
    if pad_len == 0 || pad_len > 16 || pad_len > out.len() {
        return None;
    }
    if !out[out.len() - pad_len..]
        .iter()
        .all(|byte| *byte as usize == pad_len)
    {
        return None;
    }

    out.truncate(out.len() - pad_len);
    Some(out)
}

impl Default for TclAcClient {
    fn default() -> Self {
        Self::new()
    }
}

impl CapabilityProvider for TclAcClient {
    fn descriptor(&self) -> edgerun_capabilities::CapabilityDescriptor {
        let device = self.device_addr.read().unwrap();
        let instance = device.clone().unwrap_or_else(|| "disconnected".to_string());
        edgerun_bluetooth_gatt::default_gatt_descriptor("tcl-ac", &instance)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ac_mode_roundtrip() {
        let modes = [
            AcMode::Cool,
            AcMode::Heat,
            AcMode::Auto,
            AcMode::Dry,
            AcMode::Fan,
            AcMode::Eco,
        ];
        for mode in modes {
            assert_eq!(AcMode::from_u8(mode.to_u8()), mode);
        }
    }

    #[test]
    fn fan_speed_roundtrip() {
        let speeds = [
            FanSpeed::Auto,
            FanSpeed::Low,
            FanSpeed::Medium,
            FanSpeed::High,
            FanSpeed::Turbo,
            FanSpeed::Quiet,
        ];
        for speed in speeds {
            assert_eq!(FanSpeed::from_u8(speed.to_u8()), speed);
        }
    }

    #[test]
    fn client_constructs() {
        let client = TclAcClient::new();
        assert!(!client.is_connected());
        assert!(client.device_address().is_none());
    }

    #[test]
    fn state_default() {
        let state = AcState::default();
        assert!(!state.power);
        assert_eq!(state.temperature, 24);
        assert_eq!(state.mode, AcMode::Cool);
    }

    #[test]
    fn crc8_calculation() {
        let data = [0xBB, 0x10, 0x00, 0x20];
        let crc = TclAcClient::calculate_crc8(&data);
        assert_eq!(crc, 0x50);
    }

    #[test]
    fn session_key_derivation() {
        let client = TclAcClient::new();
        let local = [0u8; 32];
        let remote = [1u8; 32];
        let key = client.derive_session_key(&local, &remote);
        assert_eq!(key.len(), 16);
    }

    #[test]
    fn aes_cbc_roundtrips_payloads() {
        for payload in [
            b"".as_slice(),
            b"short".as_slice(),
            b"sixteen byte msg".as_slice(),
        ] {
            let encrypted = aes128_cbc_encrypt_pkcs7(BASE_KEY, PRESET_IV, payload);
            assert_eq!(encrypted.len() % 16, 0);
            assert_eq!(
                aes128_cbc_decrypt_pkcs7(BASE_KEY, PRESET_IV, &encrypted).as_deref(),
                Some(payload)
            );
        }
    }

    #[test]
    fn aes_cbc_rejects_bad_padding() {
        let mut encrypted = aes128_cbc_encrypt_pkcs7(BASE_KEY, PRESET_IV, b"payload");
        let last = encrypted.len() - 1;
        encrypted[last] ^= 0xff;
        assert_eq!(
            aes128_cbc_decrypt_pkcs7(BASE_KEY, PRESET_IV, &encrypted),
            None
        );
    }
}
