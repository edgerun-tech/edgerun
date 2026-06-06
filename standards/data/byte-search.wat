
  (func $matches3 (param $b i32) (param $n1 i32) (param $n2 i32) (param $n3 i32) (result i32)
    local.get $b
    local.get $n1
    i32.eq
    local.get $b
    local.get $n2
    i32.eq
    i32.or
    local.get $b
    local.get $n3
    i32.eq
    i32.or)

  (func (export "memchr") (param $ptr i32) (param $len i32) (param $needle i32) (result i32)
    (local $i i32)
    i32.const 0
    local.set $i
    (block $done
      (loop $scan
        local.get $i
        local.get $len
        i32.ge_u
        br_if $done
        local.get $ptr
        local.get $i
        call $load8_u
        local.get $needle
        i32.eq
        if
          local.get $i
          return
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $scan))
    i32.const -1)

  (func (export "memrchr") (param $ptr i32) (param $len i32) (param $needle i32) (result i32)
    (local $i i32)
    local.get $len
    local.set $i
    (block $done
      (loop $scan
        local.get $i
        i32.eqz
        br_if $done
        local.get $i
        i32.const 1
        i32.sub
        local.set $i
        local.get $ptr
        local.get $i
        call $load8_u
        local.get $needle
        i32.eq
        if
          local.get $i
          return
        end
        br $scan))
    i32.const -1)

  (func (export "memchr2")
    (param $ptr i32) (param $len i32) (param $n1 i32) (param $n2 i32) (result i32)
    (local $i i32)
    (local $b i32)
    i32.const 0
    local.set $i
    (block $done
      (loop $scan
        local.get $i
        local.get $len
        i32.ge_u
        br_if $done
        local.get $ptr
        local.get $i
        call $load8_u
        local.set $b
        local.get $b
        local.get $n1
        i32.eq
        local.get $b
        local.get $n2
        i32.eq
        i32.or
        if
          local.get $i
          return
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $scan))
    i32.const -1)

  (func (export "memrchr2")
    (param $ptr i32) (param $len i32) (param $n1 i32) (param $n2 i32) (result i32)
    (local $i i32)
    (local $b i32)
    local.get $len
    local.set $i
    (block $done
      (loop $scan
        local.get $i
        i32.eqz
        br_if $done
        local.get $i
        i32.const 1
        i32.sub
        local.set $i
        local.get $ptr
        local.get $i
        call $load8_u
        local.set $b
        local.get $b
        local.get $n1
        i32.eq
        local.get $b
        local.get $n2
        i32.eq
        i32.or
        if
          local.get $i
          return
        end
        br $scan))
    i32.const -1)

  (func (export "memchr3")
    (param $ptr i32) (param $len i32) (param $n1 i32) (param $n2 i32) (param $n3 i32) (result i32)
    (local $i i32)
    i32.const 0
    local.set $i
    (block $done
      (loop $scan
        local.get $i
        local.get $len
        i32.ge_u
        br_if $done
        local.get $ptr
        local.get $i
        call $load8_u
        local.get $n1
        local.get $n2
        local.get $n3
        call $matches3
        if
          local.get $i
          return
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $scan))
    i32.const -1)

  (func (export "memrchr3")
    (param $ptr i32) (param $len i32) (param $n1 i32) (param $n2 i32) (param $n3 i32) (result i32)
    (local $i i32)
    local.get $len
    local.set $i
    (block $done
      (loop $scan
        local.get $i
        i32.eqz
        br_if $done
        local.get $i
        i32.const 1
        i32.sub
        local.set $i
        local.get $ptr
        local.get $i
        call $load8_u
        local.get $n1
        local.get $n2
        local.get $n3
        call $matches3
        if
          local.get $i
          return
        end
        br $scan))
    i32.const -1)

  (func $needle_at
    (param $hay_ptr i32) (param $needle_ptr i32) (param $pos i32) (param $needle_len i32)
    (result i32)
    (local $j i32)
    i32.const 0
    local.set $j
    (block $yes
      (loop $scan
        local.get $j
        local.get $needle_len
        i32.ge_u
        br_if $yes
        local.get $hay_ptr
        local.get $pos
        i32.add
        local.get $j
        i32.add
        i32.load8_u
        local.get $needle_ptr
        local.get $j
        i32.add
        i32.load8_u
        i32.ne
        if
          i32.const 0
          return
        end
        local.get $j
        i32.const 1
        i32.add
        local.set $j
        br $scan))
    i32.const 1)

  (func (export "memmem_find")
    (param $hay_ptr i32) (param $hay_len i32) (param $needle_ptr i32) (param $needle_len i32)
    (result i32)
    (local $i i32)
    (local $last i32)
    local.get $needle_len
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $needle_len
    local.get $hay_len
    i32.gt_u
    if
      i32.const -1
      return
    end
    local.get $hay_len
    local.get $needle_len
    i32.sub
    local.set $last
    i32.const 0
    local.set $i
    (block $done
      (loop $scan
        local.get $i
        local.get $last
        i32.gt_u
        br_if $done
        local.get $hay_ptr
        local.get $needle_ptr
        local.get $i
        local.get $needle_len
        call $needle_at
        if
          local.get $i
          return
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $scan))
    i32.const -1)

  (func (export "memmem_rfind")
    (param $hay_ptr i32) (param $hay_len i32) (param $needle_ptr i32) (param $needle_len i32)
    (result i32)
    (local $i i32)
    local.get $needle_len
    i32.eqz
    if
      local.get $hay_len
      return
    end
    local.get $needle_len
    local.get $hay_len
    i32.gt_u
    if
      i32.const -1
      return
    end
    local.get $hay_len
    local.get $needle_len
    i32.sub
    i32.const 1
    i32.add
    local.set $i
    (block $done
      (loop $scan
        local.get $i
        i32.eqz
        br_if $done
        local.get $i
        i32.const 1
        i32.sub
        local.set $i
        local.get $hay_ptr
        local.get $needle_ptr
        local.get $i
        local.get $needle_len
        call $needle_at
        if
          local.get $i
          return
        end
        br $scan))
    i32.const -1)

  (func (export "simd_capabilities") (result i32)
    i32.const 1)

  (func (export "memchr_simd")
    (param $ptr i32) (param $len i32) (param $needle i32) (result i32)
    (local $i i32)
    (local $v v128)
    (local $eq v128)
    (local $splat_val v128)
    local.get $needle
    i8x16.splat
    local.set $splat_val
    i32.const 0
    local.set $i
    (block $done_simd
      (loop $scan_simd
        local.get $i
        i32.const 16
        i32.add
        local.get $len
        i32.gt_u
        br_if $done_simd
        local.get $ptr
        local.get $i
        i32.add
        v128.load
        local.set $v
        local.get $v
        local.get $splat_val
        i8x16.eq
        local.set $eq
        local.get $eq
        i8x16.bitmask
        i32.ctz
        local.get $i
        i32.add
        return
        local.get $i
        i32.const 16
        i32.add
        local.set $i
        br $scan_simd))
    (block $done_tail
      (loop $scan_tail
        local.get $i
        local.get $len
        i32.ge_u
        br_if $done_tail
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        local.get $needle
        i32.eq
        if
          local.get $i
          return
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $scan_tail))
    i32.const -1)

  (func (export "memrchr_simd")
    (param $ptr i32) (param $len i32) (param $needle i32) (result i32)
    (local $i i32)
    (local $v v128)
    (local $eq v128)
    (local $splat_val v128)
    local.get $needle
    i8x16.splat
    local.set $splat_val
    local.get $len
    local.set $i
    (block $done_simd
      (loop $scan_simd
        local.get $i
        i32.const 16
        i32.lt_u
        br_if $done_simd
        local.get $i
        i32.const 16
        i32.sub
        local.set $i
        local.get $ptr
        local.get $i
        i32.add
        v128.load
        local.set $v
        local.get $v
        local.get $splat_val
        i8x16.eq
        local.set $eq
        local.get $eq
        i8x16.bitmask
        i32.clz
        i32.const 31
        i32.sub
        local.get $i
        i32.add
        return
        br $scan_simd))
    (block $done_tail
      (loop $scan_tail
        local.get $i
        i32.eqz
        br_if $done_tail
        local.get $i
        i32.const 1
        i32.sub
        local.set $i
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        local.get $needle
        i32.eq
        if
          local.get $i
          return
        end
        br $scan_tail))
    i32.const -1)

  (func (export "memchr2_simd")
    (param $ptr i32) (param $len i32) (param $n1 i32) (param $n2 i32) (result i32)
    (local $i i32)
    (local $v v128)
    (local $eq1 v128)
    (local $eq2 v128)
    (local $combined v128)
    (local $splat1 v128)
    (local $splat2 v128)
    (local $b i32)
    local.get $n1
    i8x16.splat
    local.set $splat1
    local.get $n2
    i8x16.splat
    local.set $splat2
    i32.const 0
    local.set $i
    (block $done_simd
      (loop $scan_simd
        local.get $i
        i32.const 16
        i32.add
        local.get $len
        i32.gt_u
        br_if $done_simd
        local.get $ptr
        local.get $i
        i32.add
        v128.load
        local.set $v
        local.get $v
        local.get $splat1
        i8x16.eq
        local.set $eq1
        local.get $v
        local.get $splat2
        i8x16.eq
        local.set $eq2
        local.get $eq1
        local.get $eq2
        v128.or
        local.set $combined
        local.get $combined
        i8x16.bitmask
        i32.ctz
        local.get $i
        i32.add
        return
        local.get $i
        i32.const 16
        i32.add
        local.set $i
        br $scan_simd))
    (block $done_tail
      (loop $scan_tail
        local.get $i
        local.get $len
        i32.ge_u
        br_if $done_tail
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        local.set $b
        local.get $b
        local.get $n1
        i32.eq
        local.get $b
        local.get $n2
        i32.eq
        i32.or
        if
          local.get $i
          return
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $scan_tail))
    i32.const -1)

  (func (export "memrchr2_simd")
    (param $ptr i32) (param $len i32) (param $n1 i32) (param $n2 i32) (result i32)
    (local $i i32)
    (local $v v128)
    (local $eq1 v128)
    (local $eq2 v128)
    (local $combined v128)
    (local $splat1 v128)
    (local $splat2 v128)
    (local $b i32)
    local.get $n1
    i8x16.splat
    local.set $splat1
    local.get $n2
    i8x16.splat
    local.set $splat2
    local.get $len
    local.set $i
    (block $done_simd
      (loop $scan_simd
        local.get $i
        i32.const 16
        i32.lt_u
        br_if $done_simd
        local.get $i
        i32.const 16
        i32.sub
        local.set $i
        local.get $ptr
        local.get $i
        i32.add
        v128.load
        local.set $v
        local.get $v
        local.get $splat1
        i8x16.eq
        local.set $eq1
        local.get $v
        local.get $splat2
        i8x16.eq
        local.set $eq2
        local.get $eq1
        local.get $eq2
        v128.or
        local.set $combined
        local.get $combined
        i8x16.bitmask
        i32.clz
        i32.const 31
        i32.sub
        local.get $i
        i32.add
        return
        br $scan_simd))
    (block $done_tail
      (loop $scan_tail
        local.get $i
        i32.eqz
        br_if $done_tail
        local.get $i
        i32.const 1
        i32.sub
        local.set $i
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        local.set $b
        local.get $b
        local.get $n1
        i32.eq
        local.get $b
        local.get $n2
        i32.eq
        i32.or
        if
          local.get $i
          return
        end
        br $scan_tail))
    i32.const -1)

  (func (export "memchr3_simd")
    (param $ptr i32) (param $len i32) (param $n1 i32) (param $n2 i32) (param $n3 i32) (result i32)
    (local $i i32)
    (local $v v128)
    (local $eq1 v128)
    (local $eq2 v128)
    (local $eq3 v128)
    (local $combined v128)
    (local $splat1 v128)
    (local $splat2 v128)
    (local $splat3 v128)
    local.get $n1
    i8x16.splat
    local.set $splat1
    local.get $n2
    i8x16.splat
    local.set $splat2
    local.get $n3
    i8x16.splat
    local.set $splat3
    i32.const 0
    local.set $i
    (block $done_simd
      (loop $scan_simd
        local.get $i
        i32.const 16
        i32.add
        local.get $len
        i32.gt_u
        br_if $done_simd
        local.get $ptr
        local.get $i
        i32.add
        v128.load
        local.set $v
        local.get $v
        local.get $splat1
        i8x16.eq
        local.set $eq1
        local.get $v
        local.get $splat2
        i8x16.eq
        local.set $eq2
        local.get $v
        local.get $splat3
        i8x16.eq
        local.set $eq3
        local.get $eq1
        local.get $eq2
        v128.or
        local.get $eq3
        v128.or
        local.set $combined
        local.get $combined
        i8x16.bitmask
        i32.ctz
        local.get $i
        i32.add
        return
        local.get $i
        i32.const 16
        i32.add
        local.get $i
        i32.const 16
        i32.add
        local.set $i
        br $scan_simd))
    (block $done_tail
      (loop $scan_tail
        local.get $i
        local.get $len
        i32.ge_u
        br_if $done_tail
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        local.get $n1
        local.get $n2
        local.get $n3
        call $matches3
        if
          local.get $i
          return
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $scan_tail))
    i32.const -1)

  (func (export "memrchr3_simd")
    (param $ptr i32) (param $len i32) (param $n1 i32) (param $n2 i32) (param $n3 i32) (result i32)
    (local $i i32)
    (local $v v128)
    (local $eq1 v128)
    (local $eq2 v128)
    (local $eq3 v128)
    (local $combined v128)
    (local $splat1 v128)
    (local $splat2 v128)
    (local $splat3 v128)
    local.get $n1
    i8x16.splat
    local.set $splat1
    local.get $n2
    i8x16.splat
    local.set $splat2
    local.get $n3
    i8x16.splat
    local.set $splat3
    local.get $len
    local.set $i
    (block $done_simd
      (loop $scan_simd
        local.get $i
        i32.const 16
        i32.lt_u
        br_if $done_simd
        local.get $i
        i32.const 16
        i32.sub
        local.set $i
        local.get $ptr
        local.get $i
        i32.add
        v128.load
        local.set $v
        local.get $v
        local.get $splat1
        i8x16.eq
        local.set $eq1
        local.get $v
        local.get $splat2
        i8x16.eq
        local.set $eq2
        local.get $v
        local.get $splat3
        i8x16.eq
        local.set $eq3
        local.get $eq1
        local.get $eq2
        v128.or
        local.get $eq3
        v128.or
        local.set $combined
        local.get $combined
        i8x16.bitmask
        i32.clz
        i32.const 31
        i32.sub
        local.get $i
        i32.add
        return
        br $scan_simd))
    (block $done_tail
      (loop $scan_tail
        local.get $i
        i32.eqz
        br_if $done_tail
        local.get $i
        i32.const 1
        i32.sub
        local.set $i
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        local.get $n1
        local.get $n2
        local.get $n3
        call $matches3
        if
          local.get $i
          return
        end
        br $scan_tail))
    i32.const -1)
