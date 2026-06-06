(module
  (import "edgerun-core" "memory" (memory 1))

  (global $m118TMP i32 (i32.const 65000))
  (global $m118TMP_END i32 (i32.const 65064))

  (func (export "proto_standard_id") (result i32)
    i32.const 300073)

  ;; Decimal integer formatting.
  ;; Return value is bytes written on success, or -2 when out_cap is too small.
  ;; integer_format_u128 receives little-endian split limbs: value_lo, value_hi.

  (func $copy_tmp (param $len i32) (param $out i32) (param $cap i32) (result i32)
    (local $i i32)
    local.get $cap
    local.get $len
    i32.lt_u
    if
      i32.const -2
      return
    end
    loop $again
      local.get $i
      local.get $len
      i32.lt_u
      if
        local.get $out
        local.get $i
        i32.add
        global.get $m118TMP_END
        local.get $len
        i32.sub
        local.get $i
        i32.add
        i32.load8_u
        i32.store8
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $again
      end
    end
    local.get $len)

  (func $u64_digits_to_tmp (param $value i64) (result i32)
    (local $pos i32)
    (local $q i64)
    (local $r i64)
    i32.const 64
    local.set $pos
    local.get $value
    i64.eqz
    if
      global.get $m118TMP
      i32.const 63
      i32.add
      i32.const 48
      i32.store8
      i32.const 1
      return
    end
    loop $again
      local.get $value
      i64.eqz
      i32.eqz
      if
        local.get $value
        i64.const 10
        i64.div_u
        local.set $q
        local.get $value
        i64.const 10
        i64.rem_u
        local.set $r
        local.get $pos
        i32.const 1
        i32.sub
        local.set $pos
        global.get $m118TMP
        local.get $pos
        i32.add
        local.get $r
        i32.wrap_i64
        i32.const 48
        i32.add
        i32.store8
        local.get $q
        local.set $value
        br $again
      end
    end
    i32.const 64
    local.get $pos
    i32.sub)

  (func $m118u128_digits_to_tmp (param $lo i64) (param $hi i64) (result i32)
    (local $pos i32)
    (local $qlo i64)
    (local $qhi i64)
    (local $rem i64)
    (local $bit i32)
    (local $bitval i64)
    i32.const 64
    local.set $pos
    local.get $lo
    i64.eqz
    local.get $hi
    i64.eqz
    i32.and
    if
      global.get $m118TMP
      i32.const 63
      i32.add
      i32.const 48
      i32.store8
      i32.const 1
      return
    end

    loop $digits
      local.get $lo
      i64.eqz
      local.get $hi
      i64.eqz
      i32.and
      i32.eqz
      if
        i64.const 0
        local.set $qlo
        i64.const 0
        local.set $qhi
        i64.const 0
        local.set $rem
        i32.const 127
        local.set $bit
        loop $divide
          local.get $bit
          i32.const 64
          i32.ge_u
          if
            local.get $hi
            local.get $bit
            i32.const 64
            i32.sub
            i64.extend_i32_u
            i64.shr_u
            i64.const 1
            i64.and
            local.set $bitval
          else
            local.get $lo
            local.get $bit
            i64.extend_i32_u
            i64.shr_u
            i64.const 1
            i64.and
            local.set $bitval
          end

          local.get $rem
          i64.const 1
          i64.shl
          local.get $bitval
          i64.or
          local.set $rem

          local.get $rem
          i64.const 10
          i64.ge_u
          if
            local.get $rem
            i64.const 10
            i64.sub
            local.set $rem
            local.get $bit
            i32.const 64
            i32.ge_u
            if
              local.get $qhi
              i64.const 1
              local.get $bit
              i32.const 64
              i32.sub
              i64.extend_i32_u
              i64.shl
              i64.or
              local.set $qhi
            else
              local.get $qlo
              i64.const 1
              local.get $bit
              i64.extend_i32_u
              i64.shl
              i64.or
              local.set $qlo
            end
          end

          local.get $bit
          i32.eqz
          if
          else
            local.get $bit
            i32.const 1
            i32.sub
            local.set $bit
            br $divide
          end
        end

        local.get $pos
        i32.const 1
        i32.sub
        local.set $pos
        global.get $m118TMP
        local.get $pos
        i32.add
        local.get $rem
        i32.wrap_i64
        i32.const 48
        i32.add
        i32.store8
        local.get $qlo
        local.set $lo
        local.get $qhi
        local.set $hi
        br $digits
      end
    end
    i32.const 64
    local.get $pos
    i32.sub)

  (func (export "integer_format_u64")
    (param $value i64) (param $out i32) (param $cap i32) (result i32)
    local.get $value
    call $u64_digits_to_tmp
    local.get $out
    local.get $cap
    call $copy_tmp)

  (func (export "integer_format_i64")
    (param $value i64) (param $out i32) (param $cap i32) (result i32)
    (local $negative i32)
    (local $len i32)
    (local $mag i64)
    local.get $value
    i64.const 0
    i64.lt_s
    local.set $negative
    local.get $negative
    if
      i64.const 0
      local.get $value
      i64.sub
      local.set $mag
    else
      local.get $value
      local.set $mag
    end
    local.get $mag
    call $u64_digits_to_tmp
    local.set $len
    local.get $negative
    if
      local.get $len
      i32.const 1
      i32.add
      local.set $len
      global.get $m118TMP_END
      local.get $len
      i32.sub
      i32.const 45
      i32.store8
    end
    local.get $len
    local.get $out
    local.get $cap
    call $copy_tmp)

  (func (export "integer_format_u128")
    (param $value_lo i64) (param $value_hi i64) (param $out i32) (param $cap i32)
    (result i32)
    local.get $value_lo
    local.get $value_hi
    call $m118u128_digits_to_tmp
    local.get $out
    local.get $cap
    call $copy_tmp)
)