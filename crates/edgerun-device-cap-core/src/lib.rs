// SPDX-License-Identifier: Apache-2.0
#![no_std]

pub mod arch;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityValue {
    Supported,
    Unsupported,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeSource {
    Runtime,
    CompileTime,
    NotAvailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapabilitySignal {
    pub value: CapabilityValue,
    pub source: ProbeSource,
}

impl CapabilitySignal {
    pub const fn supported(source: ProbeSource) -> Self {
        Self {
            value: CapabilityValue::Supported,
            source,
        }
    }

    pub const fn unsupported(source: ProbeSource) -> Self {
        Self {
            value: CapabilityValue::Unsupported,
            source,
        }
    }

    pub const fn unknown() -> Self {
        Self {
            value: CapabilityValue::Unknown,
            source: ProbeSource::NotAvailable,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpuCapabilities {
    pub simd128: CapabilitySignal,
    pub simd256: CapabilitySignal,
    pub simd512: CapabilitySignal,
    pub aes: CapabilitySignal,
    pub sha2: CapabilitySignal,
    pub crc32c: CapabilitySignal,
    pub random: CapabilitySignal,
    pub virtualization: CapabilitySignal,
    pub atomics_64: CapabilitySignal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlatformCapabilities {
    pub family_unix: CapabilitySignal,
    pub family_windows: CapabilitySignal,
    pub family_wasm: CapabilitySignal,
    pub atomic_8: CapabilitySignal,
    pub atomic_16: CapabilitySignal,
    pub atomic_32: CapabilitySignal,
    pub atomic_64: CapabilitySignal,
    pub atomic_ptr: CapabilitySignal,
    pub panic_abort: CapabilitySignal,
    pub panic_unwind: CapabilitySignal,
}

impl PlatformCapabilities {
    pub const fn current() -> Self {
        Self {
            family_unix: bool_signal(cfg!(target_family = "unix"), ProbeSource::CompileTime),
            family_windows: bool_signal(cfg!(target_family = "windows"), ProbeSource::CompileTime),
            family_wasm: bool_signal(cfg!(target_family = "wasm"), ProbeSource::CompileTime),
            atomic_8: bool_signal(cfg!(target_has_atomic = "8"), ProbeSource::CompileTime),
            atomic_16: bool_signal(cfg!(target_has_atomic = "16"), ProbeSource::CompileTime),
            atomic_32: bool_signal(cfg!(target_has_atomic = "32"), ProbeSource::CompileTime),
            atomic_64: bool_signal(cfg!(target_has_atomic = "64"), ProbeSource::CompileTime),
            atomic_ptr: bool_signal(cfg!(target_has_atomic = "ptr"), ProbeSource::CompileTime),
            panic_abort: bool_signal(cfg!(panic = "abort"), ProbeSource::CompileTime),
            panic_unwind: bool_signal(cfg!(panic = "unwind"), ProbeSource::CompileTime),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostEnvironmentCapabilities {
    pub container_detection: CapabilitySignal,
    pub memory_limits: CapabilitySignal,
    pub filesystem_inventory: CapabilitySignal,
    pub network_inventory: CapabilitySignal,
    pub gpu_inventory: CapabilitySignal,
    pub bluetooth_inventory: CapabilitySignal,
    pub nfc_inventory: CapabilitySignal,
    pub policy_is_root: CapabilitySignal,
    pub policy_cap_sys_admin: CapabilitySignal,
    pub policy_cap_net_admin: CapabilitySignal,
    pub policy_cap_sys_rawio: CapabilitySignal,
}

impl HostEnvironmentCapabilities {
    pub const fn no_std_unknowns() -> Self {
        Self {
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviceDomainStatus {
    pub detected: CapabilitySignal,
    pub available: CapabilitySignal,
    pub in_use: CapabilitySignal,
}

impl DeviceDomainStatus {
    pub const fn unknown() -> Self {
        Self {
            detected: CapabilitySignal::unknown(),
            available: CapabilitySignal::unknown(),
            in_use: CapabilitySignal::unknown(),
        }
    }

    pub const fn detected_only(detected: CapabilitySignal) -> Self {
        Self {
            detected,
            available: CapabilitySignal::unknown(),
            in_use: CapabilitySignal::unknown(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviceDomainStatuses {
    pub cpu: DeviceDomainStatus,
    pub storage: DeviceDomainStatus,
    pub gpu: DeviceDomainStatus,
    pub ram: DeviceDomainStatus,
    pub usb: DeviceDomainStatus,
    pub network: DeviceDomainStatus,
    pub input: DeviceDomainStatus,
    pub output: DeviceDomainStatus,
}

impl DeviceDomainStatuses {
    pub const fn no_std_baseline() -> Self {
        Self {
            cpu: DeviceDomainStatus::detected_only(
                if cfg!(any(
                    target_arch = "x86_64",
                    target_arch = "aarch64",
                    target_arch = "x86",
                    target_arch = "arm",
                    target_arch = "mips",
                    target_arch = "mips64",
                    target_arch = "mips32r6",
                    target_arch = "mips64r6",
                    target_arch = "riscv64",
                    target_arch = "riscv32"
                )) {
                    CapabilitySignal::supported(ProbeSource::CompileTime)
                } else {
                    CapabilitySignal::unknown()
                },
            ),
            storage: DeviceDomainStatus::unknown(),
            gpu: DeviceDomainStatus::unknown(),
            ram: DeviceDomainStatus::unknown(),
            usb: DeviceDomainStatus::unknown(),
            network: DeviceDomainStatus::unknown(),
            input: DeviceDomainStatus::unknown(),
            output: DeviceDomainStatus::unknown(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeConfidence {
    High,
    Medium,
    Low,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeErrorCode {
    None,
    PermissionDenied,
    NotFound,
    ParseFailed,
    IoFailure,
    NotSupported,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DomainFieldDiagnostic {
    pub confidence: ProbeConfidence,
    pub error: ProbeErrorCode,
    pub source_path: Option<&'static str>,
}

impl DomainFieldDiagnostic {
    pub const fn unknown() -> Self {
        Self {
            confidence: ProbeConfidence::Unknown,
            error: ProbeErrorCode::Unknown,
            source_path: None,
        }
    }

    pub const fn ok(confidence: ProbeConfidence) -> Self {
        Self {
            confidence,
            error: ProbeErrorCode::None,
            source_path: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviceDomainDiagnostic {
    pub detected: DomainFieldDiagnostic,
    pub available: DomainFieldDiagnostic,
    pub in_use: DomainFieldDiagnostic,
}

impl DeviceDomainDiagnostic {
    pub const fn unknown() -> Self {
        Self {
            detected: DomainFieldDiagnostic::unknown(),
            available: DomainFieldDiagnostic::unknown(),
            in_use: DomainFieldDiagnostic::unknown(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviceDomainDiagnostics {
    pub cpu: DeviceDomainDiagnostic,
    pub storage: DeviceDomainDiagnostic,
    pub gpu: DeviceDomainDiagnostic,
    pub ram: DeviceDomainDiagnostic,
    pub usb: DeviceDomainDiagnostic,
    pub network: DeviceDomainDiagnostic,
    pub input: DeviceDomainDiagnostic,
    pub output: DeviceDomainDiagnostic,
}

impl DeviceDomainDiagnostics {
    pub const fn unknown() -> Self {
        Self {
            cpu: DeviceDomainDiagnostic::unknown(),
            storage: DeviceDomainDiagnostic::unknown(),
            gpu: DeviceDomainDiagnostic::unknown(),
            ram: DeviceDomainDiagnostic::unknown(),
            usb: DeviceDomainDiagnostic::unknown(),
            network: DeviceDomainDiagnostic::unknown(),
            input: DeviceDomainDiagnostic::unknown(),
            output: DeviceDomainDiagnostic::unknown(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuantitativeResources {
    pub cpu_logical_cores_total: Option<u16>,
    pub cpu_logical_cores_allocatable: Option<u16>,
    pub ram_bytes_total: Option<u64>,
    pub ram_bytes_available: Option<u64>,
    pub storage_bytes_total: Option<u64>,
    pub storage_bytes_available: Option<u64>,
    pub gpu_count_total: Option<u16>,
    pub gpu_count_available: Option<u16>,
    pub network_interface_count: Option<u16>,
    pub usb_device_count: Option<u16>,
    pub input_device_count: Option<u16>,
    pub output_device_count: Option<u16>,
    pub bluetooth_adapter_count: Option<u16>,
    pub nfc_adapter_count: Option<u16>,
    pub cpu_load_ratio_per_core_milli: Option<u16>,
    pub ram_bytes_used: Option<u64>,
    pub storage_bytes_used: Option<u64>,
    pub network_links_up_count: Option<u16>,
    pub gpu_busy_percent: Option<u8>,
}

impl QuantitativeResources {
    pub const fn unknown() -> Self {
        Self {
            cpu_logical_cores_total: None,
            cpu_logical_cores_allocatable: None,
            ram_bytes_total: None,
            ram_bytes_available: None,
            storage_bytes_total: None,
            storage_bytes_available: None,
            gpu_count_total: None,
            gpu_count_available: None,
            network_interface_count: None,
            usb_device_count: None,
            input_device_count: None,
            output_device_count: None,
            bluetooth_adapter_count: None,
            nfc_adapter_count: None,
            cpu_load_ratio_per_core_milli: None,
            ram_bytes_used: None,
            storage_bytes_used: None,
            network_links_up_count: None,
            gpu_busy_percent: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReportMetadata {
    pub collected_unix_s: Option<u64>,
    pub ttl_s: Option<u32>,
}

impl ReportMetadata {
    pub const fn unknown() -> Self {
        Self {
            collected_unix_s: None,
            ttl_s: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JobRequirements {
    pub min_cpu_cores: Option<u16>,
    pub min_ram_bytes: Option<u64>,
    pub min_storage_bytes: Option<u64>,
    pub min_gpu_count: Option<u16>,
    pub require_network: bool,
    pub require_usb: bool,
    pub require_input: bool,
    pub require_output: bool,
    pub max_report_age_s: Option<u64>,
}

impl JobRequirements {
    pub const fn conservative() -> Self {
        Self {
            min_cpu_cores: Some(1),
            min_ram_bytes: None,
            min_storage_bytes: None,
            min_gpu_count: None,
            require_network: true,
            require_usb: false,
            require_input: false,
            require_output: false,
            max_report_age_s: Some(120),
        }
    }
}

pub const BLOCKER_STALE_REPORT: u32 = 1 << 0;
pub const BLOCKER_MISSING_CPU_METRICS: u32 = 1 << 1;
pub const BLOCKER_MISSING_RAM_METRICS: u32 = 1 << 2;
pub const BLOCKER_MISSING_STORAGE_METRICS: u32 = 1 << 3;
pub const BLOCKER_MISSING_GPU_METRICS: u32 = 1 << 4;
pub const BLOCKER_CPU_UNAVAILABLE: u32 = 1 << 5;
pub const BLOCKER_RAM_UNAVAILABLE: u32 = 1 << 6;
pub const BLOCKER_STORAGE_UNAVAILABLE: u32 = 1 << 7;
pub const BLOCKER_GPU_UNAVAILABLE: u32 = 1 << 8;
pub const BLOCKER_NETWORK_UNAVAILABLE: u32 = 1 << 9;
pub const BLOCKER_USB_UNAVAILABLE: u32 = 1 << 10;
pub const BLOCKER_INPUT_UNAVAILABLE: u32 = 1 << 11;
pub const BLOCKER_OUTPUT_UNAVAILABLE: u32 = 1 << 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlacementEvaluation {
    pub eligible: bool,
    pub blockers: u32,
}

impl PlacementEvaluation {
    pub const fn blocked_by(self, blocker: u32) -> bool {
        (self.blockers & blocker) != 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TargetInfo {
    pub arch: &'static str,
    pub os: &'static str,
    pub env: &'static str,
    pub vendor: &'static str,
    pub endian: &'static str,
    pub pointer_width: &'static str,
}

impl TargetInfo {
    pub const fn current() -> Self {
        Self {
            arch: current_arch(),
            os: current_os(),
            env: current_env(),
            vendor: current_vendor(),
            endian: current_endian(),
            pointer_width: current_pointer_width(),
        }
    }
}

const fn current_arch() -> &'static str {
    if cfg!(target_arch = "x86_64") {
        "x86_64"
    } else if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else if cfg!(target_arch = "x86") {
        "x86"
    } else if cfg!(target_arch = "arm") {
        "arm"
    } else if cfg!(target_arch = "mips") {
        "mips"
    } else if cfg!(target_arch = "mips64") {
        "mips64"
    } else if cfg!(target_arch = "mips32r6") {
        "mips32r6"
    } else if cfg!(target_arch = "mips64r6") {
        "mips64r6"
    } else if cfg!(target_arch = "riscv64") {
        "riscv64"
    } else if cfg!(target_arch = "riscv32") {
        "riscv32"
    } else {
        "unknown"
    }
}

const fn current_os() -> &'static str {
    if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "android") {
        "android"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "freebsd") {
        "freebsd"
    } else if cfg!(target_os = "none") {
        "none"
    } else {
        "unknown"
    }
}

const fn current_env() -> &'static str {
    if cfg!(target_env = "gnu") {
        "gnu"
    } else if cfg!(target_env = "musl") {
        "musl"
    } else if cfg!(target_env = "msvc") {
        "msvc"
    } else if cfg!(target_env = "sgx") {
        "sgx"
    } else {
        "unknown"
    }
}

const fn current_vendor() -> &'static str {
    if cfg!(target_vendor = "apple") {
        "apple"
    } else if cfg!(target_vendor = "pc") {
        "pc"
    } else if cfg!(target_vendor = "unknown") {
        "unknown"
    } else if cfg!(target_vendor = "fortanix") {
        "fortanix"
    } else {
        "other"
    }
}

const fn current_endian() -> &'static str {
    if cfg!(target_endian = "little") {
        "little"
    } else {
        "big"
    }
}

const fn current_pointer_width() -> &'static str {
    if cfg!(target_pointer_width = "64") {
        "64"
    } else if cfg!(target_pointer_width = "32") {
        "32"
    } else if cfg!(target_pointer_width = "16") {
        "16"
    } else {
        "unknown"
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapabilityReport {
    pub target: TargetInfo,
    pub platform: PlatformCapabilities,
    pub cpu: CpuCapabilities,
    pub domains: DeviceDomainStatuses,
    pub diagnostics: DeviceDomainDiagnostics,
    pub metrics: QuantitativeResources,
    pub metadata: ReportMetadata,
    pub host: HostEnvironmentCapabilities,
}

impl CapabilityReport {
    pub const fn with_host(self, host: HostEnvironmentCapabilities) -> Self {
        Self { host, ..self }
    }

    pub const fn with_domains(self, domains: DeviceDomainStatuses) -> Self {
        Self { domains, ..self }
    }

    pub const fn with_diagnostics(self, diagnostics: DeviceDomainDiagnostics) -> Self {
        Self {
            diagnostics,
            ..self
        }
    }

    pub const fn with_metrics(self, metrics: QuantitativeResources) -> Self {
        Self { metrics, ..self }
    }

    pub const fn with_metadata(self, metadata: ReportMetadata) -> Self {
        Self { metadata, ..self }
    }

    pub fn evaluate_for_job(
        &self,
        requirements: JobRequirements,
        now_unix_s: Option<u64>,
    ) -> PlacementEvaluation {
        let mut blockers = 0u32;

        if let Some(max_age) = requirements.max_report_age_s {
            match (self.metadata.collected_unix_s, now_unix_s) {
                (Some(collected), Some(now)) if now.saturating_sub(collected) > max_age => {
                    blockers |= BLOCKER_STALE_REPORT
                }
                (None, _) | (_, None) => blockers |= BLOCKER_STALE_REPORT,
                _ => {}
            }
        }

        if !is_supported(self.domains.cpu.available) {
            blockers |= BLOCKER_CPU_UNAVAILABLE;
        }

        if let Some(min_cpu) = requirements.min_cpu_cores {
            match self
                .metrics
                .cpu_logical_cores_allocatable
                .or(self.metrics.cpu_logical_cores_total)
            {
                Some(alloc) if alloc >= min_cpu => {}
                Some(_) => blockers |= BLOCKER_CPU_UNAVAILABLE,
                None => blockers |= BLOCKER_MISSING_CPU_METRICS,
            }
        }

        if let Some(min_ram) = requirements.min_ram_bytes {
            if !is_supported(self.domains.ram.available) {
                blockers |= BLOCKER_RAM_UNAVAILABLE;
            }
            match self
                .metrics
                .ram_bytes_available
                .or(self.metrics.ram_bytes_total)
            {
                Some(bytes) if bytes >= min_ram => {}
                Some(_) => blockers |= BLOCKER_RAM_UNAVAILABLE,
                None => blockers |= BLOCKER_MISSING_RAM_METRICS,
            }
        }

        if let Some(min_storage) = requirements.min_storage_bytes {
            if !is_supported(self.domains.storage.available) {
                blockers |= BLOCKER_STORAGE_UNAVAILABLE;
            }
            match self
                .metrics
                .storage_bytes_available
                .or(self.metrics.storage_bytes_total)
            {
                Some(bytes) if bytes >= min_storage => {}
                Some(_) => blockers |= BLOCKER_STORAGE_UNAVAILABLE,
                None => blockers |= BLOCKER_MISSING_STORAGE_METRICS,
            }
        }

        if let Some(min_gpu) = requirements.min_gpu_count {
            if !is_supported(self.domains.gpu.available) {
                blockers |= BLOCKER_GPU_UNAVAILABLE;
            }
            match self
                .metrics
                .gpu_count_available
                .or(self.metrics.gpu_count_total)
            {
                Some(count) if count >= min_gpu => {}
                Some(_) => blockers |= BLOCKER_GPU_UNAVAILABLE,
                None => blockers |= BLOCKER_MISSING_GPU_METRICS,
            }
        }

        if requirements.require_network && !is_supported(self.domains.network.available) {
            blockers |= BLOCKER_NETWORK_UNAVAILABLE;
        }
        if requirements.require_usb && !is_supported(self.domains.usb.available) {
            blockers |= BLOCKER_USB_UNAVAILABLE;
        }
        if requirements.require_input && !is_supported(self.domains.input.available) {
            blockers |= BLOCKER_INPUT_UNAVAILABLE;
        }
        if requirements.require_output && !is_supported(self.domains.output.available) {
            blockers |= BLOCKER_OUTPUT_UNAVAILABLE;
        }

        PlacementEvaluation {
            eligible: blockers == 0,
            blockers,
        }
    }
}

pub fn probe_capabilities() -> CapabilityReport {
    CapabilityReport {
        target: TargetInfo::current(),
        platform: PlatformCapabilities::current(),
        cpu: arch::probe_cpu_capabilities(),
        domains: DeviceDomainStatuses::no_std_baseline(),
        diagnostics: DeviceDomainDiagnostics::unknown(),
        metrics: QuantitativeResources::unknown(),
        metadata: ReportMetadata::unknown(),
        host: HostEnvironmentCapabilities::no_std_unknowns(),
    }
}

const fn bool_signal(value: bool, source: ProbeSource) -> CapabilitySignal {
    if value {
        CapabilitySignal::supported(source)
    } else {
        CapabilitySignal::unsupported(source)
    }
}

const fn is_supported(signal: CapabilitySignal) -> bool {
    matches!(signal.value, CapabilityValue::Supported)
}

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_has_target_metadata() {
        let report = probe_capabilities();
        assert!(!report.target.arch.is_empty());
        assert!(!report.target.os.is_empty());
        assert!(!report.target.endian.is_empty());
        assert!(!report.target.pointer_width.is_empty());
        assert_eq!(report.platform.atomic_ptr.source, ProbeSource::CompileTime);
        assert_eq!(
            report.domains.cpu.detected.value,
            CapabilityValue::Supported
        );
        assert_eq!(
            report.diagnostics.cpu.detected.error,
            ProbeErrorCode::Unknown
        );
        assert_eq!(report.metrics.cpu_logical_cores_total, None);
    }

    #[test]
    fn no_std_host_fields_default_to_unknown() {
        let host = HostEnvironmentCapabilities::no_std_unknowns();
        assert_eq!(host.container_detection.value, CapabilityValue::Unknown);
        assert_eq!(host.memory_limits.value, CapabilityValue::Unknown);
        assert_eq!(host.filesystem_inventory.value, CapabilityValue::Unknown);
        assert_eq!(host.network_inventory.value, CapabilityValue::Unknown);
        assert_eq!(host.gpu_inventory.value, CapabilityValue::Unknown);
        assert_eq!(host.bluetooth_inventory.value, CapabilityValue::Unknown);
        assert_eq!(host.nfc_inventory.value, CapabilityValue::Unknown);
        assert_eq!(host.policy_is_root.value, CapabilityValue::Unknown);
        assert_eq!(host.policy_cap_sys_admin.value, CapabilityValue::Unknown);
        assert_eq!(host.policy_cap_net_admin.value, CapabilityValue::Unknown);
        assert_eq!(host.policy_cap_sys_rawio.value, CapabilityValue::Unknown);
    }

    #[test]
    fn report_allows_host_merge() {
        let merged = probe_capabilities().with_host(HostEnvironmentCapabilities {
            container_detection: CapabilitySignal::supported(ProbeSource::Runtime),
            memory_limits: CapabilitySignal::unknown(),
            filesystem_inventory: CapabilitySignal::unknown(),
            network_inventory: CapabilitySignal::unknown(),
            gpu_inventory: CapabilitySignal::unknown(),
            bluetooth_inventory: CapabilitySignal::unknown(),
            nfc_inventory: CapabilitySignal::unknown(),
            policy_is_root: CapabilitySignal::unsupported(ProbeSource::Runtime),
            policy_cap_sys_admin: CapabilitySignal::unknown(),
            policy_cap_net_admin: CapabilitySignal::unknown(),
            policy_cap_sys_rawio: CapabilitySignal::unknown(),
        });
        assert_eq!(
            merged.host.container_detection.value,
            CapabilityValue::Supported
        );
    }

    #[test]
    fn report_allows_domain_merge() {
        let merged = probe_capabilities().with_domains(DeviceDomainStatuses {
            cpu: DeviceDomainStatus {
                detected: CapabilitySignal::supported(ProbeSource::Runtime),
                available: CapabilitySignal::supported(ProbeSource::Runtime),
                in_use: CapabilitySignal::supported(ProbeSource::Runtime),
            },
            storage: DeviceDomainStatus {
                detected: CapabilitySignal::supported(ProbeSource::Runtime),
                available: CapabilitySignal::supported(ProbeSource::Runtime),
                in_use: CapabilitySignal::unsupported(ProbeSource::Runtime),
            },
            gpu: DeviceDomainStatus::unknown(),
            ram: DeviceDomainStatus {
                detected: CapabilitySignal::supported(ProbeSource::Runtime),
                available: CapabilitySignal::supported(ProbeSource::Runtime),
                in_use: CapabilitySignal::supported(ProbeSource::Runtime),
            },
            usb: DeviceDomainStatus {
                detected: CapabilitySignal::unsupported(ProbeSource::Runtime),
                available: CapabilitySignal::unsupported(ProbeSource::Runtime),
                in_use: CapabilitySignal::unsupported(ProbeSource::Runtime),
            },
            network: DeviceDomainStatus {
                detected: CapabilitySignal::supported(ProbeSource::Runtime),
                available: CapabilitySignal::supported(ProbeSource::Runtime),
                in_use: CapabilitySignal::supported(ProbeSource::Runtime),
            },
            input: DeviceDomainStatus::unknown(),
            output: DeviceDomainStatus::unknown(),
        });
        assert_eq!(
            merged.domains.storage.available.value,
            CapabilityValue::Supported
        );
    }

    #[test]
    fn evaluator_blocks_stale_or_missing_metrics() {
        let report = probe_capabilities();
        let req = JobRequirements {
            min_cpu_cores: Some(1),
            min_ram_bytes: Some(1024),
            min_storage_bytes: None,
            min_gpu_count: None,
            require_network: true,
            require_usb: false,
            require_input: false,
            require_output: false,
            max_report_age_s: Some(30),
        };
        let eval = report.evaluate_for_job(req, Some(100));
        assert!(!eval.eligible);
        assert!(eval.blocked_by(BLOCKER_STALE_REPORT));
        assert!(eval.blocked_by(BLOCKER_MISSING_CPU_METRICS));
        assert!(eval.blocked_by(BLOCKER_MISSING_RAM_METRICS));
        assert!(eval.blocked_by(BLOCKER_NETWORK_UNAVAILABLE));
    }
}
