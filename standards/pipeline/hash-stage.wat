;; Consolidated Hash Pipeline Stages — slots 16-20
  ;; Stage type: batch (state=0)
  ;; Input:  raw bytes via input pipe
  ;; Output: hash digest via output pipe
  ;; Uses $SCRATCH_BUF (0x3000) for input and $SHA256_OUT_BUF (0x5000) for output.

  ;; SHA-256 — slot 16, 32-byte digest
  (func $process_sha256 (export "process_sha256")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $read i32) (local $result i64)
    (local.set $read (call $stage_read_input (local.get $input) (local.get $scratch) (local.get $scap)))
    (if (i32.eqz (local.get $read)) (then (return (i32.const 0))))
    (local.set $result (call $sha256 (global.get $SCRATCH_BUF) (local.get $read) (global.get $SHA256_OUT_BUF)))
    (call $stage_write_result (local.get $output) (local.get $result) (global.get $SHA256_OUT_BUF)))

  ;; SHA-1 — slot 18, 20-byte digest
  (func $process_sha1 (export "process_sha1")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $read i32) (local $result i64)
    (local.set $read (call $stage_read_input (local.get $input) (local.get $scratch) (local.get $scap)))
    (if (i32.eqz (local.get $read)) (then (return (i32.const 0))))
    (local.set $result (call $sha1 (global.get $SCRATCH_BUF) (local.get $read) (global.get $SHA256_OUT_BUF)))
    (call $stage_write_result (local.get $output) (local.get $result) (global.get $SHA256_OUT_BUF)))

  ;; SHA-512 — slot 19, 64-byte digest
  (func $process_sha512 (export "process_sha512")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $read i32) (local $result i64)
    (local.set $read (call $stage_read_input (local.get $input) (local.get $scratch) (local.get $scap)))
    (if (i32.eqz (local.get $read)) (then (return (i32.const 0))))
    (local.set $result (call $sha512 (global.get $SCRATCH_BUF) (local.get $read) (global.get $SHA256_OUT_BUF)))
    (call $stage_write_result (local.get $output) (local.get $result) (global.get $SHA256_OUT_BUF)))

  ;; SHA-384 — slot 20, 48-byte digest
  (func $process_sha384 (export "process_sha384")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $read i32) (local $result i64)
    (local.set $read (call $stage_read_input (local.get $input) (local.get $scratch) (local.get $scap)))
    (if (i32.eqz (local.get $read)) (then (return (i32.const 0))))
    (local.set $result (call $sha384 (global.get $SCRATCH_BUF) (local.get $read) (global.get $SHA256_OUT_BUF)))
    (call $stage_write_result (local.get $output) (local.get $result) (global.get $SHA256_OUT_BUF)))
