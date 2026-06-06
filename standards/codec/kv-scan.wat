

  (func $m127is_wsp (param $b i32) (result i32)
    local.get $b
    i32.const 32
    i32.eq
    local.get $b
    i32.const 9
    i32.eq
    i32.or
    local.get $b
    i32.const 10
    i32.eq
    i32.or
    local.get $b
    i32.const 13
    i32.eq
    i32.or)

  (func $m127trim_start (param $ptr i32) (param $start i32) (param $end i32) (result i32)
    (local $i i32)
    local.get $start
    local.set $i
    (block $done
      (loop $loop
        local.get $i
        local.get $end
        i32.ge_u
        br_if $done
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        call $m127is_wsp
        i32.eqz
        br_if $done
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $loop))
    local.get $i)

  (func $m127trim_end (param $ptr i32) (param $start i32) (param $end i32) (result i32)
    (local $i i32)
    local.get $end
    local.set $i
    (block $done
      (loop $loop
        local.get $i
        local.get $start
        i32.le_u
        br_if $done
        local.get $ptr
        local.get $i
        i32.const 1
        i32.sub
        i32.add
        i32.load8_u
        call $m127is_wsp
        i32.eqz
        br_if $done
        local.get $i
        i32.const 1
        i32.sub
        local.set $i
        br $loop))
    local.get $i)

  (func $m127write2 (param $out i32) (param $off i32) (param $len i32)
    local.get $out
    local.get $off
    i32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $len
    i32.store)

  (func $m127write4
    (param $out i32)
    (param $a_off i32) (param $a_len i32)
    (param $b_off i32) (param $b_len i32)
    local.get $out
    local.get $a_off
    i32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $a_len
    i32.store
    local.get $out
    i32.const 8
    i32.add
    local.get $b_off
    i32.store
    local.get $out
    i32.const 12
    i32.add
    local.get $b_len
    i32.store)

  ;; Split on the first ':' and trim outer whitespace on both spans.
  ;; out: key_off, key_len, value_off, value_len
  (func (export "kv_colon_scan")
    (param $ptr i32) (param $len i32) (param $out i32)
    (result i32)
    (local $i i32)
    (local $key_start i32)
    (local $key_end i32)
    (local $value_start i32)
    (local $value_end i32)

    i32.const 0
    local.set $i
    (block $found
      (loop $scan
        local.get $i
        local.get $len
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
        br_if $found
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $scan))

    local.get $ptr
    i32.const 0
    local.get $i
    call $m127trim_start
    local.set $key_start
    local.get $ptr
    local.get $key_start
    local.get $i
    call $m127trim_end
    local.set $key_end
    local.get $ptr
    local.get $i
    i32.const 1
    i32.add
    local.get $len
    call $m127trim_start
    local.set $value_start
    local.get $ptr
    local.get $value_start
    local.get $len
    call $m127trim_end
    local.set $value_end

    local.get $out
    local.get $key_start
    local.get $key_end
    local.get $key_start
    i32.sub
    local.get $value_start
    local.get $value_end
    local.get $value_start
    i32.sub
    call $m127write4
    i32.const 0)

  ;; Iterate comma-separated list items from either "a, b" or "[a, b]".
  ;; offset is relative to ptr. out: item_off, item_len.
  ;; packed return: low u32 status, high u32 next_offset.
  (func (export "bracket_list_next")
    (param $ptr i32) (param $len i32) (param $offset i32) (param $out i32)
    (result i64)
    (local $i i32)
    (local $b i32)
    (local $item_start i32)
    (local $item_end i32)
    (local $next i32)
    (local $scan_end i32)

    local.get $offset
    local.get $len
    i32.gt_u
    if
      i32.const 3
      local.get $offset
      call $pack
      return
    end

    local.get $offset
    local.set $i
    local.get $len
    local.set $scan_end

    ;; First call may skip outer whitespace and an opening '['.
    local.get $i
    i32.eqz
    if
      local.get $ptr
      local.get $i
      local.get $len
      call $m127trim_start
      local.set $i
      local.get $i
      local.get $len
      i32.lt_u
      if
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        i32.const 91
        i32.eq
        if
          local.get $i
          i32.const 1
          i32.add
          local.set $i
        end
      end
    end

    ;; Find a closing ']' to bound bracketed input.
    (block $close_done
      (loop $close_scan
        local.get $i
        local.get $scan_end
        i32.ge_u
        br_if $close_done
        local.get $ptr
        local.get $scan_end
        i32.const 1
        i32.sub
        i32.add
        i32.load8_u
        call $m127is_wsp
        if
          local.get $scan_end
          i32.const 1
          i32.sub
          local.set $scan_end
          br $close_scan
        end
        local.get $ptr
        local.get $scan_end
        i32.const 1
        i32.sub
        i32.add
        i32.load8_u
        i32.const 93
        i32.eq
        if
          local.get $scan_end
          i32.const 1
          i32.sub
          local.set $scan_end
        end
        br $close_done))

    ;; Skip separators and whitespace.
    (block $item_ready
      (loop $skip
        local.get $i
        local.get $scan_end
        i32.ge_u
        if
          i32.const 5
          local.get $len
          call $pack
          return
        end
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        local.tee $b
        call $m127is_wsp
        if
          local.get $i
          i32.const 1
          i32.add
          local.set $i
          br $skip
        end
        local.get $b
        i32.const 44
        i32.eq
        if
          local.get $i
          i32.const 1
          i32.add
          local.set $i
          br $skip
        end
        br $item_ready))

    local.get $i
    local.set $item_start
    local.get $i
    local.set $item_end

    (block $item_done
      (loop $item_loop
        local.get $i
        local.get $scan_end
        i32.ge_u
        br_if $item_done
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        i32.const 44
        i32.eq
        br_if $item_done
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $item_loop))

    local.get $i
    local.set $next
    local.get $ptr
    local.get $item_start
    local.get $i
    call $m127trim_end
    local.set $item_end

    local.get $out
    local.get $item_start
    local.get $item_end
    local.get $item_start
    i32.sub
    call $m127write2

    local.get $next
    local.get $len
    i32.lt_u
    if
      local.get $next
      i32.const 1
      i32.add
      local.set $next
    end
    i32.const 0
    local.get $next
    call $pack)

  ;; Trim outer whitespace and strip matching single or double quotes.
  ;; out: span_off, span_len
  (func (export "unquote_span")
    (param $ptr i32) (param $len i32) (param $out i32)
    (result i32)
    (local $start i32)
    (local $end i32)
    (local $first i32)

    local.get $ptr
    i32.const 0
    local.get $len
    call $m127trim_start
    local.set $start
    local.get $ptr
    local.get $start
    local.get $len
    call $m127trim_end
    local.set $end

    local.get $end
    local.get $start
    i32.sub
    i32.const 2
    i32.ge_u
    if
      local.get $ptr
      local.get $start
      i32.add
      i32.load8_u
      local.set $first
      local.get $first
      i32.const 34
      i32.eq
      local.get $first
      i32.const 39
      i32.eq
      i32.or
      if
        local.get $ptr
        local.get $end
        i32.const 1
        i32.sub
        i32.add
        i32.load8_u
        local.get $first
        i32.eq
        if
          local.get $start
          i32.const 1
          i32.add
          local.set $start
          local.get $end
          i32.const 1
          i32.sub
          local.set $end
        end
      end
    end

    local.get $out
    local.get $start
    local.get $end
    local.get $start
    i32.sub
    call $m127write2
    i32.const 0)
