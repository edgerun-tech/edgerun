use super::*;

fn test_dir(name: &str) -> PathBuf {
    let n = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "edgerun-oci-rootfs-copy-{name}-{:x}-{n:x}",
        std::process::id()
    ))
}

#[test]
fn merge_layer_dirs_replaces_file_entries() {
    let root = test_dir("file-replace");
    let layer1 = root.join("layer1");
    let layer2 = root.join("layer2");
    let dest = root.join("dest");
    fs::create_dir_all(&layer1).unwrap();
    fs::create_dir_all(&layer2).unwrap();
    fs::write(layer1.join("config"), b"first").unwrap();
    fs::write(layer2.join("config"), b"second").unwrap();

    merge_layer_dirs(&[layer1, layer2], &dest).unwrap();

    assert_eq!(fs::read(dest.join("config")).unwrap(), b"second");
    let temp_entries = fs::read_dir(&dest)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().starts_with(".tmp."))
        .count();
    assert_eq!(temp_entries, 0);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn merge_layer_dirs_replaces_file_with_symlink() {
    let root = test_dir("symlink-replace");
    let layer1 = root.join("layer1");
    let layer2 = root.join("layer2");
    let dest = root.join("dest");
    fs::create_dir_all(&layer1).unwrap();
    fs::create_dir_all(&layer2).unwrap();
    fs::write(layer1.join("config"), b"first").unwrap();
    symlink("target-config", layer2.join("config")).unwrap();

    merge_layer_dirs(&[layer1, layer2], &dest).unwrap();

    assert_eq!(
        fs::read_link(dest.join("config")).unwrap(),
        PathBuf::from("target-config")
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn merge_layer_dirs_applies_remove_whiteout_to_lower_layer() {
    let root = test_dir("remove-whiteout");
    let layer1 = root.join("layer1");
    let layer2 = root.join("layer2");
    let dest = root.join("dest");
    fs::create_dir_all(layer1.join("etc")).unwrap();
    fs::create_dir_all(layer2.join("etc")).unwrap();
    fs::write(layer1.join("etc/shadow"), b"old").unwrap();
    fs::write(layer2.join("etc/.wh.shadow"), b"").unwrap();

    merge_layer_dirs(&[layer1, layer2], &dest).unwrap();

    assert!(!dest.join("etc/shadow").exists());
    assert!(!dest.join("etc/.wh.shadow").exists());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn merge_layer_dirs_applies_opaque_whiteout_to_lower_directory() {
    let root = test_dir("opaque-whiteout");
    let layer1 = root.join("layer1");
    let layer2 = root.join("layer2");
    let dest = root.join("dest");
    fs::create_dir_all(layer1.join("etc")).unwrap();
    fs::create_dir_all(layer2.join("etc")).unwrap();
    fs::write(layer1.join("etc/lower"), b"old").unwrap();
    fs::write(layer2.join("etc/.wh..wh..opq"), b"").unwrap();
    fs::write(layer2.join("etc/upper"), b"new").unwrap();

    merge_layer_dirs(&[layer1, layer2], &dest).unwrap();

    assert!(!dest.join("etc/lower").exists());
    assert_eq!(fs::read(dest.join("etc/upper")).unwrap(), b"new");
    assert!(!dest.join("etc/.wh..wh..opq").exists());
    let _ = fs::remove_dir_all(root);
}
