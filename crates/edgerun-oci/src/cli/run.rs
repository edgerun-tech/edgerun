//! `ert run` — pull (if needed) + create + start + wait in one command.
//!
//! Usage:
//!   `ert run <image> [cmd...]`
//!   `ert run --rm alpine:latest echo hello`
//!   `ert run --name mycontainer --images-dir /mnt/img nginx:latest`

use crate::prelude::*;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;

use crate::cli::{default_images_dir, default_store_dir, resolve_registry_auth, GlobalOpts};
use crate::json::{parse_oci_spec, OciSpec};
use crate::lifecycle::{
    fork_container_child_with_terminal_socket, run_create_runtime_hooks, run_poststart_hooks,
    run_prestart_hooks, save_created_state, setup_container_cgroups, signal_start,
    update_state_running,
};
use crate::process::validate_spec;
use crate::state::{delete_state, load_state};

use crate::ImageRef;
use crate::RegistryClient;

struct RunOpts {
    rm: bool,
    detach: bool,
    name: Option<String>,
    env: Vec<String>,
    workdir: Option<String>,
    entrypoint: Option<String>,
    privileged: bool,
    interactive: bool,
    tty: bool,
    images_dir: PathBuf,
    store_path: PathBuf,
}

pub fn cmd_run(opts: &GlobalOpts, args: &[String]) -> io::Result<()> {
    if let Some(ref root) = opts.root {
        crate::state::set_state_dir(root.to_str().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "--root path is not valid UTF-8",
            )
        })?);
    }

    let (run_opts, image_ref_str, cmd_args) = parse_run_args(args)?;
    if run_opts.detach && run_opts.rm {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "--rm cannot be combined with detached mode yet",
        ));
    }
    if run_opts.detach && (run_opts.interactive || run_opts.tty) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "detached mode does not support -i or -t yet",
        ));
    }
    if run_opts.tty && run_opts.interactive && unsafe { libc::isatty(libc::STDIN_FILENO) } != 1 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "the input device is not a TTY",
        ));
    }

    let image: ImageRef = image_ref_str
        .parse()
        .map_err(|e: String| io::Error::new(io::ErrorKind::InvalidInput, e))?;

    // Resolve auth
    let auth = resolve_registry_auth(&image.registry)?;

    // Pull if not already local
    let bundle_path = run_opts.images_dir.join(&image.repository).join(&image.tag);

    if !bundle_path.join("config.json").exists() {
        eprintln!("Pulling {}...", image_ref_str);
        let rt = edgerun_rt::Runtime::new_multi_thread()
            .enable_all()
            .build()
            .map_err(|e| io::Error::other(e.to_string()))?;

        let image_clone = image.clone();
        let bundle_clone = bundle_path.clone();
        let store_clone = run_opts.store_path.clone();

        let mut client = RegistryClient::new().with_auth(auth);
        let result = rt
            .block_on(async move { client.pull(&image_clone, &bundle_clone, &store_clone).await });

        result.map_err(|e| io::Error::other(format!("pull failed: {}", e)))?;
    }

    // Read and override the spec
    let config_data = fs::read(bundle_path.join("config.json")).map_err(|e| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("bundle config not found: {}", e),
        )
    })?;
    let mut spec: OciSpec =
        parse_oci_spec(&config_data).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    apply_run_overrides(&mut spec, &run_opts, &cmd_args);

    // Generate container ID
    let container_id = run_opts
        .name
        .unwrap_or_else(|| generate_container_id(&image));

    // Set bundle rootfs path
    if let Some(ref mut root) = spec.root {
        root.path = bundle_path.join("rootfs").to_string_lossy().to_string();
    }

    // Validate
    validate_spec(&spec)?;
    let state_dir_for_cleanup = crate::state::container_state_dir(&container_id);

    // === CREATE phase ===
    run_prestart_hooks(&spec, &container_id)?;
    run_create_runtime_hooks(&spec, &container_id)?;

    let mut terminal_sockets = [-1i32; 2];
    let terminal_socket_fd = if run_opts.tty {
        let ret = unsafe {
            libc::socketpair(
                libc::AF_UNIX,
                libc::SOCK_STREAM,
                0,
                terminal_sockets.as_mut_ptr(),
            )
        };
        if ret != 0 {
            return Err(io::Error::last_os_error());
        }
        Some(terminal_sockets[1])
    } else {
        None
    };

    let stdio_restore = if run_opts.detach {
        Some(redirect_stdio_for_detach(&state_dir_for_cleanup)?)
    } else {
        None
    };

    let forked =
        fork_container_child_with_terminal_socket(&spec, &container_id, terminal_socket_fd)?;
    drop(stdio_restore);
    if run_opts.tty {
        unsafe { libc::close(terminal_sockets[1]) };
    }
    let child_pid = forked.pid();

    save_created_state(
        &spec,
        &container_id,
        child_pid,
        &bundle_path.to_string_lossy(),
    )?;

    // === START phase ===
    if let Some(ref linux) = spec.linux {
        if let Some(ref resources) = linux.resources {
            let raw_cgroup = linux.cgroups_path.as_deref().unwrap_or("");
            let rootless = !crate::state::is_root();
            let cgroup_path = crate::rootless::resolve_container_cgroup_path(rootless, raw_cgroup)
                .unwrap_or_else(|_| raw_cgroup.to_string());
            setup_container_cgroups(child_pid, resources, &cgroup_path);
        }
    }

    signal_start(&container_id)?;
    run_poststart_hooks(&spec, &container_id, child_pid)?;
    update_state_running(&container_id, child_pid)?;

    if run_opts.detach {
        println!("{}", container_id);
        return Ok(());
    }

    // === WAIT phase ===
    let exit_code = if run_opts.tty {
        let pty_master = recv_fd(terminal_sockets[0])
            .map_err(|error| io::Error::other(format!("failed to receive PTY: {error}")))?;
        unsafe { libc::close(terminal_sockets[0]) };
        let code = relay_pty_until_exit(pty_master, child_pid as i32, run_opts.interactive)?;
        unsafe { libc::close(pty_master) };
        code
    } else {
        wait_for_exit_code(child_pid as i32)?
    };

    // === CLEANUP phase ===
    if run_opts.rm {
        if let Ok(state) = load_state(&container_id) {
            if let Some(ref linux) = spec.linux {
                let raw_cgroup = linux.cgroups_path.as_deref().unwrap_or("");
                let rootless = !crate::state::is_root();
                let cgroup_path =
                    crate::rootless::resolve_container_cgroup_path(rootless, raw_cgroup)
                        .unwrap_or_else(|_| raw_cgroup.to_string());
                crate::lifecycle::run_poststop_and_cleanup(
                    &container_id,
                    state.pid.unwrap_or(0),
                    &state.bundle,
                    &cgroup_path,
                    &spec,
                );
            }
        }
        delete_state(&container_id);
        let _ = fs::remove_dir_all(&state_dir_for_cleanup);
    }

    std::process::exit(exit_code as i32);
}

fn wait_for_exit_code(child_pid: i32) -> io::Result<i32> {
    let mut status = 0i32;
    let pid = unsafe { libc::waitpid(child_pid, &mut status as *mut i32, 0) };
    if pid < 0 {
        return Err(io::Error::other("waitpid failed"));
    }
    Ok(exit_code_from_status(status))
}

fn exit_code_from_status(status: i32) -> i32 {
    if libc::WIFEXITED(status) {
        libc::WEXITSTATUS(status)
    } else if libc::WIFSIGNALED(status) {
        128 + libc::WTERMSIG(status)
    } else {
        128
    }
}

fn recv_fd(sock_fd: i32) -> io::Result<i32> {
    let mut msg: libc::msghdr = unsafe { std::mem::MaybeUninit::zeroed().assume_init() };
    let cmsg_space = unsafe { libc::CMSG_SPACE(std::mem::size_of::<i32>() as u32) as usize };
    let mut cmsg_buf = vec![0u8; cmsg_space];
    let mut fd_buf = 0i32;
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
            "no PTY control message received",
        ));
    }

    let fd = unsafe { std::ptr::read_unaligned(libc::CMSG_DATA(cmsg) as *const i32) };
    Ok(fd)
}

fn relay_pty_until_exit(pty_master_fd: i32, child_pid: i32, interactive: bool) -> io::Result<i32> {
    let stdin_fd = libc::STDIN_FILENO;
    let has_terminal = unsafe { libc::isatty(stdin_fd) == 1 };
    let mut saved = None;
    if interactive && has_terminal {
        let mut original: libc::termios = unsafe { std::mem::MaybeUninit::zeroed().assume_init() };
        if unsafe { libc::tcgetattr(stdin_fd, &mut original) } == 0 {
            let mut raw = original;
            raw.c_lflag &= !(libc::ECHO | libc::ICANON | libc::ISIG | libc::IEXTEN);
            raw.c_iflag &= !(libc::BRKINT | libc::ICRNL | libc::INPCK | libc::ISTRIP | libc::IXON);
            raw.c_cflag &= !(libc::CSIZE | libc::PARENB);
            raw.c_cflag |= libc::CS8;
            raw.c_cc[libc::VMIN] = 1;
            raw.c_cc[libc::VTIME] = 0;
            let _ = unsafe { libc::tcsetattr(stdin_fd, libc::TCSAFLUSH, &raw) };
            saved = Some(original);
        }
    }

    struct TerminalRestore(Option<libc::termios>);
    impl Drop for TerminalRestore {
        fn drop(&mut self) {
            if let Some(termios) = self.0 {
                let _ = unsafe { libc::tcsetattr(libc::STDIN_FILENO, libc::TCSAFLUSH, &termios) };
            }
        }
    }
    let _restore = TerminalRestore(saved);

    let mut buf_in = [0u8; 4096];
    let mut buf_out = [0u8; 4096];
    let mut stdin_open = interactive;
    loop {
        let mut fds = [
            libc::pollfd {
                fd: stdin_fd,
                events: if stdin_open { libc::POLLIN } else { 0 },
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
            continue;
        }

        if stdin_open && fds[0].revents & libc::POLLIN != 0 {
            let n = unsafe {
                libc::read(
                    stdin_fd,
                    buf_in.as_mut_ptr() as *mut libc::c_void,
                    buf_in.len(),
                )
            };
            if n > 0 {
                write_all_fd(pty_master_fd, &buf_in[..n as usize]);
            } else {
                stdin_open = false;
                write_all_fd(pty_master_fd, &[4]);
            }
        }

        if fds[1].revents & libc::POLLIN != 0 {
            let n = unsafe {
                libc::read(
                    pty_master_fd,
                    buf_out.as_mut_ptr() as *mut libc::c_void,
                    buf_out.len(),
                )
            };
            if n > 0 {
                write_all_fd(libc::STDOUT_FILENO, &buf_out[..n as usize]);
            }
        }

        let mut status = 0i32;
        let wait = unsafe { libc::waitpid(child_pid, &mut status as *mut i32, libc::WNOHANG) };
        if wait > 0 {
            drain_fd_to_stdout(pty_master_fd, &mut buf_out);
            return Ok(exit_code_from_status(status));
        }
    }
}

fn drain_fd_to_stdout(fd: i32, buffer: &mut [u8]) {
    loop {
        let n = unsafe { libc::read(fd, buffer.as_mut_ptr() as *mut libc::c_void, buffer.len()) };
        if n <= 0 {
            break;
        }
        write_all_fd(libc::STDOUT_FILENO, &buffer[..n as usize]);
    }
}

fn write_all_fd(fd: i32, bytes: &[u8]) {
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

fn parse_run_args(args: &[String]) -> io::Result<(RunOpts, String, Vec<String>)> {
    if args.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Usage: ert run [options] <image> [cmd...]",
        ));
    }

    let mut opts = RunOpts {
        rm: false,
        detach: false,
        name: None,
        env: Vec::new(),
        workdir: None,
        entrypoint: None,
        privileged: false,
        interactive: false,
        tty: false,
        images_dir: default_images_dir(),
        store_path: default_store_dir(),
    };

    let mut image: Option<String> = None;
    let mut cmd_args = Vec::new();
    let mut past_image = false;
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];
        if past_image {
            cmd_args.push(arg.clone());
            i += 1;
            continue;
        }

        match arg.as_str() {
            "--rm" => opts.rm = true,
            "-d" | "--detach" => opts.detach = true,
            "--privileged" => opts.privileged = true,
            "-it" | "-ti" => {
                opts.interactive = true;
                opts.tty = true;
            }
            "-t" | "--tty" => opts.tty = true,
            "-i" | "--interactive" => opts.interactive = true,
            "--name" => {
                if i + 1 < args.len() {
                    opts.name = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--name requires a value",
                    ));
                }
                continue;
            }
            "-e" | "--env" => {
                if i + 1 < args.len() {
                    opts.env.push(args[i + 1].clone());
                    i += 2;
                } else {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "-e requires a value",
                    ));
                }
                continue;
            }
            "-w" | "--workdir" => {
                if i + 1 < args.len() {
                    opts.workdir = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "-w requires a value",
                    ));
                }
                continue;
            }
            "--entrypoint" => {
                if i + 1 < args.len() {
                    opts.entrypoint = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--entrypoint requires a value",
                    ));
                }
                continue;
            }
            "--images-dir" => {
                if i + 1 < args.len() {
                    opts.images_dir = PathBuf::from(&args[i + 1]);
                    i += 2;
                } else {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--images-dir requires a path",
                    ));
                }
                continue;
            }
            "--store" => {
                if i + 1 < args.len() {
                    opts.store_path = PathBuf::from(&args[i + 1]);
                    i += 2;
                } else {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--store requires a path",
                    ));
                }
                continue;
            }
            "--" => {
                past_image = true;
                i += 1;
                continue;
            }
            _ if arg.starts_with('-') => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("unknown flag: {}", arg),
                ));
            }
            _ => {
                image = Some(arg.clone());
                past_image = true;
            }
        }
        i += 1;
    }

    let image =
        image.ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "image is required"))?;

    Ok((opts, image, cmd_args))
}

struct StdioRestore {
    stdin_fd: i32,
    stdout_fd: i32,
    stderr_fd: i32,
    sighup_handler: libc::sighandler_t,
}

impl Drop for StdioRestore {
    fn drop(&mut self) {
        unsafe {
            libc::dup2(self.stdin_fd, libc::STDIN_FILENO);
            libc::dup2(self.stdout_fd, libc::STDOUT_FILENO);
            libc::dup2(self.stderr_fd, libc::STDERR_FILENO);
            libc::close(self.stdin_fd);
            libc::close(self.stdout_fd);
            libc::close(self.stderr_fd);
            libc::signal(libc::SIGHUP, self.sighup_handler);
        }
    }
}

fn redirect_stdio_for_detach(state_dir: &PathBuf) -> io::Result<StdioRestore> {
    fs::create_dir_all(state_dir)?;
    let stdin_fd = dup_fd(libc::STDIN_FILENO)?;
    let stdout_fd = dup_fd(libc::STDOUT_FILENO)?;
    let stderr_fd = dup_fd(libc::STDERR_FILENO)?;
    let restore = StdioRestore {
        stdin_fd,
        stdout_fd,
        stderr_fd,
        sighup_handler: unsafe { libc::signal(libc::SIGHUP, libc::SIG_IGN) },
    };

    let null = File::open("/dev/null")?;
    let stdout = open_append_log(state_dir.join("stdout.log"))?;
    let stderr = open_append_log(state_dir.join("stderr.log"))?;
    unsafe {
        if libc::dup2(null.as_raw_fd(), libc::STDIN_FILENO) < 0
            || libc::dup2(stdout.as_raw_fd(), libc::STDOUT_FILENO) < 0
            || libc::dup2(stderr.as_raw_fd(), libc::STDERR_FILENO) < 0
        {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(restore)
}

fn dup_fd(fd: i32) -> io::Result<i32> {
    let ret = unsafe { libc::dup(fd) };
    if ret < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(ret)
    }
}

fn open_append_log(path: PathBuf) -> io::Result<File> {
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
}

fn apply_run_overrides(spec: &mut OciSpec, opts: &RunOpts, cmd_args: &[String]) {
    let process = spec.process.get_or_insert_with(Default::default);

    // Override command / entrypoint
    if let Some(ref ep) = opts.entrypoint {
        let mut full = vec![ep.clone()];
        if !cmd_args.is_empty() {
            full.extend_from_slice(cmd_args);
        } else if let Some(ref rest) = process.args {
            full.extend_from_slice(rest);
        }
        process.args = Some(full);
    } else if !cmd_args.is_empty() {
        process.args = Some(cmd_args.to_vec());
    }

    // Override env
    if !opts.env.is_empty() {
        let mut env = process.env.clone().unwrap_or_default();
        env.extend(opts.env.clone());
        process.env = Some(env);
    }

    // Override workdir
    if let Some(ref wd) = opts.workdir {
        process.cwd = Some(wd.clone());
    }

    // Privileged
    if opts.privileged {
        process.no_new_privileges = Some(false);
    }

    process.terminal = Some(opts.tty);
}

fn generate_container_id(image: &ImageRef) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let short = image
        .repository
        .split('/')
        .next_back()
        .unwrap_or(&image.repository);
    format!("{}-{}-{}", short, image.tag, ts)
}
