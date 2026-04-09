//! Image manifest types.

/// An image manifest — either a single manifest or an index/manifest list.
#[derive(Debug)]
pub enum ImageManifest {
    Single(SingleManifest),
    Index(ImageIndex),
}

/// A single image manifest with config digest and layers.
#[derive(Debug)]
pub struct SingleManifest {
    pub config_digest: String,
    pub layers: Vec<LayerDescriptor>,
}

/// An image index (manifest list).
#[derive(Debug)]
pub struct ImageIndex {
    pub media_type: Option<String>,
    pub manifests: Vec<ManifestDescriptor>,
}

/// A descriptor pointing to a specific manifest in an index.
#[derive(Debug)]
pub struct ManifestDescriptor {
    pub media_type: Option<String>,
    pub digest: String,
    pub size: u64,
    pub platform: Option<PlatformDescriptor>,
}

/// Platform descriptor (arch, os).
#[derive(Debug)]
pub struct PlatformDescriptor {
    pub architecture: Option<String>,
    pub os: Option<String>,
}

/// A layer descriptor with digest and size.
#[derive(Debug)]
pub struct LayerDescriptor {
    pub media_type: Option<String>,
    pub digest: String,
    pub size: u64,
}
