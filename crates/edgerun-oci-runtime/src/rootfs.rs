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
use std::path::Path;

use crate::json::{OciLinuxDevice, OciMount, OciRoot};
use crate::syscalls::{
    do_mount, do_pivot_root, do_umount2, makedev, mknod,
    ms, MNT_DETACH, S_IFCHR,
};

// ===========================================================================
// Mount helpers
// ===========================================================================

fn mount_flags_from_opts(opts: Option<&[String]>) -> c_ulong {
    let mut flags: c_ulong = 0;
    if let Some(opts) = opts {
        for opt in opts {
            match opt.as_str() {
                "ro"          => flags |= ms::RDONLY,
                "nosuid"      => flags |= ms::NOSUID,
                "nodev"       => flags |= ms::NODEV,
                "noexec"      => flags |= ms::NOEXEC,
                "strictatime" => flags |= ms::STRICTATIME,
                _ => {}
            }
        }
    }
    flags
}

fn setup_mount(mount: &OciMount) -> io::Result<()> {
    let dest = Path::new(&mount.destination);

    // Validate: mount destination must be absolute and not escape rootfs.
    // After pivot_root, "/" is the container root, so we check the path
    // doesn't use ".." to escape (defense-in-depth).
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

    let source = mount.source.as_deref().unwrap_or("");
    let fstype = mount.mount_type.as_deref().unwrap_or("");
    let flags = mount_flags_from_opts(mount.options.as_deref());
    let data = mount.options.as_ref().map(|o| o.join(",")).unwrap_or_default();

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
    Ok(())
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
    Ok(())
}

/// Create essential device nodes in /dev.
fn create_essential_devices() {
    create_device("/dev/null", 1, 3, 0o666);
    create_device("/dev/zero", 1, 5, 0o666);
    create_device("/dev/full", 1, 7, 0o666);
    create_device("/dev/random", 1, 8, 0o666);
    create_device("/dev/urandom", 1, 9, 0o666);
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
        if let Ok(metadata) = path.metadata() {
            if metadata.file_type().is_char_device() {
                let dev = metadata.dev();
                if dev == 0 {
                    // This is an overlay whiteout — remove it
                    let _ = fs::remove_file(&path);
                }
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
pub fn setup_rootfs(
    root: &OciRoot,
    mounts: Option<&[OciMount]>,
    masked: Option<&[String]>,
    readonly: Option<&[String]>,
    spec_devices: Option<&[OciLinuxDevice]>,
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

    // Create old_root inside rootfs for pivot_root
    let old_root = rootfs.join(".oci-old-root");
    fs::create_dir_all(&old_root)?;

    // pivot_root
    let old_root_cstr = old_root.to_str().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "old_root path is not valid UTF-8")
    })?;
    do_pivot_root(rootfs_cstr, old_root_cstr)?;

    // Detach and remove old root
    do_umount2("/.oci-old-root", MNT_DETACH)?;
    let _ = fs::remove_dir("/.oci-old-root");

    // Make / private so mounts don't propagate to host
    do_mount("", "/", "", ms::PRIVATE | ms::REC, "")?;

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
    if let Some(mounts) = mounts {
        for m in mounts {
            if let Err(e) = setup_mount(m) {
                // Log mount errors but continue — some optional mounts may not be creatable
                let _ = std::fs::write("/dev/kmsg", format!("edgerun: mount {:?} failed: {}", m.destination, e));
            }
        }
    }

    // Masked paths
    if let Some(paths) = masked {
        for p in paths {
            let _ = do_mount("/dev/null", p, "", ms::BIND, "");
        }
    }

    // Readonly Paths
    if let Some(paths) = readonly {
        for p in paths {
            let _ = do_mount(p, p, "", ms::BIND | ms::REC, "");
            let _ = do_mount(
                p, p, "",
                ms::BIND | ms::REMOUNT | ms::RDONLY | ms::NOSUID | ms::NODEV | ms::NOEXEC,
                "",
            );
        }
    }

    Ok(())
}
