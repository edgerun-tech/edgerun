//! Wire-only AppPackage reader for install-time object inspection.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use edgerun_proto::edgerun::v0::common::ObjectRef;
use edgerun_wire::{WireDecode, WireReader, WireValue};

#[derive(Clone, Debug, Default)]
pub struct AppPackageInfo {
    pub wasm_object: Option<ObjectRef>,
    pub assets: Vec<(String, ObjectRef)>,
}

pub fn decode_app_package(bytes: &[u8]) -> Result<AppPackageInfo, String> {
    let fields = decode_top_struct(bytes)?;
    Ok(AppPackageInfo {
        wasm_object: field_bytes(&fields, 4)?.map(decode_object_ref).transpose()?,
        assets: field_asset_list(&fields, 6)?.unwrap_or_default(),
    })
}

fn field_asset_list(fields: &[edgerun_wire::WireField], tag: u32) -> Result<Option<Vec<(String, ObjectRef)>>, String> {
    let Some(value) = find_field(fields, tag) else { return Ok(None); };
    let WireValue::List(items) = value else { return Err("assets_expected_list".into()); };
    let mut out = Vec::new();
    for item in items {
        let WireValue::Bytes(bytes) = item else { return Err("asset_entry_expected_bytes".into()); };
        let fields = decode_embedded_struct(bytes)?;
        let name = required_text(&fields, 1, "asset.name")?;
        let object = required_bytes(&fields, 2, "asset.object")?;
        out.push((name, decode_object_ref(object)?));
    }
    Ok(Some(out))
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

fn field_bytes(fields: &[edgerun_wire::WireField], tag: u32) -> Result<Option<Vec<u8>>, String> {
    match find_field(fields, tag) {
        None => Ok(None),
        Some(WireValue::Bytes(value)) => Ok(Some(value.clone())),
        Some(other) => Err(format!("field_{tag}_expected_bytes_got_{other:?}")),
    }
}

fn required_bytes(fields: &[edgerun_wire::WireField], tag: u32, name: &str) -> Result<Vec<u8>, String> {
    field_bytes(fields, tag)?.ok_or_else(|| format!("{name}_required"))
}

fn required_text(fields: &[edgerun_wire::WireField], tag: u32, name: &str) -> Result<String, String> {
    match find_field(fields, tag) {
        Some(WireValue::Text(value)) => Ok(value.clone()),
        Some(other) => Err(format!("{name}_expected_text_got_{other:?}")),
        None => Err(format!("{name}_required")),
    }
}

fn field_u32(fields: &[edgerun_wire::WireField], tag: u32) -> Result<Option<u32>, String> {
    match find_field(fields, tag) {
        None => Ok(None),
        Some(WireValue::U64(value)) => u32::try_from(*value).map(Some).map_err(|_| format!("field_{tag}_u32_overflow")),
        Some(other) => Err(format!("field_{tag}_expected_u64_got_{other:?}")),
    }
}
