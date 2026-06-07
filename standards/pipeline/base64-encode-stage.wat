;; Base64 Encode Pipeline Stage — slot 3
  ;; Stage type: batch (state=0)
  ;; Input:  raw bytes via input pipe
  ;; Output: Base64-encoded text via output pipe (RFC 4648, `+/` alphabet, `=` pad)
  ;; Calls $base64_encode from codec/encoding-base64.wat (merged in module scope)

  ;; Pipeline stage: Base64 standard encode (batch, copy input to safe buffer)
  ;; We copy input to 0x3000, write output to 0x5000, then forward to output pipe.
  ;; Output length = ceil(input_len / 3) * 4.
  (func $process_base64_encode (export "process_base64_encode")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $read i32) (local $result i64)
    (local $status i32)
    (local $out_len i32)
    (local.set $read (call $stage_read_input (local.get $input) (local.get $scratch) (local.get $scap)))
    (if (i32.eqz (local.get $read)) (then (return (i32.const 0))))
    (local.set $result (call $base64_encode (i32.const 0x3000) (local.get $read) (i32.const 0x5000) (i32.const 64000)))
    (local.set $status (i32.wrap_i64 (i64.shr_u (local.get $result) (i64.const 32))))
    (if (local.get $status) (then (return (i32.sub (i32.const 0) (local.get $status)))))
    (local.set $out_len (i32.wrap_i64 (local.get $result)))
    (drop (call $pipe_write (local.get $output) (i32.const 0x5000) (local.get $out_len)))
    local.get $out_len)
