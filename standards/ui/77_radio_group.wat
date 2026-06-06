  (func (export "er_ui_radio_item_count") (result i32) i32.const 2)
  (func (export "er_ui_radio_box_size") (result f32) f32.const 18)
  (func (export "er_ui_radio_text_gap") (result f32) f32.const 10)
  (func (export "er_ui_radio_dot_size") (result f32) f32.const 8)
  (func (export "er_ui_radio_option_gap") (result f32) f32.const 6)
  (func (export "er_ui_radio_label_max_lines") (result i32) i32.const 1)
  (func (export "er_ui_radio_option_height") (result f32) f32.const 18)

  (func (export "er_ui_radio_selected_index") (param $selected i32) (result i32)
    local.get $selected i32.const 2 call $er_ui_list_clamped_index)

  (func (export "er_ui_radio_indexed_id") (param $id i32) (param $selected i32) (result i32)
    local.get $id i32.const 2 i32.mul
    local.get $selected i32.const 2 call $er_ui_list_clamped_index
    i32.add)

  (func (export "er_ui_radio_hit_id") (param $id i32) (param $index i32) (result i32)
    local.get $id
    local.get $index i32.const 1 call $layout_min_i32_u
    i32.add)

  (func (export "er_ui_radio_option_bounds") (param $bounds i32) (param $index i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $bounds f32.load
    local.get $bounds i32.const 4 i32.add f32.load local.get $index f32.convert_i32_u f32.const 24 f32.mul f32.add
    local.get $bounds i32.const 8 i32.add f32.load
    f32.const 18
    call $rect_store)

  (func (export "er_ui_radio_outer_bounds") (param $option_bounds i32) (param $out i32) (result i32)
    local.get $option_bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $option_bounds f32.load
    local.get $option_bounds i32.const 4 i32.add f32.load local.get $option_bounds i32.const 12 i32.add f32.load f32.const 18 f32.sub f32.const 0.5 f32.mul f32.add
    f32.const 18
    f32.const 18
    call $rect_store)

  (func (export "er_ui_radio_dot_bounds") (param $outer_bounds i32) (param $out i32) (result i32)
    local.get $outer_bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $outer_bounds f32.load local.get $outer_bounds i32.const 8 i32.add f32.load f32.const 8 f32.sub f32.const 0.5 f32.mul f32.add
    local.get $outer_bounds i32.const 4 i32.add f32.load local.get $outer_bounds i32.const 12 i32.add f32.load f32.const 8 f32.sub f32.const 0.5 f32.mul f32.add
    f32.const 8
    f32.const 8
    call $rect_store)

  (func (export "er_ui_radio_label_bounds") (param $option_bounds i32) (param $out i32) (result i32)
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

  (func (export "er_ui_radio_measure") (param $first_ptr i32) (param $first_len i32) (param $second_ptr i32) (param $second_len i32) (param $constraints i32) (param $out i32) (result i32)
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
