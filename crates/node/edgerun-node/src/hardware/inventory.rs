use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use edgerun_capabilities::CapabilityDescriptor;
use edgerun_protocols::wire::{RuntimeAppInstall, RuntimeDeploymentConfig};

use crate::runtime::RuntimeServicePlan;

/// Full hardware inventory for this machine.
/// Platform-specific: Linux drivers on Linux.
pub struct HardwareInventory {
    pub platform: &'static str,
    pub gpus: Vec<String>,
    pub displays: Vec<String>,
    pub input_devices: Vec<String>,
    pub audio_input: Vec<String>,
    pub audio_output: Vec<String>,
    pub sensors: Vec<String>,
    pub camera: Vec<String>,
    pub fingerprint_readers: Vec<String>,
    pub bluetooth_controllers: Vec<String>,
    pub wifi_interfaces: Vec<String>,
    pub usb_devices: Vec<String>,
    pub pci_devices: Vec<String>,
    pub nfc_adapters: Vec<String>,
    pub npu_devices: Vec<String>,
    pub power_supplies: Vec<String>,
    pub cec_adapters: Vec<String>,
    pub biometric: Vec<String>,
    pub location: Vec<String>,
    pub keystore: Vec<String>,
    pub capability_descriptors: Vec<CapabilityDescriptor>,
}

impl HardwareInventory {
    pub fn capability_provider_apps(&self, developer_id: [u8; 32]) -> Vec<RuntimeAppInstall> {
        self.capability_descriptors
            .iter()
            .cloned()
            .map(|descriptor| {
                crate::app_model::capability_provider_app_record(descriptor, developer_id)
            })
            .collect()
    }

    pub fn runtime_service_plan(
        &self,
        config: &RuntimeDeploymentConfig,
        developer_id: [u8; 32],
    ) -> RuntimeServicePlan {
        RuntimeServicePlan::from_deployment_with_apps(
            config,
            self.capability_provider_apps(developer_id),
        )
    }

    /// Print a summary of discovered hardware.
    pub fn summary(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("  Platform:        {}", self.platform));
        lines.push(format!("  GPUs:            {}", self.gpus.len()));
        lines.push(format!("  Displays:        {}", self.displays.len()));
        lines.push(format!("  Input:           {}", self.input_devices.len()));
        lines.push(format!(
            "  Audio in/out:    {}/{}",
            self.audio_input.len(),
            self.audio_output.len()
        ));
        lines.push(format!("  Sensors:         {}", self.sensors.len()));
        lines.push(format!("  Camera:          {}", self.camera.len()));
        lines.push(format!(
            "  Fingerprint:     {}",
            self.fingerprint_readers.len()
        ));
        lines.push(format!(
            "  Bluetooth:       {}",
            self.bluetooth_controllers.len()
        ));
        lines.push(format!("  WiFi interfaces: {}", self.wifi_interfaces.len()));
        lines.push(format!("  USB devices:     {}", self.usb_devices.len()));
        lines.push(format!("  PCI devices:     {}", self.pci_devices.len()));
        lines.push(format!("  NFC adapters:    {}", self.nfc_adapters.len()));
        lines.push(format!("  NPU devices:     {}", self.npu_devices.len()));
        lines.push(format!("  Power supplies:  {}", self.power_supplies.len()));
        lines.push(format!("  CEC adapters:    {}", self.cec_adapters.len()));
        lines.push(format!("  Biometric:       {}", self.biometric.len()));
        lines.push(format!("  Location:        {}", self.location.len()));
        lines.push(format!("  Keystore:        {}", self.keystore.len()));
        lines.push(format!(
            "  Capabilities:    {}",
            self.capability_descriptors.len()
        ));
        lines.join("\n")
    }

    /// Stub discover for Linux builds without `all-hardware` feature.
    /// Returns an empty hardware inventory.
    #[cfg(all(not(target_os = "android"), not(feature = "all-hardware")))]
    pub fn discover() -> Self {
        Self {
            platform: "linux",
            gpus: vec![],
            displays: vec![],
            input_devices: vec![],
            audio_input: vec![],
            audio_output: vec![],
            sensors: vec![],
            camera: vec![],
            fingerprint_readers: vec![],
            bluetooth_controllers: vec![],
            wifi_interfaces: vec![],
            usb_devices: vec![],
            pci_devices: vec![],
            nfc_adapters: vec![],
            npu_devices: vec![],
            power_supplies: vec![],
            cec_adapters: vec![],
            biometric: vec![],
            location: vec![],
            keystore: if super::check_android_keystore_available() {
                vec!["available".into()]
            } else {
                vec![]
            },
            capability_descriptors: vec![],
        }
    }
}
