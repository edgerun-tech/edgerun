;; HTTP Chunked Body Scan Stage — slot 64
  ;; Input:  raw chunked body bytes via input pipe
  ;; Output: 28-byte summary record {decoded_lo, decoded_hi, chunk_count, first_off, trailer_len, next_off, flags}
  ;; Calls $http1_chunk_scan_body from protocol/http.wat

  (func $process_http_chunk_scan (export "process_http_chunk_scan")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $status i32)
    (local $read i32)
    (local.set $read (call $stage_read_input (local.get $input) (local.get $scratch) (local.get $scap)))
    (if (i32.eqz (local.get $read)) (then (return (i32.const 0))))
    (local.set $status (call $http1_chunk_scan_body (i32.const 0x3000) (local.get $read) (local.get $scratch)))
    (if (local.get $status) (then (return (i32.sub (i32.const 0) (local.get $status)))))
    (drop (call $pipe_write (local.get $output) (local.get $scratch) (i32.const 28)))
    i32.const 28)
