//! OCI spec types with edgerun-json serialization.

use crate::prelude::*;
use crate::util::StringResultExt;
use edgerun_json::ToJson;

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

// ===========================================================================
// Public API
// ===========================================================================

/// Parse an OCI spec from JSON bytes.
pub fn parse_oci_spec(data: &[u8]) -> Result<OciSpec, String> {
    edgerun_json::from_json_slice(data).string_err()
}

pub fn parse_oci_process(data: &[u8]) -> Result<OciProcess, String> {
    edgerun_json::from_json_slice(data).string_err()
}

// ===========================================================================
// OCI Spec types
// ===========================================================================

#[derive(Debug, Clone)]
pub struct OciSpec {
    pub version: String,
    pub platform: Option<OciPlatform>,
    pub process: Option<OciProcess>,
    pub root: Option<OciRoot>,
    pub hostname: Option<String>,
    /// NIS domain name for the container (OCI 1.1.0).
    /// Set via setdomainname(2) syscall.
    pub domainname: Option<String>,
    pub linux: Option<OciLinux>,
    pub mounts: Option<Vec<OciMount>>,
    pub annotations: Option<BTreeMap<String, String>>,
}

/// Target platform the bundle was built for.
/// Per OCI spec: runtime must reject bundles whose platform doesn't match the host.
#[derive(Debug, Clone, Default)]
pub struct OciPlatform {
    /// Operating system: "linux", "windows", "solaris", etc.
    pub os: Option<String>,
    /// CPU architecture: "amd64", "arm64", "riscv64", etc.
    pub arch: Option<String>,
    /// OS variant (optional): e.g., "v1", "v2" for Windows.
    pub os_version: Option<String>,
    /// OS features (optional).
    pub os_features: Option<Vec<String>>,
}

impl OciPlatform {
    /// Check if this platform matches the current host.
    pub fn matches_host(&self) -> bool {
        // OS check
        if let Some(ref os) = self.os {
            if os != crate::validate::host_os() {
                return false;
            }
        }
        // Arch check
        if let Some(ref arch) = self.arch {
            if arch != crate::validate::host_arch() {
                return false;
            }
        }
        true
    }
}

impl OciSpec {
    /// Serialize to compact JSON.
    pub fn to_json_string(&self) -> String {
        edgerun_json::to_json_string(self).unwrap_or_default()
    }

    /// Serialize to pretty-printed JSON.
    pub fn to_json_string_pretty(&self) -> String {
        edgerun_json::to_string_pretty(&self.to_json()).unwrap_or_default()
    }
}

/// Terminal console dimensions (cells, not pixels).
#[derive(Debug, Clone, Default)]
pub struct OciBox {
    pub width: u64,
    pub height: u64,
}

#[derive(Debug, Clone, Default)]
pub struct OciProcess {
    pub terminal: Option<bool>,
    pub user: Option<OciUser>,
    /// Console size for terminal (width and height in cells).
    pub console_size: Option<OciBox>,
    pub args: Option<Vec<String>>,
    pub env: Option<Vec<String>>,
    pub cwd: Option<String>,
    pub capabilities: Option<OciCapabilities>,
    pub rlimits: Option<Vec<OciRlimit>>,
    pub no_new_privileges: Option<bool>,
    pub oom_score_adj: Option<i64>,
    pub apparmor_profile: Option<String>,
    pub selinux_label: Option<String>,
    /// Real-time scheduling policy and parameters (OCI 1.0.2).
    pub scheduler: Option<OciScheduler>,
    /// I/O priority for the container process (OCI 1.1.0).
    /// Uses the Linux ioprio_set() interface: class 0-3, priority 0-7.
    pub io_priority: Option<OciIoPriority>,
}

/// I/O priority configuration (OCI 1.1.0).
/// Mirrors the Linux ioprio_set(2) interface.
#[derive(Debug, Clone, Default)]
pub struct OciIoPriority {
    /// I/O scheduling class: 0=none, 1=realtime, 2=best-effort, 3=idle.
    pub class: u32,
    /// I/O priority level within the class (0-7, lower = higher priority).
    /// Ignored for class 0 (none) and class 3 (idle).
    pub priority: Option<u32>,
}

/// Real-time scheduling configuration.
#[derive(Debug, Clone, Default)]
pub struct OciScheduler {
    /// Scheduling policy: "SCHED_OTHER", "SCHED_FIFO", "SCHED_RR", "SCHED_BATCH", "SCHED_IDLE", "SCHED_DEADLINE"
    pub policy: String,
    /// Nice value (only for SCHED_OTHER and SCHED_BATCH).
    pub nice: Option<i32>,
    /// Scheduling priority (for SCHED_FIFO and SCHED_RR, range 1-99).
    pub priority: Option<i32>,
    /// SCHED_DEADLINE parameters.
    pub deadline: Option<OciSchedDeadline>,
}

/// SCHED_DEADLINE scheduling parameters.
#[derive(Debug, Clone, Default)]
pub struct OciSchedDeadline {
    /// Runtime in nanoseconds.
    pub runtime_ns: Option<u64>,
    /// Period in nanoseconds.
    pub period_ns: Option<u64>,
    /// Deadline in nanoseconds.
    pub deadline_ns: Option<u64>,
}

#[derive(Debug, Clone, Default)]
pub struct OciUser {
    pub uid: Option<u32>,
    pub gid: Option<u32>,
    pub additional_gids: Option<Vec<u32>>,
    pub umask: Option<u32>,
}

#[derive(Debug, Clone, Default)]
pub struct OciCapabilities {
    pub bounding: Option<Vec<String>>,
    pub effective: Option<Vec<String>>,
    pub inheritable: Option<Vec<String>>,
    pub permitted: Option<Vec<String>>,
    pub ambient: Option<Vec<String>>,
}

#[derive(Debug, Clone, Default)]
pub struct OciRlimit {
    pub ns_type: String,
    pub hard: u64,
    pub soft: u64,
}

#[derive(Debug, Clone, Default)]
pub struct OciRoot {
    pub path: String,
    pub readonly: Option<bool>,
}

#[derive(Debug, Clone, Default)]
pub struct OciLinux {
    pub uid_mappings: Option<Vec<OciIdMapping>>,
    pub gid_mappings: Option<Vec<OciIdMapping>>,
    pub resources: Option<OciLinuxResources>,
    pub cgroups_path: Option<String>,
    pub namespaces: Option<Vec<OciNamespace>>,
    pub devices: Option<Vec<OciLinuxDevice>>,
    pub masked_paths: Option<Vec<String>>,
    pub readonly_paths: Option<Vec<String>>,
    pub mount_label: Option<String>,
    pub rootfs_propagation: Option<String>,
    pub sysctl: Option<BTreeMap<String, String>>,
    pub hooks: Option<OciHooks>,
    pub seccomp: Option<OciLinuxSeccomp>,
    /// Intel RDT (Resource Director Technology) configuration.
    pub intel_rdt: Option<OciLinuxIntelRdt>,
}

// ===========================================================================
// OCI Hooks types
// ===========================================================================

#[derive(Debug, Clone, Default)]
pub struct OciHooks {
    pub prestart: Option<Vec<OciHook>>,
    pub create_runtime: Option<Vec<OciHook>>,
    pub create_container: Option<Vec<OciHook>>,
    pub start_container: Option<Vec<OciHook>>,
    pub poststart: Option<Vec<OciHook>>,
    pub poststop: Option<Vec<OciHook>>,
}

#[derive(Debug, Clone, Default)]
pub struct OciHook {
    pub path: String,
    pub args: Option<Vec<String>>,
    pub env: Option<Vec<String>>,
    pub timeout: Option<u64>,
}

#[derive(Debug, Clone, Default)]
pub struct OciIdMapping {
    pub container_id: u32,
    pub host_id: u32,
    pub size: u32,
}

#[derive(Debug, Clone, Default)]
pub struct OciNamespace {
    pub ns_type: String,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct OciLinuxDevice {
    pub ns_type: String,
    pub path: String,
    pub file_mode: Option<u32>,
    pub uid: Option<u32>,
    pub gid: Option<u32>,
    pub major: Option<i64>,
    pub minor: Option<i64>,
}

#[derive(Debug, Clone, Default)]
pub struct OciLinuxCpu {
    pub shares: Option<u64>,
    pub quota: Option<i64>,
    pub period: Option<u64>,
    pub realtime_runtime: Option<i64>,
    pub realtime_period: Option<u64>,
    pub cpus: Option<String>,
    pub mems: Option<String>,
    /// CPU idle cgroup control (OCI 1.1.0).
    /// 0 = not idle, 1 = idle. When idle, CPU bandwidth is deprioritized.
    pub idle: Option<i64>,
    /// CFS bandwidth burst size in nanoseconds (OCI 1.1.0).
    /// Allows temporary CPU quota overrun for latency-sensitive workloads.
    pub burst: Option<i64>,
}

// ===========================================================================
// Block I/O cgroup types
// ===========================================================================

#[derive(Debug, Clone, Default)]
pub struct OciLinuxBlockIO {
    pub weight: Option<u16>,
    pub leaf_weight: Option<u16>,
    pub weight_device: Option<Vec<OciLinuxWeightDevice>>,
    pub leaf_weight_device: Option<Vec<OciLinuxWeightDevice>>,
    pub throttle_read_bps_device: Option<Vec<OciLinuxThrottleDevice>>,
    pub throttle_write_bps_device: Option<Vec<OciLinuxThrottleDevice>>,
    pub throttle_read_iops_device: Option<Vec<OciLinuxThrottleDevice>>,
    pub throttle_write_iops_device: Option<Vec<OciLinuxThrottleDevice>>,
}

#[derive(Debug, Clone, Default)]
pub struct OciLinuxWeightDevice {
    pub major: i64,
    pub minor: i64,
    pub weight: Option<u16>,
    pub leaf_weight: Option<u16>,
}

#[derive(Debug, Clone, Default)]
pub struct OciLinuxThrottleDevice {
    pub major: i64,
    pub minor: i64,
    pub rate: u64,
}

// ===========================================================================
// Hugepage and network cgroup types
// ===========================================================================

#[derive(Debug, Clone, Default)]
pub struct OciLinuxHugepageLimit {
    pub pagesize: String,
    pub limit: u64,
    /// Reserved huge page accounting (OCI 1.1.0).
    /// When true, apply limit to hugetlb.<size>.rsvd.max instead of hugetlb.<size>.max.
    pub rsvd: Option<bool>,
}

#[derive(Debug, Clone, Default)]
pub struct OciLinuxNetwork {
    pub class_id: Option<u32>,
    pub priorities: Option<Vec<OciLinuxNetworkPriority>>,
}

#[derive(Debug, Clone, Default)]
pub struct OciLinuxNetworkPriority {
    pub name: String,
    pub priority: u32,
}

/// Intel RDT (Resource Director Technology) configuration.
/// Controls cache and memory bandwidth allocation.
#[derive(Debug, Clone, Default)]
pub struct OciLinuxIntelRdt {
    /// Cache Bitmask (CBM) for L3 cache. E.g., "L3:0=fff"
    pub l3_cache_schema: Option<String>,
    /// Memory bandwidth schema. E.g., "MB:0=70"
    pub mem_bw_schema: Option<String>,
    /// Class of Service ID
    pub clos_id: Option<String>,
    /// Enable CMT/MBM monitoring for this container (OCI 1.3.0).
    /// When true, the runtime should enable cache and memory bandwidth monitoring.
    pub enable_monitoring: Option<bool>,
    /// Combined schemata format (OCI 1.3.0).
    /// E.g., "L3:0=fff\nMB:0=70" — overrides l3_cache_schema and mem_bw_schema if set.
    pub schemata: Option<String>,
}

// ===========================================================================
// Seccomp types
// ===========================================================================

/// Seccomp filtering configuration from the OCI spec.
#[derive(Debug, Clone, Default)]
pub struct OciLinuxSeccomp {
    pub default_action: Option<OciSeccompAction>,
    pub default_errno_ret: Option<u32>,
    pub architectures: Option<Vec<String>>,
    pub listener_path: Option<String>,
    pub listener_metadata: Option<String>,
    pub syscalls: Option<Vec<OciSeccompSyscallEntry>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OciSeccompAction {
    Kill,
    KillProcess,
    KillThread,
    Trap,
    Errno,
    Trace,
    Allow,
    Notify,
    Log,
}

impl TryFrom<String> for OciSeccompAction {
    type Error = String;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        let s = s.strip_prefix("SCMP_ACT_").unwrap_or(&s);
        match s {
            "KILL" => Ok(OciSeccompAction::Kill),
            "KILL_PROCESS" => Ok(OciSeccompAction::KillProcess),
            "KILL_THREAD" => Ok(OciSeccompAction::KillThread),
            "TRAP" => Ok(OciSeccompAction::Trap),
            "ERRNO" => Ok(OciSeccompAction::Errno),
            "TRACE" => Ok(OciSeccompAction::Trace),
            "ALLOW" => Ok(OciSeccompAction::Allow),
            "NOTIFY" => Ok(OciSeccompAction::Notify),
            "LOG" => Ok(OciSeccompAction::Log),
            _ => Err(format!("unknown seccomp action: {}", s)),
        }
    }
}

impl From<OciSeccompAction> for String {
    fn from(a: OciSeccompAction) -> Self {
        match a {
            OciSeccompAction::Kill => "SCMP_ACT_KILL".into(),
            OciSeccompAction::KillProcess => "SCMP_ACT_KILL_PROCESS".into(),
            OciSeccompAction::KillThread => "SCMP_ACT_KILL_THREAD".into(),
            OciSeccompAction::Trap => "SCMP_ACT_TRAP".into(),
            OciSeccompAction::Errno => "SCMP_ACT_ERRNO".into(),
            OciSeccompAction::Trace => "SCMP_ACT_TRACE".into(),
            OciSeccompAction::Allow => "SCMP_ACT_ALLOW".into(),
            OciSeccompAction::Notify => "SCMP_ACT_NOTIFY".into(),
            OciSeccompAction::Log => "SCMP_ACT_LOG".into(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct OciSeccompSyscallEntry {
    pub names: Option<Vec<String>>,
    pub action: Option<OciSeccompAction>,
    pub errno_ret: Option<u32>,
    pub args: Option<Vec<OciSeccompArg>>,
}

#[derive(Debug, Clone, Default)]
pub struct OciSeccompArg {
    pub index: u32,
    pub value: u64,
    pub value_two: u64,
    pub op: String,
}

// ===========================================================================
// Update OciLinux to include seccomp
// ===========================================================================

#[derive(Debug, Clone, Default)]
pub struct OciLinuxResources {
    pub devices: Option<Vec<OciLinuxDeviceCgroup>>,
    pub memory: Option<OciLinuxMemory>,
    pub cpu: Option<OciLinuxCpu>,
    pub pids: Option<OciLinuxPids>,
    pub block_io: Option<OciLinuxBlockIO>,
    pub hugepage_limits: Option<Vec<OciLinuxHugepageLimit>>,
    pub network: Option<OciLinuxNetwork>,
}

/// Device cgroup rule for allowed/denied device access.
#[derive(Debug, Clone, Default)]
pub struct OciLinuxDeviceCgroup {
    pub ns_type: String,
    pub major: Option<i64>,
    pub minor: Option<i64>,
    pub access: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct OciLinuxPids {
    pub limit: i64,
}

#[derive(Debug, Clone, Default)]
pub struct OciLinuxMemory {
    pub limit: Option<i64>,
    pub reservation: Option<i64>,
    pub swap: Option<i64>,
    pub kernel: Option<i64>,
    pub kernel_tcp: Option<i64>,
    /// Hint to runtime to validate memory limit before updating (OCI 1.1.0).
    /// When true, the runtime should check if the new limit is feasible
    /// before applying it, rather than failing after the fact.
    pub check_before_update: Option<bool>,
}

#[derive(Debug, Clone, Default)]
pub struct OciMount {
    pub destination: String,
    pub mount_type: Option<String>,
    pub source: Option<String>,
    pub options: Option<Vec<String>>,
    pub label: Option<String>,
    /// Recursive mount attribute (OCI 1.1).
    /// When true, mount options apply recursively to sub-mounts.
    pub recursive: Option<bool>,
    /// UID mappings for idmapped mounts (OCI 1.1/1.2, Linux 5.12+).
    pub uid_mappings: Option<Vec<OciIdMapping>>,
    /// GID mappings for idmapped mounts (OCI 1.1/1.2, Linux 5.12+).
    pub gid_mappings: Option<Vec<OciIdMapping>>,
}

#[cfg(all(test, not(target_os = "none")))]
#[path = "../tests/unit_src/src/spec_edgerun_json_tests.rs"]
mod edgerun_json_tests;
