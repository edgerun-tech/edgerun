(func $er_ui_patch_bool_value (export "er_ui_patch_bool_value") (param $patch i32) (param $len i32) (result i32)
    local.get $len
    i32.const 3
    i32.lt_u
    if
      i32.const -1
      return
    end
    local.get $patch
    i32.const 2
    i32.add
    i32.load8_u
    i32.const 0
    i32.ne)

  (func $er_ui_patch_u16_value (export "er_ui_patch_u16_value") (param $patch i32) (param $len i32) (result i32)
    local.get $len
    i32.const 4
    i32.lt_u
    if
      i32.const -1
      return
    end
    local.get $patch
    i32.const 2
    i32.add
    call $load16)

  (func (export "er_ui_patch_f32_value") (param $patch i32) (param $len i32) (result f32)
    local.get $len
    i32.const 6
    i32.lt_u
    if
      f32.const nan
      return
    end
    local.get $patch
    i32.const 2
    i32.add
    f32.load)

  (func $er_ui_patch_string_ptr (export "er_ui_patch_string_ptr") (param $patch i32) (param $len i32) (result i32)
    local.get $len
    i32.const 3
    i32.lt_u
    if
      i32.const 0
      return
    end
    local.get $patch
    i32.const 3
    i32.add)

  (func $er_ui_patch_string_len (export "er_ui_patch_string_len") (param $patch i32) (param $len i32) (result i32)
    (local $n i32)
    local.get $len
    i32.const 3
    i32.lt_u
    if
      i32.const 0
      return
    end
    local.get $patch
    i32.const 2
    i32.add
    i32.load8_u
    local.tee $n
    i32.const 3
    i32.add
    local.get $len
    i32.gt_u
    if
      i32.const 0
      return
    end
    local.get $n)

  (func $er_ui_patch_second_string_ptr (export "er_ui_patch_second_string_ptr") (param $patch i32) (param $len i32) (result i32)
    (local $a i32)
    local.get $len
    i32.const 4
    i32.lt_u
    if
      i32.const 0
      return
    end
    local.get $patch
    i32.const 2
    i32.add
    i32.load8_u
    local.set $a
    local.get $len
    i32.const 4
    local.get $a
    i32.add
    i32.lt_u
    if
      i32.const 0
      return
    end
    local.get $patch
    i32.const 4
    i32.add
    local.get $a
    i32.add)

  (func $er_ui_patch_second_string_len (export "er_ui_patch_second_string_len") (param $patch i32) (param $len i32) (result i32)
    (local $a i32)
    (local $b i32)
    local.get $len
    i32.const 4
    i32.lt_u
    if
      i32.const 0
      return
    end
    local.get $patch
    i32.const 2
    i32.add
    i32.load8_u
    local.set $a
    local.get $len
    i32.const 4
    local.get $a
    i32.add
    i32.lt_u
    if
      i32.const 0
      return
    end
    local.get $patch
    i32.const 3
    i32.add
    local.get $a
    i32.add
    i32.load8_u
    local.tee $b
    i32.const 4
    local.get $a
    i32.add
    i32.add
    local.get $len
    i32.gt_u
    if
      i32.const 0
      return
    end
    local.get $b)

  (func $er_ui_patch_color_value (export "er_ui_patch_color_value") (param $patch i32) (param $len i32) (result i32)
    local.get $len
    i32.const 6
    i32.lt_u
    if
      i32.const 0
      return
    end
    local.get $patch
    i32.const 2
    i32.add
    i32.load)
