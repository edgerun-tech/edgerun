  (func (export "er_ui_table_radius") (result f32) f32.const 6)
  (func (export "er_ui_table_padding_x") (result f32) f32.const 8)
  (func (export "er_ui_table_header_h") (result f32) f32.const 24)
  (func (export "er_ui_table_header_y") (result f32) f32.const 5)
  (func (export "er_ui_table_header_text_h") (result f32) f32.const 14)
  (func (export "er_ui_table_body_y") (result f32) f32.const 35)
  (func (export "er_ui_table_body_text_h") (result f32) f32.const 14)
  (func (export "er_ui_table_name_column_ratio") (result f32) f32.const 0.55)
  (func (export "er_ui_table_row_inset") (result f32) f32.const 4)
  (func (export "er_ui_table_row_radius") (result f32) f32.const 4)
  (func (export "er_ui_table_min_width") (result f32) f32.const 160)
  (func (export "er_ui_table_min_height") (result f32) f32.const 48)
  (func (export "er_ui_table_separator_height") (result f32) f32.const 1)

  (func (export "er_ui_table_row_id") (param $id i32) (result i32)
    local.get $id)

  (func (export "er_ui_table_name_header_id") (param $id i32) (result i32)
    local.get $id i32.const 1 i32.add)

  (func (export "er_ui_table_role_header_id") (param $id i32) (result i32)
    local.get $id i32.const 2 i32.add)

  (func (export "er_ui_table_column_index") (param $column i32) (result i32)
    local.get $column i32.eqz
    if i32.const 0 return end
    i32.const 1)

  (func $er_ui_table_cell_bounds (export "er_ui_table_cell_bounds") (param $bounds i32) (param $column i32) (param $y_offset f32) (param $height f32) (param $out i32) (result i32)
    (local $left_w f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 8 i32.add f32.load f32.const 0.55 f32.mul local.set $left_w
    local.get $column i32.eqz
    if
      local.get $out
      local.get $bounds f32.load f32.const 8 f32.add
      local.get $bounds i32.const 4 i32.add f32.load local.get $y_offset f32.add
      local.get $left_w f32.const 8 f32.sub f32.const 1 call $max_f32
      local.get $height
      call $rect_store
      return
    end
    local.get $out
    local.get $bounds f32.load local.get $left_w f32.add
    local.get $bounds i32.const 4 i32.add f32.load local.get $y_offset f32.add
    local.get $bounds i32.const 8 i32.add f32.load local.get $left_w f32.sub f32.const 8 f32.sub f32.const 1 call $max_f32
    local.get $height
    call $rect_store)

  (func $er_ui_table_row_bounds (export "er_ui_table_row_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $bounds f32.load
    local.get $bounds i32.const 4 i32.add f32.load f32.const 25 f32.add
    local.get $bounds i32.const 8 i32.add f32.load
    local.get $bounds i32.const 12 i32.add f32.load f32.const 25 f32.sub f32.const 1 call $max_f32
    call $rect_store)

  (func (export "er_ui_table_row_fill_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 124736 call $er_ui_table_row_bounds drop
    i32.const 124736 f32.const 4 local.get $out call $er_ui_primitives_content_inset)

  (func $er_ui_table_text_height (export "er_ui_table_text_height") (param $bounds i32) (param $column i32) (param $ptr i32) (param $len i32) (param $line_h f32) (param $max_lines i32) (result f32)
    (local $text_w f32) (local $avg f32) (local $lines i32)
    local.get $bounds i32.eqz local.get $ptr i32.eqz i32.or local.get $len i32.eqz i32.or local.get $max_lines i32.eqz i32.or
    if f32.const 0 return end
    local.get $bounds local.get $column f32.const 0 local.get $line_h i32.const 124752 call $er_ui_table_cell_bounds drop
    i32.const 124760 f32.load local.set $text_w
    local.get $ptr local.get $len local.get $line_h call $er_ui_font_text_width
    local.get $len f32.convert_i32_u f32.div
    f32.const 1 call $max_f32
    local.set $avg
    local.get $ptr local.get $len local.get $text_w local.get $avg local.get $max_lines call $er_ui_text_wrapped_line_count local.set $lines
    local.get $bounds i32.const 12 i32.add f32.load
    local.get $lines i32.const 1 call $layout_max_i32_u f32.convert_i32_u local.get $line_h f32.mul
    call $min_f32)

  (func (export "er_ui_table_header_bounds") (param $bounds i32) (param $column i32) (param $label_ptr i32) (param $label_len i32) (param $out i32) (result i32)
    (local $h f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds local.get $column local.get $label_ptr local.get $label_len f32.const 14 i32.const 1 call $er_ui_table_text_height local.set $h
    local.get $bounds local.get $column f32.const 5 local.get $h local.get $out call $er_ui_table_cell_bounds)

  (func (export "er_ui_table_body_cell_bounds") (param $bounds i32) (param $column i32) (param $value_ptr i32) (param $value_len i32) (param $out i32) (result i32)
    (local $h f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds local.get $column local.get $value_ptr local.get $value_len f32.const 14 i32.const 2 call $er_ui_table_text_height local.set $h
    local.get $bounds local.get $column f32.const 35 local.get $h local.get $out call $er_ui_table_cell_bounds)

  (func (export "er_ui_table_measure_preferred") (param $name_ptr i32) (param $name_len i32) (param $role_ptr i32) (param $role_len i32) (param $width f32) (param $out i32) (result i32)
    (local $w f32) (local $name_w f32) (local $role_w f32) (local $name_h f32) (local $role_h f32) (local $row_h f32) (local $pref_w f32)
    local.get $out i32.eqz
    if i32.const 0 return end
    local.get $width f32.const 160 call $max_f32 local.set $w
    local.get $w f32.const 0.55 f32.mul f32.const 8 f32.sub f32.const 1 call $max_f32 local.set $name_w
    local.get $w f32.const 0.45 f32.mul f32.const 8 f32.sub f32.const 1 call $max_f32 local.set $role_w
    local.get $name_ptr local.get $name_len local.get $name_w
    local.get $name_ptr local.get $name_len f32.const 14 call $er_ui_font_text_width local.get $name_len f32.convert_i32_u f32.div f32.const 1 call $max_f32
    i32.const 2 call $er_ui_text_wrapped_line_count i32.const 1 call $layout_max_i32_u f32.convert_i32_u f32.const 14 f32.mul local.set $name_h
    local.get $role_ptr local.get $role_len local.get $role_w
    local.get $role_ptr local.get $role_len f32.const 14 call $er_ui_font_text_width local.get $role_len f32.convert_i32_u f32.div f32.const 1 call $max_f32
    i32.const 2 call $er_ui_text_wrapped_line_count i32.const 1 call $layout_max_i32_u f32.convert_i32_u f32.const 14 f32.mul local.set $role_h
    f32.const 14 local.get $name_h local.get $role_h call $max_f32 call $max_f32 f32.const 8 f32.add local.set $row_h
    f32.const 160
    local.get $name_ptr local.get $name_len f32.const 14 call $er_ui_font_text_width
    local.get $role_ptr local.get $role_len f32.const 14 call $er_ui_font_text_width
    f32.add f32.const 16 f32.add
    call $max_f32 local.set $pref_w
    local.get $out local.get $pref_w f32.store
    local.get $out i32.const 4 i32.add f32.const 25 local.get $row_h f32.add f32.store
    i32.const 1)
