use anyhow::{bail, Context, Result};
use std::collections::BTreeMap;
use crate::wasmparser_mock::wasmparser::*;
use super::ir::{
    verify_ir, FuncSig, FunctionIr, GlobalValue, IrOp, LoadKind, MemOp, StoreKind, ValueType,
};

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

    for payload in wasmparser_mock::Parser::new(0).parse_all(wasm) {
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

                let all_locals = current_locals(&types, &func_type_indices, func_index, &locals)?;
                let sigs = function_sigs(&types, &func_type_indices)?;

                let mut uses_memory = false;
                let mut ops = Vec::new();
                let mut type_stack = Vec::new();
                let mut ops_reader = body.get_operators_reader()?;
                while !ops_reader.eof() {
                    let op = ops_reader.read()?;
                    let lowered = lower_operator(
                        op,
                        &globals,
                        &all_locals,
                        &sigs,
                        import_func_count,
                        &mut type_stack,
                    )?;
                    if matches!(
                        lowered,
                        IrOp::Load(_, _) | IrOp::Store(_, _) | IrOp::MemoryCopy | IrOp::MemoryFill
                    ) {
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

    Ok(functions)
}

fn current_locals(
    types: &[FuncSig],
    func_type_indices: &[u32],
    func_index: u32,
    body_locals: &[ValueType],
) -> Result<Vec<ValueType>> {
    let type_idx = *func_type_indices
        .get(func_index as usize)
        .with_context(|| format!("missing type index for function {func_index}"))?;
    let sig = types
        .get(type_idx as usize)
        .with_context(|| format!("missing function type {type_idx}"))?;
    let mut locals = sig.params.clone();
    locals.extend_from_slice(body_locals);
    Ok(locals)
}

fn function_sigs(types: &[FuncSig], func_type_indices: &[u32]) -> Result<Vec<FuncSig>> {
    func_type_indices
        .iter()
        .map(|idx| {
            types
                .get(*idx as usize)
                .cloned()
                .with_context(|| format!("missing function type {idx}"))
        })
        .collect()
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

fn parse_global_init(ty: ValueType, init_expr: wasmparser_mock::ConstExpr<'_>) -> Result<GlobalValue> {
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
    Ok(MemOp {
        offset: memarg.offset,
    })
}

fn lower_operator(
    op: Operator<'_>,
    globals: &[GlobalValue],
    locals: &[ValueType],
    function_sigs: &[FuncSig],
    import_func_count: u32,
    stack: &mut Vec<ValueType>,
) -> Result<IrOp> {
    let lowered = match op {
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
        Operator::BrIf { relative_depth } => {
            pop_ty(stack, ValueType::I32, "br_if")?;
            IrOp::BrIf(relative_depth)
        }
        Operator::I32Const { value } => {
            stack.push(ValueType::I32);
            IrOp::I32Const(value)
        }
        Operator::I64Const { value } => {
            stack.push(ValueType::I64);
            IrOp::I64Const(value)
        }
        Operator::GlobalGet { global_index } => {
            let global = globals
                .get(global_index as usize)
                .copied()
                .with_context(|| format!("global.get references missing global {global_index}"))?;
            stack.push(global.value_type());
            IrOp::GlobalGet(global_index, global.value_type())
        }
        Operator::GlobalSet { global_index } => {
            let global = globals
                .get(global_index as usize)
                .copied()
                .with_context(|| format!("global.set references missing global {global_index}"))?;
            pop_ty(stack, global.value_type(), "global.set")?;
            IrOp::GlobalSet(global_index, global.value_type())
        }
        Operator::LocalGet { local_index } => {
            let ty = *locals
                .get(local_index as usize)
                .with_context(|| format!("local.get references missing local {local_index}"))?;
            stack.push(ty);
            IrOp::LocalGet(local_index)
        }
        Operator::LocalSet { local_index } => {
            let ty = *locals
                .get(local_index as usize)
                .with_context(|| format!("local.set references missing local {local_index}"))?;
            pop_ty(stack, ty, "local.set")?;
            IrOp::LocalSet(local_index)
        }
        Operator::LocalTee { local_index } => {
            let ty = *locals
                .get(local_index as usize)
                .with_context(|| format!("local.tee references missing local {local_index}"))?;
            pop_ty(stack, ty, "local.tee")?;
            stack.push(ty);
            IrOp::LocalTee(local_index)
        }
        Operator::I32Load { memarg } => {
            pop_ty(stack, ValueType::I32, "i32.load address")?;
            stack.push(ValueType::I32);
            IrOp::Load(LoadKind::I32, mem_op(memarg)?)
        }
        Operator::I64Load { memarg } => {
            pop_ty(stack, ValueType::I32, "i64.load address")?;
            stack.push(ValueType::I64);
            IrOp::Load(LoadKind::I64, mem_op(memarg)?)
        }
        Operator::I32Load8U { memarg } => {
            pop_ty(stack, ValueType::I32, "i32.load8_u address")?;
            stack.push(ValueType::I32);
            IrOp::Load(LoadKind::I32Load8U, mem_op(memarg)?)
        }
        Operator::I32Load8S { memarg } => {
            pop_ty(stack, ValueType::I32, "i32.load8_s address")?;
            stack.push(ValueType::I32);
            IrOp::Load(LoadKind::I32Load8S, mem_op(memarg)?)
        }
        Operator::I32Load16U { memarg } => {
            pop_ty(stack, ValueType::I32, "i32.load16_u address")?;
            stack.push(ValueType::I32);
            IrOp::Load(LoadKind::I32Load16U, mem_op(memarg)?)
        }
        Operator::I32Load16S { memarg } => {
            pop_ty(stack, ValueType::I32, "i32.load16_s address")?;
            stack.push(ValueType::I32);
            IrOp::Load(LoadKind::I32Load16S, mem_op(memarg)?)
        }
        Operator::I32Store { memarg } => {
            pop_ty(stack, ValueType::I32, "i32.store value")?;
            pop_ty(stack, ValueType::I32, "i32.store address")?;
            IrOp::Store(StoreKind::I32, mem_op(memarg)?)
        }
        Operator::I64Store { memarg } => {
            pop_ty(stack, ValueType::I64, "i64.store value")?;
            pop_ty(stack, ValueType::I32, "i64.store address")?;
            IrOp::Store(StoreKind::I64, mem_op(memarg)?)
        }
        Operator::I32Store8 { memarg } => {
            pop_ty(stack, ValueType::I32, "i32.store8 value")?;
            pop_ty(stack, ValueType::I32, "i32.store8 address")?;
            IrOp::Store(StoreKind::I32Store8, mem_op(memarg)?)
        }
        Operator::I32Store16 { memarg } => {
            pop_ty(stack, ValueType::I32, "i32.store16 value")?;
            pop_ty(stack, ValueType::I32, "i32.store16 address")?;
            IrOp::Store(StoreKind::I32Store16, mem_op(memarg)?)
        }
        Operator::MemoryCopy { dst_mem, src_mem } => {
            if dst_mem != 0 || src_mem != 0 {
                bail!("baseline AOT only supports memory.copy within memory 0");
            }
            pop_ty(stack, ValueType::I32, "memory.copy length")?;
            pop_ty(stack, ValueType::I32, "memory.copy source")?;
            pop_ty(stack, ValueType::I32, "memory.copy destination")?;
            IrOp::MemoryCopy
        }
        Operator::MemoryFill { mem } => {
            if mem != 0 {
                bail!("baseline AOT only supports memory.fill within memory 0");
            }
            pop_ty(stack, ValueType::I32, "memory.fill length")?;
            pop_ty(stack, ValueType::I32, "memory.fill value")?;
            pop_ty(stack, ValueType::I32, "memory.fill destination")?;
            IrOp::MemoryFill
        }
        Operator::Call { function_index } => {
            if function_index < import_func_count {
                bail!(
                    "baseline AOT does not support imported function calls yet: {function_index}"
                );
            }
            let sig = function_sigs
                .get(function_index as usize)
                .cloned()
                .with_context(|| format!("call references missing function {function_index}"))?;
            for expected in sig.params.iter().rev().copied() {
                pop_ty(stack, expected, "call argument")?;
            }
            if sig.results.len() > 1 {
                bail!("baseline AOT does not support multi-value call returns");
            }
            if let Some(result) = sig.results.first().copied() {
                stack.push(result);
            }
            IrOp::Call(function_index, sig)
        }
        Operator::Select => {
            pop_ty(stack, ValueType::I32, "select condition")?;
            let false_ty = pop_any(stack, "select false value")?;
            let true_ty = pop_any(stack, "select true value")?;
            if true_ty != false_ty {
                bail!(
                    "select value type mismatch: true={} false={}",
                    true_ty.as_str(),
                    false_ty.as_str()
                );
            }
            stack.push(true_ty);
            IrOp::Select(true_ty)
        }
        Operator::I32Add => bin(stack, ValueType::I32, ValueType::I32, IrOp::I32Add)?,
        Operator::I32Sub => bin(stack, ValueType::I32, ValueType::I32, IrOp::I32Sub)?,
        Operator::I32Mul => bin(stack, ValueType::I32, ValueType::I32, IrOp::I32Mul)?,
        Operator::I64Add => bin(stack, ValueType::I64, ValueType::I64, IrOp::I64Add)?,
        Operator::I64Sub => bin(stack, ValueType::I64, ValueType::I64, IrOp::I64Sub)?,
        Operator::I64Mul => bin(stack, ValueType::I64, ValueType::I64, IrOp::I64Mul)?,
        Operator::I32Eqz => unary(stack, ValueType::I32, ValueType::I32, IrOp::I32Eqz)?,
        Operator::I64Eqz => unary(stack, ValueType::I64, ValueType::I32, IrOp::I64Eqz)?,
        Operator::I32Eq => bin(stack, ValueType::I32, ValueType::I32, IrOp::I32Eq)?,
        Operator::I32Ne => bin(stack, ValueType::I32, ValueType::I32, IrOp::I32Ne)?,
        Operator::I32LtS => bin(stack, ValueType::I32, ValueType::I32, IrOp::I32LtS)?,
        Operator::I32LtU => bin(stack, ValueType::I32, ValueType::I32, IrOp::I32LtU)?,
        Operator::I32GtS => bin(stack, ValueType::I32, ValueType::I32, IrOp::I32GtS)?,
        Operator::I32GtU => bin(stack, ValueType::I32, ValueType::I32, IrOp::I32GtU)?,
        Operator::I32LeS => bin(stack, ValueType::I32, ValueType::I32, IrOp::I32LeS)?,
        Operator::I32LeU => bin(stack, ValueType::I32, ValueType::I32, IrOp::I32LeU)?,
        Operator::I32GeS => bin(stack, ValueType::I32, ValueType::I32, IrOp::I32GeS)?,
        Operator::I32GeU => bin(stack, ValueType::I32, ValueType::I32, IrOp::I32GeU)?,
        Operator::I64Eq => bin(stack, ValueType::I64, ValueType::I32, IrOp::I64Eq)?,
        Operator::I64Ne => bin(stack, ValueType::I64, ValueType::I32, IrOp::I64Ne)?,
        Operator::I64LtS => bin(stack, ValueType::I64, ValueType::I32, IrOp::I64LtS)?,
        Operator::I64LtU => bin(stack, ValueType::I64, ValueType::I32, IrOp::I64LtU)?,
        Operator::I64GtS => bin(stack, ValueType::I64, ValueType::I32, IrOp::I64GtS)?,
        Operator::I64GtU => bin(stack, ValueType::I64, ValueType::I32, IrOp::I64GtU)?,
        Operator::I64LeS => bin(stack, ValueType::I64, ValueType::I32, IrOp::I64LeS)?,
        Operator::I64LeU => bin(stack, ValueType::I64, ValueType::I32, IrOp::I64LeU)?,
        Operator::I64GeS => bin(stack, ValueType::I64, ValueType::I32, IrOp::I64GeS)?,
        Operator::I64GeU => bin(stack, ValueType::I64, ValueType::I32, IrOp::I64GeU)?,
        Operator::Return => IrOp::Return,
        Operator::End => IrOp::End,
        other => bail!("unsupported baseline AOT operator: {other:?}"),
    };
    Ok(lowered)
}

fn pop_any(stack: &mut Vec<ValueType>, label: &str) -> Result<ValueType> {
    stack
        .pop()
        .with_context(|| format!("stack underflow at {label}"))
}

fn pop_ty(stack: &mut Vec<ValueType>, expected: ValueType, label: &str) -> Result<()> {
    let actual = pop_any(stack, label)?;
    if actual != expected {
        bail!(
            "type mismatch at {label}: expected {}, found {}",
            expected.as_str(),
            actual.as_str()
        );
    }
    Ok(())
}

fn unary(
    stack: &mut Vec<ValueType>,
    input: ValueType,
    output: ValueType,
    op: IrOp,
) -> Result<IrOp> {
    pop_ty(stack, input, &op.render())?;
    stack.push(output);
    Ok(op)
}

fn bin(stack: &mut Vec<ValueType>, input: ValueType, output: ValueType, op: IrOp) -> Result<IrOp> {
    pop_ty(stack, input, &op.render())?;
    pop_ty(stack, input, &op.render())?;
    stack.push(output);
    Ok(op)
}
