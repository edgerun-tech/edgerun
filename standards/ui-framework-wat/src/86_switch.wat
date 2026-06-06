  (func (export "er_ui_switch_knob_size") (result f32) f32.const 14)
  (func (export "er_ui_switch_knob_inset") (result f32) f32.const 3)
  (func (export "er_ui_switch_label_gap") (result f32) f32.const 10)
  (func (export "er_ui_switch_label_height") (result f32) f32.const 16)
  (func (export "er_ui_switch_label_max_lines") (result i32) i32.const 2)
  (func (export "er_ui_switch_shadow_inset") (result f32) f32.const 1)
  (func (export "er_ui_switch_shadow_size") (result f32) f32.const 2)

  (func $er_ui_switch_pill_bounds (export "er_ui_switch_pill_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $bounds f32.load local.get $bounds i32.const 8 i32.add f32.load f32.add f32.const 36 f32.sub
    local.get $bounds i32.const 4 i32.add f32.load local.get $bounds i32.const 12 i32.add f32.load f32.const 20 f32.sub f32.const 0.5 f32.mul f32.add
    f32.const 36
    f32.const 20
    call $rect_store)

  (func (export "er_ui_switch_pill_shadow_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 122344 call $er_ui_switch_pill_bounds drop
    local.get $out
    i32.const 122344 f32.load f32.const 1 f32.sub
    i32.const 122344 i32.const 4 i32.add f32.load f32.const 1 f32.sub
    i32.const 122344 i32.const 8 i32.add f32.load f32.const 2 f32.add
    i32.const 122344 i32.const 12 i32.add f32.load f32.const 2 f32.add
    call $rect_store)

  (func $er_ui_switch_knob_bounds (export "er_ui_switch_knob_bounds") (param $bounds i32) (param $checked i32) (param $out i32) (result i32)
    (local $x f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 122360 call $er_ui_switch_pill_bounds drop
    local.get $checked
    if
      i32.const 122360 f32.load i32.const 122360 i32.const 8 i32.add f32.load f32.add f32.const 17 f32.sub local.set $x
    else
      i32.const 122360 f32.load f32.const 3 f32.add local.set $x
    end
    local.get $out
    local.get $x
    i32.const 122360 i32.const 4 i32.add f32.load f32.const 3 f32.add
    f32.const 14
    f32.const 14
    call $rect_store)

  (func (export "er_ui_switch_knob_shadow_bounds") (param $bounds i32) (param $checked i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds local.get $checked i32.const 122376 call $er_ui_switch_knob_bounds drop
    local.get $out
    i32.const 122376 f32.load f32.const 1 f32.sub
    i32.const 122376 i32.const 4 i32.add f32.load f32.const 1 f32.sub
    i32.const 122376 i32.const 8 i32.add f32.load f32.const 2 f32.add
    i32.const 122376 i32.const 12 i32.add f32.load f32.const 2 f32.add
    call $rect_store)

  (func (export "er_ui_switch_checked_marker_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 1 i32.const 122392 call $er_ui_switch_knob_bounds drop
    local.get $out
    i32.const 122392 f32.load f32.const 5 f32.add
    i32.const 122392 i32.const 4 i32.add f32.load f32.const 5 f32.add
    i32.const 122392 i32.const 8 i32.add f32.load f32.const 10 f32.sub f32.const 0 call $max_f32
    i32.const 122392 i32.const 12 i32.add f32.load f32.const 10 f32.sub f32.const 0 call $max_f32
    call $rect_store)

  (func $er_ui_switch_label_slot_bounds (export "er_ui_switch_label_slot_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 122408 call $er_ui_switch_pill_bounds drop
    local.get $out
    local.get $bounds f32.load
    local.get $bounds i32.const 4 i32.add f32.load
    i32.const 122408 f32.load local.get $bounds f32.load f32.sub f32.const 10 f32.sub f32.const 1 call $max_f32
    local.get $bounds i32.const 12 i32.add f32.load
    call $rect_store)

  (func (export "er_ui_switch_label_text_bounds") (param $bounds i32) (param $label_ptr i32) (param $label_len i32) (param $out i32) (result i32)
    (local $label_h f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 122424 call $er_ui_switch_label_slot_bounds drop
    local.get $label_ptr local.get $label_len i32.const 122424 i32.const 8 i32.add f32.load f32.const 16 i32.const 2 call $er_ui_alert_measured_text_height
    local.get $bounds i32.const 12 i32.add f32.load
    call $min_f32
    local.set $label_h
    local.get $out
    i32.const 122424 f32.load
    i32.const 122424 i32.const 4 i32.add f32.load i32.const 122424 i32.const 12 i32.add f32.load local.get $label_h f32.sub f32.const 0.5 f32.mul f32.add
    i32.const 122424 i32.const 8 i32.add f32.load
    local.get $label_h
    call $rect_store)
