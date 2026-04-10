//! Cgroups v2 resource management.
//!
//! Pure file I/O to `/sys/fs/cgroup` — no cgroup library dependency.

use std::fs;
use std::io;
use std::path::Path;

use crate::json::OciLinuxResources;

/// Apply cgroup v2 resource limits by writing to /sys/fs/cgroup.
pub fn setup_cgroups(pid: u32, resources: &OciLinuxResources, cgroup_path: &str) -> io::Result<()> {
    let cgroup_root = Path::new("/sys/fs/cgroup").join(cgroup_path.trim_start_matches('/'));
    fs::create_dir_all(&cgroup_root)?;

    // Move PID into cgroup
    fs::write(cgroup_root.join("cgroup.procs"), format!("{}", pid))?;

    // Memory limits
    if let Some(ref mem) = resources.memory {
        if let Some(limit) = mem.limit {
            if limit >= 0 {
                let _ = fs::write(cgroup_root.join("memory.max"), format!("{}", limit));
            }
        }
        if let Some(swap) = mem.swap {
            if swap >= 0 {
                let _ = fs::write(cgroup_root.join("memory.swap.max"), format!("{}", swap));
            }
        }
    }

    // CPU limits
    if let Some(ref cpu) = resources.cpu {
        if let Some(period) = cpu.period {
            if period > 0 {
                let quota = cpu.quota.unwrap_or(-1);
                let _ = fs::write(cgroup_root.join("cpu.max"), format!("{} {}", quota, period));
            }
        }
        if let Some(shares) = cpu.shares {
            if shares > 0 {
                let _ = fs::write(cgroup_root.join("cpu.weight"), format!("{}", shares_to_weight(shares)));
            }
        }
    }

    // PID limits
    if let Some(ref pids) = resources.pids {
        if pids.limit > 0 {
            let _ = fs::write(cgroup_root.join("pids.max"), format!("{}", pids.limit));
        }
    }

    Ok(())
}

/// Convert legacy cpu.shares to cgroup v2 cpu.weight.
/// Uses saturating arithmetic to prevent overflow with large shares values.
pub fn shares_to_weight(shares: u64) -> u64 {
    if shares <= 2 { return 1; }
    // Use saturating_mul to prevent overflow: (shares - 2) * 9999
    let w = 1 + (shares - 2).saturating_mul(9999) / 262142;
    w.min(10000).max(1)
}
