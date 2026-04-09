//! PID 1 init support for edgerund.
//!
//! When run as PID 1, edgerund takes on init responsibilities:
//! - Signal handling (SIGTERM, SIGINT, SIGHUP)
//! - Zombie reaping (SIGCHLD)
//! - Graceful shutdown coordination
//! - Optional basic filesystem mounts

use std::sync::atomic::{AtomicBool, Ordering};

/// Global flag set when shutdown is requested.
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
        libc::signal(libc::SIGTERM, handle_signal as libc::sighandler_t);
        libc::signal(libc::SIGINT, handle_signal as libc::sighandler_t);
        libc::signal(libc::SIGHUP, handle_signal as libc::sighandler_t);
        libc::signal(libc::SIGCHLD, handle_signal as libc::sighandler_t);
    }
}

/// Reap all zombie child processes.
fn reap_zombies() {
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

    let source_c = std::ffi::CString::new(source).unwrap();
    let target_c = std::ffi::CString::new(target).unwrap();
    let fstype_c = std::ffi::CString::new(fstype).unwrap();

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
        let msg = format!("failed to mount {} on {}: {}\n", fstype, target, std::io::Error::last_os_error());
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
