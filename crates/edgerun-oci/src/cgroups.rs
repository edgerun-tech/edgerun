//! Cgroups v2 resource management.
//!
//! Pure file I/O to `/sys/fs/cgroup` — no cgroup library dependency.
//!
//! Supports: memory, cpu, pids, blockIO, hugepage_limits, network.

use crate::prelude::*;
use std::fs;
use std::io;
use std::io::Write;
use std::path::Path;

use crate::spec::OciLinuxResources;

/// Write to a cgroup file, logging errors to /dev/kmsg (best-effort).
pub fn cgroup_write(cgroup_root: &Path, file: &str, content: &str) {
    if let Err(e) = cgroup_write_result(cgroup_root, file, content) {
        if let Ok(mut kmsg) = fs::OpenOptions::new().write(true).open("/dev/kmsg") {
            let _ = writeln!(
                kmsg,
                "edgerun: cgroup write error {}/{}: {}",
                cgroup_root.display(),
                file,
                e
            );
        }
    }
}

/// Write to a cgroup file, returning an error if write fails.
pub fn cgroup_write_result(cgroup_root: &Path, file: &str, content: &str) -> io::Result<()> {
    fs::write(cgroup_root.join(file), content)
}

/// Append to a cgroup file, returning an error if append fails.
pub fn cgroup_append_result(cgroup_root: &Path, file: &str, content: &str) -> io::Result<()> {
    let mut f = fs::OpenOptions::new()
        .append(true)
        .open(cgroup_root.join(file))?;
    writeln!(f, "{}", content)?;
    Ok(())
}

/// Append to a cgroup file (for per-device entries like io.weight).
pub fn cgroup_append(cgroup_root: &Path, file: &str, content: &str) {
    use std::io::Write;
    if let Ok(mut f) = fs::OpenOptions::new()
        .append(true)
        .open(cgroup_root.join(file))
    {
        let _ = writeln!(f, "{}", content);
    }
}

/// Write to a specific cgroup file.
pub fn setup_container_cgroups_from_file(cgroup_root: &Path, file: &str, content: &str) {
    cgroup_write(cgroup_root, file, content);
}

/// Write to a specific cgroup file and fail on errors.
pub fn setup_container_cgroups_from_file_result(
    cgroup_root: &Path,
    file: &str,
    content: &str,
) -> io::Result<()> {
    cgroup_write_result(cgroup_root, file, content)
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
                    cgroup_write(
                        &cgroup_root,
                        "cpu.max.rt",
                        &format!("{} {}", rt_runtime, rt_period),
                    );
                }
            }
        }
        if let Some(shares) = cpu.shares {
            if shares > 0 {
                cgroup_write(
                    &cgroup_root,
                    "cpu.weight",
                    &format!("{}", shares_to_weight(shares)),
                );
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
                let v2_weight = (weight as u64).saturating_mul(100).clamp(1, 10000);
                cgroup_write(&cgroup_root, "io.bfq.weight", &format!("{}", v2_weight));
                // Also try io.weight (depends on IO scheduler)
                cgroup_write(&cgroup_root, "io.weight", &format!("{}", v2_weight));
            }
        }
        // Per-device weight (OCI weightDevice → cgroup v2 io.weight io.bfq.weight)
        if let Some(ref weight_devs) = blkio.weight_device {
            for wd in weight_devs {
                let entry = format!("{}:{}", wd.major, wd.minor);
                // Prefer the device-specific weight from the spec
                let dev_weight = wd
                    .weight
                    .map(|w| (w as u64).saturating_mul(100).clamp(1, 10000))
                    .unwrap_or(100); // default v2 weight if not specified
                let line = format!("{} {}", entry, dev_weight);
                cgroup_append(&cgroup_root, "io.bfq.weight", &line);
                cgroup_append(&cgroup_root, "io.weight", &line);
            }
        }
        // Per-device leaf weight (cgroup v1 only — skipped in v2)
        // leaf_weight_device is also cgroup v1 only — skipped
        // Throttle devices
        write_throttle_devices(
            &cgroup_root,
            "io.max",
            blkio.throttle_read_bps_device.as_deref(),
            "rbps",
        )?;
        write_throttle_devices(
            &cgroup_root,
            "io.max",
            blkio.throttle_write_bps_device.as_deref(),
            "wbps",
        )?;
        write_throttle_devices(
            &cgroup_root,
            "io.max",
            blkio.throttle_read_iops_device.as_deref(),
            "riops",
        )?;
        write_throttle_devices(
            &cgroup_root,
            "io.max",
            blkio.throttle_write_iops_device.as_deref(),
            "wiops",
        )?;
    }

    // Hugepage limits
    if let Some(ref hugepages) = resources.hugepage_limits {
        for hp in hugepages {
            cgroup_write(
                &cgroup_root,
                &format!("hugetlb.{}.max", hp.pagesize),
                &format!("{}", hp.limit),
            );
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
    devices: Option<&[crate::spec::OciLinuxThrottleDevice]>,
    key: &str,
) -> io::Result<()> {
    let Some(devices) = devices else {
        return Ok(());
    };
    if devices.is_empty() {
        return Ok(());
    };

    let content = devices
        .iter()
        .map(|d| format!("{}:{} {}={}", d.major, d.minor, key, d.rate))
        .collect::<Vec<_>>()
        .join("\n");

    cgroup_write(cgroup_root, file, &content);
    Ok(())
}

/// Convert legacy cpu.shares to cgroup v2 cpu.weight.
/// Uses saturating arithmetic to prevent overflow with large shares values.
pub fn shares_to_weight(shares: u64) -> u64 {
    if shares <= 2 {
        return 1;
    }
    // Use saturating_mul to prevent overflow: (shares - 2) * 9999
    let w = 1 + (shares - 2).saturating_mul(9999) / 262142;
    w.clamp(1, 10000)
}
