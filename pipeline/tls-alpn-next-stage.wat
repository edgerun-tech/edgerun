;; TLS ClientHello ALPN Next Stage — slot 87
  ;; Input:  ALPN extension payload prefixed with start offset (4 bytes) via input pipe
  ;; Output: 8-byte record {protocol_offset, protocol_len, next_offset}

  (func $process_tls_alpn_next (export "process_tls_alpn_next")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $start i32) (local $status i32)
    (local $read i32)
    (local.set $read (call $stage_read_input (local.get $input) (local.get $scratch) (local.get $scap)))
    (if (i32.lt_u (local.get $read) (i32.const 5)) (then (return (i32.const 0))))
    (local.set $start (i32.load (global.get $SCRATCH_BUF)))
    (local.set $status (call $tls_clienthello_alpn_next
      (i32.add (global.get $SCRATCH_BUF) (i32.const 4))
      (i32.sub (local.get $read) (i32.const 4))
      (local.get $start) (local.get $scratch)))
    (if (i32.lt_s (local.get $status) (i32.const 0))
      (then (return (i32.sub (i32.const 0) (local.get $status)))))
    (drop (call $pipe_write (local.get $output) (local.get $scratch) (i32.const 12)))
    i32.const 12)
