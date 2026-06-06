
  (func $m38write_record
    (param $out i32) (param $str_off i32) (param $str_len i32) (param $consumed i32)
    local.get $out
    local.get $str_off
    i32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $str_len
    i32.store
    local.get $out
    i32.const 8
    i32.add
    local.get $consumed
    i32.store)

  ;; Return status 0 when a NUL terminator is found and status 4 when the input
  ;; ends before a terminator. Offsets are relative to ptr.
  (func (export "c_string_scan")
    (param $ptr i32) (param $len i32) (param $out i32)
    (result i32)
    (local $i i32)
    (local $end i32)
    (local $consumed i32)

    local.get $ptr
    local.get $len
    i32.add
    local.set $end
    local.get $ptr
    local.set $i

    (block $truncated
      (loop $scan
        local.get $i
        local.get $end
        i32.ge_u
        br_if $truncated

        local.get $i
        i32.load8_u
        i32.eqz
        if
          local.get $i
          local.get $ptr
          i32.sub
          i32.const 1
          i32.add
          local.set $consumed
          local.get $out
          i32.const 0
          local.get $consumed
          i32.const 1
          i32.sub
          local.get $consumed
          call $m38write_record
          i32.const 0
          return
        end

        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $scan))

    local.get $out
    i32.const 0
    local.get $len
    local.get $len
    call $m38write_record
    i32.const 4)

  ;; Scan the next entry in a NUL-separated multi-string. Empty entries terminate
  ;; the list with status 5 and a next offset after the empty terminator.
  (func (export "c_multi_string_next")
    (param $ptr i32) (param $len i32) (param $offset i32) (param $out i32)
    (result i64)
    (local $i i32)
    (local $start i32)
    (local $end i32)
    (local $consumed i32)
    (local $str_len i32)
    (local $next_offset i32)

    local.get $offset
    local.get $len
    i32.ge_u
    if
      local.get $out
      local.get $len
      i32.const 0
      i32.const 0
      call $m38write_record
      i32.const 5
      local.get $len
      call $pack
      return
    end

    local.get $ptr
    local.get $offset
    i32.add
    local.set $start
    local.get $start
    local.set $i
    local.get $ptr
    local.get $len
    i32.add
    local.set $end

    (block $truncated
      (loop $scan
        local.get $i
        local.get $end
        i32.ge_u
        br_if $truncated

        local.get $i
        i32.load8_u
        i32.eqz
        if
          local.get $i
          local.get $start
          i32.sub
          local.set $str_len
          local.get $str_len
          i32.const 1
          i32.add
          local.set $consumed
          local.get $offset
          local.get $consumed
          i32.add
          local.set $next_offset

          local.get $out
          local.get $offset
          local.get $str_len
          local.get $consumed
          call $m38write_record

          local.get $str_len
          i32.eqz
          if
            i32.const 5
            local.get $next_offset
            call $pack
            return
          end

          i32.const 0
          local.get $next_offset
          call $pack
          return
        end

        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $scan))

    local.get $len
    local.get $offset
    i32.sub
    local.set $str_len
    local.get $out
    local.get $offset
    local.get $str_len
    local.get $str_len
    call $m38write_record
    i32.const 4
    local.get $len
    call $pack)
