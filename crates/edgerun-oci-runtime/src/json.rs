//! Manual JSON parsing for OCI spec types — replaces serde derives.

use edgerun_json::JsonValue;

// ===========================================================================
// JSON helpers
// ===========================================================================

fn str_field(v: &JsonValue, key: &str) -> Option<String> {
    if let JsonValue::Object(map) = v {
        for (k, val) in map.iter() {
            if k == key {
                if let JsonValue::String(s) = val {
                    return Some(s.clone());
                }
            }
        }
    }
    None
}

fn opt_str_field(v: &JsonValue, key: &str) -> Option<String> {
    str_field(v, key)
}

fn bool_field(v: &JsonValue, key: &str) -> Option<bool> {
    if let JsonValue::Object(map) = v {
        for (k, val) in map.iter() {
            if k == key {
                if let JsonValue::Bool(b) = val {
                    return Some(*b);
                }
            }
        }
    }
    None
}

fn u32_field(v: &JsonValue, key: &str) -> Option<u32> {
    if let JsonValue::Object(map) = v {
        for (k, val) in map.iter() {
            if k == key {
                if let JsonValue::Number(n) = val {
                    return match n {
                        edgerun_json::JsonNumber::U64(n) => Some(*n as u32),
                        edgerun_json::JsonNumber::I64(n) => Some(*n as u32),
                        edgerun_json::JsonNumber::F64(n) => Some(*n as u32),
                    };
                }
            }
        }
    }
    None
}

fn u64_field(v: &JsonValue, key: &str) -> Option<u64> {
    if let JsonValue::Object(map) = v {
        for (k, val) in map.iter() {
            if k == key {
                if let JsonValue::Number(n) = val {
                    return match n {
                        edgerun_json::JsonNumber::U64(n) => Some(*n),
                        edgerun_json::JsonNumber::I64(n) => Some(*n as u64),
                        edgerun_json::JsonNumber::F64(n) => Some(*n as u64),
                    };
                }
            }
        }
    }
    None
}

fn obj_field<'a>(v: &'a JsonValue, key: &str) -> Option<&'a JsonValue> {
    if let JsonValue::Object(map) = v {
        for (k, val) in map.iter() {
            if k == key {
                return Some(val);
            }
        }
    }
    None
}

fn arr_field<'a>(v: &'a JsonValue, key: &str) -> Option<&'a [JsonValue]> {
    if let JsonValue::Object(map) = v {
        for (k, val) in map.iter() {
            if k == key {
                if let JsonValue::Array(a) = val {
                    return Some(a);
                }
            }
        }
    }
    None
}

// ===========================================================================
// Deserialization
// ===========================================================================

pub fn parse_oci_spec(data: &[u8]) -> Result<OciSpec, String> {
    let v = edgerun_json::from_slice(data).map_err(|e| e.to_string())?;
    OciSpec::from_json(&v)
}

#[derive(Debug, Clone)]
pub struct OciSpec {
    pub version: String,
    pub process: Option<OciProcess>,
    pub root: Option<OciRoot>,
    pub hostname: Option<String>,
    pub linux: Option<OciLinux>,
    pub mounts: Option<Vec<OciMount>>,
}

impl OciSpec {
    fn from_json(v: &JsonValue) -> Result<Self, String> {
        Ok(OciSpec {
            version: str_field(v, "ociVersion").unwrap_or_default(),
            process: obj_field(v, "process").map(|x| OciProcess::from_json(x)).transpose()?,
            root: obj_field(v, "root").map(|x| OciRoot::from_json(x)).transpose()?,
            hostname: opt_str_field(v, "hostname"),
            linux: obj_field(v, "linux").map(|x| OciLinux::from_json(x)).transpose()?,
            mounts: arr_field(v, "mounts").map(|arr| arr.iter().map(|x| OciMount::from_json(x)).collect::<Result<Vec<_>, _>>()).transpose()?,
        })
    }

    pub fn to_json_string_pretty(&self) -> String {
        let mut out = String::from("{\n");
        out.push_str(&format!("  \"ociVersion\": {},\n", json_str(&self.version)));
        if let Some(p) = &self.process {
            out.push_str(&format!("  \"process\": {},\n", p.to_json_pretty(1)));
        }
        if let Some(r) = &self.root {
            out.push_str(&format!("  \"root\": {},\n", r.to_json_pretty(1)));
        }
        if let Some(h) = &self.hostname {
            out.push_str(&format!("  \"hostname\": {},\n", json_str(h)));
        }
        if let Some(l) = &self.linux {
            out.push_str(&format!("  \"linux\": {},\n", l.to_json_pretty(1)));
        }
        if let Some(m) = &self.mounts {
            out.push_str("  \"mounts\": [\n");
            for (i, mount) in m.iter().enumerate() {
                out.push_str(&format!("    {}", mount.to_json_pretty(2)));
                if i + 1 < m.len() {
                    out.push(',');
                }
                out.push('\n');
            }
            out.push_str("  ],\n");
        }
        // Remove trailing comma before closing brace
        if out.ends_with(",\n") {
            out.pop();
            out.pop();
            out.push('\n');
        }
        out.push('}');
        out
    }
}

#[derive(Debug, Clone, Default)]
pub struct OciProcess {
    pub terminal: Option<bool>,
    pub user: Option<OciUser>,
    pub args: Option<Vec<String>>,
    pub env: Option<Vec<String>>,
    pub cwd: Option<String>,
    pub capabilities: Option<OciCapabilities>,
    pub rlimits: Option<Vec<OciRlimit>>,
    pub no_new_privileges: Option<bool>,
}

impl OciProcess {
    fn from_json(v: &JsonValue) -> Result<Self, String> {
        Ok(OciProcess {
            terminal: bool_field(v, "terminal"),
            user: obj_field(v, "user").map(|x| OciUser::from_json(x)).transpose()?,
            args: arr_field(v, "args").map(|arr| arr.iter().filter_map(|x| {
                if let JsonValue::String(s) = x { Some(s.clone()) } else { None }
            }).collect()),
            env: arr_field(v, "env").map(|arr| arr.iter().filter_map(|x| {
                if let JsonValue::String(s) = x { Some(s.clone()) } else { None }
            }).collect()),
            cwd: opt_str_field(v, "cwd"),
            capabilities: obj_field(v, "capabilities").map(|x| OciCapabilities::from_json(x)).transpose()?,
            rlimits: arr_field(v, "rlimits").map(|arr| arr.iter().map(|x| OciRlimit::from_json(x)).collect::<Result<Vec<_>, _>>()).transpose()?,
            no_new_privileges: bool_field(v, "noNewPrivileges"),
        })
    }

    fn to_json_pretty(&self, indent: usize) -> String {
        let pad = "  ".repeat(indent);
        let inner = "  ".repeat(indent + 1);
        let mut out = String::from("{\n");
        if let Some(t) = self.terminal { out.push_str(&format!("{}\"terminal\": {},\n", inner, t)); }
        if let Some(u) = &self.user { out.push_str(&format!("{}\"user\": {},\n", inner, u.to_json_pretty(indent + 1))); }
        if let Some(a) = &self.args { out.push_str(&format!("{}\"args\": {},\n", inner, json_str_array(a))); }
        if let Some(e) = &self.env { out.push_str(&format!("{}\"env\": {},\n", inner, json_str_array(e))); }
        if let Some(c) = &self.cwd { out.push_str(&format!("{}\"cwd\": {},\n", inner, json_str(c))); }
        if let Some(c) = &self.capabilities { out.push_str(&format!("{}\"capabilities\": {},\n", inner, c.to_json_pretty(indent + 1))); }
        if let Some(r) = &self.rlimits {
            out.push_str(&format!("{}\"rlimits\": [\n", inner));
            for (i, rl) in r.iter().enumerate() {
                out.push_str(&format!("    {}", rl.to_json_pretty(indent + 2)));
                if i + 1 < r.len() { out.push(','); }
                out.push('\n');
            }
            out.push_str(&format!("{}],\n", inner));
        }
        if let Some(n) = self.no_new_privileges { out.push_str(&format!("{}\"noNewPrivileges\": {},\n", inner, n)); }
        if out.ends_with(",\n") { out.pop(); out.pop(); out.push('\n'); }
        out.push_str(&pad);
        out.push('}');
        out
    }
}

#[derive(Debug, Clone, Default)]
pub struct OciUser {
    pub uid: Option<u32>,
    pub gid: Option<u32>,
    pub additional_gids: Option<Vec<u32>>,
}

impl OciUser {
    fn from_json(v: &JsonValue) -> Result<Self, String> {
        Ok(OciUser {
            uid: u32_field(v, "uid"),
            gid: u32_field(v, "gid"),
            additional_gids: arr_field(v, "additionalGids").map(|arr| arr.iter().filter_map(|x| {
                if let JsonValue::Number(n) = x {
                    match n {
                        edgerun_json::JsonNumber::U64(n) => Some(*n as u32),
                        edgerun_json::JsonNumber::I64(n) => Some(*n as u32),
                        _ => None,
                    }
                } else { None }
            }).collect()),
        })
    }

    fn to_json_pretty(&self, indent: usize) -> String {
        let pad = "  ".repeat(indent);
        let inner = "  ".repeat(indent + 1);
        let mut out = String::from("{\n");
        if let Some(u) = self.uid { out.push_str(&format!("{}\"uid\": {},\n", inner, u)); }
        if let Some(g) = self.gid { out.push_str(&format!("{}\"gid\": {},\n", inner, g)); }
        if let Some(a) = &self.additional_gids { out.push_str(&format!("{}\"additionalGids\": {},\n", inner, json_u32_array(a))); }
        if out.ends_with(",\n") { out.pop(); out.pop(); out.push('\n'); }
        out.push_str(&pad);
        out.push('}');
        out
    }
}

#[derive(Debug, Clone, Default)]
pub struct OciCapabilities {
    pub bounding: Option<Vec<String>>,
    pub effective: Option<Vec<String>>,
    pub inheritable: Option<Vec<String>>,
    pub permitted: Option<Vec<String>>,
    pub ambient: Option<Vec<String>>,
}

impl OciCapabilities {
    fn from_json(v: &JsonValue) -> Result<Self, String> {
        Ok(OciCapabilities {
            bounding: str_array_field(v, "bounding"),
            effective: str_array_field(v, "effective"),
            inheritable: str_array_field(v, "inheritable"),
            permitted: str_array_field(v, "permitted"),
            ambient: str_array_field(v, "ambient"),
        })
    }

    fn to_json_pretty(&self, indent: usize) -> String {
        let pad = "  ".repeat(indent);
        let inner = "  ".repeat(indent + 1);
        let mut out = String::from("{\n");
        if let Some(a) = &self.bounding { out.push_str(&format!("{}\"bounding\": {},\n", inner, json_str_array(a))); }
        if let Some(a) = &self.effective { out.push_str(&format!("{}\"effective\": {},\n", inner, json_str_array(a))); }
        if let Some(a) = &self.inheritable { out.push_str(&format!("{}\"inheritable\": {},\n", inner, json_str_array(a))); }
        if let Some(a) = &self.permitted { out.push_str(&format!("{}\"permitted\": {},\n", inner, json_str_array(a))); }
        if let Some(a) = &self.ambient { out.push_str(&format!("{}\"ambient\": {},\n", inner, json_str_array(a))); }
        if out.ends_with(",\n") { out.pop(); out.pop(); out.push('\n'); }
        out.push_str(&pad);
        out.push('}');
        out
    }
}

#[derive(Debug, Clone, Default)]
pub struct OciRlimit {
    pub ns_type: String,
    pub hard: u64,
    pub soft: u64,
}

impl OciRlimit {
    fn from_json(v: &JsonValue) -> Result<Self, String> {
        Ok(OciRlimit {
            ns_type: str_field(v, "type").or_else(|| str_field(v, "ns_type")).unwrap_or_default(),
            hard: u64_field(v, "hard").unwrap_or(0),
            soft: u64_field(v, "soft").unwrap_or(0),
        })
    }

    fn to_json_pretty(&self, indent: usize) -> String {
        let pad = "  ".repeat(indent);
        let inner = "  ".repeat(indent + 1);
        format!(
            "{{\n{}\"type\": {},\n{}\"hard\": {},\n{}\"soft\": {}\n{}}}",
            inner, json_str(&self.ns_type),
            inner, self.hard,
            inner, self.soft,
            pad
        )
    }
}

#[derive(Debug, Clone, Default)]
pub struct OciRoot {
    pub path: String,
    pub readonly: Option<bool>,
}

impl OciRoot {
    fn from_json(v: &JsonValue) -> Result<Self, String> {
        Ok(OciRoot {
            path: str_field(v, "path").unwrap_or_default(),
            readonly: bool_field(v, "readonly"),
        })
    }

    fn to_json_pretty(&self, indent: usize) -> String {
        let pad = "  ".repeat(indent);
        let inner = "  ".repeat(indent + 1);
        let mut out = String::from("{\n");
        out.push_str(&format!("{}\"path\": {},\n", inner, json_str(&self.path)));
        if let Some(r) = self.readonly { out.push_str(&format!("{}\"readonly\": {},\n", inner, r)); }
        if out.ends_with(",\n") { out.pop(); out.pop(); out.push('\n'); }
        out.push_str(&pad);
        out.push('}');
        out
    }
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
}

impl OciLinux {
    fn from_json(v: &JsonValue) -> Result<Self, String> {
        Ok(OciLinux {
            uid_mappings: arr_field(v, "uidMappings").map(|arr| arr.iter().map(|x| OciIdMapping::from_json(x)).collect::<Result<Vec<_>, _>>()).transpose()?,
            gid_mappings: arr_field(v, "gidMappings").map(|arr| arr.iter().map(|x| OciIdMapping::from_json(x)).collect::<Result<Vec<_>, _>>()).transpose()?,
            resources: obj_field(v, "resources").map(|x| OciLinuxResources::from_json(x)).transpose()?,
            cgroups_path: opt_str_field(v, "cgroupsPath"),
            namespaces: arr_field(v, "namespaces").map(|arr| arr.iter().map(|x| OciNamespace::from_json(x)).collect::<Result<Vec<_>, _>>()).transpose()?,
            devices: arr_field(v, "devices").map(|arr| arr.iter().map(|x| OciLinuxDevice::from_json(x)).collect::<Result<Vec<_>, _>>()).transpose()?,
            masked_paths: str_array_field(v, "maskedPaths"),
            readonly_paths: str_array_field(v, "readonlyPaths"),
            mount_label: opt_str_field(v, "mountLabel"),
        })
    }

    fn to_json_pretty(&self, indent: usize) -> String {
        let pad = "  ".repeat(indent);
        let inner = "  ".repeat(indent + 1);
        let mut out = String::from("{\n");
        if let Some(a) = &self.uid_mappings { out.push_str(&format!("{}\"uidMappings\": {},\n", inner, id_mapping_array(a, indent + 1))); }
        if let Some(a) = &self.gid_mappings { out.push_str(&format!("{}\"gidMappings\": {},\n", inner, id_mapping_array(a, indent + 1))); }
        if let Some(r) = &self.resources { out.push_str(&format!("{}\"resources\": {},\n", inner, r.to_json_pretty(indent + 1))); }
        if let Some(c) = &self.cgroups_path { out.push_str(&format!("{}\"cgroupsPath\": {},\n", inner, json_str(c))); }
        if let Some(a) = &self.namespaces { out.push_str(&format!("{}\"namespaces\": {},\n", inner, namespace_array(a, indent + 1))); }
        if let Some(a) = &self.devices { out.push_str(&format!("{}\"devices\": {},\n", inner, device_array(a, indent + 1))); }
        if let Some(a) = &self.masked_paths { out.push_str(&format!("{}\"maskedPaths\": {},\n", inner, json_str_array(a))); }
        if let Some(a) = &self.readonly_paths { out.push_str(&format!("{}\"readonlyPaths\": {},\n", inner, json_str_array(a))); }
        if let Some(l) = &self.mount_label { out.push_str(&format!("{}\"mountLabel\": {},\n", inner, json_str(l))); }
        if out.ends_with(",\n") { out.pop(); out.pop(); out.push('\n'); }
        out.push_str(&pad);
        out.push('}');
        out
    }
}

#[derive(Debug, Clone, Default)]
pub struct OciIdMapping {
    pub container_id: u32,
    pub host_id: u32,
    pub size: u32,
}

impl OciIdMapping {
    fn from_json(v: &JsonValue) -> Result<Self, String> {
        Ok(OciIdMapping {
            container_id: u32_field(v, "containerID").unwrap_or(0),
            host_id: u32_field(v, "hostID").unwrap_or(0),
            size: u32_field(v, "size").unwrap_or(0),
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct OciNamespace {
    pub ns_type: String,
    pub path: Option<String>,
}

impl OciNamespace {
    fn from_json(v: &JsonValue) -> Result<Self, String> {
        Ok(OciNamespace {
            ns_type: str_field(v, "type").or_else(|| str_field(v, "ns_type")).unwrap_or_default(),
            path: opt_str_field(v, "path"),
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct OciLinuxDevice {
    pub ns_type: String,
    pub path: String,
    pub file_mode: Option<u32>,
    pub uid: Option<u32>,
    pub gid: Option<u32>,
}

impl OciLinuxDevice {
    fn from_json(v: &JsonValue) -> Result<Self, String> {
        Ok(OciLinuxDevice {
            ns_type: str_field(v, "type").or_else(|| str_field(v, "ns_type")).unwrap_or_default(),
            path: str_field(v, "path").unwrap_or_default(),
            file_mode: u32_field(v, "fileMode"),
            uid: u32_field(v, "uid"),
            gid: u32_field(v, "gid"),
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct OciLinuxPids {
    pub limit: i64,
}

impl OciLinuxPids {
    fn from_json(v: &JsonValue) -> Result<Self, String> {
        Ok(OciLinuxPids {
            limit: obj_field(v, "limit").and_then(|x| {
                if let JsonValue::Number(n) = x {
                    match n {
                        edgerun_json::JsonNumber::I64(n) => Some(*n),
                        edgerun_json::JsonNumber::U64(n) => Some(*n as i64),
                        _ => None,
                    }
                } else { None }
            }).unwrap_or(0),
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct OciLinuxResources {
    pub pids: Option<OciLinuxPids>,
    pub memory: Option<OciLinuxMemory>,
    pub cpu: Option<OciLinuxCpu>,
}

impl OciLinuxResources {
    fn from_json(v: &JsonValue) -> Result<Self, String> {
        Ok(OciLinuxResources {
            pids: obj_field(v, "pids").map(|x| OciLinuxPids::from_json(x)).transpose()?,
            memory: obj_field(v, "memory").map(|x| OciLinuxMemory::from_json(x)).transpose()?,
            cpu: obj_field(v, "cpu").map(|x| OciLinuxCpu::from_json(x)).transpose()?,
        })
    }

    fn to_json_pretty(&self, indent: usize) -> String {
        let pad = "  ".repeat(indent);
        let inner = "  ".repeat(indent + 1);
        let mut out = String::from("{\n");
        if let Some(m) = &self.memory { out.push_str(&format!("{}\"memory\": {},\n", inner, m.to_json_pretty(indent + 1))); }
        if let Some(c) = &self.cpu { out.push_str(&format!("{}\"cpu\": {},\n", inner, c.to_json_pretty(indent + 1))); }
        if out.ends_with(",\n") { out.pop(); out.pop(); out.push('\n'); }
        out.push_str(&pad);
        out.push('}');
        out
    }
}

#[derive(Debug, Clone, Default)]
pub struct OciLinuxMemory {
    pub limit: Option<i64>,
    pub reservation: Option<i64>,
    pub swap: Option<i64>,
    pub kernel: Option<i64>,
    pub kernel_tcp: Option<i64>,
}

impl OciLinuxMemory {
    fn from_json(v: &JsonValue) -> Result<Self, String> {
        fn i64_field(jv: &JsonValue, key: &str) -> Option<i64> {
            if let JsonValue::Number(n) = jv {
                // Need to search in parent
            }
            None
        }
        Ok(OciLinuxMemory {
            limit: obj_field(v, "limit").and_then(|x| {
                if let JsonValue::Number(n) = x {
                    match n {
                        edgerun_json::JsonNumber::I64(n) => Some(*n),
                        edgerun_json::JsonNumber::U64(n) => Some(*n as i64),
                        _ => None,
                    }
                } else { None }
            }),
            reservation: obj_field(v, "reservation").and_then(|x| {
                if let JsonValue::Number(n) = x {
                    match n {
                        edgerun_json::JsonNumber::I64(n) => Some(*n),
                        edgerun_json::JsonNumber::U64(n) => Some(*n as i64),
                        _ => None,
                    }
                } else { None }
            }),
            swap: obj_field(v, "swap").and_then(|x| {
                if let JsonValue::Number(n) = x {
                    match n {
                        edgerun_json::JsonNumber::I64(n) => Some(*n),
                        edgerun_json::JsonNumber::U64(n) => Some(*n as i64),
                        _ => None,
                    }
                } else { None }
            }),
            kernel: obj_field(v, "kernel").and_then(|x| {
                if let JsonValue::Number(n) = x {
                    match n {
                        edgerun_json::JsonNumber::I64(n) => Some(*n),
                        edgerun_json::JsonNumber::U64(n) => Some(*n as i64),
                        _ => None,
                    }
                } else { None }
            }),
            kernel_tcp: obj_field(v, "kernelTCP").and_then(|x| {
                if let JsonValue::Number(n) = x {
                    match n {
                        edgerun_json::JsonNumber::I64(n) => Some(*n),
                        edgerun_json::JsonNumber::U64(n) => Some(*n as i64),
                        _ => None,
                    }
                } else { None }
            }),
        })
    }

    fn to_json_pretty(&self, indent: usize) -> String {
        let pad = "  ".repeat(indent);
        let inner = "  ".repeat(indent + 1);
        let mut out = String::from("{\n");
        if let Some(v) = self.limit { out.push_str(&format!("{}\"limit\": {},\n", inner, v)); }
        if let Some(v) = self.reservation { out.push_str(&format!("{}\"reservation\": {},\n", inner, v)); }
        if let Some(v) = self.swap { out.push_str(&format!("{}\"swap\": {},\n", inner, v)); }
        if let Some(v) = self.kernel { out.push_str(&format!("{}\"kernel\": {},\n", inner, v)); }
        if let Some(v) = self.kernel_tcp { out.push_str(&format!("{}\"kernelTCP\": {},\n", inner, v)); }
        if out.ends_with(",\n") { out.pop(); out.pop(); out.push('\n'); }
        out.push_str(&pad);
        out.push('}');
        out
    }
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
}

impl OciLinuxCpu {
    fn from_json(v: &JsonValue) -> Result<Self, String> {
        Ok(OciLinuxCpu {
            shares: obj_field(v, "shares").and_then(|x| {
                if let JsonValue::Number(n) = x {
                    match n {
                        edgerun_json::JsonNumber::U64(n) => Some(*n),
                        edgerun_json::JsonNumber::I64(n) => Some(*n as u64),
                        _ => None,
                    }
                } else { None }
            }),
            quota: obj_field(v, "quota").and_then(|x| {
                if let JsonValue::Number(n) = x {
                    match n {
                        edgerun_json::JsonNumber::I64(n) => Some(*n),
                        edgerun_json::JsonNumber::U64(n) => Some(*n as i64),
                        _ => None,
                    }
                } else { None }
            }),
            period: obj_field(v, "period").and_then(|x| {
                if let JsonValue::Number(n) = x {
                    match n {
                        edgerun_json::JsonNumber::U64(n) => Some(*n),
                        edgerun_json::JsonNumber::I64(n) => Some(*n as u64),
                        _ => None,
                    }
                } else { None }
            }),
            realtime_runtime: obj_field(v, "realtimeRuntime").and_then(|x| {
                if let JsonValue::Number(n) = x {
                    match n {
                        edgerun_json::JsonNumber::I64(n) => Some(*n),
                        edgerun_json::JsonNumber::U64(n) => Some(*n as i64),
                        _ => None,
                    }
                } else { None }
            }),
            realtime_period: obj_field(v, "realtimePeriod").and_then(|x| {
                if let JsonValue::Number(n) = x {
                    match n {
                        edgerun_json::JsonNumber::U64(n) => Some(*n),
                        edgerun_json::JsonNumber::I64(n) => Some(*n as u64),
                        _ => None,
                    }
                } else { None }
            }),
            cpus: opt_str_field(v, "cpus"),
            mems: opt_str_field(v, "mems"),
        })
    }

    fn to_json_pretty(&self, indent: usize) -> String {
        let pad = "  ".repeat(indent);
        let inner = "  ".repeat(indent + 1);
        let mut out = String::from("{\n");
        if let Some(v) = self.shares { out.push_str(&format!("{}\"shares\": {},\n", inner, v)); }
        if let Some(v) = self.quota { out.push_str(&format!("{}\"quota\": {},\n", inner, v)); }
        if let Some(v) = self.period { out.push_str(&format!("{}\"period\": {},\n", inner, v)); }
        if let Some(v) = self.realtime_runtime { out.push_str(&format!("{}\"realtimeRuntime\": {},\n", inner, v)); }
        if let Some(v) = self.realtime_period { out.push_str(&format!("{}\"realtimePeriod\": {},\n", inner, v)); }
        if let Some(v) = &self.cpus { out.push_str(&format!("{}\"cpus\": {},\n", inner, json_str(v))); }
        if let Some(v) = &self.mems { out.push_str(&format!("{}\"mems\": {},\n", inner, json_str(v))); }
        if out.ends_with(",\n") { out.pop(); out.pop(); out.push('\n'); }
        out.push_str(&pad);
        out.push('}');
        out
    }
}

#[derive(Debug, Clone, Default)]
pub struct OciMount {
    pub destination: String,
    pub mount_type: Option<String>,
    pub source: Option<String>,
    pub options: Option<Vec<String>>,
}

impl OciMount {
    pub fn from_json(v: &JsonValue) -> Result<Self, String> {
        Ok(OciMount {
            destination: str_field(v, "destination").unwrap_or_default(),
            mount_type: opt_str_field(v, "type").or_else(|| str_field(v, "ns_type")),
            source: opt_str_field(v, "source"),
            options: str_array_field(v, "options"),
        })
    }

    pub fn to_json_pretty(&self, indent: usize) -> String {
        let pad = "  ".repeat(indent);
        let inner = "  ".repeat(indent + 1);
        let mut out = String::from("{\n");
        out.push_str(&format!("{}\"destination\": {},\n", inner, json_str(&self.destination)));
        if let Some(t) = &self.mount_type { out.push_str(&format!("{}\"type\": {},\n", inner, json_str(t))); }
        if let Some(s) = &self.source { out.push_str(&format!("{}\"source\": {},\n", inner, json_str(s))); }
        if let Some(o) = &self.options { out.push_str(&format!("{}\"options\": {},\n", inner, json_str_array(o))); }
        if out.ends_with(",\n") { out.pop(); out.pop(); out.push('\n'); }
        out.push_str(&pad);
        out.push('}');
        out
    }
}

// ===========================================================================
// JSON serialization helpers
// ===========================================================================

fn json_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn json_str_array(arr: &[String]) -> String {
    if arr.is_empty() {
        return "[]".to_string();
    }
    let items = arr.iter().map(|s| json_str(s)).collect::<Vec<_>>().join(", ");
    format!("[{}]", items)
}

fn json_u32_array(arr: &[u32]) -> String {
    if arr.is_empty() {
        return "[]".to_string();
    }
    let items = arr.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(", ");
    format!("[{}]", items)
}

fn str_array_field(v: &JsonValue, key: &str) -> Option<Vec<String>> {
    arr_field(v, key).map(|arr| arr.iter().filter_map(|x| {
        if let JsonValue::String(s) = x { Some(s.clone()) } else { None }
    }).collect())
}

fn id_mapping_array(arr: &[OciIdMapping], indent: usize) -> String {
    if arr.is_empty() { return "[]".to_string(); }
    let pad = "  ".repeat(indent);
    let inner = "  ".repeat(indent + 1);
    let mut out = String::from("[\n");
    for (i, m) in arr.iter().enumerate() {
        out.push_str(&format!("{}{{\n", inner));
        out.push_str(&format!("{}\"containerID\": {},\n", "  ".repeat(indent + 2), m.container_id));
        out.push_str(&format!("{}\"hostID\": {},\n", "  ".repeat(indent + 2), m.host_id));
        out.push_str(&format!("{}\"size\": {}\n", "  ".repeat(indent + 2), m.size));
        out.push_str(&format!("{}}}", inner));
        if i + 1 < arr.len() { out.push(','); }
        out.push('\n');
    }
    out.push_str(&pad);
    out.push(']');
    out
}

fn namespace_array(arr: &[OciNamespace], indent: usize) -> String {
    if arr.is_empty() { return "[]".to_string(); }
    let pad = "  ".repeat(indent);
    let inner = "  ".repeat(indent + 1);
    let mut out = String::from("[\n");
    for (i, ns) in arr.iter().enumerate() {
        out.push_str(&format!("{}{{\n", inner));
        out.push_str(&format!("{}\"type\": {},\n", "  ".repeat(indent + 2), json_str(&ns.ns_type)));
        if let Some(p) = &ns.path {
            out.push_str(&format!("{}\"path\": {}\n", "  ".repeat(indent + 2), json_str(p)));
        }
        out.push_str(&format!("{}}}", inner));
        if i + 1 < arr.len() { out.push(','); }
        out.push('\n');
    }
    out.push_str(&pad);
    out.push(']');
    out
}

fn device_array(_arr: &[OciLinuxDevice], indent: usize) -> String {
    if _arr.is_empty() { return "[]".to_string(); }
    let pad = "  ".repeat(indent);
    format!("[\n{}]", pad)
}
