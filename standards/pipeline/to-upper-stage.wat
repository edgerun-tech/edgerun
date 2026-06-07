;; ASCII Uppercase Pipeline Stage — slot 38
  ;; Stage type: batch (state=0)
  ;; Input:  ASCII bytes via input pipe
  ;; Output: uppercased ASCII bytes via output pipe
  ;; Calls $to_upper_ascii from codec/text.wat

  (func $process_to_upper (export "process_to_upper")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $len_slot i32) (local $read i32) (local $written i32)
    (local.set $len_slot (i32.sub (i32.add (local.get $scratch) (local.get $scap)) (i32.const 4)))
    (drop (call $pipe_read_ptr (local.get $input) (local.get $len_slot)))
    (local.set $read (i32.load (local.get $len_slot)))
    (if (i32.eqz (local.get $read)) (then (return (i32.const 0))))
    (drop (call $pipe_read (local.get $input) (i32.const 0x3000) (local.get $read)))
    (local.set $written (call $to_upper_ascii (i32.const 0x3000) (local.get $read) (local.get $scratch)))
    (drop (call $pipe_write (local.get $output) (local.get $scratch) (local.get $written)))
    local.get $written)
