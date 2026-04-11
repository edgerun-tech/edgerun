//! Display device remote adapter.

use edgerun_capabilities::{
    CapabilityAccessClass, CapabilityDescriptor, CapabilityError, CapabilityEventKind,
    CapabilityOperation,
};
use edgerun_display::{
    DisplayContentKind, DisplayDevice, DisplayInfo, DisplayUpdateRequest,
};
use edgerun_proto::edgerun::v0::capability::{CapabilityInvocation, CapabilityResult};
use edgerun_proto::edgerun::v0::capability_runtime::{
    CapabilitySessionAccept, CapabilitySessionOpen,
};

use crate::protocol::{accept_session_open_unchecked, RemoteCapabilityProvider, RemoteInvocationResult};

#[derive(Debug)]
pub struct DisplayRemoteAdapter<D> {
    pub device: D,
}

impl<D> DisplayRemoteAdapter<D> {
    pub fn new(device: D) -> Self {
        Self { device }
    }
}

impl<D: DisplayDevice> RemoteCapabilityProvider for DisplayRemoteAdapter<D> {
    fn descriptor(&self) -> CapabilityDescriptor {
        self.device.descriptor()
    }

    fn open_session(&mut self, open: &CapabilitySessionOpen) -> Result<CapabilitySessionAccept, CapabilityError> {
        Ok(accept_session_open_unchecked(open))
    }

    fn invoke(&mut self, _session_id: &[u8], invocation: &CapabilityInvocation, inline_parameters: Option<&[u8]>) -> Result<RemoteInvocationResult, CapabilityError> {
        let op = CapabilityOperation::try_from(invocation.operation)
            .map_err(|_| CapabilityError::InvalidRequest("invalid operation"))?;
        match op {
            CapabilityOperation::Query => {
                let info = self.device.display_info().map_err(|e| {
                    CapabilityError::Provider(format!("display query failed: {e}"))
                })?;
                let payload = display_info_to_json(&info);
                Ok(RemoteInvocationResult {
                    result: CapabilityResult {
                        result_version: 1,
                        invocation_id: invocation.invocation_id.clone(),
                        grant_id: invocation.grant_id.clone(),
                        success: true,
                        result_access_class: CapabilityAccessClass::Derived as i32,
                        produced_event_kinds: vec![CapabilityEventKind::Display as i32],
                        payload_object: None,
                        error_reason: String::new(),
                        produced_at: None,
                        signature: None,
                    },
                    inline_payload: payload,
                })
            }
            CapabilityOperation::Render => {
                let params = inline_parameters.ok_or(CapabilityError::InvalidRequest(
                    "display render requires inline parameters",
                ))?;
                let request = decode_display_update_request(&params)?;
                self.device.present(&request).map_err(|e| {
                    CapabilityError::Provider(format!("display present failed: {e}"))
                })?;
                Ok(RemoteInvocationResult {
                    result: CapabilityResult {
                        result_version: 1,
                        invocation_id: invocation.invocation_id.clone(),
                        grant_id: invocation.grant_id.clone(),
                        success: true,
                        result_access_class: CapabilityAccessClass::Derived as i32,
                        produced_event_kinds: vec![CapabilityEventKind::Display as i32],
                        payload_object: None,
                        error_reason: String::new(),
                        produced_at: None,
                        signature: None,
                    },
                    inline_payload: b"display-presented".to_vec(),
                })
            }
            _ => Err(CapabilityError::Unsupported(
                "display adapter does not support this operation",
            )),
        }
    }
}

fn decode_display_update_request(bytes: &[u8]) -> Result<DisplayUpdateRequest, CapabilityError> {
    if bytes.len() < 13 {
        return Err(CapabilityError::InvalidRequest("display update request too short"));
    }
    let content_kind = match bytes[0] {
        0 => DisplayContentKind::Text,
        1 => DisplayContentKind::Image,
        2 => DisplayContentKind::Video,
        3 => DisplayContentKind::Prompt,
        v => DisplayContentKind::Other(v as u32),
    };
    let width = if bytes[1] != 0 {
        Some(u32::from_le_bytes(bytes[2..6].try_into().unwrap()))
    } else { None };
    let height = if bytes[6] != 0 {
        Some(u32::from_le_bytes(bytes[7..11].try_into().unwrap()))
    } else { None };
    let refresh = if bytes[11] != 0 {
        Some(u32::from_le_bytes(bytes[12..16].try_into().unwrap()))
    } else { None };
    Ok(DisplayUpdateRequest { content_kind, width, height, refresh_millihz: refresh })
}

fn display_info_to_json(info: &DisplayInfo) -> Vec<u8> {
    format!(
        "{{\"display_name\":\"{}\",\"width\":{},\"height\":{},\"refresh_millihz\":{},\"hdr\":{},\"touch\":{}}}",
        info.display_name,
        info.current_mode.width,
        info.current_mode.height,
        info.current_mode.refresh_millihz,
        info.hdr_capable,
        info.touch_capable,
    ).into_bytes()
}
