//! Host-independent OCI spec validation helpers.

use super::linux_catalog::{
    is_oci_capability_name, is_oci_namespace_name, is_oci_rlimit_name, is_seccomp_arch_name,
};
use super::runtime_spec::OciSpec;
use alloc::{format, string::String, vec::Vec};
use core::fmt;

/// Default environment variables when none are specified in the OCI spec.
pub const DEFAULT_ENV: &[&str] = &[
    "PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
    "TERM=xterm",
];
pub const DEFAULT_ARGS: &[&str] = &["/bin/sh"];

pub fn default_process_args() -> Vec<String> {
    DEFAULT_ARGS.iter().map(|value| (*value).into()).collect()
}

pub fn default_process_env() -> Vec<String> {
    DEFAULT_ENV.iter().map(|value| (*value).into()).collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OciValidationError {
    message: String,
}

impl OciValidationError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for OciValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl core::error::Error for OciValidationError {}

/// Validate an OCI spec for known-invalid values before boot or container creation.
pub fn validate_spec(spec: &OciSpec) -> Result<(), OciValidationError> {
    if spec.root.is_none() {
        return err("spec missing root");
    }

    let proc = spec
        .process
        .as_ref()
        .ok_or_else(|| OciValidationError::new("spec missing process"))?;

    if let Some(ref args) = proc.args {
        if args.is_empty() {
            return err("process args must not be empty");
        }
        if args[0].is_empty() {
            return err("process executable path must not be empty");
        }
    }

    if let Some(ref caps) = proc.capabilities {
        let cap_sets = [
            caps.bounding.as_ref(),
            caps.effective.as_ref(),
            caps.permitted.as_ref(),
            caps.inheritable.as_ref(),
            caps.ambient.as_ref(),
        ];
        for cap_set in &cap_sets {
            if let Some(caps_list) = cap_set {
                for cap in caps_list.iter() {
                    if !is_oci_capability_name(cap) {
                        return err(format!("unknown capability: {}", cap));
                    }
                }
            }
        }
    }

    if let Some(ref rlimits) = proc.rlimits {
        for rlimit in rlimits.iter() {
            if !is_oci_rlimit_name(&rlimit.ns_type) {
                return err(format!("unknown rlimit type: {}", rlimit.ns_type));
            }
        }
    }

    if let Some(ref linux) = spec.linux {
        if let Some(ref namespaces) = linux.namespaces {
            for ns in namespaces {
                if ns.path.is_none() && !is_oci_namespace_name(&ns.ns_type) {
                    return err(format!("unknown namespace type: {}", ns.ns_type));
                }
            }
        }

        if let Some(ref seccomp) = linux.seccomp {
            if let Some(ref archs) = seccomp.architectures {
                for arch in archs.iter() {
                    if !is_seccomp_arch_name(arch) {
                        return err(format!("unknown seccomp architecture: {}", arch));
                    }
                }
            }
        }
    }

    Ok(())
}

fn err<T>(message: impl Into<String>) -> Result<T, OciValidationError> {
    Err(OciValidationError::new(message))
}
