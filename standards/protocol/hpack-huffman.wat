    (import "hpack-qpack" "huff_lookup" (func $huff_lookup (param $code i32) (param $bits i32) (result i32)))
  (import "hpack-qpack" "huffman_decode_internal" (func $huffman_decode_internal (param $in_ptr i32) (param $in_len i32) (param $out_ptr i32) (param $out_cap i32) (param $write i32) (result i64)))

(func (export "hpack_huffman_decode")
    (param $in_ptr i32) (param $in_len i32) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (call huffman_decode_internal
      (local.get $in_ptr) (local.get $in_len) (local.get $out_ptr) (local.get $out_cap) (i32.const 1)))

  (func (export "hpack_huffman_validate")
    (param $in_ptr i32) (param $in_len i32)
    (result i32)
    (i32.wrap_i64
      (call huffman_decode_internal
        (local.get $in_ptr) (local.get $in_len) (i32.const 0) (i32.const 2147483647) (i32.const 0))))

  (func (export "simd_capabilities") (result i32)
    i32.const 1)

  (func $m103huff_lookup_simd (param $code i32) (param $bits i32) (result i32)
    (local $i i32)
    (local $codes_vec v128)
    (local $target_vec v128)
    (local $eq_vec v128)
    (local.set $target_vec (i32x4.splat (local.get $code)))
    (block $done
      (loop $groups
        (if (i32.gt_u (local.get $i) (i32.const 252))
          (then (br $done)))
        (local.set $codes_vec
          (v128.load (i32.add (i32.const 60000) (i32.mul (local.get $i) (i32.const 4)))))
        (local.set $eq_vec (i32x4.eq (local.get $codes_vec) (local.get $target_vec)))
        (if (v128.any_true (local.get $eq_vec))
          (then
            (if (i32.and
                  (i32x4.extract_lane 0 (local.get $eq_vec))
                  (i32.eq
                    (i32.load8_u (i32.add (i32.const 61024) (local.get $i)))
                    (local.get $bits)))
              (then (return (local.get $i))))
            (if (i32.and
                  (i32x4.extract_lane 1 (local.get $eq_vec))
                  (i32.eq
                    (i32.load8_u (i32.add (i32.const 61024) (i32.add (local.get $i) (i32.const 1))))
                    (local.get $bits)))
              (then (return (i32.add (local.get $i) (i32.const 1)))))
            (if (i32.and
                  (i32x4.extract_lane 2 (local.get $eq_vec))
                  (i32.eq
                    (i32.load8_u (i32.add (i32.const 61024) (i32.add (local.get $i) (i32.const 2))))
                    (local.get $bits)))
              (then (return (i32.add (local.get $i) (i32.const 2)))))
            (if (i32.and
                  (i32x4.extract_lane 3 (local.get $eq_vec))
                  (i32.eq
                    (i32.load8_u (i32.add (i32.const 61024) (i32.add (local.get $i) (i32.const 3))))
                    (local.get $bits)))
              (then (return (i32.add (local.get $i) (i32.const 3)))))))
        (local.set $i (i32.add (local.get $i) (i32.const 4)))
        (br $groups))
      (if (i32.and
            (i32.eq
              (i32.load8_u (i32.add (i32.const 61024) (i32.const 256)))
              (local.get $bits))
            (i32.eq
              (i32.load (i32.add (i32.const 60000) (i32.mul (i32.const 256) (i32.const 4))))
              (local.get $code)))
        (then (return (i32.const 256)))))
    (i32.const -1))

  (func $m103huffman_decode_internal_simd
    (param $in_ptr i32) (param $in_len i32) (param $out_ptr i32) (param $out_cap i32) (param $write i32)
    (result i64)
    (local $pos i32)
    (local $bit i32)
    (local $byte i32)
    (local $current i32)
    (local $current_len i32)
    (local $sym i32)
    (local $written i32)
    (loop $bytes
      (if (i32.ge_u (local.get $pos) (local.get $in_len))
        (then
          (if (i32.gt_u (local.get $current_len) (i32.const 0))
            (then
              (if (i32.gt_u (local.get $current_len) (i32.const 7))
                (then (return (call $pack (i32.const 3) (local.get $written)))))
              (if
                (i32.ne
                  (local.get $current)
                  (i32.sub (i32.shl (i32.const 1) (local.get $current_len)) (i32.const 1)))
                (then (return (call $pack (i32.const 3) (local.get $written)))))))
          (return (call $pack (i32.const 0) (local.get $written)))))
      (local.set $byte (i32.load8_u (i32.add (local.get $in_ptr) (local.get $pos))))
      (local.set $bit (i32.const 7))
      (loop $bits
        (local.set $current
          (i32.or
            (i32.shl (local.get $current) (i32.const 1))
            (i32.and (i32.shr_u (local.get $byte) (local.get $bit)) (i32.const 1))))
        (local.set $current_len (i32.add (local.get $current_len) (i32.const 1)))
        (if (i32.le_u (local.get $current_len) (i32.const 30))
          (then
            (local.set $sym (call $m103huff_lookup_simd (local.get $current) (local.get $current_len)))
            (if (i32.ge_s (local.get $sym) (i32.const 0))
              (then
                (if (i32.eq (local.get $sym) (i32.const 256))
                  (then (return (call $pack (i32.const 3) (local.get $written)))))
                (if (local.get $write)
                  (then
                    (if (i32.ge_u (local.get $written) (local.get $out_cap))
                      (then (return (call $pack (i32.const 2) (local.get $written)))))
                    (i32.store8 (i32.add (local.get $out_ptr) (local.get $written)) (local.get $sym))))
                (local.set $written (i32.add (local.get $written) (i32.const 1)))
                (local.set $current (i32.const 0))
                (local.set $current_len (i32.const 0))))))
        (if (i32.eqz (local.get $bit))
          (then)
          (else
            (local.set $bit (i32.sub (local.get $bit) (i32.const 1)))
            (br $bits))))
      (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
      (br $bytes))
    (call $pack (i32.const 3) (local.get $written)))

  (func (export "hpack_huffman_decode_simd")
    (param $in_ptr i32) (param $in_len i32) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (call $m103huffman_decode_internal_simd
      (local.get $in_ptr) (local.get $in_len) (local.get $out_ptr) (local.get $out_cap) (i32.const 1)))

  (func (export "hpack_huffman_validate_simd")
    (param $in_ptr i32) (param $in_len i32)
    (result i32)
    (i32.wrap_i64
      (call $m103huffman_decode_internal_simd
        (local.get $in_ptr) (local.get $in_len) (i32.const 0) (i32.const 2147483647) (i32.const 0))))
