;; TLS ClientHello SNI Host Stage — slot 84
  ;; Input:  TLS SNI extension payload via input pipe
  ;; Output: 8-byte record {host_offset, host_len}

  (func $process_tls_sni_host (export "process_tls_sni_host")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $status i32)
    (local $read i32)
    (local.set $read (call $stage_read_input (local.get $input) (local.get $scratch) (local.get $scap)))
    (if (i32.eqz (local.get $read)) (then (return (i32.const 0))))
    (local.set $status (call $tls_clienthello_sni_host (global.get $SCRATCH_BUF) (local.get $read) (local.get $scratch)))
    (if (local.get $status) (then (return (i32.sub (i32.const 0) (local.get $status)))))
    (drop (call $pipe_write (local.get $output) (local.get $scratch) (i32.const 8)))
    i32.const 8)
