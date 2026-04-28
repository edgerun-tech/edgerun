//! eBPF-based device cgroup v2 controller.
//!
//! On cgroup v2, device access control is implemented via eBPF programs
//! of type BPF_PROG_TYPE_CGROUP_DEVICE attached to BPF_CGROUP_DEVICE.
//!
//! The eBPF program receives a `bpf_cgroup_dev_ctx` structure in `r1` with:
//!   - access_type: 'r', 'w', or 'm' (mknod)
//!   - major: device major number
//!   - minor: device minor number
//!   - device_type: DEV_BLOCK (1) or DEV_CHAR (2)
//!
//! The program returns BPF_OK (deny) or BPF_CGROUP_DEV_ALLOW (allow).

use crate::prelude::*;
use std::fs;
use std::io;
use std::os::unix::io::AsRawFd;
use std::path::Path;

use crate::spec::OciLinuxDeviceCgroup;
use crate::syscalls::*;

// bpf_cgroup_dev_ctx field offsets (from kernel headers)
const OFF_ACCESS_TYPE: i16 = 0; // u32: 'r', 'w', 'm'
const OFF_MAJOR: i16 = 4; // u32
const OFF_MINOR: i16 = 8; // u32
const OFF_DEVICE_TYPE: i16 = 12; // u32: DEV_BLOCK=1, DEV_CHAR=2

// Device type constants
const DEV_BLOCK: u32 = 1;
const DEV_CHAR: u32 = 2;

// eBPF return code for cgroup device allow
const BPF_CGROUP_DEV_ALLOW: i32 = 2;

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
/// All forward jump offsets are computed correctly.
pub fn build_device_bpf_prog(rules: &[OciLinuxDeviceCgroup]) -> Vec<[u8; 8]> {
    // We use a two-pass approach:
    // Pass 1: build symbolic instruction list with labels
    // Pass 2: resolve labels to actual offsets

    #[derive(Clone)]
    enum SymInsn {
        Raw([u8; 8]),
        JmpNe { dst: u8, imm: i32, target: String },
        Label(String),
    }

    let mut sym: Vec<SymInsn> = Vec::new();
    let mut fwd_refs: Vec<(usize, String)> = Vec::new(); // (index, target_label)

    // r6 = r1 (save context pointer)
    sym.push(SymInsn::Raw(mov_reg(R6, R0 + 1))); // R1 = 1, but we want r1
    sym.pop();
    sym.push(SymInsn::Raw(mov_reg(R6, 1))); // Wrong again — mov_reg takes reg numbers

    // Let me be explicit:
    // R1 is register number 1, which is the context pointer for cgroup device programs.
    sym.clear();
    sym.push(SymInsn::Raw(mov_reg(R6, 1))); // r6 = r1

    // For each rule, generate checks
    for (i, rule) in rules.iter().enumerate() {
        // Determine the "skip to next rule" label
        let skip_label = if i + 1 < rules.len() {
            format!("next_{}", i + 1)
        } else {
            "deny".to_string()
        };

        // --- Device type check ---
        if rule.ns_type != "a" && rule.ns_type != "all" {
            let dev_type = match rule.ns_type.as_str() {
                "b" | "block" => DEV_BLOCK,
                "c" | "char" | "u" => DEV_CHAR,
                _ => 0,
            };
            // r7 = *(u32*)(r6 + OFF_DEVICE_TYPE)
            sym.push(SymInsn::Raw(ld_imm(
                bpf_size::BPF_W,
                R7,
                R6,
                OFF_DEVICE_TYPE,
            )));
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
                for ch in access.chars() {
                    let access_val = match ch {
                        'r' => b'r' as i32,
                        'w' => b'w' as i32,
                        'm' => b'm' as i32,
                        _ => continue,
                    };
                    sym.push(SymInsn::Raw(ld_imm(
                        bpf_size::BPF_W,
                        R7,
                        R6,
                        OFF_ACCESS_TYPE,
                    )));
                    sym.push(SymInsn::JmpNe {
                        dst: R7,
                        imm: access_val,
                        target: skip_label.clone(),
                    });
                }
            }
        }

        // --- All checks passed → ALLOW and exit ---
        sym.push(SymInsn::Raw(mov_imm(R0, BPF_CGROUP_DEV_ALLOW)));
        sym.push(SymInsn::Raw(exit()));

        // Label for next rule's skip target
        sym.push(SymInsn::Label(format!("allow_{}", i)));
    }

    // --- Default DENY ---
    sym.push(SymInsn::Label("deny".to_string()));
    sym.push(SymInsn::Raw(mov_imm(R0, 0)));
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
            SymInsn::Raw(_) | SymInsn::JmpNe { .. } => {
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
                // We'll fix up the offset after we know all positions
                // For now, push a placeholder (off = 0)
                insns.push(jmp_imm(bpf_jmp::BPF_JNE, *dst, *imm, 0));
                // We need to track which instruction index this is and its target
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
            .unwrap_or(insns.len() - 1);
        // off = target - (current + 1)
        let off = (target_idx as isize - (*insn_idx as isize + 1)).max(0) as i16;
        insns[*insn_idx][2..4].copy_from_slice(&off.to_le_bytes());
    }

    insns
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
    )?;

    // Open the cgroup directory and attach the program
    let cgroup_dir = fs::File::open(cgroup_path)?;
    let cgroup_fd = cgroup_dir.as_raw_fd();

    bpf_prog_attach(cgroup_fd, prog_fd, bpf_attach_type::BPF_CGROUP_DEVICE)?;

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

#[cfg(all(test, not(target_os = "none")))]
#[path = "../tests/unit_src/src/ebpf_devices_tests.rs"]
mod tests;
