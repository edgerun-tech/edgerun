//! Intel RDT utilities — replaces `libcontainer::process::intel_rdt`.

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum IntelRdtError {
    #[error("failed to read mountinfo")]
    MountInfo(#[from] std::io::Error),
    #[error("resctrl mount point not found")]
    ResctrlMountPointNotFound,
}

/// Parse a single line from /proc/self/mountinfo.
///
/// Format: ID PARENT_ID MAJOR:MINOR ROOT MOUNT_POINT OPTIONS ... FS_TYPE SOURCE FS_OPTIONS ...
fn parse_mount_info_line(line: &str) -> Option<MountInfo> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    // Minimum fields: 1-6 are before the separator, then 7+ after
    // The separator is "-" at position 6
    if parts.len() < 10 || parts.get(6) != Some(&"-") {
        return None;
    }

    let mount_point = parts[4].to_string();
    let fs_type = parts[8].to_string();

    Some(MountInfo {
        mount_point,
        fs_type,
    })
}

struct MountInfo {
    mount_point: String,
    fs_type: String,
}

/// Find the mount point of the resctrl pseudo-filesystem.
///
/// Parses `/proc/self/mountinfo` looking for an entry with `fs_type == "resctrl"`.
pub fn find_resctrl_mount_point() -> Result<PathBuf, IntelRdtError> {
    let file = File::open("/proc/self/mountinfo")?;
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let line = line?;
        if let Some(mi) = parse_mount_info_line(&line) {
            if mi.fs_type == "resctrl" {
                let path = PathBuf::from(&mi.mount_point)
                    .canonicalize()
                    .map_err(IntelRdtError::from)?;
                return Ok(path);
            }
        }
    }

    Err(IntelRdtError::ResctrlMountPointNotFound)
}
