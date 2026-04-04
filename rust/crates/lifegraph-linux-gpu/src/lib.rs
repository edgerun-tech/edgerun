use lifegraph_capabilities::{CapabilityDescriptor, CapabilityError, CapabilityProvider};
use lifegraph_gpu::{
    default_gpu_descriptor, infer_gpu_vendor, GpuConnectorInfo, GpuDisplayMode, GpuInfo,
    GpuInventory, GpuVendor,
};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

type DrmNodeMap = HashMap<String, DrmNodeSet>;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct DrmNodeSet {
    cards: Vec<String>,
    render_nodes: Vec<String>,
    card_device_nodes: Vec<String>,
    render_device_nodes: Vec<String>,
    connectors: Vec<LinuxGpuConnector>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinuxGpuConnector {
    pub name: String,
    pub connector_id: Option<u32>,
    pub connected: bool,
    pub enabled: bool,
    pub current_mode: Option<GpuDisplayMode>,
    pub modes: Vec<GpuDisplayMode>,
    pub cec_adapter: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinuxGpuDevice {
    pub pci_address: String,
    pub sysfs_path: PathBuf,
    pub vendor_id: Option<u16>,
    pub device_id: Option<u16>,
    pub subsystem_vendor_id: Option<u16>,
    pub subsystem_device_id: Option<u16>,
    pub class_code: Option<u32>,
    pub revision: Option<u8>,
    pub driver: Option<String>,
    pub driver_module: Option<String>,
    pub modalias: Option<String>,
    pub numa_node: Option<i32>,
    pub current_link_speed: Option<String>,
    pub current_link_width: Option<u32>,
    pub max_link_speed: Option<String>,
    pub max_link_width: Option<u32>,
    pub boot_vga: bool,
    pub drm_cards: Vec<String>,
    pub drm_render_nodes: Vec<String>,
    pub drm_card_device_nodes: Vec<String>,
    pub drm_render_device_nodes: Vec<String>,
    pub connectors: Vec<LinuxGpuConnector>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinuxGpuBackend {
    pub pci_root: PathBuf,
    pub drm_root: PathBuf,
}

pub fn discover_gpus() -> Result<Vec<LinuxGpuDevice>, CapabilityError> {
    let mut gpus = discover_gpus_in(
        Path::new("/sys/bus/pci/devices"),
        Path::new("/sys/class/drm"),
    )?;
    attach_cec_adapters_to_gpus(&mut gpus);
    Ok(gpus)
}

pub fn discover_gpus_in(
    pci_root: &Path,
    drm_root: &Path,
) -> Result<Vec<LinuxGpuDevice>, CapabilityError> {
    let drm_map = build_drm_map(drm_root)?;
    let mut devices = Vec::new();
    let entries = match fs::read_dir(pci_root) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => {
            return Err(CapabilityError::Provider(format!(
                "failed to read PCI root: {err}"
            )))
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let address = entry.file_name().to_string_lossy().to_string();
        if !is_pci_address(&address) {
            continue;
        }
        let class_code = parse_hex_u32(read_trimmed(&path.join("class")));
        if !is_gpu_class(class_code) {
            continue;
        }

        let mut drm = drm_map.get(&address).cloned().unwrap_or_default();
        drm.cards.sort();
        drm.render_nodes.sort();
        drm.card_device_nodes.sort();
        drm.render_device_nodes.sort();
        drm.connectors.sort_by(|a, b| a.name.cmp(&b.name));

        let uevent = parse_uevent_map(&path.join("uevent"));
        let driver_link = fs::read_link(path.join("driver")).ok();
        let driver = driver_link
            .as_ref()
            .and_then(|link| link_name(link.as_path()))
            .or_else(|| uevent.get("DRIVER").cloned());
        let driver_module = resolve_driver_module(&path, driver_link.as_ref());

        devices.push(LinuxGpuDevice {
            pci_address: address,
            sysfs_path: path.clone(),
            vendor_id: parse_hex_u16(read_trimmed(&path.join("vendor"))),
            device_id: parse_hex_u16(read_trimmed(&path.join("device"))),
            subsystem_vendor_id: parse_hex_u16(read_trimmed(&path.join("subsystem_vendor"))),
            subsystem_device_id: parse_hex_u16(read_trimmed(&path.join("subsystem_device"))),
            class_code,
            revision: parse_hex_u8(read_trimmed(&path.join("revision"))),
            driver,
            driver_module,
            modalias: read_trimmed(&path.join("modalias"))
                .or_else(|| uevent.get("MODALIAS").cloned()),
            numa_node: parse_i32(read_trimmed(&path.join("numa_node"))),
            current_link_speed: read_trimmed(&path.join("current_link_speed")),
            current_link_width: parse_u32(read_trimmed(&path.join("current_link_width"))),
            max_link_speed: read_trimmed(&path.join("max_link_speed")),
            max_link_width: parse_u32(read_trimmed(&path.join("max_link_width"))),
            boot_vga: parse_bool_flag(read_trimmed(&path.join("boot_vga"))),
            drm_cards: drm.cards,
            drm_render_nodes: drm.render_nodes,
            drm_card_device_nodes: drm.card_device_nodes,
            drm_render_device_nodes: drm.render_device_nodes,
            connectors: drm.connectors,
        });
    }

    devices.sort_by(|a, b| a.pci_address.cmp(&b.pci_address));
    Ok(devices)
}

impl CapabilityProvider for LinuxGpuBackend {
    fn descriptor(&self) -> CapabilityDescriptor {
        default_gpu_descriptor("linux-gpu", &self.pci_root.to_string_lossy())
    }
}

impl GpuInventory for LinuxGpuBackend {
    fn list_gpus(&self) -> Result<Vec<GpuInfo>, CapabilityError> {
        let mut gpus = discover_gpus_in(&self.pci_root, &self.drm_root)?;
        attach_cec_adapters_to_gpus(&mut gpus);
        Ok(gpus.into_iter().map(LinuxGpuDevice::into_info).collect())
    }
}

impl LinuxGpuDevice {
    pub fn into_info(self) -> GpuInfo {
        let vendor = infer_gpu_vendor(self.vendor_id);
        let supports_render = !self.drm_render_nodes.is_empty() || !self.drm_cards.is_empty();
        let supports_display = self.class_code.is_some_and(is_display_class)
            || !self.drm_cards.is_empty()
            || !self.connectors.is_empty();
        let supports_compute = matches!(
            vendor,
            GpuVendor::Amd
                | GpuVendor::Intel
                | GpuVendor::Nvidia
                | GpuVendor::Qualcomm
                | GpuVendor::Apple
                | GpuVendor::Arm
                | GpuVendor::Virtio
        ) || !self.drm_render_nodes.is_empty();
        let instance_id = if !self.pci_address.is_empty() {
            self.pci_address.clone()
        } else {
            self.sysfs_path.display().to_string()
        };

        GpuInfo {
            provider: "linux-gpu".into(),
            instance_id,
            vendor,
            vendor_id: self.vendor_id,
            device_id: self.device_id,
            subsystem_vendor_id: self.subsystem_vendor_id,
            subsystem_device_id: self.subsystem_device_id,
            pci_address: Some(self.pci_address),
            drm_cards: self.drm_cards,
            drm_render_nodes: self.drm_render_nodes,
            drm_card_device_nodes: self.drm_card_device_nodes,
            drm_render_device_nodes: self.drm_render_device_nodes,
            connectors: self
                .connectors
                .into_iter()
                .map(|connector| GpuConnectorInfo {
                    name: connector.name,
                    connector_id: connector.connector_id,
                    connected: connector.connected,
                    enabled: connector.enabled,
                    current_mode: connector.current_mode,
                    modes: connector.modes,
                    cec_adapter: connector.cec_adapter,
                })
                .collect(),
            driver: self.driver,
            driver_module: self.driver_module,
            modalias: self.modalias,
            class_code: self.class_code,
            revision: self.revision,
            numa_node: self.numa_node,
            current_link_speed: self.current_link_speed,
            current_link_width: self.current_link_width,
            max_link_speed: self.max_link_speed,
            max_link_width: self.max_link_width,
            boot_vga: self.boot_vga,
            supports_display,
            supports_render,
            supports_compute,
            is_virtual: matches!(vendor, GpuVendor::Virtio | GpuVendor::Microsoft),
        }
    }
}

fn build_drm_map(root: &Path) -> Result<DrmNodeMap, CapabilityError> {
    let mut map = HashMap::new();
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(map),
        Err(err) => {
            return Err(CapabilityError::Provider(format!(
                "failed to read DRM root: {err}"
            )))
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let device_link = match fs::canonicalize(path.join("device")) {
            Ok(link) => link,
            Err(_) => continue,
        };
        let Some(pci_address) = infer_pci_address_from_drm_device_path(&device_link) else {
            continue;
        };

        let entry = map.entry(pci_address).or_insert_with(DrmNodeSet::default);
        if name.starts_with("card") && !name.contains('-') {
            entry.cards.push(name.clone());
            if let Some(device_node) = drm_device_node_for(&name) {
                entry.card_device_nodes.push(device_node);
            }
            continue;
        }
        if name.starts_with("renderD") {
            entry.render_nodes.push(name.clone());
            if let Some(device_node) = drm_device_node_for(&name) {
                entry.render_device_nodes.push(device_node);
            }
            continue;
        }
        if let Some(connector_name) = parse_connector_name(&name) {
            let modes = fs::read_to_string(path.join("modes"))
                .ok()
                .map(|raw| parse_modes(&raw))
                .unwrap_or_default();
            let current_mode =
                read_trimmed(&path.join("mode")).and_then(|line| parse_mode_line(&line));
            entry.connectors.push(LinuxGpuConnector {
                name: connector_name,
                connector_id: parse_u32(read_trimmed(&path.join("connector_id"))),
                connected: read_trimmed(&path.join("status")).is_some_and(|v| v == "connected"),
                enabled: read_trimmed(&path.join("enabled")).is_some_and(|v| v == "enabled"),
                current_mode,
                modes,
                cec_adapter: None,
            });
        }
    }

    Ok(map)
}

fn infer_pci_address_from_drm_device_path(path: &Path) -> Option<String> {
    for component in path.components().rev() {
        let value = component.as_os_str().to_string_lossy();
        if is_pci_address(&value) {
            return Some(value.to_string());
        }
    }
    None
}

fn parse_connector_name(name: &str) -> Option<String> {
    let (_, suffix) = name.split_once('-')?;
    Some(suffix.to_string())
}

fn parse_mode_line(line: &str) -> Option<GpuDisplayMode> {
    let mut parts = line.split('x');
    let width = parts.next()?.trim().parse().ok()?;
    let rest = parts.next()?;
    let mut rest_parts = rest.split(['i', 'p', '@']);
    let height: u32 = rest_parts.next()?.trim().parse().ok()?;
    let refresh = line
        .split('@')
        .nth(1)
        .and_then(|value| {
            value
                .trim_end_matches('H')
                .trim_end_matches('z')
                .parse::<f32>()
                .ok()
        })
        .map(|hz| (hz * 1000.0) as u32)
        .unwrap_or(60_000);
    Some(GpuDisplayMode {
        width,
        height,
        refresh_millihz: refresh,
    })
}

fn parse_modes(contents: &str) -> Vec<GpuDisplayMode> {
    contents.lines().filter_map(parse_mode_line).collect()
}

fn resolve_driver_module(path: &Path, driver_link: Option<&PathBuf>) -> Option<String> {
    driver_link
        .and_then(|link| fs::read_link(link.join("module")).ok())
        .as_ref()
        .and_then(|link| link_name(link.as_path()))
        .or_else(|| {
            fs::read_link(path.join("driver/module"))
                .ok()
                .as_ref()
                .and_then(|link| link_name(link.as_path()))
        })
}

fn drm_device_node_for(name: &str) -> Option<String> {
    let device_node = Path::new("/dev/dri").join(name);
    device_node
        .exists()
        .then(|| device_node.display().to_string())
}

fn parse_uevent_map(path: &Path) -> HashMap<String, String> {
    let Some(raw) = fs::read_to_string(path).ok() else {
        return HashMap::new();
    };
    raw.lines()
        .filter_map(|line| line.split_once('='))
        .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        .collect()
}

fn link_name(path: &Path) -> Option<String> {
    path.file_name()
        .map(|value| value.to_string_lossy().to_string())
}

fn is_gpu_class(class_code: Option<u32>) -> bool {
    class_code.is_some_and(|code| matches!(code >> 16, 0x03 | 0x12))
}

fn is_display_class(class_code: u32) -> bool {
    (class_code >> 16) == 0x03
}

fn read_trimmed(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok().map(|v| v.trim().to_string())
}

fn parse_bool_flag(text: Option<String>) -> bool {
    matches!(
        text.as_deref(),
        Some("1") | Some("y") | Some("yes") | Some("true") | Some("enabled")
    )
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

fn is_pci_address(name: &str) -> bool {
    let bytes = name.as_bytes();
    bytes.len() == 12 && bytes[4] == b':' && bytes[7] == b':' && bytes[10] == b'.'
}

pub fn cec_adapter_lookup() -> HashMap<(u32, u32), String> {
    cec_adapter_lookup_in(Path::new("/sys/class/cec"))
}

pub fn cec_adapter_lookup_in(root: &Path) -> HashMap<(u32, u32), String> {
    let mut out = HashMap::new();
    let Ok(entries) = fs::read_dir(root) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let instance_id = entry.file_name().to_string_lossy().to_string();
        if !instance_id.starts_with("cec") {
            continue;
        }
        let card_no = parse_u32(read_trimmed(&path.join("device/connector/card-no")));
        let connector_id = parse_u32(read_trimmed(&path.join("device/connector/connector-id")));
        if let (Some(card_no), Some(connector_id)) = (card_no, connector_id) {
            out.insert((card_no, connector_id), instance_id);
        }
    }
    out
}

pub fn attach_cec_adapters_to_gpus(gpus: &mut [LinuxGpuDevice]) {
    let lookup = cec_adapter_lookup();
    attach_cec_adapters_to_gpus_with_lookup(gpus, &lookup);
}

pub fn attach_cec_adapters_to_gpus_with_lookup(
    gpus: &mut [LinuxGpuDevice],
    lookup: &HashMap<(u32, u32), String>,
) {
    for gpu in gpus {
        let card_no = gpu.drm_cards.iter().find_map(|card| {
            card.strip_prefix("card")
                .and_then(|value| value.parse::<u32>().ok())
        });
        let Some(card_no) = card_no else {
            continue;
        };
        for connector in &mut gpu.connectors {
            connector.cec_adapter = connector
                .connector_id
                .and_then(|connector_id| lookup.get(&(card_no, connector_id)).cloned());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("{name}-{unique}"));
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn discover_gpu_from_pci_and_drm_sysfs() {
        let pci_root = temp_root("lifegraph-gpu-pci");
        let drm_root = temp_root("lifegraph-gpu-drm");
        let gpu = pci_root.join("0000:01:00.0");
        fs::create_dir_all(&gpu).unwrap();
        fs::write(gpu.join("vendor"), "0x10de\n").unwrap();
        fs::write(gpu.join("device"), "0x1cb3\n").unwrap();
        fs::write(gpu.join("class"), "0x030000\n").unwrap();
        fs::write(gpu.join("revision"), "0xa1\n").unwrap();
        fs::write(gpu.join("current_link_width"), "16\n").unwrap();
        fs::write(gpu.join("max_link_width"), "16\n").unwrap();
        fs::write(gpu.join("boot_vga"), "1\n").unwrap();
        fs::write(
            gpu.join("uevent"),
            "DRIVER=nvidia\nMODALIAS=pci:v000010DEd00001CB3bc03sc00i00\n",
        )
        .unwrap();
        fs::write(gpu.join("modalias"), "pci:v000010DEd00001CB3bc03sc00i00\n").unwrap();

        let driver_root = temp_root("lifegraph-gpu-driver");
        let driver_dir = driver_root.join("nvidia");
        fs::create_dir_all(&driver_dir).unwrap();
        let module_root = temp_root("lifegraph-gpu-module");
        let module_dir = module_root.join("nvidia");
        fs::create_dir_all(&module_dir).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&driver_dir, gpu.join("driver")).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&module_dir, driver_dir.join("module")).unwrap();

        let drm_card = drm_root.join("card0");
        fs::create_dir_all(&drm_card).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&gpu, drm_card.join("device")).unwrap();

        let connector = drm_root.join("card0-HDMI-A-1");
        fs::create_dir_all(&connector).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&gpu, connector.join("device")).unwrap();
        fs::write(connector.join("status"), "connected\n").unwrap();
        fs::write(connector.join("enabled"), "enabled\n").unwrap();
        fs::write(connector.join("connector_id"), "128\n").unwrap();
        fs::write(connector.join("mode"), "3840x2160@60\n").unwrap();
        fs::write(connector.join("modes"), "3840x2160@60\n1920x1080@60\n").unwrap();

        let render = drm_root.join("renderD128");
        fs::create_dir_all(&render).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&gpu, render.join("device")).unwrap();

        let devices = discover_gpus_in(&pci_root, &drm_root).unwrap();
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].vendor_id, Some(0x10de));
        assert_eq!(devices[0].drm_cards, vec!["card0"]);
        assert_eq!(devices[0].drm_render_nodes, vec!["renderD128"]);
        assert_eq!(devices[0].connectors.len(), 1);
        assert_eq!(devices[0].connectors[0].name, "HDMI-A-1");
        assert_eq!(devices[0].connectors[0].connector_id, Some(128));
        assert!(devices[0].connectors[0].connected);
        assert!(devices[0].connectors[0].enabled);
        assert_eq!(
            devices[0].connectors[0].current_mode,
            Some(GpuDisplayMode {
                width: 3840,
                height: 2160,
                refresh_millihz: 60_000
            })
        );
        assert_eq!(devices[0].connectors[0].modes.len(), 2);
        assert!(devices[0].boot_vga);
        assert_eq!(devices[0].driver.as_deref(), Some("nvidia"));
        assert_eq!(devices[0].driver_module.as_deref(), Some("nvidia"));
        assert_eq!(
            devices[0].modalias.as_deref(),
            Some("pci:v000010DEd00001CB3bc03sc00i00")
        );

        let info = devices.into_iter().next().unwrap().into_info();
        assert_eq!(info.vendor, GpuVendor::Nvidia);
        assert!(info.supports_display);
        assert!(info.supports_render);
        assert!(info.supports_compute);
        assert!(info.boot_vga);
        assert_eq!(info.connectors.len(), 1);
        assert_eq!(info.connectors[0].name, "HDMI-A-1");
        assert_eq!(info.connectors[0].connector_id, Some(128));
        assert!(info.connectors[0].connected);
        assert_eq!(
            info.connectors[0].current_mode,
            Some(GpuDisplayMode {
                width: 3840,
                height: 2160,
                refresh_millihz: 60_000
            })
        );
        assert_eq!(info.connectors[0].modes.len(), 2);

        fs::remove_dir_all(pci_root).unwrap();
        fs::remove_dir_all(drm_root).unwrap();
        fs::remove_dir_all(driver_root).unwrap();
        fs::remove_dir_all(module_root).unwrap();
    }

    #[test]
    fn attachs_cec_adapter_by_card_and_connector_id() {
        let mut gpus = vec![LinuxGpuDevice {
            pci_address: "0000:01:00.0".into(),
            sysfs_path: PathBuf::from("/tmp/gpu"),
            vendor_id: Some(0x10de),
            device_id: Some(0x1cb3),
            subsystem_vendor_id: None,
            subsystem_device_id: None,
            class_code: Some(0x030000),
            revision: None,
            driver: None,
            driver_module: None,
            modalias: None,
            numa_node: None,
            current_link_speed: None,
            current_link_width: None,
            max_link_speed: None,
            max_link_width: None,
            boot_vga: false,
            drm_cards: vec!["card1".into()],
            drm_render_nodes: Vec::new(),
            drm_card_device_nodes: Vec::new(),
            drm_render_device_nodes: Vec::new(),
            connectors: vec![LinuxGpuConnector {
                name: "HDMI-A-1".into(),
                connector_id: Some(128),
                connected: true,
                enabled: true,
                current_mode: None,
                modes: Vec::new(),
                cec_adapter: None,
            }],
        }];
        let mut lookup = HashMap::new();
        lookup.insert((1, 128), "cec0".to_string());
        attach_cec_adapters_to_gpus_with_lookup(&mut gpus, &lookup);
        assert_eq!(gpus[0].connectors[0].cec_adapter.as_deref(), Some("cec0"));
    }
}
