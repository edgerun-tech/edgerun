use anyhow::{bail, Context, Result};
use crate::wasmparser_mock::wasmparser::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValueType {
    I32,
    I64,
}

impl ValueType {
    pub fn from_wasm(t: ValType) -> Result<Self> {
        match t {
            ValType::I32 => Ok(Self::I32),
            ValType::I64 => Ok(Self::I64),
            other => bail!("unsupported WASM value type for baseline AOT: {other:?}"),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::I32 => "i32",
            Self::I64 => "i64",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LoadKind {
    I32,
    I64,
    I32Load8U,
    I32Load8S,
    I32Load16U,
    I32Load16S,
}

impl LoadKind {
    pub fn result_type(self) -> ValueType {
        match self {
            Self::I32 | Self::I32Load8U | Self::I32Load8S | Self::I32Load16U | Self::I32Load16S => {
                ValueType::I32
            }
            Self::I64 => ValueType::I64,
        }
    }

    pub fn render(self) -> &'static str {
        match self {
            Self::I32 => "i32.load",
            Self::I64 => "i64.load",
            Self::I32Load8U => "i32.load8_u",
            Self::I32Load8S => "i32.load8_s",
            Self::I32Load16U => "i32.load16_u",
            Self::I32Load16S => "i32.load16_s",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StoreKind {
    I32,
    I64,
    I32Store8,
    I32Store16,
}

impl StoreKind {
    pub fn value_type(self) -> ValueType {
        match self {
            Self::I32 | Self::I32Store8 | Self::I32Store16 => ValueType::I32,
            Self::I64 => ValueType::I64,
        }
    }

    pub fn render(self) -> &'static str {
        match self {
            Self::I32 => "i32.store",
            Self::I64 => "i64.store",
            Self::I32Store8 => "i32.store8",
            Self::I32Store16 => "i32.store16",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MemOp {
    pub offset: u64,
}

#[derive(Clone, Debug)]
pub struct FuncSig {
    pub params: Vec<ValueType>,
    pub results: Vec<ValueType>,
}

impl FuncSig {
    pub fn from_wasm(sig: FuncType) -> Result<Self> {
        Ok(Self {
            params: sig
                .params()
                .iter()
                .copied()
                .map(ValueType::from_wasm)
                .collect::<Result<Vec<_>>>()?,
            results: sig
                .results()
                .iter()
                .copied()
                .map(ValueType::from_wasm)
                .collect::<Result<Vec<_>>>()?,
        })
    }

    pub fn render(&self) -> String {
        let params = self
            .params
            .iter()
            .map(|t| t.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        let results = self
            .results
            .iter()
            .map(|t| t.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        format!("({params}) -> ({results})")
    }
}

#[derive(Clone, Debug)]
pub enum IrOp {
    Nop,
    Block,
    Loop,
    BlockEnd,
    Br(u32),
    BrIf(u32),
    I32Const(i32),
    I64Const(i64),
    GlobalGet(u32, ValueType),
    GlobalSet(u32, ValueType),
    LocalGet(u32),
    LocalSet(u32),
    LocalTee(u32),
    Load(LoadKind, MemOp),
    Store(StoreKind, MemOp),
    MemoryCopy,
    MemoryFill,
    Call(u32, FuncSig),
    Select(ValueType),
    I32Add,
    I32Sub,
    I32Mul,
    I64Add,
    I64Sub,
    I64Mul,
    I32Eqz,
    I64Eqz,
    I32Eq,
    I32Ne,
    I32LtS,
    I32LtU,
    I32GtS,
    I32GtU,
    I32LeS,
    I32LeU,
    I32GeS,
    I32GeU,
    I64Eq,
    I64Ne,
    I64LtS,
    I64LtU,
    I64GtS,
    I64GtU,
    I64LeS,
    I64LeU,
    I64GeS,
    I64GeU,
    Return,
    End,
}

impl IrOp {
    pub fn render(&self) -> String {
        match self {
            Self::Nop => "nop".to_string(),
            Self::Block => "block".to_string(),
            Self::Loop => "loop".to_string(),
            Self::BlockEnd => "block.end".to_string(),
            Self::Br(depth) => format!("br {depth}"),
            Self::BrIf(depth) => format!("br_if {depth}"),
            Self::I32Const(v) => format!("i32.const {v}"),
            Self::I64Const(v) => format!("i64.const {v}"),
            Self::GlobalGet(i, _) => format!("global.get {i}"),
            Self::GlobalSet(i, _) => format!("global.set {i}"),
            Self::LocalGet(i) => format!("local.get {i}"),
            Self::LocalSet(i) => format!("local.set {i}"),
            Self::LocalTee(i) => format!("local.tee {i}"),
            Self::Load(kind, mem) => format!("{} offset={}", kind.render(), mem.offset),
            Self::Store(kind, mem) => format!("{} offset={}", kind.render(), mem.offset),
            Self::MemoryCopy => "memory.copy".to_string(),
            Self::MemoryFill => "memory.fill".to_string(),
            Self::Call(index, sig) => format!("call {index} {}", sig.render()),
            Self::Select(ty) => format!("select {}", ty.as_str()),
            Self::I32Add => "i32.add".to_string(),
            Self::I32Sub => "i32.sub".to_string(),
            Self::I32Mul => "i32.mul".to_string(),
            Self::I64Add => "i64.add".to_string(),
            Self::I64Sub => "i64.sub".to_string(),
            Self::I64Mul => "i64.mul".to_string(),
            Self::I32Eqz => "i32.eqz".to_string(),
            Self::I64Eqz => "i64.eqz".to_string(),
            Self::I32Eq => "i32.eq".to_string(),
            Self::I32Ne => "i32.ne".to_string(),
            Self::I32LtS => "i32.lt_s".to_string(),
            Self::I32LtU => "i32.lt_u".to_string(),
            Self::I32GtS => "i32.gt_s".to_string(),
            Self::I32GtU => "i32.gt_u".to_string(),
            Self::I32LeS => "i32.le_s".to_string(),
            Self::I32LeU => "i32.le_u".to_string(),
            Self::I32GeS => "i32.ge_s".to_string(),
            Self::I32GeU => "i32.ge_u".to_string(),
            Self::I64Eq => "i64.eq".to_string(),
            Self::I64Ne => "i64.ne".to_string(),
            Self::I64LtS => "i64.lt_s".to_string(),
            Self::I64LtU => "i64.lt_u".to_string(),
            Self::I64GtS => "i64.gt_s".to_string(),
            Self::I64GtU => "i64.gt_u".to_string(),
            Self::I64LeS => "i64.le_s".to_string(),
            Self::I64LeU => "i64.le_u".to_string(),
            Self::I64GeS => "i64.ge_s".to_string(),
            Self::I64GeU => "i64.ge_u".to_string(),
            Self::Return => "return".to_string(),
            Self::End => "end".to_string(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct FunctionIr {
    pub index: u32,
    pub export_name: Option<String>,
    pub sig: FuncSig,
    pub locals: Vec<ValueType>,
    pub global_values: Vec<GlobalValue>,
    pub uses_memory: bool,
    pub ops: Vec<IrOp>,
}

impl FunctionIr {
    pub fn name(&self) -> String {
        self.export_name
            .clone()
            .unwrap_or_else(|| format!("func{}", self.index))
    }

    pub fn render(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("func {} {}\n", self.name(), self.sig.render()));
        if self.uses_memory {
            out.push_str("  requires_memory\n");
        }
        if self.locals.len() > self.sig.params.len() {
            out.push_str("  locals");
            for local in self.locals.iter().skip(self.sig.params.len()) {
                out.push(' ');
                out.push_str(local.as_str());
            }
            out.push('\n');
        }
        for op in &self.ops {
            if matches!(op, IrOp::Nop) {
                continue;
            }
            out.push_str("  ");
            out.push_str(&op.render());
            out.push('\n');
        }
        out
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GlobalValue {
    I32(i32),
    I64(i64),
}

impl GlobalValue {
    pub fn value_type(self) -> ValueType {
        match self {
            Self::I32(_) => ValueType::I32,
            Self::I64(_) => ValueType::I64,
        }
    }
}

pub fn verify_ir(ir: &FunctionIr) -> Result<()> {
    if ir.sig.results.len() > 1 {
        bail!("{} returns more than one value", ir.name());
    }
    if ir.sig.params.len() > 6 {
        bail!(
            "{} has more than six integer params; SysV register ABI MVP only",
            ir.name()
        );
    }

    let mut stack: Vec<ValueType> = Vec::new();
    let mut ended = false;

    for op in &ir.ops {
        match *op {
            IrOp::Nop | IrOp::Block | IrOp::Loop | IrOp::BlockEnd => {}
            IrOp::Br(depth) => {
                if depth != 0 {
                    bail!("{} only supports br depth 0 for now", ir.name());
                }
            }
            IrOp::BrIf(depth) => {
                if depth != 0 {
                    bail!("{} only supports br_if depth 0 for now", ir.name());
                }
                pop1(&mut stack, ValueType::I32, ir, op)?;
            }
            IrOp::I32Const(_) => stack.push(ValueType::I32),
            IrOp::I64Const(_) => stack.push(ValueType::I64),
            IrOp::GlobalGet(index, ty) => {
                let global = global_type(ir, index)?;
                if global != ty {
                    bail!("{} global.get {} type mismatch", ir.name(), index);
                }
                stack.push(ty);
            }
            IrOp::GlobalSet(index, ty) => {
                let global = global_type(ir, index)?;
                if global != ty {
                    bail!("{} global.set {} type mismatch", ir.name(), index);
                }
                let actual = stack.pop().with_context(|| {
                    format!("{} stack underflow at global.set {index}", ir.name())
                })?;
                if actual != ty {
                    bail!(
                        "{} global.set {} value type mismatch: expected {}, found {}",
                        ir.name(),
                        index,
                        ty.as_str(),
                        actual.as_str()
                    );
                }
            }
            IrOp::LocalGet(i) => stack.push(local_type(ir, i)?),
            IrOp::LocalSet(i) => {
                let expected = local_type(ir, i)?;
                let actual = stack
                    .pop()
                    .with_context(|| format!("{} stack underflow at local.set {i}", ir.name()))?;
                if actual != expected {
                    bail!(
                        "{} local.set {} type mismatch: expected {}, found {}",
                        ir.name(),
                        i,
                        expected.as_str(),
                        actual.as_str()
                    );
                }
            }
            IrOp::LocalTee(i) => {
                let expected = local_type(ir, i)?;
                let actual = stack
                    .pop()
                    .with_context(|| format!("{} stack underflow at local.tee {i}", ir.name()))?;
                if actual != expected {
                    bail!(
                        "{} local.tee {} type mismatch: expected {}, found {}",
                        ir.name(),
                        i,
                        expected.as_str(),
                        actual.as_str()
                    );
                }
                stack.push(expected);
            }
            IrOp::Load(kind, _) => {
                pop1(&mut stack, ValueType::I32, ir, op)?;
                stack.push(kind.result_type());
            }
            IrOp::Store(kind, _) => {
                pop1(&mut stack, kind.value_type(), ir, op)?;
                pop1(&mut stack, ValueType::I32, ir, op)?;
            }
            IrOp::MemoryCopy | IrOp::MemoryFill => {
                pop1(&mut stack, ValueType::I32, ir, op)?;
                pop1(&mut stack, ValueType::I32, ir, op)?;
                pop1(&mut stack, ValueType::I32, ir, op)?;
            }
            IrOp::Call(_, ref sig) => {
                for expected in sig.params.iter().rev().copied() {
                    pop1(&mut stack, expected, ir, op)?;
                }
                if sig.results.len() > 1 {
                    bail!("{} call returns more than one value", ir.name());
                }
                if let Some(result) = sig.results.first().copied() {
                    stack.push(result);
                }
            }
            IrOp::Select(ty) => {
                pop1(&mut stack, ValueType::I32, ir, op)?;
                pop1(&mut stack, ty, ir, op)?;
                pop1(&mut stack, ty, ir, op)?;
                stack.push(ty);
            }
            IrOp::I32Add | IrOp::I32Sub | IrOp::I32Mul => {
                pop2(&mut stack, ValueType::I32, ir, op)?;
                stack.push(ValueType::I32);
            }
            IrOp::I64Add | IrOp::I64Sub | IrOp::I64Mul => {
                pop2(&mut stack, ValueType::I64, ir, op)?;
                stack.push(ValueType::I64);
            }
            IrOp::I32Eqz => {
                pop1(&mut stack, ValueType::I32, ir, op)?;
                stack.push(ValueType::I32);
            }
            IrOp::I64Eqz => {
                pop1(&mut stack, ValueType::I64, ir, op)?;
                stack.push(ValueType::I32);
            }
            IrOp::I32Eq
            | IrOp::I32Ne
            | IrOp::I32LtS
            | IrOp::I32LtU
            | IrOp::I32GtS
            | IrOp::I32GtU
            | IrOp::I32LeS
            | IrOp::I32LeU
            | IrOp::I32GeS
            | IrOp::I32GeU => {
                pop2(&mut stack, ValueType::I32, ir, op)?;
                stack.push(ValueType::I32);
            }
            IrOp::I64Eq
            | IrOp::I64Ne
            | IrOp::I64LtS
            | IrOp::I64LtU
            | IrOp::I64GtS
            | IrOp::I64GtU
            | IrOp::I64LeS
            | IrOp::I64LeU
            | IrOp::I64GeS
            | IrOp::I64GeU => {
                pop2(&mut stack, ValueType::I64, ir, op)?;
                stack.push(ValueType::I32);
            }
            IrOp::Return | IrOp::End => {
                if let Some(expected) = ir.sig.results.first().copied() {
                    let actual = stack
                        .pop()
                        .with_context(|| format!("{} ends without result", ir.name()))?;
                    if actual != expected {
                        bail!(
                            "{} result type mismatch: expected {}, found {}",
                            ir.name(),
                            expected.as_str(),
                            actual.as_str()
                        );
                    }
                }
                ended = true;
                break;
            }
        }
    }
    if !ended {
        bail!("{} has no return/end terminator", ir.name());
    }
    Ok(())
}

fn local_type(ir: &FunctionIr, index: u32) -> Result<ValueType> {
    ir.locals
        .get(index as usize)
        .copied()
        .with_context(|| format!("{} references missing local {index}", ir.name()))
}

fn global_type(ir: &FunctionIr, index: u32) -> Result<ValueType> {
    ir.global_values
        .get(index as usize)
        .copied()
        .map(GlobalValue::value_type)
        .with_context(|| format!("{} references missing global {index}", ir.name()))
}

fn pop1(stack: &mut Vec<ValueType>, expected: ValueType, ir: &FunctionIr, op: &IrOp) -> Result<()> {
    let actual = stack
        .pop()
        .with_context(|| format!("{} stack underflow at {}", ir.name(), op.render()))?;
    if actual != expected {
        bail!(
            "{} type mismatch at {}: expected {}, found {}",
            ir.name(),
            op.render(),
            expected.as_str(),
            actual.as_str()
        );
    }
    Ok(())
}

fn pop2(stack: &mut Vec<ValueType>, expected: ValueType, ir: &FunctionIr, op: &IrOp) -> Result<()> {
    let rhs = stack
        .pop()
        .with_context(|| format!("{} stack underflow at {}", ir.name(), op.render()))?;
    let lhs = stack
        .pop()
        .with_context(|| format!("{} stack underflow at {}", ir.name(), op.render()))?;
    if lhs != expected || rhs != expected {
        bail!(
            "{} type mismatch at {}: expected {} + {}, found {} + {}",
            ir.name(),
            op.render(),
            expected.as_str(),
            expected.as_str(),
            lhs.as_str(),
            rhs.as_str()
        );
    }
    Ok(())
}
