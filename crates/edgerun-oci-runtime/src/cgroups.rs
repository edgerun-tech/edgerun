//! Cgroups v2 resource management.
//!
//! Pure file I/O to `/sys/fs/cgroup` — no cgroup library dependency.
//!
//! Supports: memory, cpu, pids, blockIO, hugepage_limits, network.

use std::fs;
use std::io;
use std::io::Write;
use std::path::Path;

use crate::json::OciLinuxResources;

/// Enable cgroup v2 controllers in a parent's `cgroup.subtree_control`.
///
/// For rootless containers, the user's delegated cgroup subtree starts with
/// no controllers enabled. Before creating child cgroups that use controllers,
/// the parent must first enable them.
///
/// This is a best-effort operation — it may fail if the parent doesn't have
/// permission to modify subtree_control, or if controllers are already enabled.
pub fn enable_subtree_controllers(cgroup_path: &str, controllers: &[&str]) {
    let parent = Path::new("/sys/fs/cgroup").join(cgroup_path.trim_start_matches('/'));
    if !parent.exists() { return; }

    let subtree_file = parent.join("cgroup.subtree_control");
    let current = match fs::read_to_string(&subtree_file) {
        Ok(s) => s,
        Err(_) => return, // Can't read — skip
    };

    let to_enable: Vec<String> = controllers
        .iter()
        .filter(|c| !current.contains(**c))
        .map(|c| format!("+{}", c))
        .collect();

    if to_enable.is_empty() { return; }

    let content = to_enable.join(" ");
    if let Err(e) = fs::write(&subtree_file, &content) {
        // Best-effort — log to kmsg
        if let Ok(mut kmsg) = fs::OpenOptions::new().write(true).open("/dev/kmsg") {
            let _ = writeln!(kmsg, "edgerun: subtree_control enable failed for {}: {}", cgroup_path, e);
        }
    }
}

/// Write to a cgroup file, logging errors to /dev/kmsg (best-effort).
pub fn cgroup_write(cgroup_root: &Path, file: &str, content: &str) {
    if let Err(e) = fs::write(cgroup_root.join(file), content) {
        if let Ok(mut kmsg) = fs::OpenOptions::new().write(true).open("/dev/kmsg") {
            let _ = writeln!(kmsg, "edgerun: cgroup write error {}/{}: {}", cgroup_root.display(), file, e);
        }
    }
}

/// Append a line to a cgroup file (for per-device weight entries).
pub fn cgroup_append(cgroup_root: &Path, file: &str, content: &str) {
    if let Ok(mut f) = fs::OpenOptions::new().append(true).open(cgroup_root.join(file)) {
        let _ = writeln!(f, "{}", content);
    }
}

/// Write to a specific cgroup file (public for update command).
pub fn setup_container_cgroups_from_file(cgroup_root: &Path, file: &str, content: &str) {
    cgroup_write(cgroup_root, file, content);
}

/// Apply cgroup v2 resource limits by writing to /sys/fs/cgroup.
pub fn setup_cgroups(pid: u32, resources: &OciLinuxResources, cgroup_path: &str) -> io::Result<()> {
    // Determine which controllers this container will use
    let mut controllers = Vec::new();
    if resources.memory.is_some() { controllers.push("memory"); }
    if resources.cpu.is_some() { controllers.push("cpu"); }
    if resources.pids.is_some() { controllers.push("pids"); }
    if resources.block_io.is_some() { controllers.push("io"); }

    // Enable controllers in the parent's subtree_control (for rootless)
    if !controllers.is_empty() {
        enable_subtree_controllers(cgroup_path, &controllers);
    }

    let cgroup_root = Path::new("/sys/fs/cgroup").join(cgroup_path.trim_start_matches('/'));
    fs::create_dir_all(&cgroup_root)?;

    // Move PID into cgroup
    fs::write(cgroup_root.join("cgroup.procs"), format!("{}", pid))?;

    // Memory limits
    if let Some(ref mem) = resources.memory {
        if let Some(limit) = mem.limit {
            if limit >= 0 {
                cgroup_write(&cgroup_root, "memory.max", &format!("{}", limit));
            }
        }
        if let Some(swap) = mem.swap {
            if swap >= 0 {
                cgroup_write(&cgroup_root, "memory.swap.max", &format!("{}", swap));
            }
        }
        // Memory swappiness (0-100); cgroup v2 uses memory.swap.max=0 for no swap
        // For cgroup v1 compatibility, write to memory.swappiness
        if let Some(reservation) = mem.reservation {
            if reservation >= 0 {
                cgroup_write(&cgroup_root, "memory.low", &format!("{}", reservation));
            }
        }
        if let Some(kernel) = mem.kernel {
            if kernel >= 0 {
                cgroup_write(&cgroup_root, "memory.kmem.max", &format!("{}", kernel));
            }
        }
    }

    // CPU limits
    if let Some(ref cpu) = resources.cpu {
        if let Some(period) = cpu.period {
            if period > 0 {
                let quota = cpu.quota.unwrap_or(-1);
                cgroup_write(&cgroup_root, "cpu.max", &format!("{} {}", quota, period));
            }
        }
        // CPU realtime limits: cpu.max.rt
        if let Some(rt_runtime) = cpu.realtime_runtime {
            if let Some(rt_period) = cpu.realtime_period {
                if rt_period > 0 {
                    cgroup_write(&cgroup_root, "cpu.max.rt", &format!("{} {}", rt_runtime, rt_period));
                }
            }
        }
        if let Some(shares) = cpu.shares {
            if shares > 0 {
                cgroup_write(&cgroup_root, "cpu.weight", &format!("{}", shares_to_weight(shares)));
            }
        }
        // CPU affinity: cpuset.cpus and cpuset.mems
        if let Some(ref cpus) = cpu.cpus {
            if !cpus.is_empty() {
                cgroup_write(&cgroup_root, "cpuset.cpus", cpus);
            }
        }
        if let Some(ref mems) = cpu.mems {
            if !mems.is_empty() {
                cgroup_write(&cgroup_root, "cpuset.mems", mems);
            }
        }
        // CPU idle (OCI 1.1.0): 1 = idle (deprioritized), 0 = not idle
        if let Some(idle) = cpu.idle {
            if idle != 0 {
                cgroup_write(&cgroup_root, "cpu.idle", "1");
            }
        }
        // CFS burst (OCI 1.1.0): write to cpu.max.burst
        if let Some(burst) = cpu.burst {
            if burst > 0 {
                cgroup_write(&cgroup_root, "cpu.max.burst", &format!("{}", burst));
            }
        }
    }

    // PID limits
    if let Some(ref pids) = resources.pids {
        if pids.limit > 0 {
            cgroup_write(&cgroup_root, "pids.max", &format!("{}", pids.limit));
        }
    }

    // Block I/O limits
    if let Some(ref blkio) = resources.block_io {
        if let Some(weight) = blkio.weight {
            if weight > 0 {
                let v2_weight = (weight as u64).saturating_mul(100).clamp(1, 10000);
                cgroup_write(&cgroup_root, "io.bfq.weight", &format!("{}", v2_weight));
                cgroup_write(&cgroup_root, "io.weight", &format!("{}", v2_weight));
            }
        }
        // Per-device weight: OCI weightDevice → cgroup v2 "major:minor weight"
        // Must be written via append because the kernel file interface requires
        // each device entry to be added separately after the default weight.
        if let Some(ref weight_devs) = blkio.weight_device {
            for wd in weight_devs {
                if let Some(w) = wd.weight {
                    let v2_w = (w as u64).saturating_mul(100).clamp(1, 10000);
                    let entry = format!("{}:{}", wd.major, wd.minor);
                    let line = format!("{} {}", entry, v2_w);
                    // Write to io.bfq.weight and io.weight via append interface
                    if let Ok(mut f1) = fs::OpenOptions::new().append(true)
                        .open(cgroup_root.join("io.bfq.weight")) {
                        let _ = writeln!(f1, "{}", line);
                    }
                    if let Ok(mut f2) = fs::OpenOptions::new().append(true)
                        .open(cgroup_root.join("io.weight")) {
                        let _ = writeln!(f2, "{}", line);
                    }
                }
            }
        }
        // Throttle devices
        write_throttle_devices(&cgroup_root, "io.max", blkio.throttle_read_bps_device.as_deref(), "rbps")?;
        write_throttle_devices(&cgroup_root, "io.max", blkio.throttle_write_bps_device.as_deref(), "wbps")?;
        write_throttle_devices(&cgroup_root, "io.max", blkio.throttle_read_iops_device.as_deref(), "riops")?;
        write_throttle_devices(&cgroup_root, "io.max", blkio.throttle_write_iops_device.as_deref(), "wiops")?;
    }

    // Hugepage limits
    if let Some(ref hugepages) = resources.hugepage_limits {
        for hp in hugepages {
            // OCI 1.1.0: rsvd applies to reserved huge page accounting
            let file = if hp.rsvd == Some(true) {
                format!("hugetlb.{}.rsvd.max", hp.pagesize)
            } else {
                format!("hugetlb.{}.max", hp.pagesize)
            };
            cgroup_write(&cgroup_root, &file, &format!("{}", hp.limit));
        }
    }

    // Note: Network class ID and device cgroup rules are now handled via eBPF
    // programs attached to the cgroup (see below).

    // Network class ID and priority via eBPF
    if let Some(ref net) = resources.network {
        if let Some(class_id) = net.class_id {
            if class_id > 0 {
                let _ = crate::ebpf_netcls::setup_netcls_cgroup_ebpf(&cgroup_root, class_id);
            }
        }
        if let Some(ref priorities) = net.priorities {
            if !priorities.is_empty() {
                let _ = crate::ebpf_netcls::setup_netprio_cgroup_ebpf(&cgroup_root, priorities);
            }
        }
    }

    // Device cgroup rules via eBPF
    if let Some(ref devices) = resources.devices {
        if !devices.is_empty() {
            let _ = crate::ebpf_devices::setup_device_cgroup_ebpf(&cgroup_root, devices);
        }
    }

    Ok(())
}

/// Write throttle device limits to io.max.
///
/// Format: `<major>:<minor> <key>=<value>`
/// e.g., `8:0 rbps=102400`
fn write_throttle_devices(
    cgroup_root: &Path,
    file: &str,
    devices: Option<&[crate::json::OciLinuxThrottleDevice]>,
    key: &str,
) -> io::Result<()> {
    let Some(devices) = devices else { return Ok(()) };
    if devices.is_empty() { return Ok(()) };

    let content = devices.iter()
        .map(|d| format!("{}:{} {}={}", d.major, d.minor, key, d.rate))
        .collect::<Vec<_>>()
        .join("\n");

    cgroup_write(cgroup_root, file, &content);
    Ok(())
}

/// Convert legacy cpu.shares to cgroup v2 cpu.weight.
/// Uses saturating arithmetic to prevent overflow with large shares values.
pub fn shares_to_weight(shares: u64) -> u64 {
    if shares <= 2 { return 1; }
    // Use saturating_mul to prevent overflow: (shares - 2) * 9999
    let w = 1 + (shares - 2).saturating_mul(9999) / 262142;
    w.clamp(1, 10000)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shares_to_weight_zero_is_one() {
        assert_eq!(shares_to_weight(0), 1);
    }

    #[test]
    fn shares_to_weight_one_is_one() {
        assert_eq!(shares_to_weight(1), 1);
    }

    #[test]
    fn shares_to_weight_two_is_one() {
        assert_eq!(shares_to_weight(2), 1);
    }

    #[test]
    fn shares_to_weight_default_shares() {
        // Default cpu.shares is 1024 → weight should be ~39
        let w = shares_to_weight(1024);
        assert_eq!(w, 39);
    }

    #[test]
    fn shares_to_weight_512() {
        let w = shares_to_weight(512);
        assert_eq!(w, 20);
    }

    #[test]
    fn shares_to_weight_262144_is_max() {
        // 262144 is the standard max shares → weight should be 10000
        let w = shares_to_weight(262144);
        assert_eq!(w, 10000);
    }

    #[test]
    fn shares_to_weight_clamps_at_max() {
        // Very large shares should clamp to 10000
        let w = shares_to_weight(u64::MAX);
        assert_eq!(w, 10000);
    }

    #[test]
    fn shares_to_weight_monotonic() {
        // Higher shares should produce >= weight
        let w1 = shares_to_weight(100);
        let w2 = shares_to_weight(1000);
        let w3 = shares_to_weight(10000);
        assert!(w1 <= w2);
        assert!(w2 <= w3);
    }

    #[test]
    fn shares_to_weight_linear_approx() {
        // weight ≈ 1 + (shares - 2) * 9999 / 262142
        // For shares=1024: weight ≈ 1 + 1022 * 9999 / 262142 ≈ 1 + 39 = 40
        // (Actually 39 due to integer division)
        let w = shares_to_weight(1024);
        assert!((38..=40).contains(&w));
    }

    #[test]
    fn shares_to_weight_no_overflow() {
        // Even with max u64, saturating_mul prevents panic
        let w = shares_to_weight(u64::MAX - 1);
        assert_eq!(w, 10000);
    }

    #[test]
    fn shares_to_weight_edge_large() {
        let w = shares_to_weight(262145);
        assert_eq!(w, 10000); // Should clamp
    }

    #[test]
    fn enable_subtree_controllers_noop_for_missing_dir() {
        // Should not panic when directory doesn't exist
        enable_subtree_controllers("/nonexistent/container", &["memory", "cpu"]);
    }
}
