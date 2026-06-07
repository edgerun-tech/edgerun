;; X.509 Certificate Scan Stage — slot 80
  ;; Input:  DER-encoded X.509v3 certificate via input pipe
  ;; Output: 40 field offset/length records to output pipe

  (func $process_x509_cert_scan (export "process_x509_cert_scan")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $status i32)
    (local $read i32)
    (local.set $read (call $stage_read_input (local.get $input) (local.get $scratch) (local.get $scap)))
    (if (i32.eqz (local.get $read)) (then (return (i32.const 0))))
    (local.set $status (call $x509_certificate_scan (i32.const 0x3000) (local.get $read) (local.get $scratch)))
    (if (local.get $status) (then (return (i32.sub (i32.const 0) (local.get $status)))))
    (drop (call $pipe_write (local.get $output) (local.get $scratch) (i32.const 40)))
    i32.const 40)
