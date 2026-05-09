//! OCI layer compression detection and decoding.

use crate::prelude::*;
use crate::tar_layer::TarLayerApplyError;
use edgerun_encoding::crc32::crc32;

pub use edgerun_protocols::oci::layer::{OciLayerCompression, layer_compression};

#[cfg(feature = "gzip")]
pub fn decompress_gzip_layer(data: &[u8]) -> Result<Vec<u8>, TarLayerApplyError> {
    if data.len() < 18 || data[0] != 0x1f || data[1] != 0x8b || data[2] != 8 {
        return Err(TarLayerApplyError::Decompress("invalid gzip header".into()));
    }

    let flags = data[3];
    if flags & 0xe0 != 0 {
        return Err(TarLayerApplyError::Decompress(
            "reserved gzip flags are set".into(),
        ));
    }

    let mut offset = 10usize;
    if flags & 0x04 != 0 {
        if offset + 2 > data.len() {
            return Err(TarLayerApplyError::Decompress(
                "truncated gzip extra field".into(),
            ));
        }
        let extra_len = u16::from_le_bytes([data[offset], data[offset + 1]]) as usize;
        offset = offset
            .checked_add(2 + extra_len)
            .ok_or_else(|| TarLayerApplyError::Decompress("gzip extra field overflow".into()))?;
    }
    if flags & 0x08 != 0 {
        offset = skip_gzip_zero_terminated(data, offset, "name")?;
    }
    if flags & 0x10 != 0 {
        offset = skip_gzip_zero_terminated(data, offset, "comment")?;
    }
    if flags & 0x02 != 0 {
        offset = offset
            .checked_add(2)
            .ok_or_else(|| TarLayerApplyError::Decompress("gzip header crc overflow".into()))?;
    }
    if offset + 8 > data.len() {
        return Err(TarLayerApplyError::Decompress("truncated gzip body".into()));
    }

    let footer = data.len() - 8;
    let out = miniz_oxide::inflate::decompress_to_vec(&data[offset..footer])
        .map_err(|_| TarLayerApplyError::Decompress("invalid deflate stream".into()))?;
    let expected_crc = u32::from_le_bytes([
        data[footer],
        data[footer + 1],
        data[footer + 2],
        data[footer + 3],
    ]);
    let expected_len = u32::from_le_bytes([
        data[footer + 4],
        data[footer + 5],
        data[footer + 6],
        data[footer + 7],
    ]);

    if expected_crc != crc32(&out) {
        return Err(TarLayerApplyError::Decompress(
            "gzip payload crc mismatch".into(),
        ));
    }
    if expected_len != out.len() as u32 {
        return Err(TarLayerApplyError::Decompress(
            "gzip payload size mismatch".into(),
        ));
    }

    Ok(out)
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

fn skip_gzip_zero_terminated(
    data: &[u8],
    offset: usize,
    field: &str,
) -> Result<usize, TarLayerApplyError> {
    data[offset..]
        .iter()
        .position(|byte| *byte == 0)
        .and_then(|relative| offset.checked_add(relative + 1))
        .ok_or_else(|| TarLayerApplyError::Decompress(format!("truncated gzip {field} field")))
}
