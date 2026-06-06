
  ;; OSRS cache smart-int encoding — prefix-variable integer formats.
  ;;
  ;; Short smart:  peek < 128 →  1 byte  (unsigned or signed with offset)
  ;;               peek ≥ 128 →  2 bytes (unsigned short − offset)
  ;; Big smart:    peek ≥ 0   →  2 bytes (unsigned short)
  ;;               peek < 0   →  4 bytes (int & MAX_VALUE)
  ;;
  ;; All decode functions take (buf, off, result_out) and return new offset.
  ;; Scratch byte at memory[0..4] for intermediate storage in compat function.
  (func (export "read_short_smart") (param $buf i32) (param $off i32) (param $out i32) (result i32)
    (local $peek i32)
    local.get $buf local.get $off i32.add i32.load8_u local.tee $peek
    i32.const 128 i32.lt_u
    if
      local.get $out local.get $peek i32.const 64 i32.sub i32.store
      local.get $off i32.const 1 i32.add
      return
    end
    local.get $out
    local.get $buf local.get $off i32.add i32.load16_u i32.const 0xC000 i32.sub i32.store
    local.get $off i32.const 2 i32.add
  )

  (func (export "read_unsigned_short_smart") (param $buf i32) (param $off i32) (param $out i32) (result i32)
    (local $peek i32)
    local.get $buf local.get $off i32.add i32.load8_u local.tee $peek
    i32.const 128 i32.lt_u
    if
      local.get $out local.get $peek i32.store
      local.get $off i32.const 1 i32.add
      return
    end
    local.get $out
    local.get $buf local.get $off i32.add i32.load16_u i32.const 0x8000 i32.sub i32.store
    local.get $off i32.const 2 i32.add
  )

  (func (export "read_unsigned_short_smart_minus_one") (param $buf i32) (param $off i32) (param $out i32) (result i32)
    (local $peek i32)
    local.get $buf local.get $off i32.add i32.load8_u local.tee $peek
    i32.const 128 i32.lt_u
    if
      local.get $out local.get $peek i32.const 1 i32.sub i32.store
      local.get $off i32.const 1 i32.add
      return
    end
    local.get $out
    local.get $buf local.get $off i32.add i32.load16_u i32.const 0x8001 i32.sub i32.store
    local.get $off i32.const 2 i32.add
  )

  (func (export "read_unsigned_int_smart_short_compat") (param $buf i32) (param $off i32) (param $out i32) (result i32)
    (local $total i32) (local $peek i32) (local $val i32)
    i32.const 0 local.set $total
    block $done
    loop $loop
      local.get $buf local.get $off i32.add i32.load8_u local.tee $peek
      i32.const 128 i32.lt_u
      if
        local.get $peek local.set $val
        local.get $off i32.const 1 i32.add local.set $off
      else
        local.get $buf local.get $off i32.add i32.load16_u i32.const 0x8000 i32.sub local.set $val
        local.get $off i32.const 2 i32.add local.set $off
      end
      local.get $val i32.const 32767 i32.ne
      if
        local.get $total local.get $val i32.add local.set $total
        br $done
      end
      local.get $total i32.const 32767 i32.add local.set $total
      br $loop
    end
    end
    local.get $out local.get $total i32.store
    local.get $off
  )

  (func (export "read_big_smart") (param $buf i32) (param $off i32) (param $out i32) (result i32)
    local.get $buf local.get $off i32.add i32.load8_s i32.const 0 i32.ge_s
    if
      local.get $out local.get $buf local.get $off i32.add i32.load16_u i32.store
      local.get $off i32.const 2 i32.add
      return
    end
    local.get $out
    local.get $buf local.get $off i32.add i32.load i32.const 0x7FFFFFFF i32.and i32.store
    local.get $off i32.const 4 i32.add
  )

  (func (export "read_big_smart2") (param $buf i32) (param $off i32) (param $out i32) (result i32)
    (local $val i32)
    local.get $buf local.get $off i32.add i32.load8_s i32.const 0 i32.lt_s
    if
      local.get $out
      local.get $buf local.get $off i32.add i32.load i32.const 0x7FFFFFFF i32.and i32.store
      local.get $off i32.const 4 i32.add
      return
    end
    local.get $buf local.get $off i32.add i32.load16_u local.set $val
    local.get $val i32.const 32767 i32.eq
    if
      local.get $out i32.const -1 i32.store
    else
      local.get $out local.get $val i32.store
    end
    local.get $off i32.const 2 i32.add
  )

  (func (export "read_24bit_int") (param $buf i32) (param $off i32) (result i32)
    local.get $buf local.get $off i32.add i32.load8_u i32.const 16 i32.shl
    local.get $buf local.get $off i32.const 1 i32.add i32.add i32.load8_u i32.const 8 i32.shl i32.or
    local.get $buf local.get $off i32.const 2 i32.add i32.add i32.load8_u i32.or
  )

  ;; write_short_smart: encode a value using the short smart format.
  ;; Returns the new offset.
  (func (export "write_short_smart") (param $buf i32) (param $off i32) (param $val i32) (result i32)
    (local $v i32)
    local.get $val i32.const 64 i32.add local.tee $v
    i32.const 128 i32.lt_u
    if
      local.get $buf local.get $off i32.add local.get $v i32.store8
      local.get $off i32.const 1 i32.add
      return
    end
    local.get $buf local.get $off i32.add
    local.get $val i32.const 0xC000 i32.add i32.store16
    local.get $off i32.const 2 i32.add
  )

  (func (export "write_unsigned_short_smart") (param $buf i32) (param $off i32) (param $val i32) (result i32)
    local.get $val i32.const 128 i32.lt_u
    if
      local.get $buf local.get $off i32.add local.get $val i32.store8
      local.get $off i32.const 1 i32.add
      return
    end
    local.get $buf local.get $off i32.add
    local.get $val i32.const 0x8000 i32.add i32.store16
    local.get $off i32.const 2 i32.add
  )

  (func (export "write_big_smart") (param $buf i32) (param $off i32) (param $val i32) (result i32)
    local.get $val i32.const 0xFFFF i32.le_u
    if
      local.get $buf local.get $off i32.add local.get $val i32.store16
      local.get $off i32.const 2 i32.add
      return
    end
    local.get $buf local.get $off i32.add local.get $val i32.const 0x7FFFFFFF i32.and i32.store
    local.get $off i32.const 4 i32.add
  )
