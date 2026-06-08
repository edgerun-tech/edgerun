  ;; ── User app: Hello World ──
  (data (i32.const 0x100) "Hello, World!\n")

  (func (export "main") (result i32)
    i32.const 1          ;; fd = stdout
    i32.const 0x100      ;; buf
    i32.const 14         ;; len
    call $write_all
    drop
    i32.const 0
    return
    unreachable)
