//! Raw syscall FFI declarations, constants, and wrappers for the OCI runtime.
//!
//! No libc crate — just direct `extern "C"` declarations and `std::os::raw`.

use std::ffi::CString;
use std::io;
use std::os::raw::{c_char, c_int, c_long, c_uint, c_ulong, c_void};

// ===========================================================================
// Raw syscall FFI
// ===========================================================================

extern "C" {
    pub fn unshare(flags: c_int) -> c_int;
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
    pub fn setuid(uid: u32) -> c_int;
    pub fn setgid(gid: u32) -> c_int;
    pub fn setgroups(size: usize, list: *const u32) -> c_int;
    pub fn prctl(option: c_int, arg2: c_ulong, arg3: c_ulong, arg4: c_ulong, arg5: c_ulong) -> c_int;
    pub fn sethostname(name: *const c_char, len: usize) -> c_int;
}

// ===========================================================================
// Constants
// ===========================================================================

/// Linux namespace clone flags (stable since 2.6).
pub mod ns {
    use super::c_int;
    pub const NEWNS: c_int     = 0x00020000;
    pub const NEWCGROUP: c_int = 0x02000000;
    pub const NEWUTS: c_int    = 0x04000000;
    pub const NEWIPC: c_int    = 0x08000000;
    pub const NEWUSER: c_int   = 0x10000000;
    pub const NEWPID: c_int    = 0x20000000;
    pub const NEWNET: c_int    = 0x40000000;
    pub const CONTAINER: c_int = NEWNS | NEWCGROUP | NEWUTS | NEWIPC | NEWPID | NEWNET;
}

/// Mount flags.
pub mod ms {
    use super::c_ulong;
    pub const NOSUID: c_ulong      = 2;
    pub const NODEV: c_ulong       = 4;
    pub const NOEXEC: c_ulong      = 8;
    pub const REC: c_ulong         = 16384;
    pub const PRIVATE: c_ulong     = 1 << 18;
    pub const BIND: c_ulong        = 4096;
    pub const RDONLY: c_ulong      = 1;
    pub const REMOUNT: c_ulong     = 1 << 14;
    pub const STRICTATIME: c_ulong = 1 << 24;
}

pub const MNT_DETACH: c_int = 2;
pub const S_IFCHR: c_uint = 0o020000;
pub const SIGTERM: c_int = 15;
pub const SIGKILL: c_int = 9;

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
    pub const SET_DUMPABLE: c_int     = 4;
    pub const PR_CAPBSET_DROP: c_int  = 24;
    pub const PR_CAP_AMBIENT: c_int   = 47;
    pub const PR_CAP_AMBIENT_IS_SET: c_int   = 1;
    pub const PR_CAP_AMBIENT_RAISE: c_int    = 2;
    pub const PR_CAP_AMBIENT_LOWER: c_int    = 3;
    pub const PR_CAP_AMBIENT_CLEAR_ALL: c_int = 4;
}

/// seccomp operations.
pub const SECCOMP_SET_MODE_FILTER: c_uint = 1;
pub const SECCOMP_FILTER_FLAG_TSYNC: c_uint = 1;

// ===========================================================================
// Syscall wrappers
// ===========================================================================

/// Call the seccomp syscall (architecture-aware).
pub fn do_seccomp(operation: c_uint, flags: c_uint, args: *const c_void) -> c_int {
    unsafe { syscall(SECCOMP_SYSCALL_NR, operation, flags, args) as c_int }
}

pub fn do_unshare(flags: c_int) -> io::Result<()> {
    let ret = unsafe { unshare(flags) };
    if ret == 0 { Ok(()) } else { Err(io::Error::last_os_error()) }
}

pub fn do_mount(source: &str, target: &str, fstype: &str, flags: c_ulong, data: &str) -> io::Result<()> {
    let s = CString::new(source).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
    let t = CString::new(target).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
    let f = CString::new(fstype).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
    let d = CString::new(data).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
    let ret = unsafe { mount(s.as_ptr(), t.as_ptr(), f.as_ptr(), flags, d.as_ptr() as *const _) };
    if ret == 0 { Ok(()) } else { Err(io::Error::last_os_error()) }
}

pub fn do_pivot_root(new_root: &str, put_old: &str) -> io::Result<()> {
    let nr = CString::new(new_root).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
    let po = CString::new(put_old).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
    let ret = unsafe { pivot_root(nr.as_ptr(), po.as_ptr()) };
    if ret == 0 { Ok(()) } else { Err(io::Error::last_os_error()) }
}

pub fn do_umount2(target: &str, flags: c_int) -> io::Result<()> {
    let t = CString::new(target).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
    let ret = unsafe { umount2(t.as_ptr(), flags) };
    if ret == 0 { Ok(()) } else { Err(io::Error::last_os_error()) }
}

pub fn do_set_hostname(name: &str) -> io::Result<()> {
    let n = CString::new(name).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
    let ret = unsafe { sethostname(n.as_ptr(), n.as_bytes().len()) };
    if ret == 0 { Ok(()) } else { Err(io::Error::last_os_error()) }
}

pub fn do_prctl_set_no_new_privs() -> io::Result<()> {
    let ret = unsafe { prctl(prctl_const::SET_NO_NEW_PRIVS, 1, 0, 0, 0) };
    if ret == 0 { Ok(()) } else { Err(io::Error::last_os_error()) }
}

pub fn do_prctl_set_dumpable(dumpable: bool) -> io::Result<()> {
    let ret = unsafe { prctl(prctl_const::SET_DUMPABLE, if dumpable { 1 } else { 0 }, 0, 0, 0) };
    if ret == 0 { Ok(()) } else { Err(io::Error::last_os_error()) }
}

/// Drop a capability from the bounding set using prctl(PR_CAPBSET_DROP).
/// Returns Ok(()) on success, Err if the prctl call failed.
pub fn do_prctl_cap_bset_drop(cap_name: &str) -> io::Result<()> {
    let cap = cap_name_to_int(cap_name);
    let ret = unsafe { prctl(prctl_const::PR_CAPBSET_DROP, cap as c_ulong, 0, 0, 0) };
    if ret == 0 { Ok(()) } else { Err(io::Error::last_os_error()) }
}

/// Convert a CAP_* string name to its integer value.
/// Returns the numeric value for known capabilities, or u32::MAX for unknown.
pub fn cap_name_to_int(name: &str) -> u32 {
    match name.strip_prefix("CAP_").unwrap_or(name) {
        "CHOWN" => 0,
        "DAC_OVERRIDE" => 1,
        "DAC_READ_SEARCH" => 2,
        "FOWNER" => 3,
        "FSETID" => 4,
        "KILL" => 5,
        "SETGID" => 6,
        "SETUID" => 7,
        "SETPCAP" => 8,
        "LINUX_IMMUTABLE" => 9,
        "NET_BIND_SERVICE" => 10,
        "NET_BROADCAST" => 11,
        "NET_ADMIN" => 12,
        "NET_RAW" => 13,
        "IPC_LOCK" => 14,
        "IPC_OWNER" => 15,
        "SYS_MODULE" => 16,
        "SYS_RAWIO" => 17,
        "SYS_CHROOT" => 18,
        "SYS_PTRACE" => 19,
        "SYS_PACCT" => 20,
        "SYS_ADMIN" => 21,
        "SYS_BOOT" => 22,
        "SYS_NICE" => 23,
        "SYS_RESOURCE" => 24,
        "SYS_TIME" => 25,
        "SYS_TTY_CONFIG" => 26,
        "MKNOD" => 27,
        "LEASE" => 28,
        "AUDIT_WRITE" => 29,
        "AUDIT_CONTROL" => 30,
        "SETFCAP" => 31,
        _ => u32::MAX, // Unknown — skip
    }
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
    // cap_data structure for version 3:
    // [0]: version (u32) = 0x20080522
    // [1]: pid (i32) = 0 (current process)
    // [2]: effective (u32)
    // [3]: permitted (u32)
    // [4]: inheritable low (u32)
    // [5]: inheritable high (u32)
    const CAP_VERSION: u32 = 0x20080522;

    let mut data: [u32; 6] = [0; 6];
    data[0] = CAP_VERSION;
    data[1] = 0; // pid = current process
    data[2] = effective as u32;
    data[3] = permitted as u32;
    data[4] = inheritable as u32;
    data[5] = (inheritable >> 32) as u32;

    let ret = unsafe { syscall(CAPSET_SYSCALL_NR, &data as *const _ as *const c_void) as c_int };
    if ret == 0 { Ok(()) } else { Err(io::Error::last_os_error()) }
}

/// Set/clear an ambient capability via prctl.
pub fn do_prctl_cap_ambient(action: c_int, cap: c_int) -> io::Result<()> {
    let ret = unsafe { prctl(prctl_const::PR_CAP_AMBIENT, action as c_ulong, cap as c_ulong, 0, 0) };
    if ret == 0 { Ok(()) } else { Err(io::Error::last_os_error()) }
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
        syscall(302, 0, resource, &new_rlim as *const _, 0 as *mut u64) as c_int
    };
    #[cfg(target_arch = "aarch64")]
    let ret = unsafe {
        syscall(267, 0, resource, &new_rlim as *const _, 0 as *mut u64) as c_int
    };
    if ret == 0 { Ok(()) } else { Err(io::Error::last_os_error()) }
}

/// Parse an RLIMIT_* name to its numeric constant.
pub fn rlimit_name_to_int(name: &str) -> Option<u32> {
    // RLIMIT_* constants on Linux (x86_64/aarch64)
    match name.strip_prefix("RLIMIT_").unwrap_or(name) {
        "CPU"        => Some(0),
        "FSIZE"      => Some(1),
        "DATA"       => Some(2),
        "STACK"      => Some(3),
        "CORE"       => Some(4),
        "RSS"        => Some(5),
        "NPROC"      => Some(6),
        "NOFILE"     => Some(7),
        "MEMLOCK"    => Some(8),
        "AS"         => Some(9),
        "LOCKS"      => Some(10),
        "SIGPENDING" => Some(11),
        "MSGQUEUE"   => Some(12),
        "NICE"       => Some(13),
        "RTPRIO"     => Some(14),
        "RTTIME"     => Some(15),
        _ => None,
    }
}
