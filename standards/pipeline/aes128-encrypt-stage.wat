;; AES-128 Encrypt Pipeline Stage — slot 42
  ;; Stage type: batch (state=0)
  ;; Input:  plaintext bytes via input pipe
  ;; Output: ciphertext bytes via output pipe
  ;; Config: 16-byte AES-128 key
  ;; Calls $aes128_encrypt from crypto/crypto-aes-block.wat

  (func $process_aes128_encrypt (export "process_aes128_encrypt")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $len_slot i32) (local $read i32)
    (local $result i64) (local $status i32) (local $out_len i32)
    (local.set $len_slot (i32.sub (i32.add (local.get $scratch) (local.get $scap)) (i32.const 4)))
    (drop (call $pipe_read_ptr (local.get $input) (local.get $len_slot)))
    (local.set $read (i32.load (local.get $len_slot)))
    (if (i32.eqz (local.get $read)) (then (return (i32.const 0))))
    (drop (call $pipe_read (local.get $input) (i32.const 0x3000) (local.get $read)))
    (local.set $result (call $aes128_encrypt (local.get $cfg) (local.get $clen) (i32.const 0x3000) (local.get $read) (local.get $scratch)))
    (local.set $status (i32.wrap_i64 (i64.shr_u (local.get $result) (i64.const 32))))
    (if (local.get $status) (then (return (i32.sub (i32.const 0) (local.get $status)))))
    (local.set $out_len (i32.wrap_i64 (local.get $result)))
    (drop (call $pipe_write (local.get $output) (local.get $scratch) (local.get $out_len)))
    local.get $out_len)
