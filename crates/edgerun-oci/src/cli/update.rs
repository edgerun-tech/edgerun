//! Update command implementation.
//!
//! Update container resource limits (live cgroup config).
//! Supports: memory limits, CPU shares/quota/period, PIDs limit, block I/O limits.

use std::fs;
use std::io;

use crate::json::OciSpec;
use crate::state::load_state;

/// Parsed update options from CLI flags.
#[derive(Debug, Default)]
struct UpdateOpts {
    /// Memory limit in bytes
    memory: Option<i64>,
    /// Memory swap limit in bytes
    memory_swap: Option<i64>,
    /// CPU shares (legacy)
    cpu_shares: Option<u64>,
    /// CPU quota (microseconds)
    cpu_quota: Option<i64>,
    /// CPU period (microseconds)
    cpu_period: Option<u64>,
    /// CPU real-time runtime (microseconds)
    cpu_rt_runtime: Option<u64>,
    /// CPU real-time period (microseconds)
    cpu_rt_period: Option<u64>,
    /// CPU affinity (cpuset.cpus)
    cpuset_cpus: Option<String>,
    /// CPU memory nodes (cpuset.mems)
    cpuset_mems: Option<String>,
    /// PIDs limit
    pids_limit: Option<i64>,
    /// Block I/O weight
    blkio_weight: Option<u64>,
}

pub fn cmd_update(opts: &crate::cli::GlobalOpts, args: &[String]) -> io::Result<()> {
    if let Some(ref root) = opts.root {
        crate::state::set_state_dir(root.to_str().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "--root path is not valid UTF-8",
            )
        })?);
    }

    if args.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "container ID required",
        ));
    }

    let id = &args[0];
    let update_opts = parse_update_flag(&args[1..])?;

    let state = load_state(id)?;
    let pid = state
        .pid
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "container has no PID"))?;

    if !crate::cli::is_process_alive(pid) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("container {} is not running", id),
        ));
    }

    // Load spec for cgroup path
    let bundle = &state.bundle;
    let config_path = std::path::Path::new(bundle).join("config.json");
    let spec: OciSpec = if let Ok(data) = fs::read(&config_path) {
        crate::json::parse_oci_spec(&data).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid OCI config: {}", e),
            )
        })?
    } else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "cannot read config.json",
        ));
    };

    let cgroup_path = spec
        .linux
        .as_ref()
        .and_then(|l| l.cgroups_path.as_ref())
        .cloned()
        .unwrap_or_else(|| "/edgerun".into());

    let cgroup_root =
        std::path::Path::new("/sys/fs/cgroup").join(cgroup_path.trim_start_matches('/'));

    // Apply updates
    apply_update(&cgroup_root, &update_opts)?;

    // Update the spec file with the new values
    update_spec_config(&config_path, &update_opts)?;

    Ok(())
}

fn apply_update(cgroup_root: &std::path::Path, opts: &UpdateOpts) -> io::Result<()> {
    // Memory
    if let Some(limit) = opts.memory {
        if limit >= 0 {
            crate::cgroups::setup_container_cgroups_from_file(
                cgroup_root,
                "memory.max",
                &limit.to_string(),
            );
        }
    }
    if let Some(swap) = opts.memory_swap {
        if swap >= 0 {
            crate::cgroups::setup_container_cgroups_from_file(
                cgroup_root,
                "memory.swap.max",
                &swap.to_string(),
            );
        }
    }

    // CPU
    if let Some(shares) = opts.cpu_shares {
        let weight = crate::cgroups::shares_to_weight(shares);
        crate::cgroups::setup_container_cgroups_from_file(
            cgroup_root,
            "cpu.weight",
            &weight.to_string(),
        );
    }
    if let Some(quota) = opts.cpu_quota {
        let period = opts.cpu_period.unwrap_or(100_000);
        crate::cgroups::setup_container_cgroups_from_file(
            cgroup_root,
            "cpu.max",
            &format!("{} {}", quota, period),
        );
    } else if let Some(period) = opts.cpu_period {
        // If only period is set, read current quota
        if let Ok(current) = fs::read_to_string(cgroup_root.join("cpu.max")) {
            if let Some(quota_str) = current.split_whitespace().next() {
                crate::cgroups::setup_container_cgroups_from_file(
                    cgroup_root,
                    "cpu.max",
                    &format!("{} {}", quota_str.trim(), period),
                );
            }
        }
    }
    if let Some(rt_runtime) = opts.cpu_rt_runtime {
        if let Some(rt_period) = opts.cpu_rt_period {
            crate::cgroups::setup_container_cgroups_from_file(
                cgroup_root,
                "cpu.max.rt",
                &format!("{} {}", rt_runtime, rt_period),
            );
        }
    }
    if let Some(ref cpus) = opts.cpuset_cpus {
        if !cpus.is_empty() {
            crate::cgroups::setup_container_cgroups_from_file(cgroup_root, "cpuset.cpus", cpus);
        }
    }
    if let Some(ref mems) = opts.cpuset_mems {
        if !mems.is_empty() {
            crate::cgroups::setup_container_cgroups_from_file(cgroup_root, "cpuset.mems", mems);
        }
    }

    // PIDs
    if let Some(limit) = opts.pids_limit {
        if limit > 0 {
            crate::cgroups::setup_container_cgroups_from_file(
                cgroup_root,
                "pids.max",
                &limit.to_string(),
            );
        }
    }

    // Block I/O
    if let Some(weight) = opts.blkio_weight {
        if weight > 0 {
            let v2_weight = weight.saturating_mul(100).clamp(1, 10000);
            crate::cgroups::setup_container_cgroups_from_file(
                cgroup_root,
                "io.weight",
                &v2_weight.to_string(),
            );
        }
    }

    Ok(())
}

fn update_spec_config(_config_path: &std::path::Path, _opts: &UpdateOpts) -> io::Result<()> {
    // In a full implementation, we'd parse the config.json, update the
    // linux.resources section, and write it back. For now, cgroup files
    // are updated directly and the config.json is a template for new containers.
    Ok(())
}

fn parse_update_flag(args: &[String]) -> io::Result<UpdateOpts> {
    let mut opts = UpdateOpts::default();
    let mut i = 0;

    while i < args.len() {
        match args[i].as_str() {
            "--memory" | "-m" => {
                i += 1;
                if i >= args.len() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--memory requires a value",
                    ));
                }
                opts.memory = Some(parse_memory_arg(&args[i])?);
            }
            "--memory-swap" => {
                i += 1;
                if i >= args.len() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--memory-swap requires a value",
                    ));
                }
                opts.memory_swap = Some(parse_memory_arg(&args[i])?);
            }
            "--cpu-shares" | "-c" => {
                i += 1;
                if i >= args.len() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--cpu-shares requires a value",
                    ));
                }
                opts.cpu_shares = Some(args[i].parse().map_err(|_| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!("invalid cpu-shares value: {}", args[i]),
                    )
                })?);
            }
            "--cpu-quota" => {
                i += 1;
                if i >= args.len() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--cpu-quota requires a value",
                    ));
                }
                opts.cpu_quota = Some(args[i].parse().map_err(|_| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!("invalid cpu-quota value: {}", args[i]),
                    )
                })?);
            }
            "--cpu-period" => {
                i += 1;
                if i >= args.len() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--cpu-period requires a value",
                    ));
                }
                opts.cpu_period = Some(args[i].parse().map_err(|_| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!("invalid cpu-period value: {}", args[i]),
                    )
                })?);
            }
            "--cpu-rt-runtime" => {
                i += 1;
                if i >= args.len() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--cpu-rt-runtime requires a value",
                    ));
                }
                opts.cpu_rt_runtime = Some(args[i].parse().map_err(|_| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!("invalid cpu-rt-runtime value: {}", args[i]),
                    )
                })?);
            }
            "--cpu-rt-period" => {
                i += 1;
                if i >= args.len() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--cpu-rt-period requires a value",
                    ));
                }
                opts.cpu_rt_period = Some(args[i].parse().map_err(|_| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!("invalid cpu-rt-period value: {}", args[i]),
                    )
                })?);
            }
            "--cpuset-cpus" => {
                i += 1;
                if i >= args.len() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--cpuset-cpus requires a value",
                    ));
                }
                opts.cpuset_cpus = Some(args[i].clone());
            }
            "--cpuset-mems" => {
                i += 1;
                if i >= args.len() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--cpuset-mems requires a value",
                    ));
                }
                opts.cpuset_mems = Some(args[i].clone());
            }
            "--pids-limit" => {
                i += 1;
                if i >= args.len() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--pids-limit requires a value",
                    ));
                }
                opts.pids_limit = Some(args[i].parse().map_err(|_| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!("invalid pids-limit value: {}", args[i]),
                    )
                })?);
            }
            "--blkio-weight" => {
                i += 1;
                if i >= args.len() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--blkio-weight requires a value",
                    ));
                }
                opts.blkio_weight = Some(args[i].parse().map_err(|_| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!("invalid blkio-weight value: {}", args[i]),
                    )
                })?);
            }
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("unknown update flag: {}", args[i]),
                ));
            }
        }
        i += 1;
    }

    if opts.memory.is_none()
        && opts.memory_swap.is_none()
        && opts.cpu_shares.is_none()
        && opts.cpu_quota.is_none()
        && opts.cpu_period.is_none()
        && opts.pids_limit.is_none()
        && opts.blkio_weight.is_none()
        && opts.cpuset_cpus.is_none()
        && opts.cpuset_mems.is_none()
        && opts.cpu_rt_runtime.is_none()
        && opts.cpu_rt_period.is_none()
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "at least one resource limit must be specified",
        ));
    }

    Ok(opts)
}

/// Parse a memory argument like "512m", "1g", "1048576" into bytes.
fn parse_memory_arg(s: &str) -> io::Result<i64> {
    let s = s.trim();
    if s.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "empty memory value",
        ));
    }

    let (num_str, multiplier) = if s.ends_with('g') || s.ends_with('G') {
        (&s[..s.len() - 1], 1024i64 * 1024 * 1024)
    } else if s.ends_with('m') || s.ends_with('M') {
        (&s[..s.len() - 1], 1024i64 * 1024)
    } else if s.ends_with('k') || s.ends_with('K') {
        (&s[..s.len() - 1], 1024i64)
    } else {
        (s, 1i64)
    };

    let num: i64 = num_str.parse().map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid memory value: {}", s),
        )
    })?;

    Ok(num * multiplier)
}
