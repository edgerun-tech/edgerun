//! OCI-compatible CLI wrapper for edgerun-oci-runtime.
//!
//! Implements the commands needed by the `opencontainers/runtime-tools` conformance suite:
//! - `create` — Set up container, clone namespaces, prepare rootfs, fork but don't exec
//! - `start` — Resume exec the container process
//! - `state` — Output container state JSON to stdout
//! - `kill` — Send signal to container process
//! - `delete` — Stop and cleanup container state

use std::os::raw::c_char;

use edgerun_oci_runtime::cli::{self, parse_args, print_usage};

/// Parse a subuid/subgid file and return ranges for the given username/uid.
///
/// Format: `username:start:count`
/// Lines starting with `#` are comments.
/// Also matches by numeric UID if the username field matches the UID string.
fn parse_subid_for_user(path: &str, username: &str, uid: u32) -> Vec<(u32, u32)> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let uidstr = format!("{}", uid);
    let mut ranges = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() != 3 { continue; }
        let name = parts[0].trim();
        // Match by username, "ALL", or numeric UID
        if name != username && name != "ALL" && name != uidstr {
            continue;
        }
        if let (Ok(start), Ok(count)) = (parts[1].trim().parse::<u32>(), parts[2].trim().parse::<u32>()) {
            ranges.push((start, count));
        }
    }
    ranges
}

/// Get the current username.
fn get_current_username() -> Option<String> {
    if let Ok(user) = std::env::var("USER") {
        return Some(user);
    }
    let uid = unsafe { libc::getuid() };
    if let Ok(content) = std::fs::read_to_string("/etc/passwd") {
        for line in content.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 3 {
                if let Ok(entry_uid) = parts[2].parse::<u32>() {
                    if entry_uid == uid {
                        return Some(parts[0].to_string());
                    }
                }
            }
        }
    }
    None
}

/// Try to use newuidmap/newgidmap helpers for subuid/subgid mapping.
///
/// Podman Strategy B: If newuidmap/newgidmap are available as setuid helpers,
/// use them to write full subuid/subgid range mappings.
fn try_newuidmap(pid: i32, host_uid: u32, subuids: &[(u32, u32)]) -> bool {
    let uid_result = std::process::Command::new("newuidmap")
        .arg(format!("{}", pid))
        .arg("0").arg(format!("{}", host_uid)).arg("1")
        .args(subuids.iter().flat_map(|(start, count)| {
            // Container IDs start at 1 (0 is reserved for the user's own UID)
            vec![format!("{}", 1), format!("{}", start), format!("{}", count)]
        }))
        .output();

    let tool = match uid_result {
        Ok(o) => o,
        Err(_) => return false,
    };
    if !tool.status.success() {
        return false;
    }

    // Also run newgidmap
    let host_gid = unsafe { libc::getgid() };
    let username = get_current_username().unwrap_or_default();
    let subgids = parse_subid_for_user("/etc/subgid", &username, host_gid);
    if !subgids.is_empty() {
        let _ = std::process::Command::new("newgidmap")
            .arg(format!("{}", pid))
            .arg("0").arg(format!("{}", host_gid)).arg("1")
            .args(subgids.iter().flat_map(|(start, count)| {
                vec![format!("{}", 1), format!("{}", start), format!("{}", count)]
            }))
            .output();
    }
    true
}

/// Write uid_map directly to /proc/<pid>/uid_map.
///
/// The kernel ONLY allows writing the caller's own UID via direct /proc write.
/// Subuid ranges require newuidmap (setuid helper with CAP_SETUID).
/// When newuidmap fails, we fall back to a single mapping of the user's own UID.
fn write_uid_map_for_pid(pid: i32, host_uid: u32) -> std::io::Result<()> {
    let uid_map_path = format!("/proc/{}/uid_map", pid);
    let setgroups_path = format!("/proc/{}/setgroups", pid);

    // Deny setgroups (required before writing gid_map)
    let _ = std::fs::write(&setgroups_path, "deny\n");

    // Only the caller's own UID can be written directly
    let map = format!("0 {} 1\n", host_uid);
    std::fs::write(&uid_map_path, &map).map_err(|e| {
        std::io::Error::new(std::io::ErrorKind::PermissionDenied,
            format!("uid_map write failed: {}", e))
    })
}

fn write_gid_map_for_pid(pid: i32, host_gid: u32) -> std::io::Result<()> {
    let gid_map_path = format!("/proc/{}/gid_map", pid);
    let map = format!("0 {} 1\n", host_gid);
    std::fs::write(&gid_map_path, &map).map_err(|e| {
        std::io::Error::new(std::io::ErrorKind::PermissionDenied,
            format!("gid_map write failed: {}", e))
    })
}

/// Podman-style rootless setup: `clone(CLONE_NEWUSER|CLONE_NEWNS)` + re-exec.
///
/// Uses the regular `clone` syscall with `child_stack = NULL` (shared stack
/// with parent), exactly like Podman's `rootless_linux.c`. The child shares
/// the parent's stack but immediately re-execs, so there's no stack corruption.
///
/// The flow (same as Podman's `reexec_in_user_namespace`):
///
/// 1. Parent calls `clone(CLONE_NEWUSER|CLONE_NEWNS|SIGCHLD, NULL)`
/// 2. Parent reads subuid/subgid ranges for the current user
/// 3. Parent tries newuidmap/newgidmap (Strategy B) — if available
/// 4. Parent falls back to direct /proc/<pid>/uid_map write (Strategy C)
/// 5. Parent writes /proc/<pid>/setgroups -> deny, then gid_map
/// 6. Parent signals child via pipe
/// 7. Child (shares parent stack, can access locals):
///    - waits for parent signal
///    - calls `setresuid(0)/setresgid(0)`
///    - sets env vars `_ERT_ROOTLESS_CHILD`, `_ERT_ROOTLESS_UID`, `_ERT_ROOTLESS_GID`
///    - re-execs same binary with same argv
/// 8. After re-exec, `main()` sees `_ERT_ROOTLESS_CHILD=1` and skips this function
/// 9. Parent waits for child and returns its exit code
///
/// Returns `Some(exit_code)` in the parent, never returns in the child.
fn become_rootless() -> Option<i32> {
    let host_uid = unsafe { libc::getuid() };
    let host_gid = unsafe { libc::getgid() };
    let username = get_current_username().unwrap_or_default();

    // Read subuid/subgid ranges
    let subuids = parse_subid_for_user("/etc/subuid", &username, host_uid);
    let subgids = parse_subid_for_user("/etc/subgid", &username, host_gid);

    // Create sync pipe: parent writes, child reads
    let mut pipe_fds: [i32; 2] = [0; 2];
    if unsafe { libc::pipe(pipe_fds.as_mut_ptr()) } != 0 {
        eprintln!("ert: pipe failed: {}", std::io::Error::last_os_error());
        return Some(1);
    }
    let pipe_r = pipe_fds[0];
    let pipe_w = pipe_fds[1];

    const CLONE_NEWUSER: libc::c_int = 0x10000000;
    const CLONE_NEWNS: libc::c_int = 0x00020000;
    const SIGCHLD: libc::c_int = 17;

    // Use raw clone syscall with NULL child_stack (shared with parent).
    // This is exactly what Podman does. The child shares the parent's stack
    // but immediately calls execve, so no stack corruption occurs.
    // On x86_64, clone syscall number is 56.
    // Signature: long clone(unsigned long flags, void *child_stack)
    #[cfg(target_arch = "x86_64")]
    let ret = unsafe {
        libc::syscall(56, (CLONE_NEWUSER | CLONE_NEWNS | SIGCHLD) as libc::c_ulong, std::ptr::null_mut::<libc::c_void>())
    };
    #[cfg(target_arch = "aarch64")]
    let ret = unsafe {
        libc::syscall(220, (CLONE_NEWUSER | CLONE_NEWNS | SIGCHLD) as libc::c_ulong, std::ptr::null_mut::<libc::c_void>())
    };

    if ret < 0 {
        eprintln!("ert: clone failed: {}", std::io::Error::last_os_error());
        // Check common kernel settings that block user namespace creation
        if let Ok(val) = std::fs::read_to_string("/proc/sys/user/max_user_namespaces") {
            if val.trim() == "0" {
                eprintln!("ert: user namespaces disabled (max_user_namespaces=0)");
            }
        }
        if let Ok(val) = std::fs::read_to_string("/proc/sys/kernel/unprivileged_userns_clone") {
            if val.trim() == "0" {
                eprintln!("ert: unprivileged user namespaces disabled (unprivileged_userns_clone=0)");
            }
        }
        unsafe { libc::close(pipe_r); libc::close(pipe_w); }
        return Some(1);
    }

    let child_pid = ret as i32;

    if child_pid > 0 {
        // === PARENT: write uid_map/gid_map, signal child, wait ===
        unsafe { libc::close(pipe_r) };

        // Strategy B: Try newuidmap/newgidmap if subuid/subgid ranges exist
        let used_newuidmap = if !subuids.is_empty() {
            try_newuidmap(child_pid, host_uid, &subuids)
        } else {
            false
        };

        // Strategy C: Direct write fallback (only own UID, not subuid ranges)
        if !used_newuidmap {
            if let Err(e) = write_uid_map_for_pid(child_pid, host_uid) {
                eprintln!("ert: {}", e);
                unsafe { libc::close(pipe_w) };
                return Some(1);
            }
            if let Err(e) = write_gid_map_for_pid(child_pid, host_gid) {
                eprintln!("ert: {}", e);
                unsafe { libc::close(pipe_w) };
                return Some(1);
            }
        }

        // Signal child: maps are written
        let zero = 0u8;
        let _ = unsafe { libc::write(pipe_w, &zero as *const u8 as *const libc::c_void, 1) };
        unsafe { libc::close(pipe_w) };

        // Wait for child to exit and return its exit code
        let mut status: i32 = 0;
        let pid = unsafe { libc::waitpid(child_pid, &mut status as *mut i32, 0) };
        if pid < 0 {
            return Some(1);
        }
        if libc::WIFEXITED(status) {
            Some(libc::WEXITSTATUS(status))
        } else {
            Some(128)
        }
    } else {
        // === CHILD: shares parent's stack, can access all locals ===
        unsafe {
            libc::close(pipe_w);

            // Wait for parent to write uid_map
            let mut buf: u8 = 0;
            let r = libc::read(pipe_r, &mut buf as *mut u8 as *mut libc::c_void, 1);
            libc::close(pipe_r);
            if r != 1 { libc::_exit(1); }

            // Map container uid/gid 0 to host uid/gid
            if libc::setresuid(0, 0, 0) != 0 { libc::_exit(1); }
            if libc::setresgid(0, 0, 0) != 0 { libc::_exit(1); }

            // Set environment variables
            libc::setenv(c"_ERT_ROOTLESS_CHILD".as_ptr(), c"1".as_ptr(), 1);

            // Format UID/GID via snprintf and setenv
            let mut uid_buf = [0i8; 16];
            let _ = libc::snprintf(uid_buf.as_mut_ptr(), 16, c"%u".as_ptr(), host_uid);
            libc::setenv(c"_ERT_ROOTLESS_UID".as_ptr(), uid_buf.as_ptr(), 1);

            let mut gid_buf = [0i8; 16];
            let _ = libc::snprintf(gid_buf.as_mut_ptr(), 16, c"%u".as_ptr(), host_gid);
            libc::setenv(c"_ERT_ROOTLESS_GID".as_ptr(), gid_buf.as_ptr(), 1);

            // chdir to current working directory
            let mut cwd_buf = [0i8; 4096];
            if libc::getcwd(cwd_buf.as_mut_ptr(), cwd_buf.len()).is_null() {
                libc::_exit(1);
            }
            libc::chdir(cwd_buf.as_mut_ptr());

            // Read argv from /proc/self/cmdline (null-separated)
            let cmdline_fd = libc::open(c"/proc/self/cmdline".as_ptr(), libc::O_RDONLY);
            if cmdline_fd < 0 { libc::_exit(1); }
            let mut cmdline_buf = [0u8; 8192];
            let n = libc::read(cmdline_fd, cmdline_buf.as_mut_ptr() as *mut libc::c_void, cmdline_buf.len());
            libc::close(cmdline_fd);
            if n <= 0 { libc::_exit(1); }

            // Parse null-separated arguments into argv pointers
            let mut argv_ptrs: [*const c_char; 128] = [std::ptr::null(); 128];
            let mut argc = 0usize;
            let mut pos = 0usize;
            let n = n as usize;
            while pos < n && argc < 127 {
                let start = pos;
                while pos < n && cmdline_buf[pos] != 0 {
                    pos += 1;
                }
                if pos > start {
                    cmdline_buf[pos] = 0;
                    argv_ptrs[argc] = cmdline_buf.as_ptr().add(start) as *const c_char;
                    argc += 1;
                }
                pos += 1;
            }
            argv_ptrs[argc] = std::ptr::null();

            // Get executable path from /proc/self/exe
            let mut exe_buf = [0i8; 4096];
            let exe_len = libc::readlink(c"/proc/self/exe".as_ptr(), exe_buf.as_mut_ptr(), exe_buf.len() - 1);
            if exe_len < 0 { libc::_exit(1); }
            exe_buf[exe_len as usize] = 0;

            // Re-exec with same argv (environ is inherited automatically)
            libc::execv(exe_buf.as_ptr(), argv_ptrs.as_ptr());
            libc::_exit(127)
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        print_usage();
        std::process::exit(1);
    }

    // Podman-style rootless detection and re-exec.
    // If we're running as non-root and haven't been re-execed yet,
    // create a user namespace via clone3 and re-exec ourselves into it.
    // The child becomes uid 0 inside the user namespace and continues
    // with the same command line arguments.
    if unsafe { libc::geteuid() } != 0 && std::env::var("_ERT_ROOTLESS_CHILD").is_err() {
        if let Some(code) = become_rootless() {
            std::process::exit(code);
        }
        // In child: re-exec replaced this process, continue below
    }

    let Some((opts, command, cmd_args)) = parse_args(&args) else {
        print_usage();
        std::process::exit(1);
    };

    let result = match command.as_str() {
        "create" => cli::cmd_create(&opts, &cmd_args),
        "start" => cli::cmd_start(&opts, &cmd_args),
        "state" => cli::cmd_state(&opts, &cmd_args),
        "kill" => cli::cmd_kill(&opts, &cmd_args),
        "delete" => cli::cmd_delete(&opts, &cmd_args),
        "exec" => cli::cmd_exec(&opts, &cmd_args),
        "update" => cli::cmd_update(&opts, &cmd_args),
        "pause" => cli::cmd_pause(&opts, &cmd_args),
        "resume" => cli::cmd_resume(&opts, &cmd_args),
        "events" => cli::cmd_events(&opts, &cmd_args),
        "ps" => cli::cmd_ps(&opts, &cmd_args),
        "features" => cli::cmd_features(&opts, &cmd_args),
        "spec" => cli::cmd_spec(&opts, &cmd_args),
        _ => {
            eprintln!("Unknown command: {}", command);
            print_usage();
            std::process::exit(127);
        }
    };

    if let Err(e) = result {
        eprintln!("edgerun-oci: {}: {}", command, e);
        std::process::exit(1);
    }
}
