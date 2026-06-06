;; UUID parse and format.
  ;; Format: 8-4-4-4-12 hex digits (36 characters).
  ;;
  ;; Exports:
  ;;   uuid_parse(in_ptr, out_ptr) -> i32  (16 bytes, 0 on success, -1 on error)
  ;;   uuid_format(in_ptr, out_ptr) -> i32  (36 chars written, -1 on error)
  (func $parse_hex_byte (param $ptr i32) (param $off i32) (result i32)
    (local $h i32) (local $l i32)
    local.get $ptr local.get $off i32.add i32.load8_u call $parse_hex_digit
    local.tee $h
    i32.const 0 i32.lt_s if
      i32.const -1 return
    end
    local.get $ptr local.get $off i32.const 1 i32.add     i32.add i32.load8_u call $parse_hex_digit
    local.tee $l
    i32.const 0 i32.lt_s if
      i32.const -1 return
    end
    local.get $h i32.const 4 i32.shl local.get $l i32.or
  )

  (func $dash_at (param $b i32) (result i32)
    local.get $b i32.const 4 i32.eq
    local.get $b i32.const 6 i32.eq i32.or
    local.get $b i32.const 8 i32.eq i32.or
    local.get $b i32.const 10 i32.eq i32.or
  )

  (func (export "uuid_parse") (param $in i32) (param $out i32) (result i32)
    (local $i i32) (local $o i32) (local $b i32)
    i32.const 0 local.set $i
    i32.const 0 local.set $o
    block $done
    loop $loop
      local.get $o i32.const 16 i32.ge_u br_if $done
      local.get $o call $dash_at
      if
        local.get $in local.get $i i32.add i32.load8_u i32.const 45 i32.ne
        if
          i32.const -1 return
        end
        local.get $i i32.const 1 i32.add local.set $i
      end
      local.get $in local.get $i call $parse_hex_byte
      local.tee $b
      i32.const 0 i32.lt_s if
        i32.const -1 return
      end
      local.get $out local.get $o i32.add local.get $b i32.store8
      local.get $o i32.const 1 i32.add local.set $o
      local.get $i i32.const 2 i32.add local.set $i
      br $loop
    end
    end
    i32.const 0
  )

  (func (export "uuid_format") (param $in i32) (param $out i32) (result i32)
    (local $i i32) (local $o i32) (local $b i32)
    i32.const 0 local.set $i
    i32.const 0 local.set $o
    block $done
    loop $loop
      local.get $i i32.const 16 i32.ge_u br_if $done
      local.get $i call $dash_at
      if
        local.get $out local.get $o i32.add i32.const 45 i32.store8
        local.get $o i32.const 1 i32.add local.set $o
      end
      local.get $in local.get $i i32.add i32.load8_u local.set $b
      local.get $b i32.const 4 i32.shr_u local.get $out local.get $o i32.add call $hex_digit
      local.get $b i32.const 0xf i32.and local.get $out local.get $o i32.const 1 i32.add i32.add call $hex_digit
      local.get $o i32.const 2 i32.add local.set $o
      local.get $i i32.const 1 i32.add local.set $i
      br $loop
    end
    end
    local.get $o
  )
