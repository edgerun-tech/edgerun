  ;; Internet Checksum Pipeline Stage — slot 25
  ;; Stage type: batch (state=0)
  ;; Input:  raw bytes via input pipe
  ;; Output: 2-byte internet checksum (RFC 1071, big-endian i16) via output pipe
  ;; Calls $inet_checksum from crypto/checksum-inet.wat
  (func $process_inet_checksum (export "process_inet_checksum")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $len_slot i32) (local $read i32) (local $csum i32)
    (local.set $len_slot (i32.sub (i32.add (local.get $scratch) (local.get $scap)) (i32.const 4)))
    (drop (call $pipe_read_ptr (local.get $input) (local.get $len_slot)))
    (local.set $read (i32.load (local.get $len_slot)))
    (if (i32.eqz (local.get $read)) (then (return (i32.const 0))))
    (drop (call $pipe_read (local.get $input) (i32.const 0x3000) (local.get $read)))
    (local.set $csum (call $inet_checksum (i32.const 0x3000) (local.get $read)))
    (i32.store16 (i32.const 0x5000) (local.get $csum))
    (drop (call $pipe_write (local.get $output) (i32.const 0x5000) (i32.const 2)))
    i32.const 2)
