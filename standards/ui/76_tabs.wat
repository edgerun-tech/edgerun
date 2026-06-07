(func (export "er_ui_tabs_item_count") (result i32) i32.const 2)
  (func (export "er_ui_tabs_list_padding") (result f32) f32.const 3)
  (func (export "er_ui_tabs_list_radius") (result f32) f32.const 8)
  (func (export "er_ui_tabs_gap") (result f32) f32.const 8)
  (func (export "er_ui_tabs_panel_padding") (result f32) f32.const 10)
  (func (export "er_ui_tabs_trigger_padding") (result f32) f32.const 8)
  (func (export "er_ui_tabs_panel_max_lines") (result i32) i32.const 2)

  (func $er_ui_tabs_active_index (export "er_ui_tabs_active_index") (param $active i32) (param $default_active i32) (result i32)
    local.get $active i32.const -1 i32.eq
    if (result i32)
      local.get $default_active
    else
      local.get $active
    end
    i32.const 2
    call $er_ui_list_clamped_index)

  (func (export "er_ui_tabs_indexed_id") (param $id i32) (param $active i32) (param $default_active i32) (result i32)
    local.get $id i32.const 2 i32.mul
    local.get $active local.get $default_active call $er_ui_tabs_active_index
    i32.add)

  (func (export "er_ui_tabs_list_bounds") (param $bounds i32) (param $out i32) (result i32)
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

  (func (export "er_ui_tabs_panel_bounds") (param $bounds i32) (param $list i32) (param $out i32) (result i32)
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

  (func $er_ui_tabs_trigger_bounds (export "er_ui_tabs_trigger_bounds") (param $list i32) (param $index i32) (param $out i32) (result i32)
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

  (func (export "er_ui_tabs_trigger_text_bounds") (param $list i32) (param $index i32) (param $out i32) (result i32)
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

  (func (export "er_ui_tabs_panel_content_bounds") (param $panel i32) (param $out i32) (result i32)
    local.get $panel local.get $out call $er_ui_tabs_content_inset_10)

  (func (export "er_ui_tabs_panel_text_bounds") (param $panel i32) (param $label_ptr i32) (param $label_len i32) (param $out i32) (result i32)
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

  (func $er_ui_tabs_measure_two_segments (param $first_ptr i32) (param $first_len i32) (param $second_ptr i32) (param $second_len i32) (param $constraints i32) (param $out i32) (result i32)
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

  (func (export "er_ui_tabs_measure") (param $first_ptr i32) (param $first_len i32) (param $second_ptr i32) (param $second_len i32) (param $active i32) (param $default_active i32) (param $constraints i32) (param $out i32) (result i32)
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
