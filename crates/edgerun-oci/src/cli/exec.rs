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

use crate::cli::user::{resolve_user, validate_user_spec};
use crate::syscalls::{do_setns, ns};
use crate::userns::{do_setgid, do_setuid};

/// Relay I/O between the host terminal and a container PTY.
///
/// Sets the host terminal to raw mode, forwards stdin→pty_master and
/// pty_master→stdout until the child exits, then restores terminal settings.
fn tty_relay(pty_master_fd: i32, child_pid: libc::pid_t) -> io::Result<std::process::ExitStatus> {
    use std::mem::MaybeUninit;
    use std::os::unix::process::ExitStatusExt;

    extern "C" {
        fn tcgetattr(fd: i32, termios: *mut libc::termios) -> i32;
        fn tcsetattr(fd: i32, optional_actions: i32, termios: *const libc::termios) -> i32;
    }

    // Save the host terminal settings
    let stdin_fd = libc::STDIN_FILENO;
    let mut orig_termios: libc::termios = unsafe { MaybeUninit::zeroed().assume_init() };
    if unsafe { tcgetattr(stdin_fd, &mut orig_termios) } != 0 {
        return Err(io::Error::last_os_error());
    }

    // Put the terminal in raw mode
    let mut raw_termios = orig_termios;
    raw_termios.c_lflag &= !(libc::ECHO | libc::ICANON | libc::ISIG | libc::IEXTEN);
    raw_termios.c_iflag &= !(libc::BRKINT | libc::ICRNL | libc::INPCK | libc::ISTRIP | libc::IXON);
    raw_termios.c_cflag &= !(libc::CSIZE | libc::PARENB);
    raw_termios.c_cflag |= libc::CS8;
    raw_termios.c_cc[libc::VMIN] = 1;
    raw_termios.c_cc[libc::VTIME] = 0;
    unsafe { tcsetattr(stdin_fd, libc::TCSAFLUSH, &raw_termios) };

    // Restore terminal on exit (including signals, panics, etc.)
    struct TerminalGuard(libc::termios);
    impl Drop for TerminalGuard {
        fn drop(&mut self) {
            unsafe { tcsetattr(libc::STDIN_FILENO, libc::TCSAFLUSH, &self.0) };
        }
    }
    let _guard = TerminalGuard(orig_termios);

    // Bidirectional relay using poll(2)
    use std::os::fd::IntoRawFd;
    use std::os::unix::io::FromRawFd;
    let stdin_file = unsafe { std::fs::File::from_raw_fd(stdin_fd) };
    let pty_file = unsafe { std::fs::File::from_raw_fd(pty_master_fd) };

    // Don't let File::drop close stdin or the pty master (owned elsewhere)
    let _ = stdin_file.into_raw_fd();
    let _ = pty_file.into_raw_fd();

    let mut buf_in = [0u8; 4096];
    let mut buf_out = [0u8; 4096];

    loop {
        let mut fds = [
            libc::pollfd {
                fd: stdin_fd,
                events: libc::POLLIN,
                revents: 0,
            },
            libc::pollfd {
                fd: pty_master_fd,
                events: libc::POLLIN,
                revents: 0,
            },
        ];

        let ret = unsafe { libc::poll(fds.as_mut_ptr(), 2, 100) };
        if ret < 0 {
            if unsafe { *libc::__errno_location() } == libc::EINTR {
                continue;
            }
            break;
        }

        // stdin → pty_master
        if fds[0].revents & libc::POLLIN != 0 {
            let n = unsafe {
                libc::read(
                    stdin_fd,
                    buf_in.as_mut_ptr() as *mut libc::c_void,
                    buf_in.len(),
                )
            };
            if n > 0 {
                let mut sent = 0isize;
                while sent < n as isize {
                    let w = unsafe {
                        libc::write(
                            pty_master_fd,
                            buf_in.as_ptr().offset(sent) as *const libc::c_void,
                            (n as isize - sent) as usize,
                        )
                    };
                    if w <= 0 {
                        break;
                    }
                    sent += w as isize;
                }
            } else if n == 0 {
                break; // stdin EOF
            }
        }

        // pty_master → stdout
        if fds[1].revents & libc::POLLIN != 0 {
            let n = unsafe {
                libc::read(
                    pty_master_fd,
                    buf_out.as_mut_ptr() as *mut libc::c_void,
                    buf_out.len(),
                )
            };
            if n > 0 {
                let mut sent = 0isize;
                while sent < n as isize {
                    let w = unsafe {
                        libc::write(
                            libc::STDOUT_FILENO,
                            buf_out.as_ptr().offset(sent) as *const libc::c_void,
                            (n as isize - sent) as usize,
                        )
                    };
                    if w <= 0 {
                        break;
                    }
                    sent += w as isize;
                }
            } else if n == 0 {
                break; // pty EOF
            }
        }

        // Check if child has exited
        let mut status: i32 = 0;
        let wait_ret = unsafe { libc::waitpid(child_pid, &mut status, libc::WNOHANG) };
        if wait_ret > 0 {
            // Drain remaining pty output
            loop {
                let n = unsafe {
                    libc::read(
                        pty_master_fd,
                        buf_out.as_mut_ptr() as *mut libc::c_void,
                        buf_out.len(),
                    )
                };
                if n <= 0 {
                    break;
                }
                let mut sent = 0isize;
                while sent < n as isize {
                    let w = unsafe {
                        libc::write(
                            libc::STDOUT_FILENO,
                            buf_out.as_ptr().offset(sent) as *const libc::c_void,
                            (n as isize - sent) as usize,
                        )
                    };
                    if w <= 0 {
                        break;
                    }
                    sent += w as isize;
                }
            }
            return Ok(std::process::ExitStatus::from_raw(status));
        }
    }

    // Child hasn't exited yet — wait for it
    let mut status: i32 = 0;
    unsafe { libc::waitpid(child_pid, &mut status, 0) };
    Ok(std::process::ExitStatus::from_raw(status))
}

/// Send a file descriptor over a Unix socket using SCM_RIGHTS.
fn send_fd(sock_fd: i32, fd: i32) -> io::Result<()> {
    let mut msg: libc::msghdr = unsafe { std::mem::MaybeUninit::zeroed().assume_init() };
    let cmsg_space = unsafe { libc::CMSG_SPACE(std::mem::size_of::<i32>() as u32) as usize };
    let mut cmsg_buf = vec![0u8; cmsg_space];

    let fd_to_send = fd;
    let iov = libc::iovec {
        iov_base: &fd_to_send as *const _ as *mut libc::c_void,
        iov_len: std::mem::size_of::<i32>(),
    };
    msg.msg_iov = &iov as *const _ as *mut libc::iovec;
    msg.msg_iovlen = 1;
    msg.msg_control = cmsg_buf.as_mut_ptr() as *mut libc::c_void;
    msg.msg_controllen = cmsg_space as _;

    let cmsg = unsafe { libc::CMSG_FIRSTHDR(&msg) };
    unsafe {
        (*cmsg).cmsg_level = libc::SOL_SOCKET;
        (*cmsg).cmsg_type = libc::SCM_RIGHTS;
        (*cmsg).cmsg_len = libc::CMSG_LEN(std::mem::size_of::<i32>() as u32) as _;
        std::ptr::copy_nonoverlapping(&fd as *const i32, libc::CMSG_DATA(cmsg) as *mut i32, 1);
    }

    let ret = unsafe { libc::sendmsg(sock_fd, &msg, 0) };
    if ret < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

/// Receive a file descriptor over a Unix socket using SCM_RIGHTS.
fn recv_fd(sock_fd: i32) -> io::Result<i32> {
    let mut msg: libc::msghdr = unsafe { std::mem::MaybeUninit::zeroed().assume_init() };
    let cmsg_space = unsafe { libc::CMSG_SPACE(std::mem::size_of::<i32>() as u32) as usize };
    let mut cmsg_buf = vec![0u8; cmsg_space];
    let mut fd_buf: i32 = 0;

    let iov = libc::iovec {
        iov_base: &mut fd_buf as *mut _ as *mut libc::c_void,
        iov_len: std::mem::size_of::<i32>(),
    };
    msg.msg_iov = &iov as *const _ as *mut libc::iovec;
    msg.msg_iovlen = 1;
    msg.msg_control = cmsg_buf.as_mut_ptr() as *mut libc::c_void;
    msg.msg_controllen = cmsg_space as _;

    let ret = unsafe { libc::recvmsg(sock_fd, &mut msg, 0) };
    if ret < 0 {
        return Err(io::Error::last_os_error());
    }

    let cmsg = unsafe { libc::CMSG_FIRSTHDR(&msg) };
    if cmsg.is_null() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "no control message received",
        ));
    }

    let fd = unsafe {
        let data = libc::CMSG_DATA(cmsg) as *const i32;
        std::ptr::read_unaligned(data)
    };
    Ok(fd)
}

pub fn cmd_exec(opts: &crate::cli::GlobalOpts, args: &[String]) -> io::Result<()> {
    if let Some(ref root) = opts.root {
        crate::state::set_state_dir(root.to_str().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "--root path is not valid UTF-8",
            )
        })?);
    }

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
    let spec = load_exec_spec(&state);
    let (root_fd, root_path) = open_exec_root(pid, spec.as_ref())?;

    // Determine final args, env, cwd, user
    let (exec_args, env_vars, cwd, uid, gid) =
        if let Some(ref process_path) = parsed.process_json_path {
            // Load process config from JSON file
            let proc_data = fs::read(process_path)?;
            let proc: crate::json::OciProcess = edgerun_json::from_slice(&proc_data)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            let args = proc.args.clone().unwrap_or_else(|| vec!["sh".into()]);
            let env = proc.env.clone().unwrap_or_default();
            let cwd = proc.cwd.clone().unwrap_or_else(|| "/".into());
            let uid = proc.user.as_ref().and_then(|u| u.uid).unwrap_or(0);
            let gid = proc.user.as_ref().and_then(|u| u.gid).unwrap_or(0);
            (args, env, cwd, uid, gid)
        } else if let Some(ref proc_json) = parsed.process_json {
            // Parse inline process JSON
            let proc: crate::json::OciProcess = edgerun_json::from_slice(proc_json.as_bytes())
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
                let entry = format!("{}={}", k, v);
                env.retain(|e| !e.starts_with(&format!("{}=", k)));
                env.push(entry);
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

            extern "C" {
                fn posix_openpt(flags: libc::c_int) -> libc::c_int;
                fn grantpt(fd: libc::c_int) -> libc::c_int;
                fn unlockpt(fd: libc::c_int) -> libc::c_int;
                fn ptsname(fd: libc::c_int) -> *const std::ffi::c_char;
            }

            let master_fd = unsafe { posix_openpt(libc::O_RDWR | libc::O_NOCTTY) };
            if master_fd < 0 {
                unsafe { libc::_exit(126) };
            }
            if unsafe { grantpt(master_fd) } != 0 || unsafe { unlockpt(master_fd) } != 0 {
                unsafe { libc::_exit(126) };
            }

            // Send master fd to parent
            if send_fd(sockets[1], master_fd).is_err() {
                unsafe { libc::_exit(126) };
            }
            unsafe { libc::close(master_fd) };

            // Open slave and dup to stdio
            let slave_path = unsafe { ptsname(master_fd) };
            if slave_path.is_null() {
                unsafe { libc::_exit(126) };
            }
            let slave_fd = unsafe { libc::open(slave_path, libc::O_RDWR) };
            if slave_fd < 0 {
                unsafe { libc::_exit(126) };
            }

            // Set controlling terminal
            unsafe { libc::ioctl(slave_fd, libc::TIOCSCTTY as _, 0) };

            unsafe { libc::dup2(slave_fd, libc::STDIN_FILENO) };
            unsafe { libc::dup2(slave_fd, libc::STDOUT_FILENO) };
            unsafe { libc::dup2(slave_fd, libc::STDERR_FILENO) };
            if slave_fd > 2 {
                unsafe { libc::close(slave_fd) };
            }
        }

        // Drop privileges if needed
        if gid != 0 {
            let _ = do_setgid(gid);
        }
        if uid != 0 {
            let _ = do_setuid(uid);
        }

        // chdir
        let cwd_c = match CString::new(cwd.as_bytes()) {
            Ok(c) => c,
            Err(_) => {
                unsafe { libc::_exit(126) };
            }
        };
        unsafe { libc::chdir(cwd_c.as_ptr()) };

        // Set environment
        unsafe { libc::clearenv() };
        for e in &env_vars {
            if let Some((k, v)) = e.split_once('=') {
                let k_c = match CString::new(k.as_bytes()) {
                    Ok(c) => c,
                    Err(_) => continue,
                };
                let v_c = match CString::new(v.as_bytes()) {
                    Ok(c) => c,
                    Err(_) => continue,
                };
                unsafe { libc::setenv(k_c.as_ptr(), v_c.as_ptr(), 1) };
            }
        }

        // Resolve executable
        let exe_path = if exec_args[0].starts_with('/') {
            exec_args[0].clone()
        } else {
            let exe_name = &exec_args[0];
            let path_env = env_vars
                .iter()
                .find(|e| e.starts_with("PATH="))
                .map(|e| &e[5..])
                .unwrap_or("/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin");
            let mut found = None;
            for dir in path_env.split(':') {
                let candidate = format!("{}/{}", dir, exe_name);
                if std::path::Path::new(&candidate).exists() {
                    found = Some(candidate);
                    break;
                }
            }
            found.unwrap_or_else(|| exe_name.clone())
        };

        // Exec
        let exe_cstr = match CString::new(exe_path.as_bytes()) {
            Ok(c) => c,
            Err(_) => {
                unsafe { libc::_exit(126) };
            }
        };
        let c_args: Vec<CString> = exec_args
            .iter()
            .filter_map(|a| CString::new(a.as_bytes()).ok())
            .collect();
        let c_ptrs: Vec<*const libc::c_char> = c_args
            .iter()
            .map(|s| s.as_ptr())
            .chain(std::iter::once(std::ptr::null()))
            .collect();

        unsafe { libc::execvp(exe_cstr.as_ptr(), c_ptrs.as_ptr()) };
        unsafe { libc::_exit(127) };
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
        let exit_status = tty_relay(pty_master, child_pid)?;
        unsafe { libc::close(pty_master) };

        if let Some(code) = exit_status.code() {
            if code != 0 {
                std::process::exit(code);
            }
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

pub(crate) fn load_exec_spec(state: &crate::state::ContainerState) -> Option<crate::json::OciSpec> {
    let runtime_config = crate::state::runtime_spec_path(&state.id);
    if let Ok(data) = fs::read(runtime_config) {
        if let Ok(spec) = crate::json::parse_oci_spec(&data) {
            return Some(spec);
        }
    }

    let config_path = std::path::Path::new(&state.bundle).join("config.json");
    fs::read(config_path)
        .ok()
        .and_then(|data| crate::json::parse_oci_spec(&data).ok())
}

pub(crate) fn open_exec_root(
    pid: u32,
    spec: Option<&crate::json::OciSpec>,
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
    let mut result = ExecArgs::default();
    let mut i = 0;

    while i < args.len() {
        match args[i].as_str() {
            "--cwd" => {
                i += 1;
                if i >= args.len() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--cwd requires a value",
                    ));
                }
                result.cwd = Some(args[i].clone());
            }
            "--env" | "-e" => {
                i += 1;
                if i >= args.len() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--env requires a value",
                    ));
                }
                if let Some((k, v)) = args[i].split_once('=') {
                    result.extra_env.push((k.into(), v.into()));
                }
            }
            "--user" | "-u" => {
                i += 1;
                if i >= args.len() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--user requires a value",
                    ));
                }
                result.user = Some(validate_user_spec(&args[i])?);
            }
            "--terminal" | "-t" => {
                result.terminal = true;
            }
            "--process" | "-p" => {
                i += 1;
                if i >= args.len() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--process requires a path",
                    ));
                }
                result.process_json_path = Some(args[i].clone());
            }
            "--process-json" => {
                i += 1;
                if i >= args.len() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--process-json requires a value",
                    ));
                }
                result.process_json = Some(args[i].clone());
            }
            "--" => {
                i += 1;
                result.exec_args = args[i..].to_vec();
                break;
            }
            s if !s.starts_with('-') => {
                if result.id.is_empty() {
                    result.id = args[i].clone();
                } else {
                    result.exec_args = args[i..].to_vec();
                    break;
                }
            }
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("unknown exec flag: {}", args[i]),
                ));
            }
        }
        i += 1;
    }

    if result.id.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "container ID required",
        ));
    }

    Ok(result)
}
