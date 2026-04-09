//! Image config types (from the config blob).

use std::collections::HashMap;

use edgerun_json::{JsonNumber, JsonValue};

use super::manifest::{LayerDescriptor, ManifestDescriptor, PlatformDescriptor};

/// Helper: parse JSON bytes into a JsonValue.
pub fn parse_json_bytes(data: &[u8]) -> Result<JsonValue, String> {
    let s = std::str::from_utf8(data).map_err(|e| e.to_string())?;
    edgerun_json::parse_json(s).map_err(|e| e.to_string())
}

/// Helper: serialize a JsonValue to a pretty-printed string.
pub fn to_string_pretty(v: &JsonValue) -> String {
    format_json_value(v, 0)
}

fn format_json_value(v: &JsonValue, indent: usize) -> String {
    match v {
        JsonValue::Null => "null".into(),
        JsonValue::Bool(b) => b.to_string(),
        JsonValue::Number(n) => match n {
            JsonNumber::I64(i) => i.to_string(),
            JsonNumber::U64(u) => u.to_string(),
            JsonNumber::F64(f) => f.to_string(),
        },
        JsonValue::String(s) => format!("\"{}\"", escape_json_string(s)),
        JsonValue::Array(arr) => {
            if arr.is_empty() {
                return "[]".into();
            }
            let pad = "  ".repeat(indent + 1);
            let inner = arr
                .iter()
                .map(|v| format!("{}{}", pad, format_json_value(v, indent + 1)))
                .collect::<Vec<_>>()
                .join(",\n");
            format!("[\n{}\n{}]", inner, "  ".repeat(indent))
        }
        JsonValue::Object(fields) => {
            if fields.is_empty() {
                return "{}".into();
            }
            let pad = "  ".repeat(indent + 1);
            let inner = fields
                .iter()
                .map(|(k, v)| {
                    format!(
                        "{}\"{}\": {}",
                        pad,
                        escape_json_string(k),
                        format_json_value(v, indent + 1)
                    )
                })
                .collect::<Vec<_>>()
                .join(",\n");
            format!("{{\n{}\n{}}}", inner, "  ".repeat(indent))
        }
    }
}

fn escape_json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out
}

/// Parsed image config.
#[derive(Debug)]
pub struct ImageConfig {
    pub architecture: Option<String>,
    pub os: Option<String>,
    pub config: Option<ImageConfigInner>,
    pub rootfs: Option<RootFs>,
    pub history: Option<Vec<HistoryEntry>>,
}

/// Inner image config (Cmd, Env, WorkingDir, etc.).
#[derive(Debug)]
pub struct ImageConfigInner {
    pub user: Option<String>,
    pub env: Option<Vec<String>>,
    pub entrypoint: Option<Vec<String>>,
    pub cmd: Option<Vec<String>>,
    pub working_dir: Option<String>,
    pub exposed_ports: Option<HashMap<String, JsonValue>>,
    pub volumes: Option<HashMap<String, JsonValue>>,
    pub labels: Option<HashMap<String, String>>,
    pub stop_signal: Option<String>,
}

/// Root filesystem info.
#[derive(Debug)]
pub struct RootFs {
    pub r#type: String,
    pub diff_ids: Vec<String>,
}

/// History entry (build layer info, from OCI image config).
#[derive(Debug)]
pub struct HistoryEntry {
    pub created: Option<String>,
    pub created_by: Option<String>,
    pub comment: Option<String>,
    pub empty_layer: Option<bool>,
}

// ===========================================================================
// Manual JSON parsing using edgerun-json (no serde)
// ===========================================================================

pub fn parse_image_config(data: &[u8]) -> Result<ImageConfig, String> {
    let value = parse_json_bytes(data)?;
    parse_config_from_value(&value)
}

fn parse_config_from_value(v: &JsonValue) -> Result<ImageConfig, String> {
    let obj = match v {
        JsonValue::Object(fields) => fields,
        _ => return Err("Expected object for image config".into()),
    };

    let architecture = find_str(obj, "architecture");
    let os = find_str(obj, "os");
    let config = find_obj(obj, "config")
        .as_ref()
        .map(|o| parse_config_inner(o))
        .transpose()?;
    let rootfs = find_obj(obj, "rootfs")
        .as_ref()
        .map(|o| parse_rootfs(o))
        .transpose()?;
    let history = find_array(obj, "history")
        .map(|arr| {
            arr.iter()
                .map(|v| parse_history(v))
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?;

    Ok(ImageConfig {
        architecture,
        os,
        config,
        rootfs,
        history,
    })
}

fn parse_config_inner(obj: &[(String, JsonValue)]) -> Result<ImageConfigInner, String> {
    let user = find_str(obj, "User");
    let env = find_array(obj, "Env").map(|arr| {
        arr.iter()
            .filter_map(|v| as_str(v))
            .map(|s| s.to_string())
            .collect()
    });
    let entrypoint = find_array(obj, "Entrypoint").map(|arr| {
        arr.iter()
            .filter_map(|v| as_str(v))
            .map(|s| s.to_string())
            .collect()
    });
    let cmd = find_array(obj, "Cmd").map(|arr| {
        arr.iter()
            .filter_map(|v| as_str(v))
            .map(|s| s.to_string())
            .collect()
    });
    let working_dir = find_str(obj, "WorkingDir");
    let exposed_ports = find_obj(obj, "ExposedPorts")
        .map(|o| o.iter().map(|(k, v)| (k.clone(), v.clone())).collect());
    let volumes = find_obj(obj, "Volumes")
        .map(|o| o.iter().map(|(k, v)| (k.clone(), v.clone())).collect());
    let labels = find_obj(obj, "Labels").map(|o| {
        o.iter()
            .filter_map(|(k, v)| Some((k.clone(), as_str(v)?.to_string())))
            .collect()
    });
    let stop_signal = find_str(obj, "StopSignal");

    Ok(ImageConfigInner {
        user,
        env,
        entrypoint,
        cmd,
        working_dir,
        exposed_ports,
        volumes,
        labels,
        stop_signal,
    })
}

fn parse_rootfs(obj: &[(String, JsonValue)]) -> Result<RootFs, String> {
    let r#type = find_str(obj, "type").unwrap_or_default();
    let diff_ids = find_array(obj, "diff_ids")
        .map(|arr| {
            arr.iter()
                .filter_map(|v| as_str(v))
                .map(|s| s.to_string())
                .collect()
        })
        .unwrap_or_default();
    Ok(RootFs {
        r#type,
        diff_ids,
    })
}

fn parse_history(v: &JsonValue) -> Result<HistoryEntry, String> {
    let obj = match v {
        JsonValue::Object(fields) => fields,
        _ => return Err("Expected object for history".into()),
    };
    let created = find_str(obj, "created");
    let created_by = find_str(obj, "created_by");
    let comment = find_str(obj, "comment");
    let empty_layer = find_bool(obj, "empty_layer");
    Ok(HistoryEntry {
        created,
        created_by,
        comment,
        empty_layer,
    })
}

pub fn parse_manifest(v: &JsonValue) -> Result<crate::manifest::ImageManifest, String> {
    let obj = match v {
        JsonValue::Object(fields) => fields,
        _ => return Err("Expected object for manifest".into()),
    };

    // Check if this is an index/manifest list
    if let Some(JsonValue::Array(manifests)) = obj
        .iter()
        .find(|(k, _)| k == "manifests")
        .map(|(_, v)| v)
    {
        let media_type = find_str(obj, "mediaType");
        let parsed_manifests: Result<Vec<ManifestDescriptor>, String> =
            manifests.iter().map(|m| parse_manifest_descriptor(m)).collect();
        return Ok(crate::manifest::ImageManifest::Index(
            crate::manifest::ImageIndex {
                media_type,
                manifests: parsed_manifests?,
            },
        ));
    }

    // Single manifest
    let layers = find_array(obj, "layers")
        .map(|arr| {
            arr.iter()
                .filter_map(|l| parse_layer_descriptor(l).ok())
                .collect()
        })
        .unwrap_or_default();

    let config_digest = find_obj(obj, "config")
        .and_then(|fields| find_str(fields, "digest"))
        .or_else(|| find_str(obj, "config"))
        .unwrap_or_default();

    Ok(crate::manifest::ImageManifest::Single(
        crate::manifest::SingleManifest {
            config_digest,
            layers,
        },
    ))
}

pub fn parse_single_manifest_from_value(
    v: &JsonValue,
) -> Result<crate::manifest::SingleManifest, String> {
    let obj = match v {
        JsonValue::Object(fields) => fields,
        _ => return Err("Expected object for manifest".into()),
    };

    let layers = find_array(obj, "layers")
        .map(|arr| {
            arr.iter()
                .filter_map(|l| parse_layer_descriptor(l).ok())
                .collect()
        })
        .unwrap_or_default();

    let config_digest = find_obj(obj, "config")
        .and_then(|fields| find_str(fields, "digest"))
        .unwrap_or_default();

    Ok(crate::manifest::SingleManifest {
        config_digest,
        layers,
    })
}

fn parse_manifest_descriptor(v: &JsonValue) -> Result<ManifestDescriptor, String> {
    let obj = match v {
        JsonValue::Object(fields) => fields,
        _ => return Err("Expected object for manifest descriptor".into()),
    };
    let media_type = find_str(obj, "mediaType");
    let digest = find_str(obj, "digest").unwrap_or_default();
    let size = find_u64(obj, "size").unwrap_or(0);
    let platform = find_obj(obj, "platform").map(|fields| PlatformDescriptor {
        architecture: find_str(&fields, "architecture"),
        os: find_str(&fields, "os"),
    });
    Ok(ManifestDescriptor {
        media_type,
        digest,
        size,
        platform,
    })
}

fn parse_layer_descriptor(v: &JsonValue) -> Result<LayerDescriptor, String> {
    let obj = match v {
        JsonValue::Object(fields) => fields,
        _ => return Err("Expected object for layer".into()),
    };
    let media_type = find_str(obj, "mediaType");
    let digest = find_str(obj, "digest").unwrap_or_default();
    let size = find_u64(obj, "size").unwrap_or(0);
    Ok(LayerDescriptor {
        media_type,
        digest,
        size,
    })
}

// ===========================================================================
// JSON field helpers
// ===========================================================================

fn find_obj<'a>(
    obj: &'a [(String, JsonValue)],
    key: &str,
) -> Option<&'a [(String, JsonValue)]> {
    obj.iter()
        .find(|(k, _)| k == key)
        .and_then(|(_, v)| match v {
            JsonValue::Object(fields) => Some(fields.as_slice()),
            _ => None,
        })
}

fn find_array<'a>(obj: &'a [(String, JsonValue)], key: &str) -> Option<&'a [JsonValue]> {
    obj.iter()
        .find(|(k, _)| k == key)
        .and_then(|(_, v)| match v {
            JsonValue::Array(arr) => Some(arr.as_slice()),
            _ => None,
        })
}

fn find_str<'a>(obj: &'a [(String, JsonValue)], key: &str) -> Option<String> {
    obj.iter()
        .find(|(k, _)| k == key)
        .and_then(|(_, v)| match v {
            JsonValue::String(s) => Some(s.clone()),
            _ => None,
        })
}

fn find_u64(obj: &[(String, JsonValue)], key: &str) -> Option<u64> {
    obj.iter()
        .find(|(k, _)| k == key)
        .and_then(|(_, v)| match v {
            JsonValue::Number(JsonNumber::U64(n)) => Some(*n),
            JsonValue::Number(JsonNumber::I64(n)) => Some(*n as u64),
            _ => None,
        })
}

fn find_bool(obj: &[(String, JsonValue)], key: &str) -> Option<bool> {
    obj.iter()
        .find(|(k, _)| k == key)
        .and_then(|(_, v)| match v {
            JsonValue::Bool(b) => Some(*b),
            _ => None,
        })
}

fn as_str(v: &JsonValue) -> Option<&str> {
    match v {
        JsonValue::String(s) => Some(s),
        _ => None,
    }
}
