  ;; ── ELF entry point — becomes the first non-import function ──
  ;; Protocol for self-hosting:
  ;;   Caller writes WASM binary to memory
  ;;   Stores wasm_ptr at 0x800000, wasm_len at 0x800004
  ;;   Calls _start() (no args, no return)
  ;;   After return, reads elf_addr from 0x800008, elf_size from 0x80000C
  ;;   ELF bytes are at memory[elf_addr .. elf_addr + elf_size]
  (func (export "_start")
    (local $ptr i32) (local $len i32)
    (local $elf_addr i32) (local $elf_size i32)
    (local $err i32)

    (local.set $ptr (i32.load (i32.const 0x800000)))
    (local.set $len (i32.load (i32.const 0x800004)))
    (if (i32.eqz (local.get $ptr))
      (then (return))
    )
    (if (i32.eqz (local.get $len))
      (then (return))
    )
    (local.set $err (call $load (local.get $ptr) (local.get $len)))
    (if (i32.ne (local.get $err) (i32.const 0))
      (then (return))
    )
    (call $compile_all_to_elf_x86_64)
    (local.set $elf_size)
    (local.set $elf_addr)
    (i32.store (i32.const 0x800008) (local.get $elf_addr))
    (i32.store (i32.const 0x80000C) (local.get $elf_size))
  )
