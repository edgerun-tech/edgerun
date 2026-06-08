(module
  ;; ── Direct syscall imports (standalone, no build script needed) ──
  (import "wasi_snapshot_preview1" "fd_write" (func $fd_write (param i32 i32 i32 i32) (result i32)))
  (import "wasi_snapshot_preview1" "proc_exit" (func $proc_exit (param i32)))

  (memory (export "memory") 1)

  ;; ── Hello data ──
  (data (i32.const 0) "Hello, World!\n")

  ;; ── Main ──
  (func (export "main") (result i32)
    ;; iovec at address 16
    i32.const 16
    i32.const 0
    i32.store          ;; iov_base = ptr to "Hello..."

    i32.const 20
    i32.const 14
    i32.store          ;; iov_len = 14

    i32.const 24
    i32.const 0
    i32.store          ;; nwritten location

    i32.const 1        ;; fd = stdout
    i32.const 16       ;; iovs
    i32.const 1        ;; iovs_len
    i32.const 24       ;; nwritten
    call $fd_write

    drop               ;; discard return value

    i32.const 0        ;; exit code 0
    return
  )
)
