(func (export "er_ui_text_component_line_height") (result f32) f32.const 18)
  (func (export "er_ui_text_component_max_lines") (result i32) i32.const 8)
  (func (export "er_ui_text_component_min_width") (result f32) f32.const 24)

  (func $er_ui_text_component_measure_value (export "er_ui_text_component_measure_value") (param $text_ptr i32) (param $text_len i32) (param $constraints i32) (param $metrics i32) (param $out i32) (result i32)
    local.get $constraints i32.eqz local.get $metrics i32.eqz i32.or local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $text_ptr local.get $text_len local.get $constraints local.get $metrics i32.const 120416 call $er_ui_layout_measure_text drop
    i32.const 120424 f32.load i32.const 120428 f32.load local.get $constraints i32.const 120448 call $er_ui_primitives_constrain_preferred_size drop
    f32.const 24 i32.const 120448 f32.load call $min_f32
    local.get $metrics f32.load i32.const 120452 f32.load call $min_f32
    i32.const 120448 f32.load
    i32.const 120452 f32.load
    i32.const 120432 f32.load
    i32.const 120436 f32.load
    local.get $out
    call $er_ui_layout_measurement_flexible
    drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)

  (func (export "er_ui_text_component_measure") (param $text_ptr i32) (param $text_len i32) (param $constraints i32) (param $out i32) (result i32)
    i32.const 120456
    f32.const 18
    local.get $text_ptr
    local.get $text_len
    f32.const 18
    call $primitives_average_width
    i32.const 8
    call $er_ui_layout_text_metrics_init
    drop
    local.get $text_ptr
    local.get $text_len
    local.get $constraints
    i32.const 120456
    local.get $out
    call $er_ui_text_component_measure_value)
