;; SHA-1 Hash Pipeline Stage — slot 18
  ;; Stage type: batch (state=0)
  ;; Input:  raw bytes via input pipe
  ;; Output: 20-byte SHA-1 hash via output pipe
  ;; Calls $sha1 from crypto/crypto-sha1.wat (merged in module scope)

  ;; Pipeline stage: SHA-1 hash (batch, copy input to safe buffer)
  ;; SHA-1 code requires input + output buffers below 65536.
  ;; We copy input to a fixed safe buffer at 0x3000, write output to 0x5000,
  ;; then forward the 20-byte hash to the output pipe.
  (func $process_sha1 (export "process_sha1")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)

    (local $result i64) (local $status i32) (local $read i32)
    (local.set $read (call $stage_read_input (local.get $input) (local.get $scratch) (local.get $scap)))
    (if (i32.eqz (local.get $read)) (then (return (i32.const 0))))
    (local.set $result (call $sha1 (i32.const 0x3000) (local.get $read) (i32.const 0x5000)))
    (local.set $status (i32.wrap_i64 (i64.shr_u (local.get $result) (i64.const 32))))
    (if (local.get $status) (then (return (i32.sub (i32.const 0) (local.get $status)))))
    (drop (call $pipe_write (local.get $output) (i32.const 0x5000) (i32.const 20)))
    i32.const 20)
