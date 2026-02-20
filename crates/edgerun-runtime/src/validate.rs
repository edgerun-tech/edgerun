use anyhow::{bail, Result};
use wasmparser::{Parser, Payload, Validator, WasmFeatures};

const IMPORT_MODULE: &str = "edgerun";
const ALLOWED_IMPORTS: [&str; 3] = ["input_len", "read_input", "write_output"];

pub fn validate_wasm_module(wasm: &[u8]) -> Result<()> {
    validate_no_floats(wasm)?;
    validate_imports(wasm)?;
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
