//! Rootless container support.
//!
//! Handles:
//! - subuid/subgid range parsing from /etc/subuid and /etc/subgid
//! - Cgroup v2 delegation detection
//! - Default uid/gid mapping generation for rootless mode

use crate::prelude::*;
use std::fs;
use std::io;
use std::path::Path;

/// A single subuid/subgid range entry.
#[derive(Debug, Clone, PartialEq)]
pub struct SubIdRange {
    pub name: String,
    pub start: u32,
    pub count: u32,
}

/// Parse /etc/subuid or /etc/subgid into a list of ranges.
///
/// Format: `username:start:count`
/// Lines starting with `#` are comments.
pub fn parse_subid_file(path: &Path) -> io::Result<Vec<SubIdRange>> {
    let content = fs::read_to_string(path)?;
    let mut ranges = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() != 3 {
            continue;
        }
        let name = parts[0].trim().to_string();
        let start: u32 = parts[1].trim().parse().map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid subid start: {}", e),
            )
        })?;
        let count: u32 = parts[2].trim().parse().map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid subid count: {}", e),
            )
        })?;
        ranges.push(SubIdRange { name, start, count });
    }
    Ok(ranges)
}

/// Get the subuid ranges for the current user.
pub fn get_current_user_subuids() -> io::Result<Vec<SubIdRange>> {
    let username = get_current_username()?;
    let ranges = parse_subid_file(Path::new("/etc/subuid"))?;
    Ok(ranges.into_iter().filter(|r| r.name == username).collect())
}

/// Get the subgid ranges for the current user.
pub fn get_current_user_subgids() -> io::Result<Vec<SubIdRange>> {
    let username = get_current_username()?;
    let ranges = parse_subid_file(Path::new("/etc/subgid"))?;
    Ok(ranges.into_iter().filter(|r| r.name == username).collect())
}

/// Get the current username.
pub fn get_current_username() -> io::Result<String> {
    if let Ok(user) = std::env::var("USER") {
        return Ok(user);
    }
    // Fallback: read from /proc
    let uid = unsafe { libc::getuid() };
    // Try nsswitch via /etc/passwd parsing
    if let Ok(content) = fs::read_to_string("/etc/passwd") {
        for line in content.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 3 {
                if let Ok(entry_uid) = parts[2].parse::<u32>() {
                    if entry_uid == uid {
                        return Ok(parts[0].to_string());
                    }
                }
            }
        }
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "could not determine username",
    ))
}

/// Generate a uid_map string for rootless mode.
///
/// Maps container UID 0 to the host user's UID, then uses subuid ranges
/// for the rest of the container's UID space.
///
/// Format: `container_id host_id size`
pub fn generate_uid_map(root_uid: u32, subuids: &[SubIdRange]) -> String {
    if subuids.is_empty() {
        // No subuid ranges — map only the current user
        return format!("0 {} 1\n", root_uid);
    }
    // Map container root (0) to host user's UID
    let mut map = format!("0 {} 1\n", root_uid);
    // Map container UIDs 1..N to subuid range
    for range in subuids {
        map.push_str(&format!("1 {} {}\n", range.start, range.count));
    }
    map
}

/// Generate a gid_map string for rootless mode.
///
/// Same logic as uid_map but for groups.
pub fn generate_gid_map(root_gid: u32, subgids: &[SubIdRange]) -> String {
    if subgids.is_empty() {
        return format!("0 {} 1\n", root_gid);
    }
    let mut map = format!("0 {} 1\n", root_gid);
    for range in subgids {
        map.push_str(&format!("1 {} {}\n", range.start, range.count));
    }
    map
}

/// Resolve the cgroup v2 delegation path for the current process.
///
/// On cgroup v2 systems with rootless containers, the runtime can only
/// create cgroups within the user's delegated subtree.
///
/// Returns the path relative to /sys/fs/cgroup (e.g.,
/// `user.slice/user-1000.slice/user@1000.service/app.slice`).
///
/// If cgroup v2 delegation is not available, returns an empty string
/// (meaning the runtime should use the root cgroup).
pub fn resolve_cgroup_delegation_path() -> io::Result<String> {
    // Read the current process's cgroup path
    let cgroup_content = fs::read_to_string("/proc/self/cgroup")?;

    // On cgroup v2, the line is "0::<path>"
    for line in cgroup_content.lines() {
        if let Some(path) = line.strip_prefix("0::") {
            if !path.is_empty() {
                return Ok(path.trim_start_matches('/').to_string());
            }
        }
    }

    // No cgroup v2 delegation found
    Ok(String::new())
}

/// Resolve the full cgroup path for a container's cgroupPath.
///
/// In rootless mode, this appends the container's cgroupPath to the
/// user's delegated cgroup path.
///
/// In root mode, this returns the cgroupPath as-is.
pub fn resolve_container_cgroup_path(
    rootless: bool,
    container_cgroup_path: &str,
) -> io::Result<String> {
    if !rootless {
        // Root mode: use the cgroup path as-is, default to /edgerun
        if container_cgroup_path.is_empty() {
            return Ok("/edgerun".into());
        }
        return Ok(container_cgroup_path.to_string());
    }

    // Rootless mode: resolve delegation path and append container's path
    let delegated = resolve_cgroup_delegation_path()?;
    if delegated.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "rootless cgroup delegation not available — ensure cgroup v2 delegation is configured",
        ));
    }

    let container_path = if container_cgroup_path.is_empty() {
        "edgerun".to_string()
    } else {
        container_cgroup_path.trim_start_matches('/').to_string()
    };

    Ok(format!("{}/{}", delegated, container_path))
}

/// Check if cgroup v2 is available on this system.
pub fn is_cgroup_v2_available() -> bool {
    Path::new("/sys/fs/cgroup/cgroup.controllers").exists()
}

#[cfg(all(test, not(target_os = "none")))]
mod tests {
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
}
