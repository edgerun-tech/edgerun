  (func (export "er_ui_ble_encode_frame") (param $out i32) (param $cap i32) (param $stream_id i32) (param $sequence i32) (param $kind i32) (param $body i32) (param $body_len i32) (result i32)
    local.get $body_len
    i32.const 19
    i32.gt_u
    local.get $kind
    i32.const 1
    i32.lt_u
    i32.or
    local.get $kind
    i32.const 4
    i32.gt_u
    i32.or
    local.get $cap
    local.get $body_len
    i32.const 8
    i32.add
    i32.lt_u
    i32.or
    if
      i32.const 0
      return
    end
    local.get $out
    i32.const 0x49555245
    call $store32
    local.get $out
    i32.const 4
    i32.add
    i32.const 1
    i32.store8
    local.get $out
    i32.const 5
    i32.add
    local.get $stream_id
    i32.store8
    local.get $out
    i32.const 6
    i32.add
    local.get $sequence
    i32.store8
    local.get $out
    i32.const 7
    i32.add
    local.get $kind
    i32.store8
    local.get $out
    i32.const 8
    i32.add
    local.get $body
    local.get $body_len
    call $copy
    local.get $body_len
    i32.const 8
    i32.add)

  (func $er_ui_ble_decode_frame (export "er_ui_ble_decode_frame") (param $frame i32) (param $len i32) (param $out i32) (param $cap i32) (result i32)
    local.get $len
    i32.const 8
    i32.lt_u
    local.get $len
    i32.const 27
    i32.gt_u
    i32.or
    local.get $cap
    i32.const 16
    i32.lt_u
    i32.or
    if
      i32.const 0
      return
    end
    local.get $frame
    i32.load
    i32.const 0x49555245
    i32.ne
    local.get $frame
    i32.const 4
    i32.add
    i32.load8_u
    i32.const 1
    i32.ne
    i32.or
    local.get $frame
    i32.const 7
    i32.add
    i32.load8_u
    i32.const 1
    i32.lt_u
    i32.or
    local.get $frame
    i32.const 7
    i32.add
    i32.load8_u
    i32.const 4
    i32.gt_u
    i32.or
    if
      i32.const 0
      return
    end
    local.get $out
    local.get $frame
    i32.const 5
    i32.add
    i32.load8_u
    i32.store8
    local.get $out
    i32.const 1
    i32.add
    local.get $frame
    i32.const 6
    i32.add
    i32.load8_u
    i32.store8
    local.get $out
    i32.const 2
    i32.add
    local.get $frame
    i32.const 7
    i32.add
    i32.load8_u
    i32.store8
    local.get $out
    i32.const 4
    i32.add
    local.get $frame
    i32.const 8
    i32.add
    call $store32
    local.get $out
    i32.const 8
    i32.add
    local.get $len
    i32.const 8
    i32.sub
    call $store32
    i32.const 12)

  (func (export "er_ui_ble_encode_route") (param $out i32) (param $cap i32) (param $route_id i32) (param $flags i32) (param $name i32) (param $name_len i32) (result i32)
    local.get $name_len
    i32.const 255
    i32.gt_u
    local.get $cap
    local.get $name_len
    i32.const 3
    i32.add
    i32.lt_u
    i32.or
    if
      i32.const 0
      return
    end
    local.get $out
    local.get $route_id
    i32.store8
    local.get $out
    i32.const 1
    i32.add
    local.get $flags
    i32.store8
    local.get $out
    i32.const 2
    i32.add
    local.get $name_len
    i32.store8
    local.get $out
    i32.const 3
    i32.add
    local.get $name
    local.get $name_len
    call $copy
    local.get $name_len
    i32.const 3
    i32.add)

  (func (export "er_ui_ble_encode_manufacturer_ad") (param $out i32) (param $cap i32) (param $frame i32) (param $frame_len i32) (result i32)
    local.get $frame_len
    i32.const 8
    i32.lt_u
    local.get $frame_len
    i32.const 27
    i32.gt_u
    i32.or
    local.get $cap
    local.get $frame_len
    i32.const 4
    i32.add
    i32.lt_u
    i32.or
    if
      i32.const 0
      return
    end
    local.get $frame
    i32.load
    i32.const 0x49555245
    i32.ne
    local.get $frame
    i32.const 4
    i32.add
    i32.load8_u
    i32.const 1
    i32.ne
    i32.or
    local.get $frame
    i32.const 7
    i32.add
    i32.load8_u
    i32.const 1
    i32.lt_u
    i32.or
    local.get $frame
    i32.const 7
    i32.add
    i32.load8_u
    i32.const 4
    i32.gt_u
    i32.or
    if
      i32.const 0
      return
    end
    local.get $out
    local.get $frame_len
    i32.const 3
    i32.add
    i32.store8
    local.get $out
    i32.const 1
    i32.add
    i32.const 0xff
    i32.store8
    local.get $out
    i32.const 2
    i32.add
    i32.const 0xffff
    call $store16
    local.get $out
    i32.const 4
    i32.add
    local.get $frame
    local.get $frame_len
    call $copy
    local.get $frame_len
    i32.const 4
    i32.add)
