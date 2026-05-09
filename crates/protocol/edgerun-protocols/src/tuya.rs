//! Tuya local protocol message model.
//!
//! This module owns Tuya message payloads only. It does not open UDP/TCP
//! sockets, sleep, scan networks, or decide where packets are sent.

use crate::prelude::*;
use alloc::format;
use core::str;
use edgerun_crypto::{AeadInPlace, Aes256GcmCipher, Nonce, Tag};
use edgerun_encoding::byteorder::{push_u16_be, push_u32_be, read_u32_be};
use edgerun_encoding::crc32;
use edgerun_json::{FromJson, JsonValue, JsonValueError, Map, ToJson};

pub const TUYA_BROADCAST_ADDR: &str = "255.255.255.255";
pub const TUYA_DISCOVERY_PORT: u16 = 6667;
pub const TUYA_CONTROL_PORT: u16 = 6668;
pub const PROTOCOL_VERSION: &str = "3.3";
pub const PREFIX_55AA: u32 = 0x0000_55aa;
pub const SUFFIX_55AA: u32 = 0x0000_aa55;
pub const PREFIX_6699: u32 = 0x0000_6699;
pub const SUFFIX_6699: u32 = 0x0000_9966;
pub const SESS_KEY_NEG_START: u32 = 0x03;
pub const SESS_KEY_NEG_RESP: u32 = 0x04;
pub const SESS_KEY_NEG_FINISH: u32 = 0x05;
pub const DP_QUERY_NEW: u32 = 0x10;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TuyaProtocolError {
    InvalidKey,
    InvalidFrame,
    InvalidPayload,
    AuthenticationFailed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TuyaWireMessage {
    pub prefix: u32,
    pub cmd: u32,
    pub payload: Vec<u8>,
    pub raw_header: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TuyaDevice {
    pub id: String,
    pub key: Option<String>,
    pub ip: String,
    pub name: Option<String>,
    pub product_type: Option<String>,
    pub version: Option<String>,
    pub state: TuyaDeviceState,
}

impl ToJson for TuyaDevice {
    fn to_json(&self) -> JsonValue {
        let mut object = Map::new();
        object.push_field("id", &self.id);
        object.push_field("ip", &self.ip);
        object.push_opt_field("name", self.name.as_ref());
        object.push_opt_field("product_type", self.product_type.as_ref());
        object.push_opt_field("version", self.version.as_ref());
        object.push_field("state", self.state.to_json());
        object.into()
    }
}

impl FromJson for TuyaDevice {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = match value {
            JsonValue::Object(object) => object,
            _ => {
                return Err(JsonValueError::WrongType(format!(
                    "expected tuya device object"
                )));
            }
        };
        Ok(Self {
            id: FromJson::from_json(
                object
                    .remove("id")
                    .ok_or_else(|| JsonValueError::WrongType(format!("missing field `id`")))?,
            )?,
            key: None,
            ip: FromJson::from_json(
                object
                    .remove("ip")
                    .ok_or_else(|| JsonValueError::WrongType(format!("missing field `ip`")))?,
            )?,
            name: object.remove("name").map(FromJson::from_json).transpose()?,
            product_type: object
                .remove("product_type")
                .map(FromJson::from_json)
                .transpose()?,
            version: object
                .remove("version")
                .map(FromJson::from_json)
                .transpose()?,
            state: FromJson::from_json(
                object
                    .remove("state")
                    .ok_or_else(|| JsonValueError::WrongType(format!("missing field `state`")))?,
            )?,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct TuyaDeviceState {
    pub on: Option<bool>,
    pub temperature: Option<i32>,
    pub mode: Option<String>,
    pub fan_speed: Option<String>,
    pub swing: Option<String>,
}

edgerun_json::impl_json_struct! {
    TuyaDeviceState {
        required {}
        optional {
            on: "on" => bool,
            temperature: "temperature" => i32,
            mode: "mode" => String,
            fan_speed: "fan_speed" => String,
            swing: "swing" => String,
        }
    }
}

#[derive(Clone, Debug)]
pub enum TuyaCommand {
    Discovery {
        protocol_version: String,
    },
    Control {
        devId: String,
        dps: edgerun_json::Value,
    },
}

impl ToJson for TuyaCommand {
    fn to_json(&self) -> JsonValue {
        let mut object = Map::new();
        match self {
            Self::Discovery { protocol_version } => {
                object.push_field("action", "discovery");
                object.push_field("protocol_version", protocol_version);
            }
            Self::Control { devId, dps } => {
                object.push_field("action", "control");
                object.push_field("devId", devId);
                object.push_field("dps", dps.clone());
            }
        }
        object.into()
    }
}

#[derive(Clone, Debug)]
pub enum TuyaResponse {
    Discovery {
        msg_id: String,
        devId: String,
        product_type: String,
        version: String,
        ability: Option<edgerun_json::Value>,
    },
    Control {
        devId: String,
        dps: edgerun_json::Value,
    },
}

impl FromJson for TuyaResponse {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = match value {
            JsonValue::Object(object) => object,
            _ => {
                return Err(JsonValueError::WrongType(format!(
                    "expected tuya response object"
                )));
            }
        };
        let action = String::from_json(
            object
                .remove("action")
                .ok_or_else(|| JsonValueError::WrongType(format!("missing field `action`")))?,
        )?;
        match action.as_str() {
            "discovery" => {
                Ok(Self::Discovery {
                    msg_id: FromJson::from_json(object.remove("msg_id").ok_or_else(|| {
                        JsonValueError::WrongType(format!("missing field `msg_id`"))
                    })?)?,
                    devId: FromJson::from_json(object.remove("devId").ok_or_else(|| {
                        JsonValueError::WrongType(format!("missing field `devId`"))
                    })?)?,
                    product_type: FromJson::from_json(object.remove("product_type").ok_or_else(
                        || JsonValueError::WrongType(format!("missing field `product_type`")),
                    )?)?,
                    version: FromJson::from_json(object.remove("version").ok_or_else(|| {
                        JsonValueError::WrongType(format!("missing field `version`"))
                    })?)?,
                    ability: object
                        .remove("ability")
                        .map(FromJson::from_json)
                        .transpose()?,
                })
            }
            "control" => {
                Ok(Self::Control {
                    devId: FromJson::from_json(object.remove("devId").ok_or_else(|| {
                        JsonValueError::WrongType(format!("missing field `devId`"))
                    })?)?,
                    dps: FromJson::from_json(object.remove("dps").ok_or_else(|| {
                        JsonValueError::WrongType(format!("missing field `dps`"))
                    })?)?,
                })
            }
            other => Err(JsonValueError::WrongType(format!(
                "unknown tuya response action `{other}`"
            ))),
        }
    }
}

pub fn discovery_request_bytes() -> Result<Vec<u8>, edgerun_json::JsonError> {
    TuyaCommand::Discovery {
        protocol_version: PROTOCOL_VERSION.to_string(),
    }
    .to_json()
    .to_json_string()
    .map(|body| body.into_bytes())
}

pub fn control_request_bytes(
    device_id: &str,
    dps: edgerun_json::Value,
) -> Result<Vec<u8>, edgerun_json::JsonError> {
    TuyaCommand::Control {
        devId: device_id.to_string(),
        dps,
    }
    .to_json()
    .to_json_string()
    .map(|body| body.into_bytes())
}

pub fn parse_response_bytes(bytes: &[u8]) -> Result<TuyaResponse, JsonValueError> {
    let text = str::from_utf8(bytes)
        .map_err(|_| JsonValueError::WrongType(format!("tuya response is not utf-8")))?;
    edgerun_json::from_json_str(text)
}

pub fn pack_55aa(seq: u32, cmd: u32, payload: &[u8]) -> Vec<u8> {
    let len = payload.len() as u32 + 8;
    let mut frame = Vec::with_capacity(16 + payload.len() + 8);
    push_u32_be(&mut frame, PREFIX_55AA);
    push_u32_be(&mut frame, seq);
    push_u32_be(&mut frame, cmd);
    push_u32_be(&mut frame, len);
    frame.extend_from_slice(payload);
    let checksum = crc32(&frame);
    push_u32_be(&mut frame, checksum);
    push_u32_be(&mut frame, SUFFIX_55AA);
    frame
}

pub fn pack_6699_with_iv(
    seq: u32,
    cmd: u32,
    payload: &[u8],
    key: &[u8],
    iv: &[u8; 12],
) -> Result<Vec<u8>, TuyaProtocolError> {
    let mut encrypted = payload.to_vec();
    let len = (iv.len() + encrypted.len() + 16) as u32;

    let mut frame = Vec::with_capacity(18 + len as usize + 4);
    push_u32_be(&mut frame, PREFIX_6699);
    push_u16_be(&mut frame, 0);
    push_u32_be(&mut frame, seq);
    push_u32_be(&mut frame, cmd);
    push_u32_be(&mut frame, len);

    let cipher = Aes256GcmCipher::new(key).map_err(|_| TuyaProtocolError::InvalidKey)?;
    let nonce = Nonce::from(*iv);
    let tag = cipher
        .encrypt_in_place_detached(&nonce, &frame[4..], &mut encrypted)
        .map_err(|_| TuyaProtocolError::AuthenticationFailed)?;

    frame.extend_from_slice(iv);
    frame.extend_from_slice(&encrypted);
    frame.extend_from_slice(tag.as_slice());
    push_u32_be(&mut frame, SUFFIX_6699);
    Ok(frame)
}

pub fn decrypt_6699_payload(
    header: &[u8],
    payload: &[u8],
    session_key: &[u8],
) -> Result<Vec<u8>, TuyaProtocolError> {
    if payload.len() < 28 || header.len() < 4 {
        return Err(TuyaProtocolError::InvalidPayload);
    }
    let iv = &payload[..12];
    let tag = &payload[payload.len() - 16..];
    let mut ciphertext = payload[12..payload.len() - 16].to_vec();
    let cipher = Aes256GcmCipher::new(session_key).map_err(|_| TuyaProtocolError::InvalidKey)?;
    let nonce = Nonce::from_slice(iv);
    let tag = Tag::from_slice(tag);
    cipher
        .decrypt_in_place_detached(&nonce, &header[4..], &mut ciphertext, &tag)
        .map_err(|_| TuyaProtocolError::AuthenticationFailed)?;
    Ok(ciphertext)
}

pub fn parse_55aa_body_len(header_rest: &[u8]) -> Result<usize, TuyaProtocolError> {
    if header_rest.len() != 12 {
        return Err(TuyaProtocolError::InvalidFrame);
    }
    let len = read_u32_be(header_rest, 8) as usize;
    if !(8..=4096).contains(&len) {
        return Err(TuyaProtocolError::InvalidFrame);
    }
    Ok(len)
}

pub fn parse_6699_body_len(header_rest: &[u8]) -> Result<usize, TuyaProtocolError> {
    if header_rest.len() != 14 {
        return Err(TuyaProtocolError::InvalidFrame);
    }
    let len = read_u32_be(header_rest, 10) as usize;
    if !(28..=4096).contains(&len) {
        return Err(TuyaProtocolError::InvalidFrame);
    }
    Ok(len)
}

pub fn parse_55aa_wire_message(
    prefix: [u8; 4],
    header_rest: &[u8],
    body: &[u8],
) -> Result<TuyaWireMessage, TuyaProtocolError> {
    if read_u32_be(&prefix, 0) != PREFIX_55AA {
        return Err(TuyaProtocolError::InvalidFrame);
    }
    let len = parse_55aa_body_len(header_rest)?;
    if body.len() != len || len < 8 {
        return Err(TuyaProtocolError::InvalidFrame);
    }
    let suffix = read_u32_be(body, len - 4);
    if suffix != SUFFIX_55AA {
        return Err(TuyaProtocolError::InvalidFrame);
    }
    let cmd = read_u32_be(header_rest, 4);
    let mut raw_header = Vec::with_capacity(16);
    raw_header.extend_from_slice(&prefix);
    raw_header.extend_from_slice(header_rest);
    Ok(TuyaWireMessage {
        prefix: PREFIX_55AA,
        cmd,
        payload: body[..len - 8].to_vec(),
        raw_header,
    })
}

pub fn parse_6699_wire_message(
    prefix: [u8; 4],
    header_rest: &[u8],
    body: &[u8],
) -> Result<TuyaWireMessage, TuyaProtocolError> {
    if read_u32_be(&prefix, 0) != PREFIX_6699 {
        return Err(TuyaProtocolError::InvalidFrame);
    }
    let len = parse_6699_body_len(header_rest)?;
    if body.len() != len + 4 {
        return Err(TuyaProtocolError::InvalidFrame);
    }
    let suffix = read_u32_be(body, len);
    if suffix != SUFFIX_6699 {
        return Err(TuyaProtocolError::InvalidFrame);
    }
    let cmd = read_u32_be(header_rest, 6);
    let mut raw_header = Vec::with_capacity(18);
    raw_header.extend_from_slice(&prefix);
    raw_header.extend_from_slice(header_rest);
    Ok(TuyaWireMessage {
        prefix: PREFIX_6699,
        cmd,
        payload: body[..len].to_vec(),
        raw_header,
    })
}

pub fn derive_v35_session_key(
    local_key: &[u8],
    local_nonce: &[u8; 16],
    remote_nonce: &[u8],
) -> Result<[u8; 16], TuyaProtocolError> {
    if remote_nonce.len() < 16 {
        return Err(TuyaProtocolError::InvalidPayload);
    }
    let mut xored = [0u8; 16];
    for i in 0..16 {
        xored[i] = local_nonce[i] ^ remote_nonce[i];
    }

    let cipher = Aes256GcmCipher::new(local_key).map_err(|_| TuyaProtocolError::InvalidKey)?;
    let nonce = Nonce::from_slice(&local_nonce[..12]);
    let _tag = cipher
        .encrypt_in_place_detached(&nonce, &[], &mut xored)
        .map_err(|_| TuyaProtocolError::AuthenticationFailed)?;
    Ok(xored)
}

pub fn strip_retcode(payload: &[u8]) -> &[u8] {
    if payload.len() >= 4 && payload[..4] == [0, 0, 0, 0] {
        &payload[4..]
    } else {
        payload
    }
}

pub fn decode_json_payload(payload: &[u8]) -> Result<String, TuyaProtocolError> {
    let mut start = 0usize;
    if payload.len() >= 5 && payload[4] == b'{' {
        start = 4;
    }
    let payload = &payload[start..];
    let json_start = payload
        .iter()
        .position(|b| *b == b'{')
        .ok_or(TuyaProtocolError::InvalidPayload)?;
    let json_end = payload
        .iter()
        .rposition(|b| *b == b'}')
        .ok_or(TuyaProtocolError::InvalidPayload)?;
    String::from_utf8(payload[json_start..=json_end].to_vec())
        .map_err(|_| TuyaProtocolError::InvalidPayload)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovery_request_is_protocol_payload_only() {
        let bytes = discovery_request_bytes().unwrap();
        let text = core::str::from_utf8(&bytes).unwrap();

        assert!(text.contains("\"action\":\"discovery\""));
        assert!(text.contains("\"protocol_version\":\"3.3\""));
    }

    #[test]
    fn device_state_defaults_empty() {
        let state = TuyaDeviceState::default();
        assert_eq!(state.on, None);
        assert_eq!(state.temperature, None);
    }

    #[test]
    fn pack_55aa_has_expected_prefix_and_suffix() {
        let frame = pack_55aa(1, DP_QUERY_NEW, b"{}");
        assert_eq!(&frame[..4], &PREFIX_55AA.to_be_bytes());
        assert_eq!(&frame[frame.len() - 4..], &SUFFIX_55AA.to_be_bytes());
    }

    #[test]
    fn parses_55aa_wire_message() {
        let frame = pack_55aa(1, DP_QUERY_NEW, b"{}");
        let message =
            parse_55aa_wire_message(PREFIX_55AA.to_be_bytes(), &frame[4..16], &frame[16..])
                .unwrap();
        assert_eq!(message.prefix, PREFIX_55AA);
        assert_eq!(message.cmd, DP_QUERY_NEW);
        assert_eq!(message.payload, b"{}");
    }

    #[test]
    fn rejects_bad_55aa_suffix() {
        let mut frame = pack_55aa(1, DP_QUERY_NEW, b"{}");
        let last = frame.len() - 1;
        frame[last] ^= 1;
        assert_eq!(
            parse_55aa_wire_message(PREFIX_55AA.to_be_bytes(), &frame[4..16], &frame[16..]),
            Err(TuyaProtocolError::InvalidFrame)
        );
    }
}
