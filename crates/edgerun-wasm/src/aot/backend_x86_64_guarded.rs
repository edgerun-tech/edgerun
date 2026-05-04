use anyhow::{bail, Result};

use super::ir::{FunctionIr, GlobalValue, IrOp, LoadKind, MemOp, StoreKind, ValueType};

pub struct X86_64Backend;

impl X86_64Backend {
    pub fn compile(ir: &FunctionIr) -> Result<Vec<u8>> {
        let mut code = Vec::new();
        let frame = Frame::new(ir)?;
        frame.emit_prologue(&mut code, ir)?;

        let mut controls = Vec::<ControlFrame>::new();
        let mut terminated = false;

        for op in &ir.ops {
            match op.clone() {
                IrOp::Nop => {}
                IrOp::Block => controls.push(ControlFrame::block()),
                IrOp::Loop => controls.push(ControlFrame::loop_at(code.len())),
                IrOp::BlockEnd => {
                    let frame = controls.pop().ok_or_else(|| {
                        anyhow::anyhow!("block.end without matching control frame")
                    })?;
                    patch_control_end(&mut code, frame)?;
                }
                IrOp::Br(depth) => emit_br(&mut code, &mut controls, depth)?,
                IrOp::BrIf(depth) => emit_br_if(&mut code, &mut controls, depth)?,
                IrOp::Call(index, _) => bail!(
                    "single-function AOT backend cannot lower call {index}; use module backend"
                ),
                IrOp::I32Const(v) => emit_push_i32(&mut code, v),
                IrOp::I64Const(v) => emit_push_i64(&mut code, v),
                IrOp::GlobalGet(i, ty) => frame.emit_global_get(&mut code, i, ty)?,
                IrOp::GlobalSet(i, ty) => frame.emit_global_set(&mut code, i, ty)?,
                IrOp::LocalGet(i) => frame.emit_local_get(&mut code, i, local_type(ir, i)?)?,
                IrOp::LocalSet(i) => frame.emit_local_set(&mut code, i, local_type(ir, i)?)?,
                IrOp::LocalTee(i) => frame.emit_local_tee(&mut code, i, local_type(ir, i)?)?,
                IrOp::Load(kind, mem) => emit_load(&mut code, kind, mem)?,
                IrOp::Store(kind, mem) => emit_store(&mut code, kind, mem)?,
                IrOp::MemoryCopy => emit_memory_copy(&mut code)?,
                IrOp::MemoryFill => emit_memory_fill(&mut code)?,
                IrOp::Select(ty) => emit_select(&mut code, ty),
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
                    if !controls.is_empty() {
                        bail!(
                            "function terminator reached with {} open control frame(s)",
                            controls.len()
                        );
                    }
                    emit_return(&mut code, ir.sig.results.first().copied());
                    terminated = true;
                    break;
                }
            }
        }

        if !terminated {
            if !controls.is_empty() {
                bail!(
                    "implicit function terminator reached with {} open control frame(s)",
                    controls.len()
                );
            }
            emit_return(&mut code, ir.sig.results.first().copied());
        }

        Ok(code)
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum ControlKind {
    Block,
    Loop,
}

struct ControlFrame {
    kind: ControlKind,
    start: usize,
    end_patches: Vec<usize>,
}

impl ControlFrame {
    fn block() -> Self {
        Self {
            kind: ControlKind::Block,
            start: 0,
            end_patches: Vec::new(),
        }
    }

    fn loop_at(start: usize) -> Self {
        Self {
            kind: ControlKind::Loop,
            start,
            end_patches: Vec::new(),
        }
    }
}

struct Frame {
    local_slots: u32,
    global_slots: u32,
}

impl Frame {
    fn new(ir: &FunctionIr) -> Result<Self> {
        let local_slots = ir.locals.len() as u32;
        let global_slots = ir.global_values.len() as u32;
        let slots = local_slots
            .checked_add(global_slots)
            .ok_or_else(|| anyhow::anyhow!("baseline backend frame slot overflow"))?;
        if slots > 4096 {
            bail!("baseline backend refuses huge frame: {slots} slots");
        }
        Ok(Self {
            local_slots,
            global_slots,
        })
    }

    fn total_slots(&self) -> u32 {
        self.local_slots + self.global_slots
    }

    fn emit_prologue(&self, code: &mut Vec<u8>, ir: &FunctionIr) -> Result<()> {
        code.push(0x55);
        code.extend_from_slice(&[0x48, 0x89, 0xE5]);
        let frame_bytes = self.total_slots() * 8;
        if frame_bytes > 0 {
            code.extend_from_slice(&[0x48, 0x81, 0xEC]);
            code.extend_from_slice(&(frame_bytes as i32).to_le_bytes());
        }
        for i in 0..ir.sig.params.len() as u32 {
            self.emit_store_arg(code, i, local_type(ir, i)?)?;
        }
        for i in ir.sig.params.len() as u32..self.local_slots {
            self.emit_zero_slot(code, i)?;
        }
        for (i, global) in ir.global_values.iter().copied().enumerate() {
            self.emit_init_global(code, i as u32, global)?;
        }
        Ok(())
    }

    fn emit_store_arg(&self, code: &mut Vec<u8>, index: u32, ty: ValueType) -> Result<()> {
        emit_arg_to_rax(code, index)?;
        if ty == ValueType::I32 {
            emit_zero_extend_eax(code);
        }
        self.emit_store_rax(code, self.local_slot(index)?)
    }

    fn emit_init_global(&self, code: &mut Vec<u8>, index: u32, global: GlobalValue) -> Result<()> {
        match global {
            GlobalValue::I32(v) => {
                code.push(0xB8);
                code.extend_from_slice(&v.to_le_bytes());
            }
            GlobalValue::I64(v) => {
                code.extend_from_slice(&[0x48, 0xB8]);
                code.extend_from_slice(&v.to_le_bytes());
            }
        }
        self.emit_store_rax(code, self.global_slot(index)?)
    }

    fn emit_zero_slot(&self, code: &mut Vec<u8>, slot: u32) -> Result<()> {
        let disp = self.slot_disp(slot)?;
        code.extend_from_slice(&[0x48, 0xC7, 0x85]);
        code.extend_from_slice(&disp.to_le_bytes());
        code.extend_from_slice(&0i32.to_le_bytes());
        Ok(())
    }

    fn emit_local_get(&self, code: &mut Vec<u8>, index: u32, ty: ValueType) -> Result<()> {
        self.emit_load_rax(code, self.local_slot(index)?)?;
        if ty == ValueType::I32 {
            emit_zero_extend_eax(code);
        }
        code.push(0x50);
        Ok(())
    }

    fn emit_local_set(&self, code: &mut Vec<u8>, index: u32, ty: ValueType) -> Result<()> {
        code.push(0x58);
        if ty == ValueType::I32 {
            emit_zero_extend_eax(code);
        }
        self.emit_store_rax(code, self.local_slot(index)?)
    }

    fn emit_local_tee(&self, code: &mut Vec<u8>, index: u32, ty: ValueType) -> Result<()> {
        code.push(0x58);
        if ty == ValueType::I32 {
            emit_zero_extend_eax(code);
        }
        self.emit_store_rax(code, self.local_slot(index)?)?;
        code.push(0x50);
        Ok(())
    }

    fn emit_global_get(&self, code: &mut Vec<u8>, index: u32, ty: ValueType) -> Result<()> {
        self.emit_load_rax(code, self.global_slot(index)?)?;
        if ty == ValueType::I32 {
            emit_zero_extend_eax(code);
        }
        code.push(0x50);
        Ok(())
    }

    fn emit_global_set(&self, code: &mut Vec<u8>, index: u32, ty: ValueType) -> Result<()> {
        code.push(0x58);
        if ty == ValueType::I32 {
            emit_zero_extend_eax(code);
        }
        self.emit_store_rax(code, self.global_slot(index)?)
    }

    fn emit_load_rax(&self, code: &mut Vec<u8>, slot: u32) -> Result<()> {
        let disp = self.slot_disp(slot)?;
        code.extend_from_slice(&[0x48, 0x8B, 0x85]);
        code.extend_from_slice(&disp.to_le_bytes());
        Ok(())
    }

    fn emit_store_rax(&self, code: &mut Vec<u8>, slot: u32) -> Result<()> {
        let disp = self.slot_disp(slot)?;
        code.extend_from_slice(&[0x48, 0x89, 0x85]);
        code.extend_from_slice(&disp.to_le_bytes());
        Ok(())
    }

    fn local_slot(&self, index: u32) -> Result<u32> {
        if index >= self.local_slots {
            bail!(
                "local index {} outside frame with {} local slots",
                index,
                self.local_slots
            );
        }
        Ok(index)
    }

    fn global_slot(&self, index: u32) -> Result<u32> {
        if index >= self.global_slots {
            bail!(
                "global index {} outside frame with {} global slots",
                index,
                self.global_slots
            );
        }
        Ok(self.local_slots + index)
    }

    fn slot_disp(&self, slot: u32) -> Result<i32> {
        if slot >= self.total_slots() {
            bail!(
                "slot index {} outside frame with {} slots",
                slot,
                self.total_slots()
            );
        }
        Ok(-8 * ((slot as i32) + 1))
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
            Self::Eq => 0x94,
            Self::Ne => 0x95,
            Self::LtS => 0x9C,
            Self::LtU => 0x92,
            Self::GtS => 0x9F,
            Self::GtU => 0x97,
            Self::LeS => 0x9E,
            Self::LeU => 0x96,
            Self::GeS => 0x9D,
            Self::GeU => 0x93,
        }
    }
}

fn local_type(ir: &FunctionIr, index: u32) -> Result<ValueType> {
    ir.locals
        .get(index as usize)
        .copied()
        .ok_or_else(|| anyhow::anyhow!("local index {} outside function frame", index))
}

fn emit_arg_to_rax(code: &mut Vec<u8>, index: u32) -> Result<()> {
    match index {
        0 => code.extend_from_slice(&[0x48, 0x89, 0xF8]),
        1 => code.extend_from_slice(&[0x48, 0x89, 0xF0]),
        2 => code.extend_from_slice(&[0x48, 0x89, 0xD0]),
        3 => code.extend_from_slice(&[0x48, 0x89, 0xC8]),
        4 => code.extend_from_slice(&[0x4C, 0x89, 0xC0]),
        5 => code.extend_from_slice(&[0x4C, 0x89, 0xC8]),
        _ => bail!("baseline backend only supports first six argument locals"),
    }
    Ok(())
}

fn emit_push_i32(code: &mut Vec<u8>, value: i32) {
    code.push(0xB8);
    code.extend_from_slice(&value.to_le_bytes());
    code.push(0x50);
}
fn emit_push_i64(code: &mut Vec<u8>, value: i64) {
    code.extend_from_slice(&[0x48, 0xB8]);
    code.extend_from_slice(&value.to_le_bytes());
    code.push(0x50);
}

fn emit_memory_address(code: &mut Vec<u8>, mem: MemOp) -> Result<()> {
    code.push(0x58);
    emit_zero_extend_eax(code);
    if mem.offset > 0 {
        if mem.offset > i32::MAX as u64 {
            bail!(
                "memory offset too large for baseline x86_64 emitter: {}",
                mem.offset
            );
        }
        code.extend_from_slice(&[0x48, 0x05]);
        code.extend_from_slice(&(mem.offset as i32).to_le_bytes());
    }
    Ok(())
}

fn emit_load(code: &mut Vec<u8>, kind: LoadKind, mem: MemOp) -> Result<()> {
    emit_memory_address(code, mem)?;
    match kind {
        LoadKind::I32 => code.extend_from_slice(&[0x41, 0x8B, 0x04, 0x02]),
        LoadKind::I64 => code.extend_from_slice(&[0x49, 0x8B, 0x04, 0x02]),
        LoadKind::I32Load8U => code.extend_from_slice(&[0x41, 0x0F, 0xB6, 0x04, 0x02]),
        LoadKind::I32Load8S => code.extend_from_slice(&[0x41, 0x0F, 0xBE, 0x04, 0x02]),
        LoadKind::I32Load16U => code.extend_from_slice(&[0x41, 0x0F, 0xB7, 0x04, 0x02]),
        LoadKind::I32Load16S => code.extend_from_slice(&[0x41, 0x0F, 0xBF, 0x04, 0x02]),
    }
    if matches!(kind, LoadKind::I32 | LoadKind::I32Load8U | LoadKind::I32Load16U) {
        emit_zero_extend_eax(code);
    }
    code.push(0x50);
    Ok(())
}

fn emit_store(code: &mut Vec<u8>, kind: StoreKind, mem: MemOp) -> Result<()> {
    code.push(0x59);
    emit_memory_address(code, mem)?;
    match kind {
        StoreKind::I32 => code.extend_from_slice(&[0x41, 0x89, 0x0C, 0x02]),
        StoreKind::I64 => code.extend_from_slice(&[0x49, 0x89, 0x0C, 0x02]),
        StoreKind::I32Store8 => code.extend_from_slice(&[0x41, 0x88, 0x0C, 0x02]),
        StoreKind::I32Store16 => code.extend_from_slice(&[0x66, 0x41, 0x89, 0x0C, 0x02]),
    }
    Ok(())
}

fn emit_memory_copy(code: &mut Vec<u8>) -> Result<()> {
    code.push(0x5A);
    code.push(0x59);
    code.push(0x58);
    emit_zero_extend_eax(code);
    code.extend_from_slice(&[0x89, 0xC9]);
    code.extend_from_slice(&[0x89, 0xD2]);
    code.extend_from_slice(&[0x4D, 0x8D, 0x04, 0x02]);
    code.extend_from_slice(&[0x4D, 0x8D, 0x0C, 0x0A]);
    let loop_start = code.len();
    code.extend_from_slice(&[0x48, 0x85, 0xD2]);
    code.extend_from_slice(&[0x0F, 0x84]);
    let done_patch = code.len();
    code.extend_from_slice(&0i32.to_le_bytes());
    code.extend_from_slice(&[0x41, 0x8A, 0x01]);
    code.extend_from_slice(&[0x41, 0x88, 0x00]);
    code.extend_from_slice(&[0x49, 0xFF, 0xC0]);
    code.extend_from_slice(&[0x49, 0xFF, 0xC1]);
    code.extend_from_slice(&[0x48, 0xFF, 0xCA]);
    emit_jmp_rel32(code, loop_start)?;
    patch_rel32(code, done_patch, code.len())
}

fn emit_memory_fill(code: &mut Vec<u8>) -> Result<()> {
    code.push(0x5A);
    code.push(0x59);
    code.push(0x58);
    emit_zero_extend_eax(code);
    code.extend_from_slice(&[0x89, 0xD2]);
    code.extend_from_slice(&[0x80, 0xE1, 0xFF]);
    code.extend_from_slice(&[0x4D, 0x8D, 0x04, 0x02]);
    let loop_start = code.len();
    code.extend_from_slice(&[0x48, 0x85, 0xD2]);
    code.extend_from_slice(&[0x0F, 0x84]);
    let done_patch = code.len();
    code.extend_from_slice(&0i32.to_le_bytes());
    code.extend_from_slice(&[0x41, 0x88, 0x08]);
    code.extend_from_slice(&[0x49, 0xFF, 0xC0]);
    code.extend_from_slice(&[0x48, 0xFF, 0xCA]);
    emit_jmp_rel32(code, loop_start)?;
    patch_rel32(code, done_patch, code.len())
}

fn emit_select(code: &mut Vec<u8>, ty: ValueType) {
    code.push(0x58);
    code.push(0x59);
    code.push(0x5A);
    code.extend_from_slice(&[0x85, 0xC0]);
    code.extend_from_slice(&[0x48, 0x0F, 0x45, 0xCA]);
    if ty == ValueType::I32 {
        code.extend_from_slice(&[0x89, 0xC9]);
    }
    code.push(0x51);
}

fn emit_i32_add(code: &mut Vec<u8>) {
    code.push(0x58);
    code.push(0x59);
    code.extend_from_slice(&[0x01, 0xC8]);
    code.push(0x50);
}
fn emit_i32_sub(code: &mut Vec<u8>) {
    code.push(0x58);
    code.push(0x59);
    code.extend_from_slice(&[0x29, 0xC1]);
    code.push(0x51);
}
fn emit_i32_mul(code: &mut Vec<u8>) {
    code.push(0x58);
    code.push(0x59);
    code.extend_from_slice(&[0x0F, 0xAF, 0xC1]);
    code.push(0x50);
}
fn emit_i64_add(code: &mut Vec<u8>) {
    code.push(0x58);
    code.push(0x59);
    code.extend_from_slice(&[0x48, 0x01, 0xC8]);
    code.push(0x50);
}
fn emit_i64_sub(code: &mut Vec<u8>) {
    code.push(0x58);
    code.push(0x59);
    code.extend_from_slice(&[0x48, 0x29, 0xC1]);
    code.push(0x51);
}
fn emit_i64_mul(code: &mut Vec<u8>) {
    code.push(0x58);
    code.push(0x59);
    code.extend_from_slice(&[0x48, 0x0F, 0xAF, 0xC1]);
    code.push(0x50);
}
fn emit_i32_eqz(code: &mut Vec<u8>) {
    code.push(0x58);
    code.extend_from_slice(&[0x85, 0xC0]);
    emit_setcc_bool(code, SetCc::Eq);
}
fn emit_i64_eqz(code: &mut Vec<u8>) {
    code.push(0x58);
    code.extend_from_slice(&[0x48, 0x85, 0xC0]);
    emit_setcc_bool(code, SetCc::Eq);
}
fn emit_i32_cmp(code: &mut Vec<u8>, cc: SetCc) {
    code.push(0x58);
    code.push(0x59);
    code.extend_from_slice(&[0x39, 0xC1]);
    emit_setcc_bool(code, cc);
}
fn emit_i64_cmp(code: &mut Vec<u8>, cc: SetCc) {
    code.push(0x58);
    code.push(0x59);
    code.extend_from_slice(&[0x48, 0x39, 0xC1]);
    emit_setcc_bool(code, cc);
}

fn emit_setcc_bool(code: &mut Vec<u8>, cc: SetCc) {
    code.extend_from_slice(&[0x0F, cc.opcode(), 0xC0]);
    code.extend_from_slice(&[0x0F, 0xB6, 0xC0]);
    code.push(0x50);
}
fn emit_zero_extend_eax(code: &mut Vec<u8>) {
    code.extend_from_slice(&[0x89, 0xC0]);
}

fn emit_return(code: &mut Vec<u8>, result: Option<ValueType>) {
    if let Some(ty) = result {
        code.push(0x58);
        if ty == ValueType::I32 {
            emit_zero_extend_eax(code);
        }
    }
    code.extend_from_slice(&[0x48, 0x89, 0xEC]);
    code.push(0x5D);
    code.push(0xC3);
}

fn emit_br(code: &mut Vec<u8>, controls: &mut [ControlFrame], depth: u32) -> Result<()> {
    let target_index = control_index(controls, depth)?;
    if controls[target_index].kind == ControlKind::Loop {
        emit_jmp_rel32(code, controls[target_index].start)
    } else {
        code.push(0xE9);
        let patch_at = code.len();
        code.extend_from_slice(&0i32.to_le_bytes());
        controls[target_index].end_patches.push(patch_at);
        Ok(())
    }
}

fn emit_br_if(code: &mut Vec<u8>, controls: &mut [ControlFrame], depth: u32) -> Result<()> {
    code.push(0x58);
    code.extend_from_slice(&[0x85, 0xC0]);
    let target_index = control_index(controls, depth)?;
    code.extend_from_slice(&[0x0F, 0x85]);
    let patch_at = code.len();
    code.extend_from_slice(&0i32.to_le_bytes());
    if controls[target_index].kind == ControlKind::Loop {
        patch_rel32(code, patch_at, controls[target_index].start)
    } else {
        controls[target_index].end_patches.push(patch_at);
        Ok(())
    }
}

fn control_index(controls: &[ControlFrame], depth: u32) -> Result<usize> {
    if depth as usize >= controls.len() {
        bail!(
            "branch depth {} outside {} open control frame(s)",
            depth,
            controls.len()
        );
    }
    Ok(controls.len() - 1 - depth as usize)
}

fn emit_jmp_rel32(code: &mut Vec<u8>, target: usize) -> Result<()> {
    code.push(0xE9);
    let patch_at = code.len();
    code.extend_from_slice(&0i32.to_le_bytes());
    patch_rel32(code, patch_at, target)
}

fn patch_control_end(code: &mut [u8], frame: ControlFrame) -> Result<()> {
    let target = code.len();
    for patch_at in frame.end_patches {
        patch_rel32(code, patch_at, target)?;
    }
    Ok(())
}

fn patch_rel32(code: &mut [u8], imm_at: usize, target: usize) -> Result<()> {
    let jump_end = imm_at
        .checked_add(4)
        .ok_or_else(|| anyhow::anyhow!("jump patch offset overflow"))?;
    if jump_end > code.len() || target > code.len() {
        bail!(
            "invalid jump patch imm_at={} target={} len={}",
            imm_at,
            target,
            code.len()
        );
    }
    let rel = target as i64 - jump_end as i64;
    if rel < i32::MIN as i64 || rel > i32::MAX as i64 {
        bail!("jump target out of rel32 range");
    }
    code[imm_at..jump_end].copy_from_slice(&(rel as i32).to_le_bytes());
    Ok(())
}
