(module
  (import "edgerun-core" "memory" (memory 1))
(func (export "proto_standard_id") (result i32)
    i32.const 300102)

  (func $m112hex_value (param $b i32) (result i32)
    local.get $b
    i32.const 48
    i32.ge_u
    local.get $b
    i32.const 57
    i32.le_u
    i32.and
    if (result i32)
      local.get $b
      i32.const 48
      i32.sub
    else
      local.get $b
      i32.const 65
      i32.ge_u
      local.get $b
      i32.const 70
      i32.le_u
      i32.and
      if (result i32)
        local.get $b
        i32.const 55
        i32.sub
      else
        local.get $b
        i32.const 97
        i32.ge_u
        local.get $b
        i32.const 102
        i32.le_u
        i32.and
        if (result i32)
          local.get $b
          i32.const 87
          i32.sub
        else
          i32.const -1
        end
      end
    end)

  (func $is_line_byte (param $b i32) (result i32)
    local.get $b
    i32.const 9
    i32.eq
    local.get $b
    i32.const 32
    i32.ge_u
    local.get $b
    i32.const 126
    i32.le_u
    i32.and
    i32.or)

  (func $find_double_crlf (param $ptr i32) (param $len i32) (param $start i32) (result i32)
    (local $i i32)
    local.get $start
    local.set $i
    block $not_found
      loop $scan
        local.get $i
        i32.const 3
        i32.add
        local.get $len
        i32.ge_u
        br_if $not_found
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        i32.const 13
        i32.eq
        local.get $ptr
        local.get $i
        i32.add
        i32.const 1
        i32.add
        i32.load8_u
        i32.const 10
        i32.eq
        i32.and
        local.get $ptr
        local.get $i
        i32.add
        i32.const 2
        i32.add
        i32.load8_u
        i32.const 13
        i32.eq
        i32.and
        local.get $ptr
        local.get $i
        i32.add
        i32.const 3
        i32.add
        i32.load8_u
        i32.const 10
        i32.eq
        i32.and
        if
          local.get $i
          return
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $scan
      end
    end
    i32.const -1)

  (func $write_chunk_record
    (param $out i32)
    (param $size i32)
    (param $size_line_off i32)
    (param $size_line_len i32)
    (param $data_off i32)
    (param $data_len i32)
    (param $next_off i32)
    (param $is_final i32)
    (param $flags i32)
    local.get $out
    local.get $size
    i32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $size_line_off
    i32.store
    local.get $out
    i32.const 8
    i32.add
    local.get $size_line_len
    i32.store
    local.get $out
    i32.const 12
    i32.add
    local.get $data_off
    i32.store
    local.get $out
    i32.const 16
    i32.add
    local.get $data_len
    i32.store
    local.get $out
    i32.const 20
    i32.add
    local.get $next_off
    i32.store
    local.get $out
    i32.const 24
    i32.add
    local.get $is_final
    i32.store
    local.get $out
    i32.const 28
    i32.add
    local.get $flags
    i32.store)

  (func $parse_next (param $ptr i32) (param $len i32) (param $start i32) (param $out i32) (result i32)
    (local $i i32)
    (local $size i32)
    (local $digit i32)
    (local $digits i32)
    (local $data_off i32)
    (local $line_end i32)
    (local $next i32)
    (local $flags i32)
    (local $trail_end i32)
    local.get $start
    local.get $len
    i32.ge_u
    if
      i32.const 1
      return
    end
    local.get $start
    local.set $i
    block $hex_done
      loop $hex
        local.get $i
        local.get $len
        i32.ge_u
        if
          i32.const 1
          return
        end
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        local.tee $digit
        i32.const 59
        i32.eq
        if
          local.get $flags
          i32.const 1
          i32.or
          local.set $flags
          br $hex_done
        end
        local.get $digit
        i32.const 13
        i32.eq
        if
          br $hex_done
        end
        local.get $digit
        call $m112hex_value
        local.tee $digit
        i32.const 0
        i32.lt_s
        if
          i32.const 3
          return
        end
        local.get $size
        i32.const 0x0fffffff
        i32.gt_u
        if
          i32.const 4
          return
        end
        local.get $size
        i32.const 4
        i32.shl
        local.get $digit
        i32.or
        local.set $size
        local.get $digits
        i32.const 1
        i32.add
        local.set $digits
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $hex
      end
    end
    local.get $digits
    i32.eqz
    if
      i32.const 3
      return
    end
    block $line_done
      loop $line
        local.get $i
        i32.const 1
        i32.add
        local.get $len
        i32.ge_u
        if
          i32.const 1
          return
        end
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        local.tee $digit
        i32.const 13
        i32.eq
        local.get $ptr
        local.get $i
        i32.add
        i32.const 1
        i32.add
        i32.load8_u
        i32.const 10
        i32.eq
        i32.and
        if
          local.get $i
          local.set $line_end
          local.get $i
          i32.const 2
          i32.add
          local.set $data_off
          br $line_done
        end
        local.get $digit
        call $is_line_byte
        i32.eqz
        if
          i32.const 3
          return
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $line
      end
    end
    local.get $size
    i32.eqz
    if
      local.get $data_off
      i32.const 1
      i32.add
      local.get $len
      i32.ge_u
      if
        i32.const 1
        return
      end
      local.get $ptr
      local.get $data_off
      i32.add
      i32.load8_u
      i32.const 13
      i32.eq
      local.get $ptr
      local.get $data_off
      i32.add
      i32.const 1
      i32.add
      i32.load8_u
      i32.const 10
      i32.eq
      i32.and
      if
        local.get $data_off
        i32.const 2
        i32.add
        local.set $next
      else
        local.get $ptr
        local.get $len
        local.get $data_off
        call $find_double_crlf
        local.tee $trail_end
        i32.const 0
        i32.lt_s
        if
          i32.const 1
          return
        end
        local.get $trail_end
        i32.const 4
        i32.add
        local.set $next
        local.get $flags
        i32.const 2
        i32.or
        local.set $flags
      end
      local.get $out
      local.get $size
      local.get $start
      local.get $line_end
      local.get $start
      i32.sub
      local.get $data_off
      i32.const 0
      local.get $next
      i32.const 1
      local.get $flags
      call $write_chunk_record
      i32.const 0
      return
    end
    local.get $data_off
    local.get $size
    i32.add
    local.tee $next
    local.get $data_off
    i32.lt_u
    if
      i32.const 4
      return
    end
    local.get $next
    i32.const 1
    i32.add
    local.get $len
    i32.ge_u
    if
      i32.const 1
      return
    end
    local.get $ptr
    local.get $next
    i32.add
    i32.load8_u
    i32.const 13
    i32.eq
    local.get $ptr
    local.get $next
    i32.add
    i32.const 1
    i32.add
    i32.load8_u
    i32.const 10
    i32.eq
    i32.and
    i32.eqz
    if
      i32.const 3
      return
    end
    local.get $next
    i32.const 2
    i32.add
    local.set $next
    local.get $out
    local.get $size
    local.get $start
    local.get $line_end
    local.get $start
    i32.sub
    local.get $data_off
    local.get $size
    local.get $next
    i32.const 0
    local.get $flags
    call $write_chunk_record
    i32.const 0)

  (func (export "http1_chunk_next") (param $ptr i32) (param $len i32) (param $start i32) (param $out i32) (result i32)
    local.get $ptr
    local.get $len
    local.get $start
    local.get $out
    call $parse_next)

  (func (export "http1_chunk_scan_body") (param $ptr i32) (param $len i32) (param $out i32) (result i32)
    (local $offset i32)
    (local $rec i32)
    (local $count i32)
    (local $decoded i64)
    (local $size i32)
    (local $next i32)
    (local $flags i32)
    (local $final_flags i32)
    local.get $out
    i32.const 32
    i32.add
    local.set $rec
    loop $chunks
      local.get $ptr
      local.get $len
      local.get $offset
      local.get $rec
      call $parse_next
      local.tee $flags
      i32.eqz
      i32.eqz
      if
        local.get $flags
        return
      end
      local.get $rec
      i32.load
      local.set $size
      local.get $rec
      i32.const 20
      i32.add
      i32.load
      local.set $next
      local.get $rec
      i32.const 24
      i32.add
      i32.load
      if
        local.get $rec
        i32.const 28
        i32.add
        i32.load
        local.set $final_flags
        local.get $out
        local.get $decoded
        i64.const 0xffffffff
        i64.and
        i32.wrap_i64
        i32.store
        local.get $out
        i32.const 4
        i32.add
        local.get $decoded
        i64.const 32
        i64.shr_u
        i32.wrap_i64
        i32.store
        local.get $out
        i32.const 8
        i32.add
        local.get $count
        i32.store
        local.get $out
        i32.const 12
        i32.add
        local.get $rec
        i32.const 12
        i32.add
        i32.load
        i32.store
        local.get $out
        i32.const 16
        i32.add
        local.get $final_flags
        i32.const 2
        i32.and
        if (result i32)
          local.get $next
          local.get $rec
          i32.const 12
          i32.add
          i32.load
          i32.sub
          i32.const 4
          i32.sub
        else
          i32.const 0
        end
        i32.store
        local.get $out
        i32.const 20
        i32.add
        local.get $next
        i32.store
        local.get $out
        i32.const 24
        i32.add
        local.get $final_flags
        i32.store
        i32.const 0
        return
      end
      local.get $decoded
      local.get $size
      i64.extend_i32_u
      i64.add
      local.set $decoded
      local.get $count
      i32.const 1
      i32.add
      local.set $count
      local.get $next
      local.set $offset
      local.get $rec
      i32.const 32
      i32.add
      local.set $rec
      br $chunks
    end
    i32.const 1)
)
