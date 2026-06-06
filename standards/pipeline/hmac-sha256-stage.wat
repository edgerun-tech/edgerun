  ;; HMAC-SHA256 Pipeline Stage — slot 17
  ;; Stage type: batch (state=0)
  ;; Input:  message bytes via input pipe
  ;; Output: 32-byte HMAC-SHA256 tag via output pipe
  ;; Config: [key_len: i32][key_bytes: key_len]
  ;; Calls $hmac_sha256 from crypto/crypto-hmac-sha256.wat (merged in module scope)

  ;; Pipeline stage: HMAC-SHA256 (batch, zero-copy input)
  ;; Config layout: [klen: i32][key: klen]
  (func (export "process_hmac_sha256")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $len_slot i32) (local $in_ptr i32) (local $read i32)
    (local $klen i32)
    (if (i32.lt_u (local.get $clen) (i32.const 4))
      (then (return (i32.const -1))))
    (local.set $klen (i32.load (local.get $cfg)))
    (if (i32.lt_u (local.get $clen) (i32.add (i32.const 4) (local.get $klen)))
      (then (return (i32.const -1))))
    (local.set $len_slot (i32.sub (i32.add (local.get $scratch) (local.get $scap)) (i32.const 4)))
    (local.set $in_ptr (call $pipe_read_ptr (local.get $input) (local.get $len_slot)))
    (local.set $read (i32.load (local.get $len_slot)))
    (if (i32.eqz (local.get $read)) (then (return (i32.const 0))))
    (drop (call $hmac_sha256
      (i32.add (local.get $cfg) (i32.const 4)) (local.get $klen)
      (local.get $in_ptr) (local.get $read)
      (local.get $scratch) (local.get $scap)))
    (call $pipe_advance (local.get $input) (local.get $read))
    (drop (call $pipe_write (local.get $output) (local.get $scratch) (i32.const 32)))
    i32.const 32)
