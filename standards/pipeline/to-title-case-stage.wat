;; Title Case Pipeline Stage — slot 53
  ;; Stage type: batch (state=0)
  ;; Input:  SNAKE_CASE ASCII bytes via input pipe
  ;; Output: "Title Case" ASCII bytes via output pipe
  ;; Calls $to_title_case from codec/text.wat

  (func $process_to_title_case (export "process_to_title_case")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $out_len i32)
    (local $read i32)
    (local.set $read (call $stage_read_input (local.get $input) (local.get $scratch) (local.get $scap)))
    (if (i32.eqz (local.get $read)) (then (return (i32.const 0))))
    (local.set $out_len (call $to_title_case (global.get $SCRATCH_BUF) (local.get $read) (local.get $scratch)))
    (if (i32.le_s (local.get $out_len) (i32.const 0)) (then (return (i32.const 0))))
    (drop (call $pipe_write (local.get $output) (local.get $scratch) (local.get $out_len)))
    local.get $out_len)
