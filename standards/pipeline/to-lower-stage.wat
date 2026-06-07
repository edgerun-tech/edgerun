;; ASCII Lowercase Pipeline Stage — slot 37
  ;; Stage type: batch (state=0)
  ;; Input:  ASCII bytes via input pipe
  ;; Output: lowercased ASCII bytes via output pipe
  ;; Calls $to_lower_ascii from codec/text.wat

  (func $process_to_lower (export "process_to_lower")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $read i32)
    (local.set $read (call $stage_read_input (local.get $input) (local.get $scratch) (local.get $scap)))
    (if (i32.eqz (local.get $read)) (then (return (i32.const 0))))
    (return (call $stage_write_output (local.get $output) (local.get $scratch)
      (call $to_lower_ascii (global.get $SCRATCH_BUF) (local.get $read) (local.get $scratch)))))
