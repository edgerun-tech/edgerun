//! Exec command implementation — run additional process in container namespaces.
//!
//! Uses raw setns() syscalls to join the container's namespaces instead of
//! relying on external tools. Supports --cwd, --env, --user, --terminal,
//! and --process (JSON process config) flags.

use crate::prelude::*;
use std::ffi::CString;
use std::fs::{self, File};
use std::io;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;

use crate::cli::env::{parse_env_pair, upsert_env};
use crate::cli::user::{resolve_user, validate_user_spec};
use crate::cli::{parse_cli_args, split_cli_prefix, EXEC_VALUE_OPTIONS};
use crate::process_exec::exec_with_env_and_cwd;
use crate::syscalls::{do_setns, ns};
use crate::terminal::{recv_fd, relay_pty_until_exit, send_fd, setup_pty_stdio};
use crate::userns::{do_setgid, do_setuid};
use edgerun_clap::cli::Action;
use edgerun_clap::{Arg, Command};

pub fn cmd_exec(opts: &crate::cli::GlobalOpts, args: &[String]) -> io::Result<()> {
    crate::cli::apply_global_opts(opts)?;

    let parsed = parse_exec_args(args)?;

    let state = crate::state::load_state(&parsed.id)?;
    let pid = state
        .pid
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "container has no PID"))?;

    if !crate::cli::is_process_alive(pid) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("container {} is not running", parsed.id),
        ));
    }

    // Load the effective runtime spec first; the pulled image bundle config is
    // only a template and misses run-time overrides such as env, user, and rootfs.
    let spec = crate::cli::load_runtime_or_bundle_spec(&state.id, &state.bundle);
    let (root_fd, root_path) = open_exec_root(pid, spec.as_ref())?;

    // Determine final args, env, cwd, user
    let (exec_args, env_vars, cwd, uid, gid) =
        if let Some(ref process_path) = parsed.process_json_path {
            // Load process config from JSON file
            let proc_data = fs::read(process_path)?;
            let proc = crate::spec::parse_oci_process(&proc_data)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            let args = proc.args.clone().unwrap_or_else(|| vec!["sh".into()]);
            let env = proc.env.clone().unwrap_or_default();
            let cwd = proc.cwd.clone().unwrap_or_else(|| "/".into());
            let uid = proc.user.as_ref().and_then(|u| u.uid).unwrap_or(0);
            let gid = proc.user.as_ref().and_then(|u| u.gid).unwrap_or(0);
            (args, env, cwd, uid, gid)
        } else if let Some(ref proc_json) = parsed.process_json {
            // Parse inline process JSON
            let proc = crate::spec::parse_oci_process(proc_json.as_bytes())
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            let args = proc.args.clone().unwrap_or_else(|| vec!["sh".into()]);
            let env = proc.env.clone().unwrap_or_default();
            let cwd = proc.cwd.clone().unwrap_or_else(|| "/".into());
            let uid = proc.user.as_ref().and_then(|u| u.uid).unwrap_or(0);
            let gid = proc.user.as_ref().and_then(|u| u.gid).unwrap_or(0);
            (args, env, cwd, uid, gid)
        } else {
            // Use spec defaults + overrides
            let proc = spec
                .as_ref()
                .and_then(|s| s.process.clone())
                .unwrap_or_default();
            let mut env = proc
                .env
                .unwrap_or_else(crate::validate::default_process_env);
            let cwd = parsed
                .cwd
                .clone()
                .or(proc.cwd)
                .unwrap_or_else(|| "/".into());
            let (uid, gid) = if let Some(ref user) = parsed.user {
                let user = resolve_user(&root_path, user)?;
                (user.uid.unwrap_or(0), user.gid.unwrap_or(0))
            } else {
                (
                    proc.user.as_ref().and_then(|u| u.uid).unwrap_or(0),
                    proc.user.as_ref().and_then(|u| u.gid).unwrap_or(0),
                )
            };

            // Apply --env overrides
            for (k, v) in &parsed.extra_env {
                upsert_env(&mut env, k, v);
            }

            let exec_args = if parsed.exec_args.is_empty() {
                proc.args.unwrap_or_else(|| vec!["sh".into()])
            } else {
                parsed.exec_args.clone()
            };
            (exec_args, env, cwd, uid, gid)
        };

    if exec_args.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "command to execute required",
        ));
    }

    // Create socketpair for passing PTY master fd from child to parent
    let mut sockets = [-1i32; 2];
    if parsed.terminal {
        let ret =
            unsafe { libc::socketpair(libc::AF_UNIX, libc::SOCK_STREAM, 0, sockets.as_mut_ptr()) };
        if ret != 0 {
            return Err(io::Error::last_os_error());
        }
    }

    // Fork and set up namespaces in child
    let child_pid = unsafe { libc::fork() };
    if child_pid < 0 {
        return Err(io::Error::last_os_error());
    }

    if child_pid == 0 {
        // Child process: join container namespaces
        let needs_pid_fork = match join_container_namespaces(pid) {
            Ok(needs_fork) => needs_fork,
            Err(_) => {
                unsafe { libc::_exit(126) };
            }
        };

        // If we joined the PID namespace, fork again so the grandchild
        // actually gets a PID inside the container's PID namespace.
        if needs_pid_fork {
            let inner_pid = unsafe { libc::fork() };
            if inner_pid < 0 {
                unsafe { libc::_exit(126) };
            }
            if inner_pid > 0 {
                // Intermediate child: exit so parent reaps us
                unsafe { libc::_exit(0) };
            }
            // Grandchild: now in the container's PID namespace
        }

        if enter_container_root(root_fd.as_raw_fd()).is_err() {
            unsafe { libc::_exit(126) };
        }

        // If --terminal, allocate a PTY and send master fd to parent
        if parsed.terminal {
            let parent_sock = sockets[0];
            // Close parent end in child
            unsafe { libc::close(parent_sock) };

            let master_fd = match setup_pty_stdio() {
                Ok(fd) => fd,
                Err(_) => unsafe { libc::_exit(126) },
            };

            // Send master fd to parent
            if send_fd(sockets[1], master_fd).is_err() {
                unsafe { libc::_exit(126) };
            }
            unsafe { libc::close(master_fd) };
        }

        // Drop privileges if needed
        if gid != 0 {
            let _ = do_setgid(gid);
        }
        if uid != 0 {
            let _ = do_setuid(uid);
        }

        let exit_code = exec_with_env_and_cwd(&exec_args, &env_vars, &cwd)
            .err()
            .map(|error| error.exit_code())
            .unwrap_or(127);
        unsafe { libc::_exit(exit_code) };
    }

    // Parent: receive PTY master fd if --terminal, then wait/relay
    if parsed.terminal {
        let parent_sock = sockets[0];
        // Close child end in parent
        unsafe { libc::close(sockets[1]) };

        let pty_master = recv_fd(parent_sock)
            .map_err(|e| io::Error::other(format!("failed to receive PTY master fd: {}", e)))?;
        unsafe { libc::close(parent_sock) };

        // Bidirectional TTY relay
        let exit_code = relay_pty_until_exit(pty_master, child_pid, true)?;
        unsafe { libc::close(pty_master) };

        if exit_code != 0 {
            std::process::exit(exit_code);
        }
    } else {
        // No terminal — just wait for child
        let mut status: i32 = 0;
        unsafe { libc::waitpid(child_pid, &mut status, 0) };

        if libc::WIFEXITED(status) {
            let code = libc::WEXITSTATUS(status);
            if code != 0 {
                std::process::exit(code);
            }
        } else if libc::WIFSIGNALED(status) {
            let sig = libc::WTERMSIG(status);
            std::process::exit(128 + sig);
        }
    }

    Ok(())
}

pub(crate) fn open_exec_root(
    pid: u32,
    spec: Option<&crate::spec::OciSpec>,
) -> io::Result<(File, PathBuf)> {
    if let Some(root_path) = spec
        .and_then(|spec| spec.root.as_ref())
        .map(|root| root.path.as_str())
        .filter(|path| !path.is_empty())
    {
        if let Ok(file) = File::open(root_path) {
            return Ok((file, PathBuf::from(root_path)));
        }
    }

    let proc_root = format!("/proc/{pid}/root");
    File::open(&proc_root)
        .map(|file| (file, PathBuf::from(&proc_root)))
        .map_err(|error| io::Error::new(error.kind(), format!("open {proc_root}: {error}")))
}

pub(crate) fn enter_container_root(root_fd: i32) -> io::Result<()> {
    let ret = unsafe { libc::fchdir(root_fd) };
    if ret != 0 {
        return Err(io::Error::last_os_error());
    }
    let dot = CString::new(".").unwrap();
    let ret = unsafe { libc::chroot(dot.as_ptr()) };
    if ret != 0 {
        return Err(io::Error::last_os_error());
    }
    let slash = CString::new("/").unwrap();
    let ret = unsafe { libc::chdir(slash.as_ptr()) };
    if ret != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// Join all relevant namespaces of the container's init process.
///
/// Returns `true` if the PID namespace was joined (caller must fork to enter it).
pub(crate) fn join_container_namespaces(pid: u32) -> io::Result<bool> {
    let mut joined_pid_ns = false;

    // Namespaces to join: mount, uts, ipc, net, pid
    // Skip user namespace (can't join existing user ns easily)
    let ns_list = [
        ("mnt", ns::NEWNS),
        ("uts", ns::NEWUTS),
        ("ipc", ns::NEWIPC),
        ("net", ns::NEWNET),
        ("pid", ns::NEWPID),
    ];

    for (name, flag) in &ns_list {
        let ns_path = format!("/proc/{}/ns/{}", pid, name);
        if let Ok(file) = fs::File::open(&ns_path) {
            let fd = file.as_raw_fd();
            if let Err(e) = do_setns(fd, *flag) {
                // PID namespace setns succeeds but the process doesn't actually
                // enter it — only children after a fork will be in the new ns.
                // For other namespaces, failure is logged but skipped.
                if *name == "pid" {
                    joined_pid_ns = true;
                } else {
                    let _ = e;
                }
            } else if *name == "pid" {
                // setns on pid namespace returns success but the caller still
                // has its old PID ns. A fork is required to actually enter it.
                joined_pid_ns = true;
            }
        }
    }

    Ok(joined_pid_ns)
}

// ===========================================================================
// Exec argument parsing
// ===========================================================================

#[derive(Debug, Default)]
struct ExecArgs {
    id: String,
    cwd: Option<String>,
    extra_env: Vec<(String, String)>,
    user: Option<String>,
    terminal: bool,
    process_json_path: Option<String>,
    process_json: Option<String>,
    exec_args: Vec<String>,
}

fn parse_exec_args(args: &[String]) -> io::Result<ExecArgs> {
    const USAGE: &str = "Usage: ert exec [options] <container-id> [cmd...]";
    let (prefix, id, exec_args) = split_cli_prefix(args, EXEC_VALUE_OPTIONS);
    let matches = parse_cli_args(
        Command::new("exec")
            .arg(Arg::new("cwd").long("cwd"))
            .arg(
                Arg::new("env")
                    .short('e')
                    .long("env")
                    .action(Action::Append),
            )
            .arg(Arg::new("user").short('u').long("user"))
            .arg(
                Arg::new("terminal")
                    .short('t')
                    .long("terminal")
                    .action(Action::StoreTrue),
            )
            .arg(Arg::new("process").short('p').long("process"))
            .arg(Arg::new("process-json").long("process-json")),
        &prefix,
        USAGE,
    )?;
    let id =
        id.ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "container ID required"))?;
    crate::cli::validate_container_id(&id)?;

    let mut result = ExecArgs {
        id,
        cwd: matches.get_one::<String>("cwd"),
        user: matches
            .get_one::<String>("user")
            .map(|user| validate_user_spec(&user))
            .transpose()?,
        terminal: matches.get_flag("terminal"),
        process_json_path: matches.get_one::<String>("process"),
        process_json: matches.get_one::<String>("process-json"),
        exec_args,
        ..Default::default()
    };

    if let Some(env) = matches.get_many::<String>("env") {
        for entry in env {
            let (key, value) = parse_env_pair(&entry)?;
            result.extra_env.push((key.into(), value.into()));
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn parses_exec_options_with_command_tail() {
        let parsed = parse_exec_args(&args(&[
            "--cwd",
            "/work",
            "-e",
            "A=1",
            "--env=B=2",
            "-t",
            "container-a",
            "sh",
            "-lc",
            "echo ok",
        ]))
        .unwrap();

        assert_eq!(parsed.id, "container-a");
        assert_eq!(parsed.cwd.as_deref(), Some("/work"));
        assert_eq!(
            parsed.extra_env,
            vec![
                ("A".to_string(), "1".to_string()),
                ("B".to_string(), "2".to_string())
            ]
        );
        assert!(parsed.terminal);
        assert_eq!(parsed.exec_args, args(&["sh", "-lc", "echo ok"]));
    }

    #[test]
    fn exec_stops_parsing_after_container_id() {
        let parsed =
            parse_exec_args(&args(&["container-a", "--not-an-exec-flag", "value"])).unwrap();

        assert_eq!(parsed.id, "container-a");
        assert_eq!(parsed.exec_args, args(&["--not-an-exec-flag", "value"]));
    }
}
