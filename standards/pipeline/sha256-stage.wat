;; SHA-256 Hash Pipeline Stage — slot 16
  ;; Stage type: batch (state=0)
  ;; Input:  raw bytes via input pipe
  ;; Output: 32-byte SHA-256 hash via output pipe
  ;; Calls $sha256 from crypto/crypto-sha256.wat (merged in module scope)

  ;; Pipeline stage: SHA-256 hash (batch, copy input to safe buffer)
  ;; SHA-256 code requires input + output buffers below 65536.
  ;; We copy input to a fixed safe buffer at 0x3000, write output to 0x5000,
  ;; then forward the 32-byte hash to the output pipe.
  (func $process_sha256 (export "process_sha256")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $len_slot i32) (local $read i32)
    (local $result i64) (local $status i32)
    (local.set $len_slot (i32.sub (i32.add (local.get $scratch) (local.get $scap)) (i32.const 4)))
    (drop (call $pipe_read_ptr (local.get $input) (local.get $len_slot)))
    (local.set $read (i32.load (local.get $len_slot)))
    (if (i32.eqz (local.get $read)) (then (return (i32.const 0))))
    (drop (call $pipe_read (local.get $input) (i32.const 0x3000) (local.get $read)))
    (local.set $result (call $sha256 (i32.const 0x3000) (local.get $read) (i32.const 0x5000)))
    (local.set $status (i32.wrap_i64 (i64.shr_u (local.get $result) (i64.const 32))))
    (if (local.get $status) (then (return (i32.sub (i32.const 0) (local.get $status)))))
    (drop (call $pipe_write (local.get $output) (i32.const 0x5000) (i32.const 32)))
    i32.const 32)
