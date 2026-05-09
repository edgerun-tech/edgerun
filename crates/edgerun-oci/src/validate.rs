//! no_std OCI spec validation helpers.

pub use edgerun_protocols::oci::{
    DEFAULT_ARGS, DEFAULT_ENV, OciValidationError, default_process_args, default_process_env,
    validate_spec,
};

/// Get the target OS string (e.g. "linux").
pub fn host_os() -> &'static str {
    if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "solaris") {
        "solaris"
    } else {
        "unknown"
    }
}

/// Get the target architecture string (e.g. "amd64").
pub fn host_arch() -> &'static str {
    if cfg!(target_arch = "x86_64") {
        "amd64"
    } else if cfg!(target_arch = "aarch64") {
        "arm64"
    } else if cfg!(target_arch = "riscv64") {
        "riscv64"
    } else if cfg!(target_arch = "arm") {
        "arm"
    } else {
        "unknown"
    }
}
