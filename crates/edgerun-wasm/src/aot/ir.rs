use anyhow::{bail, Context, Result};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValueType {
    I32,
    I64,
}

impl ValueType {
    pub fn from_wasm(t: wasmparser::ValType) -> Result<Self> {
        match t {
            wasmparser::ValType::I32 => Ok(Self::I32),
            wasmparser::ValType::I64 => Ok(Self::I64),
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

#[derive(Clone, Debug)]
pub struct FuncSig {
    pub params: Vec<ValueType>,
    pub results: Vec<ValueType>,
}

impl FuncSig {
    pub fn from_wasm(sig: wasmparser::FuncType) -> Result<Self> {
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
    I32Const(i32),
    I64Const(i64),
    LocalGet(u32),
    LocalSet(u32),
    LocalTee(u32),
    I32Add,
    I32Sub,
    I32Mul,
    I64Add,
    I64Sub,
    I64Mul,
    Return,
    End,
}

impl IrOp {
    pub fn render(&self) -> String {
        match self {
            Self::I32Const(v) => format!("i32.const {v}"),
            Self::I64Const(v) => format!("i64.const {v}"),
            Self::LocalGet(i) => format!("local.get {i}"),
            Self::LocalSet(i) => format!("local.set {i}"),
            Self::LocalTee(i) => format!("local.tee {i}"),
            Self::I32Add => "i32.add".to_string(),
            Self::I32Sub => "i32.sub".to_string(),
            Self::I32Mul => "i32.mul".to_string(),
            Self::I64Add => "i64.add".to_string(),
            Self::I64Sub => "i64.sub".to_string(),
            Self::I64Mul => "i64.mul".to_string(),
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
        if self.locals.len() > self.sig.params.len() {
            out.push_str("  locals");
            for local in self.locals.iter().skip(self.sig.params.len()) {
                out.push(' ');
                out.push_str(local.as_str());
            }
            out.push('\n');
        }
        for op in &self.ops {
            out.push_str("  ");
            out.push_str(&op.render());
            out.push('\n');
        }
        out
    }
}

pub fn verify_ir(ir: &FunctionIr) -> Result<()> {
    if ir.sig.results.len() > 1 {
        bail!("{} returns more than one value", ir.name());
    }
    if ir.sig.params.len() > 6 {
        bail!("{} has more than six integer params; SysV register ABI MVP only", ir.name());
    }

    let mut stack: Vec<ValueType> = Vec::new();
    let mut ended = false;

    for op in &ir.ops {
        match *op {
            IrOp::I32Const(_) => stack.push(ValueType::I32),
            IrOp::I64Const(_) => stack.push(ValueType::I64),
            IrOp::LocalGet(i) => {
                let ty = local_type(ir, i)?;
                stack.push(ty);
            }
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
            IrOp::I32Add | IrOp::I32Sub | IrOp::I32Mul => {
                pop2(&mut stack, ValueType::I32, ir, op)?;
                stack.push(ValueType::I32);
            }
            IrOp::I64Add | IrOp::I64Sub | IrOp::I64Mul => {
                pop2(&mut stack, ValueType::I64, ir, op)?;
                stack.push(ValueType::I64);
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
