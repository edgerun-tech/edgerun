(module
  (import "edgerun-core" "memory" (memory 1))
  (import "edgerun-core" "pack" (func $pack (param i32 i32) (result i64)))

  (func (export "proto_standard_id") (result i32)
    i32.const 300063)

  ;; Status values: 0 ok, 1 input_short, 2 output_short, 3 invalid, 4 overflow.
  ;; Read out record: u32 value_lo, u32 value_hi. Signed reads are sign-extended.
  ;; Endian values: 0 little-endian, 1 big-endian. Width values: 2, 3, 4, 8.
  ;; endian_read(ptr,len,offset,width,endian,signed,out) -> status
  ;; endian_write(value_lo,value_hi,width,endian,out_ptr,out_cap) -> packed i64
  ;; Packed i64 return: low u32 status, high u32 bytes_written.


  (func $valid_width (param $width i32) (result i32)
    local.get $width
    i32.const 2
    i32.eq
    local.get $width
    i32.const 3
    i32.eq
    i32.or
    local.get $width
    i32.const 4
    i32.eq
    i32.or
    local.get $width
    i32.const 8
    i32.eq
    i32.or)

  (func $read_be (param $addr i32) (param $width i32) (result i64)
    (local $i i32)
    (local $value i64)
    loop $again
      local.get $i
      local.get $width
      i32.lt_u
      if
        local.get $value
        i64.const 8
        i64.shl
        local.get $addr
        local.get $i
        i32.add
        i64.load8_u
        i64.or
        local.set $value
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $again
      end
    end
    local.get $value)

  (func $read_le (param $addr i32) (param $width i32) (result i64)
    (local $i i32)
    (local $shift i64)
    (local $value i64)
    loop $again
      local.get $i
      local.get $width
      i32.lt_u
      if
        local.get $addr
        local.get $i
        i32.add
        i64.load8_u
        local.get $shift
        i64.shl
        local.get $value
        i64.or
        local.set $value
        local.get $shift
        i64.const 8
        i64.add
        local.set $shift
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $again
      end
    end
    local.get $value)

  (func $sign_extend (param $value i64) (param $width i32) (result i64)
    local.get $width
    i32.const 2
    i32.eq
    if (result i64)
      local.get $value
      i64.const 48
      i64.shl
      i64.const 48
      i64.shr_s
    else
      local.get $width
      i32.const 4
      i32.eq
      if (result i64)
        local.get $value
        i64.const 32
        i64.shl
        i64.const 32
        i64.shr_s
      else
        local.get $value
      end
    end)

  (func $store_u64 (param $out i32) (param $value i64)
    local.get $out
    local.get $value
    i32.wrap_i64
    i32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $value
    i64.const 32
    i64.shr_u
    i32.wrap_i64
    i32.store)

  (func (export "endian_read")
    (param $ptr i32) (param $len i32) (param $offset i32)
    (param $width i32) (param $endian i32) (param $signed i32) (param $out i32)
    (result i32)
    (local $addr i32)
    (local $value i64)

    local.get $out
    i32.eqz
    if
      i32.const 2
      return
    end

    local.get $width
    call $valid_width
    i32.eqz
    if
      i32.const 3
      return
    end

    local.get $endian
    i32.const 1
    i32.gt_u
    if
      i32.const 3
      return
    end

    local.get $signed
    i32.const 1
    i32.gt_u
    if
      i32.const 3
      return
    end

    local.get $signed
    local.get $width
    i32.const 3
    i32.eq
    i32.and
    if
      i32.const 3
      return
    end

    local.get $offset
    local.get $len
    i32.gt_u
    if
      i32.const 1
      return
    end

    local.get $len
    local.get $offset
    i32.sub
    local.get $width
    i32.lt_u
    if
      i32.const 1
      return
    end

    local.get $ptr
    local.get $offset
    i32.add
    local.set $addr

    local.get $endian
    i32.eqz
    if (result i64)
      local.get $addr
      local.get $width
      call $read_le
    else
      local.get $addr
      local.get $width
      call $read_be
    end
    local.set $value

    local.get $signed
    if
      local.get $value
      local.get $width
      call $sign_extend
      local.set $value
    end

    local.get $out
    local.get $value
    call $store_u64
    i32.const 0)

  (func $write_le (param $value i64) (param $width i32) (param $out i32)
    (local $i i32)
    (local $v i64)
    local.get $value
    local.set $v
    loop $again
      local.get $i
      local.get $width
      i32.lt_u
      if
        local.get $out
        local.get $i
        i32.add
        local.get $v
        i32.wrap_i64
        i32.store8
        local.get $v
        i64.const 8
        i64.shr_u
        local.set $v
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $again
      end
    end)

  (func $write_be (param $value i64) (param $width i32) (param $out i32)
    (local $i i32)
    (local $shift i64)
    local.get $width
    i32.const 1
    i32.sub
    i64.extend_i32_u
    i64.const 8
    i64.mul
    local.set $shift
    loop $again
      local.get $i
      local.get $width
      i32.lt_u
      if
        local.get $out
        local.get $i
        i32.add
        local.get $value
        local.get $shift
        i64.shr_u
        i32.wrap_i64
        i32.store8
        local.get $shift
        i64.const 8
        i64.sub
        local.set $shift
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $again
      end
    end)

  (func (export "endian_write")
    (param $value_lo i32) (param $value_hi i32) (param $width i32)
    (param $endian i32) (param $out i32) (param $out_cap i32)
    (result i64)
    (local $value i64)

    local.get $width
    call $valid_width
    i32.eqz
    if
      i32.const 3
      i32.const 0
      call $pack
      return
    end

    local.get $endian
    i32.const 1
    i32.gt_u
    if
      i32.const 3
      i32.const 0
      call $pack
      return
    end

    local.get $out
    i32.eqz
    local.get $out_cap
    local.get $width
    i32.lt_u
    i32.or
    if
      i32.const 2
      i32.const 0
      call $pack
      return
    end

    local.get $value_hi
    i64.extend_i32_u
    i64.const 32
    i64.shl
    local.get $value_lo
    i64.extend_i32_u
    i64.or
    local.set $value

    local.get $width
    i32.const 2
    i32.eq
    if
      local.get $value_hi
      i32.eqz
      i32.eqz
      local.get $value_lo
      i32.const 0xffff
      i32.gt_u
      i32.or
      if
        i32.const 4
        i32.const 0
        call $pack
        return
      end
    end

    local.get $width
    i32.const 3
    i32.eq
    if
      local.get $value_hi
      i32.eqz
      i32.eqz
      local.get $value_lo
      i32.const 0x00ffffff
      i32.gt_u
      i32.or
      if
        i32.const 4
        i32.const 0
        call $pack
        return
      end
    end

    local.get $width
    i32.const 4
    i32.eq
    if
      local.get $value_hi
      i32.eqz
      local.get $value_hi
      i32.const -1
      i32.eq
      i32.or
      i32.eqz
      if
        i32.const 4
        i32.const 0
        call $pack
        return
      end
    end

    local.get $endian
    i32.eqz
    if
      local.get $value
      local.get $width
      local.get $out
      call $write_le
    else
      local.get $value
      local.get $width
      local.get $out
      call $write_be
    end

    i32.const 0
    local.get $width
    call $pack)
)