//! Container rootfs setup: pivot_root, mount filesystems, create devices.
//!
//! Fixes applied:
//! - Read-only rootfs enforcement from `OciRoot.readonly`
//! - Device creation from spec's `linux.devices`
//! - Overlay whiteout char device handling (0:0 device check)

use std::ffi::CString;
use std::fs;
use std::io;
use std::os::raw::c_ulong;
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::os::raw::c_int;
use std::path::Path;

use crate::json::{OciLinuxDevice, OciMount, OciRoot};
use crate::syscalls::{
    chown, do_mount, do_pivot_root, do_umount2, makedev, mknod,
    ms, MNT_DETACH, S_IFCHR,
    mount_attr, MountAttr, do_mount_setattr,
    open_tree, move_mount, do_open_tree, do_move_mount,
};

// ===========================================================================
// Mount helpers
// ===========================================================================

fn mount_flags_from_opts(opts: Option<&[String]>) -> c_ulong {
    let mut flags: c_ulong = 0;
    if let Some(opts) = opts {
        for opt in opts {
            match opt.as_str() {
                "ro"           => flags |= ms::RDONLY,
                "nosuid"       => flags |= ms::NOSUID,
                "nodev"        => flags |= ms::NODEV,
                "noexec"       => flags |= ms::NOEXEC,
                "strictatime"  => flags |= ms::STRICTATIME,
                "shared"       => flags |= ms::SHARED,
                "slave"        => flags |= ms::SLAVE,
                "private"      => flags |= ms::PRIVATE,
                "unbindable"   => flags |= ms::UNBINDABLE,
                _ => {}
            }
        }
    }
    flags
}

fn setup_mount(mount: &OciMount, mount_label: Option<&str>) -> io::Result<()> {
    let dest = Path::new(&mount.destination);

    // Validate: mount destination must be absolute and not escape rootfs.
    if !dest.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("mount destination must be absolute: {}", mount.destination),
        ));
    }

    let normalized = dest.components().collect::<Vec<_>>();
    let mut depth = 0isize;
    for comp in &normalized {
        use std::path::Component;
        match comp {
            Component::RootDir => {}
            Component::Normal(_) => depth += 1,
            Component::ParentDir => {
                depth -= 1;
                if depth < 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!("mount destination escapes rootfs: {}", mount.destination),
                    ));
                }
            }
            _ => {}
        }
    }

    // Check if already mounted at this destination — skip if so
    if is_already_mounted(&mount.destination, mount.mount_type.as_deref()) {
        return Ok(());
    }

    let source = mount.source.as_deref().unwrap_or("");
    let fstype = mount.mount_type.as_deref().unwrap_or("");
    let flags = mount_flags_from_opts(mount.options.as_deref());

    // Build data string: options + optional SELinux label
    let mut data = mount.options.as_ref().map(|o| o.join(",")).unwrap_or_default();
    if let Some(label) = mount_label {
        if !data.is_empty() {
            data.push(',');
        }
        data.push_str(label);
    }

    if fstype == "bind" || flags & ms::BIND != 0 {
        if Path::new(source).is_dir() {
            fs::create_dir_all(dest)?;
        } else {
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)?;
            }
            let _ = fs::File::create(dest);
        }
        do_mount(source, &mount.destination, "", flags | ms::BIND, &data)?;
        do_mount(source, &mount.destination, "", flags | ms::BIND | ms::REMOUNT, &data)?;
    } else {
        fs::create_dir_all(dest)?;
        do_mount(source, &mount.destination, fstype, flags, &data)?;
    }

    // Apply mount propagation separately (must be a distinct mount call).
    // Propagation flags: shared, slave, private, unbindable.
    let prop_flags = flags & (ms::SHARED | ms::SLAVE | ms::PRIVATE | ms::UNBINDABLE);
    if prop_flags != 0 {
        do_mount("none", &mount.destination, "", ms::REC | prop_flags, "")?;
    }

    // OCI 1.1 mount.recursive — apply mount flags recursively to sub-mounts
    // via mount_setattr(2) with MOUNT_ATTR_REC.
    if mount.recursive == Some(true) {
        let attr_set = mount_attr::REC | (flags & (mount_attr::RDONLY
            | mount_attr::NOSUID | mount_attr::NODEV | mount_attr::NOEXEC));
        let attr = MountAttr {
            attr_set,
            attr_clr: 0,
            propagation: 0,
            userns_fd: 0,
        };
        // Best-effort: kernel may not support mount_setattr (pre-5.12)
        let _ = do_mount_setattr(
            libc::AT_FDCWD,
            &mount.destination,
            &attr,
            mount_attr::REC as u32,
        );
    }

    // OCI 1.1/1.2 idmapped mounts — uid/gid mappings for the mount.
    // Requires Linux 5.12+ and the new mount API (open_tree + move_mount).
    if let Some(ref uid_mappings) = mount.uid_mappings {
        if !uid_mappings.is_empty() {
            let _ = setup_idmapped_mount(
                &mount.destination,
                source,
                fstype,
                uid_mappings,
                mount.gid_mappings.as_deref(),
            );
        }
    }

    Ok(())
}

/// Set up an idmapped mount using the new mount API (Linux 5.12+).
///
/// Creates a user namespace with the given uid/gid mappings, then uses
/// open_tree + mount_setattr(MOUNT_ATTR_IDMAP) + move_mount to create
/// an idmapped mount at the target path.
///
/// Flow:
/// 1. fork child
/// 2. child: unshare(CLONE_NEWUSER), write uid_map/gid_map, signal parent via pipe, pause
/// 3. parent: open /proc/child_pid/ns/user → userns_fd
/// 4. parent: open_tree(dest) → tree_fd
/// 5. parent: mount_setattr(tree_fd, MOUNT_ATTR_IDMAP, userns_fd)
/// 6. parent: move_mount(tree_fd, "", AT_FDCWD, dest)
/// 7. parent: signal child to exit via pipe, waitpid
fn setup_idmapped_mount(
    dest: &str,
    _source: &str,
    _fstype: &str,
    uid_mappings: &[crate::json::OciIdMapping],
    gid_mappings: Option<&[crate::json::OciIdMapping]>,
) -> io::Result<()> {
    // Build uid_map string: "container_id host_id size\n" per entry
    let uid_map_str: String = uid_mappings.iter()
        .map(|m| format!("{} {} {}\n", m.container_id, m.host_id, m.size))
        .collect();

    // Build gid_map string (same format)
    let gid_map_str: String = gid_mappings.map(|mappings| {
        mappings.iter()
            .map(|m| format!("{} {} {}\n", m.container_id, m.host_id, m.size))
            .collect()
    }).unwrap_or_default();

    // Two pipes for bidirectional synchronization:
    // child_ready: child writes "R" → parent reads
    // parent_done: parent writes "D" → child reads
    let mut child_ready: [c_int; 2] = [-1, -1]; // [0]=read(parent), [1]=write(child)
    let mut parent_done: [c_int; 2] = [-1, -1]; // [0]=read(child), [1]=write(parent)
    if unsafe { libc::pipe(child_ready.as_mut_ptr()) } != 0
        || unsafe { libc::pipe(parent_done.as_mut_ptr()) } != 0
    {
        if child_ready[0] >= 0 { unsafe { libc::close(child_ready[0]) }; }
        if child_ready[1] >= 0 { unsafe { libc::close(child_ready[1]) }; }
        if parent_done[0] >= 0 { unsafe { libc::close(parent_done[0]) }; }
        if parent_done[1] >= 0 { unsafe { libc::close(parent_done[1]) }; }
        return Err(io::Error::last_os_error());
    }

    let pid = unsafe { libc::fork() };
    if pid < 0 {
        let _ = unsafe { libc::close(child_ready[0]) };
        let _ = unsafe { libc::close(child_ready[1]) };
        let _ = unsafe { libc::close(parent_done[0]) };
        let _ = unsafe { libc::close(parent_done[1]) };
        return Err(io::Error::last_os_error());
    }

    if pid == 0 {
        // ====== CHILD PROCESS ======
        // Close ends we don't use
        unsafe { libc::close(child_ready[0]) }; // child doesn't read from child_ready
        unsafe { libc::close(parent_done[1]) }; // child doesn't write to parent_done

        // Create new user namespace
        if unsafe { libc::unshare(libc::CLONE_NEWUSER) } != 0 {
            let _ = unsafe { libc::write(child_ready[1], b"E" as *const _ as *const libc::c_void, 1) };
            unsafe { libc::_exit(1) };
        }

        // Write uid_map
        if std::fs::write("/proc/self/uid_map", &uid_map_str).is_err() {
            let _ = unsafe { libc::write(child_ready[1], b"E" as *const _ as *const libc::c_void, 1) };
            unsafe { libc::_exit(1) };
        }

        // Must deny setgroups before writing gid_map (kernel requirement)
        let _ = std::fs::write("/proc/self/setgroups", "deny");

        // Write gid_map (only if non-empty)
        if !gid_map_str.is_empty() && std::fs::write("/proc/self/gid_map", &gid_map_str).is_err() {
            let _ = unsafe { libc::write(child_ready[1], b"E" as *const _ as *const libc::c_void, 1) };
            unsafe { libc::_exit(1) };
        }

        // Signal parent that mappings are ready
        let _ = unsafe { libc::write(child_ready[1], b"R" as *const _ as *const libc::c_void, 1) };

        // Wait for parent to signal completion (or error)
        // Parent will write "D" (done) or "E" (error)
        let mut buf = [0u8; 1];
        let _ = unsafe { libc::read(parent_done[0], buf.as_mut_ptr() as *mut _, 1) };
        unsafe { libc::close(child_ready[1]) };
        unsafe { libc::close(parent_done[0]) };

        unsafe { libc::_exit(0) };
    }

    // ====== PARENT PROCESS ======
    // Close ends we don't use
    unsafe { libc::close(child_ready[1]) }; // parent doesn't write to child_ready
    unsafe { libc::close(parent_done[0]) }; // parent doesn't read from parent_done

    // Wait for child to signal ready or error
    let mut buf = [0u8; 1];
    let n = unsafe { libc::read(child_ready[0], buf.as_mut_ptr() as *mut _, 1) };
    unsafe { libc::close(child_ready[0]) };

    if n != 1 || buf[0] != b'R' {
        // Child failed — reap it
        unsafe { libc::waitpid(pid, std::ptr::null_mut(), 0) };
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "failed to create user namespace for idmapped mount",
        ));
    }

    // Open child's user namespace fd
    let userns_path = format!("/proc/{}/ns/user", pid);
    let userns_cstr = match CString::new(userns_path.as_str()) {
        Ok(c) => c,
        Err(_) => {
            unsafe { libc::waitpid(pid, std::ptr::null_mut(), 0) };
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid userns path"));
        }
    };
    let userns_fd = unsafe { libc::open(userns_cstr.as_ptr(), libc::O_RDONLY | libc::O_CLOEXEC) };
    if userns_fd < 0 {
        let err = io::Error::last_os_error();
        unsafe { libc::waitpid(pid, std::ptr::null_mut(), 0) };
        return Err(err);
    }

    // Step 1: open_tree to get a reference to the existing mount at dest
    let tree_fd = do_open_tree(libc::AT_FDCWD, dest, open_tree::CLONE | open_tree::CLOEXEC);
    if tree_fd.is_err() {
        unsafe { libc::close(userns_fd) };
        unsafe { libc::waitpid(pid, std::ptr::null_mut(), 0) };
        // Fall back gracefully — kernel may not support open_tree (pre-5.6)
        return Ok(());
    }
    let tree_fd = tree_fd.unwrap();

    // Step 2: mount_setattr with MOUNT_ATTR_IDMAP
    let attr = MountAttr {
        attr_set: mount_attr::IDMAP,
        attr_clr: 0,
        propagation: 0,
        userns_fd: userns_fd as u64,
    };
    let result = do_mount_setattr(tree_fd, "", &attr, move_mount::T_EMPTY_PATH);

    // Step 3: If mount_setattr succeeded, move_mount to re-attach the idmapped mount
    if result.is_ok() {
        let _ = do_move_mount(tree_fd, "", libc::AT_FDCWD, dest, move_mount::F_EMPTY_PATH);
    }

    // Cleanup
    unsafe { libc::close(tree_fd) };
    unsafe { libc::close(userns_fd) };

    // Signal child to exit (write "D" to parent_done pipe)
    // This unblocks the child's read on parent_done[0]
    let _ = unsafe { libc::write(parent_done[1], b"D" as *const _ as *const libc::c_void, 1) };
    unsafe { libc::close(parent_done[1]) };

    // Reap child
    unsafe { libc::waitpid(pid, std::ptr::null_mut(), 0) };

    // If mount_setattr failed, return Ok — kernel may not support idmapped mounts (pre-5.12)
    result.or(Ok(()))
}

/// Check if a filesystem of the given type is already mounted at the destination.
fn is_already_mounted(destination: &str, fstype: Option<&str>) -> bool {
    use std::fs;
    let mountinfo = match fs::read_to_string("/proc/self/mountinfo") {
        Ok(s) => s,
        Err(_) => return false,
    };

    for line in mountinfo.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 5 {
            continue;
        }
        // parts[4] is the mount point
        if parts[4] == destination {
            // parts[8] is the filesystem type (optional, after separator)
            if let Some(expected_type) = fstype {
                // Find the separator (fields after it start at index 7+)
                if let Some(sep_idx) = parts.iter().position(|&p| p == "-") {
                    if sep_idx + 1 < parts.len() && parts[sep_idx + 1] == expected_type {
                        return true;
                    }
                }
            } else {
                return true;
            }
        }
    }
    false
}

// ===========================================================================
// Device creation
// ===========================================================================

fn create_device(path: &str, major: u64, minor: u64, mode: u32) {
    let dev = makedev(major, minor);
    let path_c = match CString::new(path) {
        Ok(c) => c,
        Err(_) => return, // Path contains null byte — skip
    };
    let _ = unsafe { mknod(path_c.as_ptr(), S_IFCHR | mode, dev) };
}

/// Create a device node from an OCI spec device entry.
fn create_spec_device(device: &OciLinuxDevice) -> io::Result<()> {
    let dev_type = match device.ns_type.as_str() {
        "c" | "char" => S_IFCHR,
        "b" | "block" => 0o060000, // S_IFBLK
        "p" | "fifo" => 0o010000,  // S_IFIFO
        _ => return Ok(()), // Skip unknown types
    };

    let mode = device.file_mode.unwrap_or(0o660);
    let major = device.major.unwrap_or(0) as u64;
    let minor = device.minor.unwrap_or(0) as u64;

    // Ensure parent directory exists
    if let Some(parent) = Path::new(&device.path).parent() {
        let _ = fs::create_dir_all(parent);
    }

    let path_c = match CString::new(device.path.as_str()) {
        Ok(c) => c,
        Err(_) => return Ok(()),
    };

    let dev = makedev(major, minor);
    let ret = unsafe { mknod(path_c.as_ptr(), dev_type | mode, dev) };
    if ret != 0 {
        return Err(io::Error::last_os_error());
    }

    // Set ownership if uid/gid specified in the OCI spec
    if device.uid.is_some() || device.gid.is_some() {
        let uid = device.uid.unwrap_or(u32::MAX);
        let gid = device.gid.unwrap_or(u32::MAX);
        let ret = unsafe { chown(path_c.as_ptr(), uid, gid) };
        if ret != 0 {
            return Err(io::Error::last_os_error());
        }
    }

    Ok(())
}

/// Create essential device nodes in /dev.
fn create_essential_devices() {
    create_device("/dev/null", 1, 3, 0o666);
    create_device("/dev/zero", 1, 5, 0o666);
    create_device("/dev/full", 1, 7, 0o666);
    create_device("/dev/random", 1, 8, 0o444);
    create_device("/dev/urandom", 1, 9, 0o444);
    create_device("/dev/tty", 5, 0, 0o666);
    let _ = fs::create_dir_all("/dev/pts");
    let _ = fs::create_dir_all("/dev/shm");
}

// ===========================================================================
// Whiteout handling
// ===========================================================================

/// Apply whiteout files across layers (reverse order, top layer first).
pub fn apply_whiteouts(layer_dirs: &[std::path::PathBuf]) -> io::Result<()> {
    for layer_dir in layer_dirs.iter().rev() {
        remove_whiteout_files(layer_dir)?;
    }
    Ok(())
}

/// Remove whiteout files from a directory tree.
/// Handles both OCI-style `.wh.` prefix and overlayfs char device whiteouts (0:0).
fn remove_whiteout_files(dir: &Path) -> io::Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }

    let entries: Vec<_> = match fs::read_dir(dir) {
        Ok(entries) => entries.filter_map(|e| e.ok()).collect(),
        Err(_) => return Ok(()),
    };

    for entry in entries {
        let path = entry.path();
        let file_name = entry.file_name();

        if let Some(name) = file_name.to_str() {
            // Skip the opaque whiteout marker itself (it's a control file, not a whiteout)
            if name == ".wh..wh..opq" {
                continue;
            }
            // OCI-style whiteout: .wh.<name> → delete <name>
            if let Some(rest) = name.strip_prefix(".wh.") {
                let target = path.parent().unwrap().join(rest);
                if target.exists() {
                    if target.is_dir() {
                        let _ = fs::remove_dir_all(&target);
                    } else {
                        let _ = fs::remove_file(&target);
                    }
                }
                let _ = fs::remove_file(&path);
                continue;
            }
        }

        // Overlayfs char device whiteout (0:0 character device)
        // Must use rdev() (device numbers of the file itself), not dev() (filesystem device ID)
        if let Ok(metadata) = path.metadata() {
            if metadata.file_type().is_char_device() && metadata.rdev() == 0 {
                let _ = fs::remove_file(&path);
            }
        }

        if path.is_dir() {
            remove_whiteout_files(&path)?;
        }
    }

    Ok(())
}

// ===========================================================================
// Rootfs building
// ===========================================================================

/// Build rootfs by merging layers in order.
pub fn build_rootfs(layer_dirs: &[std::path::PathBuf], dest: &Path) -> io::Result<()> {
    for layer_dir in layer_dirs {
        copy_dir_contents(layer_dir, dest)?;
    }
    Ok(())
}

/// Copy all contents from src to dest, overwriting existing files.
fn copy_dir_contents(src: &Path, dest: &Path) -> io::Result<()> {
    use std::os::unix::fs::symlink;

    if !src.is_dir() {
        return Ok(());
    }

    let entries: Vec<_> = match fs::read_dir(src) {
        Ok(entries) => entries.filter_map(|e| e.ok()).collect(),
        Err(_) => return Ok(()),
    };

    for entry in entries {
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        let file_name = entry.file_name();

        // Skip whiteout files
        if let Some(name) = file_name.to_str() {
            if name.starts_with(".wh.") {
                continue;
            }
        }

        if src_path.is_dir() {
            let _ = fs::create_dir_all(&dest_path);
            copy_dir_contents(&src_path, &dest_path)?;
        } else if src_path.is_symlink() {
            let target = fs::read_link(&src_path)?;
            let _ = fs::remove_file(&dest_path);
            let _ = symlink(&target, &dest_path);
        } else {
            let _ = fs::remove_file(&dest_path);
            fs::copy(&src_path, &dest_path)?;
        }
    }

    Ok(())
}

// ===========================================================================
// Full rootfs setup
// ===========================================================================

/// Setup the container rootfs: pivot_root, mount filesystems, create devices.
///
/// `root.readonly` enforces a read-only rootfs when set to true.
/// `spec_devices` is the list of devices from `linux.devices` in the OCI spec.
/// `mount_label` is the SELinux mount label from `linux.mountLabel`.
/// `strict_masked`/`strict_readonly` make mount failures fatal.
pub fn setup_rootfs(
    root: &OciRoot,
    mounts: Option<&[OciMount]>,
    masked: Option<&[String]>,
    readonly: Option<&[String]>,
    spec_devices: Option<&[OciLinuxDevice]>,
    mount_label: Option<&str>,
) -> io::Result<()> {
    let rootfs = Path::new(&root.path);
    if !rootfs.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("rootfs not found: {}", rootfs.display()),
        ));
    }

    // Bind mount rootfs to make it a mount point
    let rootfs_cstr = rootfs.to_str().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "rootfs path is not valid UTF-8")
    })?;
    do_mount(
        rootfs_cstr,
        rootfs_cstr,
        "bind",
        ms::BIND | ms::REC,
        "",
    )?;

    // Make / private BEFORE pivot_root so mounts don't propagate to host
    // This can fail on some systems (EBUSY), so we make it best-effort
    let _ = do_mount("", "/", "", ms::PRIVATE | ms::REC, "");

    // Create old_root inside rootfs for pivot_root
    let old_root = rootfs.join(".oci-old-root");
    fs::create_dir_all(&old_root)?;

    // pivot_root requires CWD to be under new_root, so chdir to rootfs first
    std::env::set_current_dir(rootfs)?;

    // pivot_root (now with CWD inside rootfs, use relative paths)
    let old_root_cstr = ".oci-old-root";
    do_pivot_root(".", old_root_cstr)?;

    // Detach and remove old root
    do_umount2("/.oci-old-root", MNT_DETACH)?;
    let _ = fs::remove_dir("/.oci-old-root");

    // If root is readonly, remount the entire rootfs as read-only NOW,
    // before mounting writable filesystems on top.
    if root.readonly == Some(true) {
        do_mount("", "/", "", ms::REMOUNT | ms::RDONLY, "")?;
    }

    // Mount proc
    fs::create_dir_all("/proc")?;
    do_mount("proc", "/proc", "proc", ms::NOSUID | ms::NODEV | ms::NOEXEC | ms::REC, "")?;

    // Mount sys
    fs::create_dir_all("/sys")?;
    do_mount("sysfs", "/sys", "sysfs", ms::NOSUID | ms::NODEV | ms::NOEXEC | ms::REC, "")?;

    // Mount dev (tmpfs)
    fs::create_dir_all("/dev")?;
    do_mount("tmpfs", "/dev", "tmpfs", ms::NOSUID | ms::STRICTATIME, "mode=755,size=65536k")?;

    // Essential device nodes
    create_essential_devices();

    // Create spec-defined devices
    if let Some(devices) = spec_devices {
        for device in devices {
            let _ = create_spec_device(device); // Best-effort — some devices may not be creatable
        }
    }

    // Mount devpts
    do_mount(
        "devpts", "/dev/pts", "devpts",
        ms::NOSUID | ms::NOEXEC,
        "newinstance,ptmxmode=0666,mode=0620",
    )?;

    // Mount tmpfs on /dev/shm
    do_mount(
        "tmpfs", "/dev/shm", "tmpfs",
        ms::NOSUID | ms::NODEV,
        "mode=1777,size=65536k",
    )?;

    // /dev/ptmx -> pts/ptmx
    let _ = fs::remove_file("/dev/ptmx");
    let _ = std::os::unix::fs::symlink("pts/ptmx", "/dev/ptmx");

    // Additional mounts from spec
    if let Some(spec_mounts) = mounts {
        for m in spec_mounts {
            // Make certain filesystem types best-effort (mqueue, hugetlbfs, etc.)
            let is_optional = matches!(m.mount_type.as_deref(), Some("mqueue") | Some("hugetlbfs"));
            if is_optional {
                let _ = setup_mount(m, mount_label);
            } else {
                setup_mount(m, mount_label)?;
            }
        }
    }


    // Masked paths — security-sensitive paths masked with /dev/null
    if let Some(paths) = masked {
        for p in paths {
            do_mount("/dev/null", p, "", ms::BIND, "").map_err(|e| {
                io::Error::other(
                    format!("failed to mask path {}: {}", p, e),
                )
            })?;
        }
    }

    // Readonly Paths — bind mount and remount read-only
    if let Some(paths) = readonly {
        for p in paths {
            do_mount(p, p, "", ms::BIND | ms::REC, "").map_err(|e| {
                io::Error::other(
                    format!("failed to bind readonly path {}: {}", p, e),
                )
            })?;
            do_mount(
                p, p, "",
                ms::BIND | ms::REMOUNT | ms::RDONLY | ms::NOSUID | ms::NODEV | ms::NOEXEC,
                "",
            ).map_err(|e| {
                io::Error::other(
                    format!("failed to remount readonly path {}: {}", p, e),
                )
            })?;
        }
    }

    Ok(())
}

// ===========================================================================
// Sysctl parameter setting
// ===========================================================================

/// Apply sysctl parameters from the OCI spec.
///
/// Sysctl keys like `net.ipv4.ip_forward` are written to `/proc/sys/net/ipv4/ip_forward`.
/// This must be called after /proc is mounted.
pub fn apply_sysctl(sysctl: Option<&std::collections::HashMap<String, String>>) -> io::Result<()> {
    let Some(params) = sysctl else { return Ok(()) };

    for (key, value) in params {
        // Convert dots to slashes: net.ipv4.ip_forward → net/ipv4/ip_forward
        let proc_path = format!("/proc/sys/{}", key.replace('.', "/"));
        if let Err(e) = fs::write(&proc_path, value) {
            // Log but don't fail — some sysctls may not be available in all environments
            let _ = std::fs::write("/dev/kmsg", format!("edgerun: sysctl {:?} failed: {}", key, e));
        }
    }

    Ok(())
}

// ===========================================================================
// Rootfs propagation
// ===========================================================================

/// Set rootfs propagation mode.
///
/// Valid modes: "shared", "slave", "private", "unbindable".
/// This must be called after pivot_root, when "/" is the container root.
pub fn set_rootfs_propagation(mode: Option<&str>) -> io::Result<()> {
    let Some(mode) = mode else { return Ok(()) };

    let flags = match mode {
        "shared"     => ms::REC | 0x100,   // MS_SHARED = 0x100
        "slave"      => ms::REC | 0x200,   // MS_SLAVE = 0x200
        "unbindable" => ms::REC | 0x400,   // MS_UNBINDABLE = 0x400
        "private"    => ms::REC | ms::PRIVATE, // MS_PRIVATE = 1<<18
        _ => return Ok(()), // Unknown mode — use default
    };

    do_mount("", "/", "", flags, "")
}
