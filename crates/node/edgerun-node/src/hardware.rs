//! Hardware discovery for all platform capability backends.
//!
//! Discovers GPUs, displays, fingerprint readers, Bluetooth, WiFi, USB, PCI,
//! NFC, NPU, power supplies, CEC adapters, and audio calibration devices.
//! Each discovery block is gated behind `#[cfg(feature = "all-hardware")]`
//! and wrapped in error handlers so missing hardware is silently skipped.

use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

#[cfg(not(feature = "all-hardware"))]
use edgerun_capabilities::CapabilityDescriptor;
#[cfg(feature = "all-hardware")]
use edgerun_capabilities::{CapabilityDescriptor, CapabilityProvider};

mod inventory;
pub use inventory::HardwareInventory;

const SYS_BUS_PCI_DEVICES: &str = "/sys/bus/pci/devices";
const SYS_CLASS_DRM: &str = "/sys/class/drm";
const SYS_BUS_USB_DEVICES: &str = "/sys/bus/usb/devices";
const SYS_CLASS_POWER_SUPPLY: &str = "/sys/class/power_supply";
const PROC_ACPI: &str = "/proc/acpi";

// ===========================================================================
// GPU & Display
// ===========================================================================

#[cfg(feature = "all-hardware")]
pub fn discover_gpus() -> Vec<edgerun_linux_gpu::LinuxGpuDevice> {
    match edgerun_linux_gpu::discover_gpus() {
        Ok(backends) => backends,
        Err(e) => {
            crate::node_warn!("edged: warning: GPU discovery failed: {}", e);
            Vec::new()
        }
    }
}

#[cfg(feature = "all-hardware")]
pub fn discover_displays() -> Vec<edgerun_drm_display::DrmConnectorInfo> {
    match edgerun_drm_display::discover_drm_connectors() {
        Ok(connectors) => connectors,
        Err(e) => {
            crate::node_warn!("edged: warning: display discovery failed: {}", e);
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
        Err(e) => crate::node_warn!("edged: warning: Goodix fingerprint discovery failed: {}", e),
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
        Err(e) => crate::node_warn!("edged: warning: BT controller discovery failed: {}", e),
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
            crate::node_warn!("edged: warning: WiFi interface discovery failed: {}", e);
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
            crate::node_warn!("edged: warning: USB device discovery failed: {}", e);
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
            crate::node_warn!("edged: warning: PCI device discovery failed: {}", e);
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
        Err(e) => crate::node_warn!("edged: warning: NFC adapter discovery failed: {}", e),
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
        Err(e) => crate::node_warn!("edged: warning: Linux NPU discovery failed: {}", e),
    }

    // AMD xDNA NPU
    match edgerun_amd_xdna::discover_amd_xdna_devices() {
        Ok(xdnas) => {
            for xdna in xdnas {
                devices.push(format!("AMD xDNA NPU: {:?}", xdna));
            }
        }
        Err(e) => crate::node_warn!("edged: warning: AMD xDNA discovery failed: {}", e),
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
        Err(e) => crate::node_warn!("edged: warning: power supply discovery failed: {}", e),
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
        Err(e) => crate::node_warn!("edged: warning: power system discovery failed: {}", e),
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
        Err(e) => crate::node_warn!("edged: warning: CEC adapter discovery failed: {}", e),
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
            crate::node_warn!("edged: warning: evdev input discovery failed: {}", e);
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
            crate::node_warn!("edged: warning: ALSA microphone discovery failed: {}", e);
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
            crate::node_warn!("edged: warning: ALSA speaker discovery failed: {}", e);
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
            crate::node_warn!("edged: warning: V4L2 camera discovery failed: {}", e);
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

// ===========================================================================
// Biometrics
// ===========================================================================

#[cfg(feature = "all-hardware")]
pub fn get_biometric_state() -> edgerun_devices::biometrics::BiometricState {
    edgerun_devices::biometrics::BiometricState::default()
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

impl HardwareInventory {
    /// Discover all hardware on Linux machines (requires `all-hardware` feature).
    #[cfg(all(not(target_os = "android"), feature = "all-hardware"))]
    pub fn discover() -> Self {
        let mut capability_descriptors = Vec::new();
        let gpus = discover_gpus();
        capability_descriptors.push(
            edgerun_linux_gpu::LinuxGpuBackend {
                pci_root: SYS_BUS_PCI_DEVICES.into(),
                drm_root: SYS_CLASS_DRM.into(),
            }
            .descriptor(),
        );
        let displays_raw = discover_displays();
        let displays: Vec<String> = displays_raw
            .iter()
            .map(|c| {
                capability_descriptors.push(
                    edgerun_drm_display::DrmDisplayBackend {
                        sysfs_root: SYS_CLASS_DRM.into(),
                        connector: c.clone(),
                    }
                    .descriptor(),
                );
                format!(
                    "{} (connected={}, enabled={})",
                    c.connector_name, c.connected, c.enabled
                )
            })
            .collect();

        let fingerprint_devices = match edgerun_goodix_fingerprint::discover_supported_devices() {
            Ok(devices) => devices,
            Err(e) => {
                crate::node_warn!("edged: warning: Goodix fingerprint discovery failed: {}", e);
                Vec::new()
            }
        };
        let fingerprint_readers: Vec<String> = fingerprint_devices
            .into_iter()
            .map(|device| {
                let summary = format!(
                    "Goodix USB: bus={}, dev={}, {:04x}:{:04x}",
                    device.bus_number, device.device_number, device.vendor_id, device.product_id
                );
                if let Ok(reader) = edgerun_goodix_fingerprint::GoodixFingerprintReader::new(device)
                {
                    capability_descriptors.push(reader.descriptor());
                }
                summary
            })
            .collect();

        let bluetooth_raw = match edgerun_mgmt_bluetooth::discover_controllers() {
            Ok(ctrls) => ctrls,
            Err(e) => {
                crate::node_warn!("edged: warning: BT controller discovery failed: {}", e);
                Vec::new()
            }
        };
        let bluetooth_controllers: Vec<String> = bluetooth_raw
            .into_iter()
            .map(|ctrl| {
                capability_descriptors.push(
                    edgerun_mgmt_bluetooth::MgmtBluetoothBackend {
                        controller: ctrl.clone(),
                    }
                    .descriptor(),
                );
                format!(
                    "BT controller #{}: {} ({})",
                    ctrl.index, ctrl.name, ctrl.address
                )
            })
            .collect();

        let wifi_interfaces: Vec<String> = discover_wifi_interfaces()
            .into_iter()
            .map(|i| {
                capability_descriptors.push(
                    edgerun_linux_wifi::LinuxWifiBackend {
                        interface: i.clone(),
                    }
                    .descriptor(),
                );
                format!("{} ({:?})", i.name, i.operstate)
            })
            .collect();
        capability_descriptors.push(
            edgerun_linux_usb::LinuxUsbBackend {
                root_path: SYS_BUS_USB_DEVICES.into(),
            }
            .descriptor(),
        );
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
        capability_descriptors.push(
            edgerun_linux_pci::LinuxPciBackend {
                root_path: SYS_BUS_PCI_DEVICES.into(),
            }
            .descriptor(),
        );
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
        let nfc_adapters: Vec<String> = match edgerun_linux_nfc::discover_nfc_adapters() {
            Ok(found) => found
                .into_iter()
                .map(|adapter| {
                    capability_descriptors.push(
                        edgerun_linux_nfc::LinuxNfcBackend {
                            adapter: adapter.clone(),
                        }
                        .descriptor(),
                    );
                    format!("NFC adapter: {}", adapter.name)
                })
                .collect(),
            Err(e) => {
                crate::node_warn!("edged: warning: NFC adapter discovery failed: {}", e);
                Vec::new()
            }
        };

        let mut npu_devices = Vec::new();
        match edgerun_linux_npu::discover_linux_npus() {
            Ok(npus) => {
                for npu in npus {
                    capability_descriptors.push(
                        edgerun_linux_npu::LinuxNpuBackend { info: npu.clone() }.descriptor(),
                    );
                    npu_devices.push(format!("Linux NPU: {:?}", npu));
                }
            }
            Err(e) => crate::node_warn!("edged: warning: Linux NPU discovery failed: {}", e),
        }
        match edgerun_amd_xdna::discover_amd_xdna_devices() {
            Ok(xdnas) => {
                for xdna in xdnas {
                    capability_descriptors
                        .push(edgerun_amd_xdna::AmdXdnaBackend { info: xdna.clone() }.descriptor());
                    npu_devices.push(format!("AMD xDNA NPU: {:?}", xdna));
                }
            }
            Err(e) => crate::node_warn!("edged: warning: AMD xDNA discovery failed: {}", e),
        }

        capability_descriptors.push(
            edgerun_linux_power::LinuxPowerBackend {
                power_supply_root: SYS_CLASS_POWER_SUPPLY.into(),
                proc_acpi_root: PROC_ACPI.into(),
            }
            .descriptor(),
        );
        let mut power_supplies = Vec::new();
        match edgerun_linux_power::discover_power_supplies() {
            Ok(psus) => {
                for psu in psus {
                    power_supplies.push(format!(
                        "Power supply: {} ({:?})",
                        psu.instance_id, psu.kind
                    ));
                }
            }
            Err(e) => crate::node_warn!("edged: warning: power supply discovery failed: {}", e),
        }
        match edgerun_linux_power::discover_power_system() {
            Ok(sys) => {
                if let Some(pct) = sys.battery_percent {
                    power_supplies.push(format!("Battery: {}%", pct));
                }
                if let Some(on_ac) = sys.on_ac_power {
                    power_supplies.push(format!(
                        "AC power: {}",
                        if on_ac { "online" } else { "offline" }
                    ));
                }
                power_supplies.push(format!("Lid: {:?}", sys.lid_state));
                for source in &sys.sources {
                    power_supplies.push(format!("Source: {}", source.instance_id));
                }
            }
            Err(e) => crate::node_warn!("edged: warning: power system discovery failed: {}", e),
        }

        let cec_adapters: Vec<String> = match edgerun_linux_cec::discover_cec_adapters() {
            Ok(found) => found
                .into_iter()
                .map(|adapter| {
                    capability_descriptors.push(edgerun_devices::cec::default_cec_descriptor(
                        "linux-cec",
                        &adapter.instance_id,
                    ));
                    format!("CEC adapter: {}", adapter.adapter_name)
                })
                .collect(),
            Err(e) => {
                crate::node_warn!("edged: warning: CEC adapter discovery failed: {}", e);
                Vec::new()
            }
        };

        let input_devices: Vec<String> = match edgerun_evdev_input::discover_evdev_devices() {
            Ok(devices) => devices
                .into_iter()
                .map(|d| {
                    capability_descriptors.push(edgerun_devices::input::default_input_descriptor(
                        "evdev-kernel",
                        &d.event_node,
                    ));
                    let kind = format!("{:?}", d.kind);
                    format!("{}: {} ({})", d.event_node, d.device_name, kind)
                })
                .collect(),
            Err(e) => {
                crate::node_warn!("edged: warning: evdev input discovery failed: {}", e);
                Vec::new()
            }
        };

        let audio_input: Vec<String> = match edgerun_alsa_microphone::discover_alsa_pcms() {
            Ok(pcms) => pcms
                .into_iter()
                .filter(|p| p.capture)
                .map(|p| {
                    capability_descriptors.push(
                        edgerun_alsa_microphone::AlsaMicrophoneBackend {
                            device_path: format!(
                                "/dev/snd/pcmC{}D{}c",
                                p.card_index, p.device_index
                            ),
                            pcm: p.clone(),
                        }
                        .descriptor(),
                    );
                    format!("ALSA PCM {}:{} ({})", p.card_index, p.device_index, p.name)
                })
                .collect(),
            Err(e) => {
                crate::node_warn!("edged: warning: ALSA microphone discovery failed: {}", e);
                Vec::new()
            }
        };
        let audio_output: Vec<String> = match edgerun_alsa_speaker::discover_speakers() {
            Ok(speakers) => speakers
                .into_iter()
                .map(|s| {
                    capability_descriptors.push(s.descriptor());
                    format!(
                        "ALSA {} card={} device={} ({}ch, {}Hz)",
                        s.card_id,
                        s.card_index,
                        s.device_index,
                        s.channels,
                        s.default_sample_rate_hz
                    )
                })
                .collect(),
            Err(e) => {
                crate::node_warn!("edged: warning: ALSA speaker discovery failed: {}", e);
                Vec::new()
            }
        };
        let camera: Vec<String> = match edgerun_v4l2_camera::discover_camera_devices() {
            Ok(cameras) => cameras
                .into_iter()
                .map(|c| {
                    capability_descriptors.push(
                        edgerun_v4l2_camera::V4l2CameraBiometricReader::new(c.clone()).descriptor(),
                    );
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
                crate::node_warn!("edged: warning: V4L2 camera discovery failed: {}", e);
                Vec::new()
            }
        };
        capability_descriptors.sort_by(|a, b| {
            a.provider_name
                .cmp(&b.provider_name)
                .then(a.provider_instance_id.cmp(&b.provider_instance_id))
        });

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
            capability_descriptors,
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
        let provider_apps = inventory.capability_provider_apps([1; 32]);
        assert_eq!(provider_apps.len(), inventory.capability_descriptors.len());
        assert!(
            provider_apps
                .iter()
                .all(|app| !app.app_id.iter().all(|b| *b == 0))
        );
        assert!(
            provider_apps
                .iter()
                .all(|app| !app.provided_capabilities.is_empty())
        );
        #[cfg(not(target_os = "android"))]
        assert_eq!(inventory.platform, "linux");
        #[cfg(target_os = "android")]
        assert_eq!(inventory.platform, "android");
    }

    #[test]
    fn hardware_inventory_merges_provider_apps_into_runtime_plan() {
        let inventory = HardwareInventory::discover();
        let config = crate::runtime::runtime_deployment_config(
            crate::runtime::sha256(b"runtime"),
            [127, 0, 0, 1],
            b"runtime.local".to_vec(),
            b"local".to_vec(),
            b"admin@local".to_vec(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );
        let plan = inventory.runtime_service_plan(&config, [2; 32]);
        assert_eq!(plan.apps.len(), inventory.capability_descriptors.len());
    }
}
