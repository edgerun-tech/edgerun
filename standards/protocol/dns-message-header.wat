
(func (export "proto_standard_id") (result i32)
    i32.const 300026)

  ;; Status values: 0 ok, 1 input_short, 2 output_short, 3 invalid.

  (func $m79read_u16_be (param $ptr i32) (result i32)
    (i32.or
      (i32.shl (i32.load8_u (local.get $ptr)) (i32.const 8))
      (i32.load8_u (i32.add (local.get $ptr) (i32.const 1)))))

  (func $write_u16_be (param $ptr i32) (param $v i32)
    (i32.store8 (local.get $ptr) (i32.shr_u (local.get $v) (i32.const 8)))
    (i32.store8 (i32.add (local.get $ptr) (i32.const 1)) (local.get $v)))

  (func $valid_counts (param $qd i32) (param $an i32) (param $ns i32) (param $ar i32) (result i32)
    (i32.and
      (i32.le_u (local.get $qd) (i32.const 16))
      (i32.and
        (i32.le_u (local.get $an) (i32.const 256))
        (i32.and
          (i32.le_u (local.get $ns) (i32.const 256))
          (i32.le_u (local.get $ar) (i32.const 256))))))

  ;; Packed classifier:
  ;; bit0 QR, bits1..4 opcode, bit5 AA, bit6 TC, bit7 RD, bit8 RA, bits9..12 RCODE.
  (func (export "dns_header_flags_classify") (param $flags i32) (result i32)
    (i32.or
      (i32.or
        (i32.shr_u (i32.and (local.get $flags) (i32.const 0x8000)) (i32.const 15))
        (i32.shl (i32.and (i32.shr_u (local.get $flags) (i32.const 11)) (i32.const 0x0f)) (i32.const 1)))
      (i32.or
        (i32.or
          (i32.shr_u (i32.and (local.get $flags) (i32.const 0x0400)) (i32.const 5))
          (i32.shr_u (i32.and (local.get $flags) (i32.const 0x0200)) (i32.const 3)))
        (i32.or
          (i32.or
            (i32.shr_u (i32.and (local.get $flags) (i32.const 0x0100)) (i32.const 1))
            (i32.shl (i32.shr_u (i32.and (local.get $flags) (i32.const 0x0080)) (i32.const 7)) (i32.const 8)))
          (i32.shl (i32.and (local.get $flags) (i32.const 0x000f)) (i32.const 9))))))

  ;; Output record, little-endian:
  ;; 0:id, 4:flags, 8:qr, 12:opcode, 16:aa, 20:tc, 24:rd, 28:ra,
  ;; 32:rcode, 36:qdcount, 40:ancount, 44:nscount, 48:arcount.
  (func (export "dns_header_decode")
    (param $in_ptr i32) (param $in_len i32) (param $out_ptr i32)
    (result i32)
    (local $id i32)
    (local $flags i32)
    (local $qd i32)
    (local $an i32)
    (local $ns i32)
    (local $ar i32)
    (if (i32.lt_u (local.get $in_len) (i32.const 12))
      (then (return (i32.const 1))))
    (local.set $id (call $m79read_u16_be (local.get $in_ptr)))
    (local.set $flags (call $m79read_u16_be (i32.add (local.get $in_ptr) (i32.const 2))))
    (local.set $qd (call $m79read_u16_be (i32.add (local.get $in_ptr) (i32.const 4))))
    (local.set $an (call $m79read_u16_be (i32.add (local.get $in_ptr) (i32.const 6))))
    (local.set $ns (call $m79read_u16_be (i32.add (local.get $in_ptr) (i32.const 8))))
    (local.set $ar (call $m79read_u16_be (i32.add (local.get $in_ptr) (i32.const 10))))
    (if (i32.eqz (call $valid_counts (local.get $qd) (local.get $an) (local.get $ns) (local.get $ar)))
      (then (return (i32.const 3))))
    (i32.store (local.get $out_ptr) (local.get $id))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 4)) (local.get $flags))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 8))
      (i32.shr_u (i32.and (local.get $flags) (i32.const 0x8000)) (i32.const 15)))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 12))
      (i32.and (i32.shr_u (local.get $flags) (i32.const 11)) (i32.const 0x0f)))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 16))
      (i32.shr_u (i32.and (local.get $flags) (i32.const 0x0400)) (i32.const 10)))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 20))
      (i32.shr_u (i32.and (local.get $flags) (i32.const 0x0200)) (i32.const 9)))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 24))
      (i32.shr_u (i32.and (local.get $flags) (i32.const 0x0100)) (i32.const 8)))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 28))
      (i32.shr_u (i32.and (local.get $flags) (i32.const 0x0080)) (i32.const 7)))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 32))
      (i32.and (local.get $flags) (i32.const 0x000f)))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 36)) (local.get $qd))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 40)) (local.get $an))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 44)) (local.get $ns))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 48)) (local.get $ar))
    (i32.const 0))

  ;; Return bits: low32=status, high32=written.
  (func (export "dns_header_encode")
    (param $id i32) (param $flags i32) (param $qd i32) (param $an i32) (param $ns i32) (param $ar i32)
    (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (if (i32.lt_u (local.get $out_cap) (i32.const 12))
      (then (return (call $pack (i32.const 2) (i32.const 0)))))
    (if
      (i32.or
        (i32.or (i32.gt_u (local.get $id) (i32.const 65535)) (i32.gt_u (local.get $flags) (i32.const 65535)))
        (i32.eqz (call $valid_counts (local.get $qd) (local.get $an) (local.get $ns) (local.get $ar))))
      (then (return (call $pack (i32.const 3) (i32.const 0)))))
    (call $write_u16_be (local.get $out_ptr) (local.get $id))
    (call $write_u16_be (i32.add (local.get $out_ptr) (i32.const 2)) (local.get $flags))
    (call $write_u16_be (i32.add (local.get $out_ptr) (i32.const 4)) (local.get $qd))
    (call $write_u16_be (i32.add (local.get $out_ptr) (i32.const 6)) (local.get $an))
    (call $write_u16_be (i32.add (local.get $out_ptr) (i32.const 8)) (local.get $ns))
    (call $write_u16_be (i32.add (local.get $out_ptr) (i32.const 10)) (local.get $ar))
    (call $pack (i32.const 0) (i32.const 12)))
