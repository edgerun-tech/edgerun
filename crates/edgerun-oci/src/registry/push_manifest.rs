use crate::prelude::*;

#[derive(Debug, Clone)]
struct PushManifestConfig {
    media_type: String,
    digest: String,
    size: usize,
}

#[derive(Debug, Clone)]
struct PushManifestLayer {
    media_type: String,
    digest: String,
    size: usize,
}

#[derive(Debug, Clone)]
struct PushManifest {
    schema_version: u32,
    media_type: String,
    config: PushManifestConfig,
    layers: Vec<PushManifestLayer>,
}

edgerun_json::impl_json_struct! {
    PushManifestConfig {
        required {
            media_type: "mediaType" => String,
            digest: "digest" => String,
            size: "size" => usize,
        }
        optional {}
    }
}

edgerun_json::impl_json_struct! {
    PushManifestLayer {
        required {
            media_type: "mediaType" => String,
            digest: "digest" => String,
            size: "size" => usize,
        }
        optional {}
    }
}

edgerun_json::impl_json_struct! {
    PushManifest {
        required {
            schema_version: "schemaVersion" => u32,
            media_type: "mediaType" => String,
            config: "config" => PushManifestConfig,
            layers: "layers" => Vec<PushManifestLayer>,
        }
        optional {}
    }
}

pub(crate) fn push_manifest_json(
    config_digest: String,
    config_size: usize,
    layer_digest: String,
    layer_size: usize,
) -> Result<String, edgerun_json::JsonError> {
    edgerun_json::to_json_string(&PushManifest {
        schema_version: 2,
        media_type: "application/vnd.oci.image.manifest.v1+json".into(),
        config: PushManifestConfig {
            media_type: "application/vnd.oci.image.config.v1+json".into(),
            digest: config_digest,
            size: config_size,
        },
        layers: vec![PushManifestLayer {
            media_type: "application/vnd.oci.image.layer.v1.tar+gzip".into(),
            digest: layer_digest,
            size: layer_size,
        }],
    })
}
