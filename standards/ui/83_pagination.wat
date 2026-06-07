(func (export "er_ui_pagination_page_count") (result i32) i32.const 3)
  (func (export "er_ui_pagination_item_count") (result i32) i32.const 5)
  (func (export "er_ui_pagination_gap") (result f32) f32.const 4)
  (func (export "er_ui_pagination_text_padding") (result f32) f32.const 2)
  (func (export "er_ui_pagination_label_len") (param $index i32) (result i32)
    i32.const 1)

  (func (export "er_ui_pagination_clamped_page") (param $page i32) (result i32)
    local.get $page i32.const 3 call $er_ui_list_clamped_index)

  (func (export "er_ui_pagination_indexed_id") (param $id i32) (param $page i32) (result i32)
    local.get $id i32.const 3 i32.mul
    local.get $page i32.const 3 call $er_ui_list_clamped_index
    i32.add)

  (func (export "er_ui_pagination_hit_id") (param $id i32) (param $index i32) (result i32)
    local.get $id local.get $index i32.const 4 call $layout_min_i32_u i32.add)

  (func (export "er_ui_pagination_active_item_index") (param $page i32) (result i32)
    local.get $page i32.const 3 call $er_ui_list_clamped_index i32.const 1 i32.add)

  (func (export "er_ui_pagination_item_bounds") (param $bounds i32) (param $index i32) (param $out i32) (result i32)
    local.get $bounds local.get $index i32.const 5 f32.const 4 local.get $out call $er_ui_list_equal_segment_bounds_gap)

  (func (export "er_ui_pagination_item_text_bounds") (param $bounds i32) (param $index i32) (param $out i32) (result i32)
    (local $inner_h f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds local.get $index i32.const 5 f32.const 4 i32.const 122000 call $er_ui_list_equal_segment_bounds_gap drop
    i32.const 122000 i32.const 12 i32.add f32.load f32.const 4 f32.sub f32.const 0 call $max_f32 local.set $inner_h
    local.get $out
    i32.const 122000 f32.load f32.const 2 f32.add
    i32.const 122000 i32.const 4 i32.add f32.load f32.const 2 f32.add local.get $inner_h f32.const 16 f32.sub f32.const 0.5 f32.mul f32.add
    i32.const 122000 i32.const 8 i32.add f32.load f32.const 4 f32.sub f32.const 0 call $max_f32
    f32.const 16
    call $rect_store)

  (func (export "er_ui_pagination_measure") (param $constraints i32) (param $out i32) (result i32)
    (local $item_w f32) (local $pref_w f32) (local $pref_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    i32.const 122020 i32.const 60 i32.store8
    i32.const 122021 i32.const 49 i32.store8
    i32.const 122022 i32.const 50 i32.store8
    i32.const 122023 i32.const 51 i32.store8
    i32.const 122024 i32.const 62 i32.store8
    i32.const 122020 i32.const 1 f32.const 16 call $er_ui_font_text_width
    i32.const 122021 i32.const 1 f32.const 16 call $er_ui_font_text_width call $max_f32
    i32.const 122022 i32.const 1 f32.const 16 call $er_ui_font_text_width call $max_f32
    i32.const 122023 i32.const 1 f32.const 16 call $er_ui_font_text_width call $max_f32
    i32.const 122024 i32.const 1 f32.const 16 call $er_ui_font_text_width call $max_f32
    f32.const 4 f32.add
    local.set $item_w
    local.get $item_w f32.const 5 f32.mul f32.const 16 f32.add
    f32.const 20
    local.get $constraints
    i32.const 122032
    call $er_ui_primitives_constrain_preferred_size drop
    i32.const 122032 f32.load local.set $pref_w
    i32.const 122036 f32.load local.set $pref_h
    f32.const 21
    f32.const 20
    local.get $pref_w local.get $pref_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit
    local.get $pref_h
    local.get $out
    call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)
