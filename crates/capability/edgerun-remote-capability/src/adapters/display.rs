//! Display device remote adapter.

use crate::prelude::v1::*;
use edgerun_capabilities::{
    CapabilityDescriptor, CapabilityError, CapabilityEventKind, CapabilityOperation,
};
use edgerun_core::protocol::capability::{CapabilityInvocation, CapabilityResult};
use edgerun_display::{
    validate_display_update_request, DisplayContentKind, DisplayDevice, DisplayInfo, DisplayMode,
    DisplayUpdateRequest,
};

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

#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    edgerun_wire::Archive,
    edgerun_wire::Serialize,
    edgerun_wire::Deserialize,
)]
#[rkyv(crate = edgerun_wire)]
struct DisplayModeWire {
    width: u32,
    height: u32,
    refresh_millihz: u32,
}

#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    edgerun_wire::Archive,
    edgerun_wire::Serialize,
    edgerun_wire::Deserialize,
)]
#[rkyv(crate = edgerun_wire)]
struct DisplayInfoWire {
    provider: String,
    display_name: String,
    instance_id: String,
    built_in: bool,
    primary: bool,
    current_mode: DisplayModeWire,
    modes: Vec<DisplayModeWire>,
    hdr_capable: bool,
    touch_capable: bool,
}

#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    edgerun_wire::Archive,
    edgerun_wire::Serialize,
    edgerun_wire::Deserialize,
)]
#[rkyv(crate = edgerun_wire)]
struct DisplayUpdateRequestWire {
    content_kind: u32,
    width: Option<u32>,
    height: Option<u32>,
    refresh_millihz: Option<u32>,
}

fn display_mode_to_wire(mode: DisplayMode) -> DisplayModeWire {
    DisplayModeWire {
        width: mode.width,
        height: mode.height,
        refresh_millihz: mode.refresh_millihz,
    }
}

fn display_mode_from_wire(mode: DisplayModeWire) -> DisplayMode {
    DisplayMode {
        width: mode.width,
        height: mode.height,
        refresh_millihz: mode.refresh_millihz,
    }
}

/// Rkyv-encode display information for remote transport.
pub fn encode_display_info(info: &DisplayInfo) -> Vec<u8> {
    let wire = DisplayInfoWire {
        provider: info.provider.clone(),
        display_name: info.display_name.clone(),
        instance_id: info.instance_id.clone(),
        built_in: info.built_in,
        primary: info.primary,
        current_mode: display_mode_to_wire(info.current_mode),
        modes: info
            .modes
            .iter()
            .copied()
            .map(display_mode_to_wire)
            .collect(),
        hdr_capable: info.hdr_capable,
        touch_capable: info.touch_capable,
    };
    edgerun_wire::to_bytes::<edgerun_wire::WireError>(&wire)
        .expect("display info must serialize through rkyv")
        .into_vec()
}

/// Rkyv-decode display information from remote transport.
pub fn decode_display_info(bytes: &[u8]) -> Result<DisplayInfo, CapabilityError> {
    let owned = bytes.to_vec();
    let wire = edgerun_wire::from_bytes::<DisplayInfoWire, edgerun_wire::WireError>(&owned)
        .map_err(|_| CapabilityError::InvalidRequest("remote display info is not rkyv"))?;
    Ok(DisplayInfo {
        provider: wire.provider,
        display_name: wire.display_name,
        instance_id: wire.instance_id,
        built_in: wire.built_in,
        primary: wire.primary,
        current_mode: display_mode_from_wire(wire.current_mode),
        modes: wire.modes.into_iter().map(display_mode_from_wire).collect(),
        hdr_capable: wire.hdr_capable,
        touch_capable: wire.touch_capable,
    })
}

/// Rkyv-encode a display update request.
pub fn encode_display_update_request(request: &DisplayUpdateRequest) -> Vec<u8> {
    let wire = DisplayUpdateRequestWire {
        content_kind: content_kind_to_u32(request.content_kind),
        width: request.width,
        height: request.height,
        refresh_millihz: request.refresh_millihz,
    };
    edgerun_wire::to_bytes::<edgerun_wire::WireError>(&wire)
        .expect("display update request must serialize through rkyv")
        .into_vec()
}

/// Rkyv-decode a display update request.
pub fn decode_display_update_request(
    bytes: &[u8],
) -> Result<DisplayUpdateRequest, CapabilityError> {
    let owned = bytes.to_vec();
    let wire =
        edgerun_wire::from_bytes::<DisplayUpdateRequestWire, edgerun_wire::WireError>(&owned)
            .map_err(|_| CapabilityError::InvalidRequest("remote display update is not rkyv"))?;
    let request = DisplayUpdateRequest {
        content_kind: content_kind_from_u32(wire.content_kind),
        width: wire.width,
        height: wire.height,
        refresh_millihz: wire.refresh_millihz,
    };
    validate_display_update_request(&request)?;
    Ok(request)
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
