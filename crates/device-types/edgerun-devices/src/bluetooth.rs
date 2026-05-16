extern crate alloc;
mod prelude {
    pub mod v1 {
        pub use alloc::borrow::ToOwned;
        pub use alloc::boxed::Box;
        pub use alloc::format;
        pub use alloc::string::{String, ToString};
        pub use alloc::vec;
        pub use alloc::vec::Vec;
        pub use core::prelude::rust_2021::*;
    }
}

use edgerun_capabilities::{
    CapabilityDescriptor, CapabilityEventKind, CapabilityModality, CapabilityOperation,
    CapabilityProvider, CapabilityRole, capability_descriptor,
};
use prelude::v1::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BluetoothAddressKind {
    Public,
    Random,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BluetoothTransportKind {
    Classic,
    LowEnergy,
    DualMode,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BluetoothProfile {
    AudioSink,
    AudioSource,
    Headset,
    HandsFree,
    HearingAid,
    Microphone,
    Speaker,
    Headphones,
    CarAudio,
    Hid,
    HeartRate,
    BatteryService,
    Other,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BluetoothBeaconObservation {
    pub device_id: String,
    pub transport_kind: BluetoothTransportKind,
    pub address_kind: BluetoothAddressKind,
    pub rssi_dbm: i16,
    pub tx_power_dbm: Option<i16>,
    pub local_name: Option<String>,
    pub service_uuids: Vec<String>,
    pub profiles: Vec<BluetoothProfile>,
    pub classic_device_class: Option<u32>,
    pub advertisement_data: Vec<u8>,
    pub captured_at_unix_ms: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BluetoothScanResult {
    pub observations: Vec<BluetoothBeaconObservation>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BluetoothLinkKind {
    Sco,
    Acl,
    Esco,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BluetoothConnectionInfo {
    pub device_id: String,
    pub transport_kind: BluetoothTransportKind,
    pub address_kind: BluetoothAddressKind,
    pub link_kind: BluetoothLinkKind,
    pub outbound: bool,
    pub state: u16,
    pub local_name: Option<String>,
    pub service_uuids: Vec<String>,
    pub profiles: Vec<BluetoothProfile>,
    pub trusted: Option<bool>,
    pub paired: Option<bool>,
}

pub trait BluetoothConnectionProvider: CapabilityProvider {
    fn list_connections(
        &self,
    ) -> Result<Vec<BluetoothConnectionInfo>, edgerun_capabilities::CapabilityError>;
}

pub trait BluetoothScanner: CapabilityProvider {
    fn scan_nearby(&self) -> Result<BluetoothScanResult, edgerun_capabilities::CapabilityError>;
}

pub fn default_bluetooth_descriptor(provider: &str, instance_id: &str) -> CapabilityDescriptor {
    capability_descriptor(
        provider,
        instance_id,
        CapabilityRole::Communication,
        &[CapabilityModality::Radio],
        &[CapabilityEventKind::Radio],
        &[CapabilityOperation::Observe, CapabilityOperation::Query],
        Vec::new(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- BluetoothAddressKind ---

    #[test]
    fn address_kind_variants_are_distinct() {
        assert_ne!(BluetoothAddressKind::Public, BluetoothAddressKind::Random);
        assert_ne!(BluetoothAddressKind::Public, BluetoothAddressKind::Unknown);
        assert_ne!(BluetoothAddressKind::Random, BluetoothAddressKind::Unknown);
    }

    #[test]
    fn address_kind_is_copy_and_clone() {
        let kind = BluetoothAddressKind::Public;
        let cloned = kind;
        assert_eq!(kind, cloned);
    }

    #[test]
    fn address_kind_debug_format() {
        assert_eq!(format!("{:?}", BluetoothAddressKind::Public), "Public");
        assert_eq!(format!("{:?}", BluetoothAddressKind::Random), "Random");
        assert_eq!(format!("{:?}", BluetoothAddressKind::Unknown), "Unknown");
    }

    // --- BluetoothTransportKind ---

    #[test]
    fn transport_kind_variants_are_distinct() {
        let variants = [
            BluetoothTransportKind::Classic,
            BluetoothTransportKind::LowEnergy,
            BluetoothTransportKind::DualMode,
            BluetoothTransportKind::Unknown,
        ];
        for (i, a) in variants.iter().enumerate() {
            for (j, b) in variants.iter().enumerate() {
                if i != j {
                    assert_ne!(*a, *b, "variant {i} should differ from variant {j}");
                }
            }
        }
    }

    #[test]
    fn transport_kind_is_copy_and_clone() {
        let kind = BluetoothTransportKind::DualMode;
        let cloned = kind;
        assert_eq!(kind, cloned);
    }

    // --- BluetoothProfile ---

    #[test]
    fn profile_variants_exist() {
        let profiles = [
            BluetoothProfile::AudioSink,
            BluetoothProfile::AudioSource,
            BluetoothProfile::Headset,
            BluetoothProfile::HandsFree,
            BluetoothProfile::HearingAid,
            BluetoothProfile::Microphone,
            BluetoothProfile::Speaker,
            BluetoothProfile::Headphones,
            BluetoothProfile::CarAudio,
            BluetoothProfile::Hid,
            BluetoothProfile::HeartRate,
            BluetoothProfile::BatteryService,
            BluetoothProfile::Other,
        ];
        // Ensure all are distinct
        for (i, a) in profiles.iter().enumerate() {
            for (j, b) in profiles.iter().enumerate() {
                if i != j {
                    assert_ne!(*a, *b, "profile {i} should differ from profile {j}");
                }
            }
        }
    }

    #[test]
    fn profile_debug_format() {
        assert_eq!(format!("{:?}", BluetoothProfile::Hid), "Hid");
        assert_eq!(format!("{:?}", BluetoothProfile::HeartRate), "HeartRate");
    }

    // --- BluetoothBeaconObservation ---

    #[test]
    fn beacon_observation_default_fields() {
        let obs = BluetoothBeaconObservation {
            device_id: "dev1".to_string(),
            transport_kind: BluetoothTransportKind::LowEnergy,
            address_kind: BluetoothAddressKind::Random,
            rssi_dbm: -50,
            tx_power_dbm: None,
            local_name: None,
            service_uuids: vec![],
            profiles: vec![],
            classic_device_class: None,
            advertisement_data: vec![],
            captured_at_unix_ms: 0,
        };
        assert_eq!(obs.device_id, "dev1");
        assert_eq!(obs.rssi_dbm, -50);
        assert!(obs.tx_power_dbm.is_none());
        assert!(obs.service_uuids.is_empty());
        assert!(obs.profiles.is_empty());
    }

    #[test]
    fn beacon_observation_clone_and_eq() {
        let obs = BluetoothBeaconObservation {
            device_id: "dev2".to_string(),
            transport_kind: BluetoothTransportKind::Classic,
            address_kind: BluetoothAddressKind::Public,
            rssi_dbm: -70,
            tx_power_dbm: Some(4),
            local_name: Some("TestDevice".to_string()),
            service_uuids: vec!["uuid1".to_string()],
            profiles: vec![BluetoothProfile::Hid],
            classic_device_class: Some(0x123456),
            advertisement_data: vec![0x02, 0x01, 0x06],
            captured_at_unix_ms: 1_000_000,
        };
        let cloned = obs.clone();
        assert_eq!(obs, cloned);
    }

    #[test]
    fn beacon_observation_debug() {
        let obs = BluetoothBeaconObservation {
            device_id: "d".to_string(),
            transport_kind: BluetoothTransportKind::Unknown,
            address_kind: BluetoothAddressKind::Unknown,
            rssi_dbm: 0,
            tx_power_dbm: None,
            local_name: None,
            service_uuids: vec![],
            profiles: vec![],
            classic_device_class: None,
            advertisement_data: vec![],
            captured_at_unix_ms: 0,
        };
        let debug = format!("{:?}", obs);
        assert!(debug.contains("BluetoothBeaconObservation"));
        assert!(debug.contains("device_id"));
    }

    // --- BluetoothScanResult ---

    #[test]
    fn scan_result_empty() {
        let result = BluetoothScanResult {
            observations: vec![],
        };
        assert!(result.observations.is_empty());
    }

    #[test]
    fn scan_result_with_observations() {
        let obs = BluetoothBeaconObservation {
            device_id: "d1".to_string(),
            transport_kind: BluetoothTransportKind::LowEnergy,
            address_kind: BluetoothAddressKind::Public,
            rssi_dbm: -40,
            tx_power_dbm: None,
            local_name: None,
            service_uuids: vec![],
            profiles: vec![],
            classic_device_class: None,
            advertisement_data: vec![],
            captured_at_unix_ms: 100,
        };
        let result = BluetoothScanResult {
            observations: vec![obs],
        };
        assert_eq!(result.observations.len(), 1);
        assert_eq!(result.observations[0].device_id, "d1");
    }

    #[test]
    fn scan_result_clone_and_eq() {
        let result = BluetoothScanResult {
            observations: vec![],
        };
        assert_eq!(result.clone(), result);
    }

    // --- BluetoothLinkKind ---

    #[test]
    fn link_kind_variants_are_distinct() {
        let variants = [
            BluetoothLinkKind::Sco,
            BluetoothLinkKind::Acl,
            BluetoothLinkKind::Esco,
            BluetoothLinkKind::Unknown,
        ];
        for (i, a) in variants.iter().enumerate() {
            for (j, b) in variants.iter().enumerate() {
                if i != j {
                    assert_ne!(*a, *b);
                }
            }
        }
    }

    #[test]
    fn link_kind_debug() {
        assert_eq!(format!("{:?}", BluetoothLinkKind::Sco), "Sco");
        assert_eq!(format!("{:?}", BluetoothLinkKind::Acl), "Acl");
        assert_eq!(format!("{:?}", BluetoothLinkKind::Esco), "Esco");
    }

    // --- BluetoothConnectionInfo ---

    #[test]
    fn connection_info_construction() {
        let info = BluetoothConnectionInfo {
            device_id: "conn1".to_string(),
            transport_kind: BluetoothTransportKind::LowEnergy,
            address_kind: BluetoothAddressKind::Random,
            link_kind: BluetoothLinkKind::Acl,
            outbound: true,
            state: 0,
            local_name: Some("Connected Device".to_string()),
            service_uuids: vec!["uuid-a".to_string()],
            profiles: vec![BluetoothProfile::HeartRate],
            trusted: Some(true),
            paired: Some(true),
        };
        assert!(info.outbound);
        assert_eq!(info.state, 0);
        assert_eq!(info.trusted, Some(true));
        assert_eq!(info.paired, Some(true));
    }

    #[test]
    fn connection_info_with_optional_fields_none() {
        let info = BluetoothConnectionInfo {
            device_id: "c".to_string(),
            transport_kind: BluetoothTransportKind::Classic,
            address_kind: BluetoothAddressKind::Public,
            link_kind: BluetoothLinkKind::Sco,
            outbound: false,
            state: 1,
            local_name: None,
            service_uuids: vec![],
            profiles: vec![],
            trusted: None,
            paired: None,
        };
        assert!(info.local_name.is_none());
        assert!(info.trusted.is_none());
        assert!(info.paired.is_none());
    }

    #[test]
    fn connection_info_clone_and_eq() {
        let info = BluetoothConnectionInfo {
            device_id: "c".to_string(),
            transport_kind: BluetoothTransportKind::DualMode,
            address_kind: BluetoothAddressKind::Unknown,
            link_kind: BluetoothLinkKind::Unknown,
            outbound: true,
            state: 0,
            local_name: None,
            service_uuids: vec![],
            profiles: vec![],
            trusted: None,
            paired: None,
        };
        assert_eq!(info.clone(), info);
    }

    // --- default_bluetooth_descriptor ---

    #[test]
    fn bluetooth_descriptor_has_correct_role() {
        let descriptor = default_bluetooth_descriptor("bt", "hci0");
        assert_eq!(descriptor.role, CapabilityRole::Communication as i32);
    }

    #[test]
    fn bluetooth_descriptor_has_radio_modality() {
        let descriptor = default_bluetooth_descriptor("bt", "hci0");
        assert_eq!(
            descriptor.modalities,
            vec![CapabilityModality::Radio as i32]
        );
    }

    #[test]
    fn bluetooth_descriptor_has_radio_event_kind() {
        let descriptor = default_bluetooth_descriptor("bt", "hci0");
        assert_eq!(
            descriptor.event_kinds,
            vec![CapabilityEventKind::Radio as i32]
        );
    }

    #[test]
    fn bluetooth_descriptor_has_observe_and_query_operations() {
        let descriptor = default_bluetooth_descriptor("bt", "hci0");
        assert!(
            descriptor
                .operations
                .contains(&(CapabilityOperation::Observe as i32))
        );
        assert!(
            descriptor
                .operations
                .contains(&(CapabilityOperation::Query as i32))
        );
    }

    #[test]
    fn bluetooth_descriptor_uses_instance_id() {
        let descriptor = default_bluetooth_descriptor("bt-provider", "hci1");
        assert_eq!(descriptor.provider_name, "bt-provider");
        assert_eq!(descriptor.provider_instance_id, "hci1");
    }
}
