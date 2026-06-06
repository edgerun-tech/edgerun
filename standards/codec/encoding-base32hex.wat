
  (func (export "proto_standard_id") (result i32)
    i32.const 300052)


  (func $b32hex_char (param $n i32) (result i32)
    local.get $n
    i32.const 10
    i32.lt_u
    if (result i32)
      local.get $n
      i32.const 48
      i32.add
    else
      local.get $n
      i32.const 10
      i32.sub
      i32.const 97
      i32.add
    end)

  (func $b32hex_value (param $c i32) (result i32)
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
      i32.const 118
      i32.le_u
      i32.and
      if (result i32)
        local.get $c
        i32.const 97
        i32.sub
        i32.const 10
        i32.add
      else
        local.get $c
        i32.const 65
        i32.ge_u
        local.get $c
        i32.const 86
        i32.le_u
        i32.and
        if (result i32)
          local.get $c
          i32.const 65
          i32.sub
          i32.const 10
          i32.add
        else
          i32.const -1
        end
      end
    end)

  (func $valid_unpadded_len (param $len i32) (result i32)
    local.get $len
    i32.const 8
    i32.rem_u
    i32.const 0
    i32.eq
    local.get $len
    i32.const 8
    i32.rem_u
    i32.const 2
    i32.eq
    i32.or
    local.get $len
    i32.const 8
    i32.rem_u
    i32.const 4
    i32.eq
    i32.or
    local.get $len
    i32.const 8
    i32.rem_u
    i32.const 5
    i32.eq
    i32.or
    local.get $len
    i32.const 8
    i32.rem_u
    i32.const 7
    i32.eq
    i32.or)

  (func (export "base32hex_encode")
    (param $in_ptr i32) (param $in_len i32) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (local $needed i32)
    (local $i i32)
    (local $written i32)
    (local $bits i64)
    (local $bit_len i32)
    local.get $in_len
    i32.const 536870911
    i32.gt_u
    if
      i32.const 4
      i32.const 0
      call $pack
      return
    end
    local.get $in_len
    i32.const 3
    i32.shl
    i32.const 4
    i32.add
    i32.const 5
    i32.div_u
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
    loop $bytes
      local.get $i
      local.get $in_len
      i32.lt_u
      if
        local.get $bits
        i64.const 8
        i64.shl
        local.get $in_ptr
        local.get $i
        i32.add
        i32.load8_u
        i64.extend_i32_u
        i64.or
        local.set $bits
        local.get $bit_len
        i32.const 8
        i32.add
        local.set $bit_len
        loop $chunks
          local.get $bit_len
          i32.const 5
          i32.ge_u
          if
            local.get $bit_len
            i32.const 5
            i32.sub
            local.set $bit_len
            local.get $out_ptr
            local.get $written
            i32.add
            local.get $bits
            local.get $bit_len
            i64.extend_i32_u
            i64.shr_u
            i64.const 31
            i64.and
            i32.wrap_i64
            call $b32hex_char
            i32.store8
            local.get $written
            i32.const 1
            i32.add
            local.set $written
            br $chunks
          end
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $bytes
      end
    end
    local.get $bit_len
    i32.const 0
    i32.gt_u
    if
      local.get $out_ptr
      local.get $written
      i32.add
      local.get $bits
      i32.const 5
      local.get $bit_len
      i32.sub
      i64.extend_i32_u
      i64.shl
      i64.const 31
      i64.and
      i32.wrap_i64
      call $b32hex_char
      i32.store8
      local.get $written
      i32.const 1
      i32.add
      local.set $written
    end
    i32.const 0
    local.get $written
    call $pack)

  (func (export "base32hex_decode")
    (param $in_ptr i32) (param $in_len i32) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (local $needed i32)
    (local $i i32)
    (local $written i32)
    (local $bits i64)
    (local $bit_len i32)
    (local $value i32)
    local.get $in_len
    call $valid_unpadded_len
    i32.eqz
    if
      i32.const 3
      i32.const 0
      call $pack
      return
    end
    local.get $in_len
    i32.const 858993459
    i32.gt_u
    if
      i32.const 4
      i32.const 0
      call $pack
      return
    end
    local.get $in_len
    i32.const 5
    i32.mul
    i32.const 3
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
    loop $chars
      local.get $i
      local.get $in_len
      i32.lt_u
      if
        local.get $in_ptr
        local.get $i
        i32.add
        i32.load8_u
        call $b32hex_value
        local.set $value
        local.get $value
        i32.const 0
        i32.lt_s
        if
          i32.const 3
          i32.const 0
          call $pack
          return
        end
        local.get $bits
        i64.const 5
        i64.shl
        local.get $value
        i64.extend_i32_u
        i64.or
        local.set $bits
        local.get $bit_len
        i32.const 5
        i32.add
        local.set $bit_len
        loop $bytes
          local.get $bit_len
          i32.const 8
          i32.ge_u
          if
            local.get $bit_len
            i32.const 8
            i32.sub
            local.set $bit_len
            local.get $out_ptr
            local.get $written
            i32.add
            local.get $bits
            local.get $bit_len
            i64.extend_i32_u
            i64.shr_u
            i32.wrap_i64
            i32.store8
            local.get $written
            i32.const 1
            i32.add
            local.set $written
            br $bytes
          end
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $chars
      end
    end
    i32.const 0
    local.get $written
    call $pack)
