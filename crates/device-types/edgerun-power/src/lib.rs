#![no_std]

extern crate alloc;

#[cfg(test)]
extern crate std;

#[cfg(test)]
use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::result::Result;
use edgerun_capabilities::{
    capability_descriptor, CapabilityDescriptor, CapabilityEventKind, CapabilityModality,
    CapabilityOperation, CapabilityProvider, CapabilityRole,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PowerSupplyKind {
    Unknown,
    Battery,
    Mains,
    Usb,
    UsbC,
    Wireless,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BatteryStatus {
    Unknown,
    Charging,
    Discharging,
    Full,
    NotCharging,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LidState {
    Unknown,
    Open,
    Closed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PowerSourceInfo {
    pub provider: String,
    pub instance_id: String,
    pub kind: PowerSupplyKind,
    pub manufacturer: Option<String>,
    pub model_name: Option<String>,
    pub online: Option<bool>,
    pub present: Option<bool>,
    pub battery_status: Option<BatteryStatus>,
    pub capacity_percent: Option<u8>,
    pub voltage_now_uv: Option<u64>,
    pub current_now_ua: Option<u64>,
    pub power_now_uw: Option<u64>,
    pub energy_now_uwh: Option<u64>,
    pub energy_full_uwh: Option<u64>,
    pub energy_full_design_uwh: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PowerSystemInfo {
    pub sources: Vec<PowerSourceInfo>,
    pub lid_state: LidState,
    pub on_ac_power: Option<bool>,
    pub battery_percent: Option<u8>,
}

pub trait PowerInventory: CapabilityProvider {
    fn power_info(&self) -> Result<PowerSystemInfo, edgerun_capabilities::CapabilityError>;
}

pub fn default_power_descriptor(provider: &str, instance_id: &str) -> CapabilityDescriptor {
    capability_descriptor(
        provider,
        instance_id,
        CapabilityRole::Communication,
        &[CapabilityModality::Text],
        &[CapabilityEventKind::State],
        &[CapabilityOperation::Query, CapabilityOperation::Observe],
        Vec::new(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn power_descriptor_is_stateful() {
        let descriptor = default_power_descriptor("linux-power", "system");
        assert_eq!(descriptor.role, CapabilityRole::Communication as i32);
        assert!(descriptor
            .event_kinds
            .contains(&(CapabilityEventKind::State as i32)));
    }

    // ----- PowerSupplyKind -----

    #[test]
    fn supply_kind_all_variants() {
        let kinds = [
            PowerSupplyKind::Unknown,
            PowerSupplyKind::Battery,
            PowerSupplyKind::Mains,
            PowerSupplyKind::Usb,
            PowerSupplyKind::UsbC,
            PowerSupplyKind::Wireless,
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
    fn supply_kind_debug_format() {
        assert_eq!(format!("{:?}", PowerSupplyKind::Battery), "Battery");
        assert_eq!(format!("{:?}", PowerSupplyKind::UsbC), "UsbC");
        assert_eq!(format!("{:?}", PowerSupplyKind::Wireless), "Wireless");
    }

    #[test]
    fn supply_kind_copy() {
        let a = PowerSupplyKind::Mains;
        let b = a;
        assert_eq!(a, b);
    }

    // ----- BatteryStatus -----

    #[test]
    fn battery_status_all_variants() {
        let statuses = [
            BatteryStatus::Unknown,
            BatteryStatus::Charging,
            BatteryStatus::Discharging,
            BatteryStatus::Full,
            BatteryStatus::NotCharging,
        ];
        for (i, a) in statuses.iter().enumerate() {
            for (j, b) in statuses.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b);
                }
            }
        }
    }

    #[test]
    fn battery_status_debug_format() {
        assert_eq!(format!("{:?}", BatteryStatus::Charging), "Charging");
        assert_eq!(format!("{:?}", BatteryStatus::Discharging), "Discharging");
        assert_eq!(format!("{:?}", BatteryStatus::NotCharging), "NotCharging");
    }

    #[test]
    fn battery_status_copy() {
        let a = BatteryStatus::Full;
        let b = a;
        assert_eq!(a, b);
    }

    // ----- LidState -----

    #[test]
    fn lid_state_all_variants() {
        let states = [LidState::Unknown, LidState::Open, LidState::Closed];
        for (i, a) in states.iter().enumerate() {
            for (j, b) in states.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b);
                }
            }
        }
    }

    #[test]
    fn lid_state_debug_format() {
        assert_eq!(format!("{:?}", LidState::Open), "Open");
        assert_eq!(format!("{:?}", LidState::Closed), "Closed");
        assert_eq!(format!("{:?}", LidState::Unknown), "Unknown");
    }

    #[test]
    fn lid_state_copy() {
        let a = LidState::Open;
        let b = a;
        assert_eq!(a, b);
    }

    // ----- PowerSourceInfo -----

    #[test]
    fn power_source_info_equality() {
        let a = PowerSourceInfo {
            provider: "linux-power".into(),
            instance_id: "BAT0".into(),
            kind: PowerSupplyKind::Battery,
            manufacturer: Some("LGC".into()),
            model_name: Some("L16M2PB2".into()),
            online: None,
            present: Some(true),
            battery_status: Some(BatteryStatus::Discharging),
            capacity_percent: Some(85),
            voltage_now_uv: Some(12_345_000),
            current_now_ua: Some(1_500_000),
            power_now_uw: Some(18_500_000),
            energy_now_uwh: Some(42_000_000),
            energy_full_uwh: Some(50_000_000),
            energy_full_design_uwh: Some(56_000_000),
        };
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn power_source_info_debug_format() {
        let info = PowerSourceInfo {
            provider: "test".into(),
            instance_id: "AC".into(),
            kind: PowerSupplyKind::Mains,
            manufacturer: None,
            model_name: None,
            online: Some(true),
            present: Some(true),
            battery_status: None,
            capacity_percent: None,
            voltage_now_uv: None,
            current_now_ua: None,
            power_now_uw: None,
            energy_now_uwh: None,
            energy_full_uwh: None,
            energy_full_design_uwh: None,
        };
        let debug = format!("{:?}", info);
        assert!(debug.contains("AC"));
        assert!(debug.contains("Mains"));
    }

    #[test]
    fn power_source_info_minimal() {
        let info = PowerSourceInfo {
            provider: "linux-power".into(),
            instance_id: "unknown".into(),
            kind: PowerSupplyKind::Unknown,
            manufacturer: None,
            model_name: None,
            online: None,
            present: None,
            battery_status: None,
            capacity_percent: None,
            voltage_now_uv: None,
            current_now_ua: None,
            power_now_uw: None,
            energy_now_uwh: None,
            energy_full_uwh: None,
            energy_full_design_uwh: None,
        };
        assert_eq!(info.kind, PowerSupplyKind::Unknown);
        assert!(info.manufacturer.is_none());
    }

    #[test]
    fn power_source_info_ac_adapter() {
        let info = PowerSourceInfo {
            provider: "linux-power".into(),
            instance_id: "ADP1".into(),
            kind: PowerSupplyKind::Mains,
            manufacturer: None,
            model_name: None,
            online: Some(true),
            present: Some(true),
            battery_status: None,
            capacity_percent: None,
            voltage_now_uv: None,
            current_now_ua: None,
            power_now_uw: None,
            energy_now_uwh: None,
            energy_full_uwh: None,
            energy_full_design_uwh: None,
        };
        assert!(info.online.unwrap());
        assert!(info.battery_status.is_none());
    }

    // ----- PowerSystemInfo -----

    #[test]
    fn power_system_info_equality() {
        let a = PowerSystemInfo {
            sources: vec![PowerSourceInfo {
                provider: "linux-power".into(),
                instance_id: "BAT0".into(),
                kind: PowerSupplyKind::Battery,
                manufacturer: None,
                model_name: None,
                online: None,
                present: Some(true),
                battery_status: Some(BatteryStatus::Charging),
                capacity_percent: Some(100),
                voltage_now_uv: None,
                current_now_ua: None,
                power_now_uw: None,
                energy_now_uwh: None,
                energy_full_uwh: None,
                energy_full_design_uwh: None,
            }],
            lid_state: LidState::Open,
            on_ac_power: Some(true),
            battery_percent: Some(100),
        };
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn power_system_info_debug_format() {
        let info = PowerSystemInfo {
            sources: vec![],
            lid_state: LidState::Closed,
            on_ac_power: None,
            battery_percent: None,
        };
        let debug = format!("{:?}", info);
        assert!(debug.contains("Closed"));
    }

    #[test]
    fn power_system_info_empty_sources() {
        let info = PowerSystemInfo {
            sources: vec![],
            lid_state: LidState::Unknown,
            on_ac_power: None,
            battery_percent: None,
        };
        assert!(info.sources.is_empty());
        assert_eq!(info.lid_state, LidState::Unknown);
    }

    #[test]
    fn power_system_info_multiple_sources() {
        let info = PowerSystemInfo {
            sources: vec![
                PowerSourceInfo {
                    provider: "test".into(),
                    instance_id: "AC".into(),
                    kind: PowerSupplyKind::Mains,
                    manufacturer: None,
                    model_name: None,
                    online: Some(true),
                    present: Some(true),
                    battery_status: None,
                    capacity_percent: None,
                    voltage_now_uv: None,
                    current_now_ua: None,
                    power_now_uw: None,
                    energy_now_uwh: None,
                    energy_full_uwh: None,
                    energy_full_design_uwh: None,
                },
                PowerSourceInfo {
                    provider: "test".into(),
                    instance_id: "BAT0".into(),
                    kind: PowerSupplyKind::Battery,
                    manufacturer: None,
                    model_name: None,
                    online: None,
                    present: Some(true),
                    battery_status: Some(BatteryStatus::Full),
                    capacity_percent: Some(100),
                    voltage_now_uv: None,
                    current_now_ua: None,
                    power_now_uw: None,
                    energy_now_uwh: None,
                    energy_full_uwh: None,
                    energy_full_design_uwh: None,
                },
            ],
            lid_state: LidState::Open,
            on_ac_power: Some(true),
            battery_percent: Some(100),
        };
        assert_eq!(info.sources.len(), 2);
    }

    // ----- default_power_descriptor -----

    #[test]
    fn descriptor_has_query_and_observe() {
        let descriptor = default_power_descriptor("test", "id");
        assert!(descriptor
            .operations
            .contains(&(CapabilityOperation::Query as i32)));
        assert!(descriptor
            .operations
            .contains(&(CapabilityOperation::Observe as i32)));
    }

    #[test]
    fn descriptor_has_text_modality() {
        let descriptor = default_power_descriptor("test", "id");
        assert!(descriptor
            .modalities
            .contains(&(CapabilityModality::Text as i32)));
    }

    #[test]
    fn descriptor_has_no_constraints() {
        let descriptor = default_power_descriptor("test", "id");
        assert!(descriptor.default_constraints.is_empty());
    }

    #[test]
    fn descriptor_uses_provider_and_instance() {
        let descriptor = default_power_descriptor("my-power", "sys");
        assert_eq!(descriptor.provider_name, "my-power");
        assert_eq!(descriptor.provider_instance_id, "sys");
    }
}
