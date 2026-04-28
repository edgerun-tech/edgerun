//! Events command implementation — streams cgroup v2 stats as JSON.

use crate::prelude::*;
use std::fs;
use std::io::{self, Write};

use crate::cli::{invalid_input, parse_cli_args, required_positional};
use crate::state::load_state;
use edgerun_clap::{Arg, Command};
use edgerun_json::{JsonValue, Map};

pub fn cmd_events(opts: &crate::cli::GlobalOpts, args: &[String]) -> io::Result<()> {
    crate::cli::apply_global_opts(opts)?;

    let parsed = parse_events_args(args)?;
    let id = parsed.id.as_str();

    let state = load_state(id)?;
    let pid = state
        .pid
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "container has no PID"))?;

    let bundle = &state.bundle;
    let config_path = std::path::Path::new(bundle).join("config.json");
    let cgroup_path = if let Ok(data) = fs::read(&config_path) {
        if let Ok(spec) = crate::spec::parse_oci_spec(&data) {
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

    let Some(interval_ms) = parsed.interval_ms else {
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

#[derive(Debug)]
struct EventsArgs {
    id: String,
    interval_ms: Option<u64>,
}

fn parse_events_args(args: &[String]) -> io::Result<EventsArgs> {
    const USAGE: &str = "Usage: ert events [--interval N] <container-id>";
    let matches = parse_cli_args(
        Command::new("events").arg(Arg::new("interval").long("interval")),
        args,
        USAGE,
    )?;
    if matches.positional_count() > 1 {
        return Err(invalid_input(USAGE));
    }
    let id = required_positional(&matches, 0, "container ID required")?.to_string();
    let interval_ms = matches
        .get_one::<String>("interval")
        .map(|interval| {
            parse_interval(&interval)
                .ok_or_else(|| invalid_input(format!("invalid interval: {interval}")))
        })
        .transpose()?;
    Ok(EventsArgs { id, interval_ms })
}

fn parse_interval(s: &str) -> Option<u64> {
    if s.ends_with("ms") {
        s.trim_end_matches("ms").parse::<u64>().ok()
    } else if s.ends_with('s') {
        s.trim_end_matches('s')
            .parse::<u64>()
            .ok()
            .map(|v| v * 1000)
    } else {
        s.parse::<u64>().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn parses_events_args() {
        let parsed = parse_events_args(&args(&["--interval", "250ms", "container-a"])).unwrap();

        assert_eq!(parsed.id, "container-a");
        assert_eq!(parsed.interval_ms, Some(250));
        assert!(parse_events_args(&args(&["container-a", "extra"])).is_err());
        assert!(parse_events_args(&args(&["--interval", "bad", "container-a"])).is_err());
    }
}

fn read_cgroup_file(cgroup_dir: &std::path::Path, file: &str) -> Option<String> {
    fs::read_to_string(cgroup_dir.join(file))
        .ok()
        .map(|s| s.trim().to_string())
}

fn read_cgroup_stats(cgroup_dir: &std::path::Path, pid: u32) -> io::Result<String> {
    let mut root = Map::new();
    root.push_field("pid", pid);

    // Memory stats
    if let Some(mem_current) = read_cgroup_file(cgroup_dir, "memory.current") {
        let mut memory = Map::new();
        if let Ok(current) = mem_current.trim().parse::<u64>() {
            memory.push_field("current", current);
        }
        for part in mem_current.split_whitespace() {
            if let Some((key, val)) = part.split_once('=') {
                memory.push_field(key, number_or_string(val));
            }
        }
        // Add memory.peak if available
        if let Some(peak) = read_cgroup_file(cgroup_dir, "memory.peak") {
            memory.push_field("peak", number_or_string(&peak));
        }
        // Add memory.events if available
        if let Some(events) = read_cgroup_file(cgroup_dir, "memory.events") {
            let mut event_fields = Map::new();
            for line in events.lines() {
                if let Some((key, val)) = line.split_once(' ') {
                    event_fields.push_field(key, number_or_string(val));
                }
            }
            memory.push_field("events", JsonValue::Object(event_fields));
        }
        root.push_field("memory", JsonValue::Object(memory));
    }

    // CPU stats
    if let Some(cpu_max) = read_cgroup_file(cgroup_dir, "cpu.max") {
        let mut cpu = Map::new();
        if let Some((quota, period)) = cpu_max.split_once(' ') {
            cpu.push_field("max_quota", quota);
            cpu.push_field("max_period", period);
        }
        // Add cpu.weight
        if let Some(weight) = read_cgroup_file(cgroup_dir, "cpu.weight") {
            if let Ok(w) = weight.trim().parse::<u64>() {
                cpu.push_field("weight", w);
            }
        }
        // Add cpu.stat
        if let Some(stat) = read_cgroup_file(cgroup_dir, "cpu.stat") {
            let mut stat_fields = Map::new();
            for line in stat.lines() {
                if let Some((key, val)) = line.split_once(' ') {
                    stat_fields.push_field(key, val);
                }
            }
            cpu.push_field("stat", JsonValue::Object(stat_fields));
        }
        root.push_field("cpu", JsonValue::Object(cpu));
    }

    // PIDs stats
    if let Some(pids_current) = read_cgroup_file(cgroup_dir, "pids.current") {
        if let Ok(current) = pids_current.trim().parse::<u64>() {
            let mut pids = Map::new();
            pids.push_field("current", current);
            if let Some(pids_max) = read_cgroup_file(cgroup_dir, "pids.max") {
                let pids_max = pids_max.trim();
                if pids_max != "max" {
                    if let Ok(max) = pids_max.parse::<u64>() {
                        pids.push_field("max", max);
                    } else {
                        pids.push_field("max", "max");
                    }
                } else {
                    pids.push_field("max", "max");
                }
            }
            root.push_field("pids", JsonValue::Object(pids));
        }
    }

    // IO stats
    if let Some(io_stat) = read_cgroup_file(cgroup_dir, "io.stat") {
        let mut io = Map::new();
        io.push_field("stat", io_stat);
        root.push_field("io", JsonValue::Object(io));
    }

    edgerun_json::to_string(&JsonValue::Object(root))
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))
}

fn number_or_string(value: &str) -> JsonValue {
    value
        .trim()
        .parse::<u64>()
        .map(JsonValue::from)
        .unwrap_or_else(|_| JsonValue::from(value))
}
