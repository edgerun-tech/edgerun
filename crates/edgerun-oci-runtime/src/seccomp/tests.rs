use std::io;
use std::os::raw::c_void;

use crate::json::{OciLinuxSeccomp, OciSeccompAction};
use crate::syscalls::{do_seccomp, SECCOMP_SET_MODE_FILTER, SECCOMP_FILTER_FLAG_TSYNC, SECCOMP_FILTER_FLAG_NEW_LISTENER};

use super::*;
use actions::{action_to_bpf, arch_to_bpf};

use crate::json::OciSeccompSyscallEntry;

#[test]
fn seccomp_bpf_prog_is_non_empty() {
    let (insns, prog) = seccomp_bpf_prog();
    assert!(!insns.is_empty());
    assert!(!prog.is_empty());
}

#[test]
fn seccomp_bpf_prog_has_valid_structure() {
    let (insns, prog) = seccomp_bpf_prog();
    assert!(prog.len() >= 16);
    let len = u16::from_le_bytes([prog[0], prog[1]]) as usize;
    assert!(len > 10);
    assert_eq!(insns.len(), len * 8);
}

#[test]
fn seccomp_bpf_prog_contains_allow_and_deny() {
    let (insns, prog) = seccomp_bpf_prog();
    assert!(prog.len() >= 16, "sock_fprog should be at least 16 bytes");
    let len = u16::from_le_bytes([prog[0], prog[1]]) as usize;
    assert!(len > 10, "should have many BPF instructions, got {}", len);

    let ptr_bytes: [u8; 8] = prog[8..16].try_into().unwrap();
    let ptr = u64::from_le_bytes(ptr_bytes);
    assert_ne!(ptr, 0, "filter pointer should be non-null");

    // Verify the pointer points into our owned Vec
    assert!(ptr >= insns.as_ptr() as u64);
    assert!(ptr < (insns.as_ptr() as u64 + insns.len() as u64));

    let insns_slice: &[[u8; 8]] = unsafe { std::slice::from_raw_parts(ptr as *const [u8; 8], len) };
    let mut found_allow = false;
    let mut found_deny = false;
    for insn in insns_slice {
        let k = u32::from_le_bytes([insn[4], insn[5], insn[6], insn[7]]);
        if k == 0x7fff0000 { found_allow = true; }
        if k == 0x00050001 { found_deny = true; }
    }
    assert!(found_allow, "should contain RET_ALLOW (0x7fff0000)");
    assert!(found_deny, "should contain RET_ERRNO(EPERM) (0x00050001)");
}

#[test]
fn bpf_insn_produces_8_bytes() {
    let insn = bpf_insn(0x06, 0, 0, 0x7fff0000);
    assert_eq!(insn.len(), 8);
}

#[test]
fn bpf_insn_ret_allow_encoding() {
    let insn = bpf_insn(0x06, 0, 0, 0x7fff0000);
    assert_eq!(&insn[0..2], &[0x06, 0x00]);
    assert_eq!(&insn[4..8], &[0x00, 0x00, 0xff, 0x7f]);
}

#[test]
fn build_seccomp_prog_with_spec_rules() {
    let spec = OciLinuxSeccomp {
        default_action: Some(OciSeccompAction::Kill),
        default_errno_ret: None,
        architectures: Some(vec!["SCMP_ARCH_X86_64".into()]),
        listener_path: None,
        listener_metadata: None,
        syscalls: Some(vec![
            OciSeccompSyscallEntry {
                names: Some(vec!["getcwd".into(), "chmod".into()]),
                action: Some(OciSeccompAction::Allow),
                errno_ret: None,
                args: None,
            },
            OciSeccompSyscallEntry {
                names: Some(vec!["openat".into()]),
                action: Some(OciSeccompAction::Errno),
                errno_ret: Some(13), // EACCES
                args: None,
            },
        ]),
    };
    let (_, prog) = build_seccomp_prog(&spec);
    assert!(prog.len() >= 16);
    let len = u16::from_le_bytes([prog[0], prog[1]]) as usize;
    assert!(len > 5, "should have BPF instructions, got {}", len);
}

#[test]
fn build_seccomp_prog_empty_spec_uses_fallback() {
    let spec = OciLinuxSeccomp {
        default_action: Some(OciSeccompAction::Kill),
        default_errno_ret: None,
        architectures: None,
        listener_path: None,
        listener_metadata: None,
        syscalls: None,
    };
    let (_, prog) = build_seccomp_prog(&spec);
    // Should still generate a valid program even with empty rules
    assert!(prog.len() >= 16);
}

#[test]
fn build_seccomp_prog_with_arg_filters() {
    let spec = OciLinuxSeccomp {
        default_action: Some(OciSeccompAction::Kill),
        default_errno_ret: None,
        architectures: Some(vec!["SCMP_ARCH_X86_64".into()]),
        listener_path: None,
        listener_metadata: None,
        syscalls: Some(vec![
            OciSeccompSyscallEntry {
                names: Some(vec!["openat".into()]),
                action: Some(OciSeccompAction::Errno),
                errno_ret: Some(13),
                args: Some(vec![crate::json::OciSeccompArg {
                    index: 1,
                    value: 0o100000,
                    value_two: 0,
                    op: "SCMP_CMP_EQ".into(),
                }]),
            },
        ]),
    };
    let (_, prog) = build_seccomp_prog(&spec);
    assert!(prog.len() >= 16);
    let len = u16::from_le_bytes([prog[0], prog[1]]) as usize;
    assert!(len >= 8, "should have many BPF instructions, got {}", len);
}

#[test]
fn build_seccomp_prog_with_ne_arg_filter() {
    let spec = OciLinuxSeccomp {
        default_action: Some(OciSeccompAction::Allow),
        default_errno_ret: None,
        architectures: None,
        listener_path: None,
        listener_metadata: None,
        syscalls: Some(vec![
            OciSeccompSyscallEntry {
                names: Some(vec!["ioctl".into()]),
                action: Some(OciSeccompAction::Kill),
                errno_ret: None,
                args: Some(vec![crate::json::OciSeccompArg {
                    index: 1,
                    value: 0x5401,
                    value_two: 0,
                    op: "SCMP_CMP_NE".into(),
                }]),
            },
        ]),
    };
    let (_, prog) = build_seccomp_prog(&spec);
    let len = u16::from_le_bytes([prog[0], prog[1]]) as usize;
    assert!(len >= 5, "should have BPF instructions, got {}", len);
}

#[test]
fn action_to_bpf_values() {
    assert_eq!(action_to_bpf(&OciSeccompAction::Allow, None), 0x7fff0000);
    assert_eq!(action_to_bpf(&OciSeccompAction::Kill, None), 0x00000000);
    assert_eq!(action_to_bpf(&OciSeccompAction::Errno, Some(1)), 0x00050001);
    assert_eq!(action_to_bpf(&OciSeccompAction::Errno, Some(13)), 0x0005000d);
}

#[test]
fn syscall_nr_known_for_common_calls() {
    // On x86_64
    #[cfg(target_arch = "x86_64")]
    {
        assert_eq!(syscall_nr("read"), Some(0));
        assert_eq!(syscall_nr("write"), Some(1));
        assert_eq!(syscall_nr("exit"), Some(60));
        assert_eq!(syscall_nr("getpid"), Some(39));
    }
    // On aarch64
    #[cfg(target_arch = "aarch64")]
    {
        assert_eq!(syscall_nr("read"), Some(63));
        assert_eq!(syscall_nr("write"), Some(64));
        assert_eq!(syscall_nr("exit"), Some(93));
        assert_eq!(syscall_nr("getpid"), Some(172));
    }
}

/// Verify that LT uses JGE (0x30) with jt=skip, jf=0
#[test]
fn bpf_lt_uses_jge() {
    let spec = OciLinuxSeccomp {
        default_action: Some(OciSeccompAction::Kill),
        default_errno_ret: None,
        architectures: Some(vec!["SCMP_ARCH_X86_64".into()]),
        listener_path: None,
        listener_metadata: None,
        syscalls: Some(vec![
            OciSeccompSyscallEntry {
                names: Some(vec!["openat".into()]),
                action: Some(OciSeccompAction::Allow),
                errno_ret: None,
                args: Some(vec![crate::json::OciSeccompArg {
                    index: 0, value: 0x100, value_two: 0, op: "SCMP_CMP_LT".into(),
                }]),
            },
        ]),
    };
    let (_, prog) = build_seccomp_prog(&spec);
    let len = u16::from_le_bytes([prog[0], prog[1]]) as usize;
    assert!(len >= 8, "should have multiple instructions");

    let ptr_bytes: [u8; 8] = prog[8..16].try_into().unwrap();
    let ptr = u64::from_le_bytes(ptr_bytes);
    assert_ne!(ptr, 0);

    unsafe {
        let insns = std::slice::from_raw_parts(ptr as *const [u8; 8], len);
        let mut found_jge_for_lt = false;
        let mut found_jgt_for_hi = false;
        for insn in insns {
            let code = u16::from_le_bytes([insn[0], insn[1]]);
            let jt = insn[2];
            let jf = insn[3];
            let k = u32::from_le_bytes([insn[4], insn[5], insn[6], insn[7]]);
            // JGE with jt=skip, jf=0 for lo check (LT: skip when >=)
            if code == 0x30 && jt > 0 && jf == 0 && k == 0x100 {
                found_jge_for_lt = true;
            }
            // JGT with jt=1, jf=0 for hi check
            if code == 0x25 && jt == 1 && jf == 0 && k == 0 {
                found_jgt_for_hi = true;
            }
        }
        assert!(found_jge_for_lt, "LT should use JGE (0x30) with jt=skip, jf=0 for lo");
        assert!(found_jgt_for_hi, "LT should use JGT (0x25) with jt=1, jf=0 for hi");
    }
}

/// Verify that GT uses JGT (0x25) with jt=0, jf=skip for hi check
#[test]
fn bpf_gt_uses_jgt() {
    let spec = OciLinuxSeccomp {
        default_action: Some(OciSeccompAction::Allow),
        default_errno_ret: None,
        architectures: Some(vec!["SCMP_ARCH_X86_64".into()]),
        listener_path: None,
        listener_metadata: None,
        syscalls: Some(vec![
            OciSeccompSyscallEntry {
                names: Some(vec!["openat".into()]),
                action: Some(OciSeccompAction::Allow),
                errno_ret: None,
                args: Some(vec![crate::json::OciSeccompArg {
                    index: 0, value: 0x100, value_two: 0, op: "SCMP_CMP_GT".into(),
                }]),
            },
        ]),
    };
    let (_, prog) = build_seccomp_prog(&spec);
    let len = u16::from_le_bytes([prog[0], prog[1]]) as usize;
    assert!(len >= 8);

    let ptr_bytes: [u8; 8] = prog[8..16].try_into().unwrap();
    let ptr = u64::from_le_bytes(ptr_bytes);
    assert_ne!(ptr, 0);

    unsafe {
        let insns = std::slice::from_raw_parts(ptr as *const [u8; 8], len);
        let mut found_jgt_lo = false;
        let mut found_jgt_hi = false;
        for insn in insns {
            let code = u16::from_le_bytes([insn[0], insn[1]]);
            let jt = insn[2];
            let jf = insn[3];
            let k = u32::from_le_bytes([insn[4], insn[5], insn[6], insn[7]]);
            if code == 0x25 && jt == 0 && jf > 0 && k == 0x100 {
                found_jgt_lo = true;
            }
            if code == 0x25 && jt == 0 && jf == 1 && k == 0 {
                found_jgt_hi = true;
            }
        }
        assert!(found_jgt_lo, "GT should use JGT (0x25) with jt=0, jf=skip for lo");
        assert!(found_jgt_hi, "GT should use JGT (0x25) with jt=0, jf=1 for hi");
    }
}

/// Verify GE uses JGE (0x30)
#[test]
fn bpf_ge_uses_jge() {
    let spec = OciLinuxSeccomp {
        default_action: Some(OciSeccompAction::Allow),
        default_errno_ret: None,
        architectures: Some(vec!["SCMP_ARCH_X86_64".into()]),
        listener_path: None,
        listener_metadata: None,
        syscalls: Some(vec![
            OciSeccompSyscallEntry {
                names: Some(vec!["openat".into()]),
                action: Some(OciSeccompAction::Allow),
                errno_ret: None,
                args: Some(vec![crate::json::OciSeccompArg {
                    index: 0, value: 0x100, value_two: 0, op: "SCMP_CMP_GE".into(),
                }]),
            },
        ]),
    };
    let (_, prog) = build_seccomp_prog(&spec);
    let len = u16::from_le_bytes([prog[0], prog[1]]) as usize;
    assert!(len >= 8);

    let ptr_bytes: [u8; 8] = prog[8..16].try_into().unwrap();
    let ptr = u64::from_le_bytes(ptr_bytes);
    assert_ne!(ptr, 0);

    unsafe {
        let insns = std::slice::from_raw_parts(ptr as *const [u8; 8], len);
        let mut found_jge_lo = false;
        let mut found_jge_hi = false;
        for insn in insns {
            let code = u16::from_le_bytes([insn[0], insn[1]]);
            let jt = insn[2];
            let jf = insn[3];
            let k = u32::from_le_bytes([insn[4], insn[5], insn[6], insn[7]]);
            if code == 0x30 && jt == 0 && jf > 0 && k == 0x100 {
                found_jge_lo = true;
            }
            if code == 0x30 && jt == 0 && jf == 1 && k == 0 {
                found_jge_hi = true;
            }
        }
        assert!(found_jge_lo, "GE should use JGE (0x30) with jt=0, jf=skip for lo");
        assert!(found_jge_hi, "GE should use JGE (0x30) with jt=0, jf=1 for hi");
    }
}

/// Verify LE uses JGT (0x25)
#[test]
fn bpf_le_uses_jgt() {
    let spec = OciLinuxSeccomp {
        default_action: Some(OciSeccompAction::Allow),
        default_errno_ret: None,
        architectures: Some(vec!["SCMP_ARCH_X86_64".into()]),
        listener_path: None,
        listener_metadata: None,
        syscalls: Some(vec![
            OciSeccompSyscallEntry {
                names: Some(vec!["openat".into()]),
                action: Some(OciSeccompAction::Allow),
                errno_ret: None,
                args: Some(vec![crate::json::OciSeccompArg {
                    index: 0, value: 0x100, value_two: 0, op: "SCMP_CMP_LE".into(),
                }]),
            },
        ]),
    };
    let (_, prog) = build_seccomp_prog(&spec);
    let len = u16::from_le_bytes([prog[0], prog[1]]) as usize;
    assert!(len >= 8);

    let ptr_bytes: [u8; 8] = prog[8..16].try_into().unwrap();
    let ptr = u64::from_le_bytes(ptr_bytes);
    assert_ne!(ptr, 0);

    unsafe {
        let insns = std::slice::from_raw_parts(ptr as *const [u8; 8], len);
        let mut found_jgt_lo = false;
        let mut found_jgt_hi = false;
        for insn in insns {
            let code = u16::from_le_bytes([insn[0], insn[1]]);
            let jt = insn[2];
            let jf = insn[3];
            let k = u32::from_le_bytes([insn[4], insn[5], insn[6], insn[7]]);
            if code == 0x25 && jt > 0 && jf == 0 && k == 0x100 {
                found_jgt_lo = true;
            }
            if code == 0x25 && jt == 1 && jf == 0 && k == 0 {
                found_jgt_hi = true;
            }
        }
        assert!(found_jgt_lo, "LE should use JGT (0x25) with jt=skip, jf=0 for lo");
        assert!(found_jgt_hi, "LE should use JGT (0x25) with jt=1, jf=0 for hi");
    }
}

/// Verify MASKED_EQ uses AND (0x50) + JEQ (0x15) for both halves
#[test]
fn bpf_masked_eq_uses_and_then_jeq() {
    let spec = OciLinuxSeccomp {
        default_action: Some(OciSeccompAction::Allow),
        default_errno_ret: None,
        architectures: Some(vec!["SCMP_ARCH_X86_64".into()]),
        listener_path: None,
        listener_metadata: None,
        syscalls: Some(vec![
            OciSeccompSyscallEntry {
                names: Some(vec!["openat".into()]),
                action: Some(OciSeccompAction::Allow),
                errno_ret: None,
                args: Some(vec![crate::json::OciSeccompArg {
                    index: 0, value: 0xFF, value_two: 0x42, op: "SCMP_CMP_MASKED_EQ".into(),
                }]),
            },
        ]),
    };
    let (_, prog) = build_seccomp_prog(&spec);
    let len = u16::from_le_bytes([prog[0], prog[1]]) as usize;
    assert!(len >= 8);

    let ptr_bytes: [u8; 8] = prog[8..16].try_into().unwrap();
    let ptr = u64::from_le_bytes(ptr_bytes);
    assert_ne!(ptr, 0);

    unsafe {
        let insns = std::slice::from_raw_parts(ptr as *const [u8; 8], len);
        let mut found_and_lo = false;
        let mut found_jeq_lo = false;
        for insn in insns {
            let code = u16::from_le_bytes([insn[0], insn[1]]);
            let k = u32::from_le_bytes([insn[4], insn[5], insn[6], insn[7]]);
            if code == 0x50 && k == 0xFF { found_and_lo = true; }
            if code == 0x15 && k == 0x42 { found_jeq_lo = true; }
        }
        assert!(found_and_lo, "MASKED_EQ should use AND (0x50) with mask");
        assert!(found_jeq_lo, "MASKED_EQ should use JEQ (0x15) with expected value");
    }
}

#[test]
fn uses_notify_action_detects_notify_in_entry() {
    let spec = OciLinuxSeccomp {
        default_action: Some(OciSeccompAction::Errno),
        syscalls: Some(vec![OciSeccompSyscallEntry {
            names: Some(vec!["read".into()]),
            action: Some(OciSeccompAction::Notify),
            errno_ret: None,
            args: None,
        }]),
        ..Default::default()
    };
    assert!(uses_notify_action(&spec));
}

#[test]
fn uses_notify_action_detects_notify_as_default() {
    let spec = OciLinuxSeccomp {
        default_action: Some(OciSeccompAction::Notify),
        syscalls: None,
        ..Default::default()
    };
    assert!(uses_notify_action(&spec));
}

#[test]
fn uses_notify_action_false_for_allow_only() {
    let spec = OciLinuxSeccomp {
        default_action: Some(OciSeccompAction::Allow),
        syscalls: Some(vec![OciSeccompSyscallEntry {
            names: Some(vec!["read".into(), "write".into()]),
            action: Some(OciSeccompAction::Allow),
            errno_ret: None,
            args: None,
        }]),
        ..Default::default()
    };
    assert!(!uses_notify_action(&spec));
}

#[test]
fn uses_notify_action_false_for_empty_syscalls() {
    let spec = OciLinuxSeccomp {
        default_action: Some(OciSeccompAction::Errno),
        syscalls: Some(vec![]),
        ..Default::default()
    };
    assert!(!uses_notify_action(&spec));
}
