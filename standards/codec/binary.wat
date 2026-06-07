(func $m100hex_value (param $c i32) (result i32)
    local.get $c
    i32.const 48
    i32.ge_u
    local.get $c
    i32.const 57
    i32.le_u
    i32.and
    if (result i32)
      local.get $c
      i32.const 48
      i32.sub
    else
      local.get $c
      i32.const 97
      i32.ge_u
      local.get $c
      i32.const 102
      i32.le_u
      i32.and
      if (result i32)
        local.get $c
        i32.const 87
        i32.sub
      else
        local.get $c
        i32.const 65
        i32.ge_u
        local.get $c
        i32.const 70
        i32.le_u
        i32.and
        if (result i32)
          local.get $c
          i32.const 55
          i32.sub
        else
          i32.const -1
        end
      end
    end)

  (func $has_prefix (param $ptr i32) (param $len i32) (result i32)
    local.get $len
    i32.const 2
    i32.ge_u
    if (result i32)
      local.get $ptr
      i32.load8_u
      i32.const 48
      i32.eq
      local.get $ptr
      i32.const 1
      i32.add
      i32.load8_u
      i32.const 120
      i32.eq
      local.get $ptr
      i32.const 1
      i32.add
      i32.load8_u
      i32.const 88
      i32.eq
      i32.or
      i32.and
    else
      i32.const 0
    end)

  (func $hex_decode_compat (export "hex_decode_compat")
    (param $in_ptr i32) (param $in_len i32) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (local $pos i32)
    (local $digits i32)
    (local $needed i32)
    (local $written i32)
    (local $hi i32)
    (local $lo i32)

    local.get $in_ptr
    local.get $in_len
    call $has_prefix
    if
      i32.const 2
      local.set $pos
    end

    local.get $in_len
    local.get $pos
    i32.sub
    local.set $digits

    local.get $digits
    i32.const 1
    i32.add
    i32.const 1
    i32.shr_u
    local.set $needed

    local.get $needed
    local.get $out_cap
    i32.gt_u
    if
      i32.const 2
      i32.const 0
      call $pack
      return
    end

    local.get $digits
    i32.const 1
    i32.and
    if
      local.get $in_ptr
      local.get $pos
      i32.add
      i32.load8_u
      call $m100hex_value
      local.tee $lo
      i32.const 0
      i32.lt_s
      if
        i32.const 3
        i32.const 0
        call $pack
        return
      end
      local.get $out_ptr
      local.get $lo
      i32.store8
      local.get $written
      i32.const 1
      i32.add
      local.set $written
      local.get $pos
      i32.const 1
      i32.add
      local.set $pos
    end

    (loop $pairs
      local.get $pos
      local.get $in_len
      i32.lt_u
      if
        local.get $in_ptr
        local.get $pos
        i32.add
        i32.load8_u
        call $m100hex_value
        local.tee $hi
        i32.const 0
        i32.lt_s
        if
          i32.const 3
          i32.const 0
          call $pack
          return
        end
        local.get $in_ptr
        local.get $pos
        i32.const 1
        i32.add
        i32.add
        i32.load8_u
        call $m100hex_value
        local.tee $lo
        i32.const 0
        i32.lt_s
        if
          i32.const 3
          i32.const 0
          call $pack
          return
        end
        local.get $out_ptr
        local.get $written
        i32.add
        local.get $hi
        i32.const 4
        i32.shl
        local.get $lo
        i32.or
        i32.store8
        local.get $written
        i32.const 1
        i32.add
        local.set $written
        local.get $pos
        i32.const 2
        i32.add
        local.set $pos
        br $pairs
      end)

    i32.const 0
    local.get $written
    call $pack)

  (func (export "hex_parse_u32")
    (param $in_ptr i32) (param $in_len i32)
    (result i64)
    (local $pos i32)
    (local $value i64)
    (local $nibble i32)

    local.get $in_ptr
    local.get $in_len
    call $has_prefix
    if
      i32.const 2
      local.set $pos
    end

    local.get $pos
    local.get $in_len
    i32.ge_u
    if
      i32.const 3
      i32.const 0
      call $pack
      return
    end

    (loop $digits
      local.get $pos
      local.get $in_len
      i32.lt_u
      if
        local.get $in_ptr
        local.get $pos
        i32.add
        i32.load8_u
        call $m100hex_value
        local.tee $nibble
        i32.const 0
        i32.lt_s
        if
          i32.const 3
          i32.const 0
          call $pack
          return
        end
        local.get $value
        i64.const 4
        i64.shl
        local.get $nibble
        i64.extend_i32_u
        i64.or
        local.set $value
        local.get $value
        i64.const 4294967295
        i64.gt_u
        if
          i32.const 3
          i32.const 0
          call $pack
          return
        end
        local.get $pos
        i32.const 1
        i32.add
        local.set $pos
        br $digits
      end)

    i32.const 0
    local.get $value
    i32.wrap_i64
    call $pack)

  (func $parse_mac_into
    (param $ptr i32) (param $len i32) (param $out i32) (param $reverse i32)
    (result i32)
    (local $i i32)
    (local $out_index i32)
    (local $hi i32)
    (local $lo i32)

    local.get $len
    i32.const 17
    i32.ne
    if
      i32.const 3
      return
    end

    (loop $octets
      local.get $i
      i32.const 6
      i32.lt_u
      if
        local.get $ptr
        local.get $i
        i32.const 3
        i32.mul
        i32.add
        i32.load8_u
        call $m100hex_value
        local.tee $hi
        i32.const 0
        i32.lt_s
        if
          i32.const 3
          return
        end

        local.get $ptr
        local.get $i
        i32.const 3
        i32.mul
        i32.const 1
        i32.add
        i32.add
        i32.load8_u
        call $m100hex_value
        local.tee $lo
        i32.const 0
        i32.lt_s
        if
          i32.const 3
          return
        end

        local.get $i
        i32.const 5
        i32.lt_u
        if
          local.get $ptr
          local.get $i
          i32.const 3
          i32.mul
          i32.const 2
          i32.add
          i32.add
          i32.load8_u
          i32.const 58
          i32.ne
          if
            i32.const 3
            return
          end
        end

        local.get $reverse
        if (result i32)
          i32.const 5
          local.get $i
          i32.sub
        else
          local.get $i
        end
        local.set $out_index

        local.get $out
        local.get $out_index
        i32.add
        local.get $hi
        i32.const 4
        i32.shl
        local.get $lo
        i32.or
        i32.store8

        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $octets
      end)

    i32.const 0)

  (func $mac_scan (export "mac_scan") (param $ptr i32) (param $len i32) (param $out i32) (result i32)
    local.get $ptr
    local.get $len
    local.get $out
    i32.const 0
    call $parse_mac_into)

  (func $bdaddr_scan (export "bdaddr_scan") (param $ptr i32) (param $len i32) (param $out i32) (result i32)
    local.get $ptr
    local.get $len
    local.get $out
    i32.const 1
    call $parse_mac_into)


  ;; Status values: 0 ok, 1 input_short, 2 output_short, 3 invalid, 4 overflow.
  ;; Read out record: u32 value_lo, u32 value_hi. Signed reads are sign-extended.
  ;; Endian values: 0 little-endian, 1 big-endian. Width values: 2, 3, 4, 8.
  ;; endian_read(ptr,len,offset,width,endian,signed,out) -> status
  ;; endian_write(value_lo,value_hi,width,endian,out_ptr,out_cap) -> packed i64
  ;; Packed i64 return: low u32 status, high u32 bytes_written.

  (func $valid_width (param $width i32) (result i32)
    local.get $width
    i32.const 2
    i32.eq
    local.get $width
    i32.const 3
    i32.eq
    i32.or
    local.get $width
    i32.const 4
    i32.eq
    i32.or
    local.get $width
    i32.const 8
    i32.eq
    i32.or)

  (func $read_be (param $addr i32) (param $width i32) (result i64)
    (local $i i32)
    (local $value i64)
    loop $again
      local.get $i
      local.get $width
      i32.lt_u
      if
        local.get $value
        i64.const 8
        i64.shl
        local.get $addr
        local.get $i
        i32.add
        i64.load8_u
        i64.or
        local.set $value
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $again
      end
    end
    local.get $value)

  (func $read_le (param $addr i32) (param $width i32) (result i64)
    (local $i i32)
    (local $shift i64)
    (local $value i64)
    loop $again
      local.get $i
      local.get $width
      i32.lt_u
      if
        local.get $addr
        local.get $i
        i32.add
        i64.load8_u
        local.get $shift
        i64.shl
        local.get $value
        i64.or
        local.set $value
        local.get $shift
        i64.const 8
        i64.add
        local.set $shift
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $again
      end
    end
    local.get $value)

  (func $sign_extend (param $value i64) (param $width i32) (result i64)
    local.get $width
    i32.const 2
    i32.eq
    if (result i64)
      local.get $value
      i64.const 48
      i64.shl
      i64.const 48
      i64.shr_s
    else
      local.get $width
      i32.const 4
      i32.eq
      if (result i64)
        local.get $value
        i64.const 32
        i64.shl
        i64.const 32
        i64.shr_s
      else
        local.get $value
      end
    end)

  (func $store_u64 (param $out i32) (param $value i64)
    local.get $out
    local.get $value
    i32.wrap_i64
    i32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $value
    i64.const 32
    i64.shr_u
    i32.wrap_i64
    i32.store)

  (func $endian_read (export "endian_read")
    (param $ptr i32) (param $len i32) (param $offset i32)
    (param $width i32) (param $endian i32) (param $signed i32) (param $out i32)
    (result i32)
    (local $addr i32)
    (local $value i64)

    local.get $out
    i32.eqz
    if
      i32.const 2
      return
    end

    local.get $width
    call $valid_width
    i32.eqz
    if
      i32.const 3
      return
    end

    local.get $endian
    i32.const 1
    i32.gt_u
    if
      i32.const 3
      return
    end

    local.get $signed
    i32.const 1
    i32.gt_u
    if
      i32.const 3
      return
    end

    local.get $signed
    local.get $width
    i32.const 3
    i32.eq
    i32.and
    if
      i32.const 3
      return
    end

    local.get $offset
    local.get $len
    i32.gt_u
    if
      i32.const 1
      return
    end

    local.get $len
    local.get $offset
    i32.sub
    local.get $width
    i32.lt_u
    if
      i32.const 1
      return
    end

    local.get $ptr
    local.get $offset
    i32.add
    local.set $addr

    local.get $endian
    i32.eqz
    if (result i64)
      local.get $addr
      local.get $width
      call $read_le
    else
      local.get $addr
      local.get $width
      call $read_be
    end
    local.set $value

    local.get $signed
    if
      local.get $value
      local.get $width
      call $sign_extend
      local.set $value
    end

    local.get $out
    local.get $value
    call $store_u64
    i32.const 0)

  (func $write_le (param $value i64) (param $width i32) (param $out i32)
    (local $i i32)
    (local $v i64)
    local.get $value
    local.set $v
    loop $again
      local.get $i
      local.get $width
      i32.lt_u
      if
        local.get $out
        local.get $i
        i32.add
        local.get $v
        i32.wrap_i64
        i32.store8
        local.get $v
        i64.const 8
        i64.shr_u
        local.set $v
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $again
      end
    end)

  (func $write_be (param $value i64) (param $width i32) (param $out i32)
    (local $i i32)
    (local $shift i64)
    local.get $width
    i32.const 1
    i32.sub
    i64.extend_i32_u
    i64.const 8
    i64.mul
    local.set $shift
    loop $again
      local.get $i
      local.get $width
      i32.lt_u
      if
        local.get $out
        local.get $i
        i32.add
        local.get $value
        local.get $shift
        i64.shr_u
        i32.wrap_i64
        i32.store8
        local.get $shift
        i64.const 8
        i64.sub
        local.set $shift
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $again
      end
    end)

  (func $endian_write (export "endian_write")
    (param $value_lo i32) (param $value_hi i32) (param $width i32)
    (param $endian i32) (param $out i32) (param $out_cap i32)
    (result i64)
    (local $value i64)

    local.get $width
    call $valid_width
    i32.eqz
    if
      i32.const 3
      i32.const 0
      call $pack
      return
    end

    local.get $endian
    i32.const 1
    i32.gt_u
    if
      i32.const 3
      i32.const 0
      call $pack
      return
    end

    local.get $out
    i32.eqz
    local.get $out_cap
    local.get $width
    i32.lt_u
    i32.or
    if
      i32.const 2
      i32.const 0
      call $pack
      return
    end

    local.get $value_hi
    i64.extend_i32_u
    i64.const 32
    i64.shl
    local.get $value_lo
    i64.extend_i32_u
    i64.or
    local.set $value

    local.get $width
    i32.const 2
    i32.eq
    if
      local.get $value_hi
      i32.eqz
      i32.eqz
      local.get $value_lo
      i32.const 0xffff
      i32.gt_u
      i32.or
      if
        i32.const 4
        i32.const 0
        call $pack
        return
      end
    end

    local.get $width
    i32.const 3
    i32.eq
    if
      local.get $value_hi
      i32.eqz
      i32.eqz
      local.get $value_lo
      i32.const 0x00ffffff
      i32.gt_u
      i32.or
      if
        i32.const 4
        i32.const 0
        call $pack
        return
      end
    end

    local.get $width
    i32.const 4
    i32.eq
    if
      local.get $value_hi
      i32.eqz
      local.get $value_hi
      i32.const -1
      i32.eq
      i32.or
      i32.eqz
      if
        i32.const 4
        i32.const 0
        call $pack
        return
      end
    end

    local.get $endian
    i32.eqz
    if
      local.get $value
      local.get $width
      local.get $out
      call $write_le
    else
      local.get $value
      local.get $width
      local.get $out
      call $write_be
    end

    i32.const 0
    local.get $width
    call $pack)


  (global $m118TMP i32 (i32.const 65000))
  (global $m118TMP_END i32 (i32.const 65064))

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

  (func $integer_format_u64 (export "integer_format_u64")
    (param $value i64) (param $out i32) (param $cap i32) (result i32)
    local.get $value
    call $u64_digits_to_tmp
    local.get $out
    local.get $cap
    call $copy_tmp)

  (func $integer_format_i64 (export "integer_format_i64")
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

  (func $integer_format_u128 (export "integer_format_u128")
    (param $value_lo i64) (param $value_hi i64) (param $out i32) (param $cap i32)
    (result i32)
    local.get $value_lo
    local.get $value_hi
    call $m118u128_digits_to_tmp
    local.get $out
    local.get $cap
    call $copy_tmp)


  ;; SI-suffixed number formatting (K/M/B) and comma-delimited format.
  ;;
  ;; Exports:
  ;;   format_suffix(val, out_ptr) -> out_len  (e.g. 1234567 -> "1.23M")
  ;;   format_number(val, out_ptr) -> out_len  (e.g. 1234567 -> "1,234,567")
  ;;   parse_suffix(in_ptr, in_len) -> i64     (e.g. "1.23M" -> 1230000)
  (func $format_number (export "format_number") (param $val i32) (param $out i32) (result i32)
    (local $i i32) (local $o i32) (local $buf i32) (local $neg i32) (local $d i32) (local $n i32)
    local.get $val i32.const 0 i32.lt_s
    if
      i32.const 1 local.set $neg
      i32.const 0 local.get $val i32.sub local.set $val
    end
    i32.const 256 local.set $buf
    i32.const 0 local.set $i
    block $done_digits
    loop $digits
      local.get $val i32.const 10 i32.div_u local.set $n
      local.get $val local.get $n i32.const 10 i32.mul i32.sub local.set $d
      local.get $buf local.get $i i32.add local.get $d i32.const 48 i32.add i32.store8
      local.get $i i32.const 1 i32.add local.set $i
      local.get $n local.set $val
      local.get $val br_if $digits
    end
    end
    i32.const 0 local.set $o
    local.get $neg if
      local.get $out local.get $o i32.add i32.const 45 i32.store8
      local.get $o i32.const 1 i32.add local.set $o
    end
    block $done_out
    loop $out_loop
      local.get $i i32.const 0 i32.le_s br_if $done_out
      local.get $i i32.const 1 i32.sub local.set $i
      local.get $o i32.const 3 i32.gt_u
      local.get $i i32.const 0 i32.gt_s i32.and
      local.get $i i32.const 3 i32.rem_u i32.eqz i32.and
      if
        local.get $out local.get $o i32.add i32.const 44 i32.store8
        local.get $o i32.const 1 i32.add local.set $o
      end
      local.get $out local.get $o i32.add
      local.get $buf local.get $i i32.add i32.load8_u i32.store8
      local.get $o i32.const 1 i32.add local.set $o
      br $out_loop
    end
    end
    local.get $o
  )

  (func $write_frac (param $out_ptr i32) (param $num i32) (param $div i32)
    (local $d i32)
    local.get $num local.get $div i32.const 100 i32.div_u i32.div_u
    local.tee $d
    if
      local.get $out_ptr i32.const 46 i32.store8  ;; '.'
      local.get $d i32.const 10 i32.lt_u
      if
        local.get $out_ptr i32.const 1 i32.add i32.const 48 i32.store8  ;; '0'
        local.get $out_ptr i32.const 2 i32.add local.get $d i32.const 48 i32.add i32.store8
      else
        local.get $out_ptr i32.const 1 i32.add
        local.get $d i32.const 10 i32.div_u i32.const 48 i32.add i32.store8
        local.get $out_ptr i32.const 2 i32.add
        local.get $d i32.const 10 i32.rem_u i32.const 48 i32.add i32.store8
      end
    end
  )

  (func $format_suffix (export "format_suffix") (param $val i32) (param $out i32) (result i32)
    (local $abs i32) (local $suffix i32) (local $div i32) (local $d i32)
    (local $o i32) (local $neg i32)
    local.get $val i32.const 0 i32.lt_s
    if
      i32.const 1 local.set $neg
      i32.const 0 local.get $val i32.sub local.set $abs
    else
      local.get $val local.set $abs
    end
    local.get $abs i32.const 10000 i32.lt_u
    if
      local.get $val local.get $out call $format_number return
    end
    i32.const 0 local.set $suffix
    local.get $abs i32.const 1000000000 i32.ge_u
    if
      i32.const 3 local.set $suffix
      i32.const 1000000000 local.set $div
    else
      local.get $abs i32.const 1000000 i32.ge_u
      if
        i32.const 2 local.set $suffix
        i32.const 1000000 local.set $div
      else
        i32.const 1 local.set $suffix
        i32.const 1000 local.set $div
      end
    end
    i32.const 0 local.set $o
    local.get $neg if
      local.get $out local.get $o i32.add i32.const 45 i32.store8
      local.get $o i32.const 1 i32.add local.set $o
    end
    local.get $abs local.get $div i32.div_u local.set $d
    local.get $d i32.const 100 i32.lt_u
    if
      local.get $out local.get $o i32.add local.get $d i32.const 48 i32.add i32.store8
      local.get $o i32.const 1 i32.add local.set $o
      local.get $abs local.get $d local.get $div i32.mul i32.sub
      local.get $out local.get $o i32.add local.get $div call $write_frac
    else
      local.get $out local.get $o i32.add
      local.get $d i32.const 10 i32.div_u i32.const 48 i32.add i32.store8
      local.get $o i32.const 1 i32.add local.set $o
      local.get $d i32.const 10 i32.rem_u
      local.tee $d
      if
        local.get $out local.get $o i32.add local.get $d i32.const 48 i32.add i32.store8
        local.get $o i32.const 1 i32.add local.set $o
      end
      local.get $out local.get $o i32.add i32.const 46 i32.store8
      local.get $o i32.const 1 i32.add local.set $o
      local.get $abs local.get $d local.get $div i32.mul i32.sub
      local.get $div i32.const 10 i32.div_u i32.div_u
      local.set $d
      local.get $out local.get $o i32.add local.get $d i32.const 48 i32.add i32.store8
      local.get $o i32.const 1 i32.add local.set $o
    end
    local.get $suffix
    if (result i32)
      local.get $out local.get $o i32.add
      local.get $suffix
      i32.const 1 i32.eq if (result i32) i32.const 75 else
      local.get $suffix i32.const 2 i32.eq if (result i32) i32.const 77 else
      i32.const 66
      end end
      i32.store8
      local.get $o i32.const 1 i32.add
    else
      local.get $o
    end
  )

  (func $parse_suffix (export "parse_suffix") (param $in i32) (param $len i32) (result i64)
    (local $val i64) (local $i i32) (local $c i32) (local $neg i32)
    (local $decimal i64) (local $div i64) (local $mult i64)
    i64.const 0 local.set $val
    i64.const 1 local.set $div
    i64.const 1 local.set $mult
    i32.const 0 local.set $neg
    i32.const 0 local.set $i
    block $done
    loop $loop
      local.get $i local.get $len i32.ge_u br_if $done
      local.get $in local.get $i i32.add i32.load8_u local.set $c
      local.get $c i32.const 45 i32.eq
      if
        i32.const 1 local.set $neg
        local.get $i i32.const 1 i32.add local.set $i
        br $loop
      end
      local.get $c i32.const 44 i32.eq
      if
        local.get $i i32.const 1 i32.add local.set $i
        br $loop
      end
      local.get $c i32.const 46 i32.eq
      if
        i64.const 1 local.set $decimal
        local.get $i i32.const 1 i32.add local.set $i
        br $loop
      end
      local.get $c i32.const 48 i32.ge_u
      local.get $c i32.const 57 i32.le_u i32.and
      if
        local.get $decimal i64.const 0 i64.ne
        if
          local.get $div i64.const 10 i64.mul local.set $div
        end
        local.get $val i64.const 10 i64.mul
        local.get $c i32.const 48 i32.sub i64.extend_i32_s i64.add local.set $val
        local.get $i i32.const 1 i32.add local.set $i
        br $loop
      end
      br $done
    end
    end
    local.get $c i32.const 75 i32.eq
    if
      i64.const 1000 local.set $mult
    else
      local.get $c i32.const 77 i32.eq
      if
        i64.const 1000000 local.set $mult
      else
        local.get $c i32.const 66 i32.eq
        if
          i64.const 1000000000 local.set $mult
        end
      end
    end
    local.get $div i64.const 1 i64.gt_u
    if
      local.get $val local.get $div i64.div_u local.set $val
    end
    local.get $val local.get $mult i64.mul local.set $val
    local.get $neg if
      local.get $val i64.const -1 i64.mul local.set $val
    end
    local.get $val
  )


;; Status values: 0 ok, 1 input_short, 2 output_short, 3 invalid, 4 overflow.
  ;; Packed i64 return: low u32 status, high u32 next_offset or bytes_written.

  (func $m128read_u16_le (param $ptr i32) (result i32)
    local.get $ptr
    i32.load16_u)

  (func $m128read_u32_le (param $ptr i32) (result i32)
    local.get $ptr
    i32.load)

  (func $m128read_u32_be (param $ptr i32) (result i32)
    (i32.or
      (i32.or
        (i32.shl
          (i32.load8_u (local.get $ptr))
          (i32.const 24))
        (i32.shl
          (i32.load8_u (i32.add (local.get $ptr) (i32.const 1)))
          (i32.const 16)))
      (i32.or
        (i32.shl
          (i32.load8_u (i32.add (local.get $ptr) (i32.const 2)))
          (i32.const 8))
        (i32.load8_u (i32.add (local.get $ptr) (i32.const 3))))))

  (func $read_u64_le (param $ptr i32) (result i64)
    local.get $ptr
    i64.load)

  (func $store_span (param $out i32) (param $value_off i32) (param $value_len i32)
    local.get $out
    local.get $value_off
    i32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $value_len
    i32.store)

  (func $scan_i32
    (param $ptr i32) (param $len i32) (param $offset i32) (param $out i32)
    (param $prefix_len i32) (param $value_len i32)
    (result i64)
    (local $value_off i32)
    (local $remaining i32)
    (local $next_offset i32)

    local.get $out
    i32.eqz
    if
      i32.const 2
      local.get $offset
      call $pack
      return
    end

    local.get $offset
    local.get $len
    i32.gt_u
    if
      i32.const 1
      local.get $offset
      call $pack
      return
    end

    local.get $len
    local.get $offset
    i32.sub
    local.tee $remaining
    local.get $prefix_len
    i32.lt_u
    if
      i32.const 1
      local.get $offset
      call $pack
      return
    end

    local.get $offset
    local.get $prefix_len
    i32.add
    local.tee $value_off
    local.set $value_off

    local.get $value_len
    local.get $len
    local.get $value_off
    i32.sub
    i32.gt_u
    if
      i32.const 1
      local.get $offset
      call $pack
      return
    end

    local.get $value_off
    local.get $value_len
    i32.add
    local.set $next_offset

    local.get $out
    local.get $value_off
    local.get $value_len
    call $store_span

    i32.const 0
    local.get $next_offset
    call $pack)

  (func $length_field_scan_u8 (export "length_field_scan_u8")
    (param $ptr i32) (param $len i32) (param $offset i32) (param $out i32)
    (result i64)
    local.get $offset
    local.get $len
    i32.ge_u
    if
      i32.const 1
      local.get $offset
      call $pack
      return
    end
    local.get $ptr
    local.get $len
    local.get $offset
    local.get $out
    i32.const 1
    local.get $ptr
    local.get $offset
    i32.add
    i32.load8_u
    call $scan_i32)

  (func $length_field_scan_u16_le (export "length_field_scan_u16_le")
    (param $ptr i32) (param $len i32) (param $offset i32) (param $out i32)
    (result i64)
    local.get $offset
    local.get $len
    i32.gt_u
    if
      i32.const 1
      local.get $offset
      call $pack
      return
    end
    local.get $len
    local.get $offset
    i32.sub
    i32.const 2
    i32.lt_u
    if
      i32.const 1
      local.get $offset
      call $pack
      return
    end
    local.get $ptr
    local.get $len
    local.get $offset
    local.get $out
    i32.const 2
    local.get $ptr
    local.get $offset
    i32.add
    call $m128read_u16_le
    call $scan_i32)

  (func $length_field_scan_u32_be (export "length_field_scan_u32_be")
    (param $ptr i32) (param $len i32) (param $offset i32) (param $out i32)
    (result i64)
    local.get $offset
    local.get $len
    i32.gt_u
    if
      i32.const 1
      local.get $offset
      call $pack
      return
    end
    local.get $len
    local.get $offset
    i32.sub
    i32.const 4
    i32.lt_u
    if
      i32.const 1
      local.get $offset
      call $pack
      return
    end
    local.get $ptr
    local.get $len
    local.get $offset
    local.get $out
    i32.const 4
    local.get $ptr
    local.get $offset
    i32.add
    call $m128read_u32_be
    call $scan_i32)

  (func $length_field_scan_u32_le (export "length_field_scan_u32_le")
    (param $ptr i32) (param $len i32) (param $offset i32) (param $out i32)
    (result i64)
    local.get $offset
    local.get $len
    i32.gt_u
    if
      i32.const 1
      local.get $offset
      call $pack
      return
    end
    local.get $len
    local.get $offset
    i32.sub
    i32.const 4
    i32.lt_u
    if
      i32.const 1
      local.get $offset
      call $pack
      return
    end
    local.get $ptr
    local.get $len
    local.get $offset
    local.get $out
    i32.const 4
    local.get $ptr
    local.get $offset
    i32.add
    call $m128read_u32_le
    call $scan_i32)

  (func $length_field_scan_u64_le (export "length_field_scan_u64_le")
    (param $ptr i32) (param $len i32) (param $offset i32) (param $out i32)
    (result i64)
    (local $field_len i64)

    local.get $offset
    local.get $len
    i32.gt_u
    if
      i32.const 1
      local.get $offset
      call $pack
      return
    end
    local.get $len
    local.get $offset
    i32.sub
    i32.const 8
    i32.lt_u
    if
      i32.const 1
      local.get $offset
      call $pack
      return
    end

    local.get $ptr
    local.get $offset
    i32.add
    call $read_u64_le
    local.tee $field_len
    i64.const 0xffffffff
    i64.gt_u
    if
      i32.const 4
      local.get $offset
      call $pack
      return
    end

    local.get $ptr
    local.get $len
    local.get $offset
    local.get $out
    i32.const 8
    local.get $field_len
    i32.wrap_i64
    call $scan_i32)

  (func $write_check (param $len i64) (param $out_cap i32) (param $prefix_len i32) (param $max_len i64) (result i64)
    local.get $out_cap
    local.get $prefix_len
    i32.lt_u
    if
      i32.const 2
      i32.const 0
      call $pack
      return
    end
    local.get $len
    local.get $max_len
    i64.gt_u
    if
      i32.const 4
      i32.const 0
      call $pack
      return
    end
    i32.const 0
    local.get $prefix_len
    call $pack)

  (func $length_field_write_u8 (export "length_field_write_u8")
    (param $len i64) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    local.get $len
    local.get $out_cap
    i32.const 1
    i64.const 255
    call $write_check
    i64.const 0xffffffff
    i64.and
    i32.wrap_i64
    i32.eqz
    if
      local.get $out_ptr
      local.get $len
      i32.wrap_i64
      i32.store8
    end
    local.get $len
    local.get $out_cap
    i32.const 1
    i64.const 255
    call $write_check)

  (func $length_field_write_u16_le (export "length_field_write_u16_le")
    (param $len i64) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    local.get $len
    local.get $out_cap
    i32.const 2
    i64.const 65535
    call $write_check
    i64.const 0xffffffff
    i64.and
    i32.wrap_i64
    i32.eqz
    if
      local.get $out_ptr
      local.get $len
      i32.wrap_i64
      i32.store16
    end
    local.get $len
    local.get $out_cap
    i32.const 2
    i64.const 65535
    call $write_check)

  (func $length_field_write_u32_be (export "length_field_write_u32_be")
    (param $len i64) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    local.get $len
    local.get $out_cap
    i32.const 4
    i64.const 0xffffffff
    call $write_check
    i64.const 0xffffffff
    i64.and
    i32.wrap_i64
    i32.eqz
    if
      local.get $out_ptr
      local.get $len
      i64.const 24
      i64.shr_u
      i32.wrap_i64
      i32.store8
      local.get $out_ptr
      i32.const 1
      i32.add
      local.get $len
      i64.const 16
      i64.shr_u
      i32.wrap_i64
      i32.store8
      local.get $out_ptr
      i32.const 2
      i32.add
      local.get $len
      i64.const 8
      i64.shr_u
      i32.wrap_i64
      i32.store8
      local.get $out_ptr
      i32.const 3
      i32.add
      local.get $len
      i32.wrap_i64
      i32.store8
    end
    local.get $len
    local.get $out_cap
    i32.const 4
    i64.const 0xffffffff
    call $write_check)

  (func $length_field_write_u32_le (export "length_field_write_u32_le")
    (param $len i64) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    local.get $len
    local.get $out_cap
    i32.const 4
    i64.const 0xffffffff
    call $write_check
    i64.const 0xffffffff
    i64.and
    i32.wrap_i64
    i32.eqz
    if
      local.get $out_ptr
      local.get $len
      i32.wrap_i64
      i32.store
    end
    local.get $len
    local.get $out_cap
    i32.const 4
    i64.const 0xffffffff
    call $write_check)

  (func $length_field_write_u64_le (export "length_field_write_u64_le")
    (param $len i64) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    local.get $len
    local.get $out_cap
    i32.const 8
    i64.const -1
    call $write_check
    i64.const 0xffffffff
    i64.and
    i32.wrap_i64
    i32.eqz
    if
      local.get $out_ptr
      local.get $len
      i64.store
    end
    local.get $len
    local.get $out_cap
    i32.const 8
    i64.const -1
    call $write_check)


;; Status values: 0 ok, 1 input_short, 2 output_short, 4 overflow.
  ;; Packed encode return: low32=status, high32=written.

  ;; Output record, little-endian u32:
  ;; 0: len_low
  ;; 4: len_high
  ;; 8: header_len
  (func $m129write_record
    (param $out i32) (param $len_low i32) (param $len_high i32) (param $header_len i32)
    (i32.store (local.get $out) (local.get $len_low))
    (i32.store (i32.add (local.get $out) (i32.const 4)) (local.get $len_high))
    (i32.store (i32.add (local.get $out) (i32.const 8)) (local.get $header_len)))

  (func $frame_header_decode_u16_be (export "frame_header_decode_u16_be")
    (param $ptr i32) (param $len i32) (param $out i32)
    (result i32)
    (local $payload_len i32)
    (if (i32.lt_u (local.get $len) (i32.const 2))
      (then (return (i32.const 1))))
    (local.set $payload_len
      (i32.or
        (i32.shl (i32.load8_u (local.get $ptr)) (i32.const 8))
        (i32.load8_u (i32.add (local.get $ptr) (i32.const 1)))))
    (call $m129write_record (local.get $out) (local.get $payload_len) (i32.const 0) (i32.const 2))
    (i32.const 0))

  (func $frame_header_encode_u16_be (export "frame_header_encode_u16_be")
    (param $payload_len i32) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (if (i32.lt_u (local.get $out_cap) (i32.const 2))
      (then (return (call $pack (i32.const 2) (i32.const 0)))))
    (if (i32.gt_u (local.get $payload_len) (i32.const 65535))
      (then (return (call $pack (i32.const 4) (i32.const 0)))))
    (i32.store8 (local.get $out_ptr) (i32.shr_u (local.get $payload_len) (i32.const 8)))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 1)) (local.get $payload_len))
    (call $pack (i32.const 0) (i32.const 2)))

  (func $frame_header_decode_u64_be (export "frame_header_decode_u64_be")
    (param $ptr i32) (param $len i32) (param $out i32)
    (result i32)
    (local $hi i32)
    (local $lo i32)
    (if (i32.lt_u (local.get $len) (i32.const 8))
      (then (return (i32.const 1))))
    (local.set $hi
      (i32.or
        (i32.or
          (i32.shl (i32.load8_u (local.get $ptr)) (i32.const 24))
          (i32.shl (i32.load8_u (i32.add (local.get $ptr) (i32.const 1))) (i32.const 16)))
        (i32.or
          (i32.shl (i32.load8_u (i32.add (local.get $ptr) (i32.const 2))) (i32.const 8))
          (i32.load8_u (i32.add (local.get $ptr) (i32.const 3))))))
    (local.set $lo
      (i32.or
        (i32.or
          (i32.shl (i32.load8_u (i32.add (local.get $ptr) (i32.const 4))) (i32.const 24))
          (i32.shl (i32.load8_u (i32.add (local.get $ptr) (i32.const 5))) (i32.const 16)))
        (i32.or
          (i32.shl (i32.load8_u (i32.add (local.get $ptr) (i32.const 6))) (i32.const 8))
          (i32.load8_u (i32.add (local.get $ptr) (i32.const 7))))))
    (call $m129write_record (local.get $out) (local.get $lo) (local.get $hi) (i32.const 8))
    (i32.const 0))

  (func $frame_header_encode_u64_be (export "frame_header_encode_u64_be")
    (param $payload_len_low i32) (param $payload_len_high i32) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (if (i32.lt_u (local.get $out_cap) (i32.const 8))
      (then (return (call $pack (i32.const 2) (i32.const 0)))))
    (i32.store8 (local.get $out_ptr) (i32.shr_u (local.get $payload_len_high) (i32.const 24)))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 1))
      (i32.shr_u (local.get $payload_len_high) (i32.const 16)))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 2))
      (i32.shr_u (local.get $payload_len_high) (i32.const 8)))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 3)) (local.get $payload_len_high))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 4))
      (i32.shr_u (local.get $payload_len_low) (i32.const 24)))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 5))
      (i32.shr_u (local.get $payload_len_low) (i32.const 16)))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 6))
      (i32.shr_u (local.get $payload_len_low) (i32.const 8)))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 7)) (local.get $payload_len_low))
    (call $pack (i32.const 0) (i32.const 8)))

  (func $frame_header_decode_u64_le (export "frame_header_decode_u64_le")
    (param $ptr i32) (param $len i32) (param $out i32)
    (result i32)
    (local $hi i32)
    (local $lo i32)
    (if (i32.lt_u (local.get $len) (i32.const 8))
      (then (return (i32.const 1))))
    (local.set $lo
      (i32.or
        (i32.or
          (i32.load8_u (local.get $ptr))
          (i32.shl (i32.load8_u (i32.add (local.get $ptr) (i32.const 1))) (i32.const 8)))
        (i32.or
          (i32.shl (i32.load8_u (i32.add (local.get $ptr) (i32.const 2))) (i32.const 16))
          (i32.shl (i32.load8_u (i32.add (local.get $ptr) (i32.const 3))) (i32.const 24)))))
    (local.set $hi
      (i32.or
        (i32.or
          (i32.load8_u (i32.add (local.get $ptr) (i32.const 4)))
          (i32.shl (i32.load8_u (i32.add (local.get $ptr) (i32.const 5))) (i32.const 8)))
        (i32.or
          (i32.shl (i32.load8_u (i32.add (local.get $ptr) (i32.const 6))) (i32.const 16))
          (i32.shl (i32.load8_u (i32.add (local.get $ptr) (i32.const 7))) (i32.const 24)))))
    (call $m129write_record (local.get $out) (local.get $lo) (local.get $hi) (i32.const 8))
    (i32.const 0))

  (func $frame_header_encode_u64_le (export "frame_header_encode_u64_le")
    (param $payload_len_low i32) (param $payload_len_high i32) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (if (i32.lt_u (local.get $out_cap) (i32.const 8))
      (then (return (call $pack (i32.const 2) (i32.const 0)))))
    (i32.store8 (local.get $out_ptr) (local.get $payload_len_low))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 1))
      (i32.shr_u (local.get $payload_len_low) (i32.const 8)))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 2))
      (i32.shr_u (local.get $payload_len_low) (i32.const 16)))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 3))
      (i32.shr_u (local.get $payload_len_low) (i32.const 24)))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 4)) (local.get $payload_len_high))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 5))
      (i32.shr_u (local.get $payload_len_high) (i32.const 8)))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 6))
      (i32.shr_u (local.get $payload_len_high) (i32.const 16)))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 7))
      (i32.shr_u (local.get $payload_len_high) (i32.const 24)))
    (call $pack (i32.const 0) (i32.const 8)))


(func $m96write_record
    (param $out i32)
    (param $tag i32)
    (param $value_off i32)
    (param $value_len i32)
    (param $header_len i32)
    (param $total_len i32)

    local.get $out
    local.get $tag
    i32.store

    local.get $out
    i32.const 4
    i32.add
    local.get $value_off
    i32.store

    local.get $out
    i32.const 8
    i32.add
    local.get $value_len
    i32.store

    local.get $out
    i32.const 12
    i32.add
    local.get $header_len
    i32.store

    local.get $out
    i32.const 16
    i32.add
    local.get $total_len
    i32.store)

  ;; Parse a one-byte-tag TLV at offset.
  ;;
  ;; Return bits: low32=status, high32=next_offset.
  ;; status: 0 ok, 1 input_short, 2 output_short, 3 invalid.
  ;; Record fields are little-endian u32:
  ;; tag,value_off,value_len,header_len,total_len.
  (func $generic_tlv_next (export "generic_tlv_next")
    (param $ptr i32) (param $len i32) (param $offset i32) (param $out i32)
    (result i64)
    (local $tag i32)
    (local $first i32)
    (local $len_bytes i32)
    (local $value_len i32)
    (local $header_len i32)
    (local $total_len i32)
    (local $value_off i32)
    (local $next_offset i32)

    local.get $offset
    local.get $len
    i32.ge_u
    if
      i32.const 1
      local.get $len
      call $pack
      return
    end

    local.get $len
    local.get $offset
    i32.sub
    i32.const 2
    i32.lt_u
    if
      i32.const 1
      local.get $len
      call $pack
      return
    end

    local.get $ptr
    local.get $offset
    i32.add
    i32.load8_u
    local.set $tag

    local.get $ptr
    local.get $offset
    i32.add
    i32.const 1
    i32.add
    i32.load8_u
    local.set $first

    local.get $first
    i32.const 128
    i32.lt_u
    if
      local.get $first
      local.set $value_len
      i32.const 2
      local.set $header_len
    else
      local.get $first
      i32.const 128
      i32.eq
      if
        i32.const 3
        local.get $offset
        call $pack
        return
      end

      local.get $first
      i32.const 127
      i32.and
      local.set $len_bytes

      local.get $len_bytes
      i32.const 1
      i32.eq
      if
        local.get $len
        local.get $offset
        i32.sub
        i32.const 3
        i32.lt_u
        if
          i32.const 1
          local.get $len
          call $pack
          return
        end

        local.get $ptr
        local.get $offset
        i32.add
        i32.const 2
        i32.add
        i32.load8_u
        local.set $value_len
        i32.const 3
        local.set $header_len
      else
        local.get $len_bytes
        i32.const 2
        i32.eq
        if
          local.get $len
          local.get $offset
          i32.sub
          i32.const 4
          i32.lt_u
          if
            i32.const 1
            local.get $len
            call $pack
            return
          end

          local.get $ptr
          local.get $offset
          i32.add
          i32.const 2
          i32.add
          i32.load8_u
          i32.const 8
          i32.shl
          local.get $ptr
          local.get $offset
          i32.add
          i32.const 3
          i32.add
          i32.load8_u
          i32.or
          local.set $value_len
          i32.const 4
          local.set $header_len
        else
          i32.const 3
          local.get $offset
          call $pack
          return
        end
      end
    end

    local.get $header_len
    local.get $value_len
    i32.add
    local.set $total_len

    local.get $offset
    local.get $header_len
    i32.add
    local.set $value_off

    local.get $offset
    local.get $total_len
    i32.add
    local.set $next_offset

    local.get $next_offset
    local.get $len
    i32.gt_u
    if
      i32.const 1
      local.get $len
      call $pack
      return
    end

    local.get $out
    local.get $tag
    local.get $value_off
    local.get $value_len
    local.get $header_len
    local.get $total_len
    call $m96write_record

    i32.const 0
    local.get $next_offset
    call $pack)

  ;; Encode only the TLV header for a one-byte tag and value length.
  ;;
  ;; Return bits: low32=status, high32=written.
  ;; status: 0 ok, 2 output_short, 3 invalid.
  (func $generic_tlv_encode_header (export "generic_tlv_encode_header")
    (param $tag i32) (param $value_len i32) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (local $needed i32)

    local.get $tag
    i32.const 255
    i32.gt_u
    if
      i32.const 3
      i32.const 0
      call $pack
      return
    end

    local.get $value_len
    i32.const 65535
    i32.gt_u
    if
      i32.const 3
      i32.const 0
      call $pack
      return
    end

    local.get $value_len
    i32.const 127
    i32.le_u
    if
      i32.const 2
      local.set $needed
    else
      local.get $value_len
      i32.const 255
      i32.le_u
      if
        i32.const 3
        local.set $needed
      else
        i32.const 4
        local.set $needed
      end
    end

    local.get $out_cap
    local.get $needed
    i32.lt_u
    if
      i32.const 2
      i32.const 0
      call $pack
      return
    end

    local.get $out_ptr
    local.get $tag
    i32.store8

    local.get $needed
    i32.const 2
    i32.eq
    if
      local.get $out_ptr
      i32.const 1
      i32.add
      local.get $value_len
      i32.store8
    else
      local.get $needed
      i32.const 3
      i32.eq
      if
        local.get $out_ptr
        i32.const 1
        i32.add
        i32.const 129
        i32.store8

        local.get $out_ptr
        i32.const 2
        i32.add
        local.get $value_len
        i32.store8
      else
        local.get $out_ptr
        i32.const 1
        i32.add
        i32.const 130
        i32.store8

        local.get $out_ptr
        i32.const 2
        i32.add
        local.get $value_len
        i32.const 8
        i32.shr_u
        i32.store8

        local.get $out_ptr
        i32.const 3
        i32.add
        local.get $value_len
        i32.const 255
        i32.and
        i32.store8
      end
    end

    i32.const 0
    local.get $needed
    call $pack)


(func $m101write_record
    (param $out i32)
    (param $host_off i32)
    (param $host_len i32)
    (param $port i32)
    (param $has_explicit_port i32)
    local.get $out
    local.get $host_off
    i32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $host_len
    i32.store
    local.get $out
    i32.const 8
    i32.add
    local.get $port
    i32.store
    local.get $out
    i32.const 12
    i32.add
    local.get $has_explicit_port
    i32.store)

  ;; Scan host[:port] using the last colon as the separator, matching the Rust
  ;; parse_host_port_with_default helper. Output offsets are relative to ptr.
  (func (export "host_port_scan")
    (param $ptr i32)
    (param $len i32)
    (param $default_port i32)
    (param $out i32)
    (result i32)
    (local $i i32)
    (local $colon i32)
    (local $b i32)
    (local $digit i32)
    (local $port i32)

    local.get $len
    i32.eqz
    if
      i32.const 3
      return
    end

    i32.const -1
    local.set $colon
    i32.const 0
    local.set $i

    (block $scan_done
      (loop $scan
        local.get $i
        local.get $len
        i32.ge_u
        br_if $scan_done

        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        i32.const 58
        i32.eq
        if
          local.get $i
          local.set $colon
        end

        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $scan))

    local.get $colon
    i32.const -1
    i32.eq
    if
      local.get $out
      i32.const 0
      local.get $len
      local.get $default_port
      i32.const 0
      call $m101write_record
      i32.const 0
      return
    end

    local.get $colon
    i32.eqz
    if
      i32.const 3
      return
    end

    local.get $colon
    i32.const 1
    i32.add
    local.get $len
    i32.ge_u
    if
      i32.const 3
      return
    end

    local.get $colon
    i32.const 1
    i32.add
    local.set $i
    i32.const 0
    local.set $port

    (block $port_done
      (loop $port_loop
        local.get $i
        local.get $len
        i32.ge_u
        br_if $port_done

        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        local.tee $b
        call $is_digit
        i32.eqz
        if
          i32.const 3
          return
        end

        local.get $b
        i32.const 48
        i32.sub
        local.set $digit

        local.get $port
        i32.const 6553
        i32.gt_u
        if
          i32.const 3
          return
        end

        local.get $port
        i32.const 6553
        i32.eq
        local.get $digit
        i32.const 5
        i32.gt_u
        i32.and
        if
          i32.const 3
          return
        end

        local.get $port
        i32.const 10
        i32.mul
        local.get $digit
        i32.add
        local.set $port

        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $port_loop))

    local.get $out
    i32.const 0
    local.get $colon
    local.get $port
    i32.const 1
    call $m101write_record
    i32.const 0)


  ;; OSRS cache smart-int encoding — prefix-variable integer formats.
  ;;
  ;; Short smart:  peek < 128 →  1 byte  (unsigned or signed with offset)
  ;;               peek ≥ 128 →  2 bytes (unsigned short − offset)
  ;; Big smart:    peek ≥ 0   →  2 bytes (unsigned short)
  ;;               peek < 0   →  4 bytes (int & MAX_VALUE)
  ;;
  ;; All decode functions take (buf, off, result_out) and return new offset.
  ;; Scratch byte at memory[0..4] for intermediate storage in compat function.
  (func (export "read_short_smart") (param $buf i32) (param $off i32) (param $out i32) (result i32)
    (local $peek i32)
    local.get $buf local.get $off i32.add i32.load8_u local.tee $peek
    i32.const 128 i32.lt_u
    if
      local.get $out local.get $peek i32.const 64 i32.sub i32.store
      local.get $off i32.const 1 i32.add
      return
    end
    local.get $out
    local.get $buf local.get $off i32.add i32.load16_u i32.const 0xC000 i32.sub i32.store
    local.get $off i32.const 2 i32.add
  )

  (func (export "read_unsigned_short_smart") (param $buf i32) (param $off i32) (param $out i32) (result i32)
    (local $peek i32)
    local.get $buf local.get $off i32.add i32.load8_u local.tee $peek
    i32.const 128 i32.lt_u
    if
      local.get $out local.get $peek i32.store
      local.get $off i32.const 1 i32.add
      return
    end
    local.get $out
    local.get $buf local.get $off i32.add i32.load16_u i32.const 0x8000 i32.sub i32.store
    local.get $off i32.const 2 i32.add
  )

  (func (export "read_unsigned_short_smart_minus_one") (param $buf i32) (param $off i32) (param $out i32) (result i32)
    (local $peek i32)
    local.get $buf local.get $off i32.add i32.load8_u local.tee $peek
    i32.const 128 i32.lt_u
    if
      local.get $out local.get $peek i32.const 1 i32.sub i32.store
      local.get $off i32.const 1 i32.add
      return
    end
    local.get $out
    local.get $buf local.get $off i32.add i32.load16_u i32.const 0x8001 i32.sub i32.store
    local.get $off i32.const 2 i32.add
  )

  (func (export "read_unsigned_int_smart_short_compat") (param $buf i32) (param $off i32) (param $out i32) (result i32)
    (local $total i32) (local $peek i32) (local $val i32)
    i32.const 0 local.set $total
    block $done
    loop $loop
      local.get $buf local.get $off i32.add i32.load8_u local.tee $peek
      i32.const 128 i32.lt_u
      if
        local.get $peek local.set $val
        local.get $off i32.const 1 i32.add local.set $off
      else
        local.get $buf local.get $off i32.add i32.load16_u i32.const 0x8000 i32.sub local.set $val
        local.get $off i32.const 2 i32.add local.set $off
      end
      local.get $val i32.const 32767 i32.ne
      if
        local.get $total local.get $val i32.add local.set $total
        br $done
      end
      local.get $total i32.const 32767 i32.add local.set $total
      br $loop
    end
    end
    local.get $out local.get $total i32.store
    local.get $off
  )

  (func (export "read_big_smart") (param $buf i32) (param $off i32) (param $out i32) (result i32)
    local.get $buf local.get $off i32.add i32.load8_s i32.const 0 i32.ge_s
    if
      local.get $out local.get $buf local.get $off i32.add i32.load16_u i32.store
      local.get $off i32.const 2 i32.add
      return
    end
    local.get $out
    local.get $buf local.get $off i32.add i32.load i32.const 0x7FFFFFFF i32.and i32.store
    local.get $off i32.const 4 i32.add
  )

  (func (export "read_big_smart2") (param $buf i32) (param $off i32) (param $out i32) (result i32)
    (local $val i32)
    local.get $buf local.get $off i32.add i32.load8_s i32.const 0 i32.lt_s
    if
      local.get $out
      local.get $buf local.get $off i32.add i32.load i32.const 0x7FFFFFFF i32.and i32.store
      local.get $off i32.const 4 i32.add
      return
    end
    local.get $buf local.get $off i32.add i32.load16_u local.set $val
    local.get $val i32.const 32767 i32.eq
    if
      local.get $out i32.const -1 i32.store
    else
      local.get $out local.get $val i32.store
    end
    local.get $off i32.const 2 i32.add
  )

  (func (export "read_24bit_int") (param $buf i32) (param $off i32) (result i32)
    local.get $buf local.get $off i32.add i32.load8_u i32.const 16 i32.shl
    local.get $buf local.get $off i32.const 1 i32.add i32.add i32.load8_u i32.const 8 i32.shl i32.or
    local.get $buf local.get $off i32.const 2 i32.add i32.add i32.load8_u i32.or
  )

  ;; write_short_smart: encode a value using the short smart format.
  ;; Returns the new offset.
  (func (export "write_short_smart") (param $buf i32) (param $off i32) (param $val i32) (result i32)
    (local $v i32)
    local.get $val i32.const 64 i32.add local.tee $v
    i32.const 128 i32.lt_u
    if
      local.get $buf local.get $off i32.add local.get $v i32.store8
      local.get $off i32.const 1 i32.add
      return
    end
    local.get $buf local.get $off i32.add
    local.get $val i32.const 0xC000 i32.add i32.store16
    local.get $off i32.const 2 i32.add
  )

  (func (export "write_unsigned_short_smart") (param $buf i32) (param $off i32) (param $val i32) (result i32)
    local.get $val i32.const 128 i32.lt_u
    if
      local.get $buf local.get $off i32.add local.get $val i32.store8
      local.get $off i32.const 1 i32.add
      return
    end
    local.get $buf local.get $off i32.add
    local.get $val i32.const 0x8000 i32.add i32.store16
    local.get $off i32.const 2 i32.add
  )

  (func (export "write_big_smart") (param $buf i32) (param $off i32) (param $val i32) (result i32)
    local.get $val i32.const 0xFFFF i32.le_u
    if
      local.get $buf local.get $off i32.add local.get $val i32.store16
      local.get $off i32.const 2 i32.add
      return
    end
    local.get $buf local.get $off i32.add local.get $val i32.const 0x7FFFFFFF i32.and i32.store
    local.get $off i32.const 4 i32.add
  )
