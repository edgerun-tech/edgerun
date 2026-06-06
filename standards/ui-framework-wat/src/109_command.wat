  (func (export "er_ui_command_radius") (result f32) f32.const 8)
  (func (export "er_ui_command_input_h") (result f32) f32.const 36)
  (func (export "er_ui_command_icon_x") (result f32) f32.const 8)
  (func (export "er_ui_command_icon_size") (result f32) f32.const 14)
  (func (export "er_ui_command_text_x") (result f32) f32.const 28)
  (func (export "er_ui_command_padding_x") (result f32) f32.const 8)
  (func (export "er_ui_command_text_h") (result f32) f32.const 13)
  (func (export "er_ui_command_item_id_offset") (result i32) i32.const 1)
  (func (export "er_ui_command_list_gap") (result f32) f32.const 6)
  (func (export "er_ui_command_list_padding") (result f32) f32.const 4)
  (func (export "er_ui_command_item_h") (result f32) f32.const 24)
  (func (export "er_ui_command_item_gap") (result f32) f32.const 4)
  (func (export "er_ui_command_item_padding_x") (result f32) f32.const 8)
  (func (export "er_ui_command_shortcut_gap") (result f32) f32.const 12)
  (func (export "er_ui_command_max_visible_items") (result i32) i32.const 3)
  (func (export "er_ui_command_empty_text_h") (result f32) f32.const 14)

  (func (export "er_ui_command_input_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $bounds f32.load
    local.get $bounds i32.const 4 i32.add f32.load
    local.get $bounds i32.const 8 i32.add f32.load
    local.get $bounds i32.const 12 i32.add f32.load f32.const 36 call $min_f32
    call $rect_store)

  (func $er_ui_command_list_bounds (export "er_ui_command_list_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 12 i32.add f32.load f32.const 42 f32.le
    if i32.const 0 return end
    local.get $out
    local.get $bounds f32.load
    local.get $bounds i32.const 4 i32.add f32.load f32.const 42 f32.add
    local.get $bounds i32.const 8 i32.add f32.load
    local.get $bounds i32.const 12 i32.add f32.load f32.const 42 f32.sub f32.const 1 call $max_f32
    call $rect_store)

  (func (export "er_ui_command_icon_bounds") (param $input i32) (param $out i32) (result i32)
    local.get $input i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $input f32.load f32.const 8 f32.add
    local.get $input i32.const 4 i32.add f32.load local.get $input i32.const 12 i32.add f32.load f32.const 14 f32.sub f32.const 0.5 f32.mul f32.add
    f32.const 14
    f32.const 14
    call $rect_store)

  (func (export "er_ui_command_input_text_bounds") (param $input i32) (param $out i32) (result i32)
    local.get $input i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $input f32.load f32.const 28 f32.add
    local.get $input i32.const 4 i32.add f32.load local.get $input i32.const 12 i32.add f32.load f32.const 13 f32.sub f32.const 0.5 f32.mul f32.add
    local.get $input i32.const 8 i32.add f32.load f32.const 28 f32.sub f32.const 8 f32.sub f32.const 1 call $max_f32
    f32.const 13
    call $rect_store)

  (func (export "er_ui_command_visible_item_capacity") (param $bounds i32) (result i32)
    (local $available f32) (local $raw i32)
    local.get $bounds i32.eqz
    if i32.const 0 return end
    local.get $bounds i32.const 124704 call $er_ui_command_list_bounds
    i32.eqz
    if i32.const 0 return end
    i32.const 124716 f32.load f32.const 8 f32.sub f32.const 0 call $max_f32 local.set $available
    local.get $available f32.const 4 f32.add f32.const 28 f32.div f32.floor i32.trunc_f32_u local.set $raw
    local.get $raw i32.const 3 i32.gt_u
    if i32.const 3 return end
    local.get $raw)

  (func (export "er_ui_command_item_bounds") (param $bounds i32) (param $index i32) (param $out i32) (result i32)
    (local $idx f32) (local $y f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $index i32.const 0 i32.lt_s local.get $index i32.const 3 i32.ge_s i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 124720 call $er_ui_command_list_bounds
    i32.eqz
    if i32.const 0 return end
    local.get $index f32.convert_i32_s local.set $idx
    i32.const 124724 f32.load f32.const 4 f32.add local.get $idx f32.const 28 f32.mul f32.add local.set $y
    local.get $y f32.const 24 f32.add i32.const 124724 f32.load i32.const 124732 f32.load f32.add f32.const 4 f32.sub f32.gt
    if i32.const 0 return end
    local.get $out
    i32.const 124720 f32.load f32.const 4 f32.add
    local.get $y
    i32.const 124728 f32.load f32.const 8 f32.sub f32.const 1 call $max_f32
    f32.const 24
    call $rect_store)

  (func (export "er_ui_command_item_id") (param $id i32) (param $index i32) (result i32)
    local.get $id local.get $index i32.add i32.const 1 i32.add)

  (func (export "er_ui_command_empty_text_bounds") (param $list i32) (param $out i32) (result i32)
    local.get $list i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $list f32.load f32.const 4 f32.add
    local.get $list i32.const 4 i32.add f32.load f32.const 4 f32.add
    local.get $list i32.const 8 i32.add f32.load f32.const 8 f32.sub f32.const 1 call $max_f32
    f32.const 14
    call $rect_store)

  (func (export "er_ui_command_item_detail_width") (param $shortcut_ptr i32) (param $shortcut_len i32) (result f32)
    local.get $shortcut_ptr i32.eqz local.get $shortcut_len i32.eqz i32.or
    if f32.const 0 return end
    local.get $shortcut_ptr local.get $shortcut_len f32.const 16 call $er_ui_font_text_width f32.const 16 f32.add)

  (func (export "er_ui_command_item_label_bounds") (param $item i32) (param $detail_w f32) (param $out i32) (result i32)
    local.get $item i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $item f32.load
    local.get $item i32.const 4 i32.add f32.load
    local.get $item i32.const 8 i32.add f32.load local.get $detail_w f32.sub f32.const 1 call $max_f32
    local.get $item i32.const 12 i32.add f32.load
    call $rect_store)

  (func (export "er_ui_command_item_shortcut_bounds") (param $item i32) (param $detail_w f32) (param $out i32) (result i32)
    local.get $item i32.eqz local.get $out i32.eqz i32.or local.get $detail_w f32.const 0 f32.le i32.or
    if i32.const 0 return end
    local.get $out
    local.get $item f32.load local.get $item i32.const 8 i32.add f32.load f32.add local.get $detail_w f32.sub
    local.get $item i32.const 4 i32.add f32.load
    local.get $detail_w
    local.get $item i32.const 12 i32.add f32.load
    call $rect_store)
