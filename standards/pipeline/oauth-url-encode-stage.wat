;; OAuth URL Encode Stage — slot 77
  ;; Input:  raw bytes via input pipe
  ;; Output: RFC 3986 percent-encoded bytes via output pipe
  ;; Calls $oauth_url_encode from app/oauth.wat

  (func $process_oauth_url_encode (export "process_oauth_url_encode")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $read i32) (local $result i64)
    (local $status i32)
    (local $out_len i32)
    (local.set $read (call $stage_read_input (local.get $input) (local.get $scratch) (local.get $scap)))
    (if (i32.eqz (local.get $read)) (then (return (i32.const 0))))
    (local.set $result (call $oauth_url_encode (i32.const 0x3000) (local.get $read) (local.get $scratch) (local.get $scap)))
    (local.set $status (i32.wrap_i64 (local.get $result)))
    (if (local.get $status) (then (return (i32.sub (i32.const 0) (local.get $status)))))
    (local.set $out_len (i32.wrap_i64 (i64.shr_u (local.get $result) (i64.const 32))))
    (drop (call $pipe_write (local.get $output) (local.get $scratch) (local.get $out_len)))
    local.get $out_len)
