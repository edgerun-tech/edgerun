//! OCI image manifest, index, platform, and descriptor helpers.

use alloc::string::String;
use alloc::vec::Vec;

/// An image manifest, either a single manifest or an index/manifest list.
#[derive(Debug, Clone)]
pub enum ImageManifest {
    Single(SingleManifest),
    Index(ImageIndex),
}

/// A single image manifest with config digest and layers.
#[derive(Debug, Clone)]
pub struct SingleManifest {
    pub config_digest: String,
    pub config_size: Option<u64>,
    pub config_media_type: Option<String>,
    pub layers: Vec<LayerDescriptor>,
}

/// An image index (manifest list).
#[derive(Debug, Clone)]
pub struct ImageIndex {
    pub media_type: Option<String>,
    pub manifests: Vec<ManifestDescriptor>,
}

/// A descriptor pointing to a specific manifest in an index.
#[derive(Debug, Clone)]
pub struct ManifestDescriptor {
    pub media_type: Option<String>,
    pub digest: String,
    pub size: u64,
    pub platform: Option<PlatformDescriptor>,
}

/// Platform descriptor (arch, os).
#[derive(Debug, Clone)]
pub struct PlatformDescriptor {
    pub architecture: Option<String>,
    pub os: Option<String>,
}

/// A layer descriptor with digest and size.
#[derive(Debug, Clone)]
pub struct LayerDescriptor {
    pub media_type: Option<String>,
    pub digest: String,
    pub size: u64,
}

pub fn single_manifest(manifest: &ImageManifest) -> Option<&SingleManifest> {
    match manifest {
        ImageManifest::Single(manifest) => Some(manifest),
        ImageManifest::Index(_) => None,
    }
}

pub fn select_manifest_for_target<'a>(
    index: &'a ImageIndex,
    os: &str,
    arch: &str,
) -> Option<&'a ManifestDescriptor> {
    index.manifests.iter().find(|manifest| {
        manifest
            .platform
            .as_ref()
            .map(|platform| platform_matches(platform, os, arch))
            .unwrap_or(false)
    })
}

pub fn selected_manifest_digest_from_index_bytes(
    index_json: &[u8],
    os: &str,
    arch: &str,
) -> Result<String, String> {
    match super::config::parse_manifest(index_json)? {
        ImageManifest::Index(index) => select_manifest_for_target(&index, os, arch)
            .map(|manifest| manifest.digest.clone())
            .ok_or_else(|| alloc::format!("no manifest found for platform {os}/{arch}")),
        ImageManifest::Single(_) => Err("manifest is an index, not a single manifest".into()),
    }
}

pub fn platform_matches(platform: &PlatformDescriptor, os: &str, arch: &str) -> bool {
    let os_matches = platform
        .os
        .as_deref()
        .map(|value| value == os)
        .unwrap_or(true);
    let arch_matches = platform
        .architecture
        .as_deref()
        .map(|value| value == arch)
        .unwrap_or(true);
    os_matches && arch_matches
}

pub fn validate_digest_reference(digest: &str) -> bool {
    let Some((algorithm, hex)) = digest.split_once(':') else {
        return false;
    };

    !algorithm.is_empty()
        && algorithm.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_' || byte == b'-'
        })
        && matches!(hex.len(), 64 | 128)
        && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;
    use alloc::vec;

    #[test]
    fn digest_reference_validation() {
        assert!(validate_digest_reference(
            "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        ));
        assert!(!validate_digest_reference("sha256:not-hex"));
        assert!(!validate_digest_reference("missing-colon"));
    }

    #[test]
    fn platform_matching_defaults_missing_fields_to_wildcard() {
        let platform = PlatformDescriptor {
            architecture: Some("x86_64".to_string()),
            os: None,
        };
        assert!(platform_matches(&platform, "linux", "x86_64"));
        assert!(!platform_matches(&platform, "linux", "aarch64"));
    }

    #[test]
    fn selects_manifest_for_platform() {
        let index = ImageIndex {
            media_type: None,
            manifests: vec![ManifestDescriptor {
                media_type: None,
                digest: "sha256:abc".to_string(),
                size: 1,
                platform: Some(PlatformDescriptor {
                    architecture: Some("x86_64".to_string()),
                    os: Some("linux".to_string()),
                }),
            }],
        };
        assert_eq!(
            select_manifest_for_target(&index, "linux", "x86_64").map(|m| m.digest.as_str()),
            Some("sha256:abc")
        );
    }
}
