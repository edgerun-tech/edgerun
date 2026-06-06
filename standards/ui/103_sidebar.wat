  (func (export "er_ui_sidebar_rail_w") (result f32) f32.const 62)
  (func (export "er_ui_sidebar_content_gap") (result f32) f32.const 10)
  (func (export "er_ui_sidebar_radius") (result f32) f32.const 8)
  (func (export "er_ui_sidebar_trigger_x") (result f32) f32.const 17)
  (func (export "er_ui_sidebar_trigger_y") (result f32) f32.const 8)
  (func (export "er_ui_sidebar_trigger_size") (result f32) f32.const 28)
  (func (export "er_ui_sidebar_title_y") (result f32) f32.const 42)
  (func (export "er_ui_sidebar_title_h") (result f32) f32.const 12)
  (func (export "er_ui_sidebar_title_max_lines") (result i32) i32.const 2)
  (func (export "er_ui_sidebar_item_x") (result f32) f32.const 6)
  (func (export "er_ui_sidebar_item_y") (result f32) f32.const 66)
  (func (export "er_ui_sidebar_item_h") (result f32) f32.const 20)
  (func (export "er_ui_sidebar_item_bottom_padding") (result f32) f32.const 10)
  (func (export "er_ui_sidebar_item_radius") (result f32) f32.const 4)
  (func (export "er_ui_sidebar_item_padding") (result f32) f32.const 5)
  (func (export "er_ui_sidebar_item_text_h") (result f32) f32.const 12)
  (func (export "er_ui_sidebar_item_max_lines") (result i32) i32.const 2)
  (func (export "er_ui_sidebar_content_min_w") (result f32) f32.const 120)
  (func (export "er_ui_sidebar_min_width") (result f32) f32.const 160)
  (func (export "er_ui_sidebar_min_height") (result f32) f32.const 48)

  (func (export "er_ui_sidebar_trigger_id") (param $id i32) (result i32)
    local.get $id call $er_ui_primitives_overlay_trigger_id)

  (func (export "er_ui_sidebar_item_id") (param $id i32) (result i32)
    local.get $id i32.const 1 call $er_ui_primitives_overlay_indexed_id)

  (func (export "er_ui_sidebar_title_inner_width") (result f32)
    f32.const 50)

  (func (export "er_ui_sidebar_item_inner_width") (result f32)
    f32.const 40)

  (func (export "er_ui_sidebar_rail_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $bounds f32.load
    local.get $bounds i32.const 4 i32.add f32.load
    local.get $bounds i32.const 8 i32.add f32.load f32.const 62 call $min_f32
    local.get $bounds i32.const 12 i32.add f32.load
    call $rect_store)

  (func $er_ui_sidebar_trigger_bounds (export "er_ui_sidebar_trigger_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $bounds f32.load f32.const 17 f32.add
    local.get $bounds i32.const 4 i32.add f32.load f32.const 8 f32.add
    f32.const 28
    f32.const 28
    call $rect_store)

  (func $er_ui_sidebar_title_height (export "er_ui_sidebar_title_height") (param $title_ptr i32) (param $title_len i32) (result f32)
    local.get $title_ptr local.get $title_len f32.const 50 f32.const 12 i32.const 2 call $er_ui_alert_measured_text_height)

  (func $er_ui_sidebar_item_text_height (export "er_ui_sidebar_item_text_height") (param $item_ptr i32) (param $item_len i32) (result f32)
    local.get $item_ptr local.get $item_len f32.const 40 f32.const 12 i32.const 2 call $er_ui_alert_measured_text_height)

  (func (export "er_ui_sidebar_title_bounds") (param $bounds i32) (param $title_ptr i32) (param $title_len i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $bounds f32.load f32.const 6 f32.add
    local.get $bounds i32.const 4 i32.add f32.load f32.const 42 f32.add
    f32.const 50
    local.get $title_ptr local.get $title_len call $er_ui_sidebar_title_height
    call $rect_store)

  (func $er_ui_sidebar_item_bounds (export "er_ui_sidebar_item_bounds") (param $bounds i32) (param $title_ptr i32) (param $title_len i32) (param $item_ptr i32) (param $item_len i32) (param $out i32) (result i32)
    (local $title_h f32) (local $text_h f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $title_ptr local.get $title_len call $er_ui_sidebar_title_height local.set $title_h
    local.get $item_ptr local.get $item_len call $er_ui_sidebar_item_text_height local.set $text_h
    local.get $out
    local.get $bounds f32.load f32.const 6 f32.add
    local.get $bounds i32.const 4 i32.add f32.load f32.const 66 f32.add local.get $title_h f32.add f32.const 12 f32.sub
    f32.const 50
    f32.const 20 local.get $text_h f32.const 10 f32.add call $max_f32
    call $rect_store)

  (func (export "er_ui_sidebar_item_text_bounds") (param $item_bounds i32) (param $item_ptr i32) (param $item_len i32) (param $out i32) (result i32)
    (local $w f32) (local $h f32)
    local.get $item_bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $item_bounds i32.const 8 i32.add f32.load f32.const 10 f32.sub f32.const 1 call $max_f32 local.set $w
    local.get $item_ptr local.get $item_len local.get $w f32.const 12 i32.const 2 call $er_ui_alert_measured_text_height
    local.get $item_bounds i32.const 12 i32.add f32.load
    call $min_f32
    local.set $h
    local.get $out
    local.get $item_bounds f32.load f32.const 5 f32.add
    local.get $item_bounds i32.const 4 i32.add f32.load local.get $item_bounds i32.const 12 i32.add f32.load local.get $h f32.sub f32.const 0.5 f32.mul f32.add
    local.get $w
    local.get $h
    call $rect_store)

  (func (export "er_ui_sidebar_content_bounds") (param $bounds i32) (param $out i32) (result i32)
    (local $x f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds f32.load f32.const 72 f32.add local.set $x
    local.get $out
    local.get $x
    local.get $bounds i32.const 4 i32.add f32.load
    local.get $bounds f32.load local.get $bounds i32.const 8 i32.add f32.load f32.add local.get $x f32.sub f32.const 1 call $max_f32
    local.get $bounds i32.const 12 i32.add f32.load
    call $rect_store)

  (func (export "er_ui_sidebar_measure") (param $title_ptr i32) (param $title_len i32) (param $item_ptr i32) (param $item_len i32) (param $constraints i32) (param $out i32) (result i32)
    (local $title_h f32) (local $item_h f32) (local $rail_h f32) (local $pref_w f32) (local $pref_h f32) (local $max_w f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $title_ptr local.get $title_len call $er_ui_sidebar_title_height local.set $title_h
    local.get $item_ptr local.get $item_len call $er_ui_sidebar_item_text_height f32.const 10 f32.add f32.const 20 call $max_f32 local.set $item_h
    f32.const 66 local.get $title_h f32.add f32.const 12 f32.sub local.get $item_h f32.add f32.const 10 f32.add local.set $rail_h
    f32.const 192 local.get $rail_h f32.const 48 call $max_f32 local.get $constraints i32.const 124120 call $er_ui_primitives_constrain_preferred_size drop
    i32.const 124120 f32.load local.set $pref_w
    i32.const 124124 f32.load local.set $pref_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit local.set $max_w
    f32.const 160 local.get $pref_w call $min_f32
    f32.const 48 local.get $pref_h call $min_f32
    local.get $pref_w local.get $pref_h local.get $max_w local.get $pref_h local.get $out call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)
