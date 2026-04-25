use std::io;
use std::os::raw::c_void;

use crate::json::{OciLinuxSeccomp, OciSeccompAction};
use crate::syscalls::{
    do_seccomp, SECCOMP_FILTER_FLAG_NEW_LISTENER, SECCOMP_FILTER_FLAG_TSYNC,
    SECCOMP_SET_MODE_FILTER,
};

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
        if k == 0x7fff0000 {
            found_allow = true;
        }
        if k == 0x00050001 {
            found_deny = true;
        }
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
        syscalls: Some(vec![OciSeccompSyscallEntry {
            names: Some(vec!["openat".into()]),
            action: Some(OciSeccompAction::Errno),
            errno_ret: Some(13),
            args: Some(vec![crate::json::OciSeccompArg {
                index: 1,
                value: 0o100000,
                value_two: 0,
                op: "SCMP_CMP_EQ".into(),
            }]),
        }]),
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
        syscalls: Some(vec![OciSeccompSyscallEntry {
            names: Some(vec!["ioctl".into()]),
            action: Some(OciSeccompAction::Kill),
            errno_ret: None,
            args: Some(vec![crate::json::OciSeccompArg {
                index: 1,
                value: 0x5401,
                value_two: 0,
                op: "SCMP_CMP_NE".into(),
            }]),
        }]),
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
    assert_eq!(
        action_to_bpf(&OciSeccompAction::Errno, Some(13)),
        0x0005000d
    );
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
        syscalls: Some(vec![OciSeccompSyscallEntry {
            names: Some(vec!["openat".into()]),
            action: Some(OciSeccompAction::Allow),
            errno_ret: None,
            args: Some(vec![crate::json::OciSeccompArg {
                index: 0,
                value: 0x100,
                value_two: 0,
                op: "SCMP_CMP_LT".into(),
            }]),
        }]),
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
        assert!(
            found_jge_for_lt,
            "LT should use JGE (0x30) with jt=skip, jf=0 for lo"
        );
        assert!(
            found_jgt_for_hi,
            "LT should use JGT (0x25) with jt=1, jf=0 for hi"
        );
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
        syscalls: Some(vec![OciSeccompSyscallEntry {
            names: Some(vec!["openat".into()]),
            action: Some(OciSeccompAction::Allow),
            errno_ret: None,
            args: Some(vec![crate::json::OciSeccompArg {
                index: 0,
                value: 0x100,
                value_two: 0,
                op: "SCMP_CMP_GT".into(),
            }]),
        }]),
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
        assert!(
            found_jgt_lo,
            "GT should use JGT (0x25) with jt=0, jf=skip for lo"
        );
        assert!(
            found_jgt_hi,
            "GT should use JGT (0x25) with jt=0, jf=1 for hi"
        );
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
        syscalls: Some(vec![OciSeccompSyscallEntry {
            names: Some(vec!["openat".into()]),
            action: Some(OciSeccompAction::Allow),
            errno_ret: None,
            args: Some(vec![crate::json::OciSeccompArg {
                index: 0,
                value: 0x100,
                value_two: 0,
                op: "SCMP_CMP_GE".into(),
            }]),
        }]),
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
        assert!(
            found_jge_lo,
            "GE should use JGE (0x30) with jt=0, jf=skip for lo"
        );
        assert!(
            found_jge_hi,
            "GE should use JGE (0x30) with jt=0, jf=1 for hi"
        );
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
        syscalls: Some(vec![OciSeccompSyscallEntry {
            names: Some(vec!["openat".into()]),
            action: Some(OciSeccompAction::Allow),
            errno_ret: None,
            args: Some(vec![crate::json::OciSeccompArg {
                index: 0,
                value: 0x100,
                value_two: 0,
                op: "SCMP_CMP_LE".into(),
            }]),
        }]),
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
        assert!(
            found_jgt_lo,
            "LE should use JGT (0x25) with jt=skip, jf=0 for lo"
        );
        assert!(
            found_jgt_hi,
            "LE should use JGT (0x25) with jt=1, jf=0 for hi"
        );
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
        syscalls: Some(vec![OciSeccompSyscallEntry {
            names: Some(vec!["openat".into()]),
            action: Some(OciSeccompAction::Allow),
            errno_ret: None,
            args: Some(vec![crate::json::OciSeccompArg {
                index: 0,
                value: 0xFF,
                value_two: 0x42,
                op: "SCMP_CMP_MASKED_EQ".into(),
            }]),
        }]),
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
            if code == 0x50 && k == 0xFF {
                found_and_lo = true;
            }
            if code == 0x15 && k == 0x42 {
                found_jeq_lo = true;
            }
        }
        assert!(found_and_lo, "MASKED_EQ should use AND (0x50) with mask");
        assert!(
            found_jeq_lo,
            "MASKED_EQ should use JEQ (0x15) with expected value"
        );
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

// ===========================================================================
// Additional seccomp edge case tests
// ===========================================================================

#[test]
fn arch_to_bpf_values() {
    assert_eq!(arch_to_bpf("SCMP_ARCH_X86_64"), 0xc000003e);
    assert_eq!(arch_to_bpf("SCMP_ARCH_X86"), 0x40000003);
    assert_eq!(arch_to_bpf("SCMP_ARCH_X32"), 0x4000003e);
    assert_eq!(arch_to_bpf("SCMP_ARCH_AARCH64"), 0xc00000b7);
    assert_eq!(arch_to_bpf("SCMP_ARCH_ARM"), 0x40000028);
}

#[test]
fn arch_to_bpf_unknown_defaults_to_current() {
    let unknown = arch_to_bpf("SCMP_ARCH_UNKNOWN");
    assert_ne!(unknown, 0);
}

#[test]
fn action_to_bpf_all_variants() {
    assert_eq!(
        action_to_bpf(&OciSeccompAction::KillProcess, None),
        0x80000000
    );
    assert_eq!(
        action_to_bpf(&OciSeccompAction::KillThread, None),
        0x00000000
    );
    assert_eq!(action_to_bpf(&OciSeccompAction::Trap, None), 0x00030000);
    assert_eq!(action_to_bpf(&OciSeccompAction::Trace, None), 0x7ff00000);
    assert_eq!(action_to_bpf(&OciSeccompAction::Log, None), 0x7ffe0000);
    assert_eq!(action_to_bpf(&OciSeccompAction::Notify, None), 0x7fc00000);
    // Errno defaults to errno_ret=1 when None
    assert_eq!(action_to_bpf(&OciSeccompAction::Errno, None), 0x00050001);
}

#[test]
fn bpf_long_skip_exact_255() {
    let mut insns: Vec<[u8; 8]> = Vec::new();
    bpf_long_skip(&mut insns, 255);
    assert_eq!(insns.len(), 1);
    assert_eq!(insns[0][0], 0x15);
    assert_eq!(insns[0][2], 0);
    assert_eq!(insns[0][3], 255);
    assert_eq!(
        u32::from_le_bytes([insns[0][4], insns[0][5], insns[0][6], insns[0][7]]),
        0xFFFFFFFF
    );
}

#[test]
fn bpf_long_skip_256_splits() {
    let mut insns: Vec<[u8; 8]> = Vec::new();
    bpf_long_skip(&mut insns, 256);
    assert_eq!(insns.len(), 2);
    assert_eq!(insns[0][3], 255);
    assert_eq!(insns[1][3], 1);
}

#[test]
fn bpf_long_skip_511_needs_three() {
    let mut insns: Vec<[u8; 8]> = Vec::new();
    bpf_long_skip(&mut insns, 511);
    assert_eq!(insns.len(), 3);
    assert_eq!(insns[0][3], 255);
    assert_eq!(insns[1][3], 255);
    assert_eq!(insns[2][3], 1);
}

#[test]
fn build_seccomp_prog_with_log_action() {
    let spec = OciLinuxSeccomp {
        default_action: Some(OciSeccompAction::Log),
        ..Default::default()
    };
    let (_, prog) = build_seccomp_prog(&spec);
    let len = u16::from_le_bytes([prog[0], prog[1]]) as usize;
    assert!(len >= 4);
}

#[test]
fn build_seccomp_prog_with_trace_action() {
    let spec = OciLinuxSeccomp {
        default_action: Some(OciSeccompAction::Trace),
        syscalls: Some(vec![OciSeccompSyscallEntry {
            names: Some(vec!["read".into()]),
            action: Some(OciSeccompAction::Allow),
            ..Default::default()
        }]),
        ..Default::default()
    };
    let (_, prog) = build_seccomp_prog(&spec);
    let len = u16::from_le_bytes([prog[0], prog[1]]) as usize;
    assert!(len >= 5);
}

#[test]
fn build_seccomp_prog_multiple_syscall_names() {
    let spec = OciLinuxSeccomp {
        default_action: Some(OciSeccompAction::Kill),
        default_errno_ret: None,
        architectures: Some(vec!["SCMP_ARCH_X86_64".into()]),
        listener_path: None,
        listener_metadata: None,
        syscalls: Some(vec![OciSeccompSyscallEntry {
            names: Some(vec!["read".into(), "write".into(), "openat".into()]),
            action: Some(OciSeccompAction::Allow),
            ..Default::default()
        }]),
    };
    let (_, prog) = build_seccomp_prog(&spec);
    let len = u16::from_le_bytes([prog[0], prog[1]]) as usize;
    assert!(
        len >= 8,
        "multiple syscall names should generate multiple rules, got {}",
        len
    );
}

#[test]
fn seccomp_arg_fields_serialize_correctly() {
    let arg = crate::json::OciSeccompArg {
        index: 2,
        value: 0x123456789ABCDEF0,
        value_two: 0xFEDCBA9876543210,
        op: "SCMP_CMP_MASKED_EQ".into(),
    };
    let json = edgerun_json::to_string(&arg).unwrap();
    assert!(json.contains("\"index\":2"));
    let parsed: crate::json::OciSeccompArg = edgerun_json::from_slice(json.as_bytes()).unwrap();
    assert_eq!(parsed.index, 2);
    assert_eq!(parsed.value, 0x123456789ABCDEF0);
    assert_eq!(parsed.value_two, 0xFEDCBA9876543210);
    assert_eq!(parsed.op, "SCMP_CMP_MASKED_EQ");
}

#[test]
fn seccomp_bpf_prog_has_valid_sock_fprog_format() {
    // Verify that each call produces a valid sock_fprog struct:
    // [u16 len][padding][u64 pointer]
    let (_, prog) = seccomp_bpf_prog();
    assert_eq!(prog.len(), 16);
    let len = u16::from_le_bytes([prog[0], prog[1]]);
    assert!(len > 0);
    // Pointer should be non-zero
    let ptr = u64::from_le_bytes(prog[8..16].try_into().unwrap());
    assert_ne!(ptr, 0);
}

#[test]
fn syscall_nr_unknown_returns_none() {
    assert_eq!(syscall_nr("nonexistent_syscall_xyz"), None);
}

#[test]
fn syscall_nr_container_related() {
    #[cfg(target_arch = "x86_64")]
    {
        assert_eq!(syscall_nr("clone"), Some(56));
        assert_eq!(syscall_nr("unshare"), Some(272));
        assert_eq!(syscall_nr("setns"), Some(308));
        assert_eq!(syscall_nr("pivot_root"), Some(155));
        assert_eq!(syscall_nr("mount"), Some(165));
        assert_eq!(syscall_nr("umount2"), Some(166));
        assert_eq!(syscall_nr("prctl"), Some(157));
        assert_eq!(syscall_nr("seccomp"), Some(317));
        assert_eq!(syscall_nr("getrandom"), Some(318));
        assert_eq!(syscall_nr("execve"), Some(59));
        assert_eq!(syscall_nr("exit_group"), Some(231));
        assert_eq!(syscall_nr("futex"), Some(202));
        assert_eq!(syscall_nr("clock_gettime"), Some(228));
        assert_eq!(syscall_nr("openat"), Some(257));
        assert_eq!(syscall_nr("close"), Some(3));
        assert_eq!(syscall_nr("read"), Some(0));
        assert_eq!(syscall_nr("write"), Some(1));
        assert_eq!(syscall_nr("mmap"), Some(9));
        assert_eq!(syscall_nr("brk"), Some(12));
        assert_eq!(syscall_nr("mprotect"), Some(10));
        assert_eq!(syscall_nr("munmap"), Some(11));
    }
}
