  (func $er_ui_validate (export "er_ui_validate") (param $base i32) (param $len i32) (result i32)
    (local $node_count i32)
    (local $root_count i32)
    (local $axis i32)
    (local $i i32)
    (local $roots i32)
    (local $parent_ref i32)
    (local $walk i32)
    (local $depth i32)
    (local $used_len i32)
    local.get $len
    i32.const 20
    i32.lt_u
    if
      i32.const 0
      return
    end
    local.get $base
    i32.load
    i32.const 0x49755245
    i32.ne
    if
      i32.const 0
      return
    end
    local.get $base
    i32.const 4
    i32.add
    i32.load
    local.tee $used_len
    i32.eqz
    i32.eqz
    if
      local.get $used_len
      local.get $len
      i32.ne
      local.get $base
      local.get $used_len
      i32.add
      local.get $len
      i32.ne
      i32.and
      if
        i32.const 0
        return
      end
    end
    local.get $base
    i32.const 16
    i32.add
    call $load16
    local.tee $node_count
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $base
    i32.const 18
    i32.add
    call $load16
    local.tee $root_count
    i32.eqz
    local.get $root_count
    local.get $node_count
    i32.gt_u
    i32.or
    if
      i32.const 0
      return
    end
    local.get $len
    i32.const 20
    local.get $node_count
    i32.const 16
    i32.mul
    i32.add
    i32.lt_u
    if
      i32.const 0
      return
    end
    local.get $base
    i32.const 10
    i32.add
    call $load16
    local.tee $axis
    i32.const 1
    i32.gt_u
    if
      i32.const 0
      return
    end
    block $done
      loop $loop
        local.get $i
        local.get $node_count
        i32.ge_u
        br_if $done
        local.get $base
        local.get $i
        call $record_at
        i32.const 2
        i32.add
        call $load16
        local.tee $parent_ref
        i32.eqz
        if
          local.get $roots
          i32.const 1
          i32.add
          local.set $roots
        else
          local.get $parent_ref
          local.get $node_count
          i32.gt_u
          local.get $parent_ref
          local.get $i
          i32.const 1
          i32.add
          i32.eq
          i32.or
          if
            i32.const 0
            return
          end
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $loop
      end
    end
    local.get $roots
    local.get $root_count
    i32.ne
    if
      i32.const 0
      return
    end
    i32.const 0
    local.set $i
    block $cycle_done
      loop $cycle_node
        local.get $i
        local.get $node_count
        i32.ge_u
        br_if $cycle_done
        local.get $i
        local.set $walk
        i32.const 0
        local.set $depth
        block $parent_done
          loop $parent_loop
            local.get $base
            local.get $walk
            call $record_at
            i32.const 2
            i32.add
            call $load16
            local.tee $parent_ref
            i32.eqz
            br_if $parent_done
            local.get $depth
            local.get $node_count
            i32.ge_u
            if
              i32.const 0
              return
            end
            local.get $parent_ref
            i32.const 1
            i32.sub
            local.set $walk
            local.get $depth
            i32.const 1
            i32.add
            local.set $depth
            br $parent_loop
          end
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $cycle_node
      end
    end
    i32.const 1)

  (func (export "er_ui_node_count") (param $base i32) (param $len i32) (result i32)
    local.get $base
    local.get $len
    call $er_ui_validate
    if (result i32)
      local.get $base
      i32.const 16
      i32.add
      call $load16
    else
      i32.const 0
    end)

  (func (export "er_ui_root_count") (param $base i32) (param $len i32) (result i32)
    local.get $base
    local.get $len
    call $er_ui_validate
    if (result i32)
      local.get $base
      i32.const 18
      i32.add
      call $load16
    else
      i32.const 0
    end)

  (func (export "er_ui_axis") (param $base i32) (param $len i32) (result i32)
    local.get $base
    local.get $len
    call $er_ui_validate
    if (result i32)
      local.get $base
      i32.const 10
      i32.add
      call $load16
    else
      i32.const -1
    end)

  (func (export "er_ui_gap") (param $base i32) (param $len i32) (result i32)
    local.get $base
    local.get $len
    call $er_ui_validate
    if (result i32)
      local.get $base
      i32.const 12
      i32.add
      call $load16
    else
      i32.const 0
    end)

  (func (export "er_ui_padding") (param $base i32) (param $len i32) (result i32)
    local.get $base
    local.get $len
    call $er_ui_validate
    if (result i32)
      local.get $base
      i32.const 14
      i32.add
      call $load16
    else
      i32.const 0
    end)

  (func $record_at (param $base i32) (param $index i32) (result i32)
    local.get $base
    i32.const 20
    i32.add
    local.get $index
    i32.const 16
    i32.mul
    i32.add)

  (func $er_ui_record_kind (export "er_ui_record_kind") (param $base i32) (param $len i32) (param $index i32) (result i32)
    (local $n i32)
    local.get $base
    local.get $len
    call $er_ui_validate
    i32.eqz
    if
      i32.const -1
      return
    end
    local.get $base
    i32.const 16
    i32.add
    call $load16
    local.set $n
    local.get $index
    local.get $n
    i32.ge_u
    if
      i32.const -1
      return
    end
    local.get $base
    local.get $index
    call $record_at
    call $load16)

  (func $er_ui_record_parent (export "er_ui_record_parent") (param $base i32) (param $len i32) (param $index i32) (result i32)
    (local $n i32)
    (local $parent_ref i32)
    local.get $base
    local.get $len
    call $er_ui_validate
    i32.eqz
    if
      i32.const -2
      return
    end
    local.get $base
    i32.const 16
    i32.add
    call $load16
    local.set $n
    local.get $index
    local.get $n
    i32.ge_u
    if
      i32.const -2
      return
    end
    local.get $base
    local.get $index
    call $record_at
    i32.const 2
    i32.add
    call $load16
    local.tee $parent_ref
    i32.eqz
    if
      i32.const -1
      return
    end
    local.get $parent_ref
    i32.const 1
    i32.sub)

  (func (export "er_ui_record_is_root") (param $base i32) (param $len i32) (param $index i32) (result i32)
    local.get $base
    local.get $len
    local.get $index
    call $er_ui_record_parent
    i32.const -1
    i32.eq)

  (func $er_ui_record_id (export "er_ui_record_id") (param $base i32) (param $len i32) (param $index i32) (result i32)
    (local $n i32)
    local.get $base
    local.get $len
    call $er_ui_validate
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $base
    i32.const 16
    i32.add
    call $load16
    local.set $n
    local.get $index
    local.get $n
    i32.ge_u
    if
      i32.const 0
      return
    end
    local.get $base
    local.get $index
    call $record_at
    i32.const 4
    i32.add
    i32.load)

  (func (export "er_ui_record_state_value") (param $base i32) (param $len i32) (param $index i32) (result i32)
    (local $n i32)
    (local $record i32)
    (local $kind i32)
    (local $id i32)
    local.get $base
    local.get $len
    call $er_ui_validate
    i32.eqz
    if
      i32.const -1
      return
    end
    local.get $base
    i32.const 16
    i32.add
    call $load16
    local.set $n
    local.get $index
    local.get $n
    i32.ge_u
    if
      i32.const -1
      return
    end
    local.get $base
    local.get $index
    call $record_at
    local.set $record
    local.get $record
    call $load16
    local.set $kind
    local.get $record
    i32.const 4
    i32.add
    i32.load
    local.set $id
    local.get $kind
    local.get $id
    call $record_encoded_state)

  (func (export "er_ui_record_base_id") (param $base i32) (param $len i32) (param $index i32) (result i32)
    (local $n i32)
    (local $record i32)
    (local $kind i32)
    (local $id i32)
    local.get $base
    local.get $len
    call $er_ui_validate
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $base
    i32.const 16
    i32.add
    call $load16
    local.set $n
    local.get $index
    local.get $n
    i32.ge_u
    if
      i32.const 0
      return
    end
    local.get $base
    local.get $index
    call $record_at
    local.set $record
    local.get $record
    call $load16
    local.set $kind
    local.get $record
    i32.const 4
    i32.add
    i32.load
    local.set $id
    local.get $kind
    i32.const 26
    i32.eq
    if
      local.get $id
      i32.const 14
      i32.shr_u
      return
    end
    local.get $id
    local.get $kind
    call $record_encoded_mul
    i32.div_u)

  (func (export "er_ui_record_icon_value") (param $base i32) (param $len i32) (param $index i32) (result i32)
    (local $n i32)
    (local $record i32)
    (local $kind i32)
    (local $id i32)
    local.get $base
    local.get $len
    call $er_ui_validate
    i32.eqz
    if
      i32.const -1
      return
    end
    local.get $base
    i32.const 16
    i32.add
    call $load16
    local.set $n
    local.get $index
    local.get $n
    i32.ge_u
    if
      i32.const -1
      return
    end
    local.get $base
    local.get $index
    call $record_at
    local.set $record
    local.get $record
    call $load16
    local.set $kind
    local.get $record
    i32.const 4
    i32.add
    i32.load
    local.set $id
    local.get $kind
    i32.const 26
    i32.eq
    if
      local.get $id
      i32.const 0x3fff
      i32.and
      return
    end
    local.get $kind
    i32.const 4
    i32.eq
    local.get $kind
    i32.const 11
    i32.eq
    i32.or
    if
      local.get $id
      i32.const 65535
      i32.and
      return
    end
    local.get $kind
    i32.const 12
    i32.eq
    local.get $kind
    i32.const 13
    i32.eq
    i32.or
    local.get $kind
    i32.const 16
    i32.eq
    i32.or
    local.get $kind
    i32.const 20
    i32.eq
    i32.or
    local.get $kind
    i32.const 44
    i32.eq
    i32.or
    local.get $kind
    i32.const 55
    i32.eq
    i32.or
    i32.eqz
    if
      i32.const -1
      return
    end
    local.get $record
    i32.const 12
    i32.add
    call $load16
    i32.const 65535
    i32.and)

  (func (export "er_ui_record_variant_value") (param $base i32) (param $len i32) (param $index i32) (result i32)
    (local $n i32)
    (local $record i32)
    (local $kind i32)
    (local $id i32)
    local.get $base
    local.get $len
    call $er_ui_validate
    i32.eqz
    if
      i32.const -1
      return
    end
    local.get $base
    i32.const 16
    i32.add
    call $load16
    local.set $n
    local.get $index
    local.get $n
    i32.ge_u
    if
      i32.const -1
      return
    end
    local.get $base
    local.get $index
    call $record_at
    local.set $record
    local.get $record
    call $load16
    local.set $kind
    local.get $record
    i32.const 4
    i32.add
    i32.load
    local.set $id
    local.get $kind
    i32.const 4
    i32.eq
    if
      local.get $id
      i32.const 16
      i32.shr_u
      i32.const 65535
      i32.and
      return
    end
    local.get $kind
    i32.const 12
    i32.eq
    if
      local.get $record
      i32.const 14
      i32.add
      call $load16
      i32.const 255
      i32.and
      return
    end
    local.get $kind
    i32.const 27
    i32.eq
    if
      local.get $record
      i32.const 12
      i32.add
      call $load16
      return
    end
    i32.const -1)

  (func (export "er_ui_record_bool_value") (param $base i32) (param $len i32) (param $index i32) (result i32)
    (local $n i32)
    (local $record i32)
    (local $kind i32)
    local.get $base
    local.get $len
    call $er_ui_validate
    i32.eqz
    if
      i32.const -1
      return
    end
    local.get $base
    i32.const 16
    i32.add
    call $load16
    local.set $n
    local.get $index
    local.get $n
    i32.ge_u
    if
      i32.const -1
      return
    end
    local.get $base
    local.get $index
    call $record_at
    local.set $record
    local.get $record
    call $load16
    local.set $kind
    local.get $kind
    i32.const 22
    i32.eq
    local.get $kind
    i32.const 23
    i32.eq
    i32.or
    local.get $kind
    i32.const 56
    i32.eq
    i32.or
    i32.eqz
    if
      i32.const -1
      return
    end
    local.get $record
    i32.const 12
    i32.add
    call $load16
    i32.const 0
    i32.ne)

  (func (export "er_ui_record_unit_value") (param $base i32) (param $len i32) (param $index i32) (result i32)
    (local $n i32)
    (local $record i32)
    (local $kind i32)
    local.get $base
    local.get $len
    call $er_ui_validate
    i32.eqz
    if
      i32.const -1
      return
    end
    local.get $base
    i32.const 16
    i32.add
    call $load16
    local.set $n
    local.get $index
    local.get $n
    i32.ge_u
    if
      i32.const -1
      return
    end
    local.get $base
    local.get $index
    call $record_at
    local.set $record
    local.get $record
    call $load16
    local.set $kind
    local.get $kind
    i32.const 7
    i32.eq
    local.get $kind
    i32.const 24
    i32.eq
    i32.or
    local.get $kind
    i32.const 37
    i32.eq
    i32.or
    local.get $kind
    i32.const 57
    i32.eq
    i32.or
    i32.eqz
    if
      i32.const -1
      return
    end
    local.get $record
    i32.const 12
    i32.add
    call $load16)

  (func (export "er_ui_record_aspect_ratio_w") (param $base i32) (param $len i32) (param $index i32) (result i32)
    (local $id i32)
    local.get $base
    local.get $len
    local.get $index
    call $er_ui_record_kind
    i32.const 6
    i32.ne
    if
      i32.const -1
      return
    end
    local.get $base
    local.get $len
    local.get $index
    call $er_ui_record_id
    local.set $id
    local.get $id
    i32.const 16
    i32.shr_u
    i32.const 65535
    i32.and)

  (func (export "er_ui_record_aspect_ratio_h") (param $base i32) (param $len i32) (param $index i32) (result i32)
    local.get $base
    local.get $len
    local.get $index
    call $er_ui_record_kind
    i32.const 6
    i32.ne
    if
      i32.const -1
      return
    end
    local.get $base
    local.get $len
    local.get $index
    call $er_ui_record_id
    i32.const 65535
    i32.and)

  (func $er_ui_record_first_ref (export "er_ui_record_first_ref") (param $base i32) (param $len i32) (param $index i32) (result i32)
    (local $n i32)
    (local $r i32)
    local.get $base
    local.get $len
    call $er_ui_validate
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $base
    i32.const 16
    i32.add
    call $load16
    local.set $n
    local.get $index
    local.get $n
    i32.ge_u
    if
      i32.const 0
      return
    end
    local.get $base
    local.get $index
    call $record_at
    local.set $r
    local.get $r
    i32.const 8
    i32.add
    call $load16
    local.get $r
    i32.const 10
    i32.add
    call $load16
    call $string_ref)

  (func $er_ui_record_second_ref (export "er_ui_record_second_ref") (param $base i32) (param $len i32) (param $index i32) (result i32)
    (local $n i32)
    (local $r i32)
    local.get $base
    local.get $len
    call $er_ui_validate
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $base
    i32.const 16
    i32.add
    call $load16
    local.set $n
    local.get $index
    local.get $n
    i32.ge_u
    if
      i32.const 0
      return
    end
    local.get $base
    local.get $index
    call $record_at
    local.set $r
    local.get $r
    i32.const 12
    i32.add
    call $load16
    local.get $r
    i32.const 14
    i32.add
    call $load16
    call $string_ref)

  (func (export "er_ui_ref_offset") (param $ref i32) (result i32)
    local.get $ref
    i32.const 65535
    i32.and)

  (func (export "er_ui_ref_len") (param $ref i32) (result i32)
    local.get $ref
    call $string_len)

  (func $er_ui_string_ptr (export "er_ui_string_ptr") (param $base i32) (param $len i32) (param $ref i32) (result i32)
    (local $n i32)
    local.get $base
    local.get $len
    call $er_ui_validate
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $base
    i32.const 16
    i32.add
    call $load16
    local.set $n
    local.get $base
    local.get $n
    local.get $ref
    call $string_ptr)

  (func (export "er_ui_string_ptr_checked") (param $base i32) (param $len i32) (param $ref i32) (result i32)
    local.get $base
    local.get $len
    local.get $ref
    call $er_ui_record_ref_valid
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $base
    local.get $len
    local.get $ref
    call $er_ui_string_ptr)

  (func (export "er_ui_string_len_checked") (param $base i32) (param $len i32) (param $ref i32) (result i32)
    local.get $base
    local.get $len
    local.get $ref
    call $er_ui_record_ref_valid
    i32.eqz
    if
      i32.const -1
      return
    end
    local.get $ref
    call $string_len)

  (func $er_ui_record_ref_valid (export "er_ui_record_ref_valid") (param $base i32) (param $len i32) (param $ref i32) (result i32)
    (local $n i32)
    (local $strings_len i32)
    local.get $base
    local.get $len
    call $er_ui_validate
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $base
    i32.const 16
    i32.add
    call $load16
    local.set $n
    local.get $len
    i32.const 20
    local.get $n
    i32.const 16
    i32.mul
    i32.add
    i32.sub
    local.set $strings_len
    local.get $ref
    i32.const 65535
    i32.and
    local.get $strings_len
    i32.le_u
    local.get $ref
    i32.const 65535
    i32.and
    local.get $ref
    i32.const 16
    i32.shr_u
    i32.add
    local.get $strings_len
    i32.le_u
    i32.and)

  (func $er_ui_record_refs_valid (export "er_ui_record_refs_valid") (param $base i32) (param $len i32) (param $index i32) (result i32)
    (local $first i32)
    (local $second i32)
    (local $kind i32)
    local.get $base
    local.get $len
    local.get $index
    call $er_ui_record_kind
    local.tee $kind
    i32.const -1
    i32.eq
    if
      i32.const -1
      return
    end
    local.get $base
    local.get $len
    local.get $index
    call $er_ui_record_first_ref
    local.set $first
    local.get $base
    local.get $len
    local.get $index
    call $er_ui_record_second_ref
    local.set $second
    local.get $base
    local.get $len
    local.get $first
    call $er_ui_record_ref_valid
    local.get $kind
    call $second_ref_is_packed
    if (result i32)
      local.get $kind
      local.get $second
      call $packed_second_ref_valid
    else
      local.get $base
      local.get $len
      local.get $second
      call $er_ui_record_ref_valid
    end
    i32.and)

  (func $er_ui_validate_deep (export "er_ui_validate_deep") (param $base i32) (param $len i32) (result i32)
    (local $n i32)
    (local $i i32)
    (local $kind i32)
    local.get $base
    local.get $len
    call $er_ui_validate
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $base
    i32.const 16
    i32.add
    call $load16
    local.set $n
    block $done
      loop $loop
        local.get $i
        local.get $n
        i32.ge_u
        br_if $done
        local.get $base
        local.get $i
        call $record_at
        call $load16
        local.tee $kind
        i32.const 58
        i32.ge_u
        if
          i32.const 0
          return
        end
        local.get $base
        local.get $len
        local.get $i
        call $er_ui_record_refs_valid
        i32.eqz
        if
          i32.const 0
          return
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $loop
      end
    end
    i32.const 1)
