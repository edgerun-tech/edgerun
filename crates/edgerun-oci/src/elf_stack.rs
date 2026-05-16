//! ELF64 initial stack and auxv construction.

use crate::prelude::*;

use crate::elf::{
    OCI_ELF_AT_BASE, OCI_ELF_AT_ENTRY, OCI_ELF_AT_FLAGS, OCI_ELF_AT_PAGESZ, OCI_ELF_AT_PHDR,
    OCI_ELF_AT_PHENT, OCI_ELF_AT_PHNUM, OciElfAuxvEntry, OciElfError, OciElfInitialStack,
    OciElfLoadPlan, OciElfMemoryMap, OciElfRuntimeMemoryMap, OciElfRuntimePlan,
};
use alloc::string::String;
use alloc::vec::Vec;

pub fn build_elf64_auxv(
    plan: &OciElfRuntimePlan,
    memory: &OciElfRuntimeMemoryMap,
) -> Result<Vec<OciElfAuxvEntry>, OciElfError> {
    let phdr = load_file_offset_addr(
        &plan.executable,
        &memory.executable,
        plan.executable.program_header_offset,
    )?;
    let base = memory
        .interpreter
        .as_ref()
        .map(|map| map.load_bias)
        .unwrap_or(0);

    Ok(vec![
        OciElfAuxvEntry {
            key: OCI_ELF_AT_PHDR,
            value: phdr,
        },
        OciElfAuxvEntry {
            key: OCI_ELF_AT_PHENT,
            value: plan.executable.program_header_entry_size as u64,
        },
        OciElfAuxvEntry {
            key: OCI_ELF_AT_PHNUM,
            value: plan.executable.program_header_count as u64,
        },
        OciElfAuxvEntry {
            key: OCI_ELF_AT_PAGESZ,
            value: memory.executable.page_size,
        },
        OciElfAuxvEntry {
            key: OCI_ELF_AT_BASE,
            value: base,
        },
        OciElfAuxvEntry {
            key: OCI_ELF_AT_FLAGS,
            value: 0,
        },
        OciElfAuxvEntry {
            key: OCI_ELF_AT_ENTRY,
            value: memory.executable.entry,
        },
    ])
}

pub fn write_elf64_initial_stack(
    stack_base: u64,
    stack_top: u64,
    stack: &mut [u8],
    argv: &[String],
    env: &[String],
    auxv: &[OciElfAuxvEntry],
) -> Result<OciElfInitialStack, OciElfError> {
    write_elf64_initial_stack_aligned(stack_base, stack_top, stack, argv, env, auxv, 16)
}

pub fn write_elf64_initial_stack_aligned(
    stack_base: u64,
    stack_top: u64,
    stack: &mut [u8],
    argv: &[String],
    env: &[String],
    auxv: &[OciElfAuxvEntry],
    align: u64,
) -> Result<OciElfInitialStack, OciElfError> {
    if align == 0 || !align.is_power_of_two() {
        return Err(OciElfError::InvalidStackAlignment(align));
    }
    let stack_len = u64::try_from(stack.len()).map_err(|_| OciElfError::Overflow)?;
    if stack_base
        .checked_add(stack_len)
        .ok_or(OciElfError::Overflow)?
        != stack_top
    {
        return Err(OciElfError::StackTooSmall);
    }

    let argv_string_bytes = c_string_bytes(argv)?;
    let env_string_bytes = c_string_bytes(env)?;
    let string_bytes = argv_string_bytes
        .checked_add(env_string_bytes)
        .ok_or(OciElfError::Overflow)?;
    let auxv_slots = auxv
        .len()
        .checked_add(1)
        .and_then(|value| value.checked_mul(2))
        .ok_or(OciElfError::Overflow)?;
    let pointer_slots = 1usize
        .checked_add(argv.len())
        .and_then(|value| value.checked_add(1))
        .and_then(|value| value.checked_add(env.len()))
        .and_then(|value| value.checked_add(1))
        .and_then(|value| value.checked_add(auxv_slots))
        .ok_or(OciElfError::Overflow)?;
    let pointer_bytes = pointer_slots.checked_mul(8).ok_or(OciElfError::Overflow)?;
    let raw_bytes = string_bytes
        .checked_add(pointer_bytes)
        .ok_or(OciElfError::Overflow)?;
    let raw_bytes_u64 = u64::try_from(raw_bytes).map_err(|_| OciElfError::Overflow)?;
    let aligned_bytes = align_up(raw_bytes_u64, align)?;
    if aligned_bytes > stack_len {
        return Err(OciElfError::StackTooSmall);
    }

    stack.fill(0);
    let stack_pointer = stack_top
        .checked_sub(aligned_bytes)
        .ok_or(OciElfError::Overflow)?;
    let mut cursor = addr_to_stack_offset(stack_base, stack_pointer)?;
    let argc_addr = stack_pointer;
    write_u64_at(stack, &mut cursor, argv.len() as u64)?;
    let argv_ptrs_addr = stack_base
        .checked_add(cursor as u64)
        .ok_or(OciElfError::Overflow)?;

    let strings_addr = stack_top
        .checked_sub(string_bytes as u64)
        .ok_or(OciElfError::Overflow)?;
    let mut string_cursor = addr_to_stack_offset(stack_base, strings_addr)?;
    for value in argv {
        let addr = stack_base
            .checked_add(string_cursor as u64)
            .ok_or(OciElfError::Overflow)?;
        write_u64_at(stack, &mut cursor, addr)?;
        write_c_string_at(stack, &mut string_cursor, value)?;
    }
    write_u64_at(stack, &mut cursor, 0)?;

    let env_ptrs_addr = stack_base
        .checked_add(cursor as u64)
        .ok_or(OciElfError::Overflow)?;
    for value in env {
        let addr = stack_base
            .checked_add(string_cursor as u64)
            .ok_or(OciElfError::Overflow)?;
        write_u64_at(stack, &mut cursor, addr)?;
        write_c_string_at(stack, &mut string_cursor, value)?;
    }
    write_u64_at(stack, &mut cursor, 0)?;

    let auxv_addr = stack_base
        .checked_add(cursor as u64)
        .ok_or(OciElfError::Overflow)?;
    for entry in auxv {
        write_u64_at(stack, &mut cursor, entry.key)?;
        write_u64_at(stack, &mut cursor, entry.value)?;
    }
    write_u64_at(stack, &mut cursor, 0)?;
    write_u64_at(stack, &mut cursor, 0)?;

    Ok(OciElfInitialStack {
        stack_pointer,
        bytes_used: aligned_bytes as usize,
        argc_addr,
        argv_ptrs_addr,
        env_ptrs_addr,
        auxv_addr,
        strings_addr,
    })
}

fn load_file_offset_addr(
    plan: &OciElfLoadPlan,
    map: &OciElfMemoryMap,
    file_offset: u64,
) -> Result<u64, OciElfError> {
    for mapping in &map.mappings {
        let segment = plan
            .segments
            .get(mapping.segment_index)
            .ok_or(OciElfError::InvalidSegmentIndex(mapping.segment_index))?;
        if file_offset < segment.file_offset {
            continue;
        }
        let segment_file_offset = file_offset
            .checked_sub(segment.file_offset)
            .ok_or(OciElfError::Overflow)?;
        if segment_file_offset < segment.file_size {
            return mapping
                .segment_start
                .checked_add(segment_file_offset)
                .ok_or(OciElfError::Overflow);
        }
    }
    Err(OciElfError::MissingProgramHeaders)
}

fn align_down(value: u64, align: u64) -> u64 {
    value & !(align - 1)
}

fn align_up(value: u64, align: u64) -> Result<u64, OciElfError> {
    if value == 0 {
        return Ok(0);
    }
    value
        .checked_add(align - 1)
        .map(|value| align_down(value, align))
        .ok_or(OciElfError::Overflow)
}

fn c_string_bytes(values: &[String]) -> Result<usize, OciElfError> {
    values.iter().try_fold(0usize, |len, value| {
        if value.as_bytes().contains(&0) {
            return Err(OciElfError::InvalidStackString);
        }
        len.checked_add(value.len())
            .and_then(|len| len.checked_add(1))
            .ok_or(OciElfError::Overflow)
    })
}

fn addr_to_stack_offset(stack_base: u64, addr: u64) -> Result<usize, OciElfError> {
    let offset = addr
        .checked_sub(stack_base)
        .ok_or(OciElfError::StackTooSmall)?;
    usize::try_from(offset).map_err(|_| OciElfError::Overflow)
}

fn write_u64_at(stack: &mut [u8], cursor: &mut usize, value: u64) -> Result<(), OciElfError> {
    let end = cursor.checked_add(8).ok_or(OciElfError::Overflow)?;
    let Some(out) = stack.get_mut(*cursor..end) else {
        return Err(OciElfError::StackTooSmall);
    };
    out.copy_from_slice(&value.to_le_bytes());
    *cursor = end;
    Ok(())
}

fn write_c_string_at(stack: &mut [u8], cursor: &mut usize, value: &str) -> Result<(), OciElfError> {
    let bytes = value.as_bytes();
    let end = cursor
        .checked_add(bytes.len())
        .and_then(|end| end.checked_add(1))
        .ok_or(OciElfError::Overflow)?;
    let Some(out) = stack.get_mut(*cursor..end) else {
        return Err(OciElfError::StackTooSmall);
    };
    out[..bytes.len()].copy_from_slice(bytes);
    out[bytes.len()] = 0;
    *cursor = end;
    Ok(())
}
