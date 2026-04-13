//! CLI argument parsing and command dispatch for ert.

mod create;
mod delete;
mod events;
mod exec;
mod features;
mod kill;
mod pause;
mod ps;
mod resume;
mod spec;
mod start;
mod state;
mod update;

mod pull;
mod push;
mod registry_login;
mod registry_logout;
mod run;

pub use create::cmd_create;
pub use delete::cmd_delete;
pub use events::cmd_events;
pub use exec::cmd_exec;
pub use features::cmd_features;
pub use kill::cmd_kill;
pub use pause::cmd_pause;
pub use ps::cmd_ps;
pub use resume::cmd_resume;
pub use spec::cmd_spec;
pub use start::cmd_start;
pub use state::cmd_state;
pub use update::cmd_update;

pub use pull::cmd_pull;
pub use push::cmd_push;
pub use registry_login::cmd_login;
pub use registry_logout::cmd_logout;
pub use run::cmd_run;

/// Global options parsed from the CLI.
#[derive(Debug, Default)]
pub struct GlobalOpts {
    pub bundle: Option<std::path::PathBuf>,
    pub pid_file: Option<std::path::PathBuf>,
    pub root: Option<std::path::PathBuf>,
}

/// Parse global options and identify the command from raw arguments.
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
                    cmd_idx = Some(i);
                    found_command = true;
                    i += 1;
                } else {
                    i += 1;
                }
            }
        }
    }

    let cmd_idx = cmd_idx?;
    let command = args[cmd_idx].clone();

    let mut command_args = Vec::new();
    for (j, arg) in args.iter().enumerate().skip(cmd_idx + 1) {
        match arg.as_str() {
            "--bundle" | "--pid-file" => {
                continue;
            }
            _ => {
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
    eprintln!("Container commands:");
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
    eprintln!("Registry commands:");
    eprintln!("  pull <image>              Pull an image from a registry");
    eprintln!("  push <image>              Push an image to a registry");
    eprintln!("  run <image> [cmd...]      Pull (if needed), create, start, and wait");
    eprintln!("  registry login <reg>      Login to a registry (biometric auth)");
    eprintln!("  registry logout <reg>     Logout from a registry");
    eprintln!();
    eprintln!("Registry options:");
    eprintln!("  --images-dir <path>       Image storage directory (default: /var/lib/edgerun/images)");
    eprintln!("  --store <path>            Layer blob cache (default: /var/lib/edgerun/store)");
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
pub fn parse_kill_args(args: &[String]) -> (Option<&str>, &str) {
    if args.is_empty() {
        return (None, "");
    }
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
