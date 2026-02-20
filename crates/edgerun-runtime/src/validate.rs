use anyhow::{anyhow, bail, Result};
use std::collections::HashSet;
use wasmi::ValType;
use wasmparser::{Parser, Payload, Validator, WasmFeatures};

const IMPORT_MODULE: &str = "edgerun";
const ALLOWED_IMPORTS: [&str; 3] = ["input_len", "read_input", "write_output"];

pub fn validate_wasm_module(wasm: &[u8]) -> Result<()> {
    validate_no_floats(wasm)?;
    validate_imports(wasm)?;
    validate_exports_and_memory_policy(wasm)?;
    validate_entrypoint_present(wasm)?;
    Ok(())
}

fn validate_no_floats(wasm: &[u8]) -> Result<()> {
    let mut features = WasmFeatures::default();
    features.remove(WasmFeatures::FLOATS);
    let mut validator = Validator::new_with_features(features);

    for payload in Parser::new(0).parse_all(wasm) {
        let payload = payload?;
        validator.payload(&payload)?;
    }

    Ok(())
}

fn validate_imports(wasm: &[u8]) -> Result<()> {
    validate_allowed_import_names(wasm)?;
    validate_import_signatures(wasm)?;
    Ok(())
}

fn validate_allowed_import_names(wasm: &[u8]) -> Result<()> {
    for payload in Parser::new(0).parse_all(wasm) {
        if let Payload::ImportSection(imports) = payload? {
            for import in imports {
                let import = import?;
                let field = import.name;
                if import.module != IMPORT_MODULE || !ALLOWED_IMPORTS.contains(&field) {
                    bail!("disallowed import {}::{}", import.module, field);
                }
            }
        }
    }

    Ok(())
}

fn validate_import_signatures(wasm: &[u8]) -> Result<()> {
    let engine = wasmi::Engine::default();
    let module = wasmi::Module::new(&engine, wasm)
        .map_err(|e| anyhow!("failed to inspect import signatures: {e}"))?;

    let mut seen = HashSet::new();
    for import in module.imports() {
        if import.module() != IMPORT_MODULE {
            continue;
        }

        let name = import.name();
        let func_ty = import
            .ty()
            .func()
            .ok_or_else(|| anyhow!("import {IMPORT_MODULE}::{name} must be a function"))?;

        if !seen.insert(name.to_string()) {
            bail!("duplicate import {IMPORT_MODULE}::{name}");
        }

        match name {
            "input_len" => expect_signature(name, func_ty, &[], &[ValType::I32])?,
            "read_input" => expect_signature(
                name,
                func_ty,
                &[ValType::I32, ValType::I32, ValType::I32],
                &[ValType::I32],
            )?,
            "write_output" => expect_signature(
                name,
                func_ty,
                &[ValType::I32, ValType::I32],
                &[ValType::I32],
            )?,
            _ => bail!("disallowed import {IMPORT_MODULE}::{name}"),
        }
    }

    Ok(())
}

fn expect_signature(
    name: &str,
    ty: &wasmi::FuncType,
    params: &[ValType],
    results: &[ValType],
) -> Result<()> {
    if ty.params() != params || ty.results() != results {
        bail!(
            "import {IMPORT_MODULE}::{name} has invalid signature: params={:?} results={:?}",
            ty.params(),
            ty.results()
        );
    }
    Ok(())
}

fn validate_entrypoint_present(wasm: &[u8]) -> Result<()> {
    for payload in Parser::new(0).parse_all(wasm) {
        if let Payload::ExportSection(exports) = payload? {
            for export in exports {
                let export = export?;
                if export.name == "_start" {
                    return Ok(());
                }
            }
        }
    }

    bail!("module must export _start")
}

fn validate_exports_and_memory_policy(wasm: &[u8]) -> Result<()> {
    let engine = wasmi::Engine::default();
    let module = wasmi::Module::new(&engine, wasm)
        .map_err(|e| anyhow!("failed to inspect export/memory policy: {e}"))?;

    let mut saw_start = false;
    let mut saw_memory = false;

    for export in module.exports() {
        match export.name() {
            "_start" => {
                let Some(func_ty) = export.ty().func() else {
                    bail!("export _start must be a function");
                };
                if !func_ty.params().is_empty() || !func_ty.results().is_empty() {
                    bail!(
                        "export _start must have signature () -> (), got params={:?} results={:?}",
                        func_ty.params(),
                        func_ty.results()
                    );
                }
                saw_start = true;
            }
            "memory" => {
                let Some(memory_ty) = export.ty().memory() else {
                    bail!("export memory must be a memory");
                };
                if memory_ty.is_64() {
                    bail!("export memory must be 32-bit");
                }
                if memory_ty.maximum().is_none() {
                    bail!("export memory must define a maximum page limit");
                }
                saw_memory = true;
            }
            other => bail!("disallowed export {other}; only _start and memory are allowed"),
        }
    }

    if !saw_start {
        bail!("module must export _start");
    }
    if !saw_memory {
        bail!("module must export memory");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_wasm_module;

    #[test]
    fn accepts_valid_import_signatures() {
        let wasm = wat::parse_str(
            r#"(module
                (import "edgerun" "input_len" (func $input_len (result i32)))
                (import "edgerun" "read_input" (func $read_input (param i32 i32 i32) (result i32)))
                (import "edgerun" "write_output" (func $write_output (param i32 i32) (result i32)))
                (memory (export "memory") 1 1)
                (func (export "_start")
                    (drop (call $input_len))
                )
            )"#,
        )
        .expect("wat parse");

        validate_wasm_module(&wasm).expect("valid module");
    }

    #[test]
    fn rejects_invalid_import_signature() {
        let wasm = wat::parse_str(
            r#"(module
                (import "edgerun" "read_input" (func $read_input (param i32 i32 i32) (result i64)))
                (memory (export "memory") 1)
                (func (export "_start"))
            )"#,
        )
        .expect("wat parse");

        let err = validate_wasm_module(&wasm).expect_err("must reject");
        assert!(err.to_string().contains("invalid signature"));
    }

    #[test]
    fn rejects_non_function_import_kind() {
        let wasm = wat::parse_str(
            r#"(module
                (import "edgerun" "input_len" (memory 1))
                (func (export "_start"))
            )"#,
        )
        .expect("wat parse");

        let err = validate_wasm_module(&wasm).expect_err("must reject");
        assert!(err.to_string().contains("must be a function"));
    }

    #[test]
    fn rejects_missing_memory_export() {
        let wasm = wat::parse_str(
            r#"(module
                (func (export "_start"))
            )"#,
        )
        .expect("wat parse");

        let err = validate_wasm_module(&wasm).expect_err("must reject");
        assert!(err.to_string().contains("export memory") || err.to_string().contains("memory"));
    }

    #[test]
    fn rejects_extra_exports() {
        let wasm = wat::parse_str(
            r#"(module
                (memory (export "memory") 1 1)
                (func (export "_start"))
                (func (export "helper"))
            )"#,
        )
        .expect("wat parse");

        let err = validate_wasm_module(&wasm).expect_err("must reject");
        assert!(err.to_string().contains("disallowed export"));
    }
}
