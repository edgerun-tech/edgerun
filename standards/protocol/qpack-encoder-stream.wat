  (import "hpack-qpack" "qpack_prefix_decode" (func $qpack_prefix_decode (param $in_ptr i32) (param $in_len i32) (param $prefix_bits i32) (param $out_ptr i32) (result i32)))
  (import "hpack-qpack" "clear_record" (func $m153clear_record (param $out_ptr i32)))
  (import "hpack-qpack" "write_prefix_record" (func $m153write_prefix_record (param $out_ptr i32) (param $kind i32) (param $offset i32) (param $prefix_ptr i32)))
  (import "hpack-qpack" "copy_name_meta" (func $m153copy_name_meta (param $out_ptr i32) (param $base_offset i32) (param $meta_ptr i32)))
  (import "hpack-qpack" "copy_value_meta" (func $m153copy_value_meta (param $out_ptr i32) (param $base_offset i32) (param $meta_ptr i32)))
;; Prefix output record, little-endian:
  ;; 0:u32 flags_high_bits, 4:u32 consumed, 8:u64 value.
  ;; String meta record, little-endian:
  ;; 0:u32 flags, 4:u32 huffman, 8:u32 consumed, 12:u32 payload_offset, 16:u32 payload_len.
  (func $qpack_string_scan
    (param $size i32) (param $in_ptr i32) (param $in_len i32) (param $meta_ptr i32)
    (result i32)
    (local $prefix_bits i32)
    (local $status i32)
    (local $consumed i32)
    (local $payload_len i64)
    (if
      (i32.or
        (i32.lt_u (local.get $size) (i32.const 2))
        (i32.gt_u (local.get $size) (i32.const 8)))
      (then (return (i32.const 3))))
    (local.set $prefix_bits (i32.sub (local.get $size) (i32.const 1)))
    (local.set $status
      (call qpack_prefix_decode
        (local.get $in_ptr) (local.get $in_len) (local.get $prefix_bits) (local.get $meta_ptr)))
    (if (local.get $status) (then (return (local.get $status))))
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
    (i32.store (i32.add (local.get $meta_ptr) (i32.const 4)) (i32.load (local.get $meta_ptr)))
    (i32.store (local.get $meta_ptr) (i32.load (i32.add (local.get $meta_ptr) (i32.const 4))))
    (i32.store (i32.add (local.get $meta_ptr) (i32.const 4)) (i32.and (i32.load (local.get $meta_ptr)) (i32.const 1)))
    (i32.store (i32.add (local.get $meta_ptr) (i32.const 8)) (local.get $consumed))
    (i32.store (i32.add (local.get $meta_ptr) (i32.const 12)) (local.get $consumed))
    (i32.store (i32.add (local.get $meta_ptr) (i32.const 16)) (i32.wrap_i64 (local.get $payload_len)))
    (i32.const 0))

  ;; Encoder instruction record, little-endian:
  ;; 0:u32 kind (1 dynamic_table_size_update, 2 insert_with_name_ref,
  ;;             3 insert_without_name_ref, 4 duplicate)
  ;; 4:u32 offset
  ;; 8:u32 consumed
  ;; 12:u32 flags_high_bits
  ;; 16:u64 value (capacity, name index, or duplicate index)
  ;; 24:u32 name_payload_offset
  ;; 28:u32 name_payload_len
  ;; 32:u32 name_huffman
  ;; 36:u32 value_payload_offset
  ;; 40:u32 value_payload_len
  ;; 44:u32 value_huffman
  (func $qpack_encoder_instruction_decode (export "qpack_encoder_instruction_decode")
    (param $in_ptr i32) (param $in_len i32) (param $out_ptr i32)
    (result i32)
    (local $first i32)
    (local $kind i32)
    (local $prefix_bits i32)
    (local $status i32)
    (local $flags i32)
    (local $consumed i32)
    (local $name_total i32)
    (local $value_base i32)
    (local $value_total i32)
    (if (i32.eqz (local.get $in_len))
      (then (return (i32.const 1))))
    (call $m153clear_record (local.get $out_ptr))
    (local.set $first (i32.load8_u (local.get $in_ptr)))
    (if
      (i32.ne (i32.and (local.get $first) (i32.const 128)) (i32.const 0))
      (then
        (local.set $kind (i32.const 2))
        (local.set $prefix_bits (i32.const 6)))
      (else
        (if
          (i32.ne (i32.and (local.get $first) (i32.const 64)) (i32.const 0))
          (then
            (local.set $kind (i32.const 3)))
          (else
            (if
              (i32.eqz (i32.and (local.get $first) (i32.const 224)))
              (then
                (local.set $kind (i32.const 4))
                (local.set $prefix_bits (i32.const 5)))
              (else
                (local.set $kind (i32.const 1))
                (local.set $prefix_bits (i32.const 5))))))))

    (if (i32.eq (local.get $kind) (i32.const 3))
      (then
        (local.set $status
          (call $qpack_string_scan (i32.const 6) (local.get $in_ptr) (local.get $in_len) (i32.const 65472)))
        (if (local.get $status) (then (return (local.get $status))))
        (local.set $flags (i32.load (i32.const 65472)))
        (if
          (i32.and
            (i32.ne (local.get $flags) (i32.const 2))
            (i32.ne (local.get $flags) (i32.const 3)))
          (then (return (i32.const 3))))
        (local.set $name_total
          (i32.add
            (i32.load (i32.add (i32.const 65472) (i32.const 8)))
            (i32.load (i32.add (i32.const 65472) (i32.const 16)))))
        (if (i32.gt_u (local.get $name_total) (local.get $in_len))
          (then (return (i32.const 1))))
        (local.set $value_base (local.get $name_total))
        (local.set $status
          (call $qpack_string_scan
            (i32.const 8)
            (i32.add (local.get $in_ptr) (local.get $value_base))
            (i32.sub (local.get $in_len) (local.get $value_base))
            (i32.const 65504)))
        (if (local.get $status) (then (return (local.get $status))))
        (local.set $value_total
          (i32.add
            (i32.load (i32.add (i32.const 65504) (i32.const 8)))
            (i32.load (i32.add (i32.const 65504) (i32.const 16)))))
        (i32.store (local.get $out_ptr) (local.get $kind))
        (i32.store (i32.add (local.get $out_ptr) (i32.const 8)) (i32.add (local.get $name_total) (local.get $value_total)))
        (i32.store (i32.add (local.get $out_ptr) (i32.const 12)) (local.get $flags))
        (call $m153copy_name_meta (local.get $out_ptr) (i32.const 0) (i32.const 65472))
        (call $m153copy_value_meta (local.get $out_ptr) (local.get $value_base) (i32.const 65504))
        (return (i32.const 0))))

    (local.set $status
      (call qpack_prefix_decode
        (local.get $in_ptr) (local.get $in_len) (local.get $prefix_bits) (i32.const 65472)))
    (if (local.get $status) (then (return (local.get $status))))
    (local.set $flags (i32.load (i32.const 65472)))
    (if
      (i32.and
        (i32.eq (local.get $kind) (i32.const 2))
        (i32.and
          (i32.ne (local.get $flags) (i32.const 2))
          (i32.ne (local.get $flags) (i32.const 3))))
      (then (return (i32.const 3))))
    (if
      (i32.and
        (i32.eq (local.get $kind) (i32.const 4))
        (i32.ne (local.get $flags) (i32.const 0)))
      (then (return (i32.const 3))))
    (if
      (i32.and
        (i32.eq (local.get $kind) (i32.const 1))
        (i32.ne (local.get $flags) (i32.const 1)))
      (then (return (i32.const 3))))
    (call $m153write_prefix_record (local.get $out_ptr) (local.get $kind) (i32.const 0) (i32.const 65472))
    (if (i32.eq (local.get $kind) (i32.const 2))
      (then
        (local.set $consumed (i32.load (i32.add (i32.const 65472) (i32.const 4))))
        (local.set $status
          (call $qpack_string_scan
            (i32.const 8)
            (i32.add (local.get $in_ptr) (local.get $consumed))
            (i32.sub (local.get $in_len) (local.get $consumed))
            (i32.const 65504)))
        (if (local.get $status) (then (return (local.get $status))))
        (local.set $value_total
          (i32.add
            (i32.load (i32.add (i32.const 65504) (i32.const 8)))
            (i32.load (i32.add (i32.const 65504) (i32.const 16)))))
        (i32.store (i32.add (local.get $out_ptr) (i32.const 8)) (i32.add (local.get $consumed) (local.get $value_total)))
        (call $m153copy_value_meta (local.get $out_ptr) (local.get $consumed) (i32.const 65504))))
    (i32.const 0))

  (func (export "qpack_encoder_stream_scan")
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
        (call $qpack_encoder_instruction_decode
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
