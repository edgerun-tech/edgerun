//! Pause command implementation — freezes the container via cgroup v2 cgroup.freeze.

use std::fs;
use std::io;

use crate::state::load_state;

pub fn cmd_pause(opts: &crate::cli::GlobalOpts, args: &[String]) -> io::Result<()> {
    if let Some(ref root) = opts.root {
        crate::state::set_state_dir(root.to_str().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "--root path is not valid UTF-8",
            )
        })?);
    }

    let id = crate::cli::require_container_id(args)?;

    let state = load_state(id)?;
    let pid = state
        .pid
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "container has no PID"))?;

    if !crate::cli::is_process_alive(pid) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("container {} is not running", id),
        ));
    }

    // Resolve cgroup path from spec
    let bundle = &state.bundle;
    let config_path = std::path::Path::new(bundle).join("config.json");
    let cgroup_path = if let Ok(data) = fs::read(&config_path) {
        if let Ok(spec) = crate::json::parse_oci_spec(&data) {
            spec.linux
                .as_ref()
                .and_then(|l| l.cgroups_path.as_ref())
                .cloned()
                .unwrap_or_else(|| "/edgerun".into())
        } else {
            "/edgerun".into()
        }
    } else {
        "/edgerun".into()
    };

    let cgroup_dir =
        std::path::Path::new("/sys/fs/cgroup").join(cgroup_path.trim_start_matches('/'));
    let freeze_file = cgroup_dir.join("cgroup.freeze");

    fs::write(&freeze_file, "1")
        .map_err(|e| io::Error::other(format!("failed to freeze container {}: {}", id, e)))
}
