;; Frame Header Encode (u16 BE) Stage — slot 70
  ;; Input:  4 bytes {payload_len} as i32le via input pipe
  ;; Output: 2-byte big-endian frame header
  ;; Calls $frame_header_encode_u16_be from codec/binary.wat

  (func $process_frame_header_encode (export "process_frame_header_encode")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $len_slot i32) (local $read i32)
    (local $result i64) (local $status i32) (local $out_len i32)
    (local $payload_len i32)
    (local.set $len_slot (i32.sub (i32.add (local.get $scratch) (local.get $scap)) (i32.const 4)))
    (drop (call $pipe_read_ptr (local.get $input) (local.get $len_slot)))
    (local.set $read (i32.load (local.get $len_slot)))
    (if (i32.lt_u (local.get $read) (i32.const 4)) (then (return (i32.const 0))))
    (drop (call $pipe_read (local.get $input) (i32.const 0x3000) (local.get $read)))
    (local.set $payload_len (i32.load (i32.const 0x3000)))
    (local.set $result (call $frame_header_encode_u16_be (local.get $payload_len) (local.get $scratch) (local.get $scap)))
    (local.set $status (i32.wrap_i64 (local.get $result)))
    (if (local.get $status) (then (return (i32.sub (i32.const 0) (local.get $status)))))
    (local.set $out_len (i32.wrap_i64 (i64.shr_u (local.get $result) (i64.const 32))))
    (drop (call $pipe_write (local.get $output) (local.get $scratch) (local.get $out_len)))
    local.get $out_len)
