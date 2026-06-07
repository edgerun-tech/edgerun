(module


  (func $er_ui_runtime_hover (export "er_ui_runtime_hover") (result i32)
    global.get $runtime_hover)

  (func $er_ui_runtime_focus (export "er_ui_runtime_focus") (result i32)
    global.get $runtime_focus)

  (func $er_ui_runtime_active (export "er_ui_runtime_active") (result i32)
    global.get $runtime_active)

  (func $er_ui_runtime_pointer_x (export "er_ui_runtime_pointer_x") (result f32)
    global.get $runtime_pointer_x)

  (func $er_ui_runtime_pointer_y (export "er_ui_runtime_pointer_y") (result f32)
    global.get $runtime_pointer_y)

  (func $er_ui_runtime_overlay (export "er_ui_runtime_overlay") (result i32)
    global.get $runtime_overlay)

  (func $er_ui_runtime_load_state (export "er_ui_runtime_load_state") (param $out i32)
    local.get $out
    call $store_runtime_state)
  (func $er_ui_runtime_state_size  (result i32)
    i32.const 32)

  (func $er_ui_runtime_reset  (param $out i32) (result i32)
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

  (func $er_ui_runtime_update_pointer  (param $out i32) (param $x f32) (param $y f32) (param $hover_id i32) (result i32)
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

  (func $er_ui_runtime_update_focus  (param $out i32) (param $focus_id i32) (param $active_id i32) (result i32)
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

  (func $er_ui_runtime_update_key  (param $out i32) (param $key i32) (result i32)
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

  (func $er_ui_runtime_set_overlay  (param $out i32) (param $overlay_id i32) (result i32)
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

  (func $er_ui_runtime_snapshot  (param $out i32) (result i32)
    local.get $out
    i32.eqz
    i32.eqz
    if
      local.get $out
      call $store_runtime_state
    end
    i32.const 32)

  (func $er_ui_runtime_state_init  (param $out i32) (result i32)
    local.get $out
    call $er_ui_runtime_reset)

  (func $er_ui_runtime_set_hover  (param $out i32) (param $hover_id i32) (result i32)
    local.get $out
    global.get $runtime_pointer_x
    global.get $runtime_pointer_y
    local.get $hover_id
    call $er_ui_runtime_update_pointer)

  (func $er_ui_runtime_set_focus  (param $out i32) (param $focus_id i32) (result i32)
    local.get $out
    local.get $focus_id
    global.get $runtime_active
    call $er_ui_runtime_update_focus)

  (func $er_ui_runtime_pointer_event  (param $out i32) (param $x f32) (param $y f32) (param $hover_id i32) (result i32)
    local.get $out
    local.get $x
    local.get $y
    local.get $hover_id
    call $er_ui_runtime_update_pointer)


  (func $store_runtime_state (param $out i32)
    local.get $out
    global.get $runtime_hover
    i32.store
    local.get $out
    i32.const 4
    i32.add
    global.get $runtime_focus
    i32.store
    local.get $out
    i32.const 8
    i32.add
    global.get $runtime_active
    i32.store
    local.get $out
    i32.const 12
    i32.add
    global.get $runtime_key
    i32.store
    local.get $out
    i32.const 16
    i32.add
    global.get $runtime_pointer_x
    f32.store
    local.get $out
    i32.const 20
    i32.add
    global.get $runtime_pointer_y
    f32.store
    local.get $out
    i32.const 24
    i32.add
    global.get $runtime_overlay
    i32.store
    local.get $out
    i32.const 28
    i32.add
    i32.const 0
    i32.store)

)
