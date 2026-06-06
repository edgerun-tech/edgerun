

  (func (export "proto_standard_id") (result i32)
    i32.const 300015)


  (func $m146is_upper (param $b i32) (result i32)
    local.get $b
    i32.const 65
    i32.ge_u
    local.get $b
    i32.const 90
    i32.le_u
    i32.and)


  (func $is_label_separator (param $b i32) (result i32)
    local.get $b
    i32.const 32
    i32.eq
    local.get $b
    i32.const 45
    i32.eq
    i32.or)

  (func $m146is_space (param $b i32) (result i32)
    local.get $b
    i32.const 32
    i32.eq
    local.get $b
    i32.const 9
    i32.eq
    i32.or
    local.get $b
    i32.const 13
    i32.eq
    i32.or
    local.get $b
    i32.const 10
    i32.eq
    i32.or)

  (func $is_b64 (param $b i32) (result i32)
    local.get $b
    call $m146is_upper
    local.get $b
    i32.const 97
    i32.ge_u
    local.get $b
    i32.const 122
    i32.le_u
    i32.and
    i32.or
    local.get $b
    call $is_digit
    i32.or
    local.get $b
    i32.const 43
    i32.eq
    i32.or
    local.get $b
    i32.const 47
    i32.eq
    i32.or)

  (func $match_begin (param $ptr i32) (result i32)
    local.get $ptr
    i32.load8_u
    i32.const 45
    i32.eq
    local.get $ptr
    i32.const 1
    i32.add
    i32.load8_u
    i32.const 45
    i32.eq
    i32.and
    local.get $ptr
    i32.const 2
    i32.add
    i32.load8_u
    i32.const 45
    i32.eq
    i32.and
    local.get $ptr
    i32.const 3
    i32.add
    i32.load8_u
    i32.const 45
    i32.eq
    i32.and
    local.get $ptr
    i32.const 4
    i32.add
    i32.load8_u
    i32.const 45
    i32.eq
    i32.and
    local.get $ptr
    i32.const 5
    i32.add
    i32.load8_u
    i32.const 66
    i32.eq
    i32.and
    local.get $ptr
    i32.const 6
    i32.add
    i32.load8_u
    i32.const 69
    i32.eq
    i32.and
    local.get $ptr
    i32.const 7
    i32.add
    i32.load8_u
    i32.const 71
    i32.eq
    i32.and
    local.get $ptr
    i32.const 8
    i32.add
    i32.load8_u
    i32.const 73
    i32.eq
    i32.and
    local.get $ptr
    i32.const 9
    i32.add
    i32.load8_u
    i32.const 78
    i32.eq
    i32.and
    local.get $ptr
    i32.const 10
    i32.add
    i32.load8_u
    i32.const 32
    i32.eq
    i32.and)

  (func $match_end_prefix (param $ptr i32) (result i32)
    local.get $ptr
    i32.load8_u
    i32.const 45
    i32.eq
    local.get $ptr
    i32.const 1
    i32.add
    i32.load8_u
    i32.const 45
    i32.eq
    i32.and
    local.get $ptr
    i32.const 2
    i32.add
    i32.load8_u
    i32.const 45
    i32.eq
    i32.and
    local.get $ptr
    i32.const 3
    i32.add
    i32.load8_u
    i32.const 45
    i32.eq
    i32.and
    local.get $ptr
    i32.const 4
    i32.add
    i32.load8_u
    i32.const 45
    i32.eq
    i32.and
    local.get $ptr
    i32.const 5
    i32.add
    i32.load8_u
    i32.const 69
    i32.eq
    i32.and
    local.get $ptr
    i32.const 6
    i32.add
    i32.load8_u
    i32.const 78
    i32.eq
    i32.and
    local.get $ptr
    i32.const 7
    i32.add
    i32.load8_u
    i32.const 68
    i32.eq
    i32.and
    local.get $ptr
    i32.const 8
    i32.add
    i32.load8_u
    i32.const 32
    i32.eq
    i32.and)

  (func $has_five_hyphens (param $ptr i32) (result i32)
    local.get $ptr
    i32.load8_u
    i32.const 45
    i32.eq
    local.get $ptr
    i32.const 1
    i32.add
    i32.load8_u
    i32.const 45
    i32.eq
    i32.and
    local.get $ptr
    i32.const 2
    i32.add
    i32.load8_u
    i32.const 45
    i32.eq
    i32.and
    local.get $ptr
    i32.const 3
    i32.add
    i32.load8_u
    i32.const 45
    i32.eq
    i32.and
    local.get $ptr
    i32.const 4
    i32.add
    i32.load8_u
    i32.const 45
    i32.eq
    i32.and)

  (func $labels_equal
    (param $ptr i32) (param $a i32) (param $b i32) (param $len i32)
    (result i32)
    (local $i i32)
    block $bad
      loop $loop
        local.get $i
        local.get $len
        i32.ge_u
        if
          i32.const 1
          return
        end
        local.get $ptr
        local.get $a
        i32.add
        local.get $i
        i32.add
        i32.load8_u
        local.get $ptr
        local.get $b
        i32.add
        local.get $i
        i32.add
        i32.load8_u
        i32.ne
        br_if $bad
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $loop
      end
    end
    i32.const 0)

  (func $pem_validate_label (export "pem_validate_label") (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (local $b i32)
    (local $prev_sep i32)
    local.get $len
    i32.eqz
    if
      i32.const 3
      return
    end
    i32.const 1
    local.set $prev_sep
    loop $loop
      local.get $i
      local.get $len
      i32.lt_u
      if
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        local.set $b
        local.get $b
        call $m146is_upper
        local.get $b
        call $is_digit
        i32.or
        if
          i32.const 0
          local.set $prev_sep
        else
          local.get $b
          call $is_label_separator
          if
            local.get $prev_sep
            if
              i32.const 3
              return
            end
            i32.const 1
            local.set $prev_sep
          else
            i32.const 3
            return
          end
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $loop
      end
    end
    local.get $prev_sep
    if
      i32.const 3
      return
    end
    i32.const 0)

  (func (export "pem_find_boundaries")
    (param $ptr i32) (param $len i32) (param $out_ptr i32)
    (result i32)
    (local $i i32)
    (local $begin i32)
    (local $label_start i32)
    (local $label_len i32)
    (local $begin_end i32)
    (local $body_start i32)
    (local $end i32)
    (local $end_label_start i32)
    (local $end_len i32)
    local.get $len
    i32.const 16
    i32.lt_u
    if
      i32.const 1
      return
    end
    i32.const -1
    local.set $begin
    block $found_begin
      loop $find_begin
        local.get $i
        i32.const 11
        i32.add
        local.get $len
        i32.le_u
        if
          local.get $ptr
          local.get $i
          i32.add
          i32.load8_u
          i32.eqz
          if
            i32.const 3
            return
          end
          local.get $ptr
          local.get $i
          i32.add
          call $match_begin
          if
            local.get $i
            local.set $begin
            local.get $i
            i32.const 11
            i32.add
            local.set $label_start
            local.get $label_start
            local.set $i
            br $found_begin
          end
          local.get $i
          i32.const 1
          i32.add
          local.set $i
          br $find_begin
        end
      end
    end
    local.get $begin
    i32.const -1
    i32.eq
    if
      i32.const 1
      return
    end
    loop $find_begin_end
      local.get $i
      i32.const 5
      i32.add
      local.get $len
      i32.le_u
      if
        local.get $ptr
        local.get $i
        i32.add
        call $has_five_hyphens
        if
          local.get $i
          local.set $begin_end
        else
          local.get $i
          i32.const 1
          i32.add
          local.set $i
          br $find_begin_end
        end
      else
        i32.const 5
        return
      end
    end
    local.get $begin_end
    local.get $label_start
    i32.sub
    local.set $label_len
    local.get $ptr
    local.get $label_start
    i32.add
    local.get $label_len
    call $pem_validate_label
    if
      i32.const 3
      return
    end
    local.get $begin_end
    i32.const 5
    i32.add
    local.set $body_start
    local.get $body_start
    local.set $i
    i32.const -1
    local.set $end
    loop $find_end
      local.get $i
      i32.const 9
      i32.add
      local.get $len
      i32.le_u
      if
        local.get $ptr
        local.get $i
        i32.add
        call $match_end_prefix
        if
          local.get $i
          i32.const 9
          i32.add
          local.set $end_label_start
          local.get $end_label_start
          local.get $label_len
          i32.add
          i32.const 5
          i32.add
          local.get $len
          i32.le_u
          if
            local.get $ptr
            local.get $label_start
            local.get $end_label_start
            local.get $label_len
            call $labels_equal
            local.get $ptr
            local.get $end_label_start
            local.get $label_len
            i32.add
            i32.add
            call $has_five_hyphens
            i32.and
            if
              local.get $i
              local.set $end
            else
              local.get $i
              i32.const 1
              i32.add
              local.set $i
              br $find_end
            end
          else
            i32.const 5
            return
          end
        else
          local.get $i
          i32.const 1
          i32.add
          local.set $i
          br $find_end
        end
      else
        i32.const 5
        return
      end
    end
    local.get $end
    i32.const -1
    i32.eq
    if
      i32.const 5
      return
    end
    local.get $end_label_start
    local.get $label_len
    i32.add
    i32.const 5
    i32.add
    local.get $end
    i32.sub
    local.set $end_len
    local.get $out_ptr
    local.get $begin
    i32.store
    local.get $out_ptr
    i32.const 4
    i32.add
    local.get $body_start
    local.get $begin
    i32.sub
    i32.store
    local.get $out_ptr
    i32.const 8
    i32.add
    local.get $label_start
    i32.store
    local.get $out_ptr
    i32.const 12
    i32.add
    local.get $label_len
    i32.store
    local.get $out_ptr
    i32.const 16
    i32.add
    local.get $body_start
    i32.store
    local.get $out_ptr
    i32.const 20
    i32.add
    local.get $end
    local.get $body_start
    i32.sub
    i32.store
    local.get $out_ptr
    i32.const 24
    i32.add
    local.get $end
    i32.store
    local.get $out_ptr
    i32.const 28
    i32.add
    local.get $end_len
    i32.store
    local.get $out_ptr
    i32.const 32
    i32.add
    local.get $end_label_start
    i32.store
    local.get $out_ptr
    i32.const 36
    i32.add
    local.get $label_len
    i32.store
    i32.const 0)

  (func (export "pem_compact_base64")
    (param $ptr i32) (param $len i32) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (local $i i32)
    (local $o i32)
    (local $b i32)
    (local $pad i32)
    loop $loop
      local.get $i
      local.get $len
      i32.lt_u
      if
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        local.set $b
        local.get $b
        call $m146is_space
        if
        else
          local.get $b
          i32.const 58
          i32.eq
          if
            i32.const 3
            local.get $o
            call $pack
            return
          end
          local.get $b
          i32.const 61
          i32.eq
          if
            local.get $pad
            i32.const 2
            i32.ge_u
            if
              i32.const 3
              local.get $o
              call $pack
              return
            end
            local.get $pad
            i32.const 1
            i32.add
            local.set $pad
          else
            local.get $b
            call $is_b64
            i32.eqz
            if
              i32.const 3
              local.get $o
              call $pack
              return
            end
            local.get $pad
            if
              i32.const 3
              local.get $o
              call $pack
              return
            end
          end
          local.get $o
          local.get $out_cap
          i32.ge_u
          if
            i32.const 2
            local.get $o
            call $pack
            return
          end
          local.get $out_ptr
          local.get $o
          i32.add
          local.get $b
          i32.store8
          local.get $o
          i32.const 1
          i32.add
          local.set $o
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $loop
      end
    end
    local.get $o
    i32.const 4
    i32.rem_u
    i32.const 1
    i32.eq
    if
      i32.const 3
      local.get $o
      call $pack
      return
    end
    local.get $pad
    if
      local.get $o
      i32.const 4
      i32.rem_u
      if
        i32.const 3
        local.get $o
        call $pack
        return
      end
    end
    i32.const 0
    local.get $o
    call $pack)

  (func (export "pem_encoded_len")
    (param $der_len i32) (param $label_len i32)
    (result i64)
    (local $base64_len i32)
    (local $lines i32)
    (local $total i32)
    local.get $der_len
    i32.const 2147483645
    i32.gt_u
    if
      i32.const 4
      i32.const 0
      call $pack
      return
    end
    local.get $der_len
    i32.const 2
    i32.add
    i32.const 3
    i32.div_u
    i32.const 4
    i32.mul
    local.set $base64_len
    local.get $base64_len
    if
      local.get $base64_len
      i32.const 63
      i32.add
      i32.const 64
      i32.div_u
      local.set $lines
    end
    i32.const 11
    local.get $label_len
    i32.add
    i32.const 5
    i32.add
    i32.const 1
    i32.add
    local.get $base64_len
    i32.add
    local.get $lines
    i32.add
    i32.const 9
    i32.add
    local.get $label_len
    i32.add
    i32.const 5
    i32.add
    i32.const 1
    i32.add
    local.set $total
    local.get $total
    local.get $base64_len
    i32.lt_u
    if
      i32.const 4
      i32.const 0
      call $pack
      return
    end
    i32.const 0
    local.get $total
    call $pack)
