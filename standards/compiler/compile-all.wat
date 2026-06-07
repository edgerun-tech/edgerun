  ;; ═════════════════════════════════════════════════════════════════════
  ;; compile_all_to_elf_x86_64 — compile ALL non-import functions to ELF
  ;;
  ;; Compiles every non-import function in the currently loaded module,
  ;; fixes up cross-function calls, and wraps them in a single ELF64
  ;; binary. The first non-import function (function index = import_count)
  ;; becomes the ELF entry point (_start).
  ;;
  ;; Precondition: $load(wasm_ptr, wasm_len) must have been called to
  ;; populate interpreter state (import_count at 0x4108, func_count at
  ;; 0x4510, decoded ops at 0xA0000, syscall map at 0x90000).
  ;;
  ;; Returns: (elf_buffer_address, elf_total_size)
  ;; ═════════════════════════════════════════════════════════════════════
  (func $compile_all_to_elf_x86_64 (export "compile_all_to_elf_x86_64") (result i32 i32)
    (local $import_count i32) (local $func_count i32) (local $i i32)
    (local $code_size i32) (local $total_size i32) (local $syscall_data_size i32)
    (local $saved i32) (local $bss_va i32) (local $data_va i32)

    ;; ── 1. Read interpreter state ──
    (local.set $import_count (i32.load (i32.const 0x4108)))
    (local.set $func_count (i32.load (i32.const 0x4510)))

    ;; ── 2. Reset JIT state ──
    (i32.store (global.get $JS_CODE_PTR_x86_64) (i32.const 0))
    (i32.store (global.get $JS_FIXUP_COUNT_x86_64) (i32.const 0))

    ;; ── 3. Compile every non-import function ──
    (local.set $i (local.get $import_count))
    (block $compile_done
      (loop $compile_loop
        (if (i32.ge_u (local.get $i) (local.get $func_count))
          (then (br $compile_done))
        )
        (drop (call $jit_compile_x86_64 (local.get $i)))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $compile_loop)
      )
    )

    ;; ── 4. Fix up cross-function calls ──
    (call $fixup_calls_x86_64)

    ;; ── 5. Compute sizes ──
    (local.set $code_size (i32.load (global.get $JS_CODE_PTR_x86_64)))
    (local.set $syscall_data_size (i32.shl (local.get $import_count) (i32.const 2)))
    (local.set $total_size
      (i32.add
        (i32.add (global.get $ELF_CODE_OFF_x86_64) (local.get $code_size))
        (local.get $syscall_data_size)
      )
    )

    ;; BSS VA = page-aligned TEXT_VA + total_size
    (local.set $bss_va
      (i32.add
        (i32.and (i32.add (local.get $total_size) (i32.const 0xFFF)) (i32.const -0x1000))
        (global.get $TEXT_VA_x86_64)
      )
    )

    ;; Syscall data VA = TEXT_VA + ELF_CODE_OFF + code_size
    (local.set $data_va
      (i32.add
        (global.get $TEXT_VA_x86_64)
        (i32.add (global.get $ELF_CODE_OFF_x86_64) (local.get $code_size))
      )
    )

    ;; ── 6. Save JS_CODE_PTR, redirect to ELF output buffer ──
    (local.set $saved (i32.load (global.get $JS_CODE_PTR_x86_64)))
    (i32.store (global.get $JS_CODE_PTR_x86_64) (global.get $ELF_OUT_OFF_x86_64))

    ;; ── 7. Emit ELF64 header (1 program header) ──
    (call $emit_elf64_ehdr
      (i32.add (global.get $TEXT_VA_x86_64) (global.get $ELF_STUB_OFF_x86_64))
      (i32.const 64)
      (i32.const 1)
    )

    ;; ── 8. Emit program header (text + BSS combined LOAD with R|W|X) ──
    (call $emit_elf64_phdr
      (i32.const 1)
      (i32.const 7)
      (i32.const 0)
      (global.get $TEXT_VA_x86_64)
      (local.get $total_size)
      (i32.add
        (i32.and (i32.add (local.get $total_size) (i32.const 0xFFF)) (i32.const -0x1000))
        (global.get $BSS_SIZE_x86_64)
      )
    )

    ;; ── 9. Emit runtime stub ──
    (call $emit_elf_stub (local.get $bss_va) (local.get $data_va) (local.get $import_count))

    ;; ── 10. Pad with zeros from end of stub to ELF_CODE_OFF ──
    (block $pad_done
      (loop $pad_loop
        (if (i32.ge_u (i32.load (global.get $JS_CODE_PTR_x86_64))
                       (i32.add (global.get $ELF_OUT_OFF_x86_64) (global.get $ELF_CODE_OFF_x86_64)))
          (then (br $pad_done))
        )
        (call $emit_x86_byte (i32.const 0))
        (br $pad_loop)
      )
    )

    ;; ── 11. Copy all compiled code from JIT_CACHE to ELF output ──
    (call $copy_compiled_code_x86_64 (global.get $JIT_CACHE) (local.get $code_size))

    ;; ── 12. Advance JS_CODE_PTR past the compiled code ──
    (i32.store (global.get $JS_CODE_PTR_x86_64)
      (i32.add (i32.load (global.get $JS_CODE_PTR_x86_64)) (local.get $code_size))
    )

    ;; ── 13. Write syscall map data (from interpreter's syscall map at 0x90000) ──
    (local.set $i (i32.const 0))
    (block $data_loop_done
      (loop $data_loop
        (if (i32.ge_u (local.get $i) (local.get $import_count))
          (then (br $data_loop_done))
        )
        (call $emit_x86_dword
          (i32.load (i32.add (i32.const 0x90000) (i32.shl (local.get $i) (i32.const 2))))
        )
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $data_loop)
      )
    )

    ;; ── 14. Restore JS_CODE_PTR ──
    (i32.store (global.get $JS_CODE_PTR_x86_64) (local.get $saved))

    (return (global.get $ELF_OUT_BUF_x86_64) (local.get $total_size))
  )
