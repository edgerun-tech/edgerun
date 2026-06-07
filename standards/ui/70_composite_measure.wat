(func (export "er_ui_row_item_min_width") (result f32) f32.const 96)
  (func (export "er_ui_row_item_min_height") (result f32) f32.const 32)
  (func (export "er_ui_row_item_text_width") (param $bounds i32) (param $has_icon i32) (result f32)
    (local $width f32)
    local.get $bounds i32.eqz
    if f32.const 0 return end
    local.get $bounds i32.const 8 i32.add f32.load
    f32.const 24
    f32.sub
    local.set $width
    local.get $has_icon
    if
      local.get $width
      f32.const 36
      f32.sub
      local.set $width
    end
    local.get $width
    f32.const 1
    call $max_f32)

  (func (export "er_ui_row_item_text_bounds") (param $bounds i32) (param $has_icon i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds
    local.get $out
    f32.const 12
    local.get $has_icon
    if (result f32)
      f32.const 36
    else
      f32.const 0
    end
    f32.add
    f32.const 0
    f32.const 12
    f32.const 0
    call $er_ui_rect_inset_ltrb
    drop
    local.get $out call $er_ui_rect_valid)

  (func (export "er_ui_row_item_measure") (param $title_ptr i32) (param $title_len i32) (param $detail_ptr i32) (param $detail_len i32) (param $has_icon i32) (param $constraints i32) (param $out i32) (result i32)
    (local $icon_extra f32) (local $gap f32) (local $pref_w f32) (local $pref_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $has_icon if f32.const 36 local.set $icon_extra end
    i32.const 120624 f32.const 0 f32.store
    i32.const 120628 f32.const 12 f32.store
    i32.const 120632 f32.const 0 f32.store
    i32.const 120636 f32.const 12 local.get $icon_extra f32.add f32.store
    local.get $constraints i32.const 120624 i32.const 120640 call $er_ui_layout_constraints_inner drop
    i32.const 120668 f32.const 15 f32.store
    i32.const 120672 local.get $title_ptr local.get $title_len f32.const 15 call $primitives_average_width f32.store
    i32.const 120676 i32.const 1 i32.store
    local.get $title_ptr local.get $title_len i32.const 120640 i32.const 120668 i32.const 120680 call $er_ui_text_component_measure_value drop
    local.get $detail_len i32.eqz
    if
      f32.const 0 f32.const 0 i32.const 120704 call $er_ui_layout_measurement_fixed drop
      f32.const 0 local.set $gap
    else
      i32.const 120728 f32.const 13 f32.store
      i32.const 120732 local.get $detail_ptr local.get $detail_len f32.const 13 call $primitives_average_width f32.store
      i32.const 120736 i32.const 1 i32.store
      local.get $detail_ptr local.get $detail_len i32.const 120640 i32.const 120728 i32.const 120704 call $er_ui_text_component_measure_value drop
      f32.const 2 local.set $gap
    end
    local.get $constraints
    f32.const 96 i32.const 120688 f32.load i32.const 120712 f32.load call $max_f32 f32.const 24 f32.add local.get $icon_extra f32.add call $max_f32
    call $er_ui_layout_axis_constraint_limit
    local.set $pref_w
    local.get $constraints i32.const 8 i32.add
    f32.const 12 i32.const 120692 f32.load f32.add local.get $gap f32.add i32.const 120716 f32.load f32.add
    f32.const 38
    call $max_f32
    call $er_ui_layout_axis_constraint_limit
    local.set $pref_h
    f32.const 96 local.get $pref_w call $min_f32
    f32.const 32 local.get $pref_h call $min_f32
    local.get $pref_w local.get $pref_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit
    local.get $pref_h
    local.get $out
    call $er_ui_layout_measurement_flexible
    drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)

  (func (export "er_ui_card_measure") (param $title_ptr i32) (param $title_len i32) (param $detail_ptr i32) (param $detail_len i32) (param $constraints i32) (param $out i32) (result i32)
    (local $gap f32) (local $content_w f32) (local $content_h f32) (local $min_h f32) (local $pref_w f32) (local $pref_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    f32.const 16 i32.const 120624 call $er_ui_layout_insets_uniform drop
    local.get $constraints i32.const 120624 i32.const 120640 call $er_ui_layout_constraints_inner drop
    local.get $title_len i32.eqz
    if
      f32.const 0 f32.const 0 i32.const 120680 call $er_ui_layout_measurement_fixed drop
    else
      i32.const 120668 f32.const 18 f32.store
      i32.const 120672 f32.const 8.5 f32.store
      i32.const 120676 i32.const 2 i32.store
      local.get $title_ptr local.get $title_len i32.const 120640 i32.const 120668 i32.const 120680 call $er_ui_text_component_measure_value drop
    end
    local.get $detail_len i32.eqz
    if
      f32.const 0 f32.const 0 i32.const 120704 call $er_ui_layout_measurement_fixed drop
    else
      i32.const 120728 f32.const 16 f32.store
      i32.const 120732 f32.const 8 f32.store
      i32.const 120736 i32.const 3 i32.store
      local.get $detail_ptr local.get $detail_len i32.const 120640 i32.const 120728 i32.const 120704 call $er_ui_text_component_measure_value drop
    end
    local.get $title_len i32.const 0 i32.ne local.get $detail_len i32.const 0 i32.ne i32.and
    if f32.const 8 local.set $gap end
    i32.const 120688 f32.load i32.const 120712 f32.load call $max_f32 local.set $content_w
    i32.const 120692 f32.load local.get $gap f32.add i32.const 120716 f32.load f32.add local.set $content_h
    local.get $title_len i32.const 0 i32.ne local.get $detail_len i32.const 0 i32.ne i32.and
    if
      f32.const 74 local.set $min_h
    else
      local.get $title_len i32.const 0 i32.ne
      if
        f32.const 50 local.set $min_h
      else
        local.get $detail_len i32.const 0 i32.ne
        if f32.const 48 local.set $min_h else f32.const 32 local.set $min_h end
      end
    end
    local.get $constraints local.get $content_w f32.const 32 f32.add call $er_ui_layout_axis_constraint_limit local.set $pref_w
    local.get $constraints i32.const 8 i32.add local.get $min_h local.get $content_h f32.const 32 f32.add call $max_f32 call $er_ui_layout_axis_constraint_limit local.set $pref_h
    f32.const 160 local.get $pref_w call $min_f32
    f32.const 32 local.get $pref_h call $min_f32
    local.get $pref_w local.get $pref_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit
    local.get $constraints i32.const 8 i32.add local.get $pref_h call $er_ui_layout_axis_constraint_limit
    local.get $out
    call $er_ui_layout_measurement_flexible
    drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)

  (func (export "er_ui_empty_measure") (param $title_ptr i32) (param $title_len i32) (param $detail_ptr i32) (param $detail_len i32) (param $constraints i32) (param $out i32) (result i32)
    (local $content_w f32) (local $pref_w f32) (local $pref_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    i32.const 120624 f32.const 0 f32.store
    i32.const 120628 f32.const 24 f32.store
    i32.const 120632 f32.const 0 f32.store
    i32.const 120636 f32.const 24 f32.store
    local.get $constraints i32.const 120624 i32.const 120640 call $er_ui_layout_constraints_inner drop
    i32.const 120668 f32.const 20 f32.store
    i32.const 120672 local.get $title_ptr local.get $title_len f32.const 20 call $primitives_average_width f32.store
    i32.const 120676 i32.const 2 i32.store
    local.get $title_ptr local.get $title_len i32.const 120640 i32.const 120668 i32.const 120680 call $er_ui_text_component_measure_value drop
    i32.const 120728 f32.const 16 f32.store
    i32.const 120732 local.get $detail_ptr local.get $detail_len f32.const 16 call $primitives_average_width f32.store
    i32.const 120736 i32.const 2 i32.store
    local.get $detail_ptr local.get $detail_len i32.const 120640 i32.const 120728 i32.const 120704 call $er_ui_text_component_measure_value drop
    f32.const 40 i32.const 120688 f32.load i32.const 120712 f32.load call $max_f32 call $max_f32 local.set $content_w
    local.get $constraints f32.const 144 local.get $content_w f32.const 48 f32.add call $max_f32 call $er_ui_layout_axis_constraint_limit local.set $pref_w
    local.get $constraints i32.const 8 i32.add f32.const 48 f32.const 40 f32.add f32.const 10 f32.add i32.const 120692 f32.load f32.add f32.const 4 f32.add i32.const 120716 f32.load f32.add call $er_ui_layout_axis_constraint_limit local.set $pref_h
    f32.const 144 local.get $pref_w call $min_f32
    f32.const 96 local.get $pref_h call $min_f32
    local.get $pref_w local.get $pref_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit
    local.get $pref_h
    local.get $out
    call $er_ui_layout_measurement_flexible
    drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)
