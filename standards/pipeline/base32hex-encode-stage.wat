
;; Process base32hex_encode Stage — slot 121
  (func $process_base32hex_encode (export "process_base32hex_encode")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $read i32) (local $result i64) (local $status i32) (local $written i32)
    (local.set $read (call $stage_read_input (local.get $input) (local.get $scratch) (local.get $scap)))
    (if (i32.eqz (local.get $read)) (then (return (i32.const 0))))
        (local.set $result (call $base32hex_encode (global.get $SCRATCH_BUF) (local.get $read) (global.get $WORK_BUF) (global.get $BUF_SIZE_8K)))
    (local.set $status (i32.wrap_i64 (i64.shr_u (local.get $result) (i64.const 32))))
    (local.set $written (i32.wrap_i64 (local.get $result)))
    (if (local.get $status) (then (return (i32.sub (i32.const 0) (local.get $status)))))
    (drop (call $pipe_write (local.get $output) (global.get $WORK_BUF) (local.get $written)))
    local.get $written)
