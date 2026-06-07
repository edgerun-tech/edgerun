;; CP1252 Decode Pipeline Stage — slot 40
  ;; Stage type: batch (state=0)
  ;; Input:  CP1252-encoded bytes via input pipe
  ;; Output: UTF-8 decoded bytes via output pipe
  ;; Calls $cp1252_decode_string from codec/text.wat

  (func $process_cp1252_decode (export "process_cp1252_decode")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $out_len i32)
    (local $read i32)
    (local.set $read (call $stage_read_input (local.get $input) (local.get $scratch) (local.get $scap)))
    (if (i32.eqz (local.get $read)) (then (return (i32.const 0))))
    (local.set $out_len (call $cp1252_decode_string (i32.const 0x3000) (local.get $read) (local.get $scratch)))
    (if (i32.le_s (local.get $out_len) (i32.const 0)) (then (return (i32.const 0))))
    (drop (call $pipe_write (local.get $output) (local.get $scratch) (local.get $out_len)))
    local.get $out_len)
