//! `ert run` — pull (if needed) + create + start + wait in one command.
//!
//! Usage:
//!   `ert run <image> [cmd...]`
//!   `ert run --rm alpine:latest echo hello`
//!   `ert run --name mycontainer --images-dir /mnt/img nginx:latest`

use crate::prelude::*;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{symlink, FileTypeExt, MetadataExt, PermissionsExt};
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};

use crate::cli::pull::print_pull_progress;
use crate::cli::{default_images_dir, default_store_dir, resolve_registry_auth, GlobalOpts};
use crate::json::{parse_oci_spec, OciMount, OciSpec, OciUser};
use crate::lifecycle::{
    fork_container_child_with_terminal_socket, run_create_runtime_hooks, run_poststart_hooks,
    run_prestart_hooks, save_created_state, setup_container_cgroups, signal_start,
    update_state_running,
};
use crate::process::validate_spec;
use crate::state::{delete_state, load_state};

use crate::ImageRef;
use crate::RegistryAuth;
use crate::RegistryClient;

struct RunOpts {
    rm: bool,
    detach: bool,
    name: Option<String>,
    env: Vec<String>,
    env_files: Vec<PathBuf>,
    user: Option<String>,
    workdir: Option<String>,
    entrypoint: Option<String>,
    hostname: Option<String>,
    dns: Vec<String>,
    add_hosts: Vec<(String, String)>,
    privileged: bool,
    interactive: bool,
    tty: bool,
    mounts: Vec<OciMount>,
    pull_policy: PullPolicy,
    images_dir: PathBuf,
    store_path: PathBuf,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PullPolicy {
    Missing,
    Always,
    Never,
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

    let has_local_image = bundle_path.join("config.json").exists();
    match run_opts.pull_policy {
        PullPolicy::Never if !has_local_image => {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("image not found locally and --pull=never was set: {image_ref_str}"),
            ));
        }
        PullPolicy::Always => {
            pull_image(
                &image_ref_str,
                &image,
                &bundle_path,
                &run_opts.store_path,
                auth,
            )?;
        }
        PullPolicy::Missing if !has_local_image => {
            pull_image(
                &image_ref_str,
                &image,
                &bundle_path,
                &run_opts.store_path,
                auth,
            )?;
        }
        PullPolicy::Missing | PullPolicy::Never => {
            eprintln!("Using local image {image_ref_str}");
        }
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

    apply_run_overrides(&mut spec, &run_opts, &cmd_args)?;

    // Generate container ID
    let container_id = run_opts
        .name
        .unwrap_or_else(|| generate_container_id(&image));

    let state_dir_for_cleanup = crate::state::container_state_dir(&container_id);
    let image_rootfs = bundle_path.join("rootfs");
    let container_rootfs = state_dir_for_cleanup.join("rootfs");
    if container_rootfs.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("container {} already has a rootfs", container_id),
        ));
    }
    copy_rootfs_tree(&image_rootfs, &container_rootfs)?;
    write_container_network_files(
        &container_rootfs,
        run_opts.hostname.as_deref().unwrap_or("edgerun"),
        &run_opts.dns,
        &run_opts.add_hosts,
    )?;
    apply_user_override(&mut spec, &container_rootfs, run_opts.user.as_deref())?;

    // Set per-container rootfs path. The pulled image rootfs is immutable input;
    // runtime writes belong to this container state directory.
    if let Some(ref mut root) = spec.root {
        root.path = container_rootfs.to_string_lossy().to_string();
    }

    // Validate
    validate_spec(&spec)?;

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

fn pull_image(
    image_ref: &str,
    image: &ImageRef,
    bundle_path: &Path,
    store_path: &Path,
    auth: RegistryAuth,
) -> io::Result<()> {
    eprintln!("Pulling {image_ref}...");
    eprintln!("  bundle: {}", bundle_path.display());
    eprintln!("  store:  {}", store_path.display());
    let rt = edgerun_rt::Runtime::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|e| io::Error::other(e.to_string()))?;

    let image = image.clone();
    let bundle_path = bundle_path.to_path_buf();
    let store_path = store_path.to_path_buf();
    let mut client = RegistryClient::new().with_auth(auth);
    let result = rt.block_on(async move {
        client
            .pull_with_progress(&image, &bundle_path, &store_path, |event| {
                print_pull_progress(event);
            })
            .await
    });

    match result {
        Ok(report) => {
            eprintln!(
                "Done: {image_ref} -> {} ({} layer{})",
                report.path.display(),
                report.layers,
                if report.layers == 1 { "" } else { "s" }
            );
            Ok(())
        }
        Err(e) => Err(io::Error::other(format!("pull failed: {e}"))),
    }
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
        env_files: Vec::new(),
        user: None,
        workdir: None,
        entrypoint: None,
        hostname: None,
        dns: Vec::new(),
        add_hosts: Vec::new(),
        privileged: false,
        interactive: false,
        tty: false,
        mounts: Vec::new(),
        pull_policy: PullPolicy::Missing,
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
            "-v" | "--volume" => {
                if i + 1 < args.len() {
                    opts.mounts.push(parse_volume_mount(&args[i + 1])?);
                    i += 2;
                } else {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "-v requires host:container[:options]",
                    ));
                }
                continue;
            }
            "--mount" => {
                if i + 1 < args.len() {
                    opts.mounts.push(parse_mount_arg(&args[i + 1])?);
                    i += 2;
                } else {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--mount requires type=bind,source=...,target=...",
                    ));
                }
                continue;
            }
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
                    opts.env.push(parse_env_assignment(&args[i + 1])?);
                    i += 2;
                } else {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "-e requires a value",
                    ));
                }
                continue;
            }
            "--env-file" => {
                if i + 1 < args.len() {
                    opts.env_files.push(PathBuf::from(&args[i + 1]));
                    i += 2;
                } else {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--env-file requires a path",
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
            "-u" | "--user" => {
                if i + 1 < args.len() {
                    opts.user = Some(validate_user_spec(&args[i + 1])?);
                    i += 2;
                } else {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--user requires user[:group]",
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
            "--pull" => {
                if i + 1 < args.len() {
                    opts.pull_policy = parse_pull_policy(&args[i + 1])?;
                    i += 2;
                } else {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--pull requires always, missing, or never",
                    ));
                }
                continue;
            }
            "--hostname" | "-h" => {
                if i + 1 < args.len() {
                    opts.hostname = Some(validate_hostname(&args[i + 1])?);
                    i += 2;
                } else {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--hostname requires a value",
                    ));
                }
                continue;
            }
            "--dns" => {
                if i + 1 < args.len() {
                    opts.dns.push(validate_dns_server(&args[i + 1])?);
                    i += 2;
                } else {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--dns requires an address",
                    ));
                }
                continue;
            }
            "--add-host" => {
                if i + 1 < args.len() {
                    opts.add_hosts.push(parse_add_host(&args[i + 1])?);
                    i += 2;
                } else {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--add-host requires host:ip",
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
            arg if arg.starts_with("--mount=") => {
                opts.mounts.push(parse_mount_arg(&arg["--mount=".len()..])?);
            }
            arg if arg.starts_with("--hostname=") => {
                opts.hostname = Some(validate_hostname(&arg["--hostname=".len()..])?);
            }
            arg if arg.starts_with("--dns=") => {
                opts.dns.push(validate_dns_server(&arg["--dns=".len()..])?);
            }
            arg if arg.starts_with("--add-host=") => {
                opts.add_hosts
                    .push(parse_add_host(&arg["--add-host=".len()..])?);
            }
            arg if arg.starts_with("--env-file=") => {
                opts.env_files
                    .push(PathBuf::from(&arg["--env-file=".len()..]));
            }
            arg if arg.starts_with("--user=") => {
                opts.user = Some(validate_user_spec(&arg["--user=".len()..])?);
            }
            arg if arg.starts_with("--pull=") => {
                opts.pull_policy = parse_pull_policy(&arg["--pull=".len()..])?;
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

fn parse_volume_mount(value: &str) -> io::Result<OciMount> {
    let parts = value.split(':').collect::<Vec<_>>();
    if parts.len() < 2 || parts.len() > 3 || parts[0].is_empty() || parts[1].is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "volume must be host:container[:ro|rw]",
        ));
    }
    let mut options = vec!["bind".to_string()];
    if parts
        .get(2)
        .is_some_and(|opts| opts.split(',').any(|opt| opt == "ro"))
    {
        options.push("ro".to_string());
    } else {
        options.push("rw".to_string());
    }
    bind_mount(parts[0], parts[1], options)
}

fn parse_pull_policy(value: &str) -> io::Result<PullPolicy> {
    match value {
        "missing" | "if-missing" => Ok(PullPolicy::Missing),
        "always" => Ok(PullPolicy::Always),
        "never" => Ok(PullPolicy::Never),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "--pull must be always, missing, or never",
        )),
    }
}

fn parse_mount_arg(value: &str) -> io::Result<OciMount> {
    let mut mount_type = None::<&str>;
    let mut source = None::<&str>;
    let mut target = None::<&str>;
    let mut readonly = false;
    for item in value.split(',') {
        let Some((key, val)) = item.split_once('=') else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "mount entries must be key=value",
            ));
        };
        match key {
            "type" => mount_type = Some(val),
            "source" | "src" => source = Some(val),
            "target" | "dst" | "destination" => target = Some(val),
            "readonly" | "ro" => readonly = matches!(val, "1" | "true" | "yes"),
            _ => {}
        }
    }
    if mount_type != Some("bind") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "only bind mounts are supported by ert run --mount today",
        ));
    }
    let source = source
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "--mount requires source"))?;
    let target = target
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "--mount requires target"))?;
    let mut options = vec!["bind".to_string()];
    options.push(if readonly { "ro" } else { "rw" }.to_string());
    bind_mount(source, target, options)
}

fn bind_mount(source: &str, destination: &str, options: Vec<String>) -> io::Result<OciMount> {
    let source_path = std::path::Path::new(source);
    if !source_path.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "bind mount source must be an absolute host path",
        ));
    }
    if !source_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("bind mount source does not exist: {source}"),
        ));
    }
    if !std::path::Path::new(destination).is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "bind mount destination must be absolute",
        ));
    }
    Ok(OciMount {
        destination: destination.to_string(),
        mount_type: Some("bind".to_string()),
        source: Some(source.to_string()),
        options: Some(options),
        label: None,
        recursive: None,
        uid_mappings: None,
        gid_mappings: None,
    })
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
    OpenOptions::new().create(true).append(true).open(path)
}

fn write_container_network_files(
    rootfs: &Path,
    hostname: &str,
    dns: &[String],
    add_hosts: &[(String, String)],
) -> io::Result<()> {
    let etc = rootfs.join("etc");
    fs::create_dir_all(&etc)?;
    fs::write(etc.join("hostname"), format!("{hostname}\n"))?;
    fs::write(etc.join("hosts"), hosts_file(hostname, add_hosts))?;
    fs::write(etc.join("resolv.conf"), resolv_conf(dns))?;
    Ok(())
}

fn hosts_file(hostname: &str, add_hosts: &[(String, String)]) -> String {
    let mut out = String::new();
    out.push_str("127.0.0.1\tlocalhost\n");
    out.push_str("::1\tlocalhost ip6-localhost ip6-loopback\n");
    out.push_str("127.0.1.1\t");
    out.push_str(hostname);
    out.push('\n');
    for (host, ip) in add_hosts {
        out.push_str(ip);
        out.push('\t');
        out.push_str(host);
        out.push('\n');
    }
    out
}

fn resolv_conf(dns: &[String]) -> String {
    if dns.is_empty() {
        if let Ok(host_resolv) = fs::read_to_string("/etc/resolv.conf") {
            let filtered = host_resolv
                .lines()
                .filter(|line| {
                    let trimmed = line.trim_start();
                    trimmed.starts_with("nameserver")
                        || trimmed.starts_with("search")
                        || trimmed.starts_with("options")
                })
                .collect::<Vec<_>>()
                .join("\n");
            if !filtered.is_empty() {
                return format!("{filtered}\n");
            }
        }
        return "nameserver 1.1.1.1\nnameserver 8.8.8.8\n".to_string();
    }
    let mut out = String::new();
    for server in dns {
        out.push_str("nameserver ");
        out.push_str(server);
        out.push('\n');
    }
    out
}

fn validate_hostname(value: &str) -> io::Result<String> {
    if value.is_empty()
        || value.len() > 253
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'.')
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "hostname must contain only letters, digits, '-' or '.'",
        ));
    }
    Ok(value.to_string())
}

fn validate_dns_server(value: &str) -> io::Result<String> {
    if value.is_empty()
        || value.len() > 255
        || !value
            .bytes()
            .all(|b| b.is_ascii_hexdigit() || b == b'.' || b == b':')
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "dns server must be an IPv4 or IPv6 address",
        ));
    }
    Ok(value.to_string())
}

fn parse_add_host(value: &str) -> io::Result<(String, String)> {
    let (host, ip) = value.rsplit_once(':').ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "--add-host requires host:ip")
    })?;
    let host = validate_hostname(host)?;
    let ip = validate_dns_server(ip)?;
    Ok((host, ip))
}

fn parse_env_assignment(value: &str) -> io::Result<String> {
    let Some((key, _)) = value.split_once('=') else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "environment entries must be KEY=VALUE",
        ));
    };
    validate_env_key(key)?;
    Ok(value.to_string())
}

fn validate_env_key(key: &str) -> io::Result<()> {
    let mut bytes = key.bytes();
    let Some(first) = bytes.next() else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "environment key must not be empty",
        ));
    };
    if !(first.is_ascii_alphabetic() || first == b'_') {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "environment key must start with a letter or '_'",
        ));
    }
    if !bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_') {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "environment key must contain only letters, digits, and '_'",
        ));
    }
    Ok(())
}

fn read_env_file(path: &Path) -> io::Result<Vec<String>> {
    let data = fs::read_to_string(path)?;
    let mut out = Vec::new();
    for (index, line) in data.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("export ").unwrap_or(line);
        let parsed = parse_env_assignment(line).map_err(|error| {
            io::Error::new(
                error.kind(),
                format!("{}:{}: {}", path.display(), index + 1, error),
            )
        })?;
        out.push(parsed);
    }
    Ok(out)
}

fn validate_user_spec(value: &str) -> io::Result<String> {
    let mut parts = value.split(':');
    parts
        .next()
        .filter(|part| !part.is_empty())
        .ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "--user requires user[:group]")
        })?;
    let _group = parts.next();
    if parts.next().is_some() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "--user requires user[:group]",
        ));
    }
    Ok(value.to_string())
}

fn apply_user_override(spec: &mut OciSpec, rootfs: &Path, value: Option<&str>) -> io::Result<()> {
    let Some(value) = value else {
        return Ok(());
    };
    let resolved = resolve_user(rootfs, value)?;
    let process = spec.process.get_or_insert_with(Default::default);
    let existing = process.user.get_or_insert_with(Default::default);
    existing.uid = resolved.uid;
    existing.gid = resolved.gid;
    Ok(())
}

fn resolve_user(rootfs: &Path, value: &str) -> io::Result<OciUser> {
    let mut parts = value.split(':');
    let user_part = parts.next().unwrap_or_default();
    let group_part = parts.next();

    let passwd = read_passwd(rootfs)?;
    let groups = read_group(rootfs)?;
    let (uid, default_gid) = resolve_uid(user_part, &passwd)?;
    let gid = match group_part {
        Some("") => None,
        Some(group) => Some(resolve_gid(group, &groups)?),
        None => default_gid,
    };
    Ok(OciUser {
        uid: Some(uid),
        gid,
        additional_gids: None,
        umask: None,
    })
}

#[derive(Debug)]
struct PasswdEntry {
    name: String,
    uid: u32,
    gid: u32,
}

#[derive(Debug)]
struct GroupEntry {
    name: String,
    gid: u32,
}

fn read_passwd(rootfs: &Path) -> io::Result<Vec<PasswdEntry>> {
    let path = rootfs.join("etc/passwd");
    let data = match fs::read_to_string(&path) {
        Ok(data) => data,
        Err(error) if error.kind() == io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(error),
    };
    let mut entries = Vec::new();
    for line in data.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields = line.split(':').collect::<Vec<_>>();
        if fields.len() < 4 {
            continue;
        }
        let Ok(uid) = fields[2].parse::<u32>() else {
            continue;
        };
        let Ok(gid) = fields[3].parse::<u32>() else {
            continue;
        };
        entries.push(PasswdEntry {
            name: fields[0].to_string(),
            uid,
            gid,
        });
    }
    Ok(entries)
}

fn read_group(rootfs: &Path) -> io::Result<Vec<GroupEntry>> {
    let path = rootfs.join("etc/group");
    let data = match fs::read_to_string(&path) {
        Ok(data) => data,
        Err(error) if error.kind() == io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(error),
    };
    let mut entries = Vec::new();
    for line in data.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields = line.split(':').collect::<Vec<_>>();
        if fields.len() < 3 {
            continue;
        }
        let Ok(gid) = fields[2].parse::<u32>() else {
            continue;
        };
        entries.push(GroupEntry {
            name: fields[0].to_string(),
            gid,
        });
    }
    Ok(entries)
}

fn resolve_uid(value: &str, passwd: &[PasswdEntry]) -> io::Result<(u32, Option<u32>)> {
    if let Some(uid) = parse_numeric_id(value)? {
        return Ok((uid, None));
    }
    passwd
        .iter()
        .find(|entry| entry.name == value)
        .map(|entry| (entry.uid, Some(entry.gid)))
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("user not found in image: {value}"),
            )
        })
}

fn resolve_gid(value: &str, groups: &[GroupEntry]) -> io::Result<u32> {
    if let Some(gid) = parse_numeric_id(value)? {
        return Ok(gid);
    }
    groups
        .iter()
        .find(|entry| entry.name == value)
        .map(|entry| entry.gid)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("group not found in image: {value}"),
            )
        })
}

fn parse_numeric_id(value: &str) -> io::Result<Option<u32>> {
    if value.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "--user requires user[:group]",
        ));
    }
    if value.bytes().all(|b| b.is_ascii_digit()) {
        return value.parse::<u32>().map(Some).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("numeric id is out of range: {value}"),
            )
        });
    }
    if value.bytes().next().is_some_and(|b| b.is_ascii_digit()) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid numeric id: {value}"),
        ));
    }
    if !value
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'-'))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid user or group name: {value}"),
        ));
    }
    Ok(None)
}

fn copy_rootfs_tree(src: &Path, dest: &Path) -> io::Result<()> {
    if !src.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("image rootfs not found: {}", src.display()),
        ));
    }
    fs::create_dir_all(dest)?;
    copy_dir_contents(src, dest)
}

fn copy_dir_contents(src: &Path, dest: &Path) -> io::Result<()> {
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        copy_rootfs_entry(&src_path, &dest_path)?;
    }
    Ok(())
}

fn copy_rootfs_entry(src: &Path, dest: &Path) -> io::Result<()> {
    let metadata = fs::symlink_metadata(src)?;
    let file_type = metadata.file_type();
    if file_type.is_dir() {
        fs::create_dir_all(dest)?;
        copy_dir_contents(src, dest)?;
        fs::set_permissions(
            dest,
            fs::Permissions::from_mode(metadata.permissions().mode()),
        )?;
    } else if file_type.is_symlink() {
        let target = fs::read_link(src)?;
        let _ = fs::remove_file(dest);
        symlink(target, dest)?;
    } else if file_type.is_file() {
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(src, dest)?;
        fs::set_permissions(
            dest,
            fs::Permissions::from_mode(metadata.permissions().mode()),
        )?;
    } else if file_type.is_fifo() {
        create_special_file(dest, libc::S_IFIFO, metadata.permissions().mode(), 0)?;
    } else if file_type.is_char_device() || file_type.is_block_device() {
        let kind = if file_type.is_char_device() {
            libc::S_IFCHR
        } else {
            libc::S_IFBLK
        };
        create_special_file(dest, kind, metadata.permissions().mode(), metadata.rdev())?;
    } else {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            format!("unsupported rootfs entry type: {}", src.display()),
        ));
    }
    Ok(())
}

fn create_special_file(dest: &Path, kind: libc::mode_t, mode: u32, dev: u64) -> io::Result<()> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    let path = std::ffi::CString::new(dest.as_os_str().as_bytes())
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
    let ret = unsafe { libc::mknod(path.as_ptr(), kind | (mode as libc::mode_t), dev) };
    if ret == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

fn apply_run_overrides(spec: &mut OciSpec, opts: &RunOpts, cmd_args: &[String]) -> io::Result<()> {
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
    if !opts.env_files.is_empty() || !opts.env.is_empty() {
        let mut env = process.env.clone().unwrap_or_default();
        for path in &opts.env_files {
            env.extend(read_env_file(path)?);
        }
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
    if let Some(ref hostname) = opts.hostname {
        spec.hostname = Some(hostname.clone());
    } else if spec.hostname.is_none() {
        spec.hostname = Some("edgerun".to_string());
    }
    if !opts.dns.is_empty() || !opts.add_hosts.is_empty() {
        let annotations = spec.annotations.get_or_insert_with(Default::default);
        if !opts.dns.is_empty() {
            annotations.insert("run.edgerun.io/dns".to_string(), opts.dns.join(","));
        }
        if !opts.add_hosts.is_empty() {
            annotations.insert(
                "run.edgerun.io/add-host".to_string(),
                opts.add_hosts
                    .iter()
                    .map(|(host, ip)| format!("{host}:{ip}"))
                    .collect::<Vec<_>>()
                    .join(","),
            );
        }
    }

    if !opts.mounts.is_empty() {
        let mounts = spec.mounts.get_or_insert_with(Vec::new);
        mounts.extend(opts.mounts.clone());
    }
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn validates_user_spec_shape() {
        assert!(validate_user_spec("1000").is_ok());
        assert!(validate_user_spec("nobody:nobody").is_ok());
        assert!(validate_user_spec("").is_err());
        assert!(validate_user_spec(":group").is_err());
        assert!(validate_user_spec("user:group:extra").is_err());
    }

    #[test]
    fn parses_pull_policy() {
        assert_eq!(parse_pull_policy("missing").unwrap(), PullPolicy::Missing);
        assert_eq!(
            parse_pull_policy("if-missing").unwrap(),
            PullPolicy::Missing
        );
        assert_eq!(parse_pull_policy("always").unwrap(), PullPolicy::Always);
        assert_eq!(parse_pull_policy("never").unwrap(), PullPolicy::Never);
        assert!(parse_pull_policy("sometimes").is_err());
    }

    #[test]
    fn resolves_image_user_and_group_names() {
        let root = test_rootfs("edgerun-oci-user");
        let etc = root.join("etc");
        fs::create_dir_all(&etc).unwrap();
        fs::write(
            etc.join("passwd"),
            "root:x:0:0:root:/root:/bin/sh\nnobody:x:65534:65534:nobody:/nonexistent:/sbin/nologin\n",
        )
        .unwrap();
        fs::write(
            etc.join("group"),
            "root:x:0:\nnobody:x:65534:\napp:x:1000:\n",
        )
        .unwrap();

        let user = resolve_user(&root, "nobody").unwrap();
        assert_eq!(user.uid, Some(65534));
        assert_eq!(user.gid, Some(65534));

        let user = resolve_user(&root, "1000:nobody").unwrap();
        assert_eq!(user.uid, Some(1000));
        assert_eq!(user.gid, Some(65534));

        assert!(resolve_user(&root, "missing").is_err());
        assert!(resolve_user(&root, "nobody:missing").is_err());

        fs::remove_dir_all(root).unwrap();
    }

    fn test_rootfs(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{nanos}"))
    }
}
