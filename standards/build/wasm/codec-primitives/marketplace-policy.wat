(module
  (memory (export "memory") 1)

  (global $q0 (mut i64) (i64.const 0))
  (global $q1 (mut i64) (i64.const 0))
  (global $q2 (mut i64) (i64.const 0))
  (global $q3 (mut i64) (i64.const 0))

  (func (export "proto_abi_version") (result i32)
    i32.const 2)

  (func (export "proto_standard_id") (result i32)
    i32.const 300087)

  (func (export "marketplace_listing_status_valid") (param $id i32) (result i32)
    (i32.le_u (local.get $id) (i32.const 3)))

  (func (export "marketplace_checkout_status_valid") (param $id i32) (result i32)
    (i32.le_u (local.get $id) (i32.const 5)))

  (func $u64_le (param $a_lo i64) (param $a_hi i64) (param $b_lo i64) (param $b_hi i64) (result i32)
    (if (i64.lt_u (local.get $a_hi) (local.get $b_hi)) (then (return (i32.const 1))))
    (if (i64.gt_u (local.get $a_hi) (local.get $b_hi)) (then (return (i32.const 0))))
    (i64.le_u (local.get $a_lo) (local.get $b_lo)))

  (func (export "marketplace_listing_can_checkout")
    (param $status i32)
    (param $now_lo i64)
    (param $now_hi i64)
    (param $expires_lo i64)
    (param $expires_hi i64)
    (result i32)
    (if (i32.ne (local.get $status) (i32.const 1))
      (then (return (i32.const 0))))
    (if (i64.eqz (i64.or (local.get $expires_lo) (local.get $expires_hi)))
      (then (return (i32.const 1))))
    (call $u64_le
      (local.get $now_lo)
      (local.get $now_hi)
      (local.get $expires_lo)
      (local.get $expires_hi)))

  (func $commission_total
    (param $edgerun_bps i32)
    (param $app_bps i32)
    (param $affiliate_bps i32)
    (result i64)
    (i64.add
      (i64.add
        (i64.extend_i32_u (local.get $edgerun_bps))
        (i64.extend_i32_u (local.get $app_bps)))
      (i64.extend_i32_u (local.get $affiliate_bps))))

  (func $marketplace_commission_policy_valid (export "marketplace_commission_policy_valid")
    (param $edgerun_bps i32)
    (param $app_bps i32)
    (param $affiliate_bps i32)
    (param $max_total_bps i32)
    (result i32)
    (local $total i64)
    (if (i32.gt_u (local.get $max_total_bps) (i32.const 10000))
      (then (return (i32.const 0))))
    (local.set $total
      (call $commission_total
        (local.get $edgerun_bps)
        (local.get $app_bps)
        (local.get $affiliate_bps)))
    (i64.le_u
      (local.get $total)
      (i64.extend_i32_u (local.get $max_total_bps))))

  (func $bps_amount (param $gross_lo i64) (param $gross_hi i64) (param $bps i32) (result i32)
    (local $g0 i64)
    (local $g1 i64)
    (local $g2 i64)
    (local $g3 i64)
    (local $p0 i64)
    (local $p1 i64)
    (local $p2 i64)
    (local $p3 i64)
    (local $p4 i64)
    (local $carry i64)
    (local $rem i64)
    (local $cur i64)
    (local $q4 i64)
    (if (i32.gt_u (local.get $bps) (i32.const 10000))
      (then (return (i32.const 3))))
    (local.set $g0 (i64.and (local.get $gross_lo) (i64.const 0xffffffff)))
    (local.set $g1 (i64.shr_u (local.get $gross_lo) (i64.const 32)))
    (local.set $g2 (i64.and (local.get $gross_hi) (i64.const 0xffffffff)))
    (local.set $g3 (i64.shr_u (local.get $gross_hi) (i64.const 32)))
    (local.set $p0 (i64.mul (local.get $g0) (i64.extend_i32_u (local.get $bps))))
    (local.set $carry (i64.shr_u (local.get $p0) (i64.const 32)))
    (local.set $p0 (i64.and (local.get $p0) (i64.const 0xffffffff)))
    (local.set $p1
      (i64.add
        (i64.mul (local.get $g1) (i64.extend_i32_u (local.get $bps)))
        (local.get $carry)))
    (local.set $carry (i64.shr_u (local.get $p1) (i64.const 32)))
    (local.set $p1 (i64.and (local.get $p1) (i64.const 0xffffffff)))
    (local.set $p2
      (i64.add
        (i64.mul (local.get $g2) (i64.extend_i32_u (local.get $bps)))
        (local.get $carry)))
    (local.set $carry (i64.shr_u (local.get $p2) (i64.const 32)))
    (local.set $p2 (i64.and (local.get $p2) (i64.const 0xffffffff)))
    (local.set $p3
      (i64.add
        (i64.mul (local.get $g3) (i64.extend_i32_u (local.get $bps)))
        (local.get $carry)))
    (local.set $p4 (i64.shr_u (local.get $p3) (i64.const 32)))
    (local.set $p3 (i64.and (local.get $p3) (i64.const 0xffffffff)))

    (local.set $cur (local.get $p4))
    (local.set $q4 (i64.div_u (local.get $cur) (i64.const 10000)))
    (local.set $rem (i64.rem_u (local.get $cur) (i64.const 10000)))
    (if (i64.ne (local.get $q4) (i64.const 0))
      (then (return (i32.const 1))))

    (local.set $cur
      (i64.add (i64.shl (local.get $rem) (i64.const 32)) (local.get $p3)))
    (global.set $q3 (i64.div_u (local.get $cur) (i64.const 10000)))
    (local.set $rem (i64.rem_u (local.get $cur) (i64.const 10000)))

    (local.set $cur
      (i64.add (i64.shl (local.get $rem) (i64.const 32)) (local.get $p2)))
    (global.set $q2 (i64.div_u (local.get $cur) (i64.const 10000)))
    (local.set $rem (i64.rem_u (local.get $cur) (i64.const 10000)))

    (local.set $cur
      (i64.add (i64.shl (local.get $rem) (i64.const 32)) (local.get $p1)))
    (global.set $q1 (i64.div_u (local.get $cur) (i64.const 10000)))
    (local.set $rem (i64.rem_u (local.get $cur) (i64.const 10000)))

    (local.set $cur
      (i64.add (i64.shl (local.get $rem) (i64.const 32)) (local.get $p0)))
    (global.set $q0 (i64.div_u (local.get $cur) (i64.const 10000)))
    i32.const 0)

  (func $amount_lo (result i64)
    (i64.or
      (global.get $q0)
      (i64.shl (global.get $q1) (i64.const 32))))

  (func $amount_hi (result i64)
    (i64.or
      (global.get $q2)
      (i64.shl (global.get $q3) (i64.const 32))))

  (func $sub_u128
    (param $a_lo i64)
    (param $a_hi i64)
    (param $b_lo i64)
    (param $b_hi i64)
    (param $out_ptr i32)
    (result i32)
    (local $borrow i64)
    (if (i64.lt_u (local.get $a_hi) (local.get $b_hi))
      (then (return (i32.const 1))))
    (if
      (i32.and
        (i64.eq (local.get $a_hi) (local.get $b_hi))
        (i64.lt_u (local.get $a_lo) (local.get $b_lo)))
      (then (return (i32.const 1))))
    (local.set $borrow (i64.extend_i32_u (i64.lt_u (local.get $a_lo) (local.get $b_lo))))
    (i64.store (local.get $out_ptr) (i64.sub (local.get $a_lo) (local.get $b_lo)))
    (i64.store
      (i32.add (local.get $out_ptr) (i32.const 8))
      (i64.sub
        (i64.sub (local.get $a_hi) (local.get $b_hi))
        (local.get $borrow)))
    i32.const 0)

  (func (export "marketplace_payout_split")
    (param $gross_lo i64)
    (param $gross_hi i64)
    (param $edgerun_bps i32)
    (param $app_bps i32)
    (param $affiliate_bps i32)
    (param $out_ptr i32)
    (result i32)
    (local $seller_lo i64)
    (local $seller_hi i64)
    (local $edgerun_lo i64)
    (local $edgerun_hi i64)
    (local $app_lo i64)
    (local $app_hi i64)
    (local $affiliate_lo i64)
    (local $affiliate_hi i64)
    (local $total_lo i64)
    (local $total_hi i64)
    (local $carry i64)
    (local $status i32)
    (if (i32.eqz
      (call $marketplace_commission_policy_valid
        (local.get $edgerun_bps)
        (local.get $app_bps)
        (local.get $affiliate_bps)
        (i32.const 10000)))
      (then (return (i32.const 3))))

    (local.set $status (call $bps_amount (local.get $gross_lo) (local.get $gross_hi) (local.get $edgerun_bps)))
    (if (local.get $status) (then (return (local.get $status))))
    (local.set $edgerun_lo (call $amount_lo))
    (local.set $edgerun_hi (call $amount_hi))

    (local.set $status (call $bps_amount (local.get $gross_lo) (local.get $gross_hi) (local.get $app_bps)))
    (if (local.get $status) (then (return (local.get $status))))
    (local.set $app_lo (call $amount_lo))
    (local.set $app_hi (call $amount_hi))

    (local.set $status (call $bps_amount (local.get $gross_lo) (local.get $gross_hi) (local.get $affiliate_bps)))
    (if (local.get $status) (then (return (local.get $status))))
    (local.set $affiliate_lo (call $amount_lo))
    (local.set $affiliate_hi (call $amount_hi))

    (local.set $total_lo (i64.add (local.get $edgerun_lo) (local.get $app_lo)))
    (local.set $carry (i64.extend_i32_u (i64.lt_u (local.get $total_lo) (local.get $edgerun_lo))))
    (local.set $total_hi
      (i64.add
        (i64.add (local.get $edgerun_hi) (local.get $app_hi))
        (local.get $carry)))
    (local.set $carry (i64.extend_i32_u (i64.lt_u (local.get $total_hi) (local.get $edgerun_hi))))
    (if (i64.ne (local.get $carry) (i64.const 0)) (then (return (i32.const 1))))
    (local.set $carry (i64.extend_i32_u (i64.lt_u (local.get $total_lo) (local.get $affiliate_lo))))
    (local.set $total_lo (i64.add (local.get $total_lo) (local.get $affiliate_lo)))
    (local.set $total_hi
      (i64.add
        (i64.add (local.get $total_hi) (local.get $affiliate_hi))
        (local.get $carry)))
    (if (i64.lt_u (local.get $total_hi) (local.get $affiliate_hi))
      (then (return (i32.const 1))))
    (local.set $status
      (call $sub_u128
        (local.get $gross_lo)
        (local.get $gross_hi)
        (local.get $total_lo)
        (local.get $total_hi)
        (local.get $out_ptr)))
    (if (local.get $status) (then (return (local.get $status))))
    (local.set $seller_lo (i64.load (local.get $out_ptr)))
    (local.set $seller_hi (i64.load (i32.add (local.get $out_ptr) (i32.const 8))))
    (i64.store (local.get $out_ptr) (local.get $seller_lo))
    (i64.store (i32.add (local.get $out_ptr) (i32.const 8)) (local.get $seller_hi))
    (i64.store (i32.add (local.get $out_ptr) (i32.const 16)) (local.get $edgerun_lo))
    (i64.store (i32.add (local.get $out_ptr) (i32.const 24)) (local.get $edgerun_hi))
    (i64.store (i32.add (local.get $out_ptr) (i32.const 32)) (local.get $app_lo))
    (i64.store (i32.add (local.get $out_ptr) (i32.const 40)) (local.get $app_hi))
    (i64.store (i32.add (local.get $out_ptr) (i32.const 48)) (local.get $affiliate_lo))
    (i64.store (i32.add (local.get $out_ptr) (i32.const 56)) (local.get $affiliate_hi))
    i32.const 0)
)
