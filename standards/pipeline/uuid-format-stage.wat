;; UUID Format Pipeline Stage — slot 51
  ;; Stage type: batch (state=0)
  ;; Input:  16-byte binary UUID via input pipe
  ;; Output: 36-byte formatted UUID string (xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx) via output pipe
  ;; Calls $uuid_format from data/uuid-util.wat

  (func $process_uuid_format (export "process_uuid_format")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $out_len i32)
    (local $read i32)
    (local.set $read (call $stage_read_input (local.get $input) (local.get $scratch) (local.get $scap)))
    (if (i32.eqz (local.get $read)) (then (return (i32.const 0))))
    (local.set $out_len (call $uuid_format (i32.const 0x3000) (local.get $scratch)))
    (if (i32.le_s (local.get $out_len) (i32.const 0)) (then (return (i32.const 0))))
    (drop (call $pipe_write (local.get $output) (local.get $scratch) (local.get $out_len)))
    local.get $out_len)
