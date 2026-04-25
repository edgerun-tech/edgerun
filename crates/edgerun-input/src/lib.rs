use edgerun_capabilities::{
    capability_descriptor, constraint, CapabilityConstraintKind, CapabilityDescriptor,
    CapabilityError, CapabilityEventKind, CapabilityModality, CapabilityOperation,
    CapabilityProvider, CapabilityRole,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputDeviceKind {
    Keyboard,
    Pointer,
    Touch,
    Switch,
    Pen,
    Gamepad,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputEventKind {
    Key,
    RelativeMotion,
    AbsoluteMotion,
    Switch,
    Misc,
    Synchronization,
    Other(u16),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputDeviceInfo {
    pub provider: String,
    pub instance_id: String,
    pub display_name: String,
    pub kind: InputDeviceKind,
    pub event_node: String,
    pub physical_path: Option<String>,
    pub unique_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputEventRecord {
    pub timestamp_sec: i64,
    pub timestamp_usec: i64,
    pub kind: InputEventKind,
    pub code: u16,
    pub value: i32,
}

pub trait InputDevice: CapabilityProvider {
    fn input_info(&self) -> Result<InputDeviceInfo, CapabilityError>;
    fn read_events(&mut self, max_events: usize) -> Result<Vec<InputEventRecord>, CapabilityError>;
}

pub fn default_input_descriptor(provider: &str, instance_id: &str) -> CapabilityDescriptor {
    capability_descriptor(
        provider,
        instance_id,
        CapabilityRole::Input,
        &[CapabilityModality::Touch, CapabilityModality::Text],
        &[
            CapabilityEventKind::Touch,
            CapabilityEventKind::Text,
            CapabilityEventKind::State,
        ],
        &[
            CapabilityOperation::Query,
            CapabilityOperation::Observe,
            CapabilityOperation::Capture,
        ],
        vec![constraint(CapabilityConstraintKind::RequireLocalOnly)],
    )
}

pub fn validate_event_read_request(max_events: usize) -> Result<(), CapabilityError> {
    if max_events == 0 {
        return Err(CapabilityError::InvalidRequest(
            "input event read max_events must be greater than zero",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ----- InputDeviceKind -----

    #[test]
    fn device_kind_equality_and_copy() {
        let a = InputDeviceKind::Keyboard;
        let b = InputDeviceKind::Keyboard;
        let c = InputDeviceKind::Pointer;
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn device_kind_debug_format() {
        assert_eq!(format!("{:?}", InputDeviceKind::Touch), "Touch");
        assert_eq!(format!("{:?}", InputDeviceKind::Gamepad), "Gamepad");
        assert_eq!(format!("{:?}", InputDeviceKind::Other), "Other");
    }

    #[test]
    fn device_kind_all_variants() {
        let kinds = [
            InputDeviceKind::Keyboard,
            InputDeviceKind::Pointer,
            InputDeviceKind::Touch,
            InputDeviceKind::Switch,
            InputDeviceKind::Pen,
            InputDeviceKind::Gamepad,
            InputDeviceKind::Other,
        ];
        // All distinct
        for (i, a) in kinds.iter().enumerate() {
            for (j, b) in kinds.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b);
                }
            }
        }
    }

    // ----- InputEventKind -----

    #[test]
    fn event_kind_all_variants() {
        let kinds = [
            InputEventKind::Key,
            InputEventKind::RelativeMotion,
            InputEventKind::AbsoluteMotion,
            InputEventKind::Switch,
            InputEventKind::Misc,
            InputEventKind::Synchronization,
            InputEventKind::Other(99),
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
    fn event_kind_other_carries_code() {
        let k = InputEventKind::Other(0x12);
        assert_eq!(k, InputEventKind::Other(0x12));
        assert_ne!(k, InputEventKind::Other(0x13));
    }

    #[test]
    fn event_kind_clone() {
        let a = InputEventKind::Key;
        let b = a;
        assert_eq!(a, b);
    }

    // ----- InputDeviceInfo -----

    #[test]
    fn device_info_equality() {
        let a = InputDeviceInfo {
            provider: "evdev".into(),
            instance_id: "event0".into(),
            display_name: "Test Keyboard".into(),
            kind: InputDeviceKind::Keyboard,
            event_node: "/dev/input/event0".into(),
            physical_path: Some("usb-0000:00:14.0-1/input0".into()),
            unique_id: None,
        };
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn device_info_with_all_fields_some() {
        let info = InputDeviceInfo {
            provider: "evdev".into(),
            instance_id: "event1".into(),
            display_name: "Touch Screen".into(),
            kind: InputDeviceKind::Touch,
            event_node: "/dev/input/event1".into(),
            physical_path: Some("usb-0000:00:14.0-2/input0".into()),
            unique_id: Some("abcdef123".into()),
        };
        assert_eq!(info.provider, "evdev");
        assert_eq!(info.instance_id, "event1");
        assert_eq!(info.kind, InputDeviceKind::Touch);
        assert_eq!(
            info.physical_path.as_deref(),
            Some("usb-0000:00:14.0-2/input0")
        );
        assert_eq!(info.unique_id.as_deref(), Some("abcdef123"));
    }

    #[test]
    fn device_info_debug_format() {
        let info = InputDeviceInfo {
            provider: "test".into(),
            instance_id: "ev0".into(),
            display_name: "Test".into(),
            kind: InputDeviceKind::Other,
            event_node: "/dev/input/ev0".into(),
            physical_path: None,
            unique_id: None,
        };
        let debug = format!("{:?}", info);
        assert!(debug.contains("test"));
        assert!(debug.contains("ev0"));
    }

    // ----- InputEventRecord -----

    #[test]
    fn event_record_all_fields() {
        let rec = InputEventRecord {
            timestamp_sec: 1234567890,
            timestamp_usec: 500_000,
            kind: InputEventKind::Key,
            code: 0x1e,
            value: 1,
        };
        assert_eq!(rec.timestamp_sec, 1234567890);
        assert_eq!(rec.timestamp_usec, 500_000);
        assert_eq!(rec.code, 0x1e);
        assert_eq!(rec.value, 1);
    }

    #[test]
    fn event_record_equality() {
        let a = InputEventRecord {
            timestamp_sec: 0,
            timestamp_usec: 0,
            kind: InputEventKind::Key,
            code: 30,
            value: 0,
        };
        let b = a.clone();
        assert_eq!(a, b);
    }

    // ----- default_input_descriptor -----

    #[test]
    fn descriptor_is_input_capability() {
        let descriptor = default_input_descriptor("evdev", "event0");
        assert_eq!(descriptor.role, CapabilityRole::Input as i32);
        assert!(descriptor
            .operations
            .contains(&(CapabilityOperation::Observe as i32)));
    }

    #[test]
    fn descriptor_contains_query_and_capture() {
        let descriptor = default_input_descriptor("test", "id");
        assert!(descriptor
            .operations
            .contains(&(CapabilityOperation::Query as i32)));
        assert!(descriptor
            .operations
            .contains(&(CapabilityOperation::Capture as i32)));
    }

    #[test]
    fn descriptor_has_expected_modalities() {
        let descriptor = default_input_descriptor("test", "id");
        assert!(descriptor
            .modalities
            .contains(&(CapabilityModality::Touch as i32)));
        assert!(descriptor
            .modalities
            .contains(&(CapabilityModality::Text as i32)));
    }

    #[test]
    fn descriptor_has_expected_event_kinds() {
        let descriptor = default_input_descriptor("test", "id");
        assert!(descriptor
            .event_kinds
            .contains(&(CapabilityEventKind::Touch as i32)));
        assert!(descriptor
            .event_kinds
            .contains(&(CapabilityEventKind::Text as i32)));
        assert!(descriptor
            .event_kinds
            .contains(&(CapabilityEventKind::State as i32)));
    }

    #[test]
    fn descriptor_has_require_local_only_constraint() {
        let descriptor = default_input_descriptor("test", "id");
        assert!(descriptor
            .default_constraints
            .iter()
            .any(|c| c.kind == CapabilityConstraintKind::RequireLocalOnly as i32));
    }

    #[test]
    fn descriptor_uses_provider_and_instance() {
        let descriptor = default_input_descriptor("my-provider", "my-instance");
        assert_eq!(descriptor.provider_name, "my-provider");
        assert_eq!(descriptor.provider_instance_id, "my-instance");
    }

    // ----- validate_event_read_request -----

    #[test]
    fn read_request_requires_positive_count() {
        let err = validate_event_read_request(0).unwrap_err();
        assert_eq!(
            err,
            CapabilityError::InvalidRequest(
                "input event read max_events must be greater than zero",
            )
        );
    }

    #[test]
    fn read_request_accepts_positive_values() {
        assert!(validate_event_read_request(1).is_ok());
        assert!(validate_event_read_request(100).is_ok());
        assert!(validate_event_read_request(usize::MAX).is_ok());
    }
}
