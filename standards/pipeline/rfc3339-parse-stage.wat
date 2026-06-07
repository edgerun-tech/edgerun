;; RFC 3339 Timestamp Parse Stage — slot 85
  ;; Input:  RFC 3339/ISO 8601 timestamp string via input pipe
  ;; Output: 8-byte i64 Unix timestamp

  (func $process_rfc3339_parse (export "process_rfc3339_parse")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $status i32)
    (local $read i32)
    (local.set $read (call $stage_read_input (local.get $input) (local.get $scratch) (local.get $scap)))
    (if (i32.eqz (local.get $read)) (then (return (i32.const 0))))
    (local.set $status (call $rfc3339_parse (global.get $SCRATCH_BUF) (local.get $read) (local.get $scratch)))
    (if (local.get $status) (then (return (i32.sub (i32.const 0) (local.get $status)))))
    (drop (call $pipe_write (local.get $output) (local.get $scratch) (i32.const 8)))
    i32.const 8)
