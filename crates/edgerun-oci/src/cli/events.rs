//! Events command implementation — streams cgroup v2 stats as JSON.

use std::fs;
use std::io::{self, Write};

use crate::state::load_state;

pub fn cmd_events(opts: &crate::cli::GlobalOpts, args: &[String]) -> io::Result<()> {
    if let Some(ref root) = opts.root {
        crate::state::set_state_dir(root.to_str().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "--root path is not valid UTF-8",
            )
        })?);
    }

    let id = crate::cli::require_container_id(args)?;

    let state = load_state(id)?;
    let pid = state
        .pid
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "container has no PID"))?;

    let bundle = &state.bundle;
    let config_path = std::path::Path::new(bundle).join("config.json");
    let cgroup_path = if let Ok(data) = fs::read(&config_path) {
        if let Ok(spec) = crate::json::parse_oci_spec(&data) {
            spec.linux
                .as_ref()
                .and_then(|l| l.cgroups_path.as_ref())
                .cloned()
                .unwrap_or_else(|| "/edgerun".into())
        } else {
            "/edgerun".into()
        }
    } else {
        "/edgerun".into()
    };

    let cgroup_dir =
        std::path::Path::new("/sys/fs/cgroup").join(cgroup_path.trim_start_matches('/'));

    // Check if --interval is specified for streaming
    let interval_ms: u64 = if let Some(idx) = args.iter().position(|a| a == "--interval") {
        args.get(idx + 1)
            .and_then(|s| parse_interval(s))
            .unwrap_or(1000)
    } else {
        // No interval: just output once and exit
        let stats = read_cgroup_stats(&cgroup_dir, pid)?;
        println!("{}", stats);
        return Ok(());
    };

    // Stream events at the given interval
    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    loop {
        if !crate::cli::is_process_alive(pid) {
            break;
        }

        match read_cgroup_stats(&cgroup_dir, pid) {
            Ok(json) => {
                let _ = writeln!(stdout, "{}", json);
                let _ = stdout.flush();
            }
            Err(_) => {
                // Cgroup may have been removed; stop streaming
                break;
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(interval_ms));
    }

    Ok(())
}

fn parse_interval(s: &str) -> Option<u64> {
    if s.ends_with('s') {
        s.trim_end_matches('s')
            .parse::<u64>()
            .ok()
            .map(|v| v * 1000)
    } else if s.ends_with("ms") {
        s.trim_end_matches("ms").parse::<u64>().ok()
    } else {
        s.parse::<u64>().ok()
    }
}

fn read_cgroup_file(cgroup_dir: &std::path::Path, file: &str) -> Option<String> {
    fs::read_to_string(cgroup_dir.join(file))
        .ok()
        .map(|s| s.trim().to_string())
}

fn read_cgroup_stats(cgroup_dir: &std::path::Path, pid: u32) -> io::Result<String> {
    // Since we use edgerun-json, build a manual JSON object
    let mut obj = String::new();
    obj.push('{');

    // Container ID and PID
    obj.push_str(&format!("\"pid\":{},", pid));

    // Memory stats
    if let Some(mem_current) = read_cgroup_file(cgroup_dir, "memory.current") {
        let mut mem_obj = String::new();
        mem_obj.push_str("\"memory\":{");
        let mut first = true;
        for part in mem_current.split_whitespace() {
            if let Some((key, val)) = part.split_once('=') {
                if !first {
                    mem_obj.push(',');
                }
                mem_obj.push_str(&format!("\"{}\":{}", key, val));
                first = false;
            }
        }
        // Add memory.peak if available
        if let Some(peak) = read_cgroup_file(cgroup_dir, "memory.peak") {
            mem_obj.push_str(&format!(",\"peak\":{}", peak));
        }
        // Add memory.events if available
        if let Some(events) = read_cgroup_file(cgroup_dir, "memory.events") {
            mem_obj.push_str(",\"events\":{");
            let mut ef = true;
            for line in events.lines() {
                if let Some((key, val)) = line.split_once(' ') {
                    if !ef {
                        mem_obj.push(',');
                    }
                    mem_obj.push_str(&format!("\"{}\":{}", key, val));
                    ef = false;
                }
            }
            mem_obj.push('}');
        }
        mem_obj.push('}');
        obj.push_str(&mem_obj);
        obj.push(',');
    }

    // CPU stats
    if let Some(cpu_max) = read_cgroup_file(cgroup_dir, "cpu.max") {
        let mut cpu_obj = String::new();
        cpu_obj.push_str("\"cpu\":{");
        if let Some((quota, period)) = cpu_max.split_once(' ') {
            cpu_obj.push_str(&format!(
                "\"max_quota\":\"{}\",\"max_period\":\"{}\"",
                quota, period
            ));
        }
        // Add cpu.weight
        if let Some(weight) = read_cgroup_file(cgroup_dir, "cpu.weight") {
            if let Ok(w) = weight.trim().parse::<u64>() {
                cpu_obj.push_str(&format!(",\"weight\":{}", w));
            }
        }
        // Add cpu.stat
        if let Some(stat) = read_cgroup_file(cgroup_dir, "cpu.stat") {
            cpu_obj.push_str(",\"stat\":{");
            let mut sf = true;
            for line in stat.lines() {
                if let Some((key, val)) = line.split_once(' ') {
                    if !sf {
                        cpu_obj.push(',');
                    }
                    cpu_obj.push_str(&format!("\"{}\":\"{}\"", key, val));
                    sf = false;
                }
            }
            cpu_obj.push('}');
        }
        cpu_obj.push('}');
        obj.push_str(&cpu_obj);
        obj.push(',');
    }

    // PIDs stats
    if let Some(pids_current) = read_cgroup_file(cgroup_dir, "pids.current") {
        if let Ok(current) = pids_current.trim().parse::<u64>() {
            obj.push_str("\"pids\":{");
            obj.push_str(&format!("\"current\":{}", current));
            if let Some(pids_max) = read_cgroup_file(cgroup_dir, "pids.max") {
                let pids_max = pids_max.trim();
                if pids_max != "max" {
                    if let Ok(max) = pids_max.parse::<u64>() {
                        obj.push_str(&format!(",\"max\":{}", max));
                    } else {
                        obj.push_str(",\"max\":\"max\"");
                    }
                } else {
                    obj.push_str(",\"max\":\"max\"");
                }
            }
            obj.push_str("},");
        }
    }

    // IO stats
    if let Some(io_stat) = read_cgroup_file(cgroup_dir, "io.stat") {
        obj.push_str("\"io\":{");
        obj.push_str("\"stat\":\"");
        // Encode as string for now
        obj.push_str(&io_stat.replace('\\', "\\\\").replace('"', "\\\""));
        obj.push_str("\"},");
    }

    // Remove trailing comma
    if obj.ends_with(',') {
        obj.pop();
    }

    obj.push('}');

    Ok(obj)
}
