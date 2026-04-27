//! no_std OCI image boot planning.

use crate::prelude::*;
use core::fmt;

use crate::registry::config::{parse_image_config, parse_manifest, ImageConfig};
use crate::registry::manifest::{
    ImageIndex, ImageManifest, LayerDescriptor, ManifestDescriptor, PlatformDescriptor,
    SingleManifest,
};
use crate::registry::oci_spec::generate_oci_spec_model;
use crate::runtime_config::BareRuntimeConfig;
use crate::validate::{host_arch, host_os, OciValidationError};

/// A host-independent plan for booting an OCI image.
///
/// The plan assumes manifest and config bytes have already been fetched and
/// verified by platform code. It keeps layer descriptors and process metadata
/// together without depending on registry networking, archive extraction, or
/// Linux process setup.
#[derive(Debug, Clone)]
pub struct BareImagePlan {
    pub config_digest: String,
    pub layers: Vec<LayerDescriptor>,
    pub diff_ids: Vec<String>,
    pub image_os: Option<String>,
    pub image_arch: Option<String>,
    pub runtime: BareRuntimeConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImagePlanError {
    ParseManifest(String),
    ParseImageConfig(String),
    InvalidDigest { field: String, digest: String },
    ManifestIndex,
    PlatformNotFound { os: String, arch: String },
    Validation(OciValidationError),
}

impl fmt::Display for ImagePlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ParseManifest(error) => write!(f, "failed to parse image manifest: {error}"),
            Self::ParseImageConfig(error) => write!(f, "failed to parse image config: {error}"),
            Self::InvalidDigest { field, digest } => {
                write!(f, "invalid digest in {field}: {digest}")
            }
            Self::ManifestIndex => f.write_str("manifest is an index, not a single manifest"),
            Self::PlatformNotFound { os, arch } => {
                write!(f, "no manifest found for platform {os}/{arch}")
            }
            Self::Validation(error) => write!(f, "invalid OCI runtime config: {error}"),
        }
    }
}

impl core::error::Error for ImagePlanError {}

impl From<OciValidationError> for ImagePlanError {
    fn from(error: OciValidationError) -> Self {
        Self::Validation(error)
    }
}

impl BareImagePlan {
    pub fn from_manifest_config(
        manifest: &SingleManifest,
        image_config: &ImageConfig,
        rootfs: &str,
    ) -> Result<Self, OciValidationError> {
        let spec = generate_oci_spec_model(image_config, rootfs);
        let runtime = BareRuntimeConfig::from_spec(&spec)?;

        Ok(Self {
            config_digest: manifest.config_digest.clone(),
            layers: manifest.layers.clone(),
            diff_ids: image_config
                .rootfs
                .as_ref()
                .map(|rootfs| rootfs.diff_ids.clone())
                .unwrap_or_default(),
            image_os: image_config.os.clone(),
            image_arch: image_config.architecture.clone(),
            runtime,
        })
    }

    pub fn from_manifest_config_bytes(
        manifest_json: &[u8],
        image_config_json: &[u8],
        rootfs: &str,
    ) -> Result<Self, ImagePlanError> {
        let manifest =
            parse_single_manifest_bytes(manifest_json).map_err(ImagePlanError::ParseManifest)?;
        let image_config =
            parse_image_config(image_config_json).map_err(ImagePlanError::ParseImageConfig)?;
        Self::from_manifest_config(&manifest, &image_config, rootfs).map_err(Into::into)
    }

    pub fn layer_digests(&self) -> impl ExactSizeIterator<Item = &str> {
        self.layers.iter().map(|layer| layer.digest.as_str())
    }

    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }

    pub fn uncompressed_layer_count(&self) -> usize {
        self.diff_ids.len()
    }

    pub fn has_layer_count_mismatch(&self) -> bool {
        !self.diff_ids.is_empty() && self.layers.len() != self.diff_ids.len()
    }

    pub fn validate_descriptors(&self) -> Result<(), ImagePlanError> {
        validate_digest_field("config.digest", &self.config_digest)?;
        for (index, layer) in self.layers.iter().enumerate() {
            validate_digest_field(format!("layers[{index}].digest"), &layer.digest)?;
        }
        for (index, diff_id) in self.diff_ids.iter().enumerate() {
            validate_digest_field(format!("rootfs.diff_ids[{index}]"), diff_id)?;
        }
        Ok(())
    }
}

pub fn parse_single_manifest_bytes(data: &[u8]) -> Result<SingleManifest, String> {
    match parse_manifest(data)? {
        ImageManifest::Single(manifest) => Ok(manifest),
        ImageManifest::Index(_) => Err("manifest is an index, not a single manifest".into()),
    }
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

pub fn select_manifest_for_current_target(index: &ImageIndex) -> Option<&ManifestDescriptor> {
    select_manifest_for_target(index, host_os(), host_arch())
}

pub fn selected_manifest_digest_from_index_bytes(
    index_json: &[u8],
    os: &str,
    arch: &str,
) -> Result<String, ImagePlanError> {
    match parse_manifest(index_json).map_err(ImagePlanError::ParseManifest)? {
        ImageManifest::Index(index) => select_manifest_for_target(&index, os, arch)
            .map(|manifest| manifest.digest.clone())
            .ok_or_else(|| ImagePlanError::PlatformNotFound {
                os: os.into(),
                arch: arch.into(),
            }),
        ImageManifest::Single(_) => Err(ImagePlanError::ManifestIndex),
    }
}

pub fn selected_manifest_digest_for_current_target(
    index_json: &[u8],
) -> Result<String, ImagePlanError> {
    selected_manifest_digest_from_index_bytes(index_json, host_os(), host_arch())
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

fn validate_digest_field(field: impl Into<String>, digest: &str) -> Result<(), ImagePlanError> {
    if validate_digest_reference(digest) {
        Ok(())
    } else {
        Err(ImagePlanError::InvalidDigest {
            field: field.into(),
            digest: digest.into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::config::{ImageConfigInner, RootFs};

    fn image_config() -> ImageConfig {
        ImageConfig {
            architecture: Some("amd64".into()),
            os: Some("linux".into()),
            config: Some(ImageConfigInner {
                user: None,
                env: Some(vec!["PATH=/bin".into()]),
                entrypoint: Some(vec!["/init".into()]),
                cmd: None,
                working_dir: None,
                exposed_ports: None,
                volumes: None,
                labels: None,
                stop_signal: None,
            }),
            rootfs: Some(RootFs {
                r#type: "layers".into(),
                diff_ids: vec!["sha256:diff".into()],
            }),
            history: None,
        }
    }

    fn manifest() -> SingleManifest {
        SingleManifest {
            config_digest: "sha256:config".into(),
            layers: vec![LayerDescriptor {
                media_type: Some("application/vnd.oci.image.layer.v1.tar".into()),
                digest: "sha256:layer".into(),
                size: 12,
            }],
        }
    }

    #[test]
    fn builds_bare_image_plan() {
        let plan =
            BareImagePlan::from_manifest_config(&manifest(), &image_config(), "/rootfs").unwrap();

        assert_eq!(plan.config_digest, "sha256:config");
        assert_eq!(
            plan.layer_digests().collect::<Vec<_>>(),
            vec!["sha256:layer"]
        );
        assert_eq!(plan.diff_ids, vec!["sha256:diff"]);
        assert_eq!(plan.runtime.rootfs, "/rootfs");
        assert_eq!(plan.runtime.args, vec!["/init"]);
        assert!(!plan.has_layer_count_mismatch());
    }

    #[test]
    fn validates_digest_references() {
        assert!(validate_digest_reference(
            "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        ));
        assert!(!validate_digest_reference("sha256:layer"));
        assert!(!validate_digest_reference(
            "SHA256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        ));
        assert!(!validate_digest_reference("missing-separator"));
    }

    #[test]
    fn validates_plan_descriptors() {
        let mut manifest = manifest();
        manifest.config_digest =
            "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".into();
        manifest.layers[0].digest =
            "sha256:abcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcd".into();
        let mut config = image_config();
        config.rootfs.as_mut().unwrap().diff_ids[0] =
            "sha256:1111111111111111111111111111111111111111111111111111111111111111".into();
        let plan = BareImagePlan::from_manifest_config(&manifest, &config, "/rootfs").unwrap();

        assert!(plan.validate_descriptors().is_ok());
    }

    #[test]
    fn rejects_bad_plan_digest() {
        let plan =
            BareImagePlan::from_manifest_config(&manifest(), &image_config(), "/rootfs").unwrap();

        let error = plan.validate_descriptors().unwrap_err();

        assert_eq!(
            error,
            ImagePlanError::InvalidDigest {
                field: "config.digest".into(),
                digest: "sha256:config".into()
            }
        );
    }

    #[test]
    fn detects_layer_count_mismatch() {
        let mut config = image_config();
        config
            .rootfs
            .as_mut()
            .unwrap()
            .diff_ids
            .push("sha256:extra".into());
        let plan = BareImagePlan::from_manifest_config(&manifest(), &config, "/rootfs").unwrap();

        assert!(plan.has_layer_count_mismatch());
    }

    #[test]
    fn selects_manifest_for_platform() {
        let index = ImageIndex {
            media_type: None,
            manifests: vec![
                ManifestDescriptor {
                    media_type: None,
                    digest: "sha256:arm".into(),
                    size: 1,
                    platform: Some(PlatformDescriptor {
                        architecture: Some("arm64".into()),
                        os: Some("linux".into()),
                    }),
                },
                ManifestDescriptor {
                    media_type: None,
                    digest: "sha256:amd".into(),
                    size: 1,
                    platform: Some(PlatformDescriptor {
                        architecture: Some("amd64".into()),
                        os: Some("linux".into()),
                    }),
                },
            ],
        };

        let selected = select_manifest_for_target(&index, "linux", "amd64").unwrap();
        assert_eq!(selected.digest, "sha256:amd");
    }

    #[test]
    fn builds_plan_from_manifest_and_config_bytes() {
        let manifest_json = br#"{
            "schemaVersion":2,
            "config":{"digest":"sha256:config","size":10},
            "layers":[{"digest":"sha256:layer","size":12}]
        }"#;
        let config_json = br#"{
            "architecture":"amd64",
            "os":"linux",
            "config":{"Entrypoint":["/init"],"Env":["PATH=/bin"]},
            "rootfs":{"type":"layers","diff_ids":["sha256:diff"]}
        }"#;

        let plan = BareImagePlan::from_manifest_config_bytes(manifest_json, config_json, "/rootfs")
            .unwrap();

        assert_eq!(plan.config_digest, "sha256:config");
        assert_eq!(plan.layer_count(), 1);
        assert_eq!(plan.runtime.args, vec!["/init"]);
    }

    #[test]
    fn selects_manifest_digest_from_index_bytes() {
        let index_json = br#"{
            "schemaVersion":2,
            "manifests":[
                {"digest":"sha256:arm","size":1,"platform":{"architecture":"arm64","os":"linux"}},
                {"digest":"sha256:amd","size":1,"platform":{"architecture":"amd64","os":"linux"}}
            ]
        }"#;

        let digest =
            selected_manifest_digest_from_index_bytes(index_json, "linux", "amd64").unwrap();

        assert_eq!(digest, "sha256:amd");
    }

    #[test]
    fn missing_platform_reports_target() {
        let index_json = br#"{
            "schemaVersion":2,
            "manifests":[
                {"digest":"sha256:arm","size":1,"platform":{"architecture":"arm64","os":"linux"}}
            ]
        }"#;

        let error =
            selected_manifest_digest_from_index_bytes(index_json, "linux", "amd64").unwrap_err();

        assert_eq!(
            error,
            ImagePlanError::PlatformNotFound {
                os: "linux".into(),
                arch: "amd64".into()
            }
        );
    }
}
