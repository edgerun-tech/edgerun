//! Update command implementation.
//!
//! Update container resource limits (live cgroup config).
//! Supports: memory limits, CPU shares/quota/period, PIDs limit, block I/O limits.

use crate::prelude::*;
use std::fs;
use std::io;

use crate::cli::{invalid_input, parse_cli_args, required_positional};
use crate::spec::OciSpec;
use crate::state::load_state;
use edgerun_clap::{Arg, Command};

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
    crate::cli::apply_global_opts(opts)?;

    let (id, update_opts) = parse_update_args(args)?;

    let state = load_state(&id)?;
    if state.status != "running" {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("container {} is not running (status: {})", id, state.status),
        ));
    }
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
    let mut spec: OciSpec = if let Ok(data) = fs::read(&config_path) {
        crate::spec::parse_oci_spec(&data).map_err(|e| {
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

    let cgroup_root = crate::cli::cgroup_dir_path(&cgroup_path)?;

    // Apply updates
    apply_update(&cgroup_root, &update_opts)?;

    // Update the spec file with the new values
    update_spec_config(&config_path, &mut spec, &update_opts)?;

    Ok(())
}

fn apply_update(cgroup_root: &std::path::Path, opts: &UpdateOpts) -> io::Result<()> {
    let format_limit = |value: i64, name: &str| -> io::Result<String> {
        if value >= 0 {
            return Ok(value.to_string());
        }
        if value == -1 {
            return Ok("max".to_string());
        }
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{name} must be -1 or a non-negative value"),
        ))
    };

    // Memory
    if let Some(limit) = opts.memory {
        let value = format_limit(limit, "memory limit")?;
        crate::cgroups::setup_container_cgroups_from_file_result(cgroup_root, "memory.max", &value)?;
    }
    if let Some(swap) = opts.memory_swap {
        let value = format_limit(swap, "memory swap limit")?;
        crate::cgroups::setup_container_cgroups_from_file_result(
            cgroup_root,
            "memory.swap.max",
            &value,
        )?;
    }

    // CPU
    if let Some(shares) = opts.cpu_shares {
        let weight = crate::cgroups::shares_to_weight(shares);
        crate::cgroups::setup_container_cgroups_from_file_result(
            cgroup_root,
            "cpu.weight",
            &weight.to_string(),
        )?;
    }
    if let Some(quota) = opts.cpu_quota {
        let period = opts.cpu_period.unwrap_or(100_000);
        if period == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "cpu period must be greater than zero",
            ));
        }
        let quota = format_limit(quota, "cpu quota")?;
        crate::cgroups::setup_container_cgroups_from_file_result(
            cgroup_root,
            "cpu.max",
            &format!("{} {}", quota, period),
        )?;
    } else if let Some(period) = opts.cpu_period {
        if period == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "cpu period must be greater than zero",
            ));
        }
        // If only period is set, read current quota
        let current = fs::read_to_string(cgroup_root.join("cpu.max")).map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("failed to read current cpu.max: {error}"),
            )
        })?;
        let quota_str = current
            .split_whitespace()
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "cpu.max is empty"))?
            .trim();
        if quota_str.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "failed to read current cpu.max quota",
            ));
        }
        crate::cgroups::setup_container_cgroups_from_file_result(
            cgroup_root,
            "cpu.max",
            &format!("{} {}", quota_str, period),
        )?;
    }
    if let Some(rt_runtime) = opts.cpu_rt_runtime {
        if let Some(rt_period) = opts.cpu_rt_period {
            crate::cgroups::setup_container_cgroups_from_file_result(
                cgroup_root,
                "cpu.max.rt",
                &format!("{} {}", rt_runtime, rt_period),
            )?;
        }
    }
    if let Some(ref cpus) = opts.cpuset_cpus {
        if !cpus.is_empty() {
            crate::cgroups::setup_container_cgroups_from_file_result(
                cgroup_root,
                "cpuset.cpus",
                cpus,
            )?;
        }
    }
    if let Some(ref mems) = opts.cpuset_mems {
        if !mems.is_empty() {
            crate::cgroups::setup_container_cgroups_from_file_result(
                cgroup_root,
                "cpuset.mems",
                mems,
            )?;
        }
    }

    // PIDs
    if let Some(limit) = opts.pids_limit {
        let value = match limit {
            -1 => "max".to_string(),
            0 => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "pids limit must be -1 or greater than zero",
                ))
            }
            _ => limit.to_string(),
        };
        crate::cgroups::setup_container_cgroups_from_file_result(cgroup_root, "pids.max", &value)?;
    }

    // Block I/O
    if let Some(weight) = opts.blkio_weight {
        if weight > 0 {
            let v2_weight = weight.saturating_mul(100).clamp(1, 10000);
            crate::cgroups::setup_container_cgroups_from_file_result(
                cgroup_root,
                "io.weight",
                &v2_weight.to_string(),
            )?;
        }
    }

    Ok(())
}

fn update_spec_config(
    config_path: &std::path::Path,
    spec: &mut OciSpec,
    opts: &UpdateOpts,
) -> io::Result<()> {
    let linux = spec.linux.get_or_insert_with(Default::default);
    let resources = linux.resources.get_or_insert_with(Default::default);
    let cpu = resources.cpu.get_or_insert_with(Default::default);
    let memory = resources.memory.get_or_insert_with(Default::default);
    let pids = resources.pids.get_or_insert_with(Default::default);
    let block_io = resources.block_io.get_or_insert_with(Default::default);

    if let Some(limit) = opts.memory {
        memory.limit = Some(limit);
    }
    if let Some(swap) = opts.memory_swap {
        memory.swap = Some(swap);
    }
    if let Some(shares) = opts.cpu_shares {
        cpu.shares = Some(shares);
    }
    if let Some(quota) = opts.cpu_quota {
        cpu.quota = Some(quota);
    }
    if let Some(period) = opts.cpu_period {
        cpu.period = Some(period);
    }
    if let Some(rt_runtime) = opts.cpu_rt_runtime {
        cpu.realtime_runtime = Some(rt_runtime.try_into().map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "--cpu-rt-runtime exceeds supported signed 64-bit range",
            )
        })?);
    }
    if let Some(rt_period) = opts.cpu_rt_period {
        cpu.realtime_period = Some(rt_period);
    }
    if let Some(ref cpus) = opts.cpuset_cpus {
        cpu.cpus = Some(cpus.clone());
    }
    if let Some(ref mems) = opts.cpuset_mems {
        cpu.mems = Some(mems.clone());
    }
    if let Some(limit) = opts.pids_limit {
        pids.limit = limit;
    }
    if let Some(weight) = opts.blkio_weight {
        if weight > 0 {
            let stored_weight = if weight > 10_000 { 10_000 } else { weight };
            block_io.weight = Some(stored_weight as u16);
        }
    }

    let tmp_path = config_path.with_extension(format!("json.new.{}", std::process::id()));
    let data = spec.to_json_string_pretty();

    fs::write(&tmp_path, data.as_bytes())?;
    fs::rename(&tmp_path, config_path).map_err(|err| {
        let _ = fs::remove_file(&tmp_path);
        err
    })?;

    Ok(())
}

fn parse_update_args(args: &[String]) -> io::Result<(String, UpdateOpts)> {
    const USAGE: &str = "Usage: ert update <container-id> [resource flags]";
    let matches = parse_cli_args(
        Command::new("update")
            .arg(Arg::new("memory").short('m').long("memory"))
            .arg(Arg::new("memory-swap").long("memory-swap"))
            .arg(Arg::new("cpu-shares").short('c').long("cpu-shares"))
            .arg(Arg::new("cpu-quota").long("cpu-quota"))
            .arg(Arg::new("cpu-period").long("cpu-period"))
            .arg(Arg::new("cpu-rt-runtime").long("cpu-rt-runtime"))
            .arg(Arg::new("cpu-rt-period").long("cpu-rt-period"))
            .arg(Arg::new("cpuset-cpus").long("cpuset-cpus"))
            .arg(Arg::new("cpuset-mems").long("cpuset-mems"))
            .arg(Arg::new("pids-limit").long("pids-limit"))
            .arg(Arg::new("blkio-weight").long("blkio-weight")),
        args,
        USAGE,
    )?;
    if matches.positional_count() > 1 {
        return Err(invalid_input(USAGE));
    }

    let id = required_positional(&matches, 0, "container ID required")?.to_string();
    crate::cli::validate_container_id(&id)?;
    let mut opts = UpdateOpts::default();

    if let Some(memory) = matches.get_one::<String>("memory") {
        let value = parse_memory_arg(&memory)?;
        if value < -1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "memory limit must be -1 or a non-negative value",
            ));
        }
        opts.memory = Some(value);
    }
    if let Some(memory_swap) = matches.get_one::<String>("memory-swap") {
        let value = parse_memory_arg(&memory_swap)?;
        if value < -1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "memory swap limit must be -1 or a non-negative value",
            ));
        }
        opts.memory_swap = Some(value);
    }
    if let Some(cpu_shares) = matches.get_one::<String>("cpu-shares") {
        opts.cpu_shares = Some(parse_u64_arg("cpu-shares", &cpu_shares)?);
    }
    if let Some(cpu_quota) = matches.get_one::<String>("cpu-quota") {
        let value = parse_i64_arg("cpu-quota", &cpu_quota)?;
        if value < -1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "cpu quota must be -1 or a non-negative value",
            ));
        }
        opts.cpu_quota = Some(value);
    }
    if let Some(cpu_period) = matches.get_one::<String>("cpu-period") {
        let period = parse_u64_arg("cpu-period", &cpu_period)?;
        if period == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "cpu period must be greater than zero",
            ));
        }
        opts.cpu_period = Some(period);
    }
    if let Some(cpu_rt_runtime) = matches.get_one::<String>("cpu-rt-runtime") {
        let runtime = parse_u64_arg("cpu-rt-runtime", &cpu_rt_runtime)?;
        if runtime == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "cpu rt runtime must be greater than zero",
            ));
        }
        opts.cpu_rt_runtime = Some(runtime);
    }
    if let Some(cpu_rt_period) = matches.get_one::<String>("cpu-rt-period") {
        let period = parse_u64_arg("cpu-rt-period", &cpu_rt_period)?;
        if period == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "cpu rt period must be greater than zero",
            ));
        }
        opts.cpu_rt_period = Some(period);
    }
    if let Some(cpuset_cpus) = matches.get_one::<String>("cpuset-cpus") {
        if cpuset_cpus.trim().is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "cpuset-cpus cannot be empty",
            ));
        }
        opts.cpuset_cpus = Some(cpuset_cpus);
    }
    if let Some(cpuset_mems) = matches.get_one::<String>("cpuset-mems") {
        if cpuset_mems.trim().is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "cpuset-mems cannot be empty",
            ));
        }
        opts.cpuset_mems = Some(cpuset_mems);
    }
    if let Some(pids_limit) = matches.get_one::<String>("pids-limit") {
        let limit = parse_i64_arg("pids-limit", &pids_limit)?;
        if limit < -1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "pids limit must be -1 or greater than zero",
            ));
        }
        opts.pids_limit = Some(limit);
    }

    if opts.cpu_rt_runtime.is_some() != opts.cpu_rt_period.is_some() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "cpu rt runtime and cpu rt period must be provided together",
        ));
    }
    if let Some(blkio_weight) = matches.get_one::<String>("blkio-weight") {
        let value = parse_u64_arg("blkio-weight", &blkio_weight)?;
        if value == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "blkio-weight must be greater than zero",
            ));
        }
        if value > 10_000 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "blkio-weight must be <= 10000",
            ));
        }
        opts.blkio_weight = Some(value);
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

    Ok((id, opts))
}

fn parse_i64_arg(name: &str, value: &str) -> io::Result<i64> {
    value.parse().map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid {name} value: {value}"),
        )
    })
}

fn parse_u64_arg(name: &str, value: &str) -> io::Result<u64> {
    value.parse().map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid {name} value: {value}"),
        )
    })
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
    num.checked_mul(multiplier).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("memory value overflow: {s}"),
        )
    })
}
