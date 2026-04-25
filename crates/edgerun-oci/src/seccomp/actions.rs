use std::io;
use std::os::raw::c_void;

use crate::json::{OciLinuxSeccomp, OciSeccompAction};
use crate::syscalls::{
    do_seccomp, SECCOMP_FILTER_FLAG_NEW_LISTENER, SECCOMP_FILTER_FLAG_TSYNC,
    SECCOMP_SET_MODE_FILTER,
};

use super::*;

pub(crate) fn action_to_bpf(action: &OciSeccompAction, errno_ret: Option<u32>) -> u32 {
    match action {
        OciSeccompAction::Allow => 0x7fff0000, // SECCOMP_RET_ALLOW
        OciSeccompAction::Kill => 0x00000000,  // SECCOMP_RET_KILL_THREAD
        OciSeccompAction::KillProcess => 0x80000000, // SECCOMP_RET_KILL_PROCESS
        OciSeccompAction::KillThread => 0x00000000, // same as Kill
        OciSeccompAction::Trap => 0x00030000,  // SECCOMP_RET_TRAP
        OciSeccompAction::Errno => 0x00050000 | (errno_ret.unwrap_or(1) & 0x0000ffff), // SECCOMP_RET_ERRNO
        OciSeccompAction::Trace => 0x7ff00000, // SECCOMP_RET_TRACE
        OciSeccompAction::Log => 0x7ffe0000,   // SECCOMP_RET_LOG
        OciSeccompAction::Notify => 0x7fc00000, // SECCOMP_RET_NOTIFY
    }
}

pub(crate) fn arch_to_bpf(arch: &str) -> u32 {
    match arch {
        "SCMP_ARCH_X86_64" => 0xc000003e,
        "SCMP_ARCH_X86" => 0x40000003,
        "SCMP_ARCH_X32" => 0x4000003e,
        "SCMP_ARCH_AARCH64" => 0xc00000b7,
        "SCMP_ARCH_ARM" => 0x40000028,
        _ => CURRENT_ARCH,
    }
}
