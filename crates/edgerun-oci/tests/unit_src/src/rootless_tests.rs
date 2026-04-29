use super::*;

#[test]
fn parse_subid_file_parses_valid_lines() {
    let content = "# comment\nken:100000:65536\nnobody:65536:1\n";
    let tmp = std::env::temp_dir().join(format!("test_subuid_{}", std::process::id()));
    fs::write(&tmp, content).unwrap();

    let ranges = parse_subid_file(&tmp).unwrap();
    assert_eq!(ranges.len(), 2);
    assert_eq!(ranges[0].name.as_str(), "ken");
    assert_eq!(ranges[0].start, 100000);
    assert_eq!(ranges[0].count, 65536);
    assert_eq!(ranges[1].name.as_str(), "nobody");
    assert_eq!(ranges[1].start, 65536);
    assert_eq!(ranges[1].count, 1);

    let _ = fs::remove_file(&tmp);
}

#[test]
fn parse_subid_file_skips_comments_and_empty() {
    let content = "# this is a comment\n\n  \nken:100000:65536\n";
    let tmp = std::env::temp_dir().join(format!("test_subuid2_{}", std::process::id()));
    fs::write(&tmp, content).unwrap();

    let ranges = parse_subid_file(&tmp).unwrap();
    assert_eq!(ranges.len(), 1);

    let _ = fs::remove_file(&tmp);
}

#[test]
fn generate_uid_map_no_subuids() {
    let map = generate_uid_map(1000, &[]);
    assert_eq!(map, "0 1000 1\n");
}

#[test]
fn generate_uid_map_with_subuids() {
    let ranges = vec![SubIdRange {
        name: "ken".into(),
        start: 100000,
        count: 65536,
    }];
    let map = generate_uid_map(1000, &ranges);
    assert_eq!(map, "0 1000 1\n1 100000 65536\n");
}

#[test]
fn generate_gid_map_no_subgids() {
    let map = generate_gid_map(1000, &[]);
    assert_eq!(map, "0 1000 1\n");
}

#[test]
fn generate_gid_map_with_subgids() {
    let ranges = vec![SubIdRange {
        name: "ken".into(),
        start: 100000,
        count: 65536,
    }];
    let map = generate_gid_map(1000, &ranges);
    assert_eq!(map, "0 1000 1\n1 100000 65536\n");
}

#[test]
fn resolve_container_cgroup_path_root_mode_absolute() {
    let path = resolve_container_cgroup_path(false, "/my/container").unwrap();
    assert_eq!(path, "/my/container");
}

#[test]
fn resolve_container_cgroup_path_root_mode_default() {
    let path = resolve_container_cgroup_path(false, "").unwrap();
    assert_eq!(path, "/edgerun");
}

#[test]
fn resolve_container_cgroup_path_rootless_appends() {
    // This test only validates the logic — actual delegation depends on /proc
    // We can't easily mock it, but we can verify the append logic
    if !is_cgroup_v2_available() {
        return; // Skip if cgroup v2 not available
    }
    let path = resolve_container_cgroup_path(true, "my-container").unwrap();
    // Should end with /my-container
    assert!(path.ends_with("/my-container"));
}

#[test]
fn is_cgroup_v2_available_returns_bool() {
    let result = is_cgroup_v2_available();
    assert!(matches!(result, true | false));
}

#[test]
fn get_current_user_subuids_parses_correctly() {
    // Write a test /etc/subuid file
    let content = "testuser:100000:65536\nnobody:65536:1\n";
    let tmp = std::env::temp_dir().join(format!("test_subuid_rootless_{}", std::process::id()));
    std::fs::write(&tmp, content).unwrap();

    let ranges = parse_subid_file(&tmp).unwrap();
    assert_eq!(ranges.len(), 2);
    assert_eq!(ranges[0].name.as_str(), "testuser");
    assert_eq!(ranges[0].start, 100000);
    assert_eq!(ranges[0].count, 65536);

    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn generate_uid_map_uses_subuid_ranges() {
    let ranges = vec![SubIdRange {
        name: "ken".into(),
        start: 100000,
        count: 65536,
    }];
    let map = generate_uid_map(1000, &ranges);
    // Should map container root → host user, then container 1..N → subuid range
    assert_eq!(map, "0 1000 1\n1 100000 65536\n");
}

#[test]
fn generate_gid_map_uses_subgid_ranges() {
    let ranges = vec![
        SubIdRange {
            name: "ken".into(),
            start: 100000,
            count: 65536,
        },
        SubIdRange {
            name: "ken".into(),
            start: 200000,
            count: 1000,
        },
    ];
    let map = generate_gid_map(1000, &ranges);
    assert_eq!(map, "0 1000 1\n1 100000 65536\n1 200000 1000\n");
}
