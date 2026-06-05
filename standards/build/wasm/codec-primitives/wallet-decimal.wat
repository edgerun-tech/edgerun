(module
  (memory (export "memory") 2)

  (global $TMP i32 (i32.const 65000))
  (global $TMP_END i32 (i32.const 65064))

  (func (export "proto_abi_version") (result i32)
    i32.const 2)

  (func (export "proto_standard_id") (result i32)
    i32.const 300086)

  ;; Status: 0 ok, 1 overflow, 2 output short, 3 invalid.
  ;; Amount out layout: u64 mantissa_lo, u64 mantissa_hi, u32 scale.

  (func $store_amount (param $out i32) (param $lo i64) (param $hi i64) (param $scale i32)
    (i64.store (local.get $out) (local.get $lo))
    (i64.store (i32.add (local.get $out) (i32.const 8)) (local.get $hi))
    (i32.store (i32.add (local.get $out) (i32.const 16)) (local.get $scale)))

  (func $mul10_add (param $lo i64) (param $hi i64) (param $digit i32) (param $out i32) (result i32)
    (local $mask i64)
    (local $p0 i64)
    (local $p1 i64)
    (local $new_lo i64)
    (local $carry i64)
    (local $q0 i64)
    (local $q1 i64)
    (local.set $mask (i64.const 4294967295))
    (local.set $p0
      (i64.add
        (i64.mul (i64.and (local.get $lo) (local.get $mask)) (i64.const 10))
        (i64.extend_i32_u (local.get $digit))))
    (local.set $p1
      (i64.add
        (i64.mul (i64.shr_u (local.get $lo) (i64.const 32)) (i64.const 10))
        (i64.shr_u (local.get $p0) (i64.const 32))))
    (local.set $new_lo
      (i64.or
        (i64.and (local.get $p0) (local.get $mask))
        (i64.shl (i64.and (local.get $p1) (local.get $mask)) (i64.const 32))))
    (local.set $carry (i64.shr_u (local.get $p1) (i64.const 32)))
    (local.set $q0
      (i64.add
        (i64.mul (i64.and (local.get $hi) (local.get $mask)) (i64.const 10))
        (local.get $carry)))
    (local.set $q1
      (i64.add
        (i64.mul (i64.shr_u (local.get $hi) (i64.const 32)) (i64.const 10))
        (i64.shr_u (local.get $q0) (i64.const 32))))
    (if (i64.ne (i64.shr_u (local.get $q1) (i64.const 32)) (i64.const 0))
      (then (return (i32.const 1))))
    (i64.store (local.get $out) (local.get $new_lo))
    (i64.store
      (i32.add (local.get $out) (i32.const 8))
      (i64.or
        (i64.and (local.get $q0) (local.get $mask))
        (i64.shl (i64.and (local.get $q1) (local.get $mask)) (i64.const 32))))
    i32.const 0)

  (func $scale_to (param $lo i64) (param $hi i64) (param $scale i32) (param $target i32) (param $out i32) (result i32)
    (local $status i32)
    (local $n i32)
    (if (i32.gt_u (local.get $target) (i32.const 38))
      (then (return (i32.const 3))))
    (if (i32.gt_u (local.get $scale) (i32.const 38))
      (then (return (i32.const 3))))
    (if (i32.eq (local.get $scale) (local.get $target))
      (then
        (call $store_amount (local.get $out) (local.get $lo) (local.get $hi) (local.get $target))
        (return (i32.const 0))))
    (if (i32.lt_u (local.get $scale) (local.get $target))
      (then
        (local.set $n (i32.sub (local.get $target) (local.get $scale)))
        (block $done
          (loop $loop
            (br_if $done (i32.eqz (local.get $n)))
            (local.set $status (call $mul10_add (local.get $lo) (local.get $hi) (i32.const 0) (local.get $out)))
            (if (local.get $status) (then (return (local.get $status))))
            (local.set $lo (i64.load (local.get $out)))
            (local.set $hi (i64.load (i32.add (local.get $out) (i32.const 8))))
            (local.set $n (i32.sub (local.get $n) (i32.const 1)))
            (br $loop)))
        (call $store_amount (local.get $out) (local.get $lo) (local.get $hi) (local.get $target))
        (return (i32.const 0))))
    ;; Downscale is truncating, matching the source display normalize behavior.
    (local.set $n (i32.sub (local.get $scale) (local.get $target)))
    (block $div_done
      (loop $div_loop
        (br_if $div_done (i32.eqz (local.get $n)))
        (call $div10_store (local.get $lo) (local.get $hi) (local.get $out))
        (local.set $lo (i64.load (local.get $out)))
        (local.set $hi (i64.load (i32.add (local.get $out) (i32.const 8))))
        (local.set $n (i32.sub (local.get $n) (i32.const 1)))
        (br $div_loop)))
    (call $store_amount (local.get $out) (local.get $lo) (local.get $hi) (local.get $target))
    i32.const 0)

  (func $div10_store (param $lo i64) (param $hi i64) (param $out i32)
    (local $qlo i64)
    (local $qhi i64)
    (local $rem i64)
    (local $bit i32)
    (local $bitval i64)
    (local.set $bit (i32.const 127))
    (block $done
      (loop $loop
        (if (i32.ge_u (local.get $bit) (i32.const 64))
          (then
            (local.set $bitval
              (i64.and
                (i64.shr_u (local.get $hi) (i64.extend_i32_u (i32.sub (local.get $bit) (i32.const 64))))
                (i64.const 1))))
          (else
            (local.set $bitval
              (i64.and
                (i64.shr_u (local.get $lo) (i64.extend_i32_u (local.get $bit)))
                (i64.const 1)))))
        (local.set $rem (i64.or (i64.shl (local.get $rem) (i64.const 1)) (local.get $bitval)))
        (if (i64.ge_u (local.get $rem) (i64.const 10))
          (then
            (local.set $rem (i64.sub (local.get $rem) (i64.const 10)))
            (if (i32.ge_u (local.get $bit) (i32.const 64))
              (then
                (local.set $qhi
                  (i64.or
                    (local.get $qhi)
                    (i64.shl (i64.const 1) (i64.extend_i32_u (i32.sub (local.get $bit) (i32.const 64)))))))
              (else
                (local.set $qlo
                  (i64.or
                    (local.get $qlo)
                    (i64.shl (i64.const 1) (i64.extend_i32_u (local.get $bit)))))))))
        (br_if $done (i32.eqz (local.get $bit)))
        (local.set $bit (i32.sub (local.get $bit) (i32.const 1)))
        (br $loop)))
    (i64.store (local.get $out) (local.get $qlo))
    (i64.store (i32.add (local.get $out) (i32.const 8)) (local.get $qhi)))

  (func (export "wallet_decimal_parse") (param $ptr i32) (param $len i32) (param $out i32) (result i32)
    (local $i i32)
    (local $c i32)
    (local $seen_dot i32)
    (local $seen_digit i32)
    (local $scale i32)
    (local $lo i64)
    (local $hi i64)
    (local $status i32)
    (if (i32.eqz (local.get $len))
      (then (return (i32.const 3))))
    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $i) (local.get $len)))
        (local.set $c (i32.load8_u (i32.add (local.get $ptr) (local.get $i))))
        (if (i32.eq (local.get $c) (i32.const 46))
          (then
            (if (local.get $seen_dot) (then (return (i32.const 3))))
            (local.set $seen_dot (i32.const 1)))
          (else
            (if (i32.or (i32.lt_u (local.get $c) (i32.const 48)) (i32.gt_u (local.get $c) (i32.const 57)))
              (then (return (i32.const 3))))
            (local.set $seen_digit (i32.const 1))
            (if (local.get $seen_dot)
              (then
                (local.set $scale (i32.add (local.get $scale) (i32.const 1)))
                (if (i32.gt_u (local.get $scale) (i32.const 38)) (then (return (i32.const 3))))))
            (local.set $status
              (call $mul10_add
                (local.get $lo)
                (local.get $hi)
                (i32.sub (local.get $c) (i32.const 48))
                (local.get $out)))
            (if (local.get $status) (then (return (local.get $status))))
            (local.set $lo (i64.load (local.get $out)))
            (local.set $hi (i64.load (i32.add (local.get $out) (i32.const 8))))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop)))
    (if (i32.eqz (local.get $seen_digit))
      (then (return (i32.const 3))))
    (call $store_amount (local.get $out) (local.get $lo) (local.get $hi) (local.get $scale))
    i32.const 0)

  (func (export "wallet_decimal_normalize")
    (param $lo i64) (param $hi i64) (param $scale i32) (param $target i32) (param $out i32) (result i32)
    (call $scale_to (local.get $lo) (local.get $hi) (local.get $scale) (local.get $target) (local.get $out)))

  (func (export "wallet_decimal_add")
    (param $a_lo i64) (param $a_hi i64) (param $a_scale i32)
    (param $b_lo i64) (param $b_hi i64) (param $b_scale i32)
    (param $out i32) (result i32)
    (local $scale i32)
    (local $status i32)
    (local $alo i64)
    (local $ahi i64)
    (local $blo i64)
    (local $bhi i64)
    (local $sumlo i64)
    (local $sumhi i64)
    (local.set $scale (select (local.get $a_scale) (local.get $b_scale) (i32.gt_u (local.get $a_scale) (local.get $b_scale))))
    (local.set $status (call $scale_to (local.get $a_lo) (local.get $a_hi) (local.get $a_scale) (local.get $scale) (local.get $out)))
    (if (local.get $status) (then (return (local.get $status))))
    (local.set $alo (i64.load (local.get $out)))
    (local.set $ahi (i64.load (i32.add (local.get $out) (i32.const 8))))
    (local.set $status (call $scale_to (local.get $b_lo) (local.get $b_hi) (local.get $b_scale) (local.get $scale) (local.get $out)))
    (if (local.get $status) (then (return (local.get $status))))
    (local.set $blo (i64.load (local.get $out)))
    (local.set $bhi (i64.load (i32.add (local.get $out) (i32.const 8))))
    (local.set $sumlo (i64.add (local.get $alo) (local.get $blo)))
    (local.set $sumhi
      (i64.add
        (i64.add (local.get $ahi) (local.get $bhi))
        (i64.extend_i32_u (i64.lt_u (local.get $sumlo) (local.get $alo)))))
    (if (i32.or
          (i64.lt_u (local.get $sumhi) (local.get $ahi))
          (i32.and (i64.eq (local.get $sumhi) (local.get $ahi)) (i64.gt_u (local.get $bhi) (i64.const 0))))
      (then (return (i32.const 1))))
    (call $store_amount (local.get $out) (local.get $sumlo) (local.get $sumhi) (local.get $scale))
    i32.const 0)

  (func (export "wallet_decimal_sub")
    (param $a_lo i64) (param $a_hi i64) (param $a_scale i32)
    (param $b_lo i64) (param $b_hi i64) (param $b_scale i32)
    (param $out i32) (result i32)
    (local $scale i32)
    (local $status i32)
    (local $alo i64)
    (local $ahi i64)
    (local $blo i64)
    (local $bhi i64)
    (local $borrow i64)
    (local.set $scale (select (local.get $a_scale) (local.get $b_scale) (i32.gt_u (local.get $a_scale) (local.get $b_scale))))
    (local.set $status (call $scale_to (local.get $a_lo) (local.get $a_hi) (local.get $a_scale) (local.get $scale) (local.get $out)))
    (if (local.get $status) (then (return (local.get $status))))
    (local.set $alo (i64.load (local.get $out)))
    (local.set $ahi (i64.load (i32.add (local.get $out) (i32.const 8))))
    (local.set $status (call $scale_to (local.get $b_lo) (local.get $b_hi) (local.get $b_scale) (local.get $scale) (local.get $out)))
    (if (local.get $status) (then (return (local.get $status))))
    (local.set $blo (i64.load (local.get $out)))
    (local.set $bhi (i64.load (i32.add (local.get $out) (i32.const 8))))
    (if (i32.or (i64.gt_u (local.get $bhi) (local.get $ahi)) (i32.and (i64.eq (local.get $bhi) (local.get $ahi)) (i64.gt_u (local.get $blo) (local.get $alo))))
      (then (return (i32.const 3))))
    (local.set $borrow (i64.extend_i32_u (i64.lt_u (local.get $alo) (local.get $blo))))
    (call $store_amount
      (local.get $out)
      (i64.sub (local.get $alo) (local.get $blo))
      (i64.sub (i64.sub (local.get $ahi) (local.get $bhi)) (local.get $borrow))
      (local.get $scale))
    i32.const 0)

  (func (export "wallet_decimal_compare")
    (param $a_lo i64) (param $a_hi i64) (param $a_scale i32)
    (param $b_lo i64) (param $b_hi i64) (param $b_scale i32) (result i32)
    (local $scale i32)
    (local $status i32)
    (local $alo i64)
    (local $ahi i64)
    (local $blo i64)
    (local $bhi i64)
    (local.set $scale (select (local.get $a_scale) (local.get $b_scale) (i32.gt_u (local.get $a_scale) (local.get $b_scale))))
    (local.set $status (call $scale_to (local.get $a_lo) (local.get $a_hi) (local.get $a_scale) (local.get $scale) (i32.const 64000)))
    (if (local.get $status) (then (return (i32.const 0))))
    (local.set $alo (i64.load (i32.const 64000)))
    (local.set $ahi (i64.load (i32.const 64008)))
    (local.set $status (call $scale_to (local.get $b_lo) (local.get $b_hi) (local.get $b_scale) (local.get $scale) (i32.const 64024)))
    (if (local.get $status) (then (return (i32.const 0))))
    (local.set $blo (i64.load (i32.const 64024)))
    (local.set $bhi (i64.load (i32.const 64032)))
    (if (i64.gt_u (local.get $ahi) (local.get $bhi)) (then (return (i32.const 1))))
    (if (i64.lt_u (local.get $ahi) (local.get $bhi)) (then (return (i32.const -1))))
    (if (i64.gt_u (local.get $alo) (local.get $blo)) (then (return (i32.const 1))))
    (if (i64.lt_u (local.get $alo) (local.get $blo)) (then (return (i32.const -1))))
    i32.const 0)

  (func $u128_digits_to_tmp (param $lo i64) (param $hi i64) (result i32)
    (local $pos i32)
    (local $qlo i64)
    (local $qhi i64)
    (local $rem i64)
    (local $bit i32)
    (local $bitval i64)
    (local.set $pos (i32.const 64))
    (if (i32.and (i64.eqz (local.get $lo)) (i64.eqz (local.get $hi)))
      (then
        (i32.store8 (i32.add (global.get $TMP) (i32.const 63)) (i32.const 48))
        (return (i32.const 1))))
    (block $digits_done
      (loop $digits
        (br_if $digits_done (i32.and (i64.eqz (local.get $lo)) (i64.eqz (local.get $hi))))
        (local.set $qlo (i64.const 0))
        (local.set $qhi (i64.const 0))
        (local.set $rem (i64.const 0))
        (local.set $bit (i32.const 127))
        (block $divide_done
          (loop $divide
            (if (i32.ge_u (local.get $bit) (i32.const 64))
              (then
                (local.set $bitval
                  (i64.and
                    (i64.shr_u (local.get $hi) (i64.extend_i32_u (i32.sub (local.get $bit) (i32.const 64))))
                    (i64.const 1))))
              (else
                (local.set $bitval
                  (i64.and
                    (i64.shr_u (local.get $lo) (i64.extend_i32_u (local.get $bit)))
                    (i64.const 1)))))
            (local.set $rem (i64.or (i64.shl (local.get $rem) (i64.const 1)) (local.get $bitval)))
            (if (i64.ge_u (local.get $rem) (i64.const 10))
              (then
                (local.set $rem (i64.sub (local.get $rem) (i64.const 10)))
                (if (i32.ge_u (local.get $bit) (i32.const 64))
                  (then
                    (local.set $qhi
                      (i64.or
                        (local.get $qhi)
                        (i64.shl (i64.const 1) (i64.extend_i32_u (i32.sub (local.get $bit) (i32.const 64)))))))
                  (else
                    (local.set $qlo
                      (i64.or
                        (local.get $qlo)
                        (i64.shl (i64.const 1) (i64.extend_i32_u (local.get $bit)))))))))
            (br_if $divide_done (i32.eqz (local.get $bit)))
            (local.set $bit (i32.sub (local.get $bit) (i32.const 1)))
            (br $divide)))
        (local.set $pos (i32.sub (local.get $pos) (i32.const 1)))
        (i32.store8 (i32.add (global.get $TMP) (local.get $pos)) (i32.add (i32.wrap_i64 (local.get $rem)) (i32.const 48)))
        (local.set $lo (local.get $qlo))
        (local.set $hi (local.get $qhi))
        (br $digits)))
    (i32.sub (i32.const 64) (local.get $pos)))

  (func (export "wallet_decimal_format")
    (param $lo i64) (param $hi i64) (param $scale i32) (param $out i32) (param $cap i32) (result i64)
    (local $digits i32)
    (local $int_digits i32)
    (local $written i32)
    (local $i i32)
    (local $zero_count i32)
    (if (i32.gt_u (local.get $scale) (i32.const 38))
      (then (return (i64.const 3))))
    (local.set $digits (call $u128_digits_to_tmp (local.get $lo) (local.get $hi)))
    (if (i32.eqz (local.get $scale))
      (then
        (if (i32.lt_u (local.get $cap) (local.get $digits)) (then (return (i64.const 2))))
        (local.set $i (i32.const 0))
        (block $done
          (loop $loop
            (br_if $done (i32.ge_u (local.get $i) (local.get $digits)))
            (i32.store8
              (i32.add (local.get $out) (local.get $i))
              (i32.load8_u (i32.add (i32.sub (global.get $TMP_END) (local.get $digits)) (local.get $i))))
            (local.set $i (i32.add (local.get $i) (i32.const 1)))
            (br $loop)))
        (return (i64.shl (i64.extend_i32_u (local.get $digits)) (i64.const 32)))))
    (if (i32.gt_u (local.get $digits) (local.get $scale))
      (then
        (local.set $int_digits (i32.sub (local.get $digits) (local.get $scale)))
        (local.set $written (i32.add (i32.add (local.get $int_digits) (i32.const 1)) (local.get $scale)))
        (if (i32.lt_u (local.get $cap) (local.get $written)) (then (return (i64.const 2))))
        (local.set $i (i32.const 0))
        (block $int_done
          (loop $int_loop
            (br_if $int_done (i32.ge_u (local.get $i) (local.get $int_digits)))
            (i32.store8
              (i32.add (local.get $out) (local.get $i))
              (i32.load8_u (i32.add (i32.sub (global.get $TMP_END) (local.get $digits)) (local.get $i))))
            (local.set $i (i32.add (local.get $i) (i32.const 1)))
            (br $int_loop)))
        (i32.store8 (i32.add (local.get $out) (local.get $int_digits)) (i32.const 46))
        (local.set $i (i32.const 0))
        (block $frac_done
          (loop $frac_loop
            (br_if $frac_done (i32.ge_u (local.get $i) (local.get $scale)))
            (i32.store8
              (i32.add (i32.add (i32.add (local.get $out) (local.get $int_digits)) (i32.const 1)) (local.get $i))
              (i32.load8_u (i32.add (i32.add (i32.sub (global.get $TMP_END) (local.get $digits)) (local.get $int_digits)) (local.get $i))))
            (local.set $i (i32.add (local.get $i) (i32.const 1)))
            (br $frac_loop)))
        (return (i64.shl (i64.extend_i32_u (local.get $written)) (i64.const 32)))))
    (local.set $zero_count (i32.sub (local.get $scale) (local.get $digits)))
    (local.set $written (i32.add (i32.add (i32.const 2) (local.get $zero_count)) (local.get $digits)))
    (if (i32.lt_u (local.get $cap) (local.get $written)) (then (return (i64.const 2))))
    (i32.store8 (local.get $out) (i32.const 48))
    (i32.store8 (i32.add (local.get $out) (i32.const 1)) (i32.const 46))
    (local.set $i (i32.const 0))
    (block $zero_done
      (loop $zero_loop
        (br_if $zero_done (i32.ge_u (local.get $i) (local.get $zero_count)))
        (i32.store8 (i32.add (i32.add (local.get $out) (i32.const 2)) (local.get $i)) (i32.const 48))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $zero_loop)))
    (local.set $i (i32.const 0))
    (block $copy_done
      (loop $copy_loop
        (br_if $copy_done (i32.ge_u (local.get $i) (local.get $digits)))
        (i32.store8
          (i32.add (i32.add (i32.add (local.get $out) (i32.const 2)) (local.get $zero_count)) (local.get $i))
          (i32.load8_u (i32.add (i32.sub (global.get $TMP_END) (local.get $digits)) (local.get $i))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $copy_loop)))
    (i64.shl (i64.extend_i32_u (local.get $written)) (i64.const 32)))
)
