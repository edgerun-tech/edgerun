;; DJB2 Hash Pipeline Stage — slot 24
  ;; Stage type: batch (state=0)
  ;; Input:  raw bytes via input pipe
  ;; Output: 4-byte DJB2 hash (little-endian i32) via output pipe
  ;; Calls $djb2_hash from crypto/hash-djb2.wat
  (func $process_djb2_hash (export "process_djb2_hash")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $len_slot i32) (local $read i32) (local $hash i32)
    (local.set $len_slot (i32.sub (i32.add (local.get $scratch) (local.get $scap)) (i32.const 4)))
    (drop (call $pipe_read_ptr (local.get $input) (local.get $len_slot)))
    (local.set $read (i32.load (local.get $len_slot)))
    (if (i32.eqz (local.get $read)) (then (return (i32.const 0))))
    (drop (call $pipe_read (local.get $input) (i32.const 0x3000) (local.get $read)))
    (local.set $hash (call $djb2_hash (i32.const 0x3000) (local.get $read)))
    (i32.store (i32.const 0x5000) (local.get $hash))
    (drop (call $pipe_write (local.get $output) (i32.const 0x5000) (i32.const 4)))
    i32.const 4)
