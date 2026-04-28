//! no_std OCI spec validation helpers.

use crate::prelude::*;
use crate::spec::{OciSeccompAction, OciSpec};
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
                    if !KNOWN_CAPABILITIES.contains(&cap.as_str()) {
                        return err(format!("unknown capability: {}", cap));
                    }
                }
            }
        }
    }

    if let Some(ref rlimits) = proc.rlimits {
        for rlimit in rlimits.iter() {
            if !KNOWN_RLIMITS.contains(&rlimit.ns_type.as_str()) {
                return err(format!("unknown rlimit type: {}", rlimit.ns_type));
            }
        }
    }

    if let Some(ref linux) = spec.linux {
        if let Some(ref namespaces) = linux.namespaces {
            for ns in namespaces {
                if ns.path.is_none() && !KNOWN_NAMESPACES.contains(&ns.ns_type.as_str()) {
                    return err(format!("unknown namespace type: {}", ns.ns_type));
                }
            }
        }

        if let Some(ref seccomp) = linux.seccomp {
            if let Some(ref default_action) = seccomp.default_action {
                let action_str: String = default_action.clone().into();
                if !KNOWN_SECCOMP_ACTIONS.contains(&action_str.as_str()) {
                    return err(format!("unknown seccomp action: {}", action_str));
                }
            }

            if let Some(ref archs) = seccomp.architectures {
                for arch in archs.iter() {
                    if !KNOWN_SECCOMP_ARCHES.contains(&arch.as_str()) {
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

pub(crate) const KNOWN_CAPABILITIES: &[&str] = &[
    "CAP_CHOWN",
    "CAP_DAC_OVERRIDE",
    "CAP_DAC_READ_SEARCH",
    "CAP_FOWNER",
    "CAP_FSETID",
    "CAP_KILL",
    "CAP_SETGID",
    "CAP_SETUID",
    "CAP_SETPCAP",
    "CAP_LINUX_IMMUTABLE",
    "CAP_NET_BIND_SERVICE",
    "CAP_NET_BROADCAST",
    "CAP_NET_ADMIN",
    "CAP_NET_RAW",
    "CAP_IPC_LOCK",
    "CAP_IPC_OWNER",
    "CAP_SYS_MODULE",
    "CAP_SYS_RAWIO",
    "CAP_SYS_CHROOT",
    "CAP_SYS_PTRACE",
    "CAP_SYS_PACCT",
    "CAP_SYS_ADMIN",
    "CAP_SYS_BOOT",
    "CAP_SYS_NICE",
    "CAP_SYS_RESOURCE",
    "CAP_SYS_TIME",
    "CAP_SYS_TTY_CONFIG",
    "CAP_MKNOD",
    "CAP_LEASE",
    "CAP_AUDIT_WRITE",
    "CAP_AUDIT_CONTROL",
    "CAP_SETFCAP",
    "CAP_MAC_OVERRIDE",
    "CAP_MAC_ADMIN",
    "CAP_SYSLOG",
    "CAP_WAKE_ALARM",
    "CAP_BLOCK_SUSPEND",
    "CAP_AUDIT_READ",
    "CAP_PERFMON",
    "CAP_BPF",
    "CAP_CHECKPOINT_RESTORE",
];

pub(crate) const KNOWN_NAMESPACES: &[&str] = &[
    "mount", "pid", "network", "ipc", "uts", "user", "cgroup", "time",
];

pub(crate) const KNOWN_RLIMITS: &[&str] = &[
    "RLIMIT_CPU",
    "RLIMIT_FSIZE",
    "RLIMIT_DATA",
    "RLIMIT_STACK",
    "RLIMIT_CORE",
    "RLIMIT_RSS",
    "RLIMIT_NPROC",
    "RLIMIT_NOFILE",
    "RLIMIT_MEMLOCK",
    "RLIMIT_AS",
    "RLIMIT_LOCKS",
    "RLIMIT_SIGPENDING",
    "RLIMIT_MSGQUEUE",
    "RLIMIT_NICE",
    "RLIMIT_RTPRIO",
    "RLIMIT_RTTIME",
];

pub(crate) const KNOWN_SECCOMP_ACTIONS: &[&str] = &[
    "SCMP_ACT_ALLOW",
    "SCMP_ACT_ERRNO",
    "SCMP_ACT_KILL",
    "SCMP_ACT_KILL_PROCESS",
    "SCMP_ACT_KILL_THREAD",
    "SCMP_ACT_TRAP",
    "SCMP_ACT_LOG",
    "SCMP_ACT_TRACE",
    "SCMP_ACT_NOTIFY",
];

pub(crate) const KNOWN_SECCOMP_ARCHES: &[&str] = &[
    "SCMP_ARCH_X86",
    "SCMP_ARCH_X86_64",
    "SCMP_ARCH_X32",
    "SCMP_ARCH_ARM",
    "SCMP_ARCH_AARCH64",
    "SCMP_ARCH_MIPS",
    "SCMP_ARCH_MIPS64",
    "SCMP_ARCH_MIPS64N32",
    "SCMP_ARCH_MIPSEL",
    "SCMP_ARCH_MIPSEL64",
    "SCMP_ARCH_MIPSEL64N32",
    "SCMP_ARCH_PPC",
    "SCMP_ARCH_PPC64",
    "SCMP_ARCH_PPC64LE",
    "SCMP_ARCH_S390",
    "SCMP_ARCH_S390X",
    "SCMP_ARCH_RISCV64",
];

#[cfg(all(test, not(target_os = "none")))]
#[path = "../tests/unit_src/src/validate_tests.rs"]
mod tests;
