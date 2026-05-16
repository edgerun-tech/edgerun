use crate::prelude::*;

use super::image_ref::ImageRef;
use super::manifest::{LayerDescriptor, SingleManifest};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageProvenance {
    pub schema_version: u32,
    pub image: String,
    pub registry: String,
    pub repository: String,
    pub reference: String,
    pub reference_kind: String,
    pub config_digest: String,
    pub layers: Vec<ImageLayerProvenance>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageLayerProvenance {
    pub digest: String,
    pub size: u64,
    pub media_type: Option<String>,
}

impl ImageProvenance {
    pub fn from_manifest(image: &ImageRef, manifest: &SingleManifest) -> Self {
        Self {
            schema_version: 1,
            image: image.to_string(),
            registry: image.registry.clone(),
            repository: image.repository.clone(),
            reference: image.reference().to_string(),
            reference_kind: if image.is_digest_reference() {
                "digest".into()
            } else {
                "tag".into()
            },
            config_digest: manifest.config_digest.clone(),
            layers: manifest
                .layers
                .iter()
                .map(ImageLayerProvenance::from_layer)
                .collect(),
        }
    }
}

impl ImageLayerProvenance {
    fn from_layer(layer: &LayerDescriptor) -> Self {
        Self {
            digest: layer.digest.clone(),
            size: layer.size,
            media_type: layer.media_type.clone(),
        }
    }
}

edgerun_json::impl_json_struct! {
    ImageProvenance {
        required {
            schema_version: "schemaVersion" => u32,
            image: "image" => String,
            registry: "registry" => String,
            repository: "repository" => String,
            reference: "reference" => String,
            reference_kind: "referenceKind" => String,
            config_digest: "configDigest" => String,
            layers: "layers" => Vec<ImageLayerProvenance>,
        }
        optional {}
    }
}

edgerun_json::impl_json_struct! {
    ImageLayerProvenance {
        required {
            digest: "digest" => String,
            size: "size" => u64,
        }
        optional {
            media_type: "mediaType" => String,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::manifest::SingleManifest;

    #[test]
    fn provenance_records_digest_reference_and_layers() {
        let image: ImageRef =
            "alpine@sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                .parse()
                .unwrap();
        let manifest = SingleManifest {
            config_digest:
                "sha256:1111111111111111111111111111111111111111111111111111111111111111".into(),
            config_size: Some(1),
            config_media_type: None,
            layers: vec![LayerDescriptor {
                media_type: Some("application/vnd.oci.image.layer.v1.tar".into()),
                digest: "sha256:2222222222222222222222222222222222222222222222222222222222222222"
                    .into(),
                size: 42,
            }],
        };

        let provenance = ImageProvenance::from_manifest(&image, &manifest);

        assert_eq!(provenance.schema_version, 1);
        assert_eq!(provenance.reference_kind, "digest");
        assert_eq!(provenance.layers.len(), 1);
        assert_eq!(provenance.layers[0].size, 42);
    }
}
