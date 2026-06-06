
(global $last_carry (mut i32) (i32.const 0))

  (func (export "carry") (result i32)
    global.get $last_carry)

  (func $byte_len (param $limbs i32) (result i32)
    local.get $limbs
    i32.const 3
    i32.shl)

  (func $m55range_ok (param $ptr i32) (param $len i32) (result i32)
    (local $end i32)
    local.get $ptr
    local.get $len
    i32.add
    local.set $end
    local.get $end
    local.get $ptr
    i32.lt_u
    if
      i32.const 0
      return
    end
    local.get $end
    i32.const 131072
    i32.le_u)

  (func $limbs_ok (param $limbs i32) (result i32)
    local.get $limbs
    i32.const 0
    i32.gt_u
    local.get $limbs
    i32.const 16
    i32.le_u
    i32.and)

  (func $args_ok (param $a i32) (param $b i32) (param $limbs i32) (param $out i32) (result i32)
    (local $bytes i32)
    local.get $limbs
    call $limbs_ok
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $limbs
    call $byte_len
    local.set $bytes
    local.get $a
    local.get $bytes
    call $m55range_ok
    local.get $b
    local.get $bytes
    call $m55range_ok
    i32.and
    local.get $out
    local.get $bytes
    call $m55range_ok
    i32.and)

  (func (export "cmp") (param $a i32) (param $b i32) (param $limbs i32) (result i32)
    (local $bytes i32)
    (local $i i32)
    (local $av i64)
    (local $bv i64)
    local.get $limbs
    call $limbs_ok
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $limbs
    call $byte_len
    local.set $bytes
    local.get $a
    local.get $bytes
    call $m55range_ok
    local.get $b
    local.get $bytes
    call $m55range_ok
    i32.and
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $limbs
    i32.const 1
    i32.sub
    local.set $i
    block $done
      loop $scan
        local.get $a
        local.get $i
        i32.const 3
        i32.shl
        i32.add
        i64.load
        local.set $av
        local.get $b
        local.get $i
        i32.const 3
        i32.shl
        i32.add
        i64.load
        local.set $bv
        local.get $av
        local.get $bv
        i64.lt_u
        if
          i32.const -1
          return
        end
        local.get $av
        local.get $bv
        i64.gt_u
        if
          i32.const 1
          return
        end
        local.get $i
        i32.eqz
        br_if $done
        local.get $i
        i32.const 1
        i32.sub
        local.set $i
        br $scan
      end
    end
    i32.const 0)

  (func (export "add") (param $a i32) (param $b i32) (param $limbs i32) (param $out i32) (result i64)
    (local $i i32)
    (local $av i64)
    (local $bv i64)
    (local $sum1 i64)
    (local $sum2 i64)
    (local $carry i32)
    (local $carry1 i32)
    local.get $a
    local.get $b
    local.get $limbs
    local.get $out
    call $args_ok
    i32.eqz
    if
      i32.const 0
      global.set $last_carry
      i32.const 1
      i32.const 0
      call $pack
      return
    end
    i32.const 0
    local.set $i
    i32.const 0
    local.set $carry
    loop $each
      local.get $a
      local.get $i
      i32.const 3
      i32.shl
      i32.add
      i64.load
      local.set $av
      local.get $b
      local.get $i
      i32.const 3
      i32.shl
      i32.add
      i64.load
      local.set $bv
      local.get $av
      local.get $bv
      i64.add
      local.set $sum1
      local.get $sum1
      local.get $av
      i64.lt_u
      local.set $carry1
      local.get $sum1
      local.get $carry
      i64.extend_i32_u
      i64.add
      local.set $sum2
      local.get $out
      local.get $i
      i32.const 3
      i32.shl
      i32.add
      local.get $sum2
      i64.store
      local.get $carry1
      local.get $sum2
      local.get $sum1
      i64.lt_u
      i32.or
      local.set $carry
      local.get $i
      i32.const 1
      i32.add
      local.tee $i
      local.get $limbs
      i32.lt_u
      br_if $each
    end
    local.get $carry
    global.set $last_carry
    i32.const 0
    local.get $limbs
    call $pack)

  (func (export "sub") (param $a i32) (param $b i32) (param $limbs i32) (param $out i32) (result i64)
    (local $i i32)
    (local $av i64)
    (local $bv i64)
    (local $diff1 i64)
    (local $diff2 i64)
    (local $borrow i32)
    (local $borrow1 i32)
    local.get $a
    local.get $b
    local.get $limbs
    local.get $out
    call $args_ok
    i32.eqz
    if
      i32.const 0
      global.set $last_carry
      i32.const 1
      i32.const 0
      call $pack
      return
    end
    i32.const 0
    local.set $i
    i32.const 0
    local.set $borrow
    loop $each
      local.get $a
      local.get $i
      i32.const 3
      i32.shl
      i32.add
      i64.load
      local.set $av
      local.get $b
      local.get $i
      i32.const 3
      i32.shl
      i32.add
      i64.load
      local.set $bv
      local.get $av
      local.get $bv
      i64.sub
      local.set $diff1
      local.get $av
      local.get $bv
      i64.lt_u
      local.set $borrow1
      local.get $diff1
      local.get $borrow
      i64.extend_i32_u
      i64.sub
      local.set $diff2
      local.get $out
      local.get $i
      i32.const 3
      i32.shl
      i32.add
      local.get $diff2
      i64.store
      local.get $borrow1
      local.get $diff1
      local.get $borrow
      i64.extend_i32_u
      i64.lt_u
      i32.or
      local.set $borrow
      local.get $i
      i32.const 1
      i32.add
      local.tee $i
      local.get $limbs
      i32.lt_u
      br_if $each
    end
    local.get $borrow
    global.set $last_carry
    i32.const 0
    local.get $limbs
    call $pack)

  (func $scratch_addr (param $word i32) (result i32)
    i32.const 196608
    local.get $word
    i32.const 2
    i32.shl
    i32.add)

  (func (export "mul_low") (param $a i32) (param $b i32) (param $limbs i32) (param $out i32) (result i64)
    (local $words i32)
    (local $i i32)
    (local $j i32)
    (local $k i32)
    (local $ai i64)
    (local $bj i64)
    (local $acc i64)
    (local $carry i64)
    local.get $a
    local.get $b
    local.get $limbs
    local.get $out
    call $args_ok
    i32.eqz
    if
      i32.const 0
      global.set $last_carry
      i32.const 1
      i32.const 0
      call $pack
      return
    end
    local.get $limbs
    i32.const 1
    i32.shl
    local.set $words
    i32.const 0
    local.set $i
    loop $zero
      local.get $i
      call $scratch_addr
      i32.const 0
      i32.store
      local.get $i
      i32.const 1
      i32.add
      local.tee $i
      local.get $words
      i32.lt_u
      br_if $zero
    end
    i32.const 0
    local.set $i
    loop $outer
      local.get $a
      local.get $i
      i32.const 2
      i32.shl
      i32.add
      i32.load
      i64.extend_i32_u
      local.set $ai
      i64.const 0
      local.set $carry
      i32.const 0
      local.set $j
      loop $inner
        local.get $i
        local.get $j
        i32.add
        local.set $k
        local.get $b
        local.get $j
        i32.const 2
        i32.shl
        i32.add
        i32.load
        i64.extend_i32_u
        local.set $bj
        local.get $k
        call $scratch_addr
        i32.load
        i64.extend_i32_u
        local.get $ai
        local.get $bj
        i64.mul
        i64.add
        local.get $carry
        i64.add
        local.set $acc
        local.get $k
        call $scratch_addr
        local.get $acc
        i32.wrap_i64
        i32.store
        local.get $acc
        i64.const 32
        i64.shr_u
        local.set $carry
        local.get $j
        i32.const 1
        i32.add
        local.tee $j
        local.get $words
        local.get $i
        i32.sub
        i32.lt_u
        br_if $inner
      end
      local.get $i
      i32.const 1
      i32.add
      local.tee $i
      local.get $words
      i32.lt_u
      br_if $outer
    end
    i32.const 0
    local.set $i
    loop $copy
      local.get $out
      local.get $i
      i32.const 2
      i32.shl
      i32.add
      local.get $i
      call $scratch_addr
      i32.load
      i32.store
      local.get $i
      i32.const 1
      i32.add
      local.tee $i
      local.get $words
      i32.lt_u
      br_if $copy
    end
    i32.const 0
    global.set $last_carry
    i32.const 0
    local.get $limbs
    call $pack)
