;; ── CLI Core: syscall wrappers and I/O primitives ──
  ;; Memory layout for I/O operations:
  ;;   [0..3]   iov_base (i32)
  ;;   [4..7]   iov_len  (i32)
  ;;   [8..11]  result buffer (nwritten / nread)

  (global $IOVEC i32 (i32.const 0))
  (global $IOVEC_LEN i32 (i32.const 4))
  (global $RESULT_BUF i32 (i32.const 8))

  ;; Exit with code.
  (func $exit (export "exit") (param $code i32)
    local.get $code
    call $proc_exit
    unreachable)

  ;; Write buffer to fd. Returns bytes written (or error code).
  (func $write_all (export "write_all") (param $fd i32) (param $buf i32) (param $len i32) (result i32)
    i32.const 0
    local.get $buf
    i32.store
    i32.const 4
    local.get $len
    i32.store
    local.get $fd
    i32.const 0
    i32.const 1
    i32.const 8
    call $fd_write
    if (result i32)
      i32.const -1
    else
      i32.const 8
      i32.load
    end)
