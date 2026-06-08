
  (func $write_single_string_ref (param $base i32) (param $cap i32) (param $kind i32) (param $id i32) (param $src i32) (param $src_len i32) (param $second_ref i32) (result i32)
    (local $first_ref i32)
    local.get $base
    local.get $cap
    i32.const 1
    i32.const 1
    i32.const 0
    i32.const 0
    i32.const 0
    call $er_ui_writer_begin
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $src
    local.get $src_len
    call $er_ui_writer_string
    local.set $first_ref
    local.get $src_len
    i32.const 0
    i32.ne
    local.get $first_ref
    i32.eqz
    i32.and
    if
      i32.const 0
      return
    end
    i32.const 0
    local.get $kind
    local.get $id
    local.get $first_ref
    local.get $second_ref
    call $er_ui_writer_record
    drop
    global.get $writer_cursor)

  (func $write_id_multiplier_string (param $base i32) (param $cap i32) (param $kind i32) (param $id i32) (param $value i32) (param $mul i32) (param $src i32) (param $src_len i32) (result i32)
    local.get $base
    local.get $cap
    local.get $kind
    local.get $id
    local.get $mul
    i32.mul
    local.get $value
    i32.add
    local.get $src
    local.get $src_len
    i32.const 0
    call $write_single_string_ref)

  (func $er_ui_wasm_new_text  (param $base i32) (param $cap i32) (param $value_ptr i32) (param $value_len i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 1
    i32.const 0
    local.get $value_ptr
    local.get $value_len
    i32.const 0
    call $write_single_string_ref)

  (func $er_ui_wasm_new_button  (param $base i32) (param $cap i32) (param $id i32) (param $label_ptr i32) (param $label_len i32) (param $variant i32) (param $leading_icon i32) (param $trailing_icon i32) (result i32)
    (local $tag i32)
    (local $len i32)
    local.get $leading_icon
    local.set $tag
    local.get $variant
    i32.const 255
    i32.and
    local.set $len
    local.get $trailing_icon
    i32.const 0
    i32.ne
    if
      local.get $trailing_icon
      local.set $tag
      local.get $len
      i32.const 256
      i32.or
      local.set $len
    end
    local.get $base
    local.get $cap
    i32.const 12
    local.get $id
    local.get $label_ptr
    local.get $label_len
    local.get $tag
    local.get $len
    call $string_ref
    call $write_single_string_ref)

  (func $er_ui_wasm_new_row_item  (param $base i32) (param $cap i32) (param $id i32) (param $title_ptr i32) (param $title_len i32) (param $detail_ptr i32) (param $detail_len i32) (param $icon i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 26
    local.get $icon
    i32.eqz
    if (result i32)
      local.get $id
    else
      local.get $id
      i32.const 14
      i32.shl
      local.get $icon
      i32.const 0x3fff
      i32.and
      i32.or
    end
    local.get $title_ptr
    local.get $title_len
    local.get $detail_ptr
    local.get $detail_len
    call $er_ui_write_two_strings)

  (func $er_ui_wasm_new_badge (export "er_ui_wasm_new_badge") (param $base i32) (param $cap i32) (param $label_ptr i32) (param $label_len i32) (param $variant i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 27
    i32.const 0
    local.get $label_ptr
    local.get $label_len
    local.get $variant
    i32.const 0
    call $string_ref
    call $write_single_string_ref)

  (func $er_ui_wasm_new_checkbox  (param $base i32) (param $cap i32) (param $id i32) (param $label_ptr i32) (param $label_len i32) (param $checked i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 22
    local.get $id
    local.get $label_ptr
    local.get $label_len
    local.get $checked
    i32.const 0
    i32.ne
    i32.const 0
    call $string_ref
    call $write_single_string_ref)

  (func $er_ui_wasm_new_input  (param $base i32) (param $cap i32) (param $id i32) (param $placeholder_ptr i32) (param $placeholder_len i32) (param $leading_icon i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 16
    local.get $id
    local.get $placeholder_ptr
    local.get $placeholder_len
    local.get $leading_icon
    i32.const 0
    call $string_ref
    call $write_single_string_ref)

  (func $er_ui_wasm_new_slider  (param $base i32) (param $cap i32) (param $id i32) (param $label_ptr i32) (param $label_len i32) (param $unit16 i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 24
    local.get $id
    local.get $label_ptr
    local.get $label_len
    local.get $unit16
    i32.const 0
    call $string_ref
    call $write_single_string_ref)

  (func $er_ui_wasm_new_card (export "er_ui_wasm_new_card") (param $base i32) (param $cap i32) (param $title_ptr i32) (param $title_len i32) (param $detail_ptr i32) (param $detail_len i32) (param $variant i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 28
    local.get $variant
    local.get $title_ptr
    local.get $title_len
    local.get $detail_ptr
    local.get $detail_len
    call $er_ui_write_two_strings)

  (func $er_ui_wasm_new_separator  (param $base i32) (param $cap i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 33
    call $er_ui_write_empty)

  (func $er_ui_wasm_new_icon  (param $base i32) (param $cap i32) (param $label_ptr i32) (param $label_len i32) (param $icon_value i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 55
    i32.const 0
    local.get $label_ptr
    local.get $label_len
    i32.const 0
    local.get $icon_value
    call $string_ref
    call $write_single_string_ref)

  (func $er_ui_wasm_new_switch  (param $base i32) (param $cap i32) (param $id i32) (param $label_ptr i32) (param $label_len i32) (param $checked i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 23
    local.get $id
    local.get $label_ptr
    local.get $label_len
    local.get $checked
    i32.const 0
    i32.ne
    i32.const 0
    call $string_ref
    call $write_single_string_ref)

  (func $er_ui_wasm_new_toggle  (param $base i32) (param $cap i32) (param $id i32) (param $label_ptr i32) (param $label_len i32) (param $checked i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 56
    local.get $id
    local.get $label_ptr
    local.get $label_len
    local.get $checked
    i32.const 0
    i32.ne
    i32.const 0
    call $string_ref
    call $write_single_string_ref)

  (func $er_ui_wasm_new_progress  (param $base i32) (param $cap i32) (param $id i32) (param $label_ptr i32) (param $label_len i32) (param $unit16 i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 37
    local.get $id
    local.get $label_ptr
    local.get $label_len
    local.get $unit16
    i32.const 0
    call $string_ref
    call $write_single_string_ref)

  (func $er_ui_wasm_new_icon_button  (param $base i32) (param $cap i32) (param $id i32) (param $icon i32) (param $variant i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 1
    i32.const 1
    i32.const 0
    i32.const 0
    i32.const 0
    call $er_ui_writer_begin
    i32.eqz
    if
      i32.const 0
      return
    end
    i32.const 0
    i32.const 13
    local.get $id
    i32.const 0
    local.get $variant
    local.get $icon
    call $string_ref
    call $er_ui_writer_record
    drop
    global.get $writer_cursor)

  (func $er_ui_wasm_new_icon_button_named  (param $base i32) (param $cap i32) (param $id i32) (param $label_ptr i32) (param $label_len i32) (param $icon i32) (param $variant i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 13
    local.get $id
    local.get $label_ptr
    local.get $label_len
    local.get $variant
    local.get $icon
    call $string_ref
    call $write_single_string_ref)

  (func $er_ui_wasm_new_label  (param $base i32) (param $cap i32) (param $text_ptr i32) (param $text_len i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 31
    i32.const 0
    local.get $text_ptr
    local.get $text_len
    i32.const 0
    call $write_single_string_ref)

  (func $er_ui_wasm_new_accordion  (param $base i32) (param $cap i32) (param $id i32) (param $label_ptr i32) (param $label_len i32) (param $open i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 3
    local.get $id
    local.get $open
    i32.const 0
    i32.ne
    i32.const 2
    local.get $label_ptr
    local.get $label_len
    call $write_id_multiplier_string)

  (func $er_ui_wasm_new_accordion_full  (param $base i32) (param $cap i32) (param $id i32) (param $title_ptr i32) (param $title_len i32) (param $detail_ptr i32) (param $detail_len i32) (param $open i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 3
    local.get $id
    i32.const 2
    i32.mul
    local.get $open
    i32.const 0
    i32.ne
    i32.add
    local.get $title_ptr
    local.get $title_len
    local.get $detail_ptr
    local.get $detail_len
    call $er_ui_write_two_strings)

  (func $er_ui_wasm_new_button_group  (param $base i32) (param $cap i32) (param $id i32) (param $label_ptr i32) (param $label_len i32) (param $active i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 14
    local.get $id
    local.get $active
    i32.const 1
    call $min_i32_u
    i32.const 2
    local.get $label_ptr
    local.get $label_len
    call $write_id_multiplier_string)

  (func $er_ui_wasm_new_button_group_full  (param $base i32) (param $cap i32) (param $id i32) (param $first_ptr i32) (param $first_len i32) (param $second_ptr i32) (param $second_len i32) (param $active i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 14
    local.get $id
    i32.const 2
    i32.mul
    local.get $active
    i32.const 1
    call $min_i32_u
    i32.add
    local.get $first_ptr
    local.get $first_len
    local.get $second_ptr
    local.get $second_len
    call $er_ui_write_two_strings)

  (func $er_ui_wasm_new_toggle_group  (param $base i32) (param $cap i32) (param $id i32) (param $label_ptr i32) (param $label_len i32) (param $active i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 15
    local.get $id
    local.get $active
    i32.const 2
    call $min_i32_u
    i32.const 3
    local.get $label_ptr
    local.get $label_len
    call $write_id_multiplier_string)

  (func $er_ui_wasm_new_toggle_group_full  (param $base i32) (param $cap i32) (param $id i32) (param $first_ptr i32) (param $first_len i32) (param $second_ptr i32) (param $second_len i32) (param $active i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 15
    local.get $id
    i32.const 3
    i32.mul
    local.get $active
    i32.const 2
    call $min_i32_u
    i32.add
    local.get $first_ptr
    local.get $first_len
    local.get $second_ptr
    local.get $second_len
    call $er_ui_write_two_strings)

  (func $er_ui_wasm_new_radio_group  (param $base i32) (param $cap i32) (param $id i32) (param $label_ptr i32) (param $label_len i32) (param $selected i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 25
    local.get $id
    local.get $selected
    i32.const 1
    call $min_i32_u
    i32.const 2
    local.get $label_ptr
    local.get $label_len
    call $write_id_multiplier_string)

  (func $er_ui_wasm_new_radio_group_full  (param $base i32) (param $cap i32) (param $id i32) (param $first_ptr i32) (param $first_len i32) (param $second_ptr i32) (param $second_len i32) (param $selected i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 25
    local.get $id
    i32.const 2
    i32.mul
    local.get $selected
    i32.const 1
    call $min_i32_u
    i32.add
    local.get $first_ptr
    local.get $first_len
    local.get $second_ptr
    local.get $second_len
    call $er_ui_write_two_strings)

  (func $er_ui_wasm_new_tabs  (param $base i32) (param $cap i32) (param $id i32) (param $label_ptr i32) (param $label_len i32) (param $active i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 42
    local.get $id
    local.get $active
    i32.const 1
    call $min_i32_u
    i32.const 2
    local.get $label_ptr
    local.get $label_len
    call $write_id_multiplier_string)

  (func $er_ui_wasm_new_tabs_full  (param $base i32) (param $cap i32) (param $id i32) (param $first_ptr i32) (param $first_len i32) (param $second_ptr i32) (param $second_len i32) (param $active i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 42
    local.get $id
    i32.const 2
    i32.mul
    local.get $active
    i32.const 1
    call $min_i32_u
    i32.add
    local.get $first_ptr
    local.get $first_len
    local.get $second_ptr
    local.get $second_len
    call $er_ui_write_two_strings)

  (func $er_ui_wasm_new_direction  (param $base i32) (param $cap i32) (param $id i32) (param $label_ptr i32) (param $label_len i32) (param $active i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 43
    local.get $id
    local.get $active
    i32.const 1
    call $min_i32_u
    i32.const 2
    local.get $label_ptr
    local.get $label_len
    call $write_id_multiplier_string)

  (func $er_ui_wasm_new_pagination  (param $base i32) (param $cap i32) (param $id i32) (param $label_ptr i32) (param $label_len i32) (param $page i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 41
    local.get $id
    local.get $page
    i32.const 2
    call $min_i32_u
    i32.const 3
    local.get $label_ptr
    local.get $label_len
    call $write_id_multiplier_string)

  (func $er_ui_wasm_new_menubar  (param $base i32) (param $cap i32) (param $id i32) (param $label_ptr i32) (param $label_len i32) (param $active i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 39
    local.get $id
    local.get $active
    i32.const 2
    call $min_i32_u
    i32.const 3
    local.get $label_ptr
    local.get $label_len
    call $write_id_multiplier_string)

  (func $er_ui_wasm_new_menubar_full  (param $base i32) (param $cap i32) (param $id i32) (param $first_ptr i32) (param $first_len i32) (param $second_ptr i32) (param $second_len i32) (param $active i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 39
    local.get $id
    i32.const 3
    i32.mul
    local.get $active
    i32.const 2
    call $min_i32_u
    i32.add
    local.get $first_ptr
    local.get $first_len
    local.get $second_ptr
    local.get $second_len
    call $er_ui_write_two_strings)

  (func $er_ui_wasm_new_navigation_menu  (param $base i32) (param $cap i32) (param $id i32) (param $label_ptr i32) (param $label_len i32) (param $active i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 40
    local.get $id
    local.get $active
    i32.const 2
    call $min_i32_u
    i32.const 3
    local.get $label_ptr
    local.get $label_len
    call $write_id_multiplier_string)

  (func $er_ui_wasm_new_navigation_menu_full  (param $base i32) (param $cap i32) (param $id i32) (param $first_ptr i32) (param $first_len i32) (param $second_ptr i32) (param $second_len i32) (param $active i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 40
    local.get $id
    i32.const 3
    i32.mul
    local.get $active
    i32.const 2
    call $min_i32_u
    i32.add
    local.get $first_ptr
    local.get $first_len
    local.get $second_ptr
    local.get $second_len
    call $er_ui_write_two_strings)

  (func $er_ui_wasm_new_resizable  (param $base i32) (param $cap i32) (param $id i32) (param $label_ptr i32) (param $label_len i32) (param $unit16 i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 57
    local.get $id
    local.get $label_ptr
    local.get $label_len
    local.get $unit16
    i32.const 0
    call $string_ref
    call $write_single_string_ref)

  (func $er_ui_wasm_new_kbd  (param $base i32) (param $cap i32) (param $text_ptr i32) (param $text_len i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 30
    i32.const 0
    local.get $text_ptr
    local.get $text_len
    i32.const 0
    call $write_single_string_ref)

  (func $er_ui_wasm_new_skeleton  (param $base i32) (param $cap i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 35
    call $er_ui_write_empty)

  (func $er_ui_wasm_new_spinner  (param $base i32) (param $cap i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 36
    call $er_ui_write_empty)

  (func $er_ui_wasm_new_textarea  (param $base i32) (param $cap i32) (param $id i32) (param $value_ptr i32) (param $value_len i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 19
    local.get $id
    local.get $value_ptr
    local.get $value_len
    i32.const 0
    call $write_single_string_ref)

  (func $er_ui_wasm_new_avatar  (param $base i32) (param $cap i32) (param $label_ptr i32) (param $label_len i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 29
    i32.const 0
    local.get $label_ptr
    local.get $label_len
    i32.const 0
    call $write_single_string_ref)

  (func $er_ui_wasm_new_tooltip  (param $base i32) (param $cap i32) (param $id i32) (param $text_ptr i32) (param $text_len i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 51
    local.get $id
    local.get $text_ptr
    local.get $text_len
    i32.const 0
    call $write_single_string_ref)

  (func $er_ui_wasm_new_tooltip_full  (param $base i32) (param $cap i32) (param $id i32) (param $trigger_ptr i32) (param $trigger_len i32) (param $content_ptr i32) (param $content_len i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 51
    local.get $id
    local.get $trigger_ptr
    local.get $trigger_len
    local.get $content_ptr
    local.get $content_len
    call $er_ui_write_two_strings)

  (func $er_ui_wasm_new_alert  (param $base i32) (param $cap i32) (param $title_ptr i32) (param $title_len i32) (param $detail_ptr i32) (param $detail_len i32) (param $destructive i32) (param $icon i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 4
    local.get $destructive
    i32.const 0
    i32.ne
    i32.const 16
    i32.shl
    local.get $icon
    i32.const 65535
    i32.and
    i32.or
    local.get $title_ptr
    local.get $title_len
    local.get $detail_ptr
    local.get $detail_len
    call $er_ui_write_two_strings)

  (func $er_ui_wasm_new_alert_dialog  (param $base i32) (param $cap i32) (param $id i32) (param $title_ptr i32) (param $title_len i32) (param $detail_ptr i32) (param $detail_len i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 5
    local.get $id
    local.get $title_ptr
    local.get $title_len
    local.get $detail_ptr
    local.get $detail_len
    call $er_ui_write_two_strings)

  (func $er_ui_wasm_new_aspect_ratio  (param $base i32) (param $cap i32) (param $ratio_w i32) (param $ratio_h i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 6
    local.get $ratio_w
    i32.const 16
    i32.shl
    local.get $ratio_h
    i32.const 65535
    i32.and
    i32.or
    i32.const 0
    call $er_ui_write_ref)

  (func $er_ui_wasm_new_calendar  (param $base i32) (param $cap i32) (param $id i32) (param $month_ptr i32) (param $month_len i32) (param $selected_day i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 7
    local.get $id
    local.get $month_ptr
    local.get $month_len
    local.get $selected_day
    i32.const 0
    call $string_ref
    call $write_single_string_ref)

  (func $er_ui_wasm_new_carousel  (param $base i32) (param $cap i32) (param $id i32) (param $label_ptr i32) (param $label_len i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 8
    local.get $id
    local.get $label_ptr
    local.get $label_len
    i32.const 0
    call $write_single_string_ref)

  (func $er_ui_wasm_new_chart  (param $base i32) (param $cap i32) (param $id i32) (param $label_ptr i32) (param $label_len i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 9
    local.get $id
    local.get $label_ptr
    local.get $label_len
    i32.const 0
    call $write_single_string_ref)

  (func $er_ui_wasm_new_combobox  (param $base i32) (param $cap i32) (param $id i32) (param $placeholder_ptr i32) (param $placeholder_len i32) (param $selected_ptr i32) (param $selected_len i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 10
    local.get $id
    local.get $placeholder_ptr
    local.get $placeholder_len
    local.get $selected_ptr
    local.get $selected_len
    call $er_ui_write_two_strings)

  (func $er_ui_wasm_new_empty_state  (param $base i32) (param $cap i32) (param $title_ptr i32) (param $title_len i32) (param $detail_ptr i32) (param $detail_len i32) (param $icon i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 11
    local.get $icon
    i32.const 65535
    i32.and
    local.get $title_ptr
    local.get $title_len
    local.get $detail_ptr
    local.get $detail_len
    call $er_ui_write_two_strings)

  (func $er_ui_wasm_new_input_group  (param $base i32) (param $cap i32) (param $id i32) (param $addon_ptr i32) (param $addon_len i32) (param $placeholder_ptr i32) (param $placeholder_len i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 17
    local.get $id
    local.get $addon_ptr
    local.get $addon_len
    local.get $placeholder_ptr
    local.get $placeholder_len
    call $er_ui_write_two_strings)

  (func $er_ui_wasm_new_input_otp  (param $base i32) (param $cap i32) (param $id i32) (param $value_ptr i32) (param $value_len i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 18
    local.get $id
    local.get $value_ptr
    local.get $value_len
    i32.const 0
    call $write_single_string_ref)

  (func $er_ui_wasm_new_select  (param $base i32) (param $cap i32) (param $id i32) (param $label_ptr i32) (param $label_len i32) (param $trailing_icon i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 20
    local.get $id
    local.get $label_ptr
    local.get $label_len
    local.get $trailing_icon
    i32.const 0
    call $string_ref
    call $write_single_string_ref)

  (func $er_ui_wasm_new_field  (param $base i32) (param $cap i32) (param $id i32) (param $label_ptr i32) (param $label_len i32) (param $placeholder_ptr i32) (param $placeholder_len i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 21
    local.get $id
    local.get $label_ptr
    local.get $label_len
    local.get $placeholder_ptr
    local.get $placeholder_len
    call $er_ui_write_two_strings)

  (func $er_ui_wasm_new_table  (param $base i32) (param $cap i32) (param $id i32) (param $name_ptr i32) (param $name_len i32) (param $role_ptr i32) (param $role_len i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 32
    local.get $id
    local.get $name_ptr
    local.get $name_len
    local.get $role_ptr
    local.get $role_len
    call $er_ui_write_two_strings)

  (func $er_ui_wasm_new_scroll_area  (param $base i32) (param $cap i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 34
    call $er_ui_write_empty)

  (func $er_ui_wasm_new_breadcrumb  (param $base i32) (param $cap i32) (param $id i32) (param $first_ptr i32) (param $first_len i32) (param $current_ptr i32) (param $current_len i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 38
    local.get $id
    local.get $first_ptr
    local.get $first_len
    local.get $current_ptr
    local.get $current_len
    call $er_ui_write_two_strings)

  (func $er_ui_wasm_new_command  (param $base i32) (param $cap i32) (param $id i32) (param $placeholder_ptr i32) (param $placeholder_len i32) (param $leading_icon i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 44
    local.get $id
    local.get $placeholder_ptr
    local.get $placeholder_len
    local.get $leading_icon
    i32.const 0
    call $string_ref
    call $write_single_string_ref)

  (func $er_ui_wasm_new_context_menu  (param $base i32) (param $cap i32) (param $id i32) (param $first_ptr i32) (param $first_len i32) (param $second_ptr i32) (param $second_len i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 45
    local.get $id
    local.get $first_ptr
    local.get $first_len
    local.get $second_ptr
    local.get $second_len
    call $er_ui_write_two_strings)

  (func $er_ui_wasm_new_dialog  (param $base i32) (param $cap i32) (param $id i32) (param $title_ptr i32) (param $title_len i32) (param $detail_ptr i32) (param $detail_len i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 46
    local.get $id
    local.get $title_ptr
    local.get $title_len
    local.get $detail_ptr
    local.get $detail_len
    call $er_ui_write_two_strings)

  (func $er_ui_wasm_new_drawer  (param $base i32) (param $cap i32) (param $id i32) (param $title_ptr i32) (param $title_len i32) (param $detail_ptr i32) (param $detail_len i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 47
    local.get $id
    local.get $title_ptr
    local.get $title_len
    local.get $detail_ptr
    local.get $detail_len
    call $er_ui_write_two_strings)

  (func $er_ui_wasm_new_dropdown_menu  (param $base i32) (param $cap i32) (param $id i32) (param $first_ptr i32) (param $first_len i32) (param $second_ptr i32) (param $second_len i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 48
    local.get $id
    local.get $first_ptr
    local.get $first_len
    local.get $second_ptr
    local.get $second_len
    call $er_ui_write_two_strings)

  (func $er_ui_wasm_new_hover_card  (param $base i32) (param $cap i32) (param $id i32) (param $trigger_ptr i32) (param $trigger_len i32) (param $content_ptr i32) (param $content_len i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 49
    local.get $id
    local.get $trigger_ptr
    local.get $trigger_len
    local.get $content_ptr
    local.get $content_len
    call $er_ui_write_two_strings)

  (func $er_ui_wasm_new_popover  (param $base i32) (param $cap i32) (param $id i32) (param $trigger_ptr i32) (param $trigger_len i32) (param $content_ptr i32) (param $content_len i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 50
    local.get $id
    local.get $trigger_ptr
    local.get $trigger_len
    local.get $content_ptr
    local.get $content_len
    call $er_ui_write_two_strings)

  (func $er_ui_wasm_new_toast  (param $base i32) (param $cap i32) (param $id i32) (param $title_ptr i32) (param $title_len i32) (param $detail_ptr i32) (param $detail_len i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 52
    local.get $id
    local.get $title_ptr
    local.get $title_len
    local.get $detail_ptr
    local.get $detail_len
    call $er_ui_write_two_strings)

  (func $er_ui_wasm_new_sheet  (param $base i32) (param $cap i32) (param $id i32) (param $title_ptr i32) (param $title_len i32) (param $detail_ptr i32) (param $detail_len i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 53
    local.get $id
    local.get $title_ptr
    local.get $title_len
    local.get $detail_ptr
    local.get $detail_len
    call $er_ui_write_two_strings)

  (func $er_ui_wasm_new_sidebar  (param $base i32) (param $cap i32) (param $id i32) (param $title_ptr i32) (param $title_len i32) (param $item_ptr i32) (param $item_len i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 54
    local.get $id
    local.get $title_ptr
    local.get $title_len
    local.get $item_ptr
    local.get $item_len
    call $er_ui_write_two_strings)
