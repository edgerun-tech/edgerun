use super::*;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn validates_user_spec_shape() {
    assert!(validate_user_spec("1000").is_ok());
    assert!(validate_user_spec("nobody:nobody").is_ok());
    assert!(validate_user_spec("").is_err());
    assert!(validate_user_spec(":group").is_err());
    assert!(validate_user_spec("user:group:extra").is_err());
}

#[test]
fn resolves_image_user_and_group_names() {
    let root = test_rootfs("edgerun-oci-user");
    let etc = root.join("etc");
    fs::create_dir_all(&etc).unwrap();
    fs::write(
        etc.join("passwd"),
        "root:x:0:0:root:/root:/bin/sh\nnobody:x:65534:65534:nobody:/nonexistent:/sbin/nologin\n",
    )
    .unwrap();
    fs::write(
        etc.join("group"),
        "root:x:0:\nnobody:x:65534:\napp:x:1000:\n",
    )
    .unwrap();

    let user = resolve_user(&root, "nobody").unwrap();
    assert_eq!(user.uid, Some(65534));
    assert_eq!(user.gid, Some(65534));

    let user = resolve_user(&root, "1000:nobody").unwrap();
    assert_eq!(user.uid, Some(1000));
    assert_eq!(user.gid, Some(65534));

    assert!(resolve_user(&root, "missing").is_err());
    assert!(resolve_user(&root, "nobody:missing").is_err());

    fs::remove_dir_all(root).unwrap();
}

fn test_rootfs(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("{prefix}-{nanos}"))
}
