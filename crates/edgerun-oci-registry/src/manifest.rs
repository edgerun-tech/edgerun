//! Image manifest types.

use serde::{Deserialize, Serialize};

/// An image manifest — either a single manifest or an index/manifest list.
#[derive(Debug, Serialize, Deserialize)]
pub enum ImageManifest {
    #[serde(rename = "single")]
    Single(SingleManifest),
    #[serde(rename = "index")]
    Index(ImageIndex),
}

/// A single image manifest with config digest and layers.
#[derive(Debug, Serialize, Deserialize)]
pub struct SingleManifest {
    #[serde(rename = "config")]
    pub config_digest: String,
    pub layers: Vec<LayerDescriptor>,
}

/// An image index (manifest list).
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageIndex {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
    pub manifests: Vec<ManifestDescriptor>,
}

/// A descriptor pointing to a specific manifest in an index.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestDescriptor {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
    pub digest: String,
    pub size: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<PlatformDescriptor>,
}

/// Platform descriptor (arch, os).
#[derive(Debug, Serialize, Deserialize)]
pub struct PlatformDescriptor {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub architecture: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os: Option<String>,
}

/// A layer descriptor with digest and size.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayerDescriptor {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
    pub digest: String,
    pub size: u64,
}
