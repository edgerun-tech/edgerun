//! OCI idmapped mount setup.

use crate::libc;
use crate::prelude::*;
use std::ffi::CString;
use std::io;
use std::os::raw::c_int;

use crate::spec::OciIdMapping;
use crate::syscalls::{
    do_mount_setattr, do_move_mount, do_open_tree, mount_attr, move_mount, open_tree, MountAttr,
};

fn c_string(value: &str) -> io::Result<CString> {
    CString::new(value).map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))
}

fn map_entries(mappings: &[OciIdMapping]) -> String {
    mappings
        .iter()
        .map(|m| format!("{} {} {}\n", m.container_id, m.host_id, m.size))
        .collect()
}

pub(crate) fn setup_idmapped_mount(
    dest: &str,
    uid_mappings: &[OciIdMapping],
    gid_mappings: Option<&[OciIdMapping]>,
) -> io::Result<()> {
    let uid_map_str = map_entries(uid_mappings);
    let gid_map_str = gid_mappings.map(map_entries).unwrap_or_default();

    let mut child_ready: [c_int; 2] = [-1, -1];
    let mut parent_done: [c_int; 2] = [-1, -1];
    if unsafe { libc::pipe(child_ready.as_mut_ptr()) } != 0
        || unsafe { libc::pipe(parent_done.as_mut_ptr()) } != 0
    {
        close_pipe(child_ready);
        close_pipe(parent_done);
        return Err(io::Error::last_os_error());
    }

    let pid = unsafe { libc::fork() };
    if pid < 0 {
        close_pipe(child_ready);
        close_pipe(parent_done);
        return Err(io::Error::last_os_error());
    }

    if pid == 0 {
        run_userns_mapping_child(child_ready, parent_done, &uid_map_str, &gid_map_str);
    }

    apply_idmap_from_child_userns(pid, child_ready, parent_done, dest)
}

fn run_userns_mapping_child(
    child_ready: [c_int; 2],
    parent_done: [c_int; 2],
    uid_map_str: &str,
    gid_map_str: &str,
) -> ! {
    unsafe { libc::close(child_ready[0]) };
    unsafe { libc::close(parent_done[1]) };

    if unsafe { libc::unshare(libc::CLONE_NEWUSER) } != 0
        || std::fs::write("/proc/self/uid_map", uid_map_str).is_err()
    {
        signal_fd(child_ready[1], b'E');
        unsafe { libc::_exit(1) };
    }

    let _ = std::fs::write("/proc/self/setgroups", "deny");
    if !gid_map_str.is_empty() && std::fs::write("/proc/self/gid_map", gid_map_str).is_err() {
        signal_fd(child_ready[1], b'E');
        unsafe { libc::_exit(1) };
    }

    signal_fd(child_ready[1], b'R');
    let mut buf = [0u8; 1];
    let _ = unsafe { libc::read(parent_done[0], buf.as_mut_ptr() as *mut _, 1) };

    unsafe { libc::close(child_ready[1]) };
    unsafe { libc::close(parent_done[0]) };
    unsafe { libc::_exit(0) };
}

fn apply_idmap_from_child_userns(
    pid: c_int,
    child_ready: [c_int; 2],
    parent_done: [c_int; 2],
    dest: &str,
) -> io::Result<()> {
    unsafe { libc::close(child_ready[1]) };
    unsafe { libc::close(parent_done[0]) };

    let mut buf = [0u8; 1];
    let n = unsafe { libc::read(child_ready[0], buf.as_mut_ptr() as *mut _, 1) };
    unsafe { libc::close(child_ready[0]) };

    if n != 1 || buf[0] != b'R' {
        unsafe { libc::waitpid(pid, std::ptr::null_mut(), 0) };
        unsafe { libc::close(parent_done[1]) };
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "failed to create user namespace for idmapped mount",
        ));
    }

    let result = apply_idmap_mount_attr(pid, dest);

    signal_fd(parent_done[1], b'D');
    unsafe { libc::close(parent_done[1]) };
    unsafe { libc::waitpid(pid, std::ptr::null_mut(), 0) };

    result
}

fn apply_idmap_mount_attr(pid: c_int, dest: &str) -> io::Result<()> {
    let userns_path = c_string(&format!("/proc/{pid}/ns/user"))?;
    let userns_fd = unsafe { libc::open(userns_path.as_ptr(), libc::O_RDONLY | libc::O_CLOEXEC) };
    if userns_fd < 0 {
        return Err(io::Error::last_os_error());
    }

    let tree_fd = match do_open_tree(libc::AT_FDCWD, dest, open_tree::CLONE | open_tree::CLOEXEC) {
        Ok(fd) => fd,
        Err(_) => {
            unsafe { libc::close(userns_fd) };
            return Ok(());
        }
    };

    let attr = MountAttr {
        attr_set: mount_attr::IDMAP,
        attr_clr: 0,
        propagation: 0,
        userns_fd: userns_fd as u64,
    };
    let result = do_mount_setattr(tree_fd, "", &attr, move_mount::T_EMPTY_PATH);
    if result.is_ok() {
        let _ = do_move_mount(tree_fd, "", libc::AT_FDCWD, dest, move_mount::F_EMPTY_PATH);
    }

    unsafe { libc::close(tree_fd) };
    unsafe { libc::close(userns_fd) };
    result
}

fn signal_fd(fd: c_int, byte: u8) {
    let _ = unsafe { libc::write(fd, &byte as *const _ as *const libc::c_void, 1) };
}

fn close_pipe(pipe: [c_int; 2]) {
    for fd in pipe {
        if fd >= 0 {
            unsafe { libc::close(fd) };
        }
    }
}
