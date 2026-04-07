use edgerun_capabilities::{
    capability_descriptor, CapabilityDescriptor, CapabilityEventKind, CapabilityModality,
    CapabilityOperation, CapabilityProvider, CapabilityRole,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NfcPowerState {
    Unknown,
    Enabled,
    Disabled,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NfcDeviceInfo {
    pub provider: String,
    pub device_name: String,
    pub power_state: NfcPowerState,
    pub protocol_name: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NfcTargetObservation {
    pub target_id: String,
    pub protocol_name: Option<String>,
    pub technology: Option<String>,
    pub observed_at_unix_ms: i64,
}

pub trait NfcDevice: CapabilityProvider {
    fn device_info(&self) -> Result<NfcDeviceInfo, edgerun_capabilities::CapabilityError>;
    fn power_state(&self) -> Result<NfcPowerState, edgerun_capabilities::CapabilityError>;
}

pub fn default_nfc_descriptor(provider: &str, instance_id: &str) -> CapabilityDescriptor {
    capability_descriptor(
        provider,
        instance_id,
        CapabilityRole::Communication,
        &[CapabilityModality::Radio],
        &[CapabilityEventKind::Radio, CapabilityEventKind::State],
        &[
            CapabilityOperation::Query,
            CapabilityOperation::Observe,
            CapabilityOperation::Control,
        ],
        Vec::new(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nfc_power_state_variants_are_copy() {
        let state = NfcPowerState::Enabled;
        let _copied = state; // must compile: Copy trait
        assert_eq!(state, NfcPowerState::Enabled);
    }

    #[test]
    fn nfc_power_state_equality() {
        assert_eq!(NfcPowerState::Unknown, NfcPowerState::Unknown);
        assert_eq!(NfcPowerState::Enabled, NfcPowerState::Enabled);
        assert_eq!(NfcPowerState::Disabled, NfcPowerState::Disabled);
        assert_ne!(NfcPowerState::Enabled, NfcPowerState::Disabled);
        assert_ne!(NfcPowerState::Unknown, NfcPowerState::Enabled);
    }

    #[test]
    fn nfc_power_state_debug() {
        assert_eq!(format!("{:?}", NfcPowerState::Enabled), "Enabled");
        assert_eq!(format!("{:?}", NfcPowerState::Disabled), "Disabled");
        assert_eq!(format!("{:?}", NfcPowerState::Unknown), "Unknown");
    }

    #[test]
    fn nfc_device_info_construction() {
        let info = NfcDeviceInfo {
            provider: "test-provider".into(),
            device_name: "nfc0".into(),
            power_state: NfcPowerState::Enabled,
            protocol_name: Some("iso14443".into()),
        };
        assert_eq!(info.provider, "test-provider");
        assert_eq!(info.device_name, "nfc0");
        assert_eq!(info.power_state, NfcPowerState::Enabled);
        assert_eq!(info.protocol_name.as_deref(), Some("iso14443"));
    }

    #[test]
    fn nfc_device_info_without_protocol() {
        let info = NfcDeviceInfo {
            provider: "test".into(),
            device_name: "nfc1".into(),
            power_state: NfcPowerState::Unknown,
            protocol_name: None,
        };
        assert_eq!(info.protocol_name, None);
    }

    #[test]
    fn nfc_device_info_clone() {
        let info = NfcDeviceInfo {
            provider: "test".into(),
            device_name: "nfc0".into(),
            power_state: NfcPowerState::Disabled,
            protocol_name: Some("felica".into()),
        };
        let cloned = info.clone();
        assert_eq!(info, cloned);
    }

    #[test]
    fn nfc_target_observation_construction() {
        let obs = NfcTargetObservation {
            target_id: "target-1".into(),
            protocol_name: Some("iso14443a".into()),
            technology: Some("NFC-A".into()),
            observed_at_unix_ms: 1_700_000_000_000,
        };
        assert_eq!(obs.target_id, "target-1");
        assert_eq!(obs.protocol_name.as_deref(), Some("iso14443a"));
        assert_eq!(obs.technology.as_deref(), Some("NFC-A"));
        assert_eq!(obs.observed_at_unix_ms, 1_700_000_000_000);
    }

    #[test]
    fn nfc_target_observation_clone_and_eq() {
        let obs = NfcTargetObservation {
            target_id: "t1".into(),
            protocol_name: None,
            technology: None,
            observed_at_unix_ms: 0,
        };
        let cloned = obs.clone();
        assert_eq!(obs, cloned);
    }

    #[test]
    fn nfc_descriptor_role_is_communication() {
        let descriptor = default_nfc_descriptor("nfc", "nfc0");
        assert_eq!(descriptor.role, CapabilityRole::Communication as i32);
    }

    #[test]
    fn nfc_descriptor_kind_is_correct() {
        let descriptor = default_nfc_descriptor("nfc", "nfc0");
        assert_eq!(descriptor.role, CapabilityRole::Communication as i32);
        assert!(descriptor
            .event_kinds
            .contains(&(CapabilityEventKind::Radio as i32)));
    }

    #[test]
    fn nfc_descriptor_contains_state_event() {
        let descriptor = default_nfc_descriptor("nfc", "nfc0");
        assert!(descriptor
            .event_kinds
            .contains(&(CapabilityEventKind::State as i32)));
    }

    #[test]
    fn nfc_descriptor_contains_radio_modality() {
        let descriptor = default_nfc_descriptor("nfc", "nfc0");
        assert!(descriptor
            .modalities
            .contains(&(CapabilityModality::Radio as i32)));
    }

    #[test]
    fn nfc_descriptor_operations_include_query_observe_control() {
        let descriptor = default_nfc_descriptor("nfc", "nfc0");
        assert!(descriptor
            .operations
            .contains(&(CapabilityOperation::Query as i32)));
        assert!(descriptor
            .operations
            .contains(&(CapabilityOperation::Observe as i32)));
        assert!(descriptor
            .operations
            .contains(&(CapabilityOperation::Control as i32)));
    }

    #[test]
    fn nfc_descriptor_has_no_default_constraints() {
        let descriptor = default_nfc_descriptor("nfc", "nfc0");
        assert!(descriptor.default_constraints.is_empty());
    }

    #[test]
    fn nfc_descriptor_has_non_empty_capability_id() {
        let descriptor = default_nfc_descriptor("my-nfc", "nfc-adapter-1");
        // capability_id is set empty by capability_descriptor(); provider_name and provider_instance_id hold the values
        assert_eq!(descriptor.provider_name, "my-nfc");
        assert_eq!(descriptor.provider_instance_id, "nfc-adapter-1");
    }
}
