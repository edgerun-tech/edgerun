use anyhow::{bail, Context, Result};
use edgerun_clap::Parser;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use wasmparser::{ExternalKind, Operator, Payload, TypeRef};

#[derive(Parser, Debug)]
#[command(name = "edgerun-aot", about = "Compile a small deterministic WASM subset into an EdgeRun AOT artifact")]
struct Args {
    #[arg(default_value = "app.wasm")]
    file: String,

    #[arg(short, long)]
    output: Option<String>,

    #[arg(long)]
    emit_ir: bool,

    #[arg(short, long)]
    verbose: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ValueType {
    I32,
    I64,
}

impl ValueType {
    fn from_wasm(t: wasmparser::ValType) -> Result<Self> {
        match t {
            wasmparser::ValType::I32 => Ok(Self::I32),
            wasmparser::ValType::I64 => Ok(Self::I64),
            other => bail!("unsupported WASM value type for baseline AOT: {other:?}"),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::I32 => "i32",
            Self::I64 => "i64",
        }
    }
}

#[derive(Clone, Debug)]
struct FuncSig {
    params: Vec<ValueType>,
    results: Vec<ValueType>,
}

impl FuncSig {
    fn from_wasm(sig: wasmparser::FuncType) -> Result<Self> {
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

    fn render(&self) -> String {
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
enum IrOp {
    I32Const(i32),
    I64Const(i64),
    LocalGet(u32),
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
    fn render(&self) -> String {
        match self {
            Self::I32Const(v) => format!("i32.const {v}"),
            Self::I64Const(v) => format!("i64.const {v}"),
            Self::LocalGet(i) => format!("local.get {i}"),
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
struct FunctionIr {
    index: u32,
    export_name: Option<String>,
    sig: FuncSig,
    locals: Vec<ValueType>,
    ops: Vec<IrOp>,
}

impl FunctionIr {
    fn name(&self) -> String {
        self.export_name
            .clone()
            .unwrap_or_else(|| format!("func{}", self.index))
    }

    fn render(&self) -> String {
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

#[derive(Clone, Debug)]
struct CompiledFunction {
    index: u32,
    name: String,
    ir: FunctionIr,
    code: Vec<u8>,
}

#[derive(Clone, Debug)]
struct AotArtifact {
    wasm_sha256: [u8; 32],
    target: &'static str,
    compiler: &'static str,
    functions: Vec<CompiledFunction>,
}

impl AotArtifact {
    fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"ERAOT001");
        put_u32(&mut out, 1);
        put_bytes(&mut out, self.target.as_bytes());
        put_bytes(&mut out, self.compiler.as_bytes());
        out.extend_from_slice(&self.wasm_sha256);
        put_u32(&mut out, self.functions.len() as u32);

        for f in &self.functions {
            put_u32(&mut out, f.index);
            put_bytes(&mut out, f.name.as_bytes());
            put_bytes(&mut out, f.ir.render().as_bytes());
            put_bytes(&mut out, &f.code);
        }

        out
    }
}

fn main() -> Result<()> {
    let args = Args::parse();
    let input_path = Path::new(&args.file);
    let wasm = std::fs::read(input_path).context("failed to read WASM input")?;

    wasmparser::Validator::new()
        .validate_all(&wasm)
        .context("invalid WASM module")?;

    let module = parse_module(&wasm)?;
    let functions = lower_module(module)?;

    if functions.is_empty() {
        bail!("no baseline-AOT-compilable functions found");
    }

    let mut compiled = Vec::new();
    for ir in functions {
        let code = X86_64Backend::compile(&ir)
            .with_context(|| format!("failed to compile {}", ir.name()))?;
        if args.verbose {
            eprintln!("compiled {}: {} bytes", ir.name(), code.len());
        }
        compiled.push(CompiledFunction {
            index: ir.index,
            name: ir.name(),
            ir,
            code,
        });
    }

    let wasm_sha256: [u8; 32] = Sha256::digest(&wasm).into();
    let artifact = AotArtifact {
        wasm_sha256,
        target: "x86_64-linux-sysv",
        compiler: "edgerun-aot-baseline-v0",
        functions: compiled,
    };

    if args.emit_ir {
        for f in &artifact.functions {
            println!("{}", f.ir.render());
        }
    }

    let output = args
        .output
        .map(PathBuf::from)
        .unwrap_or_else(|| input_path.with_extension("eraot"));
    std::fs::write(&output, artifact.encode()).context("failed to write AOT artifact")?;

    println!("PASS: wrote {}", output.display());
    Ok(())
}

#[derive(Debug)]
struct ParsedModule {
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

fn parse_module(wasm: &[u8]) -> Result<ParsedModule> {
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

fn lower_module(module: ParsedModule) -> Result<Vec<FunctionIr>> {
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
        // Imported functions are allowed to exist in the index space, but this
        // baseline compiler intentionally rejects call operators. Hostcall ABI
        // lowering belongs in the next step, not hidden in this first artifact.
    }

    Ok(functions)
}

fn verify_ir(ir: &FunctionIr) -> Result<()> {
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
                let ty = *ir
                    .locals
                    .get(i as usize)
                    .with_context(|| format!("{} reads missing local {i}", ir.name()))?;
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

struct X86_64Backend;

impl X86_64Backend {
    fn compile(ir: &FunctionIr) -> Result<Vec<u8>> {
        let mut code = Vec::new();
        let mut terminated = false;

        for op in &ir.ops {
            match *op {
                IrOp::I32Const(v) => emit_push_i32(&mut code, v),
                IrOp::I64Const(v) => emit_push_i64(&mut code, v),
                IrOp::LocalGet(i) => emit_push_arg(&mut code, i)?,
                IrOp::I32Add | IrOp::I64Add => emit_add(&mut code),
                IrOp::I32Sub | IrOp::I64Sub => emit_sub(&mut code),
                IrOp::I32Mul | IrOp::I64Mul => emit_mul(&mut code),
                IrOp::Return | IrOp::End => {
                    emit_return(&mut code, ir.sig.results.len());
                    terminated = true;
                    break;
                }
            }
        }

        if !terminated {
            emit_return(&mut code, ir.sig.results.len());
        }

        Ok(code)
    }
}

fn emit_push_arg(code: &mut Vec<u8>, index: u32) -> Result<()> {
    match index {
        0 => code.push(0x57), // push rdi
        1 => code.push(0x56), // push rsi
        2 => code.push(0x52), // push rdx
        3 => code.push(0x51), // push rcx
        4 => code.extend_from_slice(&[0x41, 0x50]), // push r8
        5 => code.extend_from_slice(&[0x41, 0x51]), // push r9
        _ => bail!("baseline backend only supports first six argument locals"),
    }
    Ok(())
}

fn emit_push_i32(code: &mut Vec<u8>, value: i32) {
    code.push(0x68); // push imm32, sign-extended by x86_64
    code.extend_from_slice(&value.to_le_bytes());
}

fn emit_push_i64(code: &mut Vec<u8>, value: i64) {
    code.extend_from_slice(&[0x48, 0xB8]); // mov rax, imm64
    code.extend_from_slice(&value.to_le_bytes());
    code.push(0x50); // push rax
}

fn emit_add(code: &mut Vec<u8>) {
    code.push(0x58); // pop rax = rhs
    code.push(0x59); // pop rcx = lhs
    code.extend_from_slice(&[0x48, 0x01, 0xC8]); // add rax, rcx
    code.push(0x50); // push rax
}

fn emit_sub(code: &mut Vec<u8>) {
    code.push(0x58); // pop rax = rhs
    code.push(0x59); // pop rcx = lhs
    code.extend_from_slice(&[0x48, 0x29, 0xC1]); // sub rcx, rax
    code.push(0x51); // push rcx
}

fn emit_mul(code: &mut Vec<u8>) {
    code.push(0x58); // pop rax = rhs
    code.push(0x59); // pop rcx = lhs
    code.extend_from_slice(&[0x48, 0x0F, 0xAF, 0xC1]); // imul rax, rcx
    code.push(0x50); // push rax
}

fn emit_return(code: &mut Vec<u8>, result_count: usize) {
    if result_count == 1 {
        code.push(0x58); // pop rax
    }
    code.push(0xC3); // ret
}

fn put_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn put_bytes(out: &mut Vec<u8>, bytes: &[u8]) {
    put_u32(out, bytes.len() as u32);
    out.extend_from_slice(bytes);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn x86_add_two_i32_params() {
        let ir = FunctionIr {
            index: 0,
            export_name: Some("add".to_string()),
            sig: FuncSig {
                params: vec![ValueType::I32, ValueType::I32],
                results: vec![ValueType::I32],
            },
            locals: vec![ValueType::I32, ValueType::I32],
            ops: vec![IrOp::LocalGet(0), IrOp::LocalGet(1), IrOp::I32Add, IrOp::End],
        };

        verify_ir(&ir).unwrap();
        let code = X86_64Backend::compile(&ir).unwrap();
        assert_eq!(code, vec![0x57, 0x56, 0x58, 0x59, 0x48, 0x01, 0xC8, 0x50, 0x58, 0xC3]);
    }

    #[test]
    fn artifact_has_stable_magic() {
        let artifact = AotArtifact {
            wasm_sha256: [7u8; 32],
            target: "x86_64-linux-sysv",
            compiler: "test",
            functions: Vec::new(),
        };
        assert!(artifact.encode().starts_with(b"ERAOT001"));
    }
}
