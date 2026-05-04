#![allow(clippy::all)]

#[cfg(feature = "std")]
extern crate std;

// Minimal WASM binary parser for EdgeRun ABI validation
// Supports only the sections needed for EdgeRun: Type, Import, Function, Export

const WASM_MAGIC: [u8; 4] = *b"\0asm";
const WASM_VERSION: [u8; 4] = [1, 0, 0, 0];

// Section IDs
const SECTION_TYPE: u8 = 1;
const SECTION_IMPORT: u8 = 2;
const SECTION_FUNCTION: u8 = 3;
const SECTION_EXPORT: u8 = 7;

// Export kinds
const EXPORT_FUNC: u8 = 0;
const EXPORT_MEM: u8 = 2;

// Import kinds
const IMPORT_FUNC: u8 = 0;

// ValType encoding
const VAL_I32: u8 = 0x7F;
const VAL_I64: u8 = 0x7E;
const VAL_F32: u8 = 0x7D;
const VAL_F64: u8 = 0x7C;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValType {
    I32,
    I64,
    F32,
    F64,
}

#[derive(Debug, Clone)]
pub struct FuncType {
    pub params: Vec<ValType>,
    pub results: Vec<ValType>,
}

#[derive(Debug, Clone)]
pub struct Import {
    pub module: String,
    pub name: String,
    pub kind: ImportKind,
}

#[derive(Debug, Clone)]
pub enum ImportKind {
    Func { type_idx: u32 },
}

#[derive(Debug, Clone)]
pub struct Export {
    pub name: String,
    pub kind: ExportKind,
    pub index: u32,
}

#[derive(Debug, Clone)]
pub enum ExportKind {
    Func,
    Memory,
}

#[derive(Debug, Default)]
pub struct ModuleInfo {
    pub types: Vec<FuncType>,
    pub imports: Vec<Import>,
    pub func_type_indices: Vec<u32>,
    pub exports: Vec<Export>,
}

#[derive(Debug)]
pub enum ParseError {
    InvalidMagic,
    InvalidVersion,
    InvalidSectionId(u8),
    InvalidLeb128,
    UnexpectedEof,
    InvalidImportKind(u8),
    InvalidExportKind(u8),
    InvalidValType(u8),
}

// LEB128 decoding
fn read_u32_leb128(data: &[u8], offset: &mut usize) -> Result<u32, ParseError> {
    let mut result: u32 = 0;
    let mut shift = 0;
    loop {
        if *offset >= data.len() {
            return Err(ParseError::UnexpectedEof);
        }
        let byte = data[*offset];
        *offset += 1;
        result |= ((byte & 0x7F) as u32) << shift;
        if byte & 0x80 == 0 {
            break;
        }
        shift += 7;
        if shift >= 35 {
            return Err(ParseError::InvalidLeb128);
        }
    }
    Ok(result)
}

fn read_u8(data: &[u8], offset: &mut usize) -> Result<u8, ParseError> {
    if *offset >= data.len() {
        return Err(ParseError::UnexpectedEof);
    }
    let byte = data[*offset];
    *offset += 1;
    Ok(byte)
}

fn read_bytes<'a>(data: &'a [u8], offset: &mut usize, len: usize) -> Result<&'a [u8], ParseError> {
    if *offset + len > data.len() {
        return Err(ParseError::UnexpectedEof);
    }
    let slice = &data[*offset..*offset + len];
    *offset += len;
    Ok(slice)
}

fn read_string(data: &[u8], offset: &mut usize) -> Result<String, ParseError> {
    let len = read_u32_leb128(data, offset)? as usize;
    let bytes = read_bytes(data, offset, len)?;
    String::from_utf8(bytes.to_vec()).map_err(|_| ParseError::InvalidLeb128)
}

fn read_val_type(data: &[u8], offset: &mut usize) -> Result<ValType, ParseError> {
    let byte = read_u8(data, offset)?;
    match byte {
        VAL_I32 => Ok(ValType::I32),
        VAL_I64 => Ok(ValType::I64),
        VAL_F32 => Ok(ValType::F32),
        VAL_F64 => Ok(ValType::F64),
        _ => Err(ParseError::InvalidValType(byte)),
    }
}

impl ModuleInfo {
    pub fn parse(data: &[u8]) -> Result<Self, ParseError> {
        let mut offset = 0;

        // Check magic number
        let magic = read_bytes(data, &mut offset, 4)?;
        if magic != WASM_MAGIC {
            return Err(ParseError::InvalidMagic);
        }

        // Check version
        let version = read_bytes(data, &mut offset, 4)?;
        if version != WASM_VERSION {
            return Err(ParseError::InvalidVersion);
        }

        let mut info = ModuleInfo::default();

        // Parse sections
        while offset < data.len() {
            let section_id = read_u8(data, &mut offset)?;
            let section_size = read_u32_leb128(data, &mut offset)? as usize;
            let section_end = offset + section_size;

            match section_id {
                SECTION_TYPE => parse_type_section(data, &mut offset, &mut info)?,
                SECTION_IMPORT => parse_import_section(data, &mut offset, &mut info)?,
                SECTION_FUNCTION => parse_function_section(data, &mut offset, &mut info)?,
                SECTION_EXPORT => parse_export_section(data, &mut offset, &mut info)?,
                _ => {
                    // Skip unknown sections
                }
            }

            offset = section_end;
        }

        Ok(info)
    }
}

fn parse_type_section(data: &[u8], offset: &mut usize, info: &mut ModuleInfo) -> Result<(), ParseError> {
    let count = read_u32_leb128(data, offset)? as usize;
    for _ in 0..count {
        let form = read_u8(data, offset)?;
        if form != 0x60 {
            // func type marker
            return Err(ParseError::InvalidLeb128);
        }

        let param_count = read_u32_leb128(data, offset)? as usize;
        let mut params = Vec::with_capacity(param_count);
        for _ in 0..param_count {
            params.push(read_val_type(data, offset)?);
        }

        let result_count = read_u32_leb128(data, offset)? as usize;
        let mut results = Vec::with_capacity(result_count);
        for _ in 0..result_count {
            results.push(read_val_type(data, offset)?);
        }

        info.types.push(FuncType { params, results });
    }
    Ok(())
}

fn parse_import_section(data: &[u8], offset: &mut usize, info: &mut ModuleInfo) -> Result<(), ParseError> {
    let count = read_u32_leb128(data, offset)? as usize;
    for _ in 0..count {
        let module = read_string(data, offset)?;
        let name = read_string(data, offset)?;
        let kind = read_u8(data, offset)?;

        match kind {
            IMPORT_FUNC => {
                let type_idx = read_u32_leb128(data, offset)?;
                info.imports.push(Import {
                    module,
                    name,
                    kind: ImportKind::Func { type_idx },
                });
            }
            _ => return Err(ParseError::InvalidImportKind(kind)),
        }
    }
    Ok(())
}

fn parse_function_section(data: &[u8], offset: &mut usize, info: &mut ModuleInfo) -> Result<(), ParseError> {
    let count = read_u32_leb128(data, offset)? as usize;
    for _ in 0..count {
        let type_idx = read_u32_leb128(data, offset)?;
        info.func_type_indices.push(type_idx);
    }
    Ok(())
}

fn parse_export_section(data: &[u8], offset: &mut usize, info: &mut ModuleInfo) -> Result<(), ParseError> {
    let count = read_u32_leb128(data, offset)? as usize;
    for _ in 0..count {
        let name = read_string(data, offset)?;
        let kind = read_u8(data, offset)?;
        let index = read_u32_leb128(data, offset)?;

        let export_kind = match kind {
            EXPORT_FUNC => ExportKind::Func,
            EXPORT_MEM => ExportKind::Memory,
            _ => return Err(ParseError::InvalidExportKind(kind)),
        };

        info.exports.push(Export {
            name,
            kind: export_kind,
            index,
        });
    }
    Ok(())
}

// Validation helpers for EdgeRun ABI

impl ModuleInfo {
    /// Check that the module exports a `run` function with signature (i32, i32) -> i32
    pub fn validate_run_export(&self) -> Result<(), ValidationError> {
        let run_export = self
            .exports
            .iter()
            .find(|e| e.name == "run" && matches!(e.kind, ExportKind::Func))
            .ok_or(ValidationError::MissingRunExport)?;

        // Calculate the function index accounting for imports
        let func_idx = run_export.index as usize;

        // Determine the type index for this function
        let type_idx = if func_idx < self.imports.len() {
            match self.imports[func_idx].kind {
                ImportKind::Func { type_idx } => type_idx as usize,
            }
        } else {
            let local_idx = func_idx - self.imports.len();
            *self
                .func_type_indices
                .get(local_idx)
                .ok_or(ValidationError::InvalidFunctionIndex)? as usize
        };

        let func_type = self
            .types
            .get(type_idx)
            .ok_or(ValidationError::InvalidTypeIndex)?;

        if func_type.params.len() != 2 {
            return Err(ValidationError::InvalidRunSignature);
        }
        if !func_type.params.iter().all(|p| matches!(p, ValType::I32)) {
            return Err(ValidationError::InvalidRunSignature);
        }
        if func_type.results.len() != 1 {
            return Err(ValidationError::InvalidRunSignature);
        }
        if !matches!(func_type.results[0], ValType::I32) {
            return Err(ValidationError::InvalidRunSignature);
        }

        Ok(())
    }

    /// Check that all imports are from the "env" module (EdgeRun host functions)
    pub fn validate_imports(&self) -> Result<(), ValidationError> {
        for import in &self.imports {
            if import.module != "env" {
                return Err(ValidationError::NonEnvImport(import.module.clone()));
            }
        }
        Ok(())
    }

    /// Validate complete EdgeRun ABI compliance
    pub fn validate_edgerun_abi(&self) -> Result<(), ValidationError> {
        self.validate_imports()?;
        self.validate_run_export()?;
        Ok(())
    }
}

#[derive(Debug)]
pub enum ValidationError {
    MissingRunExport,
    InvalidRunSignature,
    InvalidFunctionIndex,
    InvalidTypeIndex,
    NonEnvImport(String),
}

/// Convenience function: validate a WASM module for EdgeRun ABI compliance
pub fn validate_wasm(data: &[u8]) -> Result<(), ValidationError> {
    let info = ModuleInfo::parse(data).map_err(|e| match e {
        ParseError::InvalidMagic => ValidationError::MissingRunExport,
        ParseError::InvalidVersion => ValidationError::MissingRunExport,
        ParseError::UnexpectedEof => ValidationError::MissingRunExport,
        ParseError::InvalidLeb128 => ValidationError::MissingRunExport,
        ParseError::InvalidValType(_) => ValidationError::MissingRunExport,
        ParseError::InvalidImportKind(_) => ValidationError::MissingRunExport,
        ParseError::InvalidExportKind(_) => ValidationError::MissingRunExport,
        _ => ValidationError::MissingRunExport,
    })?;
    info.validate_edgerun_abi()
}
