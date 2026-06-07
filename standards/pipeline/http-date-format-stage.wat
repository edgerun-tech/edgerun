;; HTTP Date Format Stage — slot 58
  ;; Stage type: batch (state=0)
  ;; Input:  8 bytes {unix_secs_lo, unix_secs_hi} as i32le via input pipe
  ;; Output: 29-byte IMF-fixdate string
  ;; Calls $http_date_format from protocol/http.wat

  (func $process_http_date_format (export "process_http_date_format")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $lo i32) (local $hi i32)
    (local $read i32) (local $result i64)
    (local $status i32)
    (local $out_len i32)
    (local.set $read (call $stage_read_input (local.get $input) (local.get $scratch) (local.get $scap)))
        (if (i32.lt_u (local.get $read) (i32.const 8)) (then (return (i32.const 0))))
    (local.set $lo (i32.load (global.get $SCRATCH_BUF)))
    (local.set $hi (i32.load (i32.add (global.get $SCRATCH_BUF) (i32.const 4))))
    (local.set $result (call $http_date_format (local.get $lo) (local.get $hi) (local.get $scratch) (local.get $scap)))
    (local.set $status (i32.wrap_i64 (local.get $result)))
    (if (local.get $status) (then (return (i32.sub (i32.const 0) (local.get $status)))))
    (local.set $out_len (i32.wrap_i64 (i64.shr_u (local.get $result) (i64.const 32))))
    (drop (call $pipe_write (local.get $output) (local.get $scratch) (local.get $out_len)))
    local.get $out_len)
