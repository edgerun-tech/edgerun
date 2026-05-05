//! `ert run` — pull (if needed) + create + start + wait in one command.
//!
//! Usage:
//!   `ert run <image> [cmd...]`
//!   `ert run --rm alpine:latest echo hello`
//!   `ert run --name mycontainer --images-dir /mnt/img nginx:latest`

use crate::prelude::*;
use std::fs::{self, File, OpenOptions};
use std::io;
use std::os::unix::io::AsRawFd;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};

use crate::cli::pull::print_pull_progress;
#[cfg(test)]
use crate::cli::run_config::parse_pull_policy;
use crate::cli::run_config::{
    apply_run_overrides, apply_user_override, generate_container_id, parse_run_args,
    write_container_network_files, PullPolicy,
};
use crate::cli::{resolve_registry_auth, GlobalOpts};
use crate::lifecycle::{
    fork_container_child_with_terminal_socket, run_create_runtime_hooks, run_prestart_hooks,
    save_and_start_forked_child,
};
use crate::process::validate_spec;
use crate::rootfs_copy::copy_rootfs_tree;
use crate::spec::{parse_oci_spec, OciSpec};
use crate::state::{delete_state_with_result, load_state};
use crate::terminal::{recv_fd, relay_pty_until_exit, send_fd, wait_for_exit_code};

use crate::ImageRef;
use crate::ImageTrustPolicy;
use crate::RegistryAuth;
use crate::RegistryClient;

pub fn cmd_run(opts: &GlobalOpts, args: &[String]) -> io::Result<()> {
    crate::cli::apply_global_opts(opts)?;

    let (run_opts, image_ref_str, cmd_args) = parse_run_args(args)?;
    let run_pid_file = run_opts.pid_file.clone().or(opts.pid_file.clone());
    if selected_bundle(opts).join("config.json").exists() {
        return run_bundle_compat(opts, run_opts, image_ref_str, cmd_args, run_pid_file);
    }
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
                run_opts.image_trust_policy,
            )?;
        }
        PullPolicy::Missing if !has_local_image => {
            pull_image(
                &image_ref_str,
                &image,
                &bundle_path,
                &run_opts.store_path,
                auth,
                run_opts.image_trust_policy,
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
    if let Some(pid_file) = run_pid_file {
        fs::write(&pid_file, format!("{}", child_pid))?;
    }

    save_and_start_forked_child(
        &spec,
        &container_id,
        child_pid,
        &bundle_path.to_string_lossy(),
    )?;

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
        let mut cleanup_err: Option<io::Error> = None;
        if let Ok(state) = load_state(&container_id) {
            if let Some(ref linux) = spec.linux {
                let cgroup_path = linux.cgroups_path.clone().unwrap_or("/edgerun".into());
                if let Err(e) = crate::lifecycle::run_poststop_and_cleanup(
                    &container_id,
                    state.pid.unwrap_or(0),
                    &state.bundle,
                    &cgroup_path,
                    &spec,
                ) {
                    cleanup_err = Some(e);
                }
            }
        }
        if let Err(e) = delete_state_with_result(&container_id) {
            cleanup_err.get_or_insert(e);
        }

        if let Some(error) = cleanup_err {
            return Err(error);
        }
    }

    std::process::exit(exit_code as i32);
}

fn selected_bundle(opts: &GlobalOpts) -> PathBuf {
    opts.bundle.clone().unwrap_or_else(|| PathBuf::from("."))
}

fn run_bundle_compat(
    opts: &GlobalOpts,
    run_opts: crate::cli::run_config::RunOpts,
    container_id: String,
    cmd_args: Vec<String>,
    pid_file: Option<PathBuf>,
) -> io::Result<()> {
    if !cmd_args.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "bundle mode does not accept command arguments",
        ));
    }
    crate::cli::validate_container_id(&container_id)?;
    if run_opts.detach && run_opts.rm {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "--rm cannot be combined with detached mode yet",
        ));
    }

    let bundle = selected_bundle(opts);
    let bundle_abs = bundle.canonicalize().map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("cannot resolve bundle path: {error}"),
        )
    })?;
    std::env::set_current_dir(&bundle_abs).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("cannot chdir to bundle: {error}"),
        )
    })?;

    let config_data = fs::read("config.json").map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("cannot read config: {error}"),
        )
    })?;
    let spec: OciSpec = parse_oci_spec(&config_data)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;

    validate_spec(&spec)?;
    run_prestart_hooks(&spec, &container_id)?;
    run_create_runtime_hooks(&spec, &container_id)?;

    let terminal = spec
        .process
        .as_ref()
        .and_then(|process| process.terminal)
        .unwrap_or(false);
    let external_console = if terminal {
        run_opts
            .console_socket
            .as_ref()
            .map(UnixStream::connect)
            .transpose()?
    } else {
        None
    };

    let mut terminal_sockets = [-1i32; 2];
    let terminal_socket_fd = if terminal && (external_console.is_some() || !run_opts.detach) {
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
        Some(redirect_stdio_for_detach(
            &crate::state::container_state_dir(&container_id),
        )?)
    } else {
        None
    };

    let forked =
        fork_container_child_with_terminal_socket(&spec, &container_id, terminal_socket_fd)?;
    drop(stdio_restore);
    if terminal && (external_console.is_some() || !run_opts.detach) {
        unsafe { libc::close(terminal_sockets[1]) };
    }
    let child_pid = forked.pid();
    if let Some(pid_file) = pid_file {
        fs::write(&pid_file, format!("{}", child_pid))?;
    }

    save_and_start_forked_child(
        &spec,
        &container_id,
        child_pid,
        &bundle_abs.to_string_lossy(),
    )?;

    if let Some(ref socket) = external_console {
        let pty_master = recv_fd(terminal_sockets[0])
            .map_err(|error| io::Error::other(format!("failed to receive PTY: {error}")))?;
        unsafe { libc::close(terminal_sockets[0]) };
        send_fd(socket.as_raw_fd(), pty_master)
            .map_err(|error| io::Error::other(format!("failed to send PTY: {error}")))?;
        unsafe { libc::close(pty_master) };
    }

    if run_opts.detach {
        return Ok(());
    }

    let exit_code = if terminal && external_console.is_none() {
        let pty_master = recv_fd(terminal_sockets[0])
            .map_err(|error| io::Error::other(format!("failed to receive PTY: {error}")))?;
        unsafe { libc::close(terminal_sockets[0]) };
        let code = relay_pty_until_exit(pty_master, child_pid as i32, true)?;
        unsafe { libc::close(pty_master) };
        code
    } else {
        wait_for_exit_code(child_pid as i32)?
    };

    std::process::exit(exit_code as i32);
}

fn pull_image(
    image_ref: &str,
    image: &ImageRef,
    bundle_path: &Path,
    store_path: &Path,
    auth: RegistryAuth,
    trust_policy: ImageTrustPolicy,
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
    let mut client = RegistryClient::new()
        .with_auth(auth)
        .with_image_trust_policy(trust_policy);
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
