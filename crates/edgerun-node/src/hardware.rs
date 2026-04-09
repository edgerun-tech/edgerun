//! Hardware discovery for all platform capability backends.
//!
//! Discovers GPUs, displays, fingerprint readers, Bluetooth, WiFi, USB, PCI,
//! NFC, NPU, power supplies, CEC adapters, and audio calibration devices.
//! Each discovery block is gated behind `#[cfg(feature = "all-hardware")]`
//! and wrapped in error handlers so missing hardware is silently skipped.

// ===========================================================================
// GPU & Display
// ===========================================================================

#[cfg(feature = "all-hardware")]
pub fn discover_gpus() -> Vec<edgerun_linux_gpu::LinuxGpuDevice> {
    match edgerun_linux_gpu::discover_gpus() {
        Ok(backends) => backends,
        Err(e) => {
            eprintln!("edgerund: warning: GPU discovery failed: {}", e);
            Vec::new()
        }
    }
}

#[cfg(not(feature = "all-hardware"))]
pub fn discover_gpus() -> Vec<edgerun_linux_gpu::LinuxGpuDevice> {
    Vec::new()
}

#[cfg(feature = "all-hardware")]
pub fn discover_displays() -> Vec<edgerun_drm_display::DrmConnectorInfo> {
    match edgerun_drm_display::discover_drm_connectors() {
        Ok(connectors) => connectors,
        Err(e) => {
            eprintln!("edgerund: warning: display discovery failed: {}", e);
            Vec::new()
        }
    }
}

#[cfg(not(feature = "all-hardware"))]
pub fn discover_displays() -> Vec<edgerun_drm_display::DrmConnectorInfo> {
    Vec::new()
}

// ===========================================================================
// Fingerprint
// ===========================================================================

#[cfg(feature = "all-hardware")]
pub fn discover_fingerprint_readers() -> Vec<String> {
    let mut readers = Vec::new();

    // Try Goodix USB fingerprint readers
    match edgerun_goodix_fingerprint::discover_supported_devices() {
        Ok(devices) => {
            for device in devices {
                readers.push(format!(
                    "Goodix USB: bus={}, dev={}, {:04x}:{:04x}",
                    device.bus_number, device.device_number, device.vendor_id, device.product_id
                ));
            }
        }
        Err(e) => eprintln!("edgerund: warning: Goodix fingerprint discovery failed: {}", e),
    }

    readers
}

#[cfg(not(feature = "all-hardware"))]
pub fn discover_fingerprint_readers() -> Vec<String> {
    Vec::new()
}

// ===========================================================================
// Bluetooth
// ===========================================================================

#[cfg(feature = "all-hardware")]
pub fn discover_bluetooth_controllers() -> Vec<String> {
    let mut controllers = Vec::new();

    // Bluetooth controller enumeration via mgmt socket
    match edgerun_mgmt_bluetooth::discover_controllers() {
        Ok(ctrls) => {
            for ctrl in ctrls {
                controllers.push(format!("BT controller #{}: {} ({})", ctrl.index, ctrl.name, ctrl.address));
            }
        }
        Err(e) => eprintln!("edgerund: warning: BT controller discovery failed: {}", e),
    }

    controllers
}

#[cfg(not(feature = "all-hardware"))]
pub fn discover_bluetooth_controllers() -> Vec<String> {
    Vec::new()
}

// ===========================================================================
// WiFi
// ===========================================================================

#[cfg(feature = "all-hardware")]
pub fn discover_wifi_interfaces() -> Vec<edgerun_linux_wifi::LinuxWifiInterface> {
    match edgerun_linux_wifi::discover_wifi_interfaces() {
        Ok(interfaces) => interfaces,
        Err(e) => {
            eprintln!("edgerund: warning: WiFi interface discovery failed: {}", e);
            Vec::new()
        }
    }
}

#[cfg(not(feature = "all-hardware"))]
pub fn discover_wifi_interfaces() -> Vec<edgerun_linux_wifi::LinuxWifiInterface> {
    Vec::new()
}

// ===========================================================================
// USB
// ===========================================================================

#[cfg(feature = "all-hardware")]
pub fn discover_usb_devices() -> Vec<edgerun_linux_usb::LinuxUsbDevice> {
    match edgerun_linux_usb::discover_usb_devices() {
        Ok(devices) => devices,
        Err(e) => {
            eprintln!("edgerund: warning: USB device discovery failed: {}", e);
            Vec::new()
        }
    }
}

#[cfg(not(feature = "all-hardware"))]
pub fn discover_usb_devices() -> Vec<edgerun_linux_usb::LinuxUsbDevice> {
    Vec::new()
}

// ===========================================================================
// PCI
// ===========================================================================

#[cfg(feature = "all-hardware")]
pub fn discover_pci_devices() -> Vec<edgerun_linux_pci::LinuxPciDevice> {
    match edgerun_linux_pci::discover_pci_devices() {
        Ok(devices) => devices,
        Err(e) => {
            eprintln!("edgerund: warning: PCI device discovery failed: {}", e);
            Vec::new()
        }
    }
}

#[cfg(not(feature = "all-hardware"))]
pub fn discover_pci_devices() -> Vec<edgerun_linux_pci::LinuxPciDevice> {
    Vec::new()
}

// ===========================================================================
// NFC
// ===========================================================================

#[cfg(feature = "all-hardware")]
pub fn discover_nfc_adapters() -> Vec<String> {
    let mut adapters = Vec::new();

    match edgerun_linux_nfc::discover_nfc_adapters() {
        Ok(found) => {
            for adapter in found {
                adapters.push(format!("NFC adapter: {}", adapter.name));
            }
        }
        Err(e) => eprintln!("edgerund: warning: NFC adapter discovery failed: {}", e),
    }

    adapters
}

#[cfg(not(feature = "all-hardware"))]
pub fn discover_nfc_adapters() -> Vec<String> {
    Vec::new()
}

// ===========================================================================
// NPU (including AMD xDNA)
// ===========================================================================

#[cfg(feature = "all-hardware")]
pub fn discover_npu_devices() -> Vec<String> {
    let mut devices = Vec::new();

    // Generic Linux NPU (sysfs-based)
    match edgerun_linux_npu::discover_linux_npus() {
        Ok(npus) => {
            for npu in npus {
                devices.push(format!("Linux NPU: {:?}", npu));
            }
        }
        Err(e) => eprintln!("edgerund: warning: Linux NPU discovery failed: {}", e),
    }

    // AMD xDNA NPU
    match edgerun_amd_xdna::discover_amd_xdna_devices() {
        Ok(xdnas) => {
            for xdna in xdnas {
                devices.push(format!("AMD xDNA NPU: {:?}", xdna));
            }
        }
        Err(e) => eprintln!("edgerund: warning: AMD xDNA discovery failed: {}", e),
    }

    devices
}

#[cfg(not(feature = "all-hardware"))]
pub fn discover_npu_devices() -> Vec<String> {
    Vec::new()
}

// ===========================================================================
// Power
// ===========================================================================

#[cfg(feature = "all-hardware")]
pub fn discover_power_supplies() -> Vec<String> {
    let mut supplies = Vec::new();

    match edgerun_linux_power::discover_power_supplies() {
        Ok(psus) => {
            for psu in psus {
                supplies.push(format!("Power supply: {} ({:?})", psu.instance_id, psu.kind));
            }
        }
        Err(e) => eprintln!("edgerund: warning: power supply discovery failed: {}", e),
    }

    // System-level power info (battery, AC, etc.)
    match edgerun_linux_power::discover_power_system() {
        Ok(sys) => {
            if let Some(pct) = sys.battery_percent {
                supplies.push(format!("Battery: {}%", pct));
            }
            if let Some(on_ac) = sys.on_ac_power {
                supplies.push(format!("AC power: {}", if on_ac { "online" } else { "offline" }));
            }
            supplies.push(format!("Lid: {:?}", sys.lid_state));
            for source in &sys.sources {
                supplies.push(format!("Source: {}", source.instance_id));
            }
        }
        Err(e) => eprintln!("edgerund: warning: power system discovery failed: {}", e),
    }

    supplies
}

#[cfg(not(feature = "all-hardware"))]
pub fn discover_power_supplies() -> Vec<String> {
    Vec::new()
}

// ===========================================================================
// CEC (Consumer Electronics Control over HDMI)
// ===========================================================================

#[cfg(feature = "all-hardware")]
pub fn discover_cec_adapters() -> Vec<String> {
    let mut adapters = Vec::new();

    match edgerun_linux_cec::discover_cec_adapters() {
        Ok(found) => {
            for adapter in found {
                adapters.push(format!("CEC adapter: {}", adapter.adapter_name));
            }
        }
        Err(e) => eprintln!("edgerund: warning: CEC adapter discovery failed: {}", e),
    }

    adapters
}

#[cfg(not(feature = "all-hardware"))]
pub fn discover_cec_adapters() -> Vec<String> {
    Vec::new()
}

// ===========================================================================
// Audio Calibration
// ===========================================================================

#[cfg(feature = "all-hardware")]
pub fn run_audio_calibration(
    speaker_card: u32,
    speaker_device: u32,
    mic_card: u32,
    mic_device: u32,
) -> Result<Vec<edgerun_audio_calibration::AudioSweepStepResult>, String> {
    use edgerun_audio_calibration::{AudioSweepConfig, run_speaker_mic_sweep};

    let config = AudioSweepConfig {
        speaker_card,
        speaker_device,
        microphone_card: mic_card,
        microphone_device: mic_device,
        start_level_percent: 10,
        end_level_percent: 100,
        step_percent: 10,
        duration_ms: 500,
        sample_rate_hz: 48_000,
        channels: 2,
        tone_hz: 1000.0,
        software_gain_percent: 100,
        lead_in_ms: 100,
    };

    run_speaker_mic_sweep(&config).map_err(|e| format!("audio calibration failed: {}", e))
}

#[cfg(not(feature = "all-hardware"))]
pub fn run_audio_calibration(
    _speaker_card: u32,
    _speaker_device: u32,
    _mic_card: u32,
    _mic_device: u32,
) -> Result<Vec<edgerun_audio_calibration::AudioSweepStepResult>, String> {
    Err("audio calibration not available".into())
}

// ===========================================================================
// Biometrics
// ===========================================================================

#[cfg(feature = "all-hardware")]
pub fn get_biometric_state() -> edgerun_biometrics::BiometricState {
    edgerun_biometrics::BiometricState::default()
}

#[cfg(not(feature = "all-hardware"))]
pub fn get_biometric_state() -> edgerun_biometrics::BiometricState {
    edgerun_biometrics::BiometricState::default()
}

// ===========================================================================
// Android Keystore
// ===========================================================================

#[cfg(feature = "all-hardware")]
pub fn check_android_keystore_available() -> bool {
    // Check if we're running on Android with KeyStore
    std::path::Path::new("/system/bin/keystore2").exists()
        || std::env::var("ANDROID_DATA").is_ok()
}

#[cfg(not(feature = "all-hardware"))]
pub fn check_android_keystore_available() -> bool {
    false
}

// ===========================================================================
// Aggregated hardware inventory
// ===========================================================================

/// Full hardware inventory for this machine.
pub struct HardwareInventory {
    pub gpus: Vec<edgerun_linux_gpu::LinuxGpuDevice>,
    pub displays: Vec<String>,
    pub fingerprint_readers: Vec<String>,
    pub bluetooth_controllers: Vec<String>,
    pub wifi_interfaces: Vec<String>,
    pub usb_devices: Vec<String>,
    pub pci_devices: Vec<String>,
    pub nfc_adapters: Vec<String>,
    pub npu_devices: Vec<String>,
    pub power_supplies: Vec<String>,
    pub cec_adapters: Vec<String>,
    pub biometric_state: edgerun_biometrics::BiometricState,
    pub android_keystore_available: bool,
}

impl HardwareInventory {
    /// Discover all hardware on this machine.
    pub fn discover() -> Self {
        let gpus = discover_gpus();
        let displays: Vec<String> = discover_displays()
            .into_iter()
            .map(|c| format!("{} (connected={}, enabled={})", c.connector_name, c.connected, c.enabled))
            .collect();
        let fingerprint_readers = discover_fingerprint_readers();
        let bluetooth_controllers = discover_bluetooth_controllers();
        let wifi_interfaces: Vec<String> = discover_wifi_interfaces()
            .into_iter()
            .map(|i| format!("{} ({:?})", i.name, i.operstate))
            .collect();
        let usb_devices: Vec<String> = discover_usb_devices()
            .into_iter()
            .map(|d| {
                let vid = d.vendor_id.map(|v| format!("{:04x}", v)).unwrap_or_else(|| "????".into());
                let pid = d.product_id.map(|p| format!("{:04x}", p)).unwrap_or_else(|| "????".into());
                let name = d.product_name.as_deref().or(d.manufacturer.as_deref()).unwrap_or("unknown");
                format!("{}:{}:{} {}", vid, pid, d.instance_id, name)
            })
            .collect();
        let pci_devices: Vec<String> = discover_pci_devices()
            .into_iter()
            .map(|d| {
                let vid = d.vendor_id.map(|v| format!("{:04x}", v)).unwrap_or_else(|| "????".into());
                let did = d.device_id.map(|p| format!("{:04x}", p)).unwrap_or_else(|| "????".into());
                format!("{} {} (driver: {})", d.address, format!("{}:{}", vid, did), d.driver.as_deref().unwrap_or("none"))
            })
            .collect();
        let nfc_adapters = discover_nfc_adapters();
        let npu_devices = discover_npu_devices();
        let power_supplies = discover_power_supplies();
        let cec_adapters = discover_cec_adapters();
        let biometric_state = get_biometric_state();
        let android_keystore_available = check_android_keystore_available();

        Self {
            gpus,
            displays,
            fingerprint_readers,
            bluetooth_controllers,
            wifi_interfaces,
            usb_devices,
            pci_devices,
            nfc_adapters,
            npu_devices,
            power_supplies,
            cec_adapters,
            biometric_state,
            android_keystore_available,
        }
    }

    /// Print a summary of discovered hardware.
    pub fn summary(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("  GPUs:            {}", self.gpus.len()));
        lines.push(format!("  Displays:        {}", self.displays.len()));
        lines.push(format!("  Fingerprint:     {}", self.fingerprint_readers.len()));
        lines.push(format!("  Bluetooth:       {}", self.bluetooth_controllers.len()));
        lines.push(format!("  WiFi interfaces: {}", self.wifi_interfaces.len()));
        lines.push(format!("  USB devices:     {}", self.usb_devices.len()));
        lines.push(format!("  PCI devices:     {}", self.pci_devices.len()));
        lines.push(format!("  NFC adapters:    {}", self.nfc_adapters.len()));
        lines.push(format!("  NPU devices:     {}", self.npu_devices.len()));
        lines.push(format!("  Power supplies:  {}", self.power_supplies.len()));
        lines.push(format!("  CEC adapters:    {}", self.cec_adapters.len()));
        lines.push(format!("  Android KS:      {}", self.android_keystore_available));
        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hardware_inventory_discover_does_not_panic() {
        let inventory = HardwareInventory::discover();
        // Should always succeed even without real hardware
        assert!(!inventory.summary().is_empty());
    }

    #[test]
    fn biometric_state_default() {
        let state = get_biometric_state();
        // Default should be a valid state
        let _ = state.assurance_strength();
    }
}
