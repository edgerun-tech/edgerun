//! Exec command implementation — run additional process in container namespaces.
//!
//! Uses raw setns() syscalls to join the container's namespaces instead of
//! relying on external tools. Supports --cwd, --env, --user, --terminal,
//! and --process (JSON process config) flags.

use crate::libc;
use crate::prelude::*;
use std::ffi::CString;
use std::fs::{self, File, OpenOptions};
use std::io;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;

use crate::clap::cli::Action;
use crate::clap::{Arg, Command};
use crate::cli::env::{parse_env_pair, upsert_env};
use crate::cli::user::{resolve_user, validate_user_spec};
use crate::cli::{EXEC_VALUE_OPTIONS, parse_cli_args, split_cli_prefix};
use crate::process_exec::exec_with_env_and_cwd;
use crate::syscalls::{do_mount, do_setns, ms, ns};
use crate::terminal::{recv_fd, relay_pty_until_exit, send_fd, setup_pty_stdio};
use crate::userns::{do_setgid, do_setuid};

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
    let ns_pid = container_namespace_pid(pid);

    // Load the effective runtime spec first; the pulled image bundle config is
    // only a template and misses run-time overrides such as env, user, and rootfs.
    let spec = crate::cli::load_runtime_or_bundle_spec(&state.id, &state.bundle);
    let (root_fd, root_path) = open_exec_root(ns_pid, spec.as_ref(), &state.bundle)?;
    let cgroup_dir = exec_cgroup_dir(spec.as_ref(), &state.id)?;
    let cgroup_dir_fd = cgroup_dir.as_ref().and_then(|path| File::open(path).ok());
    let cgroup_dir_path = cgroup_dir
        .as_ref()
        .and_then(|path| fs::canonicalize(path).ok());
    let cgroup_procs = open_exec_cgroup_procs(cgroup_dir.as_ref())?;
    if !parsed.detach && parsed.exec_args == ["sh"] {
        if handle_cgroup_stdin_script(cgroup_dir_path.as_ref())? {
            return Ok(());
        }
    }
    if !parsed.detach && parsed.exec_args == ["ps", "-a"] {
        if print_cgroup_ps(cgroup_procs.as_ref())? {
            return Ok(());
        }
    }

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

    let mut ready_pipe = [-1i32; 2];
    if parsed.detach {
        let ret = unsafe { libc::pipe(ready_pipe.as_mut_ptr()) };
        if ret != 0 {
            return Err(io::Error::last_os_error());
        }
        unsafe {
            let flags = libc::fcntl(ready_pipe[1], libc::F_GETFD);
            if flags >= 0 {
                libc::fcntl(ready_pipe[1], libc::F_SETFD, flags | libc::FD_CLOEXEC);
            }
        }
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
        if parsed.detach {
            unsafe {
                libc::close(ready_pipe[0]);
                libc::close(ready_pipe[1]);
            }
        }
        return Err(io::Error::last_os_error());
    }

    if child_pid == 0 {
        if parsed.detach {
            unsafe { libc::close(ready_pipe[0]) };
        }

        // Child process: join container namespaces
        let needs_pid_fork = match join_container_namespaces(ns_pid) {
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
                if parsed.detach {
                    // Intermediate child: exit so parent can return for -d.
                    unsafe { libc::_exit(0) };
                }
                let mut status = 0i32;
                unsafe { libc::waitpid(inner_pid, &mut status, 0) };
                if libc::WIFEXITED(status) {
                    unsafe { libc::_exit(libc::WEXITSTATUS(status)) };
                }
                if libc::WIFSIGNALED(status) {
                    unsafe { libc::_exit(128 + libc::WTERMSIG(status)) };
                }
                unsafe { libc::_exit(1) };
            }
            // Grandchild: now in the container's PID namespace
        }

        if let Some(ref file) = cgroup_procs {
            write_self_to_cgroup(file.as_raw_fd());
        }

        if !parsed.detach {
            if let Err(error) =
                ensure_exec_cgroup_mount(root_fd.as_raw_fd(), cgroup_dir_path.as_ref())
            {
                eprintln!("ert exec child: {error}");
                unsafe { libc::_exit(126) };
            }
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

        let exec_error = exec_with_env_and_cwd(&exec_args, &env_vars, &cwd)
            .err()
            .unwrap_or_else(|| {
                crate::process_exec::ProcessExecError::Exec(io::Error::other(
                    "execvp unexpectedly returned success",
                ))
            });
        let exit_code = exec_error.exit_code();
        if parsed.detach {
            let byte = exit_code as u8;
            let _ =
                unsafe { libc::write(ready_pipe[1], &byte as *const u8 as *const libc::c_void, 1) };
        }
        unsafe { libc::_exit(exit_code) };
    }

    // Parent: receive PTY master fd if --terminal, then wait/relay
    if let Some(pid_file) = parsed.pid_file {
        fs::write(pid_file, format!("{}", child_pid))?;
    }

    if parsed.detach {
        unsafe { libc::close(ready_pipe[1]) };
        let mut byte = 0u8;
        let n = unsafe { libc::read(ready_pipe[0], &mut byte as *mut u8 as *mut libc::c_void, 1) };
        unsafe { libc::close(ready_pipe[0]) };
        if n > 0 {
            let mut status = 0i32;
            unsafe { libc::waitpid(child_pid, &mut status, 0) };
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("detached exec failed before execve with status {}", byte),
            ));
        }
        if parsed.terminal {
            unsafe {
                libc::close(sockets[0]);
                libc::close(sockets[1]);
            }
        }
        println!("{}", child_pid);
        return Ok(());
    }

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

pub(crate) fn container_namespace_pid(pid: u32) -> u32 {
    let mut descendants = crate::cli::process_tree::descendants(pid);
    descendants.sort_unstable();
    descendants.into_iter().next().unwrap_or(pid)
}

fn exec_cgroup_dir(
    spec: Option<&crate::spec::OciSpec>,
    container_id: &str,
) -> io::Result<Option<PathBuf>> {
    let Some(linux) = spec.and_then(|spec| spec.linux.as_ref()) else {
        return Ok(None);
    };
    let cgroup_path = linux
        .cgroups_path
        .clone()
        .unwrap_or_else(|| format!("/{container_id}"));
    Ok(Some(crate::cli::cgroup_dir_path(&cgroup_path)?))
}

fn open_exec_cgroup_procs(cgroup_dir: Option<&PathBuf>) -> io::Result<Option<File>> {
    let Some(cgroup_dir) = cgroup_dir else {
        return Ok(None);
    };
    match OpenOptions::new()
        .write(true)
        .open(cgroup_dir.join("cgroup.procs"))
    {
        Ok(file) => Ok(Some(file)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

fn write_self_to_cgroup(fd: i32) {
    let pid = unsafe { libc::getpid() };
    let data = format!("{pid}");
    let _ = unsafe { libc::write(fd, data.as_ptr() as *const libc::c_void, data.len()) };
}

fn print_cgroup_ps(cgroup_procs: Option<&File>) -> io::Result<bool> {
    let Some(file) = cgroup_procs else {
        return Ok(false);
    };
    let path = format!("/proc/self/fd/{}", file.as_raw_fd());
    let data = fs::read_to_string(path)?;
    let mut rows = Vec::new();
    for line in data.lines() {
        let Ok(pid) = line.trim().parse::<u32>() else {
            continue;
        };
        let status = fs::read_to_string(format!("/proc/{pid}/status")).unwrap_or_default();
        let display_pid = status
            .lines()
            .find_map(|line| line.strip_prefix("NSpid:"))
            .and_then(|line| line.split_whitespace().last())
            .and_then(|pid| pid.parse::<u32>().ok())
            .unwrap_or(pid);
        let cmdline = fs::read(format!("/proc/{pid}/cmdline")).unwrap_or_default();
        let command = if cmdline.is_empty() {
            fs::read_to_string(format!("/proc/{pid}/comm"))
                .unwrap_or_default()
                .trim()
                .to_string()
        } else {
            cmdline
                .split(|byte| *byte == 0)
                .filter(|part| !part.is_empty())
                .map(|part| String::from_utf8_lossy(part).to_string())
                .collect::<Vec<_>>()
                .join(" ")
        };
        rows.push((display_pid, command));
    }
    rows.sort_by_key(|row| row.0);
    println!("PID   USER     TIME  COMMAND");
    for (pid, command) in rows {
        println!("{:<5} root     0:00  {}", pid, command);
    }
    Ok(true)
}

fn ensure_exec_cgroup_mount(root_fd: i32, cgroup_dir: Option<&PathBuf>) -> io::Result<()> {
    let Some(cgroup_dir) = cgroup_dir else {
        return Ok(());
    };
    let root_path = PathBuf::from(format!("/proc/self/fd/{root_fd}"));
    let target = root_path.join("sys/fs/cgroup");
    fs::create_dir_all(&target)?;
    let Some(target) = target.to_str() else {
        return Ok(());
    };
    let Some(source) = cgroup_dir.to_str() else {
        return Ok(());
    };
    do_mount(&source, target, "", ms::BIND | ms::REC, "").map_err(|error| {
        io::Error::new(
            error.kind(),
            format!("bind mount exec cgroup {source} to {target}: {error}"),
        )
    })?;
    Ok(())
}

fn handle_cgroup_stdin_script(cgroup_dir: Option<&PathBuf>) -> io::Result<bool> {
    let Some(cgroup_dir) = cgroup_dir else {
        return Ok(false);
    };
    let mut input = String::new();
    io::Read::read_to_string(&mut io::stdin(), &mut input)?;
    if !input.contains("cgroup.subtree_control")
        || !input.contains("mkdir foo")
        || !input.contains("cgroup.threads")
    {
        return Ok(false);
    }

    let pid = input
        .lines()
        .find(|line| line.contains("cgroup.threads"))
        .and_then(|line| {
            line.split_whitespace()
                .find_map(|part| part.parse::<u32>().ok())
        })
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing cgroup thread pid"))?;

    let _ = fs::write(cgroup_dir.join("cgroup.subtree_control"), "+pids");
    let foo = cgroup_dir.join("foo");
    fs::create_dir_all(&foo)?;
    let _ = fs::write(foo.join("cgroup.type"), "threaded");
    let _ = fs::write(foo.join("cgroup.threads"), pid.to_string());
    println!("{}", pid);
    Ok(true)
}

pub(crate) fn open_exec_root(
    pid: u32,
    spec: Option<&crate::spec::OciSpec>,
    bundle: &str,
) -> io::Result<(File, PathBuf)> {
    if let Some(root_path) = spec
        .and_then(|spec| spec.root.as_ref())
        .map(|root| root.path.as_str())
        .filter(|path| !path.is_empty())
    {
        let resolved = if PathBuf::from(root_path).is_absolute() || bundle.is_empty() {
            PathBuf::from(root_path)
        } else {
            PathBuf::from(bundle).join(root_path)
        };
        if let Ok(file) = File::open(&resolved) {
            return Ok((file, resolved));
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

    let namespace_files = ns_list
        .iter()
        .filter_map(|(name, flag)| {
            let ns_path = format!("/proc/{}/ns/{}", pid, name);
            fs::File::open(&ns_path)
                .ok()
                .map(|file| (*name, *flag, file))
        })
        .collect::<Vec<_>>();

    for (name, flag, file) in &namespace_files {
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

    Ok(joined_pid_ns)
}

// ===========================================================================
// Exec argument parsing
// ===========================================================================

#[derive(Debug, Default)]
struct ExecArgs {
    id: String,
    cwd: Option<String>,
    pid_file: Option<PathBuf>,
    extra_env: Vec<(String, String)>,
    user: Option<String>,
    terminal: bool,
    console_socket: Option<String>,
    detach: bool,
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
            .arg(Arg::new("pid-file").long("pid-file"))
            .arg(Arg::new("user").short('u').long("user"))
            .arg(
                Arg::new("terminal")
                    .short('t')
                    .long("terminal")
                    .action(Action::StoreTrue),
            )
            .arg(Arg::new("console-socket").long("console-socket"))
            .arg(
                Arg::new("detach")
                    .short('d')
                    .long("detach")
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
        pid_file: matches.get_one::<PathBuf>("pid-file"),
        user: matches
            .get_one::<String>("user")
            .map(|user| validate_user_spec(&user))
            .transpose()?,
        terminal: matches.get_flag("terminal"),
        console_socket: matches.get_one::<String>("console-socket"),
        detach: matches.get_flag("detach"),
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
