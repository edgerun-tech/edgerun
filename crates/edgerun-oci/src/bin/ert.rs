//! OCI-compatible CLI for edgerun-oci — container runtime + registry.

use std::os::raw::c_char;

use edgerun_oci::cli::{self, parse_args, print_usage};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        print_usage();
        std::process::exit(1);
    }

    // Rootless detection and re-exec (only for container commands)
    let is_container_cmd = args
        .first()
        .map(|s| {
            matches!(
                s.as_str(),
                "create"
                    | "start"
                    | "exec"
                    | "delete"
                    | "kill"
                    | "pause"
                    | "resume"
                    | "update"
                    | "state"
                    | "ps"
                    | "events"
                    | "checkpoint"
                    | "restore"
                    | "run"
            )
        })
        .unwrap_or(false);

    if is_container_cmd
        && unsafe { libc::geteuid() } != 0
        && std::env::var("_ERT_ROOTLESS_CHILD").is_err()
    {
        if let Some(code) = become_rootless() {
            std::process::exit(code);
        }
    }

    let Some((opts, command, cmd_args)) = parse_args(&args) else {
        print_usage();
        std::process::exit(1);
    };

    let result = match command.as_str() {
        // Container lifecycle
        "create" => cli::cmd_create(&opts, &cmd_args),
        "start" => cli::cmd_start(&opts, &cmd_args),
        "state" => cli::cmd_state(&opts, &cmd_args),
        "kill" => cli::cmd_kill(&opts, &cmd_args),
        "delete" => cli::cmd_delete(&opts, &cmd_args),
        "exec" => cli::cmd_exec(&opts, &cmd_args),
        "update" => cli::cmd_update(&opts, &cmd_args),
        "pause" => cli::cmd_pause(&opts, &cmd_args),
        "resume" => cli::cmd_resume(&opts, &cmd_args),
        "checkpoint" => cli::cmd_checkpoint(&opts, &cmd_args),
        "restore" => cli::cmd_restore(&opts, &cmd_args),
        "events" => cli::cmd_events(&opts, &cmd_args),
        "ps" => cli::cmd_ps(&opts, &cmd_args),
        "features" => cli::cmd_features(&opts, &cmd_args),
        "spec" => cli::cmd_spec(&opts, &cmd_args),
        // Registry
        "pull" => cli::cmd_pull(&opts, &cmd_args),
        "push" => cli::cmd_push(&opts, &cmd_args),
        "run" => cli::cmd_run(&opts, &cmd_args),
        "registry" => dispatch_registry_command(&opts, &cmd_args),
        _ => {
            eprintln!("Unknown command: {}", command);
            print_usage();
            std::process::exit(127);
        }
    };

    if let Err(e) = result {
        eprintln!("ert: {}: {}", command, e);
        std::process::exit(1);
    }
}

fn dispatch_registry_command(_opts: &cli::GlobalOpts, cmd_args: &[String]) -> std::io::Result<()> {
    if cmd_args.is_empty() {
        eprintln!("Usage: ert registry <subcommand> [options]");
        eprintln!("Subcommands: login, logout");
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "registry subcommand required",
        ));
    }

    let subcommand = &cmd_args[0];
    let sub_args = &cmd_args[1..];

    match subcommand.as_str() {
        "login" => cli::cmd_login(_opts, sub_args),
        "logout" => cli::cmd_logout(_opts, sub_args),
        _ => {
            eprintln!("Unknown registry subcommand: {}", subcommand);
            eprintln!("Available: login, logout");
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("unknown registry subcommand: {}", subcommand),
            ))
        }
    }
}

// ---------- Rootless re-exec (copied from original ert.rs) ----------

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
        if parts.len() != 3 {
            continue;
        }
        let name = parts[0].trim();
        if name != username && name != "ALL" && name != uidstr {
            continue;
        }
        if let (Ok(start), Ok(count)) = (
            parts[1].trim().parse::<u32>(),
            parts[2].trim().parse::<u32>(),
        ) {
            ranges.push((start, count));
        }
    }
    ranges
}

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

fn try_newuidmap(pid: i32, host_uid: u32, subuids: &[(u32, u32)]) -> bool {
    let uid_result = std::process::Command::new("newuidmap")
        .arg(format!("{}", pid))
        .arg("0")
        .arg(format!("{}", host_uid))
        .arg("1")
        .args(subuids.iter().flat_map(|(start, count)| {
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
    let host_gid = unsafe { libc::getgid() };
    let username = get_current_username().unwrap_or_default();
    let subgids = parse_subid_for_user("/etc/subgid", &username, host_gid);
    if !subgids.is_empty() {
        let _ = std::process::Command::new("newgidmap")
            .arg(format!("{}", pid))
            .arg("0")
            .arg(format!("{}", host_gid))
            .arg("1")
            .args(subgids.iter().flat_map(|(start, count)| {
                vec![format!("{}", 1), format!("{}", start), format!("{}", count)]
            }))
            .output();
    }
    true
}

fn write_uid_map_for_pid(pid: i32, host_uid: u32) -> std::io::Result<()> {
    let uid_map_path = format!("/proc/{}/uid_map", pid);
    let setgroups_path = format!("/proc/{}/setgroups", pid);
    let _ = std::fs::write(&setgroups_path, "deny\n");
    let map = format!("0 {} 1\n", host_uid);
    std::fs::write(&uid_map_path, &map).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            format!("uid_map write failed: {}", e),
        )
    })
}

fn write_gid_map_for_pid(pid: i32, host_gid: u32) -> std::io::Result<()> {
    let gid_map_path = format!("/proc/{}/gid_map", pid);
    let map = format!("0 {} 1\n", host_gid);
    std::fs::write(&gid_map_path, &map).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            format!("gid_map write failed: {}", e),
        )
    })
}

fn become_rootless() -> Option<i32> {
    let host_uid = unsafe { libc::getuid() };
    let host_gid = unsafe { libc::getgid() };
    let username = get_current_username().unwrap_or_default();
    let subuids = parse_subid_for_user("/etc/subuid", &username, host_uid);
    let subgids = parse_subid_for_user("/etc/subgid", &username, host_gid);

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

    #[cfg(target_arch = "x86_64")]
    let ret = unsafe {
        libc::syscall(
            56,
            (CLONE_NEWUSER | CLONE_NEWNS | SIGCHLD) as libc::c_ulong,
            std::ptr::null_mut::<libc::c_void>(),
        )
    };
    #[cfg(target_arch = "aarch64")]
    let ret = unsafe {
        libc::syscall(
            220,
            (CLONE_NEWUSER | CLONE_NEWNS | SIGCHLD) as libc::c_ulong,
            std::ptr::null_mut::<libc::c_void>(),
        )
    };

    if ret < 0 {
        eprintln!("ert: clone failed: {}", std::io::Error::last_os_error());
        unsafe {
            libc::close(pipe_r);
            libc::close(pipe_w);
        }
        return Some(1);
    }

    let child_pid = ret as i32;

    if child_pid > 0 {
        unsafe { libc::close(pipe_r) };
        let used_newuidmap = if !subuids.is_empty() {
            try_newuidmap(child_pid, host_uid, &subuids)
        } else {
            false
        };
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
        let zero = 0u8;
        let _ = unsafe { libc::write(pipe_w, &zero as *const u8 as *const libc::c_void, 1) };
        unsafe { libc::close(pipe_w) };
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
        unsafe {
            libc::close(pipe_w);
            let mut buf: u8 = 0;
            let r = libc::read(pipe_r, &mut buf as *mut u8 as *mut libc::c_void, 1);
            libc::close(pipe_r);
            if r != 1 {
                libc::_exit(1);
            }
            if libc::setresuid(0, 0, 0) != 0 {
                libc::_exit(1);
            }
            if libc::setresgid(0, 0, 0) != 0 {
                libc::_exit(1);
            }
            libc::setenv(c"_ERT_ROOTLESS_CHILD".as_ptr(), c"1".as_ptr(), 1);
            let mut uid_buf = [0i8; 16];
            let _ = libc::snprintf(uid_buf.as_mut_ptr(), 16, c"%u".as_ptr(), host_uid);
            libc::setenv(c"_ERT_ROOTLESS_UID".as_ptr(), uid_buf.as_ptr(), 1);
            let mut gid_buf = [0i8; 16];
            let _ = libc::snprintf(gid_buf.as_mut_ptr(), 16, c"%u".as_ptr(), host_gid);
            libc::setenv(c"_ERT_ROOTLESS_GID".as_ptr(), gid_buf.as_ptr(), 1);
            let mut cwd_buf = [0i8; 4096];
            if libc::getcwd(cwd_buf.as_mut_ptr(), cwd_buf.len()).is_null() {
                libc::_exit(1);
            }
            libc::chdir(cwd_buf.as_ptr());
            let cmdline_fd = libc::open(c"/proc/self/cmdline".as_ptr(), libc::O_RDONLY);
            if cmdline_fd < 0 {
                libc::_exit(1);
            }
            let mut cmdline_buf = [0u8; 8192];
            let n = libc::read(
                cmdline_fd,
                cmdline_buf.as_mut_ptr() as *mut libc::c_void,
                cmdline_buf.len(),
            );
            libc::close(cmdline_fd);
            if n <= 0 {
                libc::_exit(1);
            }
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
            let mut exe_buf = [0i8; 4096];
            let exe_len = libc::readlink(
                c"/proc/self/exe".as_ptr(),
                exe_buf.as_mut_ptr(),
                exe_buf.len() - 1,
            );
            if exe_len < 0 {
                libc::_exit(1);
            }
            exe_buf[exe_len as usize] = 0;
            libc::execv(exe_buf.as_ptr(), argv_ptrs.as_ptr());
            libc::_exit(127)
        }
    }
}
