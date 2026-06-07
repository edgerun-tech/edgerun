(func $finite_f32 (param $value f32) (result i32)
    local.get $value
    i32.reinterpret_f32
    i32.const 0x7f800000
    i32.and
    i32.const 0x7f800000
    i32.ne)

  (func $rect_store (param $out i32) (param $x f32) (param $y f32) (param $w f32) (param $h f32) (result i32)
    local.get $out
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $out
    local.get $x
    f32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $y
    f32.store
    local.get $out
    i32.const 8
    i32.add
    local.get $w
    f32.store
    local.get $out
    i32.const 12
    i32.add
    local.get $h
    f32.store
    i32.const 1)

  (func (export "er_ui_rect_init") (param $out i32) (param $x f32) (param $y f32) (param $w f32) (param $h f32) (result i32)
    local.get $out
    local.get $x
    local.get $y
    local.get $w
    local.get $h
    call $rect_store)

  (func $er_ui_rect_valid (export "er_ui_rect_valid") (param $rect i32) (result i32)
    local.get $rect
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $rect f32.load call $finite_f32
    local.get $rect i32.const 4 i32.add f32.load call $finite_f32 i32.and
    local.get $rect i32.const 8 i32.add f32.load call $finite_f32 i32.and
    local.get $rect i32.const 12 i32.add f32.load call $finite_f32 i32.and
    local.get $rect i32.const 8 i32.add f32.load f32.const 0 f32.gt i32.and
    local.get $rect i32.const 12 i32.add f32.load f32.const 0 f32.gt i32.and)

  (func (export "er_ui_rect_usable") (param $rect i32) (result i32)
    local.get $rect
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $rect f32.load call $finite_f32
    local.get $rect i32.const 4 i32.add f32.load call $finite_f32 i32.and
    local.get $rect i32.const 8 i32.add f32.load call $finite_f32 i32.and
    local.get $rect i32.const 12 i32.add f32.load call $finite_f32 i32.and
    local.get $rect i32.const 8 i32.add f32.load f32.const 0 f32.ge i32.and
    local.get $rect i32.const 12 i32.add f32.load f32.const 0 f32.ge i32.and)

  (func $er_ui_rect_inset (export "er_ui_rect_inset") (param $rect i32) (param $out i32) (param $dx f32) (param $dy f32) (result i32)
    local.get $rect
    i32.eqz
    local.get $out
    i32.eqz
    i32.or
    if
      i32.const 0
      return
    end
    local.get $out
    local.get $rect f32.load local.get $dx f32.add
    local.get $rect i32.const 4 i32.add f32.load local.get $dy f32.add
    local.get $rect i32.const 8 i32.add f32.load local.get $dx f32.const 2 f32.mul f32.sub f32.const 0 call $max_f32
    local.get $rect i32.const 12 i32.add f32.load local.get $dy f32.const 2 f32.mul f32.sub f32.const 0 call $max_f32
    call $rect_store)

  (func $er_ui_rect_inset_uniform (export "er_ui_rect_inset_uniform") (param $rect i32) (param $out i32) (param $amount f32) (result i32)
    local.get $rect
    local.get $out
    local.get $amount
    local.get $amount
    call $er_ui_rect_inset)

  (func $er_ui_rect_inset_ltrb (export "er_ui_rect_inset_ltrb") (param $rect i32) (param $out i32) (param $left f32) (param $top f32) (param $right f32) (param $bottom f32) (result i32)
    local.get $rect
    i32.eqz
    local.get $out
    i32.eqz
    i32.or
    if
      i32.const 0
      return
    end
    local.get $out
    local.get $rect f32.load local.get $left f32.add
    local.get $rect i32.const 4 i32.add f32.load local.get $top f32.add
    local.get $rect i32.const 8 i32.add f32.load local.get $left f32.sub local.get $right f32.sub f32.const 0 call $max_f32
    local.get $rect i32.const 12 i32.add f32.load local.get $top f32.sub local.get $bottom f32.sub f32.const 0 call $max_f32
    call $rect_store)

  (func $er_ui_rect_with_height_centered (export "er_ui_rect_with_height_centered") (param $rect i32) (param $out i32) (param $height f32) (result i32)
    (local $h f32)
    local.get $rect i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $height f32.const 0 local.get $rect i32.const 12 i32.add f32.load call $clamp_f32 local.set $h
    local.get $out
    local.get $rect f32.load
    local.get $rect i32.const 4 i32.add f32.load local.get $rect i32.const 12 i32.add f32.load local.get $h f32.sub f32.const 0.5 f32.mul f32.add
    local.get $rect i32.const 8 i32.add f32.load
    local.get $h
    call $rect_store)

  (func (export "er_ui_rect_with_width_centered") (param $rect i32) (param $out i32) (param $width f32) (result i32)
    (local $w f32)
    local.get $rect i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $width f32.const 0 local.get $rect i32.const 8 i32.add f32.load call $clamp_f32 local.set $w
    local.get $out
    local.get $rect f32.load local.get $rect i32.const 8 i32.add f32.load local.get $w f32.sub f32.const 0.5 f32.mul f32.add
    local.get $rect i32.const 4 i32.add f32.load
    local.get $w
    local.get $rect i32.const 12 i32.add f32.load
    call $rect_store)

  (func (export "er_ui_rect_right") (param $rect i32) (param $out i32) (param $width f32) (result i32)
    local.get $rect i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $rect f32.load local.get $rect i32.const 8 i32.add f32.load f32.add local.get $width f32.sub
    local.get $rect i32.const 4 i32.add f32.load
    local.get $width
    local.get $rect i32.const 12 i32.add f32.load
    call $rect_store)

  (func (export "er_ui_rect_bottom") (param $rect i32) (param $out i32) (param $height f32) (result i32)
    local.get $rect i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $rect f32.load
    local.get $rect i32.const 4 i32.add f32.load local.get $rect i32.const 12 i32.add f32.load f32.add local.get $height f32.sub
    local.get $rect i32.const 8 i32.add f32.load
    local.get $height
    call $rect_store)

  (func (export "er_ui_rect_contains_inclusive") (param $rect i32) (param $x f32) (param $y f32) (result i32)
    local.get $rect i32.eqz if i32.const 0 return end
    local.get $x local.get $rect f32.load f32.ge
    local.get $y local.get $rect i32.const 4 i32.add f32.load f32.ge i32.and
    local.get $x local.get $rect f32.load local.get $rect i32.const 8 i32.add f32.load f32.add f32.le i32.and
    local.get $y local.get $rect i32.const 4 i32.add f32.load local.get $rect i32.const 12 i32.add f32.load f32.add f32.le i32.and)

  (func (export "er_ui_rect_contains_exclusive") (param $rect i32) (param $x f32) (param $y f32) (result i32)
    local.get $rect i32.eqz if i32.const 0 return end
    local.get $x local.get $rect f32.load f32.ge
    local.get $y local.get $rect i32.const 4 i32.add f32.load f32.ge i32.and
    local.get $x local.get $rect f32.load local.get $rect i32.const 8 i32.add f32.load f32.add f32.lt i32.and
    local.get $y local.get $rect i32.const 4 i32.add f32.load local.get $rect i32.const 12 i32.add f32.load f32.add f32.lt i32.and)

  (func (export "er_ui_geometry_clamp") (param $value f32) (param $lo f32) (param $hi f32) (result f32)
    local.get $value
    local.get $lo
    local.get $hi
    call $clamp_f32)

  (func (export "er_ui_theme_uniform_grid_size") (result i32)
    i32.const 40)

  (func (export "er_ui_theme_empty_rect") (param $out i32) (result i32)
    local.get $out
    f32.const 0
    f32.const 0
    f32.const 0
    f32.const 0
    call $rect_store)

  (func (export "er_ui_theme_uniform_grid") (param $bounds i32) (param $columns i32) (param $rows i32) (param $gap_x f32) (param $gap_y f32) (param $out i32) (result i32)
    (local $total_gap_x f32)
    (local $total_gap_y f32)
    (local $cell_w f32)
    (local $cell_h f32)
    local.get $out
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $out
    f32.const 0
    f32.const 0
    f32.const 0
    f32.const 0
    call $rect_store
    drop
    local.get $out i32.const 16 i32.add i32.const 0 i32.store
    local.get $out i32.const 20 i32.add i32.const 0 i32.store
    local.get $out i32.const 24 i32.add f32.const 0 f32.store
    local.get $out i32.const 28 i32.add f32.const 0 f32.store
    local.get $out i32.const 32 i32.add f32.const 0 f32.store
    local.get $out i32.const 36 i32.add f32.const 0 f32.store
    local.get $bounds
    call $er_ui_rect_valid
    i32.eqz
    local.get $columns
    i32.eqz
    i32.or
    local.get $rows
    i32.eqz
    i32.or
    local.get $gap_x
    f32.const 0
    f32.lt
    i32.or
    local.get $gap_y
    f32.const 0
    f32.lt
    i32.or
    if
      i32.const 0
      return
    end
    local.get $gap_x
    local.get $columns
    i32.const 1
    i32.sub
    f32.convert_i32_u
    f32.mul
    local.set $total_gap_x
    local.get $gap_y
    local.get $rows
    i32.const 1
    i32.sub
    f32.convert_i32_u
    f32.mul
    local.set $total_gap_y
    local.get $bounds
    i32.const 8
    i32.add
    f32.load
    local.get $total_gap_x
    f32.sub
    local.get $columns
    f32.convert_i32_u
    f32.div
    local.set $cell_w
    local.get $bounds
    i32.const 12
    i32.add
    f32.load
    local.get $total_gap_y
    f32.sub
    local.get $rows
    f32.convert_i32_u
    f32.div
    local.set $cell_h
    local.get $cell_w
    f32.const 0
    f32.le
    local.get $cell_h
    f32.const 0
    f32.le
    i32.or
    if
      i32.const 0
      return
    end
    local.get $out
    local.get $bounds
    f32.load
    local.get $bounds
    i32.const 4
    i32.add
    f32.load
    local.get $bounds
    i32.const 8
    i32.add
    f32.load
    local.get $bounds
    i32.const 12
    i32.add
    f32.load
    call $rect_store
    drop
    local.get $out i32.const 16 i32.add local.get $columns i32.store
    local.get $out i32.const 20 i32.add local.get $rows i32.store
    local.get $out i32.const 24 i32.add local.get $cell_w f32.store
    local.get $out i32.const 28 i32.add local.get $cell_h f32.store
    local.get $out i32.const 32 i32.add local.get $gap_x f32.store
    local.get $out i32.const 36 i32.add local.get $gap_y f32.store
    i32.const 1)

  (func (export "er_ui_theme_uniform_grid_cell") (param $grid i32) (param $index i32) (param $out i32) (result i32)
    (local $column i32)
    (local $row i32)
    (local $columns i32)
    (local $rows i32)
    local.get $out
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $out f32.const 0 f32.const 0 f32.const 0 f32.const 0 call $rect_store drop
    local.get $grid
    call $er_ui_rect_valid
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $grid i32.const 16 i32.add i32.load local.set $columns
    local.get $grid i32.const 20 i32.add i32.load local.set $rows
    local.get $columns i32.eqz
    local.get $rows i32.eqz
    i32.or
    local.get $grid i32.const 24 i32.add f32.load f32.const 0 f32.le
    i32.or
    local.get $grid i32.const 28 i32.add f32.load f32.const 0 f32.le
    i32.or
    if
      i32.const 0
      return
    end
    local.get $index
    local.get $columns
    i32.rem_u
    local.set $column
    local.get $index
    local.get $columns
    i32.div_u
    local.tee $row
    local.get $rows
    i32.ge_u
    if
      i32.const 0
      return
    end
    local.get $out
    local.get $grid f32.load
    local.get $grid i32.const 24 i32.add f32.load
    local.get $grid i32.const 32 i32.add f32.load
    f32.add
    local.get $column
    f32.convert_i32_u
    f32.mul
    f32.add
    local.get $grid i32.const 4 i32.add f32.load
    local.get $grid i32.const 28 i32.add f32.load
    local.get $grid i32.const 36 i32.add f32.load
    f32.add
    local.get $row
    f32.convert_i32_u
    f32.mul
    f32.add
    local.get $grid i32.const 24 i32.add f32.load
    local.get $grid i32.const 28 i32.add f32.load
    call $rect_store)

  (func (export "er_ui_theme_header_h") (result f32) f32.const 56)
  (func (export "er_ui_theme_content_pad") (result f32) f32.const 20)
  (func (export "er_ui_theme_content_wide") (result f32) f32.const 1180)
  (func (export "er_ui_theme_surface_radius") (result f32) f32.const 8)
  (func (export "er_ui_theme_workspace_rail_pad") (result f32) f32.const 12)
  (func (export "er_ui_theme_workspace_icon_button") (result f32) f32.const 36)
  (func (export "er_ui_theme_icon_button_box") (result f32) f32.const 34)
  (func (export "er_ui_theme_icon_logo_box") (result f32) f32.const 24)
  (func (export "er_ui_theme_icon_logo_inset") (result f32) f32.const 5)
  (func (export "er_ui_theme_type_caption_h") (result f32) f32.const 12)
  (func (export "er_ui_theme_type_body_h") (result f32) f32.const 17)
  (func (export "er_ui_theme_type_body_line_h") (result f32) f32.const 20)
  (func (export "er_ui_theme_type_section_h") (result f32) f32.const 22)
  (func (export "er_ui_theme_type_title_h") (result f32) f32.const 26)
  (func (export "er_ui_theme_type_title_line_h") (result f32) f32.const 46)
  (func (export "er_ui_theme_type_code_h") (result f32) f32.const 13)
  (func (export "er_ui_theme_type_average_body_w") (result f32) f32.const 8.8)
  (func (export "er_ui_theme_component_control_radius") (result f32) f32.const 6)
  (func (export "er_ui_theme_component_focus_ring_outset") (result f32) f32.const 2)
  (func (export "er_ui_theme_component_state_loading_h") (result f32) f32.const 3)
  (func (export "er_ui_theme_component_row_radius") (result f32) f32.const 4)
  (func (export "er_ui_theme_component_control_text_padding") (result f32) f32.const 12)
  (func (export "er_ui_theme_component_control_label_height") (result f32) f32.const 16)
  (func (export "er_ui_theme_component_control_average_char_width") (result f32) f32.const 8.5)
  (func (export "er_ui_theme_component_surface_padding") (result f32) f32.const 16)
  (func (export "er_ui_theme_component_surface_title_height") (result f32) f32.const 18)
  (func (export "er_ui_theme_component_surface_detail_height") (result f32) f32.const 16)
  (func (export "er_ui_theme_component_surface_detail_gap") (result f32) f32.const 8)
  (func (export "er_ui_theme_component_badge_height") (result f32) f32.const 24)
  (func (export "er_ui_theme_component_badge_text_height") (result f32) f32.const 13)
  (func (export "er_ui_theme_component_badge_padding_x") (result f32) f32.const 12)

  (func (export "er_ui_theme_palette_bg") (result i32) i32.const 0xff110e0c)
  (func (export "er_ui_theme_palette_panel") (result i32) i32.const 0xff1e1916)
  (func (export "er_ui_theme_palette_panel_alt") (result i32) i32.const 0xff26201c)
  (func (export "er_ui_theme_palette_panel_floor") (result i32) i32.const 0xff16120f)
  (func (export "er_ui_theme_palette_code_bg") (result i32) i32.const 0xff0c0907)
  (func (export "er_ui_theme_palette_row") (result i32) i32.const 0xff2a231f)
  (func (export "er_ui_theme_palette_border") (result i32) i32.const 0xff52463f)
  (func (export "er_ui_theme_palette_text") (result i32) i32.const 0xfff8f3ef)
  (func (export "er_ui_theme_palette_muted") (result i32) i32.const 0xffb4a89e)
  (func (export "er_ui_theme_palette_dim") (result i32) i32.const 0xff8a7b6f)
  (func (export "er_ui_theme_palette_active") (result i32) i32.const 0xffd6701c)
  (func (export "er_ui_theme_palette_accent") (result i32) i32.const 0xffb6d635)
  (func (export "er_ui_theme_palette_danger") (result i32) i32.const 0xff7171f8)
  (func (export "er_ui_theme_palette_yellow") (result i32) i32.const 0xff15ccfa)
  (func (export "er_ui_theme_palette_cyan") (result i32) i32.const 0xffeed322)
  (func (export "er_ui_theme_state_hover_border") (result i32) i32.const 0xfffcd37d)
  (func (export "er_ui_theme_state_active_border") (result i32) i32.const 0xffeed322)
  (func (export "er_ui_theme_state_focus_border") (result i32) i32.const 0xff15ccfa)
  (func (export "er_ui_theme_state_invalid_border") (result i32) i32.const 0xff7171f8)
  (func (export "er_ui_theme_state_disabled_tint") (result i32) i32.const 0x8e140e0a)
  (func (export "er_ui_theme_state_loading_fill") (result i32) i32.const 0xffbfd42d)
