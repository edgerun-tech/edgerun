(func (export "er_ui_field_label_h") (result f32) f32.const 14)
  (func (export "er_ui_field_label_max_lines") (result i32) i32.const 2)
  (func (export "er_ui_field_gap") (result f32) f32.const 6)
  (func (export "er_ui_field_input_h") (result f32) f32.const 36)
  (func (export "er_ui_field_placeholder_max_lines") (result i32) i32.const 2)
  (func (export "er_ui_field_validation_gap") (result f32) f32.const 6)
  (func (export "er_ui_field_validation_line_h") (result f32) f32.const 12)
  (func (export "er_ui_field_validation_max_lines") (result i32) i32.const 2)
  (func (export "er_ui_field_min_width") (result f32) f32.const 120)
  (func (export "er_ui_field_min_height") (result f32) f32.const 48)

  (func $er_ui_field_measured_text_height (param $text_ptr i32) (param $text_len i32) (param $width f32) (param $line_height f32) (param $max_lines i32) (result f32)
    local.get $text_ptr local.get $text_len local.get $width local.get $line_height local.get $max_lines call $er_ui_alert_measured_text_height)

  (func $er_ui_field_label_height (export "er_ui_field_label_height") (param $bounds i32) (param $label_ptr i32) (param $label_len i32) (result f32)
    local.get $bounds i32.eqz
    if f32.const 0 return end
    local.get $bounds i32.const 12 i32.add f32.load
    local.get $label_ptr local.get $label_len local.get $bounds i32.const 8 i32.add f32.load f32.const 14 i32.const 2 call $er_ui_field_measured_text_height
    call $min_f32)

  (func (export "er_ui_field_label_bounds") (param $bounds i32) (param $label_ptr i32) (param $label_len i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $bounds f32.load
    local.get $bounds i32.const 4 i32.add f32.load
    local.get $bounds i32.const 8 i32.add f32.load
    local.get $bounds local.get $label_ptr local.get $label_len call $er_ui_field_label_height
    call $rect_store)

  (func (export "er_ui_field_input_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $bounds f32.load
    local.get $bounds i32.const 4 i32.add f32.load f32.const 20 f32.add
    local.get $bounds i32.const 8 i32.add f32.load
    local.get $bounds i32.const 12 i32.add f32.load f32.const 20 f32.sub f32.const 1 call $max_f32
    call $rect_store)

  (func (export "er_ui_field_input_bounds_for_label") (param $bounds i32) (param $label_ptr i32) (param $label_len i32) (param $out i32) (result i32)
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

  (func $er_ui_field_input_bounds_with_validation (export "er_ui_field_input_bounds_with_validation") (param $bounds i32) (param $label_ptr i32) (param $label_len i32) (param $out i32) (result i32)
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

  (func (export "er_ui_field_validation_bounds") (param $bounds i32) (param $label_ptr i32) (param $label_len i32) (param $message_ptr i32) (param $message_len i32) (param $out i32) (result i32)
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

  (func (export "er_ui_field_input_text_bounds") (param $input_bounds i32) (param $placeholder_ptr i32) (param $placeholder_len i32) (param $out i32) (result i32)
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

  (func $er_ui_field_input_measure (export "er_ui_field_input_measure") (param $placeholder_ptr i32) (param $placeholder_len i32) (param $constraints i32) (param $out i32) (result i32)
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

  (func (export "er_ui_field_measure") (param $label_ptr i32) (param $label_len i32) (param $placeholder_ptr i32) (param $placeholder_len i32) (param $validation_ptr i32) (param $validation_len i32) (param $has_validation i32) (param $constraints i32) (param $out i32) (result i32)
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
