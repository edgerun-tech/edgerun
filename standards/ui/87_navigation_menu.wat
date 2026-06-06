  (func (export "er_ui_navigation_menu_item_count") (result i32) i32.const 3)
  (func (export "er_ui_navigation_menu_id_stride") (result i32) i32.const 3)
  (func (export "er_ui_navigation_menu_gap") (result f32) f32.const 4)
  (func (export "er_ui_navigation_menu_item_h") (result f32) f32.const 36)
  (func (export "er_ui_navigation_menu_text_padding") (result f32) f32.const 10)
  (func (export "er_ui_navigation_menu_icon_size") (result f32) f32.const 12)
  (func (export "er_ui_navigation_menu_icon_space") (result f32) f32.const 16)
  (func (export "er_ui_navigation_menu_icon_padding") (result f32) f32.const 8)
  (func (export "er_ui_navigation_menu_label_max_lines") (result i32) i32.const 1)
  (func (export "er_ui_navigation_menu_third_label_len") (result i32) i32.const 6)

  (func $er_ui_navigation_menu_write_third_label (result i32)
    i32.const 122360 i32.const 66 i32.store8
    i32.const 122361 i32.const 108 i32.store8
    i32.const 122362 i32.const 111 i32.store8
    i32.const 122363 i32.const 99 i32.store8
    i32.const 122364 i32.const 107 i32.store8
    i32.const 122365 i32.const 115 i32.store8
    i32.const 122360)

  (func $er_ui_navigation_menu_item_width (export "er_ui_navigation_menu_item_width") (param $label_ptr i32) (param $label_len i32) (param $show_chevron i32) (result f32)
    local.get $label_ptr
    local.get $label_len
    f32.const 16
    i32.const 1
    f32.const 10
    call $er_ui_primitives_measured_label_width
    local.get $show_chevron
    if (result f32)
      f32.const 16
    else
      f32.const 0
    end
    f32.add)

  (func $er_ui_navigation_menu_write_widths (export "er_ui_navigation_menu_write_widths") (param $first_ptr i32) (param $first_len i32) (param $second_ptr i32) (param $second_len i32) (param $out i32) (result i32)
    local.get $out i32.eqz
    if i32.const 0 return end
    local.get $out
    local.get $first_ptr local.get $first_len i32.const 1 call $er_ui_navigation_menu_item_width
    f32.store
    local.get $out i32.const 4 i32.add
    local.get $second_ptr local.get $second_len i32.const 1 call $er_ui_navigation_menu_item_width
    f32.store
    local.get $out i32.const 8 i32.add
    call $er_ui_navigation_menu_write_third_label i32.const 6 i32.const 0 call $er_ui_navigation_menu_item_width
    f32.store
    i32.const 1)

  (func (export "er_ui_navigation_menu_clamped_active") (param $active i32) (result i32)
    local.get $active i32.const 3 call $er_ui_list_clamped_index)

  (func (export "er_ui_navigation_menu_indexed_id") (param $id i32) (param $active i32) (result i32)
    local.get $id local.get $active i32.const 3 call $er_ui_list_encoded_indexed_id)

  (func (export "er_ui_navigation_menu_hit_id") (param $id i32) (param $index i32) (result i32)
    local.get $id local.get $index i32.const 3 call $er_ui_list_clamped_index i32.add)

  (func $er_ui_navigation_menu_item_bounds (export "er_ui_navigation_menu_item_bounds") (param $bounds i32) (param $first_ptr i32) (param $first_len i32) (param $second_ptr i32) (param $second_len i32) (param $index i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $first_ptr local.get $first_len local.get $second_ptr local.get $second_len i32.const 122368 call $er_ui_navigation_menu_write_widths drop
    local.get $bounds
    local.get $index
    i32.const 122368
    i32.const 3
    f32.const 0
    f32.const 4
    f32.const 36
    local.get $out
    call $er_ui_list_item_strip_bounds)

  (func (export "er_ui_navigation_menu_item_text_bounds") (param $item_bounds i32) (param $show_chevron i32) (param $out i32) (result i32)
    (local $icon_space f32)
    local.get $item_bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $show_chevron
    if
      f32.const 16 local.set $icon_space
    else
      f32.const 0 local.set $icon_space
    end
    i32.const 122384
    local.get $item_bounds f32.load
    local.get $item_bounds i32.const 4 i32.add f32.load
    local.get $item_bounds i32.const 8 i32.add f32.load local.get $icon_space f32.sub f32.const 1 call $max_f32
    local.get $item_bounds i32.const 12 i32.add f32.load
    call $rect_store drop
    i32.const 122384 f32.const 10 i32.const 122400 call $er_ui_primitives_content_inset drop
    i32.const 122400 local.get $out f32.const 16 call $er_ui_rect_with_height_centered)

  (func (export "er_ui_navigation_menu_item_icon_bounds") (param $item_bounds i32) (param $out i32) (result i32)
    local.get $item_bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $item_bounds f32.load local.get $item_bounds i32.const 8 i32.add f32.load f32.add f32.const 20 f32.sub
    local.get $item_bounds i32.const 4 i32.add f32.load local.get $item_bounds i32.const 12 i32.add f32.load f32.const 12 f32.sub f32.const 0.5 f32.mul f32.add
    f32.const 12
    f32.const 12
    call $rect_store)

  (func (export "er_ui_navigation_menu_measure") (param $first_ptr i32) (param $first_len i32) (param $second_ptr i32) (param $second_len i32) (param $constraints i32) (param $out i32) (result i32)
    (local $pref_w f32) (local $pref_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $first_ptr local.get $first_len local.get $second_ptr local.get $second_len i32.const 122416 call $er_ui_navigation_menu_write_widths drop
    i32.const 122416 f32.load
    i32.const 122420 f32.load
    f32.add
    i32.const 122424 f32.load
    f32.add
    f32.const 8
    f32.add
    local.set $pref_w
    f32.const 36 local.set $pref_h
    local.get $pref_w local.get $pref_h local.get $constraints i32.const 122432 call $er_ui_primitives_constrain_preferred_size drop
    i32.const 122432 f32.load local.set $pref_w
    i32.const 122436 f32.load local.set $pref_h
    f32.const 11 local.get $pref_w call $min_f32
    f32.const 36 local.get $pref_h call $min_f32
    local.get $pref_w local.get $pref_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit
    local.get $pref_h
    local.get $out
    call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)
