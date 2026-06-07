
;; Process inet_checksum Stage — slot 25
  (func $process_inet_checksum (export "process_inet_checksum")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $val i32)
    (local $read i32)
    (local.set $read (call $stage_read_input (local.get $input) (local.get $scratch) (local.get $scap)))
    (if (i32.eqz (local.get $read)) (then (return (i32.const 0))))
    (local.set $val (call $inet_checksum (global.get $SCRATCH_BUF) (local.get $read)))
    (i32.store16 (global.get $SHA256_OUT_BUF) (local.get $val))
    (drop (call $pipe_write (local.get $output) (global.get $SHA256_OUT_BUF) (i32.const 2)))
    i32.const 2)
