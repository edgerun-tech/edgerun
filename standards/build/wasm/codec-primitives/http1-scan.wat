(module
  (memory (export "memory") 1)

  (func (export "proto_abi_version") (result i32)
    i32.const 2)

  (func (export "proto_standard_id") (result i32)
    i32.const 300003)

  (func $pack (param $status i32) (param $value i32) (result i64)
    local.get $value
    i64.extend_i32_u
    i64.const 32
    i64.shl
    local.get $status
    i64.extend_i32_u
    i64.or)

  (func $is_space (param $b i32) (result i32)
    local.get $b
    i32.const 32
    i32.eq
    local.get $b
    i32.const 9
    i32.eq
    i32.or)

  (func $lower (param $b i32) (result i32)
    local.get $b
    i32.const 65
    i32.ge_u
    local.get $b
    i32.const 90
    i32.le_u
    i32.and
    if (result i32)
      local.get $b
      i32.const 32
      i32.add
    else
      local.get $b
    end)

  (func $is_tchar (param $b i32) (result i32)
    local.get $b
    i32.const 65
    i32.ge_u
    local.get $b
    i32.const 90
    i32.le_u
    i32.and
    local.get $b
    i32.const 97
    i32.ge_u
    local.get $b
    i32.const 122
    i32.le_u
    i32.and
    i32.or
    local.get $b
    i32.const 48
    i32.ge_u
    local.get $b
    i32.const 57
    i32.le_u
    i32.and
    i32.or
    local.get $b
    i32.const 33
    i32.eq
    i32.or
    local.get $b
    i32.const 35
    i32.eq
    i32.or
    local.get $b
    i32.const 36
    i32.eq
    i32.or
    local.get $b
    i32.const 37
    i32.eq
    i32.or
    local.get $b
    i32.const 38
    i32.eq
    i32.or
    local.get $b
    i32.const 39
    i32.eq
    i32.or
    local.get $b
    i32.const 42
    i32.eq
    i32.or
    local.get $b
    i32.const 43
    i32.eq
    i32.or
    local.get $b
    i32.const 45
    i32.eq
    i32.or
    local.get $b
    i32.const 46
    i32.eq
    i32.or
    local.get $b
    i32.const 94
    i32.eq
    i32.or
    local.get $b
    i32.const 95
    i32.eq
    i32.or
    local.get $b
    i32.const 96
    i32.eq
    i32.or
    local.get $b
    i32.const 123
    i32.eq
    i32.or
    local.get $b
    i32.const 124
    i32.eq
    i32.or
    local.get $b
    i32.const 125
    i32.eq
    i32.or
    local.get $b
    i32.const 126
    i32.eq
    i32.or)

  (func $is_ascii_graphic (param $b i32) (result i32)
    local.get $b
    i32.const 33
    i32.ge_u
    local.get $b
    i32.const 126
    i32.le_u
    i32.and)

  (func $hex_value (param $b i32) (result i32)
    local.get $b
    i32.const 48
    i32.ge_u
    local.get $b
    i32.const 57
    i32.le_u
    i32.and
    if (result i32)
      local.get $b
      i32.const 48
      i32.sub
    else
      local.get $b
      i32.const 65
      i32.ge_u
      local.get $b
      i32.const 70
      i32.le_u
      i32.and
      if (result i32)
        local.get $b
        i32.const 55
        i32.sub
      else
        local.get $b
        i32.const 97
        i32.ge_u
        local.get $b
        i32.const 102
        i32.le_u
        i32.and
        if (result i32)
          local.get $b
          i32.const 87
          i32.sub
        else
          i32.const -1
        end
      end
    end)

  (func (export "http_find_crlf") (param $ptr i32) (param $len i32) (param $start i32) (result i64)
    (local $i i32)
    local.get $start
    local.get $len
    i32.gt_u
    if
      i32.const 3
      local.get $len
      call $pack
      return
    end
    local.get $start
    local.set $i
    block $not_found
      loop $scan
        local.get $i
        i32.const 1
        i32.add
        local.get $len
        i32.ge_u
        br_if $not_found
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        i32.const 13
        i32.eq
        local.get $ptr
        local.get $i
        i32.add
        i32.const 1
        i32.add
        i32.load8_u
        i32.const 10
        i32.eq
        i32.and
        if
          i32.const 0
          local.get $i
          call $pack
          return
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $scan
      end
    end
    i32.const 1
    local.get $len
    call $pack)

  (func (export "http_find_double_crlf") (param $ptr i32) (param $len i32) (param $start i32) (result i64)
    (local $i i32)
    local.get $start
    local.get $len
    i32.gt_u
    if
      i32.const 3
      local.get $len
      call $pack
      return
    end
    local.get $start
    local.set $i
    block $not_found
      loop $scan
        local.get $i
        i32.const 3
        i32.add
        local.get $len
        i32.ge_u
        br_if $not_found
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        i32.const 13
        i32.eq
        local.get $ptr
        local.get $i
        i32.add
        i32.const 1
        i32.add
        i32.load8_u
        i32.const 10
        i32.eq
        i32.and
        local.get $ptr
        local.get $i
        i32.add
        i32.const 2
        i32.add
        i32.load8_u
        i32.const 13
        i32.eq
        i32.and
        local.get $ptr
        local.get $i
        i32.add
        i32.const 3
        i32.add
        i32.load8_u
        i32.const 10
        i32.eq
        i32.and
        if
          i32.const 0
          local.get $i
          call $pack
          return
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $scan
      end
    end
    i32.const 1
    local.get $len
    call $pack)

  (func (export "http_validate_header_name") (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    local.get $len
    i32.eqz
    if
      i32.const 3
      return
    end
    loop $scan
      local.get $i
      local.get $len
      i32.ge_u
      if
        i32.const 0
        return
      end
      local.get $ptr
      local.get $i
      i32.add
      i32.load8_u
      call $is_tchar
      i32.eqz
      if
        i32.const 3
        return
      end
      local.get $i
      i32.const 1
      i32.add
      local.set $i
      br $scan
    end
    i32.const 0)

  (func (export "http_validate_header_value") (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (local $b i32)
    loop $scan
      local.get $i
      local.get $len
      i32.ge_u
      if
        i32.const 0
        return
      end
      local.get $ptr
      local.get $i
      i32.add
      i32.load8_u
      local.set $b
      local.get $b
      i32.const 9
      i32.eq
      local.get $b
      i32.const 32
      i32.ge_u
      i32.or
      i32.eqz
      if
        i32.const 3
        return
      end
      local.get $i
      i32.const 1
      i32.add
      local.set $i
      br $scan
    end
    i32.const 0)

  (func (export "http_lowercase_header_name") (param $ptr i32) (param $len i32) (param $out i32) (param $cap i32) (result i64)
    (local $i i32)
    (local $b i32)
    local.get $len
    i32.eqz
    if
      i32.const 3
      i32.const 0
      call $pack
      return
    end
    local.get $len
    local.get $cap
    i32.gt_u
    if
      i32.const 2
      local.get $len
      call $pack
      return
    end
    loop $scan
      local.get $i
      local.get $len
      i32.ge_u
      if
        i32.const 0
        local.get $len
        call $pack
        return
      end
      local.get $ptr
      local.get $i
      i32.add
      i32.load8_u
      local.tee $b
      call $is_tchar
      i32.eqz
      if
        i32.const 3
        local.get $i
        call $pack
        return
      end
      local.get $out
      local.get $i
      i32.add
      local.get $b
      call $lower
      i32.store8
      local.get $i
      i32.const 1
      i32.add
      local.set $i
      br $scan
    end
    i32.const 0
    local.get $len
    call $pack)

  (func $bytes_eq_3 (param $ptr i32) (param $a i32) (param $b i32) (param $c i32) (result i32)
    local.get $ptr
    i32.load8_u
    local.get $a
    i32.eq
    local.get $ptr
    i32.const 1
    i32.add
    i32.load8_u
    local.get $b
    i32.eq
    i32.and
    local.get $ptr
    i32.const 2
    i32.add
    i32.load8_u
    local.get $c
    i32.eq
    i32.and)

  (func $bytes_eq_4 (param $ptr i32) (param $a i32) (param $b i32) (param $c i32) (param $d i32) (result i32)
    local.get $ptr
    i32.load8_u
    local.get $a
    i32.eq
    local.get $ptr
    i32.const 1
    i32.add
    i32.load8_u
    local.get $b
    i32.eq
    i32.and
    local.get $ptr
    i32.const 2
    i32.add
    i32.load8_u
    local.get $c
    i32.eq
    i32.and
    local.get $ptr
    i32.const 3
    i32.add
    i32.load8_u
    local.get $d
    i32.eq
    i32.and)

  (func $bytes_eq_5 (param $ptr i32) (param $a i32) (param $b i32) (param $c i32) (param $d i32) (param $e i32) (result i32)
    local.get $ptr
    local.get $a
    local.get $b
    local.get $c
    local.get $d
    call $bytes_eq_4
    local.get $ptr
    i32.const 4
    i32.add
    i32.load8_u
    local.get $e
    i32.eq
    i32.and)

  (func $bytes_eq_6 (param $ptr i32) (param $a i32) (param $b i32) (param $c i32) (param $d i32) (param $e i32) (param $f i32) (result i32)
    local.get $ptr
    local.get $a
    local.get $b
    local.get $c
    local.get $d
    local.get $e
    call $bytes_eq_5
    local.get $ptr
    i32.const 5
    i32.add
    i32.load8_u
    local.get $f
    i32.eq
    i32.and)

  (func $bytes_eq_7 (param $ptr i32) (param $a i32) (param $b i32) (param $c i32) (param $d i32) (param $e i32) (param $f i32) (param $g i32) (result i32)
    local.get $ptr
    local.get $a
    local.get $b
    local.get $c
    local.get $d
    local.get $e
    local.get $f
    call $bytes_eq_6
    local.get $ptr
    i32.const 6
    i32.add
    i32.load8_u
    local.get $g
    i32.eq
    i32.and)

  (func (export "http_method_classify") (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    local.get $len
    i32.eqz
    if
      i32.const 0
      return
    end
    block $valid_done
      loop $validate
        local.get $i
        local.get $len
        i32.ge_u
        if
          br $valid_done
        end
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        call $is_ascii_graphic
        i32.eqz
        if
          i32.const 0
          return
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $validate
      end
    end
    local.get $len
    i32.const 3
    i32.eq
    if
      local.get $ptr
      i32.const 71
      i32.const 69
      i32.const 84
      call $bytes_eq_3
      if
        i32.const 2
        return
      end
      local.get $ptr
      i32.const 80
      i32.const 85
      i32.const 84
      call $bytes_eq_3
      if
        i32.const 4
        return
      end
    end
    local.get $len
    i32.const 4
    i32.eq
    if
      local.get $ptr
      i32.const 80
      i32.const 79
      i32.const 83
      i32.const 84
      call $bytes_eq_4
      if
        i32.const 3
        return
      end
      local.get $ptr
      i32.const 72
      i32.const 69
      i32.const 65
      i32.const 68
      call $bytes_eq_4
      if
        i32.const 7
        return
      end
    end
    local.get $len
    i32.const 5
    i32.eq
    if
      local.get $ptr
      i32.const 80
      i32.const 65
      i32.const 84
      i32.const 67
      i32.const 72
      call $bytes_eq_5
      if
        i32.const 5
        return
      end
      local.get $ptr
      i32.const 84
      i32.const 82
      i32.const 65
      i32.const 67
      i32.const 69
      call $bytes_eq_5
      if
        i32.const 10
        return
      end
    end
    local.get $len
    i32.const 6
    i32.eq
    if
      local.get $ptr
      i32.const 68
      i32.const 69
      i32.const 76
      i32.const 69
      i32.const 84
      i32.const 69
      call $bytes_eq_6
      if
        i32.const 6
        return
      end
    end
    local.get $len
    i32.const 7
    i32.eq
    if
      local.get $ptr
      i32.const 79
      i32.const 80
      i32.const 84
      i32.const 73
      i32.const 79
      i32.const 78
      i32.const 83
      call $bytes_eq_7
      if
        i32.const 8
        return
      end
      local.get $ptr
      i32.const 67
      i32.const 79
      i32.const 78
      i32.const 78
      i32.const 69
      i32.const 67
      i32.const 84
      call $bytes_eq_7
      if
        i32.const 9
        return
      end
    end
    i32.const 1)

  (func (export "http_status_code_flags") (param $code i32) (result i32)
    local.get $code
    i32.const 100
    i32.lt_u
    local.get $code
    i32.const 999
    i32.gt_u
    i32.or
    if
      i32.const 0
      return
    end
    i32.const 1
    local.get $code
    i32.const 200
    i32.ge_u
    local.get $code
    i32.const 300
    i32.lt_u
    i32.and
    if (result i32)
      i32.const 2
    else
      i32.const 0
    end
    i32.or
    local.get $code
    i32.const 400
    i32.ge_u
    local.get $code
    i32.const 500
    i32.lt_u
    i32.and
    if (result i32)
      i32.const 4
    else
      i32.const 0
    end
    i32.or
    local.get $code
    i32.const 500
    i32.ge_u
    local.get $code
    i32.const 600
    i32.lt_u
    i32.and
    if (result i32)
      i32.const 8
    else
      i32.const 0
    end
    i32.or)

  (func $token_match (param $vptr i32) (param $tptr i32) (param $len i32) (result i32)
    (local $i i32)
    loop $scan
      local.get $i
      local.get $len
      i32.ge_u
      if
        i32.const 1
        return
      end
      local.get $vptr
      local.get $i
      i32.add
      i32.load8_u
      call $lower
      local.get $tptr
      local.get $i
      i32.add
      i32.load8_u
      call $lower
      i32.ne
      if
        i32.const 0
        return
      end
      local.get $i
      i32.const 1
      i32.add
      local.set $i
      br $scan
    end
    i32.const 1)

  (func $validate_token (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    local.get $len
    i32.eqz
    if
      i32.const 0
      return
    end
    loop $scan
      local.get $i
      local.get $len
      i32.ge_u
      if
        i32.const 1
        return
      end
      local.get $ptr
      local.get $i
      i32.add
      i32.load8_u
      call $is_tchar
      i32.eqz
      if
        i32.const 0
        return
      end
      local.get $i
      i32.const 1
      i32.add
      local.set $i
      br $scan
    end
    i32.const 1)

  (func (export "http_value_has_token") (param $vptr i32) (param $vlen i32) (param $tptr i32) (param $tlen i32) (result i32)
    (local $i i32)
    (local $start i32)
    (local $end i32)
    (local $b i32)
    local.get $tptr
    local.get $tlen
    call $validate_token
    i32.eqz
    if
      i32.const 3
      return
    end
    loop $segments
      local.get $i
      local.get $vlen
      i32.ge_u
      if
        i32.const 3
        return
      end
      loop $trim_start
        local.get $i
        local.get $vlen
        i32.lt_u
        if
          local.get $vptr
          local.get $i
          i32.add
          i32.load8_u
          call $is_space
          if
            local.get $i
            i32.const 1
            i32.add
            local.set $i
            br $trim_start
          end
        end
      end
      local.get $i
      local.set $start
      loop $find_comma
        local.get $i
        local.get $vlen
        i32.lt_u
        if
          local.get $vptr
          local.get $i
          i32.add
          i32.load8_u
          local.tee $b
          i32.const 44
          i32.ne
          if
            local.get $i
            i32.const 1
            i32.add
            local.set $i
            br $find_comma
          end
        end
      end
      local.get $i
      local.set $end
      loop $trim_end
        local.get $end
        local.get $start
        i32.gt_u
        if
          local.get $vptr
          local.get $end
          i32.add
          i32.const 1
          i32.sub
          i32.load8_u
          call $is_space
          if
            local.get $end
            i32.const 1
            i32.sub
            local.set $end
            br $trim_end
          end
        end
      end
      local.get $end
      local.get $start
      i32.sub
      local.get $tlen
      i32.eq
      if
        local.get $vptr
        local.get $start
        i32.add
        local.get $tptr
        local.get $tlen
        call $token_match
        if
          i32.const 0
          return
        end
      end
      local.get $i
      local.get $vlen
      i32.ge_u
      if
        i32.const 3
        return
      end
      local.get $i
      i32.const 1
      i32.add
      local.set $i
      br $segments
    end
    i32.const 3
    return)

  (func (export "http_parse_chunk_step") (param $ptr i32) (param $len i32) (param $start i32) (param $out i32) (result i32)
    (local $i i32)
    (local $size i32)
    (local $digit i32)
    (local $digits i32)
    (local $data_off i32)
    (local $next i32)
    local.get $start
    local.get $len
    i32.ge_u
    if
      i32.const 1
      return
    end
    local.get $start
    local.set $i
    block $hex_done
      loop $hex
        local.get $i
        local.get $len
        i32.ge_u
        if
          i32.const 1
          return
        end
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        local.tee $digit
        i32.const 59
        i32.eq
        local.get $digit
        i32.const 13
        i32.eq
        i32.or
        if
          br $hex_done
        end
        local.get $digit
        call $hex_value
        local.tee $digit
        i32.const 0
        i32.lt_s
        if
          i32.const 3
          return
        end
        local.get $size
        i32.const 0x0fffffff
        i32.gt_u
        if
          i32.const 4
          return
        end
        local.get $size
        i32.const 4
        i32.shl
        local.get $digit
        i32.or
        local.set $size
        local.get $digits
        i32.const 1
        i32.add
        local.set $digits
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $hex
      end
    end
    local.get $digits
    i32.eqz
    if
      i32.const 3
      return
    end
    block $line_done
      loop $line
        local.get $i
        i32.const 1
        i32.add
        local.get $len
        i32.ge_u
        if
          i32.const 1
          return
        end
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        i32.const 13
        i32.eq
        local.get $ptr
        local.get $i
        i32.add
        i32.const 1
        i32.add
        i32.load8_u
        i32.const 10
        i32.eq
        i32.and
        if
          local.get $i
          i32.const 2
          i32.add
          local.set $data_off
          br $line_done
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $line
      end
    end
    local.get $size
    i32.eqz
    if
      local.get $data_off
      i32.const 1
      i32.add
      local.get $len
      i32.ge_u
      if
        i32.const 1
        return
      end
      local.get $ptr
      local.get $data_off
      i32.add
      i32.load8_u
      i32.const 13
      i32.eq
      local.get $ptr
      local.get $data_off
      i32.add
      i32.const 1
      i32.add
      i32.load8_u
      i32.const 10
      i32.eq
      i32.and
      if
        local.get $data_off
        i32.const 2
        i32.add
        local.set $next
      else
        local.get $data_off
        local.set $i
        block $trail_done
          loop $trail
            local.get $i
            i32.const 3
            i32.add
            local.get $len
            i32.ge_u
            if
              i32.const 1
              return
            end
            local.get $ptr
            local.get $i
            i32.add
            i32.load8_u
            i32.const 13
            i32.eq
            local.get $ptr
            local.get $i
            i32.add
            i32.const 1
            i32.add
            i32.load8_u
            i32.const 10
            i32.eq
            i32.and
            local.get $ptr
            local.get $i
            i32.add
            i32.const 2
            i32.add
            i32.load8_u
            i32.const 13
            i32.eq
            i32.and
            local.get $ptr
            local.get $i
            i32.add
            i32.const 3
            i32.add
            i32.load8_u
            i32.const 10
            i32.eq
            i32.and
            if
              local.get $i
              i32.const 4
              i32.add
              local.set $next
              br $trail_done
            end
            local.get $i
            i32.const 1
            i32.add
            local.set $i
            br $trail
          end
        end
      end
      local.get $out
      local.get $size
      i32.store
      local.get $out
      i32.const 4
      i32.add
      local.get $data_off
      i32.store
      local.get $out
      i32.const 8
      i32.add
      i32.const 0
      i32.store
      local.get $out
      i32.const 12
      i32.add
      local.get $next
      i32.store
      local.get $out
      i32.const 16
      i32.add
      i32.const 1
      i32.store
      i32.const 0
      return
    end
    local.get $data_off
    local.get $size
    i32.add
    i32.const 1
    i32.add
    local.get $len
    i32.ge_u
    if
      i32.const 1
      return
    end
    local.get $ptr
    local.get $data_off
    local.get $size
    i32.add
    i32.add
    i32.load8_u
    i32.const 13
    i32.eq
    local.get $ptr
    local.get $data_off
    local.get $size
    i32.add
    i32.add
    i32.const 1
    i32.add
    i32.load8_u
    i32.const 10
    i32.eq
    i32.and
    i32.eqz
    if
      i32.const 3
      return
    end
    local.get $data_off
    local.get $size
    i32.add
    i32.const 2
    i32.add
    local.set $next
    local.get $out
    local.get $size
    i32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $data_off
    i32.store
    local.get $out
    i32.const 8
    i32.add
    local.get $size
    i32.store
    local.get $out
    i32.const 12
    i32.add
    local.get $next
    i32.store
    local.get $out
    i32.const 16
    i32.add
    i32.const 0
    i32.store
    i32.const 0)

  (data (i32.const 8192) "\00\00\00\00\00\00\00\00\00\02\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\02\21\20\21\21\21\21\21\20\20\21\21\20\21\21\20\35\35\35\35\35\35\35\35\35\35\20\20\20\20\20\20\20\39\39\39\39\39\39\29\29\29\29\29\29\29\29\29\29\29\29\29\29\29\29\29\29\29\29\20\20\20\21\21\21\39\39\39\39\39\39\29\29\29\29\29\29\29\29\29\29\29\29\29\29\29\29\29\29\29\29\20\21\20\21\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00")

  (func (export "simd_capabilities") (result i32)
    i32.const 1)

  (func (export "http_find_crlf_simd") (param $ptr i32) (param $len i32) (param $start i32) (result i64)
    (local $i i32)
    (local $v v128)
    local.get $start
    local.get $len
    i32.gt_u
    if
      i32.const 3
      local.get $len
      call $pack
      return
    end
    local.get $start
    local.set $i
    block $scalar
      loop $simd
        local.get $i
        i32.const 16
        i32.add
        local.get $len
        i32.gt_u
        br_if $scalar
        local.get $ptr
        local.get $i
        i32.add
        v128.load
        local.tee $v
        i32.const 0x0D
        i8x16.splat
        i8x16.eq
        local.get $v
        i32.const 0x0A
        i8x16.splat
        i8x16.eq
        v128.or
        v128.any_true
        br_if $scalar
        local.get $i
        i32.const 16
        i32.add
        local.set $i
        br $simd
      end
    end
    block $not_found
      loop $scan
        local.get $i
        local.get $len
        i32.ge_u
        br_if $not_found
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        i32.const 13
        i32.eq
        if
          local.get $i
          i32.const 1
          i32.add
          local.get $len
          i32.lt_u
          if
            local.get $ptr
            local.get $i
            i32.add
            i32.const 1
            i32.add
            i32.load8_u
            i32.const 10
            i32.eq
            if
              i32.const 0
              local.get $i
              call $pack
              return
            end
          end
        end
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        i32.const 10
        i32.eq
        if
          i32.const 0
          local.get $i
          call $pack
          return
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $scan
      end
    end
    i32.const 1
    local.get $len
    call $pack)

  (func (export "http_find_double_crlf_simd") (param $ptr i32) (param $len i32) (param $start i32) (result i64)
    (local $i i32)
    (local $v v128)
    local.get $start
    local.get $len
    i32.gt_u
    if
      i32.const 3
      local.get $len
      call $pack
      return
    end
    local.get $start
    local.set $i
    block $scalar
      loop $simd
        local.get $i
        i32.const 16
        i32.add
        local.get $len
        i32.gt_u
        br_if $scalar
        local.get $ptr
        local.get $i
        i32.add
        v128.load
        local.tee $v
        i32.const 0x0D
        i8x16.splat
        i8x16.eq
        local.get $v
        i32.const 0x0A
        i8x16.splat
        i8x16.eq
        v128.or
        v128.any_true
        br_if $scalar
        local.get $i
        i32.const 16
        i32.add
        local.set $i
        br $simd
      end
    end
    block $not_found
      loop $scan
        local.get $i
        i32.const 3
        i32.add
        local.get $len
        i32.ge_u
        br_if $not_found
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        i32.const 13
        i32.eq
        local.get $ptr
        local.get $i
        i32.add
        i32.const 1
        i32.add
        i32.load8_u
        i32.const 10
        i32.eq
        i32.and
        local.get $ptr
        local.get $i
        i32.add
        i32.const 2
        i32.add
        i32.load8_u
        i32.const 13
        i32.eq
        i32.and
        local.get $ptr
        local.get $i
        i32.add
        i32.const 3
        i32.add
        i32.load8_u
        i32.const 10
        i32.eq
        i32.and
        if
          i32.const 0
          local.get $i
          call $pack
          return
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $scan
      end
    end
    i32.const 1
    local.get $len
    call $pack)

  (func (export "http_method_classify_simd") (param $ptr i32) (param $len i32) (result i32)
    local.get $ptr
    local.get $len
    call 18)
)
