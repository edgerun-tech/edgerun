use anyhow::{bail, Context, Result};
use std::collections::BTreeMap;

use super::backend_x86_64::X86_64Backend;
use super::ir::{FunctionIr, IrOp, ValueType};

const MAX_INLINE_DEPTH: usize = 32;

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
        let function_map = build_function_map(functions)?;
        let mut code = Vec::new();
        let mut out_functions = Vec::new();
        let mut seen = BTreeMap::new();

        for ir in functions {
            if seen.insert(ir.index, code.len() as u32).is_some() {
                bail!("duplicate function index in AOT module: {}", ir.index);
            }

            let expanded = inline_calls(ir, &function_map, &mut Vec::new(), 0)
                .with_context(|| format!("failed to inline calls for {}", ir.name()))?;

            let offset = code.len() as u32;
            let function_code = X86_64Backend::compile(&expanded)
                .with_context(|| format!("failed to compile {}", expanded.name()))?;
            let len = function_code.len() as u32;
            code.extend_from_slice(&function_code);

            out_functions.push(ModuleFunction {
                index: ir.index,
                name: ir.name(),
                offset,
                len,
                sig: ir.sig.render(),
                rendered_ir: expanded.render(),
            });
        }

        Ok(ModuleCode {
            code,
            functions: out_functions,
        })
    }
}

fn build_function_map(functions: &[FunctionIr]) -> Result<BTreeMap<u32, FunctionIr>> {
    let mut map = BTreeMap::new();
    for function in functions {
        if map.insert(function.index, function.clone()).is_some() {
            bail!("duplicate function index in AOT module: {}", function.index);
        }
    }
    Ok(map)
}

fn inline_calls(
    ir: &FunctionIr,
    functions: &BTreeMap<u32, FunctionIr>,
    stack: &mut Vec<u32>,
    depth: usize,
) -> Result<FunctionIr> {
    if depth > MAX_INLINE_DEPTH {
        bail!("AOT direct-call inlining exceeded depth limit {MAX_INLINE_DEPTH}");
    }
    if stack.contains(&ir.index) {
        bail!("recursive AOT direct call involving function {}", ir.index);
    }

    stack.push(ir.index);
    let mut out = ir.clone();
    out.ops.clear();

    for op in &ir.ops {
        match op {
            IrOp::Call(target, sig) => {
                if sig.results.len() > 1 {
                    bail!("AOT direct call returns more than one value: {target}");
                }
                let callee = functions
                    .get(target)
                    .with_context(|| format!("AOT direct call target not found: {target}"))?;
                if callee.sig.params != sig.params || callee.sig.results != sig.results {
                    bail!("AOT direct call signature mismatch for function {target}");
                }
                inline_callee_call(&mut out, callee, functions, stack, depth + 1)?;
            }
            other => out.ops.push(other.clone()),
        }
    }

    stack.pop();
    Ok(out)
}

fn inline_callee_call(
    caller: &mut FunctionIr,
    callee: &FunctionIr,
    functions: &BTreeMap<u32, FunctionIr>,
    stack: &mut Vec<u32>,
    depth: usize,
) -> Result<()> {
    let expanded_callee = inline_calls(callee, functions, stack, depth)?;
    if expanded_callee.sig.results.len() > 1 {
        bail!("AOT cannot inline multi-value return from {}", callee.name());
    }

    let local_base = caller.locals.len() as u32;
    caller.locals.extend(expanded_callee.locals.iter().copied());

    // The caller's WASM operand stack contains arguments in call order. Store
    // them into the inlined callee parameter locals in reverse pop order.
    for param_index in (0..expanded_callee.sig.params.len()).rev() {
        caller
            .ops
            .push(IrOp::LocalSet(local_base + param_index as u32));
    }

    for op in expanded_callee.ops {
        match op {
            IrOp::End => {}
            IrOp::Return => {
                bail!("AOT cannot inline explicit return from {} yet", callee.name());
            }
            other => caller.ops.push(remap_locals(other, local_base)?),
        }
    }

    Ok(())
}

fn remap_locals(op: IrOp, local_base: u32) -> Result<IrOp> {
    Ok(match op {
        IrOp::LocalGet(i) => IrOp::LocalGet(local_base + i),
        IrOp::LocalSet(i) => IrOp::LocalSet(local_base + i),
        IrOp::LocalTee(i) => IrOp::LocalTee(local_base + i),
        IrOp::Call(_, _) => bail!("nested call survived AOT inline expansion"),
        other => other,
    })
}

#[cfg(test)]
mod tests {
    use super::super::ir::{FuncSig, GlobalValue};
    use super::*;

    fn function(
        index: u32,
        params: Vec<ValueType>,
        results: Vec<ValueType>,
        ops: Vec<IrOp>,
    ) -> FunctionIr {
        let locals = params.clone();
        FunctionIr {
            index,
            export_name: Some(format!("f{index}")),
            sig: FuncSig { params, results },
            locals,
            global_values: Vec::<GlobalValue>::new(),
            uses_memory: false,
            ops,
        }
    }

    #[test]
    fn module_backend_inlines_direct_i32_call() {
        let callee = function(
            1,
            vec![ValueType::I32, ValueType::I32],
            vec![ValueType::I32],
            vec![
                IrOp::LocalGet(0),
                IrOp::LocalGet(1),
                IrOp::I32Add,
                IrOp::End,
            ],
        );
        let caller = function(
            0,
            vec![ValueType::I32, ValueType::I32],
            vec![ValueType::I32],
            vec![
                IrOp::LocalGet(0),
                IrOp::LocalGet(1),
                IrOp::Call(1, callee.sig.clone()),
                IrOp::End,
            ],
        );

        let module = X86_64ModuleBackend::compile_module(&[caller, callee]).unwrap();
        assert_eq!(module.functions.len(), 2);
        assert!(!module.code.is_empty());
    }
}
