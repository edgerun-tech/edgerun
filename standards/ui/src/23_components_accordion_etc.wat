  (func $er_ui_accordion_id_stride  (result i32) i32.const 2)
  (func $er_ui_accordion_trigger_h  (result f32) f32.const 36)
  (func $er_ui_accordion_trigger_text_y  (result f32) f32.const 10)
  (func $er_ui_accordion_icon_space  (result f32) f32.const 22)
  (func $er_ui_accordion_icon_size  (result f32) f32.const 14)
  (func $er_ui_accordion_icon_y  (result f32) f32.const 11)
  (func $er_ui_accordion_content_padding_top  (result f32) f32.const 8)
  (func $er_ui_accordion_detail_height  (result f32) f32.const 16)
  (func $er_ui_accordion_detail_average_w  (result f32) f32.const 7.5)
  (func $er_ui_accordion_detail_max_lines  (result i32) i32.const 2)
  (func $er_ui_accordion_title_max_lines  (result i32) i32.const 1)
  (func $er_ui_accordion_separator_height  (result f32) f32.const 1)
  (func $er_ui_accordion_closed_height  (result f32) f32.const 37)

  (func $er_ui_accordion_open_height  (param $detail_preferred_h f32) (result f32)
    f32.const 45 local.get $detail_preferred_h f32.add)

  (func $er_ui_accordion_encoded_id  (param $id i32) (param $open i32) (result i32)
    local.get $id i32.const 2 i32.mul
    local.get $open i32.const 0 i32.ne
    i32.add)

  (func $er_ui_accordion_trigger_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $bounds f32.load
    local.get $bounds i32.const 4 i32.add f32.load
    local.get $bounds i32.const 8 i32.add f32.load
    f32.const 36
    call $rect_store)

  (func $er_ui_accordion_title_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 121748 call $er_ui_accordion_trigger_bounds drop
    local.get $out
    i32.const 121748 f32.load
    i32.const 121748 i32.const 4 i32.add f32.load f32.const 10 f32.add
    i32.const 121748 i32.const 8 i32.add f32.load f32.const 22 f32.sub f32.const 1 call $max_f32
    f32.const 16
    call $rect_store)

  (func $er_ui_accordion_icon_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 121748 call $er_ui_accordion_trigger_bounds drop
    local.get $out
    i32.const 121748 f32.load i32.const 121748 i32.const 8 i32.add f32.load f32.add f32.const 14 f32.sub
    i32.const 121748 i32.const 4 i32.add f32.load f32.const 11 f32.add
    f32.const 14
    f32.const 14
    call $rect_store)

  (func $er_ui_accordion_separator_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 121748 call $er_ui_accordion_trigger_bounds drop
    local.get $out
    local.get $bounds f32.load
    i32.const 121748 i32.const 4 i32.add f32.load i32.const 121748 i32.const 12 i32.add f32.load f32.add
    local.get $bounds i32.const 8 i32.add f32.load
    f32.const 1
    call $rect_store)

  (func $er_ui_accordion_detail_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 121748 call $er_ui_accordion_trigger_bounds drop
    local.get $out
    local.get $bounds f32.load
    i32.const 121748 i32.const 4 i32.add f32.load i32.const 121748 i32.const 12 i32.add f32.load f32.add f32.const 8 f32.add
    local.get $bounds i32.const 8 i32.add f32.load
    local.get $bounds i32.const 12 i32.add f32.load f32.const 44 f32.sub f32.const 1 call $max_f32
    call $rect_store)

  (func $er_ui_accordion_measure_detail (param $detail_ptr i32) (param $detail_len i32) (param $constraints i32) (param $out i32) (result i32)
    i32.const 121764 f32.const 16 f32.store
    i32.const 121768 f32.const 7.5 f32.store
    i32.const 121772 i32.const 2 i32.store
    local.get $detail_ptr local.get $detail_len local.get $constraints i32.const 121764 local.get $out call $er_ui_text_component_measure_value)

  (func $er_ui_accordion_measure  (param $title_ptr i32) (param $title_len i32) (param $detail_ptr i32) (param $detail_len i32) (param $open i32) (param $constraints i32) (param $out i32) (result i32)
    (local $title_w f32) (local $detail_h f32) (local $closed_h f32) (local $open_h f32) (local $pref_w f32) (local $pref_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $title_ptr local.get $title_len f32.const 16 call $er_ui_font_text_width local.set $title_w
    local.get $detail_ptr local.get $detail_len local.get $constraints i32.const 121776 call $er_ui_accordion_measure_detail drop
    i32.const 121788 f32.load local.set $detail_h
    f32.const 37 local.set $closed_h
    local.get $closed_h f32.const 8 f32.add local.get $detail_h f32.add local.set $open_h
    local.get $title_w f32.const 22 f32.add
    local.get $open
    if (result f32)
      local.get $open_h
    else
      local.get $closed_h
    end
    local.get $constraints
    i32.const 121800
    call $er_ui_primitives_constrain_preferred_size drop
    i32.const 121800 f32.load local.set $pref_w
    i32.const 121804 f32.load local.set $pref_h
    f32.const 23
    local.get $closed_h
    local.get $pref_w local.get $pref_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit
    local.get $pref_h local.get $open_h call $max_f32
    local.get $out
    call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)
  (func $er_ui_field_label_h  (result f32) f32.const 14)
  (func $er_ui_field_label_max_lines  (result i32) i32.const 2)
  (func $er_ui_field_gap  (result f32) f32.const 6)
  (func $er_ui_field_input_h  (result f32) f32.const 36)
  (func $er_ui_field_placeholder_max_lines  (result i32) i32.const 2)
  (func $er_ui_field_validation_gap  (result f32) f32.const 6)
  (func $er_ui_field_validation_line_h  (result f32) f32.const 12)
  (func $er_ui_field_validation_max_lines  (result i32) i32.const 2)
  (func $er_ui_field_min_width  (result f32) f32.const 120)
  (func $er_ui_field_min_height  (result f32) f32.const 48)

  (func $er_ui_field_measured_text_height (param $text_ptr i32) (param $text_len i32) (param $width f32) (param $line_height f32) (param $max_lines i32) (result f32)
    local.get $text_ptr local.get $text_len local.get $width local.get $line_height local.get $max_lines call $er_ui_alert_measured_text_height)

  (func $er_ui_field_label_height  (param $bounds i32) (param $label_ptr i32) (param $label_len i32) (result f32)
    local.get $bounds i32.eqz
    if f32.const 0 return end
    local.get $bounds i32.const 12 i32.add f32.load
    local.get $label_ptr local.get $label_len local.get $bounds i32.const 8 i32.add f32.load f32.const 14 i32.const 2 call $er_ui_field_measured_text_height
    call $min_f32)

  (func $er_ui_field_label_bounds  (param $bounds i32) (param $label_ptr i32) (param $label_len i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $bounds f32.load
    local.get $bounds i32.const 4 i32.add f32.load
    local.get $bounds i32.const 8 i32.add f32.load
    local.get $bounds local.get $label_ptr local.get $label_len call $er_ui_field_label_height
    call $rect_store)

  (func $er_ui_field_input_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $bounds f32.load
    local.get $bounds i32.const 4 i32.add f32.load f32.const 20 f32.add
    local.get $bounds i32.const 8 i32.add f32.load
    local.get $bounds i32.const 12 i32.add f32.load f32.const 20 f32.sub f32.const 1 call $max_f32
    call $rect_store)

  (func $er_ui_field_input_bounds_for_label  (param $bounds i32) (param $label_ptr i32) (param $label_len i32) (param $out i32) (result i32)
    (local $label_h f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds local.get $label_ptr local.get $label_len call $er_ui_field_label_height local.set $label_h
    local.get $out
    local.get $bounds f32.load
    local.get $bounds i32.const 4 i32.add f32.load local.get $label_h f32.add f32.const 6 f32.add
    local.get $bounds i32.const 8 i32.add f32.load
    local.get $bounds i32.const 12 i32.add f32.load local.get $label_h f32.sub f32.const 6 f32.sub f32.const 1 call $max_f32
    call $rect_store)

  (func $er_ui_field_input_bounds_with_validation  (param $bounds i32) (param $label_ptr i32) (param $label_len i32) (param $out i32) (result i32)
    (local $label_h f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds local.get $label_ptr local.get $label_len call $er_ui_field_label_height local.set $label_h
    local.get $out
    local.get $bounds f32.load
    local.get $bounds i32.const 4 i32.add f32.load local.get $label_h f32.add f32.const 6 f32.add
    local.get $bounds i32.const 8 i32.add f32.load
    f32.const 36 local.get $bounds i32.const 12 i32.add f32.load local.get $label_h f32.sub f32.const 6 f32.sub call $min_f32 f32.const 1 call $max_f32
    call $rect_store)

  (func $er_ui_field_validation_bounds  (param $bounds i32) (param $label_ptr i32) (param $label_len i32) (param $message_ptr i32) (param $message_len i32) (param $out i32) (result i32)
    (local $y f32) (local $measured_h f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds local.get $label_ptr local.get $label_len i32.const 121840 call $er_ui_field_input_bounds_with_validation drop
    i32.const 121840 i32.const 4 i32.add f32.load i32.const 121840 i32.const 12 i32.add f32.load f32.add f32.const 6 f32.add local.set $y
    local.get $message_ptr local.get $message_len local.get $bounds i32.const 8 i32.add f32.load f32.const 12 i32.const 2 call $er_ui_field_measured_text_height local.set $measured_h
    local.get $out
    local.get $bounds f32.load
    local.get $y
    local.get $bounds i32.const 8 i32.add f32.load
    local.get $measured_h local.get $bounds i32.const 4 i32.add f32.load local.get $bounds i32.const 12 i32.add f32.load f32.add local.get $y f32.sub f32.const 1 call $max_f32 call $min_f32
    call $rect_store)

  (func $er_ui_field_input_text_bounds  (param $input_bounds i32) (param $placeholder_ptr i32) (param $placeholder_len i32) (param $out i32) (result i32)
    (local $text_h f32)
    local.get $input_bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    i32.const 121840
    local.get $input_bounds f32.load f32.const 12 f32.add
    local.get $input_bounds i32.const 4 i32.add f32.load f32.const 12 f32.add
    local.get $input_bounds i32.const 8 i32.add f32.load f32.const 24 f32.sub f32.const 0 call $max_f32
    local.get $input_bounds i32.const 12 i32.add f32.load f32.const 24 f32.sub f32.const 0 call $max_f32
    call $rect_store drop
    i32.const 121840 i32.const 12 i32.add f32.load
    local.get $placeholder_ptr local.get $placeholder_len i32.const 121840 i32.const 8 i32.add f32.load f32.const 16 i32.const 2 call $er_ui_field_measured_text_height
    call $min_f32
    local.set $text_h
    local.get $out
    i32.const 121840 f32.load
    i32.const 121840 i32.const 4 i32.add f32.load i32.const 121840 i32.const 12 i32.add f32.load local.get $text_h f32.sub f32.const 0.5 f32.mul f32.add
    i32.const 121840 i32.const 8 i32.add f32.load
    local.get $text_h
    call $rect_store)

  (func $er_ui_field_input_measure  (param $placeholder_ptr i32) (param $placeholder_len i32) (param $constraints i32) (param $out i32) (result i32)
    (local $pref_w f32) (local $pref_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    i32.const 121864 f32.const 12 f32.store
    i32.const 121868 f32.const 0 f32.store
    i32.const 121872 f32.const 12 f32.store
    i32.const 121876 f32.const 0 f32.store
    local.get $constraints i32.const 121864 i32.const 121880 call $er_ui_layout_constraints_inner drop
    i32.const 121904 f32.const 16 f32.store
    i32.const 121908 local.get $placeholder_ptr local.get $placeholder_len f32.const 16 call $primitives_average_width f32.store
    i32.const 121912 i32.const 2 i32.store
    local.get $placeholder_ptr local.get $placeholder_len i32.const 121880 i32.const 121904 i32.const 121916 call $er_ui_text_component_measure_value drop
    i32.const 121924 f32.load f32.const 24 f32.add
    i32.const 121928 f32.load f32.const 36 call $max_f32
    local.get $constraints
    i32.const 121940
    call $er_ui_primitives_constrain_preferred_size drop
    i32.const 121940 f32.load local.set $pref_w
    i32.const 121944 f32.load local.set $pref_h
    f32.const 120 local.get $pref_w call $min_f32
    f32.const 36 local.get $pref_h call $min_f32
    local.get $pref_w local.get $pref_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit
    local.get $pref_h f32.const 36 call $max_f32
    local.get $out
    call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)

  (func $er_ui_field_measure  (param $label_ptr i32) (param $label_len i32) (param $placeholder_ptr i32) (param $placeholder_len i32) (param $validation_ptr i32) (param $validation_len i32) (param $has_validation i32) (param $constraints i32) (param $out i32) (result i32)
    (local $label_w f32) (local $label_h f32) (local $input_w f32) (local $input_h f32) (local $validation_w f32) (local $validation_h f32) (local $raw_w f32) (local $raw_h f32) (local $pref_w f32) (local $pref_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    i32.const 121904 f32.const 14 f32.store
    i32.const 121908 local.get $label_ptr local.get $label_len f32.const 14 call $primitives_average_width f32.store
    i32.const 121912 i32.const 2 i32.store
    local.get $label_ptr local.get $label_len local.get $constraints i32.const 121904 i32.const 121916 call $er_ui_text_component_measure_value drop
    i32.const 121924 f32.load local.set $label_w
    i32.const 121928 f32.load local.set $label_h
    local.get $placeholder_ptr local.get $placeholder_len local.get $constraints i32.const 121964 call $er_ui_field_input_measure drop
    i32.const 121972 f32.load local.set $input_w
    i32.const 121976 f32.load local.set $input_h
    local.get $has_validation
    if
      i32.const 121904 f32.const 12 f32.store
      i32.const 121908 local.get $validation_ptr local.get $validation_len f32.const 12 call $primitives_average_width f32.store
      i32.const 121912 i32.const 2 i32.store
      local.get $validation_ptr local.get $validation_len local.get $constraints i32.const 121904 i32.const 121916 call $er_ui_text_component_measure_value drop
      i32.const 121924 f32.load local.set $validation_w
      i32.const 121928 f32.load local.set $validation_h
    end
    f32.const 120 local.get $label_w local.get $input_w call $max_f32 local.get $validation_w call $max_f32 call $max_f32 local.set $raw_w
    local.get $label_h f32.const 6 f32.add local.get $input_h f32.add local.set $raw_h
    local.get $has_validation
    if
      local.get $raw_h f32.const 6 f32.add local.get $validation_h f32.add local.set $raw_h
    end
    local.get $raw_w local.get $raw_h local.get $constraints i32.const 121940 call $er_ui_primitives_constrain_preferred_size drop
    i32.const 121940 f32.load local.set $pref_w
    i32.const 121944 f32.load local.set $pref_h
    f32.const 120 local.get $pref_w call $min_f32
    f32.const 48 local.get $pref_h call $min_f32
    local.get $pref_w local.get $pref_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit
    local.get $pref_h
    local.get $out
    call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)
  (func $er_ui_pagination_page_count  (result i32) i32.const 3)
  (func $er_ui_pagination_item_count  (result i32) i32.const 5)
  (func $er_ui_pagination_gap  (result f32) f32.const 4)
  (func $er_ui_pagination_text_padding  (result f32) f32.const 2)
  (func $er_ui_pagination_label_len  (param $index i32) (result i32)
    i32.const 1)

  (func $er_ui_pagination_clamped_page  (param $page i32) (result i32)
    local.get $page i32.const 3 call $er_ui_list_clamped_index)

  (func $er_ui_pagination_indexed_id  (param $id i32) (param $page i32) (result i32)
    local.get $id i32.const 3 i32.mul
    local.get $page i32.const 3 call $er_ui_list_clamped_index
    i32.add)

  (func $er_ui_pagination_hit_id  (param $id i32) (param $index i32) (result i32)
    local.get $id local.get $index i32.const 4 call $min_i32_u i32.add)

  (func $er_ui_pagination_active_item_index  (param $page i32) (result i32)
    local.get $page i32.const 3 call $er_ui_list_clamped_index i32.const 1 i32.add)

  (func $er_ui_pagination_item_bounds  (param $bounds i32) (param $index i32) (param $out i32) (result i32)
    local.get $bounds local.get $index i32.const 5 f32.const 4 local.get $out call $er_ui_list_equal_segment_bounds_gap)

  (func $er_ui_pagination_item_text_bounds  (param $bounds i32) (param $index i32) (param $out i32) (result i32)
    (local $inner_h f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds local.get $index i32.const 5 f32.const 4 i32.const 122000 call $er_ui_list_equal_segment_bounds_gap drop
    i32.const 122000 i32.const 12 i32.add f32.load f32.const 4 f32.sub f32.const 0 call $max_f32 local.set $inner_h
    local.get $out
    i32.const 122000 f32.load f32.const 2 f32.add
    i32.const 122000 i32.const 4 i32.add f32.load f32.const 2 f32.add local.get $inner_h f32.const 16 f32.sub f32.const 0.5 f32.mul f32.add
    i32.const 122000 i32.const 8 i32.add f32.load f32.const 4 f32.sub f32.const 0 call $max_f32
    f32.const 16
    call $rect_store)

  (func $er_ui_pagination_measure  (param $constraints i32) (param $out i32) (result i32)
    (local $item_w f32) (local $pref_w f32) (local $pref_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    i32.const 122020 i32.const 60 i32.store8
    i32.const 122021 i32.const 49 i32.store8
    i32.const 122022 i32.const 50 i32.store8
    i32.const 122023 i32.const 51 i32.store8
    i32.const 122024 i32.const 62 i32.store8
    i32.const 122020 i32.const 1 f32.const 16 call $er_ui_font_text_width
    i32.const 122021 i32.const 1 f32.const 16 call $er_ui_font_text_width call $max_f32
    i32.const 122022 i32.const 1 f32.const 16 call $er_ui_font_text_width call $max_f32
    i32.const 122023 i32.const 1 f32.const 16 call $er_ui_font_text_width call $max_f32
    i32.const 122024 i32.const 1 f32.const 16 call $er_ui_font_text_width call $max_f32
    f32.const 4 f32.add
    local.set $item_w
    local.get $item_w f32.const 5 f32.mul f32.const 16 f32.add
    f32.const 20
    local.get $constraints
    i32.const 122032
    call $er_ui_primitives_constrain_preferred_size drop
    i32.const 122032 f32.load local.set $pref_w
    i32.const 122036 f32.load local.set $pref_h
    f32.const 21
    f32.const 20
    local.get $pref_w local.get $pref_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit
    local.get $pref_h
    local.get $out
    call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)
  (func $er_ui_select_arrow_w  (result f32) f32.const 18)
  (func $er_ui_select_icon_size  (result f32) f32.const 14)
  (func $er_ui_select_label_max_lines  (result i32) i32.const 2)
  (func $er_ui_select_min_width  (result f32) f32.const 112)
  (func $er_ui_select_min_height  (result f32) f32.const 40)

  (func $er_ui_select_content_bounds  (param $bounds i32) (param $out i32) (result i32)
    (local $pad f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    f32.const 12
    local.get $bounds i32.const 8 i32.add f32.load
    local.get $bounds i32.const 12 i32.add f32.load
    call $min_f32
    f32.const 0.5
    f32.mul
    call $min_f32
    local.set $pad
    local.get $out
    local.get $bounds f32.load local.get $pad f32.add
    local.get $bounds i32.const 4 i32.add f32.load local.get $pad f32.add
    local.get $bounds i32.const 8 i32.add f32.load local.get $pad f32.const 2 f32.mul f32.sub f32.const 0 call $max_f32
    local.get $bounds i32.const 12 i32.add f32.load local.get $pad f32.const 2 f32.mul f32.sub f32.const 0 call $max_f32
    call $rect_store)

  (func $er_ui_select_label_slot_bounds  (param $content_bounds i32) (param $out i32) (result i32)
    local.get $content_bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $content_bounds f32.load
    local.get $content_bounds i32.const 4 i32.add f32.load
    local.get $content_bounds i32.const 8 i32.add f32.load f32.const 18 f32.sub f32.const 1 call $max_f32
    local.get $content_bounds i32.const 12 i32.add f32.load
    call $rect_store)

  (func $er_ui_select_label_text_bounds  (param $bounds i32) (param $label_ptr i32) (param $label_len i32) (param $out i32) (result i32)
    (local $text_h f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 122060 call $er_ui_select_content_bounds drop
    i32.const 122060 i32.const 122076 call $er_ui_select_label_slot_bounds drop
    i32.const 122076 i32.const 12 i32.add f32.load
    local.get $label_ptr local.get $label_len i32.const 122076 i32.const 8 i32.add f32.load f32.const 16 i32.const 2 call $er_ui_alert_measured_text_height
    call $min_f32
    local.set $text_h
    local.get $out
    i32.const 122076 f32.load
    i32.const 122076 i32.const 4 i32.add f32.load i32.const 122076 i32.const 12 i32.add f32.load local.get $text_h f32.sub f32.const 0.5 f32.mul f32.add
    i32.const 122076 i32.const 8 i32.add f32.load
    local.get $text_h
    call $rect_store)

  (func $er_ui_select_arrow_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 122060 call $er_ui_select_content_bounds drop
    local.get $out
    i32.const 122060 f32.load i32.const 122060 i32.const 8 i32.add f32.load f32.add f32.const 14 f32.sub
    i32.const 122060 i32.const 4 i32.add f32.load i32.const 122060 i32.const 12 i32.add f32.load f32.const 14 f32.sub f32.const 0.5 f32.mul f32.add
    f32.const 14
    f32.const 14
    call $rect_store)

  (func $er_ui_select_measure  (param $label_ptr i32) (param $label_len i32) (param $constraints i32) (param $out i32) (result i32)
    (local $label_w f32) (local $label_h f32) (local $pref_w f32) (local $pref_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    i32.const 122092 f32.const 12 f32.store
    i32.const 122096 f32.const 0 f32.store
    i32.const 122100 f32.const 30 f32.store
    i32.const 122104 f32.const 0 f32.store
    local.get $constraints i32.const 122092 i32.const 122108 call $er_ui_layout_constraints_inner drop
    i32.const 122132 f32.const 16 f32.store
    i32.const 122136 local.get $label_ptr local.get $label_len f32.const 16 call $primitives_average_width f32.store
    i32.const 122140 i32.const 2 i32.store
    local.get $label_ptr local.get $label_len i32.const 122108 i32.const 122132 i32.const 122144 call $er_ui_text_component_measure_value drop
    i32.const 122152 f32.load local.set $label_w
    i32.const 122156 f32.load local.set $label_h
    f32.const 112 local.get $label_w f32.const 42 f32.add call $max_f32
    f32.const 40 local.get $label_h f32.const 24 f32.add call $max_f32
    local.get $constraints
    i32.const 122168
    call $er_ui_primitives_constrain_preferred_size drop
    i32.const 122168 f32.load local.set $pref_w
    i32.const 122172 f32.load local.set $pref_h
    f32.const 112 local.get $pref_w call $min_f32
    f32.const 40 local.get $pref_h call $min_f32
    local.get $pref_w local.get $pref_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit
    local.get $pref_h
    local.get $out
    call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)
  (func $er_ui_combobox_input_h  (result f32) f32.const 36)
  (func $er_ui_combobox_popup_gap  (result f32) f32.const 6)
  (func $er_ui_combobox_popup_radius  (result f32) f32.const 8)
  (func $er_ui_combobox_popup_padding  (result f32) f32.const 4)
  (func $er_ui_combobox_icon_size  (result f32) f32.const 14)
  (func $er_ui_combobox_icon_space  (result f32) f32.const 22)
  (func $er_ui_combobox_option_padding  (result f32) f32.const 8)
  (func $er_ui_combobox_option_indicator_w  (result f32) f32.const 28)
  (func $er_ui_combobox_text_max_lines  (result i32) i32.const 1)

  (func $er_ui_combobox_hit_id  (param $id i32) (param $index i32) (result i32)
    local.get $id
    local.get $index i32.const 0 i32.const 1 call $clamp_i32
    i32.add)

  (func $er_ui_combobox_input_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $bounds f32.load
    local.get $bounds i32.const 4 i32.add f32.load
    local.get $bounds i32.const 8 i32.add f32.load
    f32.const 36 local.get $bounds i32.const 12 i32.add f32.load call $min_f32
    call $rect_store)

  (func $er_ui_combobox_popup_bounds  (param $bounds i32) (param $out i32) (result i32)
    (local $y f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 4 i32.add f32.load f32.const 42 f32.add local.set $y
    local.get $out
    local.get $bounds f32.load
    local.get $y
    local.get $bounds i32.const 8 i32.add f32.load
    local.get $bounds i32.const 4 i32.add f32.load local.get $bounds i32.const 12 i32.add f32.load f32.add local.get $y f32.sub f32.const 1 call $max_f32
    call $rect_store)

  (func $er_ui_combobox_option_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 122200 call $er_ui_combobox_popup_bounds drop
    local.get $out
    i32.const 122200 f32.load f32.const 4 f32.add
    i32.const 122200 i32.const 4 i32.add f32.load f32.const 4 f32.add
    i32.const 122200 i32.const 8 i32.add f32.load f32.const 8 f32.sub f32.const 1 call $max_f32
    i32.const 122200 i32.const 12 i32.add f32.load f32.const 8 f32.sub f32.const 1 call $max_f32
    call $rect_store)

  (func $er_ui_combobox_input_content_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 122216 call $er_ui_combobox_input_bounds drop
    i32.const 122216 local.get $out call $er_ui_select_content_bounds)

  (func $er_ui_combobox_input_text_slot_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 122232 call $er_ui_combobox_input_content_bounds drop
    local.get $out
    i32.const 122232 f32.load
    i32.const 122232 i32.const 4 i32.add f32.load
    i32.const 122232 i32.const 8 i32.add f32.load f32.const 22 f32.sub f32.const 1 call $max_f32
    i32.const 122232 i32.const 12 i32.add f32.load
    call $rect_store)

  (func $er_ui_combobox_input_text_bounds  (param $bounds i32) (param $out i32) (result i32)
    (local $h f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 122248 call $er_ui_combobox_input_text_slot_bounds drop
    f32.const 16 i32.const 122248 i32.const 12 i32.add f32.load call $min_f32 local.set $h
    local.get $out
    i32.const 122248 f32.load
    i32.const 122248 i32.const 4 i32.add f32.load i32.const 122248 i32.const 12 i32.add f32.load local.get $h f32.sub f32.const 0.5 f32.mul f32.add
    i32.const 122248 i32.const 8 i32.add f32.load
    local.get $h
    call $rect_store)

  (func $er_ui_combobox_input_icon_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 122264 call $er_ui_combobox_input_content_bounds drop
    local.get $out
    i32.const 122264 f32.load i32.const 122264 i32.const 8 i32.add f32.load f32.add f32.const 14 f32.sub
    i32.const 122264 i32.const 4 i32.add f32.load i32.const 122264 i32.const 12 i32.add f32.load f32.const 14 f32.sub f32.const 0.5 f32.mul f32.add
    f32.const 14
    f32.const 14
    call $rect_store)

  (func $er_ui_combobox_option_label_slot_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 122280 call $er_ui_combobox_option_bounds drop
    local.get $out
    i32.const 122280 f32.load
    i32.const 122280 i32.const 4 i32.add f32.load
    i32.const 122280 i32.const 8 i32.add f32.load f32.const 28 f32.sub f32.const 1 call $max_f32
    i32.const 122280 i32.const 12 i32.add f32.load
    call $rect_store)

  (func $er_ui_combobox_option_text_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 122296 call $er_ui_combobox_option_label_slot_bounds drop
    i32.const 122296 local.get $out call $er_ui_toggle_text_bounds)

  (func $er_ui_combobox_option_check_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 122312 call $er_ui_combobox_option_bounds drop
    local.get $out
    i32.const 122312 f32.load i32.const 122312 i32.const 8 i32.add f32.load f32.add f32.const 22 f32.sub
    i32.const 122312 i32.const 4 i32.add f32.load i32.const 122312 i32.const 12 i32.add f32.load f32.const 14 f32.sub f32.const 0.5 f32.mul f32.add
    f32.const 14
    f32.const 14
    call $rect_store)

  (func $er_ui_combobox_measure  (param $placeholder_ptr i32) (param $placeholder_len i32) (param $selected_ptr i32) (param $selected_len i32) (param $constraints i32) (param $out i32) (result i32)
    (local $input_w f32) (local $option_w f32) (local $pref_w f32) (local $pref_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $placeholder_ptr local.get $placeholder_len f32.const 16 call $er_ui_font_text_width f32.const 46 f32.add local.set $input_w
    local.get $selected_ptr local.get $selected_len f32.const 16 call $er_ui_font_text_width f32.const 36 f32.add local.set $option_w
    local.get $input_w local.get $option_w call $max_f32 local.set $pref_w
    f32.const 60 local.set $pref_h
    local.get $pref_w local.get $pref_h local.get $constraints i32.const 122328 call $er_ui_primitives_constrain_preferred_size drop
    i32.const 122328 f32.load local.set $pref_w
    i32.const 122332 f32.load local.set $pref_h
    f32.const 47 local.get $pref_w call $min_f32
    f32.const 36 local.get $pref_h call $min_f32
    local.get $pref_w local.get $pref_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit
    local.get $pref_h
    local.get $out
    call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)
  (func $er_ui_navigation_menu_item_count  (result i32) i32.const 3)
  (func $er_ui_navigation_menu_id_stride  (result i32) i32.const 3)
  (func $er_ui_navigation_menu_gap  (result f32) f32.const 4)
  (func $er_ui_navigation_menu_item_h  (result f32) f32.const 36)
  (func $er_ui_navigation_menu_text_padding  (result f32) f32.const 10)
  (func $er_ui_navigation_menu_icon_size  (result f32) f32.const 12)
  (func $er_ui_navigation_menu_icon_space  (result f32) f32.const 16)
  (func $er_ui_navigation_menu_icon_padding  (result f32) f32.const 8)
  (func $er_ui_navigation_menu_label_max_lines  (result i32) i32.const 1)
  (func $er_ui_navigation_menu_third_label_len  (result i32) i32.const 6)

  (func $er_ui_navigation_menu_write_third_label (result i32)
    i32.const 122360 i32.const 66 i32.store8
    i32.const 122361 i32.const 108 i32.store8
    i32.const 122362 i32.const 111 i32.store8
    i32.const 122363 i32.const 99 i32.store8
    i32.const 122364 i32.const 107 i32.store8
    i32.const 122365 i32.const 115 i32.store8
    i32.const 122360)

  (func $er_ui_navigation_menu_item_width  (param $label_ptr i32) (param $label_len i32) (param $show_chevron i32) (result f32)
    local.get $label_ptr
    local.get $label_len
    f32.const 16
    i32.const 1
    f32.const 10
    call $er_ui_primitives_measured_label_width
    local.get $show_chevron
    if (result f32)
      f32.const 16
    else
      f32.const 0
    end
    f32.add)

  (func $er_ui_navigation_menu_write_widths  (param $first_ptr i32) (param $first_len i32) (param $second_ptr i32) (param $second_len i32) (param $out i32) (result i32)
    local.get $out i32.eqz
    if i32.const 0 return end
    local.get $out
    local.get $first_ptr local.get $first_len i32.const 1 call $er_ui_navigation_menu_item_width
    f32.store
    local.get $out i32.const 4 i32.add
    local.get $second_ptr local.get $second_len i32.const 1 call $er_ui_navigation_menu_item_width
    f32.store
    local.get $out i32.const 8 i32.add
    call $er_ui_navigation_menu_write_third_label i32.const 6 i32.const 0 call $er_ui_navigation_menu_item_width
    f32.store
    i32.const 1)

  (func $er_ui_navigation_menu_clamped_active  (param $active i32) (result i32)
    local.get $active i32.const 3 call $er_ui_list_clamped_index)

  (func $er_ui_navigation_menu_indexed_id  (param $id i32) (param $active i32) (result i32)
    local.get $id local.get $active i32.const 3 call $er_ui_list_encoded_indexed_id)

  (func $er_ui_navigation_menu_hit_id  (param $id i32) (param $index i32) (result i32)
    local.get $id local.get $index i32.const 3 call $er_ui_list_clamped_index i32.add)

  (func $er_ui_navigation_menu_item_bounds  (param $bounds i32) (param $first_ptr i32) (param $first_len i32) (param $second_ptr i32) (param $second_len i32) (param $index i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $first_ptr local.get $first_len local.get $second_ptr local.get $second_len i32.const 122368 call $er_ui_navigation_menu_write_widths drop
    local.get $bounds
    local.get $index
    i32.const 122368
    i32.const 3
    f32.const 0
    f32.const 4
    f32.const 36
    local.get $out
    call $er_ui_list_item_strip_bounds)

  (func $er_ui_navigation_menu_item_text_bounds  (param $item_bounds i32) (param $show_chevron i32) (param $out i32) (result i32)
    (local $icon_space f32)
    local.get $item_bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $show_chevron
    if
      f32.const 16 local.set $icon_space
    else
      f32.const 0 local.set $icon_space
    end
    i32.const 122384
    local.get $item_bounds f32.load
    local.get $item_bounds i32.const 4 i32.add f32.load
    local.get $item_bounds i32.const 8 i32.add f32.load local.get $icon_space f32.sub f32.const 1 call $max_f32
    local.get $item_bounds i32.const 12 i32.add f32.load
    call $rect_store drop
    i32.const 122384 f32.const 10 i32.const 122400 call $er_ui_primitives_content_inset drop
    i32.const 122400 local.get $out f32.const 16 call $er_ui_rect_with_height_centered)

  (func $er_ui_navigation_menu_item_icon_bounds  (param $item_bounds i32) (param $out i32) (result i32)
    local.get $item_bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $item_bounds f32.load local.get $item_bounds i32.const 8 i32.add f32.load f32.add f32.const 20 f32.sub
    local.get $item_bounds i32.const 4 i32.add f32.load local.get $item_bounds i32.const 12 i32.add f32.load f32.const 12 f32.sub f32.const 0.5 f32.mul f32.add
    f32.const 12
    f32.const 12
    call $rect_store)

  (func $er_ui_navigation_menu_measure  (param $first_ptr i32) (param $first_len i32) (param $second_ptr i32) (param $second_len i32) (param $constraints i32) (param $out i32) (result i32)
    (local $pref_w f32) (local $pref_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $first_ptr local.get $first_len local.get $second_ptr local.get $second_len i32.const 122416 call $er_ui_navigation_menu_write_widths drop
    i32.const 122416 f32.load
    i32.const 122420 f32.load
    f32.add
    i32.const 122424 f32.load
    f32.add
    f32.const 8
    f32.add
    local.set $pref_w
    f32.const 36 local.set $pref_h
    local.get $pref_w local.get $pref_h local.get $constraints i32.const 122432 call $er_ui_primitives_constrain_preferred_size drop
    i32.const 122432 f32.load local.set $pref_w
    i32.const 122436 f32.load local.set $pref_h
    f32.const 11 local.get $pref_w call $min_f32
    f32.const 36 local.get $pref_h call $min_f32
    local.get $pref_w local.get $pref_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit
    local.get $pref_h
    local.get $out
    call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)
  (func $er_ui_menubar_item_count  (result i32) i32.const 3)
  (func $er_ui_menubar_id_stride  (result i32) i32.const 3)
  (func $er_ui_menubar_padding  (result f32) f32.const 4)
  (func $er_ui_menubar_item_h  (result f32) f32.const 28)
  (func $er_ui_menubar_item_padding_x  (result f32) f32.const 8)
  (func $er_ui_menubar_label_max_lines  (result i32) i32.const 1)
  (func $er_ui_menubar_third_label_len  (result i32) i32.const 4)

  (func $er_ui_menubar_active_index  (param $active i32) (result i32)
    local.get $active i32.const 3 call $er_ui_list_clamped_index)

  (func $er_ui_menubar_indexed_id  (param $id i32) (param $active i32) (result i32)
    local.get $id i32.const 3 i32.mul
    local.get $active i32.const 3 call $er_ui_list_clamped_index
    i32.add)

  (func $er_ui_menubar_hit_id  (param $id i32) (param $index i32) (result i32)
    local.get $id
    local.get $index i32.const 0 i32.const 2 call $clamp_i32
    i32.add)

  (func $er_ui_menubar_write_third_label (result i32)
    i32.const 122340 i32.const 86 i32.store8
    i32.const 122341 i32.const 105 i32.store8
    i32.const 122342 i32.const 101 i32.store8
    i32.const 122343 i32.const 119 i32.store8
    i32.const 122340)

  (func $er_ui_menubar_item_width (param $label_ptr i32) (param $label_len i32) (result f32)
    local.get $label_ptr local.get $label_len f32.const 16 call $er_ui_font_text_width
    f32.const 16
    f32.add)

  (func $er_ui_menubar_write_widths (param $first_ptr i32) (param $first_len i32) (param $second_ptr i32) (param $second_len i32) (param $out i32) (result i32)
    local.get $out i32.eqz
    if i32.const 0 return end
    local.get $out local.get $first_ptr local.get $first_len call $er_ui_menubar_item_width f32.store
    local.get $out i32.const 4 i32.add local.get $second_ptr local.get $second_len call $er_ui_menubar_item_width f32.store
    local.get $out i32.const 8 i32.add call $er_ui_menubar_write_third_label i32.const 4 call $er_ui_menubar_item_width f32.store
    i32.const 1)

  (func $er_ui_menubar_item_bounds  (param $bounds i32) (param $first_ptr i32) (param $first_len i32) (param $second_ptr i32) (param $second_len i32) (param $index i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $first_ptr local.get $first_len local.get $second_ptr local.get $second_len i32.const 122348 call $er_ui_menubar_write_widths drop
    local.get $bounds
    local.get $index
    i32.const 122348
    i32.const 3
    f32.const 4
    f32.const 0
    f32.const 28
    local.get $out
    call $er_ui_list_item_strip_bounds)

  (func $er_ui_menubar_item_text_bounds  (param $bounds i32) (param $first_ptr i32) (param $first_len i32) (param $second_ptr i32) (param $second_len i32) (param $index i32) (param $out i32) (result i32)
    (local $pad f32) (local $inner_h f32) (local $text_h f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds local.get $first_ptr local.get $first_len local.get $second_ptr local.get $second_len local.get $index i32.const 122364 call $er_ui_menubar_item_bounds drop
    f32.const 8
    i32.const 122364 i32.const 8 i32.add f32.load
    i32.const 122364 i32.const 12 i32.add f32.load
    call $min_f32
    f32.const 0.5
    f32.mul
    call $min_f32
    local.set $pad
    i32.const 122364 i32.const 12 i32.add f32.load local.get $pad f32.const 2 f32.mul f32.sub f32.const 0 call $max_f32 local.set $inner_h
    f32.const 16 local.get $inner_h call $min_f32 local.set $text_h
    local.get $out
    i32.const 122364 f32.load local.get $pad f32.add
    i32.const 122364 i32.const 4 i32.add f32.load local.get $pad f32.add local.get $inner_h local.get $text_h f32.sub f32.const 0.5 f32.mul f32.add
    i32.const 122364 i32.const 8 i32.add f32.load local.get $pad f32.const 2 f32.mul f32.sub f32.const 0 call $max_f32
    local.get $text_h
    call $rect_store)

  (func $er_ui_menubar_measure  (param $first_ptr i32) (param $first_len i32) (param $second_ptr i32) (param $second_len i32) (param $constraints i32) (param $out i32) (result i32)
    (local $pref_w f32) (local $pref_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $first_ptr local.get $first_len local.get $second_ptr local.get $second_len i32.const 122348 call $er_ui_menubar_write_widths drop
    i32.const 122348 f32.load
    i32.const 122352 f32.load f32.add
    i32.const 122356 f32.load f32.add
    f32.const 8 f32.add
    f32.const 36
    local.get $constraints
    i32.const 122380
    call $er_ui_primitives_constrain_preferred_size drop
    i32.const 122380 f32.load local.set $pref_w
    i32.const 122384 f32.load local.set $pref_h
    f32.const 11 local.get $pref_w call $min_f32
    f32.const 36 local.get $pref_h call $min_f32
    local.get $pref_w local.get $pref_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit
    local.get $pref_h
    local.get $out
    call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)
  (func $er_ui_carousel_button_size  (result f32) f32.const 28)
  (func $er_ui_carousel_gap  (result f32) f32.const 8)
  (func $er_ui_carousel_radius  (result f32) f32.const 8)
  (func $er_ui_carousel_text_padding  (result f32) f32.const 8)
  (func $er_ui_carousel_label_max_lines  (result i32) i32.const 1)

  (func $er_ui_carousel_hit_id  (param $id i32) (param $index i32) (result i32)
    local.get $id
    local.get $index i32.const 0 i32.const 1 call $clamp_i32
    i32.add)

  (func $er_ui_carousel_button_bounds  (param $bounds i32) (param $index i32) (param $out i32) (result i32)
    (local $y f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 4 i32.add f32.load
    local.get $bounds i32.const 12 i32.add f32.load f32.const 28 f32.sub f32.const 0.5 f32.mul
    f32.add
    local.set $y
    local.get $index i32.eqz
    if
      local.get $out
      local.get $bounds f32.load
      local.get $y
      f32.const 28
      f32.const 28
      call $rect_store
      return
    end
    local.get $out
    local.get $bounds f32.load local.get $bounds i32.const 8 i32.add f32.load f32.add f32.const 28 f32.sub
    local.get $y
    f32.const 28
    f32.const 28
    call $rect_store)

  (func $er_ui_carousel_content_bounds  (param $bounds i32) (param $out i32) (result i32)
    (local $x f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds f32.load f32.const 36 f32.add local.set $x
    local.get $out
    local.get $x
    local.get $bounds i32.const 4 i32.add f32.load
    local.get $bounds i32.const 8 i32.add f32.load f32.const 72 f32.sub f32.const 1 call $max_f32
    local.get $bounds i32.const 12 i32.add f32.load
    call $rect_store)

  (func $er_ui_carousel_content_inner_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 122400 call $er_ui_carousel_content_bounds drop
    i32.const 122400 f32.const 8 local.get $out call $er_ui_primitives_content_inset)

  (func $er_ui_carousel_label_text_bounds  (param $bounds i32) (param $out i32) (result i32)
    (local $h f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 122416 call $er_ui_carousel_content_inner_bounds drop
    f32.const 16 i32.const 122416 i32.const 12 i32.add f32.load call $min_f32 local.set $h
    local.get $out
    i32.const 122416 f32.load
    i32.const 122416 i32.const 4 i32.add f32.load i32.const 122416 i32.const 12 i32.add f32.load local.get $h f32.sub f32.const 0.5 f32.mul f32.add
    i32.const 122416 i32.const 8 i32.add f32.load
    local.get $h
    call $rect_store)

  (func $er_ui_carousel_measure  (param $label_ptr i32) (param $label_len i32) (param $constraints i32) (param $out i32) (result i32)
    (local $label_w f32) (local $pref_w f32) (local $pref_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $label_ptr local.get $label_len f32.const 16 call $er_ui_font_text_width local.set $label_w
    local.get $label_w f32.const 72 f32.add local.set $pref_w
    f32.const 32 local.set $pref_h
    local.get $pref_w local.get $pref_h local.get $constraints i32.const 122432 call $er_ui_primitives_constrain_preferred_size drop
    i32.const 122432 f32.load local.set $pref_w
    i32.const 122436 f32.load local.set $pref_h
    f32.const 57 local.get $pref_w call $min_f32
    f32.const 28 local.get $pref_h call $min_f32
    local.get $pref_w local.get $pref_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit
    local.get $pref_h
    local.get $out
    call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)

  (func $er_ui_direction_item_count  (result i32) i32.const 2)
  (func $er_ui_direction_ltr_label_len  (result i32) i32.const 3)
  (func $er_ui_direction_rtl_label_len  (result i32) i32.const 3)
  (func $er_ui_direction_item_h  (result f32) f32.const 20)
  (func $er_ui_direction_item_radius  (result f32) f32.const 6)
  (func $er_ui_direction_item_padding  (result f32) f32.const 5)
  (func $er_ui_direction_item_text_h  (result f32) f32.const 12)
  (func $er_ui_direction_icon_size  (result f32) f32.const 18)
  (func $er_ui_direction_gap  (result f32) f32.const 12)
  (func $er_ui_direction_vertical_padding  (result f32) f32.const 8)
  (func $er_ui_direction_label_max_lines  (result i32) i32.const 1)

  (func $er_ui_direction_active_index  (param $active i32) (result i32)
    local.get $active i32.const 2 call $er_ui_list_clamped_index)

  (func $er_ui_direction_indexed_id  (param $id i32) (param $active i32) (result i32)
    local.get $id local.get $active i32.const 2 call $er_ui_list_encoded_indexed_id)

  (func $er_ui_direction_hit_id  (param $id i32) (param $index i32) (result i32)
    local.get $id
    local.get $index i32.const 0 i32.const 1 call $clamp_i32
    i32.add)

  (func $er_ui_direction_write_ltr_label (result i32)
    i32.const 122520 i32.const 76 i32.store8
    i32.const 122521 i32.const 84 i32.store8
    i32.const 122522 i32.const 82 i32.store8
    i32.const 122520)

  (func $er_ui_direction_write_rtl_label (result i32)
    i32.const 122524 i32.const 82 i32.store8
    i32.const 122525 i32.const 84 i32.store8
    i32.const 122526 i32.const 76 i32.store8
    i32.const 122524)

  (func $er_ui_direction_item_width  (param $label_ptr i32) (param $label_len i32) (result f32)
    local.get $label_ptr local.get $label_len f32.const 12 i32.const 1 f32.const 5 call $er_ui_primitives_measured_label_width)

  (func $er_ui_direction_write_widths  (param $out i32) (result i32)
    local.get $out i32.eqz
    if i32.const 0 return end
    local.get $out call $er_ui_direction_write_ltr_label i32.const 3 call $er_ui_direction_item_width f32.store
    local.get $out i32.const 4 i32.add call $er_ui_direction_write_rtl_label i32.const 3 call $er_ui_direction_item_width f32.store
    i32.const 1)

  (func $er_ui_direction_first_w (result f32)
    i32.const 122528 call $er_ui_direction_write_widths drop
    i32.const 122528 f32.load)

  (func $er_ui_direction_item_bounds  (param $bounds i32) (param $index i32) (param $out i32) (result i32)
    (local $y f32) (local $first_w f32) (local $second_w f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    i32.const 122528 call $er_ui_direction_write_widths drop
    i32.const 122528 f32.load local.set $first_w
    i32.const 122532 f32.load local.set $second_w
    local.get $bounds i32.const 4 i32.add f32.load local.get $bounds i32.const 12 i32.add f32.load f32.const 20 f32.sub f32.const 0.5 f32.mul f32.add local.set $y
    local.get $index i32.const 0 i32.eq
    if
      local.get $out
      local.get $bounds f32.load
      local.get $y
      local.get $first_w
      f32.const 20
      call $rect_store
      return
    end
    local.get $out
    local.get $bounds f32.load local.get $first_w f32.add f32.const 42 f32.add
    local.get $y
    local.get $second_w
    f32.const 20
    call $rect_store)

  (func $er_ui_direction_item_text_bounds  (param $item_bounds i32) (param $out i32) (result i32)
    local.get $item_bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $item_bounds f32.const 5 i32.const 122536 call $er_ui_primitives_content_inset drop
    i32.const 122536 local.get $out f32.const 12 call $er_ui_rect_with_height_centered)

  (func $er_ui_direction_icon_bounds  (param $bounds i32) (param $out i32) (result i32)
    (local $first_w f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    call $er_ui_direction_first_w local.set $first_w
    local.get $out
    local.get $bounds f32.load local.get $first_w f32.add f32.const 12 f32.add
    local.get $bounds i32.const 4 i32.add f32.load local.get $bounds i32.const 12 i32.add f32.load f32.const 18 f32.sub f32.const 0.5 f32.mul f32.add
    f32.const 18
    f32.const 18
    call $rect_store)

  (func $er_ui_direction_measure  (param $constraints i32) (param $out i32) (result i32)
    (local $first_w f32) (local $second_w f32) (local $pref_w f32) (local $pref_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    i32.const 122528 call $er_ui_direction_write_widths drop
    i32.const 122528 f32.load local.set $first_w
    i32.const 122532 f32.load local.set $second_w
    local.get $first_w f32.const 12 f32.add f32.const 18 f32.add f32.const 12 f32.add local.get $second_w f32.add local.set $pref_w
    f32.const 36 local.set $pref_h
    local.get $pref_w local.get $pref_h local.get $constraints i32.const 122552 call $er_ui_primitives_constrain_preferred_size drop
    i32.const 122552 f32.load local.set $pref_w
    i32.const 122556 f32.load local.set $pref_h
    f32.const 44 local.get $pref_w call $min_f32
    f32.const 18 local.get $pref_h call $min_f32
    local.get $pref_w local.get $pref_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit
    local.get $pref_h
    local.get $out
    call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)
  (func $er_ui_resizable_handle_w  (result f32) f32.const 6)
  (func $er_ui_resizable_handle_radius  (result f32) f32.const 3)
  (func $er_ui_resizable_handle_hit_outset  (result f32) f32.const 6)
  (func $er_ui_resizable_min_width  (result f32) f32.const 96)
  (func $er_ui_resizable_min_height  (result f32) f32.const 36)

  (func $er_ui_resizable_clamped_ratio  (param $ratio f32) (result f32)
    local.get $ratio f32.const 0 call $max_f32 f32.const 1 call $min_f32)

  (func $er_ui_resizable_handle_bounds  (param $bounds i32) (param $ratio f32) (param $out i32) (result i32)
    (local $center_x f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds f32.load
    local.get $bounds i32.const 8 i32.add f32.load
    local.get $ratio call $er_ui_resizable_clamped_ratio
    f32.mul
    f32.add
    local.set $center_x
    local.get $out
    local.get $center_x f32.const 3 f32.sub
    local.get $bounds i32.const 4 i32.add f32.load
    f32.const 6
    local.get $bounds i32.const 12 i32.add f32.load
    call $rect_store)

  (func $er_ui_resizable_handle_hit_bounds  (param $bounds i32) (param $ratio f32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds local.get $ratio i32.const 123060 call $er_ui_resizable_handle_bounds drop
    i32.const 123060 local.get $out f32.const -6 call $er_ui_rect_inset_uniform)

  (func $er_ui_resizable_left_bounds  (param $bounds i32) (param $ratio f32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds local.get $ratio i32.const 123076 call $er_ui_resizable_handle_bounds drop
    local.get $out
    local.get $bounds f32.load
    local.get $bounds i32.const 4 i32.add f32.load
    i32.const 123076 f32.load local.get $bounds f32.load f32.sub f32.const 1 call $max_f32
    local.get $bounds i32.const 12 i32.add f32.load
    call $rect_store)

  (func $er_ui_resizable_right_bounds  (param $bounds i32) (param $ratio f32) (param $out i32) (result i32)
    (local $right_x f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds local.get $ratio i32.const 123092 call $er_ui_resizable_handle_bounds drop
    i32.const 123092 f32.load i32.const 123092 i32.const 8 i32.add f32.load f32.add local.set $right_x
    local.get $out
    local.get $right_x
    local.get $bounds i32.const 4 i32.add f32.load
    local.get $bounds f32.load local.get $bounds i32.const 8 i32.add f32.load f32.add local.get $right_x f32.sub f32.const 1 call $max_f32
    local.get $bounds i32.const 12 i32.add f32.load
    call $rect_store)

  (func $er_ui_resizable_measure  (param $constraints i32) (param $out i32) (result i32)
    (local $pref_w f32) (local $pref_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    f32.const 96 f32.const 36 local.get $constraints i32.const 123108 call $er_ui_primitives_constrain_preferred_size drop
    i32.const 123108 f32.load local.set $pref_w
    i32.const 123112 f32.load local.set $pref_h
    f32.const 96 local.get $pref_w call $min_f32
    f32.const 36 local.get $pref_h call $min_f32
    local.get $pref_w local.get $pref_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit
    local.get $pref_h
    local.get $out
    call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)
  (func $er_ui_scroll_area_radius  (result f32) f32.const 7)
  (func $er_ui_scroll_area_padding  (result f32) f32.const 8)
  (func $er_ui_scroll_area_content_y  (result f32) f32.const 6)
  (func $er_ui_scroll_area_text_h  (result f32) f32.const 14)
  (func $er_ui_scroll_area_scrollbar_w  (result f32) f32.const 10)
  (func $er_ui_scroll_area_track_inset_x  (result f32) f32.const 6)
  (func $er_ui_scroll_area_track_inset_y  (result f32) f32.const 5)
  (func $er_ui_scroll_area_track_w  (result f32) f32.const 3)
  (func $er_ui_scroll_area_track_radius  (result f32) f32.const 2)
  (func $er_ui_scroll_area_thumb_min_h  (result f32) f32.const 12)
  (func $er_ui_scroll_area_thumb_ratio  (result f32) f32.const 0.45)
  (func $er_ui_scroll_area_label_len  (result i32) i32.const 18)
  (func $er_ui_scroll_area_label_max_lines  (result i32) i32.const 1)

  (func $er_ui_scroll_area_write_label (result i32)
    i32.const 123140 i32.const 83 i32.store8
    i32.const 123141 i32.const 99 i32.store8
    i32.const 123142 i32.const 114 i32.store8
    i32.const 123143 i32.const 111 i32.store8
    i32.const 123144 i32.const 108 i32.store8
    i32.const 123145 i32.const 108 i32.store8
    i32.const 123146 i32.const 97 i32.store8
    i32.const 123147 i32.const 98 i32.store8
    i32.const 123148 i32.const 108 i32.store8
    i32.const 123149 i32.const 101 i32.store8
    i32.const 123150 i32.const 32 i32.store8
    i32.const 123151 i32.const 99 i32.store8
    i32.const 123152 i32.const 111 i32.store8
    i32.const 123153 i32.const 110 i32.store8
    i32.const 123154 i32.const 116 i32.store8
    i32.const 123155 i32.const 101 i32.store8
    i32.const 123156 i32.const 110 i32.store8
    i32.const 123157 i32.const 116 i32.store8
    i32.const 123140)

  (func $er_ui_scroll_area_viewport_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $bounds f32.load f32.const 8 f32.add
    local.get $bounds i32.const 4 i32.add f32.load f32.const 5 f32.add
    local.get $bounds i32.const 8 i32.add f32.load f32.const 26 f32.sub f32.const 1 call $max_f32
    local.get $bounds i32.const 12 i32.add f32.load f32.const 10 f32.sub f32.const 1 call $max_f32
    call $rect_store)

  (func $er_ui_scroll_area_track_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $bounds f32.load local.get $bounds i32.const 8 i32.add f32.load f32.add f32.const 6 f32.sub
    local.get $bounds i32.const 4 i32.add f32.load f32.const 5 f32.add
    f32.const 3
    local.get $bounds i32.const 12 i32.add f32.load f32.const 10 f32.sub f32.const 1 call $max_f32
    call $rect_store)

  (func $er_ui_scroll_area_default_metrics  (param $bounds i32) (param $out i32) (result i32)
    (local $viewport_h f32) (local $content_h f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 12 i32.add f32.load f32.const 10 f32.sub f32.const 1 call $max_f32 local.set $viewport_h
    local.get $viewport_h f32.const 0.45 f32.div local.set $content_h
    local.get $out local.get $viewport_h f32.store
    local.get $out i32.const 4 i32.add local.get $content_h f32.store
    local.get $out i32.const 8 i32.add f32.const 0 f32.store
    i32.const 1)

  (func $er_ui_scroll_area_metrics  (param $viewport_h f32) (param $content_h f32) (param $offset_y f32) (param $out i32) (result i32)
    (local $vp f32) (local $content f32) (local $max_offset f32) (local $offset f32)
    local.get $out i32.eqz
    if i32.const 0 return end
    local.get $viewport_h f32.const 1 call $max_f32 local.set $vp
    local.get $content_h local.get $vp call $max_f32 local.set $content
    local.get $content local.get $vp f32.sub f32.const 0 call $max_f32 local.set $max_offset
    local.get $offset_y f32.const 0 call $max_f32 local.get $max_offset call $min_f32 local.set $offset
    local.get $out local.get $vp f32.store
    local.get $out i32.const 4 i32.add local.get $content f32.store
    local.get $out i32.const 8 i32.add local.get $offset f32.store
    i32.const 1)

  (func $er_ui_scroll_area_metrics_max_offset  (param $metrics i32) (result f32)
    local.get $metrics i32.eqz
    if f32.const 0 return end
    local.get $metrics i32.const 4 i32.add f32.load local.get $metrics f32.load f32.sub f32.const 0 call $max_f32)

  (func $er_ui_scroll_area_thumb_bounds  (param $track i32) (param $metrics i32) (param $out i32) (result i32)
    (local $ratio f32) (local $thumb_h f32) (local $travel f32) (local $max_offset f32) (local $offset_ratio f32)
    local.get $track i32.eqz local.get $metrics i32.eqz i32.or local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $metrics f32.load
    local.get $metrics f32.load local.get $metrics i32.const 4 i32.add f32.load call $max_f32
    f32.div
    f32.const 0 call $max_f32 f32.const 1 call $min_f32
    local.set $ratio
    local.get $track i32.const 12 i32.add f32.load local.get $ratio f32.mul f32.const 12 call $max_f32
    local.get $track i32.const 12 i32.add f32.load call $min_f32
    local.set $thumb_h
    local.get $track i32.const 12 i32.add f32.load local.get $thumb_h f32.sub f32.const 0 call $max_f32 local.set $travel
    local.get $metrics call $er_ui_scroll_area_metrics_max_offset local.set $max_offset
    local.get $max_offset f32.const 0 f32.eq
    if
      f32.const 0 local.set $offset_ratio
    else
      local.get $metrics i32.const 8 i32.add f32.load local.get $max_offset f32.div local.set $offset_ratio
    end
    local.get $out
    local.get $track f32.load
    local.get $track i32.const 4 i32.add f32.load local.get $travel local.get $offset_ratio f32.mul f32.add
    local.get $track i32.const 8 i32.add f32.load
    local.get $thumb_h
    call $rect_store)

  (func $er_ui_scroll_area_measure  (param $constraints i32) (param $out i32) (result i32)
    (local $label_w f32) (local $pref_w f32) (local $pref_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    call $er_ui_scroll_area_write_label i32.const 18 f32.const 14 call $er_ui_font_text_width local.set $label_w
    local.get $label_w f32.const 26 f32.add
    f32.const 24
    local.get $constraints i32.const 123164 call $er_ui_primitives_constrain_preferred_size drop
    i32.const 123164 f32.load local.set $pref_w
    i32.const 123168 f32.load local.set $pref_h
    f32.const 27 local.get $pref_w call $min_f32
    f32.const 24 local.get $pref_h call $min_f32
    local.get $pref_w local.get $pref_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit
    local.get $pref_h
    local.get $out
    call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)
