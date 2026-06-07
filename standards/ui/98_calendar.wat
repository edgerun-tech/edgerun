(func (export "er_ui_calendar_day_count") (result i32) i32.const 28)
  (func (export "er_ui_calendar_day_id_offset") (result i32) i32.const 2)
  (func (export "er_ui_calendar_column_count") (result i32) i32.const 7)
  (func (export "er_ui_calendar_row_count") (result f32) f32.const 4)
  (func (export "er_ui_calendar_radius") (result f32) f32.const 8)
  (func (export "er_ui_calendar_padding") (result f32) f32.const 8)
  (func (export "er_ui_calendar_nav_size") (result f32) f32.const 24)
  (func (export "er_ui_calendar_caption_h") (result f32) f32.const 24)
  (func (export "er_ui_calendar_weekday_y") (result f32) f32.const 36)
  (func (export "er_ui_calendar_weekday_h") (result f32) f32.const 16)
  (func (export "er_ui_calendar_grid_y") (result f32) f32.const 56)
  (func (export "er_ui_calendar_cell_size") (result f32) f32.const 22)
  (func (export "er_ui_calendar_cell_gap") (result f32) f32.const 2)
  (func (export "er_ui_calendar_day_text_h") (result f32) f32.const 12)
  (func (export "er_ui_calendar_day_text_padding") (result f32) f32.const 2)

  (func (export "er_ui_calendar_intrinsic_width") (result f32)
    f32.const 170)

  (func (export "er_ui_calendar_intrinsic_height") (result f32)
    f32.const 152)

  (func (export "er_ui_calendar_nav_id") (param $id i32) (param $index i32) (result i32)
    local.get $id local.get $index i32.const 0 i32.const 1 call $clamp_i32 i32.add)

  (func (export "er_ui_calendar_day_id") (param $id i32) (param $index i32) (result i32)
    local.get $id i32.const 2 i32.add local.get $index i32.const 0 i32.const 27 call $clamp_i32 i32.add)

  (func (export "er_ui_calendar_nav_bounds") (param $bounds i32) (param $index i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $index i32.const 0 i32.eq
    if
      local.get $out
      local.get $bounds f32.load f32.const 8 f32.add
      local.get $bounds i32.const 4 i32.add f32.load f32.const 8 f32.add
      f32.const 24
      f32.const 24
      call $rect_store
      return
    end
    local.get $out
    local.get $bounds f32.load local.get $bounds i32.const 8 i32.add f32.load f32.add f32.const 32 f32.sub
    local.get $bounds i32.const 4 i32.add f32.load f32.const 8 f32.add
    f32.const 24
    f32.const 24
    call $rect_store)

  (func (export "er_ui_calendar_caption_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $bounds f32.load f32.const 40 f32.add
    local.get $bounds i32.const 4 i32.add f32.load f32.const 8 f32.add
    local.get $bounds i32.const 8 i32.add f32.load f32.const 80 f32.sub f32.const 1 call $max_f32
    f32.const 24
    call $rect_store)

  (func $er_ui_calendar_grid_bounds (export "er_ui_calendar_grid_bounds") (param $bounds i32) (param $out i32) (result i32)
    (local $grid_w f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    f32.const 166 local.set $grid_w
    local.get $out
    local.get $bounds f32.load local.get $bounds i32.const 8 i32.add f32.load local.get $grid_w f32.sub f32.const 0.5 f32.mul f32.add
    local.get $bounds i32.const 4 i32.add f32.load f32.const 56 f32.add
    local.get $grid_w
    local.get $bounds i32.const 12 i32.add f32.load f32.const 64 f32.sub f32.const 1 call $max_f32
    call $rect_store)

  (func (export "er_ui_calendar_weekday_bounds") (param $bounds i32) (param $index i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 123360 call $er_ui_calendar_grid_bounds drop
    local.get $out
    i32.const 123360 f32.load local.get $index i32.const 0 i32.const 6 call $clamp_i32 f32.convert_i32_u f32.const 24 f32.mul f32.add
    local.get $bounds i32.const 4 i32.add f32.load f32.const 36 f32.add
    f32.const 22
    f32.const 16
    call $rect_store)

  (func $er_ui_calendar_day_bounds (export "er_ui_calendar_day_bounds") (param $bounds i32) (param $index i32) (param $out i32) (result i32)
    (local $idx i32) (local $col i32) (local $row i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $index i32.const 0 i32.const 27 call $clamp_i32 local.set $idx
    local.get $idx i32.const 7 i32.rem_u local.set $col
    local.get $idx i32.const 7 i32.div_u local.set $row
    local.get $bounds i32.const 123376 call $er_ui_calendar_grid_bounds drop
    local.get $out
    i32.const 123376 f32.load local.get $col f32.convert_i32_u f32.const 24 f32.mul f32.add
    i32.const 123376 i32.const 4 i32.add f32.load local.get $row f32.convert_i32_u f32.const 24 f32.mul f32.add
    f32.const 22
    f32.const 22
    call $rect_store)

  (func (export "er_ui_calendar_day_text_bounds") (param $bounds i32) (param $index i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds local.get $index i32.const 123392 call $er_ui_calendar_day_bounds drop
    i32.const 123392 f32.const 2 i32.const 123408 call $er_ui_primitives_content_inset drop
    i32.const 123408 local.get $out f32.const 12 call $er_ui_rect_with_height_centered)

  (func (export "er_ui_calendar_measure") (param $constraints i32) (param $out i32) (result i32)
    (local $pref_w f32) (local $pref_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    f32.const 170 f32.const 152 local.get $constraints i32.const 123424 call $er_ui_primitives_constrain_preferred_size drop
    i32.const 123424 f32.load local.set $pref_w
    i32.const 123428 f32.load local.set $pref_h
    local.get $pref_w local.get $pref_h local.get $pref_w local.get $pref_h local.get $pref_w local.get $pref_h local.get $out call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)
