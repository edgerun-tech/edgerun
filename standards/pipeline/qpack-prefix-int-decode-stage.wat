
;; Process qpack_prefix_int_decode Stage — slot 131
  (func $process_qpack_prefix_int_decode (export "process_qpack_prefix_int_decode")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $status i32)    (local $prefix_bits i32)
    (local $read i32)
    (local.set $read (call $stage_read_input (local.get $input) (local.get $scratch) (local.get $scap)))
    (if (i32.eqz (local.get $read)) (then (return (i32.const 0))))
        (local.set $prefix_bits (i32.const 6))
    (if (i32.ge_u (local.get $clen) (i32.const 4)) (then (local.set $prefix_bits (i32.load (local.get $cfg)))))    (local.set $status (call $qpack_prefix_int_decode (global.get $SCRATCH_BUF) (local.get $read) (local.get $prefix_bits) (global.get $WORK_BUF)))
    (if (local.get $status) (then (return (i32.sub (i32.const 0) (local.get $status)))))
    (drop (call $pipe_write (local.get $output) (local.get $scratch) (i32.const 16)))
    i32.const 16)
