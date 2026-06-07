;; RSA PKCS1 v1.5 Verify Stage — slot 75
  ;; Config: 8 bytes {hash_alg, em_len_or_key_size} as i32le
  ;; Input:  concatenated {digest_bytes|em_bytes} via input pipe
  ;; Output: status only (0=valid via pipe_write of 4 zero bytes, error=return negative)
  ;; Calls $rsa_pkcs1_v15_verify from crypto/crypto-rsa-pkcs1.wat

  (func $process_rsa_pkcs1_verify (export "process_rsa_pkcs1_verify")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $len_slot i32) (local $status i32)
    (local $hash_alg i32) (local $em_len i32) (local $digest_len i32) (local $read i32)
    (local.set $len_slot (i32.sub (i32.add (local.get $scratch) (local.get $scap)) (i32.const 4)))
    (drop (call $pipe_read_ptr (local.get $input) (local.get $len_slot)))
    (local.set $read (i32.load (local.get $len_slot)))
    (if (i32.lt_u (local.get $read) (i32.const 4)) (then (return (i32.const 0))))
    (if (i32.lt_u (local.get $clen) (i32.const 8)) (then (return (i32.const -2))))
    (drop (call $pipe_read (local.get $input) (i32.const 0x3000) (local.get $read)))
    (local.set $hash_alg (i32.load (local.get $cfg)))
    (local.set $em_len (i32.load (i32.add (local.get $cfg) (i32.const 4))))
    (local.set $digest_len (i32.load (i32.const 0x3000)))
    (local.set $status (call $rsa_pkcs1_v15_verify (local.get $hash_alg) (i32.add (i32.const 0x3000) (i32.const 4)) (local.get $digest_len) (i32.add (i32.add (i32.const 0x3000) (i32.const 4)) (local.get $digest_len)) (local.get $em_len)))
    (if (local.get $status) (then (return (i32.sub (i32.const 0) (local.get $status)))))
    (i32.store (local.get $scratch) (i32.const 0))
    (drop (call $pipe_write (local.get $output) (local.get $scratch) (i32.const 4)))
    i32.const 4)
