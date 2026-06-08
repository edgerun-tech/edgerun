(module
  ;; ── Direct syscall imports (standalone, no build script needed) ──
  (import "wasi_snapshot_preview1" "fd_write" (func $fd_write (param i32 i32 i32 i32) (result i32)))
  (import "wasi_snapshot_preview1" "fd_read" (func $fd_read (param i32 i32 i32 i32) (result i32)))
  (import "wasi_snapshot_preview1" "proc_exit" (func $proc_exit (param i32)))

  (memory (export "memory") 1)

  ;; ── Buffer for reading ──
  (data (i32.const 0) "Hello, World!\n")
  (data (i32.const 100) "read stdin...\n")
  (data (i32.const 200) "stdin content: ")

  ;; ── Helper: write buffer to stdout ──
  ;; iovec at 16, nwritten at 24
  (func $write_stdout (param $buf i32) (param $len i32)
    i32.const 16
    local.get $buf
    i32.store
    i32.const 20
    local.get $len
    i32.store
    i32.const 24
    i32.const 0
    i32.store
    i32.const 1
    i32.const 16
    i32.const 1
    i32.const 24
    call $fd_write
    drop)

  ;; ── Helper: write string to stdout ──
  (func $print (param $s i32)
    (local $len i32)
    (local $p i32)
    local.get $s
    local.set $p
    block $done
      loop $scan
        local.get $p
        i32.load8_u
        i32.eqz
        br_if $done
        local.get $p
        i32.const 1
        i32.add
        local.set $p
        br $scan
      end
    end
    local.get $p
    local.get $s
    i32.sub
    local.set $len
    local.get $s
    local.get $len
    call $write_stdout)

  ;; ── Main ──
  (func (export "main") (result i32)
    ;; Print "Hello, World!\n"
    i32.const 0
    i32.const 14
    call $write_stdout

    ;; Print "read stdin...\n"
    i32.const 100
    i32.const 14
    call $write_stdout

    ;; iovec at 16, fd_read reads into buf at 1024
    i32.const 16
    i32.const 1024
    i32.store           ;; iov_base = 1024
    i32.const 20
    i32.const 4096
    i32.store           ;; iov_len = 4096
    i32.const 24
    i32.const 0
    i32.store           ;; nread = 0

    i32.const 0         ;; fd = stdin
    i32.const 16        ;; iovs
    i32.const 1         ;; iovs_len
    i32.const 24        ;; nread
    call $fd_read
    drop                ;; discard error

    i32.const 24
    i32.load            ;; actual bytes read
    local.set 0

    ;; Print "stdin content: "
    i32.const 200
    call $print

    ;; Print what we read
    i32.const 1024
    local.get 0
    call $write_stdout

    ;; Print newline
    i32.const 10
    i32.const 1
    call $write_stdout

    i32.const 0
    return
  )
)
