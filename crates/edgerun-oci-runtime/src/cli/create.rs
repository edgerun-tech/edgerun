//! Create command implementation.
//!
//! Sets up a container in the 'created' state, ready to be started.

use std::fs;
use std::io;
use std::process::Command;

use crate::cli::GlobalOpts;
use crate::json::{OciSpec, parse_oci_spec};
use crate::process::{CloneChildData, cloned_child_main};
use crate::state::{ContainerState, save_state, fifo_path, container_state_dir, state_file_path};

/// Retry reading a file that may still be being written to.
fn retry_read(path: &std::path::Path, max_retries: u32) -> io::Result<Vec<u8>> {
    for i in 0..max_retries {
        if i > 0 {
            std::thread::sleep(std::time::Duration::from_millis(100 * i as u64));
        }
        let data = fs::read(path)?;
        let trimmed = data.iter()
            .skip_while(|b| **b == b' ' || **b == b'\t' || **b == b'\n' || **b == b'\r')
            .copied()
            .collect::<Vec<_>>();
        if !trimmed.is_empty() && (trimmed[0] == b'{' || trimmed[0] == b'[') {
            return Ok(trimmed);
        }
    }
    fs::read(path)
}

/// Trampoline that calls cloned_child_main.
extern "C" fn clone_trampoline(data: *mut libc::c_void) -> libc::c_int {
    let data = unsafe { Box::from_raw(data as *mut CloneChildData) };
    cloned_child_main(*data) as libc::c_int
}

pub fn cmd_create(opts: &GlobalOpts, args: &[String]) -> io::Result<()> {
    let bundle = opts.bundle.as_deref().unwrap_or(std::path::Path::new("."));
    let id = crate::cli::require_container_id(args)?;

    // Check for duplicate ID
    if state_file_path(id).exists() {
        return Err(io::Error::new(io::ErrorKind::AlreadyExists,
            format!("container ID {} already exists", id)));
    }

    let config_path = bundle.join("config.json");
    let config_data = retry_read(&config_path, 5)?;
    let spec: OciSpec = parse_oci_spec(&config_data)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("invalid OCI config: {}", e)))?;

    // Create the FIFO for start synchronization
    let dir = container_state_dir(id);
    fs::create_dir_all(&dir)?;
    let fifo = fifo_path(id);
    let _ = fs::remove_file(&fifo);
    let mkfifo_output = Command::new("mkfifo").arg(&fifo).output()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("mkfifo failed: {}", e)))?;
    if !mkfifo_output.status.success() {
        return Err(io::Error::new(io::ErrorKind::Other,
            format!("mkfifo failed: {}", String::from_utf8_lossy(&mkfifo_output.stderr))));
    }

    // Calculate namespace flags
    let linux = spec.linux.clone().unwrap_or_default();
    let ns_list = linux.namespaces.clone().unwrap_or_else(crate::default_namespaces);
    let ns_flags = crate::namespace_flags(&ns_list);

    // Use clone() with namespace flags
    const STACK_SIZE: usize = 1024 * 1024; // 1 MB stack
    let mut stack: Vec<u8> = vec![0u8; STACK_SIZE];
    let stack_top = unsafe { stack.as_mut_ptr().add(STACK_SIZE) as *mut libc::c_void };

    // Pack data for the child
    let child_data = Box::new(CloneChildData {
        bundle: std::ffi::CString::new(bundle.to_string_lossy().as_bytes()).unwrap(),
        fifo: std::ffi::CString::new(fifo.to_string_lossy().as_bytes()).unwrap(),
        id: std::ffi::CString::new(id.as_bytes()).unwrap(),
    });
    let child_data_ptr = Box::into_raw(child_data);

    let child_pid = unsafe {
        libc::clone(
            clone_trampoline,
            stack_top,
            ns_flags | libc::SIGCHLD,
            child_data_ptr as *mut libc::c_void,
        )
    };
    if child_pid < 0 {
        return Err(io::Error::last_os_error());
    }

    // Write PID to pid-file if requested
    if let Some(ref pid_file) = opts.pid_file {
        fs::write(pid_file, format!("{}", child_pid))?;
    }

    // Save state as 'created'
    let state = ContainerState {
        oci_version: spec.version.clone(),
        id: id.to_string(),
        status: "created".to_string(),
        pid: Some(child_pid as u32),
        bundle: bundle.to_string_lossy().to_string(),
        annotations: spec.annotations.clone(),
    };
    save_state(&state, id)?;
    Ok(())
}
