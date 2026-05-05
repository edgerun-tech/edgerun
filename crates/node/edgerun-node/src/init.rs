//! PID 1 init support for edgerund.
//!
//! When run as PID 1, edgerund takes on init responsibilities:
//! - Signal handling (SIGTERM, SIGINT, SIGHUP)
//! - Zombie reaping (SIGCHLD)
//! - Graceful shutdown coordination via CancellationToken
//! - Optional basic filesystem mounts

use std::sync::atomic::{AtomicBool, Ordering};

mod libc {
    #[allow(non_camel_case_types)]
    pub type c_int = i32;
    #[allow(non_camel_case_types)]
    pub type sighandler_t = usize;

    pub const SIGTERM: c_int = 15;
    pub const SIGINT: c_int = 2;
    pub const SIGHUP: c_int = 1;
    pub const SIGCHLD: c_int = 17;
    pub const WNOHANG: c_int = 1;
    pub const STDERR_FILENO: c_int = 2;

    unsafe extern "C" {
        pub fn signal(signum: c_int, handler: sighandler_t) -> sighandler_t;
        pub fn write(fd: c_int, buf: *const core::ffi::c_void, count: usize) -> isize;
        pub fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
        pub fn getpid() -> c_int;
        pub fn mount(
            source: *const i8,
            target: *const i8,
            fstype: *const i8,
            flags: usize,
            data: *const core::ffi::c_void,
        ) -> c_int;
        pub fn sethostname(name: *const i8, len: usize) -> c_int;
    }
}

/// Global flag set when shutdown is requested.
/// Used only by the raw signal handler in init mode — the async signal
/// path in daemon.rs uses CancellationToken directly.
pub static SHUTDOWN_REQUESTED: AtomicBool = AtomicBool::new(false);

/// Install signal handlers for init-mode operation.
///
/// This is called automatically when the process is PID 1, but can also be
/// called explicitly via `--init` flag.
///
/// Handles:
/// - SIGTERM: graceful shutdown
/// - SIGINT: graceful shutdown (like ctrl_c)
/// - SIGHUP: log rotation / config reload (logged, not yet implemented)
/// - SIGCHLD: reap zombie children
pub fn install_signal_handlers() {
    extern "C" fn handle_signal(signum: libc::c_int) {
        match signum {
            libc::SIGTERM | libc::SIGINT => {
                SHUTDOWN_REQUESTED.store(true, Ordering::Relaxed);
            }
            libc::SIGHUP => {
                // Log reload request — actual reload handled in main loop
                let msg = b"SIGHUP received (reload not yet implemented)\n";
                unsafe {
                    libc::write(libc::STDERR_FILENO, msg.as_ptr() as *const _, msg.len());
                }
            }
            libc::SIGCHLD => {
                // Reap all zombie children
                reap_zombies();
            }
            _ => {}
        }
    }

    unsafe {
        // These casts are the standard way to register signal handlers with libc.
        #[allow(function_casts_as_integer)]
        libc::signal(libc::SIGTERM, handle_signal as libc::sighandler_t);
        #[allow(function_casts_as_integer)]
        libc::signal(libc::SIGINT, handle_signal as libc::sighandler_t);
        #[allow(function_casts_as_integer)]
        libc::signal(libc::SIGHUP, handle_signal as libc::sighandler_t);
        #[allow(function_casts_as_integer)]
        libc::signal(libc::SIGCHLD, handle_signal as libc::sighandler_t);
    }
}

/// Reap all zombie child processes.
pub fn reap_zombies() {
    loop {
        let mut status: libc::c_int = 0;
        let pid = unsafe { libc::waitpid(-1, &mut status, libc::WNOHANG) };
        if pid <= 0 {
            break;
        }
        // Zombie reaped — in a full init we'd log this
    }
}

/// Check if we're running as PID 1.
pub fn is_pid_one() -> bool {
    unsafe { libc::getpid() == 1 }
}

/// Perform basic init setup when running as PID 1.
///
/// This includes:
/// - Mounting essential pseudo-filesystems if not already mounted
/// - Setting up the hostname
/// - Configuring the console
pub fn init_setup() {
    if !is_pid_one() {
        return;
    }

    // Mount essential pseudo-filesystems (best effort)
    mount_if_needed("proc", "/proc", "proc");
    mount_if_needed("sysfs", "/sys", "sysfs");
    mount_if_needed("devtmpfs", "/dev", "devtmpfs");

    // Mount tmpfs for /tmp and /run
    mount_if_needed("tmpfs", "/tmp", "tmpfs");
    mount_if_needed("tmpfs", "/run", "tmpfs");

    // Set hostname from edgerun identity (if available)
    // This is done after the node identity is generated
}

/// Mount a filesystem if the mountpoint doesn't already have something mounted.
fn mount_if_needed(source: &str, target: &str, fstype: &str) {
    // Check if already mounted by looking for a file that should exist
    let check_path = match target {
        "/proc" => "/proc/self",
        "/sys" => "/sys/kernel",
        "/dev" => "/dev/null",
        "/tmp" => "/tmp",
        "/run" => "/run",
        _ => target,
    };

    if std::path::Path::new(check_path).exists() {
        return; // Already mounted or path exists
    }

    // Create mount point if needed
    if !std::path::Path::new(target).exists() {
        let _ = std::fs::create_dir_all(target);
    }

    let source_c = match std::ffi::CString::new(source) {
        Ok(c) => c,
        Err(_) => {
            let msg = "mount: source path contains NUL bytes\n";
            let bytes = msg.as_bytes();
            unsafe {
                libc::write(libc::STDERR_FILENO, bytes.as_ptr() as *const _, bytes.len());
            }
            return;
        }
    };
    let target_c = match std::ffi::CString::new(target) {
        Ok(c) => c,
        Err(_) => {
            let msg = "mount: target path contains NUL bytes\n";
            let bytes = msg.as_bytes();
            unsafe {
                libc::write(libc::STDERR_FILENO, bytes.as_ptr() as *const _, bytes.len());
            }
            return;
        }
    };
    let fstype_c = match std::ffi::CString::new(fstype) {
        Ok(c) => c,
        Err(_) => {
            let msg = "mount: fstype contains NUL bytes\n";
            let bytes = msg.as_bytes();
            unsafe {
                libc::write(libc::STDERR_FILENO, bytes.as_ptr() as *const _, bytes.len());
            }
            return;
        }
    };

    let result = unsafe {
        libc::mount(
            source_c.as_ptr(),
            target_c.as_ptr(),
            fstype_c.as_ptr(),
            0,
            std::ptr::null(),
        )
    };

    if result == 0 {
        let msg = format!("mounted {} on {}\n", fstype, target);
        let bytes = msg.as_bytes();
        unsafe {
            libc::write(libc::STDERR_FILENO, bytes.as_ptr() as *const _, bytes.len());
        }
    } else {
        let msg = format!(
            "failed to mount {} on {}: {}\n",
            fstype,
            target,
            std::io::Error::last_os_error()
        );
        let bytes = msg.as_bytes();
        unsafe {
            libc::write(libc::STDERR_FILENO, bytes.as_ptr() as *const _, bytes.len());
        }
    }
}

/// Set the system hostname.
pub fn set_hostname(name: &str) {
    if let Ok(cname) = std::ffi::CString::new(name) {
        unsafe {
            libc::sethostname(cname.as_ptr(), name.len());
        }
    }
}
