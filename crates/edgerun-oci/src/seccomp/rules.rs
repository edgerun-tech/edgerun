use std::io;
use std::os::raw::c_void;

use crate::json::{OciLinuxSeccomp, OciSeccompAction};
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
    // 1: JEQ expected_arch ? continue : kill
    let skip_to_deny = ALLOWED.len() + 2;
    insns.push(bpf_insn_j(
        0x15,
        skip_to_deny.min(255) as u8,
        0,
        CURRENT_ARCH,
    ));
    // 2: LOAD syscall_nr
    insns.push(bpf_insn(0x20, 0, 0, 0));

    // 3..N: Check each allowed syscall
    let n = ALLOWED.len();
    for (i, &nr) in ALLOWED.iter().enumerate() {
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
// Syscall number constants (x86_64)
// ===========================================================================

#[cfg(target_arch = "x86_64")]
#[allow(dead_code)] // Complete syscall reference table — not all are in allow-list
mod nr {
    pub const READ: u32 = 0;
    pub const WRITE: u32 = 1;
    pub const CLOSE: u32 = 3;
    pub const FSTAT: u32 = 5;
    pub const MMAP: u32 = 9;
    pub const MPROTECT: u32 = 10;
    pub const MUNMAP: u32 = 11;
    pub const BRK: u32 = 12;
    pub const RT_SIGACTION: u32 = 13;
    pub const RT_SIGPROCMASK: u32 = 14;
    pub const RT_SIGRETURN: u32 = 15;
    pub const IOCTL: u32 = 16;
    pub const PREAD64: u32 = 17;
    pub const PWRITE64: u32 = 18;
    pub const READV: u32 = 19;
    pub const WRITEV: u32 = 20;
    pub const ACCESS: u32 = 21;
    pub const POLL: u32 = 22;
    pub const SELECT: u32 = 23;
    pub const SCHED_YIELD: u32 = 24;
    pub const MREMAP: u32 = 25;
    pub const MINCORE: u32 = 27;
    pub const MADVISE: u32 = 28;
    pub const DUP: u32 = 32;
    pub const DUP2: u32 = 33;
    pub const NANOSLEEP: u32 = 35;
    pub const GETPID: u32 = 39;
    pub const SOCKET: u32 = 41;
    pub const CONNECT: u32 = 42;
    pub const ACCEPT: u32 = 43;
    pub const SENDTO: u32 = 44;
    pub const RECVFROM: u32 = 45;
    pub const SENDMSG: u32 = 46;
    pub const RECVMSG: u32 = 47;
    pub const SHUTDOWN: u32 = 48;
    pub const BIND: u32 = 49;
    pub const LISTEN: u32 = 50;
    pub const GETSOCKNAME: u32 = 51;
    pub const GETPEERNAME: u32 = 52;
    pub const SOCKETPAIR: u32 = 53;
    pub const SETSOCKOPT: u32 = 54;
    pub const GETSOCKOPT: u32 = 55;
    pub const CLONE: u32 = 56;
    pub const FORK: u32 = 57;
    pub const VFORK: u32 = 58;
    pub const EXECVE: u32 = 59;
    pub const EXIT: u32 = 60;
    pub const WAIT4: u32 = 61;
    pub const KILL: u32 = 62;
    pub const UNAME: u32 = 63;
    pub const FCNTL: u32 = 72;
    pub const FLOCK: u32 = 73;
    pub const FSYNC: u32 = 74;
    pub const GETCWD: u32 = 79;
    pub const CHDIR: u32 = 80;
    pub const FCHDIR: u32 = 81;
    pub const RENAME: u32 = 82;
    pub const MKDIR: u32 = 83;
    pub const RMDIR: u32 = 84;
    pub const UNLINK: u32 = 86;
    pub const SYMLINK: u32 = 87;
    pub const READLINK: u32 = 88;
    pub const CHMOD: u32 = 90;
    pub const FCHMOD: u32 = 91;
    pub const CHOWN: u32 = 92;
    pub const FCHOWN: u32 = 93;
    pub const GETRLIMIT: u32 = 97;
    pub const GETRUSAGE: u32 = 98;
    pub const GETTIMEOFDAY: u32 = 96;
    pub const SYSINFO: u32 = 99;
    pub const TIMES: u32 = 100;
    pub const GETUID: u32 = 102;
    pub const SYSLOG: u32 = 103;
    pub const GETGID: u32 = 104;
    pub const SETUID: u32 = 105;
    pub const SETGID: u32 = 106;
    pub const GETEUID: u32 = 107;
    pub const GETEGID: u32 = 108;
    pub const SETPGID: u32 = 109;
    pub const GETPPID: u32 = 110;
    pub const GETPGRP: u32 = 111;
    pub const SETSID: u32 = 112;
    pub const SETRESUID: u32 = 117;
    pub const GETRESUID: u32 = 118;
    pub const SETRESGID: u32 = 119;
    pub const GETRESGID: u32 = 120;
    pub const GETPGID: u32 = 121;
    pub const SETFSUID: u32 = 122;
    pub const SETFSGID: u32 = 123;
    pub const GETGROUPS: u32 = 115;
    pub const SETGROUPS: u32 = 116;
    pub const CAPGET: u32 = 125;
    pub const CAPSET: u32 = 126;
    pub const RT_SIGPENDING: u32 = 127;
    pub const RT_SIGTIMEDWAIT: u32 = 128;
    pub const RT_SIGQUEUEINFO: u32 = 129;
    pub const RT_SIGSUSPEND: u32 = 130;
    pub const SIGALTSTACK: u32 = 131;
    pub const TKILL: u32 = 200;
    pub const FUTEX: u32 = 202;
    pub const SCHED_SETPARAM: u32 = 142;
    pub const SCHED_GETPARAM: u32 = 143;
    pub const SCHED_SETSCHEDULER: u32 = 144;
    pub const SCHED_GETSCHEDULER: u32 = 145;
    pub const SCHED_GET_PRIORITY_MAX: u32 = 146;
    pub const SCHED_GET_PRIORITY_MIN: u32 = 147;
    pub const SCHED_RR_GET_INTERVAL: u32 = 148;
    pub const SCHED_SETAFFINITY: u32 = 203;
    pub const SCHED_GETAFFINITY: u32 = 204;
    pub const GETSID: u32 = 124;
    pub const PRCTL: u32 = 157;
    pub const ARCH_PRCTL: u32 = 158;
    pub const SETRLIMIT: u32 = 160;
    pub const MKNOD: u32 = 133;
    pub const PIVOT_ROOT: u32 = 155;
    pub const MOUNT: u32 = 165;
    pub const UMOUNT2: u32 = 166;
    pub const SETNS: u32 = 308;
    pub const UNSHARE: u32 = 272;
    pub const CLOCK_GETTIME: u32 = 228;
    pub const CLOCK_GETRES: u32 = 229;
    pub const CLOCK_NANOSLEEP: u32 = 230;
    pub const EXIT_GROUP: u32 = 231;
    pub const EPOLL_WAIT: u32 = 232;
    pub const EPOLL_CTL: u32 = 233;
    pub const TGKILL: u32 = 234;
    pub const OPENAT: u32 = 257;
    pub const MKDIRAT: u32 = 258;
    pub const MKNODAT: u32 = 259;
    pub const FCHOWNAT: u32 = 260;
    pub const NEWFSTATAT: u32 = 262;
    pub const UNLINKAT: u32 = 263;
    pub const RENAMEAT: u32 = 264;
    pub const LINKAT: u32 = 265;
    pub const SYMLINKAT: u32 = 266;
    pub const READLINKAT: u32 = 267;
    pub const FCHMODAT: u32 = 268;
    pub const FACCESSAT: u32 = 269;
    pub const PSELECT6: u32 = 270;
    pub const PPOLL: u32 = 271;
    pub const SET_ROBUST_LIST: u32 = 273;
    pub const GET_ROBUST_LIST: u32 = 274;
    pub const SIGNALFD: u32 = 282;
    pub const TIMERFD_CREATE: u32 = 283;
    pub const TIMERFD_SETTIME: u32 = 286;
    pub const TIMERFD_GETTIME: u32 = 287;
    pub const EVENTFD: u32 = 284;
    pub const EPOLL_PWAIT: u32 = 281;
    pub const ACCEPT4: u32 = 288;
    pub const SIGNALFD4: u32 = 289;
    pub const EPOLL_CREATE1: u32 = 291;
    pub const DUP3: u32 = 292;
    pub const RECVMMSG: u32 = 299;
    pub const PRLIMIT64: u32 = 302;
    pub const GETRANDOM: u32 = 318;
    pub const SECCOMP: u32 = 317;
    pub const GETDENTS64: u32 = 217;
    pub const SET_TID_ADDRESS: u32 = 218;
    pub const RSEQ: u32 = 334;
    pub const GETCPU: u32 = 309;
    pub const PIDFD_SEND_SIGNAL: u32 = 424;
    pub const PIDFD_OPEN: u32 = 434;
    pub const CLOSE_RANGE: u32 = 436;
    pub const CLONE3: u32 = 435;
    pub const FACCESSAT2: u32 = 439;
    pub const FALLOCATE: u32 = 285;
}

// ===========================================================================
// Syscall number constants (aarch64)
// ===========================================================================

#[cfg(target_arch = "aarch64")]
#[allow(dead_code)] // Complete syscall reference table — not all are in allow-list
mod nr {
    pub const READ: u32 = 63;
    pub const WRITE: u32 = 64;
    pub const CLOSE: u32 = 57;
    pub const FSTAT: u32 = 80;
    pub const MMAP: u32 = 222;
    pub const MPROTECT: u32 = 226;
    pub const MUNMAP: u32 = 215;
    pub const BRK: u32 = 214;
    pub const RT_SIGACTION: u32 = 134;
    pub const RT_SIGPROCMASK: u32 = 135;
    pub const RT_SIGRETURN: u32 = 139;
    pub const IOCTL: u32 = 29;
    pub const PREAD64: u32 = 67;
    pub const PWRITE64: u32 = 68;
    pub const READV: u32 = 65;
    pub const WRITEV: u32 = 66;
    pub const ACCESS: u32 = 48;
    pub const POLL: u32 = 72;
    pub const SELECT: u32 = 73;
    pub const SCHED_YIELD: u32 = 124;
    pub const MREMAP: u32 = 216;
    pub const MINCORE: u32 = 232;
    pub const MADVISE: u32 = 233;
    pub const DUP: u32 = 23;
    pub const DUP2: u32 = 24;
    pub const NANOSLEEP: u32 = 101;
    pub const GETPID: u32 = 172;
    pub const SOCKET: u32 = 198;
    pub const CONNECT: u32 = 203;
    pub const ACCEPT: u32 = 202;
    pub const SENDTO: u32 = 206;
    pub const RECVFROM: u32 = 207;
    pub const SENDMSG: u32 = 211;
    pub const RECVMSG: u32 = 212;
    pub const SHUTDOWN: u32 = 210;
    pub const BIND: u32 = 200;
    pub const LISTEN: u32 = 201;
    pub const GETSOCKNAME: u32 = 204;
    pub const GETPEERNAME: u32 = 205;
    pub const SOCKETPAIR: u32 = 199;
    pub const SETSOCKOPT: u32 = 208;
    pub const GETSOCKOPT: u32 = 209;
    pub const CLONE: u32 = 220;
    pub const EXECVE: u32 = 221;
    pub const EXIT: u32 = 93;
    pub const WAIT4: u32 = 260;
    pub const KILL: u32 = 129;
    pub const UNAME: u32 = 160;
    pub const FCNTL: u32 = 25;
    pub const FLOCK: u32 = 32;
    pub const FSYNC: u32 = 82;
    pub const GETCWD: u32 = 17;
    pub const CHDIR: u32 = 49;
    pub const FCHDIR: u32 = 50;
    pub const RENAME: u32 = 38;
    pub const MKDIR: u32 = 34;
    pub const RMDIR: u32 = 35;
    pub const UNLINK: u32 = 35;
    pub const SYMLINK: u32 = 36;
    pub const READLINK: u32 = 78;
    pub const CHMOD: u32 = 52;
    pub const FCHMOD: u32 = 53;
    pub const CHOWN: u32 = 55;
    pub const FCHOWN: u32 = 55;
    pub const GETRLIMIT: u32 = 163;
    pub const GETRUSAGE: u32 = 165;
    pub const GETTIMEOFDAY: u32 = 169;
    pub const SYSINFO: u32 = 179;
    pub const TIMES: u32 = 153;
    pub const GETUID: u32 = 174;
    pub const SYSLOG: u32 = 116;
    pub const GETGID: u32 = 176;
    pub const SETUID: u32 = 146;
    pub const SETGID: u32 = 144;
    pub const GETEUID: u32 = 175;
    pub const GETEGID: u32 = 177;
    pub const SETPGID: u32 = 154;
    pub const GETPPID: u32 = 173;
    pub const SETSID: u32 = 157;
    pub const SETRESUID: u32 = 147;
    pub const GETRESUID: u32 = 148;
    pub const SETRESGID: u32 = 149;
    pub const GETRESGID: u32 = 150;
    pub const GETPGID: u32 = 155;
    pub const SETFSUID: u32 = 151;
    pub const SETFSGID: u32 = 152;
    pub const GETGROUPS: u32 = 158;
    pub const SETGROUPS: u32 = 159;
    pub const CAPGET: u32 = 90;
    pub const CAPSET: u32 = 91;
    pub const RT_SIGPENDING: u32 = 136;
    pub const RT_SIGTIMEDWAIT: u32 = 137;
    pub const RT_SIGQUEUEINFO: u32 = 138;
    pub const RT_SIGSUSPEND: u32 = 133;
    pub const SIGALTSTACK: u32 = 132;
    pub const TKILL: u32 = 130;
    pub const FUTEX: u32 = 98;
    pub const SCHED_SETPARAM: u32 = 118;
    pub const SCHED_GETPARAM: u32 = 121;
    pub const SCHED_SETSCHEDULER: u32 = 119;
    pub const SCHED_GETSCHEDULER: u32 = 120;
    pub const SCHED_GET_PRIORITY_MAX: u32 = 125;
    pub const SCHED_GET_PRIORITY_MIN: u32 = 126;
    pub const SCHED_RR_GET_INTERVAL: u32 = 127;
    pub const SCHED_SETAFFINITY: u32 = 122;
    pub const SCHED_GETAFFINITY: u32 = 123;
    pub const GETSID: u32 = 156;
    pub const PRCTL: u32 = 167;
    pub const SETRLIMIT: u32 = 164;
    pub const MKNOD: u32 = 33;
    pub const PIVOT_ROOT: u32 = 41;
    pub const MOUNT: u32 = 40;
    pub const UMOUNT2: u32 = 39;
    pub const SETNS: u32 = 268;
    pub const UNSHARE: u32 = 97;
    pub const CLOCK_GETTIME: u32 = 113;
    pub const CLOCK_GETRES: u32 = 114;
    pub const CLOCK_NANOSLEEP: u32 = 115;
    pub const EXIT_GROUP: u32 = 94;
    pub const EPOLL_WAIT: u32 = 21;
    pub const EPOLL_CTL: u32 = 21;
    pub const TGKILL: u32 = 131;
    pub const OPENAT: u32 = 56;
    pub const MKDIRAT: u32 = 34;
    pub const MKNODAT: u32 = 33;
    pub const FCHOWNAT: u32 = 54;
    pub const NEWFSTATAT: u32 = 79;
    pub const UNLINKAT: u32 = 35;
    pub const RENAMEAT: u32 = 38;
    pub const LINKAT: u32 = 37;
    pub const SYMLINKAT: u32 = 36;
    pub const READLINKAT: u32 = 78;
    pub const FCHMODAT: u32 = 53;
    pub const FACCESSAT: u32 = 48;
    pub const PSELECT6: u32 = 72;
    pub const PPOLL: u32 = 73;
    pub const SET_ROBUST_LIST: u32 = 99;
    pub const GET_ROBUST_LIST: u32 = 100;
    pub const SIGNALFD: u32 = 74;
    pub const TIMERFD_CREATE: u32 = 85;
    pub const TIMERFD_SETTIME: u32 = 86;
    pub const TIMERFD_GETTIME: u32 = 87;
    pub const EVENTFD: u32 = 19;
    pub const EPOLL_PWAIT: u32 = 22;
    pub const ACCEPT4: u32 = 242;
    pub const SIGNALFD4: u32 = 74;
    pub const EPOLL_CREATE1: u32 = 20;
    pub const DUP3: u32 = 24;
    pub const RECVMMSG: u32 = 243;
    pub const PRLIMIT64: u32 = 261;
    pub const GETRANDOM: u32 = 278;
    pub const SECCOMP: u32 = 277;
    pub const GETDENTS64: u32 = 61;
    pub const SET_TID_ADDRESS: u32 = 96;
    pub const RSEQ: u32 = 293;
    pub const GETCPU: u32 = 168;
    pub const PIDFD_SEND_SIGNAL: u32 = 424;
    pub const PIDFD_OPEN: u32 = 434;
    pub const CLOSE_RANGE: u32 = 436;
    pub const CLONE3: u32 = 435;
    pub const FACCESSAT2: u32 = 439;
    pub const FALLOCATE: u32 = 47;
}

// ===========================================================================
// Built-in allow-list for fallback when no spec seccomp rules provided
// ===========================================================================

#[cfg(target_arch = "x86_64")]
const ALLOWED: &[u32] = &[
    // I/O fundamentals
    nr::READ,
    nr::WRITE,
    nr::CLOSE,
    nr::PREAD64,
    nr::PWRITE64,
    nr::READV,
    nr::WRITEV,
    nr::GETDENTS64,
    nr::FCNTL,
    nr::FLOCK,
    // File operations
    nr::OPENAT,
    nr::MKDIRAT,
    nr::MKNODAT,
    nr::FCHOWNAT,
    nr::NEWFSTATAT,
    nr::UNLINKAT,
    nr::RENAMEAT,
    nr::LINKAT,
    nr::SYMLINKAT,
    nr::READLINKAT,
    nr::FCHMODAT,
    nr::FACCESSAT,
    nr::FACCESSAT2,
    nr::FSYNC,
    nr::RENAME,
    nr::MKDIR,
    nr::RMDIR,
    nr::UNLINK,
    nr::SYMLINK,
    nr::READLINK,
    nr::CHMOD,
    nr::FCHMOD,
    nr::CHOWN,
    nr::FCHOWN,
    nr::MKNOD,
    // Memory management
    nr::MMAP,
    nr::MPROTECT,
    nr::MUNMAP,
    nr::BRK,
    nr::MREMAP,
    nr::MINCORE,
    nr::MADVISE,
    nr::FALLOCATE,
    // Signal handling
    nr::RT_SIGACTION,
    nr::RT_SIGPROCMASK,
    nr::RT_SIGRETURN,
    nr::KILL,
    nr::RT_SIGPENDING,
    nr::RT_SIGTIMEDWAIT,
    nr::RT_SIGQUEUEINFO,
    nr::RT_SIGSUSPEND,
    nr::SIGALTSTACK,
    nr::TGKILL,
    nr::TKILL,
    // Process management
    nr::GETPID,
    nr::CLONE,
    nr::CLONE3,
    nr::FORK,
    nr::VFORK,
    nr::EXECVE,
    nr::EXIT,
    nr::EXIT_GROUP,
    nr::WAIT4,
    nr::SETPGID,
    nr::GETPPID,
    nr::GETPGRP,
    nr::SETSID,
    nr::GETPGID,
    nr::GETSID,
    // Thread / TLS setup (glibc)
    nr::SET_TID_ADDRESS,
    nr::SET_ROBUST_LIST,
    nr::GET_ROBUST_LIST,
    nr::GETTIMEOFDAY,
    nr::CLOCK_GETTIME,
    nr::CLOCK_GETRES,
    nr::CLOCK_NANOSLEEP,
    nr::NANOSLEEP,
    nr::FUTEX,
    nr::ARCH_PRCTL,
    nr::RSEQ,
    nr::SYSINFO,
    nr::TIMES,
    // Random / entropy
    nr::GETRANDOM,
    // User/identity
    nr::GETUID,
    nr::GETGID,
    nr::SETUID,
    nr::SETGID,
    nr::GETEUID,
    nr::GETEGID,
    nr::GETGROUPS,
    nr::SETGROUPS,
    nr::SETRESUID,
    nr::GETRESUID,
    nr::SETRESGID,
    nr::GETRESGID,
    nr::SETFSUID,
    nr::SETFSGID,
    // Capabilities / security
    nr::CAPGET,
    nr::CAPSET,
    nr::PRCTL,
    nr::SECCOMP,
    // Resource limits
    nr::GETRLIMIT,
    nr::SETRLIMIT,
    nr::PRLIMIT64,
    nr::GETRUSAGE,
    nr::SYSLOG,
    // Network
    nr::SOCKET,
    nr::CONNECT,
    nr::ACCEPT,
    nr::ACCEPT4,
    nr::SENDTO,
    nr::RECVFROM,
    nr::SENDMSG,
    nr::RECVMSG,
    nr::RECVMMSG,
    nr::SHUTDOWN,
    nr::BIND,
    nr::LISTEN,
    nr::GETSOCKNAME,
    nr::GETPEERNAME,
    nr::SOCKETPAIR,
    nr::SETSOCKOPT,
    nr::GETSOCKOPT,
    // Event / epoll / io multiplexing
    nr::POLL,
    nr::SELECT,
    nr::PSELECT6,
    nr::PPOLL,
    nr::EPOLL_WAIT,
    nr::EPOLL_CTL,
    nr::EPOLL_PWAIT,
    nr::EPOLL_CREATE1,
    nr::SIGNALFD,
    nr::SIGNALFD4,
    nr::TIMERFD_CREATE,
    nr::TIMERFD_SETTIME,
    nr::TIMERFD_GETTIME,
    nr::EVENTFD,
    // Scheduling
    nr::SCHED_YIELD,
    nr::SCHED_SETPARAM,
    nr::SCHED_GETPARAM,
    nr::SCHED_SETSCHEDULER,
    nr::SCHED_GETSCHEDULER,
    nr::SCHED_GET_PRIORITY_MAX,
    nr::SCHED_GET_PRIORITY_MIN,
    nr::SCHED_RR_GET_INTERVAL,
    nr::SCHED_SETAFFINITY,
    nr::SCHED_GETAFFINITY,
    // Misc
    nr::UNAME,
    nr::IOCTL,
    nr::ACCESS,
    nr::DUP,
    nr::DUP2,
    nr::DUP3,
    nr::CHDIR,
    nr::FCHDIR,
    nr::GETCWD,
    nr::GETCPU,
    nr::PIDFD_SEND_SIGNAL,
    nr::PIDFD_OPEN,
    nr::CLOSE_RANGE,
];

#[cfg(target_arch = "aarch64")]
const ALLOWED: &[u32] = &[
    // I/O fundamentals
    nr::READ,
    nr::WRITE,
    nr::CLOSE,
    nr::PREAD64,
    nr::PWRITE64,
    nr::READV,
    nr::WRITEV,
    nr::GETDENTS64,
    nr::FCNTL,
    nr::FLOCK,
    // File operations
    nr::OPENAT,
    nr::MKDIRAT,
    nr::MKNODAT,
    nr::FCHOWNAT,
    nr::NEWFSTATAT,
    nr::UNLINKAT,
    nr::RENAMEAT,
    nr::LINKAT,
    nr::SYMLINKAT,
    nr::READLINKAT,
    nr::FCHMODAT,
    nr::FACCESSAT,
    nr::FACCESSAT2,
    nr::FSYNC,
    nr::FCHMOD,
    nr::FCHOWN,
    nr::GETCWD,
    // Memory management
    nr::MMAP,
    nr::MPROTECT,
    nr::MUNMAP,
    nr::BRK,
    nr::MREMAP,
    nr::MINCORE,
    nr::MADVISE,
    nr::FALLOCATE,
    // Signal handling
    nr::RT_SIGACTION,
    nr::RT_SIGPROCMASK,
    nr::RT_SIGRETURN,
    nr::KILL,
    nr::RT_SIGPENDING,
    nr::RT_SIGTIMEDWAIT,
    nr::RT_SIGQUEUEINFO,
    nr::RT_SIGSUSPEND,
    nr::SIGALTSTACK,
    nr::TGKILL,
    nr::TKILL,
    // Process management
    nr::GETPID,
    nr::CLONE,
    nr::CLONE3,
    nr::EXECVE,
    nr::EXIT,
    nr::EXIT_GROUP,
    nr::WAIT4,
    nr::SETPGID,
    nr::GETPPID,
    nr::SETSID,
    nr::GETPGID,
    nr::GETSID,
    // Thread / TLS setup (glibc)
    nr::SET_TID_ADDRESS,
    nr::SET_ROBUST_LIST,
    nr::GET_ROBUST_LIST,
    nr::GETTIMEOFDAY,
    nr::CLOCK_GETTIME,
    nr::CLOCK_GETRES,
    nr::CLOCK_NANOSLEEP,
    nr::NANOSLEEP,
    nr::FUTEX,
    nr::RSEQ,
    nr::SYSINFO,
    nr::TIMES,
    // Random / entropy
    nr::GETRANDOM,
    // User/identity
    nr::GETUID,
    nr::GETGID,
    nr::SETUID,
    nr::SETGID,
    nr::GETEUID,
    nr::GETEGID,
    nr::GETGROUPS,
    nr::SETGROUPS,
    nr::SETRESUID,
    nr::GETRESUID,
    nr::SETRESGID,
    nr::GETRESGID,
    nr::SETFSUID,
    nr::SETFSGID,
    // Capabilities / security
    nr::CAPGET,
    nr::CAPSET,
    nr::PRCTL,
    nr::SECCOMP,
    // Resource limits
    nr::GETRLIMIT,
    nr::SETRLIMIT,
    nr::PRLIMIT64,
    nr::GETRUSAGE,
    nr::SYSLOG,
    // Network
    nr::SOCKET,
    nr::CONNECT,
    nr::ACCEPT,
    nr::ACCEPT4,
    nr::SENDTO,
    nr::RECVFROM,
    nr::SENDMSG,
    nr::RECVMSG,
    nr::RECVMMSG,
    nr::SHUTDOWN,
    nr::BIND,
    nr::LISTEN,
    nr::GETSOCKNAME,
    nr::GETPEERNAME,
    nr::SOCKETPAIR,
    nr::SETSOCKOPT,
    nr::GETSOCKOPT,
    // Event / epoll / io multiplexing
    nr::PSELECT6,
    nr::PPOLL,
    nr::EPOLL_CTL,
    nr::EPOLL_PWAIT,
    nr::EPOLL_CREATE1,
    nr::SIGNALFD4,
    nr::TIMERFD_CREATE,
    nr::TIMERFD_SETTIME,
    nr::TIMERFD_GETTIME,
    nr::EVENTFD,
    // Scheduling
    nr::SCHED_YIELD,
    nr::SCHED_SETPARAM,
    nr::SCHED_GETPARAM,
    nr::SCHED_SETSCHEDULER,
    nr::SCHED_GETSCHEDULER,
    nr::SCHED_GET_PRIORITY_MAX,
    nr::SCHED_GET_PRIORITY_MIN,
    nr::SCHED_RR_GET_INTERVAL,
    nr::SCHED_SETAFFINITY,
    nr::SCHED_GETAFFINITY,
    // Misc
    nr::UNAME,
    nr::IOCTL,
    nr::ACCESS,
    nr::DUP,
    nr::DUP3,
    nr::CHDIR,
    nr::FCHDIR,
    nr::GETCPU,
    nr::PIDFD_SEND_SIGNAL,
    nr::PIDFD_OPEN,
    nr::CLOSE_RANGE,
];

pub(crate) fn bpf_insn_j(code: u16, jt: u8, jf: u8, k: u32) -> [u8; 8] {
    bpf_insn(code, jt, jf, k)
}
