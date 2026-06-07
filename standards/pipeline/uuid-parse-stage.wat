;; UUID Parse Pipeline Stage — slot 52
  ;; Stage type: batch (state=0)
  ;; Input:  36-byte formatted UUID string via input pipe
  ;; Output: 16-byte binary UUID via output pipe
  ;; Calls $uuid_parse from data/uuid-util.wat

  (func $process_uuid_parse (export "process_uuid_parse")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $len_slot i32) (local $read i32) (local $status i32)
    (local.set $len_slot (i32.sub (i32.add (local.get $scratch) (local.get $scap)) (i32.const 4)))
    (drop (call $pipe_read_ptr (local.get $input) (local.get $len_slot)))
    (local.set $read (i32.load (local.get $len_slot)))
    (if (i32.eqz (local.get $read)) (then (return (i32.const 0))))
    (drop (call $pipe_read (local.get $input) (i32.const 0x3000) (local.get $read)))
    (local.set $status (call $uuid_parse (i32.const 0x3000) (local.get $scratch)))
    (if (local.get $status) (then (return (local.get $status))))
    (drop (call $pipe_write (local.get $output) (local.get $scratch) (i32.const 16)))
    i32.const 16)
