;; Endian Write Stage — slot 74
  ;; Input:  16 bytes {value_lo, value_hi, width, endian} as i32le via input pipe
  ;; Output: binary encoded value (2/3/4 bytes)
  ;; Calls $endian_write from codec/binary.wat

  (func $process_endian_write (export "process_endian_write")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $val_lo i32) (local $val_hi i32) (local $width i32) (local $endian i32)
    (local $read i32) (local $result i64)
    (local $status i32)
    (local $out_len i32)
    (local.set $read (call $stage_read_input (local.get $input) (local.get $scratch) (local.get $scap)))
        (if (i32.lt_u (local.get $read) (i32.const 16)) (then (return (i32.const 0))))
    (local.set $val_lo (i32.load (i32.const 0x3000)))
    (local.set $val_hi (i32.load (i32.const 0x3004)))
    (local.set $width (i32.load (i32.const 0x3008)))
    (local.set $endian (i32.load (i32.const 0x300c)))
    (local.set $result (call $endian_write (local.get $val_lo) (local.get $val_hi) (local.get $width) (local.get $endian) (local.get $scratch) (local.get $scap)))
    (local.set $status (i32.wrap_i64 (local.get $result)))
    (if (local.get $status) (then (return (i32.sub (i32.const 0) (local.get $status)))))
    (local.set $out_len (i32.wrap_i64 (i64.shr_u (local.get $result) (i64.const 32))))
    (drop (call $pipe_write (local.get $output) (local.get $scratch) (local.get $out_len)))
    local.get $out_len)
