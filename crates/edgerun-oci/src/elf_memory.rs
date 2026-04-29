//! ELF load memory map and runtime layout construction.

use crate::prelude::*;

use crate::elf::{
    OciElfError, OciElfImage, OciElfLoadBias, OciElfLoadPlan, OciElfMapping, OciElfMemoryMap,
    OciElfRuntimeLayout, OciElfRuntimeMapping, OciElfRuntimeMemoryMap, OciElfRuntimePlan,
    DEFAULT_PAGE_SIZE,
};

pub fn build_elf_runtime_mapping_list(
    plan: &OciElfRuntimePlan,
    memory: &OciElfRuntimeMemoryMap,
) -> Result<Vec<OciElfRuntimeMapping>, OciElfError> {
    let mut mappings = Vec::with_capacity(
        memory.executable.mappings.len()
            + memory
                .interpreter
                .as_ref()
                .map(|map| map.mappings.len())
                .unwrap_or(0),
    );
    append_runtime_mappings(
        &mut mappings,
        OciElfImage::Executable,
        plan.executable.path.as_str(),
        &memory.executable,
    );
    if let (Some(interpreter), Some(map)) = (plan.interpreter.as_ref(), memory.interpreter.as_ref())
    {
        append_runtime_mappings(
            &mut mappings,
            OciElfImage::Interpreter,
            interpreter.path.as_str(),
            map,
        );
    } else if plan.interpreter.is_some() || memory.interpreter.is_some() {
        return Err(OciElfError::MissingInterpreter);
    }
    mappings.sort_by_key(|mapping| mapping.mapping.map_start);
    validate_runtime_mapping_list(&mappings)?;
    Ok(mappings)
}

pub fn build_elf_runtime_layout(
    plan: &OciElfRuntimePlan,
    memory: &OciElfRuntimeMemoryMap,
) -> Result<OciElfRuntimeLayout, OciElfError> {
    let mappings = build_elf_runtime_mapping_list(plan, memory)?;
    let Some(first) = mappings.first() else {
        return Ok(OciElfRuntimeLayout {
            mappings,
            map_start: 0,
            map_end: 0,
            mapped_bytes: 0,
        });
    };
    let mut map_start = first.mapping.map_start;
    let mut map_end = first
        .mapping
        .map_start
        .checked_add(first.mapping.map_size)
        .ok_or(OciElfError::Overflow)?;
    let mut mapped_bytes = first.mapping.map_size;
    for mapping in mappings.iter().skip(1) {
        map_start = core::cmp::min(map_start, mapping.mapping.map_start);
        let end = mapping
            .mapping
            .map_start
            .checked_add(mapping.mapping.map_size)
            .ok_or(OciElfError::Overflow)?;
        map_end = core::cmp::max(map_end, end);
        mapped_bytes = mapped_bytes
            .checked_add(mapping.mapping.map_size)
            .ok_or(OciElfError::Overflow)?;
    }
    Ok(OciElfRuntimeLayout {
        mappings,
        map_start,
        map_end,
        mapped_bytes,
    })
}

pub fn build_elf_memory_map(plan: &OciElfLoadPlan) -> Result<OciElfMemoryMap, OciElfError> {
    build_elf_memory_map_with_page_size(plan, DEFAULT_PAGE_SIZE)
}

pub fn build_elf_memory_map_with_load_bias(
    plan: &OciElfLoadPlan,
    load_bias: u64,
) -> Result<OciElfMemoryMap, OciElfError> {
    build_elf_memory_map_with_page_size_and_load_bias(plan, DEFAULT_PAGE_SIZE, load_bias)
}

pub fn build_elf_runtime_memory_map(
    plan: &OciElfRuntimePlan,
) -> Result<OciElfRuntimeMemoryMap, OciElfError> {
    build_elf_runtime_memory_map_with_page_size(plan, DEFAULT_PAGE_SIZE)
}

pub fn build_elf_runtime_memory_map_with_load_bias(
    plan: &OciElfRuntimePlan,
    load_bias: OciElfLoadBias,
) -> Result<OciElfRuntimeMemoryMap, OciElfError> {
    build_elf_runtime_memory_map_with_page_size_and_load_bias(plan, DEFAULT_PAGE_SIZE, load_bias)
}

pub fn build_elf_runtime_memory_map_with_page_size(
    plan: &OciElfRuntimePlan,
    page_size: u64,
) -> Result<OciElfRuntimeMemoryMap, OciElfError> {
    build_elf_runtime_memory_map_with_page_size_and_load_bias(
        plan,
        page_size,
        OciElfLoadBias::default(),
    )
}

pub fn build_elf_runtime_memory_map_with_page_size_and_load_bias(
    plan: &OciElfRuntimePlan,
    page_size: u64,
    load_bias: OciElfLoadBias,
) -> Result<OciElfRuntimeMemoryMap, OciElfError> {
    Ok(OciElfRuntimeMemoryMap {
        executable: build_elf_memory_map_with_page_size_and_load_bias(
            &plan.executable,
            page_size,
            load_bias.executable,
        )?,
        interpreter: plan
            .interpreter
            .as_ref()
            .map(|interpreter| {
                build_elf_memory_map_with_page_size_and_load_bias(
                    interpreter,
                    page_size,
                    load_bias.interpreter,
                )
            })
            .transpose()?,
    })
}

pub fn build_elf_memory_map_with_page_size(
    plan: &OciElfLoadPlan,
    page_size: u64,
) -> Result<OciElfMemoryMap, OciElfError> {
    build_elf_memory_map_with_page_size_and_load_bias(plan, page_size, 0)
}

pub fn build_elf_memory_map_with_page_size_and_load_bias(
    plan: &OciElfLoadPlan,
    page_size: u64,
    load_bias: u64,
) -> Result<OciElfMemoryMap, OciElfError> {
    if page_size == 0 || !page_size.is_power_of_two() {
        return Err(OciElfError::InvalidPageSize(page_size));
    }

    let mut mappings = Vec::with_capacity(plan.segments.len());
    for (index, segment) in plan.segments.iter().enumerate() {
        let segment_start = segment
            .virtual_addr
            .checked_add(load_bias)
            .ok_or(OciElfError::Overflow)?;
        let map_start = align_down(segment_start, page_size);
        let segment_end = segment
            .virtual_addr
            .checked_add(load_bias)
            .ok_or(OciElfError::Overflow)?
            .checked_add(segment.memory_size)
            .ok_or(OciElfError::Overflow)?;
        let map_end = align_up(segment_end, page_size)?;
        mappings.push(OciElfMapping {
            segment_index: index,
            map_start,
            map_size: map_end
                .checked_sub(map_start)
                .ok_or(OciElfError::Overflow)?,
            segment_start,
            segment_size: segment.memory_size,
            file_size: segment.file_size,
            flags: segment.flags,
        });
    }

    mappings.sort_by_key(|mapping| mapping.map_start);
    validate_mapping_list(&mappings)?;

    Ok(OciElfMemoryMap {
        page_size,
        load_bias,
        entry: plan
            .entry
            .checked_add(load_bias)
            .ok_or(OciElfError::Overflow)?,
        mappings,
    })
}

fn append_runtime_mappings(
    out: &mut Vec<OciElfRuntimeMapping>,
    image: OciElfImage,
    path: &str,
    map: &OciElfMemoryMap,
) {
    out.extend(
        map.mappings
            .iter()
            .cloned()
            .enumerate()
            .map(|(mapping_index, mapping)| OciElfRuntimeMapping {
                image,
                path: path.into(),
                mapping_index,
                mapping,
            }),
    );
}

fn validate_runtime_mapping_list(mappings: &[OciElfRuntimeMapping]) -> Result<(), OciElfError> {
    for pair in mappings.windows(2) {
        let left_end = pair[0]
            .mapping
            .map_start
            .checked_add(pair[0].mapping.map_size)
            .ok_or(OciElfError::Overflow)?;
        if left_end > pair[1].mapping.map_start {
            return Err(OciElfError::OverlappingLoadSegments);
        }
    }
    Ok(())
}

fn validate_mapping_list(mappings: &[OciElfMapping]) -> Result<(), OciElfError> {
    for pair in mappings.windows(2) {
        let left_end = pair[0]
            .map_start
            .checked_add(pair[0].map_size)
            .ok_or(OciElfError::Overflow)?;
        if left_end > pair[1].map_start {
            return Err(OciElfError::OverlappingLoadSegments);
        }
    }
    Ok(())
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
