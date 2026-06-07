;; AES-128-CTR XOR Pipeline Stage — slot 23
  ;; Stage type: batch (state=0)
  ;; Config layout:
  ;;   +0: key_len (i32) — expected 16
  ;;   +4: key (16 bytes)
  ;;   +20: ctr_len (i32) — expected 16
  ;;   +24: ctr (16 bytes)
  ;; Input:  plaintext/ciphertext via input pipe
  ;; Output: ciphertext/plaintext via output pipe (same length as input)
  ;; Calls $aes128_ctr_xor from crypto/crypto-aes-ctr.wat

  ;; AES-CTR internal scratch: 16384–16767 (same zone as AES-GCM, no overlap with 0x6000).
  (func $process_aes128_ctr_xor (export "process_aes128_ctr_xor")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $len_slot i32) (local $read i32)
    (local $key_len i32) (local $ctr_len i32)
    (local $key_ptr i32) (local $ctr_ptr i32)
    (local $result i32)

    (local.set $len_slot (i32.sub (i32.add (local.get $scratch) (local.get $scap)) (i32.const 4)))
    (drop (call $pipe_read_ptr (local.get $input) (local.get $len_slot)))
    (local.set $read (i32.load (local.get $len_slot)))
    (if (i32.eqz (local.get $read)) (then (return (i32.const 0))))

    ;; Parse config
    (local.set $key_len (i32.load (local.get $cfg)))
    (local.set $key_ptr (i32.add (local.get $cfg) (i32.const 4)))
    (local.set $ctr_len (i32.load (i32.add (local.get $key_ptr) (local.get $key_len))))
    (local.set $ctr_ptr (i32.add (i32.add (local.get $key_ptr) (local.get $key_len)) (i32.const 4)))

    ;; Copy input to safe buffer
    (drop (call $pipe_read (local.get $input) (i32.const 0x6000) (local.get $read)))

    ;; XOR with keystream
    (local.set $result
      (call $aes128_ctr_xor
        (i32.const 0x5000)       ;; out
        (i32.const 0x6000)       ;; in
        (local.get $read)        ;; len
        (local.get $key_ptr)     ;; key
        (local.get $ctr_ptr)))   ;; ctr

    (if (local.get $result) (then (return (i32.sub (i32.const 0) (local.get $result)))))
    (drop (call $pipe_write (local.get $output) (i32.const 0x5000) (local.get $read)))
    local.get $read)
