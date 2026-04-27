//! Error types for the OCI runtime.

use crate::prelude::*;
use std::error::Error;
use std::fmt;
use std::io;

#[derive(Debug)]
pub enum OciError {
    Lifecycle,
    Rootfs,
    Seccomp,
    Cgroup,
    Hook,
    Capability,
    Namespace,
    Config,
    Fifo,
}

impl fmt::Display for OciError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OciError::Lifecycle => write!(f, "lifecycle error"),
            OciError::Rootfs => write!(f, "rootfs error"),
            OciError::Seccomp => write!(f, "seccomp error"),
            OciError::Cgroup => write!(f, "cgroup error"),
            OciError::Hook => write!(f, "hook error"),
            OciError::Capability => write!(f, "capability error"),
            OciError::Namespace => write!(f, "namespace error"),
            OciError::Config => write!(f, "config error"),
            OciError::Fifo => write!(f, "fifo error"),
        }
    }
}

impl Error for OciError {}

impl From<io::Error> for OciError {
    fn from(_: io::Error) -> Self {
        OciError::Lifecycle
    }
}

#[derive(Debug)]
pub enum LifecycleError {
    Io,
    AlreadyExists,
    NotFound,
    InvalidState,
    DuplicateId,
}

impl fmt::Display for LifecycleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LifecycleError::Io => write!(f, "io error"),
            LifecycleError::AlreadyExists => write!(f, "container already exists"),
            LifecycleError::NotFound => write!(f, "container not found"),
            LifecycleError::InvalidState => write!(f, "invalid state transition"),
            LifecycleError::DuplicateId => write!(f, "duplicate container ID"),
        }
    }
}

impl Error for LifecycleError {}

#[derive(Debug)]
pub enum RootfsError {
    Io,
    NotFound,
    PathEscape,
    MountFailed,
    PivotRootFailed,
    DeviceFailed,
    SysctlFailed,
    PropagationFailed,
}

impl fmt::Display for RootfsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RootfsError::Io => write!(f, "io error"),
            RootfsError::NotFound => write!(f, "rootfs not found"),
            RootfsError::PathEscape => write!(f, "mount destination escapes rootfs"),
            RootfsError::MountFailed => write!(f, "mount failed"),
            RootfsError::PivotRootFailed => write!(f, "pivot_root failed"),
            RootfsError::DeviceFailed => write!(f, "device creation failed"),
            RootfsError::SysctlFailed => write!(f, "sysctl failed"),
            RootfsError::PropagationFailed => write!(f, "rootfs propagation failed"),
        }
    }
}

impl Error for RootfsError {}

#[derive(Debug)]
pub enum SeccompError {
    InvalidSyscall,
    InstallFailed,
    InvalidArch,
    BpfGenerationFailed,
}

impl fmt::Display for SeccompError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SeccompError::InvalidSyscall => write!(f, "invalid syscall"),
            SeccompError::InstallFailed => write!(f, "seccomp install failed"),
            SeccompError::InvalidArch => write!(f, "invalid architecture"),
            SeccompError::BpfGenerationFailed => write!(f, "BPF generation failed"),
        }
    }
}

impl Error for SeccompError {}

#[derive(Debug)]
pub enum CgroupError {
    Io,
    NotFound,
    WriteFailed,
    PidPlacementFailed,
    InvalidValue,
    CleanupFailed,
}

impl fmt::Display for CgroupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CgroupError::Io => write!(f, "io error"),
            CgroupError::NotFound => write!(f, "cgroup not found"),
            CgroupError::WriteFailed => write!(f, "cgroup write failed"),
            CgroupError::PidPlacementFailed => write!(f, "PID placement failed"),
            CgroupError::InvalidValue => write!(f, "invalid value"),
            CgroupError::CleanupFailed => write!(f, "cgroup cleanup failed"),
        }
    }
}

impl Error for CgroupError {}

pub type HookError = crate::hooks::HookError;

#[derive(Debug)]
pub enum CapabilityError {
    UnknownCapability,
    SetFailed,
    MappingFailed,
    SupplementaryGroupsFailed,
}

impl fmt::Display for CapabilityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CapabilityError::UnknownCapability => write!(f, "unknown capability"),
            CapabilityError::SetFailed => write!(f, "capability set failed"),
            CapabilityError::MappingFailed => write!(f, "mapping failed"),
            CapabilityError::SupplementaryGroupsFailed => write!(f, "supplementary groups failed"),
        }
    }
}

impl Error for CapabilityError {}

#[derive(Debug)]
pub enum NamespaceError {
    UnknownType,
    PathNotFound,
    UnshareFailed,
    SetnsFailed,
}

impl fmt::Display for NamespaceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NamespaceError::UnknownType => write!(f, "unknown namespace type"),
            NamespaceError::PathNotFound => write!(f, "namespace path not found"),
            NamespaceError::UnshareFailed => write!(f, "unshare failed"),
            NamespaceError::SetnsFailed => write!(f, "setns failed"),
        }
    }
}

impl Error for NamespaceError {}

#[derive(Debug)]
pub enum ConfigError {
    MissingField,
    InvalidValue,
    JsonParse,
    BundleDir,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::MissingField => write!(f, "missing required field"),
            ConfigError::InvalidValue => write!(f, "invalid value"),
            ConfigError::JsonParse => write!(f, "JSON parse error"),
            ConfigError::BundleDir => write!(f, "bundle directory error"),
        }
    }
}

impl Error for ConfigError {}

#[derive(Debug)]
pub enum FifoError {
    CreateFailed,
    OpenFailed,
    InvalidSignal,
    PrematureClose,
}

impl fmt::Display for FifoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FifoError::CreateFailed => write!(f, "FIFO creation failed"),
            FifoError::OpenFailed => write!(f, "FIFO open failed"),
            FifoError::InvalidSignal => write!(f, "invalid FIFO signal"),
            FifoError::PrematureClose => write!(f, "FIFO closed prematurely"),
        }
    }
}

impl Error for FifoError {}

#[cfg(all(test, not(target_os = "none")))]
mod tests {
    use super::*;

    #[test]
    fn all_errors_are_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<LifecycleError>();
        assert_send_sync::<RootfsError>();
        assert_send_sync::<SeccompError>();
        assert_send_sync::<CgroupError>();
        assert_send_sync::<CapabilityError>();
        assert_send_sync::<NamespaceError>();
        assert_send_sync::<ConfigError>();
        assert_send_sync::<FifoError>();
    }
}
