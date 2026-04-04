use lifegraph_capabilities::{
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
    fn power_info(&self) -> Result<PowerSystemInfo, lifegraph_capabilities::CapabilityError>;
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
}
