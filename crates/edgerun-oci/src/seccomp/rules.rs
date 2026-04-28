use crate::prelude::*;
use std::io;
use std::os::raw::c_void;

use crate::spec::{OciLinuxSeccomp, OciSeccompAction};
use crate::syscalls::{
    do_seccomp, SECCOMP_FILTER_FLAG_NEW_LISTENER, SECCOMP_FILTER_FLAG_TSYNC,
    SECCOMP_SET_MODE_FILTER,
};

use super::actions::{action_to_bpf, arch_to_bpf};
use super::*;

/// 5. Falls through to default_action if no rule matches
pub fn build_seccomp_prog(spec: &OciLinuxSeccomp) -> (Vec<u8>, Vec<u8>) {
    let mut insns: Vec<[u8; 8]> = Vec::new();

    // 1. Load audit_arch
    insns.push(bpf_insn(0x20, 0, 0, 4));

    // 2. Check architectures
    let archs = spec.architectures.as_deref().unwrap_or(&[]);
    let skip_past_arch_check: usize = if archs.is_empty() {
        1 // skip RET_KILL if not matching
    } else {
        archs.len() + 1
    };

    if archs.is_empty() {
        if skip_past_arch_check > 255 {
            bpf_long_skip(&mut insns, skip_past_arch_check);
        } else {
            insns.push(bpf_insn_j(
                0x15,
                skip_past_arch_check as u8,
                0,
                CURRENT_ARCH,
            ));
        }
    } else {
        for arch in archs {
            let arch_nr = arch_to_bpf(arch);
            if skip_past_arch_check > 255 {
                bpf_long_skip(&mut insns, skip_past_arch_check);
            } else {
                insns.push(bpf_insn_j(0x15, skip_past_arch_check as u8, 0, arch_nr));
            }
        }
    }
    insns.push(bpf_insn(0x06, 0, 0, 0x00000000)); // SECCOMP_RET_KILL_THREAD

    // 3. Load syscall_nr
    insns.push(bpf_insn(0x20, 0, 0, 0));

    // 4. Build syscall rules with argument filters
    let entries = spec.syscalls.as_deref().unwrap_or(&[]);
    let default_action = spec
        .default_action
        .as_ref()
        .unwrap_or(&OciSeccompAction::Kill);
    let default_ret = action_to_bpf(default_action, spec.default_errno_ret);

    for (i, entry) in entries.iter().enumerate() {
        let Some(names) = entry.names.as_ref() else {
            continue;
        };
        let action = entry.action.as_ref().unwrap_or(default_action);
        let ret_val = action_to_bpf(action, entry.errno_ret);
        let args = entry.args.as_deref().unwrap_or(&[]);

        // Count how many instructions this syscall entry will generate
        // Per syscall name: 1 (JEQ) + arg_check_count + 1 (RET)
        let arg_check_insns = args.len() * 2; // 2 insns per arg (low + high 32-bit)
        let per_syscall_insns = 1 + arg_check_insns + 1; // JEQ + args + RET

        for name in names {
            let Some(nr) = syscall_nr(name) else { continue };

            // Count remaining instructions after this JEQ
            let remaining = {
                let mut count = 0;
                // For this syscall: arg checks + RET
                count += arg_check_insns + 1;
                // For remaining syscalls in this entry
                let remaining_in_entry =
                    names.len() - names.iter().position(|n| n == name).unwrap_or(0) - 1;
                count += remaining_in_entry * per_syscall_insns;
                // For remaining entries
                for entry in entries.iter().skip(i + 1) {
                    if let Some(e) = entry.names.as_ref() {
                        count += e.len();
                    }
                }
                // Default RET
                count += 1;
                count
            };

            // JEQ: if nr matches, fall through to arg checks (jt=0)
            //       if not, skip past this syscall's block (jf=remaining)
            if remaining > 255 {
                // For large skips: first check if nr matches.
                // If equal → jt=0 (fall through to arg checks).
                // If not equal → jt=1 (skip the long-jump chain), jf=0.
                // Then emit the long-jump chain.
                insns.push(bpf_insn(0x15, 1, 0, nr));
                bpf_long_skip(&mut insns, remaining);
            } else {
                insns.push(bpf_insn(0x15, 0, remaining as u8, nr));
            }

            // Generate arg filter checks
            for (arg_idx, arg) in args.iter().enumerate() {
                let remaining_args = args.len() - arg_idx - 1;
                // For EQ/GE/GT/LE/LT: mismatch → skip past remaining args + RET to next entry/default
                let skip_to_default = remaining_args * 2 + 1;
                // For NE: mismatch means values differ → fall through to RET (skip remaining checks only)
                let skip_to_ret = remaining_args * 2;

                // Load low 32 bits of arg (offset = 16 + index*8)
                let arg_offset = 16 + arg.index * 8;
                insns.push(bpf_insn(0x20, 0, 0, arg_offset));
                let lo_val = arg.value as u32;

                // BPF comparison operators:
                //   0x25 = BPF_JGT (unsigned): jt if A > K, jf if A <= K
                //   0x30 = BPF_JGE (unsigned): jt if A >= K, jf if A < K
                //   0x15 = BPF_JEQ: jt if A == K, jf if A != K
                //
                // For each operator we want: "skip when comparison is FALSE"
                //   EQ: A != K → skip  → JEQ, jt=0, jf=skip  ✓
                //   NE: A == K → skip  → JEQ, jt=skip, jf=0  ✓
                //   LT: A < K true → continue. A >= K → skip  → JGE, jt=skip, jf=0
                //   LE: A <= K true → continue. A > K → skip  → JGT, jt=skip, jf=0
                //   GE: A >= K true → continue. A < K → skip  → JGE, jt=0, jf=skip
                //   GT: A > K true → continue. A <= K → skip  → JGT, jt=0, jf=skip

                match arg.op.as_str() {
                    // EQ: A == K → match → continue (jt=0). A != K → no match → skip (jf=skip)
                    "SCMP_CMP_EQ" => {
                        if skip_to_default > 255 {
                            insns.push(bpf_insn(0x15, 1, 0, lo_val));
                            bpf_long_skip(&mut insns, skip_to_default);
                        } else {
                            insns.push(bpf_insn(0x15, 0, skip_to_default as u8, lo_val));
                        }
                    }
                    // NE: A != K → match → jump to RET (jf=skip_to_ret). A == K → continue to hi (jt=0)
                    "SCMP_CMP_NE" => {
                        if skip_to_ret > 255 {
                            bpf_long_skip(&mut insns, skip_to_ret);
                            insns.push(bpf_insn(0x15, 1, 0, lo_val));
                        } else {
                            insns.push(bpf_insn(0x15, skip_to_ret as u8, 0, lo_val));
                        }
                    }
                    // LT: A < K. If A >= K → skip. JGE: if A >= K → jt=skip.
                    "SCMP_CMP_LT" => {
                        if skip_to_default > 255 {
                            insns.push(bpf_insn(0x30, 1, 0, lo_val));
                            bpf_long_skip(&mut insns, skip_to_default);
                        } else {
                            insns.push(bpf_insn(0x30, skip_to_default as u8, 0, lo_val));
                        }
                    }
                    // LE: A <= K. If A > K → skip. JGT: if A > K → jt=skip.
                    "SCMP_CMP_LE" => {
                        if skip_to_default > 255 {
                            insns.push(bpf_insn(0x25, 1, 0, lo_val));
                            bpf_long_skip(&mut insns, skip_to_default);
                        } else {
                            insns.push(bpf_insn(0x25, skip_to_default as u8, 0, lo_val));
                        }
                    }
                    // GE: A >= K. If A >= K → continue (jt=0). If A < K → skip (jf=skip).
                    "SCMP_CMP_GE" => {
                        if skip_to_default > 255 {
                            insns.push(bpf_insn(0x30, 0, 1, lo_val));
                            bpf_long_skip(&mut insns, skip_to_default);
                        } else {
                            insns.push(bpf_insn(0x30, 0, skip_to_default as u8, lo_val));
                        }
                    }
                    // GT: A > K. If A > K → continue (jt=0). If A <= K → skip (jf=skip).
                    "SCMP_CMP_GT" => {
                        if skip_to_default > 255 {
                            insns.push(bpf_insn(0x25, 0, 1, lo_val));
                            bpf_long_skip(&mut insns, skip_to_default);
                        } else {
                            insns.push(bpf_insn(0x25, 0, skip_to_default as u8, lo_val));
                        }
                    }
                    // MASKED_EQ: (A & mask) == valueTwo
                    //   Step 1: AND low 32 bits with mask, check == expected low
                    //   Step 2: AND high 32 bits with mask, check == expected high
                    //   Both halves must match to pass.
                    "SCMP_CMP_MASKED_EQ" => {
                        // AND low 32 bits with mask, then check == expected low
                        insns.push(bpf_insn(0x50, 0, 0, arg.value as u32));
                        // JEQ expected_lo → continue to hi check. Mismatch → skip past hi check + RET + remaining args
                        let skip_total = remaining_args * 2 + 2 + 1; // remaining + hi_load + hi_jeq + ret
                        if skip_total > 255 {
                            insns.push(bpf_insn(0x15, 1, 0, arg.value_two as u32));
                            bpf_long_skip(&mut insns, skip_total);
                        } else {
                            insns.push(bpf_insn(0x15, 0, skip_total as u8, arg.value_two as u32));
                        }
                    }
                    _ => {
                        if skip_to_default > 255 {
                            insns.push(bpf_insn(0x15, 1, 0, lo_val));
                            bpf_long_skip(&mut insns, skip_to_default);
                        } else {
                            insns.push(bpf_insn(0x15, 0, skip_to_default as u8, lo_val));
                        }
                    }
                }

                // Load high 32 bits of arg (offset = 16 + index*8 + 4)
                let arg_offset_hi = 16 + arg.index * 8 + 4;
                let hi_val = (arg.value >> 32) as u32;
                insns.push(bpf_insn(0x20, 0, 0, arg_offset_hi));

                match arg.op.as_str() {
                    // EQ: both match → fall through to RET (jt=0). hi mismatch → skip (jf=1)
                    "SCMP_CMP_EQ" => {
                        insns.push(bpf_insn(0x15, 0, 1, hi_val));
                    }
                    // NE: both match (full equality) → skip past RET (jt=1). hi differs → match → RET (jf=0)
                    "SCMP_CMP_NE" => {
                        insns.push(bpf_insn(0x15, 1, 0, hi_val));
                    }
                    // LT: A < K. If hi > K_hi → A > K → skip. If hi < K_hi → A < K → continue.
                    "SCMP_CMP_LT" => {
                        insns.push(bpf_insn(0x25, 1, 0, hi_val));
                    }
                    // LE: A <= K. If hi > K_hi → skip.
                    "SCMP_CMP_LE" => {
                        insns.push(bpf_insn(0x25, 1, 0, hi_val));
                    }
                    // GE: A >= K. If hi < K_hi → skip.
                    "SCMP_CMP_GE" => {
                        insns.push(bpf_insn(0x30, 0, 1, hi_val));
                    }
                    // GT: A > K. If hi > K_hi → continue. If hi <= K_hi → skip.
                    //   (lo already matched exactly, so hi==K_hi means A==K, not >)
                    //   JGT: if A > K → jt=0 (continue). if A <= K → jf=1 (skip RET).
                    "SCMP_CMP_GT" => {
                        insns.push(bpf_insn(0x25, 0, 1, hi_val));
                    }
                    // MASKED_EQ hi: AND high 32 bits with mask, check == expected high
                    "SCMP_CMP_MASKED_EQ" => {
                        let mask_hi = (arg.value >> 32) as u32;
                        let expected_hi = (arg.value_two >> 32) as u32;
                        insns.push(bpf_insn(0x50, 0, 0, mask_hi)); // A = A & mask_hi
                                                                   // Skip past RET on mismatch
                        insns.push(bpf_insn(0x15, 0, 1, expected_hi));
                    }
                    _ => {
                        insns.push(bpf_insn(0x15, 0, 1, hi_val));
                    }
                }
            }

            // All args matched — apply action
            insns.push(bpf_insn(0x06, 0, 0, ret_val));
        }
    }

    // 5. Default action (fallthrough)
    insns.push(bpf_insn(0x06, 0, 0, default_ret));

    // Build sock_fprog: { len: u16, filter: *sock_filter }
    // Return owned instruction bytes so the pointer stays valid after the function returns.
    let insn_bytes: Vec<u8> = insns.into_iter().flatten().collect();

    let prog_len = insn_bytes.len() as u16 / 8;
    let filter_ptr = insn_bytes.as_ptr();

    let mut prog = Vec::with_capacity(16);
    prog.extend_from_slice(&prog_len.to_le_bytes());
    prog.resize(8, 0); // padding for alignment
    prog.extend_from_slice(&(filter_ptr as u64).to_le_bytes());
    (insn_bytes, prog)
}

/// Generate the built-in fallback allow-list when no spec seccomp rules are present.
///
/// Returns the instruction bytes (owned Vec) and a sock_fprog pointer.
/// The caller must keep `insn_bytes` alive while the seccomp syscall runs.
pub fn seccomp_bpf_prog() -> (Vec<u8>, Vec<u8>) {
    let mut insns: Vec<[u8; 8]> = Vec::new();

    // 0: LOAD audit_arch
    insns.push(bpf_insn(0x20, 0, 0, 4));
    let allowed = allowed_syscalls();

    // 1: JEQ expected_arch ? continue : kill
    let skip_to_deny = allowed.len() + 2;
    insns.push(bpf_insn_j(
        0x15,
        skip_to_deny.min(255) as u8,
        0,
        CURRENT_ARCH,
    ));
    // 2: LOAD syscall_nr
    insns.push(bpf_insn(0x20, 0, 0, 0));

    // 3..N: Check each allowed syscall
    let n = allowed.len();
    for (i, nr) in allowed.into_iter().enumerate() {
        let skip = (n - i).min(255) as u8;
        insns.push(bpf_insn_j(0x15, skip, 0, nr));
    }

    // RET ERRNO(EPERM) — default deny
    insns.push(bpf_insn(0x06, 0, 0, 0x00050001));
    // RET ALLOW
    insns.push(bpf_insn(0x06, 0, 0, 0x7fff0000));

    // Return owned instruction bytes so the pointer stays valid
    let insn_bytes: Vec<u8> = insns.into_iter().flatten().collect();

    // Build sock_fprog { len, filter }
    let prog_len = insn_bytes.len() as u16 / 8;
    let filter_ptr = insn_bytes.as_ptr();

    let mut prog = Vec::with_capacity(16);
    prog.extend_from_slice(&prog_len.to_le_bytes());
    prog.resize(8, 0); // padding for alignment
    prog.extend_from_slice(&(filter_ptr as u64).to_le_bytes());

    (insn_bytes, prog)
}

// ===========================================================================
// Built-in allow-list for fallback when no spec seccomp rules are provided
// ===========================================================================

fn allowed_syscalls() -> Vec<u32> {
    ALLOWED_SYSCALL_NAMES
        .iter()
        .filter_map(|name| syscall_nr(name))
        .collect()
}

const ALLOWED_SYSCALL_NAMES: &[&str] = &[
    // I/O fundamentals
    "read",
    "write",
    "close",
    "pread64",
    "pwrite64",
    "readv",
    "writev",
    "getdents64",
    "fcntl",
    "flock",
    // File operations
    "openat",
    "mkdirat",
    "mknodat",
    "fchownat",
    "newfstatat",
    "unlinkat",
    "renameat",
    "linkat",
    "symlinkat",
    "readlinkat",
    "fchmodat",
    "faccessat",
    "faccessat2",
    "fsync",
    "rename",
    "mkdir",
    "rmdir",
    "unlink",
    "symlink",
    "readlink",
    "chmod",
    "fchmod",
    "chown",
    "fchown",
    "mknod",
    "getcwd",
    // Memory management
    "mmap",
    "mprotect",
    "munmap",
    "brk",
    "mremap",
    "mincore",
    "madvise",
    "fallocate",
    // Signal handling
    "rt_sigaction",
    "rt_sigprocmask",
    "rt_sigreturn",
    "kill",
    "rt_sigpending",
    "rt_sigtimedwait",
    "rt_sigqueueinfo",
    "rt_sigsuspend",
    "sigaltstack",
    "tgkill",
    "tkill",
    // Process management
    "getpid",
    "clone",
    "clone3",
    "fork",
    "vfork",
    "execve",
    "exit",
    "exit_group",
    "wait4",
    "setpgid",
    "getppid",
    "getpgrp",
    "setsid",
    "getpgid",
    "getsid",
    // Thread / TLS setup
    "set_tid_address",
    "set_robust_list",
    "get_robust_list",
    "gettimeofday",
    "clock_gettime",
    "clock_getres",
    "clock_nanosleep",
    "nanosleep",
    "futex",
    "arch_prctl",
    "rseq",
    "sysinfo",
    "times",
    // Random / entropy
    "getrandom",
    // User/identity
    "getuid",
    "getgid",
    "setuid",
    "setgid",
    "geteuid",
    "getegid",
    "getgroups",
    "setgroups",
    "setresuid",
    "getresuid",
    "setresgid",
    "getresgid",
    "setfsuid",
    "setfsgid",
    // Capabilities / security
    "capget",
    "capset",
    "prctl",
    "seccomp",
    // Resource limits
    "getrlimit",
    "setrlimit",
    "prlimit64",
    "getrusage",
    "syslog",
    // Network
    "socket",
    "connect",
    "accept",
    "accept4",
    "sendto",
    "recvfrom",
    "sendmsg",
    "recvmsg",
    "recvmmsg",
    "shutdown",
    "bind",
    "listen",
    "getsockname",
    "getpeername",
    "socketpair",
    "setsockopt",
    "getsockopt",
    // Event / epoll / io multiplexing
    "poll",
    "select",
    "pselect6",
    "ppoll",
    "epoll_wait",
    "epoll_ctl",
    "epoll_pwait",
    "epoll_create1",
    "signalfd",
    "signalfd4",
    "timerfd_create",
    "timerfd_settime",
    "timerfd_gettime",
    "eventfd",
    // Scheduling
    "sched_yield",
    "sched_setparam",
    "sched_getparam",
    "sched_setscheduler",
    "sched_getscheduler",
    "sched_get_priority_max",
    "sched_get_priority_min",
    "sched_rr_get_interval",
    "sched_setaffinity",
    "sched_getaffinity",
    // Misc
    "uname",
    "ioctl",
    "access",
    "dup",
    "dup2",
    "dup3",
    "chdir",
    "fchdir",
    "getcpu",
    "pidfd_send_signal",
    "pidfd_open",
    "close_range",
];

pub(crate) fn bpf_insn_j(code: u16, jt: u8, jf: u8, k: u32) -> [u8; 8] {
    bpf_insn(code, jt, jf, k)
}
