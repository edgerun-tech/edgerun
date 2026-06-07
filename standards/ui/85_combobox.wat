(func (export "er_ui_combobox_input_h") (result f32) f32.const 36)
  (func (export "er_ui_combobox_popup_gap") (result f32) f32.const 6)
  (func (export "er_ui_combobox_popup_radius") (result f32) f32.const 8)
  (func (export "er_ui_combobox_popup_padding") (result f32) f32.const 4)
  (func (export "er_ui_combobox_icon_size") (result f32) f32.const 14)
  (func (export "er_ui_combobox_icon_space") (result f32) f32.const 22)
  (func (export "er_ui_combobox_option_padding") (result f32) f32.const 8)
  (func (export "er_ui_combobox_option_indicator_w") (result f32) f32.const 28)
  (func (export "er_ui_combobox_text_max_lines") (result i32) i32.const 1)

  (func (export "er_ui_combobox_hit_id") (param $id i32) (param $index i32) (result i32)
    local.get $id
    local.get $index i32.const 0 i32.const 1 call $clamp_i32
    i32.add)

  (func $er_ui_combobox_input_bounds (export "er_ui_combobox_input_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $bounds f32.load
    local.get $bounds i32.const 4 i32.add f32.load
    local.get $bounds i32.const 8 i32.add f32.load
    f32.const 36 local.get $bounds i32.const 12 i32.add f32.load call $min_f32
    call $rect_store)

  (func $er_ui_combobox_popup_bounds (export "er_ui_combobox_popup_bounds") (param $bounds i32) (param $out i32) (result i32)
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

  (func $er_ui_combobox_option_bounds (export "er_ui_combobox_option_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 122200 call $er_ui_combobox_popup_bounds drop
    local.get $out
    i32.const 122200 f32.load f32.const 4 f32.add
    i32.const 122200 i32.const 4 i32.add f32.load f32.const 4 f32.add
    i32.const 122200 i32.const 8 i32.add f32.load f32.const 8 f32.sub f32.const 1 call $max_f32
    i32.const 122200 i32.const 12 i32.add f32.load f32.const 8 f32.sub f32.const 1 call $max_f32
    call $rect_store)

  (func $er_ui_combobox_input_content_bounds (export "er_ui_combobox_input_content_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 122216 call $er_ui_combobox_input_bounds drop
    i32.const 122216 local.get $out call $er_ui_select_content_bounds)

  (func $er_ui_combobox_input_text_slot_bounds (export "er_ui_combobox_input_text_slot_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 122232 call $er_ui_combobox_input_content_bounds drop
    local.get $out
    i32.const 122232 f32.load
    i32.const 122232 i32.const 4 i32.add f32.load
    i32.const 122232 i32.const 8 i32.add f32.load f32.const 22 f32.sub f32.const 1 call $max_f32
    i32.const 122232 i32.const 12 i32.add f32.load
    call $rect_store)

  (func (export "er_ui_combobox_input_text_bounds") (param $bounds i32) (param $out i32) (result i32)
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

  (func (export "er_ui_combobox_input_icon_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 122264 call $er_ui_combobox_input_content_bounds drop
    local.get $out
    i32.const 122264 f32.load i32.const 122264 i32.const 8 i32.add f32.load f32.add f32.const 14 f32.sub
    i32.const 122264 i32.const 4 i32.add f32.load i32.const 122264 i32.const 12 i32.add f32.load f32.const 14 f32.sub f32.const 0.5 f32.mul f32.add
    f32.const 14
    f32.const 14
    call $rect_store)

  (func $er_ui_combobox_option_label_slot_bounds (export "er_ui_combobox_option_label_slot_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 122280 call $er_ui_combobox_option_bounds drop
    local.get $out
    i32.const 122280 f32.load
    i32.const 122280 i32.const 4 i32.add f32.load
    i32.const 122280 i32.const 8 i32.add f32.load f32.const 28 f32.sub f32.const 1 call $max_f32
    i32.const 122280 i32.const 12 i32.add f32.load
    call $rect_store)

  (func (export "er_ui_combobox_option_text_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 122296 call $er_ui_combobox_option_label_slot_bounds drop
    i32.const 122296 local.get $out call $er_ui_toggle_text_bounds)

  (func (export "er_ui_combobox_option_check_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 122312 call $er_ui_combobox_option_bounds drop
    local.get $out
    i32.const 122312 f32.load i32.const 122312 i32.const 8 i32.add f32.load f32.add f32.const 22 f32.sub
    i32.const 122312 i32.const 4 i32.add f32.load i32.const 122312 i32.const 12 i32.add f32.load f32.const 14 f32.sub f32.const 0.5 f32.mul f32.add
    f32.const 14
    f32.const 14
    call $rect_store)

  (func (export "er_ui_combobox_measure") (param $placeholder_ptr i32) (param $placeholder_len i32) (param $selected_ptr i32) (param $selected_len i32) (param $constraints i32) (param $out i32) (result i32)
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
