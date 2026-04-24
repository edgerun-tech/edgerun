use aes::cipher::{block_padding::Pkcs7, KeyInit, KeyIvInit};
use cbc::cipher::{BlockDecryptMut, BlockEncryptMut};
use edgerun_bluetooth_gatt::{
    format_gatt_uuid, parse_gatt_uuid, AttProtocol, GattAddressKind, GattCharacteristic,
    GattDescriptor, GattError, GattProperty, GattService, GattUuid, L2capSocket,
};
use edgerun_capabilities::{CapabilityError, CapabilityProvider};
use hkdf::Hkdf;
use sha2::{Sha256, Digest};
use std::sync::{RwLock, Mutex};
use rand::Rng;

pub const TCL_SERVICE_UUID: &str = "0000f100-0000-1000-8000-00805f9b34fb";
pub const TCL_WRITE_CHAR_UUID: &str = "0000ff01-0000-1000-8000-00805f9b34fb";
pub const TCL_INDICATE_CHAR_UUID: &str = "0000ff02-0000-1000-8000-00805f9b34fb";

const BASE_KEY: &[u8; 16] = b"Hj8%Wd4*Qy3!Lm6@";
const PRESET_IV: &[u8; 16] = b"GjVEI7lQ382O7Ua0";
const PROTOCOL_HEAD: u8 = 0xBB;
const PROTOCOL_HEADER_LEN: usize = 37;

const CMD_SEND_APP_RANDOM: u8 = 16;
const CMD_SEND_DEVICE_RANDOM: u8 = 17;
const CMD_SEND_WIFI_INFO: u8 = 18;
const CMD_STATUS_REPORT_RESPONSE: u8 = 21;
const CMD_GET_DEVICE_INFO: u8 = 22;
const CMD_GET_DEVICE_INFO_RESPONSE: u8 = 23;

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
    local_random: [u8; 32],
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
        }
    }

    pub fn connect(&self, device_addr: &str) -> Result<(), CapabilityError> {
        if device_addr.is_empty() {
            return Err(CapabilityError::Provider("device address required".into()));
        }

        let socket = L2capSocket::new()?;
        socket.connect_device(device_addr, 0x04)?;

        *self.device_addr.write().unwrap() = Some(device_addr.to_string());
        *self.connected.write().unwrap() = true;
        *self.socket.write().unwrap() = Some(socket);

        self.discover_tcl_service()?;
        self.perform_key_exchange()?;

        Ok(())
    }

    fn discover_tcl_service(&self) -> Result<(), CapabilityError> {
        let mut proto = self.create_protocol()?;

        let primary_service_type: [u8; 2] = [0x00, 0x28];
        let data = proto.read_by_group_type(0x0001, 0xFFFF, &primary_service_type)
            .map_err(|e| CapabilityError::Provider(format!("failed to discover services: {}", e)))?;

        let services = proto.parse_read_by_group_response(&data);

        for (start, end, uuid) in services {
            let uuid_str = format_gatt_uuid(&uuid).replace("-", "");
            let tcl_str = TCL_SERVICE_UUID.replace("-", "");
            if uuid_str.ends_with(&tcl_str[tcl_str.len()-8..]) {
                *self.service_handle.write().unwrap() = Some(start);
                self.discover_characteristics_in_range(start, end)?;
                return Ok(());
            }
        }

        *self.service_handle.write().unwrap() = None;
        Ok(())
    }

    fn discover_characteristics_in_range(&self, start: u16, end: u16) -> Result<(), CapabilityError> {
        let mut proto = self.create_protocol()?;

        let char_type: [u8; 2] = [0x03, 0x28];
        let data = proto.read_by_type(start, end, &char_type)
            .map_err(|e| CapabilityError::Provider(format!("failed to discover characteristics: {}", e)))?;

        let chars = proto.parse_read_by_type_response(&data);

        let write_str = TCL_WRITE_CHAR_UUID.replace("-", "");
        let indicate_str = TCL_INDICATE_CHAR_UUID.replace("-", "");

        for (handle, value) in chars {
            if value.len() < 3 {
                continue;
            }
            let uuid = value[2..].to_vec();
            let uuid_str = format_gatt_uuid(&uuid).replace("-", "");

            if uuid_str.ends_with(&write_str[write_str.len()-8..]) {
                *self.write_char_handle.write().unwrap() = Some(handle);
            } else if uuid_str.ends_with(&indicate_str[indicate_str.len()-8..]) {
                *self.indicate_char_handle.write().unwrap() = Some(handle);
            }
        }

        Ok(())
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
        let socket = socket_guard
            .as_ref()
            .ok_or(GattError::NotConnected)?;
        let mut proto = AttProtocol::new(socket.clone());
        let mtu = *self.mtu.read().unwrap();
        proto.set_mtu(mtu);
        Ok(proto)
    }

    fn derive_session_key(&self, local_random: &[u8; 32], remote_random: &[u8; 32]) -> [u8; 16] {
        let mut combined = [0u8; 64];
        combined[..32].copy_from_slice(local_random);
        combined[32..].copy_from_slice(remote_random);

        let hk = Hkdf::<Sha256>::new(Some(BASE_KEY), &combined);
        let mut okm = [0u8; 16];
        hk.expand(&[], &mut okm).expect("HKDF expand failed");
        okm
    }

    fn perform_key_exchange(&self) -> Result<(), CapabilityError> {
        let write_handle = {
            let guard = self.write_char_handle.read().unwrap();
            guard.ok_or_else(|| CapabilityError::Provider("write characteristic not found".into()))?
        };

        let mut rng = rand::thread_rng();
        let local_random: [u8; 32] = rng.gen();

        let encrypted_random = self.encrypt_payload(&local_random);
        let send_data = self.build_protocol_packet(CMD_SEND_APP_RANDOM, &encrypted_random);

        let mut proto = self.create_protocol()?;
        proto.write_cmd(write_handle, &send_data)?;

        std::thread::sleep(std::time::Duration::from_millis(500));

        let response = proto.read_value(write_handle)
            .map_err(|e| CapabilityError::Provider(format!("failed to read key exchange response: {}", e)))?;

        let parsed = self.parse_protocol_packet(&response)
            .ok_or_else(|| CapabilityError::Provider("invalid key exchange response".into()))?;

        if parsed.0 != CMD_SEND_DEVICE_RANDOM {
            return Err(CapabilityError::Provider(format!("unexpected response cmd: {}", parsed.0)));
        }

        let decrypted = self.decrypt_payload(&parsed.1)
            .ok_or_else(|| CapabilityError::Provider("failed to decrypt remote random".into()))?;

        if decrypted.len() != 32 {
            return Err(CapabilityError::Provider("invalid remote random length".into()));
        }

        let mut remote_random = [0u8; 32];
        remote_random.copy_from_slice(&decrypted[..32]);

        let session_key = self.derive_session_key(&local_random, &remote_random);

        *self.session.write().unwrap() = Some(SessionKeys {
            local_random,
            session_key,
        });

        Ok(())
    }

    fn encrypt_payload(&self, payload: &[u8]) -> Vec<u8> {
        use aes::cipher::KeyIvInit;

        let session = self.session.read().unwrap();
        let key = session.as_ref().map(|s| &s.session_key).unwrap_or(BASE_KEY);

        type Aes128CbcEnc = cbc::Encryptor<aes::Aes128>;
        let cipher = Aes128CbcEnc::new(key.into(), PRESET_IV.into());
        cipher.encrypt_padded_vec_mut::<Pkcs7>(payload)
    }

    fn decrypt_payload(&self, payload: &[u8]) -> Option<Vec<u8>> {
        use aes::cipher::KeyIvInit;

        let session = self.session.read().unwrap();
        let key = session.as_ref().map(|s| &s.session_key).unwrap_or(BASE_KEY);

        type Aes128CbcDec = cbc::Decryptor<aes::Aes128>;
        let cipher = Aes128CbcDec::new(key.into(), PRESET_IV.into());
        cipher.decrypt_padded_vec_mut::<Pkcs7>(payload).ok().map(|v| v.to_vec())
    }

    fn build_protocol_packet(&self, cmd: u8, payload: &[u8]) -> Vec<u8> {
        let payload_len = payload.len();
        let total_len = payload_len + PROTOCOL_HEADER_LEN;

        let mut packet = vec![0u8; total_len];
        packet[0] = PROTOCOL_HEAD;
        packet[1] = cmd;
        packet[2] = ((payload_len >> 8) & 0xFF) as u8;
        packet[3] = (payload_len & 0xFF) as u8;
        packet[4..4+payload_len].copy_from_slice(payload);

        let sha = sha2::Sha256::digest(payload);
        packet[4+payload_len..4+payload_len+32].copy_from_slice(&sha);

        let crc = Self::calculate_crc8(&packet[..total_len-1]);
        packet[total_len-1] = crc;

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
        payload.copy_from_slice(&data[4..4+payload_len]);

        let mut packet_for_crc = data[..data.len()-1].to_vec();
        let crc = Self::calculate_crc8(&packet_for_crc);
        if crc != data[data.len()-1] {
            return None;
        }

        Some((cmd, payload))
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
        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        *self.connected.read().unwrap()
    }

    pub fn send_encrypted_command(&self, cmd: u8, payload: &[u8]) -> Result<(), CapabilityError> {
        let write_handle = {
            let guard = self.write_char_handle.read().unwrap();
            guard.ok_or_else(|| CapabilityError::Provider("write characteristic not found".into()))?
        };

        let encrypted = self.encrypt_payload(payload);
        let packet = self.build_protocol_packet(cmd, &encrypted);

        let mut proto = self.create_protocol_gatt()?;
        proto.write_cmd(write_handle, &packet).map_err(Into::into)
    }

    pub fn get_device_info(&self) -> Result<String, CapabilityError> {
        let indicate_handle = {
            let guard = self.indicate_char_handle.read().unwrap();
            guard.ok_or_else(|| CapabilityError::Provider("indicate characteristic not found".into()))?
        };

        self.send_encrypted_command(CMD_GET_DEVICE_INFO, b"")?;

        std::thread::sleep(std::time::Duration::from_millis(500));

        let mut proto = self.create_protocol()?;
        let response = proto.read_value(indicate_handle)
            .map_err(|e| CapabilityError::Provider(format!("failed to read device info: {}", e)))?;

        let parsed = self.parse_protocol_packet(&response)
            .ok_or_else(|| CapabilityError::Provider("invalid device info response".into()))?;

        if parsed.0 != CMD_GET_DEVICE_INFO_RESPONSE {
            return Err(CapabilityError::Provider(format!("unexpected response cmd: {}", parsed.0)));
        }

        let decrypted = self.decrypt_payload(&parsed.1)
            .ok_or_else(|| CapabilityError::Provider("failed to decrypt device info".into()))?;

        String::from_utf8(decrypted)
            .map_err(|e| CapabilityError::Provider(format!("invalid UTF-8: {}", e)))
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
            return Err(CapabilityError::Provider("characteristic not discovered".into()));
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
        if state.eco_mode { flags |= 0x01; }
        if state.turbo_mode { flags |= 0x02; }
        if state.quiet_mode { flags |= 0x04; }

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
        let modes = [AcMode::Cool, AcMode::Heat, AcMode::Auto, AcMode::Dry, AcMode::Fan, AcMode::Eco];
        for mode in modes {
            assert_eq!(AcMode::from_u8(mode.to_u8()), mode);
        }
    }

    #[test]
    fn fan_speed_roundtrip() {
        let speeds = [FanSpeed::Auto, FanSpeed::Low, FanSpeed::Medium, FanSpeed::High, FanSpeed::Turbo, FanSpeed::Quiet];
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
}