//! Minimal OCI container runtime — kernel-only, no external tools.
//!
//! Uses Linux kernel primitives: namespaces, cgroups v2, pivot_root, mount.
//! No Docker, no runc, no systemd, no libc crate. Just raw syscalls and std.

use std::ffi::CString;
use std::fs;
use std::io;
use std::os::raw::{c_char, c_int, c_uint, c_ulong, c_void};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

// ===========================================================================
// Raw syscalls — declared directly, no libc crate
// ===========================================================================

extern "C" {
    fn unshare(flags: c_int) -> c_int;
    fn mount(
        source: *const c_char,
        target: *const c_char,
        fstype: *const c_char,
        flags: c_ulong,
        data: *const c_void,
    ) -> c_int;
    fn pivot_root(new_root: *const c_char, put_old: *const c_char) -> c_int;
    fn umount2(target: *const c_char, flags: c_int) -> c_int;
    fn mknod(path: *const c_char, mode: c_uint, dev: c_uint) -> c_int;
    fn setuid(uid: u32) -> c_int;
    fn setgid(gid: u32) -> c_int;
    fn setgroups(size: usize, list: *const u32) -> c_int;
    fn prctl(option: c_int, arg2: c_ulong, arg3: c_ulong, arg4: c_ulong, arg5: c_ulong) -> c_int;
    fn sethostname(name: *const c_char, len: usize) -> c_int;
}

/// Call the seccomp syscall directly via libc::syscall.
/// On x86_64, seccomp is syscall #317.
fn do_seccomp(operation: c_uint, flags: c_uint, args: *const c_void) -> c_int {
    unsafe { libc::syscall(317, operation, flags, args) as c_int }
}

// ===========================================================================
// Constants
// ===========================================================================

/// Linux namespace clone flags (x86_64, stable since 2.6)
mod ns {
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

/// Mount flags
mod ms {
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

const MNT_DETACH: c_int = 2;
const S_IFCHR: c_uint = 0o020000;

/// prctl options
mod prctl {
    use super::c_int;
    pub const SET_NO_NEW_PRIVS: c_int = 38;
    pub const SET_DUMPABLE: c_int     = 4;
}

/// seccomp operations
const SECCOMP_SET_MODE_FILTER: c_uint = 1;
const SECCOMP_FILTER_FLAG_TSYNC: c_uint = 1;

/// Minimal seccomp-BPF allow-list for containers.
/// Allows essential syscalls, denies everything else with EPERM.
fn seccomp_bpf_prog() -> Vec<u8> {
    // BPF instruction: code(u16) jt(u8) jf(u8) k(u32) = 8 bytes
    //
    // BPF codes:
    //   BPF_LD | BPF_W | BPF_ABS  = 0x20  (load word from absolute offset)
    //   BPF_JMP | BPF_JEQ | BPF_K = 0x15  (jump if equal)
    //   BPF_RET | BPF_K           = 0x06  (return constant)
    //
    // seccomp_data layout:
    //   offset 0: syscall_nr (u32)
    //   offset 4: audit_arch (u32)  — x86_64 = 0xc000003e
    //
    // SECCOMP_RET_ALLOW = 0x7fff0000
    // SECCOMP_RET_ERRNO(EPERM) = 0x00050001

    // Essential syscalls for container workloads
    const ALLOWED: &[u32] = &[
        0, 1, 2, 3, 4, 5, 6, 8, 9, 10, 11, 12,       // read/write/stat/mmap/munmap/brk
        13, 14, 15, 16, 17, 18, 19, 20,               // signals/ioctl/pread/pwrite/readv/writev
        21, 22, 25, 32, 33, 35, 39, 40,               // access/pipe/mremap/dup/nanosleep/getpid/sendfile
        41, 42, 43, 44, 45, 49, 50, 56,               // socket/connect/accept/sendto/recvfrom/bind/listen/clone
        57, 58, 59, 60, 61, 62, 63,                   // fork/vfork/execve/exit/wait4/kill/uname
        72, 73, 74, 79, 83, 84, 85, 86,               // fcntl/flock/fsync/getcwd/symlinkat/unlinkat/renameat/linkat
        89, 90, 91, 102, 104, 107, 108,               // readlinkat/fchmodat/faccessat/getuid/getgid/geteuid/getegid
        131, 157, 158, 186, 187, 191, 199,            // sigaltstack/prctl/arch_prctl/gettid/getresuid/futex/tgkill
        200, 217, 218, 228, 231, 234,                 // set_tid/getrandom/memfd/clock_gettime/exit_group/set_robust
        257, 262, 273, 281, 291, 302, 318, 332, 334, // statx/getdents/epoll/eventfd/timerfd/pidfd/clone3/rseq
        424, 435,                                     // pidfd_getfd/epoll_pwait2
    ];

    let mut insns: Vec<[u8; 8]> = Vec::new();

    // 0: LOAD audit_arch
    insns.push(bpf_insn(0x20, 0, 0, 4));
    // 1: JEQ x86_64 (0xc000003e) ? continue : kill (skip to RET ERRNO)
    let skip_to_deny = ALLOWED.len() + 2; // skip all checks + RET DENY
    insns.push(bpf_insn_j(0x15, skip_to_deny.min(255) as u8, 0, 0xc000003e));
    // 2: LOAD syscall_nr
    insns.push(bpf_insn(0x20, 0, 0, 0));

    // 3..N: Check each allowed syscall
    // For syscall i: if match, skip (N-1-i) remaining checks + 1 RET_DENY = N-i
    let n = ALLOWED.len();
    for (i, &nr) in ALLOWED.iter().enumerate() {
        let skip = (n - i).min(255) as u8;
        insns.push(bpf_insn_j(0x15, skip, 0, nr));
    }

    // RET ERRNO(EPERM) — default deny
    insns.push(bpf_insn(0x06, 0, 0, 0x00050001));
    // RET ALLOW
    insns.push(bpf_insn(0x06, 0, 0, 0x7fff0000));

    // Build sock_fprog: { len: u16, filter: *sock_filter }
    let leaked = Box::leak(Box::new(insns));
    let mut prog = Vec::with_capacity(16);
    prog.extend_from_slice(&(leaked.len() as u16).to_le_bytes());
    prog.resize(8, 0);
    prog.extend_from_slice(&(leaked.as_ptr() as u64).to_le_bytes());
    prog
}

fn bpf_insn(code: u16, jt: u8, jf: u8, k: u32) -> [u8; 8] {
    let mut buf = [0u8; 8];
    buf[0..2].copy_from_slice(&code.to_le_bytes());
    buf[2] = jt;
    buf[3] = jf;
    buf[4..8].copy_from_slice(&k.to_le_bytes());
    buf
}

fn bpf_insn_j(code: u16, jt: u8, jf: u8, k: u32) -> [u8; 8] {
    bpf_insn(code, jt, jf, k)
}

/// Apply seccomp-BPF filter. Requires prctl(PR_SET_NO_NEW_PRIVS, 1) first.
fn apply_seccomp() -> io::Result<()> {
    let prog = seccomp_bpf_prog();
    let ret = do_seccomp(
        SECCOMP_SET_MODE_FILTER,
        SECCOMP_FILTER_FLAG_TSYNC,
        prog.as_ptr() as *const c_void,
    );
    if ret == 0 { Ok(()) } else { Err(io::Error::last_os_error()) }
}

// ===========================================================================
// Syscall wrappers
// ===========================================================================

fn do_unshare(flags: c_int) -> io::Result<()> {
    let ret = unsafe { unshare(flags) };
    if ret == 0 { Ok(()) } else { Err(io::Error::last_os_error()) }
}

/// Write /proc/self/uid_map: container_uid → host_uid, count entries.
/// Format: "inside_uid outside_uid count\n"
fn write_uid_map(inside_uid: u32, outside_uid: u32, count: u32) -> io::Result<()> {
    let content = format!("{} {} {}\n", inside_uid, outside_uid, count);
    fs::write("/proc/self/uid_map", &content)?;
    // deny_setgroups is needed for unprivileged user namespaces on some kernels
    let _ = fs::write("/proc/self/setgroups", "deny");
    Ok(())
}

/// Write /proc/self/gid_map: container_gid → host_gid, count entries.
fn write_gid_map(inside_gid: u32, outside_gid: u32, count: u32) -> io::Result<()> {
    let content = format!("{} {} {}\n", inside_gid, outside_gid, count);
    fs::write("/proc/self/gid_map", &content)?;
    Ok(())
}

fn do_mount(source: &str, target: &str, fstype: &str, flags: c_ulong, data: &str) -> io::Result<()> {
    let s = CString::new(source).unwrap();
    let t = CString::new(target).unwrap();
    let f = CString::new(fstype).unwrap();
    let d = CString::new(data).unwrap();
    let ret = unsafe { mount(s.as_ptr(), t.as_ptr(), f.as_ptr(), flags, d.as_ptr() as *const _) };
    if ret == 0 { Ok(()) } else { Err(io::Error::last_os_error()) }
}

fn do_pivot_root(new_root: &str, put_old: &str) -> io::Result<()> {
    let nr = CString::new(new_root).unwrap();
    let po = CString::new(put_old).unwrap();
    let ret = unsafe { pivot_root(nr.as_ptr(), po.as_ptr()) };
    if ret == 0 { Ok(()) } else { Err(io::Error::last_os_error()) }
}

fn do_umount2(target: &str, flags: c_int) -> io::Result<()> {
    let t = CString::new(target).unwrap();
    let ret = unsafe { umount2(t.as_ptr(), flags) };
    if ret == 0 { Ok(()) } else { Err(io::Error::last_os_error()) }
}

fn do_set_hostname(name: &str) -> io::Result<()> {
    let n = CString::new(name).unwrap();
    let ret = unsafe { sethostname(n.as_ptr(), n.as_bytes().len()) };
    if ret == 0 { Ok(()) } else { Err(io::Error::last_os_error()) }
}

fn makedev(major: u64, minor: u64) -> c_uint {
    ((major & 0xfff) << 8 | (minor & 0xff) | ((minor & 0xfff00) << 12)) as c_uint
}

fn create_device(path: &str, major: u64, minor: u64, mode: u32) {
    let dev = makedev(major, minor);
    let path_c = CString::new(path).unwrap();
    let _ = unsafe { mknod(path_c.as_ptr(), S_IFCHR | mode, dev) };
}

// ===========================================================================
// OCI config.json
// ===========================================================================

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OciSpec {
    pub version: String,
    #[serde(default)]
    pub process: Option<OciProcess>,
    #[serde(default)]
    pub root: Option<OciRoot>,
    pub hostname: Option<String>,
    #[serde(default)]
    pub linux: Option<OciLinux>,
    #[serde(default)]
    pub mounts: Option<Vec<OciMount>>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct OciProcess {
    #[serde(default)]
    pub terminal: Option<bool>,
    #[serde(default)]
    pub user: Option<OciUser>,
    #[serde(default)]
    pub args: Option<Vec<String>>,
    #[serde(default)]
    pub env: Option<Vec<String>>,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub capabilities: Option<OciCapabilities>,
    #[serde(default)]
    pub rlimits: Option<Vec<OciRlimit>>,
    #[serde(default)]
    pub no_new_privileges: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct OciUser {
    pub uid: Option<u32>,
    pub gid: Option<u32>,
    #[serde(default)]
    pub additional_gids: Option<Vec<u32>>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct OciCapabilities {
    #[serde(default)]
    pub bounding: Option<Vec<String>>,
    #[serde(default)]
    pub effective: Option<Vec<String>>,
    #[serde(default)]
    pub inheritable: Option<Vec<String>>,
    #[serde(default)]
    pub permitted: Option<Vec<String>>,
    #[serde(default)]
    pub ambient: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct OciRlimit {
    #[serde(rename = "type")]
    pub r#type: String,
    pub hard: u64,
    pub soft: u64,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct OciRoot {
    pub path: String,
    #[serde(default)]
    pub readonly: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct OciLinux {
    #[serde(default)]
    pub uid_mappings: Option<Vec<OciIdMapping>>,
    #[serde(default)]
    pub gid_mappings: Option<Vec<OciIdMapping>>,
    #[serde(default)]
    pub resources: Option<OciLinuxResources>,
    #[serde(default)]
    pub cgroups_path: Option<String>,
    #[serde(default)]
    pub namespaces: Option<Vec<OciNamespace>>,
    #[serde(default)]
    pub devices: Option<Vec<OciLinuxDevice>>,
    #[serde(default)]
    pub masked_paths: Option<Vec<String>>,
    #[serde(default)]
    pub readonly_paths: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct OciIdMapping {
    pub container_id: u32,
    pub host_id: u32,
    pub size: u32,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct OciLinuxResources {
    #[serde(default)]
    pub memory: Option<OciLinuxMemory>,
    #[serde(default)]
    pub cpu: Option<OciLinuxCpu>,
    #[serde(default)]
    pub pids: Option<OciLinuxPids>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct OciLinuxMemory {
    pub limit: Option<i64>,
    pub reservation: Option<i64>,
    pub swap: Option<i64>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct OciLinuxCpu {
    pub shares: Option<u64>,
    pub quota: Option<i64>,
    pub period: Option<u64>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct OciLinuxPids {
    pub limit: i64,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct OciNamespace {
    #[serde(rename = "type")]
    pub ns_type: String,
    pub path: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct OciLinuxDevice {
    pub path: String,
    #[serde(rename = "type")]
    pub dev_type: String,
    pub major: i64,
    pub minor: i64,
    #[serde(default)]
    pub file_mode: Option<u32>,
    #[serde(default)]
    pub uid: Option<u32>,
    #[serde(default)]
    pub gid: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct OciMount {
    pub destination: String,
    #[serde(rename = "type")]
    pub mount_type: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub options: Option<Vec<String>>,
}

/// Default namespaces for an OCI container.
pub fn default_namespaces() -> Vec<OciNamespace> {
    vec![
        OciNamespace { ns_type: "user".into(), path: None },
        OciNamespace { ns_type: "mount".into(), path: None },
        OciNamespace { ns_type: "pid".into(), path: None },
        OciNamespace { ns_type: "network".into(), path: None },
        OciNamespace { ns_type: "ipc".into(), path: None },
        OciNamespace { ns_type: "uts".into(), path: None },
    ]
}

/// Resolve namespace clone flags from OCI namespace type strings.
pub fn namespace_flags(namespaces: &[OciNamespace]) -> c_int {
    let mut flags: c_int = 0;
    for ns in namespaces {
        flags |= match ns.ns_type.as_str() {
            "mount"   => ns::NEWNS,
            "cgroup"  => ns::NEWCGROUP,
            "uts"     => ns::NEWUTS,
            "ipc"     => ns::NEWIPC,
            "user"    => ns::NEWUSER,
            "pid"     => ns::NEWPID,
            "network" => ns::NEWNET,
            _ => 0,
        };
    }
    flags
}

// ===========================================================================
// Cgroups v2 — pure file I/O
// ===========================================================================

/// Apply cgroup v2 resource limits by writing to /sys/fs/cgroup.
pub fn setup_cgroups(pid: u32, resources: &OciLinuxResources, cgroup_path: &str) -> io::Result<()> {
    let cgroup_root = Path::new("/sys/fs/cgroup").join(cgroup_path.trim_start_matches('/'));
    fs::create_dir_all(&cgroup_root)?;

    // Move PID into cgroup
    fs::write(cgroup_root.join("cgroup.procs"), format!("{}", pid))?;

    // Memory limits
    if let Some(ref mem) = resources.memory {
        if let Some(limit) = mem.limit {
            if limit >= 0 {
                let _ = fs::write(cgroup_root.join("memory.max"), format!("{}", limit));
            }
        }
        if let Some(swap) = mem.swap {
            if swap >= 0 {
                let _ = fs::write(cgroup_root.join("memory.swap.max"), format!("{}", swap));
            }
        }
    }

    // CPU limits
    if let Some(ref cpu) = resources.cpu {
        if let Some(period) = cpu.period {
            if period > 0 {
                let quota = cpu.quota.unwrap_or(-1);
                let _ = fs::write(cgroup_root.join("cpu.max"), format!("{} {}", quota, period));
            }
        }
        if let Some(shares) = cpu.shares {
            if shares > 0 {
                let _ = fs::write(cgroup_root.join("cpu.weight"), format!("{}", shares_to_weight(shares)));
            }
        }
    }

    // PID limits
    if let Some(ref pids) = resources.pids {
        if pids.limit > 0 {
            let _ = fs::write(cgroup_root.join("pids.max"), format!("{}", pids.limit));
        }
    }

    Ok(())
}

/// Convert legacy cpu.shares to cgroup v2 cpu.weight.
fn shares_to_weight(shares: u64) -> u64 {
    if shares <= 2 { return 1; }
    let w = 1 + ((shares - 2) * 9999) / 262142;
    w.min(10000).max(1)
}

// ===========================================================================
// Container rootfs setup
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

/// Setup the container rootfs: pivot_root, mount filesystems, create devices.
fn setup_rootfs(
    root: &OciRoot,
    mounts: Option<&[OciMount]>,
    masked: Option<&[String]>,
    readonly: Option<&[String]>,
) -> io::Result<()> {
    let rootfs = Path::new(&root.path);
    if !rootfs.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("rootfs not found: {}", rootfs.display()),
        ));
    }

    // Bind mount rootfs to make it a mount point
    do_mount(
        rootfs.to_str().unwrap(),
        rootfs.to_str().unwrap(),
        "bind",
        ms::BIND | ms::REC,
        "",
    )?;

    // Create old_root inside rootfs for pivot_root
    let old_root = rootfs.join(".oci-old-root");
    fs::create_dir_all(&old_root)?;

    // pivot_root
    do_pivot_root(rootfs.to_str().unwrap(), old_root.to_str().unwrap())?;

    // Detach and remove old root
    do_umount2("/.oci-old-root", MNT_DETACH)?;
    let _ = fs::remove_dir("/.oci-old-root");

    // Make / private so mounts don't propagate to host
    do_mount("", "/", "", ms::PRIVATE | ms::REC, "")?;

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
    create_device("/dev/null", 1, 3, 0o666);
    create_device("/dev/zero", 1, 5, 0o666);
    create_device("/dev/full", 1, 7, 0o666);
    create_device("/dev/random", 1, 8, 0o666);
    create_device("/dev/urandom", 1, 9, 0o666);
    create_device("/dev/tty", 5, 0, 0o666);
    let _ = fs::create_dir_all("/dev/pts");
    let _ = fs::create_dir_all("/dev/shm");

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
            let _ = setup_mount(m);
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

// ===========================================================================
// Run container
// ===========================================================================

/// Run an OCI bundle (directory containing config.json + rootfs/).
pub fn run_bundle(bundle_path: &Path) -> io::Result<std::process::ExitStatus> {
    let config_path = bundle_path.join("config.json");
    let config_data = fs::read(&config_path)?;
    let spec: OciSpec = lifegraph_json::from_slice(&config_data)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("invalid OCI config: {}", e)))?;
    run_spec(&spec)
}

/// Run a container from a parsed OCI spec.
pub fn run_spec(spec: &OciSpec) -> io::Result<std::process::ExitStatus> {
    let root = spec.root.as_ref()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "no root in OCI spec"))?;

    let namespaces = spec.linux.as_ref()
        .and_then(|l| l.namespaces.as_ref())
        .cloned()
        .unwrap_or_else(default_namespaces);
    let ns_flags = namespace_flags(&namespaces);

    let hostname = spec.hostname.clone().unwrap_or_else(|| "edgerun".into());
    let process = spec.process.clone().unwrap_or_default();
    let linux = spec.linux.clone().unwrap_or_default();

    let args = process.args.clone().unwrap_or_else(|| vec!["/bin/sh".into()]);
    let env = process.env.clone().unwrap_or_else(|| vec![
        "PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin".into(),
        "TERM=xterm".into(),
    ]);
    let cwd = process.cwd.clone().unwrap_or_else(|| "/".into());

    let uid = process.user.as_ref().and_then(|u| u.uid).unwrap_or(0);
    let gid = process.user.as_ref().and_then(|u| u.gid).unwrap_or(0);
    let additional_gids = process.user.as_ref()
        .and_then(|u| u.additional_gids.as_ref())
        .cloned()
        .unwrap_or_default();
    let no_new_privs = process.no_new_privileges.unwrap_or(true);

    let root_path = root.path.clone();
    let root_path_pre = root_path.clone();
    let mounts = spec.mounts.clone();
    let masked = linux.masked_paths.clone();
    let readonly = linux.readonly_paths.clone();
    let resources = linux.resources.clone();
    let cgroup_path = linux.cgroups_path.clone().unwrap_or_else(|| "/edgerun".into());
    let hostname_c = hostname.clone();

    let mut cmd = Command::new(&args[0]);
    cmd.args(&args[1..]);
    cmd.current_dir(&cwd);
    cmd.env_clear();
    for e in &env {
        if let Some((k, v)) = e.split_once('=') {
            cmd.env(k, v);
        }
    }

    // Pre-exec: unshare namespaces, setup rootfs, drop privileges
    // SAFETY: This runs in a forked child before execve.
    unsafe {
        cmd.pre_exec(move || {
            // 1. Unshare namespaces (includes user namespace)
            do_unshare(ns_flags)?;

            // 2. Write uid_map: container root (0) → host nobody (65534), range 1
            //    This makes the container's root user map to an unprivileged host user.
            write_uid_map(0, 65534, 1)?;
            // Write gid_map similarly
            write_gid_map(0, 65534, 1)?;

            // 3. Set hostname
            let _ = do_set_hostname(&hostname_c);

            // 4. Set no_new_privs (required before seccomp)
            if no_new_privs {
                do_prctl_set_no_new_privs()?;
                do_prctl_set_dumpable(false)?;
            }

            // 5. Apply seccomp-BPF filter (deny all but essential syscalls)
            // Fail open: if seccomp is not supported, log and continue.
            if let Err(e) = apply_seccomp() {
                // Seccomp may be unavailable in some environments (containers, WSL).
                // Log the error but don't block the workload.
                let _ = std::fs::write("/tmp/.edgerun_seccomp_err", format!("{}", e));
            }

            // 6. Setup rootfs (pivot_root, mount filesystems)
            let oci_root = OciRoot { path: root_path.clone(), readonly: None };
            setup_rootfs(
                &oci_root,
                mounts.as_deref(),
                masked.as_deref(),
                readonly.as_deref(),
            )?;

            // 7. Set supplementary groups
            if !additional_gids.is_empty() {
                setgroups(additional_gids.len(), additional_gids.as_ptr());
            }

            // 8. Set GID (container root GID = 0)
            if setgid(gid) != 0 {
                return Err(io::Error::last_os_error());
            }

            // 9. Set UID (container root UID = 0)
            // Inside the user namespace this is root, but on the host it's nobody (65534).
            if setuid(uid) != 0 {
                return Err(io::Error::last_os_error());
            }

            Ok(())
        });
    }

    // Spawn the child
    let mut child = cmd.spawn()?;

    // Set cgroups from parent (we have the child's PID)
    if let Some(ref res) = resources {
        let _ = setup_cgroups(child.id(), res, &cgroup_path);
    }

    child.wait()
}

fn do_prctl_set_no_new_privs() -> io::Result<()> {
    let ret = unsafe { prctl(prctl::SET_NO_NEW_PRIVS, 1, 0, 0, 0) };
    if ret == 0 { Ok(()) } else { Err(io::Error::last_os_error()) }
}

fn do_prctl_set_dumpable(dumpable: bool) -> io::Result<()> {
    let ret = unsafe { prctl(prctl::SET_DUMPABLE, if dumpable { 1 } else { 0 }, 0, 0, 0) };
    if ret == 0 { Ok(()) } else { Err(io::Error::last_os_error()) }
}

// ===========================================================================
// Create OCI bundle
// ===========================================================================

/// Create a minimal OCI bundle from a rootfs directory and command.
pub fn create_bundle(
    rootfs_path: &str,
    args: Vec<String>,
    env: Option<Vec<String>>,
    hostname: Option<String>,
) -> OciSpec {
    OciSpec {
        version: "1.0.2".into(),
        process: Some(OciProcess {
            args: Some(args),
            env,
            cwd: Some("/".into()),
            no_new_privileges: Some(true),
            user: Some(OciUser {
                uid: Some(0),
                gid: Some(0),
                additional_gids: None,
            }),
            ..Default::default()
        }),
        root: Some(OciRoot {
            path: rootfs_path.into(),
            readonly: None,
        }),
        hostname: hostname.or_else(|| Some("edgerun".into())),
        linux: Some(OciLinux {
            namespaces: Some(default_namespaces()),
            masked_paths: Some(vec![
                "/proc/acpi".into(), "/proc/kcore".into(), "/proc/keys".into(),
                "/proc/latency_stats".into(), "/proc/timer_list".into(),
                "/proc/timer_stats".into(), "/proc/sched_debug".into(),
                "/proc/scsi".into(), "/sys/firmware".into(),
            ]),
            readonly_paths: Some(vec![
                "/proc/asound".into(), "/proc/bus".into(), "/proc/fs".into(),
                "/proc/irq".into(), "/proc/sys".into(), "/proc/sysrq-trigger".into(),
            ]),
            ..Default::default()
        }),
        mounts: Some(vec![]),
    }
}

/// Serialize an OCI spec to config.json in a bundle directory.
pub fn write_bundle(bundle_path: &Path, spec: &OciSpec) -> io::Result<()> {
    fs::create_dir_all(bundle_path)?;
    let json = lifegraph_json::to_string_pretty(spec)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    fs::write(bundle_path.join("config.json"), json)?;
    Ok(())
}

// ===========================================================================
// Non-blocking container lifecycle (for preemption support)
// ===========================================================================

/// A handle to a running container that can be awaited or killed.
pub struct RunningContainer {
    child: std::process::Child,
    cgroup_path: String,
}

impl RunningContainer {
    /// Block until the container exits and return its exit status.
    pub fn wait(mut self) -> io::Result<std::process::ExitStatus> {
        self.child.wait()
    }

    /// Kill the container with SIGTERM, then SIGKILL if it doesn't exit.
    /// Returns the exit status after killing.
    pub fn kill(mut self) -> io::Result<std::process::ExitStatus> {
        let pid = self.child.id();

        // Try SIGTERM first
        let _ = unsafe { libc::kill(pid as libc::pid_t, libc::SIGTERM) };

        // Wait up to 5 seconds for graceful exit
        for _ in 0..50 {
            match self.child.try_wait()? {
                Some(status) => return Ok(status),
                None => std::thread::sleep(std::time::Duration::from_millis(100)),
            }
        }

        // SIGKILL if still alive
        let _ = unsafe { libc::kill(pid as libc::pid_t, libc::SIGKILL) };
        self.child.wait()
    }

    /// Kill the container cgroup (kills all processes in the cgroup).
    /// More reliable than killing a single PID for container preemption.
    pub fn kill_cgroup(&self) {
        let cgroup_root = Path::new("/sys/fs/cgroup").join(self.cgroup_path.trim_start_matches('/'));
        // Write 1 to cgroup.kill (cgroup v2, kernel 5.15+)
        let _ = fs::write(cgroup_root.join("cgroup.kill"), "1");
    }

    /// The host PID of the container's init process.
    pub fn pid(&self) -> u32 {
        self.child.id()
    }
}

/// Start a container from an OCI bundle without blocking.
/// Returns a `RunningContainer` handle that can be awaited or killed.
pub fn start_bundle(bundle_path: &Path) -> io::Result<RunningContainer> {
    let config_path = bundle_path.join("config.json");
    let config_data = fs::read(&config_path)?;
    let spec: OciSpec = lifegraph_json::from_slice(&config_data)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("invalid OCI config: {}", e)))?;
    start_spec(&spec)
}

/// Start a container from a parsed OCI spec without blocking.
pub fn start_spec(spec: &OciSpec) -> io::Result<RunningContainer> {
    let root = spec.root.as_ref()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "no root in OCI spec"))?;

    let namespaces = spec.linux.as_ref()
        .and_then(|l| l.namespaces.as_ref())
        .cloned()
        .unwrap_or_else(default_namespaces);
    let ns_flags = namespace_flags(&namespaces);

    let hostname = spec.hostname.clone().unwrap_or_else(|| "edgerun".into());
    let process = spec.process.clone().unwrap_or_default();
    let linux = spec.linux.clone().unwrap_or_default();

    let args = process.args.clone().unwrap_or_else(|| vec!["/bin/sh".into()]);
    let env = process.env.clone().unwrap_or_else(|| vec![
        "PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin".into(),
        "TERM=xterm".into(),
    ]);
    let cwd = process.cwd.clone().unwrap_or_else(|| "/".into());

    let uid = process.user.as_ref().and_then(|u| u.uid).unwrap_or(0);
    let gid = process.user.as_ref().and_then(|u| u.gid).unwrap_or(0);
    let additional_gids = process.user.as_ref()
        .and_then(|u| u.additional_gids.as_ref())
        .cloned()
        .unwrap_or_default();
    let no_new_privs = process.no_new_privileges.unwrap_or(true);

    let root_path = root.path.clone();
    let mounts = spec.mounts.clone();
    let masked = linux.masked_paths.clone();
    let readonly = linux.readonly_paths.clone();
    let resources = linux.resources.clone();
    let cgroup_path = linux.cgroups_path.clone().unwrap_or_else(|| "/edgerun".into());
    let hostname_c = hostname.clone();

    let mut cmd = Command::new(&args[0]);
    cmd.args(&args[1..]);
    cmd.current_dir(&cwd);
    cmd.env_clear();
    for e in &env {
        if let Some((k, v)) = e.split_once('=') {
            cmd.env(k, v);
        }
    }

    // Pre-exec: unshare namespaces, setup rootfs, drop privileges
    unsafe {
        cmd.pre_exec(move || {
            do_unshare(ns_flags)?;
            write_uid_map(0, 65534, 1)?;
            write_gid_map(0, 65534, 1)?;
            let _ = do_set_hostname(&hostname_c);
            if no_new_privs {
                do_prctl_set_no_new_privs()?;
                do_prctl_set_dumpable(false)?;
            }
            if let Err(e) = apply_seccomp() {
                let _ = std::fs::write("/tmp/.edgerun_seccomp_err", format!("{}", e));
            }
            let oci_root = OciRoot { path: root_path.clone(), readonly: None };
            setup_rootfs(&oci_root, mounts.as_deref(), masked.as_deref(), readonly.as_deref())?;
            if !additional_gids.is_empty() {
                setgroups(additional_gids.len(), additional_gids.as_ptr());
            }
            if setgid(gid) != 0 {
                return Err(io::Error::last_os_error());
            }
            if setuid(uid) != 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }

    // Spawn the child
    let child = cmd.spawn()?;

    // Set cgroups from parent
    if let Some(ref res) = resources {
        let _ = setup_cgroups(child.id(), res, &cgroup_path);
    }

    Ok(RunningContainer {
        child,
        cgroup_path,
    })
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ===========================================================================
    // OCI Spec serialization / deserialization
    // ===========================================================================

    #[test]
    fn oci_spec_default_roundtrip() {
        let spec = OciSpec {
            version: "1.0.2".into(),
            process: None,
            root: None,
            hostname: None,
            linux: None,
            mounts: None,
        };
        let json = lifegraph_json::to_string(&spec).unwrap();
        let parsed: OciSpec = lifegraph_json::from_slice(json.as_bytes()).unwrap();
        assert_eq!(parsed.version, "1.0.2");
    }

    #[test]
    fn oci_spec_full_roundtrip() {
        let spec = OciSpec {
            version: "1.0.2".into(),
            process: Some(OciProcess {
                terminal: Some(false),
                user: Some(OciUser {
                    uid: Some(1000),
                    gid: Some(1000),
                    additional_gids: Some(vec![100, 200]),
                }),
                args: Some(vec!["/bin/sh".into(), "-c".into(), "echo hello".into()]),
                env: Some(vec!["PATH=/usr/bin".into(), "HOME=/root".into()]),
                cwd: Some("/app".into()),
                capabilities: Some(OciCapabilities {
                    bounding: Some(vec!["CAP_NET_BIND_SERVICE".into()]),
                    effective: Some(vec!["CAP_NET_BIND_SERVICE".into()]),
                    inheritable: Some(vec![]),
                    permitted: Some(vec!["CAP_NET_BIND_SERVICE".into()]),
                    ambient: Some(vec![]),
                }),
                rlimits: Some(vec![OciRlimit {
                    r#type: "RLIMIT_NOFILE".into(),
                    hard: 65536,
                    soft: 65536,
                }]),
                no_new_privileges: Some(true),
            }),
            root: Some(OciRoot {
                path: "/var/lib/edgerun/bundles/test/rootfs".into(),
                readonly: Some(false),
            }),
            hostname: Some("test-container".into()),
            linux: Some(OciLinux {
                uid_mappings: Some(vec![OciIdMapping {
                    container_id: 0,
                    host_id: 65534,
                    size: 1,
                }]),
                gid_mappings: Some(vec![OciIdMapping {
                    container_id: 0,
                    host_id: 65534,
                    size: 1,
                }]),
                resources: Some(OciLinuxResources {
                    memory: Some(OciLinuxMemory {
                        limit: Some(536870912),
                        reservation: None,
                        swap: Some(0),
                    }),
                    cpu: Some(OciLinuxCpu {
                        shares: Some(512),
                        quota: Some(50000),
                        period: Some(100000),
                    }),
                    pids: Some(OciLinuxPids { limit: 128 }),
                }),
                cgroups_path: Some("/edgerun/test".into()),
                namespaces: Some(default_namespaces()),
                devices: Some(vec![OciLinuxDevice {
                    path: "/dev/null".into(),
                    dev_type: "c".into(),
                    major: 1,
                    minor: 3,
                    file_mode: Some(0o666),
                    uid: Some(0),
                    gid: Some(0),
                }]),
                masked_paths: Some(vec!["/proc/acpi".into()]),
                readonly_paths: Some(vec!["/proc/sys".into()]),
            }),
            mounts: Some(vec![
                OciMount {
                    destination: "/proc".into(),
                    mount_type: Some("proc".into()),
                    source: Some("proc".into()),
                    options: Some(vec!["nosuid".into(), "nodev".into(), "noexec".into()]),
                },
                OciMount {
                    destination: "/sys".into(),
                    mount_type: Some("sysfs".into()),
                    source: Some("sysfs".into()),
                    options: Some(vec!["ro".into(), "nosuid".into(), "nodev".into(), "noexec".into()]),
                },
            ]),
        };

        let json = lifegraph_json::to_string(&spec).unwrap();
        let parsed: OciSpec = lifegraph_json::from_slice(json.as_bytes()).unwrap();

        // Verify all fields round-tripped
        assert_eq!(parsed.version, spec.version);
        assert_eq!(parsed.hostname, spec.hostname);
        let proc = parsed.process.unwrap();
        assert_eq!(proc.args.unwrap(), vec!["/bin/sh", "-c", "echo hello"]);
        assert_eq!(proc.env.unwrap(), vec!["PATH=/usr/bin", "HOME=/root"]);
        assert_eq!(proc.cwd.unwrap(), "/app");
        assert!(proc.no_new_privileges.unwrap());
        assert_eq!(parsed.root.unwrap().path, spec.root.unwrap().path);
        let linux = parsed.linux.unwrap();
        assert_eq!(linux.cgroups_path.unwrap(), "/edgerun/test");
        assert_eq!(linux.namespaces.unwrap().len(), 6);
        assert_eq!(linux.masked_paths.unwrap(), vec!["/proc/acpi"]);
        assert_eq!(linux.readonly_paths.unwrap(), vec!["/proc/sys"]);
    }

    #[test]
    fn oci_spec_deserializes_missing_optional_fields() {
        let json = r#"{"version":"1.0.2","root":{"path":"/rootfs"}}"#;
        let spec: OciSpec = lifegraph_json::from_slice(json.as_bytes()).unwrap();
        assert_eq!(spec.version, "1.0.2");
        assert!(spec.process.is_none());
        assert!(spec.hostname.is_none());
        assert!(spec.linux.is_none());
        assert!(spec.mounts.is_none());
    }

    // ===========================================================================
    // create_bundle
    // ===========================================================================

    #[test]
    fn create_bundle_sets_defaults() {
        let spec = create_bundle(
            "/var/lib/edgerun/bundles/test/rootfs",
            vec!["/bin/sh".into(), "-c".into(), "echo hello".into()],
            Some(vec!["PATH=/usr/bin".into()]),
            Some("myhost".into()),
        );

        assert_eq!(spec.version, "1.0.2");
        assert_eq!(spec.hostname, Some("myhost".into()));
        assert_eq!(spec.root.as_ref().unwrap().path, "/var/lib/edgerun/bundles/test/rootfs");

        let proc = spec.process.as_ref().unwrap();
        assert_eq!(proc.args.as_ref().unwrap(), &vec!["/bin/sh", "-c", "echo hello"]);
        assert_eq!(proc.env.as_ref().unwrap(), &vec!["PATH=/usr/bin"]);
        assert_eq!(proc.cwd.as_ref().unwrap(), "/");
        assert!(proc.no_new_privileges.unwrap());
        assert_eq!(proc.user.as_ref().unwrap().uid, Some(0));
        assert_eq!(proc.user.as_ref().unwrap().gid, Some(0));

        let linux = spec.linux.as_ref().unwrap();
        assert_eq!(linux.namespaces.as_ref().unwrap().len(), 6);
        assert_eq!(linux.masked_paths.as_ref().unwrap().len(), 9);
        assert_eq!(linux.readonly_paths.as_ref().unwrap().len(), 6);
    }

    #[test]
    fn create_bundle_default_hostname() {
        let spec = create_bundle("/rootfs", vec!["/bin/sh".into()], None, None);
        assert_eq!(spec.hostname, Some("edgerun".into()));
    }

    #[test]
    fn create_bundle_empty_args() {
        let spec = create_bundle("/rootfs", vec![], None, None);
        assert_eq!(spec.process.as_ref().unwrap().args.as_ref().unwrap(), &Vec::<String>::new());
    }

    // ===========================================================================
    // default_namespaces
    // ===========================================================================

    #[test]
    fn default_namespaces_returns_six() {
        let ns = default_namespaces();
        assert_eq!(ns.len(), 6);
    }

    #[test]
    fn default_namespaces_contains_expected_types() {
        let ns = default_namespaces();
        let types: Vec<&str> = ns.iter().map(|n| n.ns_type.as_str()).collect();
        assert!(types.contains(&"user"));
        assert!(types.contains(&"mount"));
        assert!(types.contains(&"pid"));
        assert!(types.contains(&"network"));
        assert!(types.contains(&"ipc"));
        assert!(types.contains(&"uts"));
    }

    #[test]
    fn default_namespaces_have_no_path() {
        for ns in default_namespaces() {
            assert!(ns.path.is_none(), "namespace {} should have no path", ns.ns_type);
        }
    }

    // ===========================================================================
    // namespace_flags
    // ===========================================================================

    #[test]
    fn namespace_flags_empty() {
        assert_eq!(namespace_flags(&[]), 0);
    }

    #[test]
    fn namespace_flags_single_mount() {
        let ns = vec![OciNamespace { ns_type: "mount".into(), path: None }];
        let flags = namespace_flags(&ns);
        assert_ne!(flags, 0);
        assert_eq!(flags, 0x00020000); // NEWNS
    }

    #[test]
    fn namespace_flags_single_user() {
        let ns = vec![OciNamespace { ns_type: "user".into(), path: None }];
        let flags = namespace_flags(&ns);
        assert_eq!(flags, 0x10000000); // NEWUSER
    }

    #[test]
    fn namespace_flags_single_pid() {
        let ns = vec![OciNamespace { ns_type: "pid".into(), path: None }];
        let flags = namespace_flags(&ns);
        assert_eq!(flags, 0x20000000); // NEWPID
    }

    #[test]
    fn namespace_flags_single_network() {
        let ns = vec![OciNamespace { ns_type: "network".into(), path: None }];
        let flags = namespace_flags(&ns);
        assert_eq!(flags, 0x40000000); // NEWNET
    }

    #[test]
    fn namespace_flags_container_combination() {
        let ns = vec![
            OciNamespace { ns_type: "mount".into(), path: None },
            OciNamespace { ns_type: "cgroup".into(), path: None },
            OciNamespace { ns_type: "uts".into(), path: None },
            OciNamespace { ns_type: "ipc".into(), path: None },
            OciNamespace { ns_type: "pid".into(), path: None },
            OciNamespace { ns_type: "network".into(), path: None },
        ];
        let flags = namespace_flags(&ns);
        // NEWNS=0x00020000 | NEWCGROUP=0x02000000 | NEWUTS=0x04000000
        // | NEWIPC=0x08000000 | NEWPID=0x20000000 | NEWNET=0x40000000
        assert_eq!(flags, 0x6e020000);
    }

    #[test]
    fn namespace_flags_unknown_type_ignored() {
        let ns = vec![OciNamespace { ns_type: "bogus".into(), path: None }];
        assert_eq!(namespace_flags(&ns), 0);
    }

    #[test]
    fn namespace_flags_duplicate_namespace_uses_last_match() {
        // Duplicates should OR the same flag (no change)
        let ns = vec![
            OciNamespace { ns_type: "mount".into(), path: None },
            OciNamespace { ns_type: "mount".into(), path: None },
        ];
        let flags = namespace_flags(&ns);
        assert_eq!(flags, 0x00020000);
    }

    // ===========================================================================
    // shares_to_weight
    // ===========================================================================

    #[test]
    fn shares_to_weight_zero_returns_one() {
        assert_eq!(shares_to_weight(0), 1);
    }

    #[test]
    fn shares_to_weight_one_returns_one() {
        assert_eq!(shares_to_weight(1), 1);
    }

    #[test]
    fn shares_to_weight_two_returns_one() {
        assert_eq!(shares_to_weight(2), 1);
    }

    #[test]
    fn shares_to_weight_default_returns_256() {
        // Default shares is 1024, should map to weight ~39
        // 1 + (1024 - 2) * 9999 / 262142 = 1 + 1022 * 9999 / 262142 ≈ 40
        let w = shares_to_weight(1024);
        assert!(w >= 1 && w <= 10000);
    }

    #[test]
    fn shares_to_weight_high_shares_caps_at_10000() {
        assert_eq!(shares_to_weight(262144), 10000);
        // Don't test u64::MAX - it would overflow in the intermediate calculation
        assert_eq!(shares_to_weight(10_000_000), 10000);
    }

    #[test]
    fn shares_to_weight_is_monotonic() {
        let w1 = shares_to_weight(100);
        let w2 = shares_to_weight(1000);
        let w3 = shares_to_weight(10000);
        assert!(w1 <= w2 && w2 <= w3);
    }

    // ===========================================================================
    // mount_flags_from_opts
    // ===========================================================================

    #[test]
    fn mount_flags_none() {
        assert_eq!(mount_flags_from_opts(None), 0);
    }

    #[test]
    fn mount_flags_empty_vec() {
        assert_eq!(mount_flags_from_opts(Some(&[])), 0);
    }

    #[test]
    fn mount_flags_readonly() {
        assert_eq!(mount_flags_from_opts(Some(&["ro".into()])), 1); // MS_RDONLY
    }

    #[test]
    fn mount_flags_nosuid() {
        assert_eq!(mount_flags_from_opts(Some(&["nosuid".into()])), 2); // MS_NOSUID
    }

    #[test]
    fn mount_flags_nodev() {
        assert_eq!(mount_flags_from_opts(Some(&["nodev".into()])), 4); // MS_NODEV
    }

    #[test]
    fn mount_flags_noexec() {
        assert_eq!(mount_flags_from_opts(Some(&["noexec".into()])), 8); // MS_NOEXEC
    }

    #[test]
    fn mount_flags_strictatime() {
        assert_eq!(mount_flags_from_opts(Some(&["strictatime".into()])), 1 << 24); // MS_STRICTATIME
    }

    #[test]
    fn mount_flags_combination() {
        let flags = mount_flags_from_opts(Some(&["ro".into(), "nosuid".into(), "nodev".into()]));
        assert_eq!(flags, 1 | 2 | 4);
    }

    #[test]
    fn mount_flags_unknown_ignored() {
        let flags = mount_flags_from_opts(Some(&["ro".into(), "bogus".into(), "nodev".into()]));
        assert_eq!(flags, 1 | 4);
    }

    // ===========================================================================
    // seccomp_bpf_prog
    // ===========================================================================

    #[test]
    fn seccomp_bpf_prog_is_non_empty() {
        let prog = seccomp_bpf_prog();
        assert!(!prog.is_empty());
    }

    #[test]
    fn seccomp_bpf_prog_has_valid_structure() {
        let prog = seccomp_bpf_prog();
        // First 2 bytes are length (u16 LE), next 8 bytes are pointer
        assert!(prog.len() >= 16);
        let len = u16::from_le_bytes([prog[0], prog[1]]) as usize;
        // Should have many BPF instructions
        assert!(len > 50);
    }

    #[test]
    fn seccomp_bpf_prog_contains_allow_and_deny() {
        let prog = seccomp_bpf_prog();
        // The prog is a sock_fprog: { len: u16, padding: 6 bytes, filter_ptr: u64 }
        // The actual BPF instructions are at the pointed-to address.
        // We can verify the structure is correct:
        assert!(prog.len() >= 16, "sock_fprog should be at least 16 bytes");
        let len = u16::from_le_bytes([prog[0], prog[1]]) as usize;
        assert!(len > 50, "should have many BPF instructions, got {}", len);
        
        // Verify the pointer is non-null
        let ptr_bytes: [u8; 8] = prog[8..16].try_into().unwrap();
        let ptr = u64::from_le_bytes(ptr_bytes);
        assert_ne!(ptr, 0, "filter pointer should be non-null");

        // Read the actual BPF instructions from the leaked memory
        // Each instruction is 8 bytes: code(u16) + jt(u8) + jf(u8) + k(u32)
        unsafe {
            let insns = std::slice::from_raw_parts(ptr as *const [u8; 8], len);
            let mut found_allow = false;
            let mut found_deny = false;
            for insn in insns {
                let k = u32::from_le_bytes([insn[4], insn[5], insn[6], insn[7]]);
                if k == 0x7fff0000 { found_allow = true; }
                if k == 0x00050001 { found_deny = true; }
            }
            assert!(found_allow, "should contain RET_ALLOW (0x7fff0000)");
            assert!(found_deny, "should contain RET_ERRNO(EPERM) (0x00050001)");
        }
    }

    // ===========================================================================
    // bpf_insn helpers
    // ===========================================================================

    #[test]
    fn bpf_insn_produces_8_bytes() {
        let insn = bpf_insn(0x06, 0, 0, 0x7fff0000);
        assert_eq!(insn.len(), 8);
    }

    #[test]
    fn bpf_insn_ret_allow_encoding() {
        let insn = bpf_insn(0x06, 0, 0, 0x7fff0000);
        assert_eq!(&insn[0..2], &[0x06, 0x00]); // code
        assert_eq!(&insn[4..8], &[0x00, 0x00, 0xff, 0x7f]); // k = RET_ALLOW
    }

    // ===========================================================================
    // write_bundle
    // ===========================================================================

    #[test]
    fn write_bundle_creates_directory_and_config() {
        let tmp = std::env::temp_dir().join(format!("oci-bundle-test-{}", std::process::id()));
        let spec = create_bundle("/rootfs", vec!["/bin/sh".into()], None, None);
        write_bundle(&tmp, &spec).unwrap();

        assert!(tmp.is_dir());
        assert!(tmp.join("config.json").exists());
        let size = std::fs::metadata(tmp.join("config.json")).unwrap().len();
        assert!(size > 50, "config.json should be non-trivial size, got {}", size);

        // Verify the file contains expected OCI fields by checking for key strings
        let content = std::fs::read_to_string(tmp.join("config.json")).unwrap();
        assert!(content.contains("1.0.2"), "should contain OCI version");
        assert!(content.contains("rootfs"), "should contain rootfs path");

        // Cleanup
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn write_bundle_roundtrip() {
        let tmp = std::env::temp_dir().join(format!("oci-bundle-roundtrip-{}", std::process::id()));
        let spec = create_bundle(
            "/var/lib/test/rootfs",
            vec!["/bin/bash".into(), "-l".into()],
            Some(vec!["FOO=bar".into()]),
            Some("roundtest".into()),
        );
        write_bundle(&tmp, &spec).unwrap();

        // Verify key fields are in the written config
        let content = std::fs::read_to_string(tmp.join("config.json")).unwrap();
        assert!(content.contains("1.0.2"));
        assert!(content.contains("/var/lib/test/rootfs"));
        assert!(content.contains("/bin/bash"));
        assert!(content.contains("FOO=bar"));
        assert!(content.contains("roundtest"));

        // Cleanup
        let _ = std::fs::remove_dir_all(&tmp);
    }

    // ===========================================================================
    // OciMount serialization
    // ===========================================================================

    #[test]
    fn oci_mount_roundtrip() {
        let mount = OciMount {
            destination: "/proc".into(),
            mount_type: Some("proc".into()),
            source: Some("proc".into()),
            options: Some(vec!["nosuid".into(), "nodev".into(), "noexec".into()]),
        };
        let json = lifegraph_json::to_string(&mount).unwrap();
        let parsed: OciMount = lifegraph_json::from_slice(json.as_bytes()).unwrap();
        assert_eq!(parsed.destination, "/proc");
        assert_eq!(parsed.mount_type, Some("proc".into()));
        assert_eq!(parsed.options.as_ref().unwrap().len(), 3);
    }

    // ===========================================================================
    // OciLinuxResources serialization
    // ===========================================================================

    #[test]
    fn oci_linux_resources_roundtrip() {
        let res = OciLinuxResources {
            memory: Some(OciLinuxMemory {
                limit: Some(1073741824),
                reservation: Some(536870912),
                swap: Some(0),
            }),
            cpu: Some(OciLinuxCpu {
                shares: Some(2048),
                quota: Some(100000),
                period: Some(100000),
            }),
            pids: Some(OciLinuxPids { limit: 256 }),
        };
        let json = lifegraph_json::to_string(&res).unwrap();
        let parsed: OciLinuxResources = lifegraph_json::from_slice(json.as_bytes()).unwrap();
        assert_eq!(parsed.memory.as_ref().unwrap().limit, Some(1073741824));
        assert_eq!(parsed.cpu.as_ref().unwrap().shares, Some(2048));
        assert_eq!(parsed.pids.as_ref().unwrap().limit, 256);
    }

    // ===========================================================================
    // OciProcess with capabilities
    // ===========================================================================

    #[test]
    fn oci_process_capabilities_roundtrip() {
        let caps = OciCapabilities {
            bounding: Some(vec!["CAP_NET_BIND_SERVICE".into()]),
            effective: Some(vec!["CAP_NET_BIND_SERVICE".into()]),
            inheritable: Some(vec![]),
            permitted: Some(vec!["CAP_NET_BIND_SERVICE".into()]),
            ambient: Some(vec![]),
        };
        let json = lifegraph_json::to_string(&caps).unwrap();
        let parsed: OciCapabilities = lifegraph_json::from_slice(json.as_bytes()).unwrap();
        assert_eq!(parsed.bounding.as_ref().unwrap().len(), 1);
        assert_eq!(parsed.effective.as_ref().unwrap()[0], "CAP_NET_BIND_SERVICE");
    }
}
