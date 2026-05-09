//! no_std ELF executable probe for OCI rootfs launch plans.

pub use crate::elf_memory::{
    build_elf_memory_map, build_elf_memory_map_with_load_bias, build_elf_memory_map_with_page_size,
    build_elf_memory_map_with_page_size_and_load_bias, build_elf_runtime_layout,
    build_elf_runtime_mapping_list, build_elf_runtime_memory_map,
    build_elf_runtime_memory_map_with_load_bias, build_elf_runtime_memory_map_with_page_size,
    build_elf_runtime_memory_map_with_page_size_and_load_bias,
};
pub use crate::elf_stack::{
    build_elf64_auxv, write_elf64_initial_stack, write_elf64_initial_stack_aligned,
};
use crate::prelude::*;
use crate::rootfs_access::{
    OciLaunchPlan, OciRootfs, OciRootfsError, build_launch_plan, normalize_rootfs_path,
};
use core::fmt;
use edgerun_encoding::byteorder::{
    read_u16_le as read_u16, read_u32_le as read_u32, read_u64_le as read_u64,
};

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
pub(crate) const DEFAULT_PAGE_SIZE: u64 = 4096;

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
