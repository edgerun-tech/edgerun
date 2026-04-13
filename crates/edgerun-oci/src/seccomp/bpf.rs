use std::io;
use std::os::raw::c_void;

use crate::json::{OciLinuxSeccomp, OciSeccompAction};
use crate::syscalls::{do_seccomp, SECCOMP_SET_MODE_FILTER, SECCOMP_FILTER_FLAG_TSYNC, SECCOMP_FILTER_FLAG_NEW_LISTENER};

use super::*;

// BPF instruction builder
// ===========================================================================

pub(crate) fn bpf_insn(code: u16, jt: u8, jf: u8, k: u32) -> [u8; 8] {
    let mut buf = [0u8; 8];
    buf[0..2].copy_from_slice(&code.to_le_bytes());
    buf[2] = jt;
    buf[3] = jf;
    buf[4..8].copy_from_slice(&k.to_le_bytes());
    buf
}

// ===========================================================================
// BPF program generation
// ===========================================================================

/// Emit a chain of unconditional skip instructions when the offset exceeds 255.
///
/// BPF jump offsets are single bytes (max 255). For larger skips we chain
/// `JEQ 0xFFFFFFFF, jt=0, jf=255` instructions — 0xFFFFFFFF never matches
/// a real syscall number or loaded value, so the false-branch (jf) is always
/// taken, effectively acting as an unconditional skip.
pub(crate) fn bpf_long_skip(insns: &mut Vec<[u8; 8]>, mut count: usize) {
    while count > 255 {
        // JEQ A, 0xFFFFFFFF: jt=0 (never — A never equals this), jf=255 (always taken)
        insns.push(bpf_insn(0x15, 0, 255, 0xFFFFFFFF));
        count -= 255;
    }
    if count > 0 {
        insns.push(bpf_insn(0x15, 0, count as u8, 0xFFFFFFFF));
    }
}

