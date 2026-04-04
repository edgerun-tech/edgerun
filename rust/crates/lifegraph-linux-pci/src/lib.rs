use lifegraph_capabilities::{CapabilityDescriptor, CapabilityError, CapabilityProvider};
use lifegraph_pci::{default_pci_descriptor, PciDeviceInfo, PciInventory};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinuxPciDevice {
    pub address: String,
    pub sysfs_path: PathBuf,
    pub vendor_id: Option<u16>,
    pub device_id: Option<u16>,
    pub subsystem_vendor_id: Option<u16>,
    pub subsystem_device_id: Option<u16>,
    pub class_code: Option<u32>,
    pub revision: Option<u8>,
    pub driver: Option<String>,
    pub numa_node: Option<i32>,
    pub current_link_speed: Option<String>,
    pub current_link_width: Option<u32>,
    pub max_link_speed: Option<String>,
    pub max_link_width: Option<u32>,
    pub parent_address: Option<String>,
    pub child_addresses: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinuxPciBackend {
    pub root_path: PathBuf,
}

fn read_trimmed(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok().map(|v| v.trim().to_string())
}

fn parse_hex_u16(text: Option<String>) -> Option<u16> {
    text.and_then(|v| u16::from_str_radix(v.trim_start_matches("0x"), 16).ok())
}

fn parse_hex_u32(text: Option<String>) -> Option<u32> {
    text.and_then(|v| u32::from_str_radix(v.trim_start_matches("0x"), 16).ok())
}

fn parse_hex_u8(text: Option<String>) -> Option<u8> {
    text.and_then(|v| u8::from_str_radix(v.trim_start_matches("0x"), 16).ok())
}

fn parse_u32(text: Option<String>) -> Option<u32> {
    text.and_then(|v| v.parse::<u32>().ok())
}

fn parse_i32(text: Option<String>) -> Option<i32> {
    text.and_then(|v| v.parse::<i32>().ok())
}

fn infer_parent_address(path: &Path) -> Option<String> {
    let canonical = fs::canonicalize(path).ok()?;
    let parent = canonical.parent()?;
    let name = parent.file_name()?.to_string_lossy().to_string();
    if is_pci_address(&name) {
        Some(name)
    } else {
        None
    }
}

fn is_pci_address(name: &str) -> bool {
    let bytes = name.as_bytes();
    bytes.len() == 12 && bytes[4] == b':' && bytes[7] == b':' && bytes[10] == b'.'
}

pub fn discover_pci_devices() -> Result<Vec<LinuxPciDevice>, CapabilityError> {
    discover_pci_devices_in(Path::new("/sys/bus/pci/devices"))
}

pub fn discover_pci_devices_in(root: &Path) -> Result<Vec<LinuxPciDevice>, CapabilityError> {
    let mut devices = Vec::new();
    let entries = match fs::read_dir(root) {
        Ok(v) => v,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => {
            return Err(CapabilityError::Provider(format!(
                "failed to read pci sysfs: {e}"
            )))
        }
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let address = entry.file_name().to_string_lossy().to_string();
        if !is_pci_address(&address) {
            continue;
        }
        let driver = fs::read_link(path.join("driver"))
            .ok()
            .and_then(|p| p.file_name().map(|v| v.to_string_lossy().to_string()));
        devices.push(LinuxPciDevice {
            address,
            sysfs_path: path.clone(),
            vendor_id: parse_hex_u16(read_trimmed(&path.join("vendor"))),
            device_id: parse_hex_u16(read_trimmed(&path.join("device"))),
            subsystem_vendor_id: parse_hex_u16(read_trimmed(&path.join("subsystem_vendor"))),
            subsystem_device_id: parse_hex_u16(read_trimmed(&path.join("subsystem_device"))),
            class_code: parse_hex_u32(read_trimmed(&path.join("class"))),
            revision: parse_hex_u8(read_trimmed(&path.join("revision"))),
            driver,
            numa_node: parse_i32(read_trimmed(&path.join("numa_node"))),
            current_link_speed: read_trimmed(&path.join("current_link_speed")),
            current_link_width: parse_u32(read_trimmed(&path.join("current_link_width"))),
            max_link_speed: read_trimmed(&path.join("max_link_speed")),
            max_link_width: parse_u32(read_trimmed(&path.join("max_link_width"))),
            parent_address: infer_parent_address(&path),
            child_addresses: Vec::new(),
        });
    }
    let known: HashSet<String> = devices.iter().map(|v| v.address.clone()).collect();
    for device in &mut devices {
        if !device
            .parent_address
            .as_ref()
            .is_some_and(|v| known.contains(v))
        {
            device.parent_address = None;
        }
    }
    let mut children: HashMap<String, Vec<String>> = HashMap::new();
    for device in &devices {
        if let Some(parent) = &device.parent_address {
            children
                .entry(parent.clone())
                .or_default()
                .push(device.address.clone());
        }
    }
    for device in &mut devices {
        if let Some(ids) = children.get(&device.address) {
            let mut ids = ids.clone();
            ids.sort();
            device.child_addresses = ids;
        }
    }
    devices.sort_by(|a, b| a.address.cmp(&b.address));
    Ok(devices)
}

impl CapabilityProvider for LinuxPciBackend {
    fn descriptor(&self) -> CapabilityDescriptor {
        default_pci_descriptor("linux-pci", &self.root_path.to_string_lossy())
    }
}

impl PciInventory for LinuxPciBackend {
    fn list_devices(&self) -> Result<Vec<PciDeviceInfo>, CapabilityError> {
        Ok(discover_pci_devices_in(&self.root_path)?
            .into_iter()
            .map(|v| PciDeviceInfo {
                provider: "linux-pci".into(),
                address: v.address,
                vendor_id: v.vendor_id,
                device_id: v.device_id,
                subsystem_vendor_id: v.subsystem_vendor_id,
                subsystem_device_id: v.subsystem_device_id,
                class_code: v.class_code,
                revision: v.revision,
                driver: v.driver,
                numa_node: v.numa_node,
                current_link_speed: v.current_link_speed,
                current_link_width: v.current_link_width,
                max_link_speed: v.max_link_speed,
                max_link_width: v.max_link_width,
                parent_address: v.parent_address,
                child_addresses: v.child_addresses,
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn tempdir() -> PathBuf {
        let base = std::env::temp_dir().join(format!(
            "lifegraph-linux-pci-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&base).unwrap();
        base
    }

    #[test]
    fn discover_pci_device_from_sysfs() {
        let root = tempdir();
        fs::create_dir_all(root.join("0000:01:00.0")).unwrap();
        fs::write(root.join("0000:01:00.0/vendor"), "0x144d\n").unwrap();
        fs::write(root.join("0000:01:00.0/device"), "0xa80a\n").unwrap();
        fs::write(root.join("0000:01:00.0/class"), "0x010802\n").unwrap();
        fs::write(root.join("0000:01:00.0/revision"), "01\n").unwrap();
        let devices = discover_pci_devices_in(&root).unwrap();
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].vendor_id, Some(0x144d));
        assert_eq!(devices[0].class_code, Some(0x010802));
        assert_eq!(devices[0].revision, Some(0x01));
    }
}
