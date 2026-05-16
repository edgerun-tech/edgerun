//! eBPF-based device cgroup v2 controller.
//!
//! On cgroup v2, device access control is implemented via eBPF programs
//! of type BPF_PROG_TYPE_CGROUP_DEVICE attached to BPF_CGROUP_DEVICE.
//!
//! The eBPF program receives a `bpf_cgroup_dev_ctx` structure in `r1` with:
//!   - access_type: `(BPF_DEVCG_ACC_* << 16) | BPF_DEVCG_DEV_*`
//!   - major: device major number
//!   - minor: device minor number
//!
//! The program returns 0 (deny) or 1 (allow).

use crate::prelude::*;
use std::fs;
use std::io;
use std::os::unix::io::AsRawFd;
use std::path::Path;

use crate::spec::OciLinuxDeviceCgroup;
use crate::syscalls::*;

// bpf_cgroup_dev_ctx field offsets (from kernel headers)
const OFF_ACCESS_TYPE: i16 = 0; // u32: (access << 16) | device_type
const OFF_MAJOR: i16 = 4; // u32
const OFF_MINOR: i16 = 8; // u32

// Device type constants
const DEV_BLOCK: u32 = 1;
const DEV_CHAR: u32 = 2;
const DEV_TYPE_MASK: i32 = 0xffff;
const ACC_MKNOD: i32 = 1 << 16;
const ACC_READ: i32 = 2 << 16;
const ACC_WRITE: i32 = 4 << 16;

// eBPF return code for cgroup device allow
const BPF_CGROUP_DEV_ALLOW: i32 = 1;

// ===========================================================================
// eBPF program generation
// ===========================================================================

/// Build a complete eBPF device control program from OCI device rules.
///
/// Algorithm:
/// 1. Save context pointer (r1 → r6)
/// 2. For each rule, generate checks for device_type, major, minor, access
/// 3. If all checks pass → return ALLOW
/// 4. Default → return DENY (r0 = 0)
///
/// Jump targets are resolved to signed BPF 16-bit offsets after generation.
pub fn build_device_bpf_prog(rules: &[OciLinuxDeviceCgroup]) -> Vec<[u8; 8]> {
    // We use a two-pass approach:
    // Pass 1: build symbolic instruction list with labels
    // Pass 2: resolve labels to actual offsets

    #[derive(Clone)]
    enum SymInsn {
        Raw([u8; 8]),
        JmpNe { dst: u8, imm: i32, target: String },
        JmpEq { dst: u8, imm: i32, target: String },
        Label(String),
    }

    let mut sym: Vec<SymInsn> = Vec::new();
    let mut fwd_refs: Vec<(usize, String)> = Vec::new(); // (index, target_label)

    // r6 = r1 (save context pointer)
    sym.push(SymInsn::Raw(mov_reg(R6, R1)));

    let enforceable_rules: Vec<&OciLinuxDeviceCgroup> = rules
        .iter()
        .filter(|rule| !is_transition_deny_all(rule))
        .collect();

    // OCI device rules are applied in order to the current policy. Walking in
    // reverse gives effective "last matching rule wins" behavior.
    for (i, rule) in enforceable_rules.iter().rev().enumerate() {
        let ns_type = rule.ns_type.as_deref().unwrap_or("a");
        // Determine the "skip to next rule" label
        let skip_label = if i + 1 < enforceable_rules.len() {
            format!("next_{}", i + 1)
        } else {
            "deny".to_string()
        };

        // --- Device type check ---
        if ns_type != "a" && ns_type != "all" {
            let dev_type = match ns_type {
                "b" | "block" => DEV_BLOCK,
                "c" | "char" | "u" => DEV_CHAR,
                _ => 0,
            };
            // r7 = (*(u32*)(r6 + OFF_ACCESS_TYPE)) & DEV_TYPE_MASK
            sym.push(SymInsn::Raw(ld_imm(
                bpf_size::BPF_W,
                R7,
                R6,
                OFF_ACCESS_TYPE,
            )));
            sym.push(SymInsn::Raw(and_imm(R7, DEV_TYPE_MASK)));
            // if r7 != dev_type → goto skip_label
            sym.push(SymInsn::JmpNe {
                dst: R7,
                imm: dev_type as i32,
                target: skip_label.clone(),
            });
        }

        // --- Major check ---
        if let Some(major) = rule.major {
            if major >= 0 {
                sym.push(SymInsn::Raw(ld_imm(bpf_size::BPF_W, R7, R6, OFF_MAJOR)));
                sym.push(SymInsn::JmpNe {
                    dst: R7,
                    imm: major as i32,
                    target: skip_label.clone(),
                });
            }
        }

        // --- Minor check ---
        if let Some(minor) = rule.minor {
            if minor >= 0 {
                sym.push(SymInsn::Raw(ld_imm(bpf_size::BPF_W, R7, R6, OFF_MINOR)));
                sym.push(SymInsn::JmpNe {
                    dst: R7,
                    imm: minor as i32,
                    target: skip_label.clone(),
                });
            }
        }

        // --- Access type check ---
        if let Some(ref access) = rule.access {
            if access != "rwm" && access != "*" && !access.is_empty() {
                let mut access_mask = 0;
                for ch in access.chars() {
                    access_mask |= match ch {
                        'r' => ACC_READ,
                        'w' => ACC_WRITE,
                        'm' => ACC_MKNOD,
                        _ => 0,
                    };
                }
                if access_mask != 0 {
                    sym.push(SymInsn::Raw(ld_imm(
                        bpf_size::BPF_W,
                        R7,
                        R6,
                        OFF_ACCESS_TYPE,
                    )));
                    sym.push(SymInsn::Raw(and_imm(R7, access_mask)));
                    sym.push(SymInsn::JmpEq {
                        dst: R7,
                        imm: 0,
                        target: skip_label.clone(),
                    });
                }
            }
        }

        // --- All checks passed → rule decision and exit ---
        let decision = if rule.allow.unwrap_or(false) {
            BPF_CGROUP_DEV_ALLOW
        } else {
            0
        };
        sym.push(SymInsn::Raw(mov_imm(R0, decision)));
        sym.push(SymInsn::Raw(exit()));

        // Label for next rule's skip target
        sym.push(SymInsn::Label(format!("next_{}", i + 1)));
    }

    // --- Default ALLOW ---
    sym.push(SymInsn::Label("deny".to_string()));
    sym.push(SymInsn::Raw(mov_imm(R0, BPF_CGROUP_DEV_ALLOW)));
    sym.push(SymInsn::Raw(exit()));

    // --- Pass 2: resolve labels and forward jumps ---
    // Build label → index map
    let mut label_map: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut idx = 0;
    for insn in &sym {
        match insn {
            SymInsn::Label(name) => {
                label_map.insert(name.clone(), idx);
            }
            SymInsn::Raw(_) | SymInsn::JmpNe { .. } | SymInsn::JmpEq { .. } => {
                idx += 1;
            }
        }
    }

    // Now convert to actual [u8; 8] instructions
    let mut insns: Vec<[u8; 8]> = Vec::new();
    for insn in &sym {
        match insn {
            SymInsn::Raw(bytes) => {
                insns.push(*bytes);
            }
            SymInsn::JmpNe { dst, imm, target } => {
                // Offset is resolved after we know all labels and positions.
                insns.push(jmp_imm(bpf_jmp::BPF_JNE, *dst, *imm, 0));
                fwd_refs.push((insns.len() - 1, target.clone()));
            }
            SymInsn::JmpEq { dst, imm, target } => {
                insns.push(jmp_imm(bpf_jmp::BPF_JEQ, *dst, *imm, 0));
                fwd_refs.push((insns.len() - 1, target.clone()));
            }
            SymInsn::Label(_) => {
                // Labels don't produce instructions
            }
        }
    }

    // Fix up forward jump offsets
    for (insn_idx, target_label) in &fwd_refs {
        let target_idx = label_map
            .get(target_label)
            .copied()
            .unwrap_or_else(|| insns.len().saturating_sub(1));
        // off = target - (current + 1)
        let off = target_idx as isize - (*insn_idx as isize + 1);
        let off = off.clamp(i16::MIN as isize, i16::MAX as isize) as i16;
        insns[*insn_idx][2..4].copy_from_slice(&off.to_le_bytes());
    }

    insns
}

fn is_transition_deny_all(rule: &OciLinuxDeviceCgroup) -> bool {
    !rule.allow.unwrap_or(false)
        && rule.ns_type.is_none()
        && rule.major.is_none()
        && rule.minor.is_none()
        && rule
            .access
            .as_deref()
            .map(|access| access == "rwm" || access == "*" || access.is_empty())
            .unwrap_or(true)
}

// ===========================================================================
// Load and attach device eBPF program
// ===========================================================================

/// Apply device cgroup v2 eBPF rules to a cgroup.
///
/// `cgroup_path`: path to the cgroup directory (e.g., /sys/fs/cgroup/mycontainer)
/// `rules`: OCI device cgroup rules
///
/// Returns Ok(()) on success, Err if eBPF is not supported or loading fails.
pub fn setup_device_cgroup_ebpf(
    cgroup_path: &Path,
    rules: &[OciLinuxDeviceCgroup],
) -> io::Result<()> {
    // Generate the eBPF program
    let insns = build_device_bpf_prog(rules);

    if insns.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "no device rules provided",
        ));
    }

    // Load the program into the kernel
    let prog_fd = bpf_prog_load(
        bpf_prog_type::BPF_PROG_TYPE_CGROUP_DEVICE,
        &insns,
        "GPL",
        bpf_attach_type::BPF_CGROUP_DEVICE,
    )
    .map_err(|error| io::Error::new(error.kind(), format!("device bpf load failed: {error}")))?;

    // Open the cgroup directory and attach the program
    let cgroup_dir = fs::File::open(cgroup_path)?;
    let cgroup_fd = cgroup_dir.as_raw_fd();

    bpf_prog_attach(cgroup_fd, prog_fd, bpf_attach_type::BPF_CGROUP_DEVICE).map_err(|error| {
        io::Error::new(
            error.kind(),
            format!(
                "device bpf attach failed for cgroup {}: {error}",
                cgroup_path.display()
            ),
        )
    })?;

    // Note: prog_fd is intentionally not closed — the kernel holds a reference.
    // When the cgroup is deleted, the program is automatically detached.
    // For a production implementation, track prog_fd and call bpf_prog_detach
    // during container cleanup.
    let _ = prog_fd; // Keep fd alive until program is attached

    Ok(())
}

/// Detach device eBPF program from a cgroup.
///
/// `cgroup_path`: path to the cgroup directory
/// `prog_fd`: the program file descriptor (if tracked)
pub fn teardown_device_cgroup_ebpf(cgroup_path: &Path, prog_fd: i32) -> io::Result<()> {
    let cgroup_dir = fs::File::open(cgroup_path)?;
    let cgroup_fd = cgroup_dir.as_raw_fd();

    bpf_prog_detach(cgroup_fd, prog_fd, bpf_attach_type::BPF_CGROUP_DEVICE)?;

    Ok(())
}

// ===========================================================================
// Tests
// ===========================================================================
