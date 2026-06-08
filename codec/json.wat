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
        call $load8_u
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
        call $load8_u
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
    call $load8_u
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
    call $load8_u
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
    call $load8_u
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
        call $load8_u
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
        call $load8_u
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
      call $load8_u
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
            call $load8_u
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
      call $load8_u
      i32.const 101
      i32.eq
      local.get $ptr
      local.get $p
      call $load8_u
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
        call $load8_u
        i32.const 43
        i32.eq
        local.get $ptr
        local.get $p
        call $load8_u
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
        call $load8_u
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
            call $load8_u
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
        call $load8_u
        local.get $key_ptr
        local.get $i
        call $load8_u
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


  ;; Status values: 0 ok, 2 output_short, 3 invalid.
  ;; Packed return: low u32 status, high u32 bytes written.

  (func $m123put (param $out_ptr i32) (param $out_cap i32) (param $written i32) (param $c i32) (result i64)
    (if (i32.ge_u (local.get $written) (local.get $out_cap))
      (then (return (call $pack (i32.const 2) (local.get $written)))))
    (i32.store8 (i32.add (local.get $out_ptr) (local.get $written)) (local.get $c))
    (call $pack (i32.const 0) (i32.add (local.get $written) (i32.const 1))))

  (func $m123hex_digit (param $value i32) (result i32)
    (if (i32.lt_u (local.get $value) (i32.const 10))
      (then (return (i32.add (i32.const 48) (local.get $value)))))
    (i32.add (i32.const 87) (local.get $value)))

  (func $emit_bytes (param $in_ptr i32) (param $in_len i32) (param $out_ptr i32) (param $out_cap i32) (param $written i32) (result i64)
    (local $i i32)
    (local $packed i64)
    (loop $again
      (if (i32.ge_u (local.get $i) (local.get $in_len))
        (then (return (call $pack (i32.const 0) (local.get $written)))))
      (local.set $packed
        (call $m123put
          (local.get $out_ptr)
          (local.get $out_cap)
          (local.get $written)
          (call $load8_u (local.get $in_ptr) (local.get $i))))
      (if (i32.ne (i32.wrap_i64 (local.get $packed)) (i32.const 0))
        (then (return (local.get $packed))))
      (local.set $written (i32.wrap_i64 (i64.shr_u (local.get $packed) (i64.const 32))))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br $again))
    (call $pack (i32.const 0) (local.get $written)))

  (func $emit_literal_from_static (param $kind i32) (param $out_ptr i32) (param $out_cap i32) (result i64)
    ;; kind: 0 null, 1 false, 2 true.
    (if (i32.eq (local.get $kind) (i32.const 0))
      (then
        (i32.store8 (i32.const 16) (i32.const 110))
        (i32.store8 (i32.const 17) (i32.const 117))
        (i32.store8 (i32.const 18) (i32.const 108))
        (i32.store8 (i32.const 19) (i32.const 108))
        (return (call $emit_bytes (i32.const 16) (i32.const 4) (local.get $out_ptr) (local.get $out_cap) (i32.const 0)))))
    (if (i32.eq (local.get $kind) (i32.const 1))
      (then
        (i32.store8 (i32.const 16) (i32.const 102))
        (i32.store8 (i32.const 17) (i32.const 97))
        (i32.store8 (i32.const 18) (i32.const 108))
        (i32.store8 (i32.const 19) (i32.const 115))
        (i32.store8 (i32.const 20) (i32.const 101))
        (return (call $emit_bytes (i32.const 16) (i32.const 5) (local.get $out_ptr) (local.get $out_cap) (i32.const 0)))))
    (if (i32.eq (local.get $kind) (i32.const 2))
      (then
        (i32.store8 (i32.const 16) (i32.const 116))
        (i32.store8 (i32.const 17) (i32.const 114))
        (i32.store8 (i32.const 18) (i32.const 117))
        (i32.store8 (i32.const 19) (i32.const 101))
        (return (call $emit_bytes (i32.const 16) (i32.const 4) (local.get $out_ptr) (local.get $out_cap) (i32.const 0)))))
    (call $pack (i32.const 3) (i32.const 0)))

  (func (export "json_emit_literal") (param $kind i32) (param $out_ptr i32) (param $out_cap i32) (result i64)
    (call $emit_literal_from_static (local.get $kind) (local.get $out_ptr) (local.get $out_cap)))

  (func $emit_u64_at (param $value i64) (param $out_ptr i32) (param $out_cap i32) (param $written i32) (result i64)
    (local $div i64)
    (local $digit i64)
    (local $packed i64)
    (if (i64.eqz (local.get $value))
      (then (return (call $m123put (local.get $out_ptr) (local.get $out_cap) (local.get $written) (i32.const 48)))))
    (local.set $div (i64.const 10000000000000000000))
    (block $scaled
      (loop $scale
        (br_if $scaled (i64.le_u (local.get $div) (local.get $value)))
        (local.set $div (i64.div_u (local.get $div) (i64.const 10)))
        (br $scale)))
    (loop $digits
      (local.set $digit (i64.div_u (local.get $value) (local.get $div)))
      (local.set $packed
        (call $m123put
          (local.get $out_ptr)
          (local.get $out_cap)
          (local.get $written)
          (i32.add (i32.const 48) (i32.wrap_i64 (local.get $digit)))))
      (if (i32.ne (i32.wrap_i64 (local.get $packed)) (i32.const 0))
        (then (return (local.get $packed))))
      (local.set $written (i32.wrap_i64 (i64.shr_u (local.get $packed) (i64.const 32))))
      (local.set $value (i64.rem_u (local.get $value) (local.get $div)))
      (local.set $div (i64.div_u (local.get $div) (i64.const 10)))
      (br_if $digits (i64.ne (local.get $div) (i64.const 0))))
    (call $pack (i32.const 0) (local.get $written)))

  (func (export "json_emit_u64") (param $value i64) (param $out_ptr i32) (param $out_cap i32) (result i64)
    (call $emit_u64_at (local.get $value) (local.get $out_ptr) (local.get $out_cap) (i32.const 0)))

  (func (export "json_emit_i64") (param $value i64) (param $out_ptr i32) (param $out_cap i32) (result i64)
    (local $written i32)
    (local $packed i64)
    (if (i64.lt_s (local.get $value) (i64.const 0))
      (then
        (local.set $packed (call $m123put (local.get $out_ptr) (local.get $out_cap) (i32.const 0) (i32.const 45)))
        (if (i32.ne (i32.wrap_i64 (local.get $packed)) (i32.const 0))
          (then (return (local.get $packed))))
        (local.set $written (i32.wrap_i64 (i64.shr_u (local.get $packed) (i64.const 32))))
        (return (call $emit_u64_at (i64.sub (i64.const 0) (local.get $value)) (local.get $out_ptr) (local.get $out_cap) (local.get $written)))))
    (call $emit_u64_at (local.get $value) (local.get $out_ptr) (local.get $out_cap) (i32.const 0)))

  (func $emit_control_escape (param $c i32) (param $out_ptr i32) (param $out_cap i32) (param $written i32) (result i64)
    (local $packed i64)
    (if (i32.eq (local.get $c) (i32.const 8)) (then (return (call $m123put (local.get $out_ptr) (local.get $out_cap) (local.get $written) (i32.const 98)))))
    (if (i32.eq (local.get $c) (i32.const 12)) (then (return (call $m123put (local.get $out_ptr) (local.get $out_cap) (local.get $written) (i32.const 102)))))
    (if (i32.eq (local.get $c) (i32.const 10)) (then (return (call $m123put (local.get $out_ptr) (local.get $out_cap) (local.get $written) (i32.const 110)))))
    (if (i32.eq (local.get $c) (i32.const 13)) (then (return (call $m123put (local.get $out_ptr) (local.get $out_cap) (local.get $written) (i32.const 114)))))
    (if (i32.eq (local.get $c) (i32.const 9)) (then (return (call $m123put (local.get $out_ptr) (local.get $out_cap) (local.get $written) (i32.const 116)))))
    (local.set $packed (call $m123put (local.get $out_ptr) (local.get $out_cap) (local.get $written) (i32.const 117)))
    (if (i32.ne (i32.wrap_i64 (local.get $packed)) (i32.const 0)) (then (return (local.get $packed))))
    (local.set $written (i32.wrap_i64 (i64.shr_u (local.get $packed) (i64.const 32))))
    (local.set $packed (call $m123put (local.get $out_ptr) (local.get $out_cap) (local.get $written) (i32.const 48)))
    (if (i32.ne (i32.wrap_i64 (local.get $packed)) (i32.const 0)) (then (return (local.get $packed))))
    (local.set $written (i32.wrap_i64 (i64.shr_u (local.get $packed) (i64.const 32))))
    (local.set $packed (call $m123put (local.get $out_ptr) (local.get $out_cap) (local.get $written) (i32.const 48)))
    (if (i32.ne (i32.wrap_i64 (local.get $packed)) (i32.const 0)) (then (return (local.get $packed))))
    (local.set $written (i32.wrap_i64 (i64.shr_u (local.get $packed) (i64.const 32))))
    (local.set $packed (call $m123put (local.get $out_ptr) (local.get $out_cap) (local.get $written) (call $m123hex_digit (i32.shr_u (local.get $c) (i32.const 4)))))
    (if (i32.ne (i32.wrap_i64 (local.get $packed)) (i32.const 0)) (then (return (local.get $packed))))
    (local.set $written (i32.wrap_i64 (i64.shr_u (local.get $packed) (i64.const 32))))
    (call $m123put (local.get $out_ptr) (local.get $out_cap) (local.get $written) (call $m123hex_digit (i32.and (local.get $c) (i32.const 15)))))

  (func $json_emit_string (export "json_emit_string") (param $in_ptr i32) (param $in_len i32) (param $out_ptr i32) (param $out_cap i32) (result i64)
    (local $i i32)
    (local $c i32)
    (local $written i32)
    (local $packed i64)
    (local.set $packed (call $m123put (local.get $out_ptr) (local.get $out_cap) (i32.const 0) (i32.const 34)))
    (if (i32.ne (i32.wrap_i64 (local.get $packed)) (i32.const 0))
      (then (return (local.get $packed))))
    (local.set $written (i32.wrap_i64 (i64.shr_u (local.get $packed) (i64.const 32))))
    (loop $again
      (if (i32.ge_u (local.get $i) (local.get $in_len))
        (then
          (local.set $packed (call $m123put (local.get $out_ptr) (local.get $out_cap) (local.get $written) (i32.const 34)))
          (return (local.get $packed))))
      (local.set $c (call $load8_u (local.get $in_ptr) (local.get $i)))
      (if (i32.eq (local.get $c) (i32.const 34))
        (then
          (local.set $packed (call $m123put (local.get $out_ptr) (local.get $out_cap) (local.get $written) (i32.const 92)))
          (if (i32.ne (i32.wrap_i64 (local.get $packed)) (i32.const 0)) (then (return (local.get $packed))))
          (local.set $written (i32.wrap_i64 (i64.shr_u (local.get $packed) (i64.const 32))))
          (local.set $packed (call $m123put (local.get $out_ptr) (local.get $out_cap) (local.get $written) (i32.const 34)))
          (if (i32.ne (i32.wrap_i64 (local.get $packed)) (i32.const 0)) (then (return (local.get $packed))))
          (local.set $written (i32.wrap_i64 (i64.shr_u (local.get $packed) (i64.const 32))))
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
          (br $again)))
      (if (i32.eq (local.get $c) (i32.const 92))
        (then
          (local.set $packed (call $m123put (local.get $out_ptr) (local.get $out_cap) (local.get $written) (i32.const 92)))
          (if (i32.ne (i32.wrap_i64 (local.get $packed)) (i32.const 0)) (then (return (local.get $packed))))
          (local.set $written (i32.wrap_i64 (i64.shr_u (local.get $packed) (i64.const 32))))
          (local.set $packed (call $m123put (local.get $out_ptr) (local.get $out_cap) (local.get $written) (i32.const 92)))
          (if (i32.ne (i32.wrap_i64 (local.get $packed)) (i32.const 0)) (then (return (local.get $packed))))
          (local.set $written (i32.wrap_i64 (i64.shr_u (local.get $packed) (i64.const 32))))
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
          (br $again)))
      (if (i32.lt_u (local.get $c) (i32.const 32))
        (then
          (local.set $packed (call $m123put (local.get $out_ptr) (local.get $out_cap) (local.get $written) (i32.const 92)))
          (if (i32.ne (i32.wrap_i64 (local.get $packed)) (i32.const 0)) (then (return (local.get $packed))))
          (local.set $written (i32.wrap_i64 (i64.shr_u (local.get $packed) (i64.const 32))))
          (local.set $packed (call $emit_control_escape (local.get $c) (local.get $out_ptr) (local.get $out_cap) (local.get $written)))
          (if (i32.ne (i32.wrap_i64 (local.get $packed)) (i32.const 0)) (then (return (local.get $packed))))
          (local.set $written (i32.wrap_i64 (i64.shr_u (local.get $packed) (i64.const 32))))
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
          (br $again)))
      (local.set $packed (call $m123put (local.get $out_ptr) (local.get $out_cap) (local.get $written) (local.get $c)))
      (if (i32.ne (i32.wrap_i64 (local.get $packed)) (i32.const 0))
        (then (return (local.get $packed))))
      (local.set $written (i32.wrap_i64 (i64.shr_u (local.get $packed) (i64.const 32))))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br $again))
    (call $pack (i32.const 0) (local.get $written)))

  (func (export "json_emit_token") (param $kind i32) (param $out_ptr i32) (param $out_cap i32) (result i64)
    ;; kind: 0 [, 1 ], 2 {, 3 }, 4 comma, 5 colon.
    (if (i32.eq (local.get $kind) (i32.const 0)) (then (return (call $m123put (local.get $out_ptr) (local.get $out_cap) (i32.const 0) (i32.const 91)))))
    (if (i32.eq (local.get $kind) (i32.const 1)) (then (return (call $m123put (local.get $out_ptr) (local.get $out_cap) (i32.const 0) (i32.const 93)))))
    (if (i32.eq (local.get $kind) (i32.const 2)) (then (return (call $m123put (local.get $out_ptr) (local.get $out_cap) (i32.const 0) (i32.const 123)))))
    (if (i32.eq (local.get $kind) (i32.const 3)) (then (return (call $m123put (local.get $out_ptr) (local.get $out_cap) (i32.const 0) (i32.const 125)))))
    (if (i32.eq (local.get $kind) (i32.const 4)) (then (return (call $m123put (local.get $out_ptr) (local.get $out_cap) (i32.const 0) (i32.const 44)))))
    (if (i32.eq (local.get $kind) (i32.const 5)) (then (return (call $m123put (local.get $out_ptr) (local.get $out_cap) (i32.const 0) (i32.const 58)))))
    (call $pack (i32.const 3) (i32.const 0)))

  (func (export "json_emit_key") (param $in_ptr i32) (param $in_len i32) (param $out_ptr i32) (param $out_cap i32) (result i64)
    (local $packed i64)
    (local $written i32)
    (local.set $packed (call $json_emit_string (local.get $in_ptr) (local.get $in_len) (local.get $out_ptr) (local.get $out_cap)))
    (if (i32.ne (i32.wrap_i64 (local.get $packed)) (i32.const 0))
      (then (return (local.get $packed))))
    (local.set $written (i32.wrap_i64 (i64.shr_u (local.get $packed) (i64.const 32))))
    (call $m123put (local.get $out_ptr) (local.get $out_cap) (local.get $written) (i32.const 58)))


  ;; Status values: 0 ok, 1 input_short, 2 output_short, 3 invalid, 4 overflow, 5 unexpected_end.

  (func $m124is_digit_1_9 (param $c i32) (result i32)
    (i32.and
      (i32.ge_u (local.get $c) (i32.const 49))
      (i32.le_u (local.get $c) (i32.const 57))))

  (func $m124hex_value (param $c i32) (result i32)
    (if (i32.and (i32.ge_u (local.get $c) (i32.const 48)) (i32.le_u (local.get $c) (i32.const 57)))
      (then (return (i32.sub (local.get $c) (i32.const 48)))))
    (if (i32.and (i32.ge_u (local.get $c) (i32.const 65)) (i32.le_u (local.get $c) (i32.const 70)))
      (then (return (i32.add (i32.sub (local.get $c) (i32.const 65)) (i32.const 10)))))
    (if (i32.and (i32.ge_u (local.get $c) (i32.const 97)) (i32.le_u (local.get $c) (i32.const 102)))
      (then (return (i32.add (i32.sub (local.get $c) (i32.const 97)) (i32.const 10)))))
    (i32.const -1))

  (func $read_hex_quad (param $ptr i32) (param $len i32) (param $off i32) (param $out_ptr i32) (result i32)
    (local $i i32)
    (local $d i32)
    (local $value i32)
    (if (i32.gt_u (i32.add (local.get $off) (i32.const 4)) (local.get $len))
      (then (return (i32.const 5))))
    (loop $again
      (local.set $d (call $m124hex_value (call $load8_u (local.get $ptr) (i32.add (local.get $off) (local.get $i)))))
      (if (i32.lt_s (local.get $d) (i32.const 0))
        (then (return (i32.const 3))))
      (local.set $value (i32.or (i32.shl (local.get $value) (i32.const 4)) (local.get $d)))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br_if $again (i32.lt_u (local.get $i) (i32.const 4))))
    (i32.store (local.get $out_ptr) (local.get $value))
    (i32.const 0))

  (func $valid_escape (param $c i32) (result i32)
    (i32.or
      (i32.or
        (i32.or (i32.eq (local.get $c) (i32.const 34)) (i32.eq (local.get $c) (i32.const 92)))
        (i32.or (i32.eq (local.get $c) (i32.const 47)) (i32.eq (local.get $c) (i32.const 98))))
      (i32.or
        (i32.or (i32.eq (local.get $c) (i32.const 102)) (i32.eq (local.get $c) (i32.const 110)))
        (i32.or (i32.eq (local.get $c) (i32.const 114)) (i32.eq (local.get $c) (i32.const 116))))))

  (func $json_scan_string_strict (export "json_scan_string_strict") (param $ptr i32) (param $len i32) (param $offset i32) (result i64)
    (local $p i32)
    (local $c i32)
    (local $esc i32)
    (local $scalar i32)
    (local $low i32)
    (if (i32.ge_u (local.get $offset) (local.get $len))
      (then (return (call $pack (i32.const 1) (local.get $offset)))))
    (if (i32.ne (call $load8_u (local.get $ptr) (local.get $offset)) (i32.const 34))
      (then (return (call $pack (i32.const 3) (local.get $offset)))))
    (local.set $p (i32.add (local.get $offset) (i32.const 1)))
    (block $done
      (loop $again
        (if (i32.ge_u (local.get $p) (local.get $len))
          (then (return (call $pack (i32.const 5) (local.get $len)))))
        (local.set $c (call $load8_u (local.get $ptr) (local.get $p)))
        (if (i32.eq (local.get $c) (i32.const 34))
          (then (return (call $pack (i32.const 0) (i32.add (local.get $p) (i32.const 1))))))
        (if (i32.lt_u (local.get $c) (i32.const 32))
          (then (return (call $pack (i32.const 3) (local.get $p)))))
        (if (i32.eq (local.get $c) (i32.const 92))
          (then
            (local.set $esc (local.get $p))
            (local.set $p (i32.add (local.get $p) (i32.const 1)))
            (if (i32.ge_u (local.get $p) (local.get $len))
              (then (return (call $pack (i32.const 5) (local.get $len)))))
            (local.set $c (call $load8_u (local.get $ptr) (local.get $p)))
            (if (i32.eq (local.get $c) (i32.const 117))
              (then
                (if (call $read_hex_quad (local.get $ptr) (local.get $len) (i32.add (local.get $p) (i32.const 1)) (i32.const 0))
                  (then (return (call $pack (i32.const 3) (local.get $esc)))))
                (local.set $scalar (i32.load (i32.const 0)))
                (local.set $p (i32.add (local.get $p) (i32.const 5)))
                (if (i32.and (i32.ge_u (local.get $scalar) (i32.const 55296)) (i32.le_u (local.get $scalar) (i32.const 56319)))
                  (then
                    (if (i32.gt_u (i32.add (local.get $p) (i32.const 6)) (local.get $len))
                      (then (return (call $pack (i32.const 3) (local.get $esc)))))
                    (if (i32.or
                          (i32.ne (call $load8_u (local.get $ptr) (local.get $p)) (i32.const 92))
                          (i32.ne (call $load8_u (local.get $ptr) (i32.add (local.get $p) (i32.const 1))) (i32.const 117)))
                      (then (return (call $pack (i32.const 3) (local.get $esc)))))
                    (if (call $read_hex_quad (local.get $ptr) (local.get $len) (i32.add (local.get $p) (i32.const 2)) (i32.const 0))
                      (then (return (call $pack (i32.const 3) (local.get $esc)))))
                    (local.set $low (i32.load (i32.const 0)))
                    (if (i32.eqz (i32.and (i32.ge_u (local.get $low) (i32.const 56320)) (i32.le_u (local.get $low) (i32.const 57343))))
                      (then (return (call $pack (i32.const 3) (local.get $esc)))))
                    (local.set $p (i32.add (local.get $p) (i32.const 6)))))
                (if (i32.and (i32.ge_u (local.get $scalar) (i32.const 56320)) (i32.le_u (local.get $scalar) (i32.const 57343)))
                  (then (return (call $pack (i32.const 3) (local.get $esc))))))
              (else
                (if (i32.eqz (call $valid_escape (local.get $c)))
                  (then (return (call $pack (i32.const 3) (local.get $esc)))))
                (local.set $p (i32.add (local.get $p) (i32.const 1))))))
          (else
            (local.set $p (i32.add (local.get $p) (i32.const 1)))))
        (br $again)))
    (call $pack (i32.const 5) (local.get $len)))

  (func $write_utf8 (param $cp i32) (param $out_ptr i32) (param $out_cap i32) (param $written i32) (result i64)
    (if (i32.le_u (local.get $cp) (i32.const 127))
      (then
        (if (i32.ge_u (local.get $written) (local.get $out_cap))
          (then (return (call $pack (i32.const 2) (local.get $written)))))
        (i32.store8 (i32.add (local.get $out_ptr) (local.get $written)) (local.get $cp))
        (return (call $pack (i32.const 0) (i32.add (local.get $written) (i32.const 1))))))
    (if (i32.le_u (local.get $cp) (i32.const 2047))
      (then
        (if (i32.gt_u (i32.add (local.get $written) (i32.const 2)) (local.get $out_cap))
          (then (return (call $pack (i32.const 2) (local.get $written)))))
        (i32.store8 (i32.add (local.get $out_ptr) (local.get $written)) (i32.or (i32.const 192) (i32.shr_u (local.get $cp) (i32.const 6))))
        (i32.store8 (i32.add (local.get $out_ptr) (i32.add (local.get $written) (i32.const 1))) (i32.or (i32.const 128) (i32.and (local.get $cp) (i32.const 63))))
        (return (call $pack (i32.const 0) (i32.add (local.get $written) (i32.const 2))))))
    (if (i32.le_u (local.get $cp) (i32.const 65535))
      (then
        (if (i32.gt_u (i32.add (local.get $written) (i32.const 3)) (local.get $out_cap))
          (then (return (call $pack (i32.const 2) (local.get $written)))))
        (i32.store8 (i32.add (local.get $out_ptr) (local.get $written)) (i32.or (i32.const 224) (i32.shr_u (local.get $cp) (i32.const 12))))
        (i32.store8 (i32.add (local.get $out_ptr) (i32.add (local.get $written) (i32.const 1))) (i32.or (i32.const 128) (i32.and (i32.shr_u (local.get $cp) (i32.const 6)) (i32.const 63))))
        (i32.store8 (i32.add (local.get $out_ptr) (i32.add (local.get $written) (i32.const 2))) (i32.or (i32.const 128) (i32.and (local.get $cp) (i32.const 63))))
        (return (call $pack (i32.const 0) (i32.add (local.get $written) (i32.const 3))))))
    (if (i32.gt_u (i32.add (local.get $written) (i32.const 4)) (local.get $out_cap))
      (then (return (call $pack (i32.const 2) (local.get $written)))))
    (i32.store8 (i32.add (local.get $out_ptr) (local.get $written)) (i32.or (i32.const 240) (i32.shr_u (local.get $cp) (i32.const 18))))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.add (local.get $written) (i32.const 1))) (i32.or (i32.const 128) (i32.and (i32.shr_u (local.get $cp) (i32.const 12)) (i32.const 63))))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.add (local.get $written) (i32.const 2))) (i32.or (i32.const 128) (i32.and (i32.shr_u (local.get $cp) (i32.const 6)) (i32.const 63))))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.add (local.get $written) (i32.const 3))) (i32.or (i32.const 128) (i32.and (local.get $cp) (i32.const 63))))
    (call $pack (i32.const 0) (i32.add (local.get $written) (i32.const 4))))

  (func $json_unescape_string (export "json_unescape_string") (param $in_ptr i32) (param $in_len i32) (param $out_ptr i32) (param $out_cap i32) (result i64)
    (local $scan i64)
    (local $p i32)
    (local $end i32)
    (local $c i32)
    (local $written i32)
    (local $scalar i32)
    (local $low i32)
    (local $packed i64)
    (local.set $scan (call $json_scan_string_strict (local.get $in_ptr) (local.get $in_len) (i32.const 0)))
    (if (i32.ne (i32.wrap_i64 (local.get $scan)) (i32.const 0))
      (then (return (local.get $scan))))
    (local.set $end (i32.sub (i32.wrap_i64 (i64.shr_u (local.get $scan) (i64.const 32))) (i32.const 1)))
    (local.set $p (i32.const 1))
    (loop $again
      (if (i32.ge_u (local.get $p) (local.get $end))
        (then (return (call $pack (i32.const 0) (local.get $written)))))
      (local.set $c (call $load8_u (local.get $in_ptr) (local.get $p)))
      (if (i32.eq (local.get $c) (i32.const 92))
        (then
          (local.set $p (i32.add (local.get $p) (i32.const 1)))
          (local.set $c (call $load8_u (local.get $in_ptr) (local.get $p)))
          (if (i32.eq (local.get $c) (i32.const 117))
            (then
              (drop (call $read_hex_quad (local.get $in_ptr) (local.get $in_len) (i32.add (local.get $p) (i32.const 1)) (i32.const 0)))
              (local.set $scalar (i32.load (i32.const 0)))
              (local.set $p (i32.add (local.get $p) (i32.const 5)))
              (if (i32.and (i32.ge_u (local.get $scalar) (i32.const 55296)) (i32.le_u (local.get $scalar) (i32.const 56319)))
                (then
                  (drop (call $read_hex_quad (local.get $in_ptr) (local.get $in_len) (i32.add (local.get $p) (i32.const 2)) (i32.const 0)))
                  (local.set $low (i32.load (i32.const 0)))
                  (local.set $scalar
                    (i32.add
                      (i32.const 65536)
                      (i32.or
                        (i32.shl (i32.sub (local.get $scalar) (i32.const 55296)) (i32.const 10))
                        (i32.sub (local.get $low) (i32.const 56320)))))
                  (local.set $p (i32.add (local.get $p) (i32.const 6)))))
              (local.set $packed (call $write_utf8 (local.get $scalar) (local.get $out_ptr) (local.get $out_cap) (local.get $written)))
              (if (i32.ne (i32.wrap_i64 (local.get $packed)) (i32.const 0))
                (then (return (local.get $packed))))
              (local.set $written (i32.wrap_i64 (i64.shr_u (local.get $packed) (i64.const 32)))))
            (else
              (if (i32.eq (local.get $c) (i32.const 34)) (then (local.set $c (i32.const 34))))
              (if (i32.eq (local.get $c) (i32.const 92)) (then (local.set $c (i32.const 92))))
              (if (i32.eq (local.get $c) (i32.const 47)) (then (local.set $c (i32.const 47))))
              (if (i32.eq (local.get $c) (i32.const 98)) (then (local.set $c (i32.const 8))))
              (if (i32.eq (local.get $c) (i32.const 102)) (then (local.set $c (i32.const 12))))
              (if (i32.eq (local.get $c) (i32.const 110)) (then (local.set $c (i32.const 10))))
              (if (i32.eq (local.get $c) (i32.const 114)) (then (local.set $c (i32.const 13))))
              (if (i32.eq (local.get $c) (i32.const 116)) (then (local.set $c (i32.const 9))))
              (if (i32.ge_u (local.get $written) (local.get $out_cap))
                (then (return (call $pack (i32.const 2) (local.get $written)))))
              (i32.store8 (i32.add (local.get $out_ptr) (local.get $written)) (local.get $c))
              (local.set $written (i32.add (local.get $written) (i32.const 1)))
              (local.set $p (i32.add (local.get $p) (i32.const 1))))))
        (else
          (if (i32.ge_u (local.get $written) (local.get $out_cap))
            (then (return (call $pack (i32.const 2) (local.get $written)))))
          (i32.store8 (i32.add (local.get $out_ptr) (local.get $written)) (local.get $c))
          (local.set $written (i32.add (local.get $written) (i32.const 1)))
          (local.set $p (i32.add (local.get $p) (i32.const 1)))))
      (br $again))
    (call $pack (i32.const 0) (local.get $written)))

  (func (export "json_scan_number_strict") (param $ptr i32) (param $len i32) (param $offset i32) (result i64)
    (local $p i32)
    (local $c i32)
    (if (i32.ge_u (local.get $offset) (local.get $len))
      (then (return (call $pack (i32.const 1) (local.get $offset)))))
    (local.set $p (local.get $offset))
    (if (i32.eq (call $load8_u (local.get $ptr) (local.get $p)) (i32.const 45))
      (then
        (local.set $p (i32.add (local.get $p) (i32.const 1)))
        (if (i32.ge_u (local.get $p) (local.get $len))
          (then (return (call $pack (i32.const 5) (local.get $p)))))))
    (local.set $c (call $load8_u (local.get $ptr) (local.get $p)))
    (if (i32.eq (local.get $c) (i32.const 48))
      (then
        (local.set $p (i32.add (local.get $p) (i32.const 1)))
        (if (i32.and (i32.lt_u (local.get $p) (local.get $len)) (call $is_digit (call $load8_u (local.get $ptr) (local.get $p))))
          (then (return (call $pack (i32.const 3) (local.get $offset))))))
      (else
        (if (i32.eqz (call $m124is_digit_1_9 (local.get $c)))
          (then (return (call $pack (i32.const 3) (local.get $offset)))))
        (loop $digits
          (local.set $p (i32.add (local.get $p) (i32.const 1)))
          (br_if $digits
            (i32.and (i32.lt_u (local.get $p) (local.get $len)) (call $is_digit (call $load8_u (local.get $ptr) (local.get $p))))))))
    (if (i32.and (i32.lt_u (local.get $p) (local.get $len)) (i32.eq (call $load8_u (local.get $ptr) (local.get $p)) (i32.const 46)))
      (then
        (local.set $p (i32.add (local.get $p) (i32.const 1)))
        (if (i32.or (i32.ge_u (local.get $p) (local.get $len)) (i32.eqz (call $is_digit (call $load8_u (local.get $ptr) (local.get $p)))))
          (then (return (call $pack (i32.const 3) (local.get $offset)))))
        (loop $frac
          (local.set $p (i32.add (local.get $p) (i32.const 1)))
          (br_if $frac
            (i32.and (i32.lt_u (local.get $p) (local.get $len)) (call $is_digit (call $load8_u (local.get $ptr) (local.get $p))))))))
    (if (i32.and
          (i32.lt_u (local.get $p) (local.get $len))
          (i32.or (i32.eq (call $load8_u (local.get $ptr) (local.get $p)) (i32.const 101)) (i32.eq (call $load8_u (local.get $ptr) (local.get $p)) (i32.const 69))))
      (then
        (local.set $p (i32.add (local.get $p) (i32.const 1)))
        (if (i32.and
              (i32.lt_u (local.get $p) (local.get $len))
              (i32.or (i32.eq (call $load8_u (local.get $ptr) (local.get $p)) (i32.const 43)) (i32.eq (call $load8_u (local.get $ptr) (local.get $p)) (i32.const 45))))
          (then (local.set $p (i32.add (local.get $p) (i32.const 1)))))
        (if (i32.or (i32.ge_u (local.get $p) (local.get $len)) (i32.eqz (call $is_digit (call $load8_u (local.get $ptr) (local.get $p)))))
          (then (return (call $pack (i32.const 3) (local.get $offset)))))
        (loop $exp
          (local.set $p (i32.add (local.get $p) (i32.const 1)))
          (br_if $exp
            (i32.and (i32.lt_u (local.get $p) (local.get $len)) (call $is_digit (call $load8_u (local.get $ptr) (local.get $p))))))))
    (call $pack (i32.const 0) (local.get $p)))

  (func (export "json_parse_u64") (param $ptr i32) (param $len i32) (param $out_ptr i32) (result i32)
    (local $i i32)
    (local $c i32)
    (local $value i64)
    (local $digit i64)
    (if (i32.eqz (local.get $len)) (then (return (i32.const 1))))
    (if (i32.and (i32.gt_u (local.get $len) (i32.const 1)) (i32.eq (call $load8_u (local.get $ptr) (i32.const 0)) (i32.const 48)))
      (then (return (i32.const 3))))
    (loop $again
      (if (i32.ge_u (local.get $i) (local.get $len))
        (then (i64.store (local.get $out_ptr) (local.get $value)) (return (i32.const 0))))
      (local.set $c (call $load8_u (local.get $ptr) (local.get $i)))
      (if (i32.eqz (call $is_digit (local.get $c))) (then (return (i32.const 3))))
      (local.set $digit (i64.extend_i32_u (i32.sub (local.get $c) (i32.const 48))))
      (if (i64.gt_u (local.get $value) (i64.const 1844674407370955161))
        (then (return (i32.const 4))))
      (local.set $value (i64.mul (local.get $value) (i64.const 10)))
      (if (i64.gt_u (local.get $value) (i64.sub (i64.const -1) (local.get $digit)))
        (then (return (i32.const 4))))
      (local.set $value (i64.add (local.get $value) (local.get $digit)))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br $again))
    (i32.const 5))

  (func (export "json_parse_i64") (param $ptr i32) (param $len i32) (param $out_ptr i32) (result i32)
    (local $i i32)
    (local $neg i32)
    (local $c i32)
    (local $value i64)
    (local $limit i64)
    (local $digit i64)
    (if (i32.eqz (local.get $len)) (then (return (i32.const 1))))
    (local.set $limit (i64.const 9223372036854775807))
    (if (i32.eq (call $load8_u (local.get $ptr) (i32.const 0)) (i32.const 45))
      (then
        (local.set $neg (i32.const 1))
        (local.set $i (i32.const 1))
    (local.set $limit (i64.const -9223372036854775808))
        (if (i32.eq (local.get $len) (i32.const 1)) (then (return (i32.const 3)))))
      (else
        (if (i32.eq (call $load8_u (local.get $ptr) (i32.const 0)) (i32.const 43))
          (then (return (i32.const 3))))))
    (if (i32.and
          (i32.gt_u (i32.sub (local.get $len) (local.get $i)) (i32.const 1))
          (i32.eq (call $load8_u (local.get $ptr) (local.get $i)) (i32.const 48)))
      (then (return (i32.const 3))))
    (loop $again
      (if (i32.ge_u (local.get $i) (local.get $len))
        (then
          (if (local.get $neg)
            (then (i64.store (local.get $out_ptr) (i64.sub (i64.const 0) (local.get $value))))
            (else (i64.store (local.get $out_ptr) (local.get $value))))
          (return (i32.const 0))))
      (local.set $c (call $load8_u (local.get $ptr) (local.get $i)))
      (if (i32.eqz (call $is_digit (local.get $c))) (then (return (i32.const 3))))
      (local.set $digit (i64.extend_i32_u (i32.sub (local.get $c) (i32.const 48))))
      (if (i64.gt_u (local.get $value) (i64.div_u (local.get $limit) (i64.const 10)))
        (then (return (i32.const 4))))
      (local.set $value (i64.mul (local.get $value) (i64.const 10)))
      (if (i64.gt_u (local.get $value) (i64.sub (local.get $limit) (local.get $digit)))
        (then (return (i32.const 4))))
      (local.set $value (i64.add (local.get $value) (local.get $digit)))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br $again))
    (i32.const 5))


  (global $in_ptr (mut i32) (i32.const 0))
  (global $in_len (mut i32) (i32.const 0))
  (global $tok_ptr (mut i32) (i32.const 0))
  (global $tok_cap (mut i32) (i32.const 0))
  (global $tok_len (mut i32) (i32.const 0))

  (func $lo (param $packed i64) (result i32)
    local.get $packed
    i32.wrap_i64)

  (func $hi (param $packed i64) (result i32)
    local.get $packed
    i64.const 32
    i64.shr_u
    i32.wrap_i64)

  (func $m125byte_at (param $off i32) (result i32)
    global.get $in_ptr
    local.get $off
    i32.add
    i32.load8_u)

  (func $m125is_ws (param $c i32) (result i32)
    local.get $c
    i32.const 32
    i32.eq
    local.get $c
    i32.const 10
    i32.eq
    i32.or
    local.get $c
    i32.const 13
    i32.eq
    i32.or
    local.get $c
    i32.const 9
    i32.eq
    i32.or)

  (func $m125skip_ws (param $off i32) (result i32)
    (local $p i32)
    local.get $off
    local.set $p
    (block $done
      (loop $again
        local.get $p
        global.get $in_len
        i32.ge_u
        br_if $done
        local.get $p
        call $m125byte_at
        call $m125is_ws
        i32.eqz
        br_if $done
        local.get $p
        i32.const 1
        i32.add
        local.set $p
        br $again))
    local.get $p)

  (func $m125is_hex (param $c i32) (result i32)
    (call $is_hex (local.get $c)))

  (func $m125is_digit_1_9 (param $c i32) (result i32)
    local.get $c
    i32.const 49
    i32.ge_u
    local.get $c
    i32.const 57
    i32.le_u
    i32.and)

  (func $is_delim (param $c i32) (result i32)
    local.get $c
    call $m125is_ws
    local.get $c
    i32.const 44
    i32.eq
    i32.or
    local.get $c
    i32.const 93
    i32.eq
    i32.or
    local.get $c
    i32.const 125
    i32.eq
    i32.or)

  (func $json_scan_string (export "json_scan_string") (param $input_ptr i32) (param $input_len i32) (param $offset i32) (result i64)
    (local $p i32)
    (local $c i32)
    (local $n i32)
    (local $i i32)
    local.get $input_ptr
    global.set $in_ptr
    local.get $input_len
    global.set $in_len
    local.get $offset
    global.get $in_len
    i32.ge_u
    if (result i64)
      i32.const 1
      local.get $offset
      call $pack
    else
      local.get $offset
      call $m125byte_at
      i32.const 34
      i32.ne
      if (result i64)
        i32.const 3
        local.get $offset
        call $pack
      else
        local.get $offset
        i32.const 1
        i32.add
        local.set $p
        (block $done
          (loop $again
            local.get $p
            global.get $in_len
            i32.ge_u
            if
              i32.const 5
              global.get $in_len
              call $pack
              return
            end
            local.get $p
            call $m125byte_at
            local.set $c
            local.get $c
            i32.const 34
            i32.eq
            if
              i32.const 0
              local.get $p
              i32.const 1
              i32.add
              call $pack
              return
            end
            local.get $c
            i32.const 32
            i32.lt_u
            if
              i32.const 3
              local.get $p
              call $pack
              return
            end
            local.get $c
            i32.const 92
            i32.eq
            if
              local.get $p
              i32.const 1
              i32.add
              local.set $n
              local.get $n
              global.get $in_len
              i32.ge_u
              if
                i32.const 5
                global.get $in_len
                call $pack
                return
              end
              local.get $n
              call $m125byte_at
              local.set $c
              local.get $c
              i32.const 34
              i32.eq
              local.get $c
              i32.const 92
              i32.eq
              i32.or
              local.get $c
              i32.const 47
              i32.eq
              i32.or
              local.get $c
              i32.const 98
              i32.eq
              i32.or
              local.get $c
              i32.const 102
              i32.eq
              i32.or
              local.get $c
              i32.const 110
              i32.eq
              i32.or
              local.get $c
              i32.const 114
              i32.eq
              i32.or
              local.get $c
              i32.const 116
              i32.eq
              i32.or
              if
                local.get $p
                i32.const 2
                i32.add
                local.set $p
                br $again
              end
              local.get $c
              i32.const 117
              i32.ne
              if
                i32.const 3
                local.get $n
                call $pack
                return
              end
              local.get $n
              i32.const 5
              i32.add
              global.get $in_len
              i32.ge_u
              if
                i32.const 5
                global.get $in_len
                call $pack
                return
              end
              i32.const 0
              local.set $i
              (block $hex_done
                (loop $hex
                  local.get $i
                  i32.const 4
                  i32.ge_u
                  br_if $hex_done
                  local.get $n
                  i32.const 1
                  i32.add
                  local.get $i
                  i32.add
                  call $m125byte_at
                  call $m125is_hex
                  i32.eqz
                  if
                    i32.const 3
                    local.get $n
                    i32.const 1
                    i32.add
                    local.get $i
                    i32.add
                    call $pack
                    return
                  end
                  local.get $i
                  i32.const 1
                  i32.add
                  local.set $i
                  br $hex))
              local.get $p
              i32.const 6
              i32.add
              local.set $p
              br $again
            end
            local.get $p
            i32.const 1
            i32.add
            local.set $p
            br $again))
        i32.const 5
        global.get $in_len
        call $pack
      end
    end)

  (func $json_scan_number (export "json_scan_number") (param $input_ptr i32) (param $input_len i32) (param $offset i32) (result i64)
    (local $p i32)
    (local $c i32)
    (local $class i32)
    local.get $input_ptr
    global.set $in_ptr
    local.get $input_len
    global.set $in_len
    local.get $offset
    global.get $in_len
    i32.ge_u
    if
      i32.const 1
      local.get $offset
      call $pack
      return
    end
    local.get $offset
    local.set $p
    i32.const 1
    local.set $class
    local.get $p
    call $m125byte_at
    i32.const 45
    i32.eq
    if
      i32.const 2
      local.set $class
      local.get $p
      i32.const 1
      i32.add
      local.set $p
      local.get $p
      global.get $in_len
      i32.ge_u
      if
        i32.const 5
        local.get $p
        call $pack
        return
      end
    end
    local.get $p
    call $m125byte_at
    local.set $c
    local.get $c
    i32.const 48
    i32.eq
    if
      local.get $p
      i32.const 1
      i32.add
      local.set $p
      local.get $p
      global.get $in_len
      i32.lt_u
      if
        local.get $p
        call $m125byte_at
        call $is_digit
        if
          i32.const 3
          local.get $p
          call $pack
          return
        end
      end
    else
      local.get $c
      call $m125is_digit_1_9
      i32.eqz
      if
        i32.const 3
        local.get $p
        call $pack
        return
      end
      local.get $p
      i32.const 1
      i32.add
      local.set $p
      (block $digits_done
        (loop $digits
          local.get $p
          global.get $in_len
          i32.ge_u
          br_if $digits_done
          local.get $p
          call $m125byte_at
          call $is_digit
          i32.eqz
          br_if $digits_done
          local.get $p
          i32.const 1
          i32.add
          local.set $p
          br $digits))
    end
    local.get $p
    global.get $in_len
    i32.lt_u
    if
      local.get $p
      call $m125byte_at
      i32.const 46
      i32.eq
      if
        i32.const 3
        local.set $class
        local.get $p
        i32.const 1
        i32.add
        local.set $p
        local.get $p
        global.get $in_len
        i32.ge_u
        if
          i32.const 5
          local.get $p
          call $pack
          return
        end
        local.get $p
        call $m125byte_at
        call $is_digit
        i32.eqz
        if
          i32.const 3
          local.get $p
          call $pack
          return
        end
        (block $frac_done
          (loop $frac
            local.get $p
            global.get $in_len
            i32.ge_u
            br_if $frac_done
            local.get $p
            call $m125byte_at
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
    global.get $in_len
    i32.lt_u
    if
      local.get $p
      call $m125byte_at
      local.set $c
      local.get $c
      i32.const 101
      i32.eq
      local.get $c
      i32.const 69
      i32.eq
      i32.or
      if
        i32.const 3
        local.set $class
        local.get $p
        i32.const 1
        i32.add
        local.set $p
        local.get $p
        global.get $in_len
        i32.ge_u
        if
          i32.const 5
          local.get $p
          call $pack
          return
        end
        local.get $p
        call $m125byte_at
        local.set $c
        local.get $c
        i32.const 43
        i32.eq
        local.get $c
        i32.const 45
        i32.eq
        i32.or
        if
          local.get $p
          i32.const 1
          i32.add
          local.set $p
          local.get $p
          global.get $in_len
          i32.ge_u
          if
            i32.const 5
            local.get $p
            call $pack
            return
          end
        end
        local.get $p
        call $m125byte_at
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
            global.get $in_len
            i32.ge_u
            br_if $exp_done
            local.get $p
            call $m125byte_at
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
    global.get $in_len
    i32.lt_u
    if
      local.get $p
      call $m125byte_at
      call $is_delim
      i32.eqz
      if
        i32.const 3
        local.get $p
        call $pack
        return
      end
    end
    i32.const 4096
    local.get $class
    i32.store
    i32.const 0
    local.get $p
    call $pack)

  (func $emit_token (param $kind i32) (param $start i32) (param $end i32) (param $parent i32) (param $flags i32) (result i64)
    (local $idx i32)
    (local $addr i32)
    global.get $tok_len
    global.get $tok_cap
    i32.ge_u
    if
      i32.const 2
      global.get $tok_len
      call $pack
      return
    end
    global.get $tok_len
    local.set $idx
    global.get $tok_ptr
    local.get $idx
    i32.const 20
    i32.mul
    i32.add
    local.set $addr
    local.get $addr
    local.get $kind
    i32.store
    local.get $addr
    i32.const 4
    i32.add
    local.get $start
    i32.store
    local.get $addr
    i32.const 8
    i32.add
    local.get $end
    i32.store
    local.get $addr
    i32.const 12
    i32.add
    local.get $parent
    i32.store
    local.get $addr
    i32.const 16
    i32.add
    local.get $flags
    i32.store
    local.get $idx
    i32.const 1
    i32.add
    global.set $tok_len
    i32.const 0
    local.get $idx
    call $pack)

  (func $set_token_end (param $idx i32) (param $end i32)
    global.get $tok_ptr
    local.get $idx
    i32.const 20
    i32.mul
    i32.add
    i32.const 8
    i32.add
    local.get $end
    i32.store)

  (func $m125match_lit (param $off i32) (param $a i32) (param $b i32) (param $c i32) (param $d i32) (param $len i32) (result i32)
    local.get $off
    local.get $len
    i32.add
    global.get $in_len
    i32.gt_u
    if
      i32.const 0
      return
    end
    local.get $off
    call $m125byte_at
    local.get $a
    i32.ne
    if
      i32.const 0
      return
    end
    local.get $len
    i32.const 1
    i32.gt_u
    if
      local.get $off
      i32.const 1
      i32.add
      call $m125byte_at
      local.get $b
      i32.ne
      if
        i32.const 0
        return
      end
    end
    local.get $len
    i32.const 2
    i32.gt_u
    if
      local.get $off
      i32.const 2
      i32.add
      call $m125byte_at
      local.get $c
      i32.ne
      if
        i32.const 0
        return
      end
    end
    local.get $len
    i32.const 3
    i32.gt_u
    if
      local.get $off
      i32.const 3
      i32.add
      call $m125byte_at
      local.get $d
      i32.ne
      if
        i32.const 0
        return
      end
    end
    i32.const 1)

  (func $parse_value (param $off i32) (param $parent i32) (param $depth i32) (result i64)
    (local $p i32)
    (local $c i32)
    (local $packed i64)
    (local $status i32)
    (local $end i32)
    local.get $depth
    i32.const 64
    i32.gt_u
    if
      i32.const 6
      local.get $off
      call $pack
      return
    end
    local.get $off
    call $m125skip_ws
    local.set $p
    local.get $p
    global.get $in_len
    i32.ge_u
    if
      i32.const 1
      local.get $p
      call $pack
      return
    end
    local.get $p
    call $m125byte_at
    local.set $c
    local.get $c
    i32.const 123
    i32.eq
    if
      local.get $p
      local.get $parent
      local.get $depth
      call $m125parse_object
      return
    end
    local.get $c
    i32.const 91
    i32.eq
    if
      local.get $p
      local.get $parent
      local.get $depth
      call $parse_array
      return
    end
    local.get $c
    i32.const 34
    i32.eq
    if
      global.get $in_ptr
      global.get $in_len
      local.get $p
      call $json_scan_string
      local.set $packed
      local.get $packed
      call $lo
      local.set $status
      local.get $packed
      call $hi
      local.set $end
      local.get $status
      i32.const 0
      i32.ne
      if
        local.get $packed
        return
      end
      i32.const 4
      local.get $p
      local.get $end
      local.get $parent
      i32.const 0
      call $emit_token
      local.set $packed
      local.get $packed
      call $lo
      if
        local.get $packed
        return
      end
      i32.const 0
      local.get $end
      call $pack
      return
    end
    local.get $c
    i32.const 45
    i32.eq
    local.get $c
    call $is_digit
    i32.or
    if
      global.get $in_ptr
      global.get $in_len
      local.get $p
      call $json_scan_number
      local.set $packed
      local.get $packed
      call $lo
      local.set $status
      local.get $packed
      call $hi
      local.set $end
      local.get $status
      i32.const 0
      i32.ne
      if
        local.get $packed
        return
      end
      i32.const 5
      local.get $p
      local.get $end
      local.get $parent
      i32.const 4096
      i32.load
      call $emit_token
      local.set $packed
      local.get $packed
      call $lo
      if
        local.get $packed
        return
      end
      i32.const 0
      local.get $end
      call $pack
      return
    end
    local.get $p
    i32.const 116
    i32.const 114
    i32.const 117
    i32.const 101
    i32.const 4
    call $m125match_lit
    if
      i32.const 6
      local.get $p
      local.get $p
      i32.const 4
      i32.add
      local.get $parent
      i32.const 1
      call $emit_token
      local.set $packed
      local.get $packed
      call $lo
      if
        local.get $packed
        return
      end
      i32.const 0
      local.get $p
      i32.const 4
      i32.add
      call $pack
      return
    end
    local.get $p
    i32.const 102
    i32.const 97
    i32.const 108
    i32.const 115
    i32.const 4
    call $m125match_lit
    if
      local.get $p
      i32.const 5
      i32.add
      global.get $in_len
      i32.le_u
      if
        local.get $p
        i32.const 4
        i32.add
        call $m125byte_at
        i32.const 101
        i32.eq
        if
          i32.const 6
          local.get $p
          local.get $p
          i32.const 5
          i32.add
          local.get $parent
          i32.const 0
          call $emit_token
          local.set $packed
          local.get $packed
          call $lo
          if
            local.get $packed
            return
          end
          i32.const 0
          local.get $p
          i32.const 5
          i32.add
          call $pack
          return
        end
      end
    end
    local.get $p
    i32.const 110
    i32.const 117
    i32.const 108
    i32.const 108
    i32.const 4
    call $m125match_lit
    if
      i32.const 7
      local.get $p
      local.get $p
      i32.const 4
      i32.add
      local.get $parent
      i32.const 0
      call $emit_token
      local.set $packed
      local.get $packed
      call $lo
      if
        local.get $packed
        return
      end
      i32.const 0
      local.get $p
      i32.const 4
      i32.add
      call $pack
      return
    end
    i32.const 3
    local.get $p
    call $pack)

  (func $m125parse_object (param $off i32) (param $parent i32) (param $depth i32) (result i64)
    (local $idx i32)
    (local $p i32)
    (local $end i32)
    (local $packed i64)
    (local $key_idx i32)
    (local $status i32)
    i32.const 1
    local.get $off
    i32.const 0
    local.get $parent
    i32.const 0
    call $emit_token
    local.set $packed
    local.get $packed
    call $lo
    if
      local.get $packed
      return
    end
    local.get $packed
    call $hi
    local.set $idx
    local.get $off
    i32.const 1
    i32.add
    call $m125skip_ws
    local.set $p
    local.get $p
    global.get $in_len
    i32.ge_u
    if
      i32.const 5
      local.get $p
      call $pack
      return
    end
    local.get $p
    call $m125byte_at
    i32.const 125
    i32.eq
    if
      local.get $idx
      local.get $p
      i32.const 1
      i32.add
      call $set_token_end
      i32.const 0
      local.get $p
      i32.const 1
      i32.add
      call $pack
      return
    end
    (block $done
      (loop $members
        local.get $p
        global.get $in_len
        i32.ge_u
        if
          i32.const 5
          local.get $p
          call $pack
          return
        end
        local.get $p
        call $m125byte_at
        i32.const 34
        i32.ne
        if
          i32.const 3
          local.get $p
          call $pack
          return
        end
        global.get $in_ptr
        global.get $in_len
        local.get $p
        call $json_scan_string
        local.set $packed
        local.get $packed
        call $lo
        local.set $status
        local.get $packed
        call $hi
        local.set $end
        local.get $status
        i32.const 0
        i32.ne
        if
          local.get $packed
          return
        end
        i32.const 3
        local.get $p
        local.get $end
        local.get $idx
        i32.const 0
        call $emit_token
        local.set $packed
        local.get $packed
        call $lo
        if
          local.get $packed
          return
        end
        local.get $packed
        call $hi
        local.set $key_idx
        local.get $end
        call $m125skip_ws
        local.set $p
        local.get $p
        global.get $in_len
        i32.ge_u
        if
          i32.const 5
          local.get $p
          call $pack
          return
        end
        local.get $p
        call $m125byte_at
        i32.const 58
        i32.ne
        if
          i32.const 3
          local.get $p
          call $pack
          return
        end
        local.get $p
        i32.const 1
        i32.add
        local.get $key_idx
        local.get $depth
        i32.const 1
        i32.add
        call $parse_value
        local.set $packed
        local.get $packed
        call $lo
        if
          local.get $packed
          return
        end
        local.get $packed
        call $hi
        call $m125skip_ws
        local.set $p
        local.get $p
        global.get $in_len
        i32.ge_u
        if
          i32.const 5
          local.get $p
          call $pack
          return
        end
        local.get $p
        call $m125byte_at
        i32.const 44
        i32.eq
        if
          local.get $p
          i32.const 1
          i32.add
          call $m125skip_ws
          local.set $p
          br $members
        end
        local.get $p
        call $m125byte_at
        i32.const 125
        i32.eq
        if
          local.get $idx
          local.get $p
          i32.const 1
          i32.add
          call $set_token_end
          i32.const 0
          local.get $p
          i32.const 1
          i32.add
          call $pack
          return
        end
        i32.const 3
        local.get $p
        call $pack
        return))
    i32.const 3
    local.get $p
    call $pack)

  (func $parse_array (param $off i32) (param $parent i32) (param $depth i32) (result i64)
    (local $idx i32)
    (local $p i32)
    (local $packed i64)
    i32.const 2
    local.get $off
    i32.const 0
    local.get $parent
    i32.const 0
    call $emit_token
    local.set $packed
    local.get $packed
    call $lo
    if
      local.get $packed
      return
    end
    local.get $packed
    call $hi
    local.set $idx
    local.get $off
    i32.const 1
    i32.add
    call $m125skip_ws
    local.set $p
    local.get $p
    global.get $in_len
    i32.ge_u
    if
      i32.const 5
      local.get $p
      call $pack
      return
    end
    local.get $p
    call $m125byte_at
    i32.const 93
    i32.eq
    if
      local.get $idx
      local.get $p
      i32.const 1
      i32.add
      call $set_token_end
      i32.const 0
      local.get $p
      i32.const 1
      i32.add
      call $pack
      return
    end
    (block $done
      (loop $items
        local.get $p
        local.get $idx
        local.get $depth
        i32.const 1
        i32.add
        call $parse_value
        local.set $packed
        local.get $packed
        call $lo
        if
          local.get $packed
          return
        end
        local.get $packed
        call $hi
        call $m125skip_ws
        local.set $p
        local.get $p
        global.get $in_len
        i32.ge_u
        if
          i32.const 5
          local.get $p
          call $pack
          return
        end
        local.get $p
        call $m125byte_at
        i32.const 44
        i32.eq
        if
          local.get $p
          i32.const 1
          i32.add
          call $m125skip_ws
          local.set $p
          br $items
        end
        local.get $p
        call $m125byte_at
        i32.const 93
        i32.eq
        if
          local.get $idx
          local.get $p
          i32.const 1
          i32.add
          call $set_token_end
          i32.const 0
          local.get $p
          i32.const 1
          i32.add
          call $pack
          return
        end
        i32.const 3
        local.get $p
        call $pack
        return))
    i32.const 3
    local.get $p
    call $pack)

  (func $json_parse_tape (export "json_parse_tape") (param $input_ptr i32) (param $input_len i32) (param $token_ptr i32) (param $token_cap i32) (param $scratch_ptr i32) (param $scratch_len i32) (result i64)
    (local $packed i64)
    (local $end i32)
    local.get $input_ptr
    global.set $in_ptr
    local.get $input_len
    global.set $in_len
    local.get $token_ptr
    global.set $tok_ptr
    local.get $token_cap
    global.set $tok_cap
    i32.const 0
    global.set $tok_len
    i32.const 0
    i32.const -1
    i32.const 0
    call $parse_value
    local.set $packed
    local.get $packed
    call $lo
    if
      local.get $packed
      return
    end
    local.get $packed
    call $hi
    call $m125skip_ws
    local.set $end
    local.get $end
    global.get $in_len
    i32.ne
    if
      i32.const 3
      global.get $tok_len
      call $pack
      return
    end
    i32.const 0
    global.get $tok_len
    call $pack)

  (data (i32.const 60000) "9223372036854775807")
  (data (i32.const 60032) "18446744073709551615")
