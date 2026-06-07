;; Unescape Text Pipeline Stage — slot 56
  ;; Stage type: batch (state=0)
  ;; Input:  Jagex-escaped text bytes via input pipe
  ;; Output: raw decoded text bytes via output pipe
  ;; Calls $unescape_text from codec/string-url.wat

  (func $process_unescape_text (export "process_unescape_text")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $len_slot i32) (local $read i32) (local $out_len i32)
    (local.set $len_slot (i32.sub (i32.add (local.get $scratch) (local.get $scap)) (i32.const 4)))
    (drop (call $pipe_read_ptr (local.get $input) (local.get $len_slot)))
    (local.set $read (i32.load (local.get $len_slot)))
    (if (i32.eqz (local.get $read)) (then (return (i32.const 0))))
    (drop (call $pipe_read (local.get $input) (i32.const 0x3000) (local.get $read)))
    (local.set $out_len (call $unescape_text (i32.const 0x3000) (local.get $read) (local.get $scratch)))
    (if (i32.le_s (local.get $out_len) (i32.const 0)) (then (return (i32.const 0))))
    (drop (call $pipe_write (local.get $output) (local.get $scratch) (local.get $out_len)))
    local.get $out_len)
