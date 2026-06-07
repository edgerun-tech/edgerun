;; HTTP/2 Frame Header Decode Stage — slot 81
  ;; Input:  HTTP/2 frame bytes via input pipe
  ;; Config: max_frame_size as cfg:clen pair
  ;; Output: 16-byte record {payload_len, type, flags, stream_id, header_len}

  (func $process_http2_frame_header_decode (export "process_http2_frame_header_decode")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $len_slot i32) (local $read i32) (local $status i32) (local $max_fs i32)
    (local.set $max_fs (i32.load (local.get $cfg)))
    (local.set $len_slot (i32.sub (i32.add (local.get $scratch) (local.get $scap)) (i32.const 4)))
    (drop (call $pipe_read_ptr (local.get $input) (local.get $len_slot)))
    (local.set $read (i32.load (local.get $len_slot)))
    (if (i32.eqz (local.get $read)) (then (return (i32.const 0))))
    (drop (call $pipe_read (local.get $input) (i32.const 0x3000) (local.get $read)))
    (local.set $status (call $http2_frame_header_decode
      (i32.const 0x3000) (local.get $read) (local.get $max_fs) (local.get $scratch)))
    (if (local.get $status) (then (return (i32.sub (i32.const 0) (local.get $status)))))
    (drop (call $pipe_write (local.get $output) (local.get $scratch) (i32.const 16)))
    i32.const 16)
