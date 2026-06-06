  (func (export "er_ui_breadcrumb_icon_size") (result f32) f32.const 12)
  (func (export "er_ui_breadcrumb_separator_gap") (result f32) f32.const 6)
  (func (export "er_ui_breadcrumb_vertical_padding") (result f32) f32.const 8)
  (func (export "er_ui_breadcrumb_label_max_lines") (result i32) i32.const 1)
  (func (export "er_ui_breadcrumb_middle_label_len") (result i32) i32.const 4)

  (func $er_ui_breadcrumb_write_docs_label (result i32)
    i32.const 121680 i32.const 68 i32.store8
    i32.const 121681 i32.const 111 i32.store8
    i32.const 121682 i32.const 99 i32.store8
    i32.const 121683 i32.const 115 i32.store8
    i32.const 121680)

  (func (export "er_ui_breadcrumb_separator_total_width") (result f32)
    f32.const 36)

  (func (export "er_ui_breadcrumb_hit_id") (param $id i32) (param $index i32) (result i32)
    local.get $id local.get $index i32.const 1 call $layout_min_i32_u i32.add)

  (func $er_ui_breadcrumb_label_width (param $ptr i32) (param $len i32) (result f32)
    local.get $ptr local.get $len f32.const 16 call $er_ui_font_text_width)

  (func $er_ui_breadcrumb_allocated_widths (param $first_ptr i32) (param $first_len i32) (param $current_ptr i32) (param $current_len i32) (param $bounds i32) (param $out i32) (result i32)
    (local $first_w f32) (local $middle_w f32) (local $current_w f32) (local $available f32) (local $natural f32) (local $scale f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $first_ptr local.get $first_len call $er_ui_breadcrumb_label_width local.set $first_w
    call $er_ui_breadcrumb_write_docs_label i32.const 4 call $er_ui_breadcrumb_label_width local.set $middle_w
    local.get $current_ptr local.get $current_len call $er_ui_breadcrumb_label_width local.set $current_w
    f32.const 3 local.get $bounds i32.const 8 i32.add f32.load f32.const 36 f32.sub call $max_f32 local.set $available
    f32.const 1 local.get $first_w local.get $middle_w f32.add local.get $current_w f32.add call $max_f32 local.set $natural
    f32.const 1 local.get $available local.get $natural f32.div call $min_f32 local.set $scale
    local.get $out local.get $first_w local.get $scale f32.mul f32.const 1 call $max_f32 f32.store
    local.get $out i32.const 4 i32.add local.get $middle_w local.get $scale f32.mul f32.const 1 call $max_f32 f32.store
    local.get $out i32.const 8 i32.add local.get $current_w local.get $scale f32.mul f32.const 1 call $max_f32 f32.store
    i32.const 1)

  (func (export "er_ui_breadcrumb_allocated_widths") (param $first_ptr i32) (param $first_len i32) (param $current_ptr i32) (param $current_len i32) (param $bounds i32) (param $out i32) (result i32)
    local.get $first_ptr local.get $first_len local.get $current_ptr local.get $current_len local.get $bounds local.get $out call $er_ui_breadcrumb_allocated_widths)

  (func $er_ui_breadcrumb_item_bounds (export "er_ui_breadcrumb_item_bounds") (param $first_ptr i32) (param $first_len i32) (param $current_ptr i32) (param $current_len i32) (param $bounds i32) (param $index i32) (param $out i32) (result i32)
    (local $first_w f32) (local $middle_w f32) (local $middle_x f32) (local $current_x f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $first_ptr local.get $first_len local.get $current_ptr local.get $current_len local.get $bounds i32.const 121688 call $er_ui_breadcrumb_allocated_widths drop
    i32.const 121688 f32.load local.set $first_w
    i32.const 121692 f32.load local.set $middle_w
    local.get $bounds f32.load local.get $first_w f32.add f32.const 18 f32.add local.set $middle_x
    local.get $middle_x local.get $middle_w f32.add f32.const 18 f32.add local.set $current_x
    local.get $index i32.eqz
    if
      local.get $out local.get $bounds f32.load local.get $bounds i32.const 4 i32.add f32.load local.get $first_w local.get $bounds i32.const 12 i32.add f32.load call $rect_store return
    end
    local.get $index i32.const 1 i32.eq
    if
      local.get $out local.get $middle_x local.get $bounds i32.const 4 i32.add f32.load local.get $middle_w local.get $bounds i32.const 12 i32.add f32.load call $rect_store return
    end
    local.get $out
    local.get $current_x
    local.get $bounds i32.const 4 i32.add f32.load
    local.get $bounds f32.load local.get $bounds i32.const 8 i32.add f32.load f32.add local.get $current_x f32.sub f32.const 1 call $max_f32
    local.get $bounds i32.const 12 i32.add f32.load
    call $rect_store)

  (func (export "er_ui_breadcrumb_separator_bounds") (param $first_ptr i32) (param $first_len i32) (param $current_ptr i32) (param $current_len i32) (param $bounds i32) (param $index i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $first_ptr local.get $first_len local.get $current_ptr local.get $current_len local.get $bounds local.get $index i32.const 121704 call $er_ui_breadcrumb_item_bounds drop
    local.get $out
    i32.const 121704 f32.load i32.const 121704 i32.const 8 i32.add f32.load f32.add f32.const 3 f32.add
    local.get $bounds i32.const 4 i32.add f32.load local.get $bounds i32.const 12 i32.add f32.load f32.const 12 f32.sub f32.const 0.5 f32.mul f32.add
    f32.const 12
    f32.const 12
    call $rect_store)

  (func (export "er_ui_breadcrumb_label_bounds") (param $item_bounds i32) (param $out i32) (result i32)
    local.get $item_bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $item_bounds f32.load
    local.get $item_bounds i32.const 4 i32.add f32.load local.get $item_bounds i32.const 12 i32.add f32.load f32.const 16 f32.sub f32.const 0.5 f32.mul f32.add
    local.get $item_bounds i32.const 8 i32.add f32.load
    f32.const 16
    call $rect_store)

  (func (export "er_ui_breadcrumb_measure") (param $first_ptr i32) (param $first_len i32) (param $current_ptr i32) (param $current_len i32) (param $constraints i32) (param $out i32) (result i32)
    (local $first_w f32) (local $middle_w f32) (local $current_w f32) (local $pref_w f32) (local $pref_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $first_ptr local.get $first_len call $er_ui_breadcrumb_label_width local.set $first_w
    call $er_ui_breadcrumb_write_docs_label i32.const 4 call $er_ui_breadcrumb_label_width local.set $middle_w
    local.get $current_ptr local.get $current_len call $er_ui_breadcrumb_label_width local.set $current_w
    local.get $first_w local.get $middle_w f32.add local.get $current_w f32.add f32.const 36 f32.add
    f32.const 32
    local.get $constraints
    i32.const 121724
    call $er_ui_primitives_constrain_preferred_size drop
    i32.const 121724 f32.load local.set $pref_w
    i32.const 121728 f32.load local.set $pref_h
    f32.const 39
    f32.const 32
    local.get $pref_w local.get $pref_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit
    local.get $pref_h
    local.get $out
    call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)
