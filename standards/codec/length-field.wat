
;; Status values: 0 ok, 1 input_short, 2 output_short, 3 invalid, 4 overflow.
  ;; Packed i64 return: low u32 status, high u32 next_offset or bytes_written.

  (func $m128read_u16_le (param $ptr i32) (result i32)
    local.get $ptr
    i32.load16_u)

  (func $m128read_u32_le (param $ptr i32) (result i32)
    local.get $ptr
    i32.load)

  (func $m128read_u32_be (param $ptr i32) (result i32)
    (i32.or
      (i32.or
        (i32.shl
          (i32.load8_u (local.get $ptr))
          (i32.const 24))
        (i32.shl
          (i32.load8_u (i32.add (local.get $ptr) (i32.const 1)))
          (i32.const 16)))
      (i32.or
        (i32.shl
          (i32.load8_u (i32.add (local.get $ptr) (i32.const 2)))
          (i32.const 8))
        (i32.load8_u (i32.add (local.get $ptr) (i32.const 3))))))

  (func $read_u64_le (param $ptr i32) (result i64)
    local.get $ptr
    i64.load)

  (func $store_span (param $out i32) (param $value_off i32) (param $value_len i32)
    local.get $out
    local.get $value_off
    i32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $value_len
    i32.store)

  (func $scan_i32
    (param $ptr i32) (param $len i32) (param $offset i32) (param $out i32)
    (param $prefix_len i32) (param $value_len i32)
    (result i64)
    (local $value_off i32)
    (local $remaining i32)
    (local $next_offset i32)

    local.get $out
    i32.eqz
    if
      i32.const 2
      local.get $offset
      call $pack
      return
    end

    local.get $offset
    local.get $len
    i32.gt_u
    if
      i32.const 1
      local.get $offset
      call $pack
      return
    end

    local.get $len
    local.get $offset
    i32.sub
    local.tee $remaining
    local.get $prefix_len
    i32.lt_u
    if
      i32.const 1
      local.get $offset
      call $pack
      return
    end

    local.get $offset
    local.get $prefix_len
    i32.add
    local.tee $value_off
    local.set $value_off

    local.get $value_len
    local.get $len
    local.get $value_off
    i32.sub
    i32.gt_u
    if
      i32.const 1
      local.get $offset
      call $pack
      return
    end

    local.get $value_off
    local.get $value_len
    i32.add
    local.set $next_offset

    local.get $out
    local.get $value_off
    local.get $value_len
    call $store_span

    i32.const 0
    local.get $next_offset
    call $pack)

  (func (export "length_field_scan_u8")
    (param $ptr i32) (param $len i32) (param $offset i32) (param $out i32)
    (result i64)
    local.get $offset
    local.get $len
    i32.ge_u
    if
      i32.const 1
      local.get $offset
      call $pack
      return
    end
    local.get $ptr
    local.get $len
    local.get $offset
    local.get $out
    i32.const 1
    local.get $ptr
    local.get $offset
    i32.add
    i32.load8_u
    call $scan_i32)

  (func (export "length_field_scan_u16_le")
    (param $ptr i32) (param $len i32) (param $offset i32) (param $out i32)
    (result i64)
    local.get $offset
    local.get $len
    i32.gt_u
    if
      i32.const 1
      local.get $offset
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
      local.get $offset
      call $pack
      return
    end
    local.get $ptr
    local.get $len
    local.get $offset
    local.get $out
    i32.const 2
    local.get $ptr
    local.get $offset
    i32.add
    call $m128read_u16_le
    call $scan_i32)

  (func (export "length_field_scan_u32_be")
    (param $ptr i32) (param $len i32) (param $offset i32) (param $out i32)
    (result i64)
    local.get $offset
    local.get $len
    i32.gt_u
    if
      i32.const 1
      local.get $offset
      call $pack
      return
    end
    local.get $len
    local.get $offset
    i32.sub
    i32.const 4
    i32.lt_u
    if
      i32.const 1
      local.get $offset
      call $pack
      return
    end
    local.get $ptr
    local.get $len
    local.get $offset
    local.get $out
    i32.const 4
    local.get $ptr
    local.get $offset
    i32.add
    call $m128read_u32_be
    call $scan_i32)

  (func (export "length_field_scan_u32_le")
    (param $ptr i32) (param $len i32) (param $offset i32) (param $out i32)
    (result i64)
    local.get $offset
    local.get $len
    i32.gt_u
    if
      i32.const 1
      local.get $offset
      call $pack
      return
    end
    local.get $len
    local.get $offset
    i32.sub
    i32.const 4
    i32.lt_u
    if
      i32.const 1
      local.get $offset
      call $pack
      return
    end
    local.get $ptr
    local.get $len
    local.get $offset
    local.get $out
    i32.const 4
    local.get $ptr
    local.get $offset
    i32.add
    call $m128read_u32_le
    call $scan_i32)

  (func (export "length_field_scan_u64_le")
    (param $ptr i32) (param $len i32) (param $offset i32) (param $out i32)
    (result i64)
    (local $field_len i64)

    local.get $offset
    local.get $len
    i32.gt_u
    if
      i32.const 1
      local.get $offset
      call $pack
      return
    end
    local.get $len
    local.get $offset
    i32.sub
    i32.const 8
    i32.lt_u
    if
      i32.const 1
      local.get $offset
      call $pack
      return
    end

    local.get $ptr
    local.get $offset
    i32.add
    call $read_u64_le
    local.tee $field_len
    i64.const 0xffffffff
    i64.gt_u
    if
      i32.const 4
      local.get $offset
      call $pack
      return
    end

    local.get $ptr
    local.get $len
    local.get $offset
    local.get $out
    i32.const 8
    local.get $field_len
    i32.wrap_i64
    call $scan_i32)

  (func $write_check (param $len i64) (param $out_cap i32) (param $prefix_len i32) (param $max_len i64) (result i64)
    local.get $out_cap
    local.get $prefix_len
    i32.lt_u
    if
      i32.const 2
      i32.const 0
      call $pack
      return
    end
    local.get $len
    local.get $max_len
    i64.gt_u
    if
      i32.const 4
      i32.const 0
      call $pack
      return
    end
    i32.const 0
    local.get $prefix_len
    call $pack)

  (func (export "length_field_write_u8")
    (param $len i64) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    local.get $len
    local.get $out_cap
    i32.const 1
    i64.const 255
    call $write_check
    i64.const 0xffffffff
    i64.and
    i32.wrap_i64
    i32.eqz
    if
      local.get $out_ptr
      local.get $len
      i32.wrap_i64
      i32.store8
    end
    local.get $len
    local.get $out_cap
    i32.const 1
    i64.const 255
    call $write_check)

  (func (export "length_field_write_u16_le")
    (param $len i64) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    local.get $len
    local.get $out_cap
    i32.const 2
    i64.const 65535
    call $write_check
    i64.const 0xffffffff
    i64.and
    i32.wrap_i64
    i32.eqz
    if
      local.get $out_ptr
      local.get $len
      i32.wrap_i64
      i32.store16
    end
    local.get $len
    local.get $out_cap
    i32.const 2
    i64.const 65535
    call $write_check)

  (func (export "length_field_write_u32_be")
    (param $len i64) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    local.get $len
    local.get $out_cap
    i32.const 4
    i64.const 0xffffffff
    call $write_check
    i64.const 0xffffffff
    i64.and
    i32.wrap_i64
    i32.eqz
    if
      local.get $out_ptr
      local.get $len
      i64.const 24
      i64.shr_u
      i32.wrap_i64
      i32.store8
      local.get $out_ptr
      i32.const 1
      i32.add
      local.get $len
      i64.const 16
      i64.shr_u
      i32.wrap_i64
      i32.store8
      local.get $out_ptr
      i32.const 2
      i32.add
      local.get $len
      i64.const 8
      i64.shr_u
      i32.wrap_i64
      i32.store8
      local.get $out_ptr
      i32.const 3
      i32.add
      local.get $len
      i32.wrap_i64
      i32.store8
    end
    local.get $len
    local.get $out_cap
    i32.const 4
    i64.const 0xffffffff
    call $write_check)

  (func (export "length_field_write_u32_le")
    (param $len i64) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    local.get $len
    local.get $out_cap
    i32.const 4
    i64.const 0xffffffff
    call $write_check
    i64.const 0xffffffff
    i64.and
    i32.wrap_i64
    i32.eqz
    if
      local.get $out_ptr
      local.get $len
      i32.wrap_i64
      i32.store
    end
    local.get $len
    local.get $out_cap
    i32.const 4
    i64.const 0xffffffff
    call $write_check)

  (func (export "length_field_write_u64_le")
    (param $len i64) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    local.get $len
    local.get $out_cap
    i32.const 8
    i64.const -1
    call $write_check
    i64.const 0xffffffff
    i64.and
    i32.wrap_i64
    i32.eqz
    if
      local.get $out_ptr
      local.get $len
      i64.store
    end
    local.get $len
    local.get $out_cap
    i32.const 8
    i64.const -1
    call $write_check)
