use lifegraph_capabilities::{CapabilityDescriptor, CapabilityError, CapabilityProvider};
use lifegraph_nfc::{default_nfc_descriptor, NfcDevice, NfcDeviceInfo, NfcPowerState};
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

fn read_trimmed(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok().map(|v| v.trim().to_string())
}

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
        out.push(LinuxNfcAdapter {
            name,
            sysfs_path: path.clone(),
            protocol_name: read_trimmed(&path.join("protocols")),
            power_state,
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

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
            protocol_name: self.adapter.protocol_name.clone(),
        })
    }

    fn power_state(&self) -> Result<NfcPowerState, CapabilityError> {
        Ok(self.adapter.power_state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn tempdir() -> PathBuf {
        let base = std::env::temp_dir().join(format!(
            "lifegraph-linux-nfc-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&base).unwrap();
        base
    }

    #[test]
    fn discover_nfc_adapter_from_sysfs() {
        let root = tempdir();
        fs::create_dir_all(root.join("nfc0/power")).unwrap();
        fs::write(root.join("nfc0/power/control"), "on\n").unwrap();
        fs::write(root.join("nfc0/protocols"), "iso14443\n").unwrap();
        let found = discover_nfc_adapters_in(&root).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "nfc0");
        assert_eq!(found[0].power_state, NfcPowerState::Enabled);
    }
}
