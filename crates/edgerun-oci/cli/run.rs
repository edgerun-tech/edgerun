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
use std::path::{Path, PathBuf};

use crate::cli::env::{parse_env_assignment, read_env_file};
use crate::cli::pull::print_pull_progress;
use crate::cli::user::{resolve_user, validate_user_spec};
use crate::cli::{
    default_images_dir, default_store_dir, parse_cli_args, resolve_registry_auth, split_cli_prefix,
    GlobalOpts, RUN_VALUE_OPTIONS,
};
use crate::lifecycle::{
    fork_container_child_with_terminal_socket, run_create_runtime_hooks, run_prestart_hooks,
    save_and_start_forked_child,
};
use crate::process::validate_spec;
use crate::rootfs_copy::copy_rootfs_tree;
use crate::spec::{parse_oci_spec, OciMount, OciSpec};
use crate::state::{delete_state, load_state};
use crate::terminal::{recv_fd, relay_pty_until_exit, wait_for_exit_code};

use crate::ImageRef;
use crate::RegistryAuth;
use crate::RegistryClient;
use edgerun_clap::cli::Action;
use edgerun_clap::{Arg, Command};

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
    crate::cli::apply_global_opts(opts)?;

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

fn parse_run_args(args: &[String]) -> io::Result<(RunOpts, String, Vec<String>)> {
    const USAGE: &str = "Usage: ert run [options] <image> [cmd...]";
    let (prefix, image, cmd_args) = split_cli_prefix(args, RUN_VALUE_OPTIONS);
    let matches = parse_cli_args(
        Command::new("run")
            .arg(Arg::new("rm").long("rm").action(Action::StoreTrue))
            .arg(
                Arg::new("detach")
                    .short('d')
                    .long("detach")
                    .action(Action::StoreTrue),
            )
            .arg(
                Arg::new("privileged")
                    .long("privileged")
                    .action(Action::StoreTrue),
            )
            .arg(Arg::new("it").long("it").action(Action::StoreTrue))
            .arg(Arg::new("ti").long("ti").action(Action::StoreTrue))
            .arg(
                Arg::new("tty")
                    .short('t')
                    .long("tty")
                    .action(Action::StoreTrue),
            )
            .arg(
                Arg::new("interactive")
                    .short('i')
                    .long("interactive")
                    .action(Action::StoreTrue),
            )
            .arg(
                Arg::new("volume")
                    .short('v')
                    .long("volume")
                    .action(Action::Append),
            )
            .arg(Arg::new("mount").long("mount").action(Action::Append))
            .arg(Arg::new("name").long("name"))
            .arg(
                Arg::new("env")
                    .short('e')
                    .long("env")
                    .action(Action::Append),
            )
            .arg(Arg::new("env-file").long("env-file").action(Action::Append))
            .arg(Arg::new("workdir").short('w').long("workdir"))
            .arg(Arg::new("user").short('u').long("user"))
            .arg(Arg::new("entrypoint").long("entrypoint"))
            .arg(Arg::new("pull").long("pull"))
            .arg(Arg::new("hostname").short('h').long("hostname"))
            .arg(Arg::new("dns").long("dns").action(Action::Append))
            .arg(Arg::new("add-host").long("add-host").action(Action::Append))
            .arg(Arg::new("images-dir").long("images-dir"))
            .arg(Arg::new("store").long("store")),
        &prefix,
        USAGE,
    )?;
    let mut opts = RunOpts {
        rm: matches.get_flag("rm"),
        detach: matches.get_flag("detach"),
        name: matches.get_one::<String>("name"),
        env: Vec::new(),
        env_files: Vec::new(),
        user: matches
            .get_one::<String>("user")
            .map(|user| validate_user_spec(&user))
            .transpose()?,
        workdir: matches.get_one::<String>("workdir"),
        entrypoint: matches.get_one::<String>("entrypoint"),
        hostname: matches
            .get_one::<String>("hostname")
            .map(|hostname| validate_hostname(&hostname))
            .transpose()?,
        dns: Vec::new(),
        add_hosts: Vec::new(),
        privileged: matches.get_flag("privileged"),
        interactive: matches.get_flag("interactive")
            || matches.get_flag("it")
            || matches.get_flag("ti"),
        tty: matches.get_flag("tty") || matches.get_flag("it") || matches.get_flag("ti"),
        mounts: Vec::new(),
        pull_policy: matches
            .get_one::<String>("pull")
            .map(|pull| parse_pull_policy(&pull))
            .transpose()?
            .unwrap_or(PullPolicy::Missing),
        images_dir: matches
            .get_one::<PathBuf>("images-dir")
            .unwrap_or_else(default_images_dir),
        store_path: matches
            .get_one::<PathBuf>("store")
            .unwrap_or_else(default_store_dir),
    };

    if let Some(volumes) = matches.get_many::<String>("volume") {
        for volume in volumes {
            opts.mounts.push(parse_volume_mount(&volume)?);
        }
    }
    if let Some(mounts) = matches.get_many::<String>("mount") {
        for mount in mounts {
            opts.mounts.push(parse_mount_arg(&mount)?);
        }
    }
    if let Some(env) = matches.get_many::<String>("env") {
        for entry in env {
            opts.env.push(parse_env_assignment(&entry)?);
        }
    }
    if let Some(env_files) = matches.get_many::<PathBuf>("env-file") {
        opts.env_files = env_files;
    }
    if let Some(dns) = matches.get_many::<String>("dns") {
        for server in dns {
            opts.dns.push(validate_dns_server(&server)?);
        }
    }
    if let Some(add_hosts) = matches.get_many::<String>("add-host") {
        for host in add_hosts {
            opts.add_hosts.push(parse_add_host(&host)?);
        }
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
#[path = "../tests/unit_src/src/cli/run_tests.rs"]
mod tests;
