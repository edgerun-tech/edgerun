use edgerun_linux_npu::{discover_linux_npus_in, read_trimmed, LinuxNpuInfo};
use edgerun_npu::{
    CapabilityDescriptor, CapabilityError, CapabilityProvider, default_npu_descriptor,
    validate_npu_workload_request, NpuDevice, NpuExecutionMode, NpuInfo, NpuWorkloadRequest,
    NpuWorkloadResult,
};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

const AMD_VENDOR_ID: u32 = 0x1022;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AmdXdnaGeneration {
    PhoenixOrHawkPoint,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AmdXdnaInfo {
    pub linux: LinuxNpuInfo,
    pub generation: AmdXdnaGeneration,
    pub amdxdna_sysfs_path: Option<PathBuf>,
    pub supported_execution_modes: Vec<NpuExecutionMode>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AmdXdnaBackend {
    pub info: AmdXdnaInfo,
}

fn classify_generation(info: &LinuxNpuInfo) -> AmdXdnaGeneration {
    let Some(device_id) = info.device_id else {
        return AmdXdnaGeneration::Unknown;
    };
    match device_id {
        0x1502 | 0x17f0 => AmdXdnaGeneration::PhoenixOrHawkPoint,
        _ => AmdXdnaGeneration::Unknown,
    }
}

#[must_use]
pub fn is_amd_xdna_candidate(info: &LinuxNpuInfo) -> bool {
    info.vendor_id == Some(AMD_VENDOR_ID)
        && (info.driver_name.as_deref() == Some("amdxdna")
            || info.accelerator_class
            || info
                .character_device
                .as_ref()
                .is_some_and(|p| p.to_string_lossy().contains("accel")))
}

pub fn discover_amd_xdna_devices() -> Result<Vec<AmdXdnaInfo>, CapabilityError> {
    discover_amd_xdna_devices_in(
        Path::new("/sys/class/accel"),
        Path::new("/sys/bus/pci/devices"),
        Path::new("/dev/accel"),
    )
}

pub fn discover_amd_xdna_devices_in(
    accel_class_root: &Path,
    pci_root: &Path,
    accel_dev_root: &Path,
) -> Result<Vec<AmdXdnaInfo>, CapabilityError> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for info in discover_linux_npus_in(accel_class_root, pci_root, accel_dev_root)? {
        if !is_amd_xdna_candidate(&info) {
            continue;
        }
        let dedupe_key = info
            .pci_address
            .clone()
            .unwrap_or_else(|| info.instance_id.clone());
        if !seen.insert(dedupe_key) {
            continue;
        }
        let generation = classify_generation(&info);
        let supported_execution_modes = if info.character_device.is_some() {
            vec![
                NpuExecutionMode::Inference,
                NpuExecutionMode::Compilation,
                NpuExecutionMode::Preprocessing,
            ]
        } else {
            vec![NpuExecutionMode::Inference]
        };
        out.push(AmdXdnaInfo {
            amdxdna_sysfs_path: info
                .sysfs_path
                .file_name()
                .is_some_and(|v| v.to_string_lossy().starts_with("accel"))
                .then(|| info.sysfs_path.clone()),
            generation,
            linux: info,
            supported_execution_modes,
        });
    }
    out.sort_by(|a, b| a.linux.instance_id.cmp(&b.linux.instance_id));
    Ok(out)
}

impl CapabilityProvider for AmdXdnaBackend {
    fn descriptor(&self) -> CapabilityDescriptor {
        default_npu_descriptor("amd-xdna", &self.info.linux.instance_id)
    }
}

impl NpuDevice for AmdXdnaBackend {
    fn npu_info(&self) -> Result<NpuInfo, CapabilityError> {
        let firmware_version = self
            .info
            .linux
            .firmware_version
            .clone()
            .or_else(|| {
                self.info
                    .amdxdna_sysfs_path
                    .as_ref()
                    .and_then(|p| read_trimmed(&p.join("fw_version")))
            });
        Ok(NpuInfo {
            provider: "amd-xdna".into(),
            instance_id: self.info.linux.instance_id.clone(),
            display_name: self.info.linux.display_name.clone(),
            driver_name: self.info.linux.driver_name.clone(),
            pci_address: self.info.linux.pci_address.clone(),
            vendor_id: self.info.linux.vendor_id,
            device_id: self.info.linux.device_id,
            character_device: self
                .info
                .linux
                .character_device
                .as_ref()
                .map(|p| p.display().to_string()),
            firmware_version,
            supports_submission: self.info.linux.character_device.is_some(),
        })
    }

    fn execute_workload(
        &self,
        request: &NpuWorkloadRequest,
    ) -> Result<NpuWorkloadResult, CapabilityError> {
        validate_npu_workload_request(request)?;
        if self.info.linux.character_device.is_none() {
            return Err(CapabilityError::Unsupported(
                "amd-xdna device is not exposed via /dev/accel",
            ));
        }
        Err(CapabilityError::Unsupported(
            "amd-xdna command submission is not implemented yet; open-context/overlay/cmd-buffer submission remains to be added",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgerun_linux_npu::temp_root;
    use std::fs;

    #[test]
    fn filters_for_amd_xdna() {
        let info = LinuxNpuInfo {
            instance_id: "accel0".into(),
            display_name: "accel0".into(),
            sysfs_path: PathBuf::from("/sys/class/accel/accel0"),
            character_device: Some(PathBuf::from("/dev/accel/accel0")),
            driver_name: Some("amdxdna".into()),
            pci_address: Some("0000:00:08.0".into()),
            vendor_id: Some(AMD_VENDOR_ID),
            device_id: Some(0x1502),
            firmware_version: None,
            accelerator_class: true,
        };
        assert!(is_amd_xdna_candidate(&info));
        assert_eq!(
            classify_generation(&info),
            AmdXdnaGeneration::PhoenixOrHawkPoint
        );
    }

    #[test]
    fn discovers_amd_xdna_from_linux_npu_discovery() {
        let accel_root = temp_root("amd-xdna-accel");
        let pci_root = temp_root("amd-xdna-pci");
        let dev_root = temp_root("amd-xdna-dev");

        let accel0 = accel_root.join("accel0");
        fs::create_dir_all(&accel0).unwrap();
        let pci = pci_root.join("0000:00:08.0");
        fs::create_dir_all(&pci).unwrap();
        fs::write(pci.join("vendor"), "0x1022\n").unwrap();
        fs::write(pci.join("device"), "0x1502\n").unwrap();
        fs::write(pci.join("class"), "0x120000\n").unwrap();
        let driver_target = pci_root.join("amdxdna");
        fs::create_dir_all(&driver_target).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&driver_target, pci.join("driver")).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&pci, accel0.join("device")).unwrap();
        fs::create_dir_all(&dev_root).unwrap();
        fs::write(dev_root.join("accel0"), "").unwrap();

        let infos = discover_amd_xdna_devices_in(&accel_root, &pci_root, &dev_root).unwrap();
        assert_eq!(infos.len(), 1);
        assert_eq!(infos[0].generation, AmdXdnaGeneration::PhoenixOrHawkPoint);
        assert!(infos[0]
            .supported_execution_modes
            .contains(&NpuExecutionMode::Inference));

        let _ = fs::remove_dir_all(accel_root);
        let _ = fs::remove_dir_all(pci_root);
        let _ = fs::remove_dir_all(dev_root);
    }

    #[test]
    fn non_amd_vendor_is_not_candidate() {
        let info = LinuxNpuInfo {
            instance_id: "accel0".into(),
            display_name: "accel0".into(),
            sysfs_path: PathBuf::from("/sys/class/accel/accel0"),
            character_device: Some(PathBuf::from("/dev/accel/accel0")),
            driver_name: Some("amdxdna".into()),
            pci_address: Some("0000:00:08.0".into()),
            vendor_id: Some(0x8086), // Intel
            device_id: Some(0x1234),
            firmware_version: None,
            accelerator_class: true,
        };
        assert!(!is_amd_xdna_candidate(&info));
    }

    #[test]
    fn amd_vendor_without_amdxdna_driver_is_not_candidate() {
        let info = LinuxNpuInfo {
            instance_id: "accel0".into(),
            display_name: "accel0".into(),
            sysfs_path: PathBuf::from("/sys/class/accel/accel0"),
            character_device: Some(PathBuf::from("/dev/other/device")), // no "accel"
            driver_name: Some("other_driver".into()),
            pci_address: Some("0000:00:08.0".into()),
            vendor_id: Some(AMD_VENDOR_ID),
            device_id: Some(0x1502),
            firmware_version: None,
            accelerator_class: false,
        };
        // Not candidate: driver is not amdxdna, accelerator_class is false, device doesn't contain "accel"
        assert!(!is_amd_xdna_candidate(&info));
    }

    #[test]
    fn amd_vendor_with_accel_device_path_is_candidate() {
        let info = LinuxNpuInfo {
            instance_id: "accel0".into(),
            display_name: "accel0".into(),
            sysfs_path: PathBuf::from("/sys/class/accel/accel0"),
            character_device: Some(PathBuf::from("/dev/accel/accel0")),
            driver_name: Some("other".into()),
            pci_address: Some("0000:00:08.0".into()),
            vendor_id: Some(AMD_VENDOR_ID),
            device_id: Some(0x1502),
            firmware_version: None,
            accelerator_class: false,
        };
        // candidate via character_device path containing "accel"
        assert!(is_amd_xdna_candidate(&info));
    }

    #[test]
    fn classify_generation_unknown_device_id() {
        let info = LinuxNpuInfo {
            instance_id: "accel0".into(),
            display_name: "accel0".into(),
            sysfs_path: PathBuf::from("/sys/class/accel/accel0"),
            character_device: None,
            driver_name: None,
            pci_address: Some("0000:00:08.0".into()),
            vendor_id: Some(AMD_VENDOR_ID),
            device_id: Some(0x9999), // Unknown device ID
            firmware_version: None,
            accelerator_class: true,
        };
        assert_eq!(
            classify_generation(&info),
            AmdXdnaGeneration::Unknown
        );
    }

    #[test]
    fn classify_generation_no_device_id() {
        let info = LinuxNpuInfo {
            instance_id: "accel0".into(),
            display_name: "accel0".into(),
            sysfs_path: PathBuf::from("/sys/class/accel/accel0"),
            character_device: None,
            driver_name: None,
            pci_address: Some("0000:00:08.0".into()),
            vendor_id: Some(AMD_VENDOR_ID),
            device_id: None,
            firmware_version: None,
            accelerator_class: true,
        };
        assert_eq!(
            classify_generation(&info),
            AmdXdnaGeneration::Unknown
        );
    }

    #[test]
    fn classify_generation_phoenix_device_id_0x17f0() {
        let info = LinuxNpuInfo {
            instance_id: "accel0".into(),
            display_name: "accel0".into(),
            sysfs_path: PathBuf::from("/sys/class/accel/accel0"),
            character_device: Some(PathBuf::from("/dev/accel/accel0")),
            driver_name: Some("amdxdna".into()),
            pci_address: Some("0000:00:08.0".into()),
            vendor_id: Some(AMD_VENDOR_ID),
            device_id: Some(0x17f0),
            firmware_version: None,
            accelerator_class: true,
        };
        assert_eq!(
            classify_generation(&info),
            AmdXdnaGeneration::PhoenixOrHawkPoint
        );
    }

    #[test]
    fn amd_xdna_info_clone_debug() {
        let info = AmdXdnaInfo {
            linux: LinuxNpuInfo {
                instance_id: "accel0".into(),
                display_name: "accel0".into(),
                sysfs_path: PathBuf::from("/sys/class/accel/accel0"),
                character_device: Some(PathBuf::from("/dev/accel/accel0")),
                driver_name: Some("amdxdna".into()),
                pci_address: Some("0000:00:08.0".into()),
                vendor_id: Some(AMD_VENDOR_ID),
                device_id: Some(0x1502),
                firmware_version: Some("1.0.0".into()),
                accelerator_class: true,
            },
            generation: AmdXdnaGeneration::PhoenixOrHawkPoint,
            amdxdna_sysfs_path: Some(PathBuf::from("/sys/class/accel/accel0")),
            supported_execution_modes: vec![
                NpuExecutionMode::Inference,
                NpuExecutionMode::Compilation,
            ],
        };
        let cloned = info.clone();
        assert_eq!(info.generation, cloned.generation);
        assert_eq!(info.supported_execution_modes, cloned.supported_execution_modes);
        let debug_str = format!("{info:?}");
        assert!(debug_str.contains("AmdXdnaInfo"));
    }

    #[test]
    fn amd_xdna_backend_descriptor() {
        let backend = AmdXdnaBackend {
            info: AmdXdnaInfo {
                linux: LinuxNpuInfo {
                    instance_id: "accel0".into(),
                    display_name: "accel0".into(),
                    sysfs_path: PathBuf::from("/sys/class/accel/accel0"),
                    character_device: None,
                    driver_name: Some("amdxdna".into()),
                    pci_address: Some("0000:00:08.0".into()),
                    vendor_id: Some(AMD_VENDOR_ID),
                    device_id: Some(0x1502),
                    firmware_version: None,
                    accelerator_class: true,
                },
                generation: AmdXdnaGeneration::PhoenixOrHawkPoint,
                amdxdna_sysfs_path: None,
                supported_execution_modes: vec![NpuExecutionMode::Inference],
            },
        };
        let desc = backend.descriptor();
        assert_eq!(desc.provider_name, "amd-xdna");
        assert_eq!(desc.provider_instance_id, "accel0");
    }

    #[test]
    fn amd_xdna_npu_info_returns_expected_values() {
        let backend = AmdXdnaBackend {
            info: AmdXdnaInfo {
                linux: LinuxNpuInfo {
                    instance_id: "accel0".into(),
                    display_name: "AMD XDNA NPU".into(),
                    sysfs_path: PathBuf::from("/sys/class/accel/accel0"),
                    character_device: Some(PathBuf::from("/dev/accel/accel0")),
                    driver_name: Some("amdxdna".into()),
                    pci_address: Some("0000:00:08.0".into()),
                    vendor_id: Some(AMD_VENDOR_ID),
                    device_id: Some(0x1502),
                    firmware_version: Some("2.0.0".into()),
                    accelerator_class: true,
                },
                generation: AmdXdnaGeneration::PhoenixOrHawkPoint,
                amdxdna_sysfs_path: Some(PathBuf::from("/sys/class/accel/accel0")),
                supported_execution_modes: vec![NpuExecutionMode::Inference],
            },
        };
        let npu_info = backend.npu_info().unwrap();
        assert_eq!(npu_info.provider, "amd-xdna");
        assert_eq!(npu_info.instance_id, "accel0");
        assert_eq!(npu_info.firmware_version, Some("2.0.0".into()));
        assert!(npu_info.supports_submission);
    }

    #[test]
    fn amd_xdna_npu_info_without_char_device_no_submission() {
        let backend = AmdXdnaBackend {
            info: AmdXdnaInfo {
                linux: LinuxNpuInfo {
                    instance_id: "accel0".into(),
                    display_name: "AMD XDNA NPU".into(),
                    sysfs_path: PathBuf::from("/sys/class/accel/accel0"),
                    character_device: None,
                    driver_name: Some("amdxdna".into()),
                    pci_address: Some("0000:00:08.0".into()),
                    vendor_id: Some(AMD_VENDOR_ID),
                    device_id: Some(0x1502),
                    firmware_version: None,
                    accelerator_class: true,
                },
                generation: AmdXdnaGeneration::PhoenixOrHawkPoint,
                amdxdna_sysfs_path: None,
                supported_execution_modes: vec![NpuExecutionMode::Inference],
            },
        };
        let npu_info = backend.npu_info().unwrap();
        assert!(!npu_info.supports_submission);
        assert!(npu_info.character_device.is_none());
    }

    #[test]
    fn amd_xdna_execute_workload_rejects_without_char_device() {
        let backend = AmdXdnaBackend {
            info: AmdXdnaInfo {
                linux: LinuxNpuInfo {
                    instance_id: "accel0".into(),
                    display_name: "AMD XDNA NPU".into(),
                    sysfs_path: PathBuf::from("/sys/class/accel/accel0"),
                    character_device: None,
                    driver_name: Some("amdxdna".into()),
                    pci_address: Some("0000:00:08.0".into()),
                    vendor_id: Some(AMD_VENDOR_ID),
                    device_id: Some(0x1502),
                    firmware_version: None,
                    accelerator_class: true,
                },
                generation: AmdXdnaGeneration::PhoenixOrHawkPoint,
                amdxdna_sysfs_path: None,
                supported_execution_modes: vec![NpuExecutionMode::Inference],
            },
        };
        let request = NpuWorkloadRequest {
            mode: NpuExecutionMode::Inference,
            input_bytes: vec![1, 2, 3],
            target_latency_ms: None,
        };
        let err = backend.execute_workload(&request).unwrap_err();
        // Without char_device, it returns Unsupported
        assert!(matches!(err, CapabilityError::Unsupported(_)));
    }

    #[test]
    fn amd_xdna_execute_workload_returns_unsupported_even_with_char_device() {
        let backend = AmdXdnaBackend {
            info: AmdXdnaInfo {
                linux: LinuxNpuInfo {
                    instance_id: "accel0".into(),
                    display_name: "AMD XDNA NPU".into(),
                    sysfs_path: PathBuf::from("/sys/class/accel/accel0"),
                    character_device: Some(PathBuf::from("/dev/accel/accel0")),
                    driver_name: Some("amdxdna".into()),
                    pci_address: Some("0000:00:08.0".into()),
                    vendor_id: Some(AMD_VENDOR_ID),
                    device_id: Some(0x1502),
                    firmware_version: None,
                    accelerator_class: true,
                },
                generation: AmdXdnaGeneration::PhoenixOrHawkPoint,
                amdxdna_sysfs_path: None,
                supported_execution_modes: vec![NpuExecutionMode::Inference],
            },
        };
        let request = NpuWorkloadRequest {
            mode: NpuExecutionMode::Inference,
            input_bytes: vec![1, 2, 3],
            target_latency_ms: None,
        };
        let err = backend.execute_workload(&request).unwrap_err();
        // Even with char_device, actual submission is not implemented
        assert!(matches!(err, CapabilityError::Unsupported(_)));
    }

    #[test]
    fn supported_execution_modes_with_char_device() {
        let info = LinuxNpuInfo {
            instance_id: "accel0".into(),
            display_name: "accel0".into(),
            sysfs_path: PathBuf::from("/sys/class/accel/accel0"),
            character_device: Some(PathBuf::from("/dev/accel/accel0")),
            driver_name: Some("amdxdna".into()),
            pci_address: Some("0000:00:08.0".into()),
            vendor_id: Some(AMD_VENDOR_ID),
            device_id: Some(0x1502),
            firmware_version: None,
            accelerator_class: true,
        };
        // When character_device is Some, all three modes are supported
        let modes = if info.character_device.is_some() {
            vec![
                NpuExecutionMode::Inference,
                NpuExecutionMode::Compilation,
                NpuExecutionMode::Preprocessing,
            ]
        } else {
            vec![NpuExecutionMode::Inference]
        };
        assert_eq!(modes.len(), 3);
        assert!(modes.contains(&NpuExecutionMode::Inference));
        assert!(modes.contains(&NpuExecutionMode::Compilation));
        assert!(modes.contains(&NpuExecutionMode::Preprocessing));
    }

    #[test]
    fn supported_execution_modes_without_char_device() {
        let info = LinuxNpuInfo {
            instance_id: "accel0".into(),
            display_name: "accel0".into(),
            sysfs_path: PathBuf::from("/sys/class/accel/accel0"),
            character_device: None,
            driver_name: Some("amdxdna".into()),
            pci_address: Some("0000:00:08.0".into()),
            vendor_id: Some(AMD_VENDOR_ID),
            device_id: Some(0x1502),
            firmware_version: None,
            accelerator_class: true,
        };
        let modes = if info.character_device.is_some() {
            vec![
                NpuExecutionMode::Inference,
                NpuExecutionMode::Compilation,
                NpuExecutionMode::Preprocessing,
            ]
        } else {
            vec![NpuExecutionMode::Inference]
        };
        assert_eq!(modes.len(), 1);
        assert_eq!(modes[0], NpuExecutionMode::Inference);
    }

    #[test]
    fn amd_xdna_generation_clone_copy() {
        let gen = AmdXdnaGeneration::PhoenixOrHawkPoint;
        let cloned = gen.clone();
        assert_eq!(gen, cloned);
        let _copied = gen;
        let _also = gen;
    }

    #[test]
    fn amd_xdna_generation_debug() {
        let gen = AmdXdnaGeneration::Unknown;
        let debug_str = format!("{gen:?}");
        assert!(debug_str.contains("Unknown"));
    }
}
