//! Registry integration tests.
//!
//! These tests require network access to Docker Hub. They test pulling real images.
//! Run with: `cargo test -p edgerun-oci --test registry_integration`

use std::path::PathBuf;

fn tmp_dir() -> PathBuf {
    static C: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = C.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let p = std::env::temp_dir().join(format!("oci_reg_test_{}_{}", std::process::id(), n));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

#[test]
fn ping_docker_hub() {
    let rt = edgerun_rt::Runtime::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    let result = rt.block_on(async {
        let mut client = edgerun_oci::RegistryClient::new();
        client.ping("docker.io").await
    });

    assert!(result.is_ok(), "ping docker hub failed: {:?}", result);
}

#[test]
fn ping_ghcr() {
    let rt = edgerun_rt::Runtime::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    let result = rt.block_on(async {
        let mut client = edgerun_oci::RegistryClient::new();
        client.ping("ghcr.io").await
    });

    assert!(result.is_ok(), "ping ghcr.io failed: {:?}", result);
}

#[test]
fn resolve_alpine_manifest() {
    let rt = edgerun_rt::Runtime::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    let result = rt.block_on(async {
        let mut client = edgerun_oci::RegistryClient::new();
        let image: edgerun_oci::ImageRef = "alpine:latest".parse().unwrap();
        client.resolve_manifest(&image).await
    });

    assert!(
        result.is_ok(),
        "resolve alpine manifest failed: {:?}",
        result
    );
    let manifest = result.unwrap();
    match manifest {
        edgerun_oci::ImageManifest::Single(m) => {
            assert!(!m.layers.is_empty());
            assert!(!m.config_digest.is_empty());
        }
        edgerun_oci::ImageManifest::Index(idx) => {
            assert!(!idx.manifests.is_empty());
        }
    }
}

#[test]
fn resolve_busybox_manifest() {
    let rt = edgerun_rt::Runtime::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    let result = rt.block_on(async {
        let mut client = edgerun_oci::RegistryClient::new();
        let image: edgerun_oci::ImageRef = "busybox:latest".parse().unwrap();
        client.resolve_manifest(&image).await
    });

    assert!(
        result.is_ok(),
        "resolve busybox manifest failed: {:?}",
        result
    );
}
