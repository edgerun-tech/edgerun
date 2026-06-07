;; HTTP Request Line Parse Stage — slot 44
  ;; Stage type: batch (state=0)
  ;; Input:  raw HTTP request line bytes via input pipe
  ;; Output: 28-byte parse record {method_off, method_len, target_off, target_len, major, minor, next_off}
  ;; Calls $http_parse_request_line from protocol/http.wat

  (func $process_http_request_line_parse (export "process_http_request_line_parse")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $len_slot i32) (local $read i32) (local $status i32)
    (local.set $len_slot (i32.sub (i32.add (local.get $scratch) (local.get $scap)) (i32.const 4)))
    (drop (call $pipe_read_ptr (local.get $input) (local.get $len_slot)))
    (local.set $read (i32.load (local.get $len_slot)))
    (if (i32.eqz (local.get $read)) (then (return (i32.const 0))))
    (drop (call $pipe_read (local.get $input) (i32.const 0x3000) (local.get $read)))
    (local.set $status (call $http_parse_request_line (i32.const 0x3000) (local.get $read) (local.get $scratch)))
    (if (local.get $status) (then (return (i32.sub (i32.const 0) (local.get $status)))))
    (drop (call $pipe_write (local.get $output) (local.get $scratch) (i32.const 28)))
    i32.const 28)
