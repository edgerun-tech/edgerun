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

#[derive(Debug, Clone)]
struct ManifestProbe {
    manifests: Option<Vec<JsonValue>>,
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
    let has_index_manifests = ManifestProbe::from_json(value.clone())
        .ok()
        .and_then(|probe| probe.manifests)
        .is_some_and(|manifests| !manifests.is_empty());

    if has_index_manifests {
        return ImageIndex::from_json(value)
            .map(ImageManifest::Index)
            .string_err();
    }

    SingleManifest::from_json(value)
        .map(ImageManifest::Single)
        .string_err()
}

/// Parse a raw JSON blob as a single manifest.
pub fn parse_single_manifest(data: &[u8]) -> Result<SingleManifest, String> {
    from_json_slice(data).string_err()
}

edgerun_json::impl_json_struct! {
    ManifestProbe {
        required {}
        optional { manifests: "manifests" => Vec<JsonValue> }
    }
}

edgerun_json::impl_json_struct! {
    ImageConfig {
        required {}
        optional {
            architecture: "architecture" => String,
            os: "os" => String,
            config: "config" => ImageConfigInner,
            rootfs: "rootfs" => RootFs,
            history: "history" => Vec<HistoryEntry>,
        }
    }
}

edgerun_json::impl_json_struct! {
    ImageConfigInner {
        required {}
        optional {
            user: ["User", "user"] => String,
            env: ["Env", "env"] => Vec<String>,
            entrypoint: ["Entrypoint", "entrypoint"] => Vec<String>,
            cmd: ["Cmd", "cmd"] => Vec<String>,
            working_dir: ["WorkingDir", "workingDir", "working_dir"] => String,
            exposed_ports: ["ExposedPorts", "exposedPorts"] => BTreeMap<String, edgerun_json::JsonValue>,
            volumes: ["Volumes", "volumes"] => BTreeMap<String, edgerun_json::JsonValue>,
            labels: ["Labels", "labels"] => BTreeMap<String, String>,
            stop_signal: ["StopSignal", "stopSignal", "stop_signal"] => String,
        }
    }
}

edgerun_json::impl_json_struct! {
    RootFs {
        required {
            r#type: "type" => String,
            diff_ids: "diff_ids" => Vec<String>,
        }
        optional {}
    }
}

edgerun_json::impl_json_struct! {
    HistoryEntry {
        required {}
        optional {
            created: "created" => String,
            created_by: "created_by" => String,
            comment: "comment" => String,
            empty_layer: "empty_layer" => bool,
        }
    }
}

edgerun_json::impl_json_struct! {
    ImageIndex {
        required { manifests: "manifests" => Vec<super::manifest::ManifestDescriptor> }
        optional { media_type: "mediaType" => String }
    }
}

edgerun_json::impl_json_struct! {
    super::manifest::ManifestDescriptor {
        required {
            digest: "digest" => String,
            size: "size" => u64,
        }
        optional {
            media_type: "mediaType" => String,
            platform: "platform" => super::manifest::PlatformDescriptor,
        }
    }
}

edgerun_json::impl_json_struct! {
    super::manifest::PlatformDescriptor {
        required {}
        optional {
            architecture: "architecture" => String,
            os: "os" => String,
        }
    }
}

impl FromJson for SingleManifest {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("single manifest")?;
        let (config_digest, config_size, config_media_type) = take_config_descriptor(&mut object)?;
        Ok(Self {
            config_digest,
            config_size,
            config_media_type,
            layers: object.take_required("layers")?,
        })
    }
}

fn take_config_descriptor(
    object: &mut Map,
) -> Result<(String, Option<u64>, Option<String>), JsonValueError> {
    match object.remove("config") {
        Some(JsonValue::Object(mut object)) => {
            let digest = object.take_required("digest")?;
            let size = object.take_optional("size")?;
            let media_type = object.take_optional("mediaType")?;
            Ok((digest, size, media_type))
        }
        Some(JsonValue::String(value)) => Ok((value, None, None)),
        Some(other) => Err(JsonValueError::WrongType(format!(
            "manifest config must be an object or string, found {other:?}"
        ))),
        None => Err(JsonValueError::WrongType(
            "missing required field `config`".into(),
        )),
    }
}

edgerun_json::impl_json_struct! {
    super::manifest::LayerDescriptor {
        required {
            digest: "digest" => String,
            size: "size" => u64,
        }
        optional { media_type: "mediaType" => String }
    }
}

#[cfg(all(test, not(target_os = "none")))]
#[path = "../../tests/unit_src/src/registry/config_json_feature_tests.rs"]
mod json_feature_tests;
