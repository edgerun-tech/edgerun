
;; Process to_lower Stage — slot 37
  (func $process_to_lower (export "process_to_lower")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    
    (local $read i32)
    (local.set $read (call $stage_read_input (local.get $input) (local.get $scratch) (local.get $scap)))
    (if (i32.eqz (local.get $read)) (then (return (i32.const 0))))
    
    
    (return (call $stage_write_output (local.get $output) (local.get $scratch)
      (call $to_lower_ascii (global.get $SCRATCH_BUF) (local.get $read) (local.get $scratch)))))
