  (import "math" "prefix_mask" (func $m152prefix_mask (param i32) (result i32)))

(func (export "proto_standard_id") (result i32)
    i32.const 300036)

  ;; Prefix output record, little-endian:
  ;; 0:u32 flags_high_bits, 4:u32 consumed, 8:u64 value.
  (func $m152qpack_prefix_decode
    (param $in_ptr i32) (param $in_len i32) (param $prefix_bits i32) (param $out_ptr i32)
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
    (local.set $mask (call $m152prefix_mask (local.get $prefix_bits)))
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
      (if (i32.ge_u (local.get $consumed) (i32.const 10))
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

  ;; Decoder instruction record, little-endian:
  ;; 0:u32 kind (1 header_ack, 2 stream_cancel, 3 insert_count_increment)
  ;; 4:u32 offset
  ;; 8:u32 consumed
  ;; 12:u32 flags_high_bits
  ;; 16:u64 value
  (func $m152write_record
    (param $out_ptr i32) (param $kind i32) (param $offset i32) (param $prefix_ptr i32)
    (i32.store (local.get $out_ptr) (local.get $kind))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 4)) (local.get $offset))
    (i32.store
      (i32.add (local.get $out_ptr) (i32.const 8))
      (i32.load (i32.add (local.get $prefix_ptr) (i32.const 4))))
    (i32.store
      (i32.add (local.get $out_ptr) (i32.const 12))
      (i32.load (local.get $prefix_ptr)))
    (i64.store
      (i32.add (local.get $out_ptr) (i32.const 16))
      (i64.load (i32.add (local.get $prefix_ptr) (i32.const 8)))))

  (func $qpack_decoder_instruction_decode (export "qpack_decoder_instruction_decode")
    (param $in_ptr i32) (param $in_len i32) (param $out_ptr i32)
    (result i32)
    (local $first i32)
    (local $kind i32)
    (local $prefix_bits i32)
    (local $status i32)
    (local $flags i32)
    (local $value i64)
    (if (i32.eqz (local.get $in_len))
      (then (return (i32.const 1))))
    (local.set $first (i32.load8_u (local.get $in_ptr)))
    (if
      (i32.eq (i32.and (local.get $first) (i32.const 192)) (i32.const 0))
      (then
        (local.set $kind (i32.const 3))
        (local.set $prefix_bits (i32.const 6)))
      (else
        (if
          (i32.ne (i32.and (local.get $first) (i32.const 128)) (i32.const 0))
          (then
            (local.set $kind (i32.const 1))
            (local.set $prefix_bits (i32.const 7)))
          (else
            (local.set $kind (i32.const 2))
            (local.set $prefix_bits (i32.const 6))))))
    (local.set $status
      (call $m152qpack_prefix_decode
        (local.get $in_ptr) (local.get $in_len) (local.get $prefix_bits) (i32.const 65472)))
    (if (local.get $status) (then (return (local.get $status))))
    (local.set $flags (i32.load (i32.const 65472)))
    (local.set $value (i64.load (i32.const 65480)))
    (if
      (i32.and
        (i32.eq (local.get $kind) (i32.const 3))
        (i64.gt_u (local.get $value) (i64.const 64)))
      (then (return (i32.const 4))))
    (if
      (i32.and
        (i32.eq (local.get $kind) (i32.const 1))
        (i32.ne (local.get $flags) (i32.const 1)))
      (then (return (i32.const 3))))
    (if
      (i32.and
        (i32.eq (local.get $kind) (i32.const 2))
        (i32.ne (local.get $flags) (i32.const 1)))
      (then (return (i32.const 3))))
    (if
      (i32.and
        (i32.eq (local.get $kind) (i32.const 3))
        (i32.ne (local.get $flags) (i32.const 0)))
      (then (return (i32.const 3))))
    (call $m152write_record (local.get $out_ptr) (local.get $kind) (i32.const 0) (i32.const 65472))
    (i32.const 0))

  (func (export "qpack_decoder_stream_scan")
    (param $in_ptr i32) (param $in_len i32) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (local $pos i32)
    (local $count i32)
    (local $status i32)
    (local $record_ptr i32)
    (loop $again
      (if (i32.ge_u (local.get $pos) (local.get $in_len))
        (then (return (call $pack (i32.const 0) (local.get $count)))))
      (if (i32.gt_u (i32.add (i32.mul (local.get $count) (i32.const 24)) (i32.const 24)) (local.get $out_cap))
        (then (return (call $pack (i32.const 2) (local.get $count)))))
      (local.set $record_ptr
        (i32.add (local.get $out_ptr) (i32.mul (local.get $count) (i32.const 24))))
      (local.set $status
        (call $qpack_decoder_instruction_decode
          (i32.add (local.get $in_ptr) (local.get $pos))
          (i32.sub (local.get $in_len) (local.get $pos))
          (local.get $record_ptr)))
      (if (local.get $status)
        (then (return (call $pack (local.get $status) (local.get $count)))))
      (i32.store (i32.add (local.get $record_ptr) (i32.const 4)) (local.get $pos))
      (local.set $pos
        (i32.add
          (local.get $pos)
          (i32.load (i32.add (local.get $record_ptr) (i32.const 8)))))
      (local.set $count (i32.add (local.get $count) (i32.const 1)))
      (br $again))
    (call $pack (i32.const 0) (local.get $count)))
