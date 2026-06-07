;; SSH Authorized Key Scan Stage — slot 96
  ;; Input:  SSH authorized_keys line via input pipe
  ;; Output: 24-byte record {type_offset, type_len, payload_offset, payload_len, comment_offset, comment_len}

  (func $process_ssh_auth_key_scan (export "process_ssh_auth_key_scan")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $status i32)
    (local $read i32)
    (local.set $read (call $stage_read_input (local.get $input) (local.get $scratch) (local.get $scap)))
    (if (i32.eqz (local.get $read)) (then (return (i32.const 0))))
    (local.set $status (call $ssh_authorized_key_scan (i32.const 0x3000) (local.get $read) (local.get $scratch)))
    (if (local.get $status) (then (return (i32.sub (i32.const 0) (local.get $status)))))
    (drop (call $pipe_write (local.get $output) (local.get $scratch) (i32.const 24)))
    i32.const 24)
