
  (func $record_ptr (param $index i32) (result i32)
    global.get $writer_base
    i32.const 20
    i32.add
    local.get $index
    i32.const 16
    i32.mul
    i32.add)

  (func $table_start (param $base i32) (param $node_count i32) (result i32)
    local.get $base
    i32.const 20
    i32.add
    local.get $node_count
    i32.const 16
    i32.mul
    i32.add)

  (func $string_ptr (param $base i32) (param $node_count i32) (param $ref i32) (result i32)
    local.get $base
    local.get $node_count
    call $table_start
    local.get $ref
    i32.const 65535
    i32.and
    i32.add)

  (func $string_len (param $ref i32) (result i32)
    local.get $ref
    i32.const 16
    i32.shr_u
    i32.const 65535
    i32.and)

  (func $string_ref (param $offset i32) (param $len i32) (result i32)
    local.get $len
    i32.const 16
    i32.shl
    local.get $offset
    i32.const 65535
    i32.and
    i32.or)

  (func $second_ref_is_packed (param $kind i32) (result i32)
    local.get $kind
    i32.const 0x09D13080
    i32.const 0x03801020
    call $kind_in_mask)

  (func $kind_in_mask (param $kind i32) (param $lo i32) (param $hi i32) (result i32)
    local.get $kind
    i32.const 32
    i32.lt_u
    if (result i32)
      local.get $lo
      local.get $kind
      i32.shr_u
      i32.const 1
      i32.and
    else
      local.get $hi
      local.get $kind
      i32.const 32
      i32.sub
      i32.shr_u
      i32.const 1
      i32.and
    end)

  (func $packed_second_ref_valid (param $kind i32) (param $ref i32) (result i32)
    (local $ref_hi i32)
    (local $ref_lo i32)
    local.get $ref
    i32.const 16
    i32.shr_u
    local.set $ref_hi
    local.get $ref
    i32.const 65535
    i32.and
    local.set $ref_lo
    local.get $kind
    i32.const 0x00C00000
    i32.const 0x01000000
    call $kind_in_mask
    if (result i32)
      local.get $ref_hi
      i32.eqz
      local.get $ref_lo
      i32.const 1
      i32.le_u
      i32.and
    else
      local.get $kind
      i32.const 0x09110080
      i32.const 0x20010020
      call $kind_in_mask
      if (result i32)
        local.get $ref_hi
        i32.eqz
      else
        local.get $kind
        i32.const 12
        i32.eq
        if (result i32)
          local.get $ref_hi
          i32.const 511
          i32.le_u
        else
          local.get $kind
          i32.const 13
          i32.eq
          if (result i32)
            local.get $ref_lo
            i32.const 255
            i32.le_u
          else
            local.get $kind
            i32.const 55
            i32.eq
            if (result i32)
              local.get $ref_lo
              i32.eqz
            else
              i32.const 1
            end
          end
        end
      end
    end)

  (func $valid_rect (param $w f32) (param $h f32) (result i32)
    local.get $w
    f32.const 0
    f32.gt
    local.get $h
    f32.const 0
    f32.gt
    i32.and)

  (func $preferred_w (param $kind i32) (param $first_len i32) (result f32)
    local.get $kind
    i32.const 1
    i32.eq
    if (result f32)
      local.get $first_len
      f32.convert_i32_u
      f32.const 8
      f32.mul
    else
      local.get $kind
      i32.const 0
      i32.eq
      if (result f32)
        f32.const 1
      else
        local.get $kind
        i32.const 0x08000000
        i32.const 0x01001000
        call $kind_in_mask
        if (result f32)
          f32.const 64
        else
          local.get $kind
          i32.const 0x00C03000
          i32.const 0x02000490
          call $kind_in_mask
          if (result f32)
            f32.const 120
          else
            local.get $kind
            i32.const 0x030D0000
            i32.const 0x02000020
            call $kind_in_mask
            if (result f32)
              f32.const 180
            else
              local.get $kind
              i32.const 6
              i32.eq
              if (result f32)
                f32.const 160
              else
                f32.const 220
              end
            end
          end
        end
      end
    end)

  (func $preferred_h (param $kind i32) (result f32)
    local.get $kind
    i32.const 0
    i32.eq
    if (result f32)
      f32.const 1
    else
      local.get $kind
      i32.const 1
      i32.eq
      if (result f32)
        f32.const 18
      else
        local.get $kind
        i32.const 33
        i32.eq
        if (result f32)
          f32.const 1
        else
          local.get $kind
          i32.const 6
          i32.eq
          if (result f32)
            f32.const 90
          else
            local.get $kind
            i32.const 0x08000000
            i32.const 0x01000000
            call $kind_in_mask
            if (result f32)
              f32.const 24
            else
              local.get $kind
              i32.const 3
              i32.ge_u
              local.get $kind
              i32.const 58
              i32.lt_u
              i32.and
              if (result f32)
                f32.const 36
              else
                f32.const 32
              end
            end
          end
        end
      end
    end)

  (func $emit_rect (param $out i32) (param $cap i32) (param $count i32) (param $x f32) (param $y f32) (param $w f32) (param $h f32) (param $color i32) (result i32)
    (local $p i32)
    local.get $count
    i32.const 48
    i32.mul
    i32.const 48
    i32.add
    local.get $cap
    i32.gt_u
    if (result i32)
      i32.const -1
    else
      local.get $out
      local.get $count
      i32.const 48
      i32.mul
      i32.add
      local.set $p
      local.get $p
      i32.const 24
      i32.add
      i32.const 8
      i32.const 0
      memory.fill
      local.get $p
      i32.const 1
      call $store32
      local.get $p
      i32.const 4
      i32.add
      local.get $color
      call $store32
      local.get $p
      i32.const 8
      i32.add
      local.get $x
      f32.store
      local.get $p
      i32.const 12
      i32.add
      local.get $y
      f32.store
      local.get $p
      i32.const 16
      i32.add
      local.get $w
      f32.store
      local.get $p
      i32.const 20
      i32.add
      local.get $h
      f32.store
      local.get $count
      i32.const 1
      i32.add
    end)

  (func $emit_text (param $out i32) (param $cap i32) (param $count i32) (param $x f32) (param $y f32) (param $w f32) (param $h f32) (param $ptr i32) (param $len i32) (param $color i32) (result i32)
    (local $p i32)
    local.get $len
    i32.eqz
    if (result i32)
      local.get $count
    else
      local.get $count
      i32.const 48
      i32.mul
      i32.const 48
      i32.add
      local.get $cap
      i32.gt_u
      if (result i32)
        i32.const -1
      else
        local.get $out
        local.get $count
        i32.const 48
        i32.mul
        i32.add
        local.set $p
      local.get $p
      i32.const 2
      call $store32
        local.get $p
        i32.const 4
        i32.add
        local.get $color
        call $store32
        local.get $p
        i32.const 8
        i32.add
        local.get $x
        f32.store
        local.get $p
        i32.const 12
        i32.add
        local.get $y
        f32.store
        local.get $p
        i32.const 16
        i32.add
        local.get $w
        f32.store
        local.get $p
        i32.const 20
        i32.add
        local.get $h
        f32.store
        local.get $p
        i32.const 24
        i32.add
        local.get $ptr
        call $store32
        local.get $p
        i32.const 28
        i32.add
        local.get $len
        call $store32
        local.get $count
        i32.const 1
        i32.add
      end
    end)

  (func $emit_icon_or_rect (param $out i32) (param $cap i32) (param $count i32) (param $x f32) (param $y f32) (param $w f32) (param $h f32) (param $icon i32) (param $color i32) (result i32)
    local.get $icon
    call $er_ui_icon_valid
    if (result i32)
      local.get $out
      local.get $cap
      local.get $count
      local.get $x
      local.get $y
      local.get $w
      local.get $h
      local.get $icon
      local.get $color
      call $er_ui_command_write_icon
    else
      local.get $out
      local.get $cap
      local.get $count
      local.get $x
      local.get $y
      local.get $w
      local.get $h
      local.get $color
      call $emit_rect
    end)

  (func $command_set_owner_at (param $commands i32) (param $index i32) (param $id i32) (param $kind i32) (param $role i32)
    (local $p i32)
    local.get $commands
    local.get $index
    i32.const 48
    i32.mul
    i32.add
    local.set $p
    local.get $p
    i32.const 32
    i32.add
    local.get $id
    call $store32
    local.get $p
    i32.const 36
    i32.add
    local.get $kind
    call $store32
    local.get $p
    i32.const 40
    i32.add
    local.get $role
    call $store32)

  (func $command_set_meta_at (param $commands i32) (param $index i32) (param $meta i32)
    local.get $commands
    local.get $index
    i32.const 48
    i32.mul
    i32.add
    i32.const 44
    i32.add
    local.get $meta
    call $store32)

  (func $record_kind_group (param $kind i32) (result i32)
    local.get $kind
    i32.const 0x2004008
    i32.const 0xC00
    call $kind_in_mask
    if (result i32)
      i32.const 0
    else
      local.get $kind
      i32.const 0x8000
      i32.const 0x380
      call $kind_in_mask
      if (result i32)
        i32.const 1
      else
        i32.const 2
      end
    end)

  (func $record_encoded_state (param $kind i32) (param $id i32) (result i32)
    (local $group i32)
    local.get $kind
    call $record_kind_group
    local.set $group
    local.get $group
    i32.eqz
    if
      local.get $id
      i32.const 2
      i32.rem_u
      return
    end
    local.get $group
    i32.const 1
    i32.eq
    if
      local.get $id
      i32.const 3
      i32.rem_u
      return
    end
    i32.const -1)

  (func $record_encoded_mul (param $kind i32) (result i32)
    (local $group i32)
    local.get $kind
    call $record_kind_group
    local.set $group
    local.get $group
    i32.eqz
    if
      i32.const 2
      return
    end
    local.get $group
    i32.const 1
    i32.eq
    if
      i32.const 3
      return
    end
    i32.const 1)
  (func $finite_f32 (param $value f32) (result i32)
    local.get $value
    i32.reinterpret_f32
    i32.const 0x7f800000
    i32.and
    i32.const 0x7f800000
    i32.ne)
