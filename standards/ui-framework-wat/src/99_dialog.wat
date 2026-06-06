  (func (export "er_ui_dialog_trigger_y") (result f32) f32.const 6)
  (func (export "er_ui_dialog_trigger_w") (result f32) f32.const 66)
  (func (export "er_ui_dialog_trigger_h") (result f32) f32.const 30)
  (func (export "er_ui_dialog_gap") (result f32) f32.const 12)
  (func (export "er_ui_dialog_trigger_padding") (result f32) f32.const 8)
  (func (export "er_ui_dialog_panel_radius") (result f32) f32.const 10)
  (func (export "er_ui_dialog_panel_padding") (result f32) f32.const 10)
  (func (export "er_ui_dialog_panel_title_y") (result f32) f32.const 6)
  (func (export "er_ui_dialog_panel_title_h") (result f32) f32.const 14)
  (func (export "er_ui_dialog_panel_detail_y") (result f32) f32.const 22)
  (func (export "er_ui_dialog_panel_detail_h") (result f32) f32.const 12)
  (func (export "er_ui_dialog_open_label_len") (result i32) i32.const 4)

  (func $er_ui_dialog_open_label_ptr (export "er_ui_dialog_open_label_ptr") (result i32)
    i32.const 123440 i32.const 79 i32.store8
    i32.const 123441 i32.const 112 i32.store8
    i32.const 123442 i32.const 101 i32.store8
    i32.const 123443 i32.const 110 i32.store8
    i32.const 123440)

  (func $er_ui_dialog_layout (param $out i32) (result i32)
    local.get $out f32.const 6 f32.const 66 f32.const 30 f32.const 12 call $er_ui_primitives_side_panel_layout)

  (func $er_ui_dialog_panel (param $out i32) (result i32)
    local.get $out f32.const 10 f32.const 10 f32.const 6 f32.const 14 f32.const 22 f32.const 12 f32.const 0 i32.const 1 i32.const 1 call $er_ui_primitives_title_detail_panel)

  (func (export "er_ui_dialog_trigger_id") (param $id i32) (result i32)
    local.get $id call $er_ui_primitives_overlay_trigger_id)

  (func (export "er_ui_dialog_content_id") (param $id i32) (result i32)
    local.get $id i32.const 1 call $er_ui_primitives_overlay_indexed_id)

  (func $er_ui_dialog_trigger_bounds (export "er_ui_dialog_trigger_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    i32.const 123448 call $er_ui_dialog_layout drop
    local.get $bounds i32.const 123448 local.get $out call $er_ui_primitives_side_panel_trigger_bounds)

  (func $er_ui_dialog_content_bounds (export "er_ui_dialog_content_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    i32.const 123448 call $er_ui_dialog_layout drop
    local.get $bounds i32.const 123448 local.get $out call $er_ui_primitives_side_panel_content_bounds)

  (func (export "er_ui_dialog_trigger_text_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 123464 call $er_ui_dialog_trigger_bounds drop
    i32.const 123464 f32.const 8 i32.const 123480 call $er_ui_primitives_content_inset drop
    i32.const 123480 local.get $out f32.const 16 call $er_ui_rect_with_height_centered)

  (func (export "er_ui_dialog_title_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 123496 call $er_ui_dialog_content_bounds drop
    local.get $out
    i32.const 123496 f32.load f32.const 10 f32.add
    i32.const 123500 f32.load f32.const 6 f32.add
    i32.const 123504 f32.load f32.const 20 f32.sub f32.const 1 call $max_f32
    f32.const 14
    call $rect_store)

  (func (export "er_ui_dialog_detail_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 123512 call $er_ui_dialog_content_bounds drop
    local.get $out
    i32.const 123512 f32.load f32.const 10 f32.add
    i32.const 123516 f32.load f32.const 22 f32.add
    i32.const 123520 f32.load f32.const 20 f32.sub f32.const 1 call $max_f32
    f32.const 12
    call $rect_store)

  (func (export "er_ui_dialog_measure") (param $title_ptr i32) (param $title_len i32) (param $detail_ptr i32) (param $detail_len i32) (param $constraints i32) (param $out i32) (result i32)
    (local $trigger_w f32) (local $pref_w f32) (local $pref_h f32) (local $min_h f32) (local $max_w f32) (local $max_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    call $er_ui_dialog_open_label_ptr i32.const 4 f32.const 16 i32.const 1 f32.const 8 call $er_ui_primitives_measured_label_width local.set $trigger_w
    i32.const 123528 f32.const 0 f32.store
    i32.const 123532 f32.const 0 f32.store
    i32.const 123536 f32.const 0 f32.store
    i32.const 123540 f32.const 78 f32.store
    local.get $constraints i32.const 123528 i32.const 123544 call $er_ui_layout_constraints_inner drop
    i32.const 123568 call $er_ui_dialog_panel drop
    local.get $title_ptr local.get $title_len local.get $detail_ptr local.get $detail_len i32.const 123544 i32.const 123568 i32.const 123600 call $er_ui_primitives_measure_title_detail_panel drop
    local.get $constraints local.get $trigger_w f32.const 12 f32.add i32.const 123608 f32.load f32.add call $er_ui_layout_axis_constraint_limit local.set $pref_w
    local.get $constraints i32.const 8 i32.add f32.const 38 i32.const 123612 f32.load call $max_f32 call $er_ui_layout_axis_constraint_limit local.set $pref_h
    f32.const 38 i32.const 123604 f32.load call $max_f32 local.set $min_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit local.set $max_w
    local.get $pref_h i32.const 123620 f32.load call $max_f32 local.set $max_h
    f32.const 14 local.get $min_h local.get $pref_w local.get $pref_h local.get $max_w local.get $max_h local.get $out call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)
