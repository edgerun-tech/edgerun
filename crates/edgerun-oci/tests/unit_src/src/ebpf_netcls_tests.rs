use super::*;

#[test]
fn build_netcls_bpf_prog_non_empty() {
    let insns = build_netcls_bpf_prog(0x10001);
    assert!(!insns.is_empty());
    // mov_reg(1) + mov_imm(1) + st_imm(1) + mov_imm(1) + exit(1) = 5
    assert_eq!(insns.len(), 5);
}

#[test]
fn build_netprio_bpf_prog_single() {
    let priorities = vec![OciLinuxNetworkPriority {
        name: "1".into(),
        priority: 6,
    }];
    let insns = build_netprio_bpf_prog(&priorities);
    assert!(!insns.is_empty());
    assert!(insns.len() >= 5);
}

#[test]
fn build_netprio_bpf_prog_multiple() {
    let priorities = vec![
        OciLinuxNetworkPriority {
            name: "1".into(),
            priority: 6,
        },
        OciLinuxNetworkPriority {
            name: "2".into(),
            priority: 7,
        },
    ];
    let insns = build_netprio_bpf_prog(&priorities);
    assert!(!insns.is_empty());
    assert!(insns.len() >= 8);
}

#[test]
fn st_imm_encoding() {
    let insn = st_imm(bpf_size::BPF_W, R6, R2, SKB_PRIORITY_OFF);
    assert_eq!(insn.len(), 8);
    // BPF_STX | BPF_W | BPF_MEM = 0x03 | 0x00 | 0x60 = 0x63
    assert_eq!(insn[0], 0x63);
}
