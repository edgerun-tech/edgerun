use anyhow::{bail, Result};

use super::ir::{FunctionIr, GlobalValue, IrOp, ValueType};

pub struct X86_64Backend;

impl X86_64Backend {
    pub fn compile(ir: &FunctionIr) -> Result<Vec<u8>> {
        let mut code = Vec::new();
        let frame = Frame::new(ir)?;
        frame.emit_prologue(&mut code, ir)?;

        let mut terminated = false;
        for op in &ir.ops {
            match *op {
                IrOp::I32Const(v) => emit_push_i32(&mut code, v),
                IrOp::I64Const(v) => emit_push_i64(&mut code, v),
                IrOp::GlobalGet(i, _) => emit_global_get(&mut code, ir, i)?,
                IrOp::LocalGet(i) => frame.emit_local_get(&mut code, i, local_type(ir, i)?)?,
                IrOp::LocalSet(i) => frame.emit_local_set(&mut code, i, local_type(ir, i)?)?,
                IrOp::LocalTee(i) => frame.emit_local_tee(&mut code, i, local_type(ir, i)?)?,
                IrOp::I32Add => emit_i32_add(&mut code),
                IrOp::I32Sub => emit_i32_sub(&mut code),
                IrOp::I32Mul => emit_i32_mul(&mut code),
                IrOp::I64Add => emit_i64_add(&mut code),
                IrOp::I64Sub => emit_i64_sub(&mut code),
                IrOp::I64Mul => emit_i64_mul(&mut code),
                IrOp::I32Eqz => emit_i32_eqz(&mut code),
                IrOp::I64Eqz => emit_i64_eqz(&mut code),
                IrOp::I32Eq => emit_i32_cmp(&mut code, SetCc::Eq),
                IrOp::I32Ne => emit_i32_cmp(&mut code, SetCc::Ne),
                IrOp::I32LtS => emit_i32_cmp(&mut code, SetCc::LtS),
                IrOp::I32LtU => emit_i32_cmp(&mut code, SetCc::LtU),
                IrOp::I32GtS => emit_i32_cmp(&mut code, SetCc::GtS),
                IrOp::I32GtU => emit_i32_cmp(&mut code, SetCc::GtU),
                IrOp::I32LeS => emit_i32_cmp(&mut code, SetCc::LeS),
                IrOp::I32LeU => emit_i32_cmp(&mut code, SetCc::LeU),
                IrOp::I32GeS => emit_i32_cmp(&mut code, SetCc::GeS),
                IrOp::I32GeU => emit_i32_cmp(&mut code, SetCc::GeU),
                IrOp::I64Eq => emit_i64_cmp(&mut code, SetCc::Eq),
                IrOp::I64Ne => emit_i64_cmp(&mut code, SetCc::Ne),
                IrOp::I64LtS => emit_i64_cmp(&mut code, SetCc::LtS),
                IrOp::I64LtU => emit_i64_cmp(&mut code, SetCc::LtU),
                IrOp::I64GtS => emit_i64_cmp(&mut code, SetCc::GtS),
                IrOp::I64GtU => emit_i64_cmp(&mut code, SetCc::GtU),
                IrOp::I64LeS => emit_i64_cmp(&mut code, SetCc::LeS),
                IrOp::I64LeU => emit_i64_cmp(&mut code, SetCc::LeU),
                IrOp::I64GeS => emit_i64_cmp(&mut code, SetCc::GeS),
                IrOp::I64GeU => emit_i64_cmp(&mut code, SetCc::GeU),
                IrOp::Return | IrOp::End => {
                    emit_return(&mut code, ir.sig.results.first().copied());
                    terminated = true;
                    break;
                }
            }
        }

        if !terminated {
            emit_return(&mut code, ir.sig.results.first().copied());
        }

        Ok(code)
    }
}

struct Frame {
    slots: u32,
}

impl Frame {
    fn new(ir: &FunctionIr) -> Result<Self> {
        let slots = ir.locals.len() as u32;
        if slots > 4096 {
            bail!("baseline backend refuses huge local frame: {slots} slots");
        }
        Ok(Self { slots })
    }

    fn emit_prologue(&self, code: &mut Vec<u8>, ir: &FunctionIr) -> Result<()> {
        code.push(0x55); // push rbp
        code.extend_from_slice(&[0x48, 0x89, 0xE5]); // mov rbp, rsp

        let frame_bytes = self.slots * 8;
        if frame_bytes > 0 {
            code.extend_from_slice(&[0x48, 0x81, 0xEC]); // sub rsp, imm32
            code.extend_from_slice(&(frame_bytes as i32).to_le_bytes());
        }

        for i in 0..ir.sig.params.len() as u32 {
            self.emit_store_arg(code, i, local_type(ir, i)?)?;
        }
        for i in ir.sig.params.len() as u32..self.slots {
            self.emit_zero_slot(code, i)?;
        }

        Ok(())
    }

    fn emit_store_arg(&self, code: &mut Vec<u8>, index: u32, ty: ValueType) -> Result<()> {
        self.emit_arg_to_rax(code, index)?;
        if ty == ValueType::I32 {
            emit_zero_extend_eax(code);
        }
        self.emit_store_rax_to_slot(code, index)
    }

    fn emit_arg_to_rax(&self, code: &mut Vec<u8>, index: u32) -> Result<()> {
        match index {
            0 => code.extend_from_slice(&[0x48, 0x89, 0xF8]), // mov rax, rdi
            1 => code.extend_from_slice(&[0x48, 0x89, 0xF0]), // mov rax, rsi
            2 => code.extend_from_slice(&[0x48, 0x89, 0xD0]), // mov rax, rdx
            3 => code.extend_from_slice(&[0x48, 0x89, 0xC8]), // mov rax, rcx
            4 => code.extend_from_slice(&[0x4C, 0x89, 0xC0]), // mov rax, r8
            5 => code.extend_from_slice(&[0x4C, 0x89, 0xC8]), // mov rax, r9
            _ => bail!("baseline backend only supports first six argument locals"),
        }
        Ok(())
    }

    fn emit_zero_slot(&self, code: &mut Vec<u8>, index: u32) -> Result<()> {
        let disp = self.slot_disp(index)?;
        code.extend_from_slice(&[0x48, 0xC7, 0x85]); // mov qword [rbp + disp32], imm32
        code.extend_from_slice(&disp.to_le_bytes());
        code.extend_from_slice(&0i32.to_le_bytes());
        Ok(())
    }

    fn emit_local_get(&self, code: &mut Vec<u8>, index: u32, ty: ValueType) -> Result<()> {
        self.emit_load_slot_to_rax(code, index)?;
        if ty == ValueType::I32 {
            emit_zero_extend_eax(code);
        }
        code.push(0x50); // push rax
        Ok(())
    }

    fn emit_local_set(&self, code: &mut Vec<u8>, index: u32, ty: ValueType) -> Result<()> {
        code.push(0x58); // pop rax
        if ty == ValueType::I32 {
            emit_zero_extend_eax(code);
        }
        self.emit_store_rax_to_slot(code, index)
    }

    fn emit_local_tee(&self, code: &mut Vec<u8>, index: u32, ty: ValueType) -> Result<()> {
        code.push(0x58); // pop rax
        if ty == ValueType::I32 {
            emit_zero_extend_eax(code);
        }
        self.emit_store_rax_to_slot(code, index)?;
        code.push(0x50); // push rax
        Ok(())
    }

    fn emit_load_slot_to_rax(&self, code: &mut Vec<u8>, index: u32) -> Result<()> {
        let disp = self.slot_disp(index)?;
        code.extend_from_slice(&[0x48, 0x8B, 0x85]); // mov rax, [rbp + disp32]
        code.extend_from_slice(&disp.to_le_bytes());
        Ok(())
    }

    fn emit_store_rax_to_slot(&self, code: &mut Vec<u8>, index: u32) -> Result<()> {
        let disp = self.slot_disp(index)?;
        code.extend_from_slice(&[0x48, 0x89, 0x85]); // mov [rbp + disp32], rax
        code.extend_from_slice(&disp.to_le_bytes());
        Ok(())
    }

    fn slot_disp(&self, index: u32) -> Result<i32> {
        if index >= self.slots {
            bail!("local index {} outside frame with {} slots", index, self.slots);
        }
        Ok(-8 * ((index as i32) + 1))
    }
}

#[derive(Clone, Copy)]
enum SetCc {
    Eq,
    Ne,
    LtS,
    LtU,
    GtS,
    GtU,
    LeS,
    LeU,
    GeS,
    GeU,
}

impl SetCc {
    fn opcode(self) -> u8 {
        match self {
            Self::Eq => 0x94,  // sete
            Self::Ne => 0x95,  // setne
            Self::LtS => 0x9C, // setl
            Self::LtU => 0x92, // setb
            Self::GtS => 0x9F, // setg
            Self::GtU => 0x97, // seta
            Self::LeS => 0x9E, // setle
            Self::LeU => 0x96, // setbe
            Self::GeS => 0x9D, // setge
            Self::GeU => 0x93, // setae
        }
    }
}

fn local_type(ir: &FunctionIr, index: u32) -> Result<ValueType> {
    ir.locals
        .get(index as usize)
        .copied()
        .ok_or_else(|| anyhow::anyhow!("local index {} outside function frame", index))
}

fn emit_global_get(code: &mut Vec<u8>, ir: &FunctionIr, index: u32) -> Result<()> {
    let global = *ir
        .global_values
        .get(index as usize)
        .ok_or_else(|| anyhow::anyhow!("global index {} outside global table", index))?;
    match global {
        GlobalValue::I32(v) => emit_push_i32(code, v),
        GlobalValue::I64(v) => emit_push_i64(code, v),
    }
    Ok(())
}

fn emit_push_i32(code: &mut Vec<u8>, value: i32) {
    code.push(0xB8); // mov eax, imm32; zero-extends into rax
    code.extend_from_slice(&value.to_le_bytes());
    code.push(0x50); // push rax
}

fn emit_push_i64(code: &mut Vec<u8>, value: i64) {
    code.extend_from_slice(&[0x48, 0xB8]); // mov rax, imm64
    code.extend_from_slice(&value.to_le_bytes());
    code.push(0x50); // push rax
}

fn emit_i32_add(code: &mut Vec<u8>) {
    code.push(0x58); // pop rax = rhs
    code.push(0x59); // pop rcx = lhs
    code.extend_from_slice(&[0x01, 0xC8]); // add eax, ecx; zero-extends into rax
    code.push(0x50); // push rax
}

fn emit_i32_sub(code: &mut Vec<u8>) {
    code.push(0x58); // pop rax = rhs
    code.push(0x59); // pop rcx = lhs
    code.extend_from_slice(&[0x29, 0xC1]); // sub ecx, eax; zero-extends rcx
    code.push(0x51); // push rcx
}

fn emit_i32_mul(code: &mut Vec<u8>) {
    code.push(0x58); // pop rax = rhs
    code.push(0x59); // pop rcx = lhs
    code.extend_from_slice(&[0x0F, 0xAF, 0xC1]); // imul eax, ecx; low 32-bit result, zero-extends into rax
    code.push(0x50); // push rax
}

fn emit_i64_add(code: &mut Vec<u8>) {
    code.push(0x58); // pop rax = rhs
    code.push(0x59); // pop rcx = lhs
    code.extend_from_slice(&[0x48, 0x01, 0xC8]); // add rax, rcx
    code.push(0x50); // push rax
}

fn emit_i64_sub(code: &mut Vec<u8>) {
    code.push(0x58); // pop rax = rhs
    code.push(0x59); // pop rcx = lhs
    code.extend_from_slice(&[0x48, 0x29, 0xC1]); // sub rcx, rax
    code.push(0x51); // push rcx
}

fn emit_i64_mul(code: &mut Vec<u8>) {
    code.push(0x58); // pop rax = rhs
    code.push(0x59); // pop rcx = lhs
    code.extend_from_slice(&[0x48, 0x0F, 0xAF, 0xC1]); // imul rax, rcx
    code.push(0x50); // push rax
}

fn emit_i32_eqz(code: &mut Vec<u8>) {
    code.push(0x58); // pop rax
    code.extend_from_slice(&[0x85, 0xC0]); // test eax, eax
    emit_setcc_bool(code, SetCc::Eq);
}

fn emit_i64_eqz(code: &mut Vec<u8>) {
    code.push(0x58); // pop rax
    code.extend_from_slice(&[0x48, 0x85, 0xC0]); // test rax, rax
    emit_setcc_bool(code, SetCc::Eq);
}

fn emit_i32_cmp(code: &mut Vec<u8>, cc: SetCc) {
    code.push(0x58); // pop rax = rhs
    code.push(0x59); // pop rcx = lhs
    code.extend_from_slice(&[0x39, 0xC1]); // cmp ecx, eax; flags = lhs - rhs
    emit_setcc_bool(code, cc);
}

fn emit_i64_cmp(code: &mut Vec<u8>, cc: SetCc) {
    code.push(0x58); // pop rax = rhs
    code.push(0x59); // pop rcx = lhs
    code.extend_from_slice(&[0x48, 0x39, 0xC1]); // cmp rcx, rax; flags = lhs - rhs
    emit_setcc_bool(code, cc);
}

fn emit_setcc_bool(code: &mut Vec<u8>, cc: SetCc) {
    code.extend_from_slice(&[0x0F, cc.opcode(), 0xC0]); // setcc al
    code.extend_from_slice(&[0x0F, 0xB6, 0xC0]); // movzx eax, al
    code.push(0x50); // push rax
}

fn emit_zero_extend_eax(code: &mut Vec<u8>) {
    code.extend_from_slice(&[0x89, 0xC0]); // mov eax, eax
}

fn emit_return(code: &mut Vec<u8>, result: Option<ValueType>) {
    if let Some(ty) = result {
        code.push(0x58); // pop rax
        if ty == ValueType::I32 {
            emit_zero_extend_eax(code);
        }
    }
    code.extend_from_slice(&[0x48, 0x89, 0xEC]); // mov rsp, rbp
    code.push(0x5D); // pop rbp
    code.push(0xC3); // ret
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::ir::{verify_ir, FuncSig, FunctionIr, GlobalValue, IrOp, ValueType};

    fn test_ir(name: &str, sig: FuncSig, locals: Vec<ValueType>, ops: Vec<IrOp>) -> FunctionIr {
        FunctionIr {
            index: 0,
            export_name: Some(name.to_string()),
            sig,
            locals,
            global_values: Vec::new(),
            ops,
        }
    }

    #[test]
    fn x86_add_two_i32_params() {
        let ir = test_ir(
            "add",
            FuncSig {
                params: vec![ValueType::I32, ValueType::I32],
                results: vec![ValueType::I32],
            },
            vec![ValueType::I32, ValueType::I32],
            vec![IrOp::LocalGet(0), IrOp::LocalGet(1), IrOp::I32Add, IrOp::End],
        );

        verify_ir(&ir).unwrap();
        let code = X86_64Backend::compile(&ir).unwrap();
        assert!(code.starts_with(&[0x55, 0x48, 0x89, 0xE5]));
        assert!(code.ends_with(&[0x89, 0xC0, 0x48, 0x89, 0xEC, 0x5D, 0xC3]));
    }

    #[test]
    fn x86_supports_mutable_local() {
        let ir = test_ir(
            "mut_local",
            FuncSig {
                params: vec![ValueType::I64],
                results: vec![ValueType::I64],
            },
            vec![ValueType::I64, ValueType::I64],
            vec![
                IrOp::LocalGet(0),
                IrOp::I64Const(7),
                IrOp::I64Add,
                IrOp::LocalSet(1),
                IrOp::LocalGet(1),
                IrOp::End,
            ],
        );

        verify_ir(&ir).unwrap();
        let code = X86_64Backend::compile(&ir).unwrap();
        assert!(!code.is_empty());
    }

    #[test]
    fn x86_supports_i32_comparison() {
        let ir = test_ir(
            "lt",
            FuncSig {
                params: vec![ValueType::I32, ValueType::I32],
                results: vec![ValueType::I32],
            },
            vec![ValueType::I32, ValueType::I32],
            vec![IrOp::LocalGet(0), IrOp::LocalGet(1), IrOp::I32LtS, IrOp::End],
        );

        verify_ir(&ir).unwrap();
        let code = X86_64Backend::compile(&ir).unwrap();
        assert!(code.windows(3).any(|w| w == [0x0F, 0x9C, 0xC0]));
    }

    #[test]
    fn x86_supports_global_get() {
        let mut ir = test_ir(
            "global",
            FuncSig {
                params: vec![],
                results: vec![ValueType::I32],
            },
            vec![],
            vec![IrOp::GlobalGet(0, ValueType::I32), IrOp::End],
        );
        ir.global_values = vec![GlobalValue::I32(42)];

        verify_ir(&ir).unwrap();
        let code = X86_64Backend::compile(&ir).unwrap();
        assert!(code.windows(5).any(|w| w == [0xB8, 42, 0, 0, 0]));
    }
}
