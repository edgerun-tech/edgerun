use lifegraph_capabilities::{
    capability_descriptor, CapabilityDescriptor, CapabilityEventKind, CapabilityModality,
    CapabilityOperation, CapabilityProvider, CapabilityRole,
};

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
    ) -> Result<Vec<BluetoothConnectionInfo>, lifegraph_capabilities::CapabilityError>;
}

pub trait BluetoothScanner: CapabilityProvider {
    fn scan_nearby(&self) -> Result<BluetoothScanResult, lifegraph_capabilities::CapabilityError>;
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
    #[test]
    fn bluetooth_descriptor_kind_is_correct() {
        let descriptor = default_bluetooth_descriptor("bt", "hci0");
        assert_eq!(descriptor.role, CapabilityRole::Communication as i32);
        assert_eq!(
            descriptor.event_kinds,
            vec![CapabilityEventKind::Radio as i32]
        );
    }
}
