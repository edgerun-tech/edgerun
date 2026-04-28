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

    let report = apply_bare_image_layer_blobs_sha256(&plan, [layer.as_slice()], &mut sink).unwrap();

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
