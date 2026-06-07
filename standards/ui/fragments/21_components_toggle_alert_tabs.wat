(module
  ;; Imports from other UI fragments
  (import "ui" "er_ui_font_text_width" (func $er_ui_font_text_width (param i32) (param i32) (param f32) (result f32)))
  (import "ui" "er_ui_layout_axis_constraint_limit" (func $er_ui_layout_axis_constraint_limit (param i32) (param f32) (result f32)))
  (import "ui" "er_ui_layout_constraints_inner" (func $er_ui_layout_constraints_inner (param i32) (param i32) (param i32) (result i32)))
  (import "ui" "er_ui_layout_insets_uniform" (func $er_ui_layout_insets_uniform (param f32) (param i32) (result i32)))
  (import "ui" "er_ui_layout_measurement_apply_exact" (func $er_ui_layout_measurement_apply_exact (param i32) (param i32) (param i32) (result i32)))
  (import "ui" "er_ui_layout_measurement_flexible" (func $er_ui_layout_measurement_flexible (param f32) (param f32) (param f32) (param f32) (param f32) (param f32) (param i32) (result i32)))
  (import "ui" "er_ui_layout_measurement_with_insets" (func $er_ui_layout_measurement_with_insets (param i32) (param i32) (param i32) (result i32)))
  (import "ui" "er_ui_list_clamped_index" (func $er_ui_list_clamped_index (param i32) (param i32) (result i32)))
  (import "ui" "er_ui_list_equal_segment_bounds_gap" (func $er_ui_list_equal_segment_bounds_gap (param i32) (param i32) (param i32) (param f32) (param i32) (result i32)))
  (import "ui" "er_ui_primitives_constrain_preferred_size" (func $er_ui_primitives_constrain_preferred_size (param f32) (param f32) (param i32) (param i32) (result i32)))
  (import "ui" "er_ui_text_component_measure_value" (func $er_ui_text_component_measure_value (param i32) (param i32) (param i32) (param i32) (param i32) (result i32)))
  (import "ui" "layout_store_size" (func $layout_store_size (param i32) (param f32) (param f32) (result i32)))
  (import "ui" "max_f32" (func $max_f32 (param f32) (param f32) (result f32)))
  (import "ui" "min_f32" (func $min_f32 (param f32) (param f32) (result f32)))
  (import "ui" "min_i32_u" (func $min_i32_u (param i32) (param i32) (result i32)))
  (import "ui" "primitives_average_width" (func $primitives_average_width (param i32) (param i32) (param f32) (result f32)))
  (import "ui" "rect_store" (func $rect_store (param i32) (param f32) (param f32) (param f32) (param f32) (result i32)))

  (func $er_ui_toggle_text_padding  (result f32) f32.const 8)
  (func $er_ui_toggle_label_max_lines  (result i32) i32.const 1)
  (func $er_ui_toggle_group_item_count  (result i32) i32.const 3)
  (func $er_ui_toggle_group_id_stride  (result i32) i32.const 3)
  (func $er_ui_toggle_group_third_label_len  (result i32) i32.const 5)

  (func $er_ui_toggle_write_right_label (result i32)
    i32.const 121140 i32.const 82 i32.store8
    i32.const 121141 i32.const 105 i32.store8
    i32.const 121142 i32.const 103 i32.store8
    i32.const 121143 i32.const 104 i32.store8
    i32.const 121144 i32.const 116 i32.store8
    i32.const 121140)

  (func $er_ui_toggle_unconstrained_nowrap (param $out i32) (result i32)
    local.get $out i32.eqz
    if i32.const 0 return end
    local.get $out i32.const 0 i32.store
    local.get $out i32.const 4 i32.add f32.const 0 f32.store
    local.get $out i32.const 8 i32.add i32.const 0 i32.store
    local.get $out i32.const 12 i32.add f32.const 0 f32.store
    local.get $out i32.const 16 i32.add i32.const 2 i32.store
    i32.const 1)

  (func $er_ui_toggle_measure_label (param $label_ptr i32) (param $label_len i32) (param $out i32) (result i32)
    local.get $out i32.eqz
    if i32.const 0 return end
    i32.const 121148 call $er_ui_toggle_unconstrained_nowrap drop
    i32.const 121172 f32.const 16 f32.store
    i32.const 121176 local.get $label_ptr local.get $label_len f32.const 16 call $primitives_average_width f32.store
    i32.const 121180 i32.const 1 i32.store
    local.get $label_ptr local.get $label_len i32.const 121148 i32.const 121172 local.get $out call $er_ui_text_component_measure_value)

  (func $er_ui_toggle_preferred_size  (param $label_ptr i32) (param $label_len i32) (param $out i32) (result i32)
    local.get $out i32.eqz
    if i32.const 0 return end
    local.get $label_ptr local.get $label_len i32.const 121184 call $er_ui_toggle_measure_label drop
    local.get $out
    i32.const 121192 f32.load f32.const 16 f32.add
    i32.const 121196 f32.load f32.const 16 f32.add
    call $layout_store_size)

  (func $er_ui_toggle_measure  (param $label_ptr i32) (param $label_len i32) (param $constraints i32) (param $out i32) (result i32)
    (local $pref_w f32) (local $pref_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $label_ptr local.get $label_len i32.const 121184 call $er_ui_toggle_measure_label drop
    i32.const 121192 f32.load f32.const 16 f32.add
    i32.const 121196 f32.load f32.const 16 f32.add
    local.get $constraints
    i32.const 121208
    call $er_ui_primitives_constrain_preferred_size drop
    i32.const 121208 f32.load local.set $pref_w
    i32.const 121212 f32.load local.set $pref_h
    f32.const 17
    f32.const 32
    local.get $pref_w local.get $pref_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit
    local.get $pref_h
    local.get $out
    call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)

  (func $er_ui_toggle_text_bounds (export "er_ui_toggle_text_bounds")  (param $bounds i32) (param $out i32) (result i32)
    (local $pad f32) (local $inner_h f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    f32.const 8
    local.get $bounds i32.const 8 i32.add f32.load
    local.get $bounds i32.const 12 i32.add f32.load
    call $min_f32
    f32.const 0.5
    f32.mul
    call $min_f32
    local.set $pad
    local.get $bounds i32.const 12 i32.add f32.load local.get $pad f32.const 2 f32.mul f32.sub f32.const 0 call $max_f32 local.set $inner_h
    local.get $out
    local.get $bounds f32.load local.get $pad f32.add
    local.get $bounds i32.const 4 i32.add f32.load local.get $pad f32.add local.get $inner_h f32.const 16 f32.sub f32.const 0.5 f32.mul f32.add
    local.get $bounds i32.const 8 i32.add f32.load local.get $pad f32.const 2 f32.mul f32.sub f32.const 0 call $max_f32
    f32.const 16
    call $rect_store)

  (func $er_ui_toggle_group_active_index  (param $active i32) (result i32)
    local.get $active i32.const 3 call $er_ui_list_clamped_index)

  (func $er_ui_toggle_group_indexed_id  (param $id i32) (param $active i32) (result i32)
    local.get $id i32.const 3 i32.mul
    local.get $active i32.const 3 call $er_ui_list_clamped_index
    i32.add)

  (func $er_ui_toggle_group_item_bounds  (param $bounds i32) (param $index i32) (param $out i32) (result i32)
    local.get $bounds local.get $index i32.const 3 f32.const 0 local.get $out call $er_ui_list_equal_segment_bounds_gap)

  (func $er_ui_toggle_group_item_text_bounds  (param $bounds i32) (param $index i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds local.get $index i32.const 3 f32.const 0 i32.const 121224 call $er_ui_list_equal_segment_bounds_gap drop
    i32.const 121224 local.get $out call $er_ui_toggle_text_bounds)

  (func $er_ui_toggle_group_label_width (param $label_ptr i32) (param $label_len i32) (result f32)
    local.get $label_ptr local.get $label_len f32.const 16 call $er_ui_font_text_width
    f32.const 16
    f32.add)

  (func $er_ui_toggle_group_measure  (param $first_ptr i32) (param $first_len i32) (param $second_ptr i32) (param $second_len i32) (param $constraints i32) (param $out i32) (result i32)
    (local $item_w f32) (local $item_h f32) (local $pref_w f32) (local $pref_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    f32.const 0 local.set $item_w
    f32.const 32 local.set $item_h
    local.get $first_ptr local.get $first_len call $er_ui_toggle_group_label_width local.get $item_w call $max_f32 local.set $item_w
    local.get $second_ptr local.get $second_len call $er_ui_toggle_group_label_width local.get $item_w call $max_f32 local.set $item_w
    call $er_ui_toggle_write_right_label i32.const 5 call $er_ui_toggle_group_label_width local.get $item_w call $max_f32 local.set $item_w
    local.get $item_w f32.const 3 f32.mul
    local.get $item_h
    local.get $constraints
    i32.const 121208
    call $er_ui_primitives_constrain_preferred_size drop
    i32.const 121208 f32.load local.set $pref_w
    i32.const 121212 f32.load local.set $pref_h
    f32.const 3 local.get $pref_w call $min_f32
    f32.const 32 local.get $pref_h call $min_f32
    local.get $pref_w local.get $pref_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit
    local.get $pref_h
    local.get $out
    call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)
  (func $er_ui_alert_radius  (result f32) f32.const 8)
  (func $er_ui_alert_padding_x  (result f32) f32.const 16)
  (func $er_ui_alert_padding_y  (result f32) f32.const 12)
  (func $er_ui_alert_icon_size  (result f32) f32.const 16)
  (func $er_ui_alert_text_x  (result f32) f32.const 44)
  (func $er_ui_alert_title_height  (result f32) f32.const 16)
  (func $er_ui_alert_title_max_lines  (result i32) i32.const 2)
  (func $er_ui_alert_detail_gap  (result f32) f32.const 2)
  (func $er_ui_alert_detail_height  (result f32) f32.const 16)
  (func $er_ui_alert_detail_max_lines  (result i32) i32.const 2)
  (func $er_ui_alert_min_width  (result f32) f32.const 160)
  (func $er_ui_alert_min_height  (result f32) f32.const 48)
  (func $er_ui_alert_icon_shift  (result i32) i32.const 1)
  (func $er_ui_alert_danger_color  (result i32) i32.const 4282664175)

  (func $er_ui_alert_packed_id  (param $destructive i32) (param $icon_tag i32) (result i32)
    local.get $destructive i32.const 0 i32.ne
    local.get $icon_tag i32.const 1 i32.shl
    i32.or)

  (func $er_ui_alert_packed_destructive  (param $packed i32) (result i32)
    local.get $packed i32.const 1 i32.and)

  (func $er_ui_alert_packed_icon_tag  (param $packed i32) (result i32)
    local.get $packed i32.const 1 i32.shr_u)

  (func $er_ui_alert_text_width  (param $bounds i32) (result f32)
    local.get $bounds i32.eqz
    if f32.const 0 return end
    local.get $bounds i32.const 8 i32.add f32.load f32.const 60 f32.sub f32.const 1 call $max_f32)

  (func $er_ui_alert_icon_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $bounds f32.load f32.const 16 f32.add
    local.get $bounds i32.const 4 i32.add f32.load f32.const 12 f32.add
    f32.const 16
    f32.const 16
    call $rect_store)

  (func $er_ui_alert_measured_text_height (export "er_ui_alert_measured_text_height") (param $text_ptr i32) (param $text_len i32) (param $width f32) (param $line_height f32) (param $max_lines i32) (result f32)
    i32.const 121260 i32.const 1 i32.store
    i32.const 121264 local.get $width f32.const 0 call $max_f32 f32.store
    i32.const 121268 i32.const 0 i32.store
    i32.const 121272 f32.const 0 f32.store
    i32.const 121276 i32.const 1 i32.store
    i32.const 121284 local.get $line_height f32.store
    i32.const 121288 local.get $text_ptr local.get $text_len local.get $line_height call $primitives_average_width f32.store
    i32.const 121292 local.get $max_lines i32.store
    local.get $text_ptr local.get $text_len i32.const 121260 i32.const 121284 i32.const 121296 call $er_ui_text_component_measure_value drop
    i32.const 121308 f32.load)

  (func $er_ui_alert_title_bounds  (param $bounds i32) (param $title_ptr i32) (param $title_len i32) (param $out i32) (result i32)
    (local $text_w f32) (local $title_h f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds call $er_ui_alert_text_width local.set $text_w
    local.get $title_ptr local.get $title_len local.get $text_w f32.const 16 i32.const 2 call $er_ui_alert_measured_text_height local.set $title_h
    local.get $out
    local.get $bounds f32.load f32.const 44 f32.add
    local.get $bounds i32.const 4 i32.add f32.load f32.const 11 f32.add
    local.get $text_w
    local.get $title_h
    call $rect_store)

  (func $er_ui_alert_detail_bounds  (param $bounds i32) (param $title_ptr i32) (param $title_len i32) (param $out i32) (result i32)
    (local $text_w f32) (local $title_h f32) (local $y f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds call $er_ui_alert_text_width local.set $text_w
    local.get $title_ptr local.get $title_len local.get $text_w f32.const 16 i32.const 2 call $er_ui_alert_measured_text_height local.set $title_h
    local.get $bounds i32.const 4 i32.add f32.load f32.const 12 f32.add local.get $title_h f32.add f32.const 2 f32.add local.set $y
    local.get $out
    local.get $bounds f32.load f32.const 44 f32.add
    local.get $y
    local.get $text_w
    local.get $bounds i32.const 12 i32.add f32.load f32.const 24 f32.sub local.get $title_h f32.sub f32.const 1 call $max_f32
    call $rect_store)

  (func $er_ui_alert_measure  (param $title_ptr i32) (param $title_len i32) (param $detail_ptr i32) (param $detail_len i32) (param $constraints i32) (param $out i32) (result i32)
    (local $title_w f32) (local $title_h f32) (local $detail_w f32) (local $detail_h f32) (local $raw_w f32) (local $raw_h f32) (local $pref_w f32) (local $pref_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    i32.const 121324 f32.const 44 f32.store
    i32.const 121328 f32.const 0 f32.store
    i32.const 121332 f32.const 16 f32.store
    i32.const 121336 f32.const 0 f32.store
    local.get $constraints i32.const 121324 i32.const 121340 call $er_ui_layout_constraints_inner drop
    i32.const 121364 f32.const 16 f32.store
    i32.const 121368 local.get $title_ptr local.get $title_len f32.const 16 call $primitives_average_width f32.store
    i32.const 121372 i32.const 2 i32.store
    local.get $title_ptr local.get $title_len i32.const 121340 i32.const 121364 i32.const 121376 call $er_ui_text_component_measure_value drop
    i32.const 121384 f32.load local.set $title_w
    i32.const 121388 f32.load local.set $title_h
    i32.const 121364 f32.const 16 f32.store
    i32.const 121368 local.get $detail_ptr local.get $detail_len f32.const 16 call $primitives_average_width f32.store
    i32.const 121372 i32.const 2 i32.store
    local.get $detail_ptr local.get $detail_len i32.const 121340 i32.const 121364 i32.const 121376 call $er_ui_text_component_measure_value drop
    i32.const 121384 f32.load local.set $detail_w
    i32.const 121388 f32.load local.set $detail_h
    f32.const 160 local.get $title_w local.get $detail_w call $max_f32 f32.const 60 f32.add call $max_f32 local.set $raw_w
    f32.const 24
    f32.const 16
    local.get $title_h f32.const 2 f32.add local.get $detail_h f32.add
    call $max_f32
    f32.add
    local.set $raw_h
    local.get $raw_w local.get $raw_h local.get $constraints i32.const 121408 call $er_ui_primitives_constrain_preferred_size drop
    i32.const 121408 f32.load local.set $pref_w
    i32.const 121412 f32.load local.set $pref_h
    f32.const 160 local.get $pref_w call $min_f32
    f32.const 48 local.get $pref_h call $min_f32
    local.get $pref_w local.get $pref_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit
    local.get $pref_h
    local.get $out
    call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)
  (func $er_ui_tabs_item_count  (result i32) i32.const 2)
  (func $er_ui_tabs_list_padding  (result f32) f32.const 3)
  (func $er_ui_tabs_list_radius  (result f32) f32.const 8)
  (func $er_ui_tabs_gap  (result f32) f32.const 8)
  (func $er_ui_tabs_panel_padding  (result f32) f32.const 10)
  (func $er_ui_tabs_trigger_padding  (result f32) f32.const 8)
  (func $er_ui_tabs_panel_max_lines  (result i32) i32.const 2)

  (func $er_ui_tabs_active_index  (param $active i32) (param $default_active i32) (result i32)
    local.get $active i32.const -1 i32.eq
    if (result i32)
      local.get $default_active
    else
      local.get $active
    end
    i32.const 2
    call $er_ui_list_clamped_index)

  (func $er_ui_tabs_indexed_id  (param $id i32) (param $active i32) (param $default_active i32) (result i32)
    local.get $id i32.const 2 i32.mul
    local.get $active local.get $default_active call $er_ui_tabs_active_index
    i32.add)

  (func $er_ui_tabs_list_bounds  (param $bounds i32) (param $out i32) (result i32)
    (local $height f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 12 i32.add f32.load f32.const 38 call $min_f32 local.set $height
    local.get $out
    local.get $bounds f32.load
    local.get $bounds i32.const 4 i32.add f32.load
    local.get $bounds i32.const 8 i32.add f32.load
    local.get $height
    call $rect_store)

  (func $er_ui_tabs_panel_bounds  (param $bounds i32) (param $list i32) (param $out i32) (result i32)
    (local $y f32)
    local.get $bounds i32.eqz local.get $list i32.eqz i32.or local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $list i32.const 4 i32.add f32.load local.get $list i32.const 12 i32.add f32.load f32.add f32.const 8 f32.add local.set $y
    local.get $out
    local.get $bounds f32.load
    local.get $y
    local.get $bounds i32.const 8 i32.add f32.load
    local.get $bounds i32.const 4 i32.add f32.load local.get $bounds i32.const 12 i32.add f32.load f32.add local.get $y f32.sub f32.const 1 call $max_f32
    call $rect_store)

  (func $er_ui_tabs_trigger_bounds  (param $list i32) (param $index i32) (param $out i32) (result i32)
    (local $content_x f32) (local $content_y f32) (local $content_w f32) (local $content_h f32) (local $segment_w f32)
    local.get $list i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $list f32.load f32.const 3 f32.add local.set $content_x
    local.get $list i32.const 4 i32.add f32.load f32.const 3 f32.add local.set $content_y
    local.get $list i32.const 8 i32.add f32.load f32.const 6 f32.sub f32.const 0 call $max_f32 local.set $content_w
    local.get $list i32.const 12 i32.add f32.load f32.const 6 f32.sub f32.const 0 call $max_f32 local.set $content_h
    local.get $content_w f32.const 2 f32.div f32.const 1 call $max_f32 local.set $segment_w
    local.get $out
    local.get $content_x local.get $index f32.convert_i32_u local.get $segment_w f32.mul f32.add
    local.get $content_y
    local.get $segment_w
    local.get $content_h
    call $rect_store)

  (func $er_ui_tabs_trigger_text_bounds  (param $list i32) (param $index i32) (param $out i32) (result i32)
    local.get $list i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $list local.get $index i32.const 121432 call $er_ui_tabs_trigger_bounds drop
    i32.const 121432 local.get $out call $er_ui_toggle_text_bounds)

  (func $er_ui_tabs_content_inset_10 (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $bounds f32.load f32.const 10 f32.add
    local.get $bounds i32.const 4 i32.add f32.load f32.const 10 f32.add
    local.get $bounds i32.const 8 i32.add f32.load f32.const 20 f32.sub f32.const 0 call $max_f32
    local.get $bounds i32.const 12 i32.add f32.load f32.const 20 f32.sub f32.const 0 call $max_f32
    call $rect_store)

  (func $er_ui_tabs_panel_content_bounds  (param $panel i32) (param $out i32) (result i32)
    local.get $panel local.get $out call $er_ui_tabs_content_inset_10)

  (func $er_ui_tabs_panel_text_bounds  (param $panel i32) (param $label_ptr i32) (param $label_len i32) (param $out i32) (result i32)
    (local $text_h f32)
    local.get $panel i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $panel i32.const 121448 call $er_ui_tabs_content_inset_10 drop
    i32.const 121448 i32.const 12 i32.add f32.load
    local.get $label_ptr local.get $label_len i32.const 121448 i32.const 8 i32.add f32.load f32.const 16 i32.const 2 call $er_ui_alert_measured_text_height
    call $min_f32
    local.set $text_h
    local.get $out
    i32.const 121448 f32.load
    i32.const 121448 i32.const 4 i32.add f32.load i32.const 121448 i32.const 12 i32.add f32.load local.get $text_h f32.sub f32.const 0.5 f32.mul f32.add
    i32.const 121448 i32.const 8 i32.add f32.load
    local.get $text_h
    call $rect_store)

  (func $er_ui_tabs_measure_two_segments (export "er_ui_tabs_measure_two_segments") (param $first_ptr i32) (param $first_len i32) (param $second_ptr i32) (param $second_len i32) (param $constraints i32) (param $out i32) (result i32)
    (local $item_w f32) (local $item_h f32) (local $pref_w f32) (local $pref_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    f32.const 0 local.set $item_w
    f32.const 38 local.set $item_h
    local.get $first_ptr local.get $first_len f32.const 16 call $er_ui_font_text_width f32.const 22 f32.add local.get $item_w call $max_f32 local.set $item_w
    local.get $second_ptr local.get $second_len f32.const 16 call $er_ui_font_text_width f32.const 22 f32.add local.get $item_w call $max_f32 local.set $item_w
    local.get $item_w f32.const 2 f32.mul
    local.get $item_h
    local.get $constraints
    i32.const 121464
    call $er_ui_primitives_constrain_preferred_size drop
    i32.const 121464 f32.load local.set $pref_w
    i32.const 121468 f32.load local.set $pref_h
    f32.const 2 local.get $pref_w call $min_f32
    f32.const 38 local.get $pref_h call $min_f32
    local.get $pref_w local.get $pref_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit
    local.get $pref_h
    local.get $out
    call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)

  (func $er_ui_tabs_measure  (param $first_ptr i32) (param $first_len i32) (param $second_ptr i32) (param $second_len i32) (param $active i32) (param $default_active i32) (param $constraints i32) (param $out i32) (result i32)
    (local $pref_w f32) (local $pref_h f32) (local $min_w f32) (local $min_h f32) (local $max_w f32) (local $max_h f32) (local $active_idx i32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $first_ptr local.get $first_len local.get $second_ptr local.get $second_len local.get $constraints i32.const 121480 call $er_ui_tabs_measure_two_segments drop
    f32.const 10 i32.const 121504 call $er_ui_layout_insets_uniform drop
    local.get $constraints i32.const 121504 i32.const 121520 call $er_ui_layout_constraints_inner drop
    local.get $active local.get $default_active call $er_ui_tabs_active_index local.set $active_idx
    i32.const 121548 f32.const 16 f32.store
    i32.const 121552 local.get $first_ptr local.get $first_len f32.const 16 call $primitives_average_width f32.store
    i32.const 121556 i32.const 2 i32.store
    local.get $active_idx i32.const 1 i32.eq
    if
      i32.const 121552 local.get $second_ptr local.get $second_len f32.const 16 call $primitives_average_width f32.store
      local.get $second_ptr local.get $second_len i32.const 121520 i32.const 121548 i32.const 121560 call $er_ui_text_component_measure_value drop
    else
      local.get $first_ptr local.get $first_len i32.const 121520 i32.const 121548 i32.const 121560 call $er_ui_text_component_measure_value drop
    end
    i32.const 121560 i32.const 121504 i32.const 121584 call $er_ui_layout_measurement_with_insets drop
    i32.const 121488 f32.load i32.const 121592 f32.load call $max_f32 local.set $pref_w
    i32.const 121492 f32.load f32.const 8 f32.add i32.const 121596 f32.load f32.add local.set $pref_h
    local.get $pref_w local.get $pref_h local.get $constraints i32.const 121616 call $er_ui_primitives_constrain_preferred_size drop
    i32.const 121616 f32.load local.set $pref_w
    i32.const 121620 f32.load local.set $pref_h
    i32.const 121480 f32.load i32.const 121584 f32.load call $max_f32 local.set $min_w
    i32.const 121484 f32.load f32.const 8 f32.add i32.const 121588 f32.load f32.add local.set $min_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit local.set $max_w
    local.get $pref_h i32.const 121500 f32.load f32.const 8 f32.add i32.const 121604 f32.load f32.add call $max_f32 local.set $max_h
    local.get $min_w local.get $min_h local.get $pref_w local.get $pref_h local.get $max_w local.get $max_h local.get $out call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)
  (func $er_ui_radio_item_count  (result i32) i32.const 2)
  (func $er_ui_radio_box_size  (result f32) f32.const 18)
  (func $er_ui_radio_text_gap  (result f32) f32.const 10)
  (func $er_ui_radio_dot_size  (result f32) f32.const 8)
  (func $er_ui_radio_option_gap  (result f32) f32.const 6)
  (func $er_ui_radio_label_max_lines  (result i32) i32.const 1)
  (func $er_ui_radio_option_height  (result f32) f32.const 18)

  (func $er_ui_radio_selected_index  (param $selected i32) (result i32)
    local.get $selected i32.const 2 call $er_ui_list_clamped_index)

  (func $er_ui_radio_indexed_id  (param $id i32) (param $selected i32) (result i32)
    local.get $id i32.const 2 i32.mul
    local.get $selected i32.const 2 call $er_ui_list_clamped_index
    i32.add)

  (func $er_ui_radio_hit_id  (param $id i32) (param $index i32) (result i32)
    local.get $id
    local.get $index i32.const 1 call $min_i32_u
    i32.add)

  (func $er_ui_radio_option_bounds  (param $bounds i32) (param $index i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $bounds f32.load
    local.get $bounds i32.const 4 i32.add f32.load local.get $index f32.convert_i32_u f32.const 24 f32.mul f32.add
    local.get $bounds i32.const 8 i32.add f32.load
    f32.const 18
    call $rect_store)

  (func $er_ui_radio_outer_bounds  (param $option_bounds i32) (param $out i32) (result i32)
    local.get $option_bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $option_bounds f32.load
    local.get $option_bounds i32.const 4 i32.add f32.load local.get $option_bounds i32.const 12 i32.add f32.load f32.const 18 f32.sub f32.const 0.5 f32.mul f32.add
    f32.const 18
    f32.const 18
    call $rect_store)

  (func $er_ui_radio_dot_bounds  (param $outer_bounds i32) (param $out i32) (result i32)
    local.get $outer_bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $outer_bounds f32.load local.get $outer_bounds i32.const 8 i32.add f32.load f32.const 8 f32.sub f32.const 0.5 f32.mul f32.add
    local.get $outer_bounds i32.const 4 i32.add f32.load local.get $outer_bounds i32.const 12 i32.add f32.load f32.const 8 f32.sub f32.const 0.5 f32.mul f32.add
    f32.const 8
    f32.const 8
    call $rect_store)

  (func $er_ui_radio_label_bounds  (param $option_bounds i32) (param $out i32) (result i32)
    (local $label_x f32)
    local.get $option_bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $option_bounds f32.load f32.const 28 f32.add local.set $label_x
    local.get $out
    local.get $label_x
    local.get $option_bounds i32.const 4 i32.add f32.load local.get $option_bounds i32.const 12 i32.add f32.load f32.const 16 f32.sub f32.const 0.5 f32.mul f32.add
    local.get $option_bounds f32.load local.get $option_bounds i32.const 8 i32.add f32.load f32.add local.get $label_x f32.sub f32.const 1 call $max_f32
    f32.const 16
    call $rect_store)

  (func $er_ui_radio_measure  (param $first_ptr i32) (param $first_len i32) (param $second_ptr i32) (param $second_len i32) (param $constraints i32) (param $out i32) (result i32)
    (local $label_w f32) (local $pref_w f32) (local $pref_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $first_ptr local.get $first_len f32.const 16 call $er_ui_font_text_width
    local.get $second_ptr local.get $second_len f32.const 16 call $er_ui_font_text_width
    call $max_f32
    local.set $label_w
    f32.const 28 local.get $label_w f32.add
    f32.const 42
    local.get $constraints
    i32.const 121640
    call $er_ui_primitives_constrain_preferred_size drop
    i32.const 121640 f32.load local.set $pref_w
    i32.const 121644 f32.load local.set $pref_h
    f32.const 29
    f32.const 18
    local.get $pref_w local.get $pref_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit
    local.get $pref_h
    local.get $out
    call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)
  (func $er_ui_breadcrumb_icon_size  (result f32) f32.const 12)
  (func $er_ui_breadcrumb_separator_gap  (result f32) f32.const 6)
  (func $er_ui_breadcrumb_vertical_padding  (result f32) f32.const 8)
  (func $er_ui_breadcrumb_label_max_lines  (result i32) i32.const 1)
  (func $er_ui_breadcrumb_middle_label_len  (result i32) i32.const 4)

  (func $er_ui_breadcrumb_write_docs_label (result i32)
    i32.const 121680 i32.const 68 i32.store8
    i32.const 121681 i32.const 111 i32.store8
    i32.const 121682 i32.const 99 i32.store8
    i32.const 121683 i32.const 115 i32.store8
    i32.const 121680)

  (func $er_ui_breadcrumb_separator_total_width  (result f32)
    f32.const 36)

  (func $er_ui_breadcrumb_hit_id  (param $id i32) (param $index i32) (result i32)
    local.get $id local.get $index i32.const 1 call $min_i32_u i32.add)

  (func $er_ui_breadcrumb_label_width (param $ptr i32) (param $len i32) (result f32)
    local.get $ptr local.get $len f32.const 16 call $er_ui_font_text_width)

  (func $er_ui_breadcrumb_allocated_widths (param $first_ptr i32) (param $first_len i32) (param $current_ptr i32) (param $current_len i32) (param $bounds i32) (param $out i32) (result i32)
    (local $first_w f32) (local $middle_w f32) (local $current_w f32) (local $available f32) (local $natural f32) (local $scale f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $first_ptr local.get $first_len call $er_ui_breadcrumb_label_width local.set $first_w
    call $er_ui_breadcrumb_write_docs_label i32.const 4 call $er_ui_breadcrumb_label_width local.set $middle_w
    local.get $current_ptr local.get $current_len call $er_ui_breadcrumb_label_width local.set $current_w
    f32.const 3 local.get $bounds i32.const 8 i32.add f32.load f32.const 36 f32.sub call $max_f32 local.set $available
    f32.const 1 local.get $first_w local.get $middle_w f32.add local.get $current_w f32.add call $max_f32 local.set $natural
    f32.const 1 local.get $available local.get $natural f32.div call $min_f32 local.set $scale
    local.get $out local.get $first_w local.get $scale f32.mul f32.const 1 call $max_f32 f32.store
    local.get $out i32.const 4 i32.add local.get $middle_w local.get $scale f32.mul f32.const 1 call $max_f32 f32.store
    local.get $out i32.const 8 i32.add local.get $current_w local.get $scale f32.mul f32.const 1 call $max_f32 f32.store
    i32.const 1)

  (func  (param $first_ptr i32) (param $first_len i32) (param $current_ptr i32) (param $current_len i32) (param $bounds i32) (param $out i32) (result i32)
    local.get $first_ptr local.get $first_len local.get $current_ptr local.get $current_len local.get $bounds local.get $out call $er_ui_breadcrumb_allocated_widths)

  (func $er_ui_breadcrumb_item_bounds  (param $first_ptr i32) (param $first_len i32) (param $current_ptr i32) (param $current_len i32) (param $bounds i32) (param $index i32) (param $out i32) (result i32)
    (local $first_w f32) (local $middle_w f32) (local $middle_x f32) (local $current_x f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $first_ptr local.get $first_len local.get $current_ptr local.get $current_len local.get $bounds i32.const 121688 call $er_ui_breadcrumb_allocated_widths drop
    i32.const 121688 f32.load local.set $first_w
    i32.const 121692 f32.load local.set $middle_w
    local.get $bounds f32.load local.get $first_w f32.add f32.const 18 f32.add local.set $middle_x
    local.get $middle_x local.get $middle_w f32.add f32.const 18 f32.add local.set $current_x
    local.get $index i32.eqz
    if
      local.get $out local.get $bounds f32.load local.get $bounds i32.const 4 i32.add f32.load local.get $first_w local.get $bounds i32.const 12 i32.add f32.load call $rect_store return
    end
    local.get $index i32.const 1 i32.eq
    if
      local.get $out local.get $middle_x local.get $bounds i32.const 4 i32.add f32.load local.get $middle_w local.get $bounds i32.const 12 i32.add f32.load call $rect_store return
    end
    local.get $out
    local.get $current_x
    local.get $bounds i32.const 4 i32.add f32.load
    local.get $bounds f32.load local.get $bounds i32.const 8 i32.add f32.load f32.add local.get $current_x f32.sub f32.const 1 call $max_f32
    local.get $bounds i32.const 12 i32.add f32.load
    call $rect_store)

  (func $er_ui_breadcrumb_separator_bounds  (param $first_ptr i32) (param $first_len i32) (param $current_ptr i32) (param $current_len i32) (param $bounds i32) (param $index i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $first_ptr local.get $first_len local.get $current_ptr local.get $current_len local.get $bounds local.get $index i32.const 121704 call $er_ui_breadcrumb_item_bounds drop
    local.get $out
    i32.const 121704 f32.load i32.const 121704 i32.const 8 i32.add f32.load f32.add f32.const 3 f32.add
    local.get $bounds i32.const 4 i32.add f32.load local.get $bounds i32.const 12 i32.add f32.load f32.const 12 f32.sub f32.const 0.5 f32.mul f32.add
    f32.const 12
    f32.const 12
    call $rect_store)

  (func $er_ui_breadcrumb_label_bounds  (param $item_bounds i32) (param $out i32) (result i32)
    local.get $item_bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $item_bounds f32.load
    local.get $item_bounds i32.const 4 i32.add f32.load local.get $item_bounds i32.const 12 i32.add f32.load f32.const 16 f32.sub f32.const 0.5 f32.mul f32.add
    local.get $item_bounds i32.const 8 i32.add f32.load
    f32.const 16
    call $rect_store)

  (func $er_ui_breadcrumb_measure  (param $first_ptr i32) (param $first_len i32) (param $current_ptr i32) (param $current_len i32) (param $constraints i32) (param $out i32) (result i32)
    (local $first_w f32) (local $middle_w f32) (local $current_w f32) (local $pref_w f32) (local $pref_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $first_ptr local.get $first_len call $er_ui_breadcrumb_label_width local.set $first_w
    call $er_ui_breadcrumb_write_docs_label i32.const 4 call $er_ui_breadcrumb_label_width local.set $middle_w
    local.get $current_ptr local.get $current_len call $er_ui_breadcrumb_label_width local.set $current_w
    local.get $first_w local.get $middle_w f32.add local.get $current_w f32.add f32.const 36 f32.add
    f32.const 32
    local.get $constraints
    i32.const 121724
    call $er_ui_primitives_constrain_preferred_size drop
    i32.const 121724 f32.load local.set $pref_w
    i32.const 121728 f32.load local.set $pref_h
    f32.const 39
    f32.const 32
    local.get $pref_w local.get $pref_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit
    local.get $pref_h
    local.get $out
    call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)

)
