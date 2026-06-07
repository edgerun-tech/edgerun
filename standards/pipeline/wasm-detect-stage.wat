;; WASM Detect Stage — slot 13
  ;; Stage type: batch (state=0)
  ;; Input:  raw bytes from input pipe
  ;; Output: 1-byte format flag (0=WAT, 1=WASM binary) + raw bytes
  (func $process_wasm_detect (export "process_wasm_detect")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $len i32) (local $flag i32)
    (local.set $len (call $pipe_read (local.get $input) (local.get $scratch) (local.get $scap)))
    (if (i32.le_s (local.get $len) (i32.const 0))
      (then (return (local.get $len))))
    (local.set $flag (i32.const 0))
    (if (i32.eq (i32.load (local.get $scratch)) (i32.const 0x6D736100))
      (then (local.set $flag (i32.const 1))))
    (i32.store8 (local.get $scratch) (local.get $flag))
    (drop (call $pipe_write (local.get $output) (local.get $scratch) (i32.add (local.get $len) (i32.const 1))))
    (i32.add (local.get $len) (i32.const 1)))
