//! Container rootfs setup: pivot_root, mount filesystems, create devices.
//!
//! Fixes applied:
//! - Read-only rootfs enforcement from `OciRoot.readonly`
//! - Device creation from spec's `linux.devices`
//! - Overlay whiteout char device handling (0:0 device check)

use crate::libc;
use crate::prelude::*;
pub use crate::rootfs_layers::{apply_whiteouts, build_rootfs};
use std::ffi::CString;
use std::fs;
use std::io;
use std::os::raw::c_int;
use std::os::raw::c_ulong;
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};

use crate::rootfs_devices::create_rootfs_devices;
use crate::rootfs_idmap::setup_idmapped_mount;
use crate::spec::{OciLinuxDevice, OciMount, OciRoot};
use crate::syscalls::{
    MNT_DETACH, MountAttr, do_mount, do_mount_setattr, do_pivot_root, do_umount2, mount_attr, ms,
};

fn c_string(value: &str) -> io::Result<CString> {
    CString::new(value).map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))
}

fn libc_unit(ret: c_int) -> io::Result<()> {
    if ret == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

// ===========================================================================
// Mount helpers
// ===========================================================================

fn mount_flags_from_opts(opts: Option<&[String]>) -> c_ulong {
    let mut flags: c_ulong = 0;
    if let Some(opts) = opts {
        for opt in opts {
            match opt.as_str() {
                "ro" => flags |= ms::RDONLY,
                "rw" => {}
                "rbind" => flags |= ms::BIND | ms::REC,
                "nosuid" => flags |= ms::NOSUID,
                "nodev" => flags |= ms::NODEV,
                "noexec" => flags |= ms::NOEXEC,
                "strictatime" => flags |= ms::STRICTATIME,
                "shared" => flags |= ms::SHARED,
                "slave" => flags |= ms::SLAVE,
                "private" => flags |= ms::PRIVATE,
                "unbindable" => flags |= ms::UNBINDABLE,
                // OCI 1.2.0: idmap/ridmap mount options (handled via setup_idmapped_mount below)
                "idmap" => {}  // Handled by uidMappings — no flag, uses new mount API
                "ridmap" => {} // Recursive idmap — same handling
                _ => {}
            }
        }
    }
    flags
}

fn mount_data_from_opts(opts: Option<&[String]>) -> String {
    opts.map(|opts| {
        opts.iter()
            .filter(|opt| {
                !matches!(
                    opt.as_str(),
                    "ro" | "rw"
                        | "rbind"
                        | "bind"
                        | "nosuid"
                        | "nodev"
                        | "noexec"
                        | "relatime"
                        | "strictatime"
                        | "shared"
                        | "slave"
                        | "private"
                        | "unbindable"
                        | "idmap"
                        | "ridmap"
                )
            })
            .cloned()
            .collect::<Vec<_>>()
            .join(",")
    })
    .unwrap_or_default()
}

fn is_bind_mount(mount: &OciMount) -> bool {
    mount.mount_type.as_deref() == Some("bind")
        || mount
            .options
            .as_deref()
            .is_some_and(|opts| opts.iter().any(|opt| opt == "bind" || opt == "rbind"))
}

fn destination_under_rootfs(rootfs: &Path, destination: &str) -> io::Result<PathBuf> {
    Ok(rootfs.join(rootfs_relative_destination(destination)?))
}

fn rootfs_relative_destination(destination: &str) -> io::Result<PathBuf> {
    let dest = Path::new(destination);
    let mut relative = PathBuf::new();
    for component in dest.components() {
        match component {
            Component::RootDir | Component::CurDir => {}
            Component::Normal(part) => relative.push(part),
            Component::ParentDir => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("mount destination escapes rootfs: {destination}"),
                ));
            }
            Component::Prefix(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("unsupported mount destination: {destination}"),
                ));
            }
        }
    }
    Ok(relative)
}

fn setup_bind_mount_before_pivot(rootfs: &Path, mount: &OciMount) -> io::Result<()> {
    let source = mount
        .source
        .as_deref()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "bind mount requires source"))?;
    let source_path = Path::new(source);
    if !source_path.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("bind mount source must be absolute: {source}"),
        ));
    }
    if !source_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("bind mount source not found: {source}"),
        ));
    }

    let dest = destination_under_rootfs(rootfs, &mount.destination)?;
    if source_path.is_dir() {
        fs::create_dir_all(&dest)?;
    } else {
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        if !dest.exists() {
            fs::File::create(&dest)?;
        }
    }

    let target = dest.to_str().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "bind mount destination path is not valid UTF-8",
        )
    })?;
    let flags = mount_flags_from_opts(mount.options.as_deref());
    let recursive = flags & ms::REC != 0;
    let initial_flags = if recursive {
        ms::BIND | ms::REC
    } else {
        ms::BIND
    };
    do_mount(source, target, "bind", initial_flags, "")?;
    if flags & ms::RDONLY != 0 {
        let readonly_flags = (flags
            & !(ms::REC | ms::SHARED | ms::SLAVE | ms::PRIVATE | ms::UNBINDABLE))
            | ms::BIND
            | ms::REMOUNT
            | ms::RDONLY;
        remount_bind_readonly(target, readonly_flags)?;
    }
    Ok(())
}

fn remount_bind_readonly(target: &str, flags: c_ulong) -> io::Result<()> {
    let target_str = target;
    let target = c_string(target_str)?;
    let ret = unsafe {
        libc::mount(
            std::ptr::null(),
            target.as_ptr(),
            std::ptr::null(),
            flags,
            std::ptr::null(),
        )
    };
    libc_unit(ret).or_else(|remount_error| {
        let attr = MountAttr {
            attr_set: mount_attr::RDONLY,
            attr_clr: 0,
            propagation: 0,
            userns_fd: 0,
        };
        do_mount_setattr(libc::AT_FDCWD, target_str, &attr, 0).map_err(|_| remount_error)
    })
}

fn setup_mount(mount: &OciMount, mount_label: Option<&str>) -> io::Result<()> {
    // Validate: mount destination must be absolute, or a relative path that
    // doesn't escape rootfs (OCI 1.2.0 allows relative mount destinations).
    // Relative paths are resolved against "/" (the container rootfs).
    rootfs_relative_destination(&mount.destination)?;
    let dest = Path::new(&mount.destination);

    // Check if already mounted at this destination — skip if so
    if is_already_mounted(&mount.destination, mount.mount_type.as_deref()) {
        return Ok(());
    }

    let source = mount.source.as_deref().unwrap_or("");
    let fstype = mount.mount_type.as_deref().unwrap_or("");
    let flags = mount_flags_from_opts(mount.options.as_deref());

    // Build data string: options + optional SELinux label
    let mut data = mount_data_from_opts(mount.options.as_deref());
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
        do_mount(
            source,
            &mount.destination,
            "",
            flags | ms::BIND | ms::REMOUNT,
            &data,
        )?;
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
        let attr_set = mount_attr::REC
            | (flags
                & (mount_attr::RDONLY
                    | mount_attr::NOSUID
                    | mount_attr::NODEV
                    | mount_attr::NOEXEC));
        let attr = MountAttr {
            attr_set,
            attr_clr: 0,
            propagation: 0,
            userns_fd: 0,
        };
        if let Err(e) = do_mount_setattr(
            libc::AT_FDCWD,
            &mount.destination,
            &attr,
            mount_attr::REC as u32,
        ) {
            let _ = std::fs::write(
                "/dev/kmsg",
                format!(
                    "edgerun: mount.recursive failed for {}: {} (kernel may not support mount_setattr)",
                    mount.destination, e
                ),
            );
        }
    }

    // OCI 1.1/1.2 idmapped mounts — uid/gid mappings for the mount.
    // Requires Linux 5.12+ and the new mount API (open_tree + move_mount).
    if let Some(ref uid_mappings) = mount.uid_mappings {
        if !uid_mappings.is_empty() {
            if let Err(e) = setup_idmapped_mount(
                &mount.destination,
                uid_mappings,
                mount.gid_mappings.as_deref(),
            ) {
                let _ = std::fs::write(
                    "/dev/kmsg",
                    format!(
                        "edgerun: idmapped mount failed for {}: {} (kernel may not support idmapped mounts)",
                        mount.destination, e
                    ),
                );
            }
        }
    }

    Ok(())
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
    setup_rootfs_inner(
        root,
        mounts,
        masked,
        readonly,
        spec_devices,
        mount_label,
        false,
    )
}

pub fn setup_rootfs_rootless(
    root: &OciRoot,
    mounts: Option<&[OciMount]>,
    masked: Option<&[String]>,
    readonly: Option<&[String]>,
    spec_devices: Option<&[OciLinuxDevice]>,
    mount_label: Option<&str>,
) -> io::Result<()> {
    setup_rootfs_inner(
        root,
        mounts,
        masked,
        readonly,
        spec_devices,
        mount_label,
        true,
    )
}

fn setup_rootfs_inner(
    root: &OciRoot,
    mounts: Option<&[OciMount]>,
    masked: Option<&[String]>,
    readonly: Option<&[String]>,
    spec_devices: Option<&[OciLinuxDevice]>,
    mount_label: Option<&str>,
    tolerate_kernel_mount_denial: bool,
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
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "rootfs path is not valid UTF-8",
        )
    })?;
    do_mount(rootfs_cstr, rootfs_cstr, "bind", ms::BIND | ms::REC, "").map_err(|error| {
        io::Error::new(
            error.kind(),
            format!(
                "bind mount rootfs {} onto itself failed: {}",
                rootfs.display(),
                error
            ),
        )
    })?;

    // Make / private BEFORE pivot_root so mounts don't propagate to host
    // This can fail on some systems (EBUSY), so we make it best-effort
    let _ = do_mount("", "/", "", ms::PRIVATE | ms::REC, "");

    // Create old_root inside rootfs for pivot_root
    let old_root = rootfs.join(".oci-old-root");
    fs::create_dir_all(&old_root)?;

    if let Some(spec_mounts) = mounts {
        for mount in spec_mounts.iter().filter(|mount| is_bind_mount(mount)) {
            setup_bind_mount_before_pivot(rootfs, mount).map_err(|error| {
                io::Error::new(
                    error.kind(),
                    format!(
                        "bind mount {} -> {} failed before pivot: {}",
                        mount.source.as_deref().unwrap_or(""),
                        mount.destination,
                        error
                    ),
                )
            })?;
        }
    }

    // pivot_root requires CWD to be under new_root, so chdir to rootfs first
    std::env::set_current_dir(rootfs)?;

    // pivot_root (now with CWD inside rootfs, use relative paths)
    let old_root_cstr = ".oci-old-root";
    do_pivot_root(".", old_root_cstr).map_err(|error| {
        io::Error::new(
            error.kind(),
            format!(
                "pivot_root into {} with put_old {} failed: {}",
                rootfs.display(),
                old_root.display(),
                error
            ),
        )
    })?;

    // Detach and remove old root
    do_umount2("/.oci-old-root", MNT_DETACH).map_err(|error| {
        io::Error::new(error.kind(), format!("detach old root failed: {error}"))
    })?;
    let _ = fs::remove_dir("/.oci-old-root");

    // If root is readonly, remount the entire rootfs as read-only NOW,
    // before mounting writable filesystems on top.
    if root.readonly == Some(true) {
        do_mount("", "/", "", ms::REMOUNT | ms::RDONLY, "").map_err(|error| {
            io::Error::new(
                error.kind(),
                format!("remount rootfs read-only failed: {error}"),
            )
        })?;
    }

    // Mount proc
    fs::create_dir_all("/proc")?;
    let proc_result = do_mount(
        "proc",
        "/proc",
        "proc",
        ms::NOSUID | ms::NODEV | ms::NOEXEC,
        "",
    );
    if let Err(error) = proc_result {
        if !tolerate_mount_denial(tolerate_kernel_mount_denial, &error) {
            return Err(io::Error::new(
                error.kind(),
                format!("mount proc failed: {error}"),
            ));
        }
    }

    // Mount sys
    fs::create_dir_all("/sys")?;
    let sys_result = do_mount(
        "sysfs",
        "/sys",
        "sysfs",
        ms::NOSUID | ms::NODEV | ms::NOEXEC,
        "",
    );
    if let Err(error) = sys_result {
        if !tolerate_mount_denial(tolerate_kernel_mount_denial, &error) {
            return Err(io::Error::new(
                error.kind(),
                format!("mount sysfs failed: {error}"),
            ));
        }
    }

    // Mount dev (tmpfs)
    fs::create_dir_all("/dev")?;
    do_mount(
        "tmpfs",
        "/dev",
        "tmpfs",
        ms::NOSUID | ms::STRICTATIME,
        "mode=755,size=65536k",
    )
    .map_err(|error| {
        io::Error::new(error.kind(), format!("mount tmpfs on /dev failed: {error}"))
    })?;

    create_rootfs_devices(spec_devices);

    // Mount devpts
    let devpts_result = do_mount(
        "devpts",
        "/dev/pts",
        "devpts",
        ms::NOSUID | ms::NOEXEC,
        "newinstance,ptmxmode=0666,mode=0620",
    );
    if let Err(error) = devpts_result {
        if !tolerate_mount_denial(tolerate_kernel_mount_denial, &error) {
            return Err(io::Error::new(
                error.kind(),
                format!("mount devpts failed: {error}"),
            ));
        }
    }

    // Mount tmpfs on /dev/shm
    do_mount(
        "tmpfs",
        "/dev/shm",
        "tmpfs",
        ms::NOSUID | ms::NODEV,
        "mode=1777,size=65536k",
    )
    .map_err(|error| {
        io::Error::new(
            error.kind(),
            format!("mount tmpfs on /dev/shm failed: {error}"),
        )
    })?;

    // /dev/ptmx -> pts/ptmx
    let _ = fs::remove_file("/dev/ptmx");
    let _ = std::os::unix::fs::symlink("pts/ptmx", "/dev/ptmx");

    // Additional mounts from spec
    if let Some(spec_mounts) = mounts {
        for m in spec_mounts {
            if is_bind_mount(m) {
                continue;
            }
            // Make certain filesystem types best-effort (mqueue, hugetlbfs, etc.)
            let is_optional = matches!(
                m.mount_type.as_deref(),
                Some("mqueue") | Some("hugetlbfs") | Some("cgroup")
            );
            if is_optional {
                let _ = setup_mount(m, mount_label);
            } else {
                setup_mount(m, mount_label).map_err(|error| {
                    io::Error::new(
                        error.kind(),
                        format!(
                            "mount spec entry {} type {:?} source {:?} failed: {}",
                            m.destination, m.mount_type, m.source, error
                        ),
                    )
                })?;
            }
        }
    }

    // Masked paths — security-sensitive paths masked with /dev/null
    if let Some(paths) = masked {
        for p in paths {
            if Path::new(p).exists() {
                let _ = do_mount("/dev/null", p, "", ms::BIND, "");
            }
        }
    }

    // Readonly Paths — bind mount and remount read-only
    if let Some(paths) = readonly {
        for p in paths {
            if !Path::new(p).exists() {
                continue;
            }
            if do_mount(p, p, "", ms::BIND | ms::REC, "").is_err() {
                continue;
            }
            let _ = do_mount(
                p,
                p,
                "",
                ms::BIND | ms::REMOUNT | ms::RDONLY | ms::NOSUID | ms::NODEV | ms::NOEXEC,
                "",
            );
        }
    }

    Ok(())
}

fn tolerate_mount_denial(enabled: bool, error: &io::Error) -> bool {
    enabled && matches!(error.kind(), io::ErrorKind::PermissionDenied)
}

// ===========================================================================
// Sysctl parameter setting
// ===========================================================================

/// Apply sysctl parameters from the OCI spec.
///
/// Sysctl keys like `net.ipv4.ip_forward` are written to `/proc/sys/net/ipv4/ip_forward`.
/// This must be called after /proc is mounted.
pub fn apply_sysctl(
    sysctl: Option<&alloc::collections::BTreeMap<String, String>>,
) -> io::Result<()> {
    let Some(params) = sysctl else { return Ok(()) };

    for (key, value) in params {
        // Convert dots to slashes: net.ipv4.ip_forward → net/ipv4/ip_forward
        let proc_path = format!("/proc/sys/{}", key.replace('.', "/"));
        if let Err(e) = fs::write(&proc_path, value) {
            // Log but don't fail — some sysctls may not be available in all environments
            let _ = std::fs::write(
                "/dev/kmsg",
                format!("edgerun: sysctl {:?} failed: {}", key, e),
            );
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
        "shared" => ms::REC | 0x100,        // MS_SHARED = 0x100
        "slave" => ms::REC | 0x200,         // MS_SLAVE = 0x200
        "unbindable" => ms::REC | 0x400,    // MS_UNBINDABLE = 0x400
        "private" => ms::REC | ms::PRIVATE, // MS_PRIVATE = 1<<18
        _ => return Ok(()),                 // Unknown mode — use default
    };

    do_mount("", "/", "", flags, "")
}
