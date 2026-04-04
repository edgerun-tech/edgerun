use lifegraph_capabilities::{
    capability_descriptor, constraint, CapabilityConstraintKind, CapabilityDescriptor,
    CapabilityError, CapabilityEventKind, CapabilityModality, CapabilityOperation,
    CapabilityProvider, CapabilityRole,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DisplayMode {
    pub width: u32,
    pub height: u32,
    pub refresh_millihz: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DisplayInfo {
    pub provider: String,
    pub display_name: String,
    pub instance_id: String,
    pub built_in: bool,
    pub primary: bool,
    pub current_mode: DisplayMode,
    pub modes: Vec<DisplayMode>,
    pub hdr_capable: bool,
    pub touch_capable: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisplayContentKind {
    Text,
    Image,
    Video,
    Prompt,
    Other(u32),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DisplayUpdateRequest {
    pub content_kind: DisplayContentKind,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub refresh_millihz: Option<u32>,
}

pub trait DisplayDevice: CapabilityProvider {
    fn display_info(&self) -> Result<DisplayInfo, CapabilityError>;
    fn present(&mut self, request: &DisplayUpdateRequest) -> Result<(), CapabilityError>;
}

pub fn default_display_descriptor(provider: &str, instance_id: &str) -> CapabilityDescriptor {
    capability_descriptor(
        provider,
        instance_id,
        CapabilityRole::Output,
        &[CapabilityModality::Display, CapabilityModality::Visual],
        &[CapabilityEventKind::Display],
        &[
            CapabilityOperation::Query,
            CapabilityOperation::Control,
            CapabilityOperation::Render,
        ],
        vec![constraint(CapabilityConstraintKind::RequireLocalOnly)],
    )
}

pub fn validate_display_update_request(
    request: &DisplayUpdateRequest,
) -> Result<(), CapabilityError> {
    if let Some(width) = request.width {
        if width == 0 {
            return Err(CapabilityError::InvalidRequest(
                "display width must be greater than zero",
            ));
        }
    }
    if let Some(height) = request.height {
        if height == 0 {
            return Err(CapabilityError::InvalidRequest(
                "display height must be greater than zero",
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptor_is_display_output_capability() {
        let descriptor = default_display_descriptor("wayland", "eDP-1");
        assert_eq!(descriptor.role, CapabilityRole::Output as i32);
        assert!(descriptor
            .modalities
            .contains(&(CapabilityModality::Display as i32)));
        assert!(descriptor
            .default_constraints
            .iter()
            .any(|c| c.kind == CapabilityConstraintKind::RequireLocalOnly as i32));
    }

    #[test]
    fn update_request_requires_nonzero_dimensions() {
        let err = validate_display_update_request(&DisplayUpdateRequest {
            content_kind: DisplayContentKind::Text,
            width: Some(0),
            height: Some(100),
            refresh_millihz: None,
        })
        .unwrap_err();
        assert_eq!(
            err,
            CapabilityError::InvalidRequest("display width must be greater than zero")
        );
    }
}
