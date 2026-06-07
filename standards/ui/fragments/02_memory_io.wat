(module


  (func $store16 (export "store16") (param $p i32) (param $v i32)
    local.get $p
    local.get $v
    i32.store16)

  (func $store32 (export "store32") (param $p i32) (param $v i32)
    local.get $p
    local.get $v
    i32.store)

  (func $load16 (export "load16") (param $p i32) (result i32)
    local.get $p
    i32.load16_u)

  (func $copy (export "copy") (param $dst i32) (param $src i32) (param $len i32)
    local.get $dst
    local.get $src
    local.get $len
    memory.copy)

  (func $zero (export "zero") (param $dst i32) (param $len i32)
    local.get $dst
    i32.const 0
    local.get $len
    memory.fill)

)
