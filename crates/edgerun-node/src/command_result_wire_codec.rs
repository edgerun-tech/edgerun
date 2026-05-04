//! Wire-only command-result payload codec.
//!
//! Command-result payload bytes are deterministic edgerun-wire bytes. Protobuf
//! structs are boundary structs only.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use edgerun_core::protocol::{CommandRef, Digest, IdentityRef, ObjectRef};
use edgerun_core::protocol::CommandResultPayload;
use edgerun_wire::{WireDecode, WireReader, WireValue};

#[must_use]
pub fn encode_command_result_payload(payload: &CommandResultPayload) -> Vec<u8> {
    edgerun_core::wire_command::command_result_bytes(payload)
}

pub fn decode_command_result_payload(bytes: &[u8]) -> Result<CommandResultPayload, String> {
    let value =
        WireValue::from_wire_bytes(bytes).map_err(|e| format!("wire_decode_failed: {e:?}"))?;
    let fields = expect_struct(value)?;
    Ok(CommandResultPayload {
        payload_version: field_u32(&fields, 1)?.unwrap_or(0),
        command: field_bytes(&fields, 2)?
            .map(decode_command_ref)
            .transpose()?,
        issuer: field_bytes(&fields, 3)?
            .map(decode_identity_ref)
            .transpose()?,
        decision: field_u32(&fields, 4)?.unwrap_or(0) as i32,
        decision_basis: field_bytes(&fields, 5)?
            .map(decode_object_ref)
            .transpose()?,
        reason_code: field_text(&fields, 6)?.unwrap_or_default(),
        effect_summary_object: field_bytes(&fields, 7)?
            .map(decode_object_ref)
            .transpose()?,
        result_object: field_bytes(&fields, 8)?
            .map(decode_object_ref)
            .transpose()?,
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
    let value = WireValue::decode_wire(&mut reader)
        .map_err(|e| format!("embedded_wire_decode_failed: {e:?}"))?;
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
    fields
        .iter()
        .find(|field| field.tag == tag)
        .map(|field| &field.value)
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

fn field_raw_bytes(
    fields: &[edgerun_wire::WireField],
    tag: u32,
) -> Result<Option<Vec<u8>>, String> {
    field_bytes(fields, tag)
}
