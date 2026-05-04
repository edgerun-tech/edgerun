use anyhow::{bail, Context, Result};
use std::collections::BTreeMap;
use wasmparser::{ExternalKind, Operator, Payload, TypeRef};

use super::ir::{verify_ir, FuncSig, FunctionIr, IrOp, ValueType};

#[derive(Debug)]
pub struct ParsedModule {
    types: Vec<FuncSig>,
    import_func_count: u32,
    func_type_indices: Vec<u32>,
    export_names: BTreeMap<u32, String>,
    bodies: Vec<FunctionBody>,
}

#[derive(Debug)]
struct FunctionBody {
    func_index: u32,
    locals: Vec<ValueType>,
    ops: Vec<IrOp>,
}

pub fn parse_module(wasm: &[u8]) -> Result<ParsedModule> {
    let mut types = Vec::new();
    let mut import_func_count = 0u32;
    let mut func_type_indices = Vec::new();
    let mut export_names = BTreeMap::new();
    let mut defined_func_type_indices = Vec::new();
    let mut bodies = Vec::new();

    for payload in wasmparser::Parser::new(0).parse_all(wasm) {
        match payload? {
            Payload::TypeSection(section) => {
                for group in section {
                    let group = group?;
                    for sub in group.types() {
                        let func_ty = sub.composite_type.unwrap_func().clone();
                        types.push(FuncSig::from_wasm(func_ty)?);
                    }
                }
            }
            Payload::ImportSection(section) => {
                for import in section {
                    let import = import?;
                    match import.ty {
                        TypeRef::Func(type_idx) => {
                            import_func_count += 1;
                            func_type_indices.push(type_idx);
                        }
                        _ => bail!(
                            "baseline AOT rejects non-function imports: {}.{}",
                            import.module,
                            import.name
                        ),
                    }
                }
            }
            Payload::FunctionSection(section) => {
                for ty in section {
                    let type_idx = ty?;
                    func_type_indices.push(type_idx);
                    defined_func_type_indices.push(type_idx);
                }
            }
            Payload::ExportSection(section) => {
                for export in section {
                    let export = export?;
                    if export.kind == ExternalKind::Func {
                        export_names.insert(export.index, export.name.to_string());
                    }
                }
            }
            Payload::CodeSectionEntry(body) => {
                let body_idx = bodies.len() as u32;
                let func_index = import_func_count + body_idx;
                let mut locals = Vec::new();
                let mut locals_reader = body.get_locals_reader()?;
                for _ in 0..locals_reader.get_count() {
                    let (count, ty) = locals_reader.read()?;
                    let ty = ValueType::from_wasm(ty)?;
                    for _ in 0..count {
                        locals.push(ty);
                    }
                }

                let mut ops = Vec::new();
                let mut ops_reader = body.get_operators_reader()?;
                while !ops_reader.eof() {
                    let op = ops_reader.read()?;
                    ops.push(lower_operator(op)?);
                }

                bodies.push(FunctionBody {
                    func_index,
                    locals,
                    ops,
                });
            }
            _ => {}
        }
    }

    if bodies.len() != defined_func_type_indices.len() {
        bail!(
            "function/code section mismatch: {} bodies for {} declarations",
            bodies.len(),
            defined_func_type_indices.len()
        );
    }

    Ok(ParsedModule {
        types,
        import_func_count,
        func_type_indices,
        export_names,
        bodies,
    })
}

pub fn lower_module(module: ParsedModule) -> Result<Vec<FunctionIr>> {
    let mut functions = Vec::new();

    for body in module.bodies {
        let type_idx = *module
            .func_type_indices
            .get(body.func_index as usize)
            .with_context(|| format!("missing type index for function {}", body.func_index))?;
        let sig = module
            .types
            .get(type_idx as usize)
            .with_context(|| format!("missing function type {type_idx}"))?
            .clone();

        let mut locals = sig.params.clone();
        locals.extend(body.locals);

        let ir = FunctionIr {
            index: body.func_index,
            export_name: module.export_names.get(&body.func_index).cloned(),
            sig,
            locals,
            ops: body.ops,
        };

        verify_ir(&ir)?;
        functions.push(ir);
    }

    if module.import_func_count > 0 {
        // Imported functions are allowed in the function index space, but this
        // first compiler intentionally rejects call operators. Hostcall ABI
        // lowering should be explicit in the next step.
    }

    Ok(functions)
}

fn lower_operator(op: Operator<'_>) -> Result<IrOp> {
    Ok(match op {
        Operator::I32Const { value } => IrOp::I32Const(value),
        Operator::I64Const { value } => IrOp::I64Const(value),
        Operator::LocalGet { local_index } => IrOp::LocalGet(local_index),
        Operator::I32Add => IrOp::I32Add,
        Operator::I32Sub => IrOp::I32Sub,
        Operator::I32Mul => IrOp::I32Mul,
        Operator::I64Add => IrOp::I64Add,
        Operator::I64Sub => IrOp::I64Sub,
        Operator::I64Mul => IrOp::I64Mul,
        Operator::Return => IrOp::Return,
        Operator::End => IrOp::End,
        other => bail!("unsupported baseline AOT operator: {other:?}"),
    })
}
