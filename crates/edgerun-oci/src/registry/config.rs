//! Image config types (from the config blob).

use crate::prelude::*;
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
#[cfg(feature = "serde")]
use edgerun_json::from_slice;
#[cfg(feature = "json")]
use edgerun_json::{parse_json, JsonValue, Map};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use super::manifest::{ImageIndex, ImageManifest, SingleManifest};

/// Parsed image config.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct ImageConfig {
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub architecture: Option<String>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub os: Option<String>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub config: Option<ImageConfigInner>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub rootfs: Option<RootFs>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub history: Option<Vec<HistoryEntry>>,
}

/// Inner image config (Cmd, Env, WorkingDir, etc.).
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", serde(rename_all = "PascalCase"))]
pub struct ImageConfigInner {
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub user: Option<String>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub env: Option<Vec<String>>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub entrypoint: Option<Vec<String>>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub cmd: Option<Vec<String>>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub working_dir: Option<String>,
    #[cfg(feature = "json")]
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub exposed_ports: Option<BTreeMap<String, edgerun_json::JsonValue>>,
    #[cfg(feature = "json")]
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub volumes: Option<BTreeMap<String, edgerun_json::JsonValue>>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub labels: Option<BTreeMap<String, String>>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub stop_signal: Option<String>,
}

/// Root filesystem info.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct RootFs {
    pub r#type: String,
    pub diff_ids: Vec<String>,
}

/// History entry (build layer info, from OCI image config).
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub struct HistoryEntry {
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub created: Option<String>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub created_by: Option<String>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub comment: Option<String>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub empty_layer: Option<bool>,
}

// ===========================================================================
// Public API
// ===========================================================================

/// Parse an image config from JSON bytes.
#[cfg(feature = "serde")]
pub fn parse_image_config(data: &[u8]) -> Result<ImageConfig, String> {
    from_slice(data).map_err(|e| e.to_string())
}

/// Parse an image config from JSON bytes without serde.
#[cfg(all(feature = "json", not(feature = "serde")))]
pub fn parse_image_config(data: &[u8]) -> Result<ImageConfig, String> {
    let value = parse_json_bytes(data)?;
    parse_image_config_value(&value)
}

/// Parse JSON bytes into a JsonValue (for ad-hoc inspection).
#[cfg(feature = "json")]
pub fn parse_json_bytes(data: &[u8]) -> Result<edgerun_json::JsonValue, String> {
    let input = core::str::from_utf8(data).map_err(|e| e.to_string())?;
    parse_json(input).map_err(|e| e.to_string())
}

// ===========================================================================
// Manifest parsing via serde
// ===========================================================================

/// Parse a raw JSON blob into an ImageManifest (index or single).
#[cfg(all(feature = "serde", not(feature = "json")))]
pub fn parse_manifest(data: &[u8]) -> Result<ImageManifest, String> {
    // Try as index first (has "manifests" array)
    if let Ok(index) = from_slice::<ImageIndex>(data) {
        if !index.manifests.is_empty() {
            return Ok(ImageManifest::Index(index));
        }
    }

    // Fall back to single manifest
    from_slice::<SingleManifest>(data)
        .map(ImageManifest::Single)
        .map_err(|e| e.to_string())
}

/// Parse a raw JSON blob into an ImageManifest (index or single) without serde.
#[cfg(feature = "json")]
pub fn parse_manifest(data: &[u8]) -> Result<ImageManifest, String> {
    let value = parse_json_bytes(data)?;
    let obj = object(&value, "manifest")?;
    if let Some(JsonValue::Array(manifests)) = obj.get("manifests") {
        if !manifests.is_empty() {
            return Ok(ImageManifest::Index(parse_image_index_object(obj)?));
        }
    }

    parse_single_manifest_value(&value).map(ImageManifest::Single)
}

/// Parse a raw JSON blob as a single manifest.
#[cfg(all(feature = "serde", not(feature = "json")))]
pub fn parse_single_manifest(data: &[u8]) -> Result<SingleManifest, String> {
    from_slice(data).map_err(|e| e.to_string())
}

/// Parse a raw JSON blob as a single manifest without serde.
#[cfg(feature = "json")]
pub fn parse_single_manifest(data: &[u8]) -> Result<SingleManifest, String> {
    let value = parse_json_bytes(data)?;
    parse_single_manifest_value(&value)
}

#[cfg(all(feature = "json", not(feature = "serde")))]
fn parse_image_config_value(value: &JsonValue) -> Result<ImageConfig, String> {
    let obj = object(value, "image config")?;
    Ok(ImageConfig {
        architecture: optional_string(obj, "architecture"),
        os: optional_string(obj, "os"),
        config: obj
            .get("config")
            .map(parse_image_config_inner)
            .transpose()?,
        rootfs: obj.get("rootfs").map(parse_rootfs).transpose()?,
        history: parse_array(obj.get("history"), parse_history_entry)?,
    })
}

#[cfg(all(feature = "json", not(feature = "serde")))]
fn parse_image_config_inner(value: &JsonValue) -> Result<ImageConfigInner, String> {
    let obj = object(value, "image config.config")?;
    Ok(ImageConfigInner {
        user: optional_string_any(obj, &["User", "user"]),
        env: parse_string_array_any(obj, &["Env", "env"])?,
        entrypoint: parse_string_array_any(obj, &["Entrypoint", "entrypoint"])?,
        cmd: parse_string_array_any(obj, &["Cmd", "cmd"])?,
        working_dir: optional_string_any(obj, &["WorkingDir", "workingDir", "working_dir"]),
        exposed_ports: parse_json_object_map_any(obj, &["ExposedPorts", "exposedPorts"])?,
        volumes: parse_json_object_map_any(obj, &["Volumes", "volumes"])?,
        labels: parse_string_map_any(obj, &["Labels", "labels"])?,
        stop_signal: optional_string_any(obj, &["StopSignal", "stopSignal", "stop_signal"]),
    })
}

#[cfg(all(feature = "json", not(feature = "serde")))]
fn parse_rootfs(value: &JsonValue) -> Result<RootFs, String> {
    let obj = object(value, "rootfs")?;
    Ok(RootFs {
        r#type: required_string(obj, "type")?,
        diff_ids: parse_string_array_required(obj, "diff_ids")?,
    })
}

#[cfg(all(feature = "json", not(feature = "serde")))]
fn parse_history_entry(value: &JsonValue) -> Result<HistoryEntry, String> {
    let obj = object(value, "history entry")?;
    Ok(HistoryEntry {
        created: optional_string(obj, "created"),
        created_by: optional_string(obj, "created_by"),
        comment: optional_string(obj, "comment"),
        empty_layer: optional_bool(obj, "empty_layer"),
    })
}

#[cfg(feature = "json")]
fn parse_image_index_object(obj: &Map) -> Result<ImageIndex, String> {
    Ok(ImageIndex {
        media_type: optional_string(obj, "mediaType"),
        manifests: parse_array_required(obj, "manifests", parse_manifest_descriptor)?,
    })
}

#[cfg(feature = "json")]
fn parse_manifest_descriptor(
    value: &JsonValue,
) -> Result<super::manifest::ManifestDescriptor, String> {
    let obj = object(value, "manifest descriptor")?;
    Ok(super::manifest::ManifestDescriptor {
        media_type: optional_string(obj, "mediaType"),
        digest: required_string(obj, "digest")?,
        size: required_u64(obj, "size")?,
        platform: obj.get("platform").map(parse_platform).transpose()?,
    })
}

#[cfg(feature = "json")]
fn parse_platform(value: &JsonValue) -> Result<super::manifest::PlatformDescriptor, String> {
    let obj = object(value, "platform")?;
    Ok(super::manifest::PlatformDescriptor {
        architecture: optional_string(obj, "architecture"),
        os: optional_string(obj, "os"),
    })
}

#[cfg(feature = "json")]
fn parse_single_manifest_value(value: &JsonValue) -> Result<SingleManifest, String> {
    let obj = object(value, "single manifest")?;
    Ok(SingleManifest {
        config_digest: parse_config_digest(obj.get("config"))?,
        layers: parse_array_required(obj, "layers", parse_layer_descriptor)?,
    })
}

#[cfg(feature = "json")]
fn parse_config_digest(value: Option<&JsonValue>) -> Result<String, String> {
    match value {
        Some(JsonValue::Object(obj)) => required_string(obj, "digest"),
        Some(JsonValue::String(value)) => Ok(value.clone()),
        Some(_) => Err("manifest config must be an object or string".to_string()),
        None => Err("missing manifest config".to_string()),
    }
}

#[cfg(feature = "json")]
fn parse_layer_descriptor(value: &JsonValue) -> Result<super::manifest::LayerDescriptor, String> {
    let obj = object(value, "layer descriptor")?;
    Ok(super::manifest::LayerDescriptor {
        media_type: optional_string(obj, "mediaType"),
        digest: required_string(obj, "digest")?,
        size: required_u64(obj, "size")?,
    })
}

#[cfg(feature = "json")]
fn object<'a>(value: &'a JsonValue, name: &str) -> Result<&'a Map, String> {
    match value {
        JsonValue::Object(obj) => Ok(obj),
        _ => Err(format!("{name} must be an object")),
    }
}

#[cfg(feature = "json")]
fn optional_string(obj: &Map, key: &str) -> Option<String> {
    match obj.get(key) {
        Some(JsonValue::String(value)) => Some(value.clone()),
        _ => None,
    }
}

#[cfg(all(feature = "json", not(feature = "serde")))]
fn optional_string_any(obj: &Map, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| optional_string(obj, key))
}

#[cfg(feature = "json")]
fn required_string(obj: &Map, key: &str) -> Result<String, String> {
    optional_string(obj, key).ok_or_else(|| format!("missing string field {key}"))
}

#[cfg(all(feature = "json", not(feature = "serde")))]
fn optional_bool(obj: &Map, key: &str) -> Option<bool> {
    match obj.get(key) {
        Some(JsonValue::Bool(value)) => Some(*value),
        _ => None,
    }
}

#[cfg(feature = "json")]
fn required_u64(obj: &Map, key: &str) -> Result<u64, String> {
    match obj.get(key) {
        Some(JsonValue::Number(value)) => value
            .as_u64()
            .ok_or_else(|| format!("field {key} must be an unsigned integer")),
        _ => Err(format!("missing unsigned integer field {key}")),
    }
}

#[cfg(all(feature = "json", not(feature = "serde")))]
fn parse_array<T>(
    value: Option<&JsonValue>,
    parse_item: fn(&JsonValue) -> Result<T, String>,
) -> Result<Option<Vec<T>>, String> {
    match value {
        Some(JsonValue::Array(items)) => items
            .iter()
            .map(parse_item)
            .collect::<Result<Vec<_>, _>>()
            .map(Some),
        Some(_) => Err("expected array".to_string()),
        None => Ok(None),
    }
}

#[cfg(feature = "json")]
fn parse_array_required<T>(
    obj: &Map,
    key: &str,
    parse_item: fn(&JsonValue) -> Result<T, String>,
) -> Result<Vec<T>, String> {
    match obj.get(key) {
        Some(JsonValue::Array(items)) => items.iter().map(parse_item).collect(),
        Some(_) => Err(format!("field {key} must be an array")),
        None => Err(format!("missing array field {key}")),
    }
}

#[cfg(all(feature = "json", not(feature = "serde")))]
fn parse_string_array_any(obj: &Map, keys: &[&str]) -> Result<Option<Vec<String>>, String> {
    for key in keys {
        if let Some(value) = obj.get(key) {
            return parse_string_array_value(value).map(Some);
        }
    }
    Ok(None)
}

#[cfg(all(feature = "json", not(feature = "serde")))]
fn parse_string_array_required(obj: &Map, key: &str) -> Result<Vec<String>, String> {
    match obj.get(key) {
        Some(value) => parse_string_array_value(value),
        None => Err(format!("missing string array field {key}")),
    }
}

#[cfg(all(feature = "json", not(feature = "serde")))]
fn parse_string_array_value(value: &JsonValue) -> Result<Vec<String>, String> {
    match value {
        JsonValue::Array(items) => items
            .iter()
            .map(|item| match item {
                JsonValue::String(value) => Ok(value.clone()),
                _ => Err("array item must be a string".to_string()),
            })
            .collect(),
        _ => Err("expected string array".to_string()),
    }
}

#[cfg(all(feature = "json", not(feature = "serde")))]
fn parse_string_map_any(
    obj: &Map,
    keys: &[&str],
) -> Result<Option<BTreeMap<String, String>>, String> {
    for key in keys {
        if let Some(value) = obj.get(key) {
            return parse_string_map_value(value).map(Some);
        }
    }
    Ok(None)
}

#[cfg(all(feature = "json", not(feature = "serde")))]
fn parse_string_map_value(value: &JsonValue) -> Result<BTreeMap<String, String>, String> {
    let obj = object(value, "string map")?;
    let mut out = BTreeMap::new();
    for (key, value) in obj.iter() {
        if let JsonValue::String(value) = value {
            out.insert(key.clone(), value.clone());
        }
    }
    Ok(out)
}

#[cfg(all(feature = "json", not(feature = "serde")))]
fn parse_json_object_map_any(
    obj: &Map,
    keys: &[&str],
) -> Result<Option<BTreeMap<String, JsonValue>>, String> {
    for key in keys {
        if let Some(value) = obj.get(key) {
            let fields = object(value, key)?;
            let mut out = BTreeMap::new();
            for (field_key, field_value) in fields.iter() {
                out.insert(field_key.clone(), field_value.clone());
            }
            return Ok(Some(out));
        }
    }
    Ok(None)
}

#[cfg(all(test, feature = "json", not(feature = "serde")))]
mod json_feature_tests {
    use super::*;

    #[test]
    fn parses_image_config_without_serde() {
        let config = parse_image_config(
            br#"{
                "architecture":"amd64",
                "os":"linux",
                "config":{
                    "User":"1000:1001",
                    "Env":["PATH=/bin","A=B"],
                    "Entrypoint":["/init"],
                    "Cmd":["--serve"],
                    "WorkingDir":"/app",
                    "Volumes":{"/data":{}},
                    "Labels":{"org.opencontainers.image.title":"demo"},
                    "StopSignal":"SIGTERM"
                },
                "rootfs":{"type":"layers","diff_ids":["sha256:a"]},
                "history":[{"created_by":"test","empty_layer":true}]
            }"#,
        )
        .unwrap();

        assert_eq!(config.architecture.as_deref(), Some("amd64"));
        let inner = config.config.unwrap();
        assert_eq!(inner.user.as_deref(), Some("1000:1001"));
        assert_eq!(inner.entrypoint.unwrap(), vec!["/init"]);
        assert_eq!(inner.cmd.unwrap(), vec!["--serve"]);
        assert!(inner.volumes.unwrap().contains_key("/data"));
        assert_eq!(config.rootfs.unwrap().diff_ids, vec!["sha256:a"]);
        assert_eq!(config.history.unwrap()[0].empty_layer, Some(true));
    }

    #[test]
    fn parses_single_manifest_descriptor_config_without_serde() {
        let manifest = parse_single_manifest(
            br#"{
                "schemaVersion":2,
                "config":{"mediaType":"application/vnd.oci.image.config.v1+json","digest":"sha256:cfg","size":42},
                "layers":[{"mediaType":"application/vnd.oci.image.layer.v1.tar","digest":"sha256:layer","size":7}]
            }"#,
        )
        .unwrap();

        assert_eq!(manifest.config_digest, "sha256:cfg");
        assert_eq!(manifest.layers[0].digest, "sha256:layer");
    }

    #[test]
    fn parses_image_index_without_serde() {
        let manifest = parse_manifest(
            br#"{
                "schemaVersion":2,
                "mediaType":"application/vnd.oci.image.index.v1+json",
                "manifests":[{
                    "mediaType":"application/vnd.oci.image.manifest.v1+json",
                    "digest":"sha256:m",
                    "size":10,
                    "platform":{"architecture":"amd64","os":"linux"}
                }]
            }"#,
        )
        .unwrap();

        match manifest {
            ImageManifest::Index(index) => {
                assert_eq!(index.manifests[0].digest, "sha256:m");
                assert_eq!(
                    index.manifests[0]
                        .platform
                        .as_ref()
                        .unwrap()
                        .architecture
                        .as_deref(),
                    Some("amd64")
                );
            }
            ImageManifest::Single(_) => panic!("expected image index"),
        }
    }
}
