//! TCL AC BLE protocol framing.
//!
//! This module owns packet bytes only. Linux Bluetooth, GATT handles,
//! indication reassembly, retries, and timing remain outside this crate.

use crate::prelude::*;
use edgerun_crypto::sha256;

pub const PROTOCOL_HEAD: u8 = 0xBB;
pub const PROTOCOL_HEADER_LEN: usize = 37;

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
}
