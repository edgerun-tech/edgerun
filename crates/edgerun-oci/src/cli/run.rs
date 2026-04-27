//! `ert run` — pull (if needed) + create + start + wait in one command.
//!
//! Usage:
//!   `ert run <image> [cmd...]`
//!   `ert run --rm alpine:latest echo hello`
//!   `ert run --name mycontainer --images-dir /mnt/img nginx:latest`

use crate::prelude::*;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

use crate::cli::resolve_registry_auth;
use crate::cli::GlobalOpts;
use crate::json::{parse_oci_spec, OciSpec};
use crate::lifecycle::{
    fork_container_child, run_create_runtime_hooks, run_poststart_hooks, run_prestart_hooks,
    save_created_state, setup_container_cgroups, signal_start, update_state_running,
};
use crate::process::validate_spec;
use crate::state::{delete_state, load_state};

use crate::ImageRef;
use crate::RegistryClient;

struct RunOpts {
    rm: bool,
    name: Option<String>,
    env: Vec<String>,
    workdir: Option<String>,
    entrypoint: Option<String>,
    privileged: bool,
    images_dir: PathBuf,
    store_path: PathBuf,
}

pub fn cmd_run(_opts: &GlobalOpts, args: &[String]) -> io::Result<()> {
    let (run_opts, image_ref_str, cmd_args) = parse_run_args(args)?;

    let image: ImageRef = image_ref_str
        .parse()
        .map_err(|e: String| io::Error::new(io::ErrorKind::InvalidInput, e))?;

    // Resolve auth
    let auth = resolve_registry_auth(&image.registry)?;

    // Pull if not already local
    let bundle_path = run_opts.images_dir.join(&image.repository).join(&image.tag);

    if !bundle_path.join("config.json").exists() {
        eprintln!("Pulling {}...", image_ref_str);
        let rt = edgerun_bare_rt::Runtime::new_multi_thread()
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

    // === CREATE phase ===
    run_prestart_hooks(&spec, &container_id)?;
    run_create_runtime_hooks(&spec, &container_id)?;

    let forked = fork_container_child(&spec, &container_id)?;
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

    // === WAIT phase ===
    let mut status = 0i32;
    let pid = unsafe { libc::waitpid(child_pid as i32, &mut status as *mut i32, 0) };
    if pid < 0 {
        return Err(io::Error::other("waitpid failed"));
    }

    let exit_code = if libc::WIFEXITED(status) {
        libc::WEXITSTATUS(status)
    } else {
        128
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
    }

    std::process::exit(exit_code as i32);
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
        name: None,
        env: Vec::new(),
        workdir: None,
        entrypoint: None,
        privileged: false,
        images_dir: PathBuf::from("/var/lib/edgerun/images"),
        store_path: PathBuf::from("/var/lib/edgerun/store"),
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
            "--privileged" => opts.privileged = true,
            "-t" | "--tty" => {}
            "-i" | "--interactive" => {}
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

    // Always use TTY for run
    process.terminal = Some(true);
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
