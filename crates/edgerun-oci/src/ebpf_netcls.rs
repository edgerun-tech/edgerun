//! eBPF-based network class ID and priority cgroup v2 controllers.
//!
//! On cgroup v2, network class ID and priority are managed via eBPF programs
//! attached to cgroup skb hooks (BPF_CGROUP_INET_EGRESS / BPF_CGROUP_INET_INGRESS).
//!
//! These programs can set the skb->priority or tc_classid fields.

use crate::prelude::*;
use std::fs;
use std::io;
use std::os::unix::io::AsRawFd;
use std::path::Path;

use crate::spec::OciLinuxNetworkPriority;
use crate::syscalls::*;

// For cgroup skb programs, context is struct __sk_buff:
//   priority is at offset 0x18 (24 bytes) from the __sk_buff base
const SKB_PRIORITY_OFF: i16 = 0x18;

// BPF return codes for cgroup skb
const BPF_OK: i32 = 0;
const BPF_ALLOW: i32 = 1; // For cgroup skb, returning 1 means "pass"

// ===========================================================================
// Network class ID eBPF program
// ===========================================================================

/// Build an eBPF program that sets skb->priority to the given class_id.
///
/// This program is attached to BPF_CGROUP_INET_EGRESS.
/// It sets the packet priority (which maps to net_cls.classid on cgroup v1).
pub fn build_netcls_bpf_prog(class_id: u32) -> Vec<[u8; 8]> {
    // Save context (r1 → r6), set skb->priority, return
    let insns: Vec<[u8; 8]> = vec![
        mov_reg(R6, R1),
        mov_imm(R2, class_id as i32),
        st_imm(bpf_size::BPF_W, R6, R2, SKB_PRIORITY_OFF),
        mov_imm(R0, BPF_ALLOW),
        exit(),
    ];
    insns
}

/// Apply network class ID eBPF to a cgroup.
pub fn setup_netcls_cgroup_ebpf(cgroup_path: &Path, class_id: u32) -> io::Result<()> {
    let insns = build_netcls_bpf_prog(class_id);

    let prog_fd = bpf_prog_load(
        bpf_prog_type::BPF_PROG_TYPE_CGROUP_SKB,
        &insns,
        "GPL",
        bpf_attach_type::BPF_CGROUP_INET_EGRESS,
    )?;

    let cgroup_dir = fs::File::open(cgroup_path)?;
    let cgroup_fd = cgroup_dir.as_raw_fd();

    bpf_prog_attach(cgroup_fd, prog_fd, bpf_attach_type::BPF_CGROUP_INET_EGRESS)?;

    let _ = prog_fd;

    Ok(())
}

// ===========================================================================
// Network priority eBPF program
// ===========================================================================

/// Build an eBPF program that sets skb->priority based on interface priorities.
///
/// For each interface name → priority mapping, the program checks the
/// outgoing interface index and sets the priority accordingly.
///
/// This implementation resolves interface names as numeric ifindex strings at
/// setup time and emits one inline compare path per rule.
pub fn build_netprio_bpf_prog(priorities: &[OciLinuxNetworkPriority]) -> Vec<[u8; 8]> {
    #[derive(Clone)]
    enum SymInsn {
        Raw([u8; 8]),
        JmpNe { dst: u8, imm: i32, target: String },
        Label(String),
    }

    let mut sym: Vec<SymInsn> = Vec::new();
    let mut fwd_refs: Vec<(usize, String)> = Vec::new();

    let mut usable_rules: Vec<(u32, u32)> = Vec::new();

    // Save context (r1 → r6)
    sym.push(SymInsn::Raw(mov_reg(R6, R1)));

    // For cgroup skb programs, we can access:
    //   __sk_buff->ifindex at offset 0x08
    const SKB_IFINDEX_OFF: i16 = 0x08;

    // For each priority rule, generate a check:
    //   if (skb->ifindex == expected_ifindex) { skb->priority = prio; return BPF_OK; }
    //
    // Note: The OCI spec gives interface names, not ifindices.
    // In practice, the caller must resolve names to ifindices before calling this.
    // We use the `priority` field as the priority value and encode the ifindex
    // in the `name` field as a numeric string.

    for p in priorities {
        // Parse ifindex from the name field (expected to be a number)
        let ifindex: u32 = p.name.parse().unwrap_or(0);
        if ifindex != 0 {
            usable_rules.push((ifindex, p.priority));
        }
    }

    // No usable rules means we cannot build a meaningful program.
    if usable_rules.is_empty() {
        return Vec::new();
    }

    for (rule_idx, (ifindex, priority)) in usable_rules.iter().enumerate() {
        let skip_label = if rule_idx + 1 == usable_rules.len() {
            "default".to_string()
        } else {
            format!("rule_{}", rule_idx + 1)
        };

        // Load skb->ifindex
        sym.push(SymInsn::Raw(ld_imm(bpf_size::BPF_W, R2, R6, SKB_IFINDEX_OFF)));

        // Compare with expected ifindex
        // if r2 != ifindex → skip to next rule/default
        sym.push(SymInsn::JmpNe {
            dst: R2,
            imm: ifindex as i32,
            target: skip_label,
        });

        // Set skb->priority = p.priority
        sym.push(SymInsn::Raw(mov_imm(R2, *priority as i32)));
        sym.push(SymInsn::Raw(st_imm(bpf_size::BPF_W, R6, R2, SKB_PRIORITY_OFF)));
        sym.push(SymInsn::Raw(mov_imm(R0, BPF_ALLOW)));
        sym.push(SymInsn::Raw(exit()));

        // Label for next rule skip target
        sym.push(SymInsn::Label(format!("rule_{}", rule_idx + 1)));
    }

    // Default: return BPF_OK (pass through without modification)
    sym.push(SymInsn::Label("default".to_string()));
    sym.push(SymInsn::Raw(mov_imm(R0, BPF_OK)));
    sym.push(SymInsn::Raw(exit()));

    // Build label -> index map.
    let mut label_map: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut idx = 0;
    for insn in &sym {
        match insn {
            SymInsn::Label(name) => {
                label_map.insert(name.clone(), idx);
            }
            SymInsn::Raw(_) | SymInsn::JmpNe { .. } => idx += 1,
        }
    }

    let mut insns: Vec<[u8; 8]> = Vec::new();
    for insn in &sym {
        match insn {
            SymInsn::Raw(bytes) => insns.push(*bytes),
            SymInsn::JmpNe { dst, imm, target } => {
                insns.push(jmp_imm(bpf_jmp::BPF_JNE, *dst, *imm, 0));
                fwd_refs.push((insns.len() - 1, target.clone()));
            }
            SymInsn::Label(_) => {}
        }
    }

    for (insn_idx, target_label) in &fwd_refs {
        let target_idx = label_map
            .get(target_label)
            .copied()
            .unwrap_or_else(|| insns.len().saturating_sub(1));
        let off = target_idx as isize - (*insn_idx as isize + 1);
        let off = off.clamp(i16::MIN as isize, i16::MAX as isize) as i16;
        insns[*insn_idx][2..4].copy_from_slice(&off.to_le_bytes());
    }

    insns
}

/// Apply network priority eBPF to a cgroup.
pub fn setup_netprio_cgroup_ebpf(
    cgroup_path: &Path,
    priorities: &[OciLinuxNetworkPriority],
) -> io::Result<()> {
    let insns = build_netprio_bpf_prog(priorities);

    if insns.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "no network priority rules provided",
        ));
    }

    let prog_fd = bpf_prog_load(
        bpf_prog_type::BPF_PROG_TYPE_CGROUP_SKB,
        &insns,
        "GPL",
        bpf_attach_type::BPF_CGROUP_INET_EGRESS,
    )?;

    let cgroup_dir = fs::File::open(cgroup_path)?;
    let cgroup_fd = cgroup_dir.as_raw_fd();

    bpf_prog_attach(cgroup_fd, prog_fd, bpf_attach_type::BPF_CGROUP_INET_EGRESS)?;

    let _ = prog_fd;

    Ok(())
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(all(test, not(target_os = "none")))]
#[path = "../tests/unit_src/src/ebpf_netcls_tests.rs"]
mod tests;
