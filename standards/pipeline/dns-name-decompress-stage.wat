
;; Process dns_name_decompress Stage — slot 65
  (func $process_dns_name_decompress (export "process_dns_name_decompress")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $read i32) (local $result i64) (local $status i32) (local $written i32)
    (local.set $read (call $stage_read_input (local.get $input) (local.get $scratch) (local.get $scap)))
    (if (i32.eqz (local.get $read)) (then (return (i32.const 0))))
        (local.set $result (call $dns_name_to_lower_ascii_compressed (global.get $SCRATCH_BUF) (local.get $read) (i32.const 0) (local.get $scratch) (local.get $scap)))
    (local.set $status (i32.wrap_i64 (i64.shr_u (local.get $result) (i64.const 32))))
    (local.set $written (i32.wrap_i64 (local.get $result)))
    (if (local.get $status) (then (return (i32.sub (i32.const 0) (local.get $status)))))
    (drop (call $pipe_write (local.get $output) (local.get $scratch) (local.get $written)))
    local.get $written)
