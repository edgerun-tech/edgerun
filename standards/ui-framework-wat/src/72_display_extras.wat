  (func (export "er_ui_separator_height") (result f32) f32.const 1)
  (func (export "er_ui_separator_min_width") (result f32) f32.const 1)
  (func (export "er_ui_skeleton_min_width") (result f32) f32.const 96)
  (func (export "er_ui_skeleton_height") (result f32) f32.const 20)
  (func (export "er_ui_skeleton_alpha") (result i32) i32.const 32)
  (func (export "er_ui_skeleton_radius") (result f32) f32.const 6)
  (func (export "er_ui_spinner_size") (result f32) f32.const 28)
  (func (export "er_ui_spinner_slice_inset") (result f32) f32.const 3)
  (func (export "er_ui_spinner_start_turn") (result f32) f32.const 0.08)
  (func (export "er_ui_spinner_end_turn") (result f32) f32.const 0.78)
  (func (export "er_ui_aspect_ratio_min_width") (result f32) f32.const 160)
  (func (export "er_ui_kbd_height") (result f32) f32.const 24)
  (func (export "er_ui_kbd_text_height") (result f32) f32.const 12)
  (func (export "er_ui_kbd_label_max_lines") (result i32) i32.const 1)
  (func (export "er_ui_kbd_label_padding") (result f32) f32.const 8)
  (func (export "er_ui_kbd_min_width") (result f32) f32.const 24)
  (func (export "er_ui_avatar_size") (result f32) f32.const 40)
  (func (export "er_ui_avatar_text_height") (result f32) f32.const 14)
  (func (export "er_ui_avatar_label_inset") (result f32) f32.const 6)
  (func (export "er_ui_label_height") (result f32) f32.const 16)
  (func (export "er_ui_label_min_width") (result f32) f32.const 24)
  (func (export "er_ui_label_max_lines") (result i32) i32.const 2)

  (func $er_ui_measure_intrinsic (param $w f32) (param $h f32) (param $constraints i32) (param $out i32) (result i32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $w local.get $h local.get $constraints i32.const 120856 call $er_ui_primitives_constrain_preferred_size drop
    i32.const 120856 f32.load
    i32.const 120860 f32.load
    i32.const 120856 f32.load
    i32.const 120860 f32.load
    i32.const 120856 f32.load
    i32.const 120860 f32.load
    local.get $out
    call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)

  (func $er_ui_measure_flexible_line (param $min_w f32) (param $height f32) (param $constraints i32) (param $out i32) (result i32)
    (local $pref_w f32) (local $pref_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $min_w local.get $height local.get $constraints i32.const 120856 call $er_ui_primitives_constrain_preferred_size drop
    i32.const 120856 f32.load local.set $pref_w
    i32.const 120860 f32.load local.set $pref_h
    f32.const 1 local.get $pref_w call $min_f32
    local.get $height local.get $pref_h call $min_f32
    local.get $pref_w local.get $pref_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit
    local.get $pref_h
    local.get $out
    call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)

  (func (export "er_ui_separator_measure") (param $constraints i32) (param $out i32) (result i32)
    f32.const 1 f32.const 1 local.get $constraints local.get $out call $er_ui_measure_flexible_line)

  (func (export "er_ui_skeleton_measure") (param $constraints i32) (param $out i32) (result i32)
    f32.const 96 f32.const 20 local.get $constraints local.get $out call $er_ui_measure_flexible_line)

  (func (export "er_ui_spinner_measure") (param $constraints i32) (param $out i32) (result i32)
    f32.const 28 f32.const 28 local.get $constraints local.get $out call $er_ui_measure_intrinsic)

  (func $er_ui_aspect_ratio_intrinsic_size_impl (param $ratio_w i32) (param $ratio_h i32) (param $out i32) (result i32)
    (local $safe_w f32) (local $safe_h f32)
    local.get $out i32.eqz
    if i32.const 0 return end
    local.get $ratio_w f32.convert_i32_u f32.const 1 call $max_f32 local.set $safe_w
    local.get $ratio_h f32.convert_i32_u f32.const 1 call $max_f32 local.set $safe_h
    local.get $out
    f32.const 160
    f32.const 160 local.get $safe_h f32.mul local.get $safe_w f32.div
    call $layout_store_size)

  (func (export "er_ui_aspect_ratio_intrinsic_size") (param $ratio_w i32) (param $ratio_h i32) (param $out i32) (result i32)
    local.get $ratio_w local.get $ratio_h local.get $out call $er_ui_aspect_ratio_intrinsic_size_impl)

  (func (export "er_ui_aspect_ratio_measure") (param $ratio_w i32) (param $ratio_h i32) (param $constraints i32) (param $out i32) (result i32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $ratio_w local.get $ratio_h i32.const 120856 call $er_ui_aspect_ratio_intrinsic_size_impl drop
    i32.const 120856 f32.load
    i32.const 120860 f32.load
    local.get $constraints
    local.get $out
    call $er_ui_measure_intrinsic)

  (func (export "er_ui_kbd_measure") (param $label_ptr i32) (param $label_len i32) (param $constraints i32) (param $out i32) (result i32)
    (local $pref_w f32) (local $pref_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    i32.const 120864 f32.const 8 f32.store
    i32.const 120868 f32.const 0 f32.store
    i32.const 120872 f32.const 8 f32.store
    i32.const 120876 f32.const 0 f32.store
    local.get $constraints i32.const 120864 i32.const 120880 call $er_ui_layout_constraints_inner drop
    i32.const 120908 f32.const 12 f32.store
    i32.const 120912 local.get $label_ptr local.get $label_len f32.const 12 call $primitives_average_width f32.store
    i32.const 120916 i32.const 1 i32.store
    local.get $label_ptr local.get $label_len i32.const 120880 i32.const 120908 i32.const 120920 call $er_ui_text_component_measure_value drop
    i32.const 120928 f32.load f32.const 16 f32.add f32.const 24 call $max_f32
    i32.const 120932 f32.load f32.const 8 f32.add f32.const 24 call $max_f32
    local.get $constraints
    i32.const 120944
    call $er_ui_primitives_constrain_preferred_size drop
    i32.const 120944 f32.load local.set $pref_w
    i32.const 120948 f32.load local.set $pref_h
    f32.const 24 local.get $pref_w call $min_f32
    f32.const 24 local.get $pref_h call $min_f32
    local.get $pref_w local.get $pref_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit
    local.get $pref_h
    local.get $out
    call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)

  (func (export "er_ui_avatar_measure") (param $constraints i32) (param $out i32) (result i32)
    f32.const 40 f32.const 40 local.get $constraints local.get $out call $er_ui_measure_intrinsic)

  (func (export "er_ui_label_measure") (param $text_ptr i32) (param $text_len i32) (param $constraints i32) (param $out i32) (result i32)
    (local $pref_w f32) (local $pref_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    i32.const 120908 f32.const 16 f32.store
    i32.const 120912 local.get $text_ptr local.get $text_len f32.const 16 call $primitives_average_width f32.store
    i32.const 120916 i32.const 2 i32.store
    local.get $text_ptr local.get $text_len local.get $constraints i32.const 120908 i32.const 120920 call $er_ui_text_component_measure_value drop
    i32.const 120928 f32.load i32.const 120932 f32.load local.get $constraints i32.const 120944 call $er_ui_primitives_constrain_preferred_size drop
    i32.const 120944 f32.load local.set $pref_w
    i32.const 120948 f32.load local.set $pref_h
    f32.const 24 local.get $pref_w call $min_f32
    f32.const 16 local.get $pref_h call $min_f32
    local.get $pref_w local.get $pref_h
    i32.const 120936 f32.load
    i32.const 120940 f32.load
    local.get $out
    call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)

  (func (export "er_ui_separator_line_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $bounds f32.load
    local.get $bounds i32.const 4 i32.add f32.load local.get $bounds i32.const 12 i32.add f32.load f32.const 1 f32.sub f32.const 0.5 f32.mul f32.add
    local.get $bounds i32.const 8 i32.add f32.load
    f32.const 1
    call $rect_store)

  (func $er_ui_centered_square_bounds (param $bounds i32) (param $max_size f32) (param $out i32) (result i32)
    (local $size f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $max_size
    f32.const 1 local.get $bounds i32.const 8 i32.add f32.load local.get $bounds i32.const 12 i32.add f32.load call $min_f32 call $max_f32
    call $min_f32
    local.set $size
    local.get $out
    local.get $bounds f32.load local.get $bounds i32.const 8 i32.add f32.load local.get $size f32.sub f32.const 0.5 f32.mul f32.add
    local.get $bounds i32.const 4 i32.add f32.load local.get $bounds i32.const 12 i32.add f32.load local.get $size f32.sub f32.const 0.5 f32.mul f32.add
    local.get $size
    local.get $size
    call $rect_store)

  (func (export "er_ui_spinner_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds f32.const 28 local.get $out call $er_ui_centered_square_bounds)

  (func (export "er_ui_avatar_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds f32.const 40 local.get $out call $er_ui_centered_square_bounds)

  (func (export "er_ui_kbd_label_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $bounds f32.load f32.const 8 f32.add
    local.get $bounds i32.const 4 i32.add f32.load local.get $bounds i32.const 12 i32.add f32.load f32.const 12 f32.sub f32.const 0.5 f32.mul f32.add
    local.get $bounds i32.const 8 i32.add f32.load f32.const 16 f32.sub f32.const 0 call $max_f32
    f32.const 12
    call $rect_store)

  (func (export "er_ui_kbd_bounds") (param $bounds i32) (param $out i32) (result i32)
    (local $height f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    f32.const 24
    f32.const 1 local.get $bounds i32.const 12 i32.add f32.load call $max_f32
    call $min_f32
    local.set $height
    local.get $out
    local.get $bounds f32.load
    local.get $bounds i32.const 4 i32.add f32.load local.get $bounds i32.const 12 i32.add f32.load local.get $height f32.sub f32.const 0.5 f32.mul f32.add
    local.get $bounds i32.const 8 i32.add f32.load
    local.get $height
    call $rect_store)

  (func (export "er_ui_avatar_label_bounds") (param $avatar_bounds i32) (param $out i32) (result i32)
    local.get $avatar_bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $avatar_bounds f32.load f32.const 6 f32.add
    local.get $avatar_bounds i32.const 4 i32.add f32.load local.get $avatar_bounds i32.const 12 i32.add f32.load f32.const 14 f32.sub f32.const 0.5 f32.mul f32.add
    local.get $avatar_bounds i32.const 8 i32.add f32.load f32.const 12 f32.sub f32.const 0 call $max_f32
    f32.const 14
    call $rect_store)
