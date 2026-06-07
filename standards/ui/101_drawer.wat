(func (export "er_ui_drawer_trigger_y") (result f32) f32.const 4)
  (func (export "er_ui_drawer_trigger_w") (result f32) f32.const 62)
  (func (export "er_ui_drawer_trigger_h") (result f32) f32.const 30)
  (func (export "er_ui_drawer_trigger_padding") (result f32) f32.const 8)
  (func (export "er_ui_drawer_content_y") (result f32) f32.const 38)
  (func (export "er_ui_drawer_content_inset_x") (result f32) f32.const 10)
  (func (export "er_ui_drawer_radius") (result f32) f32.const 10)
  (func (export "er_ui_drawer_padding") (result f32) f32.const 12)
  (func (export "er_ui_drawer_handle_w") (result f32) f32.const 58)
  (func (export "er_ui_drawer_handle_h") (result f32) f32.const 4)
  (func (export "er_ui_drawer_handle_y") (result f32) f32.const 5)
  (func (export "er_ui_drawer_handle_radius") (result f32) f32.const 2)
  (func (export "er_ui_drawer_panel_title_y") (result f32) f32.const 14)
  (func (export "er_ui_drawer_panel_title_h") (result f32) f32.const 14)
  (func (export "er_ui_drawer_panel_detail_y") (result f32) f32.const 31)
  (func (export "er_ui_drawer_panel_detail_h") (result f32) f32.const 12)

  (func $er_ui_drawer_panel (param $out i32) (result i32)
    local.get $out f32.const 10 f32.const 12 f32.const 14 f32.const 14 f32.const 31 f32.const 12 f32.const 0 i32.const 1 i32.const 1 call $er_ui_primitives_title_detail_panel)

  (func (export "er_ui_drawer_trigger_id") (param $id i32) (result i32)
    local.get $id call $er_ui_primitives_overlay_trigger_id)

  (func (export "er_ui_drawer_content_id") (param $id i32) (result i32)
    local.get $id i32.const 1 call $er_ui_primitives_overlay_indexed_id)

  (func $er_ui_drawer_trigger_bounds (export "er_ui_drawer_trigger_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $bounds f32.load
    local.get $bounds i32.const 4 i32.add f32.load f32.const 4 f32.add
    f32.const 62
    f32.const 30
    call $rect_store)

  (func $er_ui_drawer_content_bounds (export "er_ui_drawer_content_bounds") (param $bounds i32) (param $out i32) (result i32)
    (local $y f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 4 i32.add f32.load f32.const 38 f32.add local.set $y
    local.get $out
    local.get $bounds f32.load f32.const 10 f32.add
    local.get $y
    local.get $bounds i32.const 8 i32.add f32.load f32.const 20 f32.sub f32.const 1 call $max_f32
    local.get $bounds i32.const 4 i32.add f32.load local.get $bounds i32.const 12 i32.add f32.load f32.add local.get $y f32.sub f32.const 1 call $max_f32
    call $rect_store)

  (func (export "er_ui_drawer_handle_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 123760 call $er_ui_drawer_content_bounds drop
    local.get $out
    i32.const 123760 f32.load i32.const 123768 f32.load f32.const 58 f32.sub f32.const 0.5 f32.mul f32.add
    i32.const 123764 f32.load f32.const 5 f32.add
    f32.const 58
    f32.const 4
    call $rect_store)

  (func (export "er_ui_drawer_trigger_text_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 123776 call $er_ui_drawer_trigger_bounds drop
    i32.const 123776 f32.const 8 i32.const 123792 call $er_ui_primitives_content_inset drop
    i32.const 123792 local.get $out f32.const 16 call $er_ui_rect_with_height_centered)

  (func (export "er_ui_drawer_title_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 123808 call $er_ui_drawer_content_bounds drop
    local.get $out
    i32.const 123808 f32.load f32.const 12 f32.add
    i32.const 123812 f32.load f32.const 14 f32.add
    i32.const 123816 f32.load f32.const 24 f32.sub f32.const 1 call $max_f32
    f32.const 14
    call $rect_store)

  (func (export "er_ui_drawer_detail_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 123824 call $er_ui_drawer_content_bounds drop
    local.get $out
    i32.const 123824 f32.load f32.const 12 f32.add
    i32.const 123828 f32.load f32.const 31 f32.add
    i32.const 123832 f32.load f32.const 24 f32.sub f32.const 1 call $max_f32
    f32.const 12
    call $rect_store)

  (func (export "er_ui_drawer_measure") (param $title_ptr i32) (param $title_len i32) (param $detail_ptr i32) (param $detail_len i32) (param $constraints i32) (param $out i32) (result i32)
    (local $trigger_w f32) (local $pref_w f32) (local $pref_h f32) (local $max_w f32) (local $max_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    call $er_ui_dialog_open_label_ptr i32.const 4 f32.const 16 i32.const 1 f32.const 8 call $er_ui_primitives_measured_label_width local.set $trigger_w
    i32.const 123840 f32.const 38 f32.store
    i32.const 123844 f32.const 10 f32.store
    i32.const 123848 f32.const 0 f32.store
    i32.const 123852 f32.const 10 f32.store
    local.get $constraints i32.const 123840 i32.const 123856 call $er_ui_layout_constraints_inner drop
    i32.const 123880 call $er_ui_drawer_panel drop
    local.get $title_ptr local.get $title_len local.get $detail_ptr local.get $detail_len i32.const 123856 i32.const 123880 i32.const 123912 call $er_ui_primitives_measure_title_detail_panel drop
    local.get $constraints local.get $trigger_w f32.const 16 f32.add i32.const 123920 f32.load f32.const 20 f32.add call $max_f32 call $er_ui_layout_axis_constraint_limit local.set $pref_w
    local.get $constraints i32.const 8 i32.add f32.const 38 i32.const 123924 f32.load f32.add call $er_ui_layout_axis_constraint_limit local.set $pref_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit local.set $max_w
    local.get $pref_h i32.const 123932 f32.load f32.const 38 f32.add call $max_f32 local.set $max_h
    f32.const 21 f32.const 93 local.get $pref_w local.get $pref_h local.get $max_w local.get $max_h local.get $out call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)
