
(func (export "proto_standard_id") (result i32)
    i32.const 300056)

  ;; Status values: 0 ok, 1 input_short, 2 output_short, 4 overflow.
  ;; Packed encode return: low32=status, high32=written.

  ;; Output record, little-endian u32:
  ;; 0: len_low
  ;; 4: len_high
  ;; 8: header_len
  (func $m129write_record
    (param $out i32) (param $len_low i32) (param $len_high i32) (param $header_len i32)
    (i32.store (local.get $out) (local.get $len_low))
    (i32.store (i32.add (local.get $out) (i32.const 4)) (local.get $len_high))
    (i32.store (i32.add (local.get $out) (i32.const 8)) (local.get $header_len)))

  (func (export "frame_header_decode_u16_be")
    (param $ptr i32) (param $len i32) (param $out i32)
    (result i32)
    (local $payload_len i32)
    (if (i32.lt_u (local.get $len) (i32.const 2))
      (then (return (i32.const 1))))
    (local.set $payload_len
      (i32.or
        (i32.shl (i32.load8_u (local.get $ptr)) (i32.const 8))
        (i32.load8_u (i32.add (local.get $ptr) (i32.const 1)))))
    (call $m129write_record (local.get $out) (local.get $payload_len) (i32.const 0) (i32.const 2))
    (i32.const 0))

  (func (export "frame_header_encode_u16_be")
    (param $payload_len i32) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (if (i32.lt_u (local.get $out_cap) (i32.const 2))
      (then (return (call $pack (i32.const 2) (i32.const 0)))))
    (if (i32.gt_u (local.get $payload_len) (i32.const 65535))
      (then (return (call $pack (i32.const 4) (i32.const 0)))))
    (i32.store8 (local.get $out_ptr) (i32.shr_u (local.get $payload_len) (i32.const 8)))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 1)) (local.get $payload_len))
    (call $pack (i32.const 0) (i32.const 2)))

  (func (export "frame_header_decode_u64_be")
    (param $ptr i32) (param $len i32) (param $out i32)
    (result i32)
    (local $hi i32)
    (local $lo i32)
    (if (i32.lt_u (local.get $len) (i32.const 8))
      (then (return (i32.const 1))))
    (local.set $hi
      (i32.or
        (i32.or
          (i32.shl (i32.load8_u (local.get $ptr)) (i32.const 24))
          (i32.shl (i32.load8_u (i32.add (local.get $ptr) (i32.const 1))) (i32.const 16)))
        (i32.or
          (i32.shl (i32.load8_u (i32.add (local.get $ptr) (i32.const 2))) (i32.const 8))
          (i32.load8_u (i32.add (local.get $ptr) (i32.const 3))))))
    (local.set $lo
      (i32.or
        (i32.or
          (i32.shl (i32.load8_u (i32.add (local.get $ptr) (i32.const 4))) (i32.const 24))
          (i32.shl (i32.load8_u (i32.add (local.get $ptr) (i32.const 5))) (i32.const 16)))
        (i32.or
          (i32.shl (i32.load8_u (i32.add (local.get $ptr) (i32.const 6))) (i32.const 8))
          (i32.load8_u (i32.add (local.get $ptr) (i32.const 7))))))
    (call $m129write_record (local.get $out) (local.get $lo) (local.get $hi) (i32.const 8))
    (i32.const 0))

  (func (export "frame_header_encode_u64_be")
    (param $payload_len_low i32) (param $payload_len_high i32) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (if (i32.lt_u (local.get $out_cap) (i32.const 8))
      (then (return (call $pack (i32.const 2) (i32.const 0)))))
    (i32.store8 (local.get $out_ptr) (i32.shr_u (local.get $payload_len_high) (i32.const 24)))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 1))
      (i32.shr_u (local.get $payload_len_high) (i32.const 16)))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 2))
      (i32.shr_u (local.get $payload_len_high) (i32.const 8)))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 3)) (local.get $payload_len_high))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 4))
      (i32.shr_u (local.get $payload_len_low) (i32.const 24)))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 5))
      (i32.shr_u (local.get $payload_len_low) (i32.const 16)))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 6))
      (i32.shr_u (local.get $payload_len_low) (i32.const 8)))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 7)) (local.get $payload_len_low))
    (call $pack (i32.const 0) (i32.const 8)))

  (func (export "frame_header_decode_u64_le")
    (param $ptr i32) (param $len i32) (param $out i32)
    (result i32)
    (local $hi i32)
    (local $lo i32)
    (if (i32.lt_u (local.get $len) (i32.const 8))
      (then (return (i32.const 1))))
    (local.set $lo
      (i32.or
        (i32.or
          (i32.load8_u (local.get $ptr))
          (i32.shl (i32.load8_u (i32.add (local.get $ptr) (i32.const 1))) (i32.const 8)))
        (i32.or
          (i32.shl (i32.load8_u (i32.add (local.get $ptr) (i32.const 2))) (i32.const 16))
          (i32.shl (i32.load8_u (i32.add (local.get $ptr) (i32.const 3))) (i32.const 24)))))
    (local.set $hi
      (i32.or
        (i32.or
          (i32.load8_u (i32.add (local.get $ptr) (i32.const 4)))
          (i32.shl (i32.load8_u (i32.add (local.get $ptr) (i32.const 5))) (i32.const 8)))
        (i32.or
          (i32.shl (i32.load8_u (i32.add (local.get $ptr) (i32.const 6))) (i32.const 16))
          (i32.shl (i32.load8_u (i32.add (local.get $ptr) (i32.const 7))) (i32.const 24)))))
    (call $m129write_record (local.get $out) (local.get $lo) (local.get $hi) (i32.const 8))
    (i32.const 0))

  (func (export "frame_header_encode_u64_le")
    (param $payload_len_low i32) (param $payload_len_high i32) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (if (i32.lt_u (local.get $out_cap) (i32.const 8))
      (then (return (call $pack (i32.const 2) (i32.const 0)))))
    (i32.store8 (local.get $out_ptr) (local.get $payload_len_low))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 1))
      (i32.shr_u (local.get $payload_len_low) (i32.const 8)))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 2))
      (i32.shr_u (local.get $payload_len_low) (i32.const 16)))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 3))
      (i32.shr_u (local.get $payload_len_low) (i32.const 24)))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 4)) (local.get $payload_len_high))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 5))
      (i32.shr_u (local.get $payload_len_high) (i32.const 8)))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 6))
      (i32.shr_u (local.get $payload_len_high) (i32.const 16)))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 7))
      (i32.shr_u (local.get $payload_len_high) (i32.const 24)))
    (call $pack (i32.const 0) (i32.const 8)))
