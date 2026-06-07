(func (export "er_ui_carousel_button_size") (result f32) f32.const 28)
  (func (export "er_ui_carousel_gap") (result f32) f32.const 8)
  (func (export "er_ui_carousel_radius") (result f32) f32.const 8)
  (func (export "er_ui_carousel_text_padding") (result f32) f32.const 8)
  (func (export "er_ui_carousel_label_max_lines") (result i32) i32.const 1)

  (func (export "er_ui_carousel_hit_id") (param $id i32) (param $index i32) (result i32)
    local.get $id
    local.get $index i32.const 0 i32.const 1 call $clamp_i32
    i32.add)

  (func $er_ui_carousel_button_bounds (export "er_ui_carousel_button_bounds") (param $bounds i32) (param $index i32) (param $out i32) (result i32)
    (local $y f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 4 i32.add f32.load
    local.get $bounds i32.const 12 i32.add f32.load f32.const 28 f32.sub f32.const 0.5 f32.mul
    f32.add
    local.set $y
    local.get $index i32.eqz
    if
      local.get $out
      local.get $bounds f32.load
      local.get $y
      f32.const 28
      f32.const 28
      call $rect_store
      return
    end
    local.get $out
    local.get $bounds f32.load local.get $bounds i32.const 8 i32.add f32.load f32.add f32.const 28 f32.sub
    local.get $y
    f32.const 28
    f32.const 28
    call $rect_store)

  (func $er_ui_carousel_content_bounds (export "er_ui_carousel_content_bounds") (param $bounds i32) (param $out i32) (result i32)
    (local $x f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds f32.load f32.const 36 f32.add local.set $x
    local.get $out
    local.get $x
    local.get $bounds i32.const 4 i32.add f32.load
    local.get $bounds i32.const 8 i32.add f32.load f32.const 72 f32.sub f32.const 1 call $max_f32
    local.get $bounds i32.const 12 i32.add f32.load
    call $rect_store)

  (func $er_ui_carousel_content_inner_bounds (export "er_ui_carousel_content_inner_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 122400 call $er_ui_carousel_content_bounds drop
    i32.const 122400 f32.const 8 local.get $out call $er_ui_primitives_content_inset)

  (func (export "er_ui_carousel_label_text_bounds") (param $bounds i32) (param $out i32) (result i32)
    (local $h f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 122416 call $er_ui_carousel_content_inner_bounds drop
    f32.const 16 i32.const 122416 i32.const 12 i32.add f32.load call $min_f32 local.set $h
    local.get $out
    i32.const 122416 f32.load
    i32.const 122416 i32.const 4 i32.add f32.load i32.const 122416 i32.const 12 i32.add f32.load local.get $h f32.sub f32.const 0.5 f32.mul f32.add
    i32.const 122416 i32.const 8 i32.add f32.load
    local.get $h
    call $rect_store)

  (func (export "er_ui_carousel_measure") (param $label_ptr i32) (param $label_len i32) (param $constraints i32) (param $out i32) (result i32)
    (local $label_w f32) (local $pref_w f32) (local $pref_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $label_ptr local.get $label_len f32.const 16 call $er_ui_font_text_width local.set $label_w
    local.get $label_w f32.const 72 f32.add local.set $pref_w
    f32.const 32 local.set $pref_h
    local.get $pref_w local.get $pref_h local.get $constraints i32.const 122432 call $er_ui_primitives_constrain_preferred_size drop
    i32.const 122432 f32.load local.set $pref_w
    i32.const 122436 f32.load local.set $pref_h
    f32.const 57 local.get $pref_w call $min_f32
    f32.const 28 local.get $pref_h call $min_f32
    local.get $pref_w local.get $pref_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit
    local.get $pref_h
    local.get $out
    call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)
