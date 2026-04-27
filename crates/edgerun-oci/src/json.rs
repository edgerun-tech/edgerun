//! OCI spec types with serde-based JSON serialization via edgerun-json.

use crate::prelude::*;
#[cfg(feature = "serde")]
use edgerun_json::{from_slice, to_string, to_string_pretty};
#[cfg(all(feature = "json", not(feature = "serde")))]
use edgerun_json::{parse_json, JsonValue};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

// ===========================================================================
// Public API
// ===========================================================================

/// Parse an OCI spec from JSON bytes.
#[cfg(feature = "serde")]
pub fn parse_oci_spec(data: &[u8]) -> Result<OciSpec, String> {
    from_slice(data).map_err(|e| e.to_string())
}

/// Parse an OCI spec from JSON bytes without serde.
///
/// This no_std parser covers the runtime-critical OCI fields needed by a
/// bare-metal loader: version, platform, process, root, mounts, Linux
/// namespaces, devices, mappings, sysctls, and basic resource settings.
#[cfg(all(feature = "json", not(feature = "serde")))]
pub fn parse_oci_spec(data: &[u8]) -> Result<OciSpec, String> {
    let text = core::str::from_utf8(data).map_err(|_| "OCI spec is not UTF-8".to_string())?;
    let value = parse_json(text).map_err(|e| e.to_string())?;
    parse_oci_spec_value(&value)
}

// ===========================================================================
// OCI Spec types
// ===========================================================================

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct OciSpec {
    #[cfg_attr(feature = "serde", serde(rename = "ociVersion"))]
    pub version: String,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub platform: Option<OciPlatform>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub process: Option<OciProcess>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub root: Option<OciRoot>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub hostname: Option<String>,
    /// NIS domain name for the container (OCI 1.1.0).
    /// Set via setdomainname(2) syscall.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub domainname: Option<String>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub linux: Option<OciLinux>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub mounts: Option<Vec<OciMount>>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub annotations: Option<BTreeMap<String, String>>,
}

/// Target platform the bundle was built for.
/// Per OCI spec: runtime must reject bundles whose platform doesn't match the host.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", serde(default))]
pub struct OciPlatform {
    /// Operating system: "linux", "windows", "solaris", etc.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub os: Option<String>,
    /// CPU architecture: "amd64", "arm64", "riscv64", etc.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub arch: Option<String>,
    /// OS variant (optional): e.g., "v1", "v2" for Windows.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub os_version: Option<String>,
    /// OS features (optional).
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
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
    #[cfg(feature = "serde")]
    pub fn to_json_string(&self) -> String {
        to_string(self).unwrap_or_default()
    }

    /// Serialize to pretty-printed JSON.
    #[cfg(feature = "serde")]
    pub fn to_json_string_pretty(&self) -> String {
        to_string_pretty(self).unwrap_or_default()
    }
}

/// Terminal console dimensions (cells, not pixels).
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", serde(default))]
pub struct OciBox {
    pub width: u64,
    pub height: u64,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", serde(default))]
pub struct OciProcess {
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub terminal: Option<bool>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub user: Option<OciUser>,
    /// Console size for terminal (width and height in cells).
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", rename = "consoleSize")
    )]
    pub console_size: Option<OciBox>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub args: Option<Vec<String>>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub env: Option<Vec<String>>,
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", rename = "cwd")
    )]
    pub cwd: Option<String>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub capabilities: Option<OciCapabilities>,
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", rename = "rlimits")
    )]
    pub rlimits: Option<Vec<OciRlimit>>,
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", rename = "noNewPrivileges")
    )]
    pub no_new_privileges: Option<bool>,
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", rename = "oomScoreAdj")
    )]
    pub oom_score_adj: Option<i64>,
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", rename = "apparmorProfile")
    )]
    pub apparmor_profile: Option<String>,
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", rename = "selinuxLabel")
    )]
    pub selinux_label: Option<String>,
    /// Real-time scheduling policy and parameters (OCI 1.0.2).
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub scheduler: Option<OciScheduler>,
    /// I/O priority for the container process (OCI 1.1.0).
    /// Uses the Linux ioprio_set() interface: class 0-3, priority 0-7.
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", rename = "ioPriority")
    )]
    pub io_priority: Option<OciIoPriority>,
}

/// I/O priority configuration (OCI 1.1.0).
/// Mirrors the Linux ioprio_set(2) interface.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", serde(default))]
pub struct OciIoPriority {
    /// I/O scheduling class: 0=none, 1=realtime, 2=best-effort, 3=idle.
    #[cfg_attr(feature = "serde", serde(rename = "class"))]
    pub class: u32,
    /// I/O priority level within the class (0-7, lower = higher priority).
    /// Ignored for class 0 (none) and class 3 (idle).
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub priority: Option<u32>,
}

/// Real-time scheduling configuration.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", serde(default))]
pub struct OciScheduler {
    /// Scheduling policy: "SCHED_OTHER", "SCHED_FIFO", "SCHED_RR", "SCHED_BATCH", "SCHED_IDLE", "SCHED_DEADLINE"
    pub policy: String,
    /// Nice value (only for SCHED_OTHER and SCHED_BATCH).
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub nice: Option<i32>,
    /// Scheduling priority (for SCHED_FIFO and SCHED_RR, range 1-99).
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub priority: Option<i32>,
    /// SCHED_DEADLINE parameters.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub deadline: Option<OciSchedDeadline>,
}

/// SCHED_DEADLINE scheduling parameters.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", serde(default))]
pub struct OciSchedDeadline {
    /// Runtime in nanoseconds.
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", rename = "runtime")
    )]
    pub runtime_ns: Option<u64>,
    /// Period in nanoseconds.
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", rename = "period")
    )]
    pub period_ns: Option<u64>,
    /// Deadline in nanoseconds.
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", rename = "deadline")
    )]
    pub deadline_ns: Option<u64>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", serde(default))]
pub struct OciUser {
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub uid: Option<u32>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub gid: Option<u32>,
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", rename = "additionalGids")
    )]
    pub additional_gids: Option<Vec<u32>>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub umask: Option<u32>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", serde(default))]
pub struct OciCapabilities {
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub bounding: Option<Vec<String>>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub effective: Option<Vec<String>>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub inheritable: Option<Vec<String>>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub permitted: Option<Vec<String>>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub ambient: Option<Vec<String>>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Default)]
pub struct OciRlimit {
    #[cfg_attr(feature = "serde", serde(rename = "type"))]
    pub ns_type: String,
    pub hard: u64,
    pub soft: u64,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", serde(default))]
pub struct OciRoot {
    pub path: String,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub readonly: Option<bool>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", serde(default))]
pub struct OciLinux {
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", rename = "uidMappings")
    )]
    pub uid_mappings: Option<Vec<OciIdMapping>>,
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", rename = "gidMappings")
    )]
    pub gid_mappings: Option<Vec<OciIdMapping>>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub resources: Option<OciLinuxResources>,
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", rename = "cgroupsPath")
    )]
    pub cgroups_path: Option<String>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub namespaces: Option<Vec<OciNamespace>>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub devices: Option<Vec<OciLinuxDevice>>,
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", rename = "maskedPaths")
    )]
    pub masked_paths: Option<Vec<String>>,
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", rename = "readonlyPaths")
    )]
    pub readonly_paths: Option<Vec<String>>,
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", rename = "mountLabel")
    )]
    pub mount_label: Option<String>,
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", rename = "rootfsPropagation")
    )]
    pub rootfs_propagation: Option<String>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub sysctl: Option<BTreeMap<String, String>>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub hooks: Option<OciHooks>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub seccomp: Option<OciLinuxSeccomp>,
    /// Intel RDT (Resource Director Technology) configuration.
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", rename = "intelRdt")
    )]
    pub intel_rdt: Option<OciLinuxIntelRdt>,
}

// ===========================================================================
// OCI Hooks types
// ===========================================================================

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", serde(default))]
pub struct OciHooks {
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub prestart: Option<Vec<OciHook>>,
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", rename = "createRuntime")
    )]
    pub create_runtime: Option<Vec<OciHook>>,
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", rename = "createContainer")
    )]
    pub create_container: Option<Vec<OciHook>>,
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", rename = "startContainer")
    )]
    pub start_container: Option<Vec<OciHook>>,
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", rename = "poststart")
    )]
    pub poststart: Option<Vec<OciHook>>,
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", rename = "poststop")
    )]
    pub poststop: Option<Vec<OciHook>>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", serde(default))]
pub struct OciHook {
    pub path: String,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub args: Option<Vec<String>>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub env: Option<Vec<String>>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub timeout: Option<u64>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Default)]
pub struct OciIdMapping {
    #[cfg_attr(feature = "serde", serde(rename = "containerID"))]
    pub container_id: u32,
    #[cfg_attr(feature = "serde", serde(rename = "hostID"))]
    pub host_id: u32,
    pub size: u32,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", serde(default))]
pub struct OciNamespace {
    #[cfg_attr(feature = "serde", serde(rename = "type"))]
    pub ns_type: String,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub path: Option<String>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", serde(default))]
pub struct OciLinuxDevice {
    #[cfg_attr(feature = "serde", serde(rename = "type"))]
    pub ns_type: String,
    pub path: String,
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", rename = "fileMode")
    )]
    pub file_mode: Option<u32>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub uid: Option<u32>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub gid: Option<u32>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub major: Option<i64>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub minor: Option<i64>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", serde(default))]
pub struct OciLinuxCpu {
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub shares: Option<u64>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub quota: Option<i64>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub period: Option<u64>,
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", rename = "realtimeRuntime")
    )]
    pub realtime_runtime: Option<i64>,
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", rename = "realtimePeriod")
    )]
    pub realtime_period: Option<u64>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub cpus: Option<String>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub mems: Option<String>,
    /// CPU idle cgroup control (OCI 1.1.0).
    /// 0 = not idle, 1 = idle. When idle, CPU bandwidth is deprioritized.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub idle: Option<i64>,
    /// CFS bandwidth burst size in nanoseconds (OCI 1.1.0).
    /// Allows temporary CPU quota overrun for latency-sensitive workloads.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub burst: Option<i64>,
}

// ===========================================================================
// Block I/O cgroup types
// ===========================================================================

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", serde(default))]
pub struct OciLinuxBlockIO {
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub weight: Option<u16>,
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", rename = "leafWeight")
    )]
    pub leaf_weight: Option<u16>,
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", rename = "weightDevice")
    )]
    pub weight_device: Option<Vec<OciLinuxWeightDevice>>,
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", rename = "leafWeightDevice")
    )]
    pub leaf_weight_device: Option<Vec<OciLinuxWeightDevice>>,
    #[cfg_attr(
        feature = "serde",
        serde(
            skip_serializing_if = "Option::is_none",
            rename = "throttleReadBpsDevice"
        )
    )]
    pub throttle_read_bps_device: Option<Vec<OciLinuxThrottleDevice>>,
    #[cfg_attr(
        feature = "serde",
        serde(
            skip_serializing_if = "Option::is_none",
            rename = "throttleWriteBpsDevice"
        )
    )]
    pub throttle_write_bps_device: Option<Vec<OciLinuxThrottleDevice>>,
    #[cfg_attr(
        feature = "serde",
        serde(
            skip_serializing_if = "Option::is_none",
            rename = "throttleReadIOPSDevice"
        )
    )]
    pub throttle_read_iops_device: Option<Vec<OciLinuxThrottleDevice>>,
    #[cfg_attr(
        feature = "serde",
        serde(
            skip_serializing_if = "Option::is_none",
            rename = "throttleWriteIOPSDevice"
        )
    )]
    pub throttle_write_iops_device: Option<Vec<OciLinuxThrottleDevice>>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", serde(default))]
pub struct OciLinuxWeightDevice {
    pub major: i64,
    pub minor: i64,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub weight: Option<u16>,
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", rename = "leafWeight")
    )]
    pub leaf_weight: Option<u16>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", serde(default))]
pub struct OciLinuxThrottleDevice {
    pub major: i64,
    pub minor: i64,
    pub rate: u64,
}

// ===========================================================================
// Hugepage and network cgroup types
// ===========================================================================

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Default)]
pub struct OciLinuxHugepageLimit {
    pub pagesize: String,
    pub limit: u64,
    /// Reserved huge page accounting (OCI 1.1.0).
    /// When true, apply limit to hugetlb.<size>.rsvd.max instead of hugetlb.<size>.max.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub rsvd: Option<bool>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", serde(default))]
pub struct OciLinuxNetwork {
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", rename = "classID")
    )]
    pub class_id: Option<u32>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub priorities: Option<Vec<OciLinuxNetworkPriority>>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", serde(default))]
pub struct OciLinuxNetworkPriority {
    pub name: String,
    pub priority: u32,
}

/// Intel RDT (Resource Director Technology) configuration.
/// Controls cache and memory bandwidth allocation.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", serde(default))]
pub struct OciLinuxIntelRdt {
    /// Cache Bitmask (CBM) for L3 cache. E.g., "L3:0=fff"
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", rename = "l3CacheSchema")
    )]
    pub l3_cache_schema: Option<String>,
    /// Memory bandwidth schema. E.g., "MB:0=70"
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", rename = "memBwSchema")
    )]
    pub mem_bw_schema: Option<String>,
    /// Class of Service ID
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", rename = "closID")
    )]
    pub clos_id: Option<String>,
    /// Enable CMT/MBM monitoring for this container (OCI 1.3.0).
    /// When true, the runtime should enable cache and memory bandwidth monitoring.
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", rename = "enableMonitoring")
    )]
    pub enable_monitoring: Option<bool>,
    /// Combined schemata format (OCI 1.3.0).
    /// E.g., "L3:0=fff\nMB:0=70" — overrides l3_cache_schema and mem_bw_schema if set.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub schemata: Option<String>,
}

// ===========================================================================
// Seccomp types
// ===========================================================================

/// Seccomp filtering configuration from the OCI spec.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", serde(default))]
pub struct OciLinuxSeccomp {
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", rename = "defaultAction")
    )]
    pub default_action: Option<OciSeccompAction>,
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", rename = "defaultErrnoRet")
    )]
    pub default_errno_ret: Option<u32>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub architectures: Option<Vec<String>>,
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", rename = "listenerPath")
    )]
    pub listener_path: Option<String>,
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", rename = "listenerMetadata")
    )]
    pub listener_metadata: Option<String>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub syscalls: Option<Vec<OciSeccompSyscallEntry>>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(
    feature = "serde",
    serde(
        rename_all = "SCREAMING_SNAKE_CASE",
        try_from = "String",
        into = "String"
    )
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

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", serde(default))]
pub struct OciSeccompSyscallEntry {
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub names: Option<Vec<String>>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub action: Option<OciSeccompAction>,
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", rename = "errnoRet")
    )]
    pub errno_ret: Option<u32>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub args: Option<Vec<OciSeccompArg>>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", serde(default))]
pub struct OciSeccompArg {
    pub index: u32,
    pub value: u64,
    #[cfg_attr(feature = "serde", serde(rename = "valueTwo"))]
    pub value_two: u64,
    pub op: String,
}

// ===========================================================================
// Update OciLinux to include seccomp
// ===========================================================================

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Default)]
pub struct OciLinuxResources {
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub devices: Option<Vec<OciLinuxDeviceCgroup>>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub memory: Option<OciLinuxMemory>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub cpu: Option<OciLinuxCpu>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub pids: Option<OciLinuxPids>,
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", rename = "blockIO")
    )]
    pub block_io: Option<OciLinuxBlockIO>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub hugepage_limits: Option<Vec<OciLinuxHugepageLimit>>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub network: Option<OciLinuxNetwork>,
}

/// Device cgroup rule for allowed/denied device access.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", serde(default))]
pub struct OciLinuxDeviceCgroup {
    #[cfg_attr(feature = "serde", serde(rename = "type"))]
    pub ns_type: String,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub major: Option<i64>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub minor: Option<i64>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub access: Option<String>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Default)]
pub struct OciLinuxPids {
    pub limit: i64,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", serde(default))]
pub struct OciLinuxMemory {
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub limit: Option<i64>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub reservation: Option<i64>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub swap: Option<i64>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub kernel: Option<i64>,
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", rename = "kernelTCP")
    )]
    pub kernel_tcp: Option<i64>,
    /// Hint to runtime to validate memory limit before updating (OCI 1.1.0).
    /// When true, the runtime should check if the new limit is feasible
    /// before applying it, rather than failing after the fact.
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", rename = "checkBeforeUpdate")
    )]
    pub check_before_update: Option<bool>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", serde(default))]
pub struct OciMount {
    pub destination: String,
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", rename = "type")
    )]
    pub mount_type: Option<String>,
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", rename = "source")
    )]
    pub source: Option<String>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub options: Option<Vec<String>>,
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", rename = "label")
    )]
    pub label: Option<String>,
    /// Recursive mount attribute (OCI 1.1).
    /// When true, mount options apply recursively to sub-mounts.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub recursive: Option<bool>,
    /// UID mappings for idmapped mounts (OCI 1.1/1.2, Linux 5.12+).
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", rename = "uidMappings")
    )]
    pub uid_mappings: Option<Vec<OciIdMapping>>,
    /// GID mappings for idmapped mounts (OCI 1.1/1.2, Linux 5.12+).
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", rename = "gidMappings")
    )]
    pub gid_mappings: Option<Vec<OciIdMapping>>,
}

#[cfg(all(feature = "json", not(feature = "serde")))]
fn parse_oci_spec_value(value: &JsonValue) -> Result<OciSpec, String> {
    let object = object(value, "OCI spec")?;
    let version = required_string(object.get("ociVersion"), "ociVersion")?;

    Ok(OciSpec {
        version,
        platform: object.get("platform").map(parse_platform).transpose()?,
        process: object.get("process").map(parse_process).transpose()?,
        root: object.get("root").map(parse_root).transpose()?,
        hostname: optional_string(object.get("hostname"))?,
        domainname: optional_string(object.get("domainname"))?,
        linux: object.get("linux").map(parse_linux).transpose()?,
        mounts: object.get("mounts").map(parse_mounts).transpose()?,
        annotations: object
            .get("annotations")
            .map(parse_string_map)
            .transpose()?,
    })
}

#[cfg(all(feature = "json", not(feature = "serde")))]
fn parse_platform(value: &JsonValue) -> Result<OciPlatform, String> {
    let object = object(value, "platform")?;
    Ok(OciPlatform {
        os: optional_string(object.get("os"))?,
        arch: optional_string(object.get("arch"))?,
        os_version: optional_string(object.get("os.version"))?,
        os_features: object
            .get("os.features")
            .map(parse_string_array)
            .transpose()?,
    })
}

#[cfg(all(feature = "json", not(feature = "serde")))]
fn parse_process(value: &JsonValue) -> Result<OciProcess, String> {
    let object = object(value, "process")?;
    Ok(OciProcess {
        terminal: optional_bool(object.get("terminal"))?,
        user: object.get("user").map(parse_user).transpose()?,
        console_size: object.get("consoleSize").map(parse_box).transpose()?,
        args: object.get("args").map(parse_string_array).transpose()?,
        env: object.get("env").map(parse_string_array).transpose()?,
        cwd: optional_string(object.get("cwd"))?,
        capabilities: object
            .get("capabilities")
            .map(parse_capabilities)
            .transpose()?,
        rlimits: object.get("rlimits").map(parse_rlimits).transpose()?,
        no_new_privileges: optional_bool(object.get("noNewPrivileges"))?,
        oom_score_adj: optional_i64(object.get("oomScoreAdj"))?,
        apparmor_profile: optional_string(object.get("apparmorProfile"))?,
        selinux_label: optional_string(object.get("selinuxLabel"))?,
        scheduler: object.get("scheduler").map(parse_scheduler).transpose()?,
        io_priority: object
            .get("ioPriority")
            .map(parse_io_priority)
            .transpose()?,
    })
}

#[cfg(all(feature = "json", not(feature = "serde")))]
fn parse_box(value: &JsonValue) -> Result<OciBox, String> {
    let object = object(value, "consoleSize")?;
    Ok(OciBox {
        width: required_u64(object.get("width"), "consoleSize.width")?,
        height: required_u64(object.get("height"), "consoleSize.height")?,
    })
}

#[cfg(all(feature = "json", not(feature = "serde")))]
fn parse_user(value: &JsonValue) -> Result<OciUser, String> {
    let object = object(value, "user")?;
    Ok(OciUser {
        uid: optional_u32(object.get("uid"))?,
        gid: optional_u32(object.get("gid"))?,
        additional_gids: object
            .get("additionalGids")
            .map(parse_u32_array)
            .transpose()?,
        umask: optional_u32(object.get("umask"))?,
    })
}

#[cfg(all(feature = "json", not(feature = "serde")))]
fn parse_capabilities(value: &JsonValue) -> Result<OciCapabilities, String> {
    let object = object(value, "capabilities")?;
    Ok(OciCapabilities {
        bounding: object.get("bounding").map(parse_string_array).transpose()?,
        effective: object
            .get("effective")
            .map(parse_string_array)
            .transpose()?,
        inheritable: object
            .get("inheritable")
            .map(parse_string_array)
            .transpose()?,
        permitted: object
            .get("permitted")
            .map(parse_string_array)
            .transpose()?,
        ambient: object.get("ambient").map(parse_string_array).transpose()?,
    })
}

#[cfg(all(feature = "json", not(feature = "serde")))]
fn parse_rlimits(value: &JsonValue) -> Result<Vec<OciRlimit>, String> {
    array(value, "rlimits")?
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let object = object(value, "rlimit")?;
            Ok(OciRlimit {
                ns_type: required_string(object.get("type"), "rlimit.type")?,
                hard: required_u64(object.get("hard"), "rlimit.hard")?,
                soft: required_u64(object.get("soft"), "rlimit.soft")?,
            })
            .map_err(|e: String| format!("rlimits[{index}]: {e}"))
        })
        .collect()
}

#[cfg(all(feature = "json", not(feature = "serde")))]
fn parse_scheduler(value: &JsonValue) -> Result<OciScheduler, String> {
    let object = object(value, "scheduler")?;
    Ok(OciScheduler {
        policy: required_string(object.get("policy"), "scheduler.policy")?,
        nice: optional_i32(object.get("nice"))?,
        priority: optional_i32(object.get("priority"))?,
        deadline: object
            .get("deadline")
            .map(parse_sched_deadline)
            .transpose()?,
    })
}

#[cfg(all(feature = "json", not(feature = "serde")))]
fn parse_sched_deadline(value: &JsonValue) -> Result<OciSchedDeadline, String> {
    let object = object(value, "scheduler.deadline")?;
    Ok(OciSchedDeadline {
        runtime_ns: optional_u64(object.get("runtime"))?,
        period_ns: optional_u64(object.get("period"))?,
        deadline_ns: optional_u64(object.get("deadline"))?,
    })
}

#[cfg(all(feature = "json", not(feature = "serde")))]
fn parse_io_priority(value: &JsonValue) -> Result<OciIoPriority, String> {
    let object = object(value, "ioPriority")?;
    Ok(OciIoPriority {
        class: required_u32(object.get("class"), "ioPriority.class")?,
        priority: optional_u32(object.get("priority"))?,
    })
}

#[cfg(all(feature = "json", not(feature = "serde")))]
fn parse_root(value: &JsonValue) -> Result<OciRoot, String> {
    let object = object(value, "root")?;
    Ok(OciRoot {
        path: required_string(object.get("path"), "root.path")?,
        readonly: optional_bool(object.get("readonly"))?,
    })
}

#[cfg(all(feature = "json", not(feature = "serde")))]
fn parse_linux(value: &JsonValue) -> Result<OciLinux, String> {
    let object = object(value, "linux")?;
    Ok(OciLinux {
        uid_mappings: object
            .get("uidMappings")
            .map(parse_id_mappings)
            .transpose()?,
        gid_mappings: object
            .get("gidMappings")
            .map(parse_id_mappings)
            .transpose()?,
        resources: object.get("resources").map(parse_resources).transpose()?,
        cgroups_path: optional_string(object.get("cgroupsPath"))?,
        namespaces: object.get("namespaces").map(parse_namespaces).transpose()?,
        devices: object.get("devices").map(parse_devices).transpose()?,
        masked_paths: object
            .get("maskedPaths")
            .map(parse_string_array)
            .transpose()?,
        readonly_paths: object
            .get("readonlyPaths")
            .map(parse_string_array)
            .transpose()?,
        mount_label: optional_string(object.get("mountLabel"))?,
        rootfs_propagation: optional_string(object.get("rootfsPropagation"))?,
        sysctl: object.get("sysctl").map(parse_string_map).transpose()?,
        hooks: None,
        seccomp: None,
        intel_rdt: object.get("intelRdt").map(parse_intel_rdt).transpose()?,
    })
}

#[cfg(all(feature = "json", not(feature = "serde")))]
fn parse_id_mappings(value: &JsonValue) -> Result<Vec<OciIdMapping>, String> {
    array(value, "idMappings")?
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let object = object(value, "idMapping")?;
            Ok(OciIdMapping {
                container_id: required_u32(object.get("containerID"), "containerID")?,
                host_id: required_u32(object.get("hostID"), "hostID")?,
                size: required_u32(object.get("size"), "size")?,
            })
            .map_err(|e: String| format!("idMappings[{index}]: {e}"))
        })
        .collect()
}

#[cfg(all(feature = "json", not(feature = "serde")))]
fn parse_namespaces(value: &JsonValue) -> Result<Vec<OciNamespace>, String> {
    array(value, "namespaces")?
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let object = object(value, "namespace")?;
            Ok(OciNamespace {
                ns_type: required_string(object.get("type"), "namespace.type")?,
                path: optional_string(object.get("path"))?,
            })
            .map_err(|e: String| format!("namespaces[{index}]: {e}"))
        })
        .collect()
}

#[cfg(all(feature = "json", not(feature = "serde")))]
fn parse_devices(value: &JsonValue) -> Result<Vec<OciLinuxDevice>, String> {
    array(value, "devices")?
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let object = object(value, "device")?;
            Ok(OciLinuxDevice {
                ns_type: required_string(object.get("type"), "device.type")?,
                path: required_string(object.get("path"), "device.path")?,
                file_mode: optional_u32(object.get("fileMode"))?,
                uid: optional_u32(object.get("uid"))?,
                gid: optional_u32(object.get("gid"))?,
                major: optional_i64(object.get("major"))?,
                minor: optional_i64(object.get("minor"))?,
            })
            .map_err(|e: String| format!("devices[{index}]: {e}"))
        })
        .collect()
}

#[cfg(all(feature = "json", not(feature = "serde")))]
fn parse_resources(value: &JsonValue) -> Result<OciLinuxResources, String> {
    let object = object(value, "resources")?;
    Ok(OciLinuxResources {
        devices: object
            .get("devices")
            .map(parse_device_cgroups)
            .transpose()?,
        memory: object.get("memory").map(parse_memory).transpose()?,
        cpu: object.get("cpu").map(parse_cpu).transpose()?,
        pids: object.get("pids").map(parse_pids).transpose()?,
        block_io: None,
        hugepage_limits: None,
        network: None,
    })
}

#[cfg(all(feature = "json", not(feature = "serde")))]
fn parse_device_cgroups(value: &JsonValue) -> Result<Vec<OciLinuxDeviceCgroup>, String> {
    array(value, "resources.devices")?
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let object = object(value, "device cgroup")?;
            Ok(OciLinuxDeviceCgroup {
                ns_type: required_string(object.get("type"), "device cgroup.type")?,
                major: optional_i64(object.get("major"))?,
                minor: optional_i64(object.get("minor"))?,
                access: optional_string(object.get("access"))?,
            })
            .map_err(|e: String| format!("resources.devices[{index}]: {e}"))
        })
        .collect()
}

#[cfg(all(feature = "json", not(feature = "serde")))]
fn parse_memory(value: &JsonValue) -> Result<OciLinuxMemory, String> {
    let object = object(value, "memory")?;
    Ok(OciLinuxMemory {
        limit: optional_i64(object.get("limit"))?,
        reservation: optional_i64(object.get("reservation"))?,
        swap: optional_i64(object.get("swap"))?,
        kernel: optional_i64(object.get("kernel"))?,
        kernel_tcp: optional_i64(object.get("kernelTCP"))?,
        check_before_update: optional_bool(object.get("checkBeforeUpdate"))?,
    })
}

#[cfg(all(feature = "json", not(feature = "serde")))]
fn parse_cpu(value: &JsonValue) -> Result<OciLinuxCpu, String> {
    let object = object(value, "cpu")?;
    Ok(OciLinuxCpu {
        shares: optional_u64(object.get("shares"))?,
        quota: optional_i64(object.get("quota"))?,
        period: optional_u64(object.get("period"))?,
        realtime_runtime: optional_i64(object.get("realtimeRuntime"))?,
        realtime_period: optional_u64(object.get("realtimePeriod"))?,
        cpus: optional_string(object.get("cpus"))?,
        mems: optional_string(object.get("mems"))?,
        idle: optional_i64(object.get("idle"))?,
        burst: optional_i64(object.get("burst"))?,
    })
}

#[cfg(all(feature = "json", not(feature = "serde")))]
fn parse_pids(value: &JsonValue) -> Result<OciLinuxPids, String> {
    let object = object(value, "pids")?;
    Ok(OciLinuxPids {
        limit: required_i64(object.get("limit"), "pids.limit")?,
    })
}

#[cfg(all(feature = "json", not(feature = "serde")))]
fn parse_intel_rdt(value: &JsonValue) -> Result<OciLinuxIntelRdt, String> {
    let object = object(value, "intelRdt")?;
    Ok(OciLinuxIntelRdt {
        l3_cache_schema: optional_string(object.get("l3CacheSchema"))?,
        mem_bw_schema: optional_string(object.get("memBwSchema"))?,
        clos_id: optional_string(object.get("closID"))?,
        enable_monitoring: optional_bool(object.get("enableMonitoring"))?,
        schemata: optional_string(object.get("schemata"))?,
    })
}

#[cfg(all(feature = "json", not(feature = "serde")))]
fn parse_mounts(value: &JsonValue) -> Result<Vec<OciMount>, String> {
    array(value, "mounts")?
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let object = object(value, "mount")?;
            Ok(OciMount {
                destination: required_string(object.get("destination"), "mount.destination")?,
                mount_type: optional_string(object.get("type"))?,
                source: optional_string(object.get("source"))?,
                options: object.get("options").map(parse_string_array).transpose()?,
                label: optional_string(object.get("label"))?,
                recursive: optional_bool(object.get("recursive"))?,
                uid_mappings: object
                    .get("uidMappings")
                    .map(parse_id_mappings)
                    .transpose()?,
                gid_mappings: object
                    .get("gidMappings")
                    .map(parse_id_mappings)
                    .transpose()?,
            })
            .map_err(|e: String| format!("mounts[{index}]: {e}"))
        })
        .collect()
}

#[cfg(all(feature = "json", not(feature = "serde")))]
fn parse_string_map(value: &JsonValue) -> Result<BTreeMap<String, String>, String> {
    object(value, "string map")?
        .iter()
        .map(|(key, value)| Ok((key.clone(), string(value, key)?)))
        .collect()
}

#[cfg(all(feature = "json", not(feature = "serde")))]
fn parse_string_array(value: &JsonValue) -> Result<Vec<String>, String> {
    array(value, "string array")?
        .iter()
        .enumerate()
        .map(|(index, value)| string(value, &format!("string array[{index}]")))
        .collect()
}

#[cfg(all(feature = "json", not(feature = "serde")))]
fn parse_u32_array(value: &JsonValue) -> Result<Vec<u32>, String> {
    array(value, "u32 array")?
        .iter()
        .enumerate()
        .map(|(index, value)| u32_value(value, &format!("u32 array[{index}]")))
        .collect()
}

#[cfg(all(feature = "json", not(feature = "serde")))]
fn object<'a>(value: &'a JsonValue, name: &str) -> Result<&'a edgerun_json::Map, String> {
    value
        .as_object()
        .ok_or_else(|| format!("{name} must be an object"))
}

#[cfg(all(feature = "json", not(feature = "serde")))]
fn array<'a>(value: &'a JsonValue, name: &str) -> Result<&'a Vec<JsonValue>, String> {
    value
        .as_array()
        .ok_or_else(|| format!("{name} must be an array"))
}

#[cfg(all(feature = "json", not(feature = "serde")))]
fn optional_string(value: Option<&JsonValue>) -> Result<Option<String>, String> {
    value.map(|value| string(value, "string")).transpose()
}

#[cfg(all(feature = "json", not(feature = "serde")))]
fn required_string(value: Option<&JsonValue>, name: &str) -> Result<String, String> {
    value
        .ok_or_else(|| format!("missing required field {name}"))
        .and_then(|value| string(value, name))
}

#[cfg(all(feature = "json", not(feature = "serde")))]
fn string(value: &JsonValue, name: &str) -> Result<String, String> {
    value
        .as_str()
        .map(ToString::to_string)
        .ok_or_else(|| format!("{name} must be a string"))
}

#[cfg(all(feature = "json", not(feature = "serde")))]
fn optional_bool(value: Option<&JsonValue>) -> Result<Option<bool>, String> {
    value
        .map(|value| {
            value
                .as_bool()
                .ok_or_else(|| "field must be a boolean".to_string())
        })
        .transpose()
}

#[cfg(all(feature = "json", not(feature = "serde")))]
fn optional_i32(value: Option<&JsonValue>) -> Result<Option<i32>, String> {
    value.map(|value| i32_value(value, "field")).transpose()
}

#[cfg(all(feature = "json", not(feature = "serde")))]
fn optional_i64(value: Option<&JsonValue>) -> Result<Option<i64>, String> {
    value.map(|value| i64_value(value, "field")).transpose()
}

#[cfg(all(feature = "json", not(feature = "serde")))]
fn required_i64(value: Option<&JsonValue>, name: &str) -> Result<i64, String> {
    value
        .ok_or_else(|| format!("missing required field {name}"))
        .and_then(|value| i64_value(value, name))
}

#[cfg(all(feature = "json", not(feature = "serde")))]
fn optional_u32(value: Option<&JsonValue>) -> Result<Option<u32>, String> {
    value.map(|value| u32_value(value, "field")).transpose()
}

#[cfg(all(feature = "json", not(feature = "serde")))]
fn required_u32(value: Option<&JsonValue>, name: &str) -> Result<u32, String> {
    value
        .ok_or_else(|| format!("missing required field {name}"))
        .and_then(|value| u32_value(value, name))
}

#[cfg(all(feature = "json", not(feature = "serde")))]
fn optional_u64(value: Option<&JsonValue>) -> Result<Option<u64>, String> {
    value.map(|value| u64_value(value, "field")).transpose()
}

#[cfg(all(feature = "json", not(feature = "serde")))]
fn required_u64(value: Option<&JsonValue>, name: &str) -> Result<u64, String> {
    value
        .ok_or_else(|| format!("missing required field {name}"))
        .and_then(|value| u64_value(value, name))
}

#[cfg(all(feature = "json", not(feature = "serde")))]
fn i32_value(value: &JsonValue, name: &str) -> Result<i32, String> {
    let value = i64_value(value, name)?;
    i32::try_from(value).map_err(|_| format!("{name} is out of range for i32"))
}

#[cfg(all(feature = "json", not(feature = "serde")))]
fn i64_value(value: &JsonValue, name: &str) -> Result<i64, String> {
    value
        .as_i64()
        .or_else(|| value.as_u64().and_then(|v| i64::try_from(v).ok()))
        .ok_or_else(|| format!("{name} must be an integer"))
}

#[cfg(all(feature = "json", not(feature = "serde")))]
fn u32_value(value: &JsonValue, name: &str) -> Result<u32, String> {
    let value = u64_value(value, name)?;
    u32::try_from(value).map_err(|_| format!("{name} is out of range for u32"))
}

#[cfg(all(feature = "json", not(feature = "serde")))]
fn u64_value(value: &JsonValue, name: &str) -> Result<u64, String> {
    value
        .as_u64()
        .or_else(|| value.as_i64().and_then(|v| u64::try_from(v).ok()))
        .ok_or_else(|| format!("{name} must be an unsigned integer"))
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(all(
    test,
    not(target_os = "none"),
    feature = "json",
    not(feature = "serde")
))]
mod json_feature_tests {
    use super::*;

    #[test]
    fn parse_minimal_oci_spec_without_serde() {
        let json =
            r#"{"ociVersion":"1.0.2","root":{"path":"rootfs"},"process":{"args":["/bin/sh"]}}"#;

        let spec = parse_oci_spec(json.as_bytes()).unwrap();

        assert_eq!(spec.version, "1.0.2");
        assert_eq!(spec.root.unwrap().path, "rootfs");
        assert_eq!(spec.process.unwrap().args.unwrap(), vec!["/bin/sh"]);
    }

    #[test]
    fn parse_runtime_fields_without_serde() {
        let json = r#"{
            "ociVersion": "1.0.2",
            "hostname": "edgerun",
            "process": {
                "terminal": true,
                "args": ["/init", "--boot"],
                "env": ["PATH=/bin", "TERM=xterm"],
                "cwd": "/",
                "user": { "uid": 1000, "gid": 1000, "additionalGids": [10, 11] },
                "capabilities": { "bounding": ["CAP_CHOWN"], "effective": ["CAP_CHOWN"] },
                "rlimits": [{ "type": "RLIMIT_NOFILE", "hard": 1024, "soft": 512 }],
                "noNewPrivileges": true
            },
            "root": { "path": "/rootfs", "readonly": true },
            "mounts": [{ "destination": "/proc", "type": "proc", "source": "proc", "options": ["nosuid"] }],
            "linux": {
                "namespaces": [{ "type": "mount" }, { "type": "pid" }],
                "uidMappings": [{ "containerID": 0, "hostID": 100000, "size": 65536 }],
                "maskedPaths": ["/proc/kcore"],
                "readonlyPaths": ["/proc/sys"],
                "sysctl": { "net.ipv4.ip_forward": "1" },
                "resources": {
                    "memory": { "limit": 1048576 },
                    "cpu": { "shares": 1024, "cpus": "0" },
                    "pids": { "limit": 64 },
                    "devices": [{ "type": "c", "major": 1, "minor": 3, "access": "rwm" }]
                }
            }
        }"#;

        let spec = parse_oci_spec(json.as_bytes()).unwrap();
        let process = spec.process.unwrap();
        assert_eq!(process.terminal, Some(true));
        assert_eq!(process.user.unwrap().uid, Some(1000));
        assert_eq!(process.rlimits.unwrap()[0].soft, 512);

        let linux = spec.linux.unwrap();
        assert_eq!(linux.namespaces.unwrap()[1].ns_type, "pid");
        assert_eq!(linux.uid_mappings.unwrap()[0].host_id, 100000);
        assert_eq!(
            linux.sysctl.unwrap().get("net.ipv4.ip_forward"),
            Some(&"1".into())
        );
        let resources = linux.resources.unwrap();
        assert_eq!(resources.memory.unwrap().limit, Some(1048576));
        assert_eq!(resources.cpu.unwrap().cpus, Some("0".into()));
        assert_eq!(resources.pids.unwrap().limit, 64);
        assert_eq!(resources.devices.unwrap()[0].access, Some("rwm".into()));

        let mounts = spec.mounts.unwrap();
        assert_eq!(mounts[0].destination, "/proc");
        assert_eq!(mounts[0].options.as_ref().unwrap()[0], "nosuid");
    }

    #[test]
    fn parse_oci_spec_without_serde_rejects_missing_version() {
        assert!(parse_oci_spec(br#"{"root":{"path":"rootfs"}}"#).is_err());
    }
}

#[cfg(all(test, not(target_os = "none"), feature = "serde"))]
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
            annotations: Some(BTreeMap::from([(
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
            os: Some(crate::validate::host_os().into()),
            arch: Some(crate::validate::host_arch().into()),
            ..Default::default()
        };
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
