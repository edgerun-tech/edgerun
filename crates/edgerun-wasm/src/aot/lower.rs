use anyhow::{bail, Context, Result};
use std::collections::BTreeMap;
use wasmparser::{BlockType, ExternalKind, MemArg, Operator, Payload, TypeRef};

use super::ir::{verify_ir, FuncSig, FunctionIr, GlobalValue, IrOp, LoadKind, MemOp, StoreKind, ValueType};

#[derive(Debug)]
pub struct ParsedModule {
    types: Vec<FuncSig>,
    import_func_count: u32,
    func_type_indices: Vec<u32>,
    export_names: BTreeMap<u32, String>,
    globals: Vec<GlobalValue>,
    bodies: Vec<FunctionBody>,
}

#[derive(Debug)]
struct FunctionBody {
    func_index: u32,
    locals: Vec<ValueType>,
    uses_memory: bool,
    ops: Vec<IrOp>,
}

pub fn parse_module(wasm: &[u8]) -> Result<ParsedModule> {
    let mut types = Vec::new();
    let mut import_func_count = 0u32;
    let mut func_type_indices = Vec::new();
    let mut export_names = BTreeMap::new();
    let mut globals = Vec::new();
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
                        TypeRef::Global(_) => bail!(
                            "baseline AOT rejects imported globals: {}.{}",
                            import.module,
                            import.name
                        ),
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
            Payload::GlobalSection(section) => {
                for global in section {
                    let global = global?;
                    let ty = ValueType::from_wasm(global.ty.content_type)?;
                    let value = parse_global_init(ty, global.init_expr)?;
                    globals.push(value);
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

                let mut uses_memory = false;
                let mut ops = Vec::new();
                let mut ops_reader = body.get_operators_reader()?;
                while !ops_reader.eof() {
                    let op = ops_reader.read()?;
                    let lowered = lower_operator(op, &globals)?;
                    if matches!(lowered, IrOp::Load(_, _) | IrOp::Store(_, _)) {
                        uses_memory = true;
                    }
                    ops.push(lowered);
                }
                let ops = normalize_function_ends(ops)?;

                bodies.push(FunctionBody {
                    func_index,
                    locals,
                    uses_memory,
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
        globals,
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
            global_values: module.globals.clone(),
            uses_memory: body.uses_memory,
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

fn normalize_function_ends(mut ops: Vec<IrOp>) -> Result<Vec<IrOp>> {
    let last_real = ops
        .iter()
        .rposition(|op| !matches!(op, IrOp::Nop))
        .context("function body has no terminating end")?;
    for (idx, op) in ops.iter_mut().enumerate() {
        if matches!(op, IrOp::End) && idx != last_real {
            *op = IrOp::BlockEnd;
        }
    }
    if !matches!(ops[last_real], IrOp::End | IrOp::Return) {
        bail!("function body does not end with end/return");
    }
    Ok(ops)
}

fn parse_global_init(ty: ValueType, init_expr: wasmparser::ConstExpr<'_>) -> Result<GlobalValue> {
    let mut reader = init_expr.get_operators_reader();
    let first = reader.read()?;
    let value = match (ty, first) {
        (ValueType::I32, Operator::I32Const { value }) => GlobalValue::I32(value),
        (ValueType::I64, Operator::I64Const { value }) => GlobalValue::I64(value),
        (_, other) => bail!("unsupported baseline AOT global initializer: {other:?}"),
    };
    let end = reader.read()?;
    if !matches!(end, Operator::End) || !reader.eof() {
        bail!("unsupported multi-operator global initializer");
    }
    Ok(value)
}

fn mem_op(memarg: MemArg) -> Result<MemOp> {
    if memarg.memory != 0 {
        bail!("baseline AOT only supports memory index 0");
    }
    Ok(MemOp { offset: memarg.offset })
}

fn lower_operator(op: Operator<'_>, globals: &[GlobalValue]) -> Result<IrOp> {
    Ok(match op {
        Operator::Nop => IrOp::Nop,
        Operator::Block { blockty } => {
            if !matches!(blockty, BlockType::Empty) {
                bail!("baseline AOT only supports empty block types for now: {blockty:?}");
            }
            IrOp::Block
        }
        Operator::Loop { blockty } => {
            if !matches!(blockty, BlockType::Empty) {
                bail!("baseline AOT only supports empty loop types for now: {blockty:?}");
            }
            IrOp::Loop
        }
        Operator::Br { relative_depth } => IrOp::Br(relative_depth),
        Operator::BrIf { relative_depth } => IrOp::BrIf(relative_depth),
        Operator::I32Const { value } => IrOp::I32Const(value),
        Operator::I64Const { value } => IrOp::I64Const(value),
        Operator::GlobalGet { global_index } => {
            let global = globals
                .get(global_index as usize)
                .copied()
                .with_context(|| format!("global.get references missing global {global_index}"))?;
            IrOp::GlobalGet(global_index, global.value_type())
        }
        Operator::GlobalSet { global_index } => {
            let global = globals
                .get(global_index as usize)
                .copied()
                .with_context(|| format!("global.set references missing global {global_index}"))?;
            IrOp::GlobalSet(global_index, global.value_type())
        }
        Operator::LocalGet { local_index } => IrOp::LocalGet(local_index),
        Operator::LocalSet { local_index } => IrOp::LocalSet(local_index),
        Operator::LocalTee { local_index } => IrOp::LocalTee(local_index),
        Operator::I32Load { memarg } => IrOp::Load(LoadKind::I32, mem_op(memarg)?),
        Operator::I64Load { memarg } => IrOp::Load(LoadKind::I64, mem_op(memarg)?),
        Operator::I32Store { memarg } => IrOp::Store(StoreKind::I32, mem_op(memarg)?),
        Operator::I64Store { memarg } => IrOp::Store(StoreKind::I64, mem_op(memarg)?),
        Operator::I32Add => IrOp::I32Add,
        Operator::I32Sub => IrOp::I32Sub,
        Operator::I32Mul => IrOp::I32Mul,
        Operator::I64Add => IrOp::I64Add,
        Operator::I64Sub => IrOp::I64Sub,
        Operator::I64Mul => IrOp::I64Mul,
        Operator::I32Eqz => IrOp::I32Eqz,
        Operator::I64Eqz => IrOp::I64Eqz,
        Operator::I32Eq => IrOp::I32Eq,
        Operator::I32Ne => IrOp::I32Ne,
        Operator::I32LtS => IrOp::I32LtS,
        Operator::I32LtU => IrOp::I32LtU,
        Operator::I32GtS => IrOp::I32GtS,
        Operator::I32GtU => IrOp::I32GtU,
        Operator::I32LeS => IrOp::I32LeS,
        Operator::I32LeU => IrOp::I32LeU,
        Operator::I32GeS => IrOp::I32GeS,
        Operator::I32GeU => IrOp::I32GeU,
        Operator::I64Eq => IrOp::I64Eq,
        Operator::I64Ne => IrOp::I64Ne,
        Operator::I64LtS => IrOp::I64LtS,
        Operator::I64LtU => IrOp::I64LtU,
        Operator::I64GtS => IrOp::I64GtS,
        Operator::I64GtU => IrOp::I64GtU,
        Operator::I64LeS => IrOp::I64LeS,
        Operator::I64LeU => IrOp::I64LeU,
        Operator::I64GeS => IrOp::I64GeS,
        Operator::I64GeU => IrOp::I64GeU,
        Operator::Return => IrOp::Return,
        Operator::End => IrOp::End,
        other => bail!("unsupported baseline AOT operator: {other:?}"),
    })
}
