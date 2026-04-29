//! Display device remote adapter.

use crate::prelude::v1::*;
use edgerun_capabilities::{
    CapabilityDescriptor, CapabilityError, CapabilityEventKind, CapabilityOperation,
};
use edgerun_display::{
    validate_display_update_request, DisplayContentKind, DisplayDevice, DisplayInfo, DisplayMode,
    DisplayUpdateRequest,
};
use edgerun_encoding::byteorder::read_u32_le;
use edgerun_proto::edgerun::v0::capability::{CapabilityInvocation, CapabilityResult};

use crate::adapters::common::{decode_count_u32, decode_string_field, encode_string_field};
use crate::protocol::{RemoteCapabilityProvider, RemoteInvocationResult};

fn content_kind_to_u32(kind: DisplayContentKind) -> u32 {
    match kind {
        DisplayContentKind::Text => 1,
        DisplayContentKind::Image => 2,
        DisplayContentKind::Video => 3,
        DisplayContentKind::Prompt => 4,
        DisplayContentKind::Other(v) => v | 0x8000_0000,
    }
}

fn content_kind_from_u32(raw: u32) -> DisplayContentKind {
    match raw {
        1 => DisplayContentKind::Text,
        2 => DisplayContentKind::Image,
        3 => DisplayContentKind::Video,
        4 => DisplayContentKind::Prompt,
        other if other & 0x8000_0000 != 0 => DisplayContentKind::Other(other & 0x7fff_ffff),
        other => DisplayContentKind::Other(other),
    }
}

fn encode_display_mode(mode: DisplayMode, out: &mut Vec<u8>) {
    out.extend_from_slice(&mode.width.to_le_bytes());
    out.extend_from_slice(&mode.height.to_le_bytes());
    out.extend_from_slice(&mode.refresh_millihz.to_le_bytes());
}

fn decode_display_mode(bytes: &[u8], cursor: &mut usize) -> Result<DisplayMode, CapabilityError> {
    if bytes.len().saturating_sub(*cursor) < 12 {
        return Err(CapabilityError::InvalidRequest(
            "remote display mode payload too short",
        ));
    }
    let mode = DisplayMode {
        width: read_u32_le(bytes, *cursor),
        height: read_u32_le(bytes, *cursor + 4),
        refresh_millihz: read_u32_le(bytes, *cursor + 8),
    };
    *cursor += 12;
    Ok(mode)
}

/// Binary-encode display information for remote transport.
pub fn encode_display_info(info: &DisplayInfo) -> Vec<u8> {
    let mut out = Vec::new();
    encode_string_field(&info.provider, &mut out);
    encode_string_field(&info.display_name, &mut out);
    encode_string_field(&info.instance_id, &mut out);
    out.push(info.built_in as u8);
    out.push(info.primary as u8);
    out.push(info.hdr_capable as u8);
    out.push(info.touch_capable as u8);
    encode_display_mode(info.current_mode, &mut out);
    out.extend_from_slice(&(info.modes.len() as u32).to_le_bytes());
    for mode in &info.modes {
        encode_display_mode(*mode, &mut out);
    }
    out
}

/// Binary-decode display information from remote transport.
pub fn decode_display_info(bytes: &[u8]) -> Result<DisplayInfo, CapabilityError> {
    let mut cursor = 0usize;
    let provider = decode_string_field(bytes, &mut cursor)?;
    let display_name = decode_string_field(bytes, &mut cursor)?;
    let instance_id = decode_string_field(bytes, &mut cursor)?;
    if bytes.len().saturating_sub(cursor) < 4 {
        return Err(CapabilityError::InvalidRequest(
            "remote display info flags payload too short",
        ));
    }
    let built_in = read_bool(
        bytes,
        &mut cursor,
        "remote display built-in flag is invalid",
    )?;
    let primary = read_bool(bytes, &mut cursor, "remote display primary flag is invalid")?;
    let hdr_capable = read_bool(bytes, &mut cursor, "remote display HDR flag is invalid")?;
    let touch_capable = read_bool(bytes, &mut cursor, "remote display touch flag is invalid")?;
    let current_mode = decode_display_mode(bytes, &mut cursor)?;
    let mode_count = decode_count_u32(bytes, &mut cursor, "remote display mode count missing")?;
    let mut modes = Vec::with_capacity(mode_count);
    for _ in 0..mode_count {
        modes.push(decode_display_mode(bytes, &mut cursor)?);
    }
    if cursor != bytes.len() {
        return Err(CapabilityError::InvalidRequest(
            "remote display info payload has trailing bytes",
        ));
    }
    Ok(DisplayInfo {
        provider,
        display_name,
        instance_id,
        built_in,
        primary,
        current_mode,
        modes,
        hdr_capable,
        touch_capable,
    })
}

/// Binary-encode a display update request.
pub fn encode_display_update_request(request: &DisplayUpdateRequest) -> Vec<u8> {
    let mut out = Vec::with_capacity(4 + 1 + 12);
    out.extend_from_slice(&content_kind_to_u32(request.content_kind).to_le_bytes());
    let mut flags = 0u8;
    if request.width.is_some() {
        flags |= 1;
    }
    if request.height.is_some() {
        flags |= 2;
    }
    if request.refresh_millihz.is_some() {
        flags |= 4;
    }
    out.push(flags);
    if let Some(width) = request.width {
        out.extend_from_slice(&width.to_le_bytes());
    }
    if let Some(height) = request.height {
        out.extend_from_slice(&height.to_le_bytes());
    }
    if let Some(refresh) = request.refresh_millihz {
        out.extend_from_slice(&refresh.to_le_bytes());
    }
    out
}

/// Binary-decode a display update request.
pub fn decode_display_update_request(
    bytes: &[u8],
) -> Result<DisplayUpdateRequest, CapabilityError> {
    if bytes.len() < 5 {
        return Err(CapabilityError::InvalidRequest(
            "remote display update payload too short",
        ));
    }
    let content_kind = content_kind_from_u32(read_u32_le(bytes, 0));
    let flags = bytes[4];
    let mut cursor = 5usize;
    let width = decode_optional_u32(bytes, &mut cursor, flags & 1 != 0)?;
    let height = decode_optional_u32(bytes, &mut cursor, flags & 2 != 0)?;
    let refresh_millihz = decode_optional_u32(bytes, &mut cursor, flags & 4 != 0)?;
    if flags & !7 != 0 || cursor != bytes.len() {
        return Err(CapabilityError::InvalidRequest(
            "remote display update payload is invalid",
        ));
    }
    let request = DisplayUpdateRequest {
        content_kind,
        width,
        height,
        refresh_millihz,
    };
    validate_display_update_request(&request)?;
    Ok(request)
}

fn decode_optional_u32(
    bytes: &[u8],
    cursor: &mut usize,
    present: bool,
) -> Result<Option<u32>, CapabilityError> {
    if !present {
        return Ok(None);
    }
    if bytes.len().saturating_sub(*cursor) < 4 {
        return Err(CapabilityError::InvalidRequest(
            "remote display update optional value is truncated",
        ));
    }
    let value = read_u32_le(bytes, *cursor);
    *cursor += 4;
    Ok(Some(value))
}

fn read_bool(
    bytes: &[u8],
    cursor: &mut usize,
    invalid: &'static str,
) -> Result<bool, CapabilityError> {
    let value = bytes[*cursor];
    *cursor += 1;
    match value {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(CapabilityError::InvalidRequest(invalid)),
    }
}

#[derive(Debug)]
pub struct DisplayRemoteAdapter<D> {
    pub device: D,
}

impl<D> DisplayRemoteAdapter<D> {
    pub fn new(device: D) -> Self {
        Self { device }
    }
}

impl<D> RemoteCapabilityProvider for DisplayRemoteAdapter<D>
where
    D: DisplayDevice,
{
    fn descriptor(&self) -> CapabilityDescriptor {
        self.device.descriptor()
    }

    fn invoke(
        &mut self,
        _session_id: &[u8],
        invocation: &CapabilityInvocation,
        inline_parameters: Option<&[u8]>,
    ) -> Result<RemoteInvocationResult, CapabilityError> {
        let inline_payload = if invocation.operation == CapabilityOperation::Query as i32 {
            encode_display_info(&self.device.display_info()?)
        } else if invocation.operation == CapabilityOperation::Render as i32
            || invocation.operation == CapabilityOperation::Control as i32
        {
            let parameters = inline_parameters.ok_or(CapabilityError::InvalidRequest(
                "display remote adapter requires inline update parameters",
            ))?;
            let request = decode_display_update_request(parameters)?;
            self.device.present(&request)?;
            Vec::new()
        } else {
            return Err(CapabilityError::Unsupported(
                "display remote adapter supports query, render, and control",
            ));
        };

        Ok(RemoteInvocationResult {
            result: CapabilityResult {
                result_version: 1,
                invocation_id: invocation.invocation_id.clone(),
                grant_id: invocation.grant_id.clone(),
                success: true,
                result_access_class: invocation.requested_access_class,
                produced_event_kinds: vec![CapabilityEventKind::Display as i32],
                payload_object: None,
                error_reason: String::new(),
                produced_at: None,
                signature: None,
            },
            inline_payload,
        })
    }
}
