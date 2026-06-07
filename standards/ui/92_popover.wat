(func (export "er_ui_popover_trigger_y") (result f32) f32.const 6)
  (func (export "er_ui_popover_trigger_w") (result f32) f32.const 64)
  (func (export "er_ui_popover_trigger_h") (result f32) f32.const 30)
  (func (export "er_ui_popover_gap") (result f32) f32.const 10)
  (func (export "er_ui_popover_radius") (result f32) f32.const 8)
  (func (export "er_ui_popover_padding") (result f32) f32.const 10)
  (func (export "er_ui_popover_label_max_lines") (result i32) i32.const 1)

  (func $er_ui_popover_layout (param $out i32) (result i32)
    local.get $out f32.const 6 f32.const 64 f32.const 30 f32.const 10 call $er_ui_primitives_side_panel_layout)

  (func (export "er_ui_popover_trigger_id") (param $id i32) (result i32)
    local.get $id)

  (func (export "er_ui_popover_content_id") (param $id i32) (result i32)
    local.get $id i32.const 1 i32.add)

  (func $er_ui_popover_trigger_bounds (export "er_ui_popover_trigger_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    i32.const 122760 call $er_ui_popover_layout drop
    local.get $bounds i32.const 122760 local.get $out call $er_ui_primitives_side_panel_trigger_bounds)

  (func $er_ui_popover_content_bounds (export "er_ui_popover_content_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    i32.const 122760 call $er_ui_popover_layout drop
    local.get $bounds i32.const 122760 local.get $out call $er_ui_primitives_side_panel_content_bounds)

  (func (export "er_ui_popover_trigger_text_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 122776 call $er_ui_popover_trigger_bounds drop
    i32.const 122776 f32.const 12 i32.const 122792 call $er_ui_primitives_content_inset drop
    i32.const 122792 local.get $out f32.const 16 call $er_ui_rect_with_height_centered)

  (func (export "er_ui_popover_content_text_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 122808 call $er_ui_popover_content_bounds drop
    i32.const 122808 f32.const 10 i32.const 122824 call $er_ui_primitives_content_inset drop
    i32.const 122824 local.get $out f32.const 16 call $er_ui_rect_with_height_centered)

  (func (export "er_ui_popover_measure") (param $trigger_ptr i32) (param $trigger_len i32) (param $content_ptr i32) (param $content_len i32) (param $constraints i32) (param $out i32) (result i32)
    (local $trigger_w f32) (local $content_w f32) (local $content_h f32) (local $pref_w f32) (local $pref_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $trigger_ptr local.get $trigger_len f32.const 16 call $er_ui_font_text_width local.set $trigger_w
    i32.const 122840 f32.const 0 f32.store
    i32.const 122844 local.get $trigger_w f32.const 44 f32.add f32.store
    i32.const 122848 f32.const 10 f32.store
    i32.const 122852 f32.const 0 f32.store
    local.get $constraints i32.const 122840 i32.const 122856 call $er_ui_layout_constraints_inner drop
    i32.const 122880 f32.const 16 f32.store
    i32.const 122884 local.get $content_ptr local.get $content_len f32.const 16 call $primitives_average_width f32.store
    i32.const 122888 i32.const 1 i32.store
    local.get $content_ptr local.get $content_len i32.const 122856 i32.const 122880 i32.const 122892 call $er_ui_text_component_measure_value drop
    i32.const 122900 f32.load local.set $content_w
    i32.const 122904 f32.load local.set $content_h
    local.get $trigger_w local.get $content_w f32.add f32.const 54 f32.add local.set $pref_w
    f32.const 46 local.get $content_h f32.const 20 f32.add call $max_f32 local.set $pref_h
    local.get $pref_w local.get $pref_h local.get $constraints i32.const 122916 call $er_ui_primitives_constrain_preferred_size drop
    i32.const 122916 f32.load local.set $pref_w
    i32.const 122920 f32.load local.set $pref_h
    f32.const 12 local.get $pref_w call $min_f32
    f32.const 36 local.get $pref_h call $min_f32
    local.get $pref_w local.get $pref_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit
    local.get $pref_h
    local.get $out
    call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)
