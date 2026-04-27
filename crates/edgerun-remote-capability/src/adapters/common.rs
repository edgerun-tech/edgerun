use crate::prelude::v1::*;
use edgerun_capabilities::CapabilityError;

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
