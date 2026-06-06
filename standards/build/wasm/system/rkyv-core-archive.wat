  (func (export "proto_abi_version") (result i32) i32.const 2)
  (func (export "proto_standard_id") (result i32) i32.const 300134)

  (func $is_power_of_two (param $value i64) (result i32)
    (i32.and
      (i64.gt_u (local.get $value) (i64.const 0))
      (i64.eqz (i64.and (local.get $value) (i64.sub (local.get $value) (i64.const 1))))))

  (func $align_up (param $value i64) (param $align i64) (result i64)
    (i64.and
      (i64.add (local.get $value) (i64.sub (local.get $align) (i64.const 1)))
      (i64.xor (i64.sub (local.get $align) (i64.const 1)) (i64.const -1))))

  (func (export "rkyv_endian_layout_code") (param $big_endian i32) (param $unaligned i32) (result i32)
    (if (i32.and (i32.eqz (local.get $big_endian)) (i32.eqz (local.get $unaligned))) (then (return (i32.const 1))))
    (if (i32.and (i32.eqz (local.get $big_endian)) (local.get $unaligned)) (then (return (i32.const 2))))
    (if (i32.and (local.get $big_endian) (i32.eqz (local.get $unaligned))) (then (return (i32.const 3))))
    i32.const 4)

  (func $fixed_pointer_bytes (export "rkyv_fixed_pointer_bytes") (param $pointer_width_bits i32) (result i32)
    (if (i32.eq (local.get $pointer_width_bits) (i32.const 16)) (then (return (i32.const 2))))
    (if (i32.eq (local.get $pointer_width_bits) (i32.const 64)) (then (return (i32.const 8))))
    i32.const 4)

  (func (export "rkyv_copy_optimization_allowed") (param $portable i32) (param $no_undef i32) (param $no_padding i32) (result i32)
    (i32.and (local.get $portable) (i32.and (local.get $no_undef) (local.get $no_padding))))

  (func (export "rkyv_signed_offset_status") (param $from i64) (param $to i64) (param $max_isize i64) (result i32)
    (local $diff i64)
    (local.set $diff (i64.sub (local.get $to) (local.get $from)))
    (if (i64.gt_s (local.get $diff) (local.get $max_isize)) (then (return (i32.const 2))))
    (if (i64.lt_s (local.get $diff) (i64.sub (i64.const 0) (i64.add (local.get $max_isize) (i64.const 1)))) (then (return (i32.const 2))))
    i32.const 1)

  (func (export "rkyv_signed_offset_value") (param $from i64) (param $to i64) (result i64)
    (i64.sub (local.get $to) (local.get $from)))

  (func (export "rkyv_offset_storage_fits") (param $offset i64) (param $bits i32) (param $signed i32) (result i32)
    (local $limit i64)
    (if (i32.eqz (local.get $signed))
      (then
        (if (i64.lt_s (local.get $offset) (i64.const 0)) (then (return (i32.const 0))))
        (local.set $limit (i64.sub (i64.shl (i64.const 1) (i64.extend_i32_u (local.get $bits))) (i64.const 1)))
        (return (i64.le_u (local.get $offset) (local.get $limit)))))
    (local.set $limit (i64.sub (i64.shl (i64.const 1) (i64.extend_i32_u (i32.sub (local.get $bits) (i32.const 1)))) (i64.const 1)))
    (i32.and
      (i64.le_s (local.get $offset) (local.get $limit))
      (i64.ge_s (local.get $offset) (i64.sub (i64.const 0) (i64.add (local.get $limit) (i64.const 1))))))

  (func (export "rkyv_rel_ptr_target") (param $base i64) (param $offset i64) (result i64)
    (i64.add (local.get $base) (local.get $offset)))

  (func (export "rkyv_rel_ptr_invalid") (param $offset i64) (result i32)
    (i64.eq (local.get $offset) (i64.const 1)))

  (func (export "rkyv_archive_ptr_check") (param $ptr i64) (param $size i64) (param $align i64) (param $range_start i64) (param $range_end i64) (result i32)
    (local $end i64)
    (local.set $end (i64.add (local.get $ptr) (local.get $size)))
    (if (i64.lt_u (local.get $end) (local.get $ptr)) (then (return (i32.const 2))))
    (if (i64.lt_u (local.get $ptr) (local.get $range_start)) (then (return (i32.const 2))))
    (if (i64.gt_u (local.get $end) (local.get $range_end)) (then (return (i32.const 2))))
    (if (i64.ne (i64.and (local.get $ptr) (i64.sub (local.get $align) (i64.const 1))) (i64.const 0)) (then (return (i32.const 3))))
    i32.const 1)

  (func (export "rkyv_subtree_push_status") (param $remaining_depth i64) (result i32)
    (if (i64.eqz (local.get $remaining_depth)) (then (return (i32.const 4))))
    i32.const 1)

  (func (export "rkyv_subtree_pop_status") (param $saved_start i64) (param $current_end i64) (param $depth_before_pop i64) (result i32)
    (if (i64.lt_u (local.get $saved_start) (local.get $current_end)) (then (return (i32.const 5))))
    (if (i64.eq (local.get $depth_before_pop) (i64.const 9223372036854775807)) (then (return (i32.const 6))))
    i32.const 1)

  (func (export "rkyv_aligned_vec_max_capacity") (param $alignment i64) (param $max_isize i64) (result i64)
    (if (i32.eqz (call $is_power_of_two (local.get $alignment))) (then (return (i64.const -1))))
    (if (i64.ge_u (local.get $alignment) (local.get $max_isize)) (then (return (i64.const -1))))
    (i64.sub (local.get $max_isize) (i64.sub (local.get $alignment) (i64.const 1))))

  (func (export "rkyv_buffer_write_status") (param $cap i64) (param $len i64) (param $write_len i64) (result i32)
    (if (i64.gt_u (local.get $write_len) (i64.sub (local.get $cap) (local.get $len))) (then (return (i32.const 7))))
    i32.const 1)

  (func (export "rkyv_suballocator_alloc_offset") (param $used i64) (param $size i64) (param $request_size i64) (param $align i64) (result i64)
    (local $start i64)
    (local $end i64)
    (if (i32.eqz (call $is_power_of_two (local.get $align))) (then (return (i64.const -1))))
    (local.set $start (call $align_up (local.get $used) (local.get $align)))
    (local.set $end (i64.add (local.get $start) (local.get $request_size)))
    (if (i64.lt_u (local.get $end) (local.get $start)) (then (return (i64.const -1))))
    (if (i64.gt_u (local.get $end) (local.get $size)) (then (return (i64.const -1))))
    (local.get $start))

  (func $string_inline_capacity (export "rkyv_string_inline_capacity") (param $pointer_width_bits i32) (result i32)
    (i32.mul (call $fixed_pointer_bytes (local.get $pointer_width_bits)) (i32.const 2)))

  (func (export "rkyv_string_repr_kind") (param $len i64) (param $pointer_width_bits i32) (result i32)
    (if (i64.le_u (local.get $len) (i64.extend_i32_u (call $string_inline_capacity (local.get $pointer_width_bits)))) (then (return (i32.const 1))))
    i32.const 2)

  (func (export "rkyv_string_out_of_line_capacity") (param $fixed_usize_bits i32) (result i64)
    (i64.sub (i64.shl (i64.const 1) (i64.extend_i32_u (i32.sub (local.get $fixed_usize_bits) (i32.const 2)))) (i64.const 1)))

  (func (export "rkyv_option_tag_code") (param $is_some i32) (result i32)
    (if (local.get $is_some) (then (return (i32.const 1))))
    i32.const 0)

  (func (export "rkyv_niche_zero") (param $value i64) (result i32)
    (i64.eqz (local.get $value)))

  (func (export "rkyv_niche_null") (param $address i64) (result i32)
    (i64.eqz (local.get $address)))

  (func (export "rkyv_niche_nan_f64") (param $value f64) (result i32)
    (f64.ne (local.get $value) (local.get $value)))

  (func (export "rkyv_archive_order_code") (param $phase i32) (result i32)
    (if (i32.eq (local.get $phase) (i32.const 1)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $phase) (i32.const 2)) (then (return (i32.const 2))))
    (if (i32.eq (local.get $phase) (i32.const 3)) (then (return (i32.const 3))))
    i32.const 0)