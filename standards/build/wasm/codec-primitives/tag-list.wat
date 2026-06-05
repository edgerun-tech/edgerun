(module
  (memory (export "memory") 1)

  (func (export "proto_abi_version") (result i32)
    i32.const 2)

  (func (export "proto_standard_id") (result i32)
    i32.const 300051)

  (func $pack (param $status i32) (param $next i32) (result i64)
    local.get $status
    i64.extend_i32_u
    local.get $next
    i64.extend_i32_u
    i64.const 32
    i64.shl
    i64.or)

  (func $is_wsp (param $b i32) (result i32)
    local.get $b
    i32.const 32
    i32.eq
    local.get $b
    i32.const 9
    i32.eq
    i32.or)

  (func $is_alpha (param $b i32) (result i32)
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
    i32.or)

  (func $is_digit (param $b i32) (result i32)
    local.get $b
    i32.const 48
    i32.ge_u
    local.get $b
    i32.const 57
    i32.le_u
    i32.and)

  (func $is_tag_byte (param $b i32) (result i32)
    local.get $b
    call $is_alpha
    local.get $b
    call $is_digit
    i32.or
    local.get $b
    i32.const 95
    i32.eq
    i32.or
    local.get $b
    i32.const 45
    i32.eq
    i32.or)

  (func $write_record
    (param $out i32)
    (param $tag_off i32)
    (param $tag_len i32)
    (param $value_off i32)
    (param $value_len i32)
    local.get $out
    local.get $tag_off
    i32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $tag_len
    i32.store
    local.get $out
    i32.const 8
    i32.add
    local.get $value_off
    i32.store
    local.get $out
    i32.const 12
    i32.add
    local.get $value_len
    i32.store)

  (func (export "tag_list_next")
    (param $ptr i32) (param $len i32) (param $offset i32) (param $out i32)
    (result i64)
    (local $i i32)
    (local $b i32)
    (local $tag_start i32)
    (local $tag_end i32)
    (local $value_start i32)
    (local $value_end i32)
    (local $next i32)

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

    ;; Skip separators, empty segments, and leading whitespace.
    (block $segment_ready
      (loop $skip
        local.get $i
        local.get $len
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
        call $is_wsp
        if
          local.get $i
          i32.const 1
          i32.add
          local.set $i
          br $skip
        end
        local.get $b
        i32.const 59
        i32.eq
        if
          local.get $i
          i32.const 1
          i32.add
          local.set $i
          br $skip
        end
        br $segment_ready))

    local.get $i
    local.set $tag_start

    (block $tag_done
      (loop $tag_loop
        local.get $i
        local.get $len
        i32.ge_u
        br_if $tag_done
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        call $is_tag_byte
        i32.eqz
        br_if $tag_done
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $tag_loop))

    local.get $i
    local.set $tag_end

    local.get $tag_end
    local.get $tag_start
    i32.eq
    if
      i32.const 3
      local.get $i
      call $pack
      return
    end

    ;; Whitespace may appear before the equals sign.
    (block $before_eq_done
      (loop $before_eq
        local.get $i
        local.get $len
        i32.ge_u
        br_if $before_eq_done
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        call $is_wsp
        i32.eqz
        br_if $before_eq_done
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $before_eq))

    local.get $i
    local.get $len
    i32.ge_u
    if
      i32.const 3
      local.get $i
      call $pack
      return
    end

    local.get $ptr
    local.get $i
    i32.add
    i32.load8_u
    i32.const 61
    i32.ne
    if
      i32.const 3
      local.get $i
      call $pack
      return
    end

    local.get $i
    i32.const 1
    i32.add
    local.set $i

    ;; Trim leading value whitespace.
    (block $value_start_done
      (loop $value_start_loop
        local.get $i
        local.get $len
        i32.ge_u
        br_if $value_start_done
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        call $is_wsp
        i32.eqz
        br_if $value_start_done
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $value_start_loop))

    local.get $i
    local.set $value_start

    (block $value_done
      (loop $value_loop
        local.get $i
        local.get $len
        i32.ge_u
        br_if $value_done
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        i32.const 59
        i32.eq
        br_if $value_done
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $value_loop))

    local.get $i
    local.set $value_end

    ;; Trim trailing value whitespace.
    (block $trim_done
      (loop $trim
        local.get $value_end
        local.get $value_start
        i32.le_u
        br_if $trim_done
        local.get $ptr
        local.get $value_end
        i32.const 1
        i32.sub
        i32.add
        i32.load8_u
        call $is_wsp
        i32.eqz
        br_if $trim_done
        local.get $value_end
        i32.const 1
        i32.sub
        local.set $value_end
        br $trim))

    local.get $i
    local.get $len
    i32.lt_u
    if
      local.get $i
      i32.const 1
      i32.add
      local.set $next
    else
      local.get $len
      local.set $next
    end

    local.get $out
    local.get $tag_start
    local.get $tag_end
    local.get $tag_start
    i32.sub
    local.get $value_start
    local.get $value_end
    local.get $value_start
    i32.sub
    call $write_record

    i32.const 0
    local.get $next
    call $pack)
)
