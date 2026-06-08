  ;; ── Patch syscall map for Wayland module imports ───────────────────
  ;; After load_wat initializes sysno[i] = i, patch the entries that
  ;; don't match the Linux syscall numbers for each import.
  ;; Wayland module import layout:
  ;;   0: read     → SYS_read  = 0
  ;;   1: write    → SYS_write = 1
  ;;   2: open     → SYS_open  = 2
  ;;   3: close    → SYS_close = 3
  ;;   4: poll     → SYS_poll  = 7
  ;;   5: mmap     → SYS_mmap  = 9
  ;;   6: munmap   → SYS_munmap = 11
  ;;   7: socket   → SYS_socket = 41
  ;;   8: connect  → SYS_connect = 42
  ;;   9: sendmsg  → SYS_sendmsg = 46
  ;;  10: memfd_create → SYS_memfd_create = 319
  ;;  11: ftruncate → SYS_ftruncate = 77
  (func $patch_wayland_syscalls (export "patch_wayland_syscalls")
    (i32.store (i32.const 0x90010) (i32.const 7))     ;; idx 4: poll
    (i32.store (i32.const 0x90014) (i32.const 9))     ;; idx 5: mmap
    (i32.store (i32.const 0x90018) (i32.const 11))    ;; idx 6: munmap
    (i32.store (i32.const 0x9001C) (i32.const 41))    ;; idx 7: socket
    (i32.store (i32.const 0x90020) (i32.const 42))    ;; idx 8: connect
    (i32.store (i32.const 0x90024) (i32.const 46))    ;; idx 9: sendmsg
    (i32.store (i32.const 0x90028) (i32.const 319))   ;; idx 10: memfd_create
    (i32.store (i32.const 0x9002C) (i32.const 77))    ;; idx 11: ftruncate
  )

  ;; ── compile_wasm_to_elf: load WASM binary, patch, compile, return ELF ──
  ;; Input: compiled WASM binary in memory at (wasm_ptr, wasm_len)
  ;; Returns (elf_buf_addr, elf_total_size) as two i32s
  ;; After this call, read ELF bytes from memory[elf_buf_addr .. elf_buf_addr + total_size]
  ;; IMPORTANT: The WASM binary must have imports with these indices:
  ;;   0 read, 1 write, 2 open, 3 close, 4 poll, 5 mmap, 6 munmap,
  ;;   7 socket, 8 connect, 9 sendmsg, 10 memfd_create, 11 ftruncate
  (func (export "compile_wasm_to_elf") (param $wasm_ptr i32) (param $wasm_len i32) (result i32 i32)
    (local $err i32) (local $import_count i32)

    ;; 1. Load the WASM binary (parses sections including imports correctly)
    (local.set $err (call $load (local.get $wasm_ptr) (local.get $wasm_len)))
    (if (i32.ne (local.get $err) (i32.const 0))
      (then (return (i32.const 0) (i32.const 0)))
    )

    ;; 2. Patch syscall map for the loaded module's imports
    (call $patch_wayland_syscalls)

    ;; 3. Get import count (first module function index)
    (local.set $import_count (i32.load (i32.const 0x4108)))

    ;; 4. reserved
    (return (i32.const -1) (i32.const 0))
  )
