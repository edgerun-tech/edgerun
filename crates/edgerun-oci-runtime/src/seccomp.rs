//! Seccomp-BPF filtering for OCI containers.
//!
//! Architecture-specific syscall allow-lists with default-deny policy.
//! Uses raw seccomp syscalls (no libseccomp dependency).

use std::io;
use std::os::raw::c_void;

use crate::syscalls::{do_seccomp, SECCOMP_SET_MODE_FILTER, SECCOMP_FILTER_FLAG_TSYNC};

/// Minimal seccomp-BPF allow-list for containers.
/// Allows essential syscalls, denies everything else with EPERM.
/// Supports both x86_64 and aarch64 architectures.
pub fn seccomp_bpf_prog() -> Vec<u8> {
    // BPF instruction: code(u16) jt(u8) jf(u8) k(u32) = 8 bytes
    //
    // BPF codes:
    //   BPF_LD | BPF_W | BPF_ABS  = 0x20  (load word from absolute offset)
    //   BPF_JMP | BPF_JEQ | BPF_K = 0x15  (jump if equal)
    //   BPF_RET | BPF_K           = 0x06  (return constant)
    //
    // seccomp_data layout:
    //   offset 0: syscall_nr (u32)
    //   offset 4: audit_arch (u32)  — x86_64 = 0xc000003e, aarch64 = 0xc00000b7
    //
    // SECCOMP_RET_ALLOW = 0x7fff0000
    // SECCOMP_RET_ERRNO(EPERM) = 0x00050001

    // Architecture-specific syscall numbers
    #[cfg(target_arch = "x86_64")]
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
    #[cfg(target_arch = "x86_64")]
    const AUDIT_ARCH: u32 = 0xc000003e;

    // TODO: Add proper aarch64 syscall list. The numbers below are placeholders.
    // See https://github.com/ureddit/aarch64-linux-gnu-syscall-list
    #[cfg(target_arch = "aarch64")]
    const ALLOWED: &[u32] = &[
        // Basic I/O and memory management
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11,        // io_setup..close
        13, 14, 15, 16, 17, 18, 19, 20, 21,           // readv..getpid
        22, 23, 24, 25, 27, 28, 29, 30, 31,           // sendfile..madvise
        32, 34, 35, 36, 37, 38, 39, 40, 41,           // pause..lseek
        43, 44, 45, 46, 47, 48, 49, 50, 51,           // mmap..shutdown
        52, 53, 54, 55, 56, 57, 58, 59, 60,           // setsockopt..wait4
        61, 62, 63, 73, 74, 79, 80, 81, 82,           // kill..truncate
        83, 84, 85, 86, 87, 88, 89, 90, 91,           // fcntl..symlinkat
        92, 93, 94, 98, 99, 100, 101, 102, 103,       // linkat..clock_settime
        104, 107, 108, 113, 114, 115, 116, 117, 127, // timer_create..sigaltstack
        131, 132, 133, 134, 135, 157, 158, 159, 168, // futex..epoll_ctl
        176, 191, 199, 200, 202, 217, 221, 228, 234, // prctl..clone3
        244, 248, 255, 257, 262, 273, 281, 291, 302, // open_tree..process_madvise
        332, 334, 424, 435,
    ];
    #[cfg(target_arch = "aarch64")]
    const AUDIT_ARCH: u32 = 0xc00000b7;

    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    compile_error!("seccomp-BPF only supported on x86_64 and aarch64");

    let mut insns: Vec<[u8; 8]> = Vec::new();

    // 0: LOAD audit_arch
    insns.push(bpf_insn(0x20, 0, 0, 4));
    // 1: JEQ expected_arch ? continue : kill (skip to RET ERRNO)
    let skip_to_deny = ALLOWED.len() + 2; // skip all checks + RET DENY
    insns.push(bpf_insn_j(0x15, skip_to_deny.min(255) as u8, 0, AUDIT_ARCH));
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
    // Use a thread-local cell array to avoid static mut UB
    const MAX_INSNS: usize = 256;

    if insns.len() > MAX_INSNS {
        panic!("seccomp BPF program too large: {} instructions (max {})", insns.len(), MAX_INSNS);
    }

    // Copy instructions into a stack buffer
    let mut insn_bytes = [0u8; MAX_INSNS * 8];
    let src = unsafe { std::slice::from_raw_parts(insns.as_ptr() as *const u8, insns.len() * 8) };
    insn_bytes[..src.len()].copy_from_slice(src);

    let prog_len = insns.len() as u16;
    let prog_ptr = insn_bytes.as_ptr();

    let mut prog = Vec::with_capacity(16);
    prog.extend_from_slice(&prog_len.to_le_bytes());
    prog.resize(8, 0);
    prog.extend_from_slice(&(prog_ptr as u64).to_le_bytes());
    prog
}

pub fn bpf_insn(code: u16, jt: u8, jf: u8, k: u32) -> [u8; 8] {
    let mut buf = [0u8; 8];
    buf[0..2].copy_from_slice(&code.to_le_bytes());
    buf[2] = jt;
    buf[3] = jf;
    buf[4..8].copy_from_slice(&k.to_le_bytes());
    buf
}

pub fn bpf_insn_j(code: u16, jt: u8, jf: u8, k: u32) -> [u8; 8] {
    bpf_insn(code, jt, jf, k)
}

/// Apply seccomp-BPF filter. Requires prctl(PR_SET_NO_NEW_PRIVS, 1) first.
pub fn apply_seccomp() -> io::Result<()> {
    let prog = seccomp_bpf_prog();
    let ret = do_seccomp(
        SECCOMP_SET_MODE_FILTER,
        SECCOMP_FILTER_FLAG_TSYNC,
        prog.as_ptr() as *const c_void,
    );
    if ret == 0 { Ok(()) } else { Err(io::Error::last_os_error()) }
}
