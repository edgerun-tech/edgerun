;; AES-128 Decrypt Pipeline Stage — slot 43
  ;; Stage type: batch (state=0)
  ;; Input:  ciphertext bytes via input pipe
  ;; Output: plaintext bytes via output pipe
  ;; Config: 16-byte AES-128 key
  ;; Calls $aes128_decrypt from crypto/crypto-aes-block.wat

  (func $process_aes128_decrypt (export "process_aes128_decrypt")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $read i32) (local $result i64)
    (local $status i32)
    (local $out_len i32)
    (local.set $read (call $stage_read_input (local.get $input) (local.get $scratch) (local.get $scap)))
    (if (i32.eqz (local.get $read)) (then (return (i32.const 0))))
    (local.set $result (call $aes128_decrypt (local.get $cfg) (local.get $clen) (i32.const 0x3000) (local.get $read) (local.get $scratch)))
    (local.set $status (i32.wrap_i64 (i64.shr_u (local.get $result) (i64.const 32))))
    (if (local.get $status) (then (return (i32.sub (i32.const 0) (local.get $status)))))
    (local.set $out_len (i32.wrap_i64 (local.get $result)))
    (drop (call $pipe_write (local.get $output) (local.get $scratch) (local.get $out_len)))
    local.get $out_len)
