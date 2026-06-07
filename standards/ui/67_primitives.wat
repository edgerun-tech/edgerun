(func (export "er_ui_primitives_min_extent") (result f32) f32.const 1)
  (func (export "er_ui_primitives_side_panel_layout_size") (result i32) i32.const 16)
  (func (export "er_ui_primitives_menu_list_layout_size") (result i32) i32.const 24)

  (func $er_ui_primitives_constrain_preferred_size (export "er_ui_primitives_constrain_preferred_size") (param $preferred_w f32) (param $preferred_h f32) (param $constraints i32) (param $out i32) (result i32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $constraints local.get $preferred_w call $er_ui_layout_axis_constraint_limit
    local.get $constraints i32.const 8 i32.add local.get $preferred_h call $er_ui_layout_axis_constraint_limit
    call $layout_store_size)

  (func (export "er_ui_primitives_max_measured_width") (param $constraints i32) (param $preferred_width f32) (result f32)
    local.get $constraints local.get $preferred_width call $er_ui_layout_axis_constraint_limit)

  (func (export "er_ui_primitives_max_measured_height") (param $constraints i32) (param $preferred_height f32) (result f32)
    local.get $constraints i32.const 8 i32.add local.get $preferred_height call $er_ui_layout_axis_constraint_limit)

  (func (export "er_ui_primitives_max_measured_size") (param $constraints i32) (param $preferred_w f32) (param $preferred_h f32) (param $out i32) (result i32)
    local.get $preferred_w local.get $preferred_h local.get $constraints local.get $out call $er_ui_primitives_constrain_preferred_size)

  (func $er_ui_primitives_content_inset (export "er_ui_primitives_content_inset") (param $bounds i32) (param $padding f32) (param $out i32) (result i32)
    (local $clamped f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $padding
    f32.const 0
    call $max_f32
    local.get $bounds i32.const 8 i32.add f32.load
    local.get $bounds i32.const 12 i32.add f32.load
    call $min_f32
    f32.const 0.5
    f32.mul
    call $min_f32
    local.set $clamped
    local.get $bounds
    local.get $out
    local.get $clamped
    call $er_ui_rect_inset_uniform
    drop
    local.get $out
    call $er_ui_rect_valid)

  (func $er_ui_primitives_side_panel_layout (export "er_ui_primitives_side_panel_layout") (param $out i32) (param $trigger_y f32) (param $trigger_w f32) (param $trigger_h f32) (param $gap f32) (result i32)
    local.get $out i32.eqz
    if i32.const 0 return end
    local.get $out local.get $trigger_y f32.store
    local.get $out i32.const 4 i32.add local.get $trigger_w f32.store
    local.get $out i32.const 8 i32.add local.get $trigger_h f32.store
    local.get $out i32.const 12 i32.add local.get $gap f32.store
    i32.const 1)

  (func $er_ui_primitives_side_panel_trigger_bounds (export "er_ui_primitives_side_panel_trigger_bounds") (param $bounds i32) (param $spec i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $spec i32.eqz i32.or local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $bounds f32.load
    local.get $bounds i32.const 4 i32.add f32.load local.get $spec f32.load f32.add
    local.get $spec i32.const 4 i32.add f32.load
    local.get $spec i32.const 8 i32.add f32.load
    call $rect_store)

  (func $er_ui_primitives_side_panel_content_bounds (export "er_ui_primitives_side_panel_content_bounds") (param $bounds i32) (param $spec i32) (param $out i32) (result i32)
    (local $x f32)
    local.get $bounds i32.eqz local.get $spec i32.eqz i32.or local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds f32.load
    local.get $spec i32.const 4 i32.add f32.load
    f32.add
    local.get $spec i32.const 12 i32.add f32.load
    f32.add
    local.set $x
    local.get $out
    local.get $x
    local.get $bounds i32.const 4 i32.add f32.load
    local.get $bounds f32.load local.get $bounds i32.const 8 i32.add f32.load f32.add local.get $x f32.sub f32.const 1 call $max_f32
    local.get $bounds i32.const 12 i32.add f32.load
    call $rect_store)

  (func (export "er_ui_primitives_overlay_trigger_offset") (result i32) i32.const 0)
  (func (export "er_ui_primitives_overlay_primary_offset") (result i32) i32.const 1)
  (func (export "er_ui_primitives_overlay_secondary_offset") (result i32) i32.const 2)
  (func $er_ui_primitives_overlay_trigger_id (export "er_ui_primitives_overlay_trigger_id") (param $id i32) (result i32) local.get $id)
  (func (export "er_ui_primitives_overlay_primary_id") (param $id i32) (result i32) local.get $id i32.const 1 i32.add)
  (func $er_ui_primitives_overlay_secondary_id (export "er_ui_primitives_overlay_secondary_id") (param $id i32) (result i32) local.get $id i32.const 2 i32.add)
  (func $er_ui_primitives_overlay_indexed_id (export "er_ui_primitives_overlay_indexed_id") (param $id i32) (param $index i32) (result i32)
    local.get $id local.get $index i32.add i32.const 1 i32.add)

  (func $er_ui_primitives_menu_list_layout (export "er_ui_primitives_menu_list_layout") (param $out i32) (param $padding f32) (param $item_h f32) (param $item_pitch f32) (param $item_radius f32) (param $item_padding f32) (param $item_text_h f32) (result i32)
    local.get $out i32.eqz
    if i32.const 0 return end
    local.get $out local.get $padding f32.store
    local.get $out i32.const 4 i32.add local.get $item_h f32.store
    local.get $out i32.const 8 i32.add local.get $item_pitch f32.store
    local.get $out i32.const 12 i32.add local.get $item_radius f32.store
    local.get $out i32.const 16 i32.add local.get $item_padding f32.store
    local.get $out i32.const 20 i32.add local.get $item_text_h f32.store
    i32.const 1)

  (func $er_ui_primitives_menu_item_bounds (export "er_ui_primitives_menu_item_bounds") (param $content i32) (param $index i32) (param $spec i32) (param $out i32) (result i32)
    local.get $content i32.eqz local.get $spec i32.eqz i32.or local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $content f32.load local.get $spec f32.load f32.add
    local.get $content i32.const 4 i32.add f32.load local.get $spec f32.load f32.add local.get $index f32.convert_i32_u local.get $spec i32.const 8 i32.add f32.load f32.mul f32.add
    local.get $content i32.const 8 i32.add f32.load local.get $spec f32.load f32.const 2 f32.mul f32.sub f32.const 1 call $max_f32
    local.get $spec i32.const 4 i32.add f32.load
    call $rect_store)

  (func $primitives_average_width (param $text_ptr i32) (param $text_len i32) (param $line_height f32) (result f32)
    (local $count i32)
    local.get $text_len
    i32.eqz
    if
      i32.const 120248
      i32.const 110
      i32.store8
      i32.const 120248
      i32.const 1
      local.get $line_height
      call $er_ui_font_text_width
      return
    end
    local.get $text_ptr
    local.get $text_len
    call $er_ui_utf8_codepoint_count
    local.tee $count
    i32.eqz
    if
      f32.const 1
      return
    end
    local.get $text_ptr
    local.get $text_len
    local.get $line_height
    call $er_ui_font_text_width
    local.get $count
    f32.convert_i32_u
    f32.div
    f32.const 1
    call $max_f32)

  (func $primitives_text_measure (param $text_ptr i32) (param $text_len i32) (param $constraints i32) (param $line_height f32) (param $max_lines i32) (param $out i32) (result i32)
    i32.const 120256
    local.get $line_height
    local.get $text_ptr
    local.get $text_len
    local.get $line_height
    call $primitives_average_width
    local.get $max_lines
    call $er_ui_layout_text_metrics_init
    drop
    local.get $text_ptr
    local.get $text_len
    local.get $constraints
    i32.const 120256
    local.get $out
    call $er_ui_layout_measure_text)

  (func $er_ui_primitives_measured_label_width (export "er_ui_primitives_measured_label_width") (param $text_ptr i32) (param $text_len i32) (param $line_height f32) (param $max_lines i32) (param $padding f32) (result f32)
    local.get $text_ptr
    local.get $text_len
    local.get $line_height
    call $er_ui_font_text_width
    local.get $padding
    f32.const 2
    f32.mul
    f32.add)

  (func $er_ui_primitives_measure_two_item_menu_panel (export "er_ui_primitives_measure_two_item_menu_panel") (param $first_ptr i32) (param $first_len i32) (param $second_ptr i32) (param $second_len i32) (param $constraints i32) (param $spec i32) (param $out i32) (result i32)
    (local $pref_w f32) (local $pref_h f32) (local $min_w f32) (local $min_h f32) (local $max_w f32)
    local.get $constraints i32.eqz local.get $spec i32.eqz i32.or local.get $out i32.eqz i32.or
    if i32.const 0 return end
    i32.const 120272
    local.get $spec f32.load
    local.get $spec i32.const 16 i32.add f32.load
    f32.add
    f32.store
    i32.const 120276
    local.get $spec f32.load
    local.get $spec i32.const 16 i32.add f32.load
    f32.add
    f32.store
    i32.const 120280
    local.get $spec f32.load
    f32.store
    i32.const 120284
    local.get $spec f32.load
    f32.store
    local.get $constraints
    i32.const 120272
    i32.const 120288
    call $er_ui_layout_constraints_inner
    drop
    local.get $first_ptr local.get $first_len i32.const 120288 local.get $spec i32.const 20 i32.add f32.load i32.const 1 i32.const 120320 call $primitives_text_measure drop
    local.get $second_ptr local.get $second_len i32.const 120288 local.get $spec i32.const 20 i32.add f32.load i32.const 1 i32.const 120344 call $primitives_text_measure drop
    local.get $constraints
    i32.const 120328 f32.load i32.const 120352 f32.load call $max_f32
    local.get $spec f32.load local.get $spec i32.const 16 i32.add f32.load f32.add f32.const 2 f32.mul f32.add
    call $er_ui_layout_axis_constraint_limit
    local.set $pref_w
    local.get $constraints i32.const 8 i32.add
    local.get $spec f32.load f32.const 2 f32.mul local.get $spec i32.const 4 i32.add f32.load f32.add local.get $spec i32.const 8 i32.add f32.load f32.add
    call $er_ui_layout_axis_constraint_limit
    local.set $pref_h
    f32.const 1 local.get $spec f32.load local.get $spec i32.const 16 i32.add f32.load f32.add f32.const 2 f32.mul f32.add local.set $min_w
    local.get $spec f32.load f32.const 2 f32.mul local.get $spec i32.const 4 i32.add f32.load f32.const 2 f32.mul f32.add local.set $min_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit local.set $max_w
    local.get $min_w local.get $min_h local.get $pref_w local.get $pref_h local.get $max_w local.get $pref_h local.get $out call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)

  (func (export "er_ui_primitives_title_detail_panel_size") (result i32) i32.const 32)

  (func $er_ui_primitives_title_detail_panel (export "er_ui_primitives_title_detail_panel") (param $out i32) (param $radius f32) (param $padding f32) (param $title_y f32) (param $title_h f32) (param $detail_y f32) (param $detail_h f32) (param $title_right_inset f32) (param $title_max_lines i32) (param $detail_max_lines i32) (result i32)
    local.get $out i32.eqz
    if i32.const 0 return end
    local.get $out local.get $radius f32.store
    local.get $out i32.const 4 i32.add local.get $padding f32.store
    local.get $out i32.const 8 i32.add local.get $title_y f32.store
    local.get $out i32.const 12 i32.add local.get $title_h f32.store
    local.get $out i32.const 16 i32.add local.get $detail_y f32.store
    local.get $out i32.const 20 i32.add local.get $detail_h f32.store
    local.get $out i32.const 24 i32.add local.get $title_right_inset f32.store
    local.get $out i32.const 28 i32.add local.get $title_max_lines i32.store16
    local.get $out i32.const 30 i32.add local.get $detail_max_lines i32.store16
    i32.const 1)

  (func $er_ui_primitives_measure_title_detail_panel (export "er_ui_primitives_measure_title_detail_panel") (param $title_ptr i32) (param $title_len i32) (param $detail_ptr i32) (param $detail_len i32) (param $constraints i32) (param $spec i32) (param $out i32) (result i32)
    (local $detail_gap f32) (local $pref_w f32) (local $pref_h f32) (local $min_w f32) (local $min_h f32) (local $max_w f32) (local $max_h f32)
    local.get $constraints i32.eqz local.get $spec i32.eqz i32.or local.get $out i32.eqz i32.or
    if i32.const 0 return end
    i32.const 120272 local.get $spec i32.const 4 i32.add f32.load f32.store
    i32.const 120276 local.get $spec i32.const 4 i32.add f32.load local.get $spec i32.const 24 i32.add f32.load f32.add f32.store
    i32.const 120280 f32.const 0 f32.store
    i32.const 120284 f32.const 0 f32.store
    local.get $constraints i32.const 120272 i32.const 120288 call $er_ui_layout_constraints_inner drop
    i32.const 120272 local.get $spec i32.const 4 i32.add f32.load f32.store
    i32.const 120276 local.get $spec i32.const 4 i32.add f32.load f32.store
    local.get $constraints i32.const 120272 i32.const 120360 call $er_ui_layout_constraints_inner drop
    local.get $title_ptr local.get $title_len i32.const 120288 local.get $spec i32.const 12 i32.add f32.load local.get $spec i32.const 28 i32.add i32.load16_u i32.const 120320 call $primitives_text_measure drop
    local.get $detail_ptr local.get $detail_len i32.const 120360 local.get $spec i32.const 20 i32.add f32.load local.get $spec i32.const 30 i32.add i32.load16_u i32.const 120384 call $primitives_text_measure drop
    local.get $spec i32.const 16 i32.add f32.load local.get $spec i32.const 8 i32.add f32.load f32.sub local.get $spec i32.const 12 i32.add f32.load f32.sub f32.const 0 call $max_f32 local.set $detail_gap
    local.get $constraints
    i32.const 120328 f32.load local.get $spec i32.const 24 i32.add f32.load f32.add i32.const 120392 f32.load call $max_f32 local.get $spec i32.const 4 i32.add f32.load f32.const 2 f32.mul f32.add
    call $er_ui_layout_axis_constraint_limit
    local.set $pref_w
    local.get $constraints i32.const 8 i32.add
    local.get $spec i32.const 8 i32.add f32.load i32.const 120332 f32.load f32.add local.get $detail_gap f32.add i32.const 120396 f32.load f32.add local.get $spec i32.const 4 i32.add f32.load f32.add
    call $er_ui_layout_axis_constraint_limit
    local.set $pref_h
    f32.const 1 local.get $spec i32.const 4 i32.add f32.load f32.const 2 f32.mul f32.add local.get $spec i32.const 24 i32.add f32.load f32.add local.set $min_w
    local.get $spec i32.const 8 i32.add f32.load local.get $spec i32.const 12 i32.add f32.load f32.add local.get $detail_gap f32.add local.get $spec i32.const 20 i32.add f32.load f32.add local.get $spec i32.const 4 i32.add f32.load f32.add local.set $min_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit local.set $max_w
    local.get $pref_h i32.const 120340 f32.load local.get $detail_gap f32.add i32.const 120412 f32.load f32.add local.get $spec i32.const 8 i32.add f32.load f32.add local.get $spec i32.const 4 i32.add f32.load f32.add call $max_f32 local.set $max_h
    local.get $min_w local.get $min_h local.get $pref_w local.get $pref_h local.get $max_w local.get $max_h local.get $out call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)

  (func $er_ui_primitives_measure_side_panel_menu (export "er_ui_primitives_measure_side_panel_menu") (param $trigger_ptr i32) (param $trigger_len i32) (param $first_ptr i32) (param $first_len i32) (param $second_ptr i32) (param $second_len i32) (param $constraints i32) (param $panel i32) (param $trigger_padding f32) (param $menu i32) (param $out i32) (result i32)
    (local $trigger_w f32) (local $pref_w f32) (local $pref_h f32) (local $min_w f32) (local $min_h f32) (local $max_w f32) (local $max_h f32)
    local.get $constraints i32.eqz local.get $panel i32.eqz i32.or local.get $menu i32.eqz i32.or local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $trigger_ptr local.get $trigger_len f32.const 16 i32.const 1 local.get $trigger_padding call $er_ui_primitives_measured_label_width local.set $trigger_w
    i32.const 120272 f32.const 0 f32.store
    i32.const 120276 f32.const 0 f32.store
    i32.const 120280 f32.const 0 f32.store
    i32.const 120284 local.get $trigger_w local.get $panel i32.const 12 i32.add f32.load f32.add f32.store
    local.get $constraints i32.const 120272 i32.const 120288 call $er_ui_layout_constraints_inner drop
    local.get $first_ptr local.get $first_len local.get $second_ptr local.get $second_len i32.const 120288 local.get $menu i32.const 120320 call $er_ui_primitives_measure_two_item_menu_panel drop
    local.get $constraints
    local.get $trigger_w local.get $panel i32.const 12 i32.add f32.load f32.add i32.const 120328 f32.load f32.add
    call $er_ui_layout_axis_constraint_limit
    local.set $pref_w
    local.get $constraints i32.const 8 i32.add
    local.get $panel f32.load local.get $panel i32.const 8 i32.add f32.load f32.const 16 local.get $trigger_padding f32.const 2 f32.mul f32.add call $max_f32 f32.add
    i32.const 120332 f32.load
    call $max_f32
    call $er_ui_layout_axis_constraint_limit
    local.set $pref_h
    f32.const 2 local.get $panel i32.const 12 i32.add f32.load f32.add local.set $min_w
    local.get $panel f32.load f32.const 16 local.get $trigger_padding f32.const 2 f32.mul f32.add f32.add i32.const 120324 f32.load call $max_f32 local.set $min_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit local.set $max_w
    local.get $pref_h i32.const 120340 f32.load call $max_f32 local.set $max_h
    local.get $min_w local.get $min_h local.get $pref_w local.get $pref_h local.get $max_w local.get $max_h local.get $out call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)
