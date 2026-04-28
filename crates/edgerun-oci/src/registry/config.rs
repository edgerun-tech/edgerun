//! Image config types (from the config blob).

use crate::prelude::*;
use crate::util::StringResultExt;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use edgerun_json::{from_json_slice, FromJson, JsonValue, JsonValueError, Map};

use super::manifest::{ImageIndex, ImageManifest, SingleManifest};

/// Parsed image config.
#[derive(Debug, Clone)]
pub struct ImageConfig {
    pub architecture: Option<String>,
    pub os: Option<String>,
    pub config: Option<ImageConfigInner>,
    pub rootfs: Option<RootFs>,
    pub history: Option<Vec<HistoryEntry>>,
}

/// Inner image config (Cmd, Env, WorkingDir, etc.).
#[derive(Debug, Clone)]
pub struct ImageConfigInner {
    pub user: Option<String>,
    pub env: Option<Vec<String>>,
    pub entrypoint: Option<Vec<String>>,
    pub cmd: Option<Vec<String>>,
    pub working_dir: Option<String>,
    pub exposed_ports: Option<BTreeMap<String, edgerun_json::JsonValue>>,
    pub volumes: Option<BTreeMap<String, edgerun_json::JsonValue>>,
    pub labels: Option<BTreeMap<String, String>>,
    pub stop_signal: Option<String>,
}

/// Root filesystem info.
#[derive(Debug, Clone)]
pub struct RootFs {
    pub r#type: String,
    pub diff_ids: Vec<String>,
}

/// History entry (build layer info, from OCI image config).
#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub created: Option<String>,
    pub created_by: Option<String>,
    pub comment: Option<String>,
    pub empty_layer: Option<bool>,
}

// ===========================================================================
// Public API
// ===========================================================================

/// Parse an image config from JSON bytes.
pub fn parse_image_config(data: &[u8]) -> Result<ImageConfig, String> {
    from_json_slice(data).string_err()
}

/// Parse JSON bytes into a JsonValue (for ad-hoc inspection).
pub fn parse_json_bytes(data: &[u8]) -> Result<edgerun_json::JsonValue, String> {
    from_json_slice(data).string_err()
}

// ===========================================================================
// Manifest parsing
// ===========================================================================

/// Parse a raw JSON blob into an ImageManifest (index or single).
pub fn parse_manifest(data: &[u8]) -> Result<ImageManifest, String> {
    let value = parse_json_bytes(data)?;
    if let Some(manifests) = value
        .as_object()
        .and_then(|object| object.get_array("manifests"))
    {
        if !manifests.is_empty() {
            return ImageIndex::from_json(value)
                .map(ImageManifest::Index)
                .string_err();
        }
    }

    SingleManifest::from_json(value)
        .map(ImageManifest::Single)
        .string_err()
}

/// Parse a raw JSON blob as a single manifest.
pub fn parse_single_manifest(data: &[u8]) -> Result<SingleManifest, String> {
    from_json_slice(data).string_err()
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

impl FromJson for ImageConfig {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = object(value, "image config")?;
        Ok(Self {
            architecture: take_optional(&mut object, "architecture")?,
            os: take_optional(&mut object, "os")?,
            config: take_optional(&mut object, "config")?,
            rootfs: take_optional(&mut object, "rootfs")?,
            history: take_optional(&mut object, "history")?,
        })
    }
}

impl FromJson for ImageConfigInner {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = object(value, "image config.config")?;
        Ok(Self {
            user: take_optional_any(&mut object, &["User", "user"])?,
            env: take_optional_any(&mut object, &["Env", "env"])?,
            entrypoint: take_optional_any(&mut object, &["Entrypoint", "entrypoint"])?,
            cmd: take_optional_any(&mut object, &["Cmd", "cmd"])?,
            working_dir: take_optional_any(
                &mut object,
                &["WorkingDir", "workingDir", "working_dir"],
            )?,
            exposed_ports: take_optional_any(&mut object, &["ExposedPorts", "exposedPorts"])?,
            volumes: take_optional_any(&mut object, &["Volumes", "volumes"])?,
            labels: take_optional_any(&mut object, &["Labels", "labels"])?,
            stop_signal: take_optional_any(
                &mut object,
                &["StopSignal", "stopSignal", "stop_signal"],
            )?,
        })
    }
}

impl FromJson for RootFs {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = object(value, "rootfs")?;
        Ok(Self {
            r#type: take_required(&mut object, "type")?,
            diff_ids: take_required(&mut object, "diff_ids")?,
        })
    }
}

impl FromJson for HistoryEntry {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = object(value, "history entry")?;
        Ok(Self {
            created: take_optional(&mut object, "created")?,
            created_by: take_optional(&mut object, "created_by")?,
            comment: take_optional(&mut object, "comment")?,
            empty_layer: take_optional(&mut object, "empty_layer")?,
        })
    }
}

impl FromJson for ImageIndex {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = object(value, "image index")?;
        Ok(Self {
            media_type: take_optional(&mut object, "mediaType")?,
            manifests: take_required(&mut object, "manifests")?,
        })
    }
}

impl FromJson for super::manifest::ManifestDescriptor {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = object(value, "manifest descriptor")?;
        Ok(Self {
            media_type: take_optional(&mut object, "mediaType")?,
            digest: take_required(&mut object, "digest")?,
            size: take_required(&mut object, "size")?,
            platform: take_optional(&mut object, "platform")?,
        })
    }
}

impl FromJson for super::manifest::PlatformDescriptor {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = object(value, "platform")?;
        Ok(Self {
            architecture: take_optional(&mut object, "architecture")?,
            os: take_optional(&mut object, "os")?,
        })
    }
}

impl FromJson for SingleManifest {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = object(value, "single manifest")?;
        Ok(Self {
            config_digest: take_config_digest(&mut object)?,
            layers: take_required(&mut object, "layers")?,
        })
    }
}

fn take_config_digest(object: &mut Map) -> Result<String, JsonValueError> {
    match object.remove("config") {
        Some(JsonValue::Object(mut object)) => take_required(&mut object, "digest"),
        Some(JsonValue::String(value)) => Ok(value),
        Some(other) => Err(JsonValueError::WrongType(format!(
            "manifest config must be an object or string, found {other:?}"
        ))),
        None => Err(JsonValueError::WrongType(
            "missing required field `config`".into(),
        )),
    }
}

impl FromJson for super::manifest::LayerDescriptor {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = object(value, "layer descriptor")?;
        Ok(Self {
            media_type: take_optional(&mut object, "mediaType")?,
            digest: take_required(&mut object, "digest")?,
            size: take_required(&mut object, "size")?,
        })
    }
}

#[cfg(all(test, not(target_os = "none")))]
mod json_feature_tests {
    use super::*;

    #[test]
    fn parses_image_config_with_edgerun_json() {
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
    fn parses_single_manifest_descriptor_config_with_edgerun_json() {
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
    fn parses_image_index_with_edgerun_json() {
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
