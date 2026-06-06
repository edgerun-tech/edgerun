(module
  (import "edgerun" "load8_u" (func $m190byte (param i32 i32) (result i32)))
  (import "edgerun" "is_cont" (func $m190is_cont (param i32) (result i32)))
  (import "edgerun" "pack" (func $pack (param i32 i32) (result i64)))
  (import "edgerun" "lo" (func $lo (param i64) (result i32)))
  (import "edgerun" "hi" (func $hi (param i64) (result i32)))
  (import "edgerun" "is_digit" (func $is_digit (param i32) (result i32)))
  (import "edgerun" "to_lower" (func $to_lower (param i32) (result i32)))
  (import "edgerun" "to_upper" (func $to_upper (param i32) (result i32)))
  (memory (export "memory") 1)

  ;; Status values: 0 ok, 2 output_short, 3 invalid, 5 incomplete.
  ;; utf8_scan out record: valid_up_to, error_len, suffix_len, expected_len.
  ;; error_len is 0 for incomplete suffixes.

  (func $write_scan (param $out i32) (param $valid i32) (param $err_len i32) (param $suffix i32) (param $expected i32)
    local.get $out
    local.get $valid
    i32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $err_len
    i32.store
    local.get $out
    i32.const 8
    i32.add
    local.get $suffix
    i32.store
    local.get $out
    i32.const 12
    i32.add
    local.get $expected
    i32.store)

  (func $utf8_scan (export "utf8_scan") (param $ptr i32) (param $len i32) (param $out i32) (result i32)
    (local $i i32)
    (local $c i32)
    (local $b1 i32)
    (local $b2 i32)
    (local $b3 i32)
    (local $remain i32)
    (loop $again
      local.get $i
      local.get $len
      i32.ge_u
      if
        local.get $out
        local.get $len
        i32.const 0
        i32.const 0
        i32.const 0
        call $write_scan
        i32.const 0
        return
      end
      local.get $ptr
      local.get $i
      call $m190byte
      local.set $c
      local.get $c
      i32.const 128
      i32.lt_u
      if
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $again
      end

      local.get $len
      local.get $i
      i32.sub
      local.set $remain

      ;; Two-byte sequence: C2..DF 80..BF.
      local.get $c
      i32.const 194
      i32.ge_u
      local.get $c
      i32.const 223
      i32.le_u
      i32.and
      if
        local.get $remain
        i32.const 2
        i32.lt_u
        if
          local.get $out
          local.get $i
          i32.const 0
          local.get $remain
          i32.const 2
          call $write_scan
          i32.const 5
          return
        end
        local.get $ptr
        local.get $i
        i32.const 1
        i32.add
        call $m190byte
        call $m190is_cont
        i32.eqz
        if
          local.get $out
          local.get $i
          i32.const 1
          i32.const 0
          i32.const 0
          call $write_scan
          i32.const 3
          return
        end
        local.get $i
        i32.const 2
        i32.add
        local.set $i
        br $again
      end

      ;; Three-byte sequence with overlong and surrogate exclusions.
      local.get $c
      i32.const 224
      i32.ge_u
      local.get $c
      i32.const 239
      i32.le_u
      i32.and
      if
        local.get $remain
        i32.const 3
        i32.lt_u
        if
          local.get $out
          local.get $i
          i32.const 0
          local.get $remain
          i32.const 3
          call $write_scan
          i32.const 5
          return
        end
        local.get $ptr
        local.get $i
        i32.const 1
        i32.add
        call $m190byte
        local.set $b1
        local.get $ptr
        local.get $i
        i32.const 2
        i32.add
        call $m190byte
        local.set $b2
        local.get $c
        i32.const 224
        i32.eq
        if
          local.get $b1
          i32.const 160
          i32.ge_u
          local.get $b1
          i32.const 191
          i32.le_u
          i32.and
          i32.eqz
          if
            local.get $out
            local.get $i
            i32.const 1
            i32.const 0
            i32.const 0
            call $write_scan
            i32.const 3
            return
          end
        else
          local.get $c
          i32.const 237
          i32.eq
          if
            local.get $b1
            i32.const 128
            i32.ge_u
            local.get $b1
            i32.const 159
            i32.le_u
            i32.and
            i32.eqz
            if
              local.get $out
              local.get $i
              i32.const 1
              i32.const 0
              i32.const 0
              call $write_scan
              i32.const 3
              return
            end
          else
            local.get $b1
            call $m190is_cont
            i32.eqz
            if
              local.get $out
              local.get $i
              i32.const 1
              i32.const 0
              i32.const 0
              call $write_scan
              i32.const 3
              return
            end
          end
        end
        local.get $b2
        call $m190is_cont
        i32.eqz
        if
          local.get $out
          local.get $i
          i32.const 1
          i32.const 0
          i32.const 0
          call $write_scan
          i32.const 3
          return
        end
        local.get $i
        i32.const 3
        i32.add
        local.set $i
        br $again
      end

      ;; Four-byte sequence for U+10000..U+10FFFF.
      local.get $c
      i32.const 240
      i32.ge_u
      local.get $c
      i32.const 244
      i32.le_u
      i32.and
      if
        local.get $remain
        i32.const 4
        i32.lt_u
        if
          local.get $out
          local.get $i
          i32.const 0
          local.get $remain
          i32.const 4
          call $write_scan
          i32.const 5
          return
        end
        local.get $ptr
        local.get $i
        i32.const 1
        i32.add
        call $m190byte
        local.set $b1
        local.get $ptr
        local.get $i
        i32.const 2
        i32.add
        call $m190byte
        local.set $b2
        local.get $ptr
        local.get $i
        i32.const 3
        i32.add
        call $m190byte
        local.set $b3
        local.get $c
        i32.const 240
        i32.eq
        if
          local.get $b1
          i32.const 144
          i32.ge_u
          local.get $b1
          i32.const 191
          i32.le_u
          i32.and
          i32.eqz
          if
            local.get $out
            local.get $i
            i32.const 1
            i32.const 0
            i32.const 0
            call $write_scan
            i32.const 3
            return
          end
        else
          local.get $c
          i32.const 244
          i32.eq
          if
            local.get $b1
            i32.const 128
            i32.ge_u
            local.get $b1
            i32.const 143
            i32.le_u
            i32.and
            i32.eqz
            if
              local.get $out
              local.get $i
              i32.const 1
              i32.const 0
              i32.const 0
              call $write_scan
              i32.const 3
              return
            end
          else
            local.get $b1
            call $m190is_cont
            i32.eqz
            if
              local.get $out
              local.get $i
              i32.const 1
              i32.const 0
              i32.const 0
              call $write_scan
              i32.const 3
              return
            end
          end
        end
        local.get $b2
        call $m190is_cont
        local.get $b3
        call $m190is_cont
        i32.and
        i32.eqz
        if
          local.get $out
          local.get $i
          i32.const 1
          i32.const 0
          i32.const 0
          call $write_scan
          i32.const 3
          return
        end
        local.get $i
        i32.const 4
        i32.add
        local.set $i
        br $again
      end

      local.get $out
      local.get $i
      i32.const 1
      i32.const 0
      i32.const 0
      call $write_scan
      i32.const 3
      return)
    i32.const 0)

  (func $copy_bytes (param $src i32) (param $len i32) (param $dst i32)
    (local $i i32)
    (loop $copy
      local.get $i
      local.get $len
      i32.lt_u
      if
        local.get $dst
        local.get $i
        i32.add
        local.get $src
        local.get $i
        i32.add
        i32.load8_u
        i32.store8
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $copy
      end))

  (func $write_replacement (param $out_ptr i32) (param $written i32)
    local.get $out_ptr
    local.get $written
    i32.add
    i32.const 239
    i32.store8
    local.get $out_ptr
    local.get $written
    i32.add
    i32.const 1
    i32.add
    i32.const 191
    i32.store8
    local.get $out_ptr
    local.get $written
    i32.add
    i32.const 2
    i32.add
    i32.const 189
    i32.store8)

  (func (export "utf8_lossy_repair")
    (param $in_ptr i32) (param $in_len i32) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (local $pos i32)
    (local $written i32)
    (local $status i32)
    (local $valid i32)
    (local $err_len i32)
    (local $suffix_len i32)
    (loop $again
      local.get $pos
      local.get $in_len
      i32.ge_u
      if
        i32.const 0
        local.get $written
        call $pack
        return
      end
      local.get $in_ptr
      local.get $pos
      i32.add
      local.get $in_len
      local.get $pos
      i32.sub
      i32.const 0
      call $utf8_scan
      local.set $status
      i32.const 0
      i32.load
      local.set $valid
      local.get $written
      local.get $valid
      i32.add
      local.get $out_cap
      i32.gt_u
      if
        i32.const 2
        local.get $written
        call $pack
        return
      end
      local.get $in_ptr
      local.get $pos
      i32.add
      local.get $valid
      local.get $out_ptr
      local.get $written
      i32.add
      call $copy_bytes
      local.get $written
      local.get $valid
      i32.add
      local.set $written
      local.get $status
      i32.const 0
      i32.eq
      if
        i32.const 0
        local.get $written
        call $pack
        return
      end
      local.get $written
      i32.const 3
      i32.add
      local.get $out_cap
      i32.gt_u
      if
        i32.const 2
        local.get $written
        call $pack
        return
      end
      local.get $out_ptr
      local.get $written
      call $write_replacement
      local.get $written
      i32.const 3
      i32.add
      local.set $written
      local.get $status
      i32.const 5
      i32.eq
      if
        i32.const 0
        local.get $written
        call $pack
        return
      end
      i32.const 4
      i32.load
      local.set $err_len
      i32.const 8
      i32.load
      local.set $suffix_len
      local.get $pos
      local.get $valid
      i32.add
      local.get $err_len
      i32.add
      local.set $pos
      br $again)
    i32.const 0
    local.get $written
    call $pack)

  (func (export "simd_capabilities") (result i32)
    i32.const 1)

  (func (export "utf8_scan_simd") (param $ptr i32) (param $len i32) (param $out i32) (result i32)
    (local $i i32)
    (local $j i32)
    (local $v v128)
    (local $status i32)
    (local $m190byte i32)
    (block $tail
      (loop $skip
        local.get $i
        i32.const 16
        i32.add
        local.get $len
        i32.gt_u
        if
          br $tail
        end
        local.get $ptr
        local.get $i
        i32.add
        v128.load
        local.set $v
        local.get $v
        i32.const 128
        i8x16.splat
        i8x16.lt_u
        i8x16.all_true
        if
          local.get $i
          i32.const 16
          i32.add
          local.set $i
          br $skip
        end
        i32.const 0
        local.set $j
        (block $found
          (loop $find
            local.get $j
            i32.const 16
            i32.ge_u
            if
              br $found
            end
            local.get $ptr
            local.get $i
            i32.add
            local.get $j
            i32.add
            i32.load8_u
            local.set $m190byte
            local.get $m190byte
            i32.const 128
            i32.ge_u
            if
              br $found
            end
            local.get $j
            i32.const 1
            i32.add
            local.set $j
            br $find
          )
        )
        local.get $ptr
        local.get $i
        i32.add
        local.get $j
        i32.add
        local.get $len
        local.get $i
        i32.sub
        local.get $j
        i32.sub
        local.get $out
        call $utf8_scan
        local.set $status
        local.get $out
        local.get $out
        i32.load
        local.get $i
        i32.add
        local.get $j
        i32.add
        i32.store
        local.get $status
        return
      )
    )
    local.get $i
    local.get $len
    i32.lt_u
    if
      local.get $ptr
      local.get $i
      i32.add
      local.get $len
      local.get $i
      i32.sub
      local.get $out
      call $utf8_scan
      local.set $status
      local.get $out
      local.get $out
      i32.load
      local.get $i
      i32.add
      i32.store
      local.get $status
      return
    end
    local.get $out
    local.get $len
    i32.const 0
    i32.const 0
    i32.const 0
    call $write_scan
    i32.const 0)


  ;; Status values: 0 ok, 2 output_short, 3 invalid, 5 incomplete.
  ;; Packed result: low 32 bits status, high 32 bits output byte count.

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
      call $m190byte
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
        call $m190byte
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
        call $m190byte
        local.tee $b1
        call $m190is_cont
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
        call $m190byte
        local.set $b1
        local.get $ptr
        local.get $i
        i32.const 2
        i32.add
        call $m190byte
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
        call $m190is_cont
        i32.and
        local.get $b0
        i32.const 225
        i32.ge_u
        local.get $b0
        i32.const 236
        i32.le_u
        i32.and
        local.get $b1
        call $m190is_cont
        i32.and
        local.get $b2
        call $m190is_cont
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
        call $m190is_cont
        i32.and
        local.get $b2
        call $m190is_cont
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
        call $m190is_cont
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
          call $m190is_cont
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
          call $m190byte
          local.set $b3
          local.get $ptr
          local.get $i
          i32.const 4
          i32.add
          call $m190byte
          local.set $b4
          local.get $ptr
          local.get $i
          i32.const 5
          i32.add
          call $m190byte
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
          call $m190is_cont
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
      call $m190byte
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
        call $m190byte
        local.tee $b1
        call $m190is_cont
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
        call $m190byte
        local.set $b1
        local.get $ptr
        local.get $i
        i32.const 2
        i32.add
        call $m190byte
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
        call $m190is_cont
        i32.and
        local.get $b0
        i32.const 225
        i32.ge_u
        local.get $b0
        i32.const 236
        i32.le_u
        i32.and
        local.get $b1
        call $m190is_cont
        i32.and
        local.get $b2
        call $m190is_cont
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
        call $m190is_cont
        i32.and
        local.get $b2
        call $m190is_cont
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
        call $m190is_cont
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
        call $m190byte
        local.set $b1
        local.get $ptr
        local.get $i
        i32.const 2
        i32.add
        call $m190byte
        local.set $b2
        local.get $ptr
        local.get $i
        i32.const 3
        i32.add
        call $m190byte
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
        call $m190is_cont
        i32.eqz
        i32.or
        local.get $b2
        call $m190is_cont
        i32.eqz
        i32.or
        local.get $b3
        call $m190is_cont
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


  ;; CP1252 (Windows-1252) decode table.
  ;; Bytes 0x80-0x9F map to Unicode characters via a lookup table.
  ;; All other bytes 0x00-0x7F and 0xA0-0xFF pass through as-is (Latin-1).
  ;; Exports: cp1252_decode_byte(byte) -> i32 (Unicode codepoint)
  ;;          cp1252_decode_string(input_ptr, input_len, output_ptr) -> output_len
  ;; CP1252 decode table: 32 entries × 2 bytes (little-endian i16) at offset 0
  ;; Index = byte - 0x80
  (func $cp1252_decode_byte (export "cp1252_decode_byte") (param $byte i32) (result i32)
    (local $cp i32)
    local.get $byte i32.const 128 i32.ge_u
    if
      local.get $byte i32.const 160 i32.lt_u
      if
        local.get $byte i32.const 128 i32.sub i32.const 1 i32.shl
        i32.load16_u local.tee $cp
        if (result i32)
          local.get $cp
        else
          i32.const 63
        end
        return
      end
    end
    local.get $byte
  )

  (func (export "cp1252_decode_string") (param $in i32) (param $in_len i32) (param $out i32) (result i32)
    (local $i i32) (local $out_pos i32) (local $b i32) (local $cp i32)
    i32.const 0 local.set $i
    i32.const 0 local.set $out_pos
    block $done
    loop $loop
      local.get $i local.get $in_len i32.ge_u br_if $done
      local.get $in local.get $i i32.add i32.load8_u local.tee $b
      i32.eqz if
        local.get $i i32.const 1 i32.add local.set $i
        br $done
      end
      local.get $b call $cp1252_decode_byte local.set $cp
      local.get $out local.get $out_pos i32.add
      local.get $cp i32.const 0xff i32.and i32.store8
      local.get $cp i32.const 8 i32.shr_u i32.const 0xff i32.and
      if
        local.get $out local.get $out_pos i32.const 1 i32.add i32.add
        local.get $cp i32.const 8 i32.shr_u i32.store8
        local.get $out_pos i32.const 2 i32.add local.set $out_pos
      else
        local.get $out_pos i32.const 1 i32.add local.set $out_pos
      end
      local.get $i i32.const 1 i32.add local.set $i
      br $loop
    end
    end
    local.get $out_pos
  )


  ;; ASCII string case conversion.
  ;;
  ;; Exports:
  ;;   to_title_case(in_ptr, in_len, out_ptr) -> out_len
  ;;     (SNAKE_CASE -> "Title Case", first letter of each '_'-separated word capitalized)
  ;;   to_upper_ascii(in_ptr, in_len, out_ptr) -> out_len
  ;;   to_lower_ascii(in_ptr, in_len, out_ptr) -> out_len
  (func $m39is_upper (param $c i32) (result i32)
    local.get $c i32.const 65 i32.ge_u
    local.get $c i32.const 90 i32.le_u i32.and
  )

  (func (export "to_lower_ascii") (param $in i32) (param $len i32) (param $out i32) (result i32)
    (local $i i32)
    i32.const 0 local.set $i
    block $done
    loop $loop
      local.get $i local.get $len i32.ge_u br_if $done
      local.get $out local.get $i i32.add
      local.get $in local.get $i i32.add i32.load8_u call $to_lower i32.store8
      local.get $i i32.const 1 i32.add local.set $i
      br $loop
    end
    end
    local.get $len
  )

  (func (export "to_upper_ascii") (param $in i32) (param $len i32) (param $out i32) (result i32)
    (local $i i32)
    i32.const 0 local.set $i
    block $done
    loop $loop
      local.get $i local.get $len i32.ge_u br_if $done
      local.get $out local.get $i i32.add
      local.get $in local.get $i i32.add i32.load8_u call $to_upper i32.store8
      local.get $i i32.const 1 i32.add local.set $i
      br $loop
    end
    end
    local.get $len
  )

  (func (export "to_title_case") (param $in i32) (param $len i32) (param $out i32) (result i32)
    (local $i i32) (local $o i32) (local $c i32) (local $word_start i32)
    i32.const 0 local.set $i
    i32.const 0 local.set $o
    i32.const 1 local.set $word_start
    block $done
    loop $loop
      local.get $i local.get $len i32.ge_u br_if $done
      local.get $in local.get $i i32.add i32.load8_u local.set $c
      local.get $c i32.const 95 i32.eq  ;; '_'
      if
        local.get $word_start if else
          local.get $out local.get $o i32.add i32.const 32 i32.store8  ;; space
          local.get $o i32.const 1 i32.add local.set $o
        end
        i32.const 1 local.set $word_start
        local.get $i i32.const 1 i32.add local.set $i
        br $loop
      end
      local.get $word_start
      if
        local.get $out local.get $o i32.add local.get $c call $to_upper i32.store8
        i32.const 0 local.set $word_start
      else
        local.get $out local.get $o i32.add local.get $c call $to_lower i32.store8
      end
      local.get $o i32.const 1 i32.add local.set $o
      local.get $i i32.const 1 i32.add local.set $i
      br $loop
    end
    end
    local.get $o
  )

  (data (i32.const 0)
    "\ac\20"  ;; 0x80  € U+20AC
    "\00\00"  ;; 0x81  (unused)
    "\1a\20"  ;; 0x82  ‚ U+201A
    "\92\01"  ;; 0x83  ƒ U+0192
    "\1e\20"  ;; 0x84  „ U+201E
    "\26\20"  ;; 0x85  … U+2026
    "\20\20"  ;; 0x86  † U+2020
    "\21\20"  ;; 0x87  ‡ U+2021
    "\c6\02"  ;; 0x88  ˆ U+02C6
    "\30\20"  ;; 0x89  ‰ U+2030
    "\60\01"  ;; 0x8A  Š U+0160
    "\39\20"  ;; 0x8B  ‹ U+2039
    "\52\01"  ;; 0x8C  Œ U+0152
    "\00\00"  ;; 0x8D  (unused)
    "\7d\01"  ;; 0x8E  Ž U+017D
    "\00\00"  ;; 0x8F  (unused)
    "\00\00"  ;; 0x90  (unused)
    "\18\20"  ;; 0x91  ' U+2018
    "\19\20"  ;; 0x92  ' U+2019
    "\1c\20"  ;; 0x93  " U+201C
    "\1d\20"  ;; 0x94  " U+201D
    "\22\20"  ;; 0x95  • U+2022
    "\13\20"  ;; 0x96  – U+2013
    "\14\20"  ;; 0x97  — U+2014
    "\dc\02"  ;; 0x98  ˜ U+02DC
    "\22\21"  ;; 0x99  ™ U+2122
    "\61\01"  ;; 0x9A  š U+0161
    "\3a\20"  ;; 0x9B  › U+203A
    "\53\01"  ;; 0x9C  œ U+0153
    "\00\00"  ;; 0x9D  (unused)
    "\7e\01"  ;; 0x9E  ž U+017E
    "\78\01"  ;; 0x9F  Ÿ U+0178
  )
)
