//! Image manifest types.

use crate::prelude::*;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use alloc::string::String;
use alloc::vec::Vec;

/// An image manifest — either a single manifest or an index/manifest list.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub enum ImageManifest {
    #[cfg_attr(feature = "serde", serde(rename = "single"))]
    Single(SingleManifest),
    #[cfg_attr(feature = "serde", serde(rename = "index"))]
    Index(ImageIndex),
}

/// A single image manifest with config digest and layers.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct SingleManifest {
    #[cfg_attr(feature = "serde", serde(rename = "config"))]
    pub config_digest: String,
    pub layers: Vec<LayerDescriptor>,
}

/// An image index (manifest list).
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct ImageIndex {
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub media_type: Option<String>,
    pub manifests: Vec<ManifestDescriptor>,
}

/// A descriptor pointing to a specific manifest in an index.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct ManifestDescriptor {
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub media_type: Option<String>,
    pub digest: String,
    pub size: u64,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub platform: Option<PlatformDescriptor>,
}

/// Platform descriptor (arch, os).
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct PlatformDescriptor {
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub architecture: Option<String>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub os: Option<String>,
}

/// A layer descriptor with digest and size.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct LayerDescriptor {
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub media_type: Option<String>,
    pub digest: String,
    pub size: u64,
}
