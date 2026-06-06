  (func (export "er_ui_input_group_separator_height") (result f32) f32.const 1)
  (func (export "er_ui_input_group_addon_min_w") (result f32) f32.const 42)
  (func (export "er_ui_input_group_addon_max_w") (result f32) f32.const 96)
  (func (export "er_ui_input_group_addon_padding") (result f32) f32.const 10)
  (func (export "er_ui_input_group_control_gap") (result f32) f32.const 8)
  (func (export "er_ui_input_group_separator_inset") (result f32) f32.const 8)
  (func (export "er_ui_input_group_text_max_lines") (result i32) i32.const 2)
  (func (export "er_ui_input_group_min_width") (result f32) f32.const 140)
  (func (export "er_ui_input_group_min_height") (result f32) f32.const 36)

  (func $er_ui_input_group_addon_width (export "er_ui_input_group_addon_width") (param $addon_ptr i32) (param $addon_len i32) (result f32)
    local.get $addon_ptr local.get $addon_len f32.const 16 call $er_ui_font_text_width
    f32.const 20
    f32.add
    f32.const 42
    call $max_f32
    f32.const 96
    call $min_f32)

  (func $er_ui_input_group_addon_bounds (export "er_ui_input_group_addon_bounds") (param $bounds i32) (param $addon_ptr i32) (param $addon_len i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $bounds f32.load
    local.get $bounds i32.const 4 i32.add f32.load
    local.get $addon_ptr local.get $addon_len call $er_ui_input_group_addon_width
    local.get $bounds i32.const 12 i32.add f32.load
    call $rect_store)

  (func $er_ui_input_group_control_bounds (export "er_ui_input_group_control_bounds") (param $bounds i32) (param $addon_ptr i32) (param $addon_len i32) (param $out i32) (result i32)
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

  (func (export "er_ui_input_group_separator_bounds") (param $bounds i32) (param $addon_ptr i32) (param $addon_len i32) (param $out i32) (result i32)
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

  (func (export "er_ui_input_group_addon_text_bounds") (param $bounds i32) (param $addon_ptr i32) (param $addon_len i32) (param $out i32) (result i32)
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

  (func (export "er_ui_input_group_placeholder_text_bounds") (param $bounds i32) (param $addon_ptr i32) (param $addon_len i32) (param $placeholder_ptr i32) (param $placeholder_len i32) (param $out i32) (result i32)
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

  (func (export "er_ui_input_group_measure") (param $addon_ptr i32) (param $addon_len i32) (param $placeholder_ptr i32) (param $placeholder_len i32) (param $constraints i32) (param $out i32) (result i32)
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
