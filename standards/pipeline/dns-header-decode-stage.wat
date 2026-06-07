
;; Process dns_header_decode Stage — slot 59
  (func $process_dns_header_decode (export "process_dns_header_decode")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $status i32)
    (local $read i32)
    (local.set $read (call $stage_read_input (local.get $input) (local.get $scratch) (local.get $scap)))
    (if (i32.lt_u (local.get $read) (i32.const 12)) (then (return (i32.const 0))))
    (local.set $status (call $dns_header_decode (global.get $SCRATCH_BUF) (local.get $read) (local.get $scratch)))
    (if (local.get $status) (then (return (i32.sub (i32.const 0) (local.get $status)))))
    (drop (call $pipe_write (local.get $output) (local.get $scratch) (i32.const 52)))
    i32.const 52)
