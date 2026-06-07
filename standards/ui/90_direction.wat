(func (export "er_ui_direction_item_count") (result i32) i32.const 2)
  (func (export "er_ui_direction_ltr_label_len") (result i32) i32.const 3)
  (func (export "er_ui_direction_rtl_label_len") (result i32) i32.const 3)
  (func (export "er_ui_direction_item_h") (result f32) f32.const 20)
  (func (export "er_ui_direction_item_radius") (result f32) f32.const 6)
  (func (export "er_ui_direction_item_padding") (result f32) f32.const 5)
  (func (export "er_ui_direction_item_text_h") (result f32) f32.const 12)
  (func (export "er_ui_direction_icon_size") (result f32) f32.const 18)
  (func (export "er_ui_direction_gap") (result f32) f32.const 12)
  (func (export "er_ui_direction_vertical_padding") (result f32) f32.const 8)
  (func (export "er_ui_direction_label_max_lines") (result i32) i32.const 1)

  (func (export "er_ui_direction_active_index") (param $active i32) (result i32)
    local.get $active i32.const 2 call $er_ui_list_clamped_index)

  (func (export "er_ui_direction_indexed_id") (param $id i32) (param $active i32) (result i32)
    local.get $id local.get $active i32.const 2 call $er_ui_list_encoded_indexed_id)

  (func (export "er_ui_direction_hit_id") (param $id i32) (param $index i32) (result i32)
    local.get $id
    local.get $index i32.const 0 i32.const 1 call $clamp_i32
    i32.add)

  (func $er_ui_direction_write_ltr_label (result i32)
    i32.const 122520 i32.const 76 i32.store8
    i32.const 122521 i32.const 84 i32.store8
    i32.const 122522 i32.const 82 i32.store8
    i32.const 122520)

  (func $er_ui_direction_write_rtl_label (result i32)
    i32.const 122524 i32.const 82 i32.store8
    i32.const 122525 i32.const 84 i32.store8
    i32.const 122526 i32.const 76 i32.store8
    i32.const 122524)

  (func $er_ui_direction_item_width (export "er_ui_direction_item_width") (param $label_ptr i32) (param $label_len i32) (result f32)
    local.get $label_ptr local.get $label_len f32.const 12 i32.const 1 f32.const 5 call $er_ui_primitives_measured_label_width)

  (func $er_ui_direction_write_widths (export "er_ui_direction_write_widths") (param $out i32) (result i32)
    local.get $out i32.eqz
    if i32.const 0 return end
    local.get $out call $er_ui_direction_write_ltr_label i32.const 3 call $er_ui_direction_item_width f32.store
    local.get $out i32.const 4 i32.add call $er_ui_direction_write_rtl_label i32.const 3 call $er_ui_direction_item_width f32.store
    i32.const 1)

  (func $er_ui_direction_first_w (result f32)
    i32.const 122528 call $er_ui_direction_write_widths drop
    i32.const 122528 f32.load)

  (func (export "er_ui_direction_item_bounds") (param $bounds i32) (param $index i32) (param $out i32) (result i32)
    (local $y f32) (local $first_w f32) (local $second_w f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    i32.const 122528 call $er_ui_direction_write_widths drop
    i32.const 122528 f32.load local.set $first_w
    i32.const 122532 f32.load local.set $second_w
    local.get $bounds i32.const 4 i32.add f32.load local.get $bounds i32.const 12 i32.add f32.load f32.const 20 f32.sub f32.const 0.5 f32.mul f32.add local.set $y
    local.get $index i32.const 0 i32.eq
    if
      local.get $out
      local.get $bounds f32.load
      local.get $y
      local.get $first_w
      f32.const 20
      call $rect_store
      return
    end
    local.get $out
    local.get $bounds f32.load local.get $first_w f32.add f32.const 42 f32.add
    local.get $y
    local.get $second_w
    f32.const 20
    call $rect_store)

  (func (export "er_ui_direction_item_text_bounds") (param $item_bounds i32) (param $out i32) (result i32)
    local.get $item_bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $item_bounds f32.const 5 i32.const 122536 call $er_ui_primitives_content_inset drop
    i32.const 122536 local.get $out f32.const 12 call $er_ui_rect_with_height_centered)

  (func (export "er_ui_direction_icon_bounds") (param $bounds i32) (param $out i32) (result i32)
    (local $first_w f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    call $er_ui_direction_first_w local.set $first_w
    local.get $out
    local.get $bounds f32.load local.get $first_w f32.add f32.const 12 f32.add
    local.get $bounds i32.const 4 i32.add f32.load local.get $bounds i32.const 12 i32.add f32.load f32.const 18 f32.sub f32.const 0.5 f32.mul f32.add
    f32.const 18
    f32.const 18
    call $rect_store)

  (func (export "er_ui_direction_measure") (param $constraints i32) (param $out i32) (result i32)
    (local $first_w f32) (local $second_w f32) (local $pref_w f32) (local $pref_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    i32.const 122528 call $er_ui_direction_write_widths drop
    i32.const 122528 f32.load local.set $first_w
    i32.const 122532 f32.load local.set $second_w
    local.get $first_w f32.const 12 f32.add f32.const 18 f32.add f32.const 12 f32.add local.get $second_w f32.add local.set $pref_w
    f32.const 36 local.set $pref_h
    local.get $pref_w local.get $pref_h local.get $constraints i32.const 122552 call $er_ui_primitives_constrain_preferred_size drop
    i32.const 122552 f32.load local.set $pref_w
    i32.const 122556 f32.load local.set $pref_h
    f32.const 44 local.get $pref_w call $min_f32
    f32.const 18 local.get $pref_h call $min_f32
    local.get $pref_w local.get $pref_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit
    local.get $pref_h
    local.get $out
    call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)
