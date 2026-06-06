(module
  (import "edgerun-core" "memory" (memory 1))
  (import "edgerun-core" "pack" (func $pack (param i32 i32) (result i64)))

(func (export "proto_standard_id") (result i32)
    i32.const 300004)

  (func (export "ws_decode_prefix") (param $ptr i32) (param $len i32) (param $out i32) (result i32)
    (local $b0 i32)
    (local $b1 i32)
    (local $opcode i32)
    (local $code i32)
    local.get $len
    i32.const 2
    i32.lt_u
    if
      i32.const 1
      return
    end
    local.get $ptr
    i32.load8_u
    local.set $b0
    local.get $ptr
    i32.const 1
    i32.add
    i32.load8_u
    local.set $b1
    local.get $b0
    i32.const 15
    i32.and
    local.set $opcode
    local.get $b1
    i32.const 127
    i32.and
    local.set $code
    local.get $opcode
    i32.const 0
    i32.eq
    local.get $opcode
    i32.const 1
    i32.eq
    i32.or
    local.get $opcode
    i32.const 2
    i32.eq
    i32.or
    local.get $opcode
    i32.const 8
    i32.eq
    i32.or
    local.get $opcode
    i32.const 9
    i32.eq
    i32.or
    local.get $opcode
    i32.const 10
    i32.eq
    i32.or
    i32.eqz
    if
      i32.const 3
      return
    end
    local.get $opcode
    i32.const 8
    i32.ge_u
    if
      local.get $b0
      i32.const 128
      i32.and
      i32.eqz
      if
        i32.const 3
        return
      end
      local.get $code
      i32.const 125
      i32.gt_u
      if
        i32.const 3
        return
      end
    end
    local.get $out
    local.get $b0
    i32.const 128
    i32.and
    if (result i32)
      i32.const 1
    else
      i32.const 0
    end
    i32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $opcode
    i32.store
    local.get $out
    i32.const 8
    i32.add
    local.get $b1
    i32.const 128
    i32.and
    if (result i32)
      i32.const 1
    else
      i32.const 0
    end
    i32.store
    local.get $out
    i32.const 12
    i32.add
    local.get $code
    i32.store
    local.get $out
    i32.const 16
    i32.add
    local.get $code
    i32.const 126
    i32.eq
    if (result i32)
      i32.const 2
    else
      local.get $code
      i32.const 127
      i32.eq
      if (result i32)
        i32.const 8
      else
        i32.const 0
      end
    end
    i32.store
    i32.const 0)

  (func (export "ws_decode_payload_len") (param $ptr i32) (param $len i32) (param $code i32) (param $max_len i32) (param $out i32) (result i32)
    (local $low i32)
    (local $high i32)
    (local $extra i32)
    local.get $code
    i32.const 126
    i32.lt_u
    if
      local.get $code
      local.set $low
      i32.const 0
      local.set $high
      i32.const 0
      local.set $extra
    else
      local.get $code
      i32.const 126
      i32.eq
      if
        local.get $len
        i32.const 2
        i32.lt_u
        if
          i32.const 1
          return
        end
        local.get $ptr
        i32.load8_u
        i32.const 8
        i32.shl
        local.get $ptr
        i32.const 1
        i32.add
        i32.load8_u
        i32.or
        local.set $low
        i32.const 0
        local.set $high
        i32.const 2
        local.set $extra
      else
        local.get $code
        i32.const 127
        i32.ne
        if
          i32.const 3
          return
        end
        local.get $len
        i32.const 8
        i32.lt_u
        if
          i32.const 1
          return
        end
        local.get $ptr
        i32.load8_u
        i32.const 128
        i32.and
        if
          i32.const 3
          return
        end
        local.get $ptr
        i32.load8_u
        i32.const 24
        i32.shl
        local.get $ptr
        i32.const 1
        i32.add
        i32.load8_u
        i32.const 16
        i32.shl
        i32.or
        local.get $ptr
        i32.const 2
        i32.add
        i32.load8_u
        i32.const 8
        i32.shl
        i32.or
        local.get $ptr
        i32.const 3
        i32.add
        i32.load8_u
        i32.or
        local.set $high
        local.get $ptr
        i32.const 4
        i32.add
        i32.load8_u
        i32.const 24
        i32.shl
        local.get $ptr
        i32.const 5
        i32.add
        i32.load8_u
        i32.const 16
        i32.shl
        i32.or
        local.get $ptr
        i32.const 6
        i32.add
        i32.load8_u
        i32.const 8
        i32.shl
        i32.or
        local.get $ptr
        i32.const 7
        i32.add
        i32.load8_u
        i32.or
        local.set $low
        i32.const 8
        local.set $extra
      end
    end
    local.get $code
    i32.const 126
    i32.eq
    local.get $high
    i32.eqz
    local.get $low
    i32.const 126
    i32.lt_u
    i32.and
    i32.and
    local.get $code
    i32.const 127
    i32.eq
    local.get $high
    i32.eqz
    local.get $low
    i32.const 65536
    i32.lt_u
    i32.and
    i32.and
    i32.or
    if
      i32.const 3
      return
    end
    local.get $high
    i32.eqz
    i32.eqz
    local.get $low
    local.get $max_len
    i32.gt_u
    i32.or
    if
      i32.const 4
      return
    end
    local.get $out
    local.get $low
    i32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $high
    i32.store
    local.get $out
    i32.const 8
    i32.add
    local.get $extra
    i32.store
    i32.const 0)

  (func (export "ws_apply_mask_in_place") (param $payload_ptr i32) (param $payload_len i32) (param $mask i32) (result i32)
    (local $i i32)
    (local $shift i32)
    loop $mask_loop
      local.get $i
      local.get $payload_len
      i32.ge_u
      if
        i32.const 0
        return
      end
      local.get $i
      i32.const 3
      i32.and
      i32.const 3
      i32.xor
      i32.const 8
      i32.mul
      local.set $shift
      local.get $payload_ptr
      local.get $i
      i32.add
      local.get $payload_ptr
      local.get $i
      i32.add
      i32.load8_u
      local.get $mask
      local.get $shift
      i32.shr_u
      i32.const 255
      i32.and
      i32.xor
      i32.store8
      local.get $i
      i32.const 1
      i32.add
      local.set $i
      br $mask_loop
    end
    i32.const 0)

  (func (export "ws_parse_header") (param $ptr i32) (param $len i32) (param $max_len i32) (param $out i32) (result i32)
    (local $b0 i32)
    (local $b1 i32)
    (local $code i32)
    (local $extra i32)
    (local $masked i32)
    (local $header_len i32)
    (local $low i32)
    (local $high i32)
    local.get $len
    i32.const 2
    i32.lt_u
    if
      i32.const 1
      return
    end
    local.get $ptr
    i32.load8_u
    local.set $b0
    local.get $ptr
    i32.const 1
    i32.add
    i32.load8_u
    local.set $b1
    local.get $b1
    i32.const 127
    i32.and
    local.set $code
    i32.const 2
    local.set $header_len
    local.get $b1
    i32.const 128
    i32.and
    if (result i32)
      i32.const 1
    else
      i32.const 0
    end
    local.set $masked

    local.get $b0
    i32.const 15
    i32.and
    i32.const 0
    i32.eq
    local.get $b0
    i32.const 15
    i32.and
    i32.const 1
    i32.eq
    i32.or
    local.get $b0
    i32.const 15
    i32.and
    i32.const 2
    i32.eq
    i32.or
    local.get $b0
    i32.const 15
    i32.and
    i32.const 8
    i32.eq
    i32.or
    local.get $b0
    i32.const 15
    i32.and
    i32.const 9
    i32.eq
    i32.or
    local.get $b0
    i32.const 15
    i32.and
    i32.const 10
    i32.eq
    i32.or
    i32.eqz
    if
      i32.const 3
      return
    end
    local.get $b0
    i32.const 15
    i32.and
    i32.const 8
    i32.ge_u
    if
      local.get $b0
      i32.const 128
      i32.and
      i32.eqz
      if
        i32.const 3
        return
      end
      local.get $code
      i32.const 125
      i32.gt_u
      if
        i32.const 3
        return
      end
    end

    local.get $code
    i32.const 126
    i32.lt_u
    if
      local.get $code
      local.set $low
      i32.const 0
      local.set $high
      i32.const 0
      local.set $extra
    else
      local.get $code
      i32.const 126
      i32.eq
      if
        local.get $len
        i32.const 4
        i32.lt_u
        if
          i32.const 1
          return
        end
        local.get $ptr
        i32.const 2
        i32.add
        i32.load8_u
        i32.const 8
        i32.shl
        local.get $ptr
        i32.const 3
        i32.add
        i32.load8_u
        i32.or
        local.set $low
        i32.const 0
        local.set $high
        i32.const 2
        local.set $extra
        i32.const 4
        local.set $header_len
      else
        local.get $len
        i32.const 10
        i32.lt_u
        if
          i32.const 1
          return
        end
        local.get $ptr
        i32.const 2
        i32.add
        i32.load8_u
        i32.const 128
        i32.and
        if
          i32.const 3
          return
        end
        local.get $ptr
        i32.const 2
        i32.add
        i32.load8_u
        i32.const 24
        i32.shl
        local.get $ptr
        i32.const 3
        i32.add
        i32.load8_u
        i32.const 16
        i32.shl
        i32.or
        local.get $ptr
        i32.const 4
        i32.add
        i32.load8_u
        i32.const 8
        i32.shl
        i32.or
        local.get $ptr
        i32.const 5
        i32.add
        i32.load8_u
        i32.or
        local.set $high
        local.get $ptr
        i32.const 6
        i32.add
        i32.load8_u
        i32.const 24
        i32.shl
        local.get $ptr
        i32.const 7
        i32.add
        i32.load8_u
        i32.const 16
        i32.shl
        i32.or
        local.get $ptr
        i32.const 8
        i32.add
        i32.load8_u
        i32.const 8
        i32.shl
        i32.or
        local.get $ptr
        i32.const 9
        i32.add
        i32.load8_u
        i32.or
        local.set $low
        i32.const 8
        local.set $extra
        i32.const 10
        local.set $header_len
      end
    end

    local.get $code
    i32.const 126
    i32.eq
    local.get $low
    i32.const 126
    i32.lt_u
    i32.and
    local.get $code
    i32.const 127
    i32.eq
    local.get $high
    i32.eqz
    local.get $low
    i32.const 65536
    i32.lt_u
    i32.and
    i32.and
    i32.or
    if
      i32.const 3
      return
    end
    local.get $high
    i32.eqz
    i32.eqz
    local.get $low
    local.get $max_len
    i32.gt_u
    i32.or
    if
      i32.const 4
      return
    end
    local.get $masked
    if
      local.get $len
      local.get $header_len
      i32.const 4
      i32.add
      i32.lt_u
      if
        i32.const 1
        return
      end
    end

    local.get $out
    local.get $b0
    i32.const 128
    i32.and
    if (result i32) i32.const 1 else i32.const 0 end
    i32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $b0
    i32.const 112
    i32.and
    i32.const 4
    i32.shr_u
    i32.store
    local.get $out
    i32.const 8
    i32.add
    local.get $b0
    i32.const 15
    i32.and
    i32.store
    local.get $out
    i32.const 12
    i32.add
    local.get $masked
    i32.store
    local.get $out
    i32.const 16
    i32.add
    local.get $low
    i32.store
    local.get $out
    i32.const 20
    i32.add
    local.get $high
    i32.store
    local.get $out
    i32.const 24
    i32.add
    local.get $header_len
    local.get $masked
    if (result i32) i32.const 4 else i32.const 0 end
    i32.add
    i32.store
    local.get $out
    i32.const 28
    i32.add
    local.get $masked
    if (result i32)
      local.get $ptr
      local.get $header_len
      i32.add
      i32.load8_u
      i32.const 24
      i32.shl
      local.get $ptr
      local.get $header_len
      i32.add
      i32.const 1
      i32.add
      i32.load8_u
      i32.const 16
      i32.shl
      i32.or
      local.get $ptr
      local.get $header_len
      i32.add
      i32.const 2
      i32.add
      i32.load8_u
      i32.const 8
      i32.shl
      i32.or
      local.get $ptr
      local.get $header_len
      i32.add
      i32.const 3
      i32.add
      i32.load8_u
      i32.or
    else
      i32.const 0
    end
    i32.store
    i32.const 0)

  (func (export "ws_write_frame_header") (param $opcode i32) (param $flags i32) (param $low i32) (param $high i32) (param $mask_present i32) (param $mask i32) (param $out i32) (param $cap i32) (result i64)
    (local $written i32)
    local.get $opcode
    i32.const 0
    i32.eq
    local.get $opcode
    i32.const 1
    i32.eq
    i32.or
    local.get $opcode
    i32.const 2
    i32.eq
    i32.or
    local.get $opcode
    i32.const 8
    i32.eq
    i32.or
    local.get $opcode
    i32.const 9
    i32.eq
    i32.or
    local.get $opcode
    i32.const 10
    i32.eq
    i32.or
    i32.eqz
    if
      i32.const 3
      i32.const 0
      call $pack
      return
    end
    local.get $opcode
    i32.const 8
    i32.ge_u
    if
      local.get $flags
      i32.const 128
      i32.and
      i32.eqz
      local.get $high
      i32.const 0
      i32.ne
      i32.or
      local.get $low
      i32.const 125
      i32.gt_u
      i32.or
      if
        i32.const 3
        i32.const 0
        call $pack
        return
      end
    end
    local.get $high
    i32.const 2147483648
    i32.ge_u
    if
      i32.const 3
      i32.const 0
      call $pack
      return
    end
    local.get $high
    i32.eqz
    local.get $low
    i32.const 126
    i32.lt_u
    i32.and
    if
      i32.const 2
      local.set $written
    else
      local.get $high
      i32.eqz
      local.get $low
      i32.const 65536
      i32.lt_u
      i32.and
      if
        i32.const 4
        local.set $written
      else
        i32.const 10
        local.set $written
      end
    end
    local.get $mask_present
    if
      local.get $written
      i32.const 4
      i32.add
      local.set $written
    end
    local.get $cap
    local.get $written
    i32.lt_u
    if
      i32.const 2
      i32.const 0
      call $pack
      return
    end
    local.get $out
    local.get $flags
    i32.const 240
    i32.and
    local.get $opcode
    i32.const 15
    i32.and
    i32.or
    i32.store8
    local.get $out
    i32.const 1
    i32.add
    local.get $mask_present
    if (result i32) i32.const 128 else i32.const 0 end
    local.get $written
    local.get $mask_present
    if (result i32) i32.const 4 else i32.const 0 end
    i32.sub
    i32.const 2
    i32.eq
    if (result i32)
      local.get $low
    else
      local.get $written
      local.get $mask_present
      if (result i32) i32.const 4 else i32.const 0 end
      i32.sub
      i32.const 4
      i32.eq
      if (result i32) i32.const 126 else i32.const 127 end
    end
    i32.or
    i32.store8
    local.get $written
    local.get $mask_present
    if (result i32) i32.const 4 else i32.const 0 end
    i32.sub
    i32.const 4
    i32.eq
    if
      local.get $out
      i32.const 2
      i32.add
      local.get $low
      i32.const 8
      i32.shr_u
      i32.store8
      local.get $out
      i32.const 3
      i32.add
      local.get $low
      i32.store8
    end
    local.get $written
    local.get $mask_present
    if (result i32) i32.const 4 else i32.const 0 end
    i32.sub
    i32.const 10
    i32.eq
    if
      local.get $out
      i32.const 2
      i32.add
      local.get $high
      i32.const 24
      i32.shr_u
      i32.store8
      local.get $out
      i32.const 3
      i32.add
      local.get $high
      i32.const 16
      i32.shr_u
      i32.store8
      local.get $out
      i32.const 4
      i32.add
      local.get $high
      i32.const 8
      i32.shr_u
      i32.store8
      local.get $out
      i32.const 5
      i32.add
      local.get $high
      i32.store8
      local.get $out
      i32.const 6
      i32.add
      local.get $low
      i32.const 24
      i32.shr_u
      i32.store8
      local.get $out
      i32.const 7
      i32.add
      local.get $low
      i32.const 16
      i32.shr_u
      i32.store8
      local.get $out
      i32.const 8
      i32.add
      local.get $low
      i32.const 8
      i32.shr_u
      i32.store8
      local.get $out
      i32.const 9
      i32.add
      local.get $low
      i32.store8
    end
    local.get $mask_present
    if
      local.get $out
      local.get $written
      i32.const 4
      i32.sub
      i32.add
      local.get $mask
      i32.const 24
      i32.shr_u
      i32.store8
      local.get $out
      local.get $written
      i32.const 3
      i32.sub
      i32.add
      local.get $mask
      i32.const 16
      i32.shr_u
      i32.store8
      local.get $out
      local.get $written
      i32.const 2
      i32.sub
      i32.add
      local.get $mask
      i32.const 8
      i32.shr_u
      i32.store8
      local.get $out
      local.get $written
      i32.const 1
      i32.sub
      i32.add
      local.get $mask
      i32.store8
    end
    i32.const 0
    local.get $written
    call $pack)

  (func (export "ws_parse_close_payload") (param $ptr i32) (param $len i32) (param $out i32) (result i32)
    local.get $len
    i32.const 125
    i32.gt_u
    if
      i32.const 3
      return
    end
    local.get $out
    local.get $len
    i32.const 2
    i32.ge_u
    if (result i32) i32.const 1 else i32.const 0 end
    i32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $len
    i32.const 2
    i32.ge_u
    if (result i32)
      local.get $ptr
      i32.load8_u
      i32.const 8
      i32.shl
      local.get $ptr
      i32.const 1
      i32.add
      i32.load8_u
      i32.or
    else
      i32.const 0
    end
    i32.store
    local.get $out
    i32.const 8
    i32.add
    local.get $len
    i32.const 2
    i32.ge_u
    if (result i32) local.get $len i32.const 2 i32.sub else i32.const 0 end
    i32.store
    i32.const 0)

  (func (export "ws_write_close_payload") (param $code i32) (param $reason_ptr i32) (param $reason_len i32) (param $out i32) (param $cap i32) (result i64)
    (local $i i32)
    local.get $code
    i32.const 65535
    i32.gt_u
    if
      i32.const 3
      i32.const 0
      call $pack
      return
    end
    local.get $reason_len
    i32.const 123
    i32.gt_u
    if
      i32.const 3
      i32.const 0
      call $pack
      return
    end
    local.get $cap
    local.get $reason_len
    i32.const 2
    i32.add
    i32.lt_u
    if
      i32.const 2
      i32.const 0
      call $pack
      return
    end
    local.get $out
    local.get $code
    i32.const 8
    i32.shr_u
    i32.store8
    local.get $out
    i32.const 1
    i32.add
    local.get $code
    i32.store8
    loop $copy
      local.get $i
      local.get $reason_len
      i32.ge_u
      if
        i32.const 0
        local.get $reason_len
        i32.const 2
        i32.add
        call $pack
        return
      end
      local.get $out
      i32.const 2
      i32.add
      local.get $i
      i32.add
      local.get $reason_ptr
      local.get $i
      i32.add
      i32.load8_u
      i32.store8
      local.get $i
      i32.const 1
      i32.add
      local.set $i
      br $copy
    end
    i32.const 0
    i32.const 0
    call $pack)

  (func (export "ws_write_server_frame_header") (param $opcode i32) (param $low i32) (param $high i32) (param $out i32) (param $cap i32) (result i64)
    (local $written i32)
    local.get $opcode
    i32.const 0
    i32.eq
    local.get $opcode
    i32.const 1
    i32.eq
    i32.or
    local.get $opcode
    i32.const 2
    i32.eq
    i32.or
    local.get $opcode
    i32.const 8
    i32.eq
    i32.or
    local.get $opcode
    i32.const 9
    i32.eq
    i32.or
    local.get $opcode
    i32.const 10
    i32.eq
    i32.or
    i32.eqz
    if
      i32.const 3
      i32.const 0
      call $pack
      return
    end
    local.get $opcode
    i32.const 8
    i32.ge_u
    local.get $high
    i32.eqz
    local.get $low
    i32.const 125
    i32.gt_u
    i32.and
    i32.and
    local.get $opcode
    i32.const 8
    i32.ge_u
    local.get $high
    i32.const 0
    i32.ne
    i32.and
    i32.or
    if
      i32.const 3
      i32.const 0
      call $pack
      return
    end
    local.get $high
    i32.eqz
    local.get $low
    i32.const 126
    i32.lt_u
    i32.and
    if
      i32.const 2
      local.set $written
      local.get $cap
      local.get $written
      i32.lt_u
      if
        i32.const 2
        i32.const 0
        call $pack
        return
      end
      local.get $out
      local.get $opcode
      i32.const 128
      i32.or
      i32.store8
      local.get $out
      i32.const 1
      i32.add
      local.get $low
      i32.store8
    else
      local.get $high
      i32.eqz
      local.get $low
      i32.const 65536
      i32.lt_u
      i32.and
      if
        i32.const 4
        local.set $written
        local.get $cap
        local.get $written
        i32.lt_u
        if
          i32.const 2
          i32.const 0
          call $pack
          return
        end
        local.get $out
        local.get $opcode
        i32.const 128
        i32.or
        i32.store8
        local.get $out
        i32.const 1
        i32.add
        i32.const 126
        i32.store8
        local.get $out
        i32.const 2
        i32.add
        local.get $low
        i32.const 8
        i32.shr_u
        i32.store8
        local.get $out
        i32.const 3
        i32.add
        local.get $low
        i32.store8
      else
        i32.const 10
        local.set $written
        local.get $high
        i32.const 2147483648
        i32.ge_u
        if
          i32.const 3
          i32.const 0
          call $pack
          return
        end
        local.get $cap
        local.get $written
        i32.lt_u
        if
          i32.const 2
          i32.const 0
          call $pack
          return
        end
        local.get $out
        local.get $opcode
        i32.const 128
        i32.or
        i32.store8
        local.get $out
        i32.const 1
        i32.add
        i32.const 127
        i32.store8
        local.get $out
        i32.const 2
        i32.add
        local.get $high
        i32.const 24
        i32.shr_u
        i32.store8
        local.get $out
        i32.const 3
        i32.add
        local.get $high
        i32.const 16
        i32.shr_u
        i32.store8
        local.get $out
        i32.const 4
        i32.add
        local.get $high
        i32.const 8
        i32.shr_u
        i32.store8
        local.get $out
        i32.const 5
        i32.add
        local.get $high
        i32.store8
        local.get $out
        i32.const 6
        i32.add
        local.get $low
        i32.const 24
        i32.shr_u
        i32.store8
        local.get $out
        i32.const 7
        i32.add
        local.get $low
        i32.const 16
        i32.shr_u
        i32.store8
        local.get $out
        i32.const 8
        i32.add
        local.get $low
        i32.const 8
        i32.shr_u
        i32.store8
        local.get $out
        i32.const 9
        i32.add
        local.get $low
        i32.store8
      end
    end
    i32.const 0
    local.get $written
    call $pack)
)
