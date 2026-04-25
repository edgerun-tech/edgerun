//! OCI spec types with serde-based JSON serialization via edgerun-json.

use edgerun_json::{from_slice, to_string, to_string_pretty};
use serde::{Deserialize, Serialize};

// ===========================================================================
// Public API
// ===========================================================================

/// Parse an OCI spec from JSON bytes.
pub fn parse_oci_spec(data: &[u8]) -> Result<OciSpec, String> {
    from_slice(data).map_err(|e| e.to_string())
}

// ===========================================================================
// OCI Spec types
// ===========================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OciSpec {
    #[serde(rename = "ociVersion")]
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<OciPlatform>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process: Option<OciProcess>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root: Option<OciRoot>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,
    /// NIS domain name for the container (OCI 1.1.0).
    /// Set via setdomainname(2) syscall.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domainname: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linux: Option<OciLinux>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mounts: Option<Vec<OciMount>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<std::collections::HashMap<String, String>>,
}

/// Target platform the bundle was built for.
/// Per OCI spec: runtime must reject bundles whose platform doesn't match the host.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct OciPlatform {
    /// Operating system: "linux", "windows", "solaris", etc.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os: Option<String>,
    /// CPU architecture: "amd64", "arm64", "riscv64", etc.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arch: Option<String>,
    /// OS variant (optional): e.g., "v1", "v2" for Windows.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os_version: Option<String>,
    /// OS features (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os_features: Option<Vec<String>>,
}

impl OciPlatform {
    /// Check if this platform matches the current host.
    pub fn matches_host(&self) -> bool {
        // OS check
        if let Some(ref os) = self.os {
            let host_os = if cfg!(target_os = "linux") {
                "linux"
            } else if cfg!(target_os = "windows") {
                "windows"
            } else if cfg!(target_os = "solaris") {
                "solaris"
            } else {
                "unknown"
            };
            if os != host_os {
                return false;
            }
        }
        // Arch check
        if let Some(ref arch) = self.arch {
            let host_arch = if cfg!(target_arch = "x86_64") {
                "amd64"
            } else if cfg!(target_arch = "aarch64") {
                "arm64"
            } else if cfg!(target_arch = "riscv64") {
                "riscv64"
            } else if cfg!(target_arch = "arm") {
                "arm"
            } else {
                "unknown"
            };
            if arch != host_arch {
                return false;
            }
        }
        true
    }
}

impl OciSpec {
    /// Serialize to compact JSON.
    pub fn to_json_string(&self) -> String {
        to_string(self).unwrap_or_default()
    }

    /// Serialize to pretty-printed JSON.
    pub fn to_json_string_pretty(&self) -> String {
        to_string_pretty(self).unwrap_or_default()
    }
}

/// Terminal console dimensions (cells, not pixels).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct OciBox {
    pub width: u64,
    pub height: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct OciProcess {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<OciUser>,
    /// Console size for terminal (width and height in cells).
    #[serde(skip_serializing_if = "Option::is_none", rename = "consoleSize")]
    pub console_size: Option<OciBox>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "cwd")]
    pub cwd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<OciCapabilities>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "rlimits")]
    pub rlimits: Option<Vec<OciRlimit>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "noNewPrivileges")]
    pub no_new_privileges: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "oomScoreAdj")]
    pub oom_score_adj: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "apparmorProfile")]
    pub apparmor_profile: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "selinuxLabel")]
    pub selinux_label: Option<String>,
    /// Real-time scheduling policy and parameters (OCI 1.0.2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduler: Option<OciScheduler>,
    /// I/O priority for the container process (OCI 1.1.0).
    /// Uses the Linux ioprio_set() interface: class 0-3, priority 0-7.
    #[serde(skip_serializing_if = "Option::is_none", rename = "ioPriority")]
    pub io_priority: Option<OciIoPriority>,
}

/// I/O priority configuration (OCI 1.1.0).
/// Mirrors the Linux ioprio_set(2) interface.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct OciIoPriority {
    /// I/O scheduling class: 0=none, 1=realtime, 2=best-effort, 3=idle.
    #[serde(rename = "class")]
    pub class: u32,
    /// I/O priority level within the class (0-7, lower = higher priority).
    /// Ignored for class 0 (none) and class 3 (idle).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<u32>,
}

/// Real-time scheduling configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct OciScheduler {
    /// Scheduling policy: "SCHED_OTHER", "SCHED_FIFO", "SCHED_RR", "SCHED_BATCH", "SCHED_IDLE", "SCHED_DEADLINE"
    pub policy: String,
    /// Nice value (only for SCHED_OTHER and SCHED_BATCH).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nice: Option<i32>,
    /// Scheduling priority (for SCHED_FIFO and SCHED_RR, range 1-99).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
    /// SCHED_DEADLINE parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deadline: Option<OciSchedDeadline>,
}

/// SCHED_DEADLINE scheduling parameters.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct OciSchedDeadline {
    /// Runtime in nanoseconds.
    #[serde(skip_serializing_if = "Option::is_none", rename = "runtime")]
    pub runtime_ns: Option<u64>,
    /// Period in nanoseconds.
    #[serde(skip_serializing_if = "Option::is_none", rename = "period")]
    pub period_ns: Option<u64>,
    /// Deadline in nanoseconds.
    #[serde(skip_serializing_if = "Option::is_none", rename = "deadline")]
    pub deadline_ns: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct OciUser {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uid: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gid: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "additionalGids")]
    pub additional_gids: Option<Vec<u32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub umask: Option<u32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct OciCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bounding: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effective: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inheritable: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permitted: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ambient: Option<Vec<String>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OciRlimit {
    #[serde(rename = "type")]
    pub ns_type: String,
    pub hard: u64,
    pub soft: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct OciRoot {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub readonly: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct OciLinux {
    #[serde(skip_serializing_if = "Option::is_none", rename = "uidMappings")]
    pub uid_mappings: Option<Vec<OciIdMapping>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "gidMappings")]
    pub gid_mappings: Option<Vec<OciIdMapping>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<OciLinuxResources>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "cgroupsPath")]
    pub cgroups_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespaces: Option<Vec<OciNamespace>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub devices: Option<Vec<OciLinuxDevice>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "maskedPaths")]
    pub masked_paths: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "readonlyPaths")]
    pub readonly_paths: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "mountLabel")]
    pub mount_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "rootfsPropagation")]
    pub rootfs_propagation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sysctl: Option<std::collections::HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hooks: Option<OciHooks>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seccomp: Option<OciLinuxSeccomp>,
    /// Intel RDT (Resource Director Technology) configuration.
    #[serde(skip_serializing_if = "Option::is_none", rename = "intelRdt")]
    pub intel_rdt: Option<OciLinuxIntelRdt>,
}

// ===========================================================================
// OCI Hooks types
// ===========================================================================

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct OciHooks {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prestart: Option<Vec<OciHook>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "createRuntime")]
    pub create_runtime: Option<Vec<OciHook>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "createContainer")]
    pub create_container: Option<Vec<OciHook>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "startContainer")]
    pub start_container: Option<Vec<OciHook>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "poststart")]
    pub poststart: Option<Vec<OciHook>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "poststop")]
    pub poststop: Option<Vec<OciHook>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct OciHook {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OciIdMapping {
    #[serde(rename = "containerID")]
    pub container_id: u32,
    #[serde(rename = "hostID")]
    pub host_id: u32,
    pub size: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct OciNamespace {
    #[serde(rename = "type")]
    pub ns_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct OciLinuxDevice {
    #[serde(rename = "type")]
    pub ns_type: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none", rename = "fileMode")]
    pub file_mode: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uid: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gid: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub major: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minor: Option<i64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct OciLinuxCpu {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shares: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quota: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub period: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "realtimeRuntime")]
    pub realtime_runtime: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "realtimePeriod")]
    pub realtime_period: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpus: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mems: Option<String>,
    /// CPU idle cgroup control (OCI 1.1.0).
    /// 0 = not idle, 1 = idle. When idle, CPU bandwidth is deprioritized.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle: Option<i64>,
    /// CFS bandwidth burst size in nanoseconds (OCI 1.1.0).
    /// Allows temporary CPU quota overrun for latency-sensitive workloads.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub burst: Option<i64>,
}

// ===========================================================================
// Block I/O cgroup types
// ===========================================================================

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct OciLinuxBlockIO {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "leafWeight")]
    pub leaf_weight: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "weightDevice")]
    pub weight_device: Option<Vec<OciLinuxWeightDevice>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "leafWeightDevice")]
    pub leaf_weight_device: Option<Vec<OciLinuxWeightDevice>>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        rename = "throttleReadBpsDevice"
    )]
    pub throttle_read_bps_device: Option<Vec<OciLinuxThrottleDevice>>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        rename = "throttleWriteBpsDevice"
    )]
    pub throttle_write_bps_device: Option<Vec<OciLinuxThrottleDevice>>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        rename = "throttleReadIOPSDevice"
    )]
    pub throttle_read_iops_device: Option<Vec<OciLinuxThrottleDevice>>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        rename = "throttleWriteIOPSDevice"
    )]
    pub throttle_write_iops_device: Option<Vec<OciLinuxThrottleDevice>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct OciLinuxWeightDevice {
    pub major: i64,
    pub minor: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "leafWeight")]
    pub leaf_weight: Option<u16>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct OciLinuxThrottleDevice {
    pub major: i64,
    pub minor: i64,
    pub rate: u64,
}

// ===========================================================================
// Hugepage and network cgroup types
// ===========================================================================

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OciLinuxHugepageLimit {
    pub pagesize: String,
    pub limit: u64,
    /// Reserved huge page accounting (OCI 1.1.0).
    /// When true, apply limit to hugetlb.<size>.rsvd.max instead of hugetlb.<size>.max.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rsvd: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct OciLinuxNetwork {
    #[serde(skip_serializing_if = "Option::is_none", rename = "classID")]
    pub class_id: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priorities: Option<Vec<OciLinuxNetworkPriority>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct OciLinuxNetworkPriority {
    pub name: String,
    pub priority: u32,
}

/// Intel RDT (Resource Director Technology) configuration.
/// Controls cache and memory bandwidth allocation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct OciLinuxIntelRdt {
    /// Cache Bitmask (CBM) for L3 cache. E.g., "L3:0=fff"
    #[serde(skip_serializing_if = "Option::is_none", rename = "l3CacheSchema")]
    pub l3_cache_schema: Option<String>,
    /// Memory bandwidth schema. E.g., "MB:0=70"
    #[serde(skip_serializing_if = "Option::is_none", rename = "memBwSchema")]
    pub mem_bw_schema: Option<String>,
    /// Class of Service ID
    #[serde(skip_serializing_if = "Option::is_none", rename = "closID")]
    pub clos_id: Option<String>,
    /// Enable CMT/MBM monitoring for this container (OCI 1.3.0).
    /// When true, the runtime should enable cache and memory bandwidth monitoring.
    #[serde(skip_serializing_if = "Option::is_none", rename = "enableMonitoring")]
    pub enable_monitoring: Option<bool>,
    /// Combined schemata format (OCI 1.3.0).
    /// E.g., "L3:0=fff\nMB:0=70" — overrides l3_cache_schema and mem_bw_schema if set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schemata: Option<String>,
}

// ===========================================================================
// Seccomp types
// ===========================================================================

/// Seccomp filtering configuration from the OCI spec.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct OciLinuxSeccomp {
    #[serde(skip_serializing_if = "Option::is_none", rename = "defaultAction")]
    pub default_action: Option<OciSeccompAction>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "defaultErrnoRet")]
    pub default_errno_ret: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub architectures: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "listenerPath")]
    pub listener_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "listenerMetadata")]
    pub listener_metadata: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub syscalls: Option<Vec<OciSeccompSyscallEntry>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    rename_all = "SCREAMING_SNAKE_CASE",
    try_from = "String",
    into = "String"
)]
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct OciSeccompSyscallEntry {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub names: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<OciSeccompAction>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "errnoRet")]
    pub errno_ret: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<OciSeccompArg>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct OciSeccompArg {
    pub index: u32,
    pub value: u64,
    #[serde(rename = "valueTwo")]
    pub value_two: u64,
    pub op: String,
}

// ===========================================================================
// Update OciLinux to include seccomp
// ===========================================================================

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OciLinuxResources {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub devices: Option<Vec<OciLinuxDeviceCgroup>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory: Option<OciLinuxMemory>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu: Option<OciLinuxCpu>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pids: Option<OciLinuxPids>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "blockIO")]
    pub block_io: Option<OciLinuxBlockIO>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hugepage_limits: Option<Vec<OciLinuxHugepageLimit>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<OciLinuxNetwork>,
}

/// Device cgroup rule for allowed/denied device access.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct OciLinuxDeviceCgroup {
    #[serde(rename = "type")]
    pub ns_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub major: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minor: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OciLinuxPids {
    pub limit: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct OciLinuxMemory {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reservation: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub swap: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kernel: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "kernelTCP")]
    pub kernel_tcp: Option<i64>,
    /// Hint to runtime to validate memory limit before updating (OCI 1.1.0).
    /// When true, the runtime should check if the new limit is feasible
    /// before applying it, rather than failing after the fact.
    #[serde(skip_serializing_if = "Option::is_none", rename = "checkBeforeUpdate")]
    pub check_before_update: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct OciMount {
    pub destination: String,
    #[serde(skip_serializing_if = "Option::is_none", rename = "type")]
    pub mount_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "source")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "label")]
    pub label: Option<String>,
    /// Recursive mount attribute (OCI 1.1).
    /// When true, mount options apply recursively to sub-mounts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recursive: Option<bool>,
    /// UID mappings for idmapped mounts (OCI 1.1/1.2, Linux 5.12+).
    #[serde(skip_serializing_if = "Option::is_none", rename = "uidMappings")]
    pub uid_mappings: Option<Vec<OciIdMapping>>,
    /// GID mappings for idmapped mounts (OCI 1.1/1.2, Linux 5.12+).
    #[serde(skip_serializing_if = "Option::is_none", rename = "gidMappings")]
    pub gid_mappings: Option<Vec<OciIdMapping>>,
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use edgerun_json::from_slice;

    #[test]
    fn parse_minimal_oci_spec() {
        let json =
            r#"{"ociVersion":"1.0.2","root":{"path":"rootfs"},"process":{"args":["/bin/sh"]}}"#;
        let spec = parse_oci_spec(json.as_bytes()).unwrap();
        assert_eq!(spec.version, "1.0.2");
        assert_eq!(spec.root.unwrap().path, "rootfs");
        assert_eq!(spec.process.unwrap().args.unwrap(), vec!["/bin/sh"]);
    }

    #[test]
    fn parse_oci_spec_invalid_json() {
        assert!(parse_oci_spec(b"not json").is_err());
        assert!(parse_oci_spec(b"{}").is_err()); // missing ociVersion
    }

    #[test]
    fn oci_spec_roundtrip() {
        let spec = OciSpec {
            version: "1.0.2".into(),
            hostname: Some("my-container".into()),
            domainname: Some("my-domain".into()),
            process: Some(OciProcess {
                args: Some(vec!["/bin/sh".into(), "-c".into(), "echo hello".into()]),
                env: Some(vec!["PATH=/usr/bin".into(), "HOME=/root".into()]),
                cwd: Some("/tmp".into()),
                terminal: Some(false),
                no_new_privileges: Some(true),
                oom_score_adj: Some(100),
                user: Some(OciUser {
                    uid: Some(1000),
                    gid: Some(1000),
                    additional_gids: Some(vec![1001, 1002]),
                    umask: Some(0o022),
                }),
                ..Default::default()
            }),
            root: Some(OciRoot {
                path: "/var/lib/containers/mycontainer/rootfs".into(),
                readonly: Some(true),
            }),
            linux: Some(OciLinux {
                cgroups_path: Some("/my-cgroup".into()),
                rootfs_propagation: Some("shared".into()),
                ..Default::default()
            }),
            mounts: Some(vec![OciMount {
                destination: "/proc".into(),
                mount_type: Some("proc".into()),
                source: Some("proc".into()),
                ..Default::default()
            }]),
            annotations: Some(std::collections::HashMap::from([(
                "org.edgerun.container.id".into(),
                "my-id".into(),
            )])),
            platform: None,
        };

        let json = spec.to_json_string();
        let parsed: OciSpec = from_slice(json.as_bytes()).unwrap();

        assert_eq!(parsed.version, "1.0.2");
        assert_eq!(parsed.hostname, Some("my-container".into()));
        assert_eq!(parsed.domainname, Some("my-domain".into()));
        let proc = parsed.process.as_ref().unwrap();
        assert_eq!(proc.args.as_ref().unwrap().len(), 3);
        assert_eq!(proc.cwd, Some("/tmp".into()));
        assert_eq!(proc.user.as_ref().unwrap().uid, Some(1000));
        assert_eq!(proc.user.as_ref().unwrap().umask, Some(0o022));
        assert_eq!(parsed.root.as_ref().unwrap().readonly, Some(true));
    }

    #[test]
    fn oci_spec_skip_serializing_if_omits_none_fields() {
        let spec = OciSpec {
            version: "1.0.2".into(),
            platform: None,
            process: None,
            root: None,
            hostname: None,
            domainname: None,
            linux: None,
            mounts: None,
            annotations: None,
        };
        let json = spec.to_json_string();
        assert!(!json.contains("hostname"));
        assert!(!json.contains("platform"));
        assert!(!json.contains("process"));
    }

    #[test]
    fn oci_platform_matches_host_linux_amd64() {
        let platform = OciPlatform {
            os: Some("linux".into()),
            arch: Some(if cfg!(target_arch = "x86_64") {
                "amd64".into()
            } else {
                "arm64".into()
            }),
            ..Default::default()
        };
        #[cfg(target_arch = "x86_64")]
        assert!(platform.matches_host());
        #[cfg(target_arch = "aarch64")]
        assert!(platform.matches_host());
    }

    #[test]
    fn oci_platform_rejects_windows() {
        let platform = OciPlatform {
            os: Some("windows".into()),
            ..Default::default()
        };
        assert!(!platform.matches_host());
    }

    #[test]
    fn oci_platform_rejects_wrong_arch() {
        let platform = OciPlatform {
            os: Some("linux".into()),
            arch: Some("s390x".into()),
            ..Default::default()
        };
        assert!(!platform.matches_host());
    }

    #[test]
    fn oci_platform_defaults_match_host() {
        let platform = OciPlatform::default();
        assert!(platform.matches_host());
    }

    #[test]
    fn oci_capabilities_roundtrip() {
        let caps = OciCapabilities {
            bounding: Some(vec!["CAP_NET_BIND_SERVICE".into()]),
            effective: Some(vec!["CAP_NET_BIND_SERVICE".into()]),
            inheritable: Some(vec!["CAP_NET_BIND_SERVICE".into()]),
            permitted: Some(vec!["CAP_NET_BIND_SERVICE".into()]),
            ambient: Some(vec!["CAP_NET_BIND_SERVICE".into()]),
        };
        let json = edgerun_json::to_string(&caps).unwrap();
        let parsed: OciCapabilities = from_slice(json.as_bytes()).unwrap();
        assert_eq!(parsed.bounding, caps.bounding);
        assert_eq!(parsed.effective, caps.effective);
    }

    #[test]
    fn oci_rlimit_serialization() {
        let rlimit = OciRlimit {
            ns_type: "RLIMIT_NOFILE".into(),
            hard: 65536,
            soft: 65536,
        };
        let json = edgerun_json::to_string(&rlimit).unwrap();
        assert!(json.contains("\"type\":\"RLIMIT_NOFILE\""));
        assert!(json.contains("\"hard\":65536"));
        let parsed: OciRlimit = from_slice(json.as_bytes()).unwrap();
        assert_eq!(parsed.ns_type, "RLIMIT_NOFILE");
        assert_eq!(parsed.hard, 65536);
    }

    #[test]
    fn oci_id_mapping_serialization() {
        let mapping = OciIdMapping {
            container_id: 0,
            host_id: 100000,
            size: 65536,
        };
        let json = edgerun_json::to_string(&mapping).unwrap();
        assert!(json.contains("\"containerID\":0"));
        assert!(json.contains("\"hostID\":100000"));
        let parsed: OciIdMapping = from_slice(json.as_bytes()).unwrap();
        assert_eq!(parsed.container_id, 0);
        assert_eq!(parsed.host_id, 100000);
        assert_eq!(parsed.size, 65536);
    }

    #[test]
    fn oci_linux_resources_full() {
        let resources = OciLinuxResources {
            memory: Some(OciLinuxMemory {
                limit: Some(512 * 1024 * 1024),
                swap: Some(1024 * 1024 * 1024),
                reservation: Some(256 * 1024 * 1024),
                kernel: Some(128 * 1024 * 1024),
                kernel_tcp: Some(64 * 1024 * 1024),
                check_before_update: Some(true),
            }),
            cpu: Some(OciLinuxCpu {
                shares: Some(512),
                quota: Some(100000),
                period: Some(100000),
                cpus: Some("0-3".into()),
                mems: Some("0".into()),
                idle: Some(1),
                burst: Some(500000),
                ..Default::default()
            }),
            pids: Some(OciLinuxPids { limit: 100 }),
            ..Default::default()
        };
        let json = edgerun_json::to_string(&resources).unwrap();
        let parsed: OciLinuxResources = from_slice(json.as_bytes()).unwrap();
        assert_eq!(
            parsed.memory.as_ref().unwrap().limit,
            Some(512 * 1024 * 1024)
        );
        assert_eq!(parsed.cpu.as_ref().unwrap().shares, Some(512));
        assert_eq!(parsed.cpu.as_ref().unwrap().idle, Some(1));
        assert_eq!(parsed.cpu.as_ref().unwrap().burst, Some(500000));
        assert_eq!(parsed.pids.as_ref().unwrap().limit, 100);
    }

    #[test]
    fn oci_hooks_deserialize_all_six_types() {
        let json = r#"{
            "prestart": [{"path": "/bin/prestart"}],
            "createRuntime": [{"path": "/bin/createRuntime"}],
            "createContainer": [{"path": "/bin/createContainer"}],
            "startContainer": [{"path": "/bin/startContainer"}],
            "poststart": [{"path": "/bin/poststart"}],
            "poststop": [{"path": "/bin/poststop"}]
        }"#;
        let hooks: OciHooks = from_slice(json.as_bytes()).unwrap();
        assert!(hooks.prestart.is_some());
        assert!(hooks.create_runtime.is_some());
        assert!(hooks.create_container.is_some());
        assert!(hooks.start_container.is_some());
        assert!(hooks.poststart.is_some());
        assert!(hooks.poststop.is_some());
    }

    #[test]
    fn oci_hook_with_timeout_and_env() {
        let json =
            r#"{"path":"/bin/hook","args":["hook","--flag"],"env":["FOO=bar"],"timeout":30}"#;
        let hook: OciHook = from_slice(json.as_bytes()).unwrap();
        assert_eq!(hook.path, "/bin/hook");
        assert_eq!(hook.args, Some(vec!["hook".into(), "--flag".into()]));
        assert_eq!(hook.env, Some(vec!["FOO=bar".into()]));
        assert_eq!(hook.timeout, Some(30));
    }

    #[test]
    fn oci_seccomp_action_serde() {
        // All action variants
        for action_str in &[
            "SCMP_ACT_KILL",
            "SCMP_ACT_KILL_PROCESS",
            "SCMP_ACT_KILL_THREAD",
            "SCMP_ACT_TRAP",
            "SCMP_ACT_ERRNO",
            "SCMP_ACT_TRACE",
            "SCMP_ACT_ALLOW",
            "SCMP_ACT_NOTIFY",
            "SCMP_ACT_LOG",
        ] {
            let action: OciSeccompAction =
                from_slice(format!("\"{}\"", action_str).as_bytes()).unwrap();
            let serialized: String = action.clone().into();
            assert_eq!(&serialized, action_str);
        }
    }

    #[test]
    fn oci_seccomp_action_unknown_fails() {
        let result: Result<OciSeccompAction, _> = from_slice(b"\"SCMP_ACT_UNKNOWN\"");
        assert!(result.is_err());
    }

    #[test]
    fn oci_seccomp_action_without_prefix() {
        let result: Result<OciSeccompAction, _> = from_slice(b"\"KILL\"");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), OciSeccompAction::Kill);
    }

    #[test]
    fn oci_seccomp_full_spec() {
        let json = r#"{
            "defaultAction": "SCMP_ACT_ERRNO",
            "defaultErrnoRet": 1,
            "architectures": ["SCMP_ARCH_X86_64", "SCMP_ARCH_X32"],
            "listenerPath": "/run/seccomp-listener",
            "listenerMetadata": "container-id=test",
            "syscalls": [
                {
                    "names": ["read", "write"],
                    "action": "SCMP_ACT_ALLOW",
                    "args": []
                },
                {
                    "names": ["openat"],
                    "action": "SCMP_ACT_ERRNO",
                    "errnoRet": 13,
                    "args": [{"index": 1, "value": 65, "valueTwo": 0, "op": "SCMP_CMP_EQ"}]
                }
            ]
        }"#;
        let seccomp: OciLinuxSeccomp = from_slice(json.as_bytes()).unwrap();
        assert_eq!(seccomp.default_action, Some(OciSeccompAction::Errno));
        assert_eq!(seccomp.default_errno_ret, Some(1));
        assert_eq!(
            seccomp.architectures,
            Some(vec!["SCMP_ARCH_X86_64".into(), "SCMP_ARCH_X32".into()])
        );
        assert_eq!(seccomp.listener_path, Some("/run/seccomp-listener".into()));
        assert_eq!(seccomp.listener_metadata, Some("container-id=test".into()));
        let syscalls = seccomp.syscalls.as_ref().unwrap();
        assert_eq!(syscalls.len(), 2);
        assert_eq!(syscalls[0].names, Some(vec!["read".into(), "write".into()]));
        assert_eq!(syscalls[0].action, Some(OciSeccompAction::Allow));
        assert_eq!(syscalls[1].errno_ret, Some(13));
        let args = syscalls[1].args.as_ref().unwrap();
        assert_eq!(args[0].index, 1);
        assert_eq!(args[0].value, 65);
        assert_eq!(args[0].op, "SCMP_CMP_EQ");
    }

    #[test]
    fn oci_mount_with_idmapped_mappings() {
        let mount = OciMount {
            destination: "/data".into(),
            mount_type: Some("ext4".into()),
            source: Some("/dev/sda1".into()),
            uid_mappings: Some(vec![OciIdMapping {
                container_id: 0,
                host_id: 1000,
                size: 1,
            }]),
            gid_mappings: Some(vec![OciIdMapping {
                container_id: 0,
                host_id: 1000,
                size: 1,
            }]),
            recursive: Some(true),
            ..Default::default()
        };
        let json = edgerun_json::to_string(&mount).unwrap();
        assert!(json.contains("uidMappings"));
        assert!(json.contains("gidMappings"));
        assert!(json.contains("\"recursive\":true"));
        let parsed: OciMount = from_slice(json.as_bytes()).unwrap();
        assert!(parsed.recursive.unwrap());
        assert_eq!(parsed.uid_mappings.as_ref().unwrap()[0].host_id, 1000);
    }

    #[test]
    fn oci_linux_devices_serialization() {
        let devices = vec![OciLinuxDevice {
            ns_type: "c".into(),
            path: "/dev/null".into(),
            file_mode: Some(0o666),
            uid: Some(0),
            gid: Some(0),
            major: Some(1),
            minor: Some(3),
        }];
        let json = edgerun_json::to_string(&devices).unwrap();
        let parsed: Vec<OciLinuxDevice> = from_slice(json.as_bytes()).unwrap();
        assert_eq!(parsed[0].ns_type, "c");
        assert_eq!(parsed[0].path, "/dev/null");
        assert_eq!(parsed[0].major, Some(1));
    }

    #[test]
    fn oci_scheduler_serialization() {
        let scheduler = OciScheduler {
            policy: "SCHED_DEADLINE".into(),
            deadline: Some(OciSchedDeadline {
                runtime_ns: Some(1000000),
                period_ns: Some(10000000),
                deadline_ns: Some(5000000),
            }),
            ..Default::default()
        };
        let json = edgerun_json::to_string(&scheduler).unwrap();
        assert!(json.contains("SCHED_DEADLINE"));
        let parsed: OciScheduler = from_slice(json.as_bytes()).unwrap();
        assert_eq!(parsed.deadline.as_ref().unwrap().runtime_ns, Some(1000000));
    }

    #[test]
    fn oci_io_priority_serialization() {
        let io_prio = OciIoPriority {
            class: 2,
            priority: Some(4),
        };
        let json = edgerun_json::to_string(&io_prio).unwrap();
        assert!(json.contains("\"class\":2"));
        let parsed: OciIoPriority = from_slice(json.as_bytes()).unwrap();
        assert_eq!(parsed.class, 2);
        assert_eq!(parsed.priority, Some(4));
    }

    #[test]
    fn oci_block_io_full_serialization() {
        let blkio = OciLinuxBlockIO {
            weight: Some(500),
            weight_device: Some(vec![OciLinuxWeightDevice {
                major: 8,
                minor: 0,
                weight: Some(300),
                leaf_weight: None,
            }]),
            throttle_read_bps_device: Some(vec![OciLinuxThrottleDevice {
                major: 8,
                minor: 0,
                rate: 102400,
            }]),
            throttle_write_iops_device: Some(vec![OciLinuxThrottleDevice {
                major: 8,
                minor: 0,
                rate: 1000,
            }]),
            ..Default::default()
        };
        let json = edgerun_json::to_string(&blkio).unwrap();
        let parsed: OciLinuxBlockIO = from_slice(json.as_bytes()).unwrap();
        assert_eq!(parsed.weight, Some(500));
        assert_eq!(parsed.weight_device.as_ref().unwrap()[0].major, 8);
        assert_eq!(
            parsed.throttle_read_bps_device.as_ref().unwrap()[0].rate,
            102400
        );
    }

    #[test]
    fn oci_hugepage_with_rsvd() {
        let hp = OciLinuxHugepageLimit {
            pagesize: "2MB".into(),
            limit: 536870912,
            rsvd: Some(true),
        };
        let json = edgerun_json::to_string(&hp).unwrap();
        let parsed: OciLinuxHugepageLimit = from_slice(json.as_bytes()).unwrap();
        assert_eq!(parsed.pagesize, "2MB");
        assert_eq!(parsed.rsvd, Some(true));
    }

    #[test]
    fn oci_intel_rdt_serialization() {
        let rdt = OciLinuxIntelRdt {
            l3_cache_schema: Some("L3:0=fff".into()),
            mem_bw_schema: Some("MB:0=70".into()),
            clos_id: Some("p0".into()),
            enable_monitoring: Some(true),
            schemata: Some("L3:0=fff\nMB:0=70".into()),
        };
        let json = edgerun_json::to_string(&rdt).unwrap();
        let parsed: OciLinuxIntelRdt = from_slice(json.as_bytes()).unwrap();
        assert_eq!(parsed.l3_cache_schema, Some("L3:0=fff".into()));
        assert_eq!(parsed.schemata, Some("L3:0=fff\nMB:0=70".into()));
        assert_eq!(parsed.enable_monitoring, Some(true));
    }

    #[test]
    fn oci_network_serialization() {
        let net = OciLinuxNetwork {
            class_id: Some(0x1234),
            priorities: Some(vec![OciLinuxNetworkPriority {
                name: "eth0".into(),
                priority: 3,
            }]),
        };
        let json = edgerun_json::to_string(&net).unwrap();
        let parsed: OciLinuxNetwork = from_slice(json.as_bytes()).unwrap();
        assert_eq!(parsed.class_id, Some(0x1234));
        assert_eq!(parsed.priorities.as_ref().unwrap()[0].name, "eth0");
    }

    #[test]
    fn oci_device_cgroup_serialization() {
        let dev = OciLinuxDeviceCgroup {
            ns_type: "b".into(),
            major: Some(8),
            minor: Some(0),
            access: Some("rw".into()),
        };
        let json = edgerun_json::to_string(&dev).unwrap();
        assert!(json.contains("\"type\":\"b\""));
        let parsed: OciLinuxDeviceCgroup = from_slice(json.as_bytes()).unwrap();
        assert_eq!(parsed.ns_type, "b");
        assert_eq!(parsed.major, Some(8));
    }

    #[test]
    fn oci_box_console_size() {
        let box_ = OciBox {
            width: 80,
            height: 24,
        };
        let json = edgerun_json::to_string(&box_).unwrap();
        let parsed: OciBox = from_slice(json.as_bytes()).unwrap();
        assert_eq!(parsed.width, 80);
        assert_eq!(parsed.height, 24);
    }

    #[test]
    fn oci_spec_parse_oci_spec_function() {
        let json = r#"{"ociVersion":"1.0.2","platform":{"os":"linux","arch":"amd64"}}"#;
        let spec = parse_oci_spec(json.as_bytes()).unwrap();
        assert_eq!(spec.version, "1.0.2");
        let platform = spec.platform.unwrap();
        assert_eq!(platform.os, Some("linux".into()));
        assert_eq!(platform.arch, Some("amd64".into()));
    }
}
