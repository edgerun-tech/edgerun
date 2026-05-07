//! TCL AC BLE protocol framing.
//!
//! This module owns packet bytes only. Linux Bluetooth, GATT handles,
//! indication reassembly, retries, and timing remain outside this crate.

use crate::prelude::*;
use alloc::format;
use edgerun_crypto::aes::{Aes128, AES_BLOCK_SIZE};
use edgerun_crypto::hmac_sha256;
use edgerun_crypto::sha256;

pub const PROTOCOL_HEAD: u8 = 0xBB;
pub const PROTOCOL_HEADER_LEN: usize = 37;
pub const BASE_KEY: &[u8; 16] = b"p7#z9@L2!c5%v1&k";
pub const PRESET_IV: &[u8; 16] = b"GjVEI7lQ382O7Ua0";

pub const CMD_SEND_APP_RANDOM: u8 = 16;
pub const CMD_SEND_DEVICE_RANDOM: u8 = 17;
pub const CMD_SEND_WIFI_INFO: u8 = 18;
pub const CMD_STATUS_REPORT_RESPONSE: u8 = 21;
pub const CMD_GET_DEVICE_INFO: u8 = 22;
pub const CMD_GET_DEVICE_INFO_RESPONSE: u8 = 23;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TclAcPacket {
    pub command: u8,
    pub payload: Vec<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TclAcPacketError {
    TooShort,
    BadHead,
    LengthMismatch,
    BadCrc,
    BadDigest,
}

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

pub fn build_protocol_packet(command: u8, payload: &[u8]) -> Vec<u8> {
    let payload_len = payload.len();
    let total_len = payload_len + PROTOCOL_HEADER_LEN;

    let mut packet = Vec::with_capacity(total_len);
    packet.push(PROTOCOL_HEAD);
    packet.push(command);
    packet.push(((payload_len >> 8) & 0xFF) as u8);
    packet.push((payload_len & 0xFF) as u8);
    packet.extend_from_slice(payload);

    let digest = sha256(payload);
    packet.extend_from_slice(&digest);

    let crc = calculate_crc8(&packet);
    packet.push(crc);
    packet
}

pub fn parse_state_response(data: &[u8]) -> AcState {
    let mut state = AcState::default();

    if data.is_empty() {
        return state;
    }

    state.power = data[0] == 0x01;
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

pub fn full_control_payload(state: &AcState) -> Vec<u8> {
    let mut payload = Vec::new();
    payload.push(0x10);
    payload.push(if state.power { 0x01 } else { 0x00 });
    payload.push(state.temperature as u8);
    payload.push(state.mode.to_u8());
    payload.push(state.fan_speed.to_u8());

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

    payload
}

pub fn derive_session_key(local_random: &[u8; 32], remote_random: &[u8; 32]) -> [u8; 16] {
    let mut combined = [0u8; 64];
    combined[..32].copy_from_slice(local_random);
    combined[32..].copy_from_slice(remote_random);

    let prk = hmac_sha256(&combined, BASE_KEY);
    let okm = hmac_sha256(&prk, &[1]);
    let mut key = [0u8; 16];
    key.copy_from_slice(&okm[..16]);
    key
}

pub fn encrypt_payload_with_key(key: &[u8; 16], payload: &[u8]) -> Vec<u8> {
    aes128_cbc_encrypt_pkcs7(key, PRESET_IV, payload)
}

pub fn decrypt_payload_with_key(key: &[u8; 16], payload: &[u8]) -> Option<Vec<u8>> {
    aes128_cbc_decrypt_pkcs7(key, PRESET_IV, payload)
}

pub fn encrypt_payload(session_key: Option<&[u8; 16]>, payload: &[u8]) -> Vec<u8> {
    encrypt_payload_with_key(session_key.unwrap_or(BASE_KEY), payload)
}

pub fn decrypt_payload(session_key: Option<&[u8; 16]>, payload: &[u8]) -> Option<Vec<u8>> {
    decrypt_payload_with_key(session_key.unwrap_or(BASE_KEY), payload)
}

pub fn legacy_provision_payload(
    msg_id: &str,
    unix_seconds: u64,
    ssid: &str,
    token: &str,
    bind_code: &str,
    tenant_id: Option<&str>,
    new_product_key: Option<&str>,
) -> String {
    legacy_provision_payload_with_hosts(
        msg_id,
        unix_seconds,
        ssid,
        token,
        bind_code,
        Some("prod-center.aws.tcljd.com"),
        Some("prod-center.aws.tcljd.com"),
        tenant_id,
        new_product_key,
    )
}

pub fn legacy_provision_payload_with_hosts(
    msg_id: &str,
    unix_seconds: u64,
    ssid: &str,
    token: &str,
    bind_code: &str,
    server_host: Option<&str>,
    server_host_v2: Option<&str>,
    tenant_id: Option<&str>,
    new_product_key: Option<&str>,
) -> String {
    let mut params = Vec::new();
    push_json_field(&mut params, "bindCode", bind_code);
    push_json_field(&mut params, "ssid", ssid);
    push_json_field(&mut params, "token", token);
    params.push(format!("\"timestamp\":{}", unix_seconds));
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
        json_escape(msg_id),
        params.join(",")
    )
}

pub fn parse_protocol_packet(data: &[u8]) -> Result<TclAcPacket, TclAcPacketError> {
    if data.len() < PROTOCOL_HEADER_LEN {
        return Err(TclAcPacketError::TooShort);
    }
    if data[0] != PROTOCOL_HEAD {
        return Err(TclAcPacketError::BadHead);
    }

    let command = data[1];
    let payload_len = payload_len_from_header(data).ok_or(TclAcPacketError::TooShort)?;
    let expected_total = payload_len + PROTOCOL_HEADER_LEN;
    if data.len() != expected_total {
        return Err(TclAcPacketError::LengthMismatch);
    }

    let expected_crc = calculate_crc8(&data[..data.len() - 1]);
    if expected_crc != data[data.len() - 1] {
        return Err(TclAcPacketError::BadCrc);
    }

    let payload = data[4..4 + payload_len].to_vec();
    let digest = sha256(&payload);
    if digest.as_slice() != &data[4 + payload_len..4 + payload_len + 32] {
        return Err(TclAcPacketError::BadDigest);
    }

    Ok(TclAcPacket { command, payload })
}

pub fn expected_protocol_packet_len(data: &[u8]) -> Option<usize> {
    if data.len() < 4 || data[0] != PROTOCOL_HEAD {
        return None;
    }
    payload_len_from_header(data).map(|len| len + PROTOCOL_HEADER_LEN)
}

pub fn calculate_crc8(data: &[u8]) -> u8 {
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

fn payload_len_from_header(data: &[u8]) -> Option<usize> {
    (data.len() >= 4).then(|| ((data[2] as usize) << 8) | data[3] as usize)
}

fn aes128_cbc_encrypt_pkcs7(key: &[u8; 16], iv: &[u8; 16], payload: &[u8]) -> Vec<u8> {
    let cipher = Aes128::new(key);
    let pad_len = AES_BLOCK_SIZE - (payload.len() % AES_BLOCK_SIZE);
    let mut out = Vec::with_capacity(payload.len() + pad_len);
    out.extend_from_slice(payload);
    out.extend(core::iter::repeat(pad_len as u8).take(pad_len));

    let mut previous = *iv;
    for block in out.chunks_exact_mut(AES_BLOCK_SIZE) {
        for i in 0..AES_BLOCK_SIZE {
            block[i] ^= previous[i];
        }
        let mut plaintext = [0u8; AES_BLOCK_SIZE];
        plaintext.copy_from_slice(block);
        let encrypted = cipher.encrypt_block(&plaintext);
        block.copy_from_slice(&encrypted);
        previous = encrypted;
    }

    out
}

fn aes128_cbc_decrypt_pkcs7(key: &[u8; 16], iv: &[u8; 16], payload: &[u8]) -> Option<Vec<u8>> {
    if payload.is_empty() || payload.len() % AES_BLOCK_SIZE != 0 {
        return None;
    }

    let cipher = Aes128::new(key);
    let mut out = Vec::with_capacity(payload.len());
    let mut previous = *iv;

    for block in payload.chunks_exact(AES_BLOCK_SIZE) {
        let mut encrypted = [0u8; AES_BLOCK_SIZE];
        encrypted.copy_from_slice(block);
        let mut decrypted = cipher.decrypt_block(&encrypted);
        for i in 0..AES_BLOCK_SIZE {
            decrypted[i] ^= previous[i];
        }
        out.extend_from_slice(&decrypted);
        previous = encrypted;
    }

    let pad_len = *out.last()? as usize;
    if pad_len == 0 || pad_len > AES_BLOCK_SIZE || pad_len > out.len() {
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

fn push_json_field(fields: &mut Vec<String>, key: &str, value: &str) {
    fields.push(format!("\"{}\":\"{}\"", key, json_escape(value)));
}

pub fn normalize_commission_host(value: &str) -> &str {
    let value = value
        .strip_prefix("https://")
        .or_else(|| value.strip_prefix("http://"))
        .unwrap_or(value);
    value.strip_suffix(":443").unwrap_or(value)
}

pub fn json_escape(value: &str) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc8_matches_existing_vector() {
        let data = [0xBB, 0x10, 0x00, 0x20];
        assert_eq!(calculate_crc8(&data), 0x50);
    }

    #[test]
    fn packet_roundtrip() {
        let packet = build_protocol_packet(CMD_SEND_APP_RANDOM, b"payload");
        let parsed = parse_protocol_packet(&packet).unwrap();

        assert_eq!(parsed.command, CMD_SEND_APP_RANDOM);
        assert_eq!(parsed.payload, b"payload");
        assert_eq!(
            expected_protocol_packet_len(&packet[..4]),
            Some(packet.len())
        );
    }

    #[test]
    fn rejects_bad_crc() {
        let mut packet = build_protocol_packet(CMD_SEND_WIFI_INFO, b"payload");
        let last = packet.len() - 1;
        packet[last] ^= 1;

        assert_eq!(
            parse_protocol_packet(&packet),
            Err(TclAcPacketError::BadCrc)
        );
    }

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
    fn parses_state_response() {
        let state = parse_state_response(&[1, 22, 2, 3, 1, 0b0000_0111, 0]);

        assert!(state.power);
        assert_eq!(state.temperature, 22);
        assert_eq!(state.mode, AcMode::Auto);
        assert_eq!(state.fan_speed, FanSpeed::High);
        assert_eq!(state.wind_direction, WindDirection::Swing);
        assert!(state.eco_mode);
        assert!(state.turbo_mode);
        assert!(state.quiet_mode);
        assert!(state.light_enabled);
    }

    #[test]
    fn session_key_derivation_is_stable() {
        let local = [0u8; 32];
        let remote = [1u8; 32];
        assert_eq!(
            derive_session_key(&local, &remote),
            derive_session_key(&local, &remote)
        );
    }

    #[test]
    fn aes_cbc_roundtrips_payloads() {
        for payload in [
            b"".as_slice(),
            b"short".as_slice(),
            b"sixteen byte msg".as_slice(),
        ] {
            let encrypted = encrypt_payload_with_key(BASE_KEY, payload);
            assert_eq!(encrypted.len() % 16, 0);
            assert_eq!(
                decrypt_payload_with_key(BASE_KEY, &encrypted).as_deref(),
                Some(payload)
            );
        }
    }

    #[test]
    fn legacy_provision_payload_escapes_fields() {
        let payload = legacy_provision_payload(
            "123",
            456,
            "ssid\"x",
            "token",
            "bind",
            Some("tenant"),
            Some("product"),
        );
        assert!(payload.contains("\"msgId\":\"123\""));
        assert!(payload.contains("\"timestamp\":456"));
        assert!(payload.contains("\"ssid\":\"ssid\\\"x\""));
        assert!(payload.contains("\"tenantId\":\"tenant\""));
        assert!(payload.contains("\"newProductKey\":\"product\""));
    }
}
