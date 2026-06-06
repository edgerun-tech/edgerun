
(func (export "proto_standard_id") (result i32)
    i32.const 300011)

  (func $m110prefix_mask (param $prefix_bits i32) (result i32)
    (if (result i32)
      (i32.eq (local.get $prefix_bits) (i32.const 8))
      (then (i32.const 255))
      (else (i32.sub (i32.shl (i32.const 1) (local.get $prefix_bits)) (i32.const 1)))))

  ;; Decode output record, little-endian:
  ;; 0:u32 flags_high_bits
  ;; 4:u32 consumed
  ;; 8:u64 value
  (func $prefix_decode
    (param $in_ptr i32) (param $in_len i32) (param $prefix_bits i32) (param $out_ptr i32)
    (param $octet_limit i32)
    (result i32)
    (local $first i32)
    (local $mask i32)
    (local $flags i32)
    (local $value i64)
    (local $consumed i32)
    (local $shift i32)
    (local $byte i32)
    (if
      (i32.or
        (i32.lt_u (local.get $prefix_bits) (i32.const 1))
        (i32.gt_u (local.get $prefix_bits) (i32.const 8)))
      (then (return (i32.const 3))))
    (if (i32.eqz (local.get $in_len))
      (then (return (i32.const 1))))
    (local.set $mask (call $m110prefix_mask (local.get $prefix_bits)))
    (local.set $first (i32.load8_u (local.get $in_ptr)))
    (local.set $flags (i32.shr_u (local.get $first) (local.get $prefix_bits)))
    (local.set $value (i64.extend_i32_u (i32.and (local.get $first) (local.get $mask))))
    (if (i64.lt_u (local.get $value) (i64.extend_i32_u (local.get $mask)))
      (then
        (i32.store (local.get $out_ptr) (local.get $flags))
        (i32.store (i32.add (local.get $out_ptr) (i32.const 4)) (i32.const 1))
        (i64.store (i32.add (local.get $out_ptr) (i32.const 8)) (local.get $value))
        (return (i32.const 0))))
    (local.set $consumed (i32.const 1))
    (loop $again
      (if (i32.ge_u (local.get $consumed) (local.get $octet_limit))
        (then (return (i32.const 4))))
      (if (i32.ge_u (local.get $consumed) (local.get $in_len))
        (then (return (i32.const 1))))
      (local.set $byte (i32.load8_u (i32.add (local.get $in_ptr) (local.get $consumed))))
      (if
        (i32.and
          (i32.ge_u (local.get $shift) (i32.const 63))
          (i32.ne (i32.and (local.get $byte) (i32.const 127)) (i32.const 0)))
        (then (return (i32.const 4))))
      (local.set $value
        (i64.add
          (local.get $value)
          (i64.shl
            (i64.extend_i32_u (i32.and (local.get $byte) (i32.const 127)))
            (i64.extend_i32_u (local.get $shift)))))
      (local.set $consumed (i32.add (local.get $consumed) (i32.const 1)))
      (if (i32.eqz (i32.and (local.get $byte) (i32.const 128)))
        (then
          (i32.store (local.get $out_ptr) (local.get $flags))
          (i32.store (i32.add (local.get $out_ptr) (i32.const 4)) (local.get $consumed))
          (i64.store (i32.add (local.get $out_ptr) (i32.const 8)) (local.get $value))
          (return (i32.const 0))))
      (local.set $shift (i32.add (local.get $shift) (i32.const 7)))
      (br $again))
    (i32.const 1))

  (func $prefix_encode
    (param $value i64) (param $prefix_bits i32) (param $prefix_high_bits i32)
    (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (local $mask i32)
    (local $flags i32)
    (local $remaining i64)
    (local $written i32)
    (local $byte i32)
    (if
      (i32.or
        (i32.lt_u (local.get $prefix_bits) (i32.const 1))
        (i32.gt_u (local.get $prefix_bits) (i32.const 8)))
      (then (return (call $pack (i32.const 3) (i32.const 0)))))
    (if (i32.eqz (local.get $out_cap))
      (then (return (call $pack (i32.const 2) (i32.const 0)))))
    (local.set $mask (call $m110prefix_mask (local.get $prefix_bits)))
    (if
      (i32.ge_u
        (local.get $prefix_high_bits)
        (if (result i32)
          (i32.eq (local.get $prefix_bits) (i32.const 8))
          (then (i32.const 1))
          (else (i32.shl (i32.const 1) (i32.sub (i32.const 8) (local.get $prefix_bits))))))
      (then (return (call $pack (i32.const 3) (i32.const 0)))))
    (local.set $flags (i32.shl (local.get $prefix_high_bits) (local.get $prefix_bits)))
    (if (i64.lt_u (local.get $value) (i64.extend_i32_u (local.get $mask)))
      (then
        (i32.store8
          (local.get $out_ptr)
          (i32.or (local.get $flags) (i32.wrap_i64 (local.get $value))))
        (return (call $pack (i32.const 0) (i32.const 1)))))
    (i32.store8 (local.get $out_ptr) (i32.or (local.get $flags) (local.get $mask)))
    (local.set $remaining (i64.sub (local.get $value) (i64.extend_i32_u (local.get $mask))))
    (local.set $written (i32.const 1))
    (loop $again
      (if (i32.ge_u (local.get $written) (local.get $out_cap))
        (then (return (call $pack (i32.const 2) (local.get $written)))))
      (local.set $byte (i32.and (i32.wrap_i64 (local.get $remaining)) (i32.const 127)))
      (local.set $remaining (i64.shr_u (local.get $remaining) (i64.const 7)))
      (if (i64.ne (local.get $remaining) (i64.const 0))
        (then (local.set $byte (i32.or (local.get $byte) (i32.const 128)))))
      (i32.store8 (i32.add (local.get $out_ptr) (local.get $written)) (local.get $byte))
      (local.set $written (i32.add (local.get $written) (i32.const 1)))
      (br_if $again (i64.ne (local.get $remaining) (i64.const 0))))
    (call $pack (i32.const 0) (local.get $written)))

  (func (export "hpack_prefix_int_decode")
    (param $in_ptr i32) (param $in_len i32) (param $prefix_bits i32) (param $out_ptr i32)
    (result i32)
    (call $prefix_decode
      (local.get $in_ptr) (local.get $in_len) (local.get $prefix_bits) (local.get $out_ptr)
      (i32.const 5)))

  (func (export "hpack_prefix_int_encode")
    (param $value i64) (param $prefix_bits i32) (param $prefix_high_bits i32)
    (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (call $prefix_encode
      (local.get $value) (local.get $prefix_bits) (local.get $prefix_high_bits)
      (local.get $out_ptr) (local.get $out_cap)))

  (func (export "qpack_prefix_int_decode")
    (param $in_ptr i32) (param $in_len i32) (param $prefix_bits i32) (param $out_ptr i32)
    (result i32)
    (call $prefix_decode
      (local.get $in_ptr) (local.get $in_len) (local.get $prefix_bits) (local.get $out_ptr)
      (i32.const 10)))

  (func (export "qpack_prefix_int_encode")
    (param $value i64) (param $prefix_bits i32) (param $prefix_high_bits i32)
    (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (call $prefix_encode
      (local.get $value) (local.get $prefix_bits) (local.get $prefix_high_bits)
      (local.get $out_ptr) (local.get $out_cap)))
