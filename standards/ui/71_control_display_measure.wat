(func (export "er_ui_checkbox_box_size") (result f32) f32.const 18)
  (func (export "er_ui_checkbox_min_width") (result f32) f32.const 96)

  (func (export "er_ui_checkbox_measure") (param $label_ptr i32) (param $label_len i32) (param $constraints i32) (param $out i32) (result i32)
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

  (func (export "er_ui_switch_width") (result f32) f32.const 36)
  (func (export "er_ui_switch_height") (result f32) f32.const 20)
  (func (export "er_ui_switch_min_width") (result f32) f32.const 112)

  (func (export "er_ui_switch_measure") (param $label_ptr i32) (param $label_len i32) (param $constraints i32) (param $out i32) (result i32)
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

  (func (export "er_ui_slider_thumb_size") (result f32) f32.const 12)
  (func (export "er_ui_slider_min_width") (result f32) f32.const 120)
  (func (export "er_ui_slider_min_height") (result f32) f32.const 32)

  (func (export "er_ui_slider_measure") (param $label_ptr i32) (param $label_len i32) (param $constraints i32) (param $out i32) (result i32)
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

  (func (export "er_ui_progress_height") (result f32) f32.const 8)
  (func (export "er_ui_progress_min_width") (result f32) f32.const 96)

  (func (export "er_ui_progress_measure") (param $constraints i32) (param $out i32) (result i32)
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

  (func (export "er_ui_progress_track_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $bounds f32.load
    local.get $bounds i32.const 4 i32.add f32.load local.get $bounds i32.const 12 i32.add f32.load f32.const 8 f32.sub f32.const 0.5 f32.mul f32.add
    local.get $bounds i32.const 8 i32.add f32.load
    f32.const 8
    call $rect_store)

  (func (export "er_ui_progress_fill_bounds") (param $track i32) (param $value f32) (param $out i32) (result i32)
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

  (func (export "er_ui_aspect_ratio_frame_bounds") (param $bounds i32) (param $ratio_w i32) (param $ratio_h i32) (param $out i32) (result i32)
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
