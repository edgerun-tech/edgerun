//! Resume command implementation — unfreezes the container via cgroup v2 cgroup.freeze.

use crate::prelude::*;
use std::fs;
use std::io;

use crate::state::load_state;

pub fn cmd_resume(opts: &crate::cli::GlobalOpts, args: &[String]) -> io::Result<()> {
    crate::cli::apply_global_opts(opts)?;

    let id = crate::cli::parse_container_id_args(args, "resume")?;
    let id = id.as_str();

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

    let bundle = &state.bundle;
    let config_path = std::path::Path::new(bundle).join("config.json");
    let cgroup_path = if let Ok(data) = fs::read(&config_path) {
        if let Ok(spec) = crate::spec::parse_oci_spec(&data) {
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

    let cgroup_dir = crate::cli::cgroup_dir_path(&cgroup_path)?;
    let freeze_file = cgroup_dir.join("cgroup.freeze");

    fs::write(&freeze_file, "0")
        .map_err(|e| io::Error::other(format!("failed to resume container {}: {}", id, e)))
}
