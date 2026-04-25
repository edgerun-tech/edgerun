//! Error types for the OCI runtime.
//!
//! Provides structured error types for each subsystem, enabling precise
//! error reporting and recovery decisions.

use std::fmt;
use std::io;

/// Top-level OCI runtime error.
#[derive(Debug)]
pub enum OciError {
    /// Container lifecycle error (create, start, delete).
    Lifecycle(LifecycleError),
    /// Container rootfs setup error.
    Rootfs(RootfsError),
    /// Seccomp filter error.
    Seccomp(SeccompError),
    /// Cgroup resource error.
    Cgroup(CgroupError),
    /// Hook execution error.
    Hook(crate::hooks::HookError),
    /// Capability/identity error.
    Capability(CapabilityError),
    /// Namespace error.
    Namespace(NamespaceError),
    /// Bundle/config error.
    Config(ConfigError),
    /// FIFO synchronization error.
    Fifo(FifoError),
}

impl fmt::Display for OciError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OciError::Lifecycle(e) => write!(f, "lifecycle error: {}", e),
            OciError::Rootfs(e) => write!(f, "rootfs error: {}", e),
            OciError::Seccomp(e) => write!(f, "seccomp error: {}", e),
            OciError::Cgroup(e) => write!(f, "cgroup error: {}", e),
            OciError::Hook(e) => write!(f, "hook error: {}", e),
            OciError::Capability(e) => write!(f, "capability error: {}", e),
            OciError::Namespace(e) => write!(f, "namespace error: {}", e),
            OciError::Config(e) => write!(f, "config error: {}", e),
            OciError::Fifo(e) => write!(f, "fifo error: {}", e),
        }
    }
}

impl std::error::Error for OciError {}

impl From<io::Error> for OciError {
    fn from(e: io::Error) -> Self {
        OciError::Lifecycle(LifecycleError::Io(e))
    }
}

/// Container lifecycle error.
#[derive(Debug)]
pub enum LifecycleError {
    /// IO error.
    Io(io::Error),
    /// Container already exists.
    AlreadyExists(String),
    /// Container not found.
    NotFound(String),
    /// Invalid container state transition.
    InvalidState { from: String, to: String },
    /// Duplicate container ID.
    DuplicateId(String),
}

impl fmt::Display for LifecycleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LifecycleError::Io(e) => write!(f, "io error: {}", e),
            LifecycleError::AlreadyExists(id) => write!(f, "container '{}' already exists", id),
            LifecycleError::NotFound(id) => write!(f, "container '{}' not found", id),
            LifecycleError::InvalidState { from, to } => {
                write!(f, "invalid state transition: {} -> {}", from, to)
            }
            LifecycleError::DuplicateId(id) => write!(f, "duplicate container ID: '{}'", id),
        }
    }
}

/// Rootfs setup error.
#[derive(Debug)]
pub enum RootfsError {
    /// IO error.
    Io(io::Error),
    /// Rootfs path not found.
    NotFound(String),
    /// Mount destination escapes rootfs.
    PathEscape(String),
    /// Mount failed.
    MountFailed {
        source: String,
        target: String,
        error: String,
    },
    /// pivot_root failed.
    PivotRootFailed { error: String },
    /// Device creation failed.
    DeviceFailed { path: String, error: String },
    /// Sysctl parameter write failed.
    SysctlFailed { key: String, error: String },
    /// Rootfs propagation failed.
    PropagationFailed { mode: String, error: String },
}

impl fmt::Display for RootfsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RootfsError::Io(e) => write!(f, "io error: {}", e),
            RootfsError::NotFound(p) => write!(f, "rootfs not found: {}", p),
            RootfsError::PathEscape(p) => write!(f, "mount destination escapes rootfs: {}", p),
            RootfsError::MountFailed {
                source,
                target,
                error,
            } => {
                write!(f, "mount failed: {} -> {}: {}", source, target, error)
            }
            RootfsError::PivotRootFailed { error } => write!(f, "pivot_root failed: {}", error),
            RootfsError::DeviceFailed { path, error } => {
                write!(f, "device creation failed for {}: {}", path, error)
            }
            RootfsError::SysctlFailed { key, error } => {
                write!(f, "sysctl failed for {}: {}", key, error)
            }
            RootfsError::PropagationFailed { mode, error } => {
                write!(f, "rootfs propagation failed ({}): {}", mode, error)
            }
        }
    }
}

/// Seccomp filter error.
#[derive(Debug)]
pub enum SeccompError {
    /// Invalid syscall name.
    InvalidSyscall(String),
    /// Filter installation failed.
    InstallFailed(String),
    /// Invalid architecture.
    InvalidArch(String),
    /// BPF program generation failed.
    BpfGenerationFailed(String),
}

impl fmt::Display for SeccompError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SeccompError::InvalidSyscall(name) => write!(f, "invalid syscall: {}", name),
            SeccompError::InstallFailed(msg) => write!(f, "seccomp install failed: {}", msg),
            SeccompError::InvalidArch(arch) => write!(f, "invalid architecture: {}", arch),
            SeccompError::BpfGenerationFailed(msg) => write!(f, "BPF generation failed: {}", msg),
        }
    }
}

/// Cgroup resource error.
#[derive(Debug)]
pub enum CgroupError {
    /// IO error.
    Io(io::Error),
    /// Cgroup path not found.
    NotFound(String),
    /// Cgroup file write failed.
    WriteFailed {
        file: String,
        content: String,
        error: String,
    },
    /// PID placement in cgroup failed.
    PidPlacementFailed { pid: u32, error: String },
    /// Invalid resource value.
    InvalidValue { resource: String, value: String },
    /// Cgroup cleanup failed.
    CleanupFailed { path: String, error: String },
}

impl fmt::Display for CgroupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CgroupError::Io(e) => write!(f, "io error: {}", e),
            CgroupError::NotFound(p) => write!(f, "cgroup not found: {}", p),
            CgroupError::WriteFailed {
                file,
                content,
                error,
            } => {
                write!(f, "cgroup write failed ({}): {} - {}", file, content, error)
            }
            CgroupError::PidPlacementFailed { pid, error } => {
                write!(f, "failed to place PID {} in cgroup: {}", pid, error)
            }
            CgroupError::InvalidValue { resource, value } => {
                write!(f, "invalid {} value: {}", resource, value)
            }
            CgroupError::CleanupFailed { path, error } => {
                write!(f, "cgroup cleanup failed for {}: {}", path, error)
            }
        }
    }
}

/// Hook execution error (defined in hooks.rs — re-exported here for consistency).
/// The actual implementation lives in `hooks::HookError`.
pub type HookError = crate::hooks::HookError;

/// Capability/identity error.
#[derive(Debug)]
pub enum CapabilityError {
    /// Unknown capability name.
    UnknownCapability(String),
    /// Capability set failed.
    SetFailed { capability: String, error: String },
    /// UID/GID mapping failed.
    MappingFailed { map_type: String, error: String },
    /// Supplementary group setup failed.
    SupplementaryGroupsFailed(String),
}

impl fmt::Display for CapabilityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CapabilityError::UnknownCapability(name) => write!(f, "unknown capability: {}", name),
            CapabilityError::SetFailed { capability, error } => {
                write!(f, "failed to set capability {}: {}", capability, error)
            }
            CapabilityError::MappingFailed { map_type, error } => {
                write!(f, "{} mapping failed: {}", map_type, error)
            }
            CapabilityError::SupplementaryGroupsFailed(error) => {
                write!(f, "supplementary groups setup failed: {}", error)
            }
        }
    }
}

/// Namespace error.
#[derive(Debug)]
pub enum NamespaceError {
    /// Unknown namespace type.
    UnknownType(String),
    /// Namespace path not found.
    PathNotFound(String),
    /// Unshare failed.
    UnshareFailed { flags: i32, error: String },
    /// Setns (join existing namespace) failed.
    SetnsFailed { path: String, error: String },
}

impl fmt::Display for NamespaceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NamespaceError::UnknownType(t) => write!(f, "unknown namespace type: {}", t),
            NamespaceError::PathNotFound(p) => write!(f, "namespace path not found: {}", p),
            NamespaceError::UnshareFailed { flags, error } => {
                write!(f, "unshare failed (flags=0x{:x}): {}", flags, error)
            }
            NamespaceError::SetnsFailed { path, error } => {
                write!(f, "setns failed for {}: {}", path, error)
            }
        }
    }
}

/// Bundle/config error.
#[derive(Debug)]
pub enum ConfigError {
    /// Missing required field.
    MissingField(String),
    /// Invalid field value.
    InvalidValue { field: String, reason: String },
    /// JSON parse error.
    JsonParse(String),
    /// Bundle directory error.
    BundleDir(String),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::MissingField(field) => write!(f, "missing required field: {}", field),
            ConfigError::InvalidValue { field, reason } => {
                write!(f, "invalid value for {}: {}", field, reason)
            }
            ConfigError::JsonParse(msg) => write!(f, "JSON parse error: {}", msg),
            ConfigError::BundleDir(msg) => write!(f, "bundle directory error: {}", msg),
        }
    }
}

/// FIFO synchronization error.
#[derive(Debug)]
pub enum FifoError {
    /// FIFO creation failed.
    CreateFailed(String),
    /// FIFO open failed (reader or writer).
    OpenFailed(String),
    /// Invalid signal payload.
    InvalidSignal(String),
    /// FIFO closed prematurely.
    PrematureClose(String),
}

impl fmt::Display for FifoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FifoError::CreateFailed(msg) => write!(f, "FIFO creation failed: {}", msg),
            FifoError::OpenFailed(msg) => write!(f, "FIFO open failed: {}", msg),
            FifoError::InvalidSignal(msg) => write!(f, "invalid FIFO signal: {}", msg),
            FifoError::PrematureClose(msg) => write!(f, "FIFO closed prematurely: {}", msg),
        }
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ===========================================================================
    // Display formatting
    // ===========================================================================

    #[test]
    fn lifecycle_error_display() {
        let e = LifecycleError::AlreadyExists("test-container".into());
        assert_eq!(
            format!("{}", e),
            "container 'test-container' already exists"
        );
    }

    #[test]
    fn lifecycle_error_not_found() {
        let e = LifecycleError::NotFound("missing".into());
        assert_eq!(format!("{}", e), "container 'missing' not found");
    }

    #[test]
    fn lifecycle_error_invalid_state() {
        let e = LifecycleError::InvalidState {
            from: "created".into(),
            to: "deleted".into(),
        };
        assert_eq!(
            format!("{}", e),
            "invalid state transition: created -> deleted"
        );
    }

    #[test]
    fn rootfs_error_display() {
        let e = RootfsError::NotFound("/var/lib/rootfs".into());
        assert_eq!(format!("{}", e), "rootfs not found: /var/lib/rootfs");
    }

    #[test]
    fn rootfs_error_path_escape() {
        let e = RootfsError::PathEscape("../etc/passwd".into());
        assert_eq!(
            format!("{}", e),
            "mount destination escapes rootfs: ../etc/passwd"
        );
    }

    #[test]
    fn seccomp_error_display() {
        let e = SeccompError::InvalidSyscall("bogus".into());
        assert_eq!(format!("{}", e), "invalid syscall: bogus");
    }

    #[test]
    fn cgroup_error_display() {
        let e = CgroupError::WriteFailed {
            file: "memory.max".into(),
            content: "1048576".into(),
            error: "permission denied".into(),
        };
        assert_eq!(
            format!("{}", e),
            "cgroup write failed (memory.max): 1048576 - permission denied"
        );
    }

    #[test]
    fn hook_error_display() {
        let e = crate::hooks::HookError {
            hook_path: "/usr/bin/hook".into(),
            error: std::io::Error::other("exit code 1"),
        };
        let s = format!("{}", e);
        assert!(s.contains("/usr/bin/hook"));
        assert!(s.contains("exit code 1"));
    }

    #[test]
    fn hook_error_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<crate::hooks::HookError>();
    }

    #[test]
    fn capability_error_display() {
        let e = CapabilityError::UnknownCapability("CAP_BOGUS".into());
        assert_eq!(format!("{}", e), "unknown capability: CAP_BOGUS");
    }

    #[test]
    fn namespace_error_display() {
        let e = NamespaceError::UnknownType("bogus".into());
        assert_eq!(format!("{}", e), "unknown namespace type: bogus");
    }

    #[test]
    fn config_error_display() {
        let e = ConfigError::MissingField("root.path".into());
        assert_eq!(format!("{}", e), "missing required field: root.path");
    }

    #[test]
    fn fifo_error_display() {
        let e = FifoError::CreateFailed("path too long".into());
        assert_eq!(format!("{}", e), "FIFO creation failed: path too long");
    }

    // ===========================================================================
    // OciError enum
    // ===========================================================================

    #[test]
    fn oci_error_from_io() {
        let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "access denied");
        let oci_err: OciError = io_err.into();
        match oci_err {
            OciError::Lifecycle(LifecycleError::Io(e)) => {
                assert_eq!(e.kind(), io::ErrorKind::PermissionDenied);
            }
            _ => panic!("expected Lifecycle error"),
        }
    }

    #[test]
    fn oci_error_display_variants() {
        let errors: Vec<OciError> = vec![
            OciError::Lifecycle(LifecycleError::NotFound("x".into())),
            OciError::Rootfs(RootfsError::NotFound("y".into())),
            OciError::Seccomp(SeccompError::InvalidSyscall("z".into())),
            OciError::Cgroup(CgroupError::NotFound("a".into())),
            OciError::Hook(crate::hooks::HookError {
                hook_path: "b".into(),
                error: std::io::Error::other("c"),
            }),
            OciError::Capability(CapabilityError::UnknownCapability("d".into())),
            OciError::Namespace(NamespaceError::UnknownType("e".into())),
            OciError::Config(ConfigError::MissingField("f".into())),
            OciError::Fifo(FifoError::CreateFailed("g".into())),
        ];

        for err in &errors {
            let s = format!("{}", err);
            assert!(!s.is_empty(), "error display should not be empty");
        }
    }

    #[test]
    fn all_errors_are_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<OciError>();
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
