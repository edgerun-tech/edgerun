;; JSON Parse Pipeline Stage — slot 33
  ;; Stage type: batch (state=0)
  ;; Input:  JSON text bytes via input pipe
  ;; Output: token tape (20 bytes per token) via output pipe
  ;;         followed by 4-byte token count at the end
  ;; Calls $json_parse_tape from codec/json.wat

  (func $process_json_parse (export "process_json_parse")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)

    (local $result i64) (local $status i32) (local $token_count i32) (local $read i32)
    (local $token_ptr i32) (local $token_cap i32)
    (local $scratch_end i32) (local $scratch_len i32)
    (local.set $read (call $stage_read_input (local.get $input) (local.get $scratch) (local.get $scap)))
    (if (i32.eqz (local.get $read)) (then (return (i32.const 0))))
    (local.set $scap (i32.sub (local.get $scap) (i32.const 4)))
    (local.set $token_ptr (local.get $scratch))
    (local.set $token_cap (i32.div_u (local.get $scap) (i32.const 20)))
    (local.set $scratch_end (i32.add (local.get $scratch) (local.get $scap)))
    (local.set $scratch_len (i32.sub (local.get $scap) (i32.mul (local.get $token_cap) (i32.const 20))))
    (local.set $result (call $json_parse_tape
      (global.get $SCRATCH_BUF) (local.get $read)
      (local.get $token_ptr) (local.get $token_cap)
      (local.get $scratch_end) (local.get $scratch_len)))
    (local.set $status (i32.wrap_i64 (i64.shr_u (local.get $result) (i64.const 32))))
    (if (local.get $status) (then (return (i32.sub (i32.const 0) (local.get $status)))))
    (local.set $token_count (i32.wrap_i64 (local.get $result)))
    (local.set $read (i32.mul (local.get $token_count) (i32.const 20)))
    (i32.store (i32.add (local.get $token_ptr) (local.get $read)) (local.get $token_count))
    (drop (call $pipe_write (local.get $output) (local.get $token_ptr) (i32.add (local.get $read) (i32.const 4))))
    (i32.add (local.get $read) (i32.const 4)))
