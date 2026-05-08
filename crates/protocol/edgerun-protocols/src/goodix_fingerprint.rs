//! Goodix fingerprint USB packet framing.
//!
//! This module owns the vendor packet bytes and checksums. USB transport,
//! interface claiming, timeouts, and fingerprint capability mapping remain
//! outside this crate.

use crate::prelude::*;
use alloc::vec;
use edgerun_encoding::byteorder::{
    push_u16_le, read_u16_le, read_u32_le, try_read_u16_le, write_u16_le,
};

pub const GOODIX_PACKAGE_CRC_SIZE: usize = 4;
pub const GOODIX_PACKAGE_HEADER_SIZE: usize = 8;
pub const GOODIX_RESPONSE_ACK_CMD: u8 = 0xaa;
pub const GOODIX_SUCCESS: u8 = 0x00;
pub const GOODIX_FAILED: u8 = 0x80;
pub const GOODIX_ERROR_WAIT_FINGER_UP_TIMEOUT: u8 = 0xc7;
pub const GOODIX_MAX_STORED_PRINTS: u8 = 20;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GoodixPacketError {
    ShortPacket,
    BadHeaderCrc,
    ShortPayload,
    BadPayloadLength,
    BadPacketCrc,
    InvalidAckPacket,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GoodixPayloadError {
    ShortLeU16,
    ShortVersionPayload,
    ShortTemplatePayload,
    InvalidTemplateMarker,
    InvalidTemplateDataSize,
    UserIdTooLong,
    ShortResultPayload,
    ShortFingerModePayload,
    ShortFingerConfigPayload,
    ShortEnrollInitPayload,
    ShortEnrollInitTemplateId,
    ShortEnrollUpdatePayload,
    ShortDuplicateCheckPayload,
    ShortDuplicateCheckTemplateLength,
    ShortDuplicateCheckTemplate,
    ShortCapturePayload,
    ShortIdentifyPayload,
    ShortIdentifyMatchPayload,
    ShortIdentifyTemplatePayload,
    ShortFingerListPayload,
    FingerListFailed(u8),
    ShortFingerListCount,
    FingerListCountTooLarge,
    ShortFingerListEntryLength,
    ShortFingerListEntry,
}

impl core::fmt::Display for GoodixPayloadError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::FingerListFailed(code) => write!(f, "Goodix finger-list failed: 0x{code:02x}"),
            other => f.write_str(match other {
                Self::ShortLeU16 => "short little-endian u16 field",
                Self::ShortVersionPayload => "short Goodix version payload",
                Self::ShortTemplatePayload => "short Goodix template payload",
                Self::InvalidTemplateMarker => "invalid Goodix template marker",
                Self::InvalidTemplateDataSize => "invalid Goodix template data size",
                Self::UserIdTooLong => "Goodix user id too long",
                Self::ShortResultPayload => "short Goodix result payload",
                Self::ShortFingerModePayload => "short Goodix finger-mode payload",
                Self::ShortFingerConfigPayload => "short Goodix finger-config payload",
                Self::ShortEnrollInitPayload => "short Goodix enroll-init payload",
                Self::ShortEnrollInitTemplateId => "short Goodix enroll-init template id",
                Self::ShortEnrollUpdatePayload => "short Goodix enroll-update payload",
                Self::ShortDuplicateCheckPayload => "short Goodix duplicate-check payload",
                Self::ShortDuplicateCheckTemplateLength => {
                    "short Goodix duplicate-check template length"
                }
                Self::ShortDuplicateCheckTemplate => "short Goodix duplicate-check template",
                Self::ShortCapturePayload => "short Goodix capture payload",
                Self::ShortIdentifyPayload => "short Goodix identify payload",
                Self::ShortIdentifyMatchPayload => "short Goodix identify match payload",
                Self::ShortIdentifyTemplatePayload => "short Goodix identify template payload",
                Self::ShortFingerListPayload => "short Goodix finger-list payload",
                Self::ShortFingerListCount => "short Goodix finger-list count",
                Self::FingerListCountTooLarge => {
                    "Goodix finger-list count exceeds supported maximum"
                }
                Self::ShortFingerListEntryLength => "short Goodix finger-list entry length",
                Self::ShortFingerListEntry => "short Goodix finger-list entry",
                Self::FingerListFailed(_) => unreachable!(),
            }),
        }
    }
}

impl core::fmt::Display for GoodixPacketError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::ShortPacket => "short Goodix packet",
            Self::BadHeaderCrc => "invalid Goodix header CRC8",
            Self::ShortPayload => "short Goodix packet payload",
            Self::BadPayloadLength => "invalid Goodix payload length",
            Self::BadPacketCrc => "invalid Goodix packet CRC32",
            Self::InvalidAckPacket => "invalid Goodix ack packet",
        })
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

pub fn build_goodix_package(cmd0: u8, cmd1: u8, payload: &[u8]) -> Vec<u8> {
    let mut out =
        Vec::with_capacity(GOODIX_PACKAGE_HEADER_SIZE + payload.len() + GOODIX_PACKAGE_CRC_SIZE);
    let payload_plus_crc = (payload.len() + GOODIX_PACKAGE_CRC_SIZE) as u16;
    let mut header = [0u8; GOODIX_PACKAGE_HEADER_SIZE];
    header[0] = cmd0;
    header[1] = cmd1;
    header[2] = 0;
    header[3] = 0;
    write_u16_le(&mut header, 4, payload_plus_crc);
    header[6] = goodix_crc8(&header[..6]);
    header[7] = !header[6];
    out.extend_from_slice(&header);
    out.extend_from_slice(payload);
    out.extend_from_slice(&goodix_crc32(&out));
    out
}

pub fn parse_goodix_packet(bytes: &[u8]) -> Result<GoodixPacket, GoodixPacketError> {
    if bytes.len() < GOODIX_PACKAGE_HEADER_SIZE + GOODIX_PACKAGE_CRC_SIZE {
        return Err(GoodixPacketError::ShortPacket);
    }
    let crc8 = goodix_crc8(&bytes[..6]);
    if bytes[6] != crc8 || bytes[7] != !crc8 {
        return Err(GoodixPacketError::BadHeaderCrc);
    }
    let len_with_crc = read_u16_le(bytes, 4) as usize;
    if bytes.len() < GOODIX_PACKAGE_HEADER_SIZE + len_with_crc {
        return Err(GoodixPacketError::ShortPayload);
    }
    let payload_len = len_with_crc
        .checked_sub(GOODIX_PACKAGE_CRC_SIZE)
        .ok_or(GoodixPacketError::BadPayloadLength)?;
    let end = GOODIX_PACKAGE_HEADER_SIZE + payload_len;
    let expected_crc = goodix_crc32(&bytes[..end]);
    let actual_crc: [u8; 4] = bytes[end..end + 4].try_into().unwrap();
    if expected_crc != actual_crc {
        return Err(GoodixPacketError::BadPacketCrc);
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

pub fn parse_goodix_ack(packet: &GoodixPacket) -> Result<GoodixAck, GoodixPacketError> {
    if packet.header.cmd0 != GOODIX_RESPONSE_ACK_CMD || packet.payload.len() < 2 {
        return Err(GoodixPacketError::InvalidAckPacket);
    }
    Ok(GoodixAck {
        result: packet.payload[0],
        ack_cmd: packet.payload[1],
    })
}

pub fn parse_goodix_version_info(payload: &[u8]) -> Result<GoodixVersionInfo, GoodixPayloadError> {
    if payload.len() < 1 + 112 {
        return Err(GoodixPayloadError::ShortVersionPayload);
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

pub fn parse_goodix_template(bytes: &[u8]) -> Result<GoodixTemplate, GoodixPayloadError> {
    if bytes.len() < 68 + 1 + 2 {
        return Err(GoodixPayloadError::ShortTemplatePayload);
    }
    if bytes[0] != 67 {
        return Err(GoodixPayloadError::InvalidTemplateMarker);
    }
    let payload_size = bytes[68] as usize;
    if payload_size > 56 || bytes.len() < 69 + payload_size {
        return Err(GoodixPayloadError::InvalidTemplateDataSize);
    }
    Ok(GoodixTemplate {
        template_type: bytes[1],
        finger_index: bytes[2],
        account_id: bytes[4..36].try_into().unwrap(),
        template_id: bytes[36..68].try_into().unwrap(),
        payload: bytes[69..69 + payload_size].to_vec(),
    })
}

pub fn build_goodix_finger_id(
    template_id: &[u8; 32],
    user_id: &[u8],
) -> Result<Vec<u8>, GoodixPayloadError> {
    if user_id.len() > 100 || user_id.len() > 56 {
        return Err(GoodixPayloadError::UserIdTooLong);
    }
    let total_len = 70 + user_id.len();
    let mut out = vec![0u8; total_len + 2];
    write_u16_le(&mut out, 0, total_len as u16);
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

pub fn parse_goodix_simple_result(
    payload: &[u8],
) -> Result<GoodixSimpleResult, GoodixPayloadError> {
    if payload.is_empty() {
        return Err(GoodixPayloadError::ShortResultPayload);
    }
    Ok(GoodixSimpleResult { result: payload[0] })
}

pub fn parse_goodix_finger_mode_status(
    payload: &[u8],
) -> Result<GoodixFingerModeStatus, GoodixPayloadError> {
    if payload.is_empty() {
        return Err(GoodixPayloadError::ShortFingerModePayload);
    }
    Ok(GoodixFingerModeStatus { status: payload[0] })
}

pub fn parse_goodix_finger_config(
    payload: &[u8],
) -> Result<GoodixFingerConfig, GoodixPayloadError> {
    if payload.is_empty() {
        return Err(GoodixPayloadError::ShortFingerConfigPayload);
    }
    Ok(GoodixFingerConfig {
        status: payload[0],
        max_stored_prints: payload.get(2).copied().unwrap_or(GOODIX_MAX_STORED_PRINTS),
    })
}

pub fn parse_goodix_enroll_init(
    payload: &[u8],
) -> Result<GoodixEnrollInitResult, GoodixPayloadError> {
    if payload.is_empty() {
        return Err(GoodixPayloadError::ShortEnrollInitPayload);
    }
    let result = payload[0];
    let template_id = if result == 0 {
        if payload.len() < 33 {
            return Err(GoodixPayloadError::ShortEnrollInitTemplateId);
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

pub fn parse_goodix_enroll_update(
    payload: &[u8],
) -> Result<GoodixEnrollUpdateResult, GoodixPayloadError> {
    if payload.len() < 3 {
        return Err(GoodixPayloadError::ShortEnrollUpdatePayload);
    }
    Ok(GoodixEnrollUpdateResult {
        rollback: payload[0] >= 0x80,
        overlay: payload[1],
        preoverlay: payload[2],
    })
}

pub fn parse_goodix_duplicate_check(
    payload: &[u8],
) -> Result<GoodixDuplicateCheckResult, GoodixPayloadError> {
    if payload.is_empty() {
        return Err(GoodixPayloadError::ShortDuplicateCheckPayload);
    }
    let duplicate = payload[0] != 0;
    if !duplicate {
        return Ok(GoodixDuplicateCheckResult {
            duplicate: false,
            template: None,
        });
    }
    if payload.len() < 3 {
        return Err(GoodixPayloadError::ShortDuplicateCheckTemplateLength);
    }
    let template_len = le_u16(&payload[1..3])? as usize;
    if payload.len() < 3 + template_len {
        return Err(GoodixPayloadError::ShortDuplicateCheckTemplate);
    }
    let template = parse_goodix_template(&payload[3..3 + template_len])?;
    Ok(GoodixDuplicateCheckResult {
        duplicate: true,
        template: Some(template),
    })
}

pub fn parse_goodix_capture_response(
    payload: &[u8],
) -> Result<GoodixCaptureResponse, GoodixPayloadError> {
    if payload.is_empty() {
        return Err(GoodixPayloadError::ShortCapturePayload);
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

pub fn parse_goodix_identify_result(
    payload: &[u8],
) -> Result<GoodixIdentifyResult, GoodixPayloadError> {
    if payload.is_empty() {
        return Err(GoodixPayloadError::ShortIdentifyPayload);
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
        return Err(GoodixPayloadError::ShortIdentifyMatchPayload);
    }
    let reject_detail = le_u16(&payload[1..3])?;
    let score = read_u32_le(payload, 3);
    let study = payload[7];
    let template_len = le_u16(&payload[8..10])? as usize;
    if payload.len() < 10 + template_len {
        return Err(GoodixPayloadError::ShortIdentifyTemplatePayload);
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

pub fn parse_goodix_finger_list(payload: &[u8]) -> Result<Vec<GoodixTemplate>, GoodixPayloadError> {
    if payload.is_empty() {
        return Err(GoodixPayloadError::ShortFingerListPayload);
    }
    if payload[0] >= 0x80 {
        return Err(GoodixPayloadError::FingerListFailed(payload[0]));
    }
    if payload.len() < 2 {
        return Err(GoodixPayloadError::ShortFingerListCount);
    }
    let count = payload[1] as usize;
    if count > GOODIX_MAX_STORED_PRINTS as usize {
        return Err(GoodixPayloadError::FingerListCountTooLarge);
    }
    let mut offset = 2usize;
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        if payload.len() < offset + 2 {
            return Err(GoodixPayloadError::ShortFingerListEntryLength);
        }
        let entry_len = le_u16(&payload[offset..offset + 2])? as usize;
        offset += 2;
        if payload.len() < offset + entry_len {
            return Err(GoodixPayloadError::ShortFingerListEntry);
        }
        out.push(parse_goodix_template(&payload[offset..offset + entry_len])?);
        offset += entry_len;
    }
    Ok(out)
}

pub fn goodix_crc8(bytes: &[u8]) -> u8 {
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

pub fn goodix_crc32(bytes: &[u8]) -> [u8; 4] {
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

fn le_u16(bytes: &[u8]) -> Result<u16, GoodixPayloadError> {
    try_read_u16_le(bytes, 0).ok_or(GoodixPayloadError::ShortLeU16)
}

fn fixed_c_string(bytes: &[u8]) -> String {
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn packet_roundtrip() {
        let packet = build_goodix_package(0xd0, 0x00, &[0x11, 0x22]);
        let parsed = parse_goodix_packet(&packet).unwrap();
        assert_eq!(parsed.header.cmd0, 0xd0);
        assert_eq!(parsed.payload, vec![0x11, 0x22]);
    }

    #[test]
    fn rejects_bad_crc() {
        let mut packet = build_goodix_package(0xd0, 0x00, &[0x11, 0x22]);
        let last = packet.len() - 1;
        packet[last] ^= 0xff;
        assert_eq!(
            parse_goodix_packet(&packet),
            Err(GoodixPacketError::BadPacketCrc)
        );
    }

    #[test]
    fn parses_ack() {
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
            payload: vec![0, 0xd0],
        };
        let ack = parse_goodix_ack(&packet).unwrap();
        assert_eq!(ack.result, 0);
        assert_eq!(ack.ack_cmd, 0xd0);
    }

    #[test]
    fn parses_template_payload() {
        let mut raw = vec![0u8; 69 + 5];
        raw[0] = 67;
        raw[1] = 2;
        raw[2] = 7;
        raw[68] = 5;
        raw[69..74].copy_from_slice(b"user1");

        let template = parse_goodix_template(&raw).unwrap();
        assert_eq!(template.template_type, 2);
        assert_eq!(template.finger_index, 7);
        assert_eq!(template.payload, b"user1");
    }

    #[test]
    fn parses_identify_match_payload() {
        let mut template = vec![0u8; 69 + 4];
        template[0] = 67;
        template[1] = 1;
        template[2] = 5;
        template[68] = 4;
        template[69..73].copy_from_slice(b"user");

        let mut payload = vec![0, 0x34, 0x12, 88, 0, 0, 0, 1];
        push_u16_le(&mut payload, template.len() as u16);
        payload.extend_from_slice(&template);

        let result = parse_goodix_identify_result(&payload).unwrap();
        assert!(result.matched);
        assert_eq!(result.reject_detail, Some(0x1234));
        assert_eq!(result.score, Some(88));
        assert_eq!(result.template.unwrap().finger_index, 5);
    }

    #[test]
    fn parses_finger_list_payload() {
        let mut template = vec![0u8; 69 + 4];
        template[0] = 67;
        template[2] = 3;
        template[68] = 4;
        template[69..73].copy_from_slice(b"ken1");

        let mut payload = vec![0, 1];
        push_u16_le(&mut payload, template.len() as u16);
        payload.extend_from_slice(&template);

        let templates = parse_goodix_finger_list(&payload).unwrap();
        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].finger_index, 3);
        assert_eq!(templates[0].payload, b"ken1");
    }

    #[test]
    fn builds_finger_id_payload() {
        let tid = [0x42; 32];
        let payload = build_goodix_finger_id(&tid, b"ken").unwrap();
        assert_eq!(read_u16_le(&payload, 0) as usize, 73);
        assert_eq!(payload[2], 67);
        assert_eq!(&payload[38..70], &tid);
        assert_eq!(payload[70], 3);
        assert_eq!(&payload[71..74], b"ken");
    }
}
