//! CLI argument parsing and command dispatch for edgerun-oci.

mod create;
mod start;
mod state;
mod kill;
mod delete;
mod exec;
mod update;
mod pause;
mod resume;
mod features;
mod spec;
mod ps;
mod events;

pub use create::cmd_create;
pub use start::cmd_start;
pub use state::cmd_state;
pub use kill::cmd_kill;
pub use delete::cmd_delete;
pub use exec::cmd_exec;
pub use update::cmd_update;
pub use pause::cmd_pause;
pub use resume::cmd_resume;
pub use features::cmd_features;
pub use spec::cmd_spec;
pub use ps::cmd_ps;
pub use events::cmd_events;

/// Global options parsed from the CLI.
#[derive(Debug, Default)]
pub struct GlobalOpts {
    pub bundle: Option<std::path::PathBuf>,
    pub pid_file: Option<std::path::PathBuf>,
    pub root: Option<std::path::PathBuf>,
}

/// Parse global options and identify the command from raw arguments.
///
/// The CLI format from the Go test harness is:
/// `edgerun-oci <command> [--bundle <path>] [--pid-file <path>] <container-id>`
///
/// Returns `(global_opts, command_name, command_args)`.
pub fn parse_args(args: &[String]) -> Option<(GlobalOpts, String, Vec<String>)> {
    if args.is_empty() {
        return None;
    }

    let mut opts = GlobalOpts::default();
    let mut cmd_idx = None;
    let mut i = 0;
    let mut found_command = false;

    while i < args.len() {
        match args[i].as_str() {
            "--bundle" => {
                if i + 1 < args.len() {
                    opts.bundle = Some(std::path::PathBuf::from(&args[i + 1]));
                    i += 2;
                    continue;
                } else {
                    i += 1;
                }
            }
            s if s.starts_with("--bundle=") => {
                opts.bundle = Some(std::path::PathBuf::from(&s["--bundle=".len()..]));
                i += 1;
            }
            "--pid-file" => {
                if i + 1 < args.len() {
                    opts.pid_file = Some(std::path::PathBuf::from(&args[i + 1]));
                    i += 2;
                    continue;
                } else {
                    i += 1;
                }
            }
            s if s.starts_with("--pid-file=") => {
                opts.pid_file = Some(std::path::PathBuf::from(&s["--pid-file=".len()..]));
                i += 1;
            }
            "--root" => {
                if i + 1 < args.len() {
                    opts.root = Some(std::path::PathBuf::from(&args[i + 1]));
                    i += 2;
                    continue;
                } else {
                    i += 1;
                }
            }
            s if s.starts_with("--root=") => {
                opts.root = Some(std::path::PathBuf::from(&s["--root=".len()..]));
                i += 1;
            }
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            _ => {
                if !args[i].starts_with('-') && !found_command {
                    // First non-option argument is the command
                    cmd_idx = Some(i);
                    found_command = true;
                    i += 1;
                } else {
                    // Everything after command is command args
                    i += 1;
                }
            }
        }
    }

    let cmd_idx = cmd_idx?;
    let command = args[cmd_idx].clone();

    // Command args are everything after the command (excluding global options we already parsed)
    let mut command_args = Vec::new();
    for (j, arg) in args.iter().enumerate().skip(cmd_idx + 1) {
        // Skip options we already parsed
        match arg.as_str() {
            "--bundle" | "--pid-file" => {
                // Skip this and the next value
                continue;
            }
            _ => {
                // Check if previous was --bundle or --pid-file
                if j > 0 {
                    let prev = &args[j - 1];
                    if prev == "--bundle" || prev == "--pid-file" || prev.starts_with("--bundle=") || prev.starts_with("--pid-file=") {
                        continue;
                    }
                }
                command_args.push(arg.clone());
            }
        }
    }

    Some((opts, command, command_args))
}

pub fn print_usage() {
    eprintln!("Usage: ert [global-options] <command> [command-options]");
    eprintln!();
    eprintln!("Commands:");
    eprintln!("  create <container-id>     Create a container");
    eprintln!("  start <container-id>      Start a created container");
    eprintln!("  state <container-id>      Output state of a container");
    eprintln!("  kill <container-id>       Send signal to container");
    eprintln!("  delete <container-id>     Delete container resources");
    eprintln!("  exec <container-id>       Run additional process in container");
    eprintln!("  update <container-id>     Update container resource limits");
    eprintln!("  pause <container-id>      Pause the container (cgroup freeze)");
    eprintln!("  resume <container-id>     Resume the container (cgroup unfreeze)");
    eprintln!("  events <container-id>     Stream cgroup stats");
    eprintln!("  ps <container-id>         List processes in the container");
    eprintln!("  features                  Output supported features");
    eprintln!("  spec                      Generate a default config.json");
    eprintln!();
    eprintln!("Global options:");
    eprintln!("  --bundle <path>           Path to bundle directory");
    eprintln!("  --pid-file <path>         Path to write container PID");
    eprintln!("  --root <path>             Root directory for state files");
    eprintln!("                            (default: /run/edgerun-oci as root,");
    eprintln!("                             $XDG_RUNTIME_DIR/edgerun-oci rootless)");
}

/// Extract the first positional argument (container ID) from command args.
pub fn get_container_id(args: &[String]) -> Option<&str> {
    args.first().map(|s| s.as_str())
}

/// Extract a signal string from command args (for kill command).
/// Returns (signal_string, remaining_args).
pub fn parse_kill_args(args: &[String]) -> (Option<&str>, &str) {
    // Format: [signal] <container-id>  OR  <container-id> [signal]
    // We look for the container ID (non-signal argument)
    if args.is_empty() {
        return (None, "");
    }

    // Try first arg as container ID, second as signal
    let container_id = &args[0];
    let signal = args.get(1).map(|s| s.as_str());
    (signal, container_id.as_str())
}

/// Extract delete command options (--force flag and container ID).
pub fn parse_delete_args(args: &[String]) -> (bool, Option<&str>) {
    let mut force = false;
    let mut id = None;

    for arg in args {
        match arg.as_str() {
            "--force" => force = true,
            _ => {
                if id.is_none() {
                    id = Some(arg.as_str());
                }
            }
        }
    }

    (force, id)
}

/// Check if a process is alive by sending signal 0.
pub fn is_process_alive(pid: u32) -> bool {
    unsafe { libc::kill(pid as std::os::raw::c_int, 0) == 0 }
}

/// Extract container ID from command args, returning an error if missing.
pub fn require_container_id(args: &[String]) -> std::io::Result<&str> {
    args.first()
        .map(|s| s.as_str())
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "container ID required"))
}
