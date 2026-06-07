;; Generic TLV Decode Stage — slot 67
  ;; Input:  raw TLV bytes via input pipe (offset embedded in first 4 bytes)
  ;; Output: 20-byte record {tag, value_off, value_len, header_len, total_len}
  ;; Calls $generic_tlv_next from codec/binary.wat

  (func $process_generic_tlv_decode (export "process_generic_tlv_decode")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $offset i32)
    (local $read i32)
    (local $result i64) (local $status i32) (local $next_off i32)
    (local.set $read (call $stage_read_input (local.get $input) (local.get $scratch) (local.get $scap)))
    (if (i32.lt_u (local.get $read) (i32.const 8)) (then (return (i32.const 0))))
    (local.set $offset (i32.load (global.get $SCRATCH_BUF)))
    (local.set $result (call $generic_tlv_next (i32.add (global.get $SCRATCH_BUF) (i32.const 4)) (i32.sub (local.get $read) (i32.const 4)) (local.get $offset) (local.get $scratch)))
    (local.set $status (i32.wrap_i64 (local.get $result)))
    (if (local.get $status) (then (return (i32.sub (i32.const 0) (local.get $status)))))
    (local.set $next_off (i32.wrap_i64 (i64.shr_u (local.get $result) (i64.const 32))))
    (i32.store (i32.add (local.get $scratch) (i32.const 20)) (local.get $next_off))
    (drop (call $pipe_write (local.get $output) (local.get $scratch) (i32.const 24)))
    i32.const 24)
