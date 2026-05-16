//! OCI layer compression detection and decoding.

use crate::prelude::*;
use crate::tar_layer::TarLayerApplyError;

pub use edgerun_protocols::oci::layer::{OciLayerCompression, layer_compression};

#[cfg(feature = "gzip")]
pub fn decompress_gzip_layer(data: &[u8]) -> Result<Vec<u8>, TarLayerApplyError> {
    edgerun_encoding::compression::gzip_decompress(data)
        .map_err(|error| TarLayerApplyError::Decompress(format!("{error:?}")))
}

#[cfg(not(feature = "gzip"))]
pub fn decompress_gzip_layer(_data: &[u8]) -> Result<Vec<u8>, TarLayerApplyError> {
    Err(TarLayerApplyError::UnsupportedMediaType(Some(
        "application/vnd.oci.image.layer.v1.tar+gzip".into(),
    )))
}

#[cfg(feature = "zstd")]
pub fn decompress_zstd_layer(data: &[u8]) -> Result<Vec<u8>, TarLayerApplyError> {
    use ruzstd::decoding::StreamingDecoder;
    use ruzstd::io::Read;

    let mut decoder = StreamingDecoder::new(data)
        .map_err(|error| TarLayerApplyError::Decompress(format!("invalid zstd frame: {error}")))?;
    let mut out = Vec::new();
    decoder
        .read_to_end(&mut out)
        .map_err(|error| TarLayerApplyError::Decompress(format!("invalid zstd stream: {error}")))?;
    Ok(out)
}

#[cfg(not(feature = "zstd"))]
pub fn decompress_zstd_layer(_data: &[u8]) -> Result<Vec<u8>, TarLayerApplyError> {
    Err(TarLayerApplyError::UnsupportedMediaType(Some(
        "application/vnd.oci.image.layer.v1.tar+zstd".into(),
    )))
}
