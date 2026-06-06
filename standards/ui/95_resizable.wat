  (func (export "er_ui_resizable_handle_w") (result f32) f32.const 6)
  (func (export "er_ui_resizable_handle_radius") (result f32) f32.const 3)
  (func (export "er_ui_resizable_handle_hit_outset") (result f32) f32.const 6)
  (func (export "er_ui_resizable_min_width") (result f32) f32.const 96)
  (func (export "er_ui_resizable_min_height") (result f32) f32.const 36)

  (func $er_ui_resizable_clamped_ratio (export "er_ui_resizable_clamped_ratio") (param $ratio f32) (result f32)
    local.get $ratio f32.const 0 call $max_f32 f32.const 1 call $min_f32)

  (func $er_ui_resizable_handle_bounds (export "er_ui_resizable_handle_bounds") (param $bounds i32) (param $ratio f32) (param $out i32) (result i32)
    (local $center_x f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds f32.load
    local.get $bounds i32.const 8 i32.add f32.load
    local.get $ratio call $er_ui_resizable_clamped_ratio
    f32.mul
    f32.add
    local.set $center_x
    local.get $out
    local.get $center_x f32.const 3 f32.sub
    local.get $bounds i32.const 4 i32.add f32.load
    f32.const 6
    local.get $bounds i32.const 12 i32.add f32.load
    call $rect_store)

  (func (export "er_ui_resizable_handle_hit_bounds") (param $bounds i32) (param $ratio f32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds local.get $ratio i32.const 123060 call $er_ui_resizable_handle_bounds drop
    i32.const 123060 local.get $out f32.const -6 call $er_ui_rect_inset_uniform)

  (func (export "er_ui_resizable_left_bounds") (param $bounds i32) (param $ratio f32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds local.get $ratio i32.const 123076 call $er_ui_resizable_handle_bounds drop
    local.get $out
    local.get $bounds f32.load
    local.get $bounds i32.const 4 i32.add f32.load
    i32.const 123076 f32.load local.get $bounds f32.load f32.sub f32.const 1 call $max_f32
    local.get $bounds i32.const 12 i32.add f32.load
    call $rect_store)

  (func (export "er_ui_resizable_right_bounds") (param $bounds i32) (param $ratio f32) (param $out i32) (result i32)
    (local $right_x f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds local.get $ratio i32.const 123092 call $er_ui_resizable_handle_bounds drop
    i32.const 123092 f32.load i32.const 123092 i32.const 8 i32.add f32.load f32.add local.set $right_x
    local.get $out
    local.get $right_x
    local.get $bounds i32.const 4 i32.add f32.load
    local.get $bounds f32.load local.get $bounds i32.const 8 i32.add f32.load f32.add local.get $right_x f32.sub f32.const 1 call $max_f32
    local.get $bounds i32.const 12 i32.add f32.load
    call $rect_store)

  (func (export "er_ui_resizable_measure") (param $constraints i32) (param $out i32) (result i32)
    (local $pref_w f32) (local $pref_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    f32.const 96 f32.const 36 local.get $constraints i32.const 123108 call $er_ui_primitives_constrain_preferred_size drop
    i32.const 123108 f32.load local.set $pref_w
    i32.const 123112 f32.load local.set $pref_h
    f32.const 96 local.get $pref_w call $min_f32
    f32.const 36 local.get $pref_h call $min_f32
    local.get $pref_w local.get $pref_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit
    local.get $pref_h
    local.get $out
    call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)
