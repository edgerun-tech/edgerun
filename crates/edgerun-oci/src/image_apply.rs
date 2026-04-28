//! no_std OCI image layer application helpers.

use crate::image_plan::BareImagePlan;
use crate::layer_pipeline::{
    apply_layer_chunks, format_digest, sha256_layer_digest, LayerDigest, LayerSink,
    Sha256LayerDigest,
};
use crate::prelude::*;
use crate::registry::manifest::LayerDescriptor;
use crate::tar_layer::{
    apply_uncompressed_tar_layer, apply_uncompressed_tar_layer_streaming, layer_compression,
    validate_and_decode_tar_layer, OciLayerCompression, TarLayerApplyError, TarLayerApplyReport,
    TarLayerSink,
};
use core::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BareImageApplyError {
    LayerCountMismatch {
        expected: usize,
        actual: usize,
    },
    Layer {
        index: usize,
        error: TarLayerApplyError,
    },
    DiffIdCountMismatch {
        expected: usize,
        actual: usize,
    },
    UnsupportedDiffIdAlgorithm {
        index: usize,
        expected: String,
        actual: String,
    },
    DiffIdMismatch {
        index: usize,
        expected: String,
        actual: String,
    },
}

impl fmt::Display for BareImageApplyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LayerCountMismatch { expected, actual } => {
                write!(
                    f,
                    "image layer count mismatch: expected {expected}, got {actual}"
                )
            }
            Self::Layer { index, error } => write!(f, "failed to apply layer {index}: {error}"),
            Self::DiffIdCountMismatch { expected, actual } => {
                write!(
                    f,
                    "image diff_id count mismatch: expected {expected}, got {actual}"
                )
            }
            Self::UnsupportedDiffIdAlgorithm {
                index,
                expected,
                actual,
            } => write!(
                f,
                "unsupported diff_id algorithm for layer {index}: expected {expected}, got {actual}"
            ),
            Self::DiffIdMismatch {
                index,
                expected,
                actual,
            } => write!(
                f,
                "uncompressed diff_id mismatch for layer {index}: expected {expected}, got {actual}"
            ),
        }
    }
}

impl core::error::Error for BareImageApplyError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BareImageApplyReport {
    pub layers_applied: usize,
    pub entries_applied: usize,
    pub layer_reports: Vec<TarLayerApplyReport>,
}

pub fn apply_bare_image_layer_blobs<'a, I, F, D, S>(
    plan: &BareImagePlan,
    layer_blobs: I,
    mut digest_for_layer: F,
    sink: &mut S,
) -> Result<BareImageApplyReport, BareImageApplyError>
where
    I: IntoIterator<Item = &'a [u8]>,
    F: for<'d> FnMut(&'d LayerDescriptor) -> D,
    D: LayerDigest,
    S: TarLayerSink,
{
    let blobs = layer_blobs.into_iter().collect::<Vec<_>>();
    if blobs.len() != plan.layers.len() {
        return Err(BareImageApplyError::LayerCountMismatch {
            expected: plan.layers.len(),
            actual: blobs.len(),
        });
    }
    if !plan.diff_ids.is_empty() && plan.diff_ids.len() != plan.layers.len() {
        return Err(BareImageApplyError::DiffIdCountMismatch {
            expected: plan.layers.len(),
            actual: plan.diff_ids.len(),
        });
    }

    let mut layer_reports = Vec::with_capacity(plan.layers.len());
    let mut entries_applied = 0usize;

    for (index, (descriptor, blob)) in plan.layers.iter().zip(blobs.into_iter()).enumerate() {
        let (report, entries) =
            apply_bare_image_layer_blob(plan, index, blob, &mut digest_for_layer, sink)?;
        entries_applied = entries_applied.saturating_add(entries);
        layer_reports.push(report);
    }

    Ok(BareImageApplyReport {
        layers_applied: layer_reports.len(),
        entries_applied,
        layer_reports,
    })
}

pub fn apply_bare_image_layer_blobs_sha256<'a, I, S>(
    plan: &BareImagePlan,
    layer_blobs: I,
    sink: &mut S,
) -> Result<BareImageApplyReport, BareImageApplyError>
where
    I: IntoIterator<Item = &'a [u8]>,
    S: TarLayerSink,
{
    apply_bare_image_layer_blobs(plan, layer_blobs, |_| sha256_layer_digest(), sink)
}

pub fn validate_bare_image_layer_set(plan: &BareImagePlan) -> Result<(), BareImageApplyError> {
    if !plan.diff_ids.is_empty() && plan.diff_ids.len() != plan.layers.len() {
        return Err(BareImageApplyError::DiffIdCountMismatch {
            expected: plan.layers.len(),
            actual: plan.diff_ids.len(),
        });
    }
    Ok(())
}

pub fn apply_bare_image_layer_blob<F, D, S>(
    plan: &BareImagePlan,
    index: usize,
    blob: &[u8],
    digest_for_layer: &mut F,
    sink: &mut S,
) -> Result<(TarLayerApplyReport, usize), BareImageApplyError>
where
    F: for<'d> FnMut(&'d LayerDescriptor) -> D,
    D: LayerDigest,
    S: TarLayerSink,
{
    validate_bare_image_layer_set(plan)?;
    let descriptor = plan
        .layers
        .get(index)
        .ok_or(BareImageApplyError::LayerCountMismatch {
            expected: plan.layers.len(),
            actual: index.saturating_add(1),
        })?;
    if layer_compression(descriptor.media_type.as_deref()) == OciLayerCompression::Uncompressed {
        let mut layer_sink = ValidateOnlyLayerSink;
        let layer = apply_layer_chunks(
            descriptor,
            [blob],
            digest_for_layer(descriptor),
            &mut layer_sink,
        )
        .map_err(|error| BareImageApplyError::Layer {
            index,
            error: TarLayerApplyError::Layer(error),
        })?;
        if let Some(expected_diff_id) = plan.diff_ids.get(index) {
            validate_diff_id(index, expected_diff_id, blob, digest_for_layer(descriptor))?;
        }
        let entries = apply_uncompressed_tar_layer_streaming([blob], sink).map_err(|error| {
            BareImageApplyError::Layer {
                index,
                error: TarLayerApplyError::Tar(error),
            }
        })?;
        return Ok((
            TarLayerApplyReport {
                layer,
                entries_applied: entries,
            },
            entries,
        ));
    }

    let diff_digest = digest_for_layer(descriptor);
    let decoded = validate_and_decode_tar_layer(descriptor, [blob], digest_for_layer(descriptor))
        .map_err(|error| BareImageApplyError::Layer { index, error })?;
    if let Some(expected_diff_id) = plan.diff_ids.get(index) {
        validate_diff_id(index, expected_diff_id, &decoded.tar_bytes, diff_digest)?;
    }

    let entries = apply_uncompressed_tar_layer(&decoded.tar_bytes, sink).map_err(|error| {
        BareImageApplyError::Layer {
            index,
            error: TarLayerApplyError::Tar(error),
        }
    })?;

    Ok((
        TarLayerApplyReport {
            layer: decoded.layer,
            entries_applied: entries,
        },
        entries,
    ))
}

pub fn apply_bare_image_layer_blob_sha256<S>(
    plan: &BareImagePlan,
    index: usize,
    blob: &[u8],
    sink: &mut S,
) -> Result<(TarLayerApplyReport, usize), BareImageApplyError>
where
    S: TarLayerSink,
{
    fn digest_for_layer(_: &LayerDescriptor) -> Sha256LayerDigest {
        sha256_layer_digest()
    }
    apply_bare_image_layer_blob(plan, index, blob, &mut digest_for_layer, sink)
}

struct ValidateOnlyLayerSink;

impl LayerSink for ValidateOnlyLayerSink {
    fn write_chunk(&mut self, _descriptor: &LayerDescriptor, _chunk: &[u8]) -> Result<(), String> {
        Ok(())
    }
}

fn validate_diff_id<D: LayerDigest>(
    index: usize,
    expected: &str,
    tar_bytes: &[u8],
    mut digest: D,
) -> Result<(), BareImageApplyError> {
    let expected_algorithm = expected
        .split_once(':')
        .map(|(algorithm, _)| algorithm)
        .unwrap_or_default();
    let actual_algorithm = digest.algorithm();
    if expected_algorithm != actual_algorithm {
        return Err(BareImageApplyError::UnsupportedDiffIdAlgorithm {
            index,
            expected: expected_algorithm.into(),
            actual: actual_algorithm.into(),
        });
    }

    digest.update(tar_bytes);
    let actual = format_digest(actual_algorithm, &digest.finish());
    if actual != expected {
        return Err(BareImageApplyError::DiffIdMismatch {
            index,
            expected: expected.into(),
            actual,
        });
    }

    Ok(())
}

#[cfg(all(test, not(target_os = "none")))]
mod tests {
    use super::*;
    use crate::tar_layer::{OciWhiteout, TarEntry, TarEntryKind};
    use crate::test_support::{
        bare_image_plan_for_layers as plan_for_layers,
        bare_image_plan_for_layers_and_diff_ids as plan_for_layers_and_diff_ids,
        layer_descriptor as descriptor, real_sha256_digest_for,
        real_sha256_layer_descriptor as real_sha256_descriptor, tar, tar_entry, test_digest_for,
        TestDigest,
    };
    use alloc::vec::Vec;

    #[derive(Default)]
    struct CollectSink {
        entries: Vec<(TarEntry, Vec<u8>)>,
    }

    impl TarLayerSink for CollectSink {
        fn apply_entry(&mut self, entry: &TarEntry, data: &[u8]) -> Result<(), String> {
            self.entries.push((entry.clone(), data.to_vec()));
            Ok(())
        }
    }

    #[test]
    fn applies_layers_in_manifest_order() {
        let first = tar(vec![tar_entry("etc/one", b'0', b"one")]);
        let second = tar(vec![tar_entry("etc/two", b'0', b"two")]);
        let plan = plan_for_layers(vec![descriptor(&first), descriptor(&second)]);
        let mut sink = CollectSink::default();

        let report = apply_bare_image_layer_blobs(
            &plan,
            [first.as_slice(), second.as_slice()],
            |_| TestDigest::default(),
            &mut sink,
        )
        .unwrap();

        assert_eq!(report.layers_applied, 2);
        assert_eq!(report.entries_applied, 2);
        assert_eq!(sink.entries[0].0.path, "etc/one");
        assert_eq!(sink.entries[1].0.path, "etc/two");
    }

    #[test]
    fn preserves_whiteouts_across_layers() {
        let first = tar(vec![tar_entry("etc/shadow", b'0', b"old")]);
        let second = tar(vec![tar_entry("etc/.wh.shadow", b'0', b"")]);
        let plan = plan_for_layers(vec![descriptor(&first), descriptor(&second)]);
        let mut sink = CollectSink::default();

        apply_bare_image_layer_blobs(
            &plan,
            [first.as_slice(), second.as_slice()],
            |_| TestDigest::default(),
            &mut sink,
        )
        .unwrap();

        assert_eq!(
            sink.entries[1].0.whiteout,
            Some(OciWhiteout::RemovePath("etc/shadow".into()))
        );
    }

    #[test]
    fn validates_uncompressed_diff_ids_when_present() {
        let first = tar(vec![tar_entry("etc/one", b'0', b"one")]);
        let second = tar(vec![tar_entry("etc/two", b'0', b"two")]);
        let plan = plan_for_layers_and_diff_ids(
            vec![descriptor(&first), descriptor(&second)],
            vec![test_digest_for(&first), test_digest_for(&second)],
        );
        let mut sink = CollectSink::default();

        let report = apply_bare_image_layer_blobs(
            &plan,
            [first.as_slice(), second.as_slice()],
            |_| TestDigest::default(),
            &mut sink,
        )
        .unwrap();

        assert_eq!(report.layers_applied, 2);
        assert_eq!(sink.entries.len(), 2);
    }

    #[test]
    fn applies_image_with_builtin_sha256_digest() {
        let layer = tar(vec![tar_entry("etc/one", b'0', b"one")]);
        let plan = plan_for_layers_and_diff_ids(
            vec![real_sha256_descriptor(&layer)],
            vec![real_sha256_digest_for(&layer)],
        );
        let mut sink = CollectSink::default();

        let report =
            apply_bare_image_layer_blobs_sha256(&plan, [layer.as_slice()], &mut sink).unwrap();

        assert_eq!(report.layers_applied, 1);
        assert_eq!(sink.entries[0].0.path, "etc/one");
    }

    #[test]
    fn rejects_uncompressed_diff_id_mismatch() {
        let layer = tar(vec![tar_entry("etc/one", b'0', b"one")]);
        let plan =
            plan_for_layers_and_diff_ids(vec![descriptor(&layer)], vec![test_digest_for(b"bad")]);
        let mut sink = CollectSink::default();

        let error = apply_bare_image_layer_blobs(
            &plan,
            [layer.as_slice()],
            |_| TestDigest::default(),
            &mut sink,
        )
        .unwrap_err();

        assert!(matches!(
            error,
            BareImageApplyError::DiffIdMismatch { index: 0, .. }
        ));
        assert!(sink.entries.is_empty());
    }

    #[test]
    fn rejects_diff_id_count_mismatch() {
        let layer = tar(vec![tar_entry("etc/one", b'0', b"one")]);
        let plan = plan_for_layers_and_diff_ids(
            vec![descriptor(&layer), descriptor(&layer)],
            vec![test_digest_for(&layer)],
        );
        let mut sink = CollectSink::default();

        let error = apply_bare_image_layer_blobs(
            &plan,
            [layer.as_slice(), layer.as_slice()],
            |_| TestDigest::default(),
            &mut sink,
        )
        .unwrap_err();

        assert_eq!(
            error,
            BareImageApplyError::DiffIdCountMismatch {
                expected: 2,
                actual: 1
            }
        );
    }

    #[test]
    fn rejects_layer_count_mismatch() {
        let layer = tar(vec![tar_entry("etc/one", b'0', b"one")]);
        let plan = plan_for_layers(vec![descriptor(&layer), descriptor(&layer)]);
        let mut sink = CollectSink::default();

        let error = apply_bare_image_layer_blobs(
            &plan,
            [layer.as_slice()],
            |_| TestDigest::default(),
            &mut sink,
        )
        .unwrap_err();

        assert_eq!(
            error,
            BareImageApplyError::LayerCountMismatch {
                expected: 2,
                actual: 1
            }
        );
    }

    #[test]
    fn reports_failing_layer_index() {
        let first = tar(vec![tar_entry("etc/one", b'0', b"one")]);
        let second = tar(vec![tar_entry("etc/two", b'0', b"two")]);
        let mut bad_second_descriptor = descriptor(&second);
        bad_second_descriptor.size += 1;
        let plan = plan_for_layers(vec![descriptor(&first), bad_second_descriptor]);
        let mut sink = CollectSink::default();

        let error = apply_bare_image_layer_blobs(
            &plan,
            [first.as_slice(), second.as_slice()],
            |_| TestDigest::default(),
            &mut sink,
        )
        .unwrap_err();

        assert!(matches!(error, BareImageApplyError::Layer { index: 1, .. }));
    }
}
