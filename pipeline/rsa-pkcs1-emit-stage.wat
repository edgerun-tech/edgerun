;; RSA PKCS1 v1.5 Emit Stage — slot 76
  ;; Config: 8 bytes {hash_alg, out_len_or_key_size} as i32le
  ;; Input:  digest bytes via input pipe
  ;; Output: EMSA-PKCS1-v1_5 encoded message bytes
  ;; Calls $rsa_pkcs1_v15_emit from crypto/crypto-rsa-pkcs1.wat

  (func $process_rsa_pkcs1_emit (export "process_rsa_pkcs1_emit")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $len_slot i32)
    (local $read i32)
    (local $result i64) (local $status i32) (local $out_len i32)
    (local $hash_alg i32) (local $em_len i32)
    (local.set $len_slot (i32.sub (i32.add (local.get $scratch) (local.get $scap)) (i32.const 4)))
    (drop (call $pipe_read_ptr (local.get $input) (local.get $len_slot)))
    (local.set $read (i32.load (local.get $len_slot)))
    (if (i32.eqz (local.get $read)) (then (return (i32.const 0))))
    (if (i32.lt_u (local.get $clen) (i32.const 8)) (then (return (i32.const -2))))
    (drop (call $pipe_read (local.get $input) (global.get $SCRATCH_BUF) (local.get $read)))
    (local.set $hash_alg (i32.load (local.get $cfg)))
    (local.set $em_len (i32.load (i32.add (local.get $cfg) (i32.const 4))))
    (local.set $result (call $rsa_pkcs1_v15_emit (local.get $hash_alg) (global.get $SCRATCH_BUF) (local.get $read) (local.get $scratch) (local.get $em_len)))
    (local.set $status (i32.wrap_i64 (local.get $result)))
    (if (local.get $status) (then (return (i32.sub (i32.const 0) (local.get $status)))))
    (local.set $out_len (i32.wrap_i64 (i64.shr_u (local.get $result) (i64.const 32))))
    (drop (call $pipe_write (local.get $output) (local.get $scratch) (local.get $out_len)))
    local.get $out_len)
