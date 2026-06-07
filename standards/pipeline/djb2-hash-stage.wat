;; DJB2 Hash Pipeline Stage — slot 24
  ;; Stage type: batch (state=0)
  ;; Input:  raw bytes via input pipe
  ;; Output: 4-byte DJB2 hash (little-endian i32) via output pipe
  ;; Calls $djb2_hash from crypto/hash-djb2.wat
  (func $process_djb2_hash (export "process_djb2_hash")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $hash i32)
    (local $read i32)
    (local.set $read (call $stage_read_input (local.get $input) (local.get $scratch) (local.get $scap)))
    (if (i32.eqz (local.get $read)) (then (return (i32.const 0))))
    (local.set $hash (call $djb2_hash (global.get $SCRATCH_BUF) (local.get $read)))
    (i32.store (global.get $SHA256_OUT_BUF) (local.get $hash))
    (drop (call $pipe_write (local.get $output) (global.get $SHA256_OUT_BUF) (i32.const 4)))
    i32.const 4)
