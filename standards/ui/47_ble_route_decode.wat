(func (export "er_ui_ble_decode_route") (param $route i32) (param $len i32) (param $out i32) (param $cap i32) (result i32)
    (local $name_len i32)
    local.get $len
    i32.const 3
    i32.lt_u
    local.get $cap
    i32.const 12
    i32.lt_u
    i32.or
    if
      i32.const 0
      return
    end
    local.get $route
    i32.const 2
    i32.add
    i32.load8_u
    local.set $name_len
    local.get $len
    i32.const 3
    local.get $name_len
    i32.add
    i32.ne
    if
      i32.const 0
      return
    end
    local.get $out
    local.get $route
    i32.load8_u
    i32.store8
    local.get $out
    i32.const 1
    i32.add
    local.get $route
    i32.const 1
    i32.add
    i32.load8_u
    i32.store8
    local.get $out
    i32.const 4
    i32.add
    local.get $route
    i32.const 3
    i32.add
    call $store32
    local.get $out
    i32.const 8
    i32.add
    local.get $name_len
    call $store32
    i32.const 12)

  (func (export "er_ui_ble_decode_manufacturer_ad") (param $scan i32) (param $len i32) (param $out i32) (param $cap i32) (result i32)
    (local $index i32)
    (local $ad_len i32)
    (local $ad i32)
    (local $next i32)
    loop $loop
      local.get $index
      local.get $len
      i32.lt_u
      if
        local.get $scan
        local.get $index
        i32.add
        i32.load8_u
        local.tee $ad_len
        i32.eqz
        if
          i32.const 0
          return
        end
        local.get $index
        i32.const 1
        i32.add
        local.get $ad_len
        i32.add
        local.tee $next
        local.get $len
        i32.gt_u
        if
          i32.const 0
          return
        end
        local.get $scan
        local.get $index
        i32.const 1
        i32.add
        i32.add
        local.set $ad
        local.get $ad_len
        i32.const 3
        i32.ge_u
        local.get $ad
        i32.load8_u
        i32.const 0xff
        i32.eq
        i32.and
        local.get $ad
        i32.const 1
        i32.add
        call $load16
        i32.const 0xffff
        i32.eq
        i32.and
        if
          local.get $ad
          i32.const 3
          i32.add
          local.get $ad_len
          i32.const 3
          i32.sub
          local.get $out
          local.get $cap
          call $er_ui_ble_decode_frame
          return
        end
        local.get $next
        local.set $index
        br $loop
      end
    end
    i32.const 0)
