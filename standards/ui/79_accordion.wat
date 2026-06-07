(func (export "er_ui_accordion_id_stride") (result i32) i32.const 2)
  (func (export "er_ui_accordion_trigger_h") (result f32) f32.const 36)
  (func (export "er_ui_accordion_trigger_text_y") (result f32) f32.const 10)
  (func (export "er_ui_accordion_icon_space") (result f32) f32.const 22)
  (func (export "er_ui_accordion_icon_size") (result f32) f32.const 14)
  (func (export "er_ui_accordion_icon_y") (result f32) f32.const 11)
  (func (export "er_ui_accordion_content_padding_top") (result f32) f32.const 8)
  (func (export "er_ui_accordion_detail_height") (result f32) f32.const 16)
  (func (export "er_ui_accordion_detail_average_w") (result f32) f32.const 7.5)
  (func (export "er_ui_accordion_detail_max_lines") (result i32) i32.const 2)
  (func (export "er_ui_accordion_title_max_lines") (result i32) i32.const 1)
  (func (export "er_ui_accordion_separator_height") (result f32) f32.const 1)
  (func (export "er_ui_accordion_closed_height") (result f32) f32.const 37)

  (func (export "er_ui_accordion_open_height") (param $detail_preferred_h f32) (result f32)
    f32.const 45 local.get $detail_preferred_h f32.add)

  (func (export "er_ui_accordion_encoded_id") (param $id i32) (param $open i32) (result i32)
    local.get $id i32.const 2 i32.mul
    local.get $open i32.const 0 i32.ne
    i32.add)

  (func $er_ui_accordion_trigger_bounds (export "er_ui_accordion_trigger_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $bounds f32.load
    local.get $bounds i32.const 4 i32.add f32.load
    local.get $bounds i32.const 8 i32.add f32.load
    f32.const 36
    call $rect_store)

  (func (export "er_ui_accordion_title_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 121748 call $er_ui_accordion_trigger_bounds drop
    local.get $out
    i32.const 121748 f32.load
    i32.const 121748 i32.const 4 i32.add f32.load f32.const 10 f32.add
    i32.const 121748 i32.const 8 i32.add f32.load f32.const 22 f32.sub f32.const 1 call $max_f32
    f32.const 16
    call $rect_store)

  (func (export "er_ui_accordion_icon_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 121748 call $er_ui_accordion_trigger_bounds drop
    local.get $out
    i32.const 121748 f32.load i32.const 121748 i32.const 8 i32.add f32.load f32.add f32.const 14 f32.sub
    i32.const 121748 i32.const 4 i32.add f32.load f32.const 11 f32.add
    f32.const 14
    f32.const 14
    call $rect_store)

  (func (export "er_ui_accordion_separator_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 121748 call $er_ui_accordion_trigger_bounds drop
    local.get $out
    local.get $bounds f32.load
    i32.const 121748 i32.const 4 i32.add f32.load i32.const 121748 i32.const 12 i32.add f32.load f32.add
    local.get $bounds i32.const 8 i32.add f32.load
    f32.const 1
    call $rect_store)

  (func (export "er_ui_accordion_detail_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 121748 call $er_ui_accordion_trigger_bounds drop
    local.get $out
    local.get $bounds f32.load
    i32.const 121748 i32.const 4 i32.add f32.load i32.const 121748 i32.const 12 i32.add f32.load f32.add f32.const 8 f32.add
    local.get $bounds i32.const 8 i32.add f32.load
    local.get $bounds i32.const 12 i32.add f32.load f32.const 44 f32.sub f32.const 1 call $max_f32
    call $rect_store)

  (func $er_ui_accordion_measure_detail (param $detail_ptr i32) (param $detail_len i32) (param $constraints i32) (param $out i32) (result i32)
    i32.const 121764 f32.const 16 f32.store
    i32.const 121768 f32.const 7.5 f32.store
    i32.const 121772 i32.const 2 i32.store
    local.get $detail_ptr local.get $detail_len local.get $constraints i32.const 121764 local.get $out call $er_ui_text_component_measure_value)

  (func (export "er_ui_accordion_measure") (param $title_ptr i32) (param $title_len i32) (param $detail_ptr i32) (param $detail_len i32) (param $open i32) (param $constraints i32) (param $out i32) (result i32)
    (local $title_w f32) (local $detail_h f32) (local $closed_h f32) (local $open_h f32) (local $pref_w f32) (local $pref_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $title_ptr local.get $title_len f32.const 16 call $er_ui_font_text_width local.set $title_w
    local.get $detail_ptr local.get $detail_len local.get $constraints i32.const 121776 call $er_ui_accordion_measure_detail drop
    i32.const 121788 f32.load local.set $detail_h
    f32.const 37 local.set $closed_h
    local.get $closed_h f32.const 8 f32.add local.get $detail_h f32.add local.set $open_h
    local.get $title_w f32.const 22 f32.add
    local.get $open
    if (result f32)
      local.get $open_h
    else
      local.get $closed_h
    end
    local.get $constraints
    i32.const 121800
    call $er_ui_primitives_constrain_preferred_size drop
    i32.const 121800 f32.load local.set $pref_w
    i32.const 121804 f32.load local.set $pref_h
    f32.const 23
    local.get $closed_h
    local.get $pref_w local.get $pref_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit
    local.get $pref_h local.get $open_h call $max_f32
    local.get $out
    call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)
