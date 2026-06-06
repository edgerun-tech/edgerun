  (func $er_ui_list_equal_segment_bounds_gap (export "er_ui_list_equal_segment_bounds_gap") (param $bounds i32) (param $index i32) (param $item_count i32) (param $gap f32) (param $out i32) (result i32)
    (local $count f32)
    (local $total_gap f32)
    (local $segment_w f32)
    local.get $bounds
    i32.eqz
    local.get $out
    i32.eqz
    i32.or
    if
      i32.const 0
      return
    end
    local.get $item_count
    f32.convert_i32_u
    f32.const 1
    call $max_f32
    local.set $count
    local.get $gap
    local.get $count
    f32.const 1
    f32.sub
    f32.const 0
    call $max_f32
    f32.mul
    local.set $total_gap
    local.get $bounds
    i32.const 8
    i32.add
    f32.load
    local.get $total_gap
    f32.sub
    local.get $count
    f32.div
    f32.const 1
    call $max_f32
    local.set $segment_w
    local.get $out
    local.get $bounds
    f32.load
    local.get $index
    f32.convert_i32_u
    local.get $segment_w
    local.get $gap
    f32.add
    f32.mul
    f32.add
    local.get $bounds
    i32.const 4
    i32.add
    f32.load
    local.get $segment_w
    local.get $bounds
    i32.const 12
    i32.add
    f32.load
    call $rect_store)

  (func (export "er_ui_list_equal_segment_bounds") (param $bounds i32) (param $index i32) (param $item_count i32) (param $out i32) (result i32)
    local.get $bounds
    local.get $index
    local.get $item_count
    f32.const 0
    local.get $out
    call $er_ui_list_equal_segment_bounds_gap)

  (func (export "er_ui_list_padded_equal_segment_bounds") (param $bounds i32) (param $index i32) (param $item_count i32) (param $padding f32) (param $out i32) (result i32)
    (local $content_x f32)
    (local $content_y f32)
    (local $content_w f32)
    (local $content_h f32)
    (local $count f32)
    (local $segment_w f32)
    local.get $bounds
    i32.eqz
    local.get $out
    i32.eqz
    i32.or
    if
      i32.const 0
      return
    end
    local.get $bounds
    f32.load
    local.get $padding
    f32.add
    local.set $content_x
    local.get $bounds
    i32.const 4
    i32.add
    f32.load
    local.get $padding
    f32.add
    local.set $content_y
    local.get $bounds
    i32.const 8
    i32.add
    f32.load
    local.get $padding
    f32.const 2
    f32.mul
    f32.sub
    f32.const 0
    call $max_f32
    local.set $content_w
    local.get $bounds
    i32.const 12
    i32.add
    f32.load
    local.get $padding
    f32.const 2
    f32.mul
    f32.sub
    f32.const 0
    call $max_f32
    local.set $content_h
    local.get $item_count
    f32.convert_i32_u
    f32.const 1
    call $max_f32
    local.set $count
    local.get $content_w
    local.get $count
    f32.div
    f32.const 1
    call $max_f32
    local.set $segment_w
    local.get $out
    local.get $content_x
    local.get $index
    f32.convert_i32_u
    local.get $segment_w
    f32.mul
    f32.add
    local.get $content_y
    local.get $segment_w
    local.get $content_h
    call $rect_store)

  (func $er_ui_list_item_strip_bounds (export "er_ui_list_item_strip_bounds") (param $bounds i32) (param $index i32) (param $widths_ptr i32) (param $width_count i32) (param $padding f32) (param $gap f32) (param $item_h f32) (param $out i32) (result i32)
    (local $i i32)
    (local $limit i32)
    (local $x f32)
    (local $resolved_w f32)
    (local $resolved_h f32)
    local.get $bounds
    i32.eqz
    local.get $out
    i32.eqz
    i32.or
    if
      i32.const 0
      return
    end
    local.get $bounds
    f32.load
    local.get $padding
    f32.add
    local.set $x
    local.get $index
    local.get $width_count
    i32.lt_u
    if
      local.get $index
      local.set $limit
    else
      local.get $width_count
      local.set $limit
    end
    block $done
      loop $widths
        local.get $i
        local.get $limit
        i32.ge_u
        br_if $done
        local.get $widths_ptr
        i32.eqz
        if
          br $done
        end
        local.get $x
        local.get $widths_ptr
        local.get $i
        i32.const 4
        i32.mul
        i32.add
        f32.load
        local.get $gap
        f32.add
        f32.add
        local.set $x
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $widths
      end
    end
    local.get $index
    local.get $width_count
    i32.lt_u
    local.get $widths_ptr
    i32.const 0
    i32.ne
    i32.and
    if
      local.get $widths_ptr
      local.get $index
      i32.const 4
      i32.mul
      i32.add
      f32.load
      local.set $resolved_w
    else
      f32.const 1
      local.set $resolved_w
    end
    local.get $bounds
    i32.const 12
    i32.add
    f32.load
    local.get $padding
    f32.const 2
    f32.mul
    f32.sub
    f32.const 1
    call $max_f32
    local.get $item_h
    call $min_f32
    local.set $resolved_h
    local.get $out
    local.get $x
    local.get $bounds
    i32.const 4
    i32.add
    f32.load
    local.get $padding
    f32.add
    local.get $resolved_w
    local.get $resolved_h
    call $rect_store)

  (func $er_ui_list_clamped_index (export "er_ui_list_clamped_index") (param $value i32) (param $item_count i32) (result i32)
    local.get $item_count
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $value
    local.get $item_count
    i32.const 1
    i32.sub
    i32.gt_u
    if (result i32)
      local.get $item_count
      i32.const 1
      i32.sub
    else
      local.get $value
    end)

  (func (export "er_ui_list_resolve_index") (param $controlled i32) (param $default_value i32) (param $item_count i32) (result i32)
    local.get $controlled
    i32.const -1
    i32.eq
    if (result i32)
      local.get $default_value
    else
      local.get $controlled
    end
    local.get $item_count
    call $er_ui_list_clamped_index)

  (func $er_ui_list_encoded_indexed_id (export "er_ui_list_encoded_indexed_id") (param $id i32) (param $active i32) (param $item_count i32) (result i32)
    local.get $id
    local.get $item_count
    i32.mul
    local.get $active
    local.get $item_count
    call $er_ui_list_clamped_index
    i32.add)
