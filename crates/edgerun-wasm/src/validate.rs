use anyhow::{Context, Result};
use edgerun_clap::Parser;
use std::path::Path;
mod wasmparser_mock;
use wasmparser_mock::{ExternalKind, TypeRef};

#[derive(Parser, Debug)]
#[command(name = "edgerun-validate", about = "Validate EdgeRun WASM modules")]
struct Args {
    #[arg(default_value = "app.wasm")]
    file: String,

    #[arg(short, long)]
    max_size: Option<usize>,

    #[arg(short, long)]
    verbose: bool,
}

const WASI_PREFIXES: &[&str] = &["wasi_snapshot_preview1", "wasi_ephemeral", "wasi", "wasi:"];

#[derive(Clone, Copy, PartialEq)]
enum ValType {
    I32,
    I64,
    F32,
    F64,
    V128,
    Ref,
}

impl From<wasmparser_mock::ValType> for ValType {
    fn from(t: wasmparser_mock::ValType) -> Self {
        match t {
            wasmparser_mock::ValType::I32 => ValType::I32,
            wasmparser_mock::ValType::I64 => ValType::I64,
            wasmparser_mock::ValType::F32 => ValType::F32,
            wasmparser_mock::ValType::F64 => ValType::F64,
            wasmparser_mock::ValType::V128 => ValType::V128,
            wasmparser_mock::ValType::Ref(_) => ValType::Ref,
        }
    }
}

struct TypeEntry {
    params: Vec<ValType>,
    results: Vec<ValType>,
}

impl TypeEntry {
    fn from_sig(sig: wasmparser_mock::FuncType) -> Self {
        Self {
            params: sig.params().iter().map(|&t| ValType::from(t)).collect(),
            results: sig.results().iter().map(|&t| ValType::from(t)).collect(),
        }
    }
}

struct ModuleInfo {
    types: Vec<TypeEntry>,
    func_type_indices: Vec<u32>,
    import_funcs: Vec<(String, u32)>,
    exported_func_indices: Vec<(String, u32)>,
}

fn parse_module(wasm: &[u8]) -> Result<ModuleInfo> {
    let mut types = Vec::new();
    let mut func_type_indices: Vec<u32> = Vec::new();
    let mut import_funcs: Vec<(String, u32)> = Vec::new();
    let mut exported_func_indices: Vec<(String, u32)> = Vec::new();

    for payload in wasmparser_mock::Parser::new(0).parse_all(wasm) {
        match payload? {
            wasmparser_mock::Payload::TypeSection(s) => {
                for group in s {
                    let group = group?;
                    for sub in group.types() {
                        let func_ty = sub.composite_type.unwrap_func().clone();
                        types.push(TypeEntry::from_sig(func_ty));
                    }
                }
            }
            wasmparser_mock::Payload::ImportSection(s) => {
                for import in s {
                    let import = import?;
                    if let TypeRef::Func(type_idx) = import.ty {
                        let full = format!("{}.{}", import.module, import.name);
                        import_funcs.push((full, type_idx));
                        func_type_indices.push(type_idx);
                    } else {
                        func_type_indices.push(0);
                    }
                }
            }
            wasmparser_mock::Payload::FunctionSection(s) => {
                for ty in s {
                    let ty = ty?;
                    func_type_indices.push(ty);
                }
            }
            wasmparser_mock::Payload::ExportSection(s) => {
                for export in s {
                    let export = export?;
                    if let wasmparser_mock::ExternalKind::Func = export.kind {
                        exported_func_indices.push((export.name.to_string(), export.index));
                    }
                }
            }
            _ => {}
        }
    }

    Ok(ModuleInfo {
        types,
        func_type_indices,
        import_funcs,
        exported_func_indices,
    })
}

fn main() -> Result<()> {
    let args = Args::parse();

    let path = Path::new(&args.file);
    let file_size = path.metadata().context("Failed to read file")?.len() as usize;

    let max_size = args.max_size.unwrap_or(1024 * 1024);

    if file_size > max_size {
        anyhow::bail!("File too large: {} bytes (max: {})", file_size, max_size);
    }

    if args.verbose {
        eprintln!("Checking: {} ({} bytes)", args.file, file_size);
    }

    let wasm = std::fs::read(path).context("Failed to read WASM")?;

    wasmparser_mock::Validator::new()
        .validate_all(&wasm)
        .context("Invalid WASM structure")?;

    if args.verbose {
        eprintln!("  Valid WASM structure");
    }

    let info = parse_module(&wasm)?;

    check_exports(&info, &args)?;
    check_imports(&info, &args)?;

    println!("PASS: {}", args.file);
    println!("  Size: {} bytes (max: {})", file_size, max_size);

    Ok(())
}

fn check_exports(info: &ModuleInfo, args: &Args) -> Result<()> {
    let mut has_run = false;
    let expected_params = vec![ValType::I32, ValType::I32];
    let expected_results = vec![ValType::I32];

    for (name, func_idx) in &info.exported_func_indices {
        if name == "run" {
            has_run = true;
            let type_idx = info
                .func_type_indices
                .get(*func_idx as usize)
                .copied()
                .context("Could not resolve 'run' function type")?;
            let ty = &info.types[type_idx as usize];

            if ty.params != expected_params {
                anyhow::bail!(
                    "'run' export has wrong parameter types: expected (i32, i32), found ({})",
                    ty.params
                        .iter()
                        .map(format_val_type)
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }
            if ty.results != expected_results {
                anyhow::bail!(
                    "'run' export has wrong result type: expected (i32), found ({})",
                    ty.results
                        .iter()
                        .map(format_val_type)
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }
            if args.verbose {
                eprintln!("  Found 'run' export with correct signature (i32, i32) -> i32");
            }
        }
    }

    if !has_run {
        anyhow::bail!("Missing required export: 'run(ptr: i32, len: i32) -> i32'");
    }

    Ok(())
}

fn check_imports(info: &ModuleInfo, args: &Args) -> Result<()> {
    const ALLOWED: &[(&str, &[ValType], &[ValType])] = &[
        ("write_output", &[ValType::I32, ValType::I32], &[]),
        (
            "send_message",
            &[ValType::I32, ValType::I32, ValType::I32, ValType::I32],
            &[ValType::I32],
        ),
        (
            "read_blob",
            &[ValType::I32, ValType::I32, ValType::I32],
            &[ValType::I32],
        ),
        (
            "write_blob",
            &[ValType::I32, ValType::I32, ValType::I32],
            &[ValType::I32],
        ),
        ("poll_event", &[ValType::I32, ValType::I32], &[ValType::I32]),
    ];

    for (full, type_idx) in &info.import_funcs {
        if is_wasi_module(full) {
            anyhow::bail!("WASI imports are not allowed: found '{}'", full);
        }

        let name = full.strip_prefix("env.").unwrap_or(full);

        let allowed_entry = ALLOWED.iter().find(|(n, _, _)| *n == name);

        if allowed_entry.is_none() {
            anyhow::bail!(
                "Unknown import '{}' in 'env' namespace. Only these are allowed: {}",
                full,
                ALLOWED
                    .iter()
                    .map(|(n, _, _)| *n)
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }

        let (_, expected_params, expected_results) = allowed_entry.unwrap();
        let ty = &info.types[*type_idx as usize];

        if ty.params != *expected_params {
            anyhow::bail!(
                "Import '{}' has wrong parameter types: expected ({}), found ({})",
                full,
                expected_params
                    .iter()
                    .map(format_val_type)
                    .collect::<Vec<_>>()
                    .join(", "),
                ty.params
                    .iter()
                    .map(format_val_type)
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
        if ty.results != *expected_results {
            anyhow::bail!(
                "Import '{}' has wrong result types: expected ({}), found ({})",
                full,
                expected_results
                    .iter()
                    .map(format_val_type)
                    .collect::<Vec<_>>()
                    .join(", "),
                ty.results
                    .iter()
                    .map(format_val_type)
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }

        if args.verbose {
            eprintln!("  Import: {} (correct signature)", full);
        }
    }

    if args.verbose {
        eprintln!("  All imports from 'env' namespace only");
        eprintln!("  No WASI imports detected");
        eprintln!("  All import signatures validated");
    }

    Ok(())
}

fn format_val_type(vt: &ValType) -> &'static str {
    match vt {
        ValType::I32 => "i32",
        ValType::I64 => "i64",
        ValType::F32 => "f32",
        ValType::F64 => "f64",
        ValType::V128 => "v128",
        ValType::Ref => "ref",
    }
}

fn is_wasi_module(module: &str) -> bool {
    WASI_PREFIXES
        .iter()
        .any(|prefix| module.starts_with(prefix))
}
