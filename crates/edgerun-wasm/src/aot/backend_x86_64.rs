use anyhow::{bail, Result};

use super::ir::{FunctionIr, IrOp};

pub struct X86_64Backend;

impl X86_64Backend {
    pub fn compile(ir: &FunctionIr) -> Result<Vec<u8>> {
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
        assert_eq!(code, vec![0x57, 0x56, 0x58, 0x59, 0x48, 0x01, 0xC8, 0x50, 0x58, 0xC3]);
    }
}
