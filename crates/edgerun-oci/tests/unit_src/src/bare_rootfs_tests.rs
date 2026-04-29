use super::*;
use crate::image_apply::apply_bare_image_layer_blobs;
use crate::test_support::{
    bare_image_plan_for_layers as plan_for_layers, layer_descriptor as descriptor, tar, tar_entry,
    TestDigest,
};

#[test]
fn stores_regular_files_and_parent_dirs() {
    let mut rootfs = BareRootfs::new();
    let entry = TarEntry {
        path: "etc/hosts".into(),
        kind: TarEntryKind::Regular,
        size: 9,
        mode: 0o644,
        uid: 0,
        gid: 0,
        mtime: 0,
        dev_major: None,
        dev_minor: None,
        link_name: None,
        whiteout: None,
    };

    rootfs.apply_entry(&entry, b"127.0.0.1").unwrap();

    assert!(rootfs.contains("etc"));
    assert_eq!(rootfs.read_file("etc/hosts"), Some(b"127.0.0.1".as_slice()));
}

#[test]
fn rootfs_access_reads_ranges_and_hardlinks() {
    let mut rootfs = BareRootfs::new();
    let file = TarEntry {
        path: "bin/app".into(),
        kind: TarEntryKind::Regular,
        size: 10,
        mode: 0o755,
        uid: 0,
        gid: 0,
        mtime: 7,
        dev_major: None,
        dev_minor: None,
        link_name: None,
        whiteout: None,
    };
    rootfs.apply_entry(&file, b"0123456789").unwrap();
    let mut link = file.clone();
    link.path = "bin/run".into();
    link.kind = TarEntryKind::Hardlink;
    link.size = 0;
    link.link_name = Some("bin/app".into());
    rootfs.apply_entry(&link, &[]).unwrap();

    assert_eq!(OciRootfs::file_len(&rootfs, "/bin/run").unwrap(), Some(10));
    let mut buf = [0u8; 4];
    assert_eq!(
        OciRootfs::read_file_range(&rootfs, "/bin/run", 3, &mut buf).unwrap(),
        Some(4)
    );
    assert_eq!(&buf, b"3456");

    let entry = OciRootfs::entry(&rootfs, "bin/run").unwrap().unwrap();
    assert_eq!(entry.kind, OciRootfsEntryKind::Hardlink);
    assert_eq!(entry.link_name, Some("bin/app".into()));

    let bin_children = OciRootfs::children(&rootfs, "bin").unwrap();
    assert_eq!(
        bin_children
            .iter()
            .map(|entry| entry.path.as_str())
            .collect::<Vec<_>>(),
        vec!["bin/app", "bin/run"]
    );

    let under_bin = OciRootfs::entries_under(&rootfs, "bin").unwrap();
    assert_eq!(under_bin.len(), 2);
}

#[test]
fn rootfs_access_resolves_executables() {
    let mut rootfs = BareRootfs::new();
    let file = TarEntry {
        path: "usr/bin/app".into(),
        kind: TarEntryKind::Regular,
        size: 4,
        mode: 0o755,
        uid: 0,
        gid: 0,
        mtime: 0,
        dev_major: None,
        dev_minor: None,
        link_name: None,
        whiteout: None,
    };
    rootfs.apply_entry(&file, b"app!").unwrap();

    let mut symlink = file.clone();
    symlink.path = "bin/sh".into();
    symlink.kind = TarEntryKind::Symlink;
    symlink.size = 0;
    symlink.link_name = Some("../usr/bin/app".into());
    rootfs.apply_entry(&symlink, &[]).unwrap();

    let executable = crate::rootfs_access::resolve_executable_path(
        &rootfs,
        "app",
        &["PATH=/usr/bin:/bin".into()],
        "/",
    )
    .unwrap()
    .unwrap();
    assert_eq!(executable.path, "usr/bin/app");

    let executable = crate::rootfs_access::resolve_executable_path(&rootfs, "/bin/sh", &[], "/")
        .unwrap()
        .unwrap();
    assert_eq!(executable.path, "usr/bin/app");

    let executable =
        crate::rootfs_access::resolve_executable_path(&rootfs, "./app", &[], "/usr/bin")
            .unwrap()
            .unwrap();
    assert_eq!(executable.path, "usr/bin/app");

    let plan = crate::rootfs_access::build_launch_plan(
        &rootfs,
        &["app".into(), "--version".into()],
        &["PATH=/usr/bin".into(), "TERM=xterm".into()],
        "/",
    )
    .unwrap()
    .unwrap();
    assert_eq!(plan.executable.path, "usr/bin/app");
    assert_eq!(plan.argv, vec!["app", "--version"]);
    assert_eq!(plan.env, vec!["PATH=/usr/bin", "TERM=xterm"]);
    assert_eq!(plan.cwd, "/");
}

#[test]
fn applies_remove_whiteout() {
    let lower = tar(vec![tar_entry("etc/shadow", b'0', b"old")]);
    let upper = tar(vec![tar_entry("etc/.wh.shadow", b'0', b"")]);
    let plan = plan_for_layers(vec![descriptor(&lower), descriptor(&upper)]);
    let mut rootfs = BareRootfs::new();

    apply_bare_image_layer_blobs(
        &plan,
        [lower.as_slice(), upper.as_slice()],
        |_| TestDigest::default(),
        &mut rootfs,
    )
    .unwrap();

    assert!(!rootfs.contains("etc/shadow"));
}

#[test]
fn applies_opaque_directory_whiteout() {
    let lower = tar(vec![
        tar_entry("var/cache/old", b'0', b"old"),
        tar_entry("var/lib/keep", b'0', b"keep"),
    ]);
    let upper = tar(vec![
        tar_entry("var/cache/.wh..wh..opq", b'0', b""),
        tar_entry("var/cache/new", b'0', b"new"),
    ]);
    let plan = plan_for_layers(vec![descriptor(&lower), descriptor(&upper)]);
    let mut rootfs = BareRootfs::new();

    apply_bare_image_layer_blobs(
        &plan,
        [lower.as_slice(), upper.as_slice()],
        |_| TestDigest::default(),
        &mut rootfs,
    )
    .unwrap();

    assert!(!rootfs.contains("var/cache/old"));
    assert_eq!(rootfs.read_file("var/cache/new"), Some(b"new".as_slice()));
    assert_eq!(rootfs.read_file("var/lib/keep"), Some(b"keep".as_slice()));
}
