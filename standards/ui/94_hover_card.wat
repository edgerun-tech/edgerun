(func (export "er_ui_hover_card_trigger_y") (result f32) f32.const 6)
  (func (export "er_ui_hover_card_trigger_w") (result f32) f32.const 66)
  (func (export "er_ui_hover_card_trigger_h") (result f32) f32.const 30)
  (func (export "er_ui_hover_card_gap") (result f32) f32.const 10)
  (func (export "er_ui_hover_card_radius") (result f32) f32.const 8)
  (func (export "er_ui_hover_card_padding") (result f32) f32.const 10)
  (func (export "er_ui_hover_card_panel_title_y") (result f32) f32.const 8)
  (func (export "er_ui_hover_card_panel_title_h") (result f32) f32.const 14)
  (func (export "er_ui_hover_card_panel_detail_y") (result f32) f32.const 25)
  (func (export "er_ui_hover_card_panel_detail_h") (result f32) f32.const 12)
  (func (export "er_ui_hover_card_detail_label_len") (result i32) i32.const 13)
  (func (export "er_ui_hover_card_text_max_lines") (result i32) i32.const 2)

  (func $er_ui_hover_card_layout (param $out i32) (result i32)
    local.get $out f32.const 6 f32.const 66 f32.const 30 f32.const 10 call $er_ui_primitives_side_panel_layout)

  (func $er_ui_hover_card_write_detail_label (result i32)
    i32.const 122940 i32.const 72 i32.store8
    i32.const 122941 i32.const 111 i32.store8
    i32.const 122942 i32.const 118 i32.store8
    i32.const 122943 i32.const 101 i32.store8
    i32.const 122944 i32.const 114 i32.store8
    i32.const 122945 i32.const 32 i32.store8
    i32.const 122946 i32.const 99 i32.store8
    i32.const 122947 i32.const 111 i32.store8
    i32.const 122948 i32.const 110 i32.store8
    i32.const 122949 i32.const 116 i32.store8
    i32.const 122950 i32.const 101 i32.store8
    i32.const 122951 i32.const 110 i32.store8
    i32.const 122952 i32.const 116 i32.store8
    i32.const 122940)

  (func (export "er_ui_hover_card_trigger_id") (param $id i32) (result i32)
    local.get $id)

  (func (export "er_ui_hover_card_content_id") (param $id i32) (result i32)
    local.get $id i32.const 1 i32.add)

  (func $er_ui_hover_card_trigger_bounds (export "er_ui_hover_card_trigger_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    i32.const 122956 call $er_ui_hover_card_layout drop
    local.get $bounds i32.const 122956 local.get $out call $er_ui_primitives_side_panel_trigger_bounds)

  (func $er_ui_hover_card_content_bounds (export "er_ui_hover_card_content_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    i32.const 122956 call $er_ui_hover_card_layout drop
    local.get $bounds i32.const 122956 local.get $out call $er_ui_primitives_side_panel_content_bounds)

  (func (export "er_ui_hover_card_trigger_text_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 122972 call $er_ui_hover_card_trigger_bounds drop
    i32.const 122972 f32.const 12 i32.const 122988 call $er_ui_primitives_content_inset drop
    i32.const 122988 local.get $out f32.const 16 call $er_ui_rect_with_height_centered)

  (func (export "er_ui_hover_card_title_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 123004 call $er_ui_hover_card_content_bounds drop
    local.get $out
    i32.const 123004 f32.load f32.const 10 f32.add
    i32.const 123004 i32.const 4 i32.add f32.load f32.const 8 f32.add
    i32.const 123004 i32.const 8 i32.add f32.load f32.const 20 f32.sub f32.const 1 call $max_f32
    f32.const 14
    call $rect_store)

  (func (export "er_ui_hover_card_detail_bounds") (param $bounds i32) (param $detail_ptr i32) (param $detail_len i32) (param $out i32) (result i32)
    (local $detail_h f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 123020 call $er_ui_hover_card_content_bounds drop
    local.get $detail_ptr local.get $detail_len i32.const 123020 i32.const 8 i32.add f32.load f32.const 20 f32.sub f32.const 1 call $max_f32 f32.const 12 i32.const 2 call $er_ui_alert_measured_text_height
    i32.const 123020 i32.const 12 i32.add f32.load f32.const 35 f32.sub f32.const 1 call $max_f32
    call $min_f32
    local.set $detail_h
    local.get $out
    i32.const 123020 f32.load f32.const 10 f32.add
    i32.const 123020 i32.const 4 i32.add f32.load f32.const 25 f32.add
    i32.const 123020 i32.const 8 i32.add f32.load f32.const 20 f32.sub f32.const 1 call $max_f32
    local.get $detail_h
    call $rect_store)

  (func (export "er_ui_hover_card_measure") (param $trigger_ptr i32) (param $trigger_len i32) (param $content_ptr i32) (param $content_len i32) (param $constraints i32) (param $out i32) (result i32)
    (local $trigger_w f32) (local $title_w f32) (local $detail_w f32) (local $panel_w f32) (local $pref_w f32) (local $pref_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $trigger_ptr local.get $trigger_len f32.const 16 call $er_ui_font_text_width f32.const 24 f32.add local.set $trigger_w
    call $er_ui_hover_card_write_detail_label i32.const 13 f32.const 14 call $er_ui_font_text_width local.set $title_w
    local.get $content_ptr local.get $content_len f32.const 12 call $er_ui_font_text_width local.set $detail_w
    local.get $title_w local.get $detail_w call $max_f32 f32.const 20 f32.add local.set $panel_w
    local.get $trigger_w f32.const 10 f32.add local.get $panel_w f32.add local.set $pref_w
    f32.const 47 local.set $pref_h
    local.get $pref_w local.get $pref_h local.get $constraints i32.const 123036 call $er_ui_primitives_constrain_preferred_size drop
    i32.const 123036 f32.load local.set $pref_w
    i32.const 123040 f32.load local.set $pref_h
    f32.const 12 local.get $pref_w call $min_f32
    f32.const 47 local.get $pref_h call $min_f32
    local.get $pref_w local.get $pref_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit
    local.get $pref_h
    local.get $out
    call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)
