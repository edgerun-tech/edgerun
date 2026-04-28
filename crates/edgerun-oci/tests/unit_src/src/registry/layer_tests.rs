use super::*;
use crate::test_support::{tar, tar_entry, write_field, write_octal, TEST_TAR_BLOCK_SIZE};
use std::io::Write;

fn tmp_dir() -> std::path::PathBuf {
    static C: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = C.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let p = std::env::temp_dir().join(format!("oci_layer_test_{}_{}", std::process::id(), n));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

#[test]
fn extract_layer_basic() {
    let tmp = tmp_dir();
    let dest = tmp.join("rootfs");
    std::fs::create_dir_all(&dest).unwrap();

    let tar_data = tar(vec![tar_entry("file.txt", b'0', b"hello world")]);
    let tar_file = tmp.join("layer.tar");
    std::fs::write(&tar_file, &tar_data).unwrap();

    extract_layer(&tar_file, &dest, None).unwrap();

    let extracted = dest.join("file.txt");
    assert!(extracted.exists());
    assert_eq!(std::fs::read_to_string(&extracted).unwrap(), "hello world");
}

#[test]
fn path_safe_within_root_valid() {
    assert!(path_safe_within_root(std::path::Path::new("foo/bar")));
    assert!(path_safe_within_root(std::path::Path::new("a/b/c")));
}

#[test]
fn path_safe_within_root_rejects_absolute_parent_escape() {
    assert!(!path_safe_within_root(std::path::Path::new("/foo")));
    assert!(!path_safe_within_root(std::path::Path::new("../foo")));
    assert!(!path_safe_within_root(std::path::Path::new("../../../etc")));
}

#[test]
fn verify_blob_digest_valid() {
    let tmp = tmp_dir();
    let blob_path = tmp.join("blob");
    let mut f = std::fs::File::create(&blob_path).unwrap();
    let content = b"hello";
    f.write_all(content).unwrap();
    f.sync_all().unwrap();

    let expected = crate::sha256_digest_reference(content);

    let result = verify_blob_digest(&blob_path, &expected);
    assert!(result.is_ok(), "digest mismatch: {:?}", result);
}

#[test]
fn verify_blob_digest_mismatch() {
    let tmp = tmp_dir();
    let blob_path = tmp.join("blob");
    std::fs::write(&blob_path, b"hello").unwrap();

    let result = verify_blob_digest(
        &blob_path,
        "sha256:0000000000000000000000000000000000000000000000000000000000000000",
    );
    assert!(result.is_err());
}

#[test]
fn extract_layer_rejects_write_through_symlink_parent() {
    let tmp = tmp_dir();
    let dest = tmp.join("rootfs");
    let outside = tmp.join("outside");
    std::fs::create_dir_all(&dest).unwrap();
    std::fs::create_dir_all(&outside).unwrap();

    let tar_data = tar(vec![
        tar_symlink_entry("link", outside.to_str().unwrap()),
        tar_entry("link/file.txt", b'0', b"escaped"),
    ]);
    let tar_file = tmp.join("layer.tar");
    std::fs::write(&tar_file, &tar_data).unwrap();

    let result = extract_layer(&tar_file, &dest, None);

    assert!(result.is_err());
    assert!(!outside.join("file.txt").exists());
}

#[test]
fn extract_layer_replaces_directory_with_file() {
    let tmp = tmp_dir();
    let dest = tmp.join("rootfs");
    std::fs::create_dir_all(&dest).unwrap();

    let tar_data = tar(vec![
        tar_entry("config", b'5', b""),
        tar_entry("config", b'0', b"file"),
    ]);
    let tar_file = tmp.join("layer.tar");
    std::fs::write(&tar_file, &tar_data).unwrap();

    extract_layer(&tar_file, &dest, None).unwrap();

    assert_eq!(std::fs::read(dest.join("config")).unwrap(), b"file");
}

fn tar_symlink_entry(path: &str, target: &str) -> Vec<u8> {
    let mut entry = tar_entry(path, b'2', b"");
    for byte in &mut entry[148..156] {
        *byte = b' ';
    }
    write_field(&mut entry[157..257], target.as_bytes());
    let checksum: u64 = entry[..TEST_TAR_BLOCK_SIZE]
        .iter()
        .map(|byte| u64::from(*byte))
        .sum();
    write_octal(&mut entry[148..156], checksum);
    entry
}
