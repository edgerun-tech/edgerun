
  (func (export "proto_standard_id") (result i32)
    i32.const 300062)


  (func $m100hex_value (param $c i32) (result i32)
    local.get $c
    i32.const 48
    i32.ge_u
    local.get $c
    i32.const 57
    i32.le_u
    i32.and
    if (result i32)
      local.get $c
      i32.const 48
      i32.sub
    else
      local.get $c
      i32.const 97
      i32.ge_u
      local.get $c
      i32.const 102
      i32.le_u
      i32.and
      if (result i32)
        local.get $c
        i32.const 87
        i32.sub
      else
        local.get $c
        i32.const 65
        i32.ge_u
        local.get $c
        i32.const 70
        i32.le_u
        i32.and
        if (result i32)
          local.get $c
          i32.const 55
          i32.sub
        else
          i32.const -1
        end
      end
    end)

  (func $has_prefix (param $ptr i32) (param $len i32) (result i32)
    local.get $len
    i32.const 2
    i32.ge_u
    if (result i32)
      local.get $ptr
      i32.load8_u
      i32.const 48
      i32.eq
      local.get $ptr
      i32.const 1
      i32.add
      i32.load8_u
      i32.const 120
      i32.eq
      local.get $ptr
      i32.const 1
      i32.add
      i32.load8_u
      i32.const 88
      i32.eq
      i32.or
      i32.and
    else
      i32.const 0
    end)

  (func (export "hex_decode_compat")
    (param $in_ptr i32) (param $in_len i32) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (local $pos i32)
    (local $digits i32)
    (local $needed i32)
    (local $written i32)
    (local $hi i32)
    (local $lo i32)

    local.get $in_ptr
    local.get $in_len
    call $has_prefix
    if
      i32.const 2
      local.set $pos
    end

    local.get $in_len
    local.get $pos
    i32.sub
    local.set $digits

    local.get $digits
    i32.const 1
    i32.add
    i32.const 1
    i32.shr_u
    local.set $needed

    local.get $needed
    local.get $out_cap
    i32.gt_u
    if
      i32.const 2
      i32.const 0
      call $pack
      return
    end

    local.get $digits
    i32.const 1
    i32.and
    if
      local.get $in_ptr
      local.get $pos
      i32.add
      i32.load8_u
      call $m100hex_value
      local.tee $lo
      i32.const 0
      i32.lt_s
      if
        i32.const 3
        i32.const 0
        call $pack
        return
      end
      local.get $out_ptr
      local.get $lo
      i32.store8
      local.get $written
      i32.const 1
      i32.add
      local.set $written
      local.get $pos
      i32.const 1
      i32.add
      local.set $pos
    end

    (loop $pairs
      local.get $pos
      local.get $in_len
      i32.lt_u
      if
        local.get $in_ptr
        local.get $pos
        i32.add
        i32.load8_u
        call $m100hex_value
        local.tee $hi
        i32.const 0
        i32.lt_s
        if
          i32.const 3
          i32.const 0
          call $pack
          return
        end
        local.get $in_ptr
        local.get $pos
        i32.const 1
        i32.add
        i32.add
        i32.load8_u
        call $m100hex_value
        local.tee $lo
        i32.const 0
        i32.lt_s
        if
          i32.const 3
          i32.const 0
          call $pack
          return
        end
        local.get $out_ptr
        local.get $written
        i32.add
        local.get $hi
        i32.const 4
        i32.shl
        local.get $lo
        i32.or
        i32.store8
        local.get $written
        i32.const 1
        i32.add
        local.set $written
        local.get $pos
        i32.const 2
        i32.add
        local.set $pos
        br $pairs
      end)

    i32.const 0
    local.get $written
    call $pack)

  (func (export "hex_parse_u32")
    (param $in_ptr i32) (param $in_len i32)
    (result i64)
    (local $pos i32)
    (local $value i64)
    (local $nibble i32)

    local.get $in_ptr
    local.get $in_len
    call $has_prefix
    if
      i32.const 2
      local.set $pos
    end

    local.get $pos
    local.get $in_len
    i32.ge_u
    if
      i32.const 3
      i32.const 0
      call $pack
      return
    end

    (loop $digits
      local.get $pos
      local.get $in_len
      i32.lt_u
      if
        local.get $in_ptr
        local.get $pos
        i32.add
        i32.load8_u
        call $m100hex_value
        local.tee $nibble
        i32.const 0
        i32.lt_s
        if
          i32.const 3
          i32.const 0
          call $pack
          return
        end
        local.get $value
        i64.const 4
        i64.shl
        local.get $nibble
        i64.extend_i32_u
        i64.or
        local.set $value
        local.get $value
        i64.const 4294967295
        i64.gt_u
        if
          i32.const 3
          i32.const 0
          call $pack
          return
        end
        local.get $pos
        i32.const 1
        i32.add
        local.set $pos
        br $digits
      end)

    i32.const 0
    local.get $value
    i32.wrap_i64
    call $pack)

  (func $parse_mac_into
    (param $ptr i32) (param $len i32) (param $out i32) (param $reverse i32)
    (result i32)
    (local $i i32)
    (local $out_index i32)
    (local $hi i32)
    (local $lo i32)

    local.get $len
    i32.const 17
    i32.ne
    if
      i32.const 3
      return
    end

    (loop $octets
      local.get $i
      i32.const 6
      i32.lt_u
      if
        local.get $ptr
        local.get $i
        i32.const 3
        i32.mul
        i32.add
        i32.load8_u
        call $m100hex_value
        local.tee $hi
        i32.const 0
        i32.lt_s
        if
          i32.const 3
          return
        end

        local.get $ptr
        local.get $i
        i32.const 3
        i32.mul
        i32.const 1
        i32.add
        i32.add
        i32.load8_u
        call $m100hex_value
        local.tee $lo
        i32.const 0
        i32.lt_s
        if
          i32.const 3
          return
        end

        local.get $i
        i32.const 5
        i32.lt_u
        if
          local.get $ptr
          local.get $i
          i32.const 3
          i32.mul
          i32.const 2
          i32.add
          i32.add
          i32.load8_u
          i32.const 58
          i32.ne
          if
            i32.const 3
            return
          end
        end

        local.get $reverse
        if (result i32)
          i32.const 5
          local.get $i
          i32.sub
        else
          local.get $i
        end
        local.set $out_index

        local.get $out
        local.get $out_index
        i32.add
        local.get $hi
        i32.const 4
        i32.shl
        local.get $lo
        i32.or
        i32.store8

        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $octets
      end)

    i32.const 0)

  (func (export "mac_scan") (param $ptr i32) (param $len i32) (param $out i32) (result i32)
    local.get $ptr
    local.get $len
    local.get $out
    i32.const 0
    call $parse_mac_into)

  (func (export "bdaddr_scan") (param $ptr i32) (param $len i32) (param $out i32) (result i32)
    local.get $ptr
    local.get $len
    local.get $out
    i32.const 1
    call $parse_mac_into)
