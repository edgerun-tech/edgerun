use crate::prelude::*;
use core::fmt::Write as _;

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

    pub fn to_json_string_pretty(&self) -> String {
        let mut out = String::new();
        out.push_str("{\n  \"schemaVersion\": ");
        write!(&mut out, "{}", self.schema_version).expect("writing to String cannot fail");
        out.push_str(",\n  \"image\": ");
        write_json_string(&mut out, &self.image);
        out.push_str(",\n  \"registry\": ");
        write_json_string(&mut out, &self.registry);
        out.push_str(",\n  \"repository\": ");
        write_json_string(&mut out, &self.repository);
        out.push_str(",\n  \"reference\": ");
        write_json_string(&mut out, &self.reference);
        out.push_str(",\n  \"referenceKind\": ");
        write_json_string(&mut out, &self.reference_kind);
        out.push_str(",\n  \"configDigest\": ");
        write_json_string(&mut out, &self.config_digest);
        out.push_str(",\n  \"layers\": [");
        for (index, layer) in self.layers.iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            out.push_str("\n    ");
            layer.write_json(&mut out);
        }
        if !self.layers.is_empty() {
            out.push('\n');
            out.push_str("  ");
        }
        out.push_str("]\n}");
        out
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

    fn write_json(&self, out: &mut String) {
        out.push_str("{\"digest\":");
        write_json_string(out, &self.digest);
        out.push_str(",\"size\":");
        write!(out, "{}", self.size).expect("writing to String cannot fail");
        if let Some(media_type) = &self.media_type {
            out.push_str(",\"mediaType\":");
            write_json_string(out, media_type);
        }
        out.push('}');
    }
}

fn write_json_string(out: &mut String, value: &str) {
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch <= '\u{1f}' => {
                write!(out, "\\u{:04x}", ch as u32).expect("writing to String cannot fail");
            }
            ch => out.push(ch),
        }
    }
    out.push('"');
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
