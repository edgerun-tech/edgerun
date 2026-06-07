
  (func $er_ui_badge_height  (result f32) f32.const 24)
  (func $er_ui_badge_text_height  (result f32) f32.const 13)
  (func $er_ui_badge_padding_x  (result f32) f32.const 12)
  (func $er_ui_badge_min_width  (result f32) f32.const 28)

  (func $er_ui_badge_label_bounds  (param $bounds i32) (param $out i32) (result i32)
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

  (func $er_ui_badge_measure  (param $label_ptr i32) (param $label_len i32) (param $constraints i32) (param $out i32) (result i32)
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

  (func $er_ui_button_height_for_size  (param $size i32) (result f32)
    local.get $size i32.eqz
    if f32.const 32 return end
    local.get $size i32.const 2 i32.eq
    if f32.const 44 return end
    f32.const 36)

  (func $er_ui_button_label_padding_for_size  (param $size i32) (result f32)
    local.get $size i32.eqz
    if f32.const 12 return end
    local.get $size i32.const 2 i32.eq
    if f32.const 20 return end
    f32.const 16)

  (func $er_ui_button_min_width_for_size  (param $size i32) (result f32)
    local.get $size i32.eqz
    if f32.const 36 return end
    local.get $size i32.const 2 i32.eq
    if f32.const 52 return end
    f32.const 44)

  (func $er_ui_icon_button_size_for_size  (param $size i32) (result f32)
    local.get $size i32.eqz
    if f32.const 32 return end
    local.get $size i32.const 2 i32.eq
    if f32.const 44 return end
    f32.const 36)

  (func $er_ui_button_icon_cluster_width  (param $icon_count i32) (param $has_label i32) (result f32)
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

  (func $er_ui_button_preferred_width_for_size  (param $label_ptr i32) (param $label_len i32) (param $icon_count i32) (param $size i32) (result f32)
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

  (func $er_ui_button_measure  (param $label_ptr i32) (param $label_len i32) (param $icon_count i32) (param $size i32) (param $constraints i32) (param $out i32) (result i32)
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

  (func $er_ui_icon_button_measure  (param $size i32) (param $constraints i32) (param $out i32) (result i32)
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

  (func $er_ui_input_control_height  (param $size i32) (result f32)
    local.get $size i32.eqz
    if f32.const 32 return end
    local.get $size i32.const 2 i32.eq
    if f32.const 48 return end
    f32.const 40)

  (func $er_ui_input_padding  (param $size i32) (result f32)
    local.get $size i32.eqz
    if f32.const 10 return end
    local.get $size i32.const 2 i32.eq
    if f32.const 16 return end
    f32.const 12)

  (func $er_ui_input_preferred_size  (param $size i32) (param $out i32) (result i32)
    local.get $out f32.const 44 local.get $size call $er_ui_input_control_height call $layout_store_size)

  (func $er_ui_input_measure  (param $text_ptr i32) (param $text_len i32) (param $has_icon i32) (param $size i32) (param $constraints i32) (param $out i32) (result i32)
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

  (func $er_ui_textarea_measure  (param $placeholder_ptr i32) (param $placeholder_len i32) (param $constraints i32) (param $out i32) (result i32)
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
    local.get $start local.get $len call $min_i32_u local.set $index
    block $done
      loop $scan
        local.get $index local.get $len i32.ge_u br_if $done
        local.get $ptr local.get $index i32.add i32.load8_u i32.const 10 i32.eq br_if $done
        local.get $index i32.const 1 i32.add local.set $index
        br $scan
      end
    end
    local.get $index)

  (func $er_ui_textarea_cursor_from_point  (param $text_ptr i32) (param $text_len i32) (param $bounds i32) (param $x f32) (param $y f32) (param $first_line i32) (param $line_height f32) (param $char_width f32) (param $gutter_width f32) (param $padding_left f32) (param $padding_top f32) (result i32)
    (local $local_x f32) (local $local_y f32) (local $line_offset i32) (local $target_column i32) (local $start i32) (local $end i32)
    local.get $x local.get $bounds f32.load f32.sub local.get $padding_left f32.sub local.get $gutter_width f32.sub f32.const 0 call $max_f32 local.set $local_x
    local.get $y local.get $bounds i32.const 4 i32.add f32.load f32.sub local.get $padding_top f32.sub f32.const 0 call $max_f32 local.set $local_y
    local.get $local_y local.get $line_height f32.const 1 call $max_f32 f32.div i32.trunc_f32_u local.set $line_offset
    local.get $local_x local.get $char_width f32.const 1 call $max_f32 f32.div i32.trunc_f32_u local.set $target_column
    local.get $text_ptr local.get $text_len local.get $first_line local.get $line_offset i32.add call $textarea_line_start_at local.set $start
    local.get $text_ptr local.get $text_len local.get $start call $textarea_line_end_at local.set $end
    local.get $start local.get $target_column local.get $end local.get $start i32.sub call $min_i32_u i32.add)
  (func $er_ui_row_item_min_width  (result f32) f32.const 96)
  (func $er_ui_row_item_min_height  (result f32) f32.const 32)
  (func $er_ui_row_item_text_width  (param $bounds i32) (param $has_icon i32) (result f32)
    (local $width f32)
    local.get $bounds i32.eqz
    if f32.const 0 return end
    local.get $bounds i32.const 8 i32.add f32.load
    f32.const 24
    f32.sub
    local.set $width
    local.get $has_icon
    if
      local.get $width
      f32.const 36
      f32.sub
      local.set $width
    end
    local.get $width
    f32.const 1
    call $max_f32)

  (func $er_ui_row_item_text_bounds  (param $bounds i32) (param $has_icon i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds
    local.get $out
    f32.const 12
    local.get $has_icon
    if (result f32)
      f32.const 36
    else
      f32.const 0
    end
    f32.add
    f32.const 0
    f32.const 12
    f32.const 0
    call $er_ui_rect_inset_ltrb
    drop
    local.get $out call $er_ui_rect_valid)

  (func $er_ui_row_item_measure  (param $title_ptr i32) (param $title_len i32) (param $detail_ptr i32) (param $detail_len i32) (param $has_icon i32) (param $constraints i32) (param $out i32) (result i32)
    (local $icon_extra f32) (local $gap f32) (local $pref_w f32) (local $pref_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $has_icon if f32.const 36 local.set $icon_extra end
    i32.const 120624 f32.const 0 f32.store
    i32.const 120628 f32.const 12 f32.store
    i32.const 120632 f32.const 0 f32.store
    i32.const 120636 f32.const 12 local.get $icon_extra f32.add f32.store
    local.get $constraints i32.const 120624 i32.const 120640 call $er_ui_layout_constraints_inner drop
    i32.const 120668 f32.const 15 f32.store
    i32.const 120672 local.get $title_ptr local.get $title_len f32.const 15 call $primitives_average_width f32.store
    i32.const 120676 i32.const 1 i32.store
    local.get $title_ptr local.get $title_len i32.const 120640 i32.const 120668 i32.const 120680 call $er_ui_text_component_measure_value drop
    local.get $detail_len i32.eqz
    if
      f32.const 0 f32.const 0 i32.const 120704 call $er_ui_layout_measurement_fixed drop
      f32.const 0 local.set $gap
    else
      i32.const 120728 f32.const 13 f32.store
      i32.const 120732 local.get $detail_ptr local.get $detail_len f32.const 13 call $primitives_average_width f32.store
      i32.const 120736 i32.const 1 i32.store
      local.get $detail_ptr local.get $detail_len i32.const 120640 i32.const 120728 i32.const 120704 call $er_ui_text_component_measure_value drop
      f32.const 2 local.set $gap
    end
    local.get $constraints
    f32.const 96 i32.const 120688 f32.load i32.const 120712 f32.load call $max_f32 f32.const 24 f32.add local.get $icon_extra f32.add call $max_f32
    call $er_ui_layout_axis_constraint_limit
    local.set $pref_w
    local.get $constraints i32.const 8 i32.add
    f32.const 12 i32.const 120692 f32.load f32.add local.get $gap f32.add i32.const 120716 f32.load f32.add
    f32.const 38
    call $max_f32
    call $er_ui_layout_axis_constraint_limit
    local.set $pref_h
    f32.const 96 local.get $pref_w call $min_f32
    f32.const 32 local.get $pref_h call $min_f32
    local.get $pref_w local.get $pref_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit
    local.get $pref_h
    local.get $out
    call $er_ui_layout_measurement_flexible
    drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)

  (func $er_ui_card_measure  (param $title_ptr i32) (param $title_len i32) (param $detail_ptr i32) (param $detail_len i32) (param $constraints i32) (param $out i32) (result i32)
    (local $gap f32) (local $content_w f32) (local $content_h f32) (local $min_h f32) (local $pref_w f32) (local $pref_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    f32.const 16 i32.const 120624 call $er_ui_layout_insets_uniform drop
    local.get $constraints i32.const 120624 i32.const 120640 call $er_ui_layout_constraints_inner drop
    local.get $title_len i32.eqz
    if
      f32.const 0 f32.const 0 i32.const 120680 call $er_ui_layout_measurement_fixed drop
    else
      i32.const 120668 f32.const 18 f32.store
      i32.const 120672 f32.const 8.5 f32.store
      i32.const 120676 i32.const 2 i32.store
      local.get $title_ptr local.get $title_len i32.const 120640 i32.const 120668 i32.const 120680 call $er_ui_text_component_measure_value drop
    end
    local.get $detail_len i32.eqz
    if
      f32.const 0 f32.const 0 i32.const 120704 call $er_ui_layout_measurement_fixed drop
    else
      i32.const 120728 f32.const 16 f32.store
      i32.const 120732 f32.const 8 f32.store
      i32.const 120736 i32.const 3 i32.store
      local.get $detail_ptr local.get $detail_len i32.const 120640 i32.const 120728 i32.const 120704 call $er_ui_text_component_measure_value drop
    end
    local.get $title_len i32.const 0 i32.ne local.get $detail_len i32.const 0 i32.ne i32.and
    if f32.const 8 local.set $gap end
    i32.const 120688 f32.load i32.const 120712 f32.load call $max_f32 local.set $content_w
    i32.const 120692 f32.load local.get $gap f32.add i32.const 120716 f32.load f32.add local.set $content_h
    local.get $title_len i32.const 0 i32.ne local.get $detail_len i32.const 0 i32.ne i32.and
    if
      f32.const 74 local.set $min_h
    else
      local.get $title_len i32.const 0 i32.ne
      if
        f32.const 50 local.set $min_h
      else
        local.get $detail_len i32.const 0 i32.ne
        if f32.const 48 local.set $min_h else f32.const 32 local.set $min_h end
      end
    end
    local.get $constraints local.get $content_w f32.const 32 f32.add call $er_ui_layout_axis_constraint_limit local.set $pref_w
    local.get $constraints i32.const 8 i32.add local.get $min_h local.get $content_h f32.const 32 f32.add call $max_f32 call $er_ui_layout_axis_constraint_limit local.set $pref_h
    f32.const 160 local.get $pref_w call $min_f32
    f32.const 32 local.get $pref_h call $min_f32
    local.get $pref_w local.get $pref_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit
    local.get $constraints i32.const 8 i32.add local.get $pref_h call $er_ui_layout_axis_constraint_limit
    local.get $out
    call $er_ui_layout_measurement_flexible
    drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)

  (func $er_ui_empty_measure  (param $title_ptr i32) (param $title_len i32) (param $detail_ptr i32) (param $detail_len i32) (param $constraints i32) (param $out i32) (result i32)
    (local $content_w f32) (local $pref_w f32) (local $pref_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    i32.const 120624 f32.const 0 f32.store
    i32.const 120628 f32.const 24 f32.store
    i32.const 120632 f32.const 0 f32.store
    i32.const 120636 f32.const 24 f32.store
    local.get $constraints i32.const 120624 i32.const 120640 call $er_ui_layout_constraints_inner drop
    i32.const 120668 f32.const 20 f32.store
    i32.const 120672 local.get $title_ptr local.get $title_len f32.const 20 call $primitives_average_width f32.store
    i32.const 120676 i32.const 2 i32.store
    local.get $title_ptr local.get $title_len i32.const 120640 i32.const 120668 i32.const 120680 call $er_ui_text_component_measure_value drop
    i32.const 120728 f32.const 16 f32.store
    i32.const 120732 local.get $detail_ptr local.get $detail_len f32.const 16 call $primitives_average_width f32.store
    i32.const 120736 i32.const 2 i32.store
    local.get $detail_ptr local.get $detail_len i32.const 120640 i32.const 120728 i32.const 120704 call $er_ui_text_component_measure_value drop
    f32.const 40 i32.const 120688 f32.load i32.const 120712 f32.load call $max_f32 call $max_f32 local.set $content_w
    local.get $constraints f32.const 144 local.get $content_w f32.const 48 f32.add call $max_f32 call $er_ui_layout_axis_constraint_limit local.set $pref_w
    local.get $constraints i32.const 8 i32.add f32.const 48 f32.const 40 f32.add f32.const 10 f32.add i32.const 120692 f32.load f32.add f32.const 4 f32.add i32.const 120716 f32.load f32.add call $er_ui_layout_axis_constraint_limit local.set $pref_h
    f32.const 144 local.get $pref_w call $min_f32
    f32.const 96 local.get $pref_h call $min_f32
    local.get $pref_w local.get $pref_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit
    local.get $pref_h
    local.get $out
    call $er_ui_layout_measurement_flexible
    drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)
  (func $er_ui_checkbox_box_size  (result f32) f32.const 18)
  (func $er_ui_checkbox_min_width  (result f32) f32.const 96)

  (func $er_ui_checkbox_measure  (param $label_ptr i32) (param $label_len i32) (param $constraints i32) (param $out i32) (result i32)
    (local $pref_w f32) (local $pref_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    i32.const 120760 f32.const 0 f32.store
    i32.const 120764 f32.const 0 f32.store
    i32.const 120768 f32.const 0 f32.store
    i32.const 120772 f32.const 28 f32.store
    local.get $constraints i32.const 120760 i32.const 120776 call $er_ui_layout_constraints_inner drop
    i32.const 120804 f32.const 16 f32.store
    i32.const 120808 local.get $label_ptr local.get $label_len f32.const 16 call $primitives_average_width f32.store
    i32.const 120812 i32.const 2 i32.store
    local.get $label_ptr local.get $label_len i32.const 120776 i32.const 120804 i32.const 120816 call $er_ui_text_component_measure_value drop
    local.get $constraints f32.const 96 f32.const 28 i32.const 120824 f32.load f32.add call $max_f32 call $er_ui_layout_axis_constraint_limit local.set $pref_w
    local.get $constraints i32.const 8 i32.add f32.const 18 i32.const 120828 f32.load call $max_f32 call $er_ui_layout_axis_constraint_limit local.set $pref_h
    f32.const 96 local.get $pref_w call $min_f32
    f32.const 18 local.get $pref_h call $min_f32
    local.get $pref_w local.get $pref_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit
    local.get $pref_h
    local.get $out
    call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)

  (func $er_ui_switch_width  (result f32) f32.const 36)
  (func $er_ui_switch_height  (result f32) f32.const 20)
  (func $er_ui_switch_min_width  (result f32) f32.const 112)

  (func $er_ui_switch_measure  (param $label_ptr i32) (param $label_len i32) (param $constraints i32) (param $out i32) (result i32)
    (local $pref_w f32) (local $pref_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    i32.const 120760 f32.const 0 f32.store
    i32.const 120764 f32.const 46 f32.store
    i32.const 120768 f32.const 0 f32.store
    i32.const 120772 f32.const 0 f32.store
    local.get $constraints i32.const 120760 i32.const 120776 call $er_ui_layout_constraints_inner drop
    i32.const 120804 f32.const 16 f32.store
    i32.const 120808 local.get $label_ptr local.get $label_len f32.const 16 call $primitives_average_width f32.store
    i32.const 120812 i32.const 2 i32.store
    local.get $label_ptr local.get $label_len i32.const 120776 i32.const 120804 i32.const 120816 call $er_ui_text_component_measure_value drop
    local.get $constraints f32.const 112 i32.const 120824 f32.load f32.const 46 f32.add call $max_f32 call $er_ui_layout_axis_constraint_limit local.set $pref_w
    local.get $constraints i32.const 8 i32.add f32.const 20 i32.const 120828 f32.load call $max_f32 call $er_ui_layout_axis_constraint_limit local.set $pref_h
    f32.const 112 local.get $pref_w call $min_f32
    f32.const 20 local.get $pref_h call $min_f32
    local.get $pref_w local.get $pref_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit
    local.get $pref_h
    local.get $out
    call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)

  (func $er_ui_slider_thumb_size  (result f32) f32.const 12)
  (func $er_ui_slider_min_width  (result f32) f32.const 120)
  (func $er_ui_slider_min_height  (result f32) f32.const 32)

  (func $er_ui_slider_measure  (param $label_ptr i32) (param $label_len i32) (param $constraints i32) (param $out i32) (result i32)
    (local $pref_w f32) (local $pref_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    i32.const 120804 f32.const 14 f32.store
    i32.const 120808 local.get $label_ptr local.get $label_len f32.const 14 call $primitives_average_width f32.store
    i32.const 120812 i32.const 2 i32.store
    local.get $label_ptr local.get $label_len local.get $constraints i32.const 120804 i32.const 120816 call $er_ui_text_component_measure_value drop
    local.get $constraints f32.const 120 i32.const 120824 f32.load call $max_f32 call $er_ui_layout_axis_constraint_limit local.set $pref_w
    local.get $constraints i32.const 8 i32.add i32.const 120828 f32.load f32.const 24 f32.add call $er_ui_layout_axis_constraint_limit local.set $pref_h
    f32.const 120 local.get $pref_w call $min_f32
    f32.const 32 local.get $pref_h call $min_f32
    local.get $pref_w local.get $pref_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit
    local.get $pref_h
    local.get $out
    call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)

  (func $er_ui_progress_height  (result f32) f32.const 8)
  (func $er_ui_progress_min_width  (result f32) f32.const 96)

  (func $er_ui_progress_measure  (param $constraints i32) (param $out i32) (result i32)
    (local $pref_w f32) (local $pref_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $constraints f32.const 96 call $er_ui_layout_axis_constraint_limit local.set $pref_w
    local.get $constraints i32.const 8 i32.add f32.const 8 call $er_ui_layout_axis_constraint_limit local.set $pref_h
    f32.const 96 local.get $pref_w call $min_f32
    f32.const 8 local.get $pref_h call $min_f32
    local.get $pref_w local.get $pref_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit
    local.get $pref_h
    local.get $out
    call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)

  (func $er_ui_progress_track_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $bounds f32.load
    local.get $bounds i32.const 4 i32.add f32.load local.get $bounds i32.const 12 i32.add f32.load f32.const 8 f32.sub f32.const 0.5 f32.mul f32.add
    local.get $bounds i32.const 8 i32.add f32.load
    f32.const 8
    call $rect_store)

  (func $er_ui_progress_fill_bounds  (param $track i32) (param $value f32) (param $out i32) (result i32)
    (local $clamped f32)
    local.get $track i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $value f32.const 0 f32.const 1 call $clamp_f32 local.set $clamped
    local.get $out
    local.get $track f32.load
    local.get $track i32.const 4 i32.add f32.load
    local.get $track i32.const 8 i32.add f32.load local.get $clamped f32.mul f32.const 0 call $max_f32 local.get $track i32.const 8 i32.add f32.load call $min_f32
    local.get $track i32.const 12 i32.add f32.load
    call $rect_store)

  (func $er_ui_aspect_ratio_frame_bounds  (param $bounds i32) (param $ratio_w i32) (param $ratio_h i32) (param $out i32) (result i32)
    (local $safe_w f32) (local $safe_h f32) (local $frame_w f32) (local $frame_h f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $ratio_w f32.convert_i32_u f32.const 1 call $max_f32 local.set $safe_w
    local.get $ratio_h f32.convert_i32_u f32.const 1 call $max_f32 local.set $safe_h
    local.get $bounds i32.const 8 i32.add f32.load
    local.get $bounds i32.const 12 i32.add f32.load local.get $safe_w f32.mul local.get $safe_h f32.div
    call $min_f32
    local.set $frame_w
    local.get $bounds i32.const 12 i32.add f32.load
    local.get $frame_w local.get $safe_h f32.mul local.get $safe_w f32.div
    call $min_f32
    local.set $frame_h
    local.get $out
    local.get $bounds f32.load local.get $bounds i32.const 8 i32.add f32.load local.get $frame_w f32.sub f32.const 0.5 f32.mul f32.add
    local.get $bounds i32.const 4 i32.add f32.load local.get $bounds i32.const 12 i32.add f32.load local.get $frame_h f32.sub f32.const 0.5 f32.mul f32.add
    local.get $frame_w
    local.get $frame_h
    call $rect_store)
  (func $er_ui_separator_height  (result f32) f32.const 1)
  (func $er_ui_separator_min_width  (result f32) f32.const 1)
  (func $er_ui_skeleton_min_width  (result f32) f32.const 96)
  (func $er_ui_skeleton_height  (result f32) f32.const 20)
  (func $er_ui_skeleton_alpha  (result i32) i32.const 32)
  (func $er_ui_skeleton_radius  (result f32) f32.const 6)
  (func $er_ui_spinner_size  (result f32) f32.const 28)
  (func $er_ui_spinner_slice_inset  (result f32) f32.const 3)
  (func $er_ui_spinner_start_turn  (result f32) f32.const 0.08)
  (func $er_ui_spinner_end_turn  (result f32) f32.const 0.78)
  (func $er_ui_aspect_ratio_min_width  (result f32) f32.const 160)
  (func $er_ui_kbd_height  (result f32) f32.const 24)
  (func $er_ui_kbd_text_height  (result f32) f32.const 12)
  (func $er_ui_kbd_label_max_lines  (result i32) i32.const 1)
  (func $er_ui_kbd_label_padding  (result f32) f32.const 8)
  (func $er_ui_kbd_min_width  (result f32) f32.const 24)
  (func $er_ui_avatar_size  (result f32) f32.const 40)
  (func $er_ui_avatar_text_height  (result f32) f32.const 14)
  (func $er_ui_avatar_label_inset  (result f32) f32.const 6)
  (func $er_ui_label_height  (result f32) f32.const 16)
  (func $er_ui_label_min_width  (result f32) f32.const 24)
  (func $er_ui_label_max_lines  (result i32) i32.const 2)

  (func $er_ui_measure_intrinsic (param $w f32) (param $h f32) (param $constraints i32) (param $out i32) (result i32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $w local.get $h local.get $constraints i32.const 120856 call $er_ui_primitives_constrain_preferred_size drop
    i32.const 120856 f32.load
    i32.const 120860 f32.load
    i32.const 120856 f32.load
    i32.const 120860 f32.load
    i32.const 120856 f32.load
    i32.const 120860 f32.load
    local.get $out
    call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)

  (func $er_ui_measure_flexible_line (param $min_w f32) (param $height f32) (param $constraints i32) (param $out i32) (result i32)
    (local $pref_w f32) (local $pref_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $min_w local.get $height local.get $constraints i32.const 120856 call $er_ui_primitives_constrain_preferred_size drop
    i32.const 120856 f32.load local.set $pref_w
    i32.const 120860 f32.load local.set $pref_h
    f32.const 1 local.get $pref_w call $min_f32
    local.get $height local.get $pref_h call $min_f32
    local.get $pref_w local.get $pref_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit
    local.get $pref_h
    local.get $out
    call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)

  (func $er_ui_separator_measure  (param $constraints i32) (param $out i32) (result i32)
    f32.const 1 f32.const 1 local.get $constraints local.get $out call $er_ui_measure_flexible_line)

  (func $er_ui_skeleton_measure  (param $constraints i32) (param $out i32) (result i32)
    f32.const 96 f32.const 20 local.get $constraints local.get $out call $er_ui_measure_flexible_line)

  (func $er_ui_spinner_measure  (param $constraints i32) (param $out i32) (result i32)
    f32.const 28 f32.const 28 local.get $constraints local.get $out call $er_ui_measure_intrinsic)

  (func $er_ui_aspect_ratio_intrinsic_size_impl (param $ratio_w i32) (param $ratio_h i32) (param $out i32) (result i32)
    (local $safe_w f32) (local $safe_h f32)
    local.get $out i32.eqz
    if i32.const 0 return end
    local.get $ratio_w f32.convert_i32_u f32.const 1 call $max_f32 local.set $safe_w
    local.get $ratio_h f32.convert_i32_u f32.const 1 call $max_f32 local.set $safe_h
    local.get $out
    f32.const 160
    f32.const 160 local.get $safe_h f32.mul local.get $safe_w f32.div
    call $layout_store_size)

  (func $er_ui_aspect_ratio_intrinsic_size  (param $ratio_w i32) (param $ratio_h i32) (param $out i32) (result i32)
    local.get $ratio_w local.get $ratio_h local.get $out call $er_ui_aspect_ratio_intrinsic_size_impl)

  (func $er_ui_aspect_ratio_measure  (param $ratio_w i32) (param $ratio_h i32) (param $constraints i32) (param $out i32) (result i32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $ratio_w local.get $ratio_h i32.const 120856 call $er_ui_aspect_ratio_intrinsic_size_impl drop
    i32.const 120856 f32.load
    i32.const 120860 f32.load
    local.get $constraints
    local.get $out
    call $er_ui_measure_intrinsic)

  (func $er_ui_kbd_measure  (param $label_ptr i32) (param $label_len i32) (param $constraints i32) (param $out i32) (result i32)
    (local $pref_w f32) (local $pref_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    i32.const 120864 f32.const 8 f32.store
    i32.const 120868 f32.const 0 f32.store
    i32.const 120872 f32.const 8 f32.store
    i32.const 120876 f32.const 0 f32.store
    local.get $constraints i32.const 120864 i32.const 120880 call $er_ui_layout_constraints_inner drop
    i32.const 120908 f32.const 12 f32.store
    i32.const 120912 local.get $label_ptr local.get $label_len f32.const 12 call $primitives_average_width f32.store
    i32.const 120916 i32.const 1 i32.store
    local.get $label_ptr local.get $label_len i32.const 120880 i32.const 120908 i32.const 120920 call $er_ui_text_component_measure_value drop
    i32.const 120928 f32.load f32.const 16 f32.add f32.const 24 call $max_f32
    i32.const 120932 f32.load f32.const 8 f32.add f32.const 24 call $max_f32
    local.get $constraints
    i32.const 120944
    call $er_ui_primitives_constrain_preferred_size drop
    i32.const 120944 f32.load local.set $pref_w
    i32.const 120948 f32.load local.set $pref_h
    f32.const 24 local.get $pref_w call $min_f32
    f32.const 24 local.get $pref_h call $min_f32
    local.get $pref_w local.get $pref_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit
    local.get $pref_h
    local.get $out
    call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)

  (func $er_ui_avatar_measure  (param $constraints i32) (param $out i32) (result i32)
    f32.const 40 f32.const 40 local.get $constraints local.get $out call $er_ui_measure_intrinsic)

  (func $er_ui_label_measure  (param $text_ptr i32) (param $text_len i32) (param $constraints i32) (param $out i32) (result i32)
    (local $pref_w f32) (local $pref_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    i32.const 120908 f32.const 16 f32.store
    i32.const 120912 local.get $text_ptr local.get $text_len f32.const 16 call $primitives_average_width f32.store
    i32.const 120916 i32.const 2 i32.store
    local.get $text_ptr local.get $text_len local.get $constraints i32.const 120908 i32.const 120920 call $er_ui_text_component_measure_value drop
    i32.const 120928 f32.load i32.const 120932 f32.load local.get $constraints i32.const 120944 call $er_ui_primitives_constrain_preferred_size drop
    i32.const 120944 f32.load local.set $pref_w
    i32.const 120948 f32.load local.set $pref_h
    f32.const 24 local.get $pref_w call $min_f32
    f32.const 16 local.get $pref_h call $min_f32
    local.get $pref_w local.get $pref_h
    i32.const 120936 f32.load
    i32.const 120940 f32.load
    local.get $out
    call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)

  (func $er_ui_separator_line_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $bounds f32.load
    local.get $bounds i32.const 4 i32.add f32.load local.get $bounds i32.const 12 i32.add f32.load f32.const 1 f32.sub f32.const 0.5 f32.mul f32.add
    local.get $bounds i32.const 8 i32.add f32.load
    f32.const 1
    call $rect_store)

  (func $er_ui_centered_square_bounds (param $bounds i32) (param $max_size f32) (param $out i32) (result i32)
    (local $size f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $max_size
    f32.const 1 local.get $bounds i32.const 8 i32.add f32.load local.get $bounds i32.const 12 i32.add f32.load call $min_f32 call $max_f32
    call $min_f32
    local.set $size
    local.get $out
    local.get $bounds f32.load local.get $bounds i32.const 8 i32.add f32.load local.get $size f32.sub f32.const 0.5 f32.mul f32.add
    local.get $bounds i32.const 4 i32.add f32.load local.get $bounds i32.const 12 i32.add f32.load local.get $size f32.sub f32.const 0.5 f32.mul f32.add
    local.get $size
    local.get $size
    call $rect_store)

  (func $er_ui_spinner_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds f32.const 28 local.get $out call $er_ui_centered_square_bounds)

  (func $er_ui_avatar_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds f32.const 40 local.get $out call $er_ui_centered_square_bounds)

  (func $er_ui_kbd_label_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $bounds f32.load f32.const 8 f32.add
    local.get $bounds i32.const 4 i32.add f32.load local.get $bounds i32.const 12 i32.add f32.load f32.const 12 f32.sub f32.const 0.5 f32.mul f32.add
    local.get $bounds i32.const 8 i32.add f32.load f32.const 16 f32.sub f32.const 0 call $max_f32
    f32.const 12
    call $rect_store)

  (func $er_ui_kbd_bounds  (param $bounds i32) (param $out i32) (result i32)
    (local $height f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    f32.const 24
    f32.const 1 local.get $bounds i32.const 12 i32.add f32.load call $max_f32
    call $min_f32
    local.set $height
    local.get $out
    local.get $bounds f32.load
    local.get $bounds i32.const 4 i32.add f32.load local.get $bounds i32.const 12 i32.add f32.load local.get $height f32.sub f32.const 0.5 f32.mul f32.add
    local.get $bounds i32.const 8 i32.add f32.load
    local.get $height
    call $rect_store)

  (func $er_ui_avatar_label_bounds  (param $avatar_bounds i32) (param $out i32) (result i32)
    local.get $avatar_bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $avatar_bounds f32.load f32.const 6 f32.add
    local.get $avatar_bounds i32.const 4 i32.add f32.load local.get $avatar_bounds i32.const 12 i32.add f32.load f32.const 14 f32.sub f32.const 0.5 f32.mul f32.add
    local.get $avatar_bounds i32.const 8 i32.add f32.load f32.const 12 f32.sub f32.const 0 call $max_f32
    f32.const 14
    call $rect_store)
  (func $er_ui_tooltip_trigger_y  (result f32) f32.const 8)
  (func $er_ui_tooltip_trigger_w  (result f32) f32.const 80)
  (func $er_ui_tooltip_trigger_h  (result f32) f32.const 28)
  (func $er_ui_tooltip_gap  (result f32) f32.const 10)
  (func $er_ui_tooltip_content_y  (result f32) f32.const 7)
  (func $er_ui_tooltip_content_h  (result f32) f32.const 24)
  (func $er_ui_tooltip_radius  (result f32) f32.const 6)
  (func $er_ui_tooltip_padding  (result f32) f32.const 8)
  (func $er_ui_tooltip_text_h  (result f32) f32.const 12)
  (func $er_ui_tooltip_text_max_lines  (result i32) i32.const 2)
  (func $er_ui_tooltip_trigger_max_lines  (result i32) i32.const 2)
  (func $er_ui_tooltip_min_width  (result f32) f32.const 160)
  (func $er_ui_tooltip_min_height  (result f32) f32.const 44)

  (func $er_ui_tooltip_trigger_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $bounds f32.load
    local.get $bounds i32.const 4 i32.add f32.load f32.const 8 f32.add
    f32.const 80
    f32.const 28
    call $rect_store)

  (func $er_ui_tooltip_measured_text_height (param $text_ptr i32) (param $text_len i32) (param $width f32) (param $line_height f32) (param $max_lines i32) (result f32)
    local.get $text_ptr
    local.get $text_len
    local.get $width
    local.get $text_ptr local.get $text_len local.get $line_height call $primitives_average_width
    local.get $max_lines
    call $er_ui_text_wrapped_line_count
    f32.convert_i32_u
    local.get $line_height
    f32.mul)

  (func $er_ui_tooltip_content_bounds  (param $bounds i32) (param $content_ptr i32) (param $content_len i32) (param $out i32) (result i32)
    (local $x f32) (local $width f32) (local $text_w f32) (local $content_h f32) (local $available_h f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds f32.load f32.const 80 f32.add f32.const 10 f32.add local.set $x
    local.get $bounds f32.load local.get $bounds i32.const 8 i32.add f32.load f32.add local.get $x f32.sub f32.const 1 call $max_f32 local.set $width
    local.get $width f32.const 16 f32.sub f32.const 1 call $max_f32 local.set $text_w
    f32.const 24
    local.get $content_ptr local.get $content_len local.get $text_w f32.const 12 i32.const 2 call $er_ui_tooltip_measured_text_height f32.const 16 f32.add
    call $max_f32
    local.set $content_h
    local.get $bounds i32.const 12 i32.add f32.load f32.const 7 f32.sub f32.const 1 call $max_f32 local.set $available_h
    local.get $out
    local.get $x
    local.get $bounds i32.const 4 i32.add f32.load f32.const 7 f32.add
    local.get $width
    local.get $content_h local.get $available_h call $min_f32
    call $rect_store)

  (func $er_ui_tooltip_content_inner_bounds  (param $tip_bounds i32) (param $out i32) (result i32)
    (local $pad f32)
    local.get $tip_bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    f32.const 8
    local.get $tip_bounds i32.const 8 i32.add f32.load
    local.get $tip_bounds i32.const 12 i32.add f32.load
    call $min_f32
    f32.const 0.5
    f32.mul
    call $min_f32
    local.set $pad
    local.get $out
    local.get $tip_bounds f32.load local.get $pad f32.add
    local.get $tip_bounds i32.const 4 i32.add f32.load local.get $pad f32.add
    local.get $tip_bounds i32.const 8 i32.add f32.load local.get $pad f32.const 2 f32.mul f32.sub f32.const 0 call $max_f32
    local.get $tip_bounds i32.const 12 i32.add f32.load local.get $pad f32.const 2 f32.mul f32.sub f32.const 0 call $max_f32
    call $rect_store)

  (func $er_ui_tooltip_text_bounds  (param $tip_bounds i32) (param $content_ptr i32) (param $content_len i32) (param $out i32) (result i32)
    (local $text_h f32)
    local.get $tip_bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $tip_bounds i32.const 121080 call $er_ui_tooltip_content_inner_bounds drop
    i32.const 121080 i32.const 12 i32.add f32.load
    local.get $content_ptr local.get $content_len i32.const 121080 i32.const 8 i32.add f32.load f32.const 12 i32.const 2 call $er_ui_tooltip_measured_text_height
    call $min_f32
    local.set $text_h
    local.get $out
    i32.const 121080 f32.load
    i32.const 121080 i32.const 4 i32.add f32.load i32.const 121080 i32.const 12 i32.add f32.load local.get $text_h f32.sub f32.const 0.5 f32.mul f32.add
    i32.const 121080 i32.const 8 i32.add f32.load
    local.get $text_h
    call $rect_store)

  (func $er_ui_tooltip_measure  (param $trigger_ptr i32) (param $trigger_len i32) (param $content_ptr i32) (param $content_len i32) (param $constraints i32) (param $out i32) (result i32)
    (local $trigger_w f32) (local $trigger_h f32) (local $content_w f32) (local $content_h f32) (local $control_w f32) (local $raw_w f32) (local $raw_h f32) (local $pref_w f32) (local $pref_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    i32.const 120968 f32.const 16 f32.store
    i32.const 120972 local.get $trigger_ptr local.get $trigger_len f32.const 16 call $primitives_average_width f32.store
    i32.const 120976 i32.const 2 i32.store
    local.get $trigger_ptr local.get $trigger_len local.get $constraints i32.const 120968 i32.const 120980 call $er_ui_text_component_measure_value drop
    i32.const 120988 f32.load local.set $trigger_w
    i32.const 120992 f32.load local.set $trigger_h
    i32.const 121016 f32.const 98 f32.store
    i32.const 121020 f32.const 0 f32.store
    i32.const 121024 f32.const 8 f32.store
    i32.const 121028 f32.const 0 f32.store
    local.get $constraints i32.const 121016 i32.const 121032 call $er_ui_layout_constraints_inner drop
    i32.const 120968 f32.const 12 f32.store
    i32.const 120972 local.get $content_ptr local.get $content_len f32.const 12 call $primitives_average_width f32.store
    i32.const 120976 i32.const 2 i32.store
    local.get $content_ptr local.get $content_len i32.const 121032 i32.const 120968 i32.const 120980 call $er_ui_text_component_measure_value drop
    i32.const 120988 f32.load local.set $content_w
    i32.const 120992 f32.load local.set $content_h
    local.get $trigger_w f32.const 16 f32.add f32.const 80 call $max_f32 local.set $control_w
    f32.const 160
    local.get $control_w f32.const 10 f32.add local.get $content_w f32.add f32.const 16 f32.add
    call $max_f32
    local.set $raw_w
    f32.const 44
    local.get $trigger_h local.get $content_h f32.const 16 f32.add call $max_f32
    call $max_f32
    local.set $raw_h
    local.get $raw_w local.get $raw_h local.get $constraints i32.const 121060 call $er_ui_primitives_constrain_preferred_size drop
    i32.const 121060 f32.load local.set $pref_w
    i32.const 121064 f32.load local.set $pref_h
    f32.const 160 local.get $pref_w call $min_f32
    f32.const 44 local.get $pref_h call $min_f32
    local.get $pref_w local.get $pref_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit
    local.get $pref_h
    local.get $out
    call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)
  (func $er_ui_button_group_item_count  (result i32) i32.const 2)
  (func $er_ui_button_group_text_padding  (result f32) f32.const 8)

  (func $er_ui_button_group_active_index  (param $active i32) (result i32)
    local.get $active i32.const 2 call $er_ui_list_clamped_index)

  (func $er_ui_button_group_indexed_id  (param $id i32) (param $active i32) (result i32)
    local.get $id i32.const 2 i32.mul
    local.get $active i32.const 2 call $er_ui_list_clamped_index
    i32.add)

  (func $er_ui_button_group_hit_id  (param $id i32) (param $index i32) (result i32)
    local.get $id local.get $index i32.const 1 call $min_i32_u i32.add)

  (func $er_ui_button_group_segment_bounds  (param $bounds i32) (param $index i32) (param $out i32) (result i32)
    local.get $bounds local.get $index i32.const 2 f32.const 0 local.get $out call $er_ui_list_equal_segment_bounds_gap)

  (func $er_ui_button_group_segment_text_bounds  (param $bounds i32) (param $index i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds local.get $index i32.const 2 f32.const 0 i32.const 121824 call $er_ui_list_equal_segment_bounds_gap drop
    i32.const 121824 local.get $out call $er_ui_toggle_text_bounds)

  (func $er_ui_button_group_measure  (param $first_ptr i32) (param $first_len i32) (param $second_ptr i32) (param $second_len i32) (param $constraints i32) (param $out i32) (result i32)
    local.get $first_ptr local.get $first_len local.get $second_ptr local.get $second_len local.get $constraints local.get $out call $er_ui_tabs_measure_two_segments)
  (func $er_ui_input_otp_slot_count  (result i32) i32.const 6)
  (func $er_ui_input_otp_slot_size  (result f32) f32.const 36)
  (func $er_ui_input_otp_slot_gap  (result f32) f32.const 0)
  (func $er_ui_input_otp_text_padding  (result f32) f32.const 8)

  (func $er_ui_input_otp_intrinsic_size  (param $out i32) (result i32)
    local.get $out i32.eqz
    if i32.const 0 return end
    local.get $out f32.const 216 f32.const 36 call $layout_store_size)

  (func $er_ui_input_otp_hit_id  (param $id i32) (param $index i32) (result i32)
    local.get $id local.get $index i32.const 5 call $min_i32_u i32.add)

  (func $er_ui_input_otp_slot_bounds  (param $bounds i32) (param $index i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $bounds f32.load local.get $index f32.convert_i32_u f32.const 36 f32.mul f32.add
    local.get $bounds i32.const 4 i32.add f32.load
    f32.const 36
    local.get $bounds i32.const 12 i32.add f32.load f32.const 36 call $min_f32
    call $rect_store)

  (func $er_ui_input_otp_slot_text_bounds  (param $slot_bounds i32) (param $out i32) (result i32)
    (local $pad f32)
    local.get $slot_bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    f32.const 8
    local.get $slot_bounds i32.const 8 i32.add f32.load
    local.get $slot_bounds i32.const 12 i32.add f32.load
    call $min_f32
    f32.const 0.5
    f32.mul
    call $min_f32
    local.set $pad
    local.get $out
    local.get $slot_bounds f32.load local.get $pad f32.add
    local.get $slot_bounds i32.const 4 i32.add f32.load local.get $pad f32.add local.get $slot_bounds i32.const 12 i32.add f32.load local.get $pad f32.const 2 f32.mul f32.sub f32.const 16 f32.sub f32.const 0.5 f32.mul f32.add
    local.get $slot_bounds i32.const 8 i32.add f32.load local.get $pad f32.const 2 f32.mul f32.sub f32.const 0 call $max_f32
    f32.const 16
    call $rect_store)

  (func $er_ui_input_otp_measure  (param $constraints i32) (param $out i32) (result i32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    f32.const 216 f32.const 36 local.get $constraints i32.const 121980 call $er_ui_primitives_constrain_preferred_size drop
    i32.const 121980 f32.load
    i32.const 121984 f32.load
    i32.const 121980 f32.load
    i32.const 121984 f32.load
    i32.const 121980 f32.load
    i32.const 121984 f32.load
    local.get $out
    call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)
  (func $er_ui_switch_knob_size  (result f32) f32.const 14)
  (func $er_ui_switch_knob_inset  (result f32) f32.const 3)
  (func $er_ui_switch_label_gap  (result f32) f32.const 10)
  (func $er_ui_switch_label_height  (result f32) f32.const 16)
  (func $er_ui_switch_label_max_lines  (result i32) i32.const 2)
  (func $er_ui_switch_shadow_inset  (result f32) f32.const 1)
  (func $er_ui_switch_shadow_size  (result f32) f32.const 2)

  (func $er_ui_switch_pill_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $bounds f32.load local.get $bounds i32.const 8 i32.add f32.load f32.add f32.const 36 f32.sub
    local.get $bounds i32.const 4 i32.add f32.load local.get $bounds i32.const 12 i32.add f32.load f32.const 20 f32.sub f32.const 0.5 f32.mul f32.add
    f32.const 36
    f32.const 20
    call $rect_store)

  (func $er_ui_switch_pill_shadow_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 122344 call $er_ui_switch_pill_bounds drop
    local.get $out
    i32.const 122344 f32.load f32.const 1 f32.sub
    i32.const 122344 i32.const 4 i32.add f32.load f32.const 1 f32.sub
    i32.const 122344 i32.const 8 i32.add f32.load f32.const 2 f32.add
    i32.const 122344 i32.const 12 i32.add f32.load f32.const 2 f32.add
    call $rect_store)

  (func $er_ui_switch_knob_bounds  (param $bounds i32) (param $checked i32) (param $out i32) (result i32)
    (local $x f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 122360 call $er_ui_switch_pill_bounds drop
    local.get $checked
    if
      i32.const 122360 f32.load i32.const 122360 i32.const 8 i32.add f32.load f32.add f32.const 17 f32.sub local.set $x
    else
      i32.const 122360 f32.load f32.const 3 f32.add local.set $x
    end
    local.get $out
    local.get $x
    i32.const 122360 i32.const 4 i32.add f32.load f32.const 3 f32.add
    f32.const 14
    f32.const 14
    call $rect_store)

  (func $er_ui_switch_knob_shadow_bounds  (param $bounds i32) (param $checked i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds local.get $checked i32.const 122376 call $er_ui_switch_knob_bounds drop
    local.get $out
    i32.const 122376 f32.load f32.const 1 f32.sub
    i32.const 122376 i32.const 4 i32.add f32.load f32.const 1 f32.sub
    i32.const 122376 i32.const 8 i32.add f32.load f32.const 2 f32.add
    i32.const 122376 i32.const 12 i32.add f32.load f32.const 2 f32.add
    call $rect_store)

  (func $er_ui_switch_checked_marker_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 1 i32.const 122392 call $er_ui_switch_knob_bounds drop
    local.get $out
    i32.const 122392 f32.load f32.const 5 f32.add
    i32.const 122392 i32.const 4 i32.add f32.load f32.const 5 f32.add
    i32.const 122392 i32.const 8 i32.add f32.load f32.const 10 f32.sub f32.const 0 call $max_f32
    i32.const 122392 i32.const 12 i32.add f32.load f32.const 10 f32.sub f32.const 0 call $max_f32
    call $rect_store)

  (func $er_ui_switch_label_slot_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 122408 call $er_ui_switch_pill_bounds drop
    local.get $out
    local.get $bounds f32.load
    local.get $bounds i32.const 4 i32.add f32.load
    i32.const 122408 f32.load local.get $bounds f32.load f32.sub f32.const 10 f32.sub f32.const 1 call $max_f32
    local.get $bounds i32.const 12 i32.add f32.load
    call $rect_store)

  (func $er_ui_switch_label_text_bounds  (param $bounds i32) (param $label_ptr i32) (param $label_len i32) (param $out i32) (result i32)
    (local $label_h f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 122424 call $er_ui_switch_label_slot_bounds drop
    local.get $label_ptr local.get $label_len i32.const 122424 i32.const 8 i32.add f32.load f32.const 16 i32.const 2 call $er_ui_alert_measured_text_height
    local.get $bounds i32.const 12 i32.add f32.load
    call $min_f32
    local.set $label_h
    local.get $out
    i32.const 122424 f32.load
    i32.const 122424 i32.const 4 i32.add f32.load i32.const 122424 i32.const 12 i32.add f32.load local.get $label_h f32.sub f32.const 0.5 f32.mul f32.add
    i32.const 122424 i32.const 8 i32.add f32.load
    local.get $label_h
    call $rect_store)
  (func $er_ui_input_group_separator_height  (result f32) f32.const 1)
  (func $er_ui_input_group_addon_min_w  (result f32) f32.const 42)
  (func $er_ui_input_group_addon_max_w  (result f32) f32.const 96)
  (func $er_ui_input_group_addon_padding  (result f32) f32.const 10)
  (func $er_ui_input_group_control_gap  (result f32) f32.const 8)
  (func $er_ui_input_group_separator_inset  (result f32) f32.const 8)
  (func $er_ui_input_group_text_max_lines  (result i32) i32.const 2)
  (func $er_ui_input_group_min_width  (result f32) f32.const 140)
  (func $er_ui_input_group_min_height  (result f32) f32.const 36)

  (func $er_ui_input_group_addon_width  (param $addon_ptr i32) (param $addon_len i32) (result f32)
    local.get $addon_ptr local.get $addon_len f32.const 16 call $er_ui_font_text_width
    f32.const 20
    f32.add
    f32.const 42
    call $max_f32
    f32.const 96
    call $min_f32)

  (func $er_ui_input_group_addon_bounds  (param $bounds i32) (param $addon_ptr i32) (param $addon_len i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $bounds f32.load
    local.get $bounds i32.const 4 i32.add f32.load
    local.get $addon_ptr local.get $addon_len call $er_ui_input_group_addon_width
    local.get $bounds i32.const 12 i32.add f32.load
    call $rect_store)

  (func $er_ui_input_group_control_bounds  (param $bounds i32) (param $addon_ptr i32) (param $addon_len i32) (param $out i32) (result i32)
    (local $addon_w f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $addon_ptr local.get $addon_len call $er_ui_input_group_addon_width local.set $addon_w
    local.get $out
    local.get $bounds f32.load local.get $addon_w f32.add f32.const 8 f32.add
    local.get $bounds i32.const 4 i32.add f32.load
    local.get $bounds i32.const 8 i32.add f32.load local.get $addon_w f32.sub f32.const 8 f32.sub f32.const 1 call $max_f32
    local.get $bounds i32.const 12 i32.add f32.load
    call $rect_store)

  (func $er_ui_input_group_separator_bounds  (param $bounds i32) (param $addon_ptr i32) (param $addon_len i32) (param $out i32) (result i32)
    (local $addon_w f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $addon_ptr local.get $addon_len call $er_ui_input_group_addon_width local.set $addon_w
    local.get $out
    local.get $bounds f32.load local.get $addon_w f32.add
    local.get $bounds i32.const 4 i32.add f32.load f32.const 8 f32.add
    f32.const 1
    local.get $bounds i32.const 12 i32.add f32.load f32.const 16 f32.sub f32.const 1 call $max_f32
    call $rect_store)

  (func $er_ui_input_group_addon_text_bounds  (param $bounds i32) (param $addon_ptr i32) (param $addon_len i32) (param $out i32) (result i32)
    (local $text_h f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds local.get $addon_ptr local.get $addon_len i32.const 122600 call $er_ui_input_group_addon_bounds drop
    i32.const 122600 f32.const 10 i32.const 122616 call $er_ui_primitives_content_inset drop
    local.get $addon_ptr local.get $addon_len i32.const 122616 i32.const 8 i32.add f32.load f32.const 16 i32.const 2 call $er_ui_alert_measured_text_height
    i32.const 122616 i32.const 12 i32.add f32.load
    call $min_f32
    local.set $text_h
    i32.const 122616 local.get $out local.get $text_h call $er_ui_rect_with_height_centered)

  (func $er_ui_input_group_placeholder_text_bounds  (param $bounds i32) (param $addon_ptr i32) (param $addon_len i32) (param $placeholder_ptr i32) (param $placeholder_len i32) (param $out i32) (result i32)
    (local $text_h f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds local.get $addon_ptr local.get $addon_len i32.const 122632 call $er_ui_input_group_control_bounds drop
    i32.const 122632 f32.const 12 i32.const 122648 call $er_ui_primitives_content_inset drop
    local.get $placeholder_ptr local.get $placeholder_len i32.const 122648 i32.const 8 i32.add f32.load f32.const 16 i32.const 2 call $er_ui_alert_measured_text_height
    i32.const 122648 i32.const 12 i32.add f32.load
    call $min_f32
    local.set $text_h
    i32.const 122648 local.get $out local.get $text_h call $er_ui_rect_with_height_centered)

  (func $er_ui_input_group_measure  (param $addon_ptr i32) (param $addon_len i32) (param $placeholder_ptr i32) (param $placeholder_len i32) (param $constraints i32) (param $out i32) (result i32)
    (local $addon_w f32) (local $addon_h f32) (local $placeholder_w f32) (local $placeholder_h f32) (local $pref_w f32) (local $pref_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $addon_ptr local.get $addon_len call $er_ui_input_group_addon_width local.set $addon_w
    local.get $addon_ptr local.get $addon_len local.get $addon_w f32.const 20 f32.sub f32.const 1 call $max_f32 f32.const 16 i32.const 2 call $er_ui_alert_measured_text_height local.set $addon_h
    i32.const 122664 f32.const 0 f32.store
    i32.const 122668 local.get $addon_w f32.const 20 f32.add f32.store
    i32.const 122672 f32.const 12 f32.store
    i32.const 122676 f32.const 0 f32.store
    local.get $constraints i32.const 122664 i32.const 122680 call $er_ui_layout_constraints_inner drop
    i32.const 122704 f32.const 16 f32.store
    i32.const 122708 local.get $placeholder_ptr local.get $placeholder_len f32.const 16 call $primitives_average_width f32.store
    i32.const 122712 i32.const 2 i32.store
    local.get $placeholder_ptr local.get $placeholder_len i32.const 122680 i32.const 122704 i32.const 122716 call $er_ui_text_component_measure_value drop
    i32.const 122724 f32.load local.set $placeholder_w
    i32.const 122728 f32.load local.set $placeholder_h
    f32.const 140 local.get $addon_w f32.const 8 f32.add f32.const 24 f32.add local.get $placeholder_w f32.add call $max_f32 local.set $pref_w
    f32.const 36 local.get $addon_h local.get $placeholder_h call $max_f32 f32.const 24 f32.add call $max_f32 local.set $pref_h
    local.get $pref_w local.get $pref_h local.get $constraints i32.const 122740 call $er_ui_primitives_constrain_preferred_size drop
    i32.const 122740 f32.load local.set $pref_w
    i32.const 122744 f32.load local.set $pref_h
    f32.const 140 local.get $pref_w call $min_f32
    f32.const 36 local.get $pref_h call $min_f32
    local.get $pref_w local.get $pref_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit
    local.get $pref_h
    local.get $out
    call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)
