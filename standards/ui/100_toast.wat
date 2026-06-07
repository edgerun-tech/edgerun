(func (export "er_ui_toast_radius") (result f32) f32.const 8)
  (func (export "er_ui_toast_padding") (result f32) f32.const 10)
  (func (export "er_ui_toast_icon_x") (result f32) f32.const 12)
  (func (export "er_ui_toast_icon_size") (result f32) f32.const 16)
  (func (export "er_ui_toast_text_x") (result f32) f32.const 38)
  (func (export "er_ui_toast_text_gap") (result f32) f32.const 3)
  (func (export "er_ui_toast_min_width") (result f32) f32.const 160)
  (func (export "er_ui_toast_min_height") (result f32) f32.const 40)
  (func (export "er_ui_toast_title_line_height") (result f32) f32.const 14)
  (func (export "er_ui_toast_detail_line_height") (result f32) f32.const 12)
  (func (export "er_ui_toast_text_max_lines") (result i32) i32.const 2)

  (func (export "er_ui_toast_id") (param $id i32) (result i32)
    local.get $id)

  (func $er_ui_toast_text_width (export "er_ui_toast_text_width") (param $bounds i32) (result f32)
    local.get $bounds i32.eqz
    if f32.const 0 return end
    local.get $bounds i32.const 8 i32.add f32.load f32.const 48 f32.sub f32.const 1 call $max_f32)

  (func (export "er_ui_toast_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $bounds f32.load
    local.get $bounds i32.const 4 i32.add f32.load
    local.get $bounds i32.const 8 i32.add f32.load
    local.get $bounds i32.const 12 i32.add f32.load
    call $rect_store)

  (func (export "er_ui_toast_icon_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $bounds f32.load f32.const 12 f32.add
    local.get $bounds i32.const 4 i32.add f32.load local.get $bounds i32.const 12 i32.add f32.load f32.const 16 f32.sub f32.const 0.5 f32.mul f32.add
    f32.const 16
    f32.const 16
    call $rect_store)

  (func (export "er_ui_toast_title_bounds") (param $bounds i32) (param $title_ptr i32) (param $title_len i32) (param $out i32) (result i32)
    (local $text_w f32) (local $title_h f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds call $er_ui_toast_text_width local.set $text_w
    local.get $title_ptr local.get $title_len local.get $text_w f32.const 14 i32.const 2 call $er_ui_alert_measured_text_height local.set $title_h
    local.get $out
    local.get $bounds f32.load f32.const 38 f32.add
    local.get $bounds i32.const 4 i32.add f32.load f32.const 10 f32.add
    local.get $text_w
    local.get $title_h
    call $rect_store)

  (func (export "er_ui_toast_detail_bounds") (param $bounds i32) (param $title_ptr i32) (param $title_len i32) (param $detail_ptr i32) (param $detail_len i32) (param $out i32) (result i32)
    (local $text_w f32) (local $title_h f32) (local $detail_y f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $detail_len i32.eqz
    if i32.const 0 return end
    local.get $bounds call $er_ui_toast_text_width local.set $text_w
    local.get $title_ptr local.get $title_len local.get $text_w f32.const 14 i32.const 2 call $er_ui_alert_measured_text_height local.set $title_h
    local.get $bounds i32.const 4 i32.add f32.load f32.const 10 f32.add local.get $title_h f32.add f32.const 3 f32.add local.set $detail_y
    local.get $out
    local.get $bounds f32.load f32.const 38 f32.add
    local.get $detail_y
    local.get $text_w
    local.get $bounds i32.const 4 i32.add f32.load local.get $bounds i32.const 12 i32.add f32.load f32.add local.get $detail_y f32.sub f32.const 10 f32.sub f32.const 1 call $max_f32
    call $rect_store)

  (func (export "er_ui_toast_measure") (param $title_ptr i32) (param $title_len i32) (param $detail_ptr i32) (param $detail_len i32) (param $constraints i32) (param $out i32) (result i32)
    (local $title_w f32) (local $title_h f32) (local $detail_w f32) (local $detail_h f32) (local $gap f32) (local $pref_w f32) (local $pref_h f32) (local $max_w f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    i32.const 123640 f32.const 0 f32.store
    i32.const 123644 f32.const 10 f32.store
    i32.const 123648 f32.const 0 f32.store
    i32.const 123652 f32.const 38 f32.store
    local.get $constraints i32.const 123640 i32.const 123656 call $er_ui_layout_constraints_inner drop
    local.get $title_ptr local.get $title_len i32.const 123656 f32.const 14 i32.const 2 i32.const 123680 call $primitives_text_measure drop
    i32.const 123688 f32.load local.set $title_w
    i32.const 123692 f32.load local.set $title_h
    local.get $detail_len i32.eqz
    if
      f32.const 0 local.set $detail_w
      f32.const 0 local.set $detail_h
      f32.const 0 local.set $gap
    else
      local.get $detail_ptr local.get $detail_len i32.const 123656 f32.const 12 i32.const 2 i32.const 123704 call $primitives_text_measure drop
      i32.const 123712 f32.load local.set $detail_w
      i32.const 123716 f32.load local.set $detail_h
      f32.const 3 local.set $gap
    end
    f32.const 160 local.get $title_w local.get $detail_w call $max_f32 f32.const 48 f32.add call $max_f32 local.set $pref_w
    f32.const 20 local.get $title_h f32.add local.get $gap f32.add local.get $detail_h f32.add local.set $pref_h
    local.get $pref_w local.get $pref_h local.get $constraints i32.const 123728 call $er_ui_primitives_constrain_preferred_size drop
    i32.const 123728 f32.load local.set $pref_w
    i32.const 123732 f32.load local.set $pref_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit local.set $max_w
    f32.const 160 local.get $pref_w call $min_f32
    f32.const 40 local.get $pref_h call $min_f32
    local.get $pref_w local.get $pref_h local.get $max_w local.get $pref_h local.get $out call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)
