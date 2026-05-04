//! Command-result payload codec boundary.
//!
//! Node dispatch still receives protobuf boundary structs, but committed command
//! result payload bytes should be deterministic edgerun-wire bytes. Keep this
//! helper small so command dispatch does not call prost encoding directly.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use edgerun_proto::edgerun::v0::common::{CommandRef, Digest, IdentityRef, ObjectRef};
use edgerun_proto::edgerun::v0::stream::CommandResultPayload;
use edgerun_wire::{WireDecode, WireReader, WireValue};
use prost::Message;

#[must_use]
pub fn encode_command_result_payload(payload: &CommandResultPayload) -> Vec<u8> {
    edgerun_core::wire_command::command_result_bytes(payload)
}

pub fn decode_command_result_payload(bytes: &[u8]) -> Result<CommandResultPayload, String> {
    decode_wire_command_result_payload(bytes).or_else(|wire_err| {
        CommandResultPayload::decode(bytes)
            .map_err(|prost_err| format!("command_result_decode_failed: wire={wire_err}; prost={prost_err}"))
    })
}

fn decode_wire_command_result_payload(bytes: &[u8]) -> Result<CommandResultPayload, String> {
    let value = WireValue::from_wire_bytes(bytes).map_err(|e| format!("wire_decode_failed: {e:?}"))?;
    let fields = expect_struct(value)?;
    Ok(CommandResultPayload {
        payload_version: field_u32(&fields, 1)?.unwrap_or(0),
        command: field_bytes(&fields, 2)?.map(decode_command_ref).transpose()?,
        issuer: field_bytes(&fields, 3)?.map(decode_identity_ref).transpose()?,
        decision: field_u32(&fields, 4)?.unwrap_or(0) as i32,
        decision_basis: field_bytes(&fields, 5)?.map(decode_object_ref).transpose()?,
        reason_code: field_text(&fields, 6)?.unwrap_or_default(),
        effect_summary_object: field_bytes(&fields, 7)?.map(decode_object_ref).transpose()?,
        result_object: field_bytes(&fields, 8)?.map(decode_object_ref).transpose()?,
    })
}

fn decode_command_ref(bytes: Vec<u8>) -> Result<CommandRef, String> {
    let fields = decode_embedded_struct(&bytes)?;
    Ok(CommandRef {
        command_id: field_raw_bytes(&fields, 1)?.unwrap_or_default(),
        command_hash: field_bytes(&fields, 2)?.map(decode_digest).transpose()?,
    })
}

fn decode_identity_ref(bytes: Vec<u8>) -> Result<IdentityRef, String> {
    let fields = decode_embedded_struct(&bytes)?;
    Ok(IdentityRef {
        identity_id: field_raw_bytes(&fields, 1)?.unwrap_or_default(),
        identity_kind: field_u32(&fields, 2)?.map(|v| v as i32),
        key_hint: field_raw_bytes(&fields, 3)?,
    })
}

fn decode_object_ref(bytes: Vec<u8>) -> Result<ObjectRef, String> {
    let fields = decode_embedded_struct(&bytes)?;
    Ok(ObjectRef {
        object_id: field_raw_bytes(&fields, 1)?.unwrap_or_default(),
        object_kind: field_u32(&fields, 2)?.map(|v| v as i32),
    })
}

fn decode_digest(bytes: Vec<u8>) -> Result<Digest, String> {
    let fields = decode_embedded_struct(&bytes)?;
    Ok(Digest {
        algorithm: field_u32(&fields, 1)?.unwrap_or(0) as i32,
        value: field_raw_bytes(&fields, 2)?.unwrap_or_default(),
    })
}

fn decode_embedded_struct(bytes: &[u8]) -> Result<Vec<edgerun_wire::WireField>, String> {
    let mut reader = WireReader::new(bytes);
    let value = WireValue::decode_wire(&mut reader).map_err(|e| format!("embedded_wire_decode_failed: {e:?}"))?;
    if !reader.is_empty() {
        return Err("embedded_wire_trailing_bytes".into());
    }
    expect_struct(value)
}

fn expect_struct(value: WireValue) -> Result<Vec<edgerun_wire::WireField>, String> {
    match value {
        WireValue::Struct(fields) => Ok(fields),
        other => Err(format!("expected_struct_got_{other:?}")),
    }
}

fn find_field(fields: &[edgerun_wire::WireField], tag: u32) -> Option<&WireValue> {
    fields.iter().find(|field| field.tag == tag).map(|field| &field.value)
}

fn field_u32(fields: &[edgerun_wire::WireField], tag: u32) -> Result<Option<u32>, String> {
    match find_field(fields, tag) {
        None => Ok(None),
        Some(WireValue::U64(value)) => u32::try_from(*value)
            .map(Some)
            .map_err(|_| format!("field_{tag}_u32_overflow")),
        Some(other) => Err(format!("field_{tag}_expected_u64_got_{other:?}")),
    }
}

fn field_text(fields: &[edgerun_wire::WireField], tag: u32) -> Result<Option<String>, String> {
    match find_field(fields, tag) {
        None => Ok(None),
        Some(WireValue::Text(value)) => Ok(Some(value.clone())),
        Some(other) => Err(format!("field_{tag}_expected_text_got_{other:?}")),
    }
}

fn field_bytes(fields: &[edgerun_wire::WireField], tag: u32) -> Result<Option<Vec<u8>>, String> {
    match find_field(fields, tag) {
        None => Ok(None),
        Some(WireValue::Bytes(value)) => Ok(Some(value.clone())),
        Some(other) => Err(format!("field_{tag}_expected_bytes_got_{other:?}")),
    }
}

fn field_raw_bytes(fields: &[edgerun_wire::WireField], tag: u32) -> Result<Option<Vec<u8>>, String> {
    field_bytes(fields, tag)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_result_wire_round_trips() {
        let payload = CommandResultPayload {
            payload_version: 1,
            command: Some(CommandRef {
                command_id: b"cmd".to_vec(),
                command_hash: Some(Digest { algorithm: 1, value: vec![1; 32] }),
            }),
            issuer: Some(IdentityRef {
                identity_id: b"issuer".to_vec(),
                identity_kind: Some(2),
                key_hint: Some(vec![9; 4]),
            }),
            decision: 1,
            decision_basis: None,
            reason_code: "ok".into(),
            effect_summary_object: None,
            result_object: Some(ObjectRef { object_id: b"obj".to_vec(), object_kind: Some(6) }),
        };
        let encoded = encode_command_result_payload(&payload);
        let decoded = decode_command_result_payload(&encoded).unwrap();
        assert_eq!(decoded.payload_version, payload.payload_version);
        assert_eq!(decoded.command.unwrap().command_id, b"cmd".to_vec());
        assert_eq!(decoded.issuer.unwrap().identity_id, b"issuer".to_vec());
        assert_eq!(decoded.reason_code, "ok");
        assert_eq!(decoded.result_object.unwrap().object_id, b"obj".to_vec());
    }

    #[test]
    fn command_result_protobuf_fallback_decodes() {
        let payload = CommandResultPayload {
            payload_version: 1,
            command: None,
            issuer: None,
            decision: 2,
            decision_basis: None,
            reason_code: "legacy".into(),
            effect_summary_object: None,
            result_object: None,
        };
        let encoded = payload.encode_to_vec();
        let decoded = decode_command_result_payload(&encoded).unwrap();
        assert_eq!(decoded.reason_code, "legacy");
    }
}
