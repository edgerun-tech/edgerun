//! CLI argument parsing and command dispatch for ert.

use crate::prelude::*;
mod checkpoint;
mod create;
mod delete;
mod events;
mod exec;
mod features;
mod images;
mod inspect;
mod kill;
mod logs;
mod pause;
mod ps;
mod restore;
mod resume;
mod spec;
mod start;
mod state;
mod stop;
mod update;
mod user;

mod pull;
mod push;
mod registry_login;
mod registry_logout;
mod rmi;
mod run;

pub use checkpoint::cmd_checkpoint;
pub use create::cmd_create;
pub use delete::cmd_delete;
pub use events::cmd_events;
pub use exec::cmd_exec;
pub use features::cmd_features;
pub use images::cmd_images;
pub use inspect::cmd_inspect;
pub use kill::cmd_kill;
pub use logs::cmd_logs;
pub use pause::cmd_pause;
pub use ps::cmd_ps;
pub use restore::cmd_restore;
pub use resume::cmd_resume;
pub use spec::cmd_spec;
pub use start::cmd_start;
pub use state::cmd_state;
pub use stop::cmd_stop;
pub use update::cmd_update;

pub use pull::cmd_pull;
pub use push::cmd_push;
pub use registry_login::cmd_login;
pub use registry_logout::cmd_logout;
pub use rmi::cmd_rmi;
pub use run::cmd_run;

#[cfg(all(feature = "std", not(target_os = "none")))]
pub fn default_images_dir() -> std::path::PathBuf {
    edgerun_data_dir().join("images")
}

#[cfg(all(feature = "std", not(target_os = "none")))]
pub fn default_store_dir() -> std::path::PathBuf {
    edgerun_data_dir().join("store")
}

#[cfg(all(feature = "std", not(target_os = "none")))]
fn edgerun_data_dir() -> std::path::PathBuf {
    if unsafe { libc::geteuid() } == 0 && std::env::var_os("_ERT_ROOTLESS_CHILD").is_none() {
        return std::path::PathBuf::from("/var/lib/edgerun");
    }

    if let Some(path) = std::env::var_os("EDGERUN_DATA_DIR") {
        return std::path::PathBuf::from(path);
    }
    if let Some(path) = std::env::var_os("XDG_DATA_HOME") {
        return std::path::PathBuf::from(path).join("edgerun");
    }
    if let Some(home) = std::env::var_os("HOME") {
        return std::path::PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("edgerun");
    }

    std::env::temp_dir().join("edgerun")
}

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
                    if prev == "--bundle"
                        || prev == "--pid-file"
                        || prev.starts_with("--bundle=")
                        || prev.starts_with("--pid-file=")
                    {
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
    eprintln!("  stop <container-id>       Stop a running container");
    eprintln!("  state <container-id>      Output state of a container");
    eprintln!("  inspect <container-id>    Output state and config details");
    eprintln!("  kill <container-id>       Send signal to container");
    eprintln!("  logs <container-id>       Print container stdout/stderr logs");
    eprintln!("  delete <container-id>     Delete container resources");
    eprintln!("  rm <container-id>         Alias for delete");
    eprintln!("  exec <container-id>       Run additional process in container");
    eprintln!("  update <container-id>     Update container resource limits");
    eprintln!("  pause <container-id>      Pause the container (cgroup freeze)");
    eprintln!("  resume <container-id>     Resume the container (cgroup unfreeze)");
    eprintln!("  checkpoint <container-id>  Checkpoint a running container");
    eprintln!("  restore <container-id>     Restore a checkpointed container");
    eprintln!("  events <container-id>     Stream cgroup stats");
    eprintln!("  ps <container-id>         List processes in the container");
    eprintln!("  features                  Output supported features");
    eprintln!("  spec                      Generate a default config.json");
    eprintln!();
    eprintln!("Registry commands:");
    eprintln!("  images                    List local images");
    eprintln!("  rmi <image>               Remove a local image");
    eprintln!("  pull <image>              Pull an image from a registry");
    eprintln!("  push <image>              Push an image to a registry");
    eprintln!("  run <image> [cmd...]      Pull (if needed), create, start, and wait");
    eprintln!("  registry login <reg>      Login to a registry (biometric auth)");
    eprintln!("  registry logout <reg>     Logout from a registry");
    eprintln!();
    eprintln!("Run options:");
    eprintln!("  --rm                      Remove container state after exit");
    eprintln!("  -d, --detach              Run in background");
    eprintln!("  --name <name>             Assign container ID");
    eprintln!("  -e, --env KEY=VALUE       Add environment variable");
    eprintln!("  --env-file <path>         Read environment variables from file");
    eprintln!("  -u, --user USER[:GROUP]   Run as numeric ID or image user/group name");
    eprintln!("  -w, --workdir <path>      Set working directory");
    eprintln!("  -v host:ctr[:ro|rw]       Bind mount a host path");
    eprintln!("  --pull <policy>           Image pull policy: missing, always, or never");
    eprintln!("  --hostname <name>         Set container hostname");
    eprintln!("  --dns <addr>              Write resolver nameserver");
    eprintln!("  --add-host host:ip        Add /etc/hosts entry");
    eprintln!();
    eprintln!("Registry options:");
    eprintln!("  --images-dir <path>       Image storage directory");
    eprintln!("                            (default: /var/lib/edgerun/images as root,");
    eprintln!("                             $XDG_DATA_HOME/edgerun/images rootless)");
    eprintln!("  --store <path>            Layer blob cache");
    eprintln!("                            (default: /var/lib/edgerun/store as root,");
    eprintln!("                             $XDG_DATA_HOME/edgerun/store rootless)");
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

/// Check if a process is alive and not already a zombie.
pub fn is_process_alive(pid: u32) -> bool {
    if unsafe { libc::kill(pid as std::os::raw::c_int, 0) != 0 } {
        return false;
    }
    let status_path = format!("/proc/{pid}/status");
    if let Ok(status) = std::fs::read_to_string(status_path) {
        for line in status.lines() {
            if let Some(rest) = line.strip_prefix("State:") {
                return !rest.trim_start().starts_with('Z');
            }
        }
    }
    true
}

/// Extract container ID from command args, returning an error if missing.
pub fn require_container_id(args: &[String]) -> std::io::Result<&str> {
    args.first().map(|s| s.as_str()).ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "container ID required")
    })
}

/// Resolve registry authentication via secret service.
/// Returns anonymous auth if secret service unavailable or credential not found.
pub fn resolve_registry_auth(registry: &str) -> std::io::Result<crate::RegistryAuth> {
    let mut secret_client = match crate::SecretClient::connect() {
        Ok(c) => c,
        Err(_) => return Ok(crate::RegistryAuth::Anonymous),
    };

    if secret_client.open_session().is_err() {
        return Ok(crate::RegistryAuth::Anonymous);
    }

    match secret_client.get_registry_credential(registry) {
        Ok(secret_bytes) => {
            let secret_str = String::from_utf8(secret_bytes).map_err(|_| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "invalid credential encoding",
                )
            })?;
            let (username, password) = secret_str.split_once(':').ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "credential must be username:password",
                )
            })?;
            Ok(crate::RegistryAuth::Basic {
                username: username.to_string(),
                password: password.to_string(),
            })
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(crate::RegistryAuth::Anonymous),
        Err(_) => Ok(crate::RegistryAuth::Anonymous),
    }
}
