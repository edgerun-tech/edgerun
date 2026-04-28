use super::*;

fn tmp_root() -> PathBuf {
    static C: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = C.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let p = std::env::temp_dir().join(format!("oci_auth_test_{}_{}", std::process::id(), n));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

#[test]
fn resolve_from_secret_service_roundtrip() {
    let root = tmp_root();

    // Store a credential via the secret service backend
    let coll = "/org/freedesktop/secrets/collections/registry";
    let mut backend = edgerun_secret_service::Backend::new_noop(root.clone()).unwrap();
    backend
        .put(coll, "docker.io", b"myuser:mypass123", "Docker Hub", &[])
        .unwrap();

    // Resolve it
    let creds = resolve_from_secret_service(&root, "registry", "docker.io");
    assert_eq!(creds, Some(("myuser".into(), "mypass123".into())));
}

#[test]
fn resolve_from_secret_service_missing_returns_none() {
    let root = tmp_root();
    let creds = resolve_from_secret_service(&root, "registry", "nonexistent");
    assert!(creds.is_none());
}

#[test]
fn resolve_from_secret_service_wrong_format_returns_none() {
    let root = tmp_root();

    let coll = "/org/freedesktop/secrets/collections/registry";
    let mut backend = edgerun_secret_service::Backend::new_noop(root.clone()).unwrap();
    // Store without the colon separator
    backend
        .put(coll, "docker.io", b"no-colon-here", "Bad", &[])
        .unwrap();

    let creds = resolve_from_secret_service(&root, "registry", "docker.io");
    assert!(creds.is_none());
}

#[test]
fn decode_basic_auth_roundtrip() {
    let (user, pass) = decode_basic_auth("bXl1c2VyOm15cGFzcw==").unwrap();
    assert_eq!(user, "myuser");
    assert_eq!(pass, "mypass");
}

#[test]
fn parse_bearer_auth_full() {
    let header = "Bearer realm=\"https://auth.docker.io/token\",service=\"registry.docker.io\",scope=\"repository:library/alpine:pull\"";
    let (realm, service, scope) = parse_bearer_auth(header).unwrap();
    assert_eq!(realm, "https://auth.docker.io/token");
    assert_eq!(service, "registry.docker.io");
    assert_eq!(scope, Some("repository:library/alpine:pull".into()));
}

#[test]
fn parse_bearer_auth_no_scope() {
    let header = "Bearer realm=\"https://auth.example.com/token\",service=\"registry\"";
    let (realm, service, scope) = parse_bearer_auth(header).unwrap();
    assert_eq!(realm, "https://auth.example.com/token");
    assert_eq!(service, "registry");
    assert!(scope.is_none());
}

#[test]
fn parse_bearer_auth_invalid_prefix() {
    assert!(parse_bearer_auth("Basic abc").is_none());
    assert!(parse_bearer_auth("").is_none());
}
