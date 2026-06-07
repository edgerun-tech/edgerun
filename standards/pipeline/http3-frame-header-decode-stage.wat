;; HTTP/3 Frame Header Decode Stage — slot 82
  ;; Input:  HTTP/3 (QUIC) frame bytes via input pipe
  ;; Output: 16-byte record {frame_type, payload_len, header_len, is_known_type}

  (func $process_http3_frame_header_decode (export "process_http3_frame_header_decode")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $status i32)
    (local $read i32)
    (local.set $read (call $stage_read_input (local.get $input) (local.get $scratch) (local.get $scap)))
    (if (i32.eqz (local.get $read)) (then (return (i32.const 0))))
    (local.set $status (call $http3_frame_header_decode
      (i32.const 0x3000) (local.get $read) (local.get $scratch)))
    (if (local.get $status) (then (return (i32.sub (i32.const 0) (local.get $status)))))
    (drop (call $pipe_write (local.get $output) (local.get $scratch) (i32.const 16)))
    i32.const 16)
