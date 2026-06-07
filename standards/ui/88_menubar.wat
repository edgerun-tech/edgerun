(func (export "er_ui_menubar_item_count") (result i32) i32.const 3)
  (func (export "er_ui_menubar_id_stride") (result i32) i32.const 3)
  (func (export "er_ui_menubar_padding") (result f32) f32.const 4)
  (func (export "er_ui_menubar_item_h") (result f32) f32.const 28)
  (func (export "er_ui_menubar_item_padding_x") (result f32) f32.const 8)
  (func (export "er_ui_menubar_label_max_lines") (result i32) i32.const 1)
  (func (export "er_ui_menubar_third_label_len") (result i32) i32.const 4)

  (func (export "er_ui_menubar_active_index") (param $active i32) (result i32)
    local.get $active i32.const 3 call $er_ui_list_clamped_index)

  (func (export "er_ui_menubar_indexed_id") (param $id i32) (param $active i32) (result i32)
    local.get $id i32.const 3 i32.mul
    local.get $active i32.const 3 call $er_ui_list_clamped_index
    i32.add)

  (func (export "er_ui_menubar_hit_id") (param $id i32) (param $index i32) (result i32)
    local.get $id
    local.get $index i32.const 0 i32.const 2 call $clamp_i32
    i32.add)

  (func $er_ui_menubar_write_third_label (result i32)
    i32.const 122340 i32.const 86 i32.store8
    i32.const 122341 i32.const 105 i32.store8
    i32.const 122342 i32.const 101 i32.store8
    i32.const 122343 i32.const 119 i32.store8
    i32.const 122340)

  (func $er_ui_menubar_item_width (param $label_ptr i32) (param $label_len i32) (result f32)
    local.get $label_ptr local.get $label_len f32.const 16 call $er_ui_font_text_width
    f32.const 16
    f32.add)

  (func $er_ui_menubar_write_widths (param $first_ptr i32) (param $first_len i32) (param $second_ptr i32) (param $second_len i32) (param $out i32) (result i32)
    local.get $out i32.eqz
    if i32.const 0 return end
    local.get $out local.get $first_ptr local.get $first_len call $er_ui_menubar_item_width f32.store
    local.get $out i32.const 4 i32.add local.get $second_ptr local.get $second_len call $er_ui_menubar_item_width f32.store
    local.get $out i32.const 8 i32.add call $er_ui_menubar_write_third_label i32.const 4 call $er_ui_menubar_item_width f32.store
    i32.const 1)

  (func $er_ui_menubar_item_bounds (export "er_ui_menubar_item_bounds") (param $bounds i32) (param $first_ptr i32) (param $first_len i32) (param $second_ptr i32) (param $second_len i32) (param $index i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $first_ptr local.get $first_len local.get $second_ptr local.get $second_len i32.const 122348 call $er_ui_menubar_write_widths drop
    local.get $bounds
    local.get $index
    i32.const 122348
    i32.const 3
    f32.const 4
    f32.const 0
    f32.const 28
    local.get $out
    call $er_ui_list_item_strip_bounds)

  (func (export "er_ui_menubar_item_text_bounds") (param $bounds i32) (param $first_ptr i32) (param $first_len i32) (param $second_ptr i32) (param $second_len i32) (param $index i32) (param $out i32) (result i32)
    (local $pad f32) (local $inner_h f32) (local $text_h f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds local.get $first_ptr local.get $first_len local.get $second_ptr local.get $second_len local.get $index i32.const 122364 call $er_ui_menubar_item_bounds drop
    f32.const 8
    i32.const 122364 i32.const 8 i32.add f32.load
    i32.const 122364 i32.const 12 i32.add f32.load
    call $min_f32
    f32.const 0.5
    f32.mul
    call $min_f32
    local.set $pad
    i32.const 122364 i32.const 12 i32.add f32.load local.get $pad f32.const 2 f32.mul f32.sub f32.const 0 call $max_f32 local.set $inner_h
    f32.const 16 local.get $inner_h call $min_f32 local.set $text_h
    local.get $out
    i32.const 122364 f32.load local.get $pad f32.add
    i32.const 122364 i32.const 4 i32.add f32.load local.get $pad f32.add local.get $inner_h local.get $text_h f32.sub f32.const 0.5 f32.mul f32.add
    i32.const 122364 i32.const 8 i32.add f32.load local.get $pad f32.const 2 f32.mul f32.sub f32.const 0 call $max_f32
    local.get $text_h
    call $rect_store)

  (func (export "er_ui_menubar_measure") (param $first_ptr i32) (param $first_len i32) (param $second_ptr i32) (param $second_len i32) (param $constraints i32) (param $out i32) (result i32)
    (local $pref_w f32) (local $pref_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $first_ptr local.get $first_len local.get $second_ptr local.get $second_len i32.const 122348 call $er_ui_menubar_write_widths drop
    i32.const 122348 f32.load
    i32.const 122352 f32.load f32.add
    i32.const 122356 f32.load f32.add
    f32.const 8 f32.add
    f32.const 36
    local.get $constraints
    i32.const 122380
    call $er_ui_primitives_constrain_preferred_size drop
    i32.const 122380 f32.load local.set $pref_w
    i32.const 122384 f32.load local.set $pref_h
    f32.const 11 local.get $pref_w call $min_f32
    f32.const 36 local.get $pref_h call $min_f32
    local.get $pref_w local.get $pref_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit
    local.get $pref_h
    local.get $out
    call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)
