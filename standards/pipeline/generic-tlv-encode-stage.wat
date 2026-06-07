;; Generic TLV Encode Stage — slot 68
  ;; Input:  8 bytes {tag, value_len} as i32le via input pipe
  ;; Output: TLV header bytes (1-4 bytes depending on value length)
  ;; Calls $generic_tlv_encode_header from codec/binary.wat

  (func $process_generic_tlv_encode (export "process_generic_tlv_encode")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $tag i32) (local $value_len i32)
    (local $read i32) (local $result i64)
    (local $status i32)
    (local $out_len i32)
    (local.set $read (call $stage_read_input (local.get $input) (local.get $scratch) (local.get $scap)))
        (if (i32.lt_u (local.get $read) (i32.const 8)) (then (return (i32.const 0))))
    (local.set $tag (i32.load (i32.const 0x3000)))
    (local.set $value_len (i32.load (i32.const 0x3004)))
    (local.set $result (call $generic_tlv_encode_header (local.get $tag) (local.get $value_len) (local.get $scratch) (local.get $scap)))
    (local.set $status (i32.wrap_i64 (local.get $result)))
    (if (local.get $status) (then (return (i32.sub (i32.const 0) (local.get $status)))))
    (local.set $out_len (i32.wrap_i64 (i64.shr_u (local.get $result) (i64.const 32))))
    (drop (call $pipe_write (local.get $output) (local.get $scratch) (local.get $out_len)))
    local.get $out_len)
