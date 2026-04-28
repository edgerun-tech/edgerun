use super::*;

#[test]
fn ebpf_insn_is_8_bytes() {
    let insn = mov_imm(R0, 0);
    assert_eq!(insn.len(), 8);
}

#[test]
fn build_device_bpf_prog_single_rule() {
    let rules = vec![OciLinuxDeviceCgroup {
        ns_type: "c".into(),
        major: Some(1),
        minor: Some(3),
        access: Some("rwm".into()),
    }];
    let insns = build_device_bpf_prog(&rules);
    assert!(!insns.is_empty());
    // Should have: mov_reg(1) + ld(1) + jmp(1) + ld(1) + jmp(1) + ld(1) + jmp(1) + allow(1) + exit(1) + deny(1) + exit(1) = 11
    assert!(insns.len() >= 10);
}

#[test]
fn build_device_bpf_prog_all_wildcards() {
    let rules = vec![OciLinuxDeviceCgroup {
        ns_type: "a".into(),
        major: None,
        minor: None,
        access: None,
    }];
    let insns = build_device_bpf_prog(&rules);
    assert!(!insns.is_empty());
    // Should have: mov_reg(1) + allow(1) + exit(1) + deny(1) + exit(1) = 5
    assert_eq!(insns.len(), 5);
}

#[test]
fn build_device_bpf_prog_multiple_rules() {
    let rules = vec![
        OciLinuxDeviceCgroup {
            ns_type: "c".into(),
            major: Some(1),
            minor: Some(3),
            access: Some("rw".into()),
        },
        OciLinuxDeviceCgroup {
            ns_type: "b".into(),
            major: Some(8),
            minor: None,
            access: Some("rwm".into()),
        },
    ];
    let insns = build_device_bpf_prog(&rules);
    assert!(!insns.is_empty());
    assert!(insns.len() > 10);
}

#[test]
fn build_device_bpf_prog_access_check() {
    let rules = vec![OciLinuxDeviceCgroup {
        ns_type: "c".into(),
        major: Some(1),
        minor: Some(5),
        access: Some("r".into()),
    }];
    let insns = build_device_bpf_prog(&rules);
    // Should include access type check for 'r'
    assert!(insns.len() >= 10);
}

#[test]
fn ld_imm_encoding() {
    let insn = ld_imm(bpf_size::BPF_W, R7, R6, OFF_MAJOR);
    assert_eq!(insn.len(), 8);
    // BPF_LDX | BPF_W | BPF_MEM = 0x01 | 0x00 | 0x60 = 0x61
    assert_eq!(insn[0], 0x61);
}
