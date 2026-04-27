#![no_std]

extern crate alloc;

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use edgerun_capabilities::{
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
    use alloc::format;
    use alloc::vec;

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

    // ----- DisplayMode -----

    #[test]
    fn display_mode_equality() {
        let a = DisplayMode {
            width: 1920,
            height: 1080,
            refresh_millihz: 60_000,
        };
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn display_mode_neq_different_values() {
        let a = DisplayMode {
            width: 1920,
            height: 1080,
            refresh_millihz: 60_000,
        };
        let b = DisplayMode {
            width: 2560,
            height: 1440,
            refresh_millihz: 60_000,
        };
        assert_ne!(a, b);
    }

    #[test]
    fn display_mode_debug_format() {
        let mode = DisplayMode {
            width: 3840,
            height: 2160,
            refresh_millihz: 120_000,
        };
        let debug = format!("{:?}", mode);
        assert!(debug.contains("3840"));
        assert!(debug.contains("2160"));
        assert!(debug.contains("120000"));
    }

    #[test]
    fn display_mode_copy() {
        let a = DisplayMode {
            width: 1280,
            height: 720,
            refresh_millihz: 30_000,
        };
        let b = a;
        assert_eq!(a, b);
    }

    // ----- DisplayInfo -----

    #[test]
    fn display_info_equality() {
        let info = DisplayInfo {
            provider: "wayland".into(),
            display_name: "eDP-1".into(),
            instance_id: "eDP-1".into(),
            built_in: true,
            primary: true,
            current_mode: DisplayMode {
                width: 1920,
                height: 1080,
                refresh_millihz: 60_000,
            },
            modes: vec![],
            hdr_capable: false,
            touch_capable: false,
        };
        let cloned = info.clone();
        assert_eq!(info, cloned);
    }

    #[test]
    fn display_info_debug_format() {
        let info = DisplayInfo {
            provider: "drm".into(),
            display_name: "HDMI-A-1".into(),
            instance_id: "card0-HDMI-A-1".into(),
            built_in: false,
            primary: false,
            current_mode: DisplayMode {
                width: 0,
                height: 0,
                refresh_millihz: 0,
            },
            modes: vec![],
            hdr_capable: true,
            touch_capable: false,
        };
        let debug = format!("{:?}", info);
        assert!(debug.contains("HDMI-A-1"));
        assert!(debug.contains("drm"));
    }

    #[test]
    fn display_info_with_multiple_modes() {
        let info = DisplayInfo {
            provider: "x11".into(),
            display_name: "DP-1".into(),
            instance_id: "DP-1".into(),
            built_in: false,
            primary: true,
            current_mode: DisplayMode {
                width: 2560,
                height: 1440,
                refresh_millihz: 144_000,
            },
            modes: vec![
                DisplayMode {
                    width: 2560,
                    height: 1440,
                    refresh_millihz: 144_000,
                },
                DisplayMode {
                    width: 1920,
                    height: 1080,
                    refresh_millihz: 60_000,
                },
            ],
            hdr_capable: true,
            touch_capable: false,
        };
        assert_eq!(info.modes.len(), 2);
        assert!(info.hdr_capable);
        assert!(!info.touch_capable);
    }

    // ----- DisplayContentKind -----

    #[test]
    fn content_kind_all_variants() {
        let kinds = [
            DisplayContentKind::Text,
            DisplayContentKind::Image,
            DisplayContentKind::Video,
            DisplayContentKind::Prompt,
            DisplayContentKind::Other(99),
        ];
        for (i, a) in kinds.iter().enumerate() {
            for (j, b) in kinds.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b);
                }
            }
        }
    }

    #[test]
    fn content_kind_debug_format() {
        assert_eq!(format!("{:?}", DisplayContentKind::Text), "Text");
        assert_eq!(format!("{:?}", DisplayContentKind::Video), "Video");
        assert_eq!(format!("{:?}", DisplayContentKind::Other(42)), "Other(42)");
    }

    #[test]
    fn content_kind_copy() {
        let a = DisplayContentKind::Image;
        let b = a;
        assert_eq!(a, b);
    }

    // ----- DisplayUpdateRequest -----

    #[test]
    fn update_request_equality() {
        let a = DisplayUpdateRequest {
            content_kind: DisplayContentKind::Video,
            width: Some(1920),
            height: Some(1080),
            refresh_millihz: Some(60_000),
        };
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn update_request_debug_format() {
        let req = DisplayUpdateRequest {
            content_kind: DisplayContentKind::Text,
            width: None,
            height: None,
            refresh_millihz: None,
        };
        let debug = format!("{:?}", req);
        assert!(debug.contains("Text"));
    }

    #[test]
    fn update_request_all_none() {
        let req = DisplayUpdateRequest {
            content_kind: DisplayContentKind::Other(0),
            width: None,
            height: None,
            refresh_millihz: None,
        };
        assert!(req.width.is_none());
        assert!(req.height.is_none());
    }

    // ----- validate_display_update_request -----

    #[test]
    fn update_request_rejects_zero_height() {
        let err = validate_display_update_request(&DisplayUpdateRequest {
            content_kind: DisplayContentKind::Image,
            width: Some(100),
            height: Some(0),
            refresh_millihz: None,
        })
        .unwrap_err();
        assert_eq!(
            err,
            CapabilityError::InvalidRequest("display height must be greater than zero")
        );
    }

    #[test]
    fn update_request_valid_with_both_dimensions() {
        let req = DisplayUpdateRequest {
            content_kind: DisplayContentKind::Text,
            width: Some(1920),
            height: Some(1080),
            refresh_millihz: Some(60_000),
        };
        assert!(validate_display_update_request(&req).is_ok());
    }

    #[test]
    fn update_request_valid_with_only_width() {
        let req = DisplayUpdateRequest {
            content_kind: DisplayContentKind::Text,
            width: Some(800),
            height: None,
            refresh_millihz: None,
        };
        assert!(validate_display_update_request(&req).is_ok());
    }

    #[test]
    fn update_request_valid_with_only_height() {
        let req = DisplayUpdateRequest {
            content_kind: DisplayContentKind::Image,
            width: None,
            height: Some(600),
            refresh_millihz: None,
        };
        assert!(validate_display_update_request(&req).is_ok());
    }

    #[test]
    fn update_request_valid_with_no_dimensions() {
        let req = DisplayUpdateRequest {
            content_kind: DisplayContentKind::Prompt,
            width: None,
            height: None,
            refresh_millihz: None,
        };
        assert!(validate_display_update_request(&req).is_ok());
    }

    // ----- default_display_descriptor -----

    #[test]
    fn descriptor_has_query_control_render() {
        let descriptor = default_display_descriptor("test", "id");
        assert!(descriptor
            .operations
            .contains(&(CapabilityOperation::Query as i32)));
        assert!(descriptor
            .operations
            .contains(&(CapabilityOperation::Control as i32)));
        assert!(descriptor
            .operations
            .contains(&(CapabilityOperation::Render as i32)));
    }

    #[test]
    fn descriptor_has_visual_modality() {
        let descriptor = default_display_descriptor("test", "id");
        assert!(descriptor
            .modalities
            .contains(&(CapabilityModality::Visual as i32)));
    }

    #[test]
    fn descriptor_has_display_event_kind() {
        let descriptor = default_display_descriptor("test", "id");
        assert!(descriptor
            .event_kinds
            .contains(&(CapabilityEventKind::Display as i32)));
    }

    #[test]
    fn descriptor_uses_provider_and_instance() {
        let descriptor = default_display_descriptor("drm-kms", "card0-eDP-1");
        assert_eq!(descriptor.provider_name, "drm-kms");
        assert_eq!(descriptor.provider_instance_id, "card0-eDP-1");
    }
}
