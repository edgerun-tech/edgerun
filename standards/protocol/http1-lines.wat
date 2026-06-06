  (import "http" "is_tchar" (func $is_tchar (param i32) (result i32)))
  (import "http" "is_space" (func $is_space (param i32) (result i32)))
  (import "http" "is_header_value_byte" (func $is_header_value_byte (param i32) (result i32)))

(func $m114find_crlf (param $ptr i32) (param $len i32) (param $start i32) (result i32)
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

  (func $starts_http (param $ptr i32) (param $off i32) (param $end i32) (result i32)
    local.get $off
    i32.const 8
    i32.add
    local.get $end
    i32.ne
    if
      i32.const 0
      return
    end
    local.get $ptr
    local.get $off
    i32.add
    i32.load8_u
    i32.const 72
    i32.eq
    local.get $ptr
    local.get $off
    i32.add
    i32.const 1
    i32.add
    i32.load8_u
    i32.const 84
    i32.eq
    i32.and
    local.get $ptr
    local.get $off
    i32.add
    i32.const 2
    i32.add
    i32.load8_u
    i32.const 84
    i32.eq
    i32.and
    local.get $ptr
    local.get $off
    i32.add
    i32.const 3
    i32.add
    i32.load8_u
    i32.const 80
    i32.eq
    i32.and
    local.get $ptr
    local.get $off
    i32.add
    i32.const 4
    i32.add
    i32.load8_u
    i32.const 47
    i32.eq
    i32.and
    local.get $ptr
    local.get $off
    i32.add
    i32.const 5
    i32.add
    i32.load8_u
    call $is_digit
    i32.and
    local.get $ptr
    local.get $off
    i32.add
    i32.const 6
    i32.add
    i32.load8_u
    i32.const 46
    i32.eq
    i32.and
    local.get $ptr
    local.get $off
    i32.add
    i32.const 7
    i32.add
    i32.load8_u
    call $is_digit
    i32.and)

  ;; out: method_off, method_len, target_off, target_len, major, minor, next_off
  (func (export "http_parse_request_line") (param $ptr i32) (param $len i32) (param $out i32) (result i32)
    (local $line_end i32)
    (local $i i32)
    (local $method_end i32)
    (local $target_start i32)
    (local $target_end i32)
    (local $b i32)
    local.get $ptr
    local.get $len
    i32.const 0
    call $m114find_crlf
    local.tee $line_end
    i32.const 0
    i32.lt_s
    if
      i32.const 1
      return
    end
    local.get $line_end
    i32.eqz
    if
      i32.const 3
      return
    end
    loop $method
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
      local.tee $b
      i32.const 32
      i32.eq
      if
        local.get $i
        local.set $method_end
      else
        local.get $b
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
        br $method
      end
    end
    local.get $method_end
    i32.eqz
    if
      i32.const 3
      return
    end
    local.get $method_end
    i32.const 1
    i32.add
    local.set $target_start
    local.get $target_start
    local.set $i
    loop $target
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
      i32.const 32
      i32.eq
      if
        local.get $i
        local.set $target_end
      else
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $target
      end
    end
    local.get $target_end
    local.get $target_start
    i32.le_u
    if
      i32.const 3
      return
    end
    local.get $ptr
    local.get $target_end
    i32.const 1
    i32.add
    local.get $line_end
    call $starts_http
    i32.eqz
    if
      i32.const 3
      return
    end
    local.get $out
    i32.const 0
    i32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $method_end
    i32.store
    local.get $out
    i32.const 8
    i32.add
    local.get $target_start
    i32.store
    local.get $out
    i32.const 12
    i32.add
    local.get $target_end
    local.get $target_start
    i32.sub
    i32.store
    local.get $out
    i32.const 16
    i32.add
    local.get $ptr
    local.get $target_end
    i32.add
    i32.const 6
    i32.add
    i32.load8_u
    i32.const 48
    i32.sub
    i32.store
    local.get $out
    i32.const 20
    i32.add
    local.get $ptr
    local.get $target_end
    i32.add
    i32.const 8
    i32.add
    i32.load8_u
    i32.const 48
    i32.sub
    i32.store
    local.get $out
    i32.const 24
    i32.add
    local.get $line_end
    i32.const 2
    i32.add
    i32.store
    i32.const 0)

  ;; out: major, minor, status, reason_off, reason_len, next_off
  (func (export "http_parse_status_line") (param $ptr i32) (param $len i32) (param $out i32) (result i32)
    (local $line_end i32)
    (local $code i32)
    local.get $ptr
    local.get $len
    i32.const 0
    call $m114find_crlf
    local.tee $line_end
    i32.const 0
    i32.lt_s
    if
      i32.const 1
      return
    end
    local.get $ptr
    i32.const 0
    i32.const 8
    call $starts_http
    i32.eqz
    if
      i32.const 3
      return
    end
    local.get $line_end
    i32.const 12
    i32.lt_u
    if
      i32.const 3
      return
    end
    local.get $ptr
    i32.const 8
    i32.add
    i32.load8_u
    i32.const 32
    i32.ne
    if
      i32.const 3
      return
    end
    local.get $ptr
    i32.const 9
    i32.add
    i32.load8_u
    call $is_digit
    local.get $ptr
    i32.const 10
    i32.add
    i32.load8_u
    call $is_digit
    i32.and
    local.get $ptr
    i32.const 11
    i32.add
    i32.load8_u
    call $is_digit
    i32.and
    i32.eqz
    if
      i32.const 3
      return
    end
    local.get $ptr
    i32.const 9
    i32.add
    i32.load8_u
    i32.const 48
    i32.sub
    i32.const 100
    i32.mul
    local.get $ptr
    i32.const 10
    i32.add
    i32.load8_u
    i32.const 48
    i32.sub
    i32.const 10
    i32.mul
    i32.add
    local.get $ptr
    i32.const 11
    i32.add
    i32.load8_u
    i32.const 48
    i32.sub
    i32.add
    local.tee $code
    i32.const 100
    i32.lt_u
    local.get $code
    i32.const 600
    i32.ge_u
    i32.or
    if
      i32.const 3
      return
    end
    local.get $line_end
    i32.const 12
    i32.gt_u
    if
      local.get $ptr
      i32.const 12
      i32.add
      i32.load8_u
      i32.const 32
      i32.ne
      if
        i32.const 3
        return
      end
    end
    local.get $out
    local.get $ptr
    i32.const 5
    i32.add
    i32.load8_u
    i32.const 48
    i32.sub
    i32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $ptr
    i32.const 7
    i32.add
    i32.load8_u
    i32.const 48
    i32.sub
    i32.store
    local.get $out
    i32.const 8
    i32.add
    local.get $code
    i32.store
    local.get $out
    i32.const 12
    i32.add
    i32.const 13
    i32.store
    local.get $out
    i32.const 16
    i32.add
    local.get $line_end
    i32.const 13
    i32.gt_u
    if (result i32)
      local.get $line_end
      i32.const 13
      i32.sub
    else
      i32.const 0
    end
    i32.store
    local.get $out
    i32.const 20
    i32.add
    local.get $line_end
    i32.const 2
    i32.add
    i32.store
    i32.const 0)

  ;; out: name_off, name_len, value_off, value_len, next_off, is_end
  (func (export "http_next_header") (param $ptr i32) (param $len i32) (param $start i32) (param $out i32) (result i32)
    (local $line_end i32)
    (local $i i32)
    (local $colon i32)
    (local $value_start i32)
    (local $value_end i32)
    (local $b i32)
    local.get $start
    local.get $len
    i32.gt_u
    if
      i32.const 3
      return
    end
    local.get $ptr
    local.get $len
    local.get $start
    call $m114find_crlf
    local.tee $line_end
    i32.const 0
    i32.lt_s
    if
      i32.const 1
      return
    end
    local.get $line_end
    local.get $start
    i32.eq
    if
      local.get $out
      i32.const 0
      i32.store
      local.get $out
      i32.const 4
      i32.add
      i32.const 0
      i32.store
      local.get $out
      i32.const 8
      i32.add
      i32.const 0
      i32.store
      local.get $out
      i32.const 12
      i32.add
      i32.const 0
      i32.store
      local.get $out
      i32.const 16
      i32.add
      local.get $line_end
      i32.const 2
      i32.add
      i32.store
      local.get $out
      i32.const 20
      i32.add
      i32.const 1
      i32.store
      i32.const 0
      return
    end
    local.get $start
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
      local.tee $b
      i32.const 58
      i32.eq
      if
        local.get $i
        local.set $colon
      else
        local.get $b
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
        br $name
      end
    end
    local.get $colon
    local.get $start
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
        call $is_space
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
        call $is_space
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
    loop $value
      local.get $i
      local.get $value_end
      i32.ge_u
      if
        local.get $out
        local.get $start
        i32.store
        local.get $out
        i32.const 4
        i32.add
        local.get $colon
        local.get $start
        i32.sub
        i32.store
        local.get $out
        i32.const 8
        i32.add
        local.get $value_start
        i32.store
        local.get $out
        i32.const 12
        i32.add
        local.get $value_end
        local.get $value_start
        i32.sub
        i32.store
        local.get $out
        i32.const 16
        i32.add
        local.get $line_end
        i32.const 2
        i32.add
        i32.store
        local.get $out
        i32.const 20
        i32.add
        i32.const 0
        i32.store
        i32.const 0
        return
      end
      local.get $ptr
      local.get $i
      i32.add
      i32.load8_u
      call $is_header_value_byte
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
    i32.const 0)
