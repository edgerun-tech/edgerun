  ;; Shared math utilities — import instead of redefining.
  ;; Consumers: (import "math" "min" (func $min (param i32 i32) (result i32)))

  (func (export "min") (param $a i32) (param $b i32) (result i32)
    local.get $a local.get $b i32.lt_s if (result i32) local.get $a else local.get $b end)

  (func (export "min_u") (param $a i32) (param $b i32) (result i32)
    local.get $a local.get $b i32.lt_u if (result i32) local.get $a else local.get $b end)

  (func (export "max") (param $a i32) (param $b i32) (result i32)
    local.get $a local.get $b i32.gt_s if (result i32) local.get $a else local.get $b end)

  (func (export "max_u") (param $a i32) (param $b i32) (result i32)
    local.get $a local.get $b i32.gt_u if (result i32) local.get $a else local.get $b end)

  (func (export "sat_sub") (param $a i32) (param $b i32) (result i32)
    local.get $a local.get $b i32.gt_u if (result i32)
      local.get $a local.get $b i32.sub
    else i32.const 0 end)

  (func (export "round_up") (param $n i32) (param $align i32) (result i32)
    (i32.and
      (i32.add (local.get $n) (i32.sub (local.get $align) (i32.const 1)))
      (i32.xor (i32.sub (local.get $align) (i32.const 1)) (i32.const -1))))

  (func (export "clamp") (param $v i32) (param $lo i32) (param $hi i32) (result i32)
    local.get $v local.get $lo i32.lt_s if (result i32) local.get $lo
    else local.get $v local.get $hi i32.gt_s if (result i32) local.get $hi
    else local.get $v end end)

  (func (export "max0") (param $n i32) (result i32)
    local.get $n i32.const 0 i32.gt_s if (result i32) local.get $n else i32.const 0 end)

  (func (export "min_f32") (param $a f32) (param $b f32) (result f32)
    local.get $a local.get $b f32.lt if (result f32) local.get $a else local.get $b end)

  (func (export "max_f32") (param $a f32) (param $b f32) (result f32)
    local.get $a local.get $b f32.gt if (result f32) local.get $a else local.get $b end)

  (func (export "clamp_f32") (param $v f32) (param $lo f32) (param $hi f32) (result f32)
    local.get $v local.get $lo call $max_f32 local.get $hi call $min_f32)

  (func (export "prefix_mask") (param $prefix_bits i32) (result i32)
    (if (result i32) (i32.eq (local.get $prefix_bits) (i32.const 8))
      (then (i32.const 255))
      (else (i32.sub (i32.shl (i32.const 1) (local.get $prefix_bits)) (i32.const 1)))))

  (func (export "align_up") (param $n i32) (param $align i32) (result i32)
    (i32.and
      (i32.add (local.get $n) (i32.sub (local.get $align) (i32.const 1)))
      (i32.xor (i32.sub (local.get $align) (i32.const 1)) (i32.const -1))))

  (func (export "is_power_of_two") (param $n i32) (result i32)
    (if (result i32) (i32.eqz (local.get $n))
      (then (i32.const 0))
      (else (i32.eqz (i32.and (local.get $n) (i32.sub (local.get $n) (i32.const 1)))))))

  (func (export "min64") (param $a i64) (param $b i64) (result i64)
    (if (result i64) (i64.lt_s (local.get $a) (local.get $b))
      (then (local.get $a))
      (else (local.get $b))))

  (func (export "max64") (param $a i64) (param $b i64) (result i64)
    (if (result i64) (i64.gt_s (local.get $a) (local.get $b))
      (then (local.get $a))
      (else (local.get $b))))
