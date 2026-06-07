;; HTTP Date Parse Stage — slot 57
  ;; Stage type: batch (state=0)
  ;; Input:  HTTP date string bytes via input pipe
  ;; Output: 40-byte parse record {unix_secs_lo, unix_secs_hi, year, month, day, hour, min, sec, weekday, format_kind}
  ;; Calls $http_date_parse from protocol/http.wat

  (func $process_http_date_parse (export "process_http_date_parse")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $status i32)
    (local $read i32)
    (local.set $read (call $stage_read_input (local.get $input) (local.get $scratch) (local.get $scap)))
    (if (i32.eqz (local.get $read)) (then (return (i32.const 0))))
    (local.set $status (call $http_date_parse (global.get $SCRATCH_BUF) (local.get $read) (local.get $scratch)))
    (if (local.get $status) (then (return (i32.sub (i32.const 0) (local.get $status)))))
    (drop (call $pipe_write (local.get $output) (local.get $scratch) (i32.const 40)))
    i32.const 40)
