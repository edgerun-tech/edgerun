#![no_std]

extern crate alloc;

#[cfg(not(target_os = "none"))]
extern crate std;

#[cfg(target_os = "none")]
extern crate self as std;

#[cfg(target_os = "none")]
pub use edgerun_linux_sysfs::prelude;
#[cfg(target_os = "none")]
pub use edgerun_linux_sysfs::{collections, fs, io, mem, option, os, path, result, string, vec};

use edgerun_capabilities::{CapabilityDescriptor, CapabilityError, CapabilityProvider};
use edgerun_linux_sysfs::prelude::v1::*;
use edgerun_linux_sysfs::{parse_hex_u16, parse_hex_u8, parse_u32, parse_u8, read_trimmed};
use edgerun_usb::{
    default_usb_descriptor, UsbDeviceInfo, UsbInterfaceInfo, UsbInventory, UsbSpeed,
};
use std::collections::{BTreeMap as HashMap, BTreeSet as HashSet};
#[cfg(not(target_os = "none"))]
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinuxUsbDevice {
    pub instance_id: String,
    pub sysfs_path: PathBuf,
    pub bus_num: Option<u32>,
    pub dev_num: Option<u32>,
    pub vendor_id: Option<u16>,
    pub product_id: Option<u16>,
    pub manufacturer: Option<String>,
    pub product_name: Option<String>,
    pub serial_number: Option<String>,
    pub usb_version: Option<String>,
    pub configuration_value: Option<u8>,
    pub configuration_name: Option<String>,
    pub port_path: Option<String>,
    pub max_children: Option<u32>,
    pub speed: UsbSpeed,
    pub driver: Option<String>,
    pub parent_instance_id: Option<String>,
    pub child_instance_ids: Vec<String>,
    pub interfaces: Vec<UsbInterfaceInfo>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinuxUsbBackend {
    pub root_path: PathBuf,
}

fn parse_speed(text: Option<String>) -> UsbSpeed {
    match text.as_deref() {
        Some("1.5") => UsbSpeed::Low,
        Some("12") | Some("12.0") => UsbSpeed::Full,
        Some("480") | Some("480.0") => UsbSpeed::High,
        Some("5000") | Some("5000.0") => UsbSpeed::Super,
        Some("10000") | Some("10000.0") | Some("20000") | Some("20000.0") => UsbSpeed::SuperPlus,
        _ => UsbSpeed::Unknown,
    }
}

fn is_usb_device_dir(name: &str, path: &Path) -> bool {
    if !path.is_dir() || !name.contains('-') {
        return false;
    }
    path.join("idVendor").exists() || path.join("busnum").exists()
}

fn infer_parent_instance_id(instance_id: &str) -> Option<String> {
    let (_bus, ports) = instance_id.split_once('-')?;
    let (parent_ports, _) = ports.rsplit_once('.')?;
    Some(format!(
        "{}-{}",
        instance_id.split_once('-')?.0,
        parent_ports
    ))
}

fn is_usb_interface_dir(name: &str, path: &Path) -> bool {
    path.is_dir() && name.contains(':') && path.join("bInterfaceClass").exists()
}

fn parse_usb_interface(instance_id: String, path: &Path) -> UsbInterfaceInfo {
    let driver = fs::read_link(path.join("driver"))
        .ok()
        .and_then(|p| p.file_name().map(|v| v.to_string_lossy().to_string()));
    let interface_number = parse_u8(read_trimmed(&path.join("bInterfaceNumber")));
    let alternate_setting = instance_id
        .rsplit_once('.')
        .and_then(|(_, alt)| alt.parse::<u8>().ok());
    UsbInterfaceInfo {
        instance_id,
        interface_number,
        alternate_setting,
        interface_class: parse_hex_u8(read_trimmed(&path.join("bInterfaceClass"))),
        interface_subclass: parse_hex_u8(read_trimmed(&path.join("bInterfaceSubClass"))),
        interface_protocol: parse_hex_u8(read_trimmed(&path.join("bInterfaceProtocol"))),
        interface_name: read_trimmed(&path.join("interface")),
        driver,
    }
}

pub fn discover_usb_devices() -> Result<Vec<LinuxUsbDevice>, CapabilityError> {
    discover_usb_devices_in(Path::new("/sys/bus/usb/devices"))
}

pub fn discover_usb_devices_in(root: &Path) -> Result<Vec<LinuxUsbDevice>, CapabilityError> {
    let mut devices = Vec::new();
    let mut interfaces_by_device: HashMap<String, Vec<UsbInterfaceInfo>> = HashMap::new();
    let entries = match fs::read_dir(root) {
        Ok(v) => v,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => {
            return Err(CapabilityError::Provider(format!(
                "failed to read usb sysfs: {e}"
            )))
        }
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let instance_id = entry.file_name().to_string_lossy().to_string();
        if is_usb_interface_dir(&instance_id, &path) {
            if let Some((device_id, _)) = instance_id.split_once(':') {
                interfaces_by_device
                    .entry(device_id.to_string())
                    .or_default()
                    .push(parse_usb_interface(instance_id, &path));
            }
            continue;
        }
        if !is_usb_device_dir(&instance_id, &path) {
            continue;
        }
        let driver = fs::read_link(path.join("driver"))
            .ok()
            .and_then(|p| p.file_name().map(|v| v.to_string_lossy().to_string()));
        let parent_instance_id = infer_parent_instance_id(&instance_id);
        devices.push(LinuxUsbDevice {
            instance_id,
            sysfs_path: path.clone(),
            bus_num: parse_u32(read_trimmed(&path.join("busnum"))),
            dev_num: parse_u32(read_trimmed(&path.join("devnum"))),
            vendor_id: parse_hex_u16(read_trimmed(&path.join("idVendor"))),
            product_id: parse_hex_u16(read_trimmed(&path.join("idProduct"))),
            manufacturer: read_trimmed(&path.join("manufacturer")),
            product_name: read_trimmed(&path.join("product")),
            serial_number: read_trimmed(&path.join("serial")),
            usb_version: read_trimmed(&path.join("version"))
                .or_else(|| read_trimmed(&path.join("bcdUSB"))),
            configuration_value: parse_u8(read_trimmed(&path.join("bConfigurationValue"))),
            configuration_name: read_trimmed(&path.join("configuration")),
            port_path: read_trimmed(&path.join("devpath")),
            max_children: parse_u32(read_trimmed(&path.join("maxchild"))),
            speed: parse_speed(read_trimmed(&path.join("speed"))),
            driver,
            parent_instance_id,
            child_instance_ids: Vec::new(),
            interfaces: Vec::new(),
        });
    }
    let known: HashSet<String> = devices.iter().map(|v| v.instance_id.clone()).collect();
    for device in &mut devices {
        if !device
            .parent_instance_id
            .as_ref()
            .is_some_and(|v| known.contains(v))
        {
            device.parent_instance_id = None;
        }
    }
    let mut children: HashMap<String, Vec<String>> = HashMap::new();
    for device in &devices {
        if let Some(parent) = &device.parent_instance_id {
            children
                .entry(parent.clone())
                .or_default()
                .push(device.instance_id.clone());
        }
    }
    for device in &mut devices {
        if let Some(ids) = children.get(&device.instance_id) {
            let mut ids = ids.clone();
            ids.sort();
            device.child_instance_ids = ids;
        }
        if let Some(interfaces) = interfaces_by_device.remove(&device.instance_id) {
            let mut interfaces = interfaces;
            interfaces.sort_by(|a, b| a.instance_id.cmp(&b.instance_id));
            device.interfaces = interfaces;
        }
    }
    devices.sort_by(|a, b| a.instance_id.cmp(&b.instance_id));
    Ok(devices)
}

impl CapabilityProvider for LinuxUsbBackend {
    fn descriptor(&self) -> CapabilityDescriptor {
        default_usb_descriptor("linux-usb", &self.root_path.to_string_lossy())
    }
}

impl UsbInventory for LinuxUsbBackend {
    fn list_devices(&self) -> Result<Vec<UsbDeviceInfo>, CapabilityError> {
        Ok(discover_usb_devices_in(&self.root_path)?
            .into_iter()
            .map(|v| UsbDeviceInfo {
                provider: "linux-usb".into(),
                instance_id: v.instance_id,
                bus_num: v.bus_num,
                dev_num: v.dev_num,
                vendor_id: v.vendor_id,
                product_id: v.product_id,
                manufacturer: v.manufacturer,
                product_name: v.product_name,
                serial_number: v.serial_number,
                usb_version: v.usb_version,
                configuration_value: v.configuration_value,
                configuration_name: v.configuration_name,
                port_path: v.port_path,
                max_children: v.max_children,
                speed: v.speed,
                driver: v.driver,
                parent_instance_id: v.parent_instance_id,
                child_instance_ids: v.child_instance_ids,
                interfaces: v.interfaces,
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    use edgerun_linux_sysfs::temp_root;

    #[test]
    fn discover_usb_device_from_sysfs() {
        let root = temp_root("edgerun-linux-usb");
        fs::create_dir_all(root.join("1-2")).unwrap();
        fs::create_dir_all(root.join("1-2:1.0")).unwrap();
        fs::write(root.join("1-2/busnum"), "1\n").unwrap();
        fs::write(root.join("1-2/devnum"), "3\n").unwrap();
        fs::write(root.join("1-2/idVendor"), "27c6\n").unwrap();
        fs::write(root.join("1-2/idProduct"), "609c\n").unwrap();
        fs::write(root.join("1-2/manufacturer"), "Goodix\n").unwrap();
        fs::write(root.join("1-2/product"), "Fingerprint Reader\n").unwrap();
        fs::write(root.join("1-2/speed"), "12\n").unwrap();
        fs::write(root.join("1-2/version"), "2.00\n").unwrap();
        fs::write(root.join("1-2/bConfigurationValue"), "1\n").unwrap();
        fs::write(root.join("1-2/configuration"), "Default\n").unwrap();
        fs::write(root.join("1-2/devpath"), "2\n").unwrap();
        fs::write(root.join("1-2/maxchild"), "0\n").unwrap();
        fs::write(root.join("1-2:1.0/bInterfaceNumber"), "0\n").unwrap();
        fs::write(root.join("1-2:1.0/bInterfaceClass"), "ff\n").unwrap();
        fs::write(root.join("1-2:1.0/bInterfaceSubClass"), "00\n").unwrap();
        fs::write(root.join("1-2:1.0/bInterfaceProtocol"), "00\n").unwrap();
        fs::write(root.join("1-2:1.0/interface"), "Vendor Iface\n").unwrap();
        let devices = discover_usb_devices_in(&root).unwrap();
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].vendor_id, Some(0x27c6));
        assert_eq!(devices[0].product_id, Some(0x609c));
        assert_eq!(devices[0].speed, UsbSpeed::Full);
        assert_eq!(devices[0].usb_version.as_deref(), Some("2.00"));
        assert_eq!(devices[0].configuration_value, Some(1));
        assert_eq!(devices[0].interfaces.len(), 1);
        assert_eq!(devices[0].interfaces[0].interface_class, Some(0xff));
    }

    #[test]
    fn parent_child_relationships_are_built() {
        let root = temp_root("edgerun-linux-usb");
        fs::create_dir_all(root.join("1-4")).unwrap();
        fs::write(root.join("1-4/idVendor"), "1d6b\n").unwrap();
        fs::write(root.join("1-4/busnum"), "1\n").unwrap();
        fs::create_dir_all(root.join("1-4.1")).unwrap();
        fs::write(root.join("1-4.1/idVendor"), "046d\n").unwrap();
        fs::write(root.join("1-4.1/busnum"), "1\n").unwrap();
        let mut devices = discover_usb_devices_in(&root).unwrap();
        devices.sort_by(|a, b| a.instance_id.cmp(&b.instance_id));
        assert_eq!(devices[0].instance_id, "1-4");
        assert_eq!(devices[0].child_instance_ids, vec!["1-4.1"]);
        assert_eq!(devices[1].parent_instance_id.as_deref(), Some("1-4"));
    }

    #[test]
    fn infer_parent_from_nested_port_path() {
        assert_eq!(infer_parent_instance_id("1-4"), None);
        assert_eq!(infer_parent_instance_id("1-4.1").as_deref(), Some("1-4"));
        assert_eq!(
            infer_parent_instance_id("3-6.3.2").as_deref(),
            Some("3-6.3")
        );
    }

    // --- parse_speed ---

    #[test]
    fn parse_speed_low() {
        assert_eq!(parse_speed(Some("1.5".to_string())), UsbSpeed::Low);
    }

    #[test]
    fn parse_speed_full() {
        assert_eq!(parse_speed(Some("12".to_string())), UsbSpeed::Full);
        assert_eq!(parse_speed(Some("12.0".to_string())), UsbSpeed::Full);
    }

    #[test]
    fn parse_speed_high() {
        assert_eq!(parse_speed(Some("480".to_string())), UsbSpeed::High);
        assert_eq!(parse_speed(Some("480.0".to_string())), UsbSpeed::High);
    }

    #[test]
    fn parse_speed_super() {
        assert_eq!(parse_speed(Some("5000".to_string())), UsbSpeed::Super);
        assert_eq!(parse_speed(Some("5000.0".to_string())), UsbSpeed::Super);
    }

    #[test]
    fn parse_speed_super_plus() {
        assert_eq!(parse_speed(Some("10000".to_string())), UsbSpeed::SuperPlus);
        assert_eq!(parse_speed(Some("20000".to_string())), UsbSpeed::SuperPlus);
    }

    #[test]
    fn parse_speed_unknown() {
        assert_eq!(parse_speed(None), UsbSpeed::Unknown);
        assert_eq!(parse_speed(Some("999".to_string())), UsbSpeed::Unknown);
    }

    // --- is_usb_device_dir ---

    #[test]
    fn is_usb_device_dir_true_with_vendor() {
        let root = temp_root("edgerun-linux-usb-check");
        let path = root.join("1-2");
        fs::create_dir_all(&path).unwrap();
        fs::write(path.join("idVendor"), "1234\n").unwrap();
        assert!(is_usb_device_dir("1-2", &path));
    }

    #[test]
    fn is_usb_device_dir_false_no_dash() {
        let root = temp_root("edgerun-linux-usb-check2");
        let path = root.join("usb1");
        fs::create_dir_all(&path).unwrap();
        assert!(!is_usb_device_dir("usb1", &path));
    }

    #[test]
    fn is_usb_device_dir_false_not_dir() {
        let root = temp_root("edgerun-linux-usb-check3");
        assert!(!is_usb_device_dir("1-2", &root.join("nonexistent")));
    }

    // --- is_usb_interface_dir ---

    #[test]
    fn is_usb_interface_dir_true() {
        let root = temp_root("edgerun-linux-usb-iface");
        let path = root.join("1-2:1.0");
        fs::create_dir_all(&path).unwrap();
        fs::write(path.join("bInterfaceClass"), "ff\n").unwrap();
        assert!(is_usb_interface_dir("1-2:1.0", &path));
    }

    #[test]
    fn is_usb_interface_dir_false_no_colon() {
        let root = temp_root("edgerun-linux-usb-iface2");
        assert!(!is_usb_interface_dir("1-2", &root));
    }

    // --- LinuxUsbDevice ---

    #[test]
    fn linux_usb_device_construction() {
        let device = LinuxUsbDevice {
            instance_id: "1-2".to_string(),
            sysfs_path: PathBuf::from("/sys/bus/usb/devices/1-2"),
            bus_num: Some(1),
            dev_num: Some(2),
            vendor_id: Some(0x1234),
            product_id: Some(0x5678),
            manufacturer: None,
            product_name: None,
            serial_number: None,
            usb_version: None,
            configuration_value: None,
            configuration_name: None,
            port_path: None,
            max_children: None,
            speed: UsbSpeed::Unknown,
            driver: None,
            parent_instance_id: None,
            child_instance_ids: vec![],
            interfaces: vec![],
        };
        assert_eq!(device.vendor_id, Some(0x1234));
        assert!(device.interfaces.is_empty());
    }

    #[test]
    fn linux_usb_device_clone() {
        let device = LinuxUsbDevice {
            instance_id: "i".to_string(),
            sysfs_path: PathBuf::from("/sys"),
            bus_num: None,
            dev_num: None,
            vendor_id: None,
            product_id: None,
            manufacturer: None,
            product_name: None,
            serial_number: None,
            usb_version: None,
            configuration_value: None,
            configuration_name: None,
            port_path: None,
            max_children: None,
            speed: UsbSpeed::Unknown,
            driver: None,
            parent_instance_id: None,
            child_instance_ids: vec![],
            interfaces: vec![],
        };
        assert_eq!(device.clone(), device);
    }

    // --- LinuxUsbBackend ---

    #[test]
    fn linux_usb_backend_descriptor() {
        let backend = LinuxUsbBackend {
            root_path: PathBuf::from("/sys/bus/usb/devices"),
        };
        let desc = backend.descriptor();
        assert_eq!(desc.provider_name, "linux-usb");
        assert!(desc.provider_instance_id.contains("sys/bus/usb/devices"));
    }

    #[test]
    fn linux_usb_backend_list_devices() {
        let root = temp_root("edgerun-linux-usb-list");
        fs::create_dir_all(root.join("2-1")).unwrap();
        fs::write(root.join("2-1/idVendor"), "abcd\n").unwrap();
        fs::write(root.join("2-1/busnum"), "2\n").unwrap();
        let backend = LinuxUsbBackend {
            root_path: root.clone(),
        };
        let devices = backend.list_devices().unwrap();
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].vendor_id, Some(0xabcd));
        assert_eq!(devices[0].provider, "linux-usb");
    }

    // --- discover_usb_devices_in missing root ---

    #[test]
    fn discover_usb_devices_in_missing_root() {
        let result = discover_usb_devices_in(Path::new("/nonexistent/path/usb")).unwrap();
        assert!(result.is_empty());
    }

    // --- parse_usb_interface ---

    #[test]
    fn parse_usb_interface_basic() {
        let root = temp_root("edgerun-linux-usb-parse-iface");
        let path = root.join("1-2:1.0");
        fs::create_dir_all(&path).unwrap();
        fs::write(path.join("bInterfaceNumber"), "01\n").unwrap();
        fs::write(path.join("bInterfaceClass"), "08\n").unwrap();
        fs::write(path.join("bInterfaceSubClass"), "06\n").unwrap();
        fs::write(path.join("bInterfaceProtocol"), "50\n").unwrap();
        fs::write(path.join("interface"), "Mass Storage\n").unwrap();
        let iface = parse_usb_interface("1-2:1.0".to_string(), &path);
        assert_eq!(iface.instance_id, "1-2:1.0");
        assert_eq!(iface.interface_number, Some(1));
        assert_eq!(iface.interface_class, Some(0x08));
        assert_eq!(iface.interface_subclass, Some(0x06));
        assert_eq!(iface.interface_protocol, Some(0x50));
        assert_eq!(iface.interface_name.as_deref(), Some("Mass Storage"));
    }

    // --- discover_usb_devices_in sorts results ---

    #[test]
    fn discover_usb_devices_in_sorted() {
        let root = temp_root("edgerun-linux-usb-sort");
        fs::create_dir_all(root.join("1-3")).unwrap();
        fs::write(root.join("1-3/idVendor"), "aaaa\n").unwrap();
        fs::write(root.join("1-3/busnum"), "1\n").unwrap();
        fs::create_dir_all(root.join("1-2")).unwrap();
        fs::write(root.join("1-2/idVendor"), "bbbb\n").unwrap();
        fs::write(root.join("1-2/busnum"), "1\n").unwrap();
        let devices = discover_usb_devices_in(&root).unwrap();
        assert_eq!(devices.len(), 2);
        assert_eq!(devices[0].instance_id, "1-2");
        assert_eq!(devices[1].instance_id, "1-3");
    }
}
