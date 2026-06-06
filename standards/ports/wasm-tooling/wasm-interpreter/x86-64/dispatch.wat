  (func $jit_compile (export "jit_compile") (param $func_idx i32) (result i32)
    (local $slot i32) (local $cache_base i32) (local $cache_end i32)
    (local $dec_start i32) (local $dec_end i32) (local $dec_ptr i32)
    (local $op i32) (local $imm0 i32)
    (local $result_count i32) (local $code_off i32)
    (local $i i32) (local $count i32) (local $save_off i32)
    (local $local_idx i32) (local $import_count i32)

    ;; Compute slot = func_idx & 3
    (local.set $slot (i32.and (local.get $func_idx) (i32.const 3)))
    (local.set $cache_base (i32.add (global.get $JIT_CACHE) (i32.mul (local.get $slot) (global.get $JIT_SLOT_SIZE))))
    (local.set $cache_end (i32.add (local.get $cache_base) (global.get $JIT_SLOT_SIZE)))

    ;; Reset JIT state
    (call $jit_reset_state)
    (i32.store (global.get $JS_CACHE_BASE) (local.get $cache_base))
    (i32.store (global.get $JS_CACHE_END) (local.get $cache_end))
    (i32.store (global.get $JS_FUNC_IDX) (local.get $func_idx))

    ;; Convert global func_idx to local function index (subtract imports)
    (local.set $import_count (i32.load (global.get $OFF_IMPORT_COUNT)))
    (local.set $local_idx (i32.sub (local.get $func_idx) (local.get $import_count)))

    ;; Get result_count from function type (16-bit at offset 136 in type entry)
    (local.set $code_off (i32.mul (local.get $local_idx) (i32.const 16)))
    (local.set $code_off (i32.shl (i32.load (i32.add (global.get $OFF_FUNCTIONS_BUF) (local.get $code_off))) (i32.const 8)))
    (local.set $result_count (i32.load16_u (i32.add (i32.add (global.get $OFF_TYPES_BUF) (local.get $code_off)) (i32.const 136))))
    (i32.store (global.get $JS_RESULT_COUNT) (local.get $result_count))

    ;; Get decoded ops range
    (local.set $code_off (i32.mul (local.get $local_idx) (global.get $SZ_CODE)))
    (local.set $dec_start (i32.load (i32.add (i32.add (global.get $OFF_CODE_BUF) (local.get $code_off)) (i32.const 24))))
    (local.set $dec_end (i32.add (local.get $dec_start) (i32.load (i32.add (i32.add (global.get $OFF_CODE_BUF) (local.get $code_off)) (i32.const 32)))))
    (local.set $dec_ptr (i32.add (global.get $OFF_DECODED_OPS) (i32.mul (local.get $dec_start) (global.get $DEC_SZ))))

    ;; ── Emit function prologue ──
    (call $emit_prologue)

    ;; Cache JitGlobals.locals pointer (at [r15+0]) in rbx for fast local access
    (call $emit_mov_rbx_r15)

    ;; Cache first two locals in r12/r13 (register-allocated)
    (call $emit_load_r12_rbx)
    (call $emit_load_r13_rbx8)

    ;; ── Compile loop ──
    (block $compile_done
      (loop $compile_loop
        ;; Check if done
        (if (i32.ge_u (local.get $dec_start) (local.get $dec_end))
          (then (br $compile_done))
        )

        ;; Read opcode from decoded_op: opcode(1) + pad(3) + imm0(4) + imm1(4) = 12
        (local.set $op (i32.load8_u (local.get $dec_ptr)))
        (local.set $imm0 (i32.load (i32.add (local.get $dec_ptr) (i32.const 4))))
        (global.set $CURRENT_DEC_PTR (local.get $dec_ptr))

        ;; ── Control flow: handle directly ──

        ;; block (0x02)
        (if (i32.eq (local.get $op) (i32.const 0x02))
          (then
            (call $push_label (global.get $JIT_LABEL_BLOCK))
            (local.set $dec_start (i32.add (local.get $dec_start) (i32.const 1)))
            (local.set $dec_ptr (i32.add (local.get $dec_ptr) (global.get $DEC_SZ)))
            (br $compile_loop)
          )
        )

        ;; loop (0x03)
        (if (i32.eq (local.get $op) (i32.const 0x03))
          (then
            (call $push_label (global.get $JIT_LABEL_LOOP))
            (local.set $save_off (i32.load (global.get $JS_CODE_PTR)))
            (call $set_label_offset (i32.sub (i32.load (global.get $JS_LABEL_DEPTH)) (i32.const 1)) (local.get $save_off))
            (local.set $dec_start (i32.add (local.get $dec_start) (i32.const 1)))
            (local.set $dec_ptr (i32.add (local.get $dec_ptr) (global.get $DEC_SZ)))
            (br $compile_loop)
          )
        )

        ;; if (0x04)
        (if (i32.eq (local.get $op) (i32.const 0x04))
          (then
            ;; pop condition, test, jz with placeholder
            (call $emit_pop_rax)
            (call $emit_test_eax)
            (call $emit_jz_rel32 (i32.const 0))  ;; placeholder disp
            (local.set $save_off (i32.sub (i32.load (global.get $JS_CODE_PTR)) (i32.const 4)))  ;; address of disp
            (local.set $i (i32.load (global.get $JS_LABEL_DEPTH)))
            (call $set_label_if_jz (local.get $i) (local.get $save_off))
            (call $push_label (global.get $JIT_LABEL_IF))
            (local.set $dec_start (i32.add (local.get $dec_start) (i32.const 1)))
            (local.set $dec_ptr (i32.add (local.get $dec_ptr) (global.get $DEC_SZ)))
            (br $compile_loop)
          )
        )

        ;; else (0x05)
        (if (i32.eq (local.get $op) (i32.const 0x05))
          (then
            (local.set $i (i32.sub (i32.load (global.get $JS_LABEL_DEPTH)) (i32.const 1)))
            ;; Patch if's jz to jump here
            (local.set $save_off (call $get_label_if_jz (local.get $i)))
            (if (i32.ne (local.get $save_off) (i32.const 0))
              (then
                ;; save_off is offset from JIT_CACHE
                (i32.store
                  (i32.add (global.get $JIT_CACHE) (local.get $save_off))
                  (i32.sub (i32.load (global.get $JS_CODE_PTR)) (i32.add (local.get $save_off) (i32.const 4)))
                )
                (call $set_label_if_jz (local.get $i) (i32.const 0))
              )
            )
            ;; Emit jmp to end (placeholder) and record fixup
            (call $emit_jmp_rel32 (i32.const 0))
            (local.set $save_off (i32.sub (i32.load (global.get $JS_CODE_PTR)) (i32.const 4)))
            (call $push_fixup (local.get $i))
            ;; Also save the fixup offset override
            (i32.store (i32.add (global.get $JS_FIXUP_OFFSET) (i32.shl (i32.sub (i32.load (global.get $JS_FIXUP_COUNT)) (i32.const 1)) (i32.const 2))) (local.get $save_off))
            (local.set $dec_start (i32.add (local.get $dec_start) (i32.const 1)))
            (local.set $dec_ptr (i32.add (local.get $dec_ptr) (global.get $DEC_SZ)))
            (br $compile_loop)
          )
        )

        ;; end (0x0B)
        (if (i32.eq (local.get $op) (i32.const 0x0B))
          (then
            (local.set $i (call $pop_label))
            (if (i32.eq (call $get_label_kind (local.get $i)) (global.get $JIT_LABEL_LOOP))
              (then
                ;; Loop end: jmp back to loop start
                ;; disp = target - (code_ptr + 5)  (jmp_rel32 is 5 bytes)
                (local.set $save_off (call $get_label_offset (local.get $i)))
                (call $emit_jmp_rel32
                  (i32.sub (local.get $save_off) (i32.add (i32.load (global.get $JS_CODE_PTR)) (i32.const 5)))
                )
              )
              (else
                ;; Block/IF end: patch fixups
                (call $patch_fixups (local.get $i))
                ;; If IF (no else), patch jz
                (if (i32.eq (call $get_label_kind (local.get $i)) (global.get $JIT_LABEL_IF))
                  (then
                    (local.set $save_off (call $get_label_if_jz (local.get $i)))
                    (if (i32.ne (local.get $save_off) (i32.const 0))
                      (then
                        ;; save_off is offset from JIT_CACHE
                        (i32.store
                          (i32.add (global.get $JIT_CACHE) (local.get $save_off))
                          (i32.sub (i32.load (global.get $JS_CODE_PTR)) (i32.add (local.get $save_off) (i32.const 4)))
                        )
                      )
                    )
                  )
                )
              )
            )
            (local.set $dec_start (i32.add (local.get $dec_start) (i32.const 1)))
            (local.set $dec_ptr (i32.add (local.get $dec_ptr) (global.get $DEC_SZ)))
            (br $compile_loop)
          )
        )

        ;; br (0x0C)
        (if (i32.eq (local.get $op) (i32.const 0x0C))
          (then
            ;; imm0 = WASM label depth (0 = innermost)
            ;; Target label index = label_depth - 1 - imm0
            (local.set $i (i32.sub (i32.sub (i32.load (global.get $JS_LABEL_DEPTH)) (i32.const 1)) (local.get $imm0)))
            (if (i32.eq (call $get_label_kind (local.get $i)) (global.get $JIT_LABEL_LOOP))
              (then
                ;; Backward branch: compute disp directly
                ;; disp = target - (code_ptr + 5)  (jmp_rel32 is 5 bytes)
                (local.set $save_off (call $get_label_offset (local.get $i)))
                (call $emit_jmp_rel32
                  (i32.sub (local.get $save_off) (i32.add (i32.load (global.get $JS_CODE_PTR)) (i32.const 5)))
                )
              )
              (else
                ;; Forward branch: emit jmp + fixup
                (call $emit_jmp_rel32 (i32.const 0))
                (local.set $save_off (i32.sub (i32.load (global.get $JS_CODE_PTR)) (i32.const 4)))
                (call $push_fixup (local.get $i))
                (i32.store (i32.add (global.get $JS_FIXUP_OFFSET) (i32.shl (i32.sub (i32.load (global.get $JS_FIXUP_COUNT)) (i32.const 1)) (i32.const 2))) (local.get $save_off))
              )
            )
            (local.set $dec_start (i32.add (local.get $dec_start) (i32.const 1)))
            (local.set $dec_ptr (i32.add (local.get $dec_ptr) (global.get $DEC_SZ)))
            (br $compile_loop)
          )
        )

        ;; br_if (0x0D)
        (if (i32.eq (local.get $op) (i32.const 0x0D))
          (then
            (call $emit_pop_rax)
            (call $emit_test_eax)
            (local.set $i (i32.sub (i32.sub (i32.load (global.get $JS_LABEL_DEPTH)) (i32.const 1)) (local.get $imm0)))
            (if (i32.eq (call $get_label_kind (local.get $i)) (global.get $JIT_LABEL_LOOP))
              (then
                ;; disp = target - (code_ptr + 6)  (jne_rel32 is 6 bytes)
                (local.set $save_off (call $get_label_offset (local.get $i)))
                (call $emit_jne_rel32
                  (i32.sub (local.get $save_off) (i32.add (i32.load (global.get $JS_CODE_PTR)) (i32.const 6)))
                )
              )
              (else
                (call $emit_jne_rel32 (i32.const 0))
                (local.set $save_off (i32.sub (i32.load (global.get $JS_CODE_PTR)) (i32.const 4)))
                (call $push_fixup (local.get $i))
                (i32.store (i32.add (global.get $JS_FIXUP_OFFSET) (i32.shl (i32.sub (i32.load (global.get $JS_FIXUP_COUNT)) (i32.const 1)) (i32.const 2))) (local.get $save_off))
              )
            )
            (local.set $dec_start (i32.add (local.get $dec_start) (i32.const 1)))
            (local.set $dec_ptr (i32.add (local.get $dec_ptr) (global.get $DEC_SZ)))
            (br $compile_loop)
          )
        )

        ;; return (0x0F)
        (if (i32.eq (local.get $op) (i32.const 0x0F))
          (then
            (if (i32.eq (i32.load (global.get $JS_RESULT_COUNT)) (i32.const 1))
              (then
                (call $emit_pop_rax)
              )
              (else
                (call $emit_xor_eax_eax)
              )
            )
            (call $emit_xor_edx_edx)
            (call $emit_epilogue)
            (call $emit_ud2)
            (i32.store (global.get $JS_RETURN_EMITTED) (i32.const 1))
            (local.set $dec_start (i32.add (local.get $dec_start) (i32.const 1)))
            (local.set $dec_ptr (i32.add (local.get $dec_ptr) (global.get $DEC_SZ)))
            (br $compile_loop)
          )
        )

        ;; ── 0xFC prefixed ops (trunc_sat, memory/table) ──
        (if (i32.eq (local.get $op) (i32.const 0xFC))
          (then
            (block $fc_done
              ;; i32.trunc_sat_f32_s (0xFC, 0x00)
              (if (i32.eq (local.get $imm0) (i32.const 0x00))
                (then (call $template_i32_trunc_sat_f32_s) (br $fc_done))
              )
              ;; i32.trunc_sat_f32_u (0xFC, 0x01)
              (if (i32.eq (local.get $imm0) (i32.const 0x01))
                (then (call $template_i32_trunc_sat_f32_u) (br $fc_done))
              )
              ;; i32.trunc_sat_f64_s (0xFC, 0x02)
              (if (i32.eq (local.get $imm0) (i32.const 0x02))
                (then (call $template_i32_trunc_sat_f64_s) (br $fc_done))
              )
              ;; i32.trunc_sat_f64_u (0xFC, 0x03)
              (if (i32.eq (local.get $imm0) (i32.const 0x03))
                (then (call $template_i32_trunc_sat_f64_u) (br $fc_done))
              )
              ;; i64.trunc_sat_f32_s (0xFC, 0x04)
              (if (i32.eq (local.get $imm0) (i32.const 0x04))
                (then (call $template_i64_trunc_sat_f32_s) (br $fc_done))
              )
              ;; i64.trunc_sat_f32_u (0xFC, 0x05)
              (if (i32.eq (local.get $imm0) (i32.const 0x05))
                (then (call $template_i64_trunc_sat_f32_u) (br $fc_done))
              )
              ;; i64.trunc_sat_f64_s (0xFC, 0x06)
              (if (i32.eq (local.get $imm0) (i32.const 0x06))
                (then (call $template_i64_trunc_sat_f64_s) (br $fc_done))
              )
              ;; i64.trunc_sat_f64_u (0xFC, 0x07)
              (if (i32.eq (local.get $imm0) (i32.const 0x07))
                (then (call $template_i64_trunc_sat_f64_u) (br $fc_done))
              )
              ;; memory.init / data.drop / memory.copy / memory.fill / table.init /
              ;; table.drop / table.copy / table.fill / table.get / table.set /
              ;; table.grow / table.size (0x08-0x11) — unsupported
              (call $template_unsupported)
            )
            (local.set $dec_start (i32.add (local.get $dec_start) (i32.const 1)))
            (local.set $dec_ptr (i32.add (local.get $dec_ptr) (global.get $DEC_SZ)))
            (br $compile_loop)
          )
        )

        ;; ── 0xFD prefixed ops (WASM SIMD) ──
        ;; Sub-opcode encoding uses canonical IDs (AArch64 old encoding).
        ;; The decoder in interpreter.wat translates modern wabt → canonical.
        (if (i32.eq (local.get $op) (i32.const 0xFD))
          (then
            (local.set $imm0 (i32.load (i32.add (local.get $dec_ptr) (i32.const 4))))
            (block $fd_done
              ;; ── Memory ops ──
              (if (i32.eq (local.get $imm0) (i32.const 0x00)) (then (call $template_simd_v128_load (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x0C)) (then (call $template_simd_v128_const (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x1B)) (then (call $template_simd_v128_store (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0xA6)) (then (call $template_simd_v128_load8_splat (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0xA7)) (then (call $template_simd_v128_load16_splat (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0xA8)) (then (call $template_simd_v128_load32_splat (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0xA9)) (then (call $template_simd_v128_load64_splat (local.get $dec_ptr)) (br $fd_done)))
              ;; Swizzle
              (if (i32.eq (local.get $imm0) (i32.const 0xAA)) (then (call $template_simd_i8x16_swizzle (local.get $dec_ptr)) (br $fd_done)))
              ;; ── Splats (0x2D, 0x31, 0x35, 0x39, 0x3A, 0x3D) ──
              (if (i32.eq (local.get $imm0) (i32.const 0x2D)) (then (call $template_simd_i8x16_splat (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x31)) (then (call $template_simd_i16x8_splat (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x35)) (then (call $template_simd_i32x4_splat (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x39)) (then (call $template_simd_i64x2_splat (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x3A)) (then (call $template_simd_f32x4_splat (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x3D)) (then (call $template_simd_f64x2_splat (local.get $dec_ptr)) (br $fd_done)))
              ;; ── Extract/replace lanes (0x2E-0x30, 0x32-0x34, 0x36-0x38, 0xAB, 0x3B-0x3C, 0x3E-0x3F) ──
              (if (i32.eq (local.get $imm0) (i32.const 0x2E)) (then (call $template_simd_i8x16_extract_lane_s (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x2F)) (then (call $template_simd_i8x16_extract_lane_u (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x30)) (then (call $template_simd_i8x16_replace_lane (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x32)) (then (call $template_simd_i16x8_extract_lane_s (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x33)) (then (call $template_simd_i16x8_extract_lane_u (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x34)) (then (call $template_simd_i16x8_replace_lane (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x36)) (then (call $template_simd_i32x4_extract_lane (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x37)) (then (call $template_simd_i32x4_replace_lane (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x38)) (then (call $template_simd_i64x2_extract_lane (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0xAB)) (then (call $template_simd_i64x2_replace_lane (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x3B)) (then (call $template_simd_f32x4_extract_lane (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x3C)) (then (call $template_simd_f32x4_replace_lane (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x3E)) (then (call $template_simd_f64x2_extract_lane (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x3F)) (then (call $template_simd_f64x2_replace_lane (local.get $dec_ptr)) (br $fd_done)))
              ;; ── i8x16 arithmetic ──
              (if (i32.eq (local.get $imm0) (i32.const 0x40)) (then (call $template_simd_i8x16_add (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x41)) (then (call $template_simd_i8x16_sub (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x46)) (then (call $template_simd_i8x16_neg (local.get $dec_ptr)) (br $fd_done)))
              ;; ── i8x16 comparisons ──
              (if (i32.eq (local.get $imm0) (i32.const 0x47)) (then (call $template_simd_i8x16_eq (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x48)) (then (call $template_simd_i8x16_ne (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x49)) (then (call $template_simd_i8x16_ge_s (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x4B)) (then (call $template_simd_i8x16_lt_s (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x4D)) (then (call $template_simd_i8x16_gt_s (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x4F)) (then (call $template_simd_i8x16_le_s (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x4A)) (then (call $template_simd_i8x16_lt_u (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x4C)) (then (call $template_simd_i8x16_gt_u (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x4E)) (then (call $template_simd_i8x16_le_u (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0xAE)) (then (call $template_simd_i8x16_ge_u (local.get $dec_ptr)) (br $fd_done)))
              ;; ── i8x16 min/max/avgr ──
              (if (i32.eq (local.get $imm0) (i32.const 0xB8)) (then (call $template_simd_i8x16_min_u (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0xBA)) (then (call $template_simd_i8x16_max_u (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0xBB)) (then (call $template_simd_i8x16_avgr_u (local.get $dec_ptr)) (br $fd_done)))
              ;; ── i16x8 arithmetic ──
              (if (i32.eq (local.get $imm0) (i32.const 0x51)) (then (call $template_simd_i16x8_add (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x52)) (then (call $template_simd_i16x8_sub (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x57)) (then (call $template_simd_i16x8_neg (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x61)) (then (call $template_simd_i16x8_mul (local.get $dec_ptr)) (br $fd_done)))
              ;; ── i16x8 comparisons ──
              (if (i32.eq (local.get $imm0) (i32.const 0x58)) (then (call $template_simd_i16x8_eq (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x59)) (then (call $template_simd_i16x8_ne (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x5A)) (then (call $template_simd_i16x8_ge_s (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x5C)) (then (call $template_simd_i16x8_lt_s (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x5E)) (then (call $template_simd_i16x8_gt_s (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x60)) (then (call $template_simd_i16x8_le_s (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x5B)) (then (call $template_simd_i16x8_lt_u (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x5D)) (then (call $template_simd_i16x8_gt_u (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x5F)) (then (call $template_simd_i16x8_le_u (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0xAF)) (then (call $template_simd_i16x8_ge_u (local.get $dec_ptr)) (br $fd_done)))
              ;; ── i16x8 min/max/avgr ──
              (if (i32.eq (local.get $imm0) (i32.const 0xBC)) (then (call $template_simd_i16x8_min_u (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0xBE)) (then (call $template_simd_i16x8_max_u (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0xBF)) (then (call $template_simd_i16x8_avgr_u (local.get $dec_ptr)) (br $fd_done)))
              ;; ── i32x4 arithmetic ──
              (if (i32.eq (local.get $imm0) (i32.const 0x62)) (then (call $template_simd_i32x4_add (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x63)) (then (call $template_simd_i32x4_sub (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x68)) (then (call $template_simd_i32x4_neg (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x72)) (then (call $template_simd_i32x4_mul (local.get $dec_ptr)) (br $fd_done)))
              ;; ── i32x4 comparisons ──
              (if (i32.eq (local.get $imm0) (i32.const 0x69)) (then (call $template_simd_i32x4_eq (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x6A)) (then (call $template_simd_i32x4_ne (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x6B)) (then (call $template_simd_i32x4_ge_s (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x6D)) (then (call $template_simd_i32x4_lt_s (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x6F)) (then (call $template_simd_i32x4_gt_s (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x71)) (then (call $template_simd_i32x4_le_s (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x6C)) (then (call $template_simd_i32x4_lt_u (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x6E)) (then (call $template_simd_i32x4_gt_u (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x70)) (then (call $template_simd_i32x4_le_u (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0xB0)) (then (call $template_simd_i32x4_ge_u (local.get $dec_ptr)) (br $fd_done)))
              ;; ── i64x2 arithmetic ──
              (if (i32.eq (local.get $imm0) (i32.const 0x73)) (then (call $template_simd_i64x2_add (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x74)) (then (call $template_simd_i64x2_sub (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x79)) (then (call $template_simd_i64x2_neg (local.get $dec_ptr)) (br $fd_done)))
              ;; i64x2.mul (not in old encoding) — template not yet added
              ;; ── f32x4 unary ──
              (if (i32.eq (local.get $imm0) (i32.const 0x88)) (then (call $template_simd_f32x4_abs (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x8A)) (then (call $template_simd_f32x4_neg (local.get $dec_ptr)) (br $fd_done)))
              ;; ── f32x4 binary ──
              (if (i32.eq (local.get $imm0) (i32.const 0x84)) (then (call $template_simd_f32x4_add (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x85)) (then (call $template_simd_f32x4_sub (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x87)) (then (call $template_simd_f32x4_mul (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x89)) (then (call $template_simd_f32x4_div (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0xE1)) (then (call $template_simd_f32x4_min (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0xE2)) (then (call $template_simd_f32x4_max (local.get $dec_ptr)) (br $fd_done)))
              ;; ── f32x4 comparisons ──
              (if (i32.eq (local.get $imm0) (i32.const 0x8C)) (then (call $template_simd_f32x4_eq (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x8B)) (then (call $template_simd_f32x4_ne (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x8E)) (then (call $template_simd_f32x4_lt (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x90)) (then (call $template_simd_f32x4_gt (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x92)) (then (call $template_simd_f32x4_le (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x94)) (then (call $template_simd_f32x4_ge (local.get $dec_ptr)) (br $fd_done)))
              ;; ── f64x2 unary ──
              (if (i32.eq (local.get $imm0) (i32.const 0x9A)) (then (call $template_simd_f64x2_abs (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x9B)) (then (call $template_simd_f64x2_neg (local.get $dec_ptr)) (br $fd_done)))
              ;; ── f64x2 binary ──
              (if (i32.eq (local.get $imm0) (i32.const 0x95)) (then (call $template_simd_f64x2_add (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x96)) (then (call $template_simd_f64x2_sub (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x98)) (then (call $template_simd_f64x2_mul (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x99)) (then (call $template_simd_f64x2_div (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0xE3)) (then (call $template_simd_f64x2_min (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0xE4)) (then (call $template_simd_f64x2_max (local.get $dec_ptr)) (br $fd_done)))
              ;; ── f64x2 comparisons ──
              (if (i32.eq (local.get $imm0) (i32.const 0x9D)) (then (call $template_simd_f64x2_eq (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x9C)) (then (call $template_simd_f64x2_ne (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x9F)) (then (call $template_simd_f64x2_lt (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0xA1)) (then (call $template_simd_f64x2_gt (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0xA3)) (then (call $template_simd_f64x2_le (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0xA5)) (then (call $template_simd_f64x2_ge (local.get $dec_ptr)) (br $fd_done)))
              ;; ── v128 bitwise ──
              (if (i32.eq (local.get $imm0) (i32.const 0x43)) (then (call $template_simd_v128_and (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x44)) (then (call $template_simd_v128_or (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0x45)) (then (call $template_simd_v128_xor (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0xAC)) (then (call $template_simd_v128_not (local.get $dec_ptr)) (br $fd_done)))
              (if (i32.eq (local.get $imm0) (i32.const 0xAD)) (then (call $template_simd_v128_bitselect (local.get $dec_ptr)) (br $fd_done)))
              ;; ── Fall through: unsupported SIMD opcode ──
              (call $template_simd_unsupported (local.get $dec_ptr))
            )
            (local.set $dec_start (i32.add (local.get $dec_start) (i32.const 1)))
            (local.set $dec_ptr (i32.add (local.get $dec_ptr) (global.get $DEC_SZ)))
            (br $compile_loop)
          )
        )

        ;; ── All other ops: dispatch to template (if/else chain) ──

        (block $template_done

          ;; local.get (0x20)
          (if (i32.eq (local.get $op) (i32.const 0x20))
            (then (call $template_local_get (local.get $dec_ptr)) (br $template_done))
          )
          ;; local.set (0x21)
          (if (i32.eq (local.get $op) (i32.const 0x21))
            (then (call $template_local_set (local.get $dec_ptr)) (br $template_done))
          )
          ;; local.tee (0x22)
          (if (i32.eq (local.get $op) (i32.const 0x22))
            (then (call $template_local_tee (local.get $dec_ptr)) (br $template_done))
          )
          ;; global.get (0x23)
          (if (i32.eq (local.get $op) (i32.const 0x23))
            (then (call $template_global_get (local.get $dec_ptr)) (br $template_done))
          )
          ;; global.set (0x24)
          (if (i32.eq (local.get $op) (i32.const 0x24))
            (then (call $template_global_set (local.get $dec_ptr)) (br $template_done))
          )
          ;; table.get (0x25)
          (if (i32.eq (local.get $op) (i32.const 0x25))
            (then (call $template_table_get (local.get $dec_ptr)) (br $template_done))
          )
          ;; table.set (0x26)
          (if (i32.eq (local.get $op) (i32.const 0x26))
            (then (call $template_table_set (local.get $dec_ptr)) (br $template_done))
          )
          ;; drop (0x1A)
          (if (i32.eq (local.get $op) (i32.const 0x1A))
            (then (call $template_drop) (br $template_done))
          )
          ;; select (0x1B)
          (if (i32.eq (local.get $op) (i32.const 0x1B))
            (then (call $template_select) (br $template_done))
          )
          ;; select_typed (0x1C) — same behavior as select
          (if (i32.eq (local.get $op) (i32.const 0x1C))
            (then (call $template_select) (br $template_done))
          )
          ;; i32.const (0x41)
          (if (i32.eq (local.get $op) (i32.const 0x41))
            (then (call $template_i32_const (local.get $dec_ptr)) (br $template_done))
          )
          ;; i64.const (0x42)
          (if (i32.eq (local.get $op) (i32.const 0x42))
            (then (call $template_i64_const (local.get $dec_ptr)) (br $template_done))
          )
          ;; f32.const (0x43)
          (if (i32.eq (local.get $op) (i32.const 0x43))
            (then (call $template_f32_const (local.get $dec_ptr)) (br $template_done))
          )
          ;; f64.const (0x44)
          (if (i32.eq (local.get $op) (i32.const 0x44))
            (then (call $template_f64_const (local.get $dec_ptr)) (br $template_done))
          )
          ;; nop (0x01)
          (if (i32.eq (local.get $op) (i32.const 0x01))
            (then (call $template_nop) (br $template_done))
          )
          ;; unreachable (0x00)
          (if (i32.eq (local.get $op) (i32.const 0x00))
            (then (call $template_unreachable) (br $template_done))
          )

          ;; i32 eqz (0x45)
          (if (i32.eq (local.get $op) (i32.const 0x45)) (then (call $template_i32_eqz) (br $template_done)))
          ;; i32 eq/ne/lt/gt/le/ge (0x46-0x4F)
          (if (i32.eq (local.get $op) (i32.const 0x46)) (then (call $template_i32_eq) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x47)) (then (call $template_i32_ne) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x48)) (then (call $template_i32_lt_s) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x49)) (then (call $template_i32_lt_u) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x4A)) (then (call $template_i32_gt_s) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x4B)) (then (call $template_i32_gt_u) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x4C)) (then (call $template_i32_le_s) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x4D)) (then (call $template_i32_le_u) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x4E)) (then (call $template_i32_ge_s) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x4F)) (then (call $template_i32_ge_u) (br $template_done)))

          ;; i32 unary (0x67-0x69)
          (if (i32.eq (local.get $op) (i32.const 0x67)) (then (call $template_i32_clz) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x68)) (then (call $template_i32_ctz) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x69)) (then (call $template_i32_popcnt) (br $template_done)))

          ;; i32 binary (0x6A-0x78)
          (if (i32.eq (local.get $op) (i32.const 0x6A)) (then (call $template_i32_add) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x6B)) (then (call $template_i32_sub) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x6C)) (then (call $template_i32_mul) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x6D)) (then (call $template_i32_div_s) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x6E)) (then (call $template_i32_div_u) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x6F)) (then (call $template_i32_rem_s) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x70)) (then (call $template_i32_rem_u) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x71)) (then (call $template_i32_and) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x72)) (then (call $template_i32_or) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x73)) (then (call $template_i32_xor) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x74)) (then (call $template_i32_shl) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x75)) (then (call $template_i32_shr_s) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x76)) (then (call $template_i32_shr_u) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x77)) (then (call $template_i32_rotl) (br $template_done)))
           (if (i32.eq (local.get $op) (i32.const 0x78)) (then (call $template_i32_rotr) (br $template_done)))

          ;; i64 unary (0x79-0x7B)
          (if (i32.eq (local.get $op) (i32.const 0x79)) (then (call $template_i64_clz) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x7A)) (then (call $template_i64_ctz) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x7B)) (then (call $template_i64_popcnt) (br $template_done)))

          ;; i64 binary (0x7C-0x8A)
          (if (i32.eq (local.get $op) (i32.const 0x7C)) (then (call $template_i64_add) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x7D)) (then (call $template_i64_sub) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x7E)) (then (call $template_i64_mul) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x7F)) (then (call $template_i64_div_s) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x80)) (then (call $template_i64_div_u) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x81)) (then (call $template_i64_rem_s) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x82)) (then (call $template_i64_rem_u) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x83)) (then (call $template_i64_and) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x84)) (then (call $template_i64_or) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x85)) (then (call $template_i64_xor) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x86)) (then (call $template_i64_shl) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x87)) (then (call $template_i64_shr_s) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x88)) (then (call $template_i64_shr_u) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x89)) (then (call $template_i64_rotl) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x8A)) (then (call $template_i64_rotr) (br $template_done)))

          ;; Conversions
          (if (i32.eq (local.get $op) (i32.const 0xA7)) (then (call $template_i32_wrap_i64) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xA8)) (then (call $template_i32_trunc_f32_s) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xA9)) (then (call $template_i32_trunc_f32_u) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xAA)) (then (call $template_i32_trunc_f64_s) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xAB)) (then (call $template_i32_trunc_f64_u) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xAC)) (then (call $template_i64_extend_i32_s) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xAD)) (then (call $template_i64_extend_i32_u) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xAE)) (then (call $template_i64_trunc_f32_s) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xAF)) (then (call $template_i64_trunc_f32_u) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xB0)) (then (call $template_i64_trunc_f64_s) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xB1)) (then (call $template_i64_trunc_f64_u) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xB2)) (then (call $template_f32_convert_i32_s) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xB3)) (then (call $template_f32_convert_i32_u) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xB4)) (then (call $template_f32_convert_i64_s) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xB5)) (then (call $template_f32_convert_i64_u) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xB6)) (then (call $template_f32_demote_f64) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xB7)) (then (call $template_f64_convert_i32_s) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xB8)) (then (call $template_f64_convert_i32_u) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xB9)) (then (call $template_f64_convert_i64_s) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xBA)) (then (call $template_f64_convert_i64_u) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xBB)) (then (call $template_f64_promote_f32) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xBC)) (then (call $template_i32_reinterpret_f32) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xBD)) (then (call $template_i64_reinterpret_f64) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xBE)) (then (call $template_f32_reinterpret_i32) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xBF)) (then (call $template_f64_reinterpret_i64) (br $template_done)))

          ;; Sign extension
          (if (i32.eq (local.get $op) (i32.const 0xC0)) (then (call $template_i32_extend8_s) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xC1)) (then (call $template_i32_extend16_s) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xC2)) (then (call $template_i64_extend8_s) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xC3)) (then (call $template_i64_extend16_s) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xC4)) (then (call $template_i64_extend32_s) (br $template_done)))

          ;; Reference types (0xD0-0xD2)
          (if (i32.eq (local.get $op) (i32.const 0xD0)) (then (call $template_ref_null) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xD1)) (then (call $template_ref_is_null) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xD2)) (then (call $template_ref_func (local.get $dec_ptr)) (br $template_done)))

           ;; Memory load/store
          (if (i32.eq (local.get $op) (i32.const 0x28)) (then (call $template_i32_load) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x29)) (then (call $template_i64_load) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x2A)) (then (call $template_f32_load) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x2B)) (then (call $template_f64_load) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x2C)) (then (call $template_i32_load8_s) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x2D)) (then (call $template_i32_load8_u) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x2E)) (then (call $template_i32_load16_s) (br $template_done)))
           (if (i32.eq (local.get $op) (i32.const 0x2F)) (then (call $template_i32_load16_u) (br $template_done)))
           ;; i64 narrow loads (0x30-0x35)
           (if (i32.eq (local.get $op) (i32.const 0x30)) (then (call $template_i64_load8_s) (br $template_done)))
           (if (i32.eq (local.get $op) (i32.const 0x31)) (then (call $template_i64_load8_u) (br $template_done)))
           (if (i32.eq (local.get $op) (i32.const 0x32)) (then (call $template_i64_load16_s) (br $template_done)))
           (if (i32.eq (local.get $op) (i32.const 0x33)) (then (call $template_i64_load16_u) (br $template_done)))
           (if (i32.eq (local.get $op) (i32.const 0x34)) (then (call $template_i64_load32_s) (br $template_done)))
           (if (i32.eq (local.get $op) (i32.const 0x35)) (then (call $template_i64_load32_u) (br $template_done)))
           (if (i32.eq (local.get $op) (i32.const 0x36)) (then (call $template_i32_store) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x37)) (then (call $template_i64_store) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x38)) (then (call $template_f32_store) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x39)) (then (call $template_f64_store) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x3A)) (then (call $template_i32_store8) (br $template_done)))
           (if (i32.eq (local.get $op) (i32.const 0x3B)) (then (call $template_i32_store16) (br $template_done)))
           ;; i64 narrow stores (0x3C-0x3E)
           (if (i32.eq (local.get $op) (i32.const 0x3C)) (then (call $template_i64_store8) (br $template_done)))
           (if (i32.eq (local.get $op) (i32.const 0x3D)) (then (call $template_i64_store16) (br $template_done)))
           (if (i32.eq (local.get $op) (i32.const 0x3E)) (then (call $template_i64_store32) (br $template_done)))

            ;; memory.size / memory.grow
          (if (i32.eq (local.get $op) (i32.const 0x3F)) (then (call $template_memory_size) (br $template_done)))
           (if (i32.eq (local.get $op) (i32.const 0x40)) (then (call $template_memory_grow) (br $template_done)))

          ;; i64 comparisons (0x50-0x5A)
          (if (i32.eq (local.get $op) (i32.const 0x50)) (then (call $template_i64_eqz) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x51)) (then (call $template_i64_eq) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x52)) (then (call $template_i64_ne) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x53)) (then (call $template_i64_lt_s) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x54)) (then (call $template_i64_lt_u) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x55)) (then (call $template_i64_gt_s) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x56)) (then (call $template_i64_gt_u) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x57)) (then (call $template_i64_le_s) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x58)) (then (call $template_i64_le_u) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x59)) (then (call $template_i64_ge_s) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x5A)) (then (call $template_i64_ge_u) (br $template_done)))

          ;; f32 comparisons (0x5B-0x60)
          (if (i32.eq (local.get $op) (i32.const 0x5B)) (then (call $template_f32_eq) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x5C)) (then (call $template_f32_ne) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x5D)) (then (call $template_f32_lt) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x5E)) (then (call $template_f32_gt) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x5F)) (then (call $template_f32_le) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x60)) (then (call $template_f32_ge) (br $template_done)))

          ;; f64 comparisons (0x61-0x66)
          (if (i32.eq (local.get $op) (i32.const 0x61)) (then (call $template_f64_eq) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x62)) (then (call $template_f64_ne) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x63)) (then (call $template_f64_lt) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x64)) (then (call $template_f64_gt) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x65)) (then (call $template_f64_le) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x66)) (then (call $template_f64_ge) (br $template_done)))

          ;; f32 binary (0x92-0x98)
          (if (i32.eq (local.get $op) (i32.const 0x92)) (then (call $template_f32_add) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x93)) (then (call $template_f32_sub) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x94)) (then (call $template_f32_mul) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x95)) (then (call $template_f32_div) (br $template_done)))
          ;; f32 min/max/copysign (0x96-0x98)
          (if (i32.eq (local.get $op) (i32.const 0x96)) (then (call $template_f32_min) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x97)) (then (call $template_f32_max) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x98)) (then (call $template_f32_copysign) (br $template_done)))

          ;; f64 binary (0xA0-0xA3)
          (if (i32.eq (local.get $op) (i32.const 0xA0)) (then (call $template_f64_add) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xA1)) (then (call $template_f64_sub) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xA2)) (then (call $template_f64_mul) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xA3)) (then (call $template_f64_div) (br $template_done)))
          ;; f64 min/max/copysign (0xA4-0xA6)
          (if (i32.eq (local.get $op) (i32.const 0xA4)) (then (call $template_f64_min) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xA5)) (then (call $template_f64_max) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0xA6)) (then (call $template_f64_copysign) (br $template_done)))

          ;; f32 unary
          (if (i32.eq (local.get $op) (i32.const 0x8B)) (then (call $template_f32_abs) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x8C)) (then (call $template_f32_neg) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x91)) (then (call $template_f32_sqrt) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x8D)) (then (call $template_f32_ceil) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x8E)) (then (call $template_f32_floor) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x8F)) (then (call $template_f32_trunc) (br $template_done)))
           (if (i32.eq (local.get $op) (i32.const 0x90)) (then (call $template_f32_nearest) (br $template_done)))

          ;; f64 unary (0x99-0x9F)
          (if (i32.eq (local.get $op) (i32.const 0x99)) (then (call $template_f64_abs) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x9A)) (then (call $template_f64_neg) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x9B)) (then (call $template_f64_sqrt) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x9C)) (then (call $template_f64_ceil) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x9D)) (then (call $template_f64_floor) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x9E)) (then (call $template_f64_trunc) (br $template_done)))
          (if (i32.eq (local.get $op) (i32.const 0x9F)) (then (call $template_f64_nearest) (br $template_done)))

          ;; call (0x10) — dispatch to template_call for import→syscall
          (if (i32.eq (local.get $op) (i32.const 0x10))
            (then (call $template_call (local.get $dec_ptr)) (br $template_done))
          )
          ;; call_indirect (0x11), br_table (0x0E) — not JIT-compilable
          (if (i32.eq (local.get $op) (i32.const 0x11))
            (then (call $template_unsupported) (br $template_done))
          )
          (if (i32.eq (local.get $op) (i32.const 0x0E))
            (then (call $template_unsupported) (br $template_done))
          )

          ;; Unsupported opcode — emit ud2
          (call $template_unsupported)
        )

        ;; Advance to next decoded op
        (local.set $dec_start (i32.add (local.get $dec_start) (i32.const 1)))
        (local.set $dec_ptr (i32.add (local.get $dec_ptr) (global.get $DEC_SZ)))
        (br $compile_loop)
      )
    )

    ;; ── Emit function epilogue ──
    (if (i32.eqz (i32.load (global.get $JS_RETURN_EMITTED)))
      (then
        (if (i32.eq (local.get $result_count) (i32.const 1))
          (then
            (call $emit_pop_rax)
          )
          (else
            (call $emit_xor_eax_eax)
          )
        )
        (call $emit_xor_edx_edx)
        (call $emit_epilogue)
      )
    )

    ;; Return code_ptr (offset from cache_base)
    (return (i32.sub (i32.load (global.get $JS_CODE_PTR)) (i32.mul (local.get $slot) (global.get $JIT_SLOT_SIZE))))
  )

  ;; ── Import → syscall dispatch ─────────────────────────────────────────
  ;; Import count: written by interpreter at address 16648 in linear memory
  (global $OFF_IMPORT_COUNT i32 (i32.const 16648))
  ;; Syscall map table at 0x90000: array of i32 sysno per import index
  ;; Default: sysno[i] = i (import 0 → SYS_read, import 1 → SYS_write, …)
  (global $OFF_SYSCALL_MAP i32 (i32.const 0x90000))

  ;; ═════════════════════════════════════════════════════════════════════
  ;; ELF64 binary output
  ;; ═════════════════════════════════════════════════════════════════════

  (global $ELF_OUT_BUF  i32 (i32.const 0x800000))
  (global $ELF_OUT_OFF  i32 (i32.const 0x700000))  ;; ELF_OUT_BUF - JIT_CACHE

  (global $TEXT_VA      i32 (i32.const 0x400000))
  (global $BSS_VA       i32 (i32.const 0x500000))

  (global $EHDR_SIZE    i32 (i32.const 64))
  (global $PHDR_SIZE    i32 (i32.const 56))
  (global $ELF_STUB_OFF i32 (i32.const 120))       ;; after 1 phdr (64 + 56)
  (global $ELF_CODE_OFF i32 (i32.const 256))        ;; aligned after stub
  (global $BSS_SIZE     i32 (i32.const 0x40000))    ;; 256KB

  ;; BSS item VAs (relative to BSS_VA)
  (global $BSS_JITGLOBALS i32 (i32.const 0x500000))
  (global $BSS_MEM       i32 (i32.const 0x500080))
  (global $BSS_LOCALS    i32 (i32.const 0x510080))
  (global $BSS_GLOBALS   i32 (i32.const 0x520080))
  (global $BSS_TABLE     i32 (i32.const 0x530080))

  ;; ── ELF emit helpers ───────────────────────────────────────────────

  ;; MOV r64, imm32 sign-extended: REX.W + C7 /0 id
  (func $emit_mov_r64_imm32 (param $rex i32) (param $modrm i32) (param $val i32)
    (call $emit_byte (local.get $rex))
    (call $emit_byte (i32.const 0xC7))
    (call $emit_byte (local.get $modrm))
    (call $emit_dword (local.get $val))
  )

  ;; MOV [r15+off8], rax: 49 89 47 <off>
  (func $emit_mov_r15off_rax (param $off i32)
    (call $emit_byte (i32.const 0x49))
    (call $emit_byte (i32.const 0x89))
    (call $emit_byte (i32.const 0x47))
    (call $emit_byte (local.get $off))
  )

  ;; MOV dword [r15+off8], imm32: 41 C7 47 <off> <imm32>
  (func $emit_mov_dword_r15off (param $off i32) (param $val i32)
    (call $emit_byte (i32.const 0x41))
    (call $emit_byte (i32.const 0xC7))
    (call $emit_byte (i32.const 0x47))
    (call $emit_byte (local.get $off))
    (call $emit_dword (local.get $val))
  )

  ;; CALL rel32: E8 <disp32>  (target = current_va + 5 + disp)
  (func $emit_call_rel (param $target_va i32)
    (local $saved i32)
    (local.set $saved (i32.load (global.get $JS_CODE_PTR)))
    (call $emit_byte (i32.const 0xE8))
    (call $emit_dword
      (i32.sub
        (local.get $target_va)
        (i32.add
          (i32.add (global.get $TEXT_VA) (i32.sub (local.get $saved) (global.get $ELF_OUT_OFF)))
          (i32.const 5)
        )
      )
    )
  )

  ;; MOV edi, eax: 89 C7
  (func $emit_mov_edi_eax
    (call $emit_byte (i32.const 0x89))
    (call $emit_byte (i32.const 0xC7))
  )

  ;; MOV eax, imm32: B8 <dword>
  (func $emit_mov_eax_imm (param $val i32)
    (call $emit_byte (i32.const 0xB8))
    (call $emit_dword (local.get $val))
  )

  ;; SYSCALL: 0F 05
  (func $emit_syscall
    (call $emit_byte (i32.const 0x0F))
    (call $emit_byte (i32.const 0x05))
  )

  ;; ── Syscall trampoline helpers ───────────────────────────────────────

  ;; Allocate 48 bytes (6 qwords) on stack for syscall struct
  ;; sub rsp, 48: 48 83 EC 30
  (func $emit_sub_rsp_48
    (call $emit_rex_w)
    (call $emit_byte (i32.const 0x83))
    (call $emit_byte (i32.const 0xEC))
    (call $emit_byte (i32.const 0x30))
  )

  ;; Deallocate 48 bytes: add rsp, 48
  (func $emit_add_rsp_48
    (call $emit_rex_w)
    (call $emit_byte (i32.const 0x83))
    (call $emit_byte (i32.const 0xC4))
    (call $emit_byte (i32.const 0x30))
  )

  ;; mov rdi, [rsp + disp32]: 48 8B BC 24 <dword>
  (func $emit_load_rdi_rsp_disp (param $disp i32)
    (call $emit_rex_w)
    (call $emit_byte (i32.const 0x8B))
    (call $emit_byte (i32.const 0xBC))
    (call $emit_sib (i32.const 0) (i32.const 4) (i32.const 4))
    (call $emit_dword (local.get $disp))
  )

  ;; mov rsi, [rsp + disp32]: 48 8B B4 24 <dword>
  (func $emit_load_rsi_rsp_disp (param $disp i32)
    (call $emit_rex_w)
    (call $emit_byte (i32.const 0x8B))
    (call $emit_byte (i32.const 0xB4))
    (call $emit_sib (i32.const 0) (i32.const 4) (i32.const 4))
    (call $emit_dword (local.get $disp))
  )

  ;; mov rdx, [rsp + disp32]: 48 8B 94 24 <dword>
  (func $emit_load_rdx_rsp_disp (param $disp i32)
    (call $emit_rex_w)
    (call $emit_byte (i32.const 0x8B))
    (call $emit_byte (i32.const 0x94))
    (call $emit_sib (i32.const 0) (i32.const 4) (i32.const 4))
    (call $emit_dword (local.get $disp))
  )

  ;; mov r10, [rsp + disp32]: 4C 8B 54 24 <dword> (no SIB needed)
  ;; mov r10, [rsp + disp32]: 4C 8B 94 24 <dword>
  ;; r10 = reg 10 (0xA): low 3 bits=010(2), REX.R=1 → REX=0x4C
  (func $emit_load_r10_rsp_disp (param $disp i32)
    (call $emit_byte (i32.const 0x4C))  ;; REX.W R=1
    (call $emit_byte (i32.const 0x8B))  ;; mov r64, r/m64
    (call $emit_byte (i32.const 0x94))  ;; ModRM: mod=10, reg=010(r10), rm=100(SIB)
    (call $emit_sib (i32.const 0) (i32.const 4) (i32.const 4))  ;; [rsp]
    (call $emit_dword (local.get $disp))
  )

  ;; mov r8, [rsp + disp32]: 4C 8B 84 24 <dword>  (mod=10 for disp32)
  (func $emit_load_r8_rsp_disp (param $disp i32)
    (call $emit_byte (i32.const 0x4C))  ;; REX.W R=1
    (call $emit_byte (i32.const 0x8B))
    (call $emit_byte (i32.const 0x84))  ;; ModRM: mod=10, reg=000(r8), rm=100(SIB)
    (call $emit_sib (i32.const 0) (i32.const 4) (i32.const 4))  ;; [rsp]
    (call $emit_dword (local.get $disp))
  )

  ;; mov r9, [rsp + disp32]: 4C 8B 8C 24 <dword>
  (func $emit_load_r9_rsp_disp (param $disp i32)
    (call $emit_byte (i32.const 0x4C))  ;; REX.W R=1
    (call $emit_byte (i32.const 0x8B))
    (call $emit_byte (i32.const 0x8C))  ;; ModRM: mod=10, reg=001(r9), rm=100(SIB)
    (call $emit_sib (i32.const 0) (i32.const 4) (i32.const 4))  ;; [rsp]
    (call $emit_dword (local.get $disp))
  )

  ;; xor eax, eax (2 bytes): 31 C0
  (func $emit_xor_eax_eax_32
    (call $emit_byte (i32.const 0x31))
    (call $emit_byte (i32.const 0xC0))
  )

  ;; ── ELF64 header ─────────────────────────────────────────────────

  (func $emit_elf64_ehdr (param $entry_va i32) (param $phoff i32) (param $phnum i32)
    (call $emit_byte (i32.const 0x7F))
    (call $emit_byte (i32.const 0x45)) (call $emit_byte (i32.const 0x4C)) (call $emit_byte (i32.const 0x46))
    (call $emit_byte (i32.const 2))     (call $emit_byte (i32.const 1))
    (call $emit_byte (i32.const 1))     (call $emit_byte (i32.const 0))
    (call $emit_qword (i64.const 0))    ;; padding bytes 8-15
    (call $emit_byte (i32.const 2)) (call $emit_byte (i32.const 0))  ;; e_type=ET_EXEC
    (call $emit_byte (i32.const 0x3E)) (call $emit_byte (i32.const 0))  ;; e_machine=x86_64
    (call $emit_dword (i32.const 1))    ;; e_version
    (call $emit_qword (i64.extend_i32_u (local.get $entry_va)))  ;; e_entry
    (call $emit_qword (i64.extend_i32_u (local.get $phoff)))     ;; e_phoff
    (call $emit_qword (i64.const 0))    ;; e_shoff
    (call $emit_dword (i32.const 0))    ;; e_flags
    (call $emit_byte (i32.const 64)) (call $emit_byte (i32.const 0))  ;; e_ehsize
    (call $emit_byte (i32.const 56)) (call $emit_byte (i32.const 0))  ;; e_phentsize
    (call $emit_byte (i32.wrap_i64 (i64.extend_i32_u (local.get $phnum)))) 
    (call $emit_byte (i32.const 0))   ;; e_phnum
    (call $emit_byte (i32.const 0)) (call $emit_byte (i32.const 0))  ;; e_shentsize=0
    (call $emit_byte (i32.const 0)) (call $emit_byte (i32.const 0))  ;; e_shnum=0
    (call $emit_byte (i32.const 0)) (call $emit_byte (i32.const 0))  ;; e_shstrndx=0
  )

  ;; ── ELF64 program header (56 bytes) ──────────────────────────────

  (func $emit_elf64_phdr (param $type i32) (param $flags i32)
                         (param $offset i32) (param $vaddr i32)
                         (param $filesz i32) (param $memsz i32)
    (call $emit_dword (local.get $type))     ;; p_type
    (call $emit_dword (local.get $flags))    ;; p_flags
    (call $emit_qword (i64.extend_i32_u (local.get $offset)))  ;; p_offset
    (call $emit_qword (i64.extend_i32_u (local.get $vaddr)))   ;; p_vaddr
    (call $emit_qword (i64.extend_i32_u (local.get $vaddr)))   ;; p_paddr = p_vaddr
    (call $emit_qword (i64.extend_i32_u (local.get $filesz)))  ;; p_filesz
    (call $emit_qword (i64.extend_i32_u (local.get $memsz)))   ;; p_memsz
    (call $emit_qword (i64.const 0x1000))    ;; p_align
  )

  ;; ── Runtime stub: _start that calls compiled code and exits ──────
  ;; $bss_va = page-aligned start of BSS (right after compiled code)

  (func $emit_elf_stub (param $bss_va i32)
    (local $jitglobs i32) (local $mem i32) (local $locals i32)
    (local $globals i32) (local $table i32)

    (local.set $jitglobs (local.get $bss_va))
    (local.set $mem     (i32.add (local.get $bss_va) (i32.const 0x80)))
    (local.set $locals  (i32.add (local.get $bss_va) (i32.const 0x10080)))
    (local.set $globals (i32.add (local.get $bss_va) (i32.const 0x20080)))
    (local.set $table   (i32.add (local.get $bss_va) (i32.const 0x30080)))

    ;; mov r15, $jitglobs
    (call $emit_mov_r64_imm32 (i32.const 0x4D) (i32.const 0xC7) (local.get $jitglobs))

    ;; mov rax, $mem; mov [r15+8], rax   (JitGlobals.mem_ptr)
    (call $emit_mov_r64_imm32 (i32.const 0x48) (i32.const 0xC0) (local.get $mem))
    (call $emit_mov_r15off_rax (i32.const 8))

    ;; mov rax, $locals; mov [r15+0], rax  (JitGlobals.locals)
    (call $emit_mov_r64_imm32 (i32.const 0x48) (i32.const 0xC0) (local.get $locals))
    (call $emit_mov_r15off_rax (i32.const 0))

    ;; mov rax, $globals; mov [r15+24], rax (JitGlobals.globals_buf)
    (call $emit_mov_r64_imm32 (i32.const 0x48) (i32.const 0xC0) (local.get $globals))
    (call $emit_mov_r15off_rax (i32.const 24))

    ;; mov rax, $table; mov [r15+48], rax  (JitGlobals.table_entries)
    (call $emit_mov_r64_imm32 (i32.const 0x48) (i32.const 0xC0) (local.get $table))
    (call $emit_mov_r15off_rax (i32.const 48))

    ;; mov dword [r15+64], 1  (JitGlobals.memory_pages)
    (call $emit_mov_dword_r15off (i32.const 64) (i32.const 1))

    ;; call compiled_code
    (call $emit_call_rel (i32.add (global.get $TEXT_VA) (global.get $ELF_CODE_OFF)))

    ;; exit(result)  — rax holds the return value from compiled code
    (call $emit_mov_edi_eax)
    (call $emit_mov_eax_imm (global.get $LINUX_SYS_X64_EXIT))  ;; SYS_exit
    (call $emit_syscall)
  )

  ;; ── Copy compiled code from cache into ELF output ────────────────

  (func $copy_compiled_code (param $src i32) (param $size i32)
    (local $i i32) (local $dst i32)
    (local.set $dst (i32.add (global.get $ELF_OUT_BUF) (global.get $ELF_CODE_OFF)))
    (local.set $i (i32.const 0))
    (block $done
      (loop $copy
        (if (i32.ge_u (local.get $i) (local.get $size))
          (then (br $done))
        )
        (i32.store8
          (i32.add (local.get $dst) (local.get $i))
          (i32.load8_u (i32.add (local.get $src) (local.get $i)))
        )
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $copy)
      )
    )
  )

  ;; ── compile_to_elf: compile a WASM function and emit ELF64 ───────
  ;; Returns (start_address, total_size)
  (func (export "compile_to_elf") (param $func_idx i32) (result i32 i32)
    (local $code_size i32) (local $total_size i32)
    (local $saved i32) (local $bss_va i32)

    ;; 1. Run the compiler (produces code at 0x100000)
    (local.set $code_size (call $jit_compile (local.get $func_idx)))

    ;; 2. Compute total ELF file size
    (local.set $total_size (i32.add (global.get $ELF_CODE_OFF) (local.get $code_size)))

    ;; 3. Compute BSS base: page-align TEXT_VA + total_size
    (local.set $bss_va
      (i32.add
        (i32.and (i32.add (local.get $total_size) (i32.const 0xFFF)) (i32.const -0x1000))
        (global.get $TEXT_VA)
      )
    )

    ;; 4. Save JS_CODE_PTR, redirect emit to ELF output buffer
    (local.set $saved (i32.load (global.get $JS_CODE_PTR)))
    (i32.store (global.get $JS_CODE_PTR) (global.get $ELF_OUT_OFF))

    ;; 5. Emit ELF64 header (1 program header: text + BSS combined)
    (call $emit_elf64_ehdr
      (i32.add (global.get $TEXT_VA) (global.get $ELF_STUB_OFF))  ;; entry = _start
      (i32.const 64)   ;; e_phoff
      (i32.const 1)    ;; 1 program header (text + bss)
    )

    ;; 6. Emit PH#1: text + BSS combined LOAD (R|W|X), offset=0
    ;;     filesz = total_size, memsz includes BSS
    (call $emit_elf64_phdr
      (i32.const 1)         ;; PT_LOAD
      (i32.const 7)         ;; PF_R | PF_W | PF_X
      (i32.const 0)         ;; p_offset = 0
      (global.get $TEXT_VA) ;; p_vaddr
      (local.get $total_size) ;; p_filesz
      (i32.add (i32.and (i32.add (local.get $total_size) (i32.const 0xFFF)) (i32.const -0x1000)) (global.get $BSS_SIZE)) ;; p_memsz includes BSS
    )

    ;; 7. Emit runtime stub (with BSS base address)
    (call $emit_elf_stub (local.get $bss_va))

    ;; 8. Pad with zeros from end of stub to ELF_CODE_OFF
    (block $pad_done
      (loop $pad_loop
        (if (i32.ge_u (i32.load (global.get $JS_CODE_PTR))
                       (i32.add (global.get $ELF_OUT_OFF) (global.get $ELF_CODE_OFF)))
          (then (br $pad_done))
        )
        (call $emit_byte (i32.const 0))
        (br $pad_loop)
      )
    )

    ;; 9. Copy compiled code from cache (0x100000) to ELF output
    (call $copy_compiled_code (global.get $JIT_CACHE) (local.get $code_size))

    ;; 10. Restore JS_CODE_PTR
    (i32.store (global.get $JS_CODE_PTR) (local.get $saved))

    ;; Return (buffer_address, total_size)
    (return (global.get $ELF_OUT_BUF) (local.get $total_size))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Flat binary output (bare-metal x86-64)
  ;; ═════════════════════════════════════════════════════════════════════

  (global $BIN_OUT_BUF  i32 (i32.const 0x900000))
  (global $BIN_OUT_OFF  i32 (i32.const 0x800000))  ;; BIN_OUT_BUF - JIT_CACHE

  ;; MOV moffs32, eAX: 67 A3 <addr>
  (func $emit_mov_abs32_eax (param $addr i32)
    (call $emit_byte (i32.const 0x67))
    (call $emit_byte (i32.const 0xA3))
    (call $emit_dword (local.get $addr))
  )

  ;; CALL rel32 with displacement computed relative to the binary base
  ;; target_off = offset from start of binary to the target instruction
  (func $emit_call_flat (param $target_off i32)
    (local $saved i32)
    (local.set $saved (i32.load (global.get $JS_CODE_PTR)))
    (call $emit_byte (i32.const 0xE8))
    (call $emit_dword
      (i32.sub
        (local.get $target_off)
        (i32.add (i32.sub (local.get $saved) (global.get $BIN_OUT_OFF)) (i32.const 5))
      )
    )
  )

  ;; Emit bare-metal runtime stub (no ELF headers, starts at offset 0)
  ;; The compiled code will be placed right after the stub.
  (func $emit_bare_metal_stub (result i32)
    (local $stub_size i32)
    ;; Same init as ELF: set up r15, JitGlobals
    (call $emit_mov_r64_imm32 (i32.const 0x4D) (i32.const 0xC7) (global.get $BSS_JITGLOBALS))
    (call $emit_mov_r64_imm32 (i32.const 0x48) (i32.const 0xC0) (global.get $BSS_MEM))
    (call $emit_mov_r15off_rax (i32.const 8))
    (call $emit_mov_r64_imm32 (i32.const 0x48) (i32.const 0xC0) (global.get $BSS_LOCALS))
    (call $emit_mov_r15off_rax (i32.const 0))
    (call $emit_mov_r64_imm32 (i32.const 0x48) (i32.const 0xC0) (global.get $BSS_GLOBALS))
    (call $emit_mov_r15off_rax (i32.const 24))
    (call $emit_mov_r64_imm32 (i32.const 0x48) (i32.const 0xC0) (global.get $BSS_TABLE))
    (call $emit_mov_r15off_rax (i32.const 48))
    (call $emit_mov_dword_r15off (i32.const 64) (i32.const 1))
    ;; After init, compute the eventual stub size, then emit CALL placeholder.
    ;; The CALL's target will be the start of compiled code (offset = stub_size).
    ;; We know the instructions after CALL are fixed size: 6 (mov_abs) + 1 (cli) + 1 (hlt) = 8 bytes.
    ;; So if we save current offset before CALL and add CALL = 5 + 8 = 13, we get stub_size.
    (local.set $stub_size (i32.add (i32.sub (i32.load (global.get $JS_CODE_PTR)) (global.get $BIN_OUT_OFF)) (i32.const 13)))
    ;; Emit CALL to compiled code at offset stub_size
    (call $emit_call_flat (local.get $stub_size))
    ;; Store result at 0x500 and halt
    (call $emit_mov_abs32_eax (i32.const 0x500))
    (call $emit_byte (i32.const 0xFA))   ;; CLI
    (call $emit_byte (i32.const 0xF4))   ;; HLT
    (return (local.get $stub_size))
  )

  ;; Copy compiled code from cache to output buffer at given offset
  (func $copy_code_to (param $dst_buf i32) (param $code_size i32) (param $code_off i32)
    (local $i i32) (local $dst i32)
    (local.set $dst (i32.add (local.get $dst_buf) (local.get $code_off)))
    (local.set $i (i32.const 0))
    (block $done
      (loop $copy
        (if (i32.ge_u (local.get $i) (local.get $code_size)) (then (br $done)))
        (i32.store8
          (i32.add (local.get $dst) (local.get $i))
          (i32.load8_u (i32.add (global.get $JIT_CACHE) (local.get $i)))
        )
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $copy)
      )
    )
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; compile_to_bin: emit flat binary with bare-metal runtime stub
  ;; Returns (buffer_address, total_size)
  ;; ═════════════════════════════════════════════════════════════════════
  (func (export "compile_to_bin") (param $func_idx i32) (result i32 i32)
    (local $code_size i32) (local $stub_size i32) (local $total_size i32)
    (local $saved i32)

    ;; 1. Run the compiler
    (local.set $code_size (call $jit_compile (local.get $func_idx)))

    ;; 2. Redirect emit to binary output buffer
    (local.set $saved (i32.load (global.get $JS_CODE_PTR)))
    (i32.store (global.get $JS_CODE_PTR) (global.get $BIN_OUT_OFF))

    ;; 3. Emit bare-metal stub — returns stub size (where compiled code goes)
    (local.set $stub_size (call $emit_bare_metal_stub))

    ;; 4. Copy compiled code after stub
    (call $copy_code_to (global.get $BIN_OUT_BUF) (local.get $code_size) (local.get $stub_size))

    ;; 5. Total size = stub_size + code_size
    (local.set $total_size (i32.add (local.get $stub_size) (local.get $code_size)))

    ;; 6. Restore JS_CODE_PTR
    (i32.store (global.get $JS_CODE_PTR) (local.get $saved))

    (return (global.get $BIN_OUT_BUF) (local.get $total_size))
  )
