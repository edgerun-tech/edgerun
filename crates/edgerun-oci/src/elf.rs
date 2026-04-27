//! no_std ELF executable probe for OCI rootfs launch plans.

use crate::prelude::*;
use crate::rootfs_access::{
    build_launch_plan, normalize_rootfs_path, OciLaunchPlan, OciRootfs, OciRootfsError,
};
use core::fmt;

const ELF_HEADER_LEN: usize = 64;
const ELF64_PHDR_LEN: usize = 56;
const EI_CLASS: usize = 4;
const EI_DATA: usize = 5;
const EI_VERSION: usize = 6;
const ELFCLASS64: u8 = 2;
const ELFDATA2LSB: u8 = 1;
const EV_CURRENT: u8 = 1;
const PT_LOAD: u32 = 1;
const PT_INTERP: u32 = 3;
const PF_X: u32 = 1;
const PF_W: u32 = 2;
const PF_R: u32 = 4;
const DEFAULT_PAGE_SIZE: u64 = 4096;

pub const OCI_ELF_AT_NULL: u64 = 0;
pub const OCI_ELF_AT_PHDR: u64 = 3;
pub const OCI_ELF_AT_PHENT: u64 = 4;
pub const OCI_ELF_AT_PHNUM: u64 = 5;
pub const OCI_ELF_AT_PAGESZ: u64 = 6;
pub const OCI_ELF_AT_BASE: u64 = 7;
pub const OCI_ELF_AT_FLAGS: u64 = 8;
pub const OCI_ELF_AT_ENTRY: u64 = 9;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OciElfError {
    Rootfs(OciRootfsError),
    NotFound(String),
    ShortRead {
        path: String,
        expected: usize,
        actual: usize,
    },
    BadMagic,
    UnsupportedClass(u8),
    UnsupportedEndian(u8),
    UnsupportedVersion(u8),
    UnsupportedProgramHeaderSize(u16),
    InvalidLoadSegment(String),
    InvalidInterpreter(String),
    InvalidPageSize(u64),
    OverlappingLoadSegments,
    InvalidSegmentIndex(usize),
    InvalidMappingIndex(usize),
    MissingInterpreter,
    MissingProgramHeaders,
    EntryNotExecutable(u64),
    InvalidStackAlignment(u64),
    InvalidStackString,
    StackTooSmall,
    ScratchBufferTooSmall,
    Overflow,
}

impl fmt::Display for OciElfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rootfs(error) => write!(f, "rootfs read failed: {error}"),
            Self::NotFound(path) => write!(f, "ELF executable not found: {path}"),
            Self::ShortRead {
                path,
                expected,
                actual,
            } => write!(
                f,
                "short ELF read from {path}: expected {expected} bytes, got {actual}"
            ),
            Self::BadMagic => f.write_str("invalid ELF magic"),
            Self::UnsupportedClass(class) => write!(f, "unsupported ELF class: {class}"),
            Self::UnsupportedEndian(endian) => write!(f, "unsupported ELF endian: {endian}"),
            Self::UnsupportedVersion(version) => write!(f, "unsupported ELF version: {version}"),
            Self::UnsupportedProgramHeaderSize(size) => {
                write!(f, "unsupported ELF program header size: {size}")
            }
            Self::InvalidLoadSegment(error) => write!(f, "invalid ELF load segment: {error}"),
            Self::InvalidInterpreter(error) => write!(f, "invalid ELF interpreter: {error}"),
            Self::InvalidPageSize(size) => write!(f, "invalid ELF mapping page size: {size}"),
            Self::OverlappingLoadSegments => f.write_str("overlapping ELF load segments"),
            Self::InvalidSegmentIndex(index) => write!(f, "invalid ELF segment index: {index}"),
            Self::InvalidMappingIndex(index) => write!(f, "invalid ELF mapping index: {index}"),
            Self::MissingInterpreter => f.write_str("ELF runtime plan has no interpreter"),
            Self::MissingProgramHeaders => f.write_str("ELF program headers are not loadable"),
            Self::EntryNotExecutable(entry) => {
                write!(
                    f,
                    "ELF entry point is not in an executable segment: {entry:#x}"
                )
            }
            Self::InvalidStackAlignment(align) => {
                write!(f, "invalid ELF initial stack alignment: {align}")
            }
            Self::InvalidStackString => f.write_str("ELF initial stack strings cannot contain NUL"),
            Self::StackTooSmall => f.write_str("ELF initial stack buffer is too small"),
            Self::ScratchBufferTooSmall => f.write_str("ELF load scratch buffer is too small"),
            Self::Overflow => f.write_str("ELF metadata overflow"),
        }
    }
}

impl core::error::Error for OciElfError {}

impl From<OciRootfsError> for OciElfError {
    fn from(value: OciRootfsError) -> Self {
        Self::Rootfs(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OciElfType {
    Relocatable,
    Executable,
    Shared,
    Core,
    Other(u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OciElfMachine {
    X86_64,
    AArch64,
    RiscV,
    Other(u16),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OciElfInfo {
    pub path: String,
    pub elf_type: OciElfType,
    pub machine: OciElfMachine,
    pub entry: u64,
    pub program_header_offset: u64,
    pub program_header_entry_size: u16,
    pub program_header_count: u16,
    pub interpreter: Option<String>,
    pub program_headers: Vec<OciElfProgramHeader>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OciElfProgramHeader {
    pub kind: u32,
    pub flags: u32,
    pub offset: u64,
    pub virtual_addr: u64,
    pub physical_addr: u64,
    pub file_size: u64,
    pub memory_size: u64,
    pub align: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OciElfPermissions {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
}

impl OciElfPermissions {
    pub const fn from_flags(flags: u32) -> Self {
        Self {
            read: flags & PF_R != 0,
            write: flags & PF_W != 0,
            execute: flags & PF_X != 0,
        }
    }
}

impl OciElfProgramHeader {
    pub fn is_load(&self) -> bool {
        self.kind == PT_LOAD
    }

    pub fn is_interpreter(&self) -> bool {
        self.kind == PT_INTERP
    }

    pub fn is_executable(&self) -> bool {
        self.flags & PF_X != 0
    }

    pub fn permissions(&self) -> OciElfPermissions {
        OciElfPermissions::from_flags(self.flags)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OciElfLoadPlan {
    pub path: String,
    pub elf_type: OciElfType,
    pub entry: u64,
    pub machine: OciElfMachine,
    pub program_header_offset: u64,
    pub program_header_entry_size: u16,
    pub program_header_count: u16,
    pub interpreter: Option<String>,
    pub segments: Vec<OciElfLoadSegment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OciElfRuntimePlan {
    pub executable: OciElfLoadPlan,
    pub interpreter: Option<OciElfLoadPlan>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OciElfImage {
    Executable,
    Interpreter,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OciElfLoadBias {
    pub executable: u64,
    pub interpreter: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OciElfRuntimeMemoryMap {
    pub executable: OciElfMemoryMap,
    pub interpreter: Option<OciElfMemoryMap>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OciElfRuntimeMapping {
    pub image: OciElfImage,
    pub path: String,
    pub mapping_index: usize,
    pub mapping: OciElfMapping,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OciElfRuntimeLayout {
    pub mappings: Vec<OciElfRuntimeMapping>,
    pub map_start: u64,
    pub map_end: u64,
    pub mapped_bytes: u64,
}

pub trait OciElfMapper {
    fn map_elf_region(&mut self, mapping: &OciElfRuntimeMapping) -> Result<(), OciElfError>;

    fn write_elf_region(
        &mut self,
        mapping: &OciElfRuntimeMapping,
        mapping_offset: u64,
        bytes: &[u8],
    ) -> Result<(), OciElfError>;

    fn protect_elf_region(&mut self, _mapping: &OciElfRuntimeMapping) -> Result<(), OciElfError> {
        Ok(())
    }

    fn map_stack_region(&mut self, stack_base: u64, stack_size: u64) -> Result<(), OciElfError>;

    fn write_stack_region(&mut self, stack_base: u64, bytes: &[u8]) -> Result<(), OciElfError>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct OciElfUnsafeIdentityMapper;

impl OciElfUnsafeIdentityMapper {
    pub const fn new() -> Self {
        Self
    }
}

impl OciElfMapper for OciElfUnsafeIdentityMapper {
    fn map_elf_region(&mut self, _mapping: &OciElfRuntimeMapping) -> Result<(), OciElfError> {
        Ok(())
    }

    fn write_elf_region(
        &mut self,
        mapping: &OciElfRuntimeMapping,
        mapping_offset: u64,
        bytes: &[u8],
    ) -> Result<(), OciElfError> {
        let addr = mapping
            .mapping
            .map_start
            .checked_add(mapping_offset)
            .ok_or(OciElfError::Overflow)?;
        unsafe { copy_to_addr(addr, bytes) }
    }

    fn protect_elf_region(&mut self, _mapping: &OciElfRuntimeMapping) -> Result<(), OciElfError> {
        Ok(())
    }

    fn map_stack_region(&mut self, _stack_base: u64, _stack_size: u64) -> Result<(), OciElfError> {
        Ok(())
    }

    fn write_stack_region(&mut self, stack_base: u64, bytes: &[u8]) -> Result<(), OciElfError> {
        unsafe { copy_to_addr(stack_base, bytes) }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OciPreparedProgram {
    pub launch: OciLaunchPlan,
    pub runtime: OciElfRuntimePlan,
    pub memory: OciElfRuntimeMemoryMap,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OciPreparedLaunchState {
    pub entry_point: u64,
    pub executable_entry: u64,
    pub interpreter_entry: Option<u64>,
    pub stack_pointer: u64,
    pub stack: OciElfInitialStack,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OciElfLoadSegment {
    pub file_offset: u64,
    pub virtual_addr: u64,
    pub file_size: u64,
    pub memory_size: u64,
    pub flags: u32,
    pub align: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OciElfSegmentRead {
    pub bytes_read: usize,
    pub zero_fill_bytes: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OciElfAuxvEntry {
    pub key: u64,
    pub value: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OciElfInitialStack {
    pub stack_pointer: u64,
    pub bytes_used: usize,
    pub argc_addr: u64,
    pub argv_ptrs_addr: u64,
    pub env_ptrs_addr: u64,
    pub auxv_addr: u64,
    pub strings_addr: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OciElfMemoryMap {
    pub page_size: u64,
    pub load_bias: u64,
    pub entry: u64,
    pub mappings: Vec<OciElfMapping>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OciElfMapping {
    pub segment_index: usize,
    pub map_start: u64,
    pub map_size: u64,
    pub segment_start: u64,
    pub segment_size: u64,
    pub file_size: u64,
    pub flags: u32,
}

impl OciElfLoadSegment {
    pub fn is_executable(&self) -> bool {
        self.flags & PF_X != 0
    }

    pub fn permissions(&self) -> OciElfPermissions {
        OciElfPermissions::from_flags(self.flags)
    }
}

impl OciElfMapping {
    pub fn is_executable(&self) -> bool {
        self.flags & PF_X != 0
    }

    pub fn permissions(&self) -> OciElfPermissions {
        OciElfPermissions::from_flags(self.flags)
    }
}

pub fn inspect_launch_elf<R: OciRootfs>(
    rootfs: &R,
    plan: &OciLaunchPlan,
) -> Result<OciElfInfo, OciElfError> {
    inspect_elf(rootfs, plan.executable.path.as_str())
}

pub fn inspect_elf<R: OciRootfs>(rootfs: &R, path: &str) -> Result<OciElfInfo, OciElfError> {
    let mut header = [0u8; ELF_HEADER_LEN];
    read_exact_rootfs(rootfs, path, 0, &mut header)?;

    if header[0..4] != [0x7f, b'E', b'L', b'F'] {
        return Err(OciElfError::BadMagic);
    }
    if header[EI_CLASS] != ELFCLASS64 {
        return Err(OciElfError::UnsupportedClass(header[EI_CLASS]));
    }
    if header[EI_DATA] != ELFDATA2LSB {
        return Err(OciElfError::UnsupportedEndian(header[EI_DATA]));
    }
    if header[EI_VERSION] != EV_CURRENT {
        return Err(OciElfError::UnsupportedVersion(header[EI_VERSION]));
    }

    let elf_type = elf_type(read_u16(&header, 16));
    let machine = elf_machine(read_u16(&header, 18));
    let entry = read_u64(&header, 24);
    let phoff = read_u64(&header, 32);
    let phentsize = read_u16(&header, 54);
    let phnum = read_u16(&header, 56);

    if phentsize != ELF64_PHDR_LEN as u16 {
        return Err(OciElfError::UnsupportedProgramHeaderSize(phentsize));
    }

    let mut program_headers = Vec::with_capacity(phnum as usize);
    for index in 0..phnum {
        let offset = phoff
            .checked_add((index as u64).saturating_mul(phentsize as u64))
            .ok_or(OciElfError::Overflow)?;
        let mut phdr = [0u8; ELF64_PHDR_LEN];
        read_exact_rootfs(rootfs, path, offset, &mut phdr)?;
        program_headers.push(parse_program_header(&phdr));
    }

    let interpreter = read_interpreter(rootfs, path, &program_headers)?;

    Ok(OciElfInfo {
        path: path.into(),
        elf_type,
        machine,
        entry,
        program_header_offset: phoff,
        program_header_entry_size: phentsize,
        program_header_count: phnum,
        interpreter,
        program_headers,
    })
}

pub fn build_elf_load_plan<R: OciRootfs>(
    rootfs: &R,
    path: &str,
) -> Result<OciElfLoadPlan, OciElfError> {
    let info = inspect_elf(rootfs, path)?;
    elf_load_plan(rootfs, info)
}

pub fn build_launch_elf_load_plan<R: OciRootfs>(
    rootfs: &R,
    plan: &OciLaunchPlan,
) -> Result<OciElfLoadPlan, OciElfError> {
    build_elf_load_plan(rootfs, plan.executable.path.as_str())
}

pub fn build_elf_runtime_plan<R: OciRootfs>(
    rootfs: &R,
    path: &str,
) -> Result<OciElfRuntimePlan, OciElfError> {
    let executable = build_elf_load_plan(rootfs, path)?;
    let interpreter = if let Some(interpreter) = executable.interpreter.as_deref() {
        let interpreter_path = normalize_rootfs_path(interpreter, false)?;
        Some(build_elf_load_plan(rootfs, interpreter_path.as_str())?)
    } else {
        None
    };
    Ok(OciElfRuntimePlan {
        executable,
        interpreter,
    })
}

pub fn build_launch_elf_runtime_plan<R: OciRootfs>(
    rootfs: &R,
    plan: &OciLaunchPlan,
) -> Result<OciElfRuntimePlan, OciElfError> {
    build_elf_runtime_plan(rootfs, plan.executable.path.as_str())
}

pub fn prepare_oci_elf_program<R: OciRootfs>(
    rootfs: &R,
    args: &[String],
    env: &[String],
    cwd: &str,
) -> Result<Option<OciPreparedProgram>, OciElfError> {
    prepare_oci_elf_program_with_page_size(rootfs, args, env, cwd, DEFAULT_PAGE_SIZE)
}

pub fn prepare_oci_elf_program_with_page_size<R: OciRootfs>(
    rootfs: &R,
    args: &[String],
    env: &[String],
    cwd: &str,
    page_size: u64,
) -> Result<Option<OciPreparedProgram>, OciElfError> {
    prepare_oci_elf_program_with_page_size_and_load_bias(
        rootfs,
        args,
        env,
        cwd,
        page_size,
        OciElfLoadBias::default(),
    )
}

pub fn prepare_oci_elf_program_with_load_bias<R: OciRootfs>(
    rootfs: &R,
    args: &[String],
    env: &[String],
    cwd: &str,
    load_bias: OciElfLoadBias,
) -> Result<Option<OciPreparedProgram>, OciElfError> {
    prepare_oci_elf_program_with_page_size_and_load_bias(
        rootfs,
        args,
        env,
        cwd,
        DEFAULT_PAGE_SIZE,
        load_bias,
    )
}

pub fn prepare_oci_elf_program_with_page_size_and_load_bias<R: OciRootfs>(
    rootfs: &R,
    args: &[String],
    env: &[String],
    cwd: &str,
    page_size: u64,
    load_bias: OciElfLoadBias,
) -> Result<Option<OciPreparedProgram>, OciElfError> {
    let Some(launch) = build_launch_plan(rootfs, args, env, cwd)? else {
        return Ok(None);
    };
    let runtime = build_launch_elf_runtime_plan(rootfs, &launch)?;
    let memory =
        build_elf_runtime_memory_map_with_page_size_and_load_bias(&runtime, page_size, load_bias)?;
    Ok(Some(OciPreparedProgram {
        launch,
        runtime,
        memory,
    }))
}

pub fn read_elf_segment_chunk<R: OciRootfs>(
    rootfs: &R,
    plan: &OciElfLoadPlan,
    segment_index: usize,
    segment_offset: u64,
    out: &mut [u8],
) -> Result<OciElfSegmentRead, OciElfError> {
    let segment = plan
        .segments
        .get(segment_index)
        .ok_or(OciElfError::InvalidSegmentIndex(segment_index))?;
    if segment_offset >= segment.memory_size || out.is_empty() {
        return Ok(OciElfSegmentRead {
            bytes_read: 0,
            zero_fill_bytes: 0,
        });
    }

    let requested = core::cmp::min(out.len() as u64, segment.memory_size - segment_offset);
    let file_available = segment.file_size.saturating_sub(segment_offset);
    let file_count = core::cmp::min(requested, file_available) as usize;
    let zero_fill_bytes = requested as usize - file_count;

    if file_count > 0 {
        let file_offset = segment
            .file_offset
            .checked_add(segment_offset)
            .ok_or(OciElfError::Overflow)?;
        read_exact_rootfs(
            rootfs,
            plan.path.as_str(),
            file_offset,
            &mut out[..file_count],
        )?;
    }
    if zero_fill_bytes > 0 {
        out[file_count..requested as usize].fill(0);
    }

    Ok(OciElfSegmentRead {
        bytes_read: file_count,
        zero_fill_bytes,
    })
}

pub fn read_elf_runtime_segment_chunk<R: OciRootfs>(
    rootfs: &R,
    plan: &OciElfRuntimePlan,
    image: OciElfImage,
    segment_index: usize,
    segment_offset: u64,
    out: &mut [u8],
) -> Result<OciElfSegmentRead, OciElfError> {
    let plan = match image {
        OciElfImage::Executable => &plan.executable,
        OciElfImage::Interpreter => plan
            .interpreter
            .as_ref()
            .ok_or(OciElfError::MissingInterpreter)?,
    };
    read_elf_segment_chunk(rootfs, plan, segment_index, segment_offset, out)
}

pub fn read_elf_mapping_chunk<R: OciRootfs>(
    rootfs: &R,
    plan: &OciElfLoadPlan,
    map: &OciElfMemoryMap,
    mapping_index: usize,
    mapping_offset: u64,
    out: &mut [u8],
) -> Result<OciElfSegmentRead, OciElfError> {
    let mapping = map
        .mappings
        .get(mapping_index)
        .ok_or(OciElfError::InvalidMappingIndex(mapping_index))?;
    let segment = plan
        .segments
        .get(mapping.segment_index)
        .ok_or(OciElfError::InvalidSegmentIndex(mapping.segment_index))?;
    if mapping_offset >= mapping.map_size || out.is_empty() {
        return Ok(OciElfSegmentRead {
            bytes_read: 0,
            zero_fill_bytes: 0,
        });
    }

    let requested = core::cmp::min(out.len() as u64, mapping.map_size - mapping_offset);
    out[..requested as usize].fill(0);

    let request_start = mapping
        .map_start
        .checked_add(mapping_offset)
        .ok_or(OciElfError::Overflow)?;
    let request_end = request_start
        .checked_add(requested)
        .ok_or(OciElfError::Overflow)?;
    let file_end = mapping
        .segment_start
        .checked_add(segment.file_size)
        .ok_or(OciElfError::Overflow)?;
    let file_start = core::cmp::max(request_start, mapping.segment_start);
    let file_end = core::cmp::min(request_end, file_end);

    let bytes_read = if file_start < file_end {
        let segment_offset = file_start
            .checked_sub(mapping.segment_start)
            .ok_or(OciElfError::Overflow)?;
        let file_offset = segment
            .file_offset
            .checked_add(segment_offset)
            .ok_or(OciElfError::Overflow)?;
        let out_offset = file_start
            .checked_sub(request_start)
            .ok_or(OciElfError::Overflow)? as usize;
        let file_count = (file_end - file_start) as usize;
        read_exact_rootfs(
            rootfs,
            plan.path.as_str(),
            file_offset,
            &mut out[out_offset..out_offset + file_count],
        )?;
        file_count
    } else {
        0
    };

    Ok(OciElfSegmentRead {
        bytes_read,
        zero_fill_bytes: requested as usize - bytes_read,
    })
}

pub fn read_elf_runtime_mapping_chunk<R: OciRootfs>(
    rootfs: &R,
    plan: &OciElfRuntimePlan,
    memory: &OciElfRuntimeMemoryMap,
    image: OciElfImage,
    mapping_index: usize,
    mapping_offset: u64,
    out: &mut [u8],
) -> Result<OciElfSegmentRead, OciElfError> {
    let (plan, map) = match image {
        OciElfImage::Executable => (&plan.executable, &memory.executable),
        OciElfImage::Interpreter => (
            plan.interpreter
                .as_ref()
                .ok_or(OciElfError::MissingInterpreter)?,
            memory
                .interpreter
                .as_ref()
                .ok_or(OciElfError::MissingInterpreter)?,
        ),
    };
    read_elf_mapping_chunk(rootfs, plan, map, mapping_index, mapping_offset, out)
}

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

pub fn read_elf_runtime_mapping_list_chunk<R: OciRootfs>(
    rootfs: &R,
    plan: &OciElfRuntimePlan,
    memory: &OciElfRuntimeMemoryMap,
    mapping: &OciElfRuntimeMapping,
    mapping_offset: u64,
    out: &mut [u8],
) -> Result<OciElfSegmentRead, OciElfError> {
    read_elf_runtime_mapping_chunk(
        rootfs,
        plan,
        memory,
        mapping.image,
        mapping.mapping_index,
        mapping_offset,
        out,
    )
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
    for pair in mappings.windows(2) {
        let left_end = pair[0]
            .map_start
            .checked_add(pair[0].map_size)
            .ok_or(OciElfError::Overflow)?;
        if left_end > pair[1].map_start {
            return Err(OciElfError::OverlappingLoadSegments);
        }
    }

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

pub fn write_prepared_elf64_initial_stack(
    program: &OciPreparedProgram,
    stack_base: u64,
    stack_top: u64,
    stack: &mut [u8],
) -> Result<OciElfInitialStack, OciElfError> {
    write_prepared_elf64_initial_stack_aligned(program, stack_base, stack_top, stack, 16)
}

pub fn write_prepared_elf64_initial_stack_aligned(
    program: &OciPreparedProgram,
    stack_base: u64,
    stack_top: u64,
    stack: &mut [u8],
    align: u64,
) -> Result<OciElfInitialStack, OciElfError> {
    let auxv = build_elf64_auxv(&program.runtime, &program.memory)?;
    write_elf64_initial_stack_aligned(
        stack_base,
        stack_top,
        stack,
        &program.launch.argv,
        &program.launch.env,
        &auxv,
        align,
    )
}

pub fn write_prepared_elf64_launch_state(
    program: &OciPreparedProgram,
    stack_base: u64,
    stack_top: u64,
    stack: &mut [u8],
) -> Result<OciPreparedLaunchState, OciElfError> {
    write_prepared_elf64_launch_state_aligned(program, stack_base, stack_top, stack, 16)
}

pub fn write_prepared_elf64_launch_state_aligned(
    program: &OciPreparedProgram,
    stack_base: u64,
    stack_top: u64,
    stack: &mut [u8],
    align: u64,
) -> Result<OciPreparedLaunchState, OciElfError> {
    let stack =
        write_prepared_elf64_initial_stack_aligned(program, stack_base, stack_top, stack, align)?;
    let executable_entry = program.memory.executable.entry;
    let interpreter_entry = program.memory.interpreter.as_ref().map(|map| map.entry);
    let entry_point = interpreter_entry.unwrap_or(executable_entry);
    Ok(OciPreparedLaunchState {
        entry_point,
        executable_entry,
        interpreter_entry,
        stack_pointer: stack.stack_pointer,
        stack,
    })
}

pub fn load_prepared_elf64_program<R: OciRootfs, M: OciElfMapper>(
    rootfs: &R,
    program: &OciPreparedProgram,
    mapper: &mut M,
    scratch: &mut [u8],
    stack_base: u64,
    stack_top: u64,
    stack: &mut [u8],
) -> Result<OciPreparedLaunchState, OciElfError> {
    load_prepared_elf64_program_aligned(
        rootfs, program, mapper, scratch, stack_base, stack_top, stack, 16,
    )
}

pub fn prepare_and_load_oci_elf_program<R: OciRootfs, M: OciElfMapper>(
    rootfs: &R,
    args: &[String],
    env: &[String],
    cwd: &str,
    mapper: &mut M,
    scratch: &mut [u8],
    stack_base: u64,
    stack_top: u64,
    stack: &mut [u8],
) -> Result<Option<OciPreparedLaunchState>, OciElfError> {
    prepare_and_load_oci_elf_program_with_page_size_and_load_bias(
        rootfs,
        args,
        env,
        cwd,
        mapper,
        scratch,
        stack_base,
        stack_top,
        stack,
        DEFAULT_PAGE_SIZE,
        OciElfLoadBias::default(),
    )
}

pub fn prepare_and_load_oci_elf_program_with_load_bias<R: OciRootfs, M: OciElfMapper>(
    rootfs: &R,
    args: &[String],
    env: &[String],
    cwd: &str,
    mapper: &mut M,
    scratch: &mut [u8],
    stack_base: u64,
    stack_top: u64,
    stack: &mut [u8],
    load_bias: OciElfLoadBias,
) -> Result<Option<OciPreparedLaunchState>, OciElfError> {
    prepare_and_load_oci_elf_program_with_page_size_and_load_bias(
        rootfs,
        args,
        env,
        cwd,
        mapper,
        scratch,
        stack_base,
        stack_top,
        stack,
        DEFAULT_PAGE_SIZE,
        load_bias,
    )
}

pub fn prepare_and_load_oci_elf_program_with_page_size_and_load_bias<
    R: OciRootfs,
    M: OciElfMapper,
>(
    rootfs: &R,
    args: &[String],
    env: &[String],
    cwd: &str,
    mapper: &mut M,
    scratch: &mut [u8],
    stack_base: u64,
    stack_top: u64,
    stack: &mut [u8],
    page_size: u64,
    load_bias: OciElfLoadBias,
) -> Result<Option<OciPreparedLaunchState>, OciElfError> {
    let Some(program) = prepare_oci_elf_program_with_page_size_and_load_bias(
        rootfs, args, env, cwd, page_size, load_bias,
    )?
    else {
        return Ok(None);
    };
    load_prepared_elf64_program(
        rootfs, &program, mapper, scratch, stack_base, stack_top, stack,
    )
    .map(Some)
}

pub fn load_prepared_elf64_program_aligned<R: OciRootfs, M: OciElfMapper>(
    rootfs: &R,
    program: &OciPreparedProgram,
    mapper: &mut M,
    scratch: &mut [u8],
    stack_base: u64,
    stack_top: u64,
    stack: &mut [u8],
    stack_align: u64,
) -> Result<OciPreparedLaunchState, OciElfError> {
    if scratch.is_empty() {
        return Err(OciElfError::ScratchBufferTooSmall);
    }

    let layout = build_elf_runtime_layout(&program.runtime, &program.memory)?;
    for mapping in &layout.mappings {
        mapper.map_elf_region(mapping)?;
        let mut offset = 0u64;
        while offset < mapping.mapping.map_size {
            let requested =
                core::cmp::min(scratch.len() as u64, mapping.mapping.map_size - offset) as usize;
            read_elf_runtime_mapping_list_chunk(
                rootfs,
                &program.runtime,
                &program.memory,
                mapping,
                offset,
                &mut scratch[..requested],
            )?;
            mapper.write_elf_region(mapping, offset, &scratch[..requested])?;
            offset = offset
                .checked_add(requested as u64)
                .ok_or(OciElfError::Overflow)?;
        }
        mapper.protect_elf_region(mapping)?;
    }

    let stack_size = stack_top
        .checked_sub(stack_base)
        .ok_or(OciElfError::StackTooSmall)?;
    let launch = write_prepared_elf64_launch_state_aligned(
        program,
        stack_base,
        stack_top,
        stack,
        stack_align,
    )?;
    mapper.map_stack_region(stack_base, stack_size)?;
    mapper.write_stack_region(stack_base, stack)?;
    Ok(launch)
}

/// Enter a loaded x86_64 ELF program.
///
/// # Safety
///
/// The caller must ensure `entry_point` points to executable mapped code,
/// `stack_pointer` points to a valid initial stack, and the runtime has
/// installed the syscall/exception handling needed by the loaded program.
#[cfg(target_arch = "x86_64")]
pub unsafe fn enter_elf64(entry_point: u64, stack_pointer: u64) -> ! {
    unsafe {
        core::arch::asm!(
            "mov rsp, {stack}",
            "xor rbp, rbp",
            "jmp {entry}",
            stack = in(reg) stack_pointer,
            entry = in(reg) entry_point,
            options(noreturn)
        );
    }
}

/// Enter a loaded x86_64 ELF program from a prepared launch state.
///
/// # Safety
///
/// Same requirements as [`enter_elf64`].
#[cfg(target_arch = "x86_64")]
pub unsafe fn enter_elf64_launch_state(launch: &OciPreparedLaunchState) -> ! {
    unsafe { enter_elf64(launch.entry_point, launch.stack_pointer) }
}

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

fn elf_load_plan<R: OciRootfs>(
    rootfs: &R,
    info: OciElfInfo,
) -> Result<OciElfLoadPlan, OciElfError> {
    let file_len = rootfs
        .file_len(info.path.as_str())?
        .ok_or_else(|| OciElfError::NotFound(info.path.clone()))? as u64;
    let mut segments = Vec::new();

    for header in info
        .program_headers
        .iter()
        .filter(|header| header.is_load())
    {
        validate_load_segment(header, file_len)?;
        segments.push(OciElfLoadSegment {
            file_offset: header.offset,
            virtual_addr: header.virtual_addr,
            file_size: header.file_size,
            memory_size: header.memory_size,
            flags: header.flags,
            align: header.align,
        });
    }

    if !segments.iter().any(|segment| {
        segment.is_executable()
            && contains_addr(segment.virtual_addr, segment.memory_size, info.entry)
    }) {
        return Err(OciElfError::EntryNotExecutable(info.entry));
    }

    Ok(OciElfLoadPlan {
        path: info.path,
        elf_type: info.elf_type,
        entry: info.entry,
        machine: info.machine,
        program_header_offset: info.program_header_offset,
        program_header_entry_size: info.program_header_entry_size,
        program_header_count: info.program_header_count,
        interpreter: info.interpreter,
        segments,
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

unsafe fn copy_to_addr(addr: u64, bytes: &[u8]) -> Result<(), OciElfError> {
    let ptr = usize::try_from(addr).map_err(|_| OciElfError::Overflow)? as *mut u8;
    unsafe {
        core::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
    }
    Ok(())
}

fn read_interpreter<R: OciRootfs>(
    rootfs: &R,
    path: &str,
    headers: &[OciElfProgramHeader],
) -> Result<Option<String>, OciElfError> {
    let Some(header) = headers.iter().find(|header| header.is_interpreter()) else {
        return Ok(None);
    };
    if header.file_size == 0 {
        return Err(OciElfError::InvalidInterpreter("empty PT_INTERP".into()));
    }
    let len = usize::try_from(header.file_size).map_err(|_| OciElfError::Overflow)?;
    let mut bytes = vec![0u8; len];
    read_exact_rootfs(rootfs, path, header.offset, &mut bytes)?;
    let Some(nul) = bytes.iter().position(|byte| *byte == 0) else {
        return Err(OciElfError::InvalidInterpreter(
            "PT_INTERP is not NUL-terminated".into(),
        ));
    };
    if nul == 0 {
        return Err(OciElfError::InvalidInterpreter(
            "PT_INTERP path is empty".into(),
        ));
    }
    let path = core::str::from_utf8(&bytes[..nul])
        .map_err(|_| OciElfError::InvalidInterpreter("PT_INTERP is not UTF-8".into()))?;
    Ok(Some(path.into()))
}

fn validate_load_segment(header: &OciElfProgramHeader, file_len: u64) -> Result<(), OciElfError> {
    if header.memory_size < header.file_size {
        return Err(OciElfError::InvalidLoadSegment(
            "memory size is smaller than file size".into(),
        ));
    }
    header
        .offset
        .checked_add(header.file_size)
        .ok_or(OciElfError::Overflow)
        .and_then(|end| {
            if end > file_len {
                Err(OciElfError::InvalidLoadSegment(
                    "file range extends past executable".into(),
                ))
            } else {
                Ok(())
            }
        })?;
    header
        .virtual_addr
        .checked_add(header.memory_size)
        .ok_or(OciElfError::Overflow)?;
    Ok(())
}

fn contains_addr(start: u64, len: u64, addr: u64) -> bool {
    start
        .checked_add(len)
        .map(|end| addr >= start && addr < end)
        .unwrap_or(false)
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

fn read_exact_rootfs<R: OciRootfs>(
    rootfs: &R,
    path: &str,
    offset: u64,
    out: &mut [u8],
) -> Result<(), OciElfError> {
    let Some(actual) = rootfs.read_file_range(path, offset, out)? else {
        return Err(OciElfError::NotFound(path.into()));
    };
    if actual != out.len() {
        return Err(OciElfError::ShortRead {
            path: path.into(),
            expected: out.len(),
            actual,
        });
    }
    Ok(())
}

fn parse_program_header(input: &[u8; ELF64_PHDR_LEN]) -> OciElfProgramHeader {
    OciElfProgramHeader {
        kind: read_u32(input, 0),
        flags: read_u32(input, 4),
        offset: read_u64(input, 8),
        virtual_addr: read_u64(input, 16),
        physical_addr: read_u64(input, 24),
        file_size: read_u64(input, 32),
        memory_size: read_u64(input, 40),
        align: read_u64(input, 48),
    }
}

fn elf_type(value: u16) -> OciElfType {
    match value {
        1 => OciElfType::Relocatable,
        2 => OciElfType::Executable,
        3 => OciElfType::Shared,
        4 => OciElfType::Core,
        value => OciElfType::Other(value),
    }
}

fn elf_machine(value: u16) -> OciElfMachine {
    match value {
        62 => OciElfMachine::X86_64,
        183 => OciElfMachine::AArch64,
        243 => OciElfMachine::RiscV,
        value => OciElfMachine::Other(value),
    }
}

fn read_u16(input: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([input[offset], input[offset + 1]])
}

fn read_u32(input: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        input[offset],
        input[offset + 1],
        input[offset + 2],
        input[offset + 3],
    ])
}

fn read_u64(input: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes([
        input[offset],
        input[offset + 1],
        input[offset + 2],
        input[offset + 3],
        input[offset + 4],
        input[offset + 5],
        input[offset + 6],
        input[offset + 7],
    ])
}

#[cfg(all(test, not(target_os = "none")))]
mod tests {
    use super::*;
    use crate::bare_rootfs::BareRootfs;
    use crate::rootfs_access::build_launch_plan;
    use crate::tar_layer::{TarEntry, TarEntryKind, TarLayerSink};

    #[test]
    fn inspects_launch_plan_elf_program_headers() {
        let mut rootfs = BareRootfs::new();
        let entry = TarEntry {
            path: "bin/app".into(),
            kind: TarEntryKind::Regular,
            size: minimal_elf64().len() as u64,
            mode: 0o755,
            uid: 0,
            gid: 0,
            mtime: 0,
            dev_major: None,
            dev_minor: None,
            link_name: None,
            whiteout: None,
        };
        let elf = minimal_elf64();
        rootfs.apply_entry(&entry, &elf).unwrap();

        let plan = build_launch_plan(&rootfs, &["/bin/app".into()], &[], "/")
            .unwrap()
            .unwrap();
        let info = inspect_launch_elf(&rootfs, &plan).unwrap();

        assert_eq!(info.path, "bin/app");
        assert_eq!(info.elf_type, OciElfType::Executable);
        assert_eq!(info.machine, OciElfMachine::X86_64);
        assert_eq!(info.entry, 0x401000);
        assert_eq!(info.program_header_offset, ELF_HEADER_LEN as u64);
        assert_eq!(info.program_header_entry_size, ELF64_PHDR_LEN as u16);
        assert_eq!(info.program_header_count, 1);
        assert_eq!(info.interpreter, None);
        assert_eq!(info.program_headers.len(), 1);
        assert!(info.program_headers[0].is_load());
        assert_eq!(
            info.program_headers[0].permissions(),
            OciElfPermissions {
                read: true,
                write: false,
                execute: true
            }
        );
        assert_eq!(info.program_headers[0].virtual_addr, 0x400000);
        assert_eq!(info.program_headers[0].memory_size, 0x2000);

        let load = build_launch_elf_load_plan(&rootfs, &plan).unwrap();
        assert_eq!(load.path, "bin/app");
        assert_eq!(load.elf_type, OciElfType::Executable);
        assert_eq!(load.entry, 0x401000);
        assert_eq!(load.machine, OciElfMachine::X86_64);
        assert_eq!(load.program_header_offset, ELF_HEADER_LEN as u64);
        assert_eq!(load.program_header_entry_size, ELF64_PHDR_LEN as u16);
        assert_eq!(load.program_header_count, 1);
        assert_eq!(load.interpreter, None);
        assert_eq!(load.segments.len(), 1);
        assert_eq!(load.segments[0].file_offset, 0);
        assert_eq!(load.segments[0].virtual_addr, 0x400000);
        assert_eq!(load.segments[0].file_size, 0x1000);
        assert_eq!(load.segments[0].memory_size, 0x2000);
        assert!(load.segments[0].is_executable());
        assert_eq!(
            load.segments[0].permissions(),
            OciElfPermissions {
                read: true,
                write: false,
                execute: true
            }
        );

        let mut buf = [0u8; 16];
        let read = read_elf_segment_chunk(&rootfs, &load, 0, 0, &mut buf).unwrap();
        assert_eq!(
            read,
            OciElfSegmentRead {
                bytes_read: 16,
                zero_fill_bytes: 0
            }
        );
        assert_eq!(&buf[..4], &[0x7f, b'E', b'L', b'F']);

        buf.fill(0xaa);
        let read = read_elf_segment_chunk(&rootfs, &load, 0, 0x0ff8, &mut buf).unwrap();
        assert_eq!(
            read,
            OciElfSegmentRead {
                bytes_read: 8,
                zero_fill_bytes: 8
            }
        );
        assert!(buf[8..].iter().all(|byte| *byte == 0));

        buf.fill(0xaa);
        let read = read_elf_segment_chunk(&rootfs, &load, 0, 0x1000, &mut buf).unwrap();
        assert_eq!(
            read,
            OciElfSegmentRead {
                bytes_read: 0,
                zero_fill_bytes: 16
            }
        );
        assert!(buf.iter().all(|byte| *byte == 0));
        assert!(matches!(
            read_elf_segment_chunk(&rootfs, &load, 1, 0, &mut buf),
            Err(OciElfError::InvalidSegmentIndex(1))
        ));

        let map = build_elf_memory_map(&load).unwrap();
        assert_eq!(map.page_size, 4096);
        assert_eq!(map.load_bias, 0);
        assert_eq!(map.entry, 0x401000);
        assert_eq!(map.mappings.len(), 1);
        assert_eq!(map.mappings[0].segment_index, 0);
        assert_eq!(map.mappings[0].map_start, 0x400000);
        assert_eq!(map.mappings[0].map_size, 0x2000);
        assert_eq!(map.mappings[0].segment_start, 0x400000);
        assert_eq!(map.mappings[0].segment_size, 0x2000);
        assert_eq!(
            map.mappings[0].permissions(),
            OciElfPermissions {
                read: true,
                write: false,
                execute: true
            }
        );
        assert!(matches!(
            build_elf_memory_map_with_page_size(&load, 3000),
            Err(OciElfError::InvalidPageSize(3000))
        ));
        let biased_map = build_elf_memory_map_with_load_bias(&load, 0x10000000).unwrap();
        assert_eq!(biased_map.load_bias, 0x10000000);
        assert_eq!(biased_map.entry, 0x10401000);
        assert_eq!(biased_map.mappings[0].map_start, 0x10400000);
        assert_eq!(biased_map.mappings[0].segment_start, 0x10400000);

        buf.fill(0xaa);
        let read = read_elf_mapping_chunk(&rootfs, &load, &map, 0, 0x0ff8, &mut buf).unwrap();
        assert_eq!(
            read,
            OciElfSegmentRead {
                bytes_read: 8,
                zero_fill_bytes: 8
            }
        );
        assert!(buf[8..].iter().all(|byte| *byte == 0));

        let prepared = prepare_oci_elf_program(
            &rootfs,
            &["app".into(), "--flag".into()],
            &["PATH=/bin".into()],
            "/",
        )
        .unwrap()
        .unwrap();
        assert_eq!(prepared.launch.argv, vec!["app", "--flag"]);
        assert_eq!(prepared.runtime.executable.path, "bin/app");
        assert!(prepared.runtime.interpreter.is_none());
        assert_eq!(prepared.memory.executable.entry, 0x401000);
        assert!(prepared.memory.interpreter.is_none());
        let auxv = build_elf64_auxv(&prepared.runtime, &prepared.memory).unwrap();
        assert_eq!(auxv_value(&auxv, OCI_ELF_AT_PHDR), Some(0x400040));
        assert_eq!(
            auxv_value(&auxv, OCI_ELF_AT_PHENT),
            Some(ELF64_PHDR_LEN as u64)
        );
        assert_eq!(auxv_value(&auxv, OCI_ELF_AT_PHNUM), Some(1));
        assert_eq!(auxv_value(&auxv, OCI_ELF_AT_PAGESZ), Some(4096));
        assert_eq!(auxv_value(&auxv, OCI_ELF_AT_BASE), Some(0));
        assert_eq!(auxv_value(&auxv, OCI_ELF_AT_FLAGS), Some(0));
        assert_eq!(auxv_value(&auxv, OCI_ELF_AT_ENTRY), Some(0x401000));
        let mappings = build_elf_runtime_mapping_list(&prepared.runtime, &prepared.memory).unwrap();
        assert_eq!(mappings.len(), 1);
        assert_eq!(mappings[0].image, OciElfImage::Executable);
        assert_eq!(mappings[0].path, "bin/app");
        assert_eq!(mappings[0].mapping_index, 0);
        assert_eq!(mappings[0].mapping.map_start, 0x400000);
        let layout = build_elf_runtime_layout(&prepared.runtime, &prepared.memory).unwrap();
        assert_eq!(layout.map_start, 0x400000);
        assert_eq!(layout.map_end, 0x402000);
        assert_eq!(layout.mapped_bytes, 0x2000);
        assert_eq!(layout.mappings.len(), 1);

        let mut mapper = RecordingMapper::new();
        let mut scratch = [0u8; 512];
        let mut stack = [0u8; 512];
        let stack_base = 0x7000_0000u64;
        let stack_top = stack_base + stack.len() as u64;
        let launch = load_prepared_elf64_program(
            &rootfs,
            &prepared,
            &mut mapper,
            &mut scratch,
            stack_base,
            stack_top,
            &mut stack,
        )
        .unwrap();
        assert_eq!(launch.entry_point, 0x401000);
        assert_eq!(mapper.mapped.len(), 1);
        assert_eq!(mapper.mapped[0].mapping.map_start, 0x400000);
        assert_eq!(mapper.protected.len(), 1);
        assert_eq!(mapper.writes.len(), 16);
        assert_eq!(mapper.writes[0].0, 0x400000);
        assert_eq!(&mapper.writes[0].1[..4], &[0x7f, b'E', b'L', b'F']);
        assert_eq!(mapper.writes[15].0, 0x401e00);
        assert!(mapper.writes[15].1.iter().all(|byte| *byte == 0));
        assert_eq!(mapper.stack_base, Some(stack_base));
        assert_eq!(mapper.stack_bytes.as_deref(), Some(stack.as_slice()));

        let mut buf = [0u8; 16];
        let read = read_elf_runtime_segment_chunk(
            &rootfs,
            &prepared.runtime,
            OciElfImage::Executable,
            0,
            0,
            &mut buf,
        )
        .unwrap();
        assert_eq!(
            read,
            OciElfSegmentRead {
                bytes_read: 16,
                zero_fill_bytes: 0
            }
        );
        assert_eq!(&buf[..4], &[0x7f, b'E', b'L', b'F']);
        let read = read_elf_runtime_mapping_chunk(
            &rootfs,
            &prepared.runtime,
            &prepared.memory,
            OciElfImage::Executable,
            0,
            0,
            &mut buf,
        )
        .unwrap();
        assert_eq!(
            read,
            OciElfSegmentRead {
                bytes_read: 16,
                zero_fill_bytes: 0
            }
        );
        assert_eq!(&buf[..4], &[0x7f, b'E', b'L', b'F']);
        assert!(matches!(
            read_elf_runtime_segment_chunk(
                &rootfs,
                &prepared.runtime,
                OciElfImage::Interpreter,
                0,
                0,
                &mut buf
            ),
            Err(OciElfError::MissingInterpreter)
        ));

        let missing =
            prepare_oci_elf_program(&rootfs, &["missing".into()], &["PATH=/bin".into()], "/")
                .unwrap();
        assert!(missing.is_none());
    }

    #[test]
    fn reads_dynamic_elf_interpreter() {
        let mut rootfs = BareRootfs::new();
        let elf = elf64_with_interpreter("/lib64/ld-linux-x86-64.so.2");
        let entry = TarEntry {
            path: "bin/app".into(),
            kind: TarEntryKind::Regular,
            size: elf.len() as u64,
            mode: 0o755,
            uid: 0,
            gid: 0,
            mtime: 0,
            dev_major: None,
            dev_minor: None,
            link_name: None,
            whiteout: None,
        };
        rootfs.apply_entry(&entry, &elf).unwrap();

        let info = inspect_elf(&rootfs, "bin/app").unwrap();
        assert_eq!(info.interpreter, Some("/lib64/ld-linux-x86-64.so.2".into()));

        let load = build_elf_load_plan(&rootfs, "bin/app").unwrap();
        assert_eq!(load.interpreter, Some("/lib64/ld-linux-x86-64.so.2".into()));

        let interpreter = minimal_elf64();
        let interpreter_entry = TarEntry {
            path: "lib64/ld-linux-x86-64.so.2".into(),
            kind: TarEntryKind::Regular,
            size: interpreter.len() as u64,
            mode: 0o755,
            uid: 0,
            gid: 0,
            mtime: 0,
            dev_major: None,
            dev_minor: None,
            link_name: None,
            whiteout: None,
        };
        rootfs
            .apply_entry(&interpreter_entry, &interpreter)
            .unwrap();

        let runtime = build_elf_runtime_plan(&rootfs, "bin/app").unwrap();
        assert_eq!(runtime.executable.path, "bin/app");
        assert_eq!(
            runtime.executable.interpreter,
            Some("/lib64/ld-linux-x86-64.so.2".into())
        );
        let interpreter = runtime.interpreter.unwrap();
        assert_eq!(interpreter.path, "lib64/ld-linux-x86-64.so.2");
        assert_eq!(interpreter.interpreter, None);

        let runtime = build_elf_runtime_plan(&rootfs, "bin/app").unwrap();
        let maps = build_elf_runtime_memory_map(&runtime).unwrap();
        assert_eq!(maps.executable.entry, 0x401000);
        assert!(maps.interpreter.is_some());
        assert_eq!(maps.interpreter.as_ref().unwrap().entry, 0x401000);
        let maps = build_elf_runtime_memory_map_with_page_size(&runtime, 0x2000).unwrap();
        assert_eq!(maps.executable.page_size, 0x2000);
        assert_eq!(maps.interpreter.as_ref().unwrap().page_size, 0x2000);
        let biased_maps = build_elf_runtime_memory_map_with_page_size_and_load_bias(
            &runtime,
            0x2000,
            OciElfLoadBias {
                executable: 0x10000000,
                interpreter: 0x20000000,
            },
        )
        .unwrap();
        assert_eq!(biased_maps.executable.load_bias, 0x10000000);
        assert_eq!(biased_maps.executable.entry, 0x10401000);
        assert_eq!(
            biased_maps.interpreter.as_ref().unwrap().load_bias,
            0x20000000
        );
        assert_eq!(biased_maps.interpreter.as_ref().unwrap().entry, 0x20401000);
        let auxv = build_elf64_auxv(&runtime, &biased_maps).unwrap();
        assert_eq!(auxv_value(&auxv, OCI_ELF_AT_PHDR), Some(0x10400040));
        assert_eq!(auxv_value(&auxv, OCI_ELF_AT_PHNUM), Some(2));
        assert_eq!(auxv_value(&auxv, OCI_ELF_AT_PAGESZ), Some(0x2000));
        assert_eq!(auxv_value(&auxv, OCI_ELF_AT_BASE), Some(0x20000000));
        assert_eq!(auxv_value(&auxv, OCI_ELF_AT_ENTRY), Some(0x10401000));

        let biased_prepared = prepare_oci_elf_program_with_page_size_and_load_bias(
            &rootfs,
            &["app".into()],
            &["PATH=/bin".into()],
            "/",
            0x2000,
            OciElfLoadBias {
                executable: 0x10000000,
                interpreter: 0x20000000,
            },
        )
        .unwrap()
        .unwrap();
        let mut stack = [0u8; 512];
        let stack_base = 0x7000_0000u64;
        let stack_top = stack_base + stack.len() as u64;
        let launch =
            write_prepared_elf64_launch_state(&biased_prepared, stack_base, stack_top, &mut stack)
                .unwrap();
        assert_eq!(launch.executable_entry, 0x10401000);
        assert_eq!(launch.interpreter_entry, Some(0x20401000));
        assert_eq!(launch.entry_point, 0x20401000);
        assert_eq!(
            stack_auxv_value(&stack, stack_base, launch.stack.auxv_addr, OCI_ELF_AT_BASE),
            0x20000000
        );
        let mappings =
            build_elf_runtime_mapping_list(&biased_prepared.runtime, &biased_prepared.memory)
                .unwrap();
        assert_eq!(mappings.len(), 2);
        assert_eq!(mappings[0].image, OciElfImage::Executable);
        assert_eq!(mappings[0].path, "bin/app");
        assert_eq!(mappings[0].mapping.map_start, 0x10400000);
        assert_eq!(mappings[1].image, OciElfImage::Interpreter);
        assert_eq!(mappings[1].path, "lib64/ld-linux-x86-64.so.2");
        assert_eq!(mappings[1].mapping.map_start, 0x20400000);
        let layout =
            build_elf_runtime_layout(&biased_prepared.runtime, &biased_prepared.memory).unwrap();
        assert_eq!(layout.map_start, 0x10400000);
        assert_eq!(layout.map_end, 0x20402000);
        assert_eq!(layout.mapped_bytes, 0x4000);
        assert_eq!(layout.mappings.len(), 2);
        let mut buf = [0u8; 16];
        let read = read_elf_runtime_mapping_list_chunk(
            &rootfs,
            &biased_prepared.runtime,
            &biased_prepared.memory,
            &mappings[1],
            0,
            &mut buf,
        )
        .unwrap();
        assert_eq!(
            read,
            OciElfSegmentRead {
                bytes_read: 16,
                zero_fill_bytes: 0
            }
        );
        assert_eq!(&buf[..4], &[0x7f, b'E', b'L', b'F']);

        let prepared = prepare_oci_elf_program_with_page_size(
            &rootfs,
            &["app".into()],
            &["PATH=/bin".into()],
            "/",
            0x2000,
        )
        .unwrap()
        .unwrap();
        assert_eq!(prepared.runtime.executable.path, "bin/app");
        assert_eq!(
            prepared.runtime.interpreter.as_ref().unwrap().path,
            "lib64/ld-linux-x86-64.so.2"
        );
        assert_eq!(prepared.memory.executable.page_size, 0x2000);
        assert_eq!(
            prepared.memory.interpreter.as_ref().unwrap().page_size,
            0x2000
        );

        let mut stack = [0u8; 512];
        let stack_base = 0x7000_0000u64;
        let stack_top = stack_base + stack.len() as u64;
        let launch =
            write_prepared_elf64_launch_state(&prepared, stack_base, stack_top, &mut stack)
                .unwrap();
        assert_eq!(launch.executable_entry, 0x401000);
        assert_eq!(launch.interpreter_entry, Some(0x401000));
        assert_eq!(launch.entry_point, 0x401000);
        assert_eq!(
            stack_auxv_value(&stack, stack_base, launch.stack.auxv_addr, OCI_ELF_AT_BASE),
            0
        );

        let mut buf = [0u8; 16];
        let read = read_elf_runtime_segment_chunk(
            &rootfs,
            &prepared.runtime,
            OciElfImage::Interpreter,
            0,
            0,
            &mut buf,
        )
        .unwrap();
        assert_eq!(
            read,
            OciElfSegmentRead {
                bytes_read: 16,
                zero_fill_bytes: 0
            }
        );
        assert_eq!(&buf[..4], &[0x7f, b'E', b'L', b'F']);
        let read = read_elf_runtime_mapping_chunk(
            &rootfs,
            &prepared.runtime,
            &prepared.memory,
            OciElfImage::Interpreter,
            0,
            0,
            &mut buf,
        )
        .unwrap();
        assert_eq!(
            read,
            OciElfSegmentRead {
                bytes_read: 16,
                zero_fill_bytes: 0
            }
        );
        assert_eq!(&buf[..4], &[0x7f, b'E', b'L', b'F']);
    }

    #[test]
    fn reads_page_aligned_mapping_prefix_and_tail_zeroes() {
        let mut rootfs = BareRootfs::new();
        let mut elf = minimal_elf64();
        let segment_start = 0x400123u64;
        let ph = ELF_HEADER_LEN;
        elf[ph + 16..ph + 24].copy_from_slice(&segment_start.to_le_bytes());
        elf[ph + 24..ph + 32].copy_from_slice(&segment_start.to_le_bytes());
        let entry = TarEntry {
            path: "bin/app".into(),
            kind: TarEntryKind::Regular,
            size: elf.len() as u64,
            mode: 0o755,
            uid: 0,
            gid: 0,
            mtime: 0,
            dev_major: None,
            dev_minor: None,
            link_name: None,
            whiteout: None,
        };
        rootfs.apply_entry(&entry, &elf).unwrap();

        let load = build_elf_load_plan(&rootfs, "bin/app").unwrap();
        let map = build_elf_memory_map(&load).unwrap();
        assert_eq!(map.mappings[0].map_start, 0x400000);
        assert_eq!(map.mappings[0].segment_start, segment_start);

        let mut buf = [0xaau8; 0x130];
        let read = read_elf_mapping_chunk(&rootfs, &load, &map, 0, 0, &mut buf).unwrap();
        assert_eq!(
            read,
            OciElfSegmentRead {
                bytes_read: 0x0d,
                zero_fill_bytes: 0x123
            }
        );
        assert!(buf[..0x123].iter().all(|byte| *byte == 0));
        assert_eq!(&buf[0x123..0x127], &[0x7f, b'E', b'L', b'F']);

        let mut buf = [0xaau8; 16];
        let tail_offset = 0x123 + 0x1000 - 8;
        let read = read_elf_mapping_chunk(&rootfs, &load, &map, 0, tail_offset, &mut buf).unwrap();
        assert_eq!(
            read,
            OciElfSegmentRead {
                bytes_read: 8,
                zero_fill_bytes: 8
            }
        );
        assert!(buf[8..].iter().all(|byte| *byte == 0));
    }

    #[test]
    fn writes_elf64_initial_stack_for_argv_env_and_auxv() {
        let stack_base = 0x7000_0000u64;
        let mut stack = [0xaau8; 512];
        let stack_top = stack_base + stack.len() as u64;
        let plan = write_elf64_initial_stack(
            stack_base,
            stack_top,
            &mut stack,
            &["app".into(), "--flag".into()],
            &["PATH=/bin".into()],
            &[
                OciElfAuxvEntry {
                    key: 3,
                    value: 0x400040,
                },
                OciElfAuxvEntry {
                    key: 9,
                    value: 0x401000,
                },
            ],
        )
        .unwrap();

        assert_eq!(plan.stack_pointer % 16, 0);
        assert_eq!(stack_u64(&stack, stack_base, plan.argc_addr), 2);
        let argv0 = stack_u64(&stack, stack_base, plan.argv_ptrs_addr);
        let argv1 = stack_u64(&stack, stack_base, plan.argv_ptrs_addr + 8);
        assert_eq!(stack_u64(&stack, stack_base, plan.argv_ptrs_addr + 16), 0);
        assert_eq!(stack_cstr(&stack, stack_base, argv0), "app");
        assert_eq!(stack_cstr(&stack, stack_base, argv1), "--flag");

        let env0 = stack_u64(&stack, stack_base, plan.env_ptrs_addr);
        assert_eq!(stack_u64(&stack, stack_base, plan.env_ptrs_addr + 8), 0);
        assert_eq!(stack_cstr(&stack, stack_base, env0), "PATH=/bin");
        assert_eq!(stack_u64(&stack, stack_base, plan.auxv_addr), 3);
        assert_eq!(stack_u64(&stack, stack_base, plan.auxv_addr + 8), 0x400040);
        assert_eq!(stack_u64(&stack, stack_base, plan.auxv_addr + 16), 9);
        assert_eq!(stack_u64(&stack, stack_base, plan.auxv_addr + 24), 0x401000);
        assert_eq!(stack_u64(&stack, stack_base, plan.auxv_addr + 32), 0);
        assert_eq!(stack_u64(&stack, stack_base, plan.auxv_addr + 40), 0);
    }

    #[test]
    fn prepares_biased_program_and_writes_prepared_initial_stack() {
        let mut rootfs = BareRootfs::new();
        let elf = minimal_elf64();
        let entry = TarEntry {
            path: "bin/app".into(),
            kind: TarEntryKind::Regular,
            size: elf.len() as u64,
            mode: 0o755,
            uid: 0,
            gid: 0,
            mtime: 0,
            dev_major: None,
            dev_minor: None,
            link_name: None,
            whiteout: None,
        };
        rootfs.apply_entry(&entry, &elf).unwrap();

        let prepared = prepare_oci_elf_program_with_load_bias(
            &rootfs,
            &["app".into(), "--flag".into()],
            &["PATH=/bin".into()],
            "/",
            OciElfLoadBias {
                executable: 0x10000000,
                interpreter: 0,
            },
        )
        .unwrap()
        .unwrap();
        assert_eq!(prepared.memory.executable.entry, 0x10401000);

        let stack_base = 0x7000_0000u64;
        let mut stack = [0u8; 512];
        let stack_top = stack_base + stack.len() as u64;
        let stack_plan =
            write_prepared_elf64_initial_stack(&prepared, stack_base, stack_top, &mut stack)
                .unwrap();
        assert_eq!(stack_u64(&stack, stack_base, stack_plan.argc_addr), 2);
        assert_eq!(
            stack_u64(&stack, stack_base, stack_plan.auxv_addr + 8),
            0x10400040
        );
        assert_eq!(
            stack_auxv_value(&stack, stack_base, stack_plan.auxv_addr, OCI_ELF_AT_ENTRY),
            0x10401000
        );

        let mut stack = [0u8; 512];
        let launch =
            write_prepared_elf64_launch_state(&prepared, stack_base, stack_top, &mut stack)
                .unwrap();
        assert_eq!(launch.executable_entry, 0x10401000);
        assert_eq!(launch.interpreter_entry, None);
        assert_eq!(launch.entry_point, 0x10401000);
        assert_eq!(launch.stack_pointer, launch.stack.stack_pointer);
    }

    #[test]
    fn prepares_and_loads_oci_elf_program() {
        let mut rootfs = BareRootfs::new();
        let elf = minimal_elf64();
        let entry = TarEntry {
            path: "bin/app".into(),
            kind: TarEntryKind::Regular,
            size: elf.len() as u64,
            mode: 0o755,
            uid: 0,
            gid: 0,
            mtime: 0,
            dev_major: None,
            dev_minor: None,
            link_name: None,
            whiteout: None,
        };
        rootfs.apply_entry(&entry, &elf).unwrap();

        let stack_base = 0x7000_0000u64;
        let mut stack = [0u8; 512];
        let stack_top = stack_base + stack.len() as u64;
        let mut scratch = [0u8; 128];
        let mut mapper = RecordingMapper::new();
        let launch = prepare_and_load_oci_elf_program(
            &rootfs,
            &["app".into()],
            &["PATH=/bin".into()],
            "/",
            &mut mapper,
            &mut scratch,
            stack_base,
            stack_top,
            &mut stack,
        )
        .unwrap()
        .unwrap();

        assert_eq!(launch.entry_point, 0x401000);
        assert_eq!(mapper.mapped.len(), 1);
        assert_eq!(mapper.protected.len(), 1);
        assert_eq!(mapper.stack_base, Some(stack_base));
        assert_eq!(mapper.stack_size, Some(stack.len() as u64));
        assert!(mapper
            .stack_bytes
            .as_ref()
            .unwrap()
            .iter()
            .any(|byte| *byte != 0));
        assert_eq!(&mapper.writes[0].1[..4], &[0x7f, b'E', b'L', b'F']);
    }

    #[test]
    fn rejects_invalid_initial_stack_inputs() {
        let stack_base = 0x7000_0000u64;
        let mut stack = [0u8; 32];
        let stack_top = stack_base + stack.len() as u64;

        assert!(matches!(
            write_elf64_initial_stack_aligned(stack_base, stack_top, &mut stack, &[], &[], &[], 24),
            Err(OciElfError::InvalidStackAlignment(24))
        ));
        assert!(matches!(
            write_elf64_initial_stack(
                stack_base,
                stack_top,
                &mut stack,
                &["bad\0arg".into()],
                &[],
                &[]
            ),
            Err(OciElfError::InvalidStackString)
        ));
        assert!(matches!(
            write_elf64_initial_stack(
                stack_base,
                stack_top + 8,
                &mut stack,
                &["app".into()],
                &[],
                &[]
            ),
            Err(OciElfError::StackTooSmall)
        ));
    }

    #[test]
    fn dynamic_runtime_plan_requires_interpreter_file() {
        let mut rootfs = BareRootfs::new();
        let elf = elf64_with_interpreter("/lib64/ld-linux-x86-64.so.2");
        let entry = TarEntry {
            path: "bin/app".into(),
            kind: TarEntryKind::Regular,
            size: elf.len() as u64,
            mode: 0o755,
            uid: 0,
            gid: 0,
            mtime: 0,
            dev_major: None,
            dev_minor: None,
            link_name: None,
            whiteout: None,
        };
        rootfs.apply_entry(&entry, &elf).unwrap();

        let err = build_elf_runtime_plan(&rootfs, "bin/app").unwrap_err();
        assert!(matches!(err, OciElfError::NotFound(path) if path == "lib64/ld-linux-x86-64.so.2"));
    }

    #[test]
    fn rejects_overlapping_runtime_image_mappings() {
        let mut rootfs = BareRootfs::new();
        let elf = elf64_with_interpreter("/lib64/ld-linux-x86-64.so.2");
        let entry = TarEntry {
            path: "bin/app".into(),
            kind: TarEntryKind::Regular,
            size: elf.len() as u64,
            mode: 0o755,
            uid: 0,
            gid: 0,
            mtime: 0,
            dev_major: None,
            dev_minor: None,
            link_name: None,
            whiteout: None,
        };
        rootfs.apply_entry(&entry, &elf).unwrap();
        let interpreter = minimal_elf64();
        let interpreter_entry = TarEntry {
            path: "lib64/ld-linux-x86-64.so.2".into(),
            kind: TarEntryKind::Regular,
            size: interpreter.len() as u64,
            mode: 0o755,
            uid: 0,
            gid: 0,
            mtime: 0,
            dev_major: None,
            dev_minor: None,
            link_name: None,
            whiteout: None,
        };
        rootfs
            .apply_entry(&interpreter_entry, &interpreter)
            .unwrap();

        let runtime = build_elf_runtime_plan(&rootfs, "bin/app").unwrap();
        let memory = build_elf_runtime_memory_map_with_load_bias(
            &runtime,
            OciElfLoadBias {
                executable: 0,
                interpreter: 0,
            },
        )
        .unwrap();
        assert!(matches!(
            build_elf_runtime_mapping_list(&runtime, &memory),
            Err(OciElfError::OverlappingLoadSegments)
        ));
        assert!(matches!(
            build_elf_runtime_layout(&runtime, &memory),
            Err(OciElfError::OverlappingLoadSegments)
        ));
    }

    #[test]
    fn rejects_non_elf_executables() {
        let mut rootfs = BareRootfs::new();
        let entry = TarEntry {
            path: "bin/app".into(),
            kind: TarEntryKind::Regular,
            size: 4,
            mode: 0o755,
            uid: 0,
            gid: 0,
            mtime: 0,
            dev_major: None,
            dev_minor: None,
            link_name: None,
            whiteout: None,
        };
        rootfs.apply_entry(&entry, b"nope").unwrap();

        let err = inspect_elf(&rootfs, "bin/app").unwrap_err();
        assert!(matches!(err, OciElfError::ShortRead { .. }));
    }

    #[test]
    fn rejects_invalid_load_segment_ranges() {
        let mut rootfs = BareRootfs::new();
        let mut elf = minimal_elf64();
        let ph = ELF_HEADER_LEN;
        let bad_file_size = elf.len() as u64 + 1;
        elf[ph + 32..ph + 40].copy_from_slice(&bad_file_size.to_le_bytes());
        let entry = TarEntry {
            path: "bin/app".into(),
            kind: TarEntryKind::Regular,
            size: elf.len() as u64,
            mode: 0o755,
            uid: 0,
            gid: 0,
            mtime: 0,
            dev_major: None,
            dev_minor: None,
            link_name: None,
            whiteout: None,
        };
        rootfs.apply_entry(&entry, &elf).unwrap();

        let err = build_elf_load_plan(&rootfs, "bin/app").unwrap_err();
        assert!(matches!(err, OciElfError::InvalidLoadSegment(_)));
    }

    #[test]
    fn rejects_overlapping_memory_mappings() {
        let load = OciElfLoadPlan {
            path: "bin/app".into(),
            elf_type: OciElfType::Executable,
            entry: 0x1000,
            machine: OciElfMachine::X86_64,
            program_header_offset: ELF_HEADER_LEN as u64,
            program_header_entry_size: ELF64_PHDR_LEN as u16,
            program_header_count: 1,
            interpreter: None,
            segments: vec![
                OciElfLoadSegment {
                    file_offset: 0,
                    virtual_addr: 0x1000,
                    file_size: 0x1000,
                    memory_size: 0x1000,
                    flags: PF_X,
                    align: 0x1000,
                },
                OciElfLoadSegment {
                    file_offset: 0x1000,
                    virtual_addr: 0x1800,
                    file_size: 0x1000,
                    memory_size: 0x1000,
                    flags: 0,
                    align: 0x1000,
                },
            ],
        };

        assert!(matches!(
            build_elf_memory_map(&load),
            Err(OciElfError::OverlappingLoadSegments)
        ));
    }

    fn auxv_value(auxv: &[OciElfAuxvEntry], key: u64) -> Option<u64> {
        auxv.iter()
            .find(|entry| entry.key == key)
            .map(|entry| entry.value)
    }

    fn stack_u64(stack: &[u8], stack_base: u64, addr: u64) -> u64 {
        let offset = (addr - stack_base) as usize;
        read_u64(stack, offset)
    }

    fn stack_cstr(stack: &[u8], stack_base: u64, addr: u64) -> &str {
        let offset = (addr - stack_base) as usize;
        let end = stack[offset..]
            .iter()
            .position(|byte| *byte == 0)
            .map(|len| offset + len)
            .unwrap();
        core::str::from_utf8(&stack[offset..end]).unwrap()
    }

    fn stack_auxv_value(stack: &[u8], stack_base: u64, auxv_addr: u64, key: u64) -> u64 {
        let mut addr = auxv_addr;
        loop {
            let entry_key = stack_u64(stack, stack_base, addr);
            let value = stack_u64(stack, stack_base, addr + 8);
            if entry_key == key || entry_key == OCI_ELF_AT_NULL {
                return value;
            }
            addr += 16;
        }
    }

    struct RecordingMapper {
        mapped: Vec<OciElfRuntimeMapping>,
        writes: Vec<(u64, Vec<u8>)>,
        protected: Vec<OciElfRuntimeMapping>,
        stack_base: Option<u64>,
        stack_size: Option<u64>,
        stack_bytes: Option<Vec<u8>>,
    }

    impl RecordingMapper {
        fn new() -> Self {
            Self {
                mapped: Vec::new(),
                writes: Vec::new(),
                protected: Vec::new(),
                stack_base: None,
                stack_size: None,
                stack_bytes: None,
            }
        }
    }

    impl OciElfMapper for RecordingMapper {
        fn map_elf_region(&mut self, mapping: &OciElfRuntimeMapping) -> Result<(), OciElfError> {
            self.mapped.push(mapping.clone());
            Ok(())
        }

        fn write_elf_region(
            &mut self,
            mapping: &OciElfRuntimeMapping,
            mapping_offset: u64,
            bytes: &[u8],
        ) -> Result<(), OciElfError> {
            let addr = mapping
                .mapping
                .map_start
                .checked_add(mapping_offset)
                .ok_or(OciElfError::Overflow)?;
            self.writes.push((addr, bytes.to_vec()));
            Ok(())
        }

        fn protect_elf_region(
            &mut self,
            mapping: &OciElfRuntimeMapping,
        ) -> Result<(), OciElfError> {
            self.protected.push(mapping.clone());
            Ok(())
        }

        fn map_stack_region(
            &mut self,
            stack_base: u64,
            stack_size: u64,
        ) -> Result<(), OciElfError> {
            self.stack_base = Some(stack_base);
            self.stack_size = Some(stack_size);
            Ok(())
        }

        fn write_stack_region(
            &mut self,
            _stack_base: u64,
            bytes: &[u8],
        ) -> Result<(), OciElfError> {
            self.stack_bytes = Some(bytes.to_vec());
            Ok(())
        }
    }

    fn minimal_elf64() -> Vec<u8> {
        let mut out = vec![0u8; 0x1000];
        out[0..4].copy_from_slice(&[0x7f, b'E', b'L', b'F']);
        out[EI_CLASS] = ELFCLASS64;
        out[EI_DATA] = ELFDATA2LSB;
        out[EI_VERSION] = EV_CURRENT;
        out[16..18].copy_from_slice(&2u16.to_le_bytes());
        out[18..20].copy_from_slice(&62u16.to_le_bytes());
        out[20..24].copy_from_slice(&1u32.to_le_bytes());
        out[24..32].copy_from_slice(&0x401000u64.to_le_bytes());
        out[32..40].copy_from_slice(&(ELF_HEADER_LEN as u64).to_le_bytes());
        out[52..54].copy_from_slice(&(ELF_HEADER_LEN as u16).to_le_bytes());
        out[54..56].copy_from_slice(&(ELF64_PHDR_LEN as u16).to_le_bytes());
        out[56..58].copy_from_slice(&1u16.to_le_bytes());

        let ph = ELF_HEADER_LEN;
        out[ph..ph + 4].copy_from_slice(&PT_LOAD.to_le_bytes());
        out[ph + 4..ph + 8].copy_from_slice(&5u32.to_le_bytes());
        out[ph + 8..ph + 16].copy_from_slice(&0u64.to_le_bytes());
        out[ph + 16..ph + 24].copy_from_slice(&0x400000u64.to_le_bytes());
        out[ph + 24..ph + 32].copy_from_slice(&0x400000u64.to_le_bytes());
        out[ph + 32..ph + 40].copy_from_slice(&0x1000u64.to_le_bytes());
        out[ph + 40..ph + 48].copy_from_slice(&0x2000u64.to_le_bytes());
        out[ph + 48..ph + 56].copy_from_slice(&0x1000u64.to_le_bytes());
        out
    }

    fn elf64_with_interpreter(interpreter: &str) -> Vec<u8> {
        let mut out = minimal_elf64();
        let interp_offset = 0x200usize;
        let phoff = ELF_HEADER_LEN;
        let interp_ph = phoff + ELF64_PHDR_LEN;
        let interp_len = interpreter.len() + 1;
        out[32..40].copy_from_slice(&(phoff as u64).to_le_bytes());
        out[56..58].copy_from_slice(&2u16.to_le_bytes());

        out[interp_ph..interp_ph + 4].copy_from_slice(&PT_INTERP.to_le_bytes());
        out[interp_ph + 8..interp_ph + 16].copy_from_slice(&(interp_offset as u64).to_le_bytes());
        out[interp_ph + 32..interp_ph + 40].copy_from_slice(&(interp_len as u64).to_le_bytes());
        out[interp_ph + 40..interp_ph + 48].copy_from_slice(&(interp_len as u64).to_le_bytes());
        out[interp_offset..interp_offset + interpreter.len()]
            .copy_from_slice(interpreter.as_bytes());
        out[interp_offset + interpreter.len()] = 0;
        out
    }
}
