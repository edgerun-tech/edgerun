use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use edgerun_linux_sysfs::parse_hex_u32_from_str;
use edgerun_linux_sysfs::prelude::v1::*;
// Re-export sysfs helpers that downstream NPU backends need.
use edgerun_devices::npu::{
    CapabilityDescriptor, CapabilityError, CapabilityProvider, NpuDevice, NpuInfo,
    NpuWorkloadRequest, NpuWorkloadResult, default_npu_descriptor, validate_npu_workload_request,
};
pub use edgerun_linux_sysfs::{read_trimmed, temp_root};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinuxNpuInfo {
    pub instance_id: String,
    pub display_name: String,
    pub sysfs_path: PathBuf,
    pub character_device: Option<PathBuf>,
    pub driver_name: Option<String>,
    pub pci_address: Option<String>,
    pub vendor_id: Option<u32>,
    pub device_id: Option<u32>,
    pub firmware_version: Option<String>,
    pub accelerator_class: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinuxNpuBackend {
    pub info: LinuxNpuInfo,
}

fn read_driver_name(device_path: &Path) -> Option<String> {
    fs::read_link(device_path.join("driver"))
        .ok()
        .and_then(|p| p.file_name().map(|v| v.to_string_lossy().to_string()))
}

fn read_firmware_version(device_path: &Path) -> Option<String> {
    for rel in [
        "fw_version",
        "firmware_version",
        "device/fw_version",
        "device/firmware_version",
    ] {
        if let Some(v) = read_trimmed(&device_path.join(rel)) {
            if !v.is_empty() {
                return Some(v);
            }
        }
    }
    None
}

fn pci_info_from_device(device_path: &Path) -> (Option<String>, Option<u32>, Option<u32>, bool) {
    let pci_address = device_path
        .file_name()
        .map(|v| v.to_string_lossy().to_string());
    let vendor_id =
        read_trimmed(&device_path.join("vendor")).and_then(|v| parse_hex_u32_from_str(&v));
    let device_id =
        read_trimmed(&device_path.join("device")).and_then(|v| parse_hex_u32_from_str(&v));
    let accel_class = read_trimmed(&device_path.join("class"))
        .and_then(|v| parse_hex_u32_from_str(&v))
        .is_some_and(|class_code| class_code >> 16 == 0x12);
    (pci_address, vendor_id, device_id, accel_class)
}

fn collect_accel_class_devices(
    root: &Path,
    dev_root: &Path,
) -> Result<Vec<LinuxNpuInfo>, CapabilityError> {
    let mut out = Vec::new();
    if !root.exists() {
        return Ok(out);
    }
    for entry in fs::read_dir(root).map_err(|e| CapabilityError::Provider(e.to_string()))? {
        let entry = entry.map_err(|e| CapabilityError::Provider(e.to_string()))?;
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with("accel") {
            continue;
        }
        let class_path = entry.path();
        let real = fs::canonicalize(&class_path).unwrap_or(class_path.clone());
        let device_path = fs::canonicalize(real.join("device")).unwrap_or(real.join("device"));
        let (pci_address, vendor_id, device_id, accel_class) = pci_info_from_device(&device_path);
        let char_path = dev_root.join(&name);
        out.push(LinuxNpuInfo {
            instance_id: name.clone(),
            display_name: name,
            sysfs_path: real.clone(),
            character_device: char_path.exists().then_some(char_path),
            driver_name: read_driver_name(&device_path),
            pci_address,
            vendor_id,
            device_id,
            firmware_version: read_firmware_version(&real)
                .or_else(|| read_firmware_version(&device_path)),
            accelerator_class: accel_class,
        });
    }
    out.sort_by(|a, b| a.instance_id.cmp(&b.instance_id));
    Ok(out)
}

fn known_npu_driver(name: &str) -> bool {
    matches!(name, "amdxdna" | "ivpu" | "intel_vpu")
}

fn collect_pci_driver_npus(root: &Path) -> Result<Vec<LinuxNpuInfo>, CapabilityError> {
    let mut out = Vec::new();
    if !root.exists() {
        return Ok(out);
    }
    for entry in fs::read_dir(root).map_err(|e| CapabilityError::Provider(e.to_string()))? {
        let entry = entry.map_err(|e| CapabilityError::Provider(e.to_string()))?;
        let path = entry.path();
        let Some(driver_name) = read_driver_name(&path) else {
            continue;
        };
        if !known_npu_driver(&driver_name) {
            continue;
        }
        let (pci_address, vendor_id, device_id, accel_class) = pci_info_from_device(&path);
        let instance_id = pci_address
            .clone()
            .unwrap_or_else(|| entry.file_name().to_string_lossy().to_string());
        out.push(LinuxNpuInfo {
            display_name: format!("{} ({})", driver_name, instance_id),
            instance_id,
            sysfs_path: path.clone(),
            character_device: None,
            driver_name: Some(driver_name),
            pci_address,
            vendor_id,
            device_id,
            firmware_version: read_firmware_version(&path),
            accelerator_class: accel_class,
        });
    }
    out.sort_by(|a, b| a.instance_id.cmp(&b.instance_id));
    Ok(out)
}

pub fn discover_linux_npus() -> Result<Vec<LinuxNpuInfo>, CapabilityError> {
    discover_linux_npus_in(
        Path::new("/sys/class/accel"),
        Path::new("/sys/bus/pci/devices"),
        Path::new("/dev/accel"),
    )
}

pub fn discover_linux_npus_in(
    accel_class_root: &Path,
    pci_root: &Path,
    accel_dev_root: &Path,
) -> Result<Vec<LinuxNpuInfo>, CapabilityError> {
    let mut by_id = BTreeSet::new();
    let mut out = Vec::new();
    for info in collect_accel_class_devices(accel_class_root, accel_dev_root)?
        .into_iter()
        .chain(collect_pci_driver_npus(pci_root)?)
    {
        if by_id.insert(info.instance_id.clone()) {
            out.push(info);
        }
    }
    out.sort_by(|a, b| a.instance_id.cmp(&b.instance_id));
    Ok(out)
}

impl CapabilityProvider for LinuxNpuBackend {
    fn descriptor(&self) -> CapabilityDescriptor {
        default_npu_descriptor("linux-npu", &self.info.instance_id)
    }
}

impl NpuDevice for LinuxNpuBackend {
    fn npu_info(&self) -> Result<NpuInfo, CapabilityError> {
        Ok(NpuInfo {
            provider: "linux-npu".into(),
            instance_id: self.info.instance_id.clone(),
            display_name: self.info.display_name.clone(),
            driver_name: self.info.driver_name.clone(),
            pci_address: self.info.pci_address.clone(),
            vendor_id: self.info.vendor_id,
            device_id: self.info.device_id,
            character_device: self
                .info
                .character_device
                .as_ref()
                .map(|p| p.display().to_string()),
            firmware_version: self.info.firmware_version.clone(),
            supports_submission: self.info.character_device.is_some(),
        })
    }

    fn execute_workload(
        &self,
        request: &NpuWorkloadRequest,
    ) -> Result<NpuWorkloadResult, CapabilityError> {
        validate_npu_workload_request(request)?;
        Err(CapabilityError::Unsupported(
            "linux npu backend does not yet implement command submission",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    use edgerun_linux_sysfs::temp_root;

    #[test]
    fn discovers_accel_class_device() {
        let accel_root = temp_root("npu-accel-class");
        let pci_root = temp_root("npu-pci-root");
        let dev_root = temp_root("npu-dev-root");

        let accel0 = accel_root.join("accel0");
        fs::create_dir_all(accel0.join("device/driver")).unwrap();
        fs::write(accel0.join("device/vendor"), "0x1022\n").unwrap();
        fs::write(accel0.join("device/device"), "0x1502\n").unwrap();
        fs::write(accel0.join("device/class"), "0x120000\n").unwrap();
        let driver_target = accel_root.join("amdxdna-driver");
        fs::create_dir_all(&driver_target).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&driver_target, accel0.join("device/driver-link")).ok();
        let _ = fs::remove_dir_all(accel0.join("device/driver"));
        #[cfg(unix)]
        std::os::unix::fs::symlink(&driver_target, accel0.join("device/driver")).unwrap();
        fs::create_dir_all(&dev_root).unwrap();
        fs::write(dev_root.join("accel0"), "").unwrap();

        let infos = discover_linux_npus_in(&accel_root, &pci_root, &dev_root).unwrap();
        assert_eq!(infos.len(), 1);
        assert_eq!(infos[0].instance_id, "accel0");
        assert_eq!(infos[0].vendor_id, Some(0x1022));
        assert!(infos[0].character_device.is_some());

        let _ = fs::remove_dir_all(accel_root);
        let _ = fs::remove_dir_all(pci_root);
        let _ = fs::remove_dir_all(dev_root);
    }

    #[test]
    fn discovers_known_pci_driver_fallback() {
        let accel_root = temp_root("npu-accel-empty");
        let pci_root = temp_root("npu-pci");
        let dev_root = temp_root("npu-dev-empty");
        let dev = pci_root.join("0000:00:08.0");
        fs::create_dir_all(&dev).unwrap();
        fs::write(dev.join("vendor"), "0x8086\n").unwrap();
        fs::write(dev.join("device"), "0x7d1d\n").unwrap();
        fs::write(dev.join("class"), "0x120000\n").unwrap();
        let driver_target = pci_root.join("ivpu");
        fs::create_dir_all(&driver_target).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&driver_target, dev.join("driver")).unwrap();

        let infos = discover_linux_npus_in(&accel_root, &pci_root, &dev_root).unwrap();
        assert_eq!(infos.len(), 1);
        assert_eq!(infos[0].driver_name.as_deref(), Some("ivpu"));
        assert_eq!(infos[0].pci_address.as_deref(), Some("0000:00:08.0"));

        let _ = fs::remove_dir_all(accel_root);
        let _ = fs::remove_dir_all(pci_root);
        let _ = fs::remove_dir_all(dev_root);
    }

    #[test]
    fn discovers_empty_when_no_accel_or_pci_dirs() {
        let accel_root = temp_root("npu-empty-accel");
        let pci_root = temp_root("npu-empty-pci");
        let dev_root = temp_root("npu-empty-dev");

        let infos = discover_linux_npus_in(&accel_root, &pci_root, &dev_root).unwrap();
        assert!(infos.is_empty());

        let _ = fs::remove_dir_all(accel_root);
        let _ = fs::remove_dir_all(pci_root);
        let _ = fs::remove_dir_all(dev_root);
    }

    #[test]
    fn known_npu_driver_accepts_known_names() {
        assert!(known_npu_driver("amdxdna"));
        assert!(known_npu_driver("ivpu"));
        assert!(known_npu_driver("intel_vpu"));
    }

    #[test]
    fn known_npu_driver_rejects_unknown_names() {
        assert!(!known_npu_driver("nvidia"));
        assert!(!known_npu_driver("amdgpu"));
        assert!(!known_npu_driver(""));
    }

    #[test]
    fn linux_npu_info_clone_and_eq() {
        let info = LinuxNpuInfo {
            instance_id: "accel0".into(),
            display_name: "accel0".into(),
            sysfs_path: PathBuf::from("/sys/class/accel/accel0"),
            character_device: Some(PathBuf::from("/dev/accel0")),
            driver_name: Some("amdxdna".into()),
            pci_address: Some("0000:01:00.0".into()),
            vendor_id: Some(0x1022),
            device_id: Some(0x1234),
            firmware_version: Some("v1".into()),
            accelerator_class: true,
        };
        let cloned = info.clone();
        assert_eq!(info, cloned);
    }

    #[test]
    fn linux_npu_backend_descriptor() {
        let info = LinuxNpuInfo {
            instance_id: "npu-test".into(),
            display_name: "NPU Test".into(),
            sysfs_path: PathBuf::from("/sys/class/accel/npu-test"),
            character_device: None,
            driver_name: Some("ivpu".into()),
            pci_address: None,
            vendor_id: None,
            device_id: None,
            firmware_version: None,
            accelerator_class: false,
        };
        let backend = LinuxNpuBackend { info };
        let descriptor = backend.descriptor();
        assert_eq!(descriptor.provider_name, "linux-npu");
        assert_eq!(descriptor.provider_instance_id, "npu-test");
    }

    #[test]
    fn linux_npu_backend_npu_info() {
        let info = LinuxNpuInfo {
            instance_id: "accel1".into(),
            display_name: "My NPU".into(),
            sysfs_path: PathBuf::from("/sys/class/accel/accel1"),
            character_device: Some(PathBuf::from("/dev/accel1")),
            driver_name: Some("amdxdna".into()),
            pci_address: Some("0000:00:08.0".into()),
            vendor_id: Some(0x1022),
            device_id: Some(0x1502),
            firmware_version: Some("fw1.0".into()),
            accelerator_class: true,
        };
        let backend = LinuxNpuBackend { info };
        let npu_info = backend.npu_info().unwrap();
        assert_eq!(npu_info.provider, "linux-npu");
        assert_eq!(npu_info.instance_id, "accel1");
        assert_eq!(npu_info.display_name, "My NPU");
        assert_eq!(npu_info.driver_name.as_deref(), Some("amdxdna"));
        assert_eq!(npu_info.character_device.as_deref(), Some("/dev/accel1"));
        assert!(npu_info.supports_submission);
    }

    #[test]
    fn linux_npu_backend_npu_info_without_char_device() {
        let info = LinuxNpuInfo {
            instance_id: "pci-npu".into(),
            display_name: "PCI NPU".into(),
            sysfs_path: PathBuf::from("/sys/bus/pci/devices/0000:00:08.0"),
            character_device: None,
            driver_name: Some("ivpu".into()),
            pci_address: Some("0000:00:08.0".into()),
            vendor_id: None,
            device_id: None,
            firmware_version: None,
            accelerator_class: false,
        };
        let backend = LinuxNpuBackend { info };
        let npu_info = backend.npu_info().unwrap();
        assert_eq!(npu_info.character_device, None);
        assert!(!npu_info.supports_submission);
    }

    #[test]
    fn linux_npu_backend_execute_workload_rejects_empty_bytes() {
        let info = LinuxNpuInfo {
            instance_id: "test".into(),
            display_name: "Test".into(),
            sysfs_path: PathBuf::from("/tmp"),
            character_device: Some(PathBuf::from("/dev/accel0")),
            driver_name: None,
            pci_address: None,
            vendor_id: None,
            device_id: None,
            firmware_version: None,
            accelerator_class: false,
        };
        let backend = LinuxNpuBackend { info };
        let err = backend
            .execute_workload(&edgerun_devices::npu::NpuWorkloadRequest {
                mode: edgerun_devices::npu::NpuExecutionMode::Inference,
                input_bytes: Vec::new(),
                target_latency_ms: None,
            })
            .unwrap_err();
        assert!(matches!(err, CapabilityError::InvalidRequest(_)));
    }

    #[test]
    fn linux_npu_backend_execute_workload_returns_unsupported() {
        let info = LinuxNpuInfo {
            instance_id: "test".into(),
            display_name: "Test".into(),
            sysfs_path: PathBuf::from("/tmp"),
            character_device: Some(PathBuf::from("/dev/accel0")),
            driver_name: None,
            pci_address: None,
            vendor_id: None,
            device_id: None,
            firmware_version: None,
            accelerator_class: false,
        };
        let backend = LinuxNpuBackend { info };
        let err = backend
            .execute_workload(&edgerun_devices::npu::NpuWorkloadRequest {
                mode: edgerun_devices::npu::NpuExecutionMode::Inference,
                input_bytes: vec![1, 2, 3],
                target_latency_ms: None,
            })
            .unwrap_err();
        assert!(matches!(err, CapabilityError::Unsupported(_)));
    }

    #[test]
    fn linux_npu_backend_clone() {
        let info = LinuxNpuInfo {
            instance_id: "t".into(),
            display_name: "t".into(),
            sysfs_path: PathBuf::new(),
            character_device: None,
            driver_name: None,
            pci_address: None,
            vendor_id: None,
            device_id: None,
            firmware_version: None,
            accelerator_class: false,
        };
        let backend = LinuxNpuBackend { info };
        assert_eq!(backend.clone(), backend);
    }

    #[test]
    fn discover_deduplicates_accel_and_pci_by_instance_id() {
        let accel_root = temp_root("npu-dedup-accel");
        let pci_root = temp_root("npu-dedup-pci");
        let dev_root = temp_root("npu-dedup-dev");

        // Create accel0 in accel class
        let accel0 = accel_root.join("accel0");
        fs::create_dir_all(accel0.join("device/driver")).unwrap();
        fs::write(accel0.join("device/class"), "0x120000\n").unwrap();
        fs::create_dir_all(&dev_root).unwrap();
        fs::write(dev_root.join("accel0"), "").unwrap();

        // Create same accel0 in pci with ivpu driver
        let pci_dev = pci_root.join("0000:00:08.0");
        fs::create_dir_all(&pci_dev).unwrap();
        fs::write(pci_dev.join("class"), "0x120000\n").unwrap();
        let driver_target = pci_root.join("ivpu");
        fs::create_dir_all(&driver_target).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&driver_target, pci_dev.join("driver")).unwrap();

        let infos = discover_linux_npus_in(&accel_root, &pci_root, &dev_root).unwrap();
        // Should be deduplicated - only one entry for accel0
        let accel_count = infos.iter().filter(|i| i.instance_id == "accel0").count();
        assert_eq!(accel_count, 1);

        let _ = fs::remove_dir_all(accel_root);
        let _ = fs::remove_dir_all(pci_root);
        let _ = fs::remove_dir_all(dev_root);
    }

    #[test]
    fn pci_discovery_ignores_unknown_drivers() {
        let accel_root = temp_root("npu-no-accel");
        let pci_root = temp_root("npu-pci-unknown");
        let dev_root = temp_root("npu-no-dev");

        let dev = pci_root.join("0000:00:01.0");
        fs::create_dir_all(&dev).unwrap();
        fs::write(dev.join("class"), "0x120000\n").unwrap();
        let driver_target = pci_root.join("some-unknown-driver");
        fs::create_dir_all(&driver_target).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&driver_target, dev.join("driver")).unwrap();

        let infos = discover_linux_npus_in(&accel_root, &pci_root, &dev_root).unwrap();
        assert!(infos.is_empty());

        let _ = fs::remove_dir_all(accel_root);
        let _ = fs::remove_dir_all(pci_root);
        let _ = fs::remove_dir_all(dev_root);
    }
}
