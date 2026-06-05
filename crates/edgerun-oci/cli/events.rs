//! Events command implementation — streams cgroup v2 stats as JSON.

use crate::prelude::*;
use std::fs;
use std::io::{self, Write};

use crate::clap::{Arg, Command};
use crate::cli::json;
use crate::cli::{invalid_input, parse_cli_args, required_positional};
use crate::state::load_state;

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

    let cgroup_dir = crate::cli::cgroup_dir_path(&cgroup_path)?;

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
    crate::cli::validate_container_id(&id)?;
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
    let mut root = String::new();
    let mut first_root = true;
    root.push('{');
    json::push_u64_field(&mut root, &mut first_root, "pid", u64::from(pid));

    // Memory stats
    if let Some(mem_current) = read_cgroup_file(cgroup_dir, "memory.current") {
        let mut memory = String::new();
        let mut first_memory = true;
        memory.push('{');
        if let Ok(current) = mem_current.trim().parse::<u64>() {
            json::push_u64_field(&mut memory, &mut first_memory, "current", current);
        }
        for part in mem_current.split_whitespace() {
            if let Some((key, val)) = part.split_once('=') {
                push_number_or_string_field(&mut memory, &mut first_memory, key, val);
            }
        }
        // Add memory.peak if available
        if let Some(peak) = read_cgroup_file(cgroup_dir, "memory.peak") {
            push_number_or_string_field(&mut memory, &mut first_memory, "peak", &peak);
        }
        // Add memory.events if available
        if let Some(events) = read_cgroup_file(cgroup_dir, "memory.events") {
            let mut event_fields = String::new();
            let mut first_event = true;
            event_fields.push('{');
            for line in events.lines() {
                if let Some((key, val)) = line.split_once(' ') {
                    push_number_or_string_field(&mut event_fields, &mut first_event, key, val);
                }
            }
            event_fields.push('}');
            json::push_field_prefix(&mut memory, &mut first_memory, "events");
            memory.push_str(&event_fields);
        }
        memory.push('}');
        json::push_field_prefix(&mut root, &mut first_root, "memory");
        root.push_str(&memory);
    }

    // CPU stats
    if let Some(cpu_max) = read_cgroup_file(cgroup_dir, "cpu.max") {
        let mut cpu = String::new();
        let mut first_cpu = true;
        cpu.push('{');
        if let Some((quota, period)) = cpu_max.split_once(' ') {
            json::push_string_field(&mut cpu, &mut first_cpu, "max_quota", quota);
            json::push_string_field(&mut cpu, &mut first_cpu, "max_period", period);
        }
        // Add cpu.weight
        if let Some(weight) = read_cgroup_file(cgroup_dir, "cpu.weight") {
            if let Ok(w) = weight.trim().parse::<u64>() {
                json::push_u64_field(&mut cpu, &mut first_cpu, "weight", w);
            }
        }
        // Add cpu.stat
        if let Some(stat) = read_cgroup_file(cgroup_dir, "cpu.stat") {
            let mut stat_fields = String::new();
            let mut first_stat = true;
            stat_fields.push('{');
            for line in stat.lines() {
                if let Some((key, val)) = line.split_once(' ') {
                    json::push_string_field(&mut stat_fields, &mut first_stat, key, val);
                }
            }
            stat_fields.push('}');
            json::push_field_prefix(&mut cpu, &mut first_cpu, "stat");
            cpu.push_str(&stat_fields);
        }
        cpu.push('}');
        json::push_field_prefix(&mut root, &mut first_root, "cpu");
        root.push_str(&cpu);
    }

    // PIDs stats
    if let Some(pids_current) = read_cgroup_file(cgroup_dir, "pids.current") {
        if let Ok(current) = pids_current.trim().parse::<u64>() {
            let mut pids = String::new();
            let mut first_pids = true;
            pids.push('{');
            json::push_u64_field(&mut pids, &mut first_pids, "current", current);
            if let Some(pids_max) = read_cgroup_file(cgroup_dir, "pids.max") {
                let pids_max = pids_max.trim();
                if pids_max != "max" {
                    if let Ok(max) = pids_max.parse::<u64>() {
                        json::push_u64_field(&mut pids, &mut first_pids, "max", max);
                    } else {
                        json::push_string_field(&mut pids, &mut first_pids, "max", "max");
                    }
                } else {
                    json::push_string_field(&mut pids, &mut first_pids, "max", "max");
                }
            }
            pids.push('}');
            json::push_field_prefix(&mut root, &mut first_root, "pids");
            root.push_str(&pids);
        }
    }

    // IO stats
    if let Some(io_stat) = read_cgroup_file(cgroup_dir, "io.stat") {
        let mut io = String::new();
        let mut first_io = true;
        io.push('{');
        json::push_string_field(&mut io, &mut first_io, "stat", &io_stat);
        io.push('}');
        json::push_field_prefix(&mut root, &mut first_root, "io");
        root.push_str(&io);
    }

    root.push('}');
    Ok(root)
}

fn push_number_or_string_field(out: &mut String, first: &mut bool, name: &str, value: &str) {
    json::push_field_prefix(out, first, name);
    match value.trim().parse::<u64>() {
        Ok(number) => out.push_str(&number.to_string()),
        Err(_) => json::write_string(out, value),
    }
}
