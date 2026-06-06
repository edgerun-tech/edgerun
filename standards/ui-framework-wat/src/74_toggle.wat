  (func (export "er_ui_toggle_text_padding") (result f32) f32.const 8)
  (func (export "er_ui_toggle_label_max_lines") (result i32) i32.const 1)
  (func (export "er_ui_toggle_group_item_count") (result i32) i32.const 3)
  (func (export "er_ui_toggle_group_id_stride") (result i32) i32.const 3)
  (func (export "er_ui_toggle_group_third_label_len") (result i32) i32.const 5)

  (func $er_ui_toggle_write_right_label (result i32)
    i32.const 121140 i32.const 82 i32.store8
    i32.const 121141 i32.const 105 i32.store8
    i32.const 121142 i32.const 103 i32.store8
    i32.const 121143 i32.const 104 i32.store8
    i32.const 121144 i32.const 116 i32.store8
    i32.const 121140)

  (func $er_ui_toggle_unconstrained_nowrap (param $out i32) (result i32)
    local.get $out i32.eqz
    if i32.const 0 return end
    local.get $out i32.const 0 i32.store
    local.get $out i32.const 4 i32.add f32.const 0 f32.store
    local.get $out i32.const 8 i32.add i32.const 0 i32.store
    local.get $out i32.const 12 i32.add f32.const 0 f32.store
    local.get $out i32.const 16 i32.add i32.const 2 i32.store
    i32.const 1)

  (func $er_ui_toggle_measure_label (param $label_ptr i32) (param $label_len i32) (param $out i32) (result i32)
    local.get $out i32.eqz
    if i32.const 0 return end
    i32.const 121148 call $er_ui_toggle_unconstrained_nowrap drop
    i32.const 121172 f32.const 16 f32.store
    i32.const 121176 local.get $label_ptr local.get $label_len f32.const 16 call $primitives_average_width f32.store
    i32.const 121180 i32.const 1 i32.store
    local.get $label_ptr local.get $label_len i32.const 121148 i32.const 121172 local.get $out call $er_ui_text_component_measure_value)

  (func (export "er_ui_toggle_preferred_size") (param $label_ptr i32) (param $label_len i32) (param $out i32) (result i32)
    local.get $out i32.eqz
    if i32.const 0 return end
    local.get $label_ptr local.get $label_len i32.const 121184 call $er_ui_toggle_measure_label drop
    local.get $out
    i32.const 121192 f32.load f32.const 16 f32.add
    i32.const 121196 f32.load f32.const 16 f32.add
    call $layout_store_size)

  (func (export "er_ui_toggle_measure") (param $label_ptr i32) (param $label_len i32) (param $constraints i32) (param $out i32) (result i32)
    (local $pref_w f32) (local $pref_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $label_ptr local.get $label_len i32.const 121184 call $er_ui_toggle_measure_label drop
    i32.const 121192 f32.load f32.const 16 f32.add
    i32.const 121196 f32.load f32.const 16 f32.add
    local.get $constraints
    i32.const 121208
    call $er_ui_primitives_constrain_preferred_size drop
    i32.const 121208 f32.load local.set $pref_w
    i32.const 121212 f32.load local.set $pref_h
    f32.const 17
    f32.const 32
    local.get $pref_w local.get $pref_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit
    local.get $pref_h
    local.get $out
    call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)

  (func $er_ui_toggle_text_bounds (export "er_ui_toggle_text_bounds") (param $bounds i32) (param $out i32) (result i32)
    (local $pad f32) (local $inner_h f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    f32.const 8
    local.get $bounds i32.const 8 i32.add f32.load
    local.get $bounds i32.const 12 i32.add f32.load
    call $min_f32
    f32.const 0.5
    f32.mul
    call $min_f32
    local.set $pad
    local.get $bounds i32.const 12 i32.add f32.load local.get $pad f32.const 2 f32.mul f32.sub f32.const 0 call $max_f32 local.set $inner_h
    local.get $out
    local.get $bounds f32.load local.get $pad f32.add
    local.get $bounds i32.const 4 i32.add f32.load local.get $pad f32.add local.get $inner_h f32.const 16 f32.sub f32.const 0.5 f32.mul f32.add
    local.get $bounds i32.const 8 i32.add f32.load local.get $pad f32.const 2 f32.mul f32.sub f32.const 0 call $max_f32
    f32.const 16
    call $rect_store)

  (func (export "er_ui_toggle_group_active_index") (param $active i32) (result i32)
    local.get $active i32.const 3 call $er_ui_list_clamped_index)

  (func (export "er_ui_toggle_group_indexed_id") (param $id i32) (param $active i32) (result i32)
    local.get $id i32.const 3 i32.mul
    local.get $active i32.const 3 call $er_ui_list_clamped_index
    i32.add)

  (func (export "er_ui_toggle_group_item_bounds") (param $bounds i32) (param $index i32) (param $out i32) (result i32)
    local.get $bounds local.get $index i32.const 3 f32.const 0 local.get $out call $er_ui_list_equal_segment_bounds_gap)

  (func (export "er_ui_toggle_group_item_text_bounds") (param $bounds i32) (param $index i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds local.get $index i32.const 3 f32.const 0 i32.const 121224 call $er_ui_list_equal_segment_bounds_gap drop
    i32.const 121224 local.get $out call $er_ui_toggle_text_bounds)

  (func $er_ui_toggle_group_label_width (param $label_ptr i32) (param $label_len i32) (result f32)
    local.get $label_ptr local.get $label_len f32.const 16 call $er_ui_font_text_width
    f32.const 16
    f32.add)

  (func (export "er_ui_toggle_group_measure") (param $first_ptr i32) (param $first_len i32) (param $second_ptr i32) (param $second_len i32) (param $constraints i32) (param $out i32) (result i32)
    (local $item_w f32) (local $item_h f32) (local $pref_w f32) (local $pref_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    f32.const 0 local.set $item_w
    f32.const 32 local.set $item_h
    local.get $first_ptr local.get $first_len call $er_ui_toggle_group_label_width local.get $item_w call $max_f32 local.set $item_w
    local.get $second_ptr local.get $second_len call $er_ui_toggle_group_label_width local.get $item_w call $max_f32 local.set $item_w
    call $er_ui_toggle_write_right_label i32.const 5 call $er_ui_toggle_group_label_width local.get $item_w call $max_f32 local.set $item_w
    local.get $item_w f32.const 3 f32.mul
    local.get $item_h
    local.get $constraints
    i32.const 121208
    call $er_ui_primitives_constrain_preferred_size drop
    i32.const 121208 f32.load local.set $pref_w
    i32.const 121212 f32.load local.set $pref_h
    f32.const 3 local.get $pref_w call $min_f32
    f32.const 32 local.get $pref_h call $min_f32
    local.get $pref_w local.get $pref_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit
    local.get $pref_h
    local.get $out
    call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)
