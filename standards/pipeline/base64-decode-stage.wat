;; Base64 Decode Pipeline Stage — slot 4
  ;; Stage type: batch (state=0)
  ;; Input:  Base64 text via input pipe (RFC 4648, `+/` alphabet, `=` pad)
  ;; Output: Decoded raw bytes via output pipe
  ;; Calls $base64_decode from codec/encoding-base64.wat (merged in module scope)

  ;; Pipeline stage: Base64 standard decode (batch, copy input to safe buffer)
  ;; We copy input to 0x3000, write output to 0x5000, then forward to output pipe.
  (func $process_base64_decode (export "process_base64_decode")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $read i32) (local $result i64)
    (local $status i32)
    (local $out_len i32)
    (local.set $read (call $stage_read_input (local.get $input) (local.get $scratch) (local.get $scap)))
    (if (i32.eqz (local.get $read)) (then (return (i32.const 0))))
    (local.set $result (call $base64_decode (global.get $SCRATCH_BUF) (local.get $read) (global.get $SHA256_OUT_BUF) (global.get $BUF_SIZE_64K)))
    (local.set $status (i32.wrap_i64 (i64.shr_u (local.get $result) (i64.const 32))))
    (if (local.get $status) (then (return (i32.sub (i32.const 0) (local.get $status)))))
    (local.set $out_len (i32.wrap_i64 (local.get $result)))
    (drop (call $pipe_write (local.get $output) (global.get $SHA256_OUT_BUF) (local.get $out_len)))
    local.get $out_len)
