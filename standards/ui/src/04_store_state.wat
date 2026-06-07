
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
