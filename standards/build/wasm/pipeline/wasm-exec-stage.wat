(module
  (import "edgerun-core" "memory" (memory 1))
  (import "pipe-core" "pipe_read" (func $pipe_read (param i32 i32 i32) (result i32)))
  (import "pipe-core" "pipe_write" (func $pipe_write (param i32 i32 i32) (result i32)))
  (import "wasm-interpreter" "load" (func $interp_load (param i32 i32) (result i32)))
  (import "wasm-interpreter" "call" (func $interp_call (param i32 i32 i32) (result i32)))
  (import "wasm-interpreter" "get_result_value" (func $get_result_value (param i32) (result i64)))
  (import "wasm-interpreter" "get_result_count" (func $get_result_count (result i32)))

  (func (export "proto_standard_id") (result i32) i32.const 300200)

  ;; Config (variable length):
  ;;   +0: func_idx  i32  (function index; -1 = function 0)
  ;;   +4: arg_count i32  (number of i32 arguments)
  ;;   +8: args[]    i32  (inline argument values)

  (func (export "process_wasm_exec")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $wasm_len i32) (local $err i32) (local $func_idx i32)
    (local $arg_count i32) (local $arg_ptr i32)
    (local $res_count i32) (local $result i64)

    ;; 1. Read WASM binary from input pipe into scratch
    (local.set $wasm_len (call $pipe_read (local.get $input) (local.get $scratch) (local.get $scap)))
    (if (i32.le_s (local.get $wasm_len) (i32.const 0))
      (then (return (local.get $wasm_len))))

    ;; 2. Load WASM binary via interpreter
    (local.set $err (call $interp_load (local.get $scratch) (local.get $wasm_len)))
    (if (local.get $err)
      (then (return (i32.sub (i32.const 0) (local.get $err)))))

    ;; 3. Parse config
    (local.set $func_idx (i32.const 0))
    (local.set $arg_count (i32.const 0))
    (local.set $arg_ptr (i32.const 0))
    (if (i32.ge_s (local.get $clen) (i32.const 4))
      (then
        (local.set $func_idx (i32.load (local.get $cfg)))
        (if (i32.eq (local.get $func_idx) (i32.const -1))
          (then (local.set $func_idx (i32.const 0))))))
    (if (i32.ge_s (local.get $clen) (i32.const 8))
      (then
        (local.set $arg_count (i32.load offset=4 (local.get $cfg)))
        (if (i32.gt_s (local.get $arg_count) (i32.const 0))
          (then
            (local.set $arg_ptr (i32.add (local.get $cfg) (i32.const 8)))))))

    ;; 4. Call function with args
    (local.set $err (call $interp_call (local.get $func_idx) (local.get $arg_ptr) (local.get $arg_count)))
    (if (local.get $err)
      (then (return (i32.sub (i32.const 0) (local.get $err)))))

    ;; 5. Read first result value and write to output
    (local.set $res_count (call $get_result_count))
    (if (i32.gt_s (local.get $res_count) (i32.const 0))
      (then
        (local.set $result (call $get_result_value (i32.const 0)))
        (i32.store (local.get $scratch) (i32.wrap_i64 (local.get $result)))
        (drop (call $pipe_write (local.get $output) (local.get $scratch) (i32.const 4)))
        (return (i32.const 4))))

    ;; No result — return 0 bytes written
    (i32.const 0))
)
