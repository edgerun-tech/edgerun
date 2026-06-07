(func (export "er_ui_sheet_trigger_y") (result f32) f32.const 4)
  (func (export "er_ui_sheet_trigger_w") (result f32) f32.const 62)
  (func (export "er_ui_sheet_trigger_h") (result f32) f32.const 30)
  (func (export "er_ui_sheet_trigger_padding") (result f32) f32.const 8)
  (func (export "er_ui_sheet_content_w") (result f32) f32.const 96)
  (func (export "er_ui_sheet_content_min_left") (result f32) f32.const 82)
  (func (export "er_ui_sheet_radius") (result f32) f32.const 8)
  (func (export "er_ui_sheet_padding") (result f32) f32.const 10)
  (func (export "er_ui_sheet_close_size") (result f32) f32.const 28)
  (func (export "er_ui_sheet_close_inset") (result f32) f32.const 8)
  (func (export "er_ui_sheet_close_space") (result f32) f32.const 34)
  (func (export "er_ui_sheet_panel_title_y") (result f32) f32.const 10)
  (func (export "er_ui_sheet_panel_title_h") (result f32) f32.const 14)
  (func (export "er_ui_sheet_panel_detail_y") (result f32) f32.const 29)
  (func (export "er_ui_sheet_panel_detail_h") (result f32) f32.const 12)

  (func $er_ui_sheet_panel (param $out i32) (result i32)
    local.get $out f32.const 8 f32.const 10 f32.const 10 f32.const 14 f32.const 29 f32.const 12 f32.const 34 i32.const 1 i32.const 1 call $er_ui_primitives_title_detail_panel)

  (func (export "er_ui_sheet_trigger_id") (param $id i32) (result i32)
    local.get $id call $er_ui_primitives_overlay_trigger_id)

  (func (export "er_ui_sheet_content_id") (param $id i32) (result i32)
    local.get $id i32.const 1 call $er_ui_primitives_overlay_indexed_id)

  (func (export "er_ui_sheet_close_id") (param $id i32) (result i32)
    local.get $id call $er_ui_primitives_overlay_secondary_id)

  (func $er_ui_sheet_trigger_bounds (export "er_ui_sheet_trigger_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $bounds f32.load
    local.get $bounds i32.const 4 i32.add f32.load f32.const 4 f32.add
    f32.const 62
    f32.const 30
    call $rect_store)

  (func $er_ui_sheet_content_width_for_bounds (export "er_ui_sheet_content_width_for_bounds") (param $bounds i32) (result f32)
    local.get $bounds i32.eqz
    if f32.const 1 return end
    f32.const 96
    local.get $bounds i32.const 8 i32.add f32.load f32.const 82 f32.sub f32.const 1 call $max_f32
    call $min_f32)

  (func $er_ui_sheet_content_bounds (export "er_ui_sheet_content_bounds") (param $bounds i32) (param $out i32) (result i32)
    (local $w f32) (local $x f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds call $er_ui_sheet_content_width_for_bounds local.set $w
    local.get $bounds f32.load local.get $bounds i32.const 8 i32.add f32.load f32.add local.get $w f32.sub local.set $x
    local.get $out
    local.get $x
    local.get $bounds i32.const 4 i32.add f32.load
    local.get $bounds f32.load local.get $bounds i32.const 8 i32.add f32.load f32.add local.get $x f32.sub f32.const 1 call $max_f32
    local.get $bounds i32.const 12 i32.add f32.load
    call $rect_store)

  (func (export "er_ui_sheet_close_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 123960 call $er_ui_sheet_content_bounds drop
    local.get $out
    i32.const 123960 f32.load i32.const 123968 f32.load f32.add f32.const 36 f32.sub
    i32.const 123964 f32.load f32.const 8 f32.add
    f32.const 28
    f32.const 28
    call $rect_store)

  (func (export "er_ui_sheet_trigger_text_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 123976 call $er_ui_sheet_trigger_bounds drop
    i32.const 123976 f32.const 8 i32.const 123992 call $er_ui_primitives_content_inset drop
    i32.const 123992 local.get $out f32.const 16 call $er_ui_rect_with_height_centered)

  (func (export "er_ui_sheet_title_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 124008 call $er_ui_sheet_content_bounds drop
    local.get $out
    i32.const 124008 f32.load f32.const 10 f32.add
    i32.const 124012 f32.load f32.const 10 f32.add
    i32.const 124016 f32.load f32.const 54 f32.sub f32.const 1 call $max_f32
    f32.const 14
    call $rect_store)

  (func (export "er_ui_sheet_detail_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 124024 call $er_ui_sheet_content_bounds drop
    local.get $out
    i32.const 124024 f32.load f32.const 10 f32.add
    i32.const 124028 f32.load f32.const 29 f32.add
    i32.const 124032 f32.load f32.const 20 f32.sub f32.const 1 call $max_f32
    f32.const 12
    call $rect_store)

  (func (export "er_ui_sheet_measure") (param $title_ptr i32) (param $title_len i32) (param $detail_ptr i32) (param $detail_len i32) (param $constraints i32) (param $out i32) (result i32)
    (local $trigger_w f32) (local $pref_w f32) (local $pref_h f32) (local $max_w f32) (local $max_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    call $er_ui_dialog_open_label_ptr i32.const 4 f32.const 16 i32.const 1 f32.const 8 call $er_ui_primitives_measured_label_width local.set $trigger_w
    i32.const 124040 call $er_ui_sheet_panel drop
    local.get $title_ptr local.get $title_len local.get $detail_ptr local.get $detail_len local.get $constraints i32.const 124040 i32.const 124072 call $er_ui_primitives_measure_title_detail_panel drop
    local.get $constraints local.get $trigger_w f32.const 16 f32.add i32.const 124080 f32.load f32.const 82 f32.add call $max_f32 call $er_ui_layout_axis_constraint_limit local.set $pref_w
    local.get $constraints i32.const 8 i32.add f32.const 36 i32.const 124084 f32.load call $max_f32 call $er_ui_layout_axis_constraint_limit local.set $pref_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit local.set $max_w
    local.get $pref_h i32.const 124092 f32.load call $max_f32 local.set $max_h
    f32.const 83 f32.const 51 local.get $pref_w local.get $pref_h local.get $max_w local.get $max_h local.get $out call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)
