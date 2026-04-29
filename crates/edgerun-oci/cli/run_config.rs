use crate::prelude::*;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::cli::env::{parse_env_assignment, read_env_file};
use crate::cli::user::{resolve_user, validate_user_spec};
use crate::cli::{
    default_images_dir, default_store_dir, parse_cli_args, split_cli_prefix, RUN_VALUE_OPTIONS,
};
use crate::spec::{OciMount, OciSpec};
use crate::ImageTrustPolicy;
use edgerun_clap::cli::Action;
use edgerun_clap::{Arg, Command};

pub(super) struct RunOpts {
    pub rm: bool,
    pub detach: bool,
    pub name: Option<String>,
    pub pid_file: Option<std::path::PathBuf>,
    pub console_socket: Option<String>,
    pub env: Vec<String>,
    pub env_files: Vec<PathBuf>,
    pub user: Option<String>,
    pub workdir: Option<String>,
    pub entrypoint: Option<String>,
    pub hostname: Option<String>,
    pub dns: Vec<String>,
    pub add_hosts: Vec<(String, String)>,
    pub privileged: bool,
    pub interactive: bool,
    pub tty: bool,
    pub mounts: Vec<OciMount>,
    pub pull_policy: PullPolicy,
    pub image_trust_policy: ImageTrustPolicy,
    pub images_dir: PathBuf,
    pub store_path: PathBuf,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum PullPolicy {
    Missing,
    Always,
    Never,
}

pub(super) fn parse_run_args(args: &[String]) -> io::Result<(RunOpts, String, Vec<String>)> {
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
            .arg(Arg::new("pid-file").long("pid-file"))
            .arg(Arg::new("console-socket").long("console-socket"))
            .arg(Arg::new("pull").long("pull"))
            .arg(
                Arg::new("allow-unverified-tags")
                    .long("allow-unverified-tags")
                    .action(Action::StoreTrue),
            )
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
        pid_file: matches.get_one::<PathBuf>("pid-file"),
        console_socket: matches.get_one::<String>("console-socket"),
        name: matches
            .get_one::<String>("name")
            .map(|name| {
                crate::cli::validate_container_id(&name)?;
                Ok::<String, std::io::Error>(name.to_string())
            })
            .transpose()?,
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
        image_trust_policy: if matches.get_flag("allow-unverified-tags") {
            ImageTrustPolicy::AllowTagReference
        } else {
            ImageTrustPolicy::RequireDigestReference
        },
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

pub(super) fn parse_pull_policy(value: &str) -> io::Result<PullPolicy> {
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
    let source_path = Path::new(source);
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
    if !Path::new(destination).is_absolute() {
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

pub(super) fn write_container_network_files(
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

pub(super) fn apply_user_override(
    spec: &mut OciSpec,
    rootfs: &Path,
    value: Option<&str>,
) -> io::Result<()> {
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

pub(super) fn apply_run_overrides(
    spec: &mut OciSpec,
    opts: &RunOpts,
    cmd_args: &[String],
) -> io::Result<()> {
    let process = spec.process.get_or_insert_with(Default::default);

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

    if !opts.env_files.is_empty() || !opts.env.is_empty() {
        let mut env = process.env.clone().unwrap_or_default();
        for path in &opts.env_files {
            env.extend(read_env_file(path)?);
        }
        env.extend(opts.env.clone());
        process.env = Some(env);
    }

    if let Some(ref wd) = opts.workdir {
        process.cwd = Some(wd.clone());
    }

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

pub(super) fn generate_container_id(image: &crate::ImageRef) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis());
    let short = image
        .repository
        .split('/')
        .next_back()
        .unwrap_or(&image.repository);
    format!("{}-{}-{}", short, image.tag, ts)
}
