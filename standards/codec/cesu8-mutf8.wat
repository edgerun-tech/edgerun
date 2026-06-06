  (import "edgerun" "load8_u" (func $m41byte (param i32 i32) (result i32)))
  (import "edgerun" "is_cont" (func $m41is_cont (param i32) (result i32)))

  ;; Status values: 0 ok, 2 output_short, 3 invalid, 5 incomplete.
  ;; Packed result: low 32 bits status, high 32 bits output byte count.

  (func (export "proto_standard_id") (result i32)
    i32.const 300069)


  (func $put1 (param $out i32) (param $cap i32) (param $j i32) (param $b0 i32) (result i32)
    local.get $j
    i32.const 1
    i32.add
    local.get $cap
    i32.gt_u
    if
      i32.const -1
      return
    end
    local.get $out
    local.get $j
    i32.add
    local.get $b0
    i32.store8
    local.get $j
    i32.const 1
    i32.add)

  (func $m41put2 (param $out i32) (param $cap i32) (param $j i32) (param $b0 i32) (param $b1 i32) (result i32)
    local.get $j
    i32.const 2
    i32.add
    local.get $cap
    i32.gt_u
    if
      i32.const -1
      return
    end
    local.get $out
    local.get $j
    i32.add
    local.get $b0
    i32.store8
    local.get $out
    local.get $j
    i32.add
    i32.const 1
    i32.add
    local.get $b1
    i32.store8
    local.get $j
    i32.const 2
    i32.add)

  (func $put3 (param $out i32) (param $cap i32) (param $j i32) (param $b0 i32) (param $b1 i32) (param $b2 i32) (result i32)
    local.get $j
    i32.const 3
    i32.add
    local.get $cap
    i32.gt_u
    if
      i32.const -1
      return
    end
    local.get $out
    local.get $j
    i32.add
    local.get $b0
    i32.store8
    local.get $out
    local.get $j
    i32.add
    i32.const 1
    i32.add
    local.get $b1
    i32.store8
    local.get $out
    local.get $j
    i32.add
    i32.const 2
    i32.add
    local.get $b2
    i32.store8
    local.get $j
    i32.const 3
    i32.add)

  (func $m41put4 (param $out i32) (param $cap i32) (param $j i32) (param $b0 i32) (param $b1 i32) (param $b2 i32) (param $b3 i32) (result i32)
    local.get $j
    i32.const 4
    i32.add
    local.get $cap
    i32.gt_u
    if
      i32.const -1
      return
    end
    local.get $out
    local.get $j
    i32.add
    local.get $b0
    i32.store8
    local.get $out
    local.get $j
    i32.add
    i32.const 1
    i32.add
    local.get $b1
    i32.store8
    local.get $out
    local.get $j
    i32.add
    i32.const 2
    i32.add
    local.get $b2
    i32.store8
    local.get $out
    local.get $j
    i32.add
    i32.const 3
    i32.add
    local.get $b3
    i32.store8
    local.get $j
    i32.const 4
    i32.add)

  (func $decode_surrogate (param $b1 i32) (param $b2 i32) (result i32)
    i32.const 53248
    local.get $b1
    i32.const 63
    i32.and
    i32.const 6
    i32.shl
    i32.or
    local.get $b2
    i32.const 63
    i32.and
    i32.or)

  (func $encode_surrogate (param $out i32) (param $cap i32) (param $j i32) (param $s i32) (result i32)
    local.get $out
    local.get $cap
    local.get $j
    i32.const 224
    local.get $s
    i32.const 12
    i32.shr_u
    i32.or
    i32.const 128
    local.get $s
    i32.const 6
    i32.shr_u
    i32.const 63
    i32.and
    i32.or
    i32.const 128
    local.get $s
    i32.const 63
    i32.and
    i32.or
    call $put3)

  (func $decode (param $ptr i32) (param $len i32) (param $out i32) (param $cap i32) (param $mutf i32) (result i64)
    (local $i i32)
    (local $j i32)
    (local $b0 i32)
    (local $b1 i32)
    (local $b2 i32)
    (local $b3 i32)
    (local $b4 i32)
    (local $b5 i32)
    (local $high i32)
    (local $low i32)
    (local $cp i32)
    (loop $again
      local.get $i
      local.get $len
      i32.ge_u
      if
        i32.const 0
        local.get $j
        call $pack
        return
      end

      local.get $ptr
      local.get $i
      call $m41byte
      local.set $b0

      local.get $mutf
      local.get $b0
      i32.eqz
      i32.and
      if
        i32.const 3
        local.get $j
        call $pack
        return
      end

      local.get $b0
      i32.const 128
      i32.lt_u
      if
        local.get $out
        local.get $cap
        local.get $j
        local.get $b0
        call $put1
        local.tee $j
        i32.const -1
        i32.eq
        if
          i32.const 2
          local.get $cap
          call $pack
          return
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $again
      end

      local.get $b0
      i32.const 192
      i32.eq
      local.get $mutf
      i32.and
      if
        local.get $i
        i32.const 2
        i32.add
        local.get $len
        i32.gt_u
        if
          i32.const 5
          local.get $j
          call $pack
          return
        end
        local.get $ptr
        local.get $i
        i32.const 1
        i32.add
        call $m41byte
        i32.const 128
        i32.ne
        if
          i32.const 3
          local.get $j
          call $pack
          return
        end
        local.get $out
        local.get $cap
        local.get $j
        i32.const 0
        call $put1
        local.tee $j
        i32.const -1
        i32.eq
        if
          i32.const 2
          local.get $cap
          call $pack
          return
        end
        local.get $i
        i32.const 2
        i32.add
        local.set $i
        br $again
      end

      local.get $b0
      i32.const 194
      i32.ge_u
      local.get $b0
      i32.const 223
      i32.le_u
      i32.and
      if
        local.get $i
        i32.const 2
        i32.add
        local.get $len
        i32.gt_u
        if
          i32.const 5
          local.get $j
          call $pack
          return
        end
        local.get $ptr
        local.get $i
        i32.const 1
        i32.add
        call $m41byte
        local.tee $b1
        call $m41is_cont
        i32.eqz
        if
          i32.const 3
          local.get $j
          call $pack
          return
        end
        local.get $out
        local.get $cap
        local.get $j
        local.get $b0
        local.get $b1
        call $m41put2
        local.tee $j
        i32.const -1
        i32.eq
        if
          i32.const 2
          local.get $cap
          call $pack
          return
        end
        local.get $i
        i32.const 2
        i32.add
        local.set $i
        br $again
      end

      local.get $b0
      i32.const 224
      i32.ge_u
      local.get $b0
      i32.const 239
      i32.le_u
      i32.and
      if
        local.get $i
        i32.const 3
        i32.add
        local.get $len
        i32.gt_u
        if
          i32.const 5
          local.get $j
          call $pack
          return
        end
        local.get $ptr
        local.get $i
        i32.const 1
        i32.add
        call $m41byte
        local.set $b1
        local.get $ptr
        local.get $i
        i32.const 2
        i32.add
        call $m41byte
        local.set $b2

        ;; Non-surrogate BMP triples copied unchanged.
        local.get $b0
        i32.const 224
        i32.eq
        local.get $b1
        i32.const 160
        i32.ge_u
        i32.and
        local.get $b1
        i32.const 191
        i32.le_u
        i32.and
        local.get $b2
        call $m41is_cont
        i32.and
        local.get $b0
        i32.const 225
        i32.ge_u
        local.get $b0
        i32.const 236
        i32.le_u
        i32.and
        local.get $b1
        call $m41is_cont
        i32.and
        local.get $b2
        call $m41is_cont
        i32.and
        i32.or
        local.get $b0
        i32.const 238
        i32.ge_u
        local.get $b0
        i32.const 239
        i32.le_u
        i32.and
        local.get $b1
        call $m41is_cont
        i32.and
        local.get $b2
        call $m41is_cont
        i32.and
        i32.or
        local.get $b0
        i32.const 237
        i32.eq
        local.get $b1
        i32.const 128
        i32.ge_u
        i32.and
        local.get $b1
        i32.const 159
        i32.le_u
        i32.and
        local.get $b2
        call $m41is_cont
        i32.and
        i32.or
        if
          local.get $out
          local.get $cap
          local.get $j
          local.get $b0
          local.get $b1
          local.get $b2
          call $put3
          local.tee $j
          i32.const -1
          i32.eq
          if
            i32.const 2
            local.get $cap
            call $pack
            return
          end
          local.get $i
          i32.const 3
          i32.add
          local.set $i
          br $again
        end

        ;; CESU-8 surrogate pair: ED A0..AF xx ED B0..BF xx.
        local.get $b0
        i32.const 237
        i32.eq
        local.get $b1
        i32.const 160
        i32.ge_u
        i32.and
        local.get $b1
        i32.const 175
        i32.le_u
        i32.and
        if
          local.get $i
          i32.const 6
          i32.add
          local.get $len
          i32.gt_u
          if
            i32.const 5
            local.get $j
            call $pack
            return
          end
          local.get $b2
          call $m41is_cont
          i32.eqz
          if
            i32.const 3
            local.get $j
            call $pack
            return
          end
          local.get $ptr
          local.get $i
          i32.const 3
          i32.add
          call $m41byte
          local.set $b3
          local.get $ptr
          local.get $i
          i32.const 4
          i32.add
          call $m41byte
          local.set $b4
          local.get $ptr
          local.get $i
          i32.const 5
          i32.add
          call $m41byte
          local.set $b5
          local.get $b3
          i32.const 237
          i32.ne
          local.get $b4
          i32.const 176
          i32.lt_u
          i32.or
          local.get $b4
          i32.const 191
          i32.gt_u
          i32.or
          local.get $b5
          call $m41is_cont
          i32.eqz
          i32.or
          if
            i32.const 3
            local.get $j
            call $pack
            return
          end

          local.get $b1
          local.get $b2
          call $decode_surrogate
          local.set $high
          local.get $b4
          local.get $b5
          call $decode_surrogate
          local.set $low
          i32.const 65536
          local.get $high
          i32.const 55296
          i32.sub
          i32.const 10
          i32.shl
          local.get $low
          i32.const 56320
          i32.sub
          i32.or
          i32.add
          local.set $cp
          local.get $out
          local.get $cap
          local.get $j
          i32.const 240
          local.get $cp
          i32.const 18
          i32.shr_u
          i32.or
          i32.const 128
          local.get $cp
          i32.const 12
          i32.shr_u
          i32.const 63
          i32.and
          i32.or
          i32.const 128
          local.get $cp
          i32.const 6
          i32.shr_u
          i32.const 63
          i32.and
          i32.or
          i32.const 128
          local.get $cp
          i32.const 63
          i32.and
          i32.or
          call $m41put4
          local.tee $j
          i32.const -1
          i32.eq
          if
            i32.const 2
            local.get $cap
            call $pack
            return
          end
          local.get $i
          i32.const 6
          i32.add
          local.set $i
          br $again
        end

        i32.const 3
        local.get $j
        call $pack
        return
      end

      i32.const 3
      local.get $j
      call $pack
      return
    )
    i32.const 0
    local.get $j
    call $pack)

  (func $encode (param $ptr i32) (param $len i32) (param $out i32) (param $cap i32) (param $mutf i32) (result i64)
    (local $i i32)
    (local $j i32)
    (local $b0 i32)
    (local $b1 i32)
    (local $b2 i32)
    (local $b3 i32)
    (local $cp i32)
    (local $tmp i32)
    (loop $again
      local.get $i
      local.get $len
      i32.ge_u
      if
        i32.const 0
        local.get $j
        call $pack
        return
      end
      local.get $ptr
      local.get $i
      call $m41byte
      local.set $b0

      local.get $b0
      i32.const 128
      i32.lt_u
      if
        local.get $mutf
        local.get $b0
        i32.eqz
        i32.and
        if
          local.get $out
          local.get $cap
          local.get $j
          i32.const 192
          i32.const 128
          call $m41put2
          local.tee $j
          i32.const -1
          i32.eq
          if
            i32.const 2
            local.get $cap
            call $pack
            return
          end
        else
          local.get $out
          local.get $cap
          local.get $j
          local.get $b0
          call $put1
          local.tee $j
          i32.const -1
          i32.eq
          if
            i32.const 2
            local.get $cap
            call $pack
            return
          end
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $again
      end

      local.get $b0
      i32.const 194
      i32.ge_u
      local.get $b0
      i32.const 223
      i32.le_u
      i32.and
      if
        local.get $i
        i32.const 2
        i32.add
        local.get $len
        i32.gt_u
        if
          i32.const 5
          local.get $j
          call $pack
          return
        end
        local.get $ptr
        local.get $i
        i32.const 1
        i32.add
        call $m41byte
        local.tee $b1
        call $m41is_cont
        i32.eqz
        if
          i32.const 3
          local.get $j
          call $pack
          return
        end
        local.get $out
        local.get $cap
        local.get $j
        local.get $b0
        local.get $b1
        call $m41put2
        local.tee $j
        i32.const -1
        i32.eq
        if
          i32.const 2
          local.get $cap
          call $pack
          return
        end
        local.get $i
        i32.const 2
        i32.add
        local.set $i
        br $again
      end

      local.get $b0
      i32.const 224
      i32.ge_u
      local.get $b0
      i32.const 239
      i32.le_u
      i32.and
      if
        local.get $i
        i32.const 3
        i32.add
        local.get $len
        i32.gt_u
        if
          i32.const 5
          local.get $j
          call $pack
          return
        end
        local.get $ptr
        local.get $i
        i32.const 1
        i32.add
        call $m41byte
        local.set $b1
        local.get $ptr
        local.get $i
        i32.const 2
        i32.add
        call $m41byte
        local.set $b2
        local.get $b0
        i32.const 224
        i32.eq
        local.get $b1
        i32.const 160
        i32.ge_u
        i32.and
        local.get $b1
        i32.const 191
        i32.le_u
        i32.and
        local.get $b2
        call $m41is_cont
        i32.and
        local.get $b0
        i32.const 225
        i32.ge_u
        local.get $b0
        i32.const 236
        i32.le_u
        i32.and
        local.get $b1
        call $m41is_cont
        i32.and
        local.get $b2
        call $m41is_cont
        i32.and
        i32.or
        local.get $b0
        i32.const 238
        i32.ge_u
        local.get $b0
        i32.const 239
        i32.le_u
        i32.and
        local.get $b1
        call $m41is_cont
        i32.and
        local.get $b2
        call $m41is_cont
        i32.and
        i32.or
        local.get $b0
        i32.const 237
        i32.eq
        local.get $b1
        i32.const 128
        i32.ge_u
        i32.and
        local.get $b1
        i32.const 159
        i32.le_u
        i32.and
        local.get $b2
        call $m41is_cont
        i32.and
        i32.or
        i32.eqz
        if
          i32.const 3
          local.get $j
          call $pack
          return
        end
        local.get $out
        local.get $cap
        local.get $j
        local.get $b0
        local.get $b1
        local.get $b2
        call $put3
        local.tee $j
        i32.const -1
        i32.eq
        if
          i32.const 2
          local.get $cap
          call $pack
          return
        end
        local.get $i
        i32.const 3
        i32.add
        local.set $i
        br $again
      end

      local.get $b0
      i32.const 240
      i32.ge_u
      local.get $b0
      i32.const 244
      i32.le_u
      i32.and
      if
        local.get $i
        i32.const 4
        i32.add
        local.get $len
        i32.gt_u
        if
          i32.const 5
          local.get $j
          call $pack
          return
        end
        local.get $ptr
        local.get $i
        i32.const 1
        i32.add
        call $m41byte
        local.set $b1
        local.get $ptr
        local.get $i
        i32.const 2
        i32.add
        call $m41byte
        local.set $b2
        local.get $ptr
        local.get $i
        i32.const 3
        i32.add
        call $m41byte
        local.set $b3
        local.get $b0
        i32.const 240
        i32.eq
        local.get $b1
        i32.const 144
        i32.lt_u
        i32.and
        local.get $b0
        i32.const 244
        i32.eq
        local.get $b1
        i32.const 143
        i32.gt_u
        i32.and
        i32.or
        local.get $b1
        call $m41is_cont
        i32.eqz
        i32.or
        local.get $b2
        call $m41is_cont
        i32.eqz
        i32.or
        local.get $b3
        call $m41is_cont
        i32.eqz
        i32.or
        if
          i32.const 3
          local.get $j
          call $pack
          return
        end
        local.get $b0
        i32.const 7
        i32.and
        i32.const 18
        i32.shl
        local.get $b1
        i32.const 63
        i32.and
        i32.const 12
        i32.shl
        i32.or
        local.get $b2
        i32.const 63
        i32.and
        i32.const 6
        i32.shl
        i32.or
        local.get $b3
        i32.const 63
        i32.and
        i32.or
        i32.const 65536
        i32.sub
        local.set $cp

        local.get $out
        local.get $cap
        local.get $j
        i32.const 55296
        local.get $cp
        i32.const 10
        i32.shr_u
        i32.or
        call $encode_surrogate
        local.tee $tmp
        i32.const -1
        i32.eq
        if
          i32.const 2
          local.get $cap
          call $pack
          return
        end
        local.get $out
        local.get $cap
        local.get $tmp
        i32.const 56320
        local.get $cp
        i32.const 1023
        i32.and
        i32.or
        call $encode_surrogate
        local.tee $j
        i32.const -1
        i32.eq
        if
          i32.const 2
          local.get $cap
          call $pack
          return
        end
        local.get $i
        i32.const 4
        i32.add
        local.set $i
        br $again
      end

      i32.const 3
      local.get $j
      call $pack
      return
    )
    i32.const 0
    local.get $j
    call $pack)

  (func (export "cesu8_encode_utf8") (param $ptr i32) (param $len i32) (param $out i32) (param $cap i32) (result i64)
    local.get $ptr
    local.get $len
    local.get $out
    local.get $cap
    i32.const 0
    call $encode)

  (func (export "mutf8_encode_utf8") (param $ptr i32) (param $len i32) (param $out i32) (param $cap i32) (result i64)
    local.get $ptr
    local.get $len
    local.get $out
    local.get $cap
    i32.const 1
    call $encode)

  (func (export "cesu8_decode_strict") (param $ptr i32) (param $len i32) (param $out i32) (param $cap i32) (result i64)
    local.get $ptr
    local.get $len
    local.get $out
    local.get $cap
    i32.const 0
    call $decode)

  (func (export "mutf8_decode_strict") (param $ptr i32) (param $len i32) (param $out i32) (param $cap i32) (result i64)
    local.get $ptr
    local.get $len
    local.get $out
    local.get $cap
    i32.const 1
    call $decode)
