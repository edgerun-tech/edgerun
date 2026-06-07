;; AES-128-GCM Encrypt Pipeline Stage — slot 21
  ;; Stage type: batch (state=0)
  ;; Config layout:
  ;;   +0: key_len (i32) — expected 16
  ;;   +4: key (key_len bytes)
  ;;   +4+key_len: iv_len (i32) — expected 12
  ;;   +4+key_len+4: iv (iv_len bytes)
  ;;   +4+key_len+4+iv_len: aad_len (i32)
  ;;   +4+key_len+4+iv_len+4: aad (aad_len bytes)
  ;; Input:  plaintext via input pipe
  ;; Output: ciphertext + 16-byte auth tag via output pipe
  ;; Calls $aes128_gcm_encrypt from crypto/crypto-aes128-gcm.wat

  ;; AES-GCM internal scratch: 16384–16767 (no overlap with 0x6000 input buf).
  ;; We copy input to 0x6000, write output to 0x5000, tag follows output.
  (func $process_aes128_gcm_encrypt (export "process_aes128_gcm_encrypt")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $len_slot i32)
    (local $key_len i32) (local $iv_len i32) (local $aad_len i32) (local $read i32)
    (local $key_ptr i32) (local $iv_ptr i32) (local $aad_ptr i32)
    (local $result i32) (local $tag_ptr i32)

    (local.set $len_slot (i32.sub (i32.add (local.get $scratch) (local.get $scap)) (i32.const 4)))
    (drop (call $pipe_read_ptr (local.get $input) (local.get $len_slot)))
    (local.set $read (i32.load (local.get $len_slot)))
    (if (i32.eqz (local.get $read)) (then (return (i32.const 0))))

    ;; Parse config
    (local.set $key_len (i32.load (local.get $cfg)))
    (local.set $key_ptr (i32.add (local.get $cfg) (i32.const 4)))
    (local.set $iv_len (i32.load (i32.add (local.get $key_ptr) (local.get $key_len))))
    (local.set $iv_ptr (i32.add (i32.add (local.get $key_ptr) (local.get $key_len)) (i32.const 4)))
    (local.set $aad_len (i32.load (i32.add (local.get $iv_ptr) (local.get $iv_len))))
    (local.set $aad_ptr (i32.add (i32.add (local.get $iv_ptr) (local.get $iv_len)) (i32.const 4)))
    (local.set $tag_ptr (i32.add (i32.const 0x5000) (local.get $read)))

    ;; Copy input to safe buffer
    (drop (call $pipe_read (local.get $input) (i32.const 0x6000) (local.get $read)))

    ;; Encrypt
    (local.set $result
      (call $aes128_gcm_encrypt
        (i32.const 0x5000)       ;; out
        (i32.const 0x6000)       ;; in
        (local.get $read)        ;; len
        (local.get $aad_ptr)     ;; aad
        (local.get $aad_len)     ;; aad_len
        (local.get $key_ptr)     ;; key
        (local.get $iv_ptr)      ;; iv
        (local.get $tag_ptr)))   ;; tag

    (if (local.get $result) (then (return (i32.sub (i32.const 0) (local.get $result)))))
    (drop (call $pipe_write (local.get $output) (i32.const 0x5000) (local.get $read)))
    (drop (call $pipe_write (local.get $output) (local.get $tag_ptr) (i32.const 16)))
    (i32.add (local.get $read) (i32.const 16)))
