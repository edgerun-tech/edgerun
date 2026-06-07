(func (export "er_ui_input_otp_slot_count") (result i32) i32.const 6)
  (func (export "er_ui_input_otp_slot_size") (result f32) f32.const 36)
  (func (export "er_ui_input_otp_slot_gap") (result f32) f32.const 0)
  (func (export "er_ui_input_otp_text_padding") (result f32) f32.const 8)

  (func (export "er_ui_input_otp_intrinsic_size") (param $out i32) (result i32)
    local.get $out i32.eqz
    if i32.const 0 return end
    local.get $out f32.const 216 f32.const 36 call $layout_store_size)

  (func (export "er_ui_input_otp_hit_id") (param $id i32) (param $index i32) (result i32)
    local.get $id local.get $index i32.const 5 call $layout_min_i32_u i32.add)

  (func (export "er_ui_input_otp_slot_bounds") (param $bounds i32) (param $index i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $bounds f32.load local.get $index f32.convert_i32_u f32.const 36 f32.mul f32.add
    local.get $bounds i32.const 4 i32.add f32.load
    f32.const 36
    local.get $bounds i32.const 12 i32.add f32.load f32.const 36 call $min_f32
    call $rect_store)

  (func (export "er_ui_input_otp_slot_text_bounds") (param $slot_bounds i32) (param $out i32) (result i32)
    (local $pad f32)
    local.get $slot_bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    f32.const 8
    local.get $slot_bounds i32.const 8 i32.add f32.load
    local.get $slot_bounds i32.const 12 i32.add f32.load
    call $min_f32
    f32.const 0.5
    f32.mul
    call $min_f32
    local.set $pad
    local.get $out
    local.get $slot_bounds f32.load local.get $pad f32.add
    local.get $slot_bounds i32.const 4 i32.add f32.load local.get $pad f32.add local.get $slot_bounds i32.const 12 i32.add f32.load local.get $pad f32.const 2 f32.mul f32.sub f32.const 16 f32.sub f32.const 0.5 f32.mul f32.add
    local.get $slot_bounds i32.const 8 i32.add f32.load local.get $pad f32.const 2 f32.mul f32.sub f32.const 0 call $max_f32
    f32.const 16
    call $rect_store)

  (func (export "er_ui_input_otp_measure") (param $constraints i32) (param $out i32) (result i32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    f32.const 216 f32.const 36 local.get $constraints i32.const 121980 call $er_ui_primitives_constrain_preferred_size drop
    i32.const 121980 f32.load
    i32.const 121984 f32.load
    i32.const 121980 f32.load
    i32.const 121984 f32.load
    i32.const 121980 f32.load
    i32.const 121984 f32.load
    local.get $out
    call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)
