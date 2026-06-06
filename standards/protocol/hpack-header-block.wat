
(func (export "proto_standard_id") (result i32)
    i32.const 300038)

  (func $m102prefix_mask (param $prefix_bits i32) (result i32)
    (if (result i32)
      (i32.eq (local.get $prefix_bits) (i32.const 8))
      (then (i32.const 255))
      (else (i32.sub (i32.shl (i32.const 1) (local.get $prefix_bits)) (i32.const 1)))))

  ;; Prefix output record, little-endian:
  ;; 0:u32 flags_high_bits, 4:u32 consumed, 8:u64 value.
  (func $m102hpack_prefix_decode
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
    (local.set $mask (call $m102prefix_mask (local.get $prefix_bits)))
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

  ;; String meta record, little-endian:
  ;; 0:u32 flags, 4:u32 huffman, 8:u32 consumed, 12:u32 payload_offset, 16:u32 payload_len.
  (func $hpack_string_scan
    (param $in_ptr i32) (param $in_len i32) (param $meta_ptr i32)
    (result i32)
    (local $status i32)
    (local $flags i32)
    (local $consumed i32)
    (local $payload_len i64)
    (local.set $status
      (call $m102hpack_prefix_decode
        (local.get $in_ptr) (local.get $in_len) (i32.const 7) (local.get $meta_ptr)))
    (if (local.get $status) (then (return (local.get $status))))
    (local.set $flags (i32.load (local.get $meta_ptr)))
    (local.set $consumed (i32.load (i32.add (local.get $meta_ptr) (i32.const 4))))
    (local.set $payload_len (i64.load (i32.add (local.get $meta_ptr) (i32.const 8))))
    (if
      (i64.gt_u
        (local.get $payload_len)
        (i64.extend_i32_u (i32.sub (local.get $in_len) (local.get $consumed))))
      (then (return (i32.const 1))))
    (if
      (i64.gt_u (local.get $payload_len) (i64.const 4294967295))
      (then (return (i32.const 4))))
    (i32.store (local.get $meta_ptr) (local.get $flags))
    (i32.store (i32.add (local.get $meta_ptr) (i32.const 4)) (i32.and (local.get $flags) (i32.const 1)))
    (i32.store (i32.add (local.get $meta_ptr) (i32.const 8)) (local.get $consumed))
    (i32.store (i32.add (local.get $meta_ptr) (i32.const 12)) (local.get $consumed))
    (i32.store (i32.add (local.get $meta_ptr) (i32.const 16)) (i32.wrap_i64 (local.get $payload_len)))
    (i32.const 0))

  ;; Header instruction record, little-endian:
  ;; 0:u32 kind (1 indexed_header, 2 literal_indexed_name,
  ;;             3 literal_new_name, 4 never_indexed, 5 dynamic_table_size_update)
  ;; 4:u32 offset
  ;; 8:u32 consumed
  ;; 12:u32 flags_high_bits
  ;; 16:u64 value (header index, name index, or dynamic table size)
  ;; 24:u32 name_payload_offset
  ;; 28:u32 name_payload_len
  ;; 32:u32 name_huffman
  ;; 36:u32 value_payload_offset
  ;; 40:u32 value_payload_len
  ;; 44:u32 value_huffman
  (func $m102clear_record (param $out_ptr i32)
    (i64.store (local.get $out_ptr) (i64.const 0))
    (i64.store (i32.add (local.get $out_ptr) (i32.const 8)) (i64.const 0))
    (i64.store (i32.add (local.get $out_ptr) (i32.const 16)) (i64.const 0))
    (i64.store (i32.add (local.get $out_ptr) (i32.const 24)) (i64.const 0))
    (i64.store (i32.add (local.get $out_ptr) (i32.const 32)) (i64.const 0))
    (i64.store (i32.add (local.get $out_ptr) (i32.const 40)) (i64.const 0)))

  (func $m102write_prefix_record
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

  (func $m102copy_name_meta (param $out_ptr i32) (param $base_offset i32) (param $meta_ptr i32)
    (i32.store
      (i32.add (local.get $out_ptr) (i32.const 24))
      (i32.add (local.get $base_offset) (i32.load (i32.add (local.get $meta_ptr) (i32.const 12)))))
    (i32.store
      (i32.add (local.get $out_ptr) (i32.const 28))
      (i32.load (i32.add (local.get $meta_ptr) (i32.const 16))))
    (i32.store
      (i32.add (local.get $out_ptr) (i32.const 32))
      (i32.load (i32.add (local.get $meta_ptr) (i32.const 4)))))

  (func $m102copy_value_meta (param $out_ptr i32) (param $base_offset i32) (param $meta_ptr i32)
    (i32.store
      (i32.add (local.get $out_ptr) (i32.const 36))
      (i32.add (local.get $base_offset) (i32.load (i32.add (local.get $meta_ptr) (i32.const 12)))))
    (i32.store
      (i32.add (local.get $out_ptr) (i32.const 40))
      (i32.load (i32.add (local.get $meta_ptr) (i32.const 16))))
    (i32.store
      (i32.add (local.get $out_ptr) (i32.const 44))
      (i32.load (i32.add (local.get $meta_ptr) (i32.const 4)))))

  (func $decode_literal
    (param $in_ptr i32) (param $in_len i32) (param $out_ptr i32)
    (param $kind i32) (param $prefix_bits i32)
    (result i32)
    (local $status i32)
    (local $prefix_consumed i32)
    (local $name_index i64)
    (local $name_total i32)
    (local $value_base i32)
    (local $value_total i32)
    (local.set $status
      (call $m102hpack_prefix_decode
        (local.get $in_ptr) (local.get $in_len) (local.get $prefix_bits) (i32.const 65472)))
    (if (local.get $status) (then (return (local.get $status))))
    (call $m102write_prefix_record (local.get $out_ptr) (local.get $kind) (i32.const 0) (i32.const 65472))
    (local.set $prefix_consumed (i32.load (i32.add (i32.const 65472) (i32.const 4))))
    (local.set $name_index (i64.load (i32.add (i32.const 65472) (i32.const 8))))
    (if (i64.eqz (local.get $name_index))
      (then
        (i32.store (local.get $out_ptr) (i32.const 3))
        (if (i32.eq (local.get $kind) (i32.const 4))
          (then (i32.store (local.get $out_ptr) (i32.const 4))))
        (local.set $status
          (call $hpack_string_scan
            (i32.add (local.get $in_ptr) (local.get $prefix_consumed))
            (i32.sub (local.get $in_len) (local.get $prefix_consumed))
            (i32.const 65472)))
        (if (local.get $status) (then (return (local.get $status))))
        (local.set $name_total
          (i32.add
            (i32.load (i32.add (i32.const 65472) (i32.const 8)))
            (i32.load (i32.add (i32.const 65472) (i32.const 16)))))
        (local.set $value_base (i32.add (local.get $prefix_consumed) (local.get $name_total)))
        (call $m102copy_name_meta (local.get $out_ptr) (local.get $prefix_consumed) (i32.const 65472)))
      (else
        (local.set $value_base (local.get $prefix_consumed))))
    (local.set $status
      (call $hpack_string_scan
        (i32.add (local.get $in_ptr) (local.get $value_base))
        (i32.sub (local.get $in_len) (local.get $value_base))
        (i32.const 65504)))
    (if (local.get $status) (then (return (local.get $status))))
    (local.set $value_total
      (i32.add
        (i32.load (i32.add (i32.const 65504) (i32.const 8)))
        (i32.load (i32.add (i32.const 65504) (i32.const 16)))))
    (i32.store
      (i32.add (local.get $out_ptr) (i32.const 8))
      (i32.add (local.get $value_base) (local.get $value_total)))
    (call $m102copy_value_meta (local.get $out_ptr) (local.get $value_base) (i32.const 65504))
    (i32.const 0))

  (func $hpack_header_instruction_decode (export "hpack_header_instruction_decode")
    (param $in_ptr i32) (param $in_len i32) (param $out_ptr i32)
    (result i32)
    (local $first i32)
    (local $status i32)
    (if (i32.eqz (local.get $in_len))
      (then (return (i32.const 1))))
    (call $m102clear_record (local.get $out_ptr))
    (local.set $first (i32.load8_u (local.get $in_ptr)))
    (if (i32.ne (i32.and (local.get $first) (i32.const 128)) (i32.const 0))
      (then
        (local.set $status
          (call $m102hpack_prefix_decode
            (local.get $in_ptr) (local.get $in_len) (i32.const 7) (i32.const 65472)))
        (if (local.get $status) (then (return (local.get $status))))
        (if (i64.eqz (i64.load (i32.add (i32.const 65472) (i32.const 8))))
          (then (return (i32.const 3))))
        (call $m102write_prefix_record (local.get $out_ptr) (i32.const 1) (i32.const 0) (i32.const 65472))
        (return (i32.const 0))))
    (if (i32.ne (i32.and (local.get $first) (i32.const 64)) (i32.const 0))
      (then
        (return
          (call $decode_literal
            (local.get $in_ptr) (local.get $in_len) (local.get $out_ptr)
            (i32.const 2) (i32.const 6)))))
    (if (i32.eq (i32.and (local.get $first) (i32.const 224)) (i32.const 32))
      (then
        (local.set $status
          (call $m102hpack_prefix_decode
            (local.get $in_ptr) (local.get $in_len) (i32.const 5) (i32.const 65472)))
        (if (local.get $status) (then (return (local.get $status))))
        (call $m102write_prefix_record (local.get $out_ptr) (i32.const 5) (i32.const 0) (i32.const 65472))
        (return (i32.const 0))))
    (if (i32.eq (i32.and (local.get $first) (i32.const 240)) (i32.const 16))
      (then
        (return
          (call $decode_literal
            (local.get $in_ptr) (local.get $in_len) (local.get $out_ptr)
            (i32.const 4) (i32.const 4)))))
    (i32.const 3))

  (func (export "hpack_header_block_scan")
    (param $in_ptr i32) (param $in_len i32) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (local $pos i32)
    (local $count i32)
    (local $status i32)
    (local $record_ptr i32)
    (loop $again
      (if (i32.ge_u (local.get $pos) (local.get $in_len))
        (then (return (call $pack (i32.const 0) (local.get $count)))))
      (if (i32.gt_u (i32.add (i32.mul (local.get $count) (i32.const 48)) (i32.const 48)) (local.get $out_cap))
        (then (return (call $pack (i32.const 2) (local.get $count)))))
      (local.set $record_ptr
        (i32.add (local.get $out_ptr) (i32.mul (local.get $count) (i32.const 48))))
      (local.set $status
        (call $hpack_header_instruction_decode
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
