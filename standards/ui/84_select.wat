(func (export "er_ui_select_arrow_w") (result f32) f32.const 18)
  (func (export "er_ui_select_icon_size") (result f32) f32.const 14)
  (func (export "er_ui_select_label_max_lines") (result i32) i32.const 2)
  (func (export "er_ui_select_min_width") (result f32) f32.const 112)
  (func (export "er_ui_select_min_height") (result f32) f32.const 40)

  (func $er_ui_select_content_bounds (export "er_ui_select_content_bounds") (param $bounds i32) (param $out i32) (result i32)
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

  (func $er_ui_select_label_slot_bounds (export "er_ui_select_label_slot_bounds") (param $content_bounds i32) (param $out i32) (result i32)
    local.get $content_bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $content_bounds f32.load
    local.get $content_bounds i32.const 4 i32.add f32.load
    local.get $content_bounds i32.const 8 i32.add f32.load f32.const 18 f32.sub f32.const 1 call $max_f32
    local.get $content_bounds i32.const 12 i32.add f32.load
    call $rect_store)

  (func (export "er_ui_select_label_text_bounds") (param $bounds i32) (param $label_ptr i32) (param $label_len i32) (param $out i32) (result i32)
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

  (func (export "er_ui_select_arrow_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 122060 call $er_ui_select_content_bounds drop
    local.get $out
    i32.const 122060 f32.load i32.const 122060 i32.const 8 i32.add f32.load f32.add f32.const 14 f32.sub
    i32.const 122060 i32.const 4 i32.add f32.load i32.const 122060 i32.const 12 i32.add f32.load f32.const 14 f32.sub f32.const 0.5 f32.mul f32.add
    f32.const 14
    f32.const 14
    call $rect_store)

  (func (export "er_ui_select_measure") (param $label_ptr i32) (param $label_len i32) (param $constraints i32) (param $out i32) (result i32)
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
