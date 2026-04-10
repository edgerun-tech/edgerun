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
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OciLinuxPids {
    pub limit: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct OciLinuxResources {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pids: Option<OciLinuxPids>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory: Option<OciLinuxMemory>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu: Option<OciLinuxCpu>,
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
}
