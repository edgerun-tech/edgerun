//! OCI image config and manifest parser compatibility re-exports.

pub use edgerun_protocols::oci::config::{
    parse_image_config, parse_json_bytes, parse_manifest, parse_single_manifest, HistoryEntry,
    ImageConfig, ImageConfigInner, RootFs,
};
