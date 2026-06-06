  (import "hpack-qpack" "qpack_prefix_decode" (func $qpack_prefix_decode (param $in_ptr i32) (param $in_len i32) (param $prefix_bits i32) (param $out_ptr i32) (result i32)))
  (import "hpack-qpack" "prefix_encode" (func $prefix_encode (param $value i64) (param $prefix_bits i32) (param $prefix_high_bits i32) (param $out_ptr i32) (param $out_cap i32) (result i64)))
  (import "hpack-qpack" "table_entry_size" (func $table_entry_size (param $name_len i32) (param $value_len i32) (result i64)))

  (func $qpack_table_entry_size (export "qpack_table_entry_size") (param $name_len i32) (param $value_len i32) (result i64)
    (call $table_entry_size (local.get $name_len) (local.get $value_len)))

  ;; Prefix output record, little-endian:
  ;; 0:u32 flags_high_bits, 4:u32 consumed, 8:u64 value.
  (func (export "qpack_prefix_int_decode")
    (param $in_ptr i32) (param $in_len i32) (param $prefix_bits i32) (param $out_ptr i32)
    (result i32)
    (call qpack_prefix_decode
      (local.get $in_ptr) (local.get $in_len) (local.get $prefix_bits) (local.get $out_ptr)))

  (func (export "qpack_prefix_int_encode")
    (param $value i64) (param $prefix_bits i32) (param $prefix_high_bits i32)
    (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (call prefix_encode
      (local.get $value) (local.get $prefix_bits) (local.get $prefix_high_bits)
      (local.get $out_ptr) (local.get $out_cap)))

  ;; String meta record, little-endian:
  ;; 0:u32 flags, 4:u32 huffman, 8:u32 consumed, 12:u32 payload_offset, 16:u32 payload_len.
  (func (export "qpack_string_scan")
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
    (if (i64.gt_u (local.get $payload_len) (i64.const 4294967295))
      (then (return (i32.const 4))))
    (i32.store (local.get $meta_ptr) (i32.load (local.get $meta_ptr)))
    (i32.store
      (i32.add (local.get $meta_ptr) (i32.const 4))
      (i32.and (i32.load (local.get $meta_ptr)) (i32.const 1)))
    (i32.store (i32.add (local.get $meta_ptr) (i32.const 8)) (local.get $consumed))
    (i32.store (i32.add (local.get $meta_ptr) (i32.const 12)) (local.get $consumed))
    (i32.store (i32.add (local.get $meta_ptr) (i32.const 16)) (i32.wrap_i64 (local.get $payload_len)))
    (i32.const 0))

  ;; QPACK dynamic table entry size is name_len + value_len + 32.
  ;; Returns packed low32=status, high32=size.
  ;; Insert-plan output record, little-endian:
  ;; 0:u32 new_entry_size
  ;; 4:u32 retained_size
  ;; 8:u32 evict_existing_count
  ;; 12:u32 inserted
  ;;
  ;; oldest_sizes_ptr and tracked_ptr point to u32 arrays in FIFO eviction order.
  ;; Tracked entries stop eviction; an oversize entry is a decoder-side error.
  (func $qpack_entry_insert_plan
    (param $max_size i32)
    (param $current_size i32)
    (param $oldest_sizes_ptr i32)
    (param $tracked_ptr i32)
    (param $oldest_count i32)
    (param $entry_size i32)
    (param $out_ptr i32)
    (result i32)
    (local $retained_size i32)
    (local $evict_count i32)
    (local $old_size i32)
    (if (i32.gt_u (local.get $max_size) (i32.const 1073741823))
      (then (return (i32.const 5))))
    (i32.store (local.get $out_ptr) (local.get $entry_size))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 4)) (i32.const 0))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 8)) (i32.const 0))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 12)) (i32.const 0))
    (if (i32.gt_u (local.get $entry_size) (local.get $max_size))
      (then (return (i32.const 6))))
    (local.set $retained_size (i32.add (local.get $current_size) (local.get $entry_size)))
    (if (i32.lt_u (local.get $retained_size) (local.get $current_size))
      (then (return (i32.const 4))))
    (block $done
      (loop $again
        (if (i32.le_u (local.get $retained_size) (local.get $max_size))
          (then (br $done)))
        (if (i32.ge_u (local.get $evict_count) (local.get $oldest_count))
          (then (return (i32.const 7))))
        (if
          (i32.ne
            (i32.load
              (i32.add
                (local.get $tracked_ptr)
                (i32.mul (local.get $evict_count) (i32.const 4))))
            (i32.const 0))
          (then (return (i32.const 7))))
        (local.set $old_size
          (i32.load
            (i32.add
              (local.get $oldest_sizes_ptr)
              (i32.mul (local.get $evict_count) (i32.const 4)))))
        (if (i32.gt_u (local.get $old_size) (local.get $retained_size))
          (then (return (i32.const 7))))
        (local.set $retained_size (i32.sub (local.get $retained_size) (local.get $old_size)))
        (local.set $evict_count (i32.add (local.get $evict_count) (i32.const 1)))
        (br $again)))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 4)) (local.get $retained_size))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 8)) (local.get $evict_count))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 12)) (i32.const 1))
    (i32.const 0))

  (func (export "qpack_table_insert_plan")
    (param $max_size i32)
    (param $current_size i32)
    (param $oldest_sizes_ptr i32)
    (param $tracked_ptr i32)
    (param $oldest_count i32)
    (param $name_len i32)
    (param $value_len i32)
    (param $out_ptr i32)
    (result i32)
    (local $packed i64)
    (local.set $packed
      (call $qpack_table_entry_size (local.get $name_len) (local.get $value_len)))
    (if (i32.ne (i32.wrap_i64 (local.get $packed)) (i32.const 0))
      (then (return (i32.wrap_i64 (local.get $packed)))))
    (call $qpack_entry_insert_plan
      (local.get $max_size)
      (local.get $current_size)
      (local.get $oldest_sizes_ptr)
      (local.get $tracked_ptr)
      (local.get $oldest_count)
      (i32.wrap_i64 (i64.shr_u (local.get $packed) (i64.const 32)))
      (local.get $out_ptr)))

  (func (export "qpack_table_duplicate_plan")
    (param $max_size i32)
    (param $current_size i32)
    (param $oldest_sizes_ptr i32)
    (param $tracked_ptr i32)
    (param $oldest_count i32)
    (param $duplicate_entry_size i32)
    (param $out_ptr i32)
    (result i32)
    (call $qpack_entry_insert_plan
      (local.get $max_size)
      (local.get $current_size)
      (local.get $oldest_sizes_ptr)
      (local.get $tracked_ptr)
      (local.get $oldest_count)
      (local.get $duplicate_entry_size)
      (local.get $out_ptr)))

  ;; Resize-plan output record, little-endian:
  ;; 0:u32 retained_size, 4:u32 evict_existing_count.
  (func (export "qpack_table_resize_plan")
    (param $new_max_size i32)
    (param $current_size i32)
    (param $oldest_sizes_ptr i32)
    (param $tracked_ptr i32)
    (param $oldest_count i32)
    (param $out_ptr i32)
    (result i32)
    (local $retained_size i32)
    (local $evict_count i32)
    (local $old_size i32)
    (if (i32.gt_u (local.get $new_max_size) (i32.const 1073741823))
      (then (return (i32.const 5))))
    (local.set $retained_size (local.get $current_size))
    (block $done
      (loop $again
        (if (i32.le_u (local.get $retained_size) (local.get $new_max_size))
          (then (br $done)))
        (if (i32.ge_u (local.get $evict_count) (local.get $oldest_count))
          (then (return (i32.const 7))))
        (if
          (i32.ne
            (i32.load
              (i32.add
                (local.get $tracked_ptr)
                (i32.mul (local.get $evict_count) (i32.const 4))))
            (i32.const 0))
          (then (return (i32.const 7))))
        (local.set $old_size
          (i32.load
            (i32.add
              (local.get $oldest_sizes_ptr)
              (i32.mul (local.get $evict_count) (i32.const 4)))))
        (if (i32.gt_u (local.get $old_size) (local.get $retained_size))
          (then (return (i32.const 7))))
        (local.set $retained_size (i32.sub (local.get $retained_size) (local.get $old_size)))
        (local.set $evict_count (i32.add (local.get $evict_count) (i32.const 1)))
        (br $again)))
    (i32.store (local.get $out_ptr) (local.get $retained_size))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 4)) (local.get $evict_count))
    (i32.const 0))

  (func $qpack_blocked_max_validate (export "qpack_blocked_max_validate")
    (param $max_blocked i32)
    (result i32)
    (if (i32.ge_u (local.get $max_blocked) (i32.const 65535))
      (then (return (i32.const 8))))
    (i32.const 0))

  ;; Returns packed low32=status, high32=new_blocked_count.
  (func (export "qpack_blocked_register")
    (param $blocked_count i32) (param $blocked_max i32)
    (param $largest_ref i32) (param $largest_known_received i32)
    (result i64)
    (if (call $qpack_blocked_max_validate (local.get $blocked_max))
      (then (return (call $pack (i32.const 8) (local.get $blocked_count)))))
    (if (i32.le_u (local.get $largest_ref) (local.get $largest_known_received))
      (then (return (call $pack (i32.const 0) (local.get $blocked_count)))))
    (if (i32.ge_u (local.get $blocked_count) (local.get $blocked_max))
      (then (return (call $pack (i32.const 9) (local.get $blocked_count)))))
    (call $pack (i32.const 0) (i32.add (local.get $blocked_count) (i32.const 1))))

  ;; Ack-plan output record, little-endian:
  ;; 0:u32 new_largest_known_received, 4:u32 acked_count, 8:u32 remaining_count.
  ;; largest_refs_ptr and counts_ptr point to parallel u32 arrays.
  (func (export "qpack_blocked_ack_plan")
    (param $largest_known_received i32)
    (param $increment i32)
    (param $largest_refs_ptr i32)
    (param $counts_ptr i32)
    (param $pair_count i32)
    (param $out_ptr i32)
    (result i32)
    (local $new_known i32)
    (local $i i32)
    (local $largest i32)
    (local $count i32)
    (local $acked i32)
    (local $remaining i32)
    (local.set $new_known (i32.add (local.get $largest_known_received) (local.get $increment)))
    (if (i32.lt_u (local.get $new_known) (local.get $largest_known_received))
      (then (return (i32.const 4))))
    (loop $again
      (if (i32.ge_u (local.get $i) (local.get $pair_count))
        (then
          (i32.store (local.get $out_ptr) (local.get $new_known))
          (i32.store (i32.add (local.get $out_ptr) (i32.const 4)) (local.get $acked))
          (i32.store (i32.add (local.get $out_ptr) (i32.const 8)) (local.get $remaining))
          (return (i32.const 0))))
      (local.set $largest
        (i32.load
          (i32.add
            (local.get $largest_refs_ptr)
            (i32.mul (local.get $i) (i32.const 4)))))
      (local.set $count
        (i32.load
          (i32.add
            (local.get $counts_ptr)
            (i32.mul (local.get $i) (i32.const 4)))))
      (if (i32.le_u (local.get $largest) (local.get $new_known))
        (then (local.set $acked (i32.add (local.get $acked) (local.get $count))))
        (else (local.set $remaining (i32.add (local.get $remaining) (local.get $count)))))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br $again))
    (i32.const 0))

  (func (export "qpack_encoder_instruction_classify")
    (param $first i32)
    (result i32)
    (if (i32.ne (i32.and (local.get $first) (i32.const 128)) (i32.const 0))
      (then (return (i32.const 2))))
    (if (i32.ne (i32.and (local.get $first) (i32.const 64)) (i32.const 0))
      (then (return (i32.const 3))))
    (if (i32.eqz (i32.and (local.get $first) (i32.const 224)))
      (then (return (i32.const 4))))
    (if (i32.ne (i32.and (local.get $first) (i32.const 32)) (i32.const 0))
      (then (return (i32.const 1))))
    (i32.const 0))

  (func (export "qpack_decoder_instruction_classify")
    (param $first i32)
    (result i32)
    (if (i32.eqz (i32.and (local.get $first) (i32.const 192)))
      (then (return (i32.const 3))))
    (if (i32.ne (i32.and (local.get $first) (i32.const 128)) (i32.const 0))
      (then (return (i32.const 1))))
    (if (i32.ne (i32.and (local.get $first) (i32.const 64)) (i32.const 0))
      (then (return (i32.const 2))))
    (i32.const 0))
