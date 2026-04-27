//! no_std OCI layer streaming pipeline contracts.

use crate::image_plan::{validate_digest_reference, ImagePlanError};
use crate::prelude::*;
use crate::registry::manifest::LayerDescriptor;
use core::fmt;
use edgerun_crypto::digest::Digest as _;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayerPipelineError {
    InvalidDescriptor(ImagePlanError),
    UnsupportedDigestAlgorithm(String),
    SizeMismatch { expected: u64, actual: u64 },
    DigestMismatch { expected: String, actual: String },
    Sink(String),
}

impl fmt::Display for LayerPipelineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDescriptor(error) => write!(f, "invalid layer descriptor: {error}"),
            Self::UnsupportedDigestAlgorithm(algorithm) => {
                write!(f, "unsupported digest algorithm: {algorithm}")
            }
            Self::SizeMismatch { expected, actual } => {
                write!(f, "layer size mismatch: expected {expected}, got {actual}")
            }
            Self::DigestMismatch { expected, actual } => {
                write!(
                    f,
                    "layer digest mismatch: expected {expected}, got {actual}"
                )
            }
            Self::Sink(error) => write!(f, "layer sink failed: {error}"),
        }
    }
}

impl core::error::Error for LayerPipelineError {}

pub trait LayerDigest {
    fn algorithm(&self) -> &'static str;
    fn update(&mut self, chunk: &[u8]);
    fn finish(self) -> Vec<u8>;
}

#[derive(Debug, Clone, Default)]
pub struct Sha256LayerDigest {
    hasher: edgerun_crypto::Sha256,
}

impl LayerDigest for Sha256LayerDigest {
    fn algorithm(&self) -> &'static str {
        "sha256"
    }

    fn update(&mut self, chunk: &[u8]) {
        self.hasher.update(chunk);
    }

    fn finish(self) -> Vec<u8> {
        self.hasher.finalize().to_vec()
    }
}

pub fn sha256_layer_digest() -> Sha256LayerDigest {
    Sha256LayerDigest::default()
}

pub trait LayerSink {
    fn write_chunk(&mut self, descriptor: &LayerDescriptor, chunk: &[u8]) -> Result<(), String>;

    fn finish_layer(&mut self, _descriptor: &LayerDescriptor) -> Result<(), String> {
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerApplyReport {
    pub digest: String,
    pub bytes_written: u64,
    pub media_type: Option<String>,
}

pub fn apply_layer_chunks<'a, I, D, S>(
    descriptor: &LayerDescriptor,
    chunks: I,
    mut digest: D,
    sink: &mut S,
) -> Result<LayerApplyReport, LayerPipelineError>
where
    I: IntoIterator<Item = &'a [u8]>,
    D: LayerDigest,
    S: LayerSink,
{
    validate_layer_descriptor(descriptor, digest.algorithm())?;

    let mut bytes_written = 0u64;
    for chunk in chunks {
        bytes_written = bytes_written.checked_add(chunk.len() as u64).ok_or(
            LayerPipelineError::SizeMismatch {
                expected: descriptor.size,
                actual: u64::MAX,
            },
        )?;
        digest.update(chunk);
        sink.write_chunk(descriptor, chunk)
            .map_err(LayerPipelineError::Sink)?;
    }

    if bytes_written != descriptor.size {
        return Err(LayerPipelineError::SizeMismatch {
            expected: descriptor.size,
            actual: bytes_written,
        });
    }

    let actual_digest = format_digest(digest.algorithm(), &digest.finish());
    if actual_digest != descriptor.digest {
        return Err(LayerPipelineError::DigestMismatch {
            expected: descriptor.digest.clone(),
            actual: actual_digest,
        });
    }

    sink.finish_layer(descriptor)
        .map_err(LayerPipelineError::Sink)?;

    Ok(LayerApplyReport {
        digest: descriptor.digest.clone(),
        bytes_written,
        media_type: descriptor.media_type.clone(),
    })
}

pub fn validate_layer_descriptor(
    descriptor: &LayerDescriptor,
    supported_algorithm: &str,
) -> Result<(), LayerPipelineError> {
    if !validate_digest_reference(&descriptor.digest) {
        return Err(LayerPipelineError::InvalidDescriptor(
            ImagePlanError::InvalidDigest {
                field: "layer.digest".into(),
                digest: descriptor.digest.clone(),
            },
        ));
    }

    let algorithm = descriptor
        .digest
        .split_once(':')
        .map(|(algorithm, _)| algorithm)
        .unwrap_or_default();
    if algorithm != supported_algorithm {
        return Err(LayerPipelineError::UnsupportedDigestAlgorithm(
            algorithm.into(),
        ));
    }

    Ok(())
}

pub fn format_digest(algorithm: &str, bytes: &[u8]) -> String {
    format!("{algorithm}:{}", bytes_to_hex(bytes))
}

pub fn bytes_to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

    #[derive(Default)]
    struct TestDigest {
        bytes: Vec<u8>,
    }

    impl LayerDigest for TestDigest {
        fn algorithm(&self) -> &'static str {
            "sha256"
        }

        fn update(&mut self, chunk: &[u8]) {
            self.bytes.extend_from_slice(chunk);
        }

        fn finish(self) -> Vec<u8> {
            let mut out = [0u8; 32];
            for (index, byte) in self.bytes.iter().enumerate() {
                out[index % 32] = out[index % 32].wrapping_add(*byte);
            }
            out.to_vec()
        }
    }

    #[derive(Default)]
    struct VecSink {
        bytes: Vec<u8>,
        finished: bool,
    }

    impl LayerSink for VecSink {
        fn write_chunk(
            &mut self,
            _descriptor: &LayerDescriptor,
            chunk: &[u8],
        ) -> Result<(), String> {
            self.bytes.extend_from_slice(chunk);
            Ok(())
        }

        fn finish_layer(&mut self, _descriptor: &LayerDescriptor) -> Result<(), String> {
            self.finished = true;
            Ok(())
        }
    }

    fn digest_for(bytes: &[u8]) -> String {
        let mut digest = TestDigest::default();
        digest.update(bytes);
        format!("sha256:{}", bytes_to_hex(&digest.finish()))
    }

    #[test]
    fn streams_layer_chunks_into_sink() {
        let data = b"hello layer";
        let descriptor = LayerDescriptor {
            media_type: Some("application/vnd.oci.image.layer.v1.tar".into()),
            digest: digest_for(data),
            size: data.len() as u64,
        };
        let mut sink = VecSink::default();
        let chunks: [&[u8]; 2] = [&data[..5], &data[5..]];

        let report =
            apply_layer_chunks(&descriptor, chunks, TestDigest::default(), &mut sink).unwrap();

        assert_eq!(sink.bytes, data);
        assert!(sink.finished);
        assert_eq!(report.bytes_written, data.len() as u64);
        assert_eq!(report.digest, descriptor.digest);
    }

    #[test]
    fn rejects_size_mismatch() {
        let descriptor = LayerDescriptor {
            media_type: None,
            digest: digest_for(b"abc"),
            size: 4,
        };
        let mut sink = VecSink::default();

        let error = apply_layer_chunks(
            &descriptor,
            [b"abc".as_slice()],
            TestDigest::default(),
            &mut sink,
        )
        .unwrap_err();

        assert_eq!(
            error,
            LayerPipelineError::SizeMismatch {
                expected: 4,
                actual: 3
            }
        );
    }

    #[test]
    fn rejects_digest_mismatch() {
        let descriptor = LayerDescriptor {
            media_type: None,
            digest: digest_for(b"abc"),
            size: 3,
        };
        let mut sink = VecSink::default();

        let error = apply_layer_chunks(
            &descriptor,
            [b"abd".as_slice()],
            TestDigest::default(),
            &mut sink,
        )
        .unwrap_err();

        assert!(matches!(error, LayerPipelineError::DigestMismatch { .. }));
    }
}
