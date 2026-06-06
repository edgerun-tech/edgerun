(module
  ;; X25519 scalar multiplication (radix-2^25.5, 10 limbs).
  ;; Ported from edgerun_crypto_curve25519.inc — self-contained.
  (memory (export "memory") 2)

  (func (export "proto_abi_version") (result i32) i32.const 2)
  (func (export "proto_standard_id") (result i32) i32.const 300080)

  ;; Scratch: 8192 = product (19×i64), 8448+ = temp fe (80 bytes each)
  ;; A=8448 AA=8528 B=8608 BB=8688 E=8768 C=8848 D=8928 DA=9008
  ;; CB=9088 T0=9168 T1=9248 X1=9328 X2=9408 Z2=9488 X3=9568 Z3=9648
  ;; scalar_copy=9728 Z2INV=9760 TMP=9840

  (func $mb (param $i i32) (result i64)
    local.get $i i32.const 1 i32.and if (result i64) i64.const 25 else i64.const 26 end)

  (func $mask (param $i i32) (result i64)
    local.get $i i32.const 1 i32.and
    if (result i64) i64.const 0x1FFFFFF else i64.const 0x3FFFFFF end)

  (func $load4 (param $p i32) (result i64)
    local.get $p i32.load8_u offset=0 i64.extend_i32_u
    local.get $p i32.load8_u offset=1 i64.extend_i32_u i64.const 8 i64.shl i64.or
    local.get $p i32.load8_u offset=2 i64.extend_i32_u i64.const 16 i64.shl i64.or
    local.get $p i32.load8_u offset=3 i64.extend_i32_u i64.const 24 i64.shl i64.or)

  (func $fe_copy (param $d i32) (param $s i32)
    (local $i i32)
    i32.const 0 local.set $i
    block $done
    loop $loop
      local.get $d local.get $i i32.const 3 i32.shl i32.add
      local.get $s local.get $i i32.const 3 i32.shl i32.add i64.load i64.store
      local.get $i i32.const 1 i32.add local.tee $i i32.const 10 i32.lt_u br_if $loop
    end
    end)

  (func $fe_0 (param $d i32)
    (local $i i32)
    i32.const 0 local.set $i
    block $done
    loop $loop
      local.get $d local.get $i i32.const 3 i32.shl i32.add i64.const 0 i64.store
      local.get $i i32.const 1 i32.add local.tee $i i32.const 10 i32.lt_u br_if $loop
    end
    end)

  (func $fe_1 (param $d i32)
    local.get $d call $fe_0
    local.get $d i64.const 1 i64.store)

  (func $fe_norm (param $d i32)
    (local $i i32) (local $r i32) (local $carry i64) (local $v i64)
    i32.const 0 local.set $r
    block $rdone
    loop $rloop
      local.get $r i32.const 3 i32.ge_u br_if $rdone
      i64.const 0 local.set $carry
      i32.const 0 local.set $i
      block $ldone
      loop $lloop
        local.get $d local.get $i i32.const 3 i32.shl i32.add
        local.get $d local.get $i i32.const 3 i32.shl i32.add i64.load
        local.get $carry i64.add
        local.tee $v
        local.get $i call $mask i64.and i64.store
        local.get $v local.get $i call $mb i64.shr_u local.set $carry
        local.get $i i32.const 1 i32.add local.tee $i i32.const 10 i32.lt_u br_if $lloop
      end
      end
      local.get $d local.get $d i64.load
      local.get $carry i64.const 19 i64.mul i64.add i64.store
      local.get $r i32.const 1 i32.add local.set $r
      br $rloop
    end
    end)

  (func $fe_add (param $d i32) (param $a i32) (param $b i32)
    (local $i i32)
    i32.const 0 local.set $i
    block $done
    loop $loop
      local.get $d local.get $i i32.const 3 i32.shl i32.add
      local.get $a local.get $i i32.const 3 i32.shl i32.add i64.load
      local.get $b local.get $i i32.const 3 i32.shl i32.add i64.load i64.add i64.store
      local.get $i i32.const 1 i32.add local.tee $i i32.const 10 i32.lt_u br_if $loop
    end
    end
    local.get $d call $fe_norm)

  (func $fe_sub (param $d i32) (param $a i32) (param $b i32)
    (local $i i32) (local $bi i64)
    i32.const 0 local.set $i
    block $done
    loop $loop
      local.get $i i32.const 1 i32.and
      if (result i64) i64.const 1 i64.const 28 i64.shl else i64.const 1 i64.const 29 i64.shl end
      local.set $bi
      local.get $d local.get $i i32.const 3 i32.shl i32.add
      local.get $a local.get $i i32.const 3 i32.shl i32.add i64.load
      local.get $bi i64.add
      local.get $b local.get $i i32.const 3 i32.shl i32.add i64.load i64.sub i64.store
      local.get $i i32.const 1 i32.add local.tee $i i32.const 10 i32.lt_u br_if $loop
    end
    end
    local.get $d call $fe_norm)

  (func $fe_mul (param $d i32) (param $a i32) (param $b i32)
    (local $i i32) (local $j i32) (local $p i32) (local $ta i64) (local $tb i64)
    i32.const 8192 local.set $p
    ;; clear product array (19 limbs)
    i32.const 0 local.set $i
    block $cldone
    loop $clloop
      local.get $p local.get $i i32.add i64.const 0 i64.store
      local.get $i i32.const 8 i32.add local.tee $i i32.const 152 i32.lt_u br_if $clloop
    end
    end
    ;; schoolbook
    i32.const 0 local.set $i
    block $odone
    loop $oloop
      local.get $a local.get $i i32.const 3 i32.shl i32.add i64.load local.set $ta
      i32.const 0 local.set $j
      block $idone
      loop $iloop
        local.get $b local.get $j i32.const 3 i32.shl i32.add i64.load local.set $tb
        local.get $p local.get $i local.get $j i32.add i32.const 3 i32.shl i32.add
        local.get $p local.get $i local.get $j i32.add i32.const 3 i32.shl i32.add i64.load
        local.get $ta local.get $tb i64.mul i64.add i64.store
        local.get $j i32.const 1 i32.add local.tee $j i32.const 10 i32.lt_u br_if $iloop
      end
      end
      local.get $i i32.const 1 i32.add local.tee $i i32.const 10 i32.lt_u br_if $oloop
    end
    end
    ;; reduce: for k=10..18, p[k-10] += p[k] * 19
    i32.const 10 local.set $i
    block $rdone
    loop $rloop
      local.get $p local.get $i i32.const 10 i32.sub i32.const 3 i32.shl i32.add
      local.get $p local.get $i i32.const 10 i32.sub i32.const 3 i32.shl i32.add i64.load
      local.get $p local.get $i i32.const 3 i32.shl i32.add i64.load i64.const 19 i64.mul i64.add i64.store
      local.get $i i32.const 1 i32.add local.tee $i i32.const 19 i32.lt_u br_if $rloop
    end
    end
    ;; copy p[0..9] to dst
    i32.const 0 local.set $i
    block $cpdone
    loop $cploop
      local.get $d local.get $i i32.const 3 i32.shl i32.add
      local.get $p local.get $i i32.const 3 i32.shl i32.add i64.load i64.store
      local.get $i i32.const 1 i32.add local.tee $i i32.const 10 i32.lt_u br_if $cploop
    end
    end
    local.get $d call $fe_norm)

  (func $fe_sq (param $d i32) (param $a i32)
    local.get $d local.get $a local.get $a call $fe_mul)

  (func $fe_mul121665 (param $d i32) (param $a i32)
    (local $i i32) (local $carry i64) (local $v i64)
    i64.const 0 local.set $carry
    i32.const 0 local.set $i
    block $done
    loop $loop
      local.get $d local.get $i i32.const 3 i32.shl i32.add
      local.get $a local.get $i i32.const 3 i32.shl i32.add i64.load
      i64.const 121665 i64.mul local.get $carry i64.add
      local.tee $v
      local.get $i call $mask i64.and
      i64.store
      local.get $v local.get $i call $mb i64.shr_u local.set $carry
      local.get $i i32.const 1 i32.add local.tee $i i32.const 10 i32.lt_u br_if $loop
    end
    end
    local.get $d local.get $d i64.load
    local.get $carry i64.const 19 i64.mul i64.add i64.store
    local.get $d call $fe_norm)

  (func $fe_frombytes (param $d i32) (param $s i32)
    local.get $d local.get $s call $load4 i64.const 0x3FFFFFF i64.and i64.store
    local.get $d local.get $s i32.const 3 i32.add call $load4 i64.const 2 i64.shr_u i64.const 0x1FFFFFF i64.and i64.store offset=8
    local.get $d local.get $s i32.const 6 i32.add call $load4 i64.const 4 i64.shr_u i64.const 0x3FFFFFF i64.and i64.store offset=16
    local.get $d local.get $s i32.const 9 i32.add call $load4 i64.const 6 i64.shr_u i64.const 0x1FFFFFF i64.and i64.store offset=24
    local.get $d local.get $s i32.const 12 i32.add call $load4 i64.const 1 i64.shr_u i64.const 0x3FFFFFF i64.and i64.store offset=32
    local.get $d local.get $s i32.const 15 i32.add call $load4 i64.const 3 i64.shr_u i64.const 0x1FFFFFF i64.and i64.store offset=40
    local.get $d local.get $s i32.const 18 i32.add call $load4 i64.const 5 i64.shr_u i64.const 0x3FFFFFF i64.and i64.store offset=48
    local.get $d local.get $s i32.const 21 i32.add call $load4 i64.const 7 i64.shr_u i64.const 0x1FFFFFF i64.and i64.store offset=56
    local.get $d local.get $s i32.const 24 i32.add call $load4 i64.const 0x3FFFFFF i64.and i64.store offset=64
    local.get $d local.get $s i32.const 27 i32.add call $load4 i64.const 6 i64.shr_u i64.const 0x1FFFFFF i64.and i64.store offset=72)

  (func $fe_invert (param $d i32) (param $z i32)
    (local $t0 i32) (local $t1 i32) (local $t2 i32) (local $t3 i32)
    (local $t4 i32) (local $k i32)
    i32.const 8448 local.set $t0
    i32.const 8528 local.set $t1
    i32.const 8608 local.set $t2
    i32.const 8688 local.set $t3
    i32.const 8768 local.set $t4

    ;; t0 = z^2
    local.get $t0 local.get $z call $fe_sq
    ;; t1 = t0^2 = z^4
    local.get $t1 local.get $t0 call $fe_sq
    ;; t1 = t1^2 = z^8
    local.get $t1 local.get $t1 call $fe_sq
    ;; t1 = t1 * z = z^9
    local.get $t1 local.get $t1 local.get $z call $fe_mul
    ;; t0 = t0 * t1 = z^11
    local.get $t0 local.get $t0 local.get $t1 call $fe_mul
    ;; t2 = t0^2 = z^22
    local.get $t2 local.get $t0 call $fe_sq
    ;; t1 = t1 * t2 = z^31 = 2^5-1
    local.get $t1 local.get $t1 local.get $t2 call $fe_mul
    ;; === 2^5-1 reached ===

    ;; t2 = t1^2 → t2^16 → t1 = t2*t1 = z^(2^10-1)
    local.get $t2 local.get $t1 call $fe_sq
    i32.const 0 local.set $k
    block $s4d
    loop $s4l
      local.get $k i32.const 4 i32.ge_u br_if $s4d
      local.get $t2 local.get $t2 call $fe_sq
      local.get $k i32.const 1 i32.add local.set $k
      br $s4l
    end
    end
    local.get $t1 local.get $t2 local.get $t1 call $fe_mul
    ;; === 2^10-1 reached ===

    ;; t2 = t1^2 → t2^512 → t2 = t2*t1 = z^(2^20-1)
    local.get $t2 local.get $t1 call $fe_sq
    i32.const 0 local.set $k
    block $s9d
    loop $s9l
      local.get $k i32.const 9 i32.ge_u br_if $s9d
      local.get $t2 local.get $t2 call $fe_sq
      local.get $k i32.const 1 i32.add local.set $k
      br $s9l
    end
    end
    local.get $t2 local.get $t2 local.get $t1 call $fe_mul
    ;; === 2^20-1 reached ===

    ;; t3 = t2^2 → t3^2^19 → t3 = t3*t2 = z^(2^40-1)
    local.get $t3 local.get $t2 call $fe_sq
    i32.const 0 local.set $k
    block $s19d
    loop $s19l
      local.get $k i32.const 19 i32.ge_u br_if $s19d
      local.get $t3 local.get $t3 call $fe_sq
      local.get $k i32.const 1 i32.add local.set $k
      br $s19l
    end
    end
    local.get $t3 local.get $t3 local.get $t2 call $fe_mul
    ;; === 2^40-1 reached ===

    ;; t3 = t3^2 → t3^2^9 → t1 = t3*t1 = z^(2^50-1)
    local.get $t3 local.get $t3 call $fe_sq
    i32.const 0 local.set $k
    block $s9bd
    loop $s9bl
      local.get $k i32.const 9 i32.ge_u br_if $s9bd
      local.get $t3 local.get $t3 call $fe_sq
      local.get $k i32.const 1 i32.add local.set $k
      br $s9bl
    end
    end
    local.get $t1 local.get $t3 local.get $t1 call $fe_mul
    ;; === 2^50-1 reached ===

    ;; t2 = t1^2 → t2^49 → t2 = t2*t1 = z^(2^100-1)
    local.get $t2 local.get $t1 call $fe_sq
    i32.const 0 local.set $k
    block $s49d
    loop $s49l
      local.get $k i32.const 49 i32.ge_u br_if $s49d
      local.get $t2 local.get $t2 call $fe_sq
      local.get $k i32.const 1 i32.add local.set $k
      br $s49l
    end
    end
    local.get $t2 local.get $t2 local.get $t1 call $fe_mul
    ;; === 2^100-1 reached ===

    ;; t3 = t2^2 → t3^99 → t3 = t3*t2 = z^(2^200-1)
    local.get $t3 local.get $t2 call $fe_sq
    i32.const 0 local.set $k
    block $s99d
    loop $s99l
      local.get $k i32.const 99 i32.ge_u br_if $s99d
      local.get $t3 local.get $t3 call $fe_sq
      local.get $k i32.const 1 i32.add local.set $k
      br $s99l
    end
    end
    local.get $t3 local.get $t3 local.get $t2 call $fe_mul
    ;; === 2^200-1 reached ===

    ;; t3 = t3^2 → t3^49 → t1 = t3*t1 = z^(2^250-1)
    local.get $t3 local.get $t3 call $fe_sq
    i32.const 0 local.set $k
    block $s49bd
    loop $s49bl
      local.get $k i32.const 49 i32.ge_u br_if $s49bd
      local.get $t3 local.get $t3 call $fe_sq
      local.get $k i32.const 1 i32.add local.set $k
      br $s49bl
    end
    end
    local.get $t1 local.get $t3 local.get $t1 call $fe_mul
    ;; === 2^250-1 reached ===

    ;; t1 = t1^2 ×5 → t1 = t1 * t0 = z^(2^255-21) = z^p-2
    local.get $t1 local.get $t1 call $fe_sq
    local.get $t1 local.get $t1 call $fe_sq
    local.get $t1 local.get $t1 call $fe_sq
    local.get $t1 local.get $t1 call $fe_sq
    local.get $t1 local.get $t1 call $fe_sq
    local.get $t1 local.get $t1 local.get $t0 call $fe_mul

    local.get $d local.get $t1 call $fe_copy)

  (func $fe_tobytes (param $d i32) (param $s i32)
    (local $f0 i64) (local $f1 i64) (local $f2 i64) (local $f3 i64)
    (local $f4 i64) (local $f5 i64) (local $f6 i64) (local $f7 i64) (local $f8 i64) (local $f9 i64)
    (local $t0 i64) (local $t1 i64) (local $t2 i64) (local $t3 i64) (local $t4 i64)
    (local $t5 i64) (local $t6 i64) (local $t7 i64) (local $t8 i64) (local $t9 i64)
    (local $carry i64) (local $mask_val i64)

    local.get $s i64.load offset=0 local.set $f0
    local.get $s i64.load offset=8 local.set $f1
    local.get $s i64.load offset=16 local.set $f2
    local.get $s i64.load offset=24 local.set $f3
    local.get $s i64.load offset=32 local.set $f4
    local.get $s i64.load offset=40 local.set $f5
    local.get $s i64.load offset=48 local.set $f6
    local.get $s i64.load offset=56 local.set $f7
    local.get $s i64.load offset=64 local.set $f8
    local.get $s i64.load offset=72 local.set $f9
    ;; 3 rounds of carry propagation in locals
    ;; Round 1
    local.get $f1 local.get $f0 i64.const 26 i64.shr_u i64.add local.set $f1
    local.get $f0 i64.const 0x3FFFFFF i64.and local.set $f0
    local.get $f2 local.get $f1 i64.const 25 i64.shr_u i64.add local.set $f2
    local.get $f1 i64.const 0x1FFFFFF i64.and local.set $f1
    local.get $f3 local.get $f2 i64.const 26 i64.shr_u i64.add local.set $f3
    local.get $f2 i64.const 0x3FFFFFF i64.and local.set $f2
    local.get $f4 local.get $f3 i64.const 25 i64.shr_u i64.add local.set $f4
    local.get $f3 i64.const 0x1FFFFFF i64.and local.set $f3
    local.get $f5 local.get $f4 i64.const 26 i64.shr_u i64.add local.set $f5
    local.get $f4 i64.const 0x3FFFFFF i64.and local.set $f4
    local.get $f6 local.get $f5 i64.const 25 i64.shr_u i64.add local.set $f6
    local.get $f5 i64.const 0x1FFFFFF i64.and local.set $f5
    local.get $f7 local.get $f6 i64.const 26 i64.shr_u i64.add local.set $f7
    local.get $f6 i64.const 0x3FFFFFF i64.and local.set $f6
    local.get $f8 local.get $f7 i64.const 25 i64.shr_u i64.add local.set $f8
    local.get $f7 i64.const 0x1FFFFFF i64.and local.set $f7
    local.get $f9 local.get $f8 i64.const 26 i64.shr_u i64.add local.set $f9
    local.get $f8 i64.const 0x3FFFFFF i64.and local.set $f8
    local.get $f0 local.get $f9 i64.const 25 i64.shr_u i64.const 19 i64.mul i64.add local.set $f0
    local.get $f9 i64.const 0x1FFFFFF i64.and local.set $f9
    ;; Round 2
    local.get $f1 local.get $f0 i64.const 26 i64.shr_u i64.add local.set $f1
    local.get $f0 i64.const 0x3FFFFFF i64.and local.set $f0
    local.get $f2 local.get $f1 i64.const 25 i64.shr_u i64.add local.set $f2
    local.get $f1 i64.const 0x1FFFFFF i64.and local.set $f1
    local.get $f3 local.get $f2 i64.const 26 i64.shr_u i64.add local.set $f3
    local.get $f2 i64.const 0x3FFFFFF i64.and local.set $f2
    local.get $f4 local.get $f3 i64.const 25 i64.shr_u i64.add local.set $f4
    local.get $f3 i64.const 0x1FFFFFF i64.and local.set $f3
    local.get $f5 local.get $f4 i64.const 26 i64.shr_u i64.add local.set $f5
    local.get $f4 i64.const 0x3FFFFFF i64.and local.set $f4
    local.get $f6 local.get $f5 i64.const 25 i64.shr_u i64.add local.set $f6
    local.get $f5 i64.const 0x1FFFFFF i64.and local.set $f5
    local.get $f7 local.get $f6 i64.const 26 i64.shr_u i64.add local.set $f7
    local.get $f6 i64.const 0x3FFFFFF i64.and local.set $f6
    local.get $f8 local.get $f7 i64.const 25 i64.shr_u i64.add local.set $f8
    local.get $f7 i64.const 0x1FFFFFF i64.and local.set $f7
    local.get $f9 local.get $f8 i64.const 26 i64.shr_u i64.add local.set $f9
    local.get $f8 i64.const 0x3FFFFFF i64.and local.set $f8
    local.get $f0 local.get $f9 i64.const 25 i64.shr_u i64.const 19 i64.mul i64.add local.set $f0
    local.get $f9 i64.const 0x1FFFFFF i64.and local.set $f9
    ;; Round 3
    local.get $f1 local.get $f0 i64.const 26 i64.shr_u i64.add local.set $f1
    local.get $f0 i64.const 0x3FFFFFF i64.and local.set $f0
    local.get $f2 local.get $f1 i64.const 25 i64.shr_u i64.add local.set $f2
    local.get $f1 i64.const 0x1FFFFFF i64.and local.set $f1
    local.get $f3 local.get $f2 i64.const 26 i64.shr_u i64.add local.set $f3
    local.get $f2 i64.const 0x3FFFFFF i64.and local.set $f2
    local.get $f4 local.get $f3 i64.const 25 i64.shr_u i64.add local.set $f4
    local.get $f3 i64.const 0x1FFFFFF i64.and local.set $f3
    local.get $f5 local.get $f4 i64.const 26 i64.shr_u i64.add local.set $f5
    local.get $f4 i64.const 0x3FFFFFF i64.and local.set $f4
    local.get $f6 local.get $f5 i64.const 25 i64.shr_u i64.add local.set $f6
    local.get $f5 i64.const 0x1FFFFFF i64.and local.set $f5
    local.get $f7 local.get $f6 i64.const 26 i64.shr_u i64.add local.set $f7
    local.get $f6 i64.const 0x3FFFFFF i64.and local.set $f6
    local.get $f8 local.get $f7 i64.const 25 i64.shr_u i64.add local.set $f8
    local.get $f7 i64.const 0x1FFFFFF i64.and local.set $f7
    local.get $f9 local.get $f8 i64.const 26 i64.shr_u i64.add local.set $f9
    local.get $f8 i64.const 0x3FFFFFF i64.and local.set $f8
    local.get $f0 local.get $f9 i64.const 25 i64.shr_u i64.const 19 i64.mul i64.add local.set $f0
    local.get $f9 i64.const 0x1FFFFFF i64.and local.set $f9
    ;; Constant-time reduce: t = f + 19 (then propagate), select if t didn't overflow
    local.get $f0 i64.const 19 i64.add local.set $t0
    local.get $f1 local.set $t1
    local.get $f2 local.set $t2
    local.get $f3 local.set $t3
    local.get $f4 local.set $t4
    local.get $f5 local.set $t5
    local.get $f6 local.set $t6
    local.get $f7 local.set $t7
    local.get $f8 local.set $t8
    local.get $f9 local.set $t9
    local.get $t0 i64.const 26 i64.shr_u local.set $carry
    local.get $t0 i64.const 0x3FFFFFF i64.and local.set $t0
    local.get $t1 local.get $carry i64.add local.tee $t1 i64.const 25 i64.shr_u local.set $carry
    local.get $t1 i64.const 0x1FFFFFF i64.and local.set $t1
    local.get $t2 local.get $carry i64.add local.tee $t2 i64.const 26 i64.shr_u local.set $carry
    local.get $t2 i64.const 0x3FFFFFF i64.and local.set $t2
    local.get $t3 local.get $carry i64.add local.tee $t3 i64.const 25 i64.shr_u local.set $carry
    local.get $t3 i64.const 0x1FFFFFF i64.and local.set $t3
    local.get $t4 local.get $carry i64.add local.tee $t4 i64.const 26 i64.shr_u local.set $carry
    local.get $t4 i64.const 0x3FFFFFF i64.and local.set $t4
    local.get $t5 local.get $carry i64.add local.tee $t5 i64.const 25 i64.shr_u local.set $carry
    local.get $t5 i64.const 0x1FFFFFF i64.and local.set $t5
    local.get $t6 local.get $carry i64.add local.tee $t6 i64.const 26 i64.shr_u local.set $carry
    local.get $t6 i64.const 0x3FFFFFF i64.and local.set $t6
    local.get $t7 local.get $carry i64.add local.tee $t7 i64.const 25 i64.shr_u local.set $carry
    local.get $t7 i64.const 0x1FFFFFF i64.and local.set $t7
    local.get $t8 local.get $carry i64.add local.tee $t8 i64.const 26 i64.shr_u local.set $carry
    local.get $t8 i64.const 0x3FFFFFF i64.and local.set $t8
    local.get $t9 local.get $carry i64.add local.tee $t9 i64.const 25 i64.shr_u local.set $carry
    local.get $t9 i64.const 0x1FFFFFF i64.and local.set $t9
    local.get $t0 local.get $carry i64.const 19 i64.mul i64.add local.set $t0
    ;; if carry == 0: f < p, use f; if carry != 0: f >= p, use t
    local.get $carry i64.const 1 i64.xor i64.const 1 i64.sub local.set $mask_val
    local.get $f0 local.get $t0 i64.xor local.get $mask_val i64.and local.get $f0 i64.xor local.set $f0
    local.get $f1 local.get $t1 i64.xor local.get $mask_val i64.and local.get $f1 i64.xor local.set $f1
    local.get $f2 local.get $t2 i64.xor local.get $mask_val i64.and local.get $f2 i64.xor local.set $f2
    local.get $f3 local.get $t3 i64.xor local.get $mask_val i64.and local.get $f3 i64.xor local.set $f3
    local.get $f4 local.get $t4 i64.xor local.get $mask_val i64.and local.get $f4 i64.xor local.set $f4
    local.get $f5 local.get $t5 i64.xor local.get $mask_val i64.and local.get $f5 i64.xor local.set $f5
    local.get $f6 local.get $t6 i64.xor local.get $mask_val i64.and local.get $f6 i64.xor local.set $f6
    local.get $f7 local.get $t7 i64.xor local.get $mask_val i64.and local.get $f7 i64.xor local.set $f7
    local.get $f8 local.get $t8 i64.xor local.get $mask_val i64.and local.get $f8 i64.xor local.set $f8
    local.get $f9 local.get $t9 i64.xor local.get $mask_val i64.and local.get $f9 i64.xor local.set $f9
    ;; encode
    local.get $d local.get $f0 i64.const 0xFF i64.and i32.wrap_i64 i32.store8 offset=0
    local.get $d local.get $f0 i64.const 8 i64.shr_u i64.const 0xFF i64.and i32.wrap_i64 i32.store8 offset=1
    local.get $d local.get $f0 i64.const 16 i64.shr_u i64.const 0xFF i64.and i32.wrap_i64 i32.store8 offset=2
    local.get $d local.get $f0 i64.const 24 i64.shr_u i64.const 0x3F i64.and
               local.get $f1 i64.const 2 i64.shl i64.const 0xFF i64.and i64.or i32.wrap_i64 i32.store8 offset=3
    local.get $d local.get $f1 i64.const 6 i64.shr_u i64.const 0xFF i64.and i32.wrap_i64 i32.store8 offset=4
    local.get $d local.get $f1 i64.const 14 i64.shr_u i64.const 0xFF i64.and i32.wrap_i64 i32.store8 offset=5
    local.get $d local.get $f1 i64.const 22 i64.shr_u i64.const 0x3 i64.and
               local.get $f2 i64.const 4 i64.shl i64.const 0xFF i64.and i64.or i32.wrap_i64 i32.store8 offset=6
    local.get $d local.get $f2 i64.const 4 i64.shr_u i64.const 0xFF i64.and i32.wrap_i64 i32.store8 offset=7
    local.get $d local.get $f2 i64.const 12 i64.shr_u i64.const 0xFF i64.and i32.wrap_i64 i32.store8 offset=8
    local.get $d local.get $f2 i64.const 20 i64.shr_u i64.const 0x3F i64.and
               local.get $f3 i64.const 6 i64.shl i64.const 0xFF i64.and i64.or i32.wrap_i64 i32.store8 offset=9
    local.get $d local.get $f3 i64.const 2 i64.shr_u i64.const 0xFF i64.and i32.wrap_i64 i32.store8 offset=10
    local.get $d local.get $f3 i64.const 10 i64.shr_u i64.const 0xFF i64.and i32.wrap_i64 i32.store8 offset=11
    local.get $d local.get $f3 i64.const 18 i64.shr_u i64.const 0xFF i64.and i32.wrap_i64 i32.store8 offset=12
    local.get $d local.get $f3 i64.const 26 i64.shr_u i64.const 0x3 i64.and
               local.get $f4 i64.const 5 i64.shl i64.const 0xFF i64.and i64.or i32.wrap_i64 i32.store8 offset=13
    local.get $d local.get $f4 i64.const 5 i64.shr_u i64.const 0xFF i64.and i32.wrap_i64 i32.store8 offset=14
    local.get $d local.get $f4 i64.const 13 i64.shr_u i64.const 0xFF i64.and i32.wrap_i64 i32.store8 offset=15
    local.get $d local.get $f4 i64.const 21 i64.shr_u i64.const 0x7 i64.and
               local.get $f5 i64.const 3 i64.shl i64.const 0xFF i64.and i64.or i32.wrap_i64 i32.store8 offset=16
    local.get $d local.get $f5 i64.const 6 i64.shr_u i64.const 0xFF i64.and i32.wrap_i64 i32.store8 offset=17
    local.get $d local.get $f5 i64.const 14 i64.shr_u i64.const 0xFF i64.and i32.wrap_i64 i32.store8 offset=18
    local.get $d local.get $f5 i64.const 22 i64.shr_u i64.const 0x1 i64.and
               local.get $f6 i64.const 1 i64.shl i64.const 0xFF i64.and i64.or i32.wrap_i64 i32.store8 offset=19
    local.get $d local.get $f6 i64.const 4 i64.shr_u i64.const 0xFF i64.and i32.wrap_i64 i32.store8 offset=20
    local.get $d local.get $f6 i64.const 12 i64.shr_u i64.const 0xFF i64.and i32.wrap_i64 i32.store8 offset=21
    local.get $d local.get $f6 i64.const 20 i64.shr_u i64.const 0x3F i64.and
               local.get $f7 i64.const 6 i64.shl i64.const 0xFF i64.and i64.or i32.wrap_i64 i32.store8 offset=22
    local.get $d local.get $f7 i64.const 2 i64.shr_u i64.const 0xFF i64.and i32.wrap_i64 i32.store8 offset=23
    local.get $d local.get $f7 i64.const 10 i64.shr_u i64.const 0xFF i64.and i32.wrap_i64 i32.store8 offset=24
    local.get $d local.get $f7 i64.const 18 i64.shr_u i64.const 0xFF i64.and i32.wrap_i64 i32.store8 offset=25
    local.get $d local.get $f7 i64.const 26 i64.shr_u i64.const 0x3 i64.and
               local.get $f8 i64.const 5 i64.shl i64.const 0xFF i64.and i64.or i32.wrap_i64 i32.store8 offset=26
    local.get $d local.get $f8 i64.const 5 i64.shr_u i64.const 0xFF i64.and i32.wrap_i64 i32.store8 offset=27
    local.get $d local.get $f8 i64.const 13 i64.shr_u i64.const 0xFF i64.and i32.wrap_i64 i32.store8 offset=28
    local.get $d local.get $f8 i64.const 21 i64.shr_u i64.const 0x7 i64.and
               local.get $f9 i64.const 3 i64.shl i64.const 0xFF i64.and i64.or i32.wrap_i64 i32.store8 offset=29
    local.get $d local.get $f9 i64.const 6 i64.shr_u i64.const 0xFF i64.and i32.wrap_i64 i32.store8 offset=30
    local.get $d local.get $f9 i64.const 14 i64.shr_u i64.const 0xFF i64.and i32.wrap_i64 i32.store8 offset=31)

  (func $fe_cswap (param $a i32) (param $b i32) (param $swap i32)
    (local $i i32) (local $mask_i64 i64) (local $ai i64) (local $bi i64)
    local.get $swap
    if (result i64) i64.const 0xFFFFFFFFFFFFFFFF else i64.const 0 end
    local.set $mask_i64
    i32.const 0 local.set $i
    block $done
    loop $loop
      local.get $a local.get $i i32.const 3 i32.shl i32.add i64.load local.set $ai
      local.get $b local.get $i i32.const 3 i32.shl i32.add i64.load local.set $bi
      local.get $a local.get $i i32.const 3 i32.shl i32.add
      local.get $ai local.get $bi i64.xor local.get $mask_i64 i64.and local.get $ai i64.xor i64.store
      local.get $b local.get $i i32.const 3 i32.shl i32.add
      local.get $bi local.get $ai i64.xor local.get $mask_i64 i64.and local.get $bi i64.xor i64.store
      local.get $i i32.const 1 i32.add local.tee $i i32.const 10 i32.lt_u br_if $loop
    end
    end)

  ;; ── ladder_step(x2, z2, x3, z3, x1) via scratch ──
  ;; Parameters: all pointers to field elements in scratch
  (func $ladder_step (param $x2 i32) (param $z2 i32) (param $x3 i32) (param $z3 i32) (param $x1 i32)
    (local $A i32) (local $AA i32) (local $B i32) (local $BB i32) (local $E i32)
    (local $C i32) (local $D i32) (local $DA i32) (local $CB i32) (local $T0 i32) (local $T1 i32)
    i32.const 8448 local.set $A
    i32.const 8528 local.set $AA
    i32.const 8608 local.set $B
    i32.const 8688 local.set $BB
    i32.const 8768 local.set $E
    i32.const 8848 local.set $C
    i32.const 8928 local.set $D
    i32.const 9008 local.set $DA
    i32.const 9088 local.set $CB
    i32.const 9168 local.set $T0
    i32.const 9248 local.set $T1

    ;; A = x2 + z2
    local.get $A local.get $x2 local.get $z2 call $fe_add
    ;; AA = A^2
    local.get $AA local.get $A call $fe_sq
    ;; B = x2 - z2
    local.get $B local.get $x2 local.get $z2 call $fe_sub
    ;; BB = B^2
    local.get $BB local.get $B call $fe_sq
    ;; E = AA - BB
    local.get $E local.get $AA local.get $BB call $fe_sub
    ;; C = x3 + z3
    local.get $C local.get $x3 local.get $z3 call $fe_add
    ;; D = x3 - z3
    local.get $D local.get $x3 local.get $z3 call $fe_sub
    ;; DA = D * A
    local.get $DA local.get $D local.get $A call $fe_mul
    ;; CB = C * B
    local.get $CB local.get $C local.get $B call $fe_mul
    ;; x3 = (DA + CB)^2
    local.get $T0 local.get $DA local.get $CB call $fe_add
    local.get $x3 local.get $T0 call $fe_sq
    ;; z3 = x1 * (DA - CB)^2
    local.get $T1 local.get $DA local.get $CB call $fe_sub
    local.get $T1 local.get $T1 call $fe_sq
    local.get $z3 local.get $x1 local.get $T1 call $fe_mul
    ;; x2 = AA * BB
    local.get $x2 local.get $AA local.get $BB call $fe_mul
    ;; z2 = E * (AA + (121665 * E))
    local.get $T0 local.get $E call $fe_mul121665
    local.get $T0 local.get $AA local.get $T0 call $fe_add
    local.get $z2 local.get $E local.get $T0 call $fe_mul)

  ;; ── x25519_scalar_mult(out[32], scalar[32], point[32]) ──
  (func (export "x25519_scalar_mult")
    (param $out i32) (param $scalar i32) (param $point i32)
    (result i32)
    (local $sc i32) (local $x1 i32) (local $x2 i32) (local $z2 i32)
    (local $x3 i32) (local $z3 i32) (local $z2inv i32) (local $tmp i32)
    (local $b i32) (local $i i32) (local $bit i32) (local $swap i32)

    i32.const 9728 local.set $sc
    i32.const 9328 local.set $x1
    i32.const 9408 local.set $x2
    i32.const 9488 local.set $z2
    i32.const 9568 local.set $x3
    i32.const 9648 local.set $z3
    i32.const 9760 local.set $z2inv
    i32.const 9840 local.set $tmp

    ;; Copy scalar and clamp
    local.get $sc local.get $scalar i32.const 32 call $memcpy
    local.get $sc local.get $sc i32.load8_u i32.const 248 i32.and i32.store8
    local.get $sc i32.const 31 i32.add
    local.get $sc i32.const 31 i32.add i32.load8_u i32.const 127 i32.and i32.const 64 i32.or
    i32.store8

    ;; Decode point
    local.get $x1 local.get $point call $fe_frombytes

    ;; Set x2 = 1, z2 = 0, x3 = x1, z3 = 1
    local.get $x2 call $fe_1
    local.get $z2 call $fe_0
    local.get $x3 local.get $x1 call $fe_copy
    local.get $z3 call $fe_1

    i32.const 0 local.set $swap

    ;; Montgomery ladder: 255 bits (254 down to 0)
    i32.const 254 local.set $i
    block $ldone
    loop $lloop
      local.get $i i32.const 0 i32.lt_s br_if $ldone

      ;; bit = (scalar[i/8] >> (i%8)) & 1
      local.get $sc local.get $i i32.const 3 i32.shr_u i32.add i32.load8_u
      local.get $i i32.const 7 i32.and
      i32.shr_u
      i32.const 1 i32.and
      local.tee $bit

      ;; swap ^= bit
      local.get $swap i32.xor
      local.tee $swap

      ;; cswap(x2, x3, swap)
      local.get $x2 local.get $x3 local.get $swap call $fe_cswap
      ;; cswap(z2, z3, swap)
      local.get $z2 local.get $z3 local.get $swap call $fe_cswap

      ;; swap = bit
      local.get $bit local.set $swap

      ;; ladder_step(x2, z2, x3, z3, x1)
      local.get $x2 local.get $z2 local.get $x3 local.get $z3 local.get $x1
      call $ladder_step

      local.get $i i32.const 1 i32.sub local.set $i
      br $lloop
    end
    end

    ;; Final conditional swap
    local.get $x2 local.get $x3 local.get $swap call $fe_cswap
    local.get $z2 local.get $z3 local.get $swap call $fe_cswap

    ;; z2inv = 1/z2
    local.get $z2inv local.get $z2 call $fe_invert

    ;; x2 = x2 * z2inv
    local.get $tmp local.get $x2 local.get $z2inv call $fe_mul

    ;; encode x2 to 32 bytes
    local.get $out local.get $tmp call $fe_tobytes

    i32.const 0)

  (func $memcpy (param $dst i32) (param $src i32) (param $len i32)
    (local $i i32)
    i32.const 0 local.set $i
    block $done
    loop $loop
      local.get $i local.get $len i32.ge_u br_if $done
      local.get $dst local.get $i i32.add
      local.get $src local.get $i i32.add i32.load8_u i32.store8
      local.get $i i32.const 1 i32.add local.tee $i br_if $loop
    end
    end)
  )
