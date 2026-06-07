(func (export "er_ui_alert_radius") (result f32) f32.const 8)
  (func (export "er_ui_alert_padding_x") (result f32) f32.const 16)
  (func (export "er_ui_alert_padding_y") (result f32) f32.const 12)
  (func (export "er_ui_alert_icon_size") (result f32) f32.const 16)
  (func (export "er_ui_alert_text_x") (result f32) f32.const 44)
  (func (export "er_ui_alert_title_height") (result f32) f32.const 16)
  (func (export "er_ui_alert_title_max_lines") (result i32) i32.const 2)
  (func (export "er_ui_alert_detail_gap") (result f32) f32.const 2)
  (func (export "er_ui_alert_detail_height") (result f32) f32.const 16)
  (func (export "er_ui_alert_detail_max_lines") (result i32) i32.const 2)
  (func (export "er_ui_alert_min_width") (result f32) f32.const 160)
  (func (export "er_ui_alert_min_height") (result f32) f32.const 48)
  (func (export "er_ui_alert_icon_shift") (result i32) i32.const 1)
  (func (export "er_ui_alert_danger_color") (result i32) i32.const 4282664175)

  (func (export "er_ui_alert_packed_id") (param $destructive i32) (param $icon_tag i32) (result i32)
    local.get $destructive i32.const 0 i32.ne
    local.get $icon_tag i32.const 1 i32.shl
    i32.or)

  (func (export "er_ui_alert_packed_destructive") (param $packed i32) (result i32)
    local.get $packed i32.const 1 i32.and)

  (func (export "er_ui_alert_packed_icon_tag") (param $packed i32) (result i32)
    local.get $packed i32.const 1 i32.shr_u)

  (func $er_ui_alert_text_width (export "er_ui_alert_text_width") (param $bounds i32) (result f32)
    local.get $bounds i32.eqz
    if f32.const 0 return end
    local.get $bounds i32.const 8 i32.add f32.load f32.const 60 f32.sub f32.const 1 call $max_f32)

  (func (export "er_ui_alert_icon_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $bounds f32.load f32.const 16 f32.add
    local.get $bounds i32.const 4 i32.add f32.load f32.const 12 f32.add
    f32.const 16
    f32.const 16
    call $rect_store)

  (func $er_ui_alert_measured_text_height (param $text_ptr i32) (param $text_len i32) (param $width f32) (param $line_height f32) (param $max_lines i32) (result f32)
    i32.const 121260 i32.const 1 i32.store
    i32.const 121264 local.get $width f32.const 0 call $max_f32 f32.store
    i32.const 121268 i32.const 0 i32.store
    i32.const 121272 f32.const 0 f32.store
    i32.const 121276 i32.const 1 i32.store
    i32.const 121284 local.get $line_height f32.store
    i32.const 121288 local.get $text_ptr local.get $text_len local.get $line_height call $primitives_average_width f32.store
    i32.const 121292 local.get $max_lines i32.store
    local.get $text_ptr local.get $text_len i32.const 121260 i32.const 121284 i32.const 121296 call $er_ui_text_component_measure_value drop
    i32.const 121308 f32.load)

  (func (export "er_ui_alert_title_bounds") (param $bounds i32) (param $title_ptr i32) (param $title_len i32) (param $out i32) (result i32)
    (local $text_w f32) (local $title_h f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds call $er_ui_alert_text_width local.set $text_w
    local.get $title_ptr local.get $title_len local.get $text_w f32.const 16 i32.const 2 call $er_ui_alert_measured_text_height local.set $title_h
    local.get $out
    local.get $bounds f32.load f32.const 44 f32.add
    local.get $bounds i32.const 4 i32.add f32.load f32.const 11 f32.add
    local.get $text_w
    local.get $title_h
    call $rect_store)

  (func (export "er_ui_alert_detail_bounds") (param $bounds i32) (param $title_ptr i32) (param $title_len i32) (param $out i32) (result i32)
    (local $text_w f32) (local $title_h f32) (local $y f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds call $er_ui_alert_text_width local.set $text_w
    local.get $title_ptr local.get $title_len local.get $text_w f32.const 16 i32.const 2 call $er_ui_alert_measured_text_height local.set $title_h
    local.get $bounds i32.const 4 i32.add f32.load f32.const 12 f32.add local.get $title_h f32.add f32.const 2 f32.add local.set $y
    local.get $out
    local.get $bounds f32.load f32.const 44 f32.add
    local.get $y
    local.get $text_w
    local.get $bounds i32.const 12 i32.add f32.load f32.const 24 f32.sub local.get $title_h f32.sub f32.const 1 call $max_f32
    call $rect_store)

  (func (export "er_ui_alert_measure") (param $title_ptr i32) (param $title_len i32) (param $detail_ptr i32) (param $detail_len i32) (param $constraints i32) (param $out i32) (result i32)
    (local $title_w f32) (local $title_h f32) (local $detail_w f32) (local $detail_h f32) (local $raw_w f32) (local $raw_h f32) (local $pref_w f32) (local $pref_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    i32.const 121324 f32.const 44 f32.store
    i32.const 121328 f32.const 0 f32.store
    i32.const 121332 f32.const 16 f32.store
    i32.const 121336 f32.const 0 f32.store
    local.get $constraints i32.const 121324 i32.const 121340 call $er_ui_layout_constraints_inner drop
    i32.const 121364 f32.const 16 f32.store
    i32.const 121368 local.get $title_ptr local.get $title_len f32.const 16 call $primitives_average_width f32.store
    i32.const 121372 i32.const 2 i32.store
    local.get $title_ptr local.get $title_len i32.const 121340 i32.const 121364 i32.const 121376 call $er_ui_text_component_measure_value drop
    i32.const 121384 f32.load local.set $title_w
    i32.const 121388 f32.load local.set $title_h
    i32.const 121364 f32.const 16 f32.store
    i32.const 121368 local.get $detail_ptr local.get $detail_len f32.const 16 call $primitives_average_width f32.store
    i32.const 121372 i32.const 2 i32.store
    local.get $detail_ptr local.get $detail_len i32.const 121340 i32.const 121364 i32.const 121376 call $er_ui_text_component_measure_value drop
    i32.const 121384 f32.load local.set $detail_w
    i32.const 121388 f32.load local.set $detail_h
    f32.const 160 local.get $title_w local.get $detail_w call $max_f32 f32.const 60 f32.add call $max_f32 local.set $raw_w
    f32.const 24
    f32.const 16
    local.get $title_h f32.const 2 f32.add local.get $detail_h f32.add
    call $max_f32
    f32.add
    local.set $raw_h
    local.get $raw_w local.get $raw_h local.get $constraints i32.const 121408 call $er_ui_primitives_constrain_preferred_size drop
    i32.const 121408 f32.load local.set $pref_w
    i32.const 121412 f32.load local.set $pref_h
    f32.const 160 local.get $pref_w call $min_f32
    f32.const 48 local.get $pref_h call $min_f32
    local.get $pref_w local.get $pref_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit
    local.get $pref_h
    local.get $out
    call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)
