use lifegraph_capabilities::{
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

    #[test]
    fn descriptor_is_input_capability() {
        let descriptor = default_input_descriptor("evdev", "event0");
        assert_eq!(descriptor.role, CapabilityRole::Input as i32);
        assert!(descriptor
            .operations
            .contains(&(CapabilityOperation::Observe as i32)));
    }

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
}
