;; Endian Read Stage — slot 73
  ;; Input:  20 bytes {offset, width, endian, signed, buf_bytes...} via input pipe
  ;; Output: 8 bytes {value_lo, value_hi} as i32le
  ;; Calls $endian_read from codec/binary.wat

  (func $process_endian_read (export "process_endian_read")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $len_slot i32) (local $read i32) (local $status i32)
    (local $offset i32) (local $width i32) (local $endian i32) (local $signed i32)
    (local.set $len_slot (i32.sub (i32.add (local.get $scratch) (local.get $scap)) (i32.const 4)))
    (drop (call $pipe_read_ptr (local.get $input) (local.get $len_slot)))
    (local.set $read (i32.load (local.get $len_slot)))
    (if (i32.lt_u (local.get $read) (i32.const 20)) (then (return (i32.const 0))))
    (drop (call $pipe_read (local.get $input) (i32.const 0x3000) (local.get $read)))
    (local.set $offset (i32.load (i32.const 0x3000)))
    (local.set $width (i32.load (i32.const 0x3004)))
    (local.set $endian (i32.load (i32.const 0x3008)))
    (local.set $signed (i32.load (i32.const 0x300c)))
    (local.set $status (call $endian_read (i32.add (i32.const 0x3000) (i32.const 16)) (i32.sub (local.get $read) (i32.const 16)) (local.get $offset) (local.get $width) (local.get $endian) (local.get $signed) (local.get $scratch)))
    (if (local.get $status) (then (return (i32.sub (i32.const 0) (local.get $status)))))
    (drop (call $pipe_write (local.get $output) (local.get $scratch) (i32.const 8)))
    i32.const 8)
