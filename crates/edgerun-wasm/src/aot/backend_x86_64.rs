use anyhow::{bail, Result};

use super::ir::{FunctionIr, IrOp};

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
                IrOp::LocalGet(i) => frame.emit_local_get(&mut code, i)?,
                IrOp::LocalSet(i) => frame.emit_local_set(&mut code, i)?,
                IrOp::LocalTee(i) => frame.emit_local_tee(&mut code, i)?,
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
            self.emit_store_arg(code, i)?;
        }
        for i in ir.sig.params.len() as u32..self.slots {
            self.emit_zero_slot(code, i)?;
        }

        Ok(())
    }

    fn emit_store_arg(&self, code: &mut Vec<u8>, index: u32) -> Result<()> {
        self.emit_arg_to_rax(code, index)?;
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

    fn emit_local_get(&self, code: &mut Vec<u8>, index: u32) -> Result<()> {
        self.emit_load_slot_to_rax(code, index)?;
        code.push(0x50); // push rax
        Ok(())
    }

    fn emit_local_set(&self, code: &mut Vec<u8>, index: u32) -> Result<()> {
        code.push(0x58); // pop rax
        self.emit_store_rax_to_slot(code, index)
    }

    fn emit_local_tee(&self, code: &mut Vec<u8>, index: u32) -> Result<()> {
        code.push(0x58); // pop rax
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
    code.extend_from_slice(&[0x48, 0x89, 0xEC]); // mov rsp, rbp
    code.push(0x5D); // pop rbp
    code.push(0xC3); // ret
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::ir::{verify_ir, FuncSig, FunctionIr, IrOp, ValueType};

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
        assert!(code.starts_with(&[0x55, 0x48, 0x89, 0xE5]));
        assert!(code.ends_with(&[0x48, 0x89, 0xEC, 0x5D, 0xC3]));
    }

    #[test]
    fn x86_supports_mutable_local() {
        let ir = FunctionIr {
            index: 0,
            export_name: Some("mut_local".to_string()),
            sig: FuncSig {
                params: vec![ValueType::I64],
                results: vec![ValueType::I64],
            },
            locals: vec![ValueType::I64, ValueType::I64],
            ops: vec![
                IrOp::LocalGet(0),
                IrOp::I64Const(7),
                IrOp::I64Add,
                IrOp::LocalSet(1),
                IrOp::LocalGet(1),
                IrOp::End,
            ],
        };

        verify_ir(&ir).unwrap();
        let code = X86_64Backend::compile(&ir).unwrap();
        assert!(!code.is_empty());
    }
}
