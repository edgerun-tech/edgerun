use super::*;
use crate::registry::tar_push::create_tar_from_dir;
use std::path::PathBuf;

fn tmp_root() -> PathBuf {
    static C: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = C.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let p = std::env::temp_dir().join(format!("oci_client_test_{}_{}", std::process::id(), n));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

#[test]
fn new_client_is_anonymous() {
    let client = RegistryClient::new();
    assert!(matches!(client.auth, RegistryAuth::Anonymous));
    assert_eq!(
        client.trust_policy,
        ImageTrustPolicy::RequireDigestReference
    );
}

#[test]
fn with_auth_sets_credentials() {
    let client = RegistryClient::new().with_auth(RegistryAuth::Basic {
        username: "user".into(),
        password: "pass".into(),
    });
    if let RegistryAuth::Basic { username, password } = client.auth {
        assert_eq!(username, "user");
        assert_eq!(password, "pass");
    } else {
        panic!("expected Basic");
    }
}

#[test]
fn with_secret_service_auth_sets_variant() {
    let root = tmp_root();
    let client = RegistryClient::with_secret_service_auth(&root, "registry", "docker.io");
    if let RegistryAuth::FromSecretService {
        data_root,
        namespace,
        registry_host,
    } = client.auth
    {
        assert_eq!(data_root, root);
        assert_eq!(namespace, "registry");
        assert_eq!(registry_host, "docker.io");
    } else {
        panic!("expected FromSecretService");
    }
}

#[test]
fn secret_service_auth_requires_usable_credentials() {
    let root = tmp_root();
    let mut client = RegistryClient::with_secret_service_auth(&root, "registry", "docker.io");

    let result = edgerun_rt::block_on(client.handle_auth_challenge(
        "docker.io",
        "Bearer realm=\"https://auth.example/token\",service=\"registry.example\"",
    ));

    match result {
        Err(RegistryError::AuthError(message)) => {
            assert!(message.contains("no usable secret-service credentials"));
        }
        other => panic!("expected AuthError for missing secret-service credentials, got {other:?}"),
    }
}

#[test]
fn create_tar_from_dir_roundtrips_through_layer_extractor() {
    let tmp = tmp_root();
    let src = tmp.join("src");
    let dest = tmp.join("dest");
    std::fs::create_dir_all(src.join("nested")).unwrap();
    std::fs::create_dir_all(&dest).unwrap();
    std::fs::write(src.join("nested/file.txt"), b"hello from tar").unwrap();

    let layer = create_tar_from_dir(&src).unwrap();
    let blob = tmp.join("layer.tar.gz");
    std::fs::write(&blob, layer).unwrap();

    crate::registry::layer::extract_layer(
        &blob,
        &dest,
        Some("application/vnd.oci.image.layer.v1.tar+gzip"),
    )
    .unwrap();

    assert_eq!(
        std::fs::read_to_string(dest.join("nested/file.txt")).unwrap(),
        "hello from tar"
    );
}

#[test]
fn registry_redirect_location_resolves_absolute_and_relative_targets() {
    assert_eq!(
        RegistryClient::test_resolve_redirect_location(
            "https://registry.example/v2/repo/blobs/sha256:abc",
            "https://cdn.example/blob"
        )
        .unwrap(),
        "https://cdn.example/blob"
    );
    assert_eq!(
        RegistryClient::test_resolve_redirect_location(
            "https://registry.example/v2/repo/blobs/sha256:abc",
            "/storage/blob"
        )
        .unwrap(),
        "https://registry.example/storage/blob"
    );
    assert_eq!(
        RegistryClient::test_resolve_redirect_location(
            "https://registry.example/v2/repo/blobs/sha256:abc",
            "next"
        )
        .unwrap(),
        "https://registry.example/v2/repo/blobs/next"
    );
}

#[test]
fn image_ref_parsing() {
    let img: ImageRef = "docker.io/library/alpine:latest".parse().unwrap();
    assert_eq!(img.registry, "docker.io");
    assert_eq!(img.repository, "library/alpine");
    assert_eq!(img.tag, "latest");
}

#[test]
fn image_ref_default_tag() {
    let img: ImageRef = "myregistry/myrepo".parse().unwrap();
    assert_eq!(img.tag, "latest");
}

#[test]
fn image_ref_docker_hub_library() {
    let img: ImageRef = "alpine:3.18".parse().unwrap();
    assert_eq!(img.registry, "docker.io");
    assert_eq!(img.repository, "library/alpine");
    assert_eq!(img.tag, "3.18");
}

#[test]
fn image_ref_display() {
    let img = ImageRef {
        registry: "ghcr.io".into(),
        repository: "owner/repo".into(),
        tag: "v1".into(),
    };
    assert_eq!(img.to_string(), "ghcr.io/owner/repo:v1");
}

#[test]
fn image_ref_digest_reference_uses_digest_as_registry_reference() {
    let digest = "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    let img: ImageRef = format!("alpine@{digest}").parse().unwrap();

    assert_eq!(img.registry, "docker.io");
    assert_eq!(img.repository, "library/alpine");
    assert_eq!(img.tag, digest);
    assert_eq!(img.reference(), digest);
    assert!(img.is_digest_reference());
    assert_eq!(
        img.to_string(),
        format!("docker.io/library/alpine@{digest}")
    );
}

#[test]
fn image_ref_digest_reference_ignores_optional_tag_for_manifest_lookup() {
    let digest = "sha256:fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210";
    let img: ImageRef = format!("ghcr.io/owner/repo:v1@{digest}").parse().unwrap();

    assert_eq!(img.registry, "ghcr.io");
    assert_eq!(img.repository, "owner/repo");
    assert_eq!(img.reference(), digest);
    assert_eq!(img.to_string(), format!("ghcr.io/owner/repo@{digest}"));
}

#[test]
fn image_ref_rejects_empty_and_malformed_digest_references() {
    assert!("".parse::<ImageRef>().is_err());
    assert!("alpine@".parse::<ImageRef>().is_err());
    assert!("@sha256:abc".parse::<ImageRef>().is_err());
    assert!("alpine@not-a-digest".parse::<ImageRef>().is_err());
    assert!("alpine@sha256:abc".parse::<ImageRef>().is_err());
    assert!("alpine:".parse::<ImageRef>().is_err());
    assert!("UPPER/repo:tag".parse::<ImageRef>().is_err());
    assert!("alpine:bad tag".parse::<ImageRef>().is_err());
}
