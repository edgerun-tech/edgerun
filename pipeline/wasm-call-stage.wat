;; WASM Call Stage — slot 144
  ;; Stage type: batch (state=0)
  ;; Input:  trigger marker from previous stage
  ;; Output: result value (4 bytes) to output pipe
  ;; Config: +0: func_idx i32, +4: arg_count i32, +8: args[] i32
  (func $process_wasm_call (export "process_wasm_call")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $len i32) (local $func_idx i32) (local $arg_count i32)
    (local $arg_ptr i32) (local $err i32) (local $res_count i32)
    (local $result i64)
    (local.set $len (call $pipe_read (local.get $input) (local.get $scratch) (i32.const 4)))
    (if (i32.le_s (local.get $len) (i32.const 0))
      (then (return (i32.const 0))))
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
    (local.set $err (call $call (local.get $func_idx) (local.get $arg_ptr) (local.get $arg_count)))
    (if (local.get $err)
      (then (return (i32.sub (i32.const 0) (local.get $err)))))
    (local.set $res_count (call $get_result_count))
    (if (i32.gt_s (local.get $res_count) (i32.const 0))
      (then
        (local.set $result (call $get_result_value (i32.const 0)))
        (i32.store (local.get $scratch) (i32.wrap_i64 (local.get $result)))
        (drop (call $pipe_write (local.get $output) (local.get $scratch) (i32.const 4)))
        (return (i32.const 4))))
    (i32.const 0))
