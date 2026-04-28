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

impl FromJson for ImageConfig {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("image config")?;
        Ok(Self {
            architecture: object.take_optional("architecture")?,
            os: object.take_optional("os")?,
            config: object.take_optional("config")?,
            rootfs: object.take_optional("rootfs")?,
            history: object.take_optional("history")?,
        })
    }
}

impl FromJson for ImageConfigInner {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("image config.config")?;
        Ok(Self {
            user: object.take_optional_any(&["User", "user"])?,
            env: object.take_optional_any(&["Env", "env"])?,
            entrypoint: object.take_optional_any(&["Entrypoint", "entrypoint"])?,
            cmd: object.take_optional_any(&["Cmd", "cmd"])?,
            working_dir: object.take_optional_any(&["WorkingDir", "workingDir", "working_dir"])?,
            exposed_ports: object.take_optional_any(&["ExposedPorts", "exposedPorts"])?,
            volumes: object.take_optional_any(&["Volumes", "volumes"])?,
            labels: object.take_optional_any(&["Labels", "labels"])?,
            stop_signal: object.take_optional_any(&["StopSignal", "stopSignal", "stop_signal"])?,
        })
    }
}

impl FromJson for RootFs {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("rootfs")?;
        Ok(Self {
            r#type: object.take_required("type")?,
            diff_ids: object.take_required("diff_ids")?,
        })
    }
}

impl FromJson for HistoryEntry {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("history entry")?;
        Ok(Self {
            created: object.take_optional("created")?,
            created_by: object.take_optional("created_by")?,
            comment: object.take_optional("comment")?,
            empty_layer: object.take_optional("empty_layer")?,
        })
    }
}

impl FromJson for ImageIndex {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("image index")?;
        Ok(Self {
            media_type: object.take_optional("mediaType")?,
            manifests: object.take_required("manifests")?,
        })
    }
}

impl FromJson for super::manifest::ManifestDescriptor {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("manifest descriptor")?;
        Ok(Self {
            media_type: object.take_optional("mediaType")?,
            digest: object.take_required("digest")?,
            size: object.take_required("size")?,
            platform: object.take_optional("platform")?,
        })
    }
}

impl FromJson for super::manifest::PlatformDescriptor {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("platform")?;
        Ok(Self {
            architecture: object.take_optional("architecture")?,
            os: object.take_optional("os")?,
        })
    }
}

impl FromJson for SingleManifest {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("single manifest")?;
        Ok(Self {
            config_digest: take_config_digest(&mut object)?,
            layers: object.take_required("layers")?,
        })
    }
}

fn take_config_digest(object: &mut Map) -> Result<String, JsonValueError> {
    match object.remove("config") {
        Some(JsonValue::Object(mut object)) => object.take_required("digest"),
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
        let mut object = value.into_object("layer descriptor")?;
        Ok(Self {
            media_type: object.take_optional("mediaType")?,
            digest: object.take_required("digest")?,
            size: object.take_required("size")?,
        })
    }
}

#[cfg(all(test, not(target_os = "none")))]
#[path = "../../tests/unit_src/src/registry/config_json_feature_tests.rs"]
mod json_feature_tests;
