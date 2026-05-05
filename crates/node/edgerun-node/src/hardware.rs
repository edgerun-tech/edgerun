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
            edgerun_log::warn!("edgerund: warning: GPU discovery failed: {}", e);
            Vec::new()
        }
    }
}

#[cfg(feature = "all-hardware")]
pub fn discover_displays() -> Vec<edgerun_drm_display::DrmConnectorInfo> {
    match edgerun_drm_display::discover_drm_connectors() {
        Ok(connectors) => connectors,
        Err(e) => {
            edgerun_log::warn!("edgerund: warning: display discovery failed: {}", e);
            Vec::new()
        }
    }
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
        Err(e) => edgerun_log::warn!(
            "edgerund: warning: Goodix fingerprint discovery failed: {}",
            e
        ),
    }

    readers
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
                controllers.push(format!(
                    "BT controller #{}: {} ({})",
                    ctrl.index, ctrl.name, ctrl.address
                ));
            }
        }
        Err(e) => edgerun_log::warn!("edgerund: warning: BT controller discovery failed: {}", e),
    }

    controllers
}

// ===========================================================================
// WiFi
// ===========================================================================

#[cfg(feature = "all-hardware")]
pub fn discover_wifi_interfaces() -> Vec<edgerun_linux_wifi::LinuxWifiInterface> {
    match edgerun_linux_wifi::discover_wifi_interfaces() {
        Ok(interfaces) => interfaces,
        Err(e) => {
            edgerun_log::warn!("edgerund: warning: WiFi interface discovery failed: {}", e);
            Vec::new()
        }
    }
}

// ===========================================================================
// USB
// ===========================================================================

#[cfg(feature = "all-hardware")]
pub fn discover_usb_devices() -> Vec<edgerun_linux_usb::LinuxUsbDevice> {
    match edgerun_linux_usb::discover_usb_devices() {
        Ok(devices) => devices,
        Err(e) => {
            edgerun_log::warn!("edgerund: warning: USB device discovery failed: {}", e);
            Vec::new()
        }
    }
}

// ===========================================================================
// PCI
// ===========================================================================

#[cfg(feature = "all-hardware")]
pub fn discover_pci_devices() -> Vec<edgerun_linux_pci::LinuxPciDevice> {
    match edgerun_linux_pci::discover_pci_devices() {
        Ok(devices) => devices,
        Err(e) => {
            edgerun_log::warn!("edgerund: warning: PCI device discovery failed: {}", e);
            Vec::new()
        }
    }
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
        Err(e) => edgerun_log::warn!("edgerund: warning: NFC adapter discovery failed: {}", e),
    }

    adapters
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
        Err(e) => edgerun_log::warn!("edgerund: warning: Linux NPU discovery failed: {}", e),
    }

    // AMD xDNA NPU
    match edgerun_amd_xdna::discover_amd_xdna_devices() {
        Ok(xdnas) => {
            for xdna in xdnas {
                devices.push(format!("AMD xDNA NPU: {:?}", xdna));
            }
        }
        Err(e) => edgerun_log::warn!("edgerund: warning: AMD xDNA discovery failed: {}", e),
    }

    devices
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
                supplies.push(format!(
                    "Power supply: {} ({:?})",
                    psu.instance_id, psu.kind
                ));
            }
        }
        Err(e) => edgerun_log::warn!("edgerund: warning: power supply discovery failed: {}", e),
    }

    // System-level power info (battery, AC, etc.)
    match edgerun_linux_power::discover_power_system() {
        Ok(sys) => {
            if let Some(pct) = sys.battery_percent {
                supplies.push(format!("Battery: {}%", pct));
            }
            if let Some(on_ac) = sys.on_ac_power {
                supplies.push(format!(
                    "AC power: {}",
                    if on_ac { "online" } else { "offline" }
                ));
            }
            supplies.push(format!("Lid: {:?}", sys.lid_state));
            for source in &sys.sources {
                supplies.push(format!("Source: {}", source.instance_id));
            }
        }
        Err(e) => edgerun_log::warn!("edgerund: warning: power system discovery failed: {}", e),
    }

    supplies
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
        Err(e) => edgerun_log::warn!("edgerund: warning: CEC adapter discovery failed: {}", e),
    }

    adapters
}

// ===========================================================================
// Input (evdev)
// ===========================================================================

#[cfg(feature = "all-hardware")]
pub fn discover_input_devices() -> Vec<String> {
    match edgerun_evdev_input::discover_evdev_devices() {
        Ok(devices) => devices
            .into_iter()
            .map(|d| {
                let kind = format!("{:?}", d.kind);
                format!("{}: {} ({})", d.event_node, d.device_name, kind)
            })
            .collect(),
        Err(e) => {
            edgerun_log::warn!("edgerund: warning: evdev input discovery failed: {}", e);
            Vec::new()
        }
    }
}

// ===========================================================================
// Audio Input (ALSA microphone)
// ===========================================================================

#[cfg(feature = "all-hardware")]
pub fn discover_audio_input() -> Vec<String> {
    match edgerun_alsa_microphone::discover_alsa_pcms() {
        Ok(pcms) => pcms
            .into_iter()
            .filter(|p| p.capture)
            .map(|p| format!("ALSA PCM {}:{} ({})", p.card_index, p.device_index, p.name))
            .collect(),
        Err(e) => {
            edgerun_log::warn!("edgerund: warning: ALSA microphone discovery failed: {}", e);
            Vec::new()
        }
    }
}

// ===========================================================================
// Audio Output (ALSA speaker)
// ===========================================================================

#[cfg(feature = "all-hardware")]
pub fn discover_audio_output() -> Vec<String> {
    match edgerun_alsa_speaker::discover_speakers() {
        Ok(speakers) => speakers
            .into_iter()
            .map(|s| {
                format!(
                    "ALSA {} card={} device={} ({}ch, {}Hz)",
                    s.card_id, s.card_index, s.device_index, s.channels, s.default_sample_rate_hz
                )
            })
            .collect(),
        Err(e) => {
            edgerun_log::warn!("edgerund: warning: ALSA speaker discovery failed: {}", e);
            Vec::new()
        }
    }
}

// ===========================================================================
// Camera (V4L2)
// ===========================================================================

#[cfg(feature = "all-hardware")]
pub fn discover_cameras() -> Vec<String> {
    match edgerun_v4l2_camera::discover_camera_devices() {
        Ok(cameras) => cameras
            .into_iter()
            .map(|c| {
                let caps = match c.query_info() {
                    Ok(info) => {
                        let mut parts = Vec::new();
                        if info.supports_video_capture() {
                            parts.push("capture");
                        }
                        if info.supports_streaming() {
                            parts.push("streaming");
                        }
                        if parts.is_empty() {
                            parts.push("unknown");
                        }
                        parts.join(", ")
                    }
                    Err(_) => "unknown".into(),
                };
                format!("{} ({})", c.devnode.display(), caps)
            })
            .collect(),
        Err(e) => {
            edgerun_log::warn!("edgerund: warning: V4L2 camera discovery failed: {}", e);
            Vec::new()
        }
    }
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
    use edgerun_audio_calibration::{run_speaker_mic_sweep, AudioSweepConfig};

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

// ===========================================================================
// Biometrics
// ===========================================================================

#[cfg(feature = "all-hardware")]
pub fn get_biometric_state() -> edgerun_biometrics::BiometricState {
    edgerun_biometrics::BiometricState::default()
}

// ===========================================================================
// Android Keystore
// ===========================================================================

/// Check if we're running on Android with KeyStore.
/// Uses only stdlib, always available.
pub fn check_android_keystore_available() -> bool {
    // Check if we're running on Android with KeyStore
    std::path::Path::new("/system/bin/keystore2").exists() || std::env::var("ANDROID_DATA").is_ok()
}

// ===========================================================================
// Aggregated hardware inventory
// ===========================================================================

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
}

impl HardwareInventory {
    /// Discover all hardware on Linux machines (requires `all-hardware` feature).
    #[cfg(all(not(target_os = "android"), feature = "all-hardware"))]
    pub fn discover() -> Self {
        // On Linux: use existing Linux drivers
        let gpus = discover_gpus();
        let displays: Vec<String> = discover_displays()
            .into_iter()
            .map(|c| {
                format!(
                    "{} (connected={}, enabled={})",
                    c.connector_name, c.connected, c.enabled
                )
            })
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
                let vid = d
                    .vendor_id
                    .map(|v| format!("{:04x}", v))
                    .unwrap_or_else(|| "????".into());
                let pid = d
                    .product_id
                    .map(|p| format!("{:04x}", p))
                    .unwrap_or_else(|| "????".into());
                let name = d
                    .product_name
                    .as_deref()
                    .or(d.manufacturer.as_deref())
                    .unwrap_or("unknown");
                format!("{}:{}:{} {}", vid, pid, d.instance_id, name)
            })
            .collect();
        let pci_devices: Vec<String> = discover_pci_devices()
            .into_iter()
            .map(|d| {
                let vid = d
                    .vendor_id
                    .map(|v| format!("{:04x}", v))
                    .unwrap_or_else(|| "????".into());
                let did = d
                    .device_id
                    .map(|p| format!("{:04x}", p))
                    .unwrap_or_else(|| "????".into());
                format!(
                    "{} {} (driver: {})",
                    d.address,
                    format!("{}:{}", vid, did),
                    d.driver.as_deref().unwrap_or("none")
                )
            })
            .collect();
        let nfc_adapters = discover_nfc_adapters();
        let npu_devices = discover_npu_devices();
        let power_supplies = discover_power_supplies();
        let cec_adapters = discover_cec_adapters();
        let input_devices = discover_input_devices();
        let audio_input = discover_audio_input();
        let audio_output = discover_audio_output();
        let camera = discover_cameras();

        Self {
            platform: "linux",
            gpus: gpus
                .iter()
                .map(|g| {
                    format!(
                        "GPU: {} ({}:{})",
                        g.pci_address,
                        g.vendor_id
                            .map(|v| format!("{:04x}", v))
                            .unwrap_or_else(|| "????".into()),
                        g.device_id
                            .map(|d| format!("{:04x}", d))
                            .unwrap_or_else(|| "????".into())
                    )
                })
                .collect(),
            displays,
            input_devices,
            audio_input,
            audio_output,
            sensors: vec![],
            camera,
            fingerprint_readers,
            bluetooth_controllers,
            wifi_interfaces,
            usb_devices,
            pci_devices,
            nfc_adapters,
            npu_devices,
            power_supplies,
            cec_adapters,
            biometric: vec![],
            location: vec![],
            keystore: if check_android_keystore_available() {
                vec!["available".into()]
            } else {
                vec![]
            },
        }
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
            keystore: if check_android_keystore_available() {
                vec!["available".into()]
            } else {
                vec![]
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hardware_inventory_discover_does_not_panic() {
        let inventory = HardwareInventory::discover();
        assert!(!inventory.summary().is_empty());
        #[cfg(not(target_os = "android"))]
        assert_eq!(inventory.platform, "linux");
        #[cfg(target_os = "android")]
        assert_eq!(inventory.platform, "android");
    }
}
