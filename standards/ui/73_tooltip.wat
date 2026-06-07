(func (export "er_ui_tooltip_trigger_y") (result f32) f32.const 8)
  (func (export "er_ui_tooltip_trigger_w") (result f32) f32.const 80)
  (func (export "er_ui_tooltip_trigger_h") (result f32) f32.const 28)
  (func (export "er_ui_tooltip_gap") (result f32) f32.const 10)
  (func (export "er_ui_tooltip_content_y") (result f32) f32.const 7)
  (func (export "er_ui_tooltip_content_h") (result f32) f32.const 24)
  (func (export "er_ui_tooltip_radius") (result f32) f32.const 6)
  (func (export "er_ui_tooltip_padding") (result f32) f32.const 8)
  (func (export "er_ui_tooltip_text_h") (result f32) f32.const 12)
  (func (export "er_ui_tooltip_text_max_lines") (result i32) i32.const 2)
  (func (export "er_ui_tooltip_trigger_max_lines") (result i32) i32.const 2)
  (func (export "er_ui_tooltip_min_width") (result f32) f32.const 160)
  (func (export "er_ui_tooltip_min_height") (result f32) f32.const 44)

  (func (export "er_ui_tooltip_trigger_bounds") (param $bounds i32) (param $out i32) (result i32)
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

  (func (export "er_ui_tooltip_content_bounds") (param $bounds i32) (param $content_ptr i32) (param $content_len i32) (param $out i32) (result i32)
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

  (func $er_ui_tooltip_content_inner_bounds (export "er_ui_tooltip_content_inner_bounds") (param $tip_bounds i32) (param $out i32) (result i32)
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

  (func (export "er_ui_tooltip_text_bounds") (param $tip_bounds i32) (param $content_ptr i32) (param $content_len i32) (param $out i32) (result i32)
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

  (func (export "er_ui_tooltip_measure") (param $trigger_ptr i32) (param $trigger_len i32) (param $content_ptr i32) (param $content_len i32) (param $constraints i32) (param $out i32) (result i32)
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
