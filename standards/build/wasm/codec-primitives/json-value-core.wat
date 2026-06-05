(module
  (memory (export "memory") 1)
  (data (i32.const 60000) "9223372036854775807")
  (data (i32.const 60032) "18446744073709551615")

  (func (export "proto_abi_version") (result i32)
    i32.const 2)

  (func (export "proto_standard_id") (result i32)
    i32.const 300041)

  (func $pack (param $status i32) (param $value i32) (result i64)
    local.get $value
    i64.extend_i32_u
    i64.const 32
    i64.shl
    local.get $status
    i64.extend_i32_u
    i64.or)

  (func $token_addr (param $token_ptr i32) (param $idx i32) (result i32)
    local.get $token_ptr
    local.get $idx
    i32.const 20
    i32.mul
    i32.add)

  (func $token_kind (param $token_ptr i32) (param $idx i32) (result i32)
    local.get $token_ptr
    local.get $idx
    call $token_addr
    i32.load)

  (func $token_start (param $token_ptr i32) (param $idx i32) (result i32)
    local.get $token_ptr
    local.get $idx
    call $token_addr
    i32.const 4
    i32.add
    i32.load)

  (func $token_parent (param $token_ptr i32) (param $idx i32) (result i32)
    local.get $token_ptr
    local.get $idx
    call $token_addr
    i32.const 12
    i32.add
    i32.load)

  (func $is_digit (param $c i32) (result i32)
    local.get $c
    i32.const 48
    i32.ge_u
    local.get $c
    i32.const 57
    i32.le_u
    i32.and)

  (func $byte_at (param $ptr i32) (param $off i32) (result i32)
    local.get $ptr
    local.get $off
    i32.add
    i32.load8_u)

  (func $limit_digit (param $limit i32) (param $idx i32) (result i32)
    local.get $limit
    i32.const 0
    i32.eq
    if
      i32.const 60000
      local.get $idx
      i32.add
      i32.load8_u
      i32.const 48
      i32.sub
      return
    end
    local.get $limit
    i32.const 1
    i32.eq
    if
      i32.const 60032
      local.get $idx
      i32.add
      i32.load8_u
      i32.const 48
      i32.sub
      return
    end
    i32.const 0)

  (func $digits_le_limit (param $ptr i32) (param $start i32) (param $end i32) (param $limit i32) (result i32)
    (local $digits i32)
    (local $i i32)
    (local $digit i32)
    (local $limit_digit i32)
    local.get $end
    local.get $start
    i32.sub
    local.set $digits
    local.get $limit
    i32.const 0
    i32.eq
    if
      local.get $digits
      i32.const 19
      i32.lt_u
      if
        i32.const 1
        return
      end
      local.get $digits
      i32.const 19
      i32.gt_u
      if
        i32.const 0
        return
      end
    else
      local.get $digits
      i32.const 20
      i32.lt_u
      if
        i32.const 1
        return
      end
      local.get $digits
      i32.const 20
      i32.gt_u
      if
        i32.const 0
        return
      end
    end
    i32.const 0
    local.set $i
    (block $done
      (loop $again
        local.get $i
        local.get $digits
        i32.ge_u
        br_if $done
        local.get $ptr
        local.get $start
        local.get $i
        i32.add
        call $byte_at
        i32.const 48
        i32.sub
        local.set $digit
        local.get $limit
        local.get $i
        call $limit_digit
        local.set $limit_digit
        local.get $digit
        local.get $limit_digit
        i32.lt_u
        if
          i32.const 1
          return
        end
        local.get $digit
        local.get $limit_digit
        i32.gt_u
        if
          i32.const 0
          return
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $again))
    i32.const 1)

  (func $is_value_token (param $kind i32) (result i32)
    local.get $kind
    i32.const 1
    i32.eq
    local.get $kind
    i32.const 2
    i32.eq
    i32.or
    local.get $kind
    i32.const 3
    i32.eq
    i32.or
    local.get $kind
    i32.const 5
    i32.eq
    i32.or
    local.get $kind
    i32.const 6
    i32.eq
    i32.or
    local.get $kind
    i32.const 7
    i32.eq
    i32.or)

  ;; Returns JsonValue variant ids: null=0, bool=1, number=2, string=3,
  ;; array=4, object=5. Unknown tape kinds return -1.
  (func (export "json_value_variant_for_tape_kind") (param $tape_kind i32) (result i32)
    local.get $tape_kind
    i32.const 7
    i32.eq
    if
      i32.const 0
      return
    end
    local.get $tape_kind
    i32.const 6
    i32.eq
    if
      i32.const 1
      return
    end
    local.get $tape_kind
    i32.const 5
    i32.eq
    if
      i32.const 2
      return
    end
    local.get $tape_kind
    i32.const 3
    i32.eq
    if
      i32.const 3
      return
    end
    local.get $tape_kind
    i32.const 2
    i32.eq
    if
      i32.const 4
      return
    end
    local.get $tape_kind
    i32.const 1
    i32.eq
    if
      i32.const 5
      return
    end
    i32.const -1)

  ;; Conversion target ids: null=0, bool=1, number=2, string=3, array=4,
  ;; object=5, i64=6, u64=7, f64=8, i32=9, u32=10, usize=11.
  (func (export "json_value_can_convert") (param $variant i32) (param $number_caps i32) (param $target i32) (result i32)
    local.get $target
    i32.const 6
    i32.ge_u
    if
      local.get $variant
      i32.const 2
      i32.ne
      if
        i32.const 0
        return
      end
      local.get $target
      i32.const 6
      i32.eq
      if
        local.get $number_caps
        i32.const 1
        i32.and
        i32.const 0
        i32.ne
        return
      end
      local.get $target
      i32.const 7
      i32.eq
      if
        local.get $number_caps
        i32.const 2
        i32.and
        i32.const 0
        i32.ne
        return
      end
      local.get $target
      i32.const 8
      i32.eq
      if
        local.get $number_caps
        i32.const 4
        i32.and
        i32.const 0
        i32.ne
        return
      end
      local.get $target
      i32.const 9
      i32.eq
      if
        local.get $number_caps
        i32.const 8
        i32.and
        i32.const 0
        i32.ne
        return
      end
      local.get $target
      i32.const 10
      i32.eq
      if
        local.get $number_caps
        i32.const 16
        i32.and
        i32.const 0
        i32.ne
        return
      end
      local.get $target
      i32.const 11
      i32.eq
      if
        local.get $number_caps
        i32.const 32
        i32.and
        i32.const 0
        i32.ne
        return
      end
      i32.const 0
      return
    end
    local.get $variant
    local.get $target
    i32.eq)

  (func $parse_uint_cap (param $ptr i32) (param $start i32) (param $end i32) (param $max i64) (result i64)
    (local $p i32)
    (local $value i64)
    (local $digit i64)
    local.get $start
    local.set $p
    i64.const 0
    local.set $value
    (block $done
      (loop $again
        local.get $p
        local.get $end
        i32.ge_u
        br_if $done
        local.get $ptr
        local.get $p
        call $byte_at
        i64.extend_i32_u
        i64.const 48
        i64.sub
        local.set $digit
        local.get $value
        local.get $max
        i64.const 10
        i64.div_u
        i64.gt_u
        if
          i64.const -1
          return
        end
        local.get $value
        local.get $max
        i64.const 10
        i64.div_u
        i64.eq
        local.get $digit
        local.get $max
        i64.const 10
        i64.rem_u
        i64.gt_u
        i32.and
        if
          i64.const -1
          return
        end
        local.get $value
        i64.const 10
        i64.mul
        local.get $digit
        i64.add
        local.set $value
        local.get $p
        i32.const 1
        i32.add
        local.set $p
        br $again))
    local.get $value)

  ;; Returns status/value where value is a capability bitset:
  ;; bit0 as_i64/is_i64, bit1 as_u64/is_u64, bit2 as_f64, bit3 as_i32,
  ;; bit4 as_u32, bit5 as_usize, bit6 as_i128, bit7 as_u128, bit8 f64-class.
  (func (export "json_number_caps") (param $ptr i32) (param $len i32) (result i64)
    (local $p i32)
    (local $start_digits i32)
    (local $negative i32)
    (local $floaty i32)
    (local $value i64)
    local.get $len
    i32.eqz
    if
      i32.const 3
      i32.const 0
      call $pack
      return
    end
    i32.const 0
    local.set $p
    local.get $ptr
    i32.const 0
    call $byte_at
    i32.const 45
    i32.eq
    if
      i32.const 1
      local.set $negative
      i32.const 1
      local.set $p
      local.get $p
      local.get $len
      i32.ge_u
      if
        i32.const 3
        local.get $p
        call $pack
        return
      end
    end
    local.get $p
    local.set $start_digits
    local.get $ptr
    local.get $p
    call $byte_at
    call $is_digit
    i32.eqz
    if
      i32.const 3
      local.get $p
      call $pack
      return
    end
    local.get $ptr
    local.get $p
    call $byte_at
    i32.const 48
    i32.eq
    if
      local.get $p
      i32.const 1
      i32.add
      local.get $len
      i32.lt_u
      if
        local.get $ptr
        local.get $p
        i32.const 1
        i32.add
        call $byte_at
        call $is_digit
        if
          i32.const 3
          local.get $p
          i32.const 1
          i32.add
          call $pack
          return
        end
      end
    end
    (block $digits_done
      (loop $digits
        local.get $p
        local.get $len
        i32.ge_u
        br_if $digits_done
        local.get $ptr
        local.get $p
        call $byte_at
        call $is_digit
        i32.eqz
        br_if $digits_done
        local.get $p
        i32.const 1
        i32.add
        local.set $p
        br $digits))
    local.get $p
    local.get $len
    i32.lt_u
    if
      local.get $ptr
      local.get $p
      call $byte_at
      i32.const 46
      i32.eq
      if
        i32.const 1
        local.set $floaty
        local.get $p
        i32.const 1
        i32.add
        local.set $p
        local.get $p
        local.get $len
        i32.ge_u
        if
          i32.const 3
          local.get $p
          call $pack
          return
        end
        (block $frac_done
          (loop $frac
            local.get $p
            local.get $len
            i32.ge_u
            br_if $frac_done
            local.get $ptr
            local.get $p
            call $byte_at
            call $is_digit
            i32.eqz
            br_if $frac_done
            local.get $p
            i32.const 1
            i32.add
            local.set $p
            br $frac))
      end
    end
    local.get $p
    local.get $len
    i32.lt_u
    if
      local.get $ptr
      local.get $p
      call $byte_at
      i32.const 101
      i32.eq
      local.get $ptr
      local.get $p
      call $byte_at
      i32.const 69
      i32.eq
      i32.or
      if
        i32.const 1
        local.set $floaty
        local.get $p
        i32.const 1
        i32.add
        local.set $p
        local.get $p
        local.get $len
        i32.ge_u
        if
          i32.const 3
          local.get $p
          call $pack
          return
        end
        local.get $ptr
        local.get $p
        call $byte_at
        i32.const 43
        i32.eq
        local.get $ptr
        local.get $p
        call $byte_at
        i32.const 45
        i32.eq
        i32.or
        if
          local.get $p
          i32.const 1
          i32.add
          local.set $p
        end
        local.get $p
        local.get $len
        i32.ge_u
        if
          i32.const 3
          local.get $p
          call $pack
          return
        end
        local.get $ptr
        local.get $p
        call $byte_at
        call $is_digit
        i32.eqz
        if
          i32.const 3
          local.get $p
          call $pack
          return
        end
        (block $exp_done
          (loop $exp
            local.get $p
            local.get $len
            i32.ge_u
            br_if $exp_done
            local.get $ptr
            local.get $p
            call $byte_at
            call $is_digit
            i32.eqz
            br_if $exp_done
            local.get $p
            i32.const 1
            i32.add
            local.set $p
            br $exp))
      end
    end
    local.get $p
    local.get $len
    i32.ne
    if
      i32.const 3
      local.get $p
      call $pack
      return
    end
    local.get $floaty
    if
      i32.const 0
      i32.const 260
      call $pack
      return
    end
    local.get $negative
    if
      local.get $ptr
      local.get $start_digits
      local.get $len
      i64.const 9223372036854775808
      call $parse_uint_cap
      local.set $value
      local.get $value
      i64.const -1
      i64.eq
      if
        i32.const 4
        i32.const 0
        call $pack
        return
      end
      local.get $value
      i64.const 2147483648
      i64.le_u
      if
        i32.const 0
        i32.const 77
        call $pack
        return
      end
      i32.const 0
      i32.const 69
      call $pack
      return
    end
    local.get $ptr
    local.get $start_digits
    local.get $len
    i32.const 1
    call $digits_le_limit
    i32.eqz
    if
      i32.const 4
      i32.const 0
      call $pack
      return
    end
    local.get $ptr
    local.get $start_digits
    local.get $len
    i32.const 0
    call $digits_le_limit
    i32.eqz
    if
      i32.const 0
      i32.const 230
      call $pack
      return
    end
    local.get $ptr
    local.get $start_digits
    local.get $len
    i64.const 9223372036854775807
    call $parse_uint_cap
    local.set $value
    local.get $value
    i64.const -1
    i64.eq
    if
      i32.const 4
      i32.const 0
      call $pack
      return
    end
    local.get $value
    i64.const 9223372036854775807
    i64.le_u
    if
      local.get $value
      i64.const 2147483647
      i64.le_u
      if
        i32.const 0
        i32.const 255
        call $pack
        return
      end
      local.get $value
      i64.const 4294967295
      i64.le_u
      if
        i32.const 0
        i32.const 247
        call $pack
        return
      end
      i32.const 0
      i32.const 231
      call $pack
      return
    end
    i32.const 0
    i32.const 230
    call $pack)

  (func $raw_string_equals (param $input_ptr i32) (param $string_start i32) (param $string_end i32) (param $key_ptr i32) (param $key_len i32) (result i32)
    (local $i i32)
    local.get $string_end
    local.get $string_start
    i32.sub
    i32.const 2
    i32.lt_u
    if
      i32.const 0
      return
    end
    local.get $string_end
    local.get $string_start
    i32.sub
    i32.const 2
    i32.sub
    local.get $key_len
    i32.ne
    if
      i32.const 0
      return
    end
    i32.const 0
    local.set $i
    (block $done
      (loop $again
        local.get $i
        local.get $key_len
        i32.ge_u
        br_if $done
        local.get $input_ptr
        local.get $string_start
        i32.const 1
        i32.add
        local.get $i
        i32.add
        call $byte_at
        local.get $key_ptr
        local.get $i
        call $byte_at
        i32.ne
        if
          i32.const 0
          return
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $again))
    i32.const 1)

  ;; Finds the value token for the first raw ASCII object key match.
  ;; Returns status/value: 0/value_index, 1/not found, 2/bad tape.
  (func (export "json_object_find_field") (param $input_ptr i32) (param $input_len i32) (param $token_ptr i32) (param $token_count i32) (param $object_index i32) (param $key_ptr i32) (param $key_len i32) (result i64)
    (local $i i32)
    (local $kind i32)
    (local $start i32)
    (local $end i32)
    local.get $object_index
    local.get $token_count
    i32.ge_u
    if
      i32.const 2
      i32.const 0
      call $pack
      return
    end
    local.get $token_ptr
    local.get $object_index
    call $token_kind
    i32.const 1
    i32.ne
    if
      i32.const 2
      i32.const 0
      call $pack
      return
    end
    local.get $object_index
    i32.const 1
    i32.add
    local.set $i
    (block $done
      (loop $again
        local.get $i
        local.get $token_count
        i32.ge_u
        br_if $done
        local.get $token_ptr
        local.get $i
        call $token_parent
        local.get $object_index
        i32.eq
        if
          local.get $token_ptr
          local.get $i
          call $token_kind
          local.set $kind
          local.get $kind
          i32.const 3
          i32.eq
          if
            local.get $token_ptr
            local.get $i
            call $token_start
            local.set $start
            local.get $token_ptr
            local.get $i
            call $token_addr
            i32.const 8
            i32.add
            i32.load
            local.set $end
            local.get $end
            local.get $input_len
            i32.gt_u
            if
              i32.const 2
              i32.const 0
              call $pack
              return
            end
            local.get $input_ptr
            local.get $start
            local.get $end
            local.get $key_ptr
            local.get $key_len
            call $raw_string_equals
            if
              local.get $i
              i32.const 1
              i32.add
              local.get $token_count
              i32.ge_u
              if
                i32.const 2
                i32.const 0
                call $pack
                return
              end
              local.get $token_ptr
              local.get $i
              i32.const 1
              i32.add
              call $token_kind
              call $is_value_token
              i32.eqz
              if
                i32.const 2
                i32.const 0
                call $pack
                return
              end
              i32.const 0
              local.get $i
              i32.const 1
              i32.add
              call $pack
              return
            end
          end
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $again))
    i32.const 1
    i32.const 0
    call $pack)
)
