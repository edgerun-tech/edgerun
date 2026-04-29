//! Raw syscall FFI declarations, constants, and wrappers for the OCI runtime.
//!
//! No libc crate — just direct `extern "C"` declarations and `std::os::raw`.

use crate::prelude::*;
use std::ffi::CString;
use std::io;
use std::os::raw::{c_char, c_int, c_long, c_uint, c_ulong, c_void};

use crate::linux_catalog::capability_number;

// ===========================================================================
// Raw syscall FFI
// ===========================================================================

extern "C" {
    pub fn syscall(number: c_long, ...) -> c_long;
    pub fn kill(pid: c_int, sig: c_int) -> c_int;
    pub fn mount(
        source: *const c_char,
        target: *const c_char,
        fstype: *const c_char,
        flags: c_ulong,
        data: *const c_void,
    ) -> c_int;
    pub fn pivot_root(new_root: *const c_char, put_old: *const c_char) -> c_int;
    pub fn umount2(target: *const c_char, flags: c_int) -> c_int;
    pub fn mknod(path: *const c_char, mode: c_uint, dev: c_uint) -> c_int;
    pub fn chown(path: *const c_char, owner: u32, group: u32) -> c_int;
    pub fn setuid(uid: u32) -> c_int;
    pub fn setgid(gid: u32) -> c_int;
    pub fn setgroups(size: usize, list: *const u32) -> c_int;
    pub fn prctl(
        option: c_int,
        arg2: c_ulong,
        arg3: c_ulong,
        arg4: c_ulong,
        arg5: c_ulong,
    ) -> c_int;
    pub fn sethostname(name: *const c_char, len: usize) -> c_int;
}

// ===========================================================================
// Constants
// ===========================================================================

/// Linux namespace clone flags (stable since 2.6).
pub mod ns {
    use super::c_int;
    pub const NEWNS: c_int = 0x00020000;
    pub const NEWCGROUP: c_int = 0x02000000;
    pub const NEWUTS: c_int = 0x04000000;
    pub const NEWIPC: c_int = 0x08000000;
    pub const NEWUSER: c_int = 0x10000000;
    pub const NEWPID: c_int = 0x20000000;
    pub const NEWNET: c_int = 0x40000000;
    pub const CONTAINER: c_int = NEWNS | NEWCGROUP | NEWUTS | NEWIPC | NEWPID | NEWNET;
}

/// Mount flags.
pub mod ms {
    use super::c_ulong;
    pub const NOSUID: c_ulong = 2;
    pub const NODEV: c_ulong = 4;
    pub const NOEXEC: c_ulong = 8;
    pub const REC: c_ulong = 16384;
    pub const PRIVATE: c_ulong = 1 << 18;
    pub const BIND: c_ulong = 4096;
    pub const RDONLY: c_ulong = 1;
    pub const REMOUNT: c_ulong = 1 << 14;
    pub const STRICTATIME: c_ulong = 1 << 24;
    pub const SHARED: c_ulong = 1 << 20;
    pub const SLAVE: c_ulong = 1 << 19;
    pub const UNBINDABLE: c_ulong = 1 << 21;
}

pub const MNT_DETACH: c_int = 2;
pub const S_IFCHR: c_uint = 0o020000;
pub const SIGTERM: c_int = 15;
pub const SIGKILL: c_int = 9;

// ===========================================================================
// New mount API (Linux 5.6+) — open_tree, move_mount, mount_setattr
// ===========================================================================

/// open_tree(2) syscall number (same on x86_64 and aarch64).
#[cfg(target_arch = "x86_64")]
pub const SYS_OPEN_TREE: i64 = 428;
#[cfg(target_arch = "aarch64")]
pub const SYS_OPEN_TREE: i64 = 428;

/// move_mount(2) syscall number (same on x86_64 and aarch64).
#[cfg(target_arch = "x86_64")]
pub const SYS_MOVE_MOUNT: i64 = 429;
#[cfg(target_arch = "aarch64")]
pub const SYS_MOVE_MOUNT: i64 = 429;

/// mount_setattr(2) syscall number (same on x86_64 and aarch64).
#[cfg(target_arch = "x86_64")]
pub const SYS_MOUNT_SETATTR: i64 = 442;
#[cfg(target_arch = "aarch64")]
pub const SYS_MOUNT_SETATTR: i64 = 442;

/// open_tree flags.
pub mod open_tree {
    use super::c_uint;
    pub const CLOEXEC: c_uint = 0x001;
    pub const CLONE: c_uint = 0x002;
}

/// move_mount flags.
pub mod move_mount {
    use super::c_uint;
    pub const F_EMPTY_PATH: c_uint = 0x0100;
    pub const T_EMPTY_PATH: c_uint = 0x0200;
}

/// mount_setattr flags (MOUNT_ATTR_*).
pub mod mount_attr {
    use super::c_ulong;
    pub const RDONLY: c_ulong = 0x00000001;
    pub const NOSUID: c_ulong = 0x00000002;
    pub const NODEV: c_ulong = 0x00000004;
    pub const NOEXEC: c_ulong = 0x00000008;
    pub const REC: c_ulong = 0x00001000; // Apply recursively to sub-mounts
    pub const IDMAP: c_ulong = 0x00100000; // Idmapped mount (Linux 5.12+)
}

/// mount_attr structure for mount_setattr(2).
#[repr(C)]
pub struct MountAttr {
    pub attr_set: u64,
    pub attr_clr: u64,
    pub propagation: u64,
    pub userns_fd: u64,
}

fn c_string(value: &str) -> io::Result<CString> {
    CString::new(value).map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))
}

fn syscall_unit(ret: c_int) -> io::Result<()> {
    if ret == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

/// Open a mount tree at the given path.
///
/// Returns a file descriptor referencing the mount. Use with `move_mount` or `mount_setattr`.
pub fn do_open_tree(dirfd: c_int, pathname: &str, flags: c_uint) -> io::Result<c_int> {
    let path_c = c_string(pathname)?;
    let ret = unsafe { syscall(SYS_OPEN_TREE, dirfd, path_c.as_ptr(), flags) as c_int };
    if ret < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(ret)
    }
}

/// Move a mount from one location to another.
///
/// `from_dfd`/`from_path` is the source mount (from `open_tree` or AT_FDCWD).
/// `to_dfd`/`to_path` is the destination path.
pub fn do_move_mount(
    from_dfd: c_int,
    from_path: &str,
    to_dfd: c_int,
    to_path: &str,
    flags: c_uint,
) -> io::Result<()> {
    let from_c = c_string(from_path)?;
    let to_c = c_string(to_path)?;
    let ret = unsafe {
        syscall(
            SYS_MOVE_MOUNT,
            from_dfd,
            from_c.as_ptr(),
            to_dfd,
            to_c.as_ptr(),
            flags,
        ) as c_int
    };
    syscall_unit(ret)
}

/// Apply mount attributes recursively.
///
/// Used for `mount.recursive` (OCI 1.1) — sets/clears mount flags on all sub-mounts.
pub fn do_mount_setattr(
    dirfd: c_int,
    path: &str,
    attr: &MountAttr,
    flags: c_uint,
) -> io::Result<()> {
    let path_c = c_string(path)?;
    let ret = unsafe {
        syscall(
            SYS_MOUNT_SETATTR,
            dirfd,
            path_c.as_ptr(),
            flags,
            attr as *const _ as u64,
            std::mem::size_of::<MountAttr>() as u64,
        ) as c_int
    };
    syscall_unit(ret)
}

/// Seccomp syscall numbers per architecture.
#[cfg(target_arch = "x86_64")]
pub const SECCOMP_SYSCALL_NR: c_long = 317;

#[cfg(target_arch = "aarch64")]
pub const SECCOMP_SYSCALL_NR: c_long = 277; // __NR_seccomp on arm64

/// capset syscall numbers per architecture.
#[cfg(target_arch = "x86_64")]
pub const CAPSET_SYSCALL_NR: c_long = 126;

#[cfg(target_arch = "aarch64")]
pub const CAPSET_SYSCALL_NR: c_long = 94;

/// prctl options.
pub mod prctl_const {
    use super::c_int;
    pub const SET_NO_NEW_PRIVS: c_int = 38;
    pub const SET_DUMPABLE: c_int = 4;
    pub const PR_CAPBSET_DROP: c_int = 24;
    pub const PR_CAP_AMBIENT: c_int = 47;
    pub const PR_CAP_AMBIENT_IS_SET: c_int = 1;
    pub const PR_CAP_AMBIENT_RAISE: c_int = 2;
    pub const PR_CAP_AMBIENT_LOWER: c_int = 3;
    pub const PR_CAP_AMBIENT_CLEAR_ALL: c_int = 4;
}

/// Set process umask.
pub fn do_umask(mask: u32) -> u32 {
    // umask syscall number: x86_64=95, aarch64=166
    #[cfg(target_arch = "x86_64")]
    let ret = unsafe { syscall(95, mask) as u32 };
    #[cfg(target_arch = "aarch64")]
    let ret = unsafe { syscall(166, mask) as u32 };
    ret
}

/// seccomp operations.
pub const SECCOMP_SET_MODE_FILTER: c_uint = 1;
pub const SECCOMP_FILTER_FLAG_TSYNC: c_uint = 1;
pub const SECCOMP_FILTER_FLAG_NEW_LISTENER: c_uint = 8;

// ===========================================================================
// Syscall wrappers
// ===========================================================================

/// Call the seccomp syscall (architecture-aware).
///
/// # Safety
///
/// `args` must be a valid pointer (or null) for the seccomp operation.
pub unsafe fn do_seccomp(operation: c_uint, flags: c_uint, args: *const c_void) -> c_int {
    syscall(SECCOMP_SYSCALL_NR, operation, flags, args) as c_int
}

pub fn do_unshare(flags: c_int) -> io::Result<()> {
    // Use raw syscall to avoid potential libc wrapper issues after fork
    #[cfg(target_arch = "x86_64")]
    let ret = unsafe { syscall(272, flags) as c_int }; // __NR_unshare
    #[cfg(target_arch = "aarch64")]
    let ret = unsafe { syscall(97, flags) as c_int }; // __NR_unshare
    syscall_unit(ret)
}

pub fn do_mount(
    source: &str,
    target: &str,
    fstype: &str,
    flags: c_ulong,
    data: &str,
) -> io::Result<()> {
    let s = c_string(source)?;
    let t = c_string(target)?;
    let f = c_string(fstype)?;
    let d = c_string(data)?;
    let ret = unsafe {
        mount(
            s.as_ptr(),
            t.as_ptr(),
            f.as_ptr(),
            flags,
            d.as_ptr() as *const _,
        )
    };
    syscall_unit(ret)
}

pub fn do_pivot_root(new_root: &str, put_old: &str) -> io::Result<()> {
    let nr = c_string(new_root)?;
    let po = c_string(put_old)?;
    let ret = unsafe { pivot_root(nr.as_ptr(), po.as_ptr()) };
    syscall_unit(ret)
}

pub fn do_umount2(target: &str, flags: c_int) -> io::Result<()> {
    let t = c_string(target)?;
    let ret = unsafe { umount2(t.as_ptr(), flags) };
    syscall_unit(ret)
}

pub fn do_set_hostname(name: &str) -> io::Result<()> {
    let n = c_string(name)?;
    let ret = unsafe { sethostname(n.as_ptr(), n.as_bytes().len()) };
    syscall_unit(ret)
}

pub fn do_prctl_set_no_new_privs() -> io::Result<()> {
    let ret = unsafe { prctl(prctl_const::SET_NO_NEW_PRIVS, 1, 0, 0, 0) };
    syscall_unit(ret)
}

pub fn do_prctl_set_dumpable(dumpable: bool) -> io::Result<()> {
    let ret = unsafe {
        prctl(
            prctl_const::SET_DUMPABLE,
            if dumpable { 1 } else { 0 },
            0,
            0,
            0,
        )
    };
    syscall_unit(ret)
}

/// Drop a capability from the bounding set using prctl(PR_CAPBSET_DROP).
/// Returns Ok(()) on success, Err if the prctl call failed.
pub fn do_prctl_cap_bset_drop(cap_name: &str) -> io::Result<()> {
    let cap = capability_number(cap_name).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid capability name: {}", cap_name),
        )
    })?;
    let ret = unsafe { prctl(prctl_const::PR_CAPBSET_DROP, cap as c_ulong, 0, 0, 0) };
    syscall_unit(ret)
}

pub fn makedev(major: u64, minor: u64) -> c_uint {
    ((major & 0xfff) << 8 | (minor & 0xff) | ((minor & 0xfff00) << 12)) as c_uint
}

/// Set process capabilities via capset syscall.
///
/// The `effective`, `permitted`, and `inheritable` arguments are bitmasks
/// of capability indices (each bit = one cap, max 64 caps).
///
/// This must be called BEFORE dropping privileges (setuid/setgid).
pub fn do_capset(effective: u64, permitted: u64, inheritable: u64) -> io::Result<()> {
    #[repr(C)]
    struct CapHeader {
        version: u32,
        pid: i32,
    }

    #[repr(C)]
    struct CapData {
        effective: u32,
        permitted: u32,
        inheritable: u32,
    }

    const CAP_VERSION: u32 = 0x20080522;

    let header = CapHeader {
        version: CAP_VERSION,
        pid: 0,
    };
    let data = [
        CapData {
            effective: effective as u32,
            permitted: permitted as u32,
            inheritable: inheritable as u32,
        },
        CapData {
            effective: (effective >> 32) as u32,
            permitted: (permitted >> 32) as u32,
            inheritable: (inheritable >> 32) as u32,
        },
    ];

    let ret = unsafe {
        syscall(
            CAPSET_SYSCALL_NR,
            &header as *const _ as *const c_void,
            data.as_ptr() as *const c_void,
        ) as c_int
    };
    syscall_unit(ret)
}

/// Set/clear an ambient capability via prctl.
pub fn do_prctl_cap_ambient(action: c_int, cap: c_int) -> io::Result<()> {
    let ret = unsafe {
        prctl(
            prctl_const::PR_CAP_AMBIENT,
            action as c_ulong,
            cap as c_ulong,
            0,
            0,
        )
    };
    syscall_unit(ret)
}

/// Set a resource limit using prlimit64 syscall.
///
/// `resource` is the RLIMIT_* constant (e.g., RLIMIT_NOFILE=7).
/// `soft` and `hard` are the limits.
pub fn do_setrlimit(resource: u32, soft: u64, hard: u64) -> io::Result<()> {
    // prlimit64(pid, resource, new_rlimit, old_rlimit)
    // rlimit64 struct: { rlim_cur: u64, rlim_max: u64 }
    let new_rlim: [u64; 2] = [soft, hard];

    #[cfg(target_arch = "x86_64")]
    let ret = unsafe {
        syscall(
            302,
            0,
            resource,
            &new_rlim as *const _,
            std::ptr::null_mut::<u64>(),
        ) as c_int
    };
    #[cfg(target_arch = "aarch64")]
    let ret = unsafe {
        syscall(
            267,
            0,
            resource,
            &new_rlim as *const _,
            std::ptr::null_mut::<u64>(),
        ) as c_int
    };
    syscall_unit(ret)
}

/// Set a namespace via setns syscall.
/// `fd` is a file descriptor for the namespace file (e.g., from opening /proc/PID/ns/mnt).
/// `ns_type` is one of the CLONE_NEW* flags.
///
/// Returns 0 on success, -1 on error.
pub fn do_setns(fd: c_int, ns_type: c_int) -> io::Result<()> {
    // setns syscall number: x86_64=308, aarch64=268
    #[cfg(target_arch = "x86_64")]
    let ret = unsafe { syscall(308, fd, ns_type) as c_int };
    #[cfg(target_arch = "aarch64")]
    let ret = unsafe { syscall(268, fd, ns_type) as c_int };
    syscall_unit(ret)
}

// ===========================================================================
// eBPF program constants and syscalls
// ===========================================================================

/// eBPF instruction encoding constants (kernel include/uapi/linux/bpf.h).
pub mod ebpf_insn {
    // eBPF instruction classes
    pub const BPF_LD: u8 = 0x00;
    pub const BPF_LDX: u8 = 0x01;
    pub const BPF_ST: u8 = 0x02;
    pub const BPF_STX: u8 = 0x03;
    pub const BPF_ALU: u8 = 0x04;
    pub const BPF_JMP: u8 = 0x05;
    pub const BPF_RET: u8 = 0x06;
    pub const BPF_ALU64: u8 = 0x07;

    // eBPF source/destination modifiers
    pub const BPF_K: u8 = 0x00; // immediate
    pub const BPF_X: u8 = 0x08; // register

    // eBPF size modifiers (for LD/STX/LDX)
    pub const BPF_W: u8 = 0x00; // 32-bit word
    pub const BPF_H: u8 = 0x08; // 16-bit half-word
    pub const BPF_B: u8 = 0x10; // 8-bit byte
    pub const BPF_DW: u8 = 0x18; // 64-bit double-word

    // eBPF mode modifiers (for LD/ST)
    pub const BPF_IMM: u8 = 0x00;
    pub const BPF_MEM: u8 = 0x60;

    // eBPF ALU/ALU64 opcodes
    pub const BPF_MOV: u8 = 0xb0; // BPF_ALU64 | BPF_MOV | BPF_K

    // eBPF JMP opcodes
    pub const BPF_JEQ: u8 = 0x10;
    pub const BPF_JNE: u8 = 0x50;
    pub const BPF_JGT: u8 = 0x20;
    pub const BPF_JGE: u8 = 0x30;
    pub const BPF_JSGT: u8 = 0x60;
    pub const BPF_JSGE: u8 = 0x70;
    pub const BPF_EXIT: u8 = 0x90;
}

/// eBPF JMP instruction constants (alias for compatibility).
pub mod bpf_jmp {
    pub use crate::syscalls::ebpf_insn::BPF_EXIT;
    pub use crate::syscalls::ebpf_insn::BPF_JEQ;
    pub use crate::syscalls::ebpf_insn::BPF_JGE;
    pub use crate::syscalls::ebpf_insn::BPF_JGT;
    pub use crate::syscalls::ebpf_insn::BPF_JNE;
}

/// eBPF size constants (alias for compatibility).
pub mod bpf_size {
    pub use crate::syscalls::ebpf_insn::BPF_B;
    pub use crate::syscalls::ebpf_insn::BPF_DW;
    pub use crate::syscalls::ebpf_insn::BPF_H;
    pub use crate::syscalls::ebpf_insn::BPF_W;
}

/// eBPF program types (kernel include/uapi/linux/bpf.h).
pub mod bpf_prog_type {
    pub const BPF_PROG_TYPE_UNSPEC: u32 = 0;
    pub const BPF_PROG_TYPE_SOCKET_FILTER: u32 = 1;
    pub const BPF_PROG_TYPE_KPROBE: u32 = 2;
    pub const BPF_PROG_TYPE_SCHED_CLS: u32 = 3;
    pub const BPF_PROG_TYPE_SCHED_ACT: u32 = 4;
    pub const BPF_PROG_TYPE_CGROUP_SKB: u32 = 8;
    pub const BPF_PROG_TYPE_CGROUP_SOCK: u32 = 9;
    pub const BPF_PROG_TYPE_CGROUP_DEVICE: u32 = 15;
}

/// eBPF attach types (kernel include/uapi/linux/bpf.h).
pub mod bpf_attach_type {
    pub const BPF_CGROUP_INET_INGRESS: u32 = 0;
    pub const BPF_CGROUP_INET_EGRESS: u32 = 1;
    pub const BPF_CGROUP_DEVICE: u32 = 6;
}

/// eBPF return value for cgroup device programs (allow device access).
pub const BPF_CGROUP_DEV_ALLOW: i32 = 1;

/// Build an eBPF instruction (8 bytes, kernel bpf_insn format).
///
/// Layout:
///   code: u8  — opcode
///   dst_reg: u4, src_reg: u4 — packed in byte 1
///   off: i16 — little-endian bytes 2-3
///   imm: i32 — little-endian bytes 4-7
pub fn ebpf_insn(code: u8, dst: u8, src: u8, off: i16, imm: i32) -> [u8; 8] {
    let mut buf = [0u8; 8];
    buf[0] = code;
    buf[1] = (dst & 0x0f) | ((src & 0x0f) << 4);
    buf[2..4].copy_from_slice(&off.to_le_bytes());
    buf[4..8].copy_from_slice(&imm.to_le_bytes());
    buf
}

/// eBPF syscall numbers.
#[cfg(target_arch = "x86_64")]
const SYS_BPF: i64 = 321;
#[cfg(target_arch = "aarch64")]
const SYS_BPF: i64 = 280;

/// Attributes for BPF_PROG_LOAD.
#[repr(C)]
struct BpfProgLoadAttr {
    prog_type: u32,
    insn_cnt: u32,
    insns: u64,
    license: u64,
    log_level: u32,
    log_size: u32,
    log_buf: u64,
    kern_version: u32,
    prog_flags: u32,
    _padding: [u32; 4],
}

/// Load an eBPF program into the kernel.
///
/// Convenience wrapper: takes a slice of 8-byte instructions and a license string.
pub fn bpf_prog_load(
    prog_type: u32,
    insns: &[[u8; 8]],
    license: &str,
    _attach_type: u32,
) -> std::io::Result<i32> {
    let insn_cnt = insns.len() as u32;
    let license_c = std::ffi::CString::new(license).unwrap();
    let mut log_buf = vec![0u8; 64 * 1024];

    let attr = BpfProgLoadAttr {
        prog_type,
        insn_cnt,
        insns: insns.as_ptr() as u64,
        license: license_c.as_ptr() as u64,
        log_level: 1,
        log_size: log_buf.len() as u32,
        log_buf: log_buf.as_mut_ptr() as u64,
        kern_version: 0,
        prog_flags: 0,
        _padding: [0; 4],
    };

    let ret = unsafe {
        libc::syscall(
            SYS_BPF,
            5i64, /* BPF_PROG_LOAD */
            &attr as *const _ as libc::c_ulong,
            std::mem::size_of::<BpfProgLoadAttr>(),
        )
    };
    if ret < 0 {
        let error = std::io::Error::last_os_error();
        let nul = log_buf
            .iter()
            .position(|b| *b == 0)
            .unwrap_or(log_buf.len());
        let verifier_log = String::from_utf8_lossy(&log_buf[..nul]).trim().to_string();
        if verifier_log.is_empty() {
            Err(error)
        } else {
            Err(std::io::Error::new(
                error.kind(),
                format!("{}: {}", error, verifier_log),
            ))
        }
    } else {
        Ok(ret as i32)
    }
}

/// Attach an eBPF program to a cgroup.
pub fn bpf_prog_attach(cgroup_fd: i32, prog_fd: i32, attach_type: u32) -> std::io::Result<()> {
    #[repr(C)]
    struct BpfProgAttachAttr {
        target_fd: i32,
        attach_bpf_fd: i32,
        attach_type: u32,
        attach_flags: u32,
    }

    let attr = BpfProgAttachAttr {
        target_fd: cgroup_fd,
        attach_bpf_fd: prog_fd,
        attach_type,
        attach_flags: 0,
    };

    let ret = unsafe {
        libc::syscall(
            SYS_BPF,
            8i64, /* BPF_PROG_ATTACH */
            &attr as *const _ as libc::c_ulong,
            std::mem::size_of::<BpfProgAttachAttr>(),
        )
    };
    if ret < 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

/// Detach an eBPF program from a cgroup.
pub fn bpf_prog_detach(cgroup_fd: i32, prog_fd: i32, attach_type: u32) -> std::io::Result<()> {
    #[repr(C)]
    struct BpfProgDetachAttr {
        target_fd: i32,
        attach_bpf_fd: i32,
        attach_type: u32,
    }

    let attr = BpfProgDetachAttr {
        target_fd: cgroup_fd,
        attach_bpf_fd: prog_fd,
        attach_type,
    };

    let ret = unsafe {
        libc::syscall(
            SYS_BPF,
            15i64, /* BPF_PROG_DETACH */
            &attr as *const _ as libc::c_ulong,
            std::mem::size_of::<BpfProgDetachAttr>(),
        )
    };
    if ret < 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

// eBPF register helpers used by ebpf_devices/ebpf_netcls

/// BPF_STX | BPF_MEM | size: *(size *)(dst + off) = src
pub fn st_imm(size: u8, dst: u8, src: u8, off: i16) -> [u8; 8] {
    ebpf_insn(0x03 | size | 0x60, dst, src, off, 0)
}

/// BPF_LDX | BPF_MEM | size: dst = *(size *)(src + off)
pub fn ld_imm(size: u8, dst: u8, src: u8, off: i16) -> [u8; 8] {
    ebpf_insn(0x01 | size | 0x60, dst, src, off, 0)
}

/// BPF_ALU64 | BPF_MOV | BPF_K: dst = imm
pub fn mov_imm(dst: u8, imm: i32) -> [u8; 8] {
    ebpf_insn(0xb7, dst, 0, 0, imm)
}

/// BPF_ALU64 | BPF_MOV | BPF_X: dst = src
pub fn mov_reg(dst: u8, src: u8) -> [u8; 8] {
    ebpf_insn(0xbf, dst, src, 0, 0)
}

/// BPF_ALU64 | BPF_AND | BPF_K: dst &= imm
pub fn and_imm(dst: u8, imm: i32) -> [u8; 8] {
    ebpf_insn(0x57, dst, 0, 0, imm)
}

/// BPF_JMP | BPF_JNE | BPF_K: if dst != imm then jt else jf
pub fn jmp_imm(jmp: u8, dst: u8, imm: i32, _jt: u8) -> [u8; 8] {
    ebpf_insn(0x05 | jmp, dst, 0, 0, imm)
        .into_iter()
        .enumerate()
        .fold([0u8; 8], |mut arr, (i, b)| {
            arr[i] = b;
            arr
        })
}

/// BPF_JMP | BPF_EXIT: return
pub fn exit() -> [u8; 8] {
    ebpf_insn(0x95, 0, 0, 0, 0)
}

/// R6 register number.
pub const R0: u8 = 0;
/// R6 register number (callee-saved).
pub const R6: u8 = 6;
/// R7 register number (callee-saved).
pub const R7: u8 = 7;
/// R1 register number (first argument / context pointer).
pub const R1: u8 = 1;
/// R2 register number (second argument / temp).
pub const R2: u8 = 2;

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(all(test, not(target_os = "none")))]
#[path = "../tests/unit_src/src/syscalls_tests.rs"]
mod tests;
