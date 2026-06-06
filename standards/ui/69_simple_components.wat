  (func (export "er_ui_component_size_small") (result i32) i32.const 0)
  (func (export "er_ui_component_size_default") (result i32) i32.const 1)
  (func (export "er_ui_component_size_large") (result i32) i32.const 2)

  (func (export "er_ui_badge_height") (result f32) f32.const 24)
  (func (export "er_ui_badge_text_height") (result f32) f32.const 13)
  (func (export "er_ui_badge_padding_x") (result f32) f32.const 12)
  (func (export "er_ui_badge_min_width") (result f32) f32.const 28)

  (func (export "er_ui_badge_label_bounds") (param $bounds i32) (param $out i32) (result i32)
    (local $padding f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    f32.const 12
    local.get $bounds i32.const 8 i32.add f32.load f32.const 0.5 f32.mul
    call $min_f32
    local.set $padding
    local.get $out
    local.get $bounds f32.load local.get $padding f32.add
    local.get $bounds i32.const 4 i32.add f32.load local.get $bounds i32.const 12 i32.add f32.load f32.const 13 f32.sub f32.const 0.5 f32.mul f32.add f32.const 1 f32.add
    local.get $bounds i32.const 8 i32.add f32.load local.get $padding f32.const 2 f32.mul f32.sub f32.const 1 call $max_f32
    f32.const 13
    call $rect_store)

  (func (export "er_ui_badge_measure") (param $label_ptr i32) (param $label_len i32) (param $constraints i32) (param $out i32) (result i32)
    (local $preferred_w f32) (local $preferred_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $constraints
    local.get $label_ptr local.get $label_len f32.const 13 call $er_ui_font_text_width
    f32.const 24
    f32.add
    f32.const 28
    call $max_f32
    call $er_ui_layout_axis_constraint_limit
    local.set $preferred_w
    local.get $constraints i32.const 8 i32.add f32.const 24 call $er_ui_layout_axis_constraint_limit local.set $preferred_h
    f32.const 28 local.get $preferred_w call $min_f32
    f32.const 24 local.get $preferred_h call $min_f32
    local.get $preferred_w local.get $preferred_h
    local.get $constraints local.get $preferred_w call $er_ui_layout_axis_constraint_limit
    f32.const 24
    local.get $out
    call $er_ui_layout_measurement_flexible
    drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)

  (func $er_ui_button_height_for_size (export "er_ui_button_height_for_size") (param $size i32) (result f32)
    local.get $size i32.eqz
    if f32.const 32 return end
    local.get $size i32.const 2 i32.eq
    if f32.const 44 return end
    f32.const 36)

  (func $er_ui_button_label_padding_for_size (export "er_ui_button_label_padding_for_size") (param $size i32) (result f32)
    local.get $size i32.eqz
    if f32.const 12 return end
    local.get $size i32.const 2 i32.eq
    if f32.const 20 return end
    f32.const 16)

  (func $er_ui_button_min_width_for_size (export "er_ui_button_min_width_for_size") (param $size i32) (result f32)
    local.get $size i32.eqz
    if f32.const 36 return end
    local.get $size i32.const 2 i32.eq
    if f32.const 52 return end
    f32.const 44)

  (func $er_ui_icon_button_size_for_size (export "er_ui_icon_button_size_for_size") (param $size i32) (result f32)
    local.get $size i32.eqz
    if f32.const 32 return end
    local.get $size i32.const 2 i32.eq
    if f32.const 44 return end
    f32.const 36)

  (func $er_ui_button_icon_cluster_width (export "er_ui_button_icon_cluster_width") (param $icon_count i32) (param $has_label i32) (result f32)
    (local $width f32)
    local.get $icon_count i32.eqz
    if f32.const 0 return end
    local.get $icon_count f32.convert_i32_u f32.const 18 f32.mul
    local.get $icon_count i32.const 1 i32.sub f32.convert_i32_u f32.const 8 f32.mul
    f32.add
    local.set $width
    local.get $has_label
    if
      local.get $width
      local.get $icon_count f32.convert_i32_u f32.const 8 f32.mul
      f32.add
      local.set $width
    end
    local.get $width)

  (func $er_ui_button_preferred_width_for_size (export "er_ui_button_preferred_width_for_size") (param $label_ptr i32) (param $label_len i32) (param $icon_count i32) (param $size i32) (result f32)
    (local $label_w f32)
    local.get $label_len
    i32.eqz
    if
      f32.const 0
      local.set $label_w
    else
      local.get $label_ptr local.get $label_len f32.const 17 call $er_ui_font_text_width
      f32.const 8
      call $max_f32
      local.set $label_w
    end
    local.get $size call $er_ui_button_min_width_for_size
    local.get $label_w
    local.get $icon_count
    local.get $label_len i32.const 0 i32.ne
    call $er_ui_button_icon_cluster_width
    f32.add
    local.get $size call $er_ui_button_label_padding_for_size f32.const 2 f32.mul f32.add
    call $max_f32)

  (func (export "er_ui_button_measure") (param $label_ptr i32) (param $label_len i32) (param $icon_count i32) (param $size i32) (param $constraints i32) (param $out i32) (result i32)
    (local $preferred_w f32) (local $preferred_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $constraints local.get $label_ptr local.get $label_len local.get $icon_count local.get $size call $er_ui_button_preferred_width_for_size call $er_ui_layout_axis_constraint_limit local.set $preferred_w
    local.get $constraints i32.const 8 i32.add local.get $size call $er_ui_button_height_for_size call $er_ui_layout_axis_constraint_limit local.set $preferred_h
    local.get $size call $er_ui_button_min_width_for_size local.get $preferred_w call $min_f32
    local.get $size call $er_ui_button_height_for_size local.get $preferred_h call $min_f32
    local.get $preferred_w local.get $preferred_h
    local.get $constraints local.get $preferred_w call $er_ui_layout_axis_constraint_limit
    local.get $size call $er_ui_button_height_for_size
    local.get $out
    call $er_ui_layout_measurement_flexible
    drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)

  (func (export "er_ui_icon_button_measure") (param $size i32) (param $constraints i32) (param $out i32) (result i32)
    (local $resolved f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $size call $er_ui_icon_button_size_for_size local.set $resolved
    local.get $constraints local.get $resolved call $er_ui_layout_axis_constraint_limit
    local.get $constraints i32.const 8 i32.add local.get $resolved call $er_ui_layout_axis_constraint_limit
    local.get $out
    call $er_ui_layout_measurement_fixed
    drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)

  (func $er_ui_input_control_height (export "er_ui_input_control_height") (param $size i32) (result f32)
    local.get $size i32.eqz
    if f32.const 32 return end
    local.get $size i32.const 2 i32.eq
    if f32.const 48 return end
    f32.const 40)

  (func $er_ui_input_padding (export "er_ui_input_padding") (param $size i32) (result f32)
    local.get $size i32.eqz
    if f32.const 10 return end
    local.get $size i32.const 2 i32.eq
    if f32.const 16 return end
    f32.const 12)

  (func (export "er_ui_input_preferred_size") (param $size i32) (param $out i32) (result i32)
    local.get $out f32.const 44 local.get $size call $er_ui_input_control_height call $layout_store_size)

  (func (export "er_ui_input_measure") (param $text_ptr i32) (param $text_len i32) (param $has_icon i32) (param $size i32) (param $constraints i32) (param $out i32) (result i32)
    (local $padding f32) (local $icon_w f32) (local $label_w f32) (local $label_h f32) (local $height f32) (local $preferred_w f32) (local $preferred_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $size call $er_ui_input_padding local.set $padding
    local.get $has_icon if f32.const 24 local.set $icon_w end
    local.get $text_ptr local.get $text_len f32.const 16 call $er_ui_font_text_width local.set $label_w
    f32.const 16 local.set $label_h
    local.get $size call $er_ui_input_control_height local.set $height
    local.get $constraints local.get $label_w local.get $padding f32.const 2 f32.mul f32.add local.get $icon_w f32.add call $er_ui_layout_axis_constraint_limit local.set $preferred_w
    local.get $constraints i32.const 8 i32.add local.get $height local.get $label_h local.get $padding f32.const 2 f32.mul f32.add call $max_f32 call $er_ui_layout_axis_constraint_limit local.set $preferred_h
    f32.const 44 local.get $preferred_w call $min_f32
    local.get $height local.get $preferred_h call $min_f32
    local.get $preferred_w local.get $preferred_h
    local.get $constraints local.get $preferred_w call $er_ui_layout_axis_constraint_limit
    local.get $preferred_h local.get $height call $max_f32
    local.get $out
    call $er_ui_layout_measurement_flexible
    drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)

  (func (export "er_ui_textarea_measure") (param $placeholder_ptr i32) (param $placeholder_len i32) (param $constraints i32) (param $out i32) (result i32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    i32.const 120488 f32.const 12 f32.store
    i32.const 120492 f32.const 12 f32.store
    i32.const 120496 f32.const 12 f32.store
    i32.const 120500 f32.const 12 f32.store
    local.get $constraints i32.const 120488 i32.const 120504 call $er_ui_layout_constraints_inner drop
    i32.const 120532 f32.const 16 f32.store
    i32.const 120536 f32.const 8.5 f32.store
    i32.const 120540 i32.const 4 i32.store
    local.get $placeholder_ptr local.get $placeholder_len i32.const 120504 i32.const 120532 i32.const 120544 call $er_ui_text_component_measure_value drop
    i32.const 120544 i32.const 120488 i32.const 120568 call $er_ui_layout_measurement_with_insets drop
    i32.const 120576 f32.load i32.const 120580 f32.load local.get $constraints i32.const 120600 call $er_ui_primitives_constrain_preferred_size drop
    f32.const 96 i32.const 120600 f32.load call $min_f32
    f32.const 40 i32.const 120604 f32.load call $min_f32
    i32.const 120600 f32.load
    i32.const 120604 f32.load
    local.get $constraints i32.const 120600 f32.load call $er_ui_layout_axis_constraint_limit
    i32.const 120604 f32.load i32.const 120588 f32.load call $max_f32
    local.get $out
    call $er_ui_layout_measurement_flexible
    drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)

  (func $textarea_line_start_at (param $ptr i32) (param $len i32) (param $target_line i32) (result i32)
    (local $line i32) (local $index i32)
    block $done
      loop $scan
        local.get $index local.get $len i32.ge_u
        local.get $line local.get $target_line i32.ge_u
        i32.or
        br_if $done
        local.get $ptr local.get $index i32.add i32.load8_u i32.const 10 i32.eq
        if local.get $line i32.const 1 i32.add local.set $line end
        local.get $index i32.const 1 i32.add local.set $index
        br $scan
      end
    end
    local.get $index)

  (func $textarea_line_end_at (param $ptr i32) (param $len i32) (param $start i32) (result i32)
    (local $index i32)
    local.get $start local.get $len call $layout_min_i32_u local.set $index
    block $done
      loop $scan
        local.get $index local.get $len i32.ge_u br_if $done
        local.get $ptr local.get $index i32.add i32.load8_u i32.const 10 i32.eq br_if $done
        local.get $index i32.const 1 i32.add local.set $index
        br $scan
      end
    end
    local.get $index)

  (func (export "er_ui_textarea_cursor_from_point") (param $text_ptr i32) (param $text_len i32) (param $bounds i32) (param $x f32) (param $y f32) (param $first_line i32) (param $line_height f32) (param $char_width f32) (param $gutter_width f32) (param $padding_left f32) (param $padding_top f32) (result i32)
    (local $local_x f32) (local $local_y f32) (local $line_offset i32) (local $target_column i32) (local $start i32) (local $end i32)
    local.get $x local.get $bounds f32.load f32.sub local.get $padding_left f32.sub local.get $gutter_width f32.sub f32.const 0 call $max_f32 local.set $local_x
    local.get $y local.get $bounds i32.const 4 i32.add f32.load f32.sub local.get $padding_top f32.sub f32.const 0 call $max_f32 local.set $local_y
    local.get $local_y local.get $line_height f32.const 1 call $max_f32 f32.div i32.trunc_f32_u local.set $line_offset
    local.get $local_x local.get $char_width f32.const 1 call $max_f32 f32.div i32.trunc_f32_u local.set $target_column
    local.get $text_ptr local.get $text_len local.get $first_line local.get $line_offset i32.add call $textarea_line_start_at local.set $start
    local.get $text_ptr local.get $text_len local.get $start call $textarea_line_end_at local.set $end
    local.get $start local.get $target_column local.get $end local.get $start i32.sub call $layout_min_i32_u i32.add)
