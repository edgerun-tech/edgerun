
(func (export "proto_standard_id") (result i32)
    i32.const 300009)

  (data (i32.const 65500) "18446744073709551615")

  (func $m111lower (param $b i32) (result i32)
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

  (func $m111is_space (param $b i32) (result i32)
    local.get $b
    i32.const 32
    i32.eq
    local.get $b
    i32.const 9
    i32.eq
    i32.or)

  (func $m111is_tchar (param $b i32) (result i32)
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
    i32.const 124
    i32.eq
    i32.or
    local.get $b
    i32.const 126
    i32.eq
    i32.or)

  (func $m111is_header_value_byte (param $b i32) (result i32)
    local.get $b
    i32.const 9
    i32.eq
    local.get $b
    i32.const 32
    i32.ge_u
    local.get $b
    i32.const 126
    i32.le_u
    i32.and
    i32.or)

  (func $m111ascii_eq_ci (param $aptr i32) (param $alen i32) (param $bptr i32) (param $blen i32) (result i32)
    (local $i i32)
    local.get $alen
    local.get $blen
    i32.ne
    if
      i32.const 0
      return
    end
    loop $scan
      local.get $i
      local.get $alen
      i32.ge_u
      if
        i32.const 1
        return
      end
      local.get $aptr
      local.get $i
      i32.add
      i32.load8_u
      call $m111lower
      local.get $bptr
      local.get $i
      i32.add
      i32.load8_u
      call $m111lower
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

  (func $m111parse_u64_decimal (param $ptr i32) (param $len i32) (param $out i32) (result i32)
    (local $start i32)
    (local $end i32)
    (local $digits i32)
    (local $i i32)
    (local $acc i64)
    (local $b i32)
    loop $trim_start
      local.get $start
      local.get $len
      i32.lt_u
      if
        local.get $ptr
        local.get $start
        i32.add
        i32.load8_u
        call $m111is_space
        if
          local.get $start
          i32.const 1
          i32.add
          local.set $start
          br $trim_start
        end
      end
    end
    local.get $len
    local.set $end
    loop $trim_end
      local.get $end
      local.get $start
      i32.gt_u
      if
        local.get $ptr
        local.get $end
        i32.const 1
        i32.sub
        i32.add
        i32.load8_u
        call $m111is_space
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
    i32.le_u
    if
      i32.const 3
      return
    end
    local.get $end
    local.get $start
    i32.sub
    local.tee $digits
    i32.const 20
    i32.gt_u
    if
      i32.const 4
      return
    end
    local.get $digits
    i32.const 20
    i32.eq
    if
      i32.const 0
      local.set $i
      block $max_done
        loop $max_cmp
          local.get $i
          i32.const 20
          i32.ge_u
          if
            br $max_done
          end
          local.get $ptr
          local.get $start
          local.get $i
          i32.add
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
          i32.const 65500
          local.get $i
          i32.add
          i32.load8_u
          i32.gt_u
          if
            i32.const 4
            return
          end
          local.get $b
          i32.const 65500
          local.get $i
          i32.add
          i32.load8_u
          i32.lt_u
          if
            br $max_done
          end
          local.get $i
          i32.const 1
          i32.add
          local.set $i
          br $max_cmp
        end
      end
    end
    local.get $start
    local.set $i
    loop $digits
      local.get $i
      local.get $end
      i32.ge_u
      if
        local.get $out
        local.get $acc
        i64.store
        i32.const 0
        return
      end
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
      local.get $acc
      i64.const 10
      i64.mul
      local.get $b
      i32.const 48
      i32.sub
      i64.extend_i32_u
      i64.add
      local.set $acc
      local.get $i
      i32.const 1
      i32.add
      local.set $i
      br $digits
    end
    i32.const 0)

  (func (export "http_parse_content_length") (param $ptr i32) (param $len i32) (param $out i32) (result i32)
    local.get $ptr
    local.get $len
    local.get $out
    call $m111parse_u64_decimal)

  (func $http_has_transfer_token (export "http_has_transfer_token") (param $vptr i32) (param $vlen i32) (param $tptr i32) (param $tlen i32) (result i32)
    (local $i i32)
    (local $start i32)
    (local $end i32)
    (local $b i32)
    local.get $tlen
    i32.eqz
    if
      i32.const 3
      return
    end
    i32.const 0
    local.set $i
    block $validate_done
      loop $validate
        local.get $i
        local.get $tlen
        i32.ge_u
        if
          br $validate_done
        end
        local.get $tptr
        local.get $i
        i32.add
        i32.load8_u
        call $m111is_tchar
        i32.eqz
        if
          i32.const 3
          return
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $validate
      end
    end
    i32.const 0
    local.set $i
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
          call $m111is_space
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
            local.get $b
            call $m111is_header_value_byte
            i32.eqz
            if
              i32.const 3
              return
            end
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
          i32.const 1
          i32.sub
          i32.add
          i32.load8_u
          call $m111is_space
          if
            local.get $end
            i32.const 1
            i32.sub
            local.set $end
            br $trim_end
          end
        end
      end
      local.get $vptr
      local.get $start
      i32.add
      local.get $end
      local.get $start
      i32.sub
      local.get $tptr
      local.get $tlen
      call $m111ascii_eq_ci
      if
        i32.const 0
        return
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
    i32.const 3)

  (func $m111find_crlf (param $ptr i32) (param $len i32) (param $start i32) (result i32)
    (local $i i32)
    local.get $start
    local.get $len
    i32.gt_u
    if
      i32.const -1
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
          local.get $i
          return
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $scan
      end
    end
    i32.const -1)

  (func $name_is (param $ptr i32) (param $len i32) (param $lit i32) (param $lit_len i32) (result i32)
    local.get $ptr
    local.get $len
    local.get $lit
    local.get $lit_len
    call $m111ascii_eq_ci)

  (data (i32.const 65440) "content-length")
  (data (i32.const 65456) "transfer-encoding")
  (data (i32.const 65480) "chunked")

  ;; out: kind, content_length_low, content_length_high, header_count
  ;; kind: 0 none, 1 content-length, 2 transfer-encoding chunked
  (func (export "http_classify_body_framing") (param $ptr i32) (param $len i32) (param $out i32) (result i32)
    (local $pos i32)
    (local $line_end i32)
    (local $i i32)
    (local $colon i32)
    (local $value_start i32)
    (local $value_end i32)
    (local $kind i32)
    (local $count i32)
    (local $tmp i32)
    (local $have_cl i32)
    (local $cl i64)
    (local $parsed i64)
    (local $status i32)
    block $headers_done
      loop $headers
        local.get $pos
        local.get $len
        i32.ge_u
        if
          br $headers_done
        end
        local.get $ptr
        local.get $len
        local.get $pos
        call $m111find_crlf
        local.tee $line_end
        i32.const 0
        i32.lt_s
        if
          i32.const 1
          return
        end
        local.get $line_end
        local.get $pos
        i32.eq
        if
          local.get $line_end
          i32.const 2
          i32.add
          local.set $pos
          br $headers_done
        end
      local.get $pos
      local.set $i
      i32.const -1
      local.set $colon
      loop $name
        local.get $i
        local.get $line_end
        i32.ge_u
        if
          i32.const 3
          return
        end
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        i32.const 58
        i32.eq
        if
          local.get $i
          local.set $colon
        else
          local.get $ptr
          local.get $i
          i32.add
          i32.load8_u
          call $m111is_tchar
          i32.eqz
          if
            i32.const 3
            return
          end
          local.get $i
          i32.const 1
          i32.add
          local.set $i
          br $name
        end
      end
      local.get $colon
      local.get $pos
      i32.le_s
      if
        i32.const 3
        return
      end
      local.get $colon
      i32.const 1
      i32.add
      local.set $value_start
      loop $trim_start
        local.get $value_start
        local.get $line_end
        i32.lt_u
        if
          local.get $ptr
          local.get $value_start
          i32.add
          i32.load8_u
          call $m111is_space
          if
            local.get $value_start
            i32.const 1
            i32.add
            local.set $value_start
            br $trim_start
          end
        end
      end
      local.get $line_end
      local.set $value_end
      loop $trim_end
        local.get $value_end
        local.get $value_start
        i32.gt_u
        if
          local.get $ptr
          local.get $value_end
          i32.const 1
          i32.sub
          i32.add
          i32.load8_u
          call $m111is_space
          if
            local.get $value_end
            i32.const 1
            i32.sub
            local.set $value_end
            br $trim_end
          end
        end
      end
        local.get $value_start
        local.set $i
        block $value_done
          loop $value
            local.get $i
            local.get $value_end
            i32.ge_u
            if
              br $value_done
            end
            local.get $ptr
            local.get $i
            i32.add
            i32.load8_u
            call $m111is_header_value_byte
            i32.eqz
            if
              i32.const 3
              return
            end
            local.get $i
            i32.const 1
            i32.add
            local.set $i
            br $value
          end
        end
      local.get $ptr
      local.get $pos
      i32.add
      local.get $colon
      local.get $pos
      i32.sub
      i32.const 65440
      i32.const 14
      call $name_is
      if
        local.get $ptr
        local.get $value_start
        i32.add
        local.get $value_end
        local.get $value_start
        i32.sub
        local.get $out
        i32.const 32
        i32.add
        call $m111parse_u64_decimal
        local.tee $status
        i32.const 0
        i32.ne
        if
          local.get $status
          return
        end
        local.get $out
        i32.const 32
        i32.add
        i64.load
        local.set $parsed
        local.get $have_cl
        if
          local.get $parsed
          local.get $cl
          i64.ne
          if
            i32.const 3
            return
          end
        else
          i32.const 1
          local.set $have_cl
          local.get $parsed
          local.set $cl
        end
      end
      local.get $ptr
      local.get $pos
      i32.add
      local.get $colon
      local.get $pos
      i32.sub
      i32.const 65456
      i32.const 17
      call $name_is
      if
        local.get $ptr
        local.get $value_start
        i32.add
        local.get $value_end
        local.get $value_start
        i32.sub
        i32.const 65480
        i32.const 7
        call $http_has_transfer_token
        i32.const 0
        i32.eq
        if
          i32.const 2
          local.set $kind
        end
      end
      local.get $count
      i32.const 1
      i32.add
      local.set $count
      local.get $line_end
      i32.const 2
      i32.add
      local.set $pos
      br $headers
    end
    end
    local.get $kind
    i32.eqz
    if
      local.get $have_cl
      if
        i32.const 1
        local.set $kind
      end
    end
    local.get $out
    local.get $kind
    i32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $cl
    i32.wrap_i64
    i32.store
    local.get $out
    i32.const 8
    i32.add
    local.get $cl
    i64.const 32
    i64.shr_u
    i32.wrap_i64
    i32.store
    local.get $out
    i32.const 12
    i32.add
    local.get $count
    i32.store
    i32.const 0)
