//! OCI spec types with edgerun-json serialization.

use crate::prelude::*;
use crate::util::StringResultExt;
use edgerun_json::{FromJson, JsonValue, JsonValueError, Map, ToJson};

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

fn object(value: JsonValue, name: &str) -> Result<Map, JsonValueError> {
    match value {
        JsonValue::Object(object) => Ok(object),
        other => Err(JsonValueError::WrongType(format!(
            "{name} must be an object, found {other:?}"
        ))),
    }
}

fn take_optional<T: FromJson>(object: &mut Map, key: &str) -> Result<Option<T>, JsonValueError> {
    object
        .remove(key)
        .map(|value| match value {
            JsonValue::Null => Ok(None),
            value => T::from_json(value).map(Some),
        })
        .transpose()
        .map(Option::flatten)
}

fn take_optional_any<T: FromJson>(
    object: &mut Map,
    keys: &[&str],
) -> Result<Option<T>, JsonValueError> {
    for key in keys {
        if object.contains_key(key) {
            return take_optional(object, key);
        }
    }
    Ok(None)
}

fn take_required<T: FromJson>(object: &mut Map, key: &str) -> Result<T, JsonValueError> {
    let value = object
        .remove(key)
        .ok_or_else(|| JsonValueError::WrongType(format!("missing required field `{key}`")))?;
    T::from_json(value)
}

macro_rules! impl_to_json {
    ($($ty:ty => $func:ident),* $(,)?) => {
        $(
            impl ToJson for $ty {
                fn to_json(&self) -> JsonValue {
                    $func(self)
                }
            }
        )*
    };
}

impl FromJson for OciSpec {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = object(value, "OCI spec")?;
        Ok(Self {
            version: take_required(&mut object, "ociVersion")?,
            platform: take_optional(&mut object, "platform")?,
            process: take_optional(&mut object, "process")?,
            root: take_optional(&mut object, "root")?,
            hostname: take_optional(&mut object, "hostname")?,
            domainname: take_optional(&mut object, "domainname")?,
            linux: take_optional(&mut object, "linux")?,
            mounts: take_optional(&mut object, "mounts")?,
            annotations: take_optional(&mut object, "annotations")?,
        })
    }
}

impl FromJson for OciPlatform {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = object(value, "platform")?;
        Ok(Self {
            os: take_optional(&mut object, "os")?,
            arch: take_optional(&mut object, "arch")?,
            os_version: take_optional(&mut object, "os.version")?,
            os_features: take_optional(&mut object, "os.features")?,
        })
    }
}

impl FromJson for OciProcess {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = object(value, "process")?;
        Ok(Self {
            terminal: take_optional(&mut object, "terminal")?,
            user: take_optional(&mut object, "user")?,
            console_size: take_optional(&mut object, "consoleSize")?,
            args: take_optional(&mut object, "args")?,
            env: take_optional(&mut object, "env")?,
            cwd: take_optional(&mut object, "cwd")?,
            capabilities: take_optional(&mut object, "capabilities")?,
            rlimits: take_optional(&mut object, "rlimits")?,
            no_new_privileges: take_optional(&mut object, "noNewPrivileges")?,
            oom_score_adj: take_optional(&mut object, "oomScoreAdj")?,
            apparmor_profile: take_optional(&mut object, "apparmorProfile")?,
            selinux_label: take_optional(&mut object, "selinuxLabel")?,
            scheduler: take_optional(&mut object, "scheduler")?,
            io_priority: take_optional(&mut object, "ioPriority")?,
        })
    }
}

impl FromJson for OciBox {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = object(value, "box")?;
        Ok(Self {
            width: take_required(&mut object, "width")?,
            height: take_required(&mut object, "height")?,
        })
    }
}

impl FromJson for OciUser {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = object(value, "user")?;
        Ok(Self {
            uid: take_optional(&mut object, "uid")?,
            gid: take_optional(&mut object, "gid")?,
            additional_gids: take_optional(&mut object, "additionalGids")?,
            umask: take_optional(&mut object, "umask")?,
        })
    }
}

impl FromJson for OciCapabilities {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = object(value, "capabilities")?;
        Ok(Self {
            bounding: take_optional(&mut object, "bounding")?,
            effective: take_optional(&mut object, "effective")?,
            inheritable: take_optional(&mut object, "inheritable")?,
            permitted: take_optional(&mut object, "permitted")?,
            ambient: take_optional(&mut object, "ambient")?,
        })
    }
}

impl FromJson for OciRlimit {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = object(value, "rlimit")?;
        Ok(Self {
            ns_type: take_required(&mut object, "type")?,
            hard: take_required(&mut object, "hard")?,
            soft: take_required(&mut object, "soft")?,
        })
    }
}

impl FromJson for OciRoot {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = object(value, "root")?;
        Ok(Self {
            path: take_required(&mut object, "path")?,
            readonly: take_optional(&mut object, "readonly")?,
        })
    }
}

impl FromJson for OciScheduler {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = object(value, "scheduler")?;
        Ok(Self {
            policy: take_required(&mut object, "policy")?,
            nice: take_optional(&mut object, "nice")?,
            priority: take_optional(&mut object, "priority")?,
            deadline: take_optional(&mut object, "deadline")?,
        })
    }
}

impl FromJson for OciSchedDeadline {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = object(value, "scheduler deadline")?;
        Ok(Self {
            runtime_ns: take_optional(&mut object, "runtime")?,
            period_ns: take_optional(&mut object, "period")?,
            deadline_ns: take_optional(&mut object, "deadline")?,
        })
    }
}

impl FromJson for OciIoPriority {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = object(value, "ioPriority")?;
        Ok(Self {
            class: take_required(&mut object, "class")?,
            priority: take_optional(&mut object, "priority")?,
        })
    }
}

impl FromJson for OciLinux {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = object(value, "linux")?;
        Ok(Self {
            uid_mappings: take_optional(&mut object, "uidMappings")?,
            gid_mappings: take_optional(&mut object, "gidMappings")?,
            resources: take_optional(&mut object, "resources")?,
            cgroups_path: take_optional(&mut object, "cgroupsPath")?,
            namespaces: take_optional(&mut object, "namespaces")?,
            devices: take_optional(&mut object, "devices")?,
            masked_paths: take_optional(&mut object, "maskedPaths")?,
            readonly_paths: take_optional(&mut object, "readonlyPaths")?,
            mount_label: take_optional(&mut object, "mountLabel")?,
            rootfs_propagation: take_optional(&mut object, "rootfsPropagation")?,
            sysctl: take_optional(&mut object, "sysctl")?,
            hooks: take_optional(&mut object, "hooks")?,
            seccomp: take_optional(&mut object, "seccomp")?,
            intel_rdt: take_optional(&mut object, "intelRdt")?,
        })
    }
}

impl FromJson for OciHooks {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = object(value, "hooks")?;
        Ok(Self {
            prestart: take_optional(&mut object, "prestart")?,
            create_runtime: take_optional(&mut object, "createRuntime")?,
            create_container: take_optional(&mut object, "createContainer")?,
            start_container: take_optional(&mut object, "startContainer")?,
            poststart: take_optional(&mut object, "poststart")?,
            poststop: take_optional(&mut object, "poststop")?,
        })
    }
}

impl FromJson for OciHook {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = object(value, "hook")?;
        Ok(Self {
            path: take_required(&mut object, "path")?,
            args: take_optional(&mut object, "args")?,
            env: take_optional(&mut object, "env")?,
            timeout: take_optional(&mut object, "timeout")?,
        })
    }
}

impl FromJson for OciIdMapping {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = object(value, "id mapping")?;
        Ok(Self {
            container_id: take_required(&mut object, "containerID")?,
            host_id: take_required(&mut object, "hostID")?,
            size: take_required(&mut object, "size")?,
        })
    }
}

impl FromJson for OciNamespace {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = object(value, "namespace")?;
        Ok(Self {
            ns_type: take_required(&mut object, "type")?,
            path: take_optional(&mut object, "path")?,
        })
    }
}

impl FromJson for OciLinuxDevice {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = object(value, "linux device")?;
        Ok(Self {
            ns_type: take_required(&mut object, "type")?,
            path: take_required(&mut object, "path")?,
            file_mode: take_optional(&mut object, "fileMode")?,
            uid: take_optional(&mut object, "uid")?,
            gid: take_optional(&mut object, "gid")?,
            major: take_optional(&mut object, "major")?,
            minor: take_optional(&mut object, "minor")?,
        })
    }
}

impl FromJson for OciLinuxCpu {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = object(value, "linux cpu")?;
        Ok(Self {
            shares: take_optional(&mut object, "shares")?,
            quota: take_optional(&mut object, "quota")?,
            period: take_optional(&mut object, "period")?,
            realtime_runtime: take_optional(&mut object, "realtimeRuntime")?,
            realtime_period: take_optional(&mut object, "realtimePeriod")?,
            cpus: take_optional(&mut object, "cpus")?,
            mems: take_optional(&mut object, "mems")?,
            idle: take_optional(&mut object, "idle")?,
            burst: take_optional(&mut object, "burst")?,
        })
    }
}

impl FromJson for OciLinuxBlockIO {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = object(value, "blockIO")?;
        Ok(Self {
            weight: take_optional(&mut object, "weight")?,
            leaf_weight: take_optional(&mut object, "leafWeight")?,
            weight_device: take_optional(&mut object, "weightDevice")?,
            leaf_weight_device: take_optional(&mut object, "leafWeightDevice")?,
            throttle_read_bps_device: take_optional(&mut object, "throttleReadBpsDevice")?,
            throttle_write_bps_device: take_optional(&mut object, "throttleWriteBpsDevice")?,
            throttle_read_iops_device: take_optional(&mut object, "throttleReadIOPSDevice")?,
            throttle_write_iops_device: take_optional(&mut object, "throttleWriteIOPSDevice")?,
        })
    }
}

impl FromJson for OciLinuxWeightDevice {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = object(value, "weight device")?;
        Ok(Self {
            major: take_required(&mut object, "major")?,
            minor: take_required(&mut object, "minor")?,
            weight: take_optional(&mut object, "weight")?,
            leaf_weight: take_optional(&mut object, "leafWeight")?,
        })
    }
}

impl FromJson for OciLinuxThrottleDevice {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = object(value, "throttle device")?;
        Ok(Self {
            major: take_required(&mut object, "major")?,
            minor: take_required(&mut object, "minor")?,
            rate: take_required(&mut object, "rate")?,
        })
    }
}

impl FromJson for OciLinuxHugepageLimit {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = object(value, "hugepage limit")?;
        Ok(Self {
            pagesize: take_required(&mut object, "pagesize")?,
            limit: take_required(&mut object, "limit")?,
            rsvd: take_optional(&mut object, "rsvd")?,
        })
    }
}

impl FromJson for OciLinuxNetwork {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = object(value, "network")?;
        Ok(Self {
            class_id: take_optional(&mut object, "classID")?,
            priorities: take_optional(&mut object, "priorities")?,
        })
    }
}

impl FromJson for OciLinuxNetworkPriority {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = object(value, "network priority")?;
        Ok(Self {
            name: take_required(&mut object, "name")?,
            priority: take_required(&mut object, "priority")?,
        })
    }
}

impl FromJson for OciLinuxIntelRdt {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = object(value, "intelRdt")?;
        Ok(Self {
            l3_cache_schema: take_optional(&mut object, "l3CacheSchema")?,
            mem_bw_schema: take_optional(&mut object, "memBwSchema")?,
            clos_id: take_optional(&mut object, "closID")?,
            enable_monitoring: take_optional(&mut object, "enableMonitoring")?,
            schemata: take_optional(&mut object, "schemata")?,
        })
    }
}

impl FromJson for OciSeccompAction {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let action = String::from_json(value)?;
        Self::try_from(action).map_err(JsonValueError::WrongType)
    }
}

impl FromJson for OciLinuxSeccomp {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = object(value, "seccomp")?;
        Ok(Self {
            default_action: take_optional(&mut object, "defaultAction")?,
            default_errno_ret: take_optional(&mut object, "defaultErrnoRet")?,
            architectures: take_optional(&mut object, "architectures")?,
            listener_path: take_optional(&mut object, "listenerPath")?,
            listener_metadata: take_optional(&mut object, "listenerMetadata")?,
            syscalls: take_optional(&mut object, "syscalls")?,
        })
    }
}

impl FromJson for OciSeccompSyscallEntry {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = object(value, "seccomp syscall")?;
        Ok(Self {
            names: take_optional(&mut object, "names")?,
            action: take_optional(&mut object, "action")?,
            errno_ret: take_optional(&mut object, "errnoRet")?,
            args: take_optional(&mut object, "args")?,
        })
    }
}

impl FromJson for OciSeccompArg {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = object(value, "seccomp arg")?;
        Ok(Self {
            index: take_required(&mut object, "index")?,
            value: take_required(&mut object, "value")?,
            value_two: take_required(&mut object, "valueTwo")?,
            op: take_required(&mut object, "op")?,
        })
    }
}

impl FromJson for OciLinuxResources {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = object(value, "resources")?;
        Ok(Self {
            devices: take_optional(&mut object, "devices")?,
            memory: take_optional(&mut object, "memory")?,
            cpu: take_optional(&mut object, "cpu")?,
            pids: take_optional(&mut object, "pids")?,
            block_io: take_optional(&mut object, "blockIO")?,
            hugepage_limits: take_optional_any(
                &mut object,
                &["hugepageLimits", "hugepage_limits"],
            )?,
            network: take_optional(&mut object, "network")?,
        })
    }
}

impl FromJson for OciLinuxDeviceCgroup {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = object(value, "device cgroup")?;
        Ok(Self {
            ns_type: take_required(&mut object, "type")?,
            major: take_optional(&mut object, "major")?,
            minor: take_optional(&mut object, "minor")?,
            access: take_optional(&mut object, "access")?,
        })
    }
}

impl FromJson for OciLinuxPids {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = object(value, "pids")?;
        Ok(Self {
            limit: take_required(&mut object, "limit")?,
        })
    }
}

impl FromJson for OciLinuxMemory {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = object(value, "memory")?;
        Ok(Self {
            limit: take_optional(&mut object, "limit")?,
            reservation: take_optional(&mut object, "reservation")?,
            swap: take_optional(&mut object, "swap")?,
            kernel: take_optional(&mut object, "kernel")?,
            kernel_tcp: take_optional(&mut object, "kernelTCP")?,
            check_before_update: take_optional(&mut object, "checkBeforeUpdate")?,
        })
    }
}

impl FromJson for OciMount {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = object(value, "mount")?;
        Ok(Self {
            destination: take_required(&mut object, "destination")?,
            mount_type: take_optional(&mut object, "type")?,
            source: take_optional(&mut object, "source")?,
            options: take_optional(&mut object, "options")?,
            label: take_optional(&mut object, "label")?,
            recursive: take_optional(&mut object, "recursive")?,
            uid_mappings: take_optional(&mut object, "uidMappings")?,
            gid_mappings: take_optional(&mut object, "gidMappings")?,
        })
    }
}

fn oci_spec_to_value(spec: &OciSpec) -> edgerun_json::JsonValue {
    let mut object = edgerun_json::Map::new();
    object.push_field("ociVersion", spec.version.clone());
    object.push_opt_field("platform", spec.platform.as_ref().map(platform_to_value));
    object.push_opt_field("process", spec.process.as_ref().map(process_to_value));
    object.push_opt_field("root", spec.root.as_ref().map(root_to_value));
    object.push_opt_field("hostname", spec.hostname.as_deref());
    object.push_opt_field("domainname", spec.domainname.as_deref());
    object.push_opt_field("linux", spec.linux.as_ref().map(linux_to_value));
    object.push_opt_field(
        "mounts",
        spec.mounts.as_ref().map(array_to_value(mount_to_value)),
    );
    object.push_opt_field(
        "annotations",
        spec.annotations.as_ref().map(string_map_to_value),
    );
    object.into()
}

fn platform_to_value(platform: &OciPlatform) -> edgerun_json::JsonValue {
    let mut object = edgerun_json::Map::new();
    object.push_opt_field("os", platform.os.as_deref());
    object.push_opt_field("arch", platform.arch.as_deref());
    object.push_opt_field("os.version", platform.os_version.as_deref());
    object.push_opt_field(
        "os.features",
        platform.os_features.as_ref().map(strings_to_value),
    );
    object.into()
}

fn process_to_value(process: &OciProcess) -> edgerun_json::JsonValue {
    let mut object = edgerun_json::Map::new();
    object.push_opt_field("terminal", process.terminal);
    object.push_opt_field("user", process.user.as_ref().map(user_to_value));
    object.push_opt_field(
        "consoleSize",
        process.console_size.as_ref().map(box_to_value),
    );
    object.push_opt_field("args", process.args.as_ref().map(strings_to_value));
    object.push_opt_field("env", process.env.as_ref().map(strings_to_value));
    object.push_opt_field("cwd", process.cwd.as_deref());
    object.push_opt_field(
        "capabilities",
        process.capabilities.as_ref().map(capabilities_to_value),
    );
    object.push_opt_field(
        "rlimits",
        process
            .rlimits
            .as_ref()
            .map(array_to_value(rlimit_to_value)),
    );
    object.push_opt_field("noNewPrivileges", process.no_new_privileges);
    object.push_opt_field("oomScoreAdj", process.oom_score_adj);
    object.push_opt_field("apparmorProfile", process.apparmor_profile.as_deref());
    object.push_opt_field("selinuxLabel", process.selinux_label.as_deref());
    object.push_opt_field(
        "scheduler",
        process.scheduler.as_ref().map(scheduler_to_value),
    );
    object.push_opt_field(
        "ioPriority",
        process.io_priority.as_ref().map(io_priority_to_value),
    );
    object.into()
}

fn box_to_value(value: &OciBox) -> edgerun_json::JsonValue {
    let mut object = edgerun_json::Map::new();
    object.push_field("width", value.width);
    object.push_field("height", value.height);
    object.into()
}

fn user_to_value(user: &OciUser) -> edgerun_json::JsonValue {
    let mut object = edgerun_json::Map::new();
    object.push_opt_field("uid", user.uid);
    object.push_opt_field("gid", user.gid);
    object.push_opt_field(
        "additionalGids",
        user.additional_gids
            .as_ref()
            .map(array_to_value(|value| *value)),
    );
    object.push_opt_field("umask", user.umask);
    object.into()
}

fn capabilities_to_value(caps: &OciCapabilities) -> edgerun_json::JsonValue {
    let mut object = edgerun_json::Map::new();
    object.push_opt_field("bounding", caps.bounding.as_ref().map(strings_to_value));
    object.push_opt_field("effective", caps.effective.as_ref().map(strings_to_value));
    object.push_opt_field(
        "inheritable",
        caps.inheritable.as_ref().map(strings_to_value),
    );
    object.push_opt_field("permitted", caps.permitted.as_ref().map(strings_to_value));
    object.push_opt_field("ambient", caps.ambient.as_ref().map(strings_to_value));
    object.into()
}

fn rlimit_to_value(rlimit: &OciRlimit) -> edgerun_json::JsonValue {
    let mut object = edgerun_json::Map::new();
    object.push_field("type", rlimit.ns_type.clone());
    object.push_field("hard", rlimit.hard);
    object.push_field("soft", rlimit.soft);
    object.into()
}

fn scheduler_to_value(scheduler: &OciScheduler) -> edgerun_json::JsonValue {
    let mut object = edgerun_json::Map::new();
    object.push_field("policy", scheduler.policy.clone());
    object.push_opt_field("nice", scheduler.nice);
    object.push_opt_field("priority", scheduler.priority);
    object.push_opt_field(
        "deadline",
        scheduler.deadline.as_ref().map(sched_deadline_to_value),
    );
    object.into()
}

fn sched_deadline_to_value(deadline: &OciSchedDeadline) -> edgerun_json::JsonValue {
    let mut object = edgerun_json::Map::new();
    object.push_opt_field("runtime", deadline.runtime_ns);
    object.push_opt_field("period", deadline.period_ns);
    object.push_opt_field("deadline", deadline.deadline_ns);
    object.into()
}

fn io_priority_to_value(ioprio: &OciIoPriority) -> edgerun_json::JsonValue {
    let mut object = edgerun_json::Map::new();
    object.push_field("class", ioprio.class);
    object.push_opt_field("priority", ioprio.priority);
    object.into()
}

fn root_to_value(root: &OciRoot) -> edgerun_json::JsonValue {
    let mut object = edgerun_json::Map::new();
    object.push_field("path", root.path.clone());
    object.push_opt_field("readonly", root.readonly);
    object.into()
}

fn linux_to_value(linux: &OciLinux) -> edgerun_json::JsonValue {
    let mut object = edgerun_json::Map::new();
    object.push_opt_field(
        "uidMappings",
        linux
            .uid_mappings
            .as_ref()
            .map(array_to_value(id_mapping_to_value)),
    );
    object.push_opt_field(
        "gidMappings",
        linux
            .gid_mappings
            .as_ref()
            .map(array_to_value(id_mapping_to_value)),
    );
    object.push_opt_field(
        "resources",
        linux.resources.as_ref().map(resources_to_value),
    );
    object.push_opt_field("cgroupsPath", linux.cgroups_path.as_deref());
    object.push_opt_field(
        "namespaces",
        linux
            .namespaces
            .as_ref()
            .map(array_to_value(namespace_to_value)),
    );
    object.push_opt_field(
        "devices",
        linux.devices.as_ref().map(array_to_value(device_to_value)),
    );
    object.push_opt_field(
        "maskedPaths",
        linux.masked_paths.as_ref().map(strings_to_value),
    );
    object.push_opt_field(
        "readonlyPaths",
        linux.readonly_paths.as_ref().map(strings_to_value),
    );
    object.push_opt_field("mountLabel", linux.mount_label.as_deref());
    object.push_opt_field("rootfsPropagation", linux.rootfs_propagation.as_deref());
    object.push_opt_field("sysctl", linux.sysctl.as_ref().map(string_map_to_value));
    object.push_opt_field("hooks", linux.hooks.as_ref().map(hooks_to_value));
    object.push_opt_field("seccomp", linux.seccomp.as_ref().map(seccomp_to_value));
    object.push_opt_field("intelRdt", linux.intel_rdt.as_ref().map(intel_rdt_to_value));
    object.into()
}

fn hooks_to_value(hooks: &OciHooks) -> edgerun_json::JsonValue {
    let mut object = edgerun_json::Map::new();
    object.push_opt_field(
        "prestart",
        hooks.prestart.as_ref().map(array_to_value(hook_to_value)),
    );
    object.push_opt_field(
        "createRuntime",
        hooks
            .create_runtime
            .as_ref()
            .map(array_to_value(hook_to_value)),
    );
    object.push_opt_field(
        "createContainer",
        hooks
            .create_container
            .as_ref()
            .map(array_to_value(hook_to_value)),
    );
    object.push_opt_field(
        "startContainer",
        hooks
            .start_container
            .as_ref()
            .map(array_to_value(hook_to_value)),
    );
    object.push_opt_field(
        "poststart",
        hooks.poststart.as_ref().map(array_to_value(hook_to_value)),
    );
    object.push_opt_field(
        "poststop",
        hooks.poststop.as_ref().map(array_to_value(hook_to_value)),
    );
    object.into()
}

fn hook_to_value(hook: &OciHook) -> edgerun_json::JsonValue {
    let mut object = edgerun_json::Map::new();
    object.push_field("path", hook.path.clone());
    object.push_opt_field("args", hook.args.as_ref().map(strings_to_value));
    object.push_opt_field("env", hook.env.as_ref().map(strings_to_value));
    object.push_opt_field("timeout", hook.timeout);
    object.into()
}

fn id_mapping_to_value(mapping: &OciIdMapping) -> edgerun_json::JsonValue {
    let mut object = edgerun_json::Map::new();
    object.push_field("containerID", mapping.container_id);
    object.push_field("hostID", mapping.host_id);
    object.push_field("size", mapping.size);
    object.into()
}

fn namespace_to_value(namespace: &OciNamespace) -> edgerun_json::JsonValue {
    let mut object = edgerun_json::Map::new();
    object.push_field("type", namespace.ns_type.clone());
    object.push_opt_field("path", namespace.path.as_deref());
    object.into()
}

fn device_to_value(device: &OciLinuxDevice) -> edgerun_json::JsonValue {
    let mut object = edgerun_json::Map::new();
    object.push_field("type", device.ns_type.clone());
    object.push_field("path", device.path.clone());
    object.push_opt_field("fileMode", device.file_mode);
    object.push_opt_field("uid", device.uid);
    object.push_opt_field("gid", device.gid);
    object.push_opt_field("major", device.major);
    object.push_opt_field("minor", device.minor);
    object.into()
}

fn resources_to_value(resources: &OciLinuxResources) -> edgerun_json::JsonValue {
    let mut object = edgerun_json::Map::new();
    object.push_opt_field(
        "devices",
        resources
            .devices
            .as_ref()
            .map(array_to_value(device_cgroup_to_value)),
    );
    object.push_opt_field("memory", resources.memory.as_ref().map(memory_to_value));
    object.push_opt_field("cpu", resources.cpu.as_ref().map(cpu_to_value));
    object.push_opt_field("pids", resources.pids.as_ref().map(pids_to_value));
    object.push_opt_field(
        "blockIO",
        resources.block_io.as_ref().map(block_io_to_value),
    );
    object.push_opt_field(
        "hugepageLimits",
        resources
            .hugepage_limits
            .as_ref()
            .map(array_to_value(hugepage_limit_to_value)),
    );
    object.push_opt_field("network", resources.network.as_ref().map(network_to_value));
    object.into()
}

fn device_cgroup_to_value(device: &OciLinuxDeviceCgroup) -> edgerun_json::JsonValue {
    let mut object = edgerun_json::Map::new();
    object.push_field("type", device.ns_type.clone());
    object.push_opt_field("major", device.major);
    object.push_opt_field("minor", device.minor);
    object.push_opt_field("access", device.access.as_deref());
    object.into()
}

fn memory_to_value(memory: &OciLinuxMemory) -> edgerun_json::JsonValue {
    let mut object = edgerun_json::Map::new();
    object.push_opt_field("limit", memory.limit);
    object.push_opt_field("reservation", memory.reservation);
    object.push_opt_field("swap", memory.swap);
    object.push_opt_field("kernel", memory.kernel);
    object.push_opt_field("kernelTCP", memory.kernel_tcp);
    object.push_opt_field("checkBeforeUpdate", memory.check_before_update);
    object.into()
}

fn cpu_to_value(cpu: &OciLinuxCpu) -> edgerun_json::JsonValue {
    let mut object = edgerun_json::Map::new();
    object.push_opt_field("shares", cpu.shares);
    object.push_opt_field("quota", cpu.quota);
    object.push_opt_field("period", cpu.period);
    object.push_opt_field("realtimeRuntime", cpu.realtime_runtime);
    object.push_opt_field("realtimePeriod", cpu.realtime_period);
    object.push_opt_field("cpus", cpu.cpus.as_deref());
    object.push_opt_field("mems", cpu.mems.as_deref());
    object.push_opt_field("idle", cpu.idle);
    object.push_opt_field("burst", cpu.burst);
    object.into()
}

fn pids_to_value(pids: &OciLinuxPids) -> edgerun_json::JsonValue {
    let mut object = edgerun_json::Map::new();
    object.push_field("limit", pids.limit);
    object.into()
}

fn block_io_to_value(block_io: &OciLinuxBlockIO) -> edgerun_json::JsonValue {
    let mut object = edgerun_json::Map::new();
    object.push_opt_field("weight", block_io.weight);
    object.push_opt_field("leafWeight", block_io.leaf_weight);
    object.push_opt_field(
        "weightDevice",
        block_io
            .weight_device
            .as_ref()
            .map(array_to_value(weight_device_to_value)),
    );
    object.push_opt_field(
        "leafWeightDevice",
        block_io
            .leaf_weight_device
            .as_ref()
            .map(array_to_value(weight_device_to_value)),
    );
    object.push_opt_field(
        "throttleReadBpsDevice",
        block_io
            .throttle_read_bps_device
            .as_ref()
            .map(array_to_value(throttle_device_to_value)),
    );
    object.push_opt_field(
        "throttleWriteBpsDevice",
        block_io
            .throttle_write_bps_device
            .as_ref()
            .map(array_to_value(throttle_device_to_value)),
    );
    object.push_opt_field(
        "throttleReadIOPSDevice",
        block_io
            .throttle_read_iops_device
            .as_ref()
            .map(array_to_value(throttle_device_to_value)),
    );
    object.push_opt_field(
        "throttleWriteIOPSDevice",
        block_io
            .throttle_write_iops_device
            .as_ref()
            .map(array_to_value(throttle_device_to_value)),
    );
    object.into()
}

fn weight_device_to_value(device: &OciLinuxWeightDevice) -> edgerun_json::JsonValue {
    let mut object = edgerun_json::Map::new();
    object.push_field("major", device.major);
    object.push_field("minor", device.minor);
    object.push_opt_field("weight", device.weight);
    object.push_opt_field("leafWeight", device.leaf_weight);
    object.into()
}

fn throttle_device_to_value(device: &OciLinuxThrottleDevice) -> edgerun_json::JsonValue {
    let mut object = edgerun_json::Map::new();
    object.push_field("major", device.major);
    object.push_field("minor", device.minor);
    object.push_field("rate", device.rate);
    object.into()
}

fn hugepage_limit_to_value(limit: &OciLinuxHugepageLimit) -> edgerun_json::JsonValue {
    let mut object = edgerun_json::Map::new();
    object.push_field("pagesize", limit.pagesize.clone());
    object.push_field("limit", limit.limit);
    object.push_opt_field("rsvd", limit.rsvd);
    object.into()
}

fn network_to_value(network: &OciLinuxNetwork) -> edgerun_json::JsonValue {
    let mut object = edgerun_json::Map::new();
    object.push_opt_field("classID", network.class_id);
    object.push_opt_field(
        "priorities",
        network
            .priorities
            .as_ref()
            .map(array_to_value(network_priority_to_value)),
    );
    object.into()
}

fn network_priority_to_value(priority: &OciLinuxNetworkPriority) -> edgerun_json::JsonValue {
    let mut object = edgerun_json::Map::new();
    object.push_field("name", priority.name.clone());
    object.push_field("priority", priority.priority);
    object.into()
}

fn intel_rdt_to_value(rdt: &OciLinuxIntelRdt) -> edgerun_json::JsonValue {
    let mut object = edgerun_json::Map::new();
    object.push_opt_field("l3CacheSchema", rdt.l3_cache_schema.as_deref());
    object.push_opt_field("memBwSchema", rdt.mem_bw_schema.as_deref());
    object.push_opt_field("closID", rdt.clos_id.as_deref());
    object.push_opt_field("enableMonitoring", rdt.enable_monitoring);
    object.push_opt_field("schemata", rdt.schemata.as_deref());
    object.into()
}

fn seccomp_to_value(seccomp: &OciLinuxSeccomp) -> edgerun_json::JsonValue {
    let mut object = edgerun_json::Map::new();
    object.push_opt_field(
        "defaultAction",
        seccomp.default_action.as_ref().map(seccomp_action_to_value),
    );
    object.push_opt_field("defaultErrnoRet", seccomp.default_errno_ret);
    object.push_opt_field(
        "architectures",
        seccomp.architectures.as_ref().map(strings_to_value),
    );
    object.push_opt_field("listenerPath", seccomp.listener_path.as_deref());
    object.push_opt_field("listenerMetadata", seccomp.listener_metadata.as_deref());
    object.push_opt_field(
        "syscalls",
        seccomp
            .syscalls
            .as_ref()
            .map(array_to_value(seccomp_syscall_to_value)),
    );
    object.into()
}

fn seccomp_action_to_value(action: &OciSeccompAction) -> edgerun_json::JsonValue {
    let value: String = action.clone().into();
    value.into()
}

fn seccomp_syscall_to_value(syscall: &OciSeccompSyscallEntry) -> edgerun_json::JsonValue {
    let mut object = edgerun_json::Map::new();
    object.push_opt_field("names", syscall.names.as_ref().map(strings_to_value));
    object.push_opt_field(
        "action",
        syscall.action.as_ref().map(seccomp_action_to_value),
    );
    object.push_opt_field("errnoRet", syscall.errno_ret);
    object.push_opt_field(
        "args",
        syscall
            .args
            .as_ref()
            .map(array_to_value(seccomp_arg_to_value)),
    );
    object.into()
}

fn seccomp_arg_to_value(arg: &OciSeccompArg) -> edgerun_json::JsonValue {
    let mut object = edgerun_json::Map::new();
    object.push_field("index", arg.index);
    object.push_field("value", arg.value);
    object.push_field("valueTwo", arg.value_two);
    object.push_field("op", arg.op.clone());
    object.into()
}

fn mount_to_value(mount: &OciMount) -> edgerun_json::JsonValue {
    let mut object = edgerun_json::Map::new();
    object.push_field("destination", mount.destination.clone());
    object.push_opt_field("type", mount.mount_type.as_deref());
    object.push_opt_field("source", mount.source.as_deref());
    object.push_opt_field("options", mount.options.as_ref().map(strings_to_value));
    object.push_opt_field("label", mount.label.as_deref());
    object.push_opt_field("recursive", mount.recursive);
    object.push_opt_field(
        "uidMappings",
        mount
            .uid_mappings
            .as_ref()
            .map(array_to_value(id_mapping_to_value)),
    );
    object.push_opt_field(
        "gidMappings",
        mount
            .gid_mappings
            .as_ref()
            .map(array_to_value(id_mapping_to_value)),
    );
    object.into()
}

impl_to_json! {
    OciSpec => oci_spec_to_value,
    OciPlatform => platform_to_value,
    OciProcess => process_to_value,
    OciBox => box_to_value,
    OciUser => user_to_value,
    OciCapabilities => capabilities_to_value,
    OciRlimit => rlimit_to_value,
    OciScheduler => scheduler_to_value,
    OciSchedDeadline => sched_deadline_to_value,
    OciIoPriority => io_priority_to_value,
    OciRoot => root_to_value,
    OciLinux => linux_to_value,
    OciHooks => hooks_to_value,
    OciHook => hook_to_value,
    OciIdMapping => id_mapping_to_value,
    OciNamespace => namespace_to_value,
    OciLinuxDevice => device_to_value,
    OciLinuxResources => resources_to_value,
    OciLinuxDeviceCgroup => device_cgroup_to_value,
    OciLinuxMemory => memory_to_value,
    OciLinuxCpu => cpu_to_value,
    OciLinuxPids => pids_to_value,
    OciLinuxBlockIO => block_io_to_value,
    OciLinuxWeightDevice => weight_device_to_value,
    OciLinuxThrottleDevice => throttle_device_to_value,
    OciLinuxHugepageLimit => hugepage_limit_to_value,
    OciLinuxNetwork => network_to_value,
    OciLinuxNetworkPriority => network_priority_to_value,
    OciLinuxIntelRdt => intel_rdt_to_value,
    OciLinuxSeccomp => seccomp_to_value,
    OciSeccompAction => seccomp_action_to_value,
    OciSeccompSyscallEntry => seccomp_syscall_to_value,
    OciSeccompArg => seccomp_arg_to_value,
    OciMount => mount_to_value,
}

fn strings_to_value(values: &Vec<String>) -> edgerun_json::JsonValue {
    edgerun_json::JsonValue::array_from_iter(values.iter())
}

fn string_map_to_value(values: &BTreeMap<String, String>) -> edgerun_json::JsonValue {
    edgerun_json::Map::from_iter(
        values
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str())),
    )
    .into()
}

fn array_to_value<T, V>(
    mut to_value: impl FnMut(&T) -> V,
) -> impl FnMut(&Vec<T>) -> edgerun_json::JsonValue
where
    V: Into<edgerun_json::JsonValue>,
{
    move |values| edgerun_json::JsonValue::array_from_iter(values.iter().map(&mut to_value))
}

#[cfg(all(test, not(target_os = "none")))]
mod edgerun_json_tests {
    use super::*;

    #[test]
    fn parse_minimal_oci_spec_with_edgerun_json() {
        let json =
            r#"{"ociVersion":"1.0.2","root":{"path":"rootfs"},"process":{"args":["/bin/sh"]}}"#;

        let spec = parse_oci_spec(json.as_bytes()).unwrap();

        assert_eq!(spec.version, "1.0.2");
        assert_eq!(spec.root.unwrap().path, "rootfs");
        assert_eq!(spec.process.unwrap().args.unwrap(), vec!["/bin/sh"]);
    }

    #[test]
    fn parse_runtime_fields_with_edgerun_json() {
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
    fn parse_oci_spec_with_edgerun_json_rejects_missing_version() {
        assert!(parse_oci_spec(br#"{"root":{"path":"rootfs"}}"#).is_err());
    }
}
