use anyhow::{bail, Context, Result};
use std::collections::BTreeMap;

use super::backend_x86_64::X86_64Backend;
use super::ir::FunctionIr;

#[derive(Clone, Debug)]
pub struct ModuleCode {
    pub code: Vec<u8>,
    pub functions: Vec<ModuleFunction>,
}

#[derive(Clone, Debug)]
pub struct ModuleFunction {
    pub index: u32,
    pub name: String,
    pub offset: u32,
    pub len: u32,
    pub sig: String,
    pub rendered_ir: String,
}

pub struct X86_64ModuleBackend;

impl X86_64ModuleBackend {
    pub fn compile_module(functions: &[FunctionIr]) -> Result<ModuleCode> {
        let mut code = Vec::new();
        let mut out_functions = Vec::new();
        let mut seen = BTreeMap::new();

        for ir in functions {
            if seen.insert(ir.index, code.len() as u32).is_some() {
                bail!("duplicate function index in AOT module: {}", ir.index);
            }

            let offset = code.len() as u32;
            let function_code = X86_64Backend::compile(ir)
                .with_context(|| format!("failed to compile {}", ir.name()))?;
            let len = function_code.len() as u32;
            code.extend_from_slice(&function_code);

            out_functions.push(ModuleFunction {
                index: ir.index,
                name: ir.name(),
                offset,
                len,
                sig: ir.sig.render(),
                rendered_ir: ir.render(),
            });
        }

        Ok(ModuleCode {
            code,
            functions: out_functions,
        })
    }
}
