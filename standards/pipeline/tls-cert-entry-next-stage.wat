;; TLS Certificate Entry Next Stage — slot 94
  ;; Input:  certificate list bytes prefixed with start offset (4 bytes) via input pipe
  ;; Output: 20-byte record {cert_offset, cert_len, ext_offset, ext_len, next_offset}

  (func $process_tls_cert_entry_next (export "process_tls_cert_entry_next")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $start i32) (local $status i32)
    (local $read i32)
    (local.set $read (call $stage_read_input (local.get $input) (local.get $scratch) (local.get $scap)))
    (if (i32.lt_u (local.get $read) (i32.const 5)) (then (return (i32.const 0))))
    (local.set $start (i32.load (i32.const 0x3000)))
    (local.set $status (call $tls_certificate_entry_next
      (i32.add (i32.const 0x3000) (i32.const 4))
      (i32.sub (local.get $read) (i32.const 4))
      (local.get $start) (local.get $scratch)))
    (if (i32.lt_s (local.get $status) (i32.const 0))
      (then (return (i32.sub (i32.const 0) (local.get $status)))))
    (drop (call $pipe_write (local.get $output) (local.get $scratch) (i32.const 20)))
    i32.const 20)
