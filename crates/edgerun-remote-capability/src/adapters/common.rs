use crate::prelude::v1::*;
use edgerun_capabilities::CapabilityError;
use edgerun_proto::edgerun::v0::capability::{CapabilityInvocation, CapabilityResult};

use crate::protocol::RemoteInvocationResult;

pub(super) fn encode_string_field(value: &str, out: &mut Vec<u8>) {
    edgerun_encoding::string_field::encode_string_field_u32(value, out)
        .expect("string field encode failed");
}

pub(super) fn decode_string_field(
    bytes: &[u8],
    cursor: &mut usize,
) -> Result<String, CapabilityError> {
    edgerun_encoding::string_field::decode_string_field_u32(bytes, cursor)
        .map_err(|_| CapabilityError::InvalidRequest("remote string field decode failed"))
}

pub(super) fn encode_optional_string_field(value: &Option<String>, out: &mut Vec<u8>) {
    edgerun_encoding::string_field::encode_optional_string_field_u32(value.as_deref(), out)
        .expect("optional string field encode failed");
}

pub(super) fn decode_optional_string_field(
    bytes: &[u8],
    cursor: &mut usize,
) -> Result<Option<String>, CapabilityError> {
    edgerun_encoding::string_field::decode_optional_string_field_u32(bytes, cursor)
        .map_err(|_| CapabilityError::InvalidRequest("remote optional string field decode failed"))
}

pub(super) fn encode_string_vec(values: &[String], out: &mut Vec<u8>) {
    edgerun_encoding::string_field::encode_string_vec_u32(values, out)
        .expect("string vector encode failed");
}

pub(super) fn decode_string_vec(
    bytes: &[u8],
    cursor: &mut usize,
) -> Result<Vec<String>, CapabilityError> {
    edgerun_encoding::string_field::decode_string_vec_u32(bytes, cursor)
        .map_err(|_| CapabilityError::InvalidRequest("remote string vector decode failed"))
}

pub(super) fn encode_byte_field(value: &[u8], out: &mut Vec<u8>) {
    edgerun_encoding::string_field::encode_bytes_u32(value, out).expect("byte field encode failed");
}

pub(super) fn decode_byte_field(
    bytes: &[u8],
    cursor: &mut usize,
) -> Result<Vec<u8>, CapabilityError> {
    edgerun_encoding::string_field::decode_bytes_u32(bytes, cursor)
        .map_err(|_| CapabilityError::InvalidRequest("remote byte field decode failed"))
}

pub(super) fn stream_oriented_error(
    invocation: &CapabilityInvocation,
    label: &str,
) -> RemoteInvocationResult {
    RemoteInvocationResult {
        result: CapabilityResult {
            result_version: 1,
            invocation_id: invocation.invocation_id.clone(),
            grant_id: invocation.grant_id.clone(),
            success: false,
            result_access_class: invocation.requested_access_class,
            produced_event_kinds: Vec::new(),
            payload_object: None,
            error_reason: format!("{label} remote adapter is stream-oriented; use session events"),
            produced_at: None,
            signature: None,
        },
        inline_payload: Vec::new(),
    }
}
