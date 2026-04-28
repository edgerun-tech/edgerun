//! Image-local user and group lookup helpers for run/exec.

use crate::prelude::*;
use std::fs;
use std::io;
use std::path::Path;

use crate::spec::OciUser;

pub(crate) fn validate_user_spec(value: &str) -> io::Result<String> {
    let mut parts = value.split(':');
    parts
        .next()
        .filter(|part| !part.is_empty())
        .ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "--user requires user[:group]")
        })?;
    let _group = parts.next();
    if parts.next().is_some() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "--user requires user[:group]",
        ));
    }
    Ok(value.to_string())
}

pub(crate) fn resolve_user(rootfs: &Path, value: &str) -> io::Result<OciUser> {
    validate_user_spec(value)?;

    let mut parts = value.split(':');
    let user_part = parts.next().unwrap_or_default();
    let group_part = parts.next();

    let passwd = read_passwd(rootfs)?;
    let groups = read_group(rootfs)?;
    let (uid, default_gid) = resolve_uid(user_part, &passwd)?;
    let gid = match group_part {
        Some("") => None,
        Some(group) => Some(resolve_gid(group, &groups)?),
        None => default_gid,
    };
    Ok(OciUser {
        uid: Some(uid),
        gid,
        additional_gids: None,
        umask: None,
    })
}

#[derive(Debug)]
struct PasswdEntry {
    name: String,
    uid: u32,
    gid: u32,
}

#[derive(Debug)]
struct GroupEntry {
    name: String,
    gid: u32,
}

fn read_passwd(rootfs: &Path) -> io::Result<Vec<PasswdEntry>> {
    let path = rootfs.join("etc/passwd");
    let data = match fs::read_to_string(&path) {
        Ok(data) => data,
        Err(error) if error.kind() == io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(error),
    };
    let mut entries = Vec::new();
    for line in data.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields = line.split(':').collect::<Vec<_>>();
        if fields.len() < 4 {
            continue;
        }
        let Ok(uid) = fields[2].parse::<u32>() else {
            continue;
        };
        let Ok(gid) = fields[3].parse::<u32>() else {
            continue;
        };
        entries.push(PasswdEntry {
            name: fields[0].to_string(),
            uid,
            gid,
        });
    }
    Ok(entries)
}

fn read_group(rootfs: &Path) -> io::Result<Vec<GroupEntry>> {
    let path = rootfs.join("etc/group");
    let data = match fs::read_to_string(&path) {
        Ok(data) => data,
        Err(error) if error.kind() == io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(error),
    };
    let mut entries = Vec::new();
    for line in data.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields = line.split(':').collect::<Vec<_>>();
        if fields.len() < 3 {
            continue;
        }
        let Ok(gid) = fields[2].parse::<u32>() else {
            continue;
        };
        entries.push(GroupEntry {
            name: fields[0].to_string(),
            gid,
        });
    }
    Ok(entries)
}

fn resolve_uid(value: &str, passwd: &[PasswdEntry]) -> io::Result<(u32, Option<u32>)> {
    if let Some(uid) = parse_numeric_id(value)? {
        return Ok((uid, None));
    }
    passwd
        .iter()
        .find(|entry| entry.name == value)
        .map(|entry| (entry.uid, Some(entry.gid)))
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("user not found in image: {value}"),
            )
        })
}

fn resolve_gid(value: &str, groups: &[GroupEntry]) -> io::Result<u32> {
    if let Some(gid) = parse_numeric_id(value)? {
        return Ok(gid);
    }
    groups
        .iter()
        .find(|entry| entry.name == value)
        .map(|entry| entry.gid)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("group not found in image: {value}"),
            )
        })
}

fn parse_numeric_id(value: &str) -> io::Result<Option<u32>> {
    if value.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "--user requires user[:group]",
        ));
    }
    if value.bytes().all(|b| b.is_ascii_digit()) {
        return value.parse::<u32>().map(Some).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("numeric id is out of range: {value}"),
            )
        });
    }
    if value.bytes().next().is_some_and(|b| b.is_ascii_digit()) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid numeric id: {value}"),
        ));
    }
    if !value
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'-'))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid user or group name: {value}"),
        ));
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
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
}
