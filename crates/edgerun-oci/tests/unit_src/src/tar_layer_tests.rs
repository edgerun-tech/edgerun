use super::*;
use crate::test_support::{
    real_sha256_digest_for, tar, tar_device_entry, tar_entry, tar_entry_with_mtime,
    test_digest_for, TestDigest, TEST_TAR_BLOCK_SIZE,
};
use alloc::vec::Vec;
use edgerun_encoding::crc32::crc32;

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

fn pax_record(key: &str, value: &str) -> Vec<u8> {
    let body = format!("{key}={value}\n");
    let mut len = body.len() + 2;
    loop {
        let next = body.len() + len.to_string().len() + 1;
        if next == len {
            break format!("{len} {body}").into_bytes();
        }
        len = next;
    }
}

fn gzip(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&[0x1f, 0x8b, 0x08, 0x00, 0, 0, 0, 0, 0x00, 0xff]);
    out.extend_from_slice(&miniz_oxide::deflate::compress_to_vec(bytes, 6));
    out.extend_from_slice(&crc32(bytes).to_le_bytes());
    out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    out
}

#[cfg(feature = "zstd")]
fn zstd(bytes: &[u8]) -> Vec<u8> {
    ruzstd::encoding::compress_to_vec(bytes, ruzstd::encoding::CompressionLevel::Fastest)
}

#[test]
fn parses_regular_file() {
    let data = tar(vec![tar_entry("etc/hosts", b'0', b"127.0.0.1\n")]);
    let mut sink = CollectSink::default();

    let count = apply_uncompressed_tar_layer(&data, &mut sink).unwrap();

    assert_eq!(count, 1);
    assert_eq!(sink.entries[0].0.path, "etc/hosts");
    assert_eq!(sink.entries[0].0.kind, TarEntryKind::Regular);
    assert_eq!(sink.entries[0].1, b"127.0.0.1\n");
}

#[test]
fn parses_device_major_minor() {
    let data = tar(vec![tar_device_entry("dev/null", b'3', 1, 3)]);
    let mut sink = CollectSink::default();

    let count = apply_uncompressed_tar_layer(&data, &mut sink).unwrap();

    assert_eq!(count, 1);
    assert_eq!(sink.entries[0].0.kind, TarEntryKind::Character);
    assert_eq!(sink.entries[0].0.dev_major, Some(1));
    assert_eq!(sink.entries[0].0.dev_minor, Some(3));
}

#[test]
fn parses_mtime() {
    let data = tar(vec![tar_entry_with_mtime(
        "etc/hosts",
        b'0',
        b"127.0.0.1\n",
        1_700_000_000,
    )]);
    let mut sink = CollectSink::default();

    let count = apply_uncompressed_tar_layer(&data, &mut sink).unwrap();

    assert_eq!(count, 1);
    assert_eq!(sink.entries[0].0.mtime, 1_700_000_000);
}

#[test]
fn validates_layer_before_applying_tar() {
    let data = tar(vec![tar_entry("bin/app", b'0', b"run")]);
    let descriptor = LayerDescriptor {
        media_type: Some("application/vnd.oci.image.layer.v1.tar".into()),
        digest: test_digest_for(&data),
        size: data.len() as u64,
    };
    let mut sink = CollectSink::default();
    let chunks: [&[u8]; 2] = [&data[..TEST_TAR_BLOCK_SIZE], &data[TEST_TAR_BLOCK_SIZE..]];

    let report = apply_validated_uncompressed_tar_layer(
        &descriptor,
        chunks,
        TestDigest::default(),
        &mut sink,
    )
    .unwrap();

    assert_eq!(report.entries_applied, 1);
    assert_eq!(report.layer.bytes_written, data.len() as u64);
    assert_eq!(sink.entries[0].0.path, "bin/app");
    assert_eq!(sink.entries[0].1, b"run");
}

#[test]
fn streams_uncompressed_tar_across_arbitrary_chunks() {
    let data = tar(vec![
        tar_entry("etc/hostname", b'0', b"edge"),
        tar_entry("bin/app", b'0', b"run"),
    ]);
    let chunks: Vec<&[u8]> = data.chunks(137).collect();
    let mut sink = CollectSink::default();

    let count = apply_uncompressed_tar_layer_streaming(chunks, &mut sink).unwrap();

    assert_eq!(count, 2);
    assert_eq!(sink.entries[0].0.path, "etc/hostname");
    assert_eq!(sink.entries[0].1, b"edge");
    assert_eq!(sink.entries[1].0.path, "bin/app");
    assert_eq!(sink.entries[1].1, b"run");
}

#[test]
fn streaming_tar_preserves_pax_overrides() {
    let mut pax = Vec::new();
    pax.extend_from_slice(&pax_record("path", "very/long/path/from/pax"));
    let data = tar(vec![
        tar_entry("pax", b'x', &pax),
        tar_entry("short", b'0', b"body"),
    ]);
    let mut stream_sink = CollectSink::default();
    let chunks: Vec<&[u8]> = data.chunks(211).collect();

    let count = apply_uncompressed_tar_layer_streaming(chunks, &mut stream_sink).unwrap();

    assert_eq!(count, 1);
    assert_eq!(stream_sink.entries[0].0.path, "very/long/path/from/pax");
    assert_eq!(stream_sink.entries[0].1, b"body");
}

#[test]
fn streaming_tar_rejects_truncated_header() {
    let data = vec![0u8; TEST_TAR_BLOCK_SIZE - 1];
    let mut sink = CollectSink::default();

    let error = apply_uncompressed_tar_layer_streaming([data.as_slice()], &mut sink).unwrap_err();

    assert_eq!(error, TarLayerError::TruncatedHeader);
}

#[test]
fn validates_layer_with_builtin_sha256_digest() {
    let data = tar(vec![tar_entry("bin/app", b'0', b"run")]);
    let descriptor = LayerDescriptor {
        media_type: Some("application/vnd.oci.image.layer.v1.tar".into()),
        digest: real_sha256_digest_for(&data),
        size: data.len() as u64,
    };
    let mut sink = CollectSink::default();

    let report =
        apply_validated_tar_layer_sha256(&descriptor, [data.as_slice()], &mut sink).unwrap();

    assert_eq!(report.entries_applied, 1);
    assert_eq!(sink.entries[0].0.path, "bin/app");
}

#[test]
fn validates_and_applies_gzip_tar_layer() {
    let tar = tar(vec![tar_entry("bin/app", b'0', b"run")]);
    let data = gzip(&tar);
    let descriptor = LayerDescriptor {
        media_type: Some("application/vnd.oci.image.layer.v1.tar+gzip".into()),
        digest: test_digest_for(&data),
        size: data.len() as u64,
    };
    let mut sink = CollectSink::default();

    let report = apply_validated_tar_layer(
        &descriptor,
        [data.as_slice()],
        TestDigest::default(),
        &mut sink,
    )
    .unwrap();

    assert_eq!(report.entries_applied, 1);
    assert_eq!(report.layer.bytes_written, data.len() as u64);
    assert_eq!(sink.entries[0].0.path, "bin/app");
    assert_eq!(sink.entries[0].1, b"run");
}

#[cfg(feature = "zstd")]
#[test]
fn validates_and_applies_zstd_tar_layer() {
    let tar = tar(vec![tar_entry("bin/app", b'0', b"run")]);
    let data = zstd(&tar);
    let descriptor = LayerDescriptor {
        media_type: Some("application/vnd.oci.image.layer.v1.tar+zstd".into()),
        digest: test_digest_for(&data),
        size: data.len() as u64,
    };
    let mut sink = CollectSink::default();

    let report = apply_validated_tar_layer(
        &descriptor,
        [data.as_slice()],
        TestDigest::default(),
        &mut sink,
    )
    .unwrap();

    assert_eq!(report.entries_applied, 1);
    assert_eq!(report.layer.bytes_written, data.len() as u64);
    assert_eq!(sink.entries[0].0.path, "bin/app");
    assert_eq!(sink.entries[0].1, b"run");
}

#[test]
fn classifies_layer_media_types() {
    assert_eq!(
        layer_compression(Some("application/vnd.oci.image.layer.v1.tar")),
        OciLayerCompression::Uncompressed
    );
    assert_eq!(
        layer_compression(Some("application/vnd.oci.image.layer.v1.tar+gzip")),
        OciLayerCompression::Gzip
    );
    assert_eq!(
        layer_compression(Some("application/vnd.docker.image.rootfs.diff.tar.gzip")),
        OciLayerCompression::Gzip
    );
    assert_eq!(
        layer_compression(Some("application/vnd.oci.image.layer.v1.tar+zstd")),
        OciLayerCompression::Zstd
    );
    assert_eq!(layer_compression(None), OciLayerCompression::Unknown);
}

#[test]
fn rejects_compressed_layer_media_type_before_tar_apply() {
    let data = tar(vec![tar_entry("bin/app", b'0', b"run")]);
    let descriptor = LayerDescriptor {
        media_type: Some("application/vnd.oci.image.layer.v1.tar+gzip".into()),
        digest: test_digest_for(&data),
        size: data.len() as u64,
    };
    let mut sink = CollectSink::default();

    let error = apply_validated_uncompressed_tar_layer(
        &descriptor,
        [data.as_slice()],
        TestDigest::default(),
        &mut sink,
    )
    .unwrap_err();

    assert_eq!(
        error,
        TarLayerApplyError::UnsupportedMediaType(descriptor.media_type)
    );
    assert!(sink.entries.is_empty());
}

#[test]
fn rejects_gzip_crc_mismatch() {
    let tar = tar(vec![tar_entry("bin/app", b'0', b"run")]);
    let mut data = gzip(&tar);
    let footer = data.len() - 8;
    data[footer] ^= 0xff;

    let error = decompress_gzip_layer(&data).unwrap_err();

    assert!(matches!(error, TarLayerApplyError::Decompress(_)));
}

#[test]
fn detects_whiteouts() {
    let data = tar(vec![
        tar_entry("etc/.wh.shadow", b'0', b""),
        tar_entry("var/.wh..wh..opq", b'0', b""),
    ]);
    let mut sink = CollectSink::default();

    apply_uncompressed_tar_layer(&data, &mut sink).unwrap();

    assert_eq!(
        sink.entries[0].0.whiteout,
        Some(OciWhiteout::RemovePath("etc/shadow".into()))
    );
    assert_eq!(
        sink.entries[1].0.whiteout,
        Some(OciWhiteout::OpaqueDirectory("var".into()))
    );
}

#[test]
fn applies_pax_path_and_linkpath() {
    let mut pax = Vec::new();
    pax.extend_from_slice(&pax_record("path", "very/long/path/from/pax"));
    pax.extend_from_slice(&pax_record("linkpath", "target/from/pax"));
    let data = tar(vec![
        tar_entry("pax", b'x', &pax),
        tar_entry("short-link", b'2', b""),
    ]);
    let mut sink = CollectSink::default();

    apply_uncompressed_tar_layer(&data, &mut sink).unwrap();

    assert_eq!(sink.entries.len(), 1);
    assert_eq!(sink.entries[0].0.path, "very/long/path/from/pax");
    assert_eq!(
        sink.entries[0].0.link_name.as_deref(),
        Some("target/from/pax")
    );
}

#[test]
fn applies_gnu_long_name() {
    let long_path = "long/component/name/that/does/not/fit/in/header";
    let mut long_name = long_path.as_bytes().to_vec();
    long_name.push(0);
    let data = tar(vec![
        tar_entry("././@LongLink", b'L', &long_name),
        tar_entry("short", b'0', b"body"),
    ]);
    let mut sink = CollectSink::default();

    apply_uncompressed_tar_layer(&data, &mut sink).unwrap();

    assert_eq!(sink.entries.len(), 1);
    assert_eq!(sink.entries[0].0.path, long_path);
    assert_eq!(sink.entries[0].1, b"body");
}

#[test]
fn rejects_unsafe_pax_path() {
    let data = tar(vec![
        tar_entry("pax", b'x', &pax_record("path", "../escape")),
        tar_entry("short", b'0', b"bad"),
    ]);
    let mut sink = CollectSink::default();

    let error = apply_uncompressed_tar_layer(&data, &mut sink).unwrap_err();

    assert_eq!(error, TarLayerError::InvalidPath("../escape".into()));
}

#[test]
fn rejects_parent_escape() {
    let data = tar(vec![tar_entry("../escape", b'0', b"bad")]);
    let mut sink = CollectSink::default();

    let error = apply_uncompressed_tar_layer(&data, &mut sink).unwrap_err();

    assert_eq!(error, TarLayerError::InvalidPath("../escape".into()));
}

#[test]
fn rejects_bad_checksum() {
    let mut data = tar(vec![tar_entry("file", b'0', b"ok")]);
    data[0] = b'X';
    let mut sink = CollectSink::default();

    let error = apply_uncompressed_tar_layer(&data, &mut sink).unwrap_err();

    assert!(matches!(error, TarLayerError::InvalidHeader(_)));
}
