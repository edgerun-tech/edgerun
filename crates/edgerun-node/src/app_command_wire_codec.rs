//! Wire-only app command payload codec.

use alloc::format;
use alloc::vec::Vec;

use edgerun_proto::edgerun::v0::common::ObjectRef;
use edgerun_wire::{WireDecode, WireReader, WireValue};

#[derive(Clone, Debug)]
pub struct InstallAppCommandPayload {
    pub app_package: ObjectRef,
    pub domain: String,
}

#[derive(Clone, Debug)]
pub struct UninstallAppCommandPayload {
    pub app_id: Vec<u8>,
}

use alloc::string::String;

pub fn decode_install_app_payload(bytes: &[u8]) -> Result<InstallAppCommandPayload, String> {
    let fields = decode_top_struct(bytes)?;
    let app_package = required_bytes(&fields, 2, "app_package")?;
    Ok(InstallAppCommandPayload {
        app_package: decode_object_ref(app_package)?,
        domain: optional_text(&fields, 3)?.unwrap_or_default(),
    })
}

pub fn decode_uninstall_app_payload(bytes: &[u8]) -> Result<UninstallAppCommandPayload, String> {
    let fields = decode_top_struct(bytes)?;
    let app_id = required_bytes(&fields, 3, "app_id")?;
    Ok(UninstallAppCommandPayload { app_id })
}

fn decode_object_ref(bytes: Vec<u8>) -> Result<ObjectRef, String> {
    let fields = decode_embedded_struct(&bytes)?;
    Ok(ObjectRef {
        object_id: required_bytes(&fields, 1, "object_id")?,
        object_kind: field_u32(&fields, 2)?.map(|v| v as i32),
    })
}

fn decode_top_struct(bytes: &[u8]) -> Result<Vec<edgerun_wire::WireField>, String> {
    match WireValue::from_wire_bytes(bytes).map_err(|e| format!("wire_decode_failed: {e:?}"))? {
        WireValue::Struct(fields) => Ok(fields),
        other => Err(format!("expected_struct_got_{other:?}")),
    }
}

fn decode_embedded_struct(bytes: &[u8]) -> Result<Vec<edgerun_wire::WireField>, String> {
    let mut reader = WireReader::new(bytes);
    let value = WireValue::decode_wire(&mut reader).map_err(|e| format!("embedded_wire_decode_failed: {e:?}"))?;
    if !reader.is_empty() { return Err("embedded_wire_trailing_bytes".into()); }
    match value {
        WireValue::Struct(fields) => Ok(fields),
        other => Err(format!("expected_embedded_struct_got_{other:?}")),
    }
}

fn find_field(fields: &[edgerun_wire::WireField], tag: u32) -> Option<&WireValue> {
    fields.iter().find(|field| field.tag == tag).map(|field| &field.value)
}

fn required_bytes(fields: &[edgerun_wire::WireField], tag: u32, name: &str) -> Result<Vec<u8>, String> {
    match find_field(fields, tag) {
        Some(WireValue::Bytes(value)) => Ok(value.clone()),
        Some(other) => Err(format!("{name}_expected_bytes_got_{other:?}")),
        None => Err(format!("{name}_required")),
    }
}

fn optional_text(fields: &[edgerun_wire::WireField], tag: u32) -> Result<Option<String>, String> {
    match find_field(fields, tag) {
        None => Ok(None),
        Some(WireValue::Text(value)) => Ok(Some(value.clone())),
        Some(other) => Err(format!("field_{tag}_expected_text_got_{other:?}")),
    }
}

fn field_u32(fields: &[edgerun_wire::WireField], tag: u32) -> Result<Option<u32>, String> {
    match find_field(fields, tag) {
        None => Ok(None),
        Some(WireValue::U64(value)) => u32::try_from(*value).map(Some).map_err(|_| format!("field_{tag}_u32_overflow")),
        Some(other) => Err(format!("field_{tag}_expected_u64_got_{other:?}")),
    }
}
