
(func $m96write_record
    (param $out i32)
    (param $tag i32)
    (param $value_off i32)
    (param $value_len i32)
    (param $header_len i32)
    (param $total_len i32)

    local.get $out
    local.get $tag
    i32.store

    local.get $out
    i32.const 4
    i32.add
    local.get $value_off
    i32.store

    local.get $out
    i32.const 8
    i32.add
    local.get $value_len
    i32.store

    local.get $out
    i32.const 12
    i32.add
    local.get $header_len
    i32.store

    local.get $out
    i32.const 16
    i32.add
    local.get $total_len
    i32.store)

  ;; Parse a one-byte-tag TLV at offset.
  ;;
  ;; Return bits: low32=status, high32=next_offset.
  ;; status: 0 ok, 1 input_short, 2 output_short, 3 invalid.
  ;; Record fields are little-endian u32:
  ;; tag,value_off,value_len,header_len,total_len.
  (func (export "generic_tlv_next")
    (param $ptr i32) (param $len i32) (param $offset i32) (param $out i32)
    (result i64)
    (local $tag i32)
    (local $first i32)
    (local $len_bytes i32)
    (local $value_len i32)
    (local $header_len i32)
    (local $total_len i32)
    (local $value_off i32)
    (local $next_offset i32)

    local.get $offset
    local.get $len
    i32.ge_u
    if
      i32.const 1
      local.get $len
      call $pack
      return
    end

    local.get $len
    local.get $offset
    i32.sub
    i32.const 2
    i32.lt_u
    if
      i32.const 1
      local.get $len
      call $pack
      return
    end

    local.get $ptr
    local.get $offset
    i32.add
    i32.load8_u
    local.set $tag

    local.get $ptr
    local.get $offset
    i32.add
    i32.const 1
    i32.add
    i32.load8_u
    local.set $first

    local.get $first
    i32.const 128
    i32.lt_u
    if
      local.get $first
      local.set $value_len
      i32.const 2
      local.set $header_len
    else
      local.get $first
      i32.const 128
      i32.eq
      if
        i32.const 3
        local.get $offset
        call $pack
        return
      end

      local.get $first
      i32.const 127
      i32.and
      local.set $len_bytes

      local.get $len_bytes
      i32.const 1
      i32.eq
      if
        local.get $len
        local.get $offset
        i32.sub
        i32.const 3
        i32.lt_u
        if
          i32.const 1
          local.get $len
          call $pack
          return
        end

        local.get $ptr
        local.get $offset
        i32.add
        i32.const 2
        i32.add
        i32.load8_u
        local.set $value_len
        i32.const 3
        local.set $header_len
      else
        local.get $len_bytes
        i32.const 2
        i32.eq
        if
          local.get $len
          local.get $offset
          i32.sub
          i32.const 4
          i32.lt_u
          if
            i32.const 1
            local.get $len
            call $pack
            return
          end

          local.get $ptr
          local.get $offset
          i32.add
          i32.const 2
          i32.add
          i32.load8_u
          i32.const 8
          i32.shl
          local.get $ptr
          local.get $offset
          i32.add
          i32.const 3
          i32.add
          i32.load8_u
          i32.or
          local.set $value_len
          i32.const 4
          local.set $header_len
        else
          i32.const 3
          local.get $offset
          call $pack
          return
        end
      end
    end

    local.get $header_len
    local.get $value_len
    i32.add
    local.set $total_len

    local.get $offset
    local.get $header_len
    i32.add
    local.set $value_off

    local.get $offset
    local.get $total_len
    i32.add
    local.set $next_offset

    local.get $next_offset
    local.get $len
    i32.gt_u
    if
      i32.const 1
      local.get $len
      call $pack
      return
    end

    local.get $out
    local.get $tag
    local.get $value_off
    local.get $value_len
    local.get $header_len
    local.get $total_len
    call $m96write_record

    i32.const 0
    local.get $next_offset
    call $pack)

  ;; Encode only the TLV header for a one-byte tag and value length.
  ;;
  ;; Return bits: low32=status, high32=written.
  ;; status: 0 ok, 2 output_short, 3 invalid.
  (func (export "generic_tlv_encode_header")
    (param $tag i32) (param $value_len i32) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (local $needed i32)

    local.get $tag
    i32.const 255
    i32.gt_u
    if
      i32.const 3
      i32.const 0
      call $pack
      return
    end

    local.get $value_len
    i32.const 65535
    i32.gt_u
    if
      i32.const 3
      i32.const 0
      call $pack
      return
    end

    local.get $value_len
    i32.const 127
    i32.le_u
    if
      i32.const 2
      local.set $needed
    else
      local.get $value_len
      i32.const 255
      i32.le_u
      if
        i32.const 3
        local.set $needed
      else
        i32.const 4
        local.set $needed
      end
    end

    local.get $out_cap
    local.get $needed
    i32.lt_u
    if
      i32.const 2
      i32.const 0
      call $pack
      return
    end

    local.get $out_ptr
    local.get $tag
    i32.store8

    local.get $needed
    i32.const 2
    i32.eq
    if
      local.get $out_ptr
      i32.const 1
      i32.add
      local.get $value_len
      i32.store8
    else
      local.get $needed
      i32.const 3
      i32.eq
      if
        local.get $out_ptr
        i32.const 1
        i32.add
        i32.const 129
        i32.store8

        local.get $out_ptr
        i32.const 2
        i32.add
        local.get $value_len
        i32.store8
      else
        local.get $out_ptr
        i32.const 1
        i32.add
        i32.const 130
        i32.store8

        local.get $out_ptr
        i32.const 2
        i32.add
        local.get $value_len
        i32.const 8
        i32.shr_u
        i32.store8

        local.get $out_ptr
        i32.const 3
        i32.add
        local.get $value_len
        i32.const 255
        i32.and
        i32.store8
      end
    end

    i32.const 0
    local.get $needed
    call $pack)
