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
    pub process: Option<OciProcess>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root: Option<OciRoot>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linux: Option<OciLinux>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mounts: Option<Vec<OciMount>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<std::collections::HashMap<String, String>>,
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct OciProcess {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<OciUser>,
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
    #[serde(skip_serializing_if = "Option::is_none", rename = "throttleReadBpsDevice")]
    pub throttle_read_bps_device: Option<Vec<OciLinuxThrottleDevice>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "throttleWriteBpsDevice")]
    pub throttle_write_bps_device: Option<Vec<OciLinuxThrottleDevice>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "throttleReadIOPSDevice")]
    pub throttle_read_iops_device: Option<Vec<OciLinuxThrottleDevice>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "throttleWriteIOPSDevice")]
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

// ===========================================================================
// Seccomp types
// ===========================================================================

/// Seccomp filtering configuration from the OCI spec.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct OciLinuxSeccomp {
    #[serde(skip_serializing_if = "Option::is_none")]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE", try_from = "String", into = "String")]
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
}
