// SPDX-License-Identifier: Apache-2.0

pub mod benchmark;
pub mod proto;

use edgerun_device_cap_core::{
    CapabilityReport, CapabilitySignal, CapabilityValue, DeviceDomainDiagnostic,
    DeviceDomainDiagnostics, DeviceDomainStatus, DeviceDomainStatuses, DomainFieldDiagnostic,
    HostEnvironmentCapabilities, ProbeConfidence, ProbeErrorCode, ProbeSource,
    QuantitativeResources, ReportMetadata,
};

pub struct PolicyContext {
    pub is_root: CapabilitySignal,
    pub cap_sys_admin: CapabilitySignal,
    pub cap_net_admin: CapabilitySignal,
    pub cap_sys_rawio: CapabilitySignal,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DomainResolvedPaths {
    pub detected: Vec<String>,
    pub available: Vec<String>,
    pub in_use: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResolvedSourcePaths {
    pub cpu: DomainResolvedPaths,
    pub storage: DomainResolvedPaths,
    pub gpu: DomainResolvedPaths,
    pub ram: DomainResolvedPaths,
    pub usb: DomainResolvedPaths,
    pub network: DomainResolvedPaths,
    pub input: DomainResolvedPaths,
    pub output: DomainResolvedPaths,
    pub bluetooth: DomainResolvedPaths,
    pub nfc: DomainResolvedPaths,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityReportWithDetails {
    pub report: CapabilityReport,
    pub resolved_sources: ResolvedSourcePaths,
}

pub fn probe_host_capabilities() -> HostEnvironmentCapabilities {
    probe_host_capabilities_impl()
}

pub fn probe_device_domains() -> DeviceDomainStatuses {
    probe_device_domains_impl()
}

pub fn probe_policy_context() -> PolicyContext {
    probe_policy_context_impl()
}

pub fn probe_capabilities_with_host() -> CapabilityReport {
    let host = probe_host_capabilities();
    let detected = probe_device_domains();
    let domains = apply_policy_to_domains(detected, &host);
    let metrics = probe_quantitative_resources();
    let metadata = probe_report_metadata();
    let diagnostics = probe_domain_diagnostics(detected, domains);

    edgerun_device_cap_core::probe_capabilities()
        .with_host(host)
        .with_domains(domains)
        .with_diagnostics(diagnostics)
        .with_metrics(metrics)
        .with_metadata(metadata)
}

pub fn probe_capabilities_with_host_details() -> CapabilityReportWithDetails {
    CapabilityReportWithDetails {
        report: probe_capabilities_with_host(),
        resolved_sources: probe_resolved_source_paths(),
    }
}

pub fn probe_quantitative_resources() -> QuantitativeResources {
    probe_quantitative_resources_impl()
}

pub fn probe_report_metadata() -> ReportMetadata {
    probe_report_metadata_impl()
}

pub fn probe_resolved_source_paths() -> ResolvedSourcePaths {
    probe_resolved_source_paths_impl()
}

pub fn probe_domain_diagnostics(
    detected: DeviceDomainStatuses,
    effective: DeviceDomainStatuses,
) -> DeviceDomainDiagnostics {
    DeviceDomainDiagnostics {
        cpu: diagnose_domain(
            detected.cpu,
            effective.cpu,
            "/proc/cpuinfo",
            "/proc/self/status",
            "/proc/loadavg",
        ),
        storage: diagnose_domain(
            detected.storage,
            effective.storage,
            "/sys/block|/proc/partitions",
            "/proc/partitions|/sys/block",
            "/proc/diskstats",
        ),
        gpu: diagnose_domain(
            detected.gpu,
            effective.gpu,
            "/dev/dri|/sys/class/drm|/dev/kgsl-3d0|/sys/class/kgsl",
            "/dev/dri|/sys/class/drm|/dev/kgsl-3d0|/sys/class/kgsl",
            "/sys/class/drm/card*/device/gpu_busy_percent",
        ),
        ram: diagnose_domain(
            detected.ram,
            effective.ram,
            "/proc/meminfo",
            "/proc/meminfo",
            "/proc/meminfo",
        ),
        usb: diagnose_domain(
            detected.usb,
            effective.usb,
            "/sys/bus/usb/devices",
            "/sys/bus/usb/devices",
            "/sys/bus/usb/devices",
        ),
        network: diagnose_domain(
            detected.network,
            effective.network,
            "/sys/class/net",
            "/sys/class/net|socket:0.0.0.0:0",
            "/sys/class/net/*/operstate",
        ),
        input: diagnose_domain(
            detected.input,
            effective.input,
            "/dev/input|/sys/class/input",
            "/dev/input|/sys/class/input",
            "/dev/input|/sys/class/input",
        ),
        output: diagnose_domain(
            detected.output,
            effective.output,
            "/sys/class/drm|/dev/fb0|/dev/tty0",
            "/sys/class/drm|/dev/fb0|/dev/tty0",
            "/sys/class/drm|/dev/fb0|/dev/tty0",
        ),
    }
}

fn apply_policy_to_domains(
    detected: DeviceDomainStatuses,
    host: &HostEnvironmentCapabilities,
) -> DeviceDomainStatuses {
    DeviceDomainStatuses {
        cpu: finalize_domain(
            detected.cpu,
            CapabilitySignal::supported(ProbeSource::Runtime),
        ),
        storage: finalize_domain(
            detected.storage,
            permission_read_any(&["/proc/partitions", "/sys/block"]),
        ),
        gpu: finalize_domain(detected.gpu, permission_gpu_access()),
        ram: finalize_domain(
            detected.ram,
            CapabilitySignal::supported(ProbeSource::Runtime),
        ),
        usb: finalize_domain(detected.usb, permission_read_any(&["/sys/bus/usb/devices"])),
        network: finalize_domain(detected.network, permission_network_access(host)),
        input: finalize_domain(
            detected.input,
            permission_read_any(&["/dev/input", "/sys/class/input"]),
        ),
        output: finalize_domain(detected.output, permission_output_access()),
    }
}

fn finalize_domain(
    detected: DeviceDomainStatus,
    permission: CapabilitySignal,
) -> DeviceDomainStatus {
    let available = match detected.detected.value {
        CapabilityValue::Unsupported => CapabilitySignal::unsupported(detected.detected.source),
        CapabilityValue::Unknown => CapabilitySignal::unknown(),
        CapabilityValue::Supported => match permission.value {
            CapabilityValue::Supported => CapabilitySignal::supported(ProbeSource::Runtime),
            CapabilityValue::Unsupported => CapabilitySignal::unsupported(ProbeSource::Runtime),
            CapabilityValue::Unknown => CapabilitySignal::unknown(),
        },
    };

    let in_use = match detected.detected.value {
        CapabilityValue::Unsupported => CapabilitySignal::unsupported(detected.detected.source),
        _ => detected.in_use,
    };

    DeviceDomainStatus {
        detected: detected.detected,
        available,
        in_use,
    }
}

fn diagnose_domain(
    detected: DeviceDomainStatus,
    effective: DeviceDomainStatus,
    detected_source_path: &'static str,
    available_source_path: &'static str,
    in_use_source_path: &'static str,
) -> DeviceDomainDiagnostic {
    DeviceDomainDiagnostic {
        detected: diagnose_detected_field(detected.detected, detected_source_path),
        available: diagnose_available_field(
            detected.detected,
            effective.available,
            available_source_path,
        ),
        in_use: diagnose_in_use_field(detected.detected, effective.in_use, in_use_source_path),
    }
}

fn diagnose_detected_field(
    signal: CapabilitySignal,
    source_path: &'static str,
) -> DomainFieldDiagnostic {
    match signal.value {
        CapabilityValue::Supported => DomainFieldDiagnostic {
            source_path: Some(source_path),
            ..DomainFieldDiagnostic::ok(ProbeConfidence::High)
        },
        CapabilityValue::Unsupported => DomainFieldDiagnostic {
            confidence: ProbeConfidence::Medium,
            error: ProbeErrorCode::NotFound,
            source_path: Some(source_path),
        },
        CapabilityValue::Unknown => DomainFieldDiagnostic {
            confidence: ProbeConfidence::Low,
            error: ProbeErrorCode::IoFailure,
            source_path: Some(source_path),
        },
    }
}

fn diagnose_available_field(
    detected: CapabilitySignal,
    available: CapabilitySignal,
    source_path: &'static str,
) -> DomainFieldDiagnostic {
    match available.value {
        CapabilityValue::Supported => DomainFieldDiagnostic {
            source_path: Some(source_path),
            ..DomainFieldDiagnostic::ok(ProbeConfidence::High)
        },
        CapabilityValue::Unsupported => {
            if matches!(detected.value, CapabilityValue::Supported) {
                DomainFieldDiagnostic {
                    confidence: ProbeConfidence::Medium,
                    error: ProbeErrorCode::PermissionDenied,
                    source_path: Some(source_path),
                }
            } else {
                DomainFieldDiagnostic {
                    confidence: ProbeConfidence::Medium,
                    error: ProbeErrorCode::NotSupported,
                    source_path: Some(source_path),
                }
            }
        }
        CapabilityValue::Unknown => DomainFieldDiagnostic {
            confidence: ProbeConfidence::Low,
            error: ProbeErrorCode::IoFailure,
            source_path: Some(source_path),
        },
    }
}

fn diagnose_in_use_field(
    detected: CapabilitySignal,
    in_use: CapabilitySignal,
    source_path: &'static str,
) -> DomainFieldDiagnostic {
    match in_use.value {
        CapabilityValue::Supported | CapabilityValue::Unsupported => DomainFieldDiagnostic {
            source_path: Some(source_path),
            ..DomainFieldDiagnostic::ok(ProbeConfidence::Medium)
        },
        CapabilityValue::Unknown => {
            if matches!(detected.value, CapabilityValue::Unsupported) {
                DomainFieldDiagnostic {
                    confidence: ProbeConfidence::Medium,
                    error: ProbeErrorCode::NotSupported,
                    source_path: Some(source_path),
                }
            } else {
                DomainFieldDiagnostic {
                    confidence: ProbeConfidence::Low,
                    error: ProbeErrorCode::NotSupported,
                    source_path: Some(source_path),
                }
            }
        }
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn probe_policy_context_impl() -> PolicyContext {
    let status = std::fs::read_to_string("/proc/self/status");

    let is_root = status
        .as_ref()
        .ok()
        .and_then(|contents| parse_linux_uid(contents))
        .map_or_else(CapabilitySignal::unknown, |uid| {
            bool_signal(uid == 0, ProbeSource::Runtime)
        });

    let caps = status
        .as_ref()
        .ok()
        .and_then(|contents| parse_linux_effective_caps(contents));

    PolicyContext {
        is_root,
        cap_sys_admin: caps
            .map(|mask| bool_signal(mask_has_cap(mask, 21), ProbeSource::Runtime))
            .unwrap_or_else(CapabilitySignal::unknown),
        cap_net_admin: caps
            .map(|mask| bool_signal(mask_has_cap(mask, 12), ProbeSource::Runtime))
            .unwrap_or_else(CapabilitySignal::unknown),
        cap_sys_rawio: caps
            .map(|mask| bool_signal(mask_has_cap(mask, 17), ProbeSource::Runtime))
            .unwrap_or_else(CapabilitySignal::unknown),
    }
}

#[cfg(target_os = "windows")]
fn probe_policy_context_impl() -> PolicyContext {
    let is_admin = windows_is_elevated_admin();
    PolicyContext {
        is_root: is_admin
            .map(|v| bool_signal(v, ProbeSource::Runtime))
            .unwrap_or_else(CapabilitySignal::unknown),
        cap_sys_admin: is_admin
            .map(|v| bool_signal(v, ProbeSource::Runtime))
            .unwrap_or_else(CapabilitySignal::unknown),
        cap_net_admin: CapabilitySignal::unknown(),
        cap_sys_rawio: CapabilitySignal::unknown(),
    }
}

#[cfg(not(any(target_os = "linux", target_os = "android", target_os = "windows")))]
fn probe_policy_context_impl() -> PolicyContext {
    PolicyContext {
        is_root: CapabilitySignal::unknown(),
        cap_sys_admin: CapabilitySignal::unknown(),
        cap_net_admin: CapabilitySignal::unknown(),
        cap_sys_rawio: CapabilitySignal::unknown(),
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn probe_host_capabilities_impl() -> HostEnvironmentCapabilities {
    let policy = probe_policy_context();

    HostEnvironmentCapabilities {
        container_detection: probe_linux_container_detection(),
        memory_limits: probe_linux_memory_limits(),
        filesystem_inventory: probe_linux_filesystem_inventory(),
        network_inventory: probe_linux_network_inventory(),
        gpu_inventory: probe_linux_gpu_inventory_detected(),
        bluetooth_inventory: probe_linux_bluetooth_inventory(),
        nfc_inventory: probe_linux_nfc_inventory(),
        policy_is_root: policy.is_root,
        policy_cap_sys_admin: policy.cap_sys_admin,
        policy_cap_net_admin: policy.cap_net_admin,
        policy_cap_sys_rawio: policy.cap_sys_rawio,
    }
}

#[cfg(target_os = "windows")]
fn probe_host_capabilities_impl() -> HostEnvironmentCapabilities {
    let policy = probe_policy_context();
    let network_counts = windows_network_adapter_counts();
    let network_if_count = network_counts.map(|(total, _)| total);
    HostEnvironmentCapabilities {
        container_detection: CapabilitySignal::unknown(),
        memory_limits: CapabilitySignal::unknown(),
        filesystem_inventory: bool_signal(
            windows_drive_count().map(|v| v > 0).unwrap_or(false),
            ProbeSource::Runtime,
        ),
        network_inventory: if network_if_count.map(|c| c > 0).unwrap_or(false) {
            CapabilitySignal::supported(ProbeSource::Runtime)
        } else if windows_can_bind_udp() {
            CapabilitySignal::supported(ProbeSource::Runtime)
        } else {
            CapabilitySignal::unknown()
        },
        gpu_inventory: CapabilitySignal::unknown(),
        bluetooth_inventory: windows_service_exists("bthserv")
            .map(|v| bool_signal(v, ProbeSource::Runtime))
            .unwrap_or_else(CapabilitySignal::unknown),
        nfc_inventory: windows_service_exists("NfcSvc")
            .map(|v| bool_signal(v, ProbeSource::Runtime))
            .unwrap_or_else(CapabilitySignal::unknown),
        policy_is_root: policy.is_root,
        policy_cap_sys_admin: policy.cap_sys_admin,
        policy_cap_net_admin: CapabilitySignal::unknown(),
        policy_cap_sys_rawio: CapabilitySignal::unknown(),
    }
}

#[cfg(not(any(target_os = "linux", target_os = "android", target_os = "windows")))]
fn probe_host_capabilities_impl() -> HostEnvironmentCapabilities {
    HostEnvironmentCapabilities {
        container_detection: CapabilitySignal::unknown(),
        memory_limits: CapabilitySignal::unknown(),
        filesystem_inventory: CapabilitySignal::unknown(),
        network_inventory: CapabilitySignal::unknown(),
        gpu_inventory: CapabilitySignal::unknown(),
        bluetooth_inventory: CapabilitySignal::unknown(),
        nfc_inventory: CapabilitySignal::unknown(),
        policy_is_root: CapabilitySignal::unknown(),
        policy_cap_sys_admin: CapabilitySignal::unknown(),
        policy_cap_net_admin: CapabilitySignal::unknown(),
        policy_cap_sys_rawio: CapabilitySignal::unknown(),
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn probe_device_domains_impl() -> DeviceDomainStatuses {
    DeviceDomainStatuses {
        cpu: domain(probe_linux_cpu_detected(), probe_linux_cpu_in_use()),
        storage: domain(probe_linux_storage_detected(), probe_linux_storage_in_use()),
        gpu: domain(
            probe_linux_gpu_inventory_detected(),
            probe_linux_gpu_in_use(),
        ),
        ram: domain(probe_linux_ram_detected(), probe_linux_ram_in_use()),
        usb: domain(probe_linux_usb_detected(), CapabilitySignal::unknown()),
        network: domain(
            probe_linux_network_inventory(),
            probe_linux_network_in_use(),
        ),
        input: domain(probe_linux_input_detected(), CapabilitySignal::unknown()),
        output: domain(probe_linux_output_detected(), CapabilitySignal::unknown()),
    }
}

#[cfg(target_os = "windows")]
fn probe_device_domains_impl() -> DeviceDomainStatuses {
    let (ram_total, ram_available) = windows_memory_total_available().unwrap_or((None, None));
    let ram_in_use = match (ram_total, ram_available) {
        (Some(t), Some(a)) if a < t => CapabilitySignal::supported(ProbeSource::Runtime),
        (Some(_), Some(_)) => CapabilitySignal::unsupported(ProbeSource::Runtime),
        _ => CapabilitySignal::unknown(),
    };
    let (storage_total, storage_available) =
        windows_storage_total_available_c().unwrap_or((None, None));
    let storage_in_use = match (storage_total, storage_available) {
        (Some(t), Some(a)) if a < t => CapabilitySignal::supported(ProbeSource::Runtime),
        (Some(_), Some(_)) => CapabilitySignal::unsupported(ProbeSource::Runtime),
        _ => CapabilitySignal::unknown(),
    };
    let network_counts = windows_network_adapter_counts();
    let network_if_count = network_counts.map(|(total, _)| total);
    let network_up_count = network_counts.map(|(_, up)| up);
    let cpu_detected = if std::thread::available_parallelism().is_ok() {
        CapabilitySignal::supported(ProbeSource::Runtime)
    } else {
        CapabilitySignal::unknown()
    };
    let storage_detected = windows_drive_count()
        .map(|v| bool_signal(v > 0, ProbeSource::Runtime))
        .unwrap_or_else(CapabilitySignal::unknown);
    let ram_detected = windows_memory_total_available()
        .map(|(total, _)| bool_signal(total.map(|v| v > 0).unwrap_or(false), ProbeSource::Runtime))
        .unwrap_or_else(CapabilitySignal::unknown);
    let network_detected =
        if network_if_count.map(|c| c > 0).unwrap_or(false) || windows_can_bind_udp() {
            CapabilitySignal::supported(ProbeSource::Runtime)
        } else {
            CapabilitySignal::unknown()
        };

    DeviceDomainStatuses {
        cpu: domain(cpu_detected, CapabilitySignal::unknown()),
        storage: domain(storage_detected, storage_in_use),
        gpu: domain(CapabilitySignal::unknown(), CapabilitySignal::unknown()),
        ram: domain(ram_detected, ram_in_use),
        usb: domain(CapabilitySignal::unknown(), CapabilitySignal::unknown()),
        network: domain(
            network_detected,
            if network_up_count.map(|c| c > 0).unwrap_or(false) {
                CapabilitySignal::supported(ProbeSource::Runtime)
            } else {
                CapabilitySignal::unsupported(ProbeSource::Runtime)
            },
        ),
        input: domain(CapabilitySignal::unknown(), CapabilitySignal::unknown()),
        output: domain(
            windows_monitor_count()
                .map(|c| bool_signal(c > 0, ProbeSource::Runtime))
                .unwrap_or_else(CapabilitySignal::unknown),
            CapabilitySignal::unknown(),
        ),
    }
}

#[cfg(not(any(target_os = "linux", target_os = "android", target_os = "windows")))]
fn probe_device_domains_impl() -> DeviceDomainStatuses {
    DeviceDomainStatuses::no_std_baseline()
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn probe_resolved_source_paths_impl() -> ResolvedSourcePaths {
    const PER_FIELD_LIMIT: usize = 16;
    ResolvedSourcePaths {
        cpu: DomainResolvedPaths {
            detected: normalize_paths(existing_paths(&["/proc/cpuinfo"]), PER_FIELD_LIMIT),
            available: normalize_paths(existing_paths(&["/proc/self/status"]), PER_FIELD_LIMIT),
            in_use: normalize_paths(existing_paths(&["/proc/loadavg"]), PER_FIELD_LIMIT),
        },
        storage: DomainResolvedPaths {
            detected: normalize_paths(
                merge_paths(
                    existing_paths(&["/proc/partitions"]),
                    list_entries("/sys/block", PER_FIELD_LIMIT),
                ),
                PER_FIELD_LIMIT,
            ),
            available: normalize_paths(
                existing_paths(&["/proc/partitions", "/sys/block"]),
                PER_FIELD_LIMIT,
            ),
            in_use: normalize_paths(existing_paths(&["/proc/diskstats"]), PER_FIELD_LIMIT),
        },
        gpu: DomainResolvedPaths {
            detected: normalize_paths(
                merge_paths(
                    existing_paths(&[
                        "/dev/dri",
                        "/sys/class/drm",
                        "/dev/kgsl-3d0",
                        "/sys/class/kgsl",
                    ]),
                    list_entries("/sys/class/drm", PER_FIELD_LIMIT),
                ),
                PER_FIELD_LIMIT,
            ),
            available: normalize_paths(
                existing_paths(&[
                    "/dev/dri",
                    "/sys/class/drm",
                    "/dev/kgsl-3d0",
                    "/sys/class/kgsl",
                ]),
                PER_FIELD_LIMIT,
            ),
            in_use: normalize_paths(
                merge_paths(
                    existing_paths(&["/sys/class/drm/card0/device/gpu_busy_percent"]),
                    list_gpu_busy_paths(PER_FIELD_LIMIT),
                ),
                PER_FIELD_LIMIT,
            ),
        },
        ram: DomainResolvedPaths {
            detected: normalize_paths(existing_paths(&["/proc/meminfo"]), PER_FIELD_LIMIT),
            available: normalize_paths(existing_paths(&["/proc/meminfo"]), PER_FIELD_LIMIT),
            in_use: normalize_paths(existing_paths(&["/proc/meminfo"]), PER_FIELD_LIMIT),
        },
        usb: DomainResolvedPaths {
            detected: normalize_paths(
                merge_paths(
                    existing_paths(&["/sys/bus/usb/devices"]),
                    list_entries("/sys/bus/usb/devices", PER_FIELD_LIMIT),
                ),
                PER_FIELD_LIMIT,
            ),
            available: normalize_paths(existing_paths(&["/sys/bus/usb/devices"]), PER_FIELD_LIMIT),
            in_use: normalize_paths(existing_paths(&["/sys/bus/usb/devices"]), PER_FIELD_LIMIT),
        },
        network: DomainResolvedPaths {
            detected: normalize_paths(
                merge_paths(
                    existing_paths(&["/sys/class/net"]),
                    list_entries("/sys/class/net", PER_FIELD_LIMIT),
                ),
                PER_FIELD_LIMIT,
            ),
            available: normalize_paths(existing_paths(&["/sys/class/net"]), PER_FIELD_LIMIT),
            in_use: normalize_paths(collect_operstate_paths(PER_FIELD_LIMIT), PER_FIELD_LIMIT),
        },
        input: DomainResolvedPaths {
            detected: normalize_paths(
                merge_paths(
                    existing_paths(&["/dev/input", "/sys/class/input"]),
                    list_entries("/sys/class/input", PER_FIELD_LIMIT),
                ),
                PER_FIELD_LIMIT,
            ),
            available: normalize_paths(
                existing_paths(&["/dev/input", "/sys/class/input"]),
                PER_FIELD_LIMIT,
            ),
            in_use: normalize_paths(
                existing_paths(&["/dev/input", "/sys/class/input"]),
                PER_FIELD_LIMIT,
            ),
        },
        output: DomainResolvedPaths {
            detected: normalize_paths(
                existing_paths(&["/sys/class/drm", "/dev/fb0", "/dev/tty0"]),
                PER_FIELD_LIMIT,
            ),
            available: normalize_paths(
                existing_paths(&["/sys/class/drm", "/dev/fb0", "/dev/tty0"]),
                PER_FIELD_LIMIT,
            ),
            in_use: normalize_paths(
                merge_paths(
                    existing_paths(&[
                        "/sys/class/drm/card0/device/gpu_busy_percent",
                        "/dev/fb0",
                        "/dev/tty0",
                    ]),
                    list_gpu_busy_paths(PER_FIELD_LIMIT),
                ),
                PER_FIELD_LIMIT,
            ),
        },
        bluetooth: DomainResolvedPaths {
            detected: normalize_paths(
                merge_paths(
                    existing_paths(&["/sys/class/bluetooth", "/sys/class/rfkill"]),
                    list_entries("/sys/class/bluetooth", PER_FIELD_LIMIT),
                ),
                PER_FIELD_LIMIT,
            ),
            available: normalize_paths(
                existing_paths(&["/sys/class/bluetooth", "/sys/class/rfkill"]),
                PER_FIELD_LIMIT,
            ),
            in_use: normalize_paths(existing_paths(&["/sys/class/bluetooth"]), PER_FIELD_LIMIT),
        },
        nfc: DomainResolvedPaths {
            detected: normalize_paths(
                merge_paths(
                    existing_paths(&["/sys/class/nfc", "/dev"]),
                    list_entries("/sys/class/nfc", PER_FIELD_LIMIT),
                ),
                PER_FIELD_LIMIT,
            ),
            available: normalize_paths(
                existing_paths(&["/sys/class/nfc", "/dev/nfc0"]),
                PER_FIELD_LIMIT,
            ),
            in_use: normalize_paths(existing_paths(&["/sys/class/nfc"]), PER_FIELD_LIMIT),
        },
    }
}

#[cfg(target_os = "windows")]
fn probe_resolved_source_paths_impl() -> ResolvedSourcePaths {
    ResolvedSourcePaths {
        cpu: DomainResolvedPaths {
            detected: vec!["GetActiveProcessorCount/std::thread::available_parallelism".to_string()],
            available: vec!["std::thread::available_parallelism".to_string()],
            in_use: Vec::new(),
        },
        storage: DomainResolvedPaths {
            detected: vec!["GetLogicalDrives".to_string()],
            available: vec!["GetDiskFreeSpaceExW(C:\\\\)".to_string()],
            in_use: Vec::new(),
        },
        gpu: DomainResolvedPaths::default(),
        ram: DomainResolvedPaths {
            detected: vec!["GlobalMemoryStatusEx".to_string()],
            available: vec!["GlobalMemoryStatusEx".to_string()],
            in_use: vec!["GlobalMemoryStatusEx".to_string()],
        },
        usb: DomainResolvedPaths::default(),
        network: DomainResolvedPaths {
            detected: vec!["UdpSocket::bind(0.0.0.0:0)".to_string()],
            available: vec!["UdpSocket::bind(0.0.0.0:0)".to_string()],
            in_use: Vec::new(),
        },
        input: DomainResolvedPaths::default(),
        output: DomainResolvedPaths::default(),
        bluetooth: DomainResolvedPaths {
            detected: vec!["Service:bthserv".to_string()],
            available: vec!["Service:bthserv".to_string()],
            in_use: Vec::new(),
        },
        nfc: DomainResolvedPaths {
            detected: vec!["Service:NfcSvc".to_string()],
            available: vec!["Service:NfcSvc".to_string()],
            in_use: Vec::new(),
        },
    }
}

#[cfg(not(any(target_os = "linux", target_os = "android", target_os = "windows")))]
fn probe_resolved_source_paths_impl() -> ResolvedSourcePaths {
    ResolvedSourcePaths::default()
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn probe_quantitative_resources_impl() -> QuantitativeResources {
    let cpu_total = std::thread::available_parallelism()
        .ok()
        .and_then(|n| u16::try_from(n.get()).ok());
    let cpu_allocatable = probe_linux_cpu_allocatable(cpu_total);
    let cpu_load_ratio_per_core_milli = probe_linux_cpu_load_ratio_per_core_milli(cpu_total);

    let meminfo = std::fs::read_to_string("/proc/meminfo").ok();
    let ram_total = meminfo
        .as_deref()
        .and_then(|s| parse_meminfo_value_kib(s, "MemTotal:"))
        .and_then(kib_to_bytes);
    let ram_available = meminfo
        .as_deref()
        .and_then(|s| parse_meminfo_value_kib(s, "MemAvailable:"))
        .and_then(kib_to_bytes);
    let ram_used = match (ram_total, ram_available) {
        (Some(total), Some(available)) => Some(total.saturating_sub(available)),
        _ => None,
    };

    let (storage_total, storage_available) = probe_linux_root_storage_bytes();
    let storage_used = match (storage_total, storage_available) {
        (Some(total), Some(available)) => Some(total.saturating_sub(available)),
        _ => None,
    };
    let gpu_total = count_drm_cards("/sys/class/drm");
    let gpu_available = gpu_total;
    let bt_count = probe_linux_bluetooth_adapter_count();
    let nfc_count = probe_linux_nfc_adapter_count();
    let network_links_up_count = probe_linux_network_links_up_count();
    let gpu_busy_percent = probe_linux_gpu_busy_percent();

    QuantitativeResources {
        cpu_logical_cores_total: cpu_total,
        cpu_logical_cores_allocatable: cpu_allocatable,
        ram_bytes_total: ram_total,
        ram_bytes_available: ram_available,
        ram_bytes_used: ram_used,
        storage_bytes_total: storage_total,
        storage_bytes_available: storage_available,
        storage_bytes_used: storage_used,
        gpu_count_total: gpu_total,
        gpu_count_available: gpu_available,
        network_interface_count: count_dir_entries("/sys/class/net"),
        network_links_up_count,
        usb_device_count: count_dir_entries("/sys/bus/usb/devices"),
        input_device_count: count_dir_entries("/sys/class/input"),
        output_device_count: output_device_count(),
        bluetooth_adapter_count: bt_count,
        nfc_adapter_count: nfc_count,
        cpu_load_ratio_per_core_milli,
        gpu_busy_percent,
    }
}

#[cfg(target_os = "windows")]
fn probe_quantitative_resources_impl() -> QuantitativeResources {
    let cpu_total = std::thread::available_parallelism()
        .ok()
        .and_then(|n| u16::try_from(n.get()).ok());
    let (ram_total, ram_available) = windows_memory_total_available().unwrap_or((None, None));
    let (storage_total, storage_available) =
        windows_storage_total_available_c().unwrap_or((None, None));
    let ram_used = match (ram_total, ram_available) {
        (Some(total), Some(avail)) => Some(total.saturating_sub(avail)),
        _ => None,
    };
    let storage_used = match (storage_total, storage_available) {
        (Some(total), Some(avail)) => Some(total.saturating_sub(avail)),
        _ => None,
    };

    let network_counts = windows_network_adapter_counts();
    let network_if_count = network_counts.map(|(total, _)| total);
    let network_up_count = network_counts.map(|(_, up)| up);
    let output_count = windows_monitor_count();
    let bt_count = windows_service_exists("bthserv").map(|v| if v { 1 } else { 0 });
    let nfc_count = windows_service_exists("NfcSvc").map(|v| if v { 1 } else { 0 });
    QuantitativeResources {
        cpu_logical_cores_total: cpu_total,
        cpu_logical_cores_allocatable: cpu_total,
        ram_bytes_total: ram_total,
        ram_bytes_available: ram_available,
        storage_bytes_total: storage_total,
        storage_bytes_available: storage_available,
        gpu_count_total: None,
        gpu_count_available: None,
        network_interface_count: network_if_count,
        usb_device_count: None,
        input_device_count: None,
        output_device_count: output_count,
        bluetooth_adapter_count: bt_count,
        nfc_adapter_count: nfc_count,
        cpu_load_ratio_per_core_milli: None,
        ram_bytes_used: ram_used,
        storage_bytes_used: storage_used,
        network_links_up_count: network_up_count,
        gpu_busy_percent: None,
    }
}

#[cfg(not(any(target_os = "linux", target_os = "android", target_os = "windows")))]
fn probe_quantitative_resources_impl() -> QuantitativeResources {
    QuantitativeResources::unknown()
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn probe_report_metadata_impl() -> ReportMetadata {
    let collected = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs());
    ReportMetadata {
        collected_unix_s: collected,
        ttl_s: Some(30),
    }
}

#[cfg(target_os = "windows")]
fn probe_report_metadata_impl() -> ReportMetadata {
    let collected = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs());
    ReportMetadata {
        collected_unix_s: collected,
        ttl_s: Some(30),
    }
}

#[cfg(not(any(target_os = "linux", target_os = "android", target_os = "windows")))]
fn probe_report_metadata_impl() -> ReportMetadata {
    ReportMetadata::unknown()
}

fn domain(detected: CapabilitySignal, in_use: CapabilitySignal) -> DeviceDomainStatus {
    DeviceDomainStatus {
        detected,
        available: CapabilitySignal::unknown(),
        in_use,
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn probe_linux_container_detection() -> CapabilitySignal {
    if std::path::Path::new("/.dockerenv").exists()
        || std::path::Path::new("/run/.containerenv").exists()
    {
        return CapabilitySignal::supported(ProbeSource::Runtime);
    }

    match std::fs::read_to_string("/proc/1/cgroup") {
        Ok(contents) => {
            if has_container_marker(&contents) {
                CapabilitySignal::supported(ProbeSource::Runtime)
            } else {
                CapabilitySignal::unsupported(ProbeSource::Runtime)
            }
        }
        Err(_) => CapabilitySignal::unknown(),
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn probe_linux_memory_limits() -> CapabilitySignal {
    for path in [
        "/sys/fs/cgroup/memory.max",
        "/sys/fs/cgroup/memory/memory.limit_in_bytes",
    ] {
        match std::fs::read_to_string(path) {
            Ok(contents) => {
                return if has_finite_memory_limit(&contents) {
                    CapabilitySignal::supported(ProbeSource::Runtime)
                } else {
                    CapabilitySignal::unsupported(ProbeSource::Runtime)
                };
            }
            Err(err) => {
                if err.kind() == std::io::ErrorKind::NotFound {
                    continue;
                }
                return CapabilitySignal::unknown();
            }
        }
    }

    CapabilitySignal::unknown()
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn probe_linux_cpu_detected() -> CapabilitySignal {
    match std::fs::read_to_string("/proc/cpuinfo") {
        Ok(contents) => {
            if contents.trim().is_empty() {
                CapabilitySignal::unsupported(ProbeSource::Runtime)
            } else {
                CapabilitySignal::supported(ProbeSource::Runtime)
            }
        }
        Err(_) => CapabilitySignal::unknown(),
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn probe_linux_cpu_in_use() -> CapabilitySignal {
    match probe_linux_cpu_load_ratio_per_core_milli(
        std::thread::available_parallelism()
            .ok()
            .and_then(|n| u16::try_from(n.get()).ok()),
    ) {
        Some(v) if v > 0 => CapabilitySignal::supported(ProbeSource::Runtime),
        Some(_) => CapabilitySignal::unsupported(ProbeSource::Runtime),
        None => CapabilitySignal::unknown(),
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn probe_linux_storage_detected() -> CapabilitySignal {
    if path_has_entries("/sys/block") {
        return CapabilitySignal::supported(ProbeSource::Runtime);
    }

    match std::fs::read_to_string("/proc/partitions") {
        Ok(contents) => {
            if contents.lines().skip(2).any(|line| !line.trim().is_empty()) {
                CapabilitySignal::supported(ProbeSource::Runtime)
            } else {
                CapabilitySignal::unsupported(ProbeSource::Runtime)
            }
        }
        Err(_) => CapabilitySignal::unknown(),
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn probe_linux_storage_in_use() -> CapabilitySignal {
    match probe_linux_disk_io_seen() {
        Some(true) => CapabilitySignal::supported(ProbeSource::Runtime),
        Some(false) => CapabilitySignal::unsupported(ProbeSource::Runtime),
        None => CapabilitySignal::unknown(),
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn probe_linux_ram_detected() -> CapabilitySignal {
    match std::fs::read_to_string("/proc/meminfo") {
        Ok(contents) => {
            if parse_meminfo_value_kib(&contents, "MemTotal:").is_some() {
                CapabilitySignal::supported(ProbeSource::Runtime)
            } else {
                CapabilitySignal::unsupported(ProbeSource::Runtime)
            }
        }
        Err(_) => CapabilitySignal::unknown(),
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn probe_linux_ram_in_use() -> CapabilitySignal {
    match std::fs::read_to_string("/proc/meminfo") {
        Ok(contents) => {
            let total = parse_meminfo_value_kib(&contents, "MemTotal:");
            let available = parse_meminfo_value_kib(&contents, "MemAvailable:");
            match (total, available) {
                (Some(t), Some(a)) if a < t => CapabilitySignal::supported(ProbeSource::Runtime),
                (Some(_), Some(_)) => CapabilitySignal::unsupported(ProbeSource::Runtime),
                _ => CapabilitySignal::unknown(),
            }
        }
        Err(_) => CapabilitySignal::unknown(),
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn probe_linux_usb_detected() -> CapabilitySignal {
    if path_has_entries("/sys/bus/usb/devices") {
        return CapabilitySignal::supported(ProbeSource::Runtime);
    }
    if std::path::Path::new("/sys/bus/usb/devices").exists() {
        return CapabilitySignal::unsupported(ProbeSource::Runtime);
    }
    CapabilitySignal::unknown()
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn probe_linux_filesystem_inventory() -> CapabilitySignal {
    match std::fs::read_to_string("/proc/mounts") {
        Ok(contents) => {
            if contents.trim().is_empty() {
                CapabilitySignal::unsupported(ProbeSource::Runtime)
            } else {
                CapabilitySignal::supported(ProbeSource::Runtime)
            }
        }
        Err(_) => CapabilitySignal::unknown(),
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn probe_linux_network_inventory() -> CapabilitySignal {
    match std::fs::read_dir("/sys/class/net") {
        Ok(mut entries) => {
            if entries.next().is_some() {
                CapabilitySignal::supported(ProbeSource::Runtime)
            } else {
                CapabilitySignal::unsupported(ProbeSource::Runtime)
            }
        }
        Err(_) => CapabilitySignal::unknown(),
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn probe_linux_network_in_use() -> CapabilitySignal {
    match probe_linux_network_links_up_count() {
        Some(v) if v > 0 => CapabilitySignal::supported(ProbeSource::Runtime),
        Some(_) => CapabilitySignal::unsupported(ProbeSource::Runtime),
        None => CapabilitySignal::unknown(),
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn probe_linux_input_detected() -> CapabilitySignal {
    if path_has_entries("/dev/input") || path_has_entries("/sys/class/input") {
        return CapabilitySignal::supported(ProbeSource::Runtime);
    }
    if std::path::Path::new("/dev/input").exists()
        || std::path::Path::new("/sys/class/input").exists()
    {
        return CapabilitySignal::unsupported(ProbeSource::Runtime);
    }
    CapabilitySignal::unknown()
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn probe_linux_output_detected() -> CapabilitySignal {
    if path_has_entries("/sys/class/drm")
        || std::path::Path::new("/dev/fb0").exists()
        || std::path::Path::new("/dev/tty0").exists()
    {
        return CapabilitySignal::supported(ProbeSource::Runtime);
    }
    if std::path::Path::new("/sys/class/drm").exists() {
        return CapabilitySignal::unsupported(ProbeSource::Runtime);
    }
    CapabilitySignal::unknown()
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn probe_linux_gpu_inventory_detected() -> CapabilitySignal {
    if path_has_entries("/dev/dri")
        || path_has_entries("/sys/class/drm")
        || std::path::Path::new("/dev/kgsl-3d0").exists()
        || path_has_entries("/sys/class/kgsl")
    {
        return CapabilitySignal::supported(ProbeSource::Runtime);
    }

    if std::path::Path::new("/dev/dri").exists()
        || std::path::Path::new("/sys/class/drm").exists()
        || std::path::Path::new("/dev/kgsl-3d0").exists()
        || std::path::Path::new("/sys/class/kgsl").exists()
    {
        return CapabilitySignal::unsupported(ProbeSource::Runtime);
    }

    CapabilitySignal::unsupported(ProbeSource::Runtime)
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn probe_linux_gpu_in_use() -> CapabilitySignal {
    match probe_linux_gpu_busy_percent() {
        Some(v) if v > 0 => CapabilitySignal::supported(ProbeSource::Runtime),
        Some(_) => CapabilitySignal::unsupported(ProbeSource::Runtime),
        None => CapabilitySignal::unknown(),
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn probe_linux_bluetooth_inventory() -> CapabilitySignal {
    match probe_linux_bluetooth_adapter_count() {
        Some(v) if v > 0 => CapabilitySignal::supported(ProbeSource::Runtime),
        Some(_) => CapabilitySignal::unsupported(ProbeSource::Runtime),
        None => CapabilitySignal::unknown(),
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn probe_linux_nfc_inventory() -> CapabilitySignal {
    match probe_linux_nfc_adapter_count() {
        Some(v) if v > 0 => CapabilitySignal::supported(ProbeSource::Runtime),
        Some(_) => CapabilitySignal::unsupported(ProbeSource::Runtime),
        None => CapabilitySignal::unknown(),
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn path_has_entries(path: &str) -> bool {
    match std::fs::read_dir(path) {
        Ok(mut entries) => entries.next().is_some(),
        Err(_) => false,
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn permission_read_any(paths: &[&str]) -> CapabilitySignal {
    let mut found = false;
    for path in paths {
        match std::fs::metadata(path) {
            Ok(meta) => {
                found = true;
                if meta.is_dir() {
                    match std::fs::read_dir(path) {
                        Ok(_) => return CapabilitySignal::supported(ProbeSource::Runtime),
                        Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => {
                            return CapabilitySignal::unsupported(ProbeSource::Runtime)
                        }
                        Err(_) => continue,
                    }
                } else {
                    match std::fs::File::open(path) {
                        Ok(_) => return CapabilitySignal::supported(ProbeSource::Runtime),
                        Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => {
                            return CapabilitySignal::unsupported(ProbeSource::Runtime)
                        }
                        Err(_) => continue,
                    }
                }
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
            Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => {
                return CapabilitySignal::unsupported(ProbeSource::Runtime)
            }
            Err(_) => continue,
        }
    }

    if found {
        CapabilitySignal::unsupported(ProbeSource::Runtime)
    } else {
        CapabilitySignal::unknown()
    }
}

#[cfg(target_os = "windows")]
fn permission_read_any(paths: &[&str]) -> CapabilitySignal {
    for path in paths {
        match std::fs::metadata(path) {
            Ok(_) => return CapabilitySignal::supported(ProbeSource::Runtime),
            Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => {
                return CapabilitySignal::unsupported(ProbeSource::Runtime)
            }
            Err(_) => continue,
        }
    }
    CapabilitySignal::unknown()
}

#[cfg(not(any(target_os = "linux", target_os = "android", target_os = "windows")))]
fn permission_read_any(_paths: &[&str]) -> CapabilitySignal {
    CapabilitySignal::unknown()
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn permission_gpu_access() -> CapabilitySignal {
    permission_read_any(&["/dev/dri", "/sys/class/drm"])
}

#[cfg(target_os = "windows")]
fn permission_gpu_access() -> CapabilitySignal {
    CapabilitySignal::unknown()
}

#[cfg(not(any(target_os = "linux", target_os = "android", target_os = "windows")))]
fn permission_gpu_access() -> CapabilitySignal {
    CapabilitySignal::unknown()
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn permission_output_access() -> CapabilitySignal {
    let tty = std::fs::OpenOptions::new().write(true).open("/dev/tty0");
    if tty.is_ok() {
        return CapabilitySignal::supported(ProbeSource::Runtime);
    }

    let fb = std::fs::OpenOptions::new().write(true).open("/dev/fb0");
    if fb.is_ok() {
        return CapabilitySignal::supported(ProbeSource::Runtime);
    }

    permission_read_any(&["/sys/class/drm"])
}

#[cfg(target_os = "windows")]
fn permission_output_access() -> CapabilitySignal {
    CapabilitySignal::unknown()
}

#[cfg(not(any(target_os = "linux", target_os = "android", target_os = "windows")))]
fn permission_output_access() -> CapabilitySignal {
    CapabilitySignal::unknown()
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn permission_network_access(host: &HostEnvironmentCapabilities) -> CapabilitySignal {
    let bind = std::net::UdpSocket::bind("0.0.0.0:0");
    if bind.is_ok() {
        return CapabilitySignal::supported(ProbeSource::Runtime);
    }

    if host.policy_is_root.value == CapabilityValue::Supported
        || host.policy_cap_net_admin.value == CapabilityValue::Supported
    {
        return CapabilitySignal::supported(ProbeSource::Runtime);
    }

    match bind {
        Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => {
            CapabilitySignal::unsupported(ProbeSource::Runtime)
        }
        Err(_) => CapabilitySignal::unknown(),
        Ok(_) => CapabilitySignal::supported(ProbeSource::Runtime),
    }
}

#[cfg(target_os = "windows")]
fn permission_network_access(_host: &HostEnvironmentCapabilities) -> CapabilitySignal {
    if windows_can_bind_udp() {
        CapabilitySignal::supported(ProbeSource::Runtime)
    } else {
        CapabilitySignal::unknown()
    }
}

#[cfg(not(any(target_os = "linux", target_os = "android", target_os = "windows")))]
fn permission_network_access(_host: &HostEnvironmentCapabilities) -> CapabilitySignal {
    CapabilitySignal::unknown()
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn probe_linux_cpu_allocatable(cpu_total: Option<u16>) -> Option<u16> {
    let quota = probe_linux_cpu_quota_cores();
    match (cpu_total, quota) {
        (Some(total), Some(q)) => Some(total.min(q)),
        (Some(total), None) => Some(total),
        (None, Some(q)) => Some(q),
        (None, None) => None,
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn probe_linux_cpu_quota_cores() -> Option<u16> {
    if let Ok(cpu_max) = std::fs::read_to_string("/sys/fs/cgroup/cpu.max") {
        let mut parts = cpu_max.split_whitespace();
        let quota = parts.next()?;
        let period = parts.next()?.parse::<u64>().ok()?;
        if quota == "max" || period == 0 {
            return None;
        }
        let quota_us = quota.parse::<u64>().ok()?;
        return ceil_div_u64_to_u16(quota_us, period);
    }

    let quota = std::fs::read_to_string("/sys/fs/cgroup/cpu/cpu.cfs_quota_us")
        .ok()
        .and_then(|v| v.trim().parse::<i64>().ok())?;
    if quota <= 0 {
        return None;
    }
    let period = std::fs::read_to_string("/sys/fs/cgroup/cpu/cpu.cfs_period_us")
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())?;
    if period == 0 {
        return None;
    }
    ceil_div_u64_to_u16(quota as u64, period)
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn ceil_div_u64_to_u16(numer: u64, denom: u64) -> Option<u16> {
    if denom == 0 {
        return None;
    }
    let value = numer.saturating_add(denom - 1) / denom;
    u16::try_from(value).ok()
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn kib_to_bytes(kib: u64) -> Option<u64> {
    kib.checked_mul(1024)
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn probe_linux_root_storage_bytes() -> (Option<u64>, Option<u64>) {
    use std::ffi::CString;
    use std::mem::MaybeUninit;

    let path = match CString::new("/") {
        Ok(v) => v,
        Err(_) => return (None, None),
    };

    let mut stat = MaybeUninit::<libc::statvfs>::uninit();
    // SAFETY: We pass a valid, NUL-terminated path and a valid out pointer.
    let rc = unsafe { libc::statvfs(path.as_ptr(), stat.as_mut_ptr()) };
    if rc != 0 {
        return (None, None);
    }

    // SAFETY: statvfs returned success, so the struct is initialized.
    let stat = unsafe { stat.assume_init() };
    let total = (stat.f_blocks as u128).checked_mul(stat.f_frsize as u128);
    let available = (stat.f_bavail as u128).checked_mul(stat.f_frsize as u128);

    (
        total.and_then(|v| u64::try_from(v).ok()),
        available.and_then(|v| u64::try_from(v).ok()),
    )
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn count_dir_entries(path: &str) -> Option<u16> {
    let entries = std::fs::read_dir(path).ok()?;
    let count = entries.filter_map(Result::ok).count();
    u16::try_from(count).ok()
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn count_drm_cards(path: &str) -> Option<u16> {
    let entries = std::fs::read_dir(path).ok()?;
    let count = entries
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_str()
                .map(|name| {
                    name.starts_with("card") && name[4..].chars().all(|c| c.is_ascii_digit())
                })
                .unwrap_or(false)
        })
        .count();
    u16::try_from(count).ok()
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn output_device_count() -> Option<u16> {
    let drm = count_drm_cards("/sys/class/drm").unwrap_or(0);
    let fb = if std::path::Path::new("/dev/fb0").exists() {
        1
    } else {
        0
    };
    let tty = if std::path::Path::new("/dev/tty0").exists() {
        1
    } else {
        0
    };
    let total = u32::from(drm) + fb + tty;
    u16::try_from(total).ok()
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn existing_paths(paths: &[&str]) -> Vec<String> {
    paths
        .iter()
        .filter_map(|path| {
            if std::path::Path::new(path).exists() {
                Some((*path).to_string())
            } else {
                None
            }
        })
        .collect()
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn list_entries(base: &str, limit: usize) -> Vec<String> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(base) else {
        return out;
    };
    for entry in entries.flatten().take(limit) {
        out.push(entry.path().display().to_string());
    }
    out
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn collect_operstate_paths(limit: usize) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir("/sys/class/net") else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in entries.flatten().take(limit) {
        let operstate = entry.path().join("operstate");
        if operstate.exists() {
            out.push(operstate.display().to_string());
        }
    }
    out
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn merge_paths(mut a: Vec<String>, b: Vec<String>) -> Vec<String> {
    for path in b {
        if !a.iter().any(|existing| existing == &path) {
            a.push(path);
        }
    }
    a
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn normalize_paths(mut paths: Vec<String>, limit: usize) -> Vec<String> {
    paths.sort();
    paths.dedup();
    if paths.len() > limit {
        paths.truncate(limit);
    }
    paths
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn list_gpu_busy_paths(limit: usize) -> Vec<String> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir("/sys/class/drm") else {
        return out;
    };
    for entry in entries.flatten().take(limit) {
        let path = entry.path().join("device").join("gpu_busy_percent");
        if path.exists() {
            out.push(path.display().to_string());
        }
    }
    out
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn probe_linux_bluetooth_adapter_count() -> Option<u16> {
    if let Some(count) = count_dir_entries("/sys/class/bluetooth") {
        return Some(count);
    }
    let Ok(entries) = std::fs::read_dir("/sys/class/rfkill") else {
        return None;
    };
    let mut count = 0u32;
    for entry in entries.flatten() {
        let ty = entry.path().join("type");
        if let Ok(kind) = std::fs::read_to_string(ty) {
            if kind.trim() == "bluetooth" {
                count = count.saturating_add(1);
            }
        }
    }
    u16::try_from(count).ok()
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn probe_linux_nfc_adapter_count() -> Option<u16> {
    if let Some(count) = count_dir_entries("/sys/class/nfc") {
        return Some(count);
    }
    let Ok(entries) = std::fs::read_dir("/dev") else {
        return None;
    };
    let mut count = 0u32;
    for entry in entries.flatten() {
        if let Some(name) = entry.file_name().to_str() {
            if name.starts_with("nfc") {
                count = count.saturating_add(1);
            }
        }
    }
    u16::try_from(count).ok()
}

#[cfg(target_os = "windows")]
fn windows_can_bind_udp() -> bool {
    std::net::UdpSocket::bind("0.0.0.0:0").is_ok()
}

#[cfg(target_os = "windows")]
fn windows_is_elevated_admin() -> Option<bool> {
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::Security::{
        GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
    };
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    let mut token: HANDLE = core::ptr::null_mut();
    // SAFETY: OpenProcessToken takes current process handle and writes token handle.
    let opened = unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) };
    if opened == 0 {
        return None;
    }

    let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
    let mut returned_len = 0u32;
    // SAFETY: pointers are valid and sized for TOKEN_ELEVATION output.
    let ok = unsafe {
        GetTokenInformation(
            token,
            TokenElevation,
            &mut elevation as *mut TOKEN_ELEVATION as *mut core::ffi::c_void,
            core::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut returned_len as *mut u32,
        )
    };
    // SAFETY: token was opened successfully and must be closed.
    unsafe { CloseHandle(token) };

    if ok == 0 {
        None
    } else {
        Some(elevation.TokenIsElevated != 0)
    }
}

#[cfg(target_os = "windows")]
fn windows_drive_count() -> Option<u16> {
    use windows_sys::Win32::Storage::FileSystem::GetLogicalDrives;
    // SAFETY: GetLogicalDrives has no preconditions and returns a bitmask.
    let mask = unsafe { GetLogicalDrives() };
    if mask == 0 {
        return None;
    }
    u16::try_from(mask.count_ones()).ok()
}

#[cfg(target_os = "windows")]
fn windows_network_adapter_counts() -> Option<(u16, u16)> {
    use windows_sys::Win32::Foundation::{ERROR_BUFFER_OVERFLOW, NO_ERROR};
    use windows_sys::Win32::NetworkManagement::IpHelper::{
        GetAdaptersAddresses, IP_ADAPTER_ADDRESSES_LH,
    };
    use windows_sys::Win32::NetworkManagement::Ndis::IfOperStatusUp;
    use windows_sys::Win32::Networking::WinSock::AF_UNSPEC;

    let mut buflen: u32 = 15_000;
    for _ in 0..3 {
        let mut buffer = vec![0u8; buflen as usize];
        let ptr = buffer.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES_LH;
        // SAFETY: buffer is valid for writes up to buflen bytes.
        let ret = unsafe {
            GetAdaptersAddresses(
                AF_UNSPEC as u32,
                0,
                core::ptr::null_mut(),
                ptr,
                &mut buflen as *mut u32,
            )
        };

        if ret == ERROR_BUFFER_OVERFLOW {
            continue;
        }
        if ret != NO_ERROR {
            return None;
        }

        let mut count = 0u32;
        let mut up_count = 0u32;
        let mut current = ptr;
        while !current.is_null() {
            count = count.saturating_add(1);
            // SAFETY: current points to valid adapter node.
            let oper = unsafe { (*current).OperStatus };
            if oper == IfOperStatusUp {
                up_count = up_count.saturating_add(1);
            }
            // SAFETY: list nodes returned by GetAdaptersAddresses are valid linked nodes.
            current = unsafe { (*current).Next };
        }
        let total = u16::try_from(count).ok()?;
        let up = u16::try_from(up_count).ok()?;
        return Some((total, up));
    }
    None
}

#[cfg(target_os = "windows")]
fn windows_memory_total_available() -> Option<(Option<u64>, Option<u64>)> {
    use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    // SAFETY: zeroed MEMORYSTATUSEX is valid when dwLength is set before call.
    let mut status: MEMORYSTATUSEX = unsafe { core::mem::zeroed() };
    status = MEMORYSTATUSEX {
        dwLength: core::mem::size_of::<MEMORYSTATUSEX>() as u32,
        ..status
    };
    // SAFETY: status points to valid writable MEMORYSTATUSEX with correct dwLength.
    let ok = unsafe { GlobalMemoryStatusEx(&mut status as *mut MEMORYSTATUSEX) };
    if ok == 0 {
        return None;
    }
    Some((Some(status.ullTotalPhys), Some(status.ullAvailPhys)))
}

#[cfg(target_os = "windows")]
fn windows_monitor_count() -> Option<u16> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CMONITORS};
    // SAFETY: GetSystemMetrics has no preconditions.
    let monitors = unsafe { GetSystemMetrics(SM_CMONITORS) };
    if monitors <= 0 {
        None
    } else {
        u16::try_from(monitors as u32).ok()
    }
}

#[cfg(target_os = "windows")]
fn windows_storage_total_available_c() -> Option<(Option<u64>, Option<u64>)> {
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    let path: Vec<u16> = "C:\\".encode_utf16().chain(core::iter::once(0)).collect();
    let mut free_for_caller = 0u64;
    let mut total = 0u64;
    let mut free_total = 0u64;
    // SAFETY: pointers are valid for writable u64 values and path is NUL-terminated UTF-16.
    let ok = unsafe {
        GetDiskFreeSpaceExW(
            path.as_ptr(),
            &mut free_for_caller as *mut u64,
            &mut total as *mut u64,
            &mut free_total as *mut u64,
        )
    };
    if ok == 0 {
        return None;
    }
    Some((Some(total), Some(free_for_caller)))
}

#[cfg(target_os = "windows")]
fn windows_service_exists(service_name: &str) -> Option<bool> {
    use windows_sys::Win32::Foundation::GetLastError;
    use windows_sys::Win32::System::Services::{
        CloseServiceHandle, OpenSCManagerW, OpenServiceW, SC_MANAGER_CONNECT, SERVICE_QUERY_STATUS,
    };

    let scm = unsafe { OpenSCManagerW(core::ptr::null(), core::ptr::null(), SC_MANAGER_CONNECT) };
    if scm.is_null() {
        return None;
    }

    let wide: Vec<u16> = service_name
        .encode_utf16()
        .chain(core::iter::once(0))
        .collect();
    let service = unsafe { OpenServiceW(scm, wide.as_ptr(), SERVICE_QUERY_STATUS) };
    let result = if service.is_null() {
        let err = unsafe { GetLastError() };
        if err == 1060 {
            Some(false)
        } else {
            None
        }
    } else {
        unsafe { CloseServiceHandle(service) };
        Some(true)
    };

    unsafe { CloseServiceHandle(scm) };
    result
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn probe_linux_cpu_load_ratio_per_core_milli(cpu_total: Option<u16>) -> Option<u16> {
    let total = cpu_total?;
    if total == 0 {
        return None;
    }
    let contents = std::fs::read_to_string("/proc/loadavg").ok()?;
    let load = contents.split_whitespace().next()?.parse::<f64>().ok()?;
    let per_core = load / f64::from(total);
    let milli = (per_core * 1000.0).round();
    if milli.is_finite() && milli >= 0.0 && milli <= f64::from(u16::MAX) {
        Some(milli as u16)
    } else {
        None
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn probe_linux_network_links_up_count() -> Option<u16> {
    let entries = std::fs::read_dir("/sys/class/net").ok()?;
    let mut up = 0u32;
    for entry in entries.flatten() {
        let state_path = entry.path().join("operstate");
        if let Ok(state) = std::fs::read_to_string(state_path) {
            if state.trim() == "up" {
                up = up.saturating_add(1);
            }
        }
    }
    u16::try_from(up).ok()
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn probe_linux_gpu_busy_percent() -> Option<u8> {
    let entries = std::fs::read_dir("/sys/class/drm").ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if !(name.starts_with("card") && name[4..].chars().all(|c| c.is_ascii_digit())) {
            continue;
        }
        let path = entry.path().join("device").join("gpu_busy_percent");
        if let Ok(contents) = std::fs::read_to_string(path) {
            if let Ok(val) = contents.trim().parse::<u8>() {
                return Some(val.min(100));
            }
        }
    }
    None
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn probe_linux_disk_io_seen() -> Option<bool> {
    let contents = std::fs::read_to_string("/proc/diskstats").ok()?;
    let mut saw_any = false;
    for line in contents.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 8 {
            continue;
        }
        let name = parts[2];
        if name.starts_with("loop") || name.starts_with("ram") {
            continue;
        }
        saw_any = true;
        let reads = parts[3].parse::<u64>().ok().unwrap_or(0);
        let writes = parts[7].parse::<u64>().ok().unwrap_or(0);
        if reads > 0 || writes > 0 {
            return Some(true);
        }
    }
    if saw_any {
        Some(false)
    } else {
        None
    }
}

#[cfg(any(target_os = "linux", target_os = "android", test))]
fn has_container_marker(contents: &str) -> bool {
    const MARKERS: [&str; 6] = [
        "docker",
        "containerd",
        "kubepods",
        "podman",
        "lxc",
        "libpod",
    ];
    MARKERS.iter().any(|marker| contents.contains(marker))
}

#[cfg(any(target_os = "linux", target_os = "android", test))]
fn has_finite_memory_limit(contents: &str) -> bool {
    let value = contents.trim();
    if value.is_empty() || value == "max" {
        return false;
    }

    match value.parse::<u128>() {
        Ok(parsed) => parsed > 0 && parsed < u64::MAX as u128,
        Err(_) => false,
    }
}

#[cfg(any(target_os = "linux", target_os = "android", test))]
fn parse_meminfo_value_kib(contents: &str, key: &str) -> Option<u64> {
    let line = contents.lines().find(|line| line.starts_with(key))?;
    let mut parts = line.split_whitespace();
    let _ = parts.next()?;
    let value = parts.next()?.parse::<u64>().ok()?;
    if value > 0 {
        Some(value)
    } else {
        None
    }
}

#[cfg(any(target_os = "linux", target_os = "android", test))]
fn parse_linux_uid(contents: &str) -> Option<u32> {
    let line = contents.lines().find(|line| line.starts_with("Uid:"))?;
    let mut parts = line.split_whitespace();
    let _ = parts.next()?;
    parts.next()?.parse::<u32>().ok()
}

#[cfg(any(target_os = "linux", target_os = "android", test))]
fn parse_linux_effective_caps(contents: &str) -> Option<u64> {
    let line = contents.lines().find(|line| line.starts_with("CapEff:"))?;
    let mut parts = line.split_whitespace();
    let _ = parts.next()?;
    u64::from_str_radix(parts.next()?, 16).ok()
}

#[cfg(any(target_os = "linux", target_os = "android", test))]
fn mask_has_cap(mask: u64, cap_index: u8) -> bool {
    let bit = 1u64.checked_shl(cap_index as u32).unwrap_or(0);
    bit != 0 && (mask & bit) != 0
}

const fn bool_signal(value: bool, source: ProbeSource) -> CapabilitySignal {
    if value {
        CapabilitySignal::supported(source)
    } else {
        CapabilitySignal::unsupported(source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn container_markers_detected() {
        assert!(has_container_marker("12:cpu:/docker/abc"));
        assert!(has_container_marker("0::/kubepods/besteffort"));
        assert!(!has_container_marker("0::/user.slice"));
    }

    #[test]
    fn finite_memory_limit_detection() {
        assert!(!has_finite_memory_limit("max"));
        assert!(!has_finite_memory_limit("0"));
        assert!(has_finite_memory_limit("1073741824"));
        assert!(!has_finite_memory_limit("18446744073709551615"));
    }

    #[test]
    fn parse_memtotal_kib_parses_expected_line() {
        let sample = "MemTotal:       16384256 kB\nMemFree:         1234567 kB\n";
        assert_eq!(parse_meminfo_value_kib(sample, "MemTotal:"), Some(16384256));
    }

    #[test]
    fn parse_linux_status_fields() {
        let sample = "Uid:\t1000\t1000\t1000\t1000\nCapEff:\t0000000000001000\n";
        assert_eq!(parse_linux_uid(sample), Some(1000));
        assert_eq!(parse_linux_effective_caps(sample), Some(0x1000));
        assert!(mask_has_cap(0x1000, 12));
    }

    #[test]
    fn merged_probe_keeps_core_shape() {
        let report = probe_capabilities_with_host();
        let _ = report.target.arch;
        let _ = report.platform.atomic_ptr;
        let _ = report.cpu.simd128;
        let _ = report.host.network_inventory;
        let _ = report.domains.usb.detected;
        let _ = report.domains.usb.available;
        let _ = report.domains.usb.in_use;
        assert_eq!(
            report.diagnostics.cpu.detected.source_path,
            Some("/proc/cpuinfo")
        );
    }

    #[test]
    fn detailed_probe_exposes_resolved_paths() {
        let details = probe_capabilities_with_host_details();
        let _ = details.report.metrics.cpu_logical_cores_total;
        assert!(details
            .resolved_sources
            .cpu
            .detected
            .iter()
            .any(|p| p == "/proc/cpuinfo"));
    }
}
