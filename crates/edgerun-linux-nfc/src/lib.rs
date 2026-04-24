use edgerun_capabilities::{CapabilityDescriptor, CapabilityError, CapabilityProvider};
use edgerun_linux_sysfs::read_trimmed;
use edgerun_nfc::{
    default_nfc_descriptor, NdefMessage, NfcDevice, NfcDeviceInfo, NfcPowerState, NfcReader,
    NfcScanner, NfcTarget, NfcTechnology,
};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinuxNfcAdapter {
    pub name: String,
    pub sysfs_path: PathBuf,
    pub protocol_name: Option<String>,
    pub power_state: NfcPowerState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinuxNfcBackend {
    pub adapter: LinuxNfcAdapter,
}

// ---------------------------------------------------------------------------
// Target discovery via sysfs
// ---------------------------------------------------------------------------

pub fn discover_nfc_adapters() -> Result<Vec<LinuxNfcAdapter>, CapabilityError> {
    discover_nfc_adapters_in(Path::new("/sys/class/nfc"))
}

pub fn discover_nfc_adapters_in(root: &Path) -> Result<Vec<LinuxNfcAdapter>, CapabilityError> {
    let mut out = Vec::new();
    let entries = match fs::read_dir(root) {
        Ok(v) => v,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => {
            return Err(CapabilityError::Provider(format!(
                "failed to read nfc sysfs: {e}"
            )))
        }
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let power_state = match read_trimmed(&path.join("power/control")).as_deref() {
            Some("on") => NfcPowerState::Enabled,
            Some("auto") => NfcPowerState::Disabled,
            _ => NfcPowerState::Unknown,
        };
        let protocols = read_trimmed(&path.join("protocols"));
        let _supported_technologies = parse_supported_technologies(&protocols);
        out.push(LinuxNfcAdapter {
            name,
            sysfs_path: path.clone(),
            protocol_name: protocols,
            power_state,
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

fn parse_supported_technologies(protocols: &Option<String>) -> Vec<NfcTechnology> {
    let Some(protocols) = protocols else { return Vec::new() };
    protocols
        .split_whitespace()
        .map(NfcTechnology::parse)
        .filter(|t| *t != NfcTechnology::Unknown)
        .collect()
}

// ---------------------------------------------------------------------------
// CapabilityProvider + NfcDevice
// ---------------------------------------------------------------------------

impl CapabilityProvider for LinuxNfcBackend {
    fn descriptor(&self) -> CapabilityDescriptor {
        default_nfc_descriptor("linux-nfc", &self.adapter.name)
    }
}

impl NfcDevice for LinuxNfcBackend {
    fn device_info(&self) -> Result<NfcDeviceInfo, CapabilityError> {
        Ok(NfcDeviceInfo {
            provider: "linux-nfc".into(),
            device_name: self.adapter.name.clone(),
            power_state: self.adapter.power_state,
            supported_technologies: parse_supported_technologies(&self.adapter.protocol_name),
        })
    }

    fn power_state(&self) -> Result<NfcPowerState, CapabilityError> {
        Ok(self.adapter.power_state)
    }

    fn set_power_state(&self, _state: NfcPowerState) -> Result<NfcPowerState, CapabilityError> {
        // Linux kernel NFC subsystem doesn't expose a standard sysfs power toggle.
        // Power management is handled by the kernel driver or rfkill.
        Err(CapabilityError::Unsupported(
            "nfc power state control requires rfkill or kernel netlink interface",
        ))
    }
}

// ---------------------------------------------------------------------------
// NfcScanner — requires libnfc or kernel NFC target discovery
// ---------------------------------------------------------------------------

impl NfcScanner for LinuxNfcBackend {
    fn scan_targets(&self) -> Result<Vec<NfcTarget>, CapabilityError> {
        // Linux kernel NFC target discovery is not exposed via sysfs.
        // Requires libnfc (nfc_initiator_list_passive_target) or
        // /dev/nfcX character device with ioctl.
        Err(CapabilityError::Unsupported(
            "nfc target scanning requires libnfc or /dev/nfcX character device — \
             implement with nfc_initiator_list_passive_target() from libnfc",
        ))
    }
}

// ---------------------------------------------------------------------------
// NfcReader — requires libnfc for NDEF read/write and APDU transceive
// ---------------------------------------------------------------------------

impl NfcReader for LinuxNfcBackend {
    fn read_ndef(&self, _target_id: &str) -> Result<Option<NdefMessage>, CapabilityError> {
        Err(CapabilityError::Unsupported(
            "ndef read requires libnfc or /dev/nfcX — \
             implement with nfc_initiator_transceive_bytes() + NDEF TLV parsing",
        ))
    }

    fn write_ndef(&self, _target_id: &str, _message: &NdefMessage) -> Result<(), CapabilityError> {
        Err(CapabilityError::Unsupported(
            "ndef write requires libnfc or /dev/nfcX — \
             implement with nfc_initiator_transceive_bytes() + NDEF TLV encoding",
        ))
    }

    fn transceive(&self, _target_id: &str, _command: &[u8]) -> Result<Vec<u8>, CapabilityError> {
        Err(CapabilityError::Unsupported(
            "raw apdu transceive requires libnfc or /dev/nfcX — \
             implement with nfc_initiator_transceive_bytes()",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgerun_linux_sysfs::temp_root;

    #[test]
    fn discover_nfc_adapter_from_sysfs() {
        let root = temp_root("edgerun-linux-nfc");
        fs::create_dir_all(root.join("nfc0/power")).unwrap();
        fs::write(root.join("nfc0/power/control"), "on\n").unwrap();
        fs::write(root.join("nfc0/protocols"), "iso14443\n").unwrap();
        let found = discover_nfc_adapters_in(&root).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "nfc0");
        assert_eq!(found[0].power_state, NfcPowerState::Enabled);
    }

    #[test]
    fn discover_nfc_adapters_returns_empty_for_missing_dir() {
        let result = discover_nfc_adapters_in(Path::new("/nonexistent/nfc/path"));
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn discover_nfc_adapters_empty_directory() {
        let root = temp_root("edgerun-linux-nfc-empty");
        let found = discover_nfc_adapters_in(&root).unwrap();
        assert!(found.is_empty());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn discover_nfc_adapters_multiple_sorted() {
        let root = temp_root("edgerun-linux-nfc-multi");
        fs::create_dir_all(root.join("nfc2/power")).unwrap();
        fs::write(root.join("nfc2/power/control"), "on\n").unwrap();
        fs::create_dir_all(root.join("nfc0/power")).unwrap();
        fs::write(root.join("nfc0/power/control"), "auto\n").unwrap();
        fs::create_dir_all(root.join("nfc1/power")).unwrap();
        fs::write(root.join("nfc1/power/control"), "on\n").unwrap();

        let found = discover_nfc_adapters_in(&root).unwrap();
        assert_eq!(found.len(), 3);
        assert_eq!(found[0].name, "nfc0");
        assert_eq!(found[0].power_state, NfcPowerState::Disabled);
        assert_eq!(found[1].name, "nfc1");
        assert_eq!(found[1].power_state, NfcPowerState::Enabled);
        assert_eq!(found[2].name, "nfc2");
        assert_eq!(found[2].power_state, NfcPowerState::Enabled);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn discover_nfc_power_state_unknown() {
        let root = temp_root("edgerun-linux-nfc-unknown");
        fs::create_dir_all(root.join("nfc0/power")).unwrap();
        fs::write(root.join("nfc0/power/control"), "suspend\n").unwrap();

        let found = discover_nfc_adapters_in(&root).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].power_state, NfcPowerState::Unknown);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn linux_nfc_adapter_clone_and_eq() {
        let adapter = LinuxNfcAdapter {
            name: "nfc0".into(),
            sysfs_path: PathBuf::from("/sys/class/nfc/nfc0"),
            protocol_name: Some("iso14443".into()),
            power_state: NfcPowerState::Enabled,
        };
        let cloned = adapter.clone();
        assert_eq!(adapter, cloned);
    }

    #[test]
    fn linux_nfc_backend_descriptor() {
        let adapter = LinuxNfcAdapter {
            name: "nfc0".into(),
            sysfs_path: PathBuf::from("/sys/class/nfc/nfc0"),
            protocol_name: None,
            power_state: NfcPowerState::Disabled,
        };
        let backend = LinuxNfcBackend { adapter };
        let descriptor = backend.descriptor();
        assert_eq!(descriptor.provider_name, "linux-nfc");
        assert_eq!(descriptor.provider_instance_id, "nfc0");
    }

    #[test]
    fn linux_nfc_backend_device_info() {
        let adapter = LinuxNfcAdapter {
            name: "nfc1".into(),
            sysfs_path: PathBuf::from("/sys/class/nfc/nfc1"),
            protocol_name: Some("felica".into()),
            power_state: NfcPowerState::Enabled,
        };
        let backend = LinuxNfcBackend { adapter };
        let info = backend.device_info().unwrap();
        assert_eq!(info.provider, "linux-nfc");
        assert_eq!(info.device_name, "nfc1");
        assert_eq!(info.power_state, NfcPowerState::Enabled);
        assert!(info.supported_technologies.contains(&NfcTechnology::NfcF));
    }

    #[test]
    fn linux_nfc_backend_power_state() {
        let adapter = LinuxNfcAdapter {
            name: "nfc0".into(),
            sysfs_path: PathBuf::from("/sys/class/nfc/nfc0"),
            protocol_name: None,
            power_state: NfcPowerState::Unknown,
        };
        let backend = LinuxNfcBackend { adapter };
        assert_eq!(backend.power_state().unwrap(), NfcPowerState::Unknown);
    }

    #[test]
    fn linux_nfc_adapter_debug() {
        let adapter = LinuxNfcAdapter {
            name: "nfc0".into(),
            sysfs_path: PathBuf::from("/sys/class/nfc/nfc0"),
            protocol_name: Some("iso14443".into()),
            power_state: NfcPowerState::Enabled,
        };
        let debug = format!("{:?}", adapter);
        assert!(debug.contains("nfc0"));
        assert!(debug.contains("Enabled"));
    }

    #[test]
    fn linux_nfc_backend_clone() {
        let adapter = LinuxNfcAdapter {
            name: "nfc0".into(),
            sysfs_path: PathBuf::from("/sys/class/nfc/nfc0"),
            protocol_name: None,
            power_state: NfcPowerState::Disabled,
        };
        let backend = LinuxNfcBackend { adapter };
        let cloned = backend.clone();
        assert_eq!(backend, cloned);
    }
}
