;; Unquote Span Stage — slot 136
  (func $process_unquote_span (export "process_unquote_span")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $read i32)

    (local.set $read (call $stage_read_input (local.get $input) (local.get $scratch) (local.get $scap)))
    (if (i32.lt_u (local.get $read) (i32.const 1)) (then (return (i32.const 0))))
    (drop (call $unquote_span (global.get $SCRATCH_BUF) (local.get $read) (local.get $scratch)))
    (drop (call $pipe_write (local.get $output) (local.get $scratch) (i32.const 8)))
    i32.const 8)
