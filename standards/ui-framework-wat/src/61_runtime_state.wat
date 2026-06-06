  (func (export "er_ui_runtime_state_size") (result i32)
    i32.const 32)

  (func $er_ui_runtime_reset (export "er_ui_runtime_reset") (param $out i32) (result i32)
    i32.const 0
    global.set $runtime_hover
    i32.const 0
    global.set $runtime_focus
    i32.const 0
    global.set $runtime_active
    i32.const 0
    global.set $runtime_key
    f32.const 0
    global.set $runtime_pointer_x
    f32.const 0
    global.set $runtime_pointer_y
    i32.const 0
    global.set $runtime_overlay
    local.get $out
    i32.eqz
    i32.eqz
    if
      local.get $out
      call $store_runtime_state
    end
    i32.const 32)

  (func $er_ui_runtime_update_pointer (export "er_ui_runtime_update_pointer") (param $out i32) (param $x f32) (param $y f32) (param $hover_id i32) (result i32)
    local.get $x
    global.set $runtime_pointer_x
    local.get $y
    global.set $runtime_pointer_y
    local.get $hover_id
    global.set $runtime_hover
    local.get $out
    i32.eqz
    i32.eqz
    if
      local.get $out
      call $store_runtime_state
    end
    i32.const 32)

  (func $er_ui_runtime_update_focus (export "er_ui_runtime_update_focus") (param $out i32) (param $focus_id i32) (param $active_id i32) (result i32)
    local.get $focus_id
    global.set $runtime_focus
    local.get $active_id
    global.set $runtime_active
    local.get $out
    i32.eqz
    i32.eqz
    if
      local.get $out
      call $store_runtime_state
    end
    i32.const 32)

  (func (export "er_ui_runtime_update_key") (param $out i32) (param $key i32) (result i32)
    local.get $key
    global.set $runtime_key
    local.get $out
    i32.eqz
    i32.eqz
    if
      local.get $out
      call $store_runtime_state
    end
    i32.const 32)

  (func (export "er_ui_runtime_set_overlay") (param $out i32) (param $overlay_id i32) (result i32)
    local.get $overlay_id
    global.set $runtime_overlay
    local.get $out
    i32.eqz
    i32.eqz
    if
      local.get $out
      call $store_runtime_state
    end
    i32.const 32)

  (func (export "er_ui_runtime_snapshot") (param $out i32) (result i32)
    local.get $out
    i32.eqz
    i32.eqz
    if
      local.get $out
      call $store_runtime_state
    end
    i32.const 32)

  (func (export "er_ui_runtime_state_init") (param $out i32) (result i32)
    local.get $out
    call $er_ui_runtime_reset)

  (func (export "er_ui_runtime_set_hover") (param $out i32) (param $hover_id i32) (result i32)
    local.get $out
    global.get $runtime_pointer_x
    global.get $runtime_pointer_y
    local.get $hover_id
    call $er_ui_runtime_update_pointer)

  (func (export "er_ui_runtime_set_focus") (param $out i32) (param $focus_id i32) (result i32)
    local.get $out
    local.get $focus_id
    global.get $runtime_active
    call $er_ui_runtime_update_focus)

  (func (export "er_ui_runtime_pointer_event") (param $out i32) (param $x f32) (param $y f32) (param $hover_id i32) (result i32)
    local.get $out
    local.get $x
    local.get $y
    local.get $hover_id
    call $er_ui_runtime_update_pointer)
