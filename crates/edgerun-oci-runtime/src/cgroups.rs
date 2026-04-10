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

/// Write to a cgroup file, logging errors to /dev/kmsg (best-effort).
fn cgroup_write(cgroup_root: &Path, file: &str, content: &str) {
    if let Err(e) = fs::write(cgroup_root.join(file), content) {
        if let Ok(mut kmsg) = fs::OpenOptions::new().write(true).open("/dev/kmsg") {
            let _ = writeln!(kmsg, "edgerun: cgroup write error {}/{}: {}", cgroup_root.display(), file, e);
        }
    }
}

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
                // cgroup v2 weight is 1-10000, blkio weight is 10-1000
                let v2_weight = (weight as u64).saturating_mul(100).min(10000).max(1);
                cgroup_write(&cgroup_root, "io.bfq.weight", &format!("{}", v2_weight));
                // Also try io.weight (depends on IO scheduler)
                cgroup_write(&cgroup_root, "io.weight", &format!("{}", v2_weight));
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
            cgroup_write(&cgroup_root, &format!("hugetlb.{}.max", hp.pagesize), &format!("{}", hp.limit));
        }
    }

    // Network class ID (cgroup v1 compatibility — written to net_cls.classid)
    if let Some(ref net) = resources.network {
        if let Some(class_id) = net.class_id {
            cgroup_write(&cgroup_root, "net_cls.classid", &format!("{}", class_id));
        }
        // Network priorities
        if let Some(ref priorities) = net.priorities {
            for p in priorities {
                cgroup_write(&cgroup_root, "net_prio.prioidx", &format!("{} {}", p.name, p.priority));
            }
        }
    }

    // Device cgroup rules (cgroup v1 — devices controller)
    // For cgroup v2, devices are managed via eBPF and are not supported here.
    if let Some(ref devices) = resources.devices {
        for dev in devices {
            let dev_type = &dev.ns_type;
            let major = dev.major.map(|m| m.to_string()).unwrap_or_else(|| "*".into());
            let minor = dev.minor.map(|m| m.to_string()).unwrap_or_else(|| "*".into());
            let access = dev.access.as_deref().unwrap_or("rwm");

            let rule = format!("{} {}:{} {}", dev_type, major, minor, access);
            // Write to devices.allow (v1) — best-effort for v2
            cgroup_write(&cgroup_root, "devices.allow", &rule);
            cgroup_write(&cgroup_root, "cgroup.devices.allow", &rule);
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
    w.min(10000).max(1)
}
