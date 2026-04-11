//! Local cgroup utilities — replaces the `libcgroups` crate from youki.
//!
//! Provides just enough cgroup v1/v2 functionality for the contest test
//! framework to compile and run against `edgerun-oci-runtime`.

use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

pub const DEFAULT_CGROUP_ROOT: &str = "/sys/fs/cgroup";

// ---------------------------------------------------------------------------
// Cgroup setup detection
// ---------------------------------------------------------------------------

/// Cgroup hierarchy mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CgroupSetup {
    /// Unified — pure cgroup v2 hierarchy
    Unified,
    /// Hybrid — cgroup v2 with cgroup v1 compatibility
    Hybrid,
    /// Legacy/Legacy — pure cgroup v1
    Legacy,
}

/// Detect the cgroup setup by reading `/proc/self/cgroup` and `/proc/self/mountinfo`.
pub fn get_cgroup_setup() -> Result<CgroupSetup> {
    let cgroup_file = "/proc/self/cgroup";
    let mountinfo_file = "/proc/self/mountinfo";

    let cgroup_content = fs::read_to_string(cgroup_file)
        .with_context(|| format!("failed to read {cgroup_file}"))?;

    // If cgroup v2 is in use, /proc/self/cgroup will have "0::/..."
    let has_v2 = cgroup_content
        .lines()
        .any(|line| line.starts_with("0::"));

    if has_v2 {
        // Check if there are also v1 mounts (hybrid mode)
        if Path::new("/sys/fs/cgroup/cgroup.controllers").exists() {
            // Pure unified hierarchy
            return Ok(CgroupSetup::Unified);
        }
    }

    // Check for v1 mounts
    let mountinfo = fs::read_to_string(mountinfo_file)
        .with_context(|| format!("failed to read {mountinfo_file}"))?;

    let has_v1 = mountinfo
        .lines()
        .any(|line| line.contains(" cgroup "));

    if has_v2 && has_v1 {
        Ok(CgroupSetup::Hybrid)
    } else if has_v1 {
        Ok(CgroupSetup::Legacy)
    } else if has_v2 {
        Ok(CgroupSetup::Unified)
    } else {
        Ok(CgroupSetup::Legacy)
    }
}

// ---------------------------------------------------------------------------
// Cgroup v2 controller helpers
// ---------------------------------------------------------------------------

/// Available cgroup v2 controller types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControllerType {
    Cpu,
    Io,
    Memory,
    Pids,
    HugeTlb,
    Misc,
}

impl ControllerType {
    fn as_str(&self) -> &'static str {
        match self {
            ControllerType::Cpu => "cpu",
            ControllerType::Io => "io",
            ControllerType::Memory => "memory",
            ControllerType::Pids => "pids",
            ControllerType::HugeTlb => "hugetlb",
            ControllerType::Misc => "misc",
        }
    }

    fn from_str(s: &str) -> Option<Self> {
        match s {
            "cpu" => Some(ControllerType::Cpu),
            "io" => Some(ControllerType::Io),
            "memory" => Some(ControllerType::Memory),
            "pids" => Some(ControllerType::Pids),
            "hugetlb" => Some(ControllerType::HugeTlb),
            "misc" => Some(ControllerType::Misc),
            _ => None,
        }
    }
}

/// Read available controllers from the root cgroup.
pub fn get_available_controllers(root: &str) -> Result<Vec<ControllerType>> {
    let controllers_path = Path::new(root).join("cgroup.controllers");
    let content = fs::read_to_string(&controllers_path)
        .with_context(|| format!("failed to read {controllers_path:?}"))?;

    let controllers: Vec<ControllerType> = content
        .split_whitespace()
        .filter_map(ControllerType::from_str)
        .collect();

    Ok(controllers)
}
