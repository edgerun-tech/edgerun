(func $m151b (param $ptr i32) (param $off i32) (result i32)
    (i32.load8_u (i32.add (local.get $ptr) (local.get $off))))

  (func $m151u16be (param $ptr i32) (param $off i32) (result i32)
    (i32.or
      (i32.shl (call $m151b (local.get $ptr) (local.get $off)) (i32.const 8))
      (call $m151b (local.get $ptr) (i32.add (local.get $off) (i32.const 1)))))

  (func $u32be (param $ptr i32) (param $off i32) (result i32)
    (i32.or
      (i32.shl (call $m151b (local.get $ptr) (local.get $off)) (i32.const 24))
      (i32.or
        (i32.shl (call $m151b (local.get $ptr) (i32.add (local.get $off) (i32.const 1))) (i32.const 16))
        (i32.or
          (i32.shl (call $m151b (local.get $ptr) (i32.add (local.get $off) (i32.const 2))) (i32.const 8))
          (call $m151b (local.get $ptr) (i32.add (local.get $off) (i32.const 3)))))))

  (func $u32le (param $ptr i32) (result i32)
    (i32.or
      (call $m151b (local.get $ptr) (i32.const 0))
      (i32.or
        (i32.shl (call $m151b (local.get $ptr) (i32.const 1)) (i32.const 8))
        (i32.or
          (i32.shl (call $m151b (local.get $ptr) (i32.const 2)) (i32.const 16))
          (i32.shl (call $m151b (local.get $ptr) (i32.const 3)) (i32.const 24))))))

  (func $m151upper (param $b i32) (result i32)
    (if (result i32)
      (i32.and (i32.ge_u (local.get $b) (i32.const 97)) (i32.le_u (local.get $b) (i32.const 122)))
      (then (i32.sub (local.get $b) (i32.const 32)))
      (else (local.get $b))))
  (func $eq_ci_byte (param $ptr i32) (param $off i32) (param $c i32) (result i32)
    (i32.eq (call $m151upper (call $m151b (local.get $ptr) (local.get $off))) (local.get $c)))

  ;; TFTP opcodes: 1 RRQ, 2 WRQ, 3 DATA, 4 ACK, 5 ERROR, 6 OACK. Returns 0 for invalid wire.
  (func (export "tftp_message_kind") (param $ptr i32) (param $len i32) (result i32)
    (local $op i32)
    (if (i32.lt_u (local.get $len) (i32.const 2)) (then (return (i32.const 0))))
    (local.set $op (call $m151u16be (local.get $ptr) (i32.const 0)))
    (if (i32.or (i32.lt_u (local.get $op) (i32.const 1)) (i32.gt_u (local.get $op) (i32.const 6)))
      (then (return (i32.const 0))))
    (if
      (i32.and
        (i32.or
          (i32.or (i32.eq (local.get $op) (i32.const 3)) (i32.eq (local.get $op) (i32.const 4)))
          (i32.eq (local.get $op) (i32.const 5)))
        (i32.lt_u (local.get $len) (i32.const 4)))
      (then (return (i32.const 0))))
    local.get $op)

  ;; TFTP error classes: 0 undefined, 1 file-not-found, 2 access, 3 full,
  ;; 4 illegal-operation, 5 unknown-tid, 6 exists, 7 no-user, -1 invalid.
  (func (export "tftp_error_class") (param $code i32) (result i32)
    (if (i32.le_u (local.get $code) (i32.const 7)) (then (return (local.get $code))))
    i32.const -1)

  ;; TFTP option names: blksize=1, tsize=2, timeout=3, unknown=0.
  (func (export "tftp_option_code") (param $ptr i32) (param $len i32) (result i32)
    (if (i32.eq (local.get $len) (i32.const 7))
      (then
        (if
          (i32.and
            (i32.and (call $eq_ci_byte (local.get $ptr) (i32.const 0) (i32.const 66)) (call $eq_ci_byte (local.get $ptr) (i32.const 1) (i32.const 76)))
            (i32.and
              (i32.and (call $eq_ci_byte (local.get $ptr) (i32.const 2) (i32.const 75)) (call $eq_ci_byte (local.get $ptr) (i32.const 3) (i32.const 83)))
              (i32.and
                (i32.and (call $eq_ci_byte (local.get $ptr) (i32.const 4) (i32.const 73)) (call $eq_ci_byte (local.get $ptr) (i32.const 5) (i32.const 90)))
                (call $eq_ci_byte (local.get $ptr) (i32.const 6) (i32.const 69)))))
          (then (return (i32.const 1))))
        (if
          (i32.and
            (i32.and (call $eq_ci_byte (local.get $ptr) (i32.const 0) (i32.const 84)) (call $eq_ci_byte (local.get $ptr) (i32.const 1) (i32.const 73)))
            (i32.and
              (i32.and (call $eq_ci_byte (local.get $ptr) (i32.const 2) (i32.const 77)) (call $eq_ci_byte (local.get $ptr) (i32.const 3) (i32.const 69)))
              (i32.and
                (i32.and (call $eq_ci_byte (local.get $ptr) (i32.const 4) (i32.const 79)) (call $eq_ci_byte (local.get $ptr) (i32.const 5) (i32.const 85)))
                (call $eq_ci_byte (local.get $ptr) (i32.const 6) (i32.const 84)))))
          (then (return (i32.const 3))))))
    (if
      (i32.and
        (i32.eq (local.get $len) (i32.const 5))
        (i32.and
          (i32.and (call $eq_ci_byte (local.get $ptr) (i32.const 0) (i32.const 84)) (call $eq_ci_byte (local.get $ptr) (i32.const 1) (i32.const 83)))
          (i32.and
            (i32.and (call $eq_ci_byte (local.get $ptr) (i32.const 2) (i32.const 73)) (call $eq_ci_byte (local.get $ptr) (i32.const 3) (i32.const 90)))
            (call $eq_ci_byte (local.get $ptr) (i32.const 4) (i32.const 69)))))
      (then (return (i32.const 2))))
    i32.const 0)

  (func (export "tftp_blksize_clamp") (param $n i32) (result i32)
    (if (i32.lt_u (local.get $n) (i32.const 8)) (then (return (i32.const 8))))
    (if (i32.gt_u (local.get $n) (i32.const 65464)) (then (return (i32.const 65464))))
    local.get $n)

  ;; Read-transfer decision codes: 0 ignore, 1 send-data, 2 send-oack, 3 finish,
  ;; 4 error-not-found, 5 error-access, 6 error-illegal.
  (func (export "tftp_read_action")
    (param $opcode i32) (param $mode_octet i32) (param $file_exists i32)
    (param $options_changed i32) (param $ack_matches i32) (param $eof i32)
    (result i32)
    (if (i32.eq (local.get $opcode) (i32.const 1))
      (then
        (if (i32.eqz (local.get $mode_octet)) (then (return (i32.const 6))))
        (if (i32.eqz (local.get $file_exists)) (then (return (i32.const 4))))
        (if (local.get $options_changed) (then (return (i32.const 2))))
        (return (i32.const 1))))
    (if (i32.eq (local.get $opcode) (i32.const 2)) (then (return (i32.const 5))))
    (if (i32.eq (local.get $opcode) (i32.const 4))
      (then
        (if (i32.eqz (local.get $ack_matches)) (then (return (i32.const 0))))
        (if (local.get $eof) (then (return (i32.const 3))))
        (return (i32.const 1))))
    (if (i32.eq (local.get $opcode) (i32.const 5)) (then (return (i32.const 3))))
    (if (i32.or (i32.eq (local.get $opcode) (i32.const 3)) (i32.eq (local.get $opcode) (i32.const 6)))
      (then (return (i32.const 6))))
    i32.const 0)

  ;; Block protocol errors: 1 read-only, 2 out-of-range, 3 misaligned, 4 unsupported,
  ;; 5 timeout, 6 not-ready, 7 backend/protocol failure. Returns Linux/NBD errno.
  (func (export "nbd_errno_from_block_error") (param $err i32) (result i32)
    (if (i32.eq (local.get $err) (i32.const 1)) (then (return (i32.const 30))))
    (if (i32.or (i32.eq (local.get $err) (i32.const 2)) (i32.eq (local.get $err) (i32.const 3))) (then (return (i32.const 22))))
    (if (i32.eq (local.get $err) (i32.const 4)) (then (return (i32.const 95))))
    (if (i32.eq (local.get $err) (i32.const 5)) (then (return (i32.const 110))))
    (if (i32.eq (local.get $err) (i32.const 6)) (then (return (i32.const 11))))
    (if (i32.eq (local.get $err) (i32.const 7)) (then (return (i32.const 5))))
    i32.const 0)

  ;; NBD frame classes: 0 unknown/short, 1 server-handshake, 2 option-request,
  ;; 3 request, 4 reply, 5 option-reply.
  (func (export "nbd_frame_kind") (param $ptr i32) (param $len i32) (result i32)
    (if
      (i32.and
        (i32.ge_u (local.get $len) (i32.const 18))
        (i32.and
          (i32.eq (call $u32be (local.get $ptr) (i32.const 0)) (i32.const 0x4e42444d))
          (i32.and
            (i32.eq (call $u32be (local.get $ptr) (i32.const 4)) (i32.const 0x41474943))
            (i32.and
              (i32.eq (call $u32be (local.get $ptr) (i32.const 8)) (i32.const 0x49484156))
              (i32.eq (call $u32be (local.get $ptr) (i32.const 12)) (i32.const 0x454f5054))))))
      (then (return (i32.const 1))))
    (if
      (i32.and
        (i32.ge_u (local.get $len) (i32.const 16))
        (i32.and
          (i32.eq (call $u32be (local.get $ptr) (i32.const 0)) (i32.const 0x49484156))
          (i32.eq (call $u32be (local.get $ptr) (i32.const 4)) (i32.const 0x454f5054))))
      (then (return (i32.const 2))))
    (if
      (i32.and
        (i32.ge_u (local.get $len) (i32.const 28))
        (i32.eq (call $u32be (local.get $ptr) (i32.const 0)) (i32.const 0x25609513)))
      (then (return (i32.const 3))))
    (if
      (i32.and
        (i32.ge_u (local.get $len) (i32.const 16))
        (i32.eq (call $u32be (local.get $ptr) (i32.const 0)) (i32.const 0x67446698)))
      (then (return (i32.const 4))))
    (if
      (i32.and
        (i32.ge_u (local.get $len) (i32.const 20))
        (i32.and
          (i32.eq (call $u32be (local.get $ptr) (i32.const 0)) (i32.const 0x0003e889))
          (i32.eq (call $u32be (local.get $ptr) (i32.const 4)) (i32.const 0x045565a9))))
      (then (return (i32.const 5))))
    i32.const 0)

  ;; NBD command classes: 1 read, 2 write, 3 disconnect, 4 flush, 5 trim, 6 write-zeroes.
  (func (export "nbd_command_class") (param $cmd i32) (result i32)
    (if (i32.eq (local.get $cmd) (i32.const 0)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $cmd) (i32.const 1)) (then (return (i32.const 2))))
    (if (i32.eq (local.get $cmd) (i32.const 2)) (then (return (i32.const 3))))
    (if (i32.eq (local.get $cmd) (i32.const 3)) (then (return (i32.const 4))))
    (if (i32.eq (local.get $cmd) (i32.const 4)) (then (return (i32.const 5))))
    (if (i32.eq (local.get $cmd) (i32.const 6)) (then (return (i32.const 6))))
    i32.const 0)

  (func (export "nbd_transmission_flags")
    (param $readonly i32) (param $flush i32) (param $discard i32) (param $zeroes i32)
    (result i32)
    (local $flags i32)
    (local.set $flags (i32.const 1))
    (if (local.get $readonly) (then (local.set $flags (i32.or (local.get $flags) (i32.const 2)))))
    (if (local.get $flush) (then (local.set $flags (i32.or (local.get $flags) (i32.const 4)))))
    (if (local.get $discard) (then (local.set $flags (i32.or (local.get $flags) (i32.const 32)))))
    (if (local.get $zeroes) (then (local.set $flags (i32.or (local.get $flags) (i32.const 64)))))
    local.get $flags)

  ;; Block frame status: 0..16777216 payload length, -1 truncated, -2 too-large, -3 mismatched length.
  (func (export "block_frame_payload_len") (param $ptr i32) (param $len i32) (result i32)
    (local $n i32)
    (if (i32.lt_u (local.get $len) (i32.const 4)) (then (return (i32.const -1))))
    (local.set $n (call $u32le (local.get $ptr)))
    (if (i32.gt_u (local.get $n) (i32.const 16777216)) (then (return (i32.const -2))))
    (if (i32.ne (local.get $len) (i32.add (local.get $n) (i32.const 4))) (then (return (i32.const -3))))
    local.get $n)

  ;; Validates range and payload length. Status: 0 ok, 1 protocol, 2 out-of-range, 3 misaligned.
  (func (export "block_transfer_validate")
    (param $m151block_size i32) (param $m151block_count_low i32) (param $lba_low i32) (param $m151blocks i32) (param $actual_len i32)
    (result i32)
    (local $end i32)
    (local $expected i32)
    (if (i32.or (i32.eqz (local.get $m151block_size)) (i32.eqz (local.get $m151blocks))) (then (return (i32.const 1))))
    (local.set $end (i32.add (local.get $lba_low) (local.get $m151blocks)))
    (if (i32.or (i32.lt_u (local.get $end) (local.get $lba_low)) (i32.gt_u (local.get $end) (local.get $m151block_count_low)))
      (then (return (i32.const 2))))
    (local.set $expected (i32.mul (local.get $m151blocks) (local.get $m151block_size)))
    (if (i32.ne (local.get $actual_len) (local.get $expected)) (then (return (i32.const 3))))
    i32.const 0)
