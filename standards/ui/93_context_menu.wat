(func (export "er_ui_context_menu_trigger_label_len") (result i32) i32.const 7)
  (func (export "er_ui_context_menu_item_count") (result i32) i32.const 2)
  (func (export "er_ui_context_menu_trigger_y") (result f32) f32.const 4)
  (func (export "er_ui_context_menu_trigger_w") (result f32) f32.const 64)
  (func (export "er_ui_context_menu_trigger_h") (result f32) f32.const 30)
  (func (export "er_ui_context_menu_gap") (result f32) f32.const 8)
  (func (export "er_ui_context_menu_radius") (result f32) f32.const 8)
  (func (export "er_ui_context_menu_trigger_padding") (result f32) f32.const 8)
  (func (export "er_ui_context_menu_list_padding") (result f32) f32.const 5)
  (func (export "er_ui_context_menu_item_h") (result f32) f32.const 14)
  (func (export "er_ui_context_menu_item_pitch") (result f32) f32.const 16)
  (func (export "er_ui_context_menu_item_radius") (result f32) f32.const 4)
  (func (export "er_ui_context_menu_item_padding") (result f32) f32.const 5)
  (func (export "er_ui_context_menu_item_text_h") (result f32) f32.const 12)
  (func (export "er_ui_context_menu_label_max_lines") (result i32) i32.const 1)

  (func $er_ui_context_menu_write_trigger_label (result i32)
    i32.const 122936 i32.const 67 i32.store8
    i32.const 122937 i32.const 111 i32.store8
    i32.const 122938 i32.const 110 i32.store8
    i32.const 122939 i32.const 116 i32.store8
    i32.const 122940 i32.const 101 i32.store8
    i32.const 122941 i32.const 120 i32.store8
    i32.const 122942 i32.const 116 i32.store8
    i32.const 122936)

  (func $er_ui_context_menu_panel_layout (param $out i32) (result i32)
    local.get $out f32.const 4 f32.const 64 f32.const 30 f32.const 8
    call $er_ui_primitives_side_panel_layout)

  (func $er_ui_context_menu_list_layout (param $out i32) (result i32)
    local.get $out f32.const 5 f32.const 14 f32.const 16 f32.const 4 f32.const 5 f32.const 12
    call $er_ui_primitives_menu_list_layout)

  (func (export "er_ui_context_menu_trigger_id") (param $id i32) (result i32)
    local.get $id call $er_ui_primitives_overlay_trigger_id)

  (func (export "er_ui_context_menu_item_id") (param $id i32) (param $index i32) (result i32)
    local.get $id local.get $index i32.const 0 i32.const 1 call $clamp_i32 call $er_ui_primitives_overlay_indexed_id)

  (func $er_ui_context_menu_trigger_bounds (export "er_ui_context_menu_trigger_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    i32.const 122948 call $er_ui_context_menu_panel_layout drop
    local.get $bounds i32.const 122948 local.get $out call $er_ui_primitives_side_panel_trigger_bounds)

  (func $er_ui_context_menu_content_bounds (export "er_ui_context_menu_content_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    i32.const 122948 call $er_ui_context_menu_panel_layout drop
    local.get $bounds i32.const 122948 local.get $out call $er_ui_primitives_side_panel_content_bounds)

  (func $er_ui_context_menu_item_bounds (export "er_ui_context_menu_item_bounds") (param $bounds i32) (param $index i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 122964 call $er_ui_context_menu_content_bounds drop
    i32.const 122980 call $er_ui_context_menu_list_layout drop
    i32.const 122964 local.get $index i32.const 0 i32.const 1 call $clamp_i32 i32.const 122980 local.get $out
    call $er_ui_primitives_menu_item_bounds)

  (func (export "er_ui_context_menu_trigger_text_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 123004 call $er_ui_context_menu_trigger_bounds drop
    i32.const 123004 f32.const 8 i32.const 123020 call $er_ui_primitives_content_inset drop
    i32.const 123020 local.get $out f32.const 16 call $er_ui_rect_with_height_centered)

  (func (export "er_ui_context_menu_item_text_bounds") (param $bounds i32) (param $index i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds local.get $index i32.const 123036 call $er_ui_context_menu_item_bounds drop
    i32.const 123036 f32.const 5 i32.const 123052 call $er_ui_primitives_content_inset drop
    i32.const 123052 local.get $out f32.const 12 call $er_ui_rect_with_height_centered)

  (func (export "er_ui_context_menu_measure_two_item_panel") (param $first_ptr i32) (param $first_len i32) (param $second_ptr i32) (param $second_len i32) (param $constraints i32) (param $out i32) (result i32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    i32.const 122980 call $er_ui_context_menu_list_layout drop
    local.get $first_ptr local.get $first_len local.get $second_ptr local.get $second_len local.get $constraints i32.const 122980 local.get $out
    call $er_ui_primitives_measure_two_item_menu_panel)

  (func (export "er_ui_context_menu_measure") (param $first_ptr i32) (param $first_len i32) (param $second_ptr i32) (param $second_len i32) (param $constraints i32) (param $out i32) (result i32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    i32.const 122948 call $er_ui_context_menu_panel_layout drop
    i32.const 122980 call $er_ui_context_menu_list_layout drop
    call $er_ui_context_menu_write_trigger_label i32.const 7
    local.get $first_ptr local.get $first_len
    local.get $second_ptr local.get $second_len
    local.get $constraints
    i32.const 122948
    f32.const 8
    i32.const 122980
    local.get $out
    call $er_ui_primitives_measure_side_panel_menu)
