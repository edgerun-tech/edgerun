;; WebSocket Parse Header Stage — slot 100
  ;; Config: max_len as first 4 bytes of cfg
  (func $process_ws_parse_header (export "process_ws_parse_header")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $max_len i32) (local $status i32)
    (local $read i32)
    (local.set $read (call $stage_read_input (local.get $input) (local.get $scratch) (local.get $scap)))
    (if (i32.eqz (local.get $read)) (then (return (i32.const 0))))
    (local.set $max_len (i32.load (local.get $cfg)))
    (local.set $status (call $ws_parse_header
      (global.get $SCRATCH_BUF) (local.get $read) (local.get $max_len) (local.get $scratch)))
    (if (local.get $status) (then (return (i32.sub (i32.const 0) (local.get $status)))))
    (drop (call $pipe_write (local.get $output) (local.get $scratch) (i32.const 20)))
    i32.const 20)
