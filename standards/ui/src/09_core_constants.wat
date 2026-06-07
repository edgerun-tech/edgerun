  (func $er_ui_version  (result i32)
    i32.const 1)

  (func $er_ui_record_kind_count  (result i32)
    i32.const 58)

  (func $er_ui_command_size  (result i32)
    i32.const 48)

  (func $er_ui_color_pack  (param $r i32) (param $g i32) (param $b i32) (param $a i32) (result i32)
    local.get $r
    i32.const 255
    i32.and
    local.get $g
    i32.const 255
    i32.and
    i32.const 8
    i32.shl
    i32.or
    local.get $b
    i32.const 255
    i32.and
    i32.const 16
    i32.shl
    i32.or
    local.get $a
    i32.const 255
    i32.and
    i32.const 24
    i32.shl
    i32.or)

  (func $er_ui_encode_unit  (param $value f32) (result i32)
    local.get $value
    f32.const 0
    f32.const 1
    call $clamp_f32
    f32.const 65535
    f32.mul
    f32.nearest
    i32.trunc_f32_u)

  (func $er_ui_decode_unit  (param $value i32) (result f32)
    local.get $value
    i32.const 65535
    i32.and
    f32.convert_i32_u
    f32.const 65535
    f32.div)

  (func $er_ui_font_reference_body_offset  (result i32)
    i32.const 148)

  (func $er_ui_font_glyph_record_size  (result i32)
    i32.const 20)

  (func $er_ui_font_kern_record_size  (result i32)
    i32.const 12)

  (func $er_ui_font_command_record_size  (result i32)
    i32.const 20)

  (func $er_ui_font_render_px  (param $origin_h f32) (result i32)
    local.get $origin_h
    f32.const 0
    f32.le
    if
      i32.const 0
      return
    end
    local.get $origin_h
    f32.ceil
    i32.trunc_f32_u)

  (func $er_ui_font_should_snap_text  (param $px i32) (result i32)
    local.get $px
    i32.const 18
    i32.le_u)

  (func $er_ui_font_snap_text_position  (param $value f32) (param $px i32) (result f32)
    local.get $px
    call $er_ui_font_should_snap_text
    if (result f32)
      local.get $value
      f32.nearest
    else
      local.get $value
    end)
