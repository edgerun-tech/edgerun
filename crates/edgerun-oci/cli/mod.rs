//! CLI argument parsing and command dispatch for ert.

use crate::libc;
use crate::prelude::*;
use std::io;
use std::path::PathBuf;
mod checkpoint;
mod create;
mod delete;
mod display;
mod env;
mod events;
mod exec;
mod features;
mod images;
mod inspect;
mod json;
mod kill;
mod list;
mod logs;
mod pause;
mod process_tree;
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
mod run_config;

pub use checkpoint::cmd_checkpoint;
pub use create::cmd_create;
pub use delete::cmd_delete;
pub use events::cmd_events;
pub use exec::cmd_exec;
pub use features::cmd_features;
pub use images::cmd_images;
pub use inspect::cmd_inspect;
pub use kill::cmd_kill;
pub use list::cmd_list;
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

type CommandHandler = fn(&GlobalOpts, &[String]) -> io::Result<()>;

struct CommandSpec {
    name: &'static str,
    handler: CommandHandler,
    container: bool,
}

macro_rules! command_spec {
    ($name:literal, $handler:ident, $container:literal) => {
        CommandSpec {
            name: $name,
            handler: $handler,
            container: $container,
        }
    };
}

const COMMANDS: &[CommandSpec] = &[
    command_spec!("create", cmd_create, true),
    command_spec!("start", cmd_start, true),
    command_spec!("stop", cmd_stop, true),
    command_spec!("state", cmd_state, true),
    command_spec!("inspect", cmd_inspect, true),
    command_spec!("kill", cmd_kill, true),
    command_spec!("list", cmd_list, false),
    command_spec!("logs", cmd_logs, true),
    command_spec!("delete", cmd_delete, true),
    command_spec!("rm", cmd_delete, true),
    command_spec!("exec", cmd_exec, true),
    command_spec!("update", cmd_update, true),
    command_spec!("pause", cmd_pause, true),
    command_spec!("resume", cmd_resume, true),
    command_spec!("checkpoint", cmd_checkpoint, true),
    command_spec!("restore", cmd_restore, true),
    command_spec!("events", cmd_events, true),
    command_spec!("ps", cmd_ps, true),
    command_spec!("features", cmd_features, false),
    command_spec!("spec", cmd_spec, false),
    command_spec!("pull", cmd_pull, false),
    command_spec!("push", cmd_push, false),
    command_spec!("images", cmd_images, false),
    command_spec!("rmi", cmd_rmi, false),
    command_spec!("run", cmd_run, true),
    command_spec!("registry", dispatch_registry_command, false),
];

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

pub fn apply_global_opts(opts: &GlobalOpts) -> io::Result<()> {
    if let Some(ref root) = opts.root {
        crate::state::set_state_dir(root.to_str().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "--root path is not valid UTF-8",
            )
        })?);
    }
    Ok(())
}

pub(crate) fn write_all_fd(fd: i32, bytes: &[u8]) {
    let mut written = 0usize;
    while written < bytes.len() {
        let n = unsafe {
            libc::write(
                fd,
                bytes[written..].as_ptr() as *const libc::c_void,
                bytes.len() - written,
            )
        };
        if n <= 0 {
            break;
        }
        written += n as usize;
    }
}

pub(crate) fn load_bundle_spec(bundle: &str) -> io::Result<crate::spec::OciSpec> {
    let data = std::fs::read(std::path::Path::new(bundle).join("config.json"))?;
    crate::spec::parse_oci_spec(&data).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

pub(crate) fn load_runtime_spec(id: &str) -> io::Result<crate::spec::OciSpec> {
    let data = std::fs::read(crate::state::runtime_spec_path(id))?;
    crate::spec::parse_oci_spec(&data).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

pub(crate) fn load_runtime_or_bundle_spec(id: &str, bundle: &str) -> Option<crate::spec::OciSpec> {
    load_runtime_spec(id)
        .ok()
        .or_else(|| load_bundle_spec(bundle).ok())
}

pub(crate) fn invalid_input(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message.into())
}

pub(crate) fn parse_cli_args(
    command: crate::clap::Command,
    args: &[String],
    usage: &str,
) -> io::Result<crate::clap::ArgMatches> {
    let matches = command.get_matches_from_iter(args.iter().map(String::as_str));
    if let Some(arg) = matches.unknown_args().first() {
        return Err(invalid_input(format!("{usage}: unknown option {arg}")));
    }
    if let Some(arg) = matches.missing_value_args().first() {
        return Err(invalid_input(format!("{usage}: {arg} requires a value")));
    }
    Ok(matches)
}

pub const RUN_VALUE_OPTIONS: &[&str] = &[
    "--console-socket",
    "--pid-file",
    "-v",
    "--volume",
    "--mount",
    "--name",
    "-e",
    "--env",
    "--env-file",
    "-w",
    "--workdir",
    "-u",
    "--user",
    "--entrypoint",
    "--pull",
    "-h",
    "--hostname",
    "--dns",
    "--add-host",
    "--images-dir",
    "--store",
];

pub(crate) const EXEC_VALUE_OPTIONS: &[&str] = &[
    "--cwd",
    "--env",
    "-e",
    "--user",
    "-u",
    "--process",
    "-p",
    "--process-json",
    "--pid-file",
    "--console-socket",
];

pub(crate) fn required_positional<'a>(
    matches: &'a crate::clap::ArgMatches,
    index: usize,
    usage: &str,
) -> io::Result<&'a str> {
    matches
        .get_positional(index)
        .ok_or_else(|| invalid_input(usage))
}

pub fn split_cli_prefix(
    args: &[String],
    value_options: &[&str],
) -> (Vec<String>, Option<String>, Vec<String>) {
    let mut prefix = Vec::new();
    let mut i = 0usize;
    while i < args.len() {
        let arg = args[i].as_str();
        if arg == "--" {
            return (prefix, None, args[i + 1..].to_vec());
        }
        if !arg.starts_with('-') || arg == "-" {
            return (prefix, Some(args[i].clone()), args[i + 1..].to_vec());
        }
        prefix.push(args[i].clone());
        if option_takes_value(arg, value_options) {
            if let Some(value) = args.get(i + 1) {
                if !value.starts_with('-') {
                    prefix.push(value.clone());
                    i += 1;
                }
            }
        }
        i += 1;
    }
    (prefix, None, Vec::new())
}

fn option_takes_value(arg: &str, value_options: &[&str]) -> bool {
    let name = arg.split_once('=').map_or(arg, |(name, _)| name);
    value_options.iter().any(|option| *option == name) && !arg.contains('=')
}

fn command_spec(command: &str) -> Option<&'static CommandSpec> {
    COMMANDS.iter().find(|candidate| candidate.name == command)
}

pub fn dispatch_command(opts: &GlobalOpts, command: &str, args: &[String]) -> io::Result<()> {
    let spec = command_spec(command).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("unknown command: {command}"),
        )
    })?;
    (spec.handler)(opts, args)
}

pub fn is_container_command(command: &str) -> bool {
    command_spec(command).is_some_and(|spec| spec.container)
}

pub fn first_command(args: &[String]) -> Option<&str> {
    let mut i = 0usize;
    while i < args.len() {
        let arg = args[i].as_str();
        if let Some(consumed) = global_option_consumed(arg) {
            i = i.saturating_add(consumed);
            continue;
        }
        if arg.starts_with('-') {
            return None;
        }
        return Some(arg);
    }
    None
}

fn dispatch_registry_command(opts: &GlobalOpts, cmd_args: &[String]) -> io::Result<()> {
    let Some((subcommand, sub_args)) = cmd_args.split_first() else {
        eprintln!("Usage: ert registry <subcommand> [options]");
        eprintln!("Subcommands: login, logout");
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "registry subcommand required",
        ));
    };

    match subcommand.as_str() {
        "login" => cmd_login(opts, sub_args),
        "logout" => cmd_logout(opts, sub_args),
        _ => {
            eprintln!("Unknown registry subcommand: {}", subcommand);
            eprintln!("Available: login, logout");
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("unknown registry subcommand: {}", subcommand),
            ))
        }
    }
}

/// Parse global options and identify the command from raw arguments.
pub fn parse_args(args: &[String]) -> Option<(GlobalOpts, String, Vec<String>)> {
    if args.is_empty() {
        return None;
    }

    let mut opts = GlobalOpts::default();
    let mut command = None;
    let mut command_args = Vec::new();
    let mut i = 0usize;

    while i < args.len() {
        let arg = args[i].as_str();
        if let Some(consumed) = parse_global_option_at(args, i, &mut opts) {
            match consumed {
                Ok(consumed) => {
                    i = i.saturating_add(consumed);
                    continue;
                }
                Err(error) => {
                    eprintln!("Invalid argument: {error}");
                    return None;
                }
            }
        }
        if matches!(arg, "--help" | "-h") {
            print_usage();
            std::process::exit(0);
        }
        if command.is_none() {
            if arg.starts_with('-') {
                eprintln!("Invalid argument: unknown global option {arg}");
                return None;
            }
            if !arg.starts_with('-') {
                command = Some(args[i].clone());
            }
        } else {
            command_args.push(args[i].clone());
        }
        i += 1;
    }

    Some((opts, command?, command_args))
}

fn global_option_consumed(arg: &str) -> Option<usize> {
    match arg {
        "--bundle" | "-b" | "--pid-file" | "--root" => Some(2),
        _ if global_option_inline_kind(arg).is_some() => Some(1),
        _ => None,
    }
}

fn parse_global_option_at(
    args: &[String],
    index: usize,
    opts: &mut GlobalOpts,
) -> Option<Result<usize, String>> {
    let arg = args.get(index)?;
    if let Some(kind) = GlobalOption::from_name(arg) {
        if let Some(value) = args.get(index + 1) {
            kind.apply(opts, value);
            return Some(Ok(2));
        }
        return Some(Err(format!("missing value for {arg}")));
    }
    if let Some((kind, value)) = global_option_inline_kind(arg) {
        kind.apply(opts, value);
        return Some(Ok(1));
    }
    None
}

fn global_option_inline_kind(arg: &str) -> Option<(GlobalOption, &str)> {
    if let Some(value) = arg.strip_prefix("--bundle=") {
        return Some((GlobalOption::Bundle, value));
    }
    if let Some(value) = arg.strip_prefix("--pid-file=") {
        return Some((GlobalOption::PidFile, value));
    }
    if let Some(value) = arg.strip_prefix("--root=") {
        return Some((GlobalOption::Root, value));
    }
    None
}

#[derive(Clone, Copy)]
enum GlobalOption {
    Bundle,
    PidFile,
    Root,
}

impl GlobalOption {
    fn from_name(name: &str) -> Option<Self> {
        match name {
            "--bundle" => Some(Self::Bundle),
            "-b" => Some(Self::Bundle),
            "--pid-file" => Some(Self::PidFile),
            "--root" => Some(Self::Root),
            _ => None,
        }
    }

    fn apply(self, opts: &mut GlobalOpts, value: impl AsRef<std::path::Path>) {
        match self {
            Self::Bundle => opts.bundle = Some(value.as_ref().to_path_buf()),
            Self::PidFile => opts.pid_file = Some(value.as_ref().to_path_buf()),
            Self::Root => opts.root = Some(value.as_ref().to_path_buf()),
        }
    }
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
    eprintln!("  list [-q|--format json]   List containers");
    eprintln!("  logs [--tail N] <id>      Print container stdout/stderr logs");
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
    eprintln!("  --bundle, -b <path>       Path to bundle directory");
    eprintln!("  --pid-file <path>         Path to write container PID");
    eprintln!("  --root <path>             Root directory for state files");
    eprintln!("                            (default: /run/edgerun-oci as root,");
    eprintln!("                             $XDG_RUNTIME_DIR/edgerun-oci rootless)");
}

/// Extract a signal string from command args (for kill command).
pub fn parse_kill_args(args: &[String]) -> io::Result<(Option<String>, String)> {
    const USAGE: &str = "Usage: ert kill <container-id> [signal]";
    let matches = parse_cli_args(crate::clap::Command::new("kill"), args, USAGE)?;
    if matches.positional_count() > 2 {
        return Err(invalid_input(USAGE));
    }
    let container_id = required_positional(&matches, 0, "container ID required")?.to_string();
    validate_container_id(&container_id)?;
    let signal = matches.get_positional(1).map(str::to_string);
    Ok((signal, container_id))
}

/// Extract delete command options (--force flag and container ID).
pub fn parse_delete_args(args: &[String]) -> io::Result<(bool, Option<String>)> {
    const USAGE: &str = "Usage: ert delete [--force] <container-id>";
    let matches = parse_cli_args(
        crate::clap::Command::new("delete").arg(
            crate::clap::Arg::new("force")
                .short('f')
                .long("force")
                .action(crate::clap::cli::Action::StoreTrue),
        ),
        args,
        USAGE,
    )?;
    if matches.positional_count() > 1 {
        return Err(invalid_input(USAGE));
    }
    Ok((
        matches.get_flag("force"),
        matches
            .get_positional(0)
            .map(|id| {
                validate_container_id(id)?;
                Ok::<String, io::Error>(id.to_string())
            })
            .transpose()?,
    ))
}

/// Check if a process is alive and not already a zombie.
pub fn is_process_alive(pid: u32) -> bool {
    process_tree::process_alive(pid)
}

/// Extract container ID from command args, returning an error if missing.
pub fn parse_container_id_args(args: &[String], command: &'static str) -> io::Result<String> {
    let usage = format!("Usage: ert {command} <container-id>");
    let matches = parse_cli_args(crate::clap::Command::new(command), args, &usage)?;
    if matches.positional_count() > 1 {
        return Err(invalid_input(usage.clone()));
    }
    let id = required_positional(&matches, 0, "container ID required")?.to_string();
    validate_container_id(&id)?;
    Ok(id)
}

pub(crate) fn cgroup_dir_path(cgroup_path: &str) -> io::Result<PathBuf> {
    let normalized = if cgroup_path.is_empty() {
        "/edgerun".to_string()
    } else {
        strip_sysfs_prefix(cgroup_path)
    };
    let resolved = crate::rootless::resolve_container_cgroup_path(
        crate::state::is_rootless_mode(),
        &normalized,
    )?;
    Ok(std::path::Path::new("/sys/fs/cgroup").join(resolved.trim_start_matches('/')))
}

fn strip_sysfs_prefix(cgroup_path: &str) -> String {
    let mut normalized = cgroup_path.trim();
    if normalized.is_empty() {
        return "/edgerun".to_string();
    }
    normalized = normalized.trim_start_matches('/');
    if let Some(trimmed) = normalized.strip_prefix("sys/fs/cgroup") {
        normalized = trimmed.trim_start_matches('/');
    }
    if normalized.is_empty() {
        "/edgerun".to_string()
    } else {
        normalized.to_string()
    }
}

fn validate_container_id(value: &str) -> io::Result<()> {
    if value.is_empty() || value.len() > 255 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "container ID must be between 1 and 255 characters",
        ));
    }
    let Some(first) = value.chars().next() else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "container ID cannot be empty",
        ));
    };
    if !first.is_ascii_alphanumeric() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "container ID must start with an alphanumeric character",
        ));
    }
    if !value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "container ID may contain only letters, digits, '-', '_' and '.'",
        ));
    }
    Ok(())
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
            let token = String::from_utf8(secret_bytes).map_err(|_| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "invalid credential encoding",
                )
            })?;
            Ok(crate::RegistryAuth::Bearer { token })
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(crate::RegistryAuth::Anonymous),
        Err(_) => Ok(crate::RegistryAuth::Anonymous),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn parse_args_strips_global_options_before_and_after_command() {
        let parsed = parse_args(&args(&[
            "--root",
            "/tmp/state",
            "run",
            "alpine",
            "--bundle=/tmp/bundle",
            "--pid-file",
            "/tmp/pid",
            "echo",
        ]))
        .unwrap();

        assert_eq!(parsed.1, "run");
        assert_eq!(
            parsed.0.root.unwrap(),
            std::path::PathBuf::from("/tmp/state")
        );
        assert_eq!(
            parsed.0.bundle.unwrap(),
            std::path::PathBuf::from("/tmp/bundle")
        );
        assert_eq!(
            parsed.0.pid_file.unwrap(),
            std::path::PathBuf::from("/tmp/pid")
        );
        assert_eq!(parsed.2, args(&["alpine", "echo"]));
    }

    #[test]
    fn first_command_uses_same_global_option_rules_as_parser() {
        let values = args(&[
            "--bundle",
            "/tmp/bundle",
            "--root=/tmp/state",
            "create",
            "container-id",
        ]);
        assert_eq!(first_command(&values), Some("create"));
        assert!(is_container_command("create"));
        assert!(!is_container_command("pull"));
    }

    #[test]
    fn dispatch_unknown_command_reports_not_found() {
        let err = dispatch_command(&GlobalOpts::default(), "nope", &[]).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn container_id_parser_rejects_extra_args_and_unknown_options() {
        assert_eq!(
            parse_container_id_args(&args(&["abc"]), "state").unwrap(),
            "abc"
        );
        assert!(parse_container_id_args(&args(&["abc", "extra"]), "state").is_err());
        assert!(parse_container_id_args(&args(&["--bad", "abc"]), "state").is_err());
    }

    #[test]
    fn container_id_parser_rejects_path_like_ids() {
        assert!(parse_container_id_args(&args(&["../bad"]), "state").is_err());
        assert!(parse_container_id_args(&args(&["bad/id"]), "state").is_err());
    }
}
