;; HTTP/1 Validate Header Block Stage — slot 89
  ;; Input:  raw HTTP/1 header block via input pipe
  ;; Config: max_headers (4 bytes) + max_bytes (4 bytes) at cfg
  ;; Output: 12-byte record {header_count, total_bytes, status_flags}

  (func $process_http_validate_headers (export "process_http_validate_headers")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $max_h i32) (local $max_b i32) (local $status i32)
    (local $read i32)
    (local.set $read (call $stage_read_input (local.get $input) (local.get $scratch) (local.get $scap)))
    (if (i32.eqz (local.get $read)) (then (return (i32.const 0))))
    (local.set $max_h (i32.load (local.get $cfg)))
    (local.set $max_b (i32.load (i32.add (local.get $cfg) (i32.const 4))))
    (local.set $status (call $http1_validate_header_block
      (i32.const 0x3000) (local.get $read) (local.get $max_h) (local.get $max_b) (local.get $scratch)))
    (if (local.get $status) (then (return (i32.sub (i32.const 0) (local.get $status)))))
    (drop (call $pipe_write (local.get $output) (local.get $scratch) (i32.const 12)))
    i32.const 12)
