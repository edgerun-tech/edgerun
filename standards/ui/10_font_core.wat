(func (export "er_ui_version") (result i32)
    i32.const 1)

  (func (export "er_ui_record_kind_count") (result i32)
    i32.const 58)

  (func (export "er_ui_command_size") (result i32)
    i32.const 48)

  (func (export "er_ui_color_pack") (param $r i32) (param $g i32) (param $b i32) (param $a i32) (result i32)
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

  (func (export "er_ui_encode_unit") (param $value f32) (result i32)
    local.get $value
    f32.const 0
    f32.const 1
    call $clamp_f32
    f32.const 65535
    f32.mul
    f32.nearest
    i32.trunc_f32_u)

  (func (export "er_ui_decode_unit") (param $value i32) (result f32)
    local.get $value
    i32.const 65535
    i32.and
    f32.convert_i32_u
    f32.const 65535
    f32.div)

  (func (export "er_ui_font_reference_body_offset") (result i32)
    i32.const 148)

  (func (export "er_ui_font_glyph_record_size") (result i32)
    i32.const 20)

  (func (export "er_ui_font_kern_record_size") (result i32)
    i32.const 12)

  (func (export "er_ui_font_command_record_size") (result i32)
    i32.const 20)

  (func (export "er_ui_font_weight_regular") (result i32)
    i32.const 0)

  (func (export "er_ui_font_weight_semibold") (result i32)
    i32.const 1)

  (func (export "er_ui_font_weight_bold") (result i32)
    i32.const 2)

  (func (export "er_ui_font_weight_value") (param $weight i32) (result f32)
    local.get $weight
    i32.const 0
    i32.eq
    if
      f32.const 400
      return
    end
    local.get $weight
    i32.const 1
    i32.eq
    if
      f32.const 600
      return
    end
    local.get $weight
    i32.const 2
    i32.eq
    if
      f32.const 700
      return
    end
    f32.const 0)

  (func (export "er_ui_font_render_px") (param $origin_h f32) (result i32)
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

  (func $er_ui_font_should_snap_text (export "er_ui_font_should_snap_text") (param $px i32) (result i32)
    local.get $px
    i32.const 18
    i32.le_u)

  (func (export "er_ui_font_snap_text_position") (param $value f32) (param $px i32) (result f32)
    local.get $px
    call $er_ui_font_should_snap_text
    if (result f32)
      local.get $value
      f32.nearest
    else
      local.get $value
    end)

  (func (export "er_ui_font_glyph_key") (param $codepoint i32) (param $weight i32) (result i32)
    local.get $codepoint
    i32.const 0x10ffff
    i32.gt_u
    if
      i32.const 0
      return
    end
    local.get $weight
    i32.const 2
    i32.gt_u
    if
      i32.const 0
      return
    end
    local.get $codepoint
    i32.const 4
    i32.mul
    local.get $weight
    i32.add)

  (func (export "er_ui_font_glyph_key_codepoint") (param $glyph_key i32) (result i32)
    local.get $glyph_key
    i32.const 2
    i32.shr_u)

  (func (export "er_ui_font_glyph_key_weight") (param $glyph_key i32) (result i32)
    local.get $glyph_key
    i32.const 3
    i32.and)

  (func $font_body_valid_at (param $ptr i32) (param $len i32) (result i32)
    (local $glyphs i32)
    (local $commands i32)
    (local $kerns i32)
    (local $total i32)
    local.get $len
    i32.const 48
    i32.lt_u
    if
      i32.const 0
      return
    end
    local.get $ptr
    i32.load
    i32.const 0x4e465245
    i32.ne
    if
      i32.const 0
      return
    end
    local.get $ptr
    i32.const 4
    i32.add
    i32.load
    i32.const 0x0a335654
    i32.ne
    if
      i32.const 0
      return
    end
    local.get $ptr
    i32.const 8
    i32.add
    i32.load16_u
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $ptr
    i32.const 10
    i32.add
    i32.load16_u
    i32.const 0
    i32.ne
    if
      i32.const 0
      return
    end
    local.get $ptr
    i32.const 44
    i32.add
    i32.load
    i32.const 0
    i32.ne
    if
      i32.const 0
      return
    end
    local.get $ptr
    i32.const 12
    i32.add
    i32.load
    local.set $glyphs
    local.get $ptr
    i32.const 16
    i32.add
    i32.load
    local.set $commands
    local.get $ptr
    i32.const 20
    i32.add
    i32.load
    local.set $kerns
    i32.const 48
    local.get $glyphs
    i32.const 20
    i32.mul
    i32.add
    local.get $kerns
    i32.const 12
    i32.mul
    i32.add
    local.get $commands
    i32.const 20
    i32.mul
    i32.add
    local.set $total
    local.get $total
    local.get $len
    i32.eq)

  (func (export "er_ui_font_body_set") (param $ptr i32) (param $len i32) (result i32)
    local.get $ptr
    local.get $len
    call $font_body_valid_at
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $ptr
    global.set $font_body_ptr
    local.get $len
    global.set $font_body_len
    i32.const 0
    global.set $font_weight
    local.get $ptr
    global.set $font_regular_ptr
    local.get $len
    global.set $font_regular_len
    i32.const 1)

  (func (export "er_ui_font_body_set_weight") (param $weight i32) (param $ptr i32) (param $len i32) (result i32)
    local.get $ptr
    local.get $len
    call $font_body_valid_at
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $weight
    i32.const 0
    i32.eq
    if
      local.get $ptr
      global.set $font_regular_ptr
      local.get $len
      global.set $font_regular_len
      i32.const 1
      return
    end
    local.get $weight
    i32.const 1
    i32.eq
    if
      local.get $ptr
      global.set $font_semibold_ptr
      local.get $len
      global.set $font_semibold_len
      i32.const 1
      return
    end
    local.get $weight
    i32.const 2
    i32.eq
    if
      local.get $ptr
      global.set $font_bold_ptr
      local.get $len
      global.set $font_bold_len
      i32.const 1
      return
    end
    i32.const 0)

  (func (export "er_ui_font_select_weight") (param $weight i32) (result i32)
    local.get $weight
    i32.const 0
    i32.eq
    if
      global.get $font_regular_ptr
      global.get $font_regular_len
      call $font_body_valid_at
      i32.eqz
      if
        i32.const 0
        return
      end
      global.get $font_regular_ptr
      global.set $font_body_ptr
      global.get $font_regular_len
      global.set $font_body_len
      i32.const 0
      global.set $font_weight
      i32.const 1
      return
    end
    local.get $weight
    i32.const 1
    i32.eq
    if
      global.get $font_semibold_ptr
      global.get $font_semibold_len
      call $font_body_valid_at
      i32.eqz
      if
        i32.const 0
        return
      end
      global.get $font_semibold_ptr
      global.set $font_body_ptr
      global.get $font_semibold_len
      global.set $font_body_len
      i32.const 1
      global.set $font_weight
      i32.const 1
      return
    end
    local.get $weight
    i32.const 2
    i32.eq
    if
      global.get $font_bold_ptr
      global.get $font_bold_len
      call $font_body_valid_at
      i32.eqz
      if
        i32.const 0
        return
      end
      global.get $font_bold_ptr
      global.set $font_body_ptr
      global.get $font_bold_len
      global.set $font_body_len
      i32.const 2
      global.set $font_weight
      i32.const 1
      return
    end
    i32.const 0)

  (func (export "er_ui_font_selected_weight") (result i32)
    global.get $font_weight)

  (func (export "er_ui_font_weight_loaded") (param $weight i32) (result i32)
    local.get $weight
    i32.const 0
    i32.eq
    if
      global.get $font_regular_ptr
      global.get $font_regular_len
      call $font_body_valid_at
      return
    end
    local.get $weight
    i32.const 1
    i32.eq
    if
      global.get $font_semibold_ptr
      global.get $font_semibold_len
      call $font_body_valid_at
      return
    end
    local.get $weight
    i32.const 2
    i32.eq
    if
      global.get $font_bold_ptr
      global.get $font_bold_len
      call $font_body_valid_at
      return
    end
    i32.const 0)

  (func (export "er_ui_font_weight_body_ptr") (param $weight i32) (result i32)
    local.get $weight
    i32.const 0
    i32.eq
    if
      global.get $font_regular_ptr
      return
    end
    local.get $weight
    i32.const 1
    i32.eq
    if
      global.get $font_semibold_ptr
      return
    end
    local.get $weight
    i32.const 2
    i32.eq
    if
      global.get $font_bold_ptr
      return
    end
    i32.const 0)

  (func (export "er_ui_font_weight_body_len") (param $weight i32) (result i32)
    local.get $weight
    i32.const 0
    i32.eq
    if
      global.get $font_regular_len
      return
    end
    local.get $weight
    i32.const 1
    i32.eq
    if
      global.get $font_semibold_len
      return
    end
    local.get $weight
    i32.const 2
    i32.eq
    if
      global.get $font_bold_len
      return
    end
    i32.const 0)

  (func (export "er_ui_font_reference_loaded") (result i32)
    global.get $font_body_ptr
    global.get $font_body_len
    call $font_body_valid_at)

  (func (export "er_ui_font_body_ptr") (result i32)
    global.get $font_body_ptr)

  (func (export "er_ui_font_body_len") (result i32)
    global.get $font_body_len)

  (func $font_glyph_count (result i32)
    global.get $font_body_ptr
    i32.const 12
    i32.add
    i32.load)

  (func $font_command_count (result i32)
    global.get $font_body_ptr
    i32.const 16
    i32.add
    i32.load)

  (func $font_kern_count (result i32)
    global.get $font_body_ptr
    i32.const 20
    i32.add
    i32.load)

  (func (export "er_ui_font_ref_glyph_count") (result i32)
    call $font_glyph_count)

  (func (export "er_ui_font_ref_command_count") (result i32)
    call $font_command_count)

  (func (export "er_ui_font_ref_kern_count") (result i32)
    call $font_kern_count)

  (func $font_units_per_em (result f32)
    global.get $font_body_ptr
    i32.const 8
    i32.add
    i32.load16_u
    f32.convert_i32_u)

  (func (export "er_ui_font_ref_units_per_em") (result i32)
    global.get $font_body_ptr
    i32.const 8
    i32.add
    i32.load16_u)

  (func (export "er_ui_font_ref_ascender") (result f32)
    global.get $font_body_ptr
    i32.const 24
    i32.add
    f32.load)

  (func (export "er_ui_font_ref_descender") (result f32)
    global.get $font_body_ptr
    i32.const 28
    i32.add
    f32.load)

  (func (export "er_ui_font_ref_line_gap") (result f32)
    global.get $font_body_ptr
    i32.const 32
    i32.add
    f32.load)

  (func (export "er_ui_font_ref_y_min") (result f32)
    global.get $font_body_ptr
    i32.const 36
    i32.add
    f32.load)

  (func (export "er_ui_font_ref_y_max") (result f32)
    global.get $font_body_ptr
    i32.const 40
    i32.add
    f32.load)

  (func $font_glyph_record_ptr_by_index (param $index i32) (result i32)
    global.get $font_body_ptr
    i32.const 48
    i32.add
    local.get $index
    i32.const 20
    i32.mul
    i32.add)

  (func $font_glyph_record_ptr (param $codepoint i32) (result i32)
    (local $i i32)
    (local $count i32)
    global.get $font_body_ptr
    global.get $font_body_len
    call $font_body_valid_at
    i32.eqz
    if
      i32.const 0
      return
    end
    call $font_glyph_count
    local.set $count
    block $done
      loop $loop
        local.get $i
        local.get $count
        i32.ge_u
        br_if $done
        local.get $i
        call $font_glyph_record_ptr_by_index
        i32.load
        local.get $codepoint
        i32.eq
        if
          local.get $i
          call $font_glyph_record_ptr_by_index
          return
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $loop
      end
    end
    i32.const 0)

  (func (export "er_ui_font_ref_glyph_index") (param $codepoint i32) (result i32)
    (local $i i32)
    (local $count i32)
    global.get $font_body_ptr
    global.get $font_body_len
    call $font_body_valid_at
    i32.eqz
    if
      i32.const -1
      return
    end
    call $font_glyph_count
    local.set $count
    block $done
      loop $loop
        local.get $i
        local.get $count
        i32.ge_u
        br_if $done
        local.get $i
        call $font_glyph_record_ptr_by_index
        i32.load
        local.get $codepoint
        i32.eq
        if
          local.get $i
          return
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $loop
      end
    end
    i32.const -1)

  (func (export "er_ui_font_ref_glyph_id") (param $codepoint i32) (result i32)
    local.get $codepoint
    call $font_glyph_record_ptr
    local.tee $codepoint
    i32.eqz
    if
      i32.const -1
      return
    end
    local.get $codepoint
    i32.const 4
    i32.add
    i32.load16_u)

  (func (export "er_ui_font_ref_glyph_command_offset") (param $codepoint i32) (result i32)
    local.get $codepoint
    call $font_glyph_record_ptr
    local.tee $codepoint
    i32.eqz
    if
      i32.const -1
      return
    end
    local.get $codepoint
    i32.const 8
    i32.add
    i32.load)

  (func (export "er_ui_font_ref_glyph_command_count") (param $codepoint i32) (result i32)
    local.get $codepoint
    call $font_glyph_record_ptr
    local.tee $codepoint
    i32.eqz
    if
      i32.const -1
      return
    end
    local.get $codepoint
    i32.const 12
    i32.add
    i32.load)

  (func (export "er_ui_font_ref_glyph_advance_units") (param $codepoint i32) (result f32)
    local.get $codepoint
    call $font_glyph_record_ptr
    local.tee $codepoint
    i32.eqz
    if
      f32.const 0
      return
    end
    local.get $codepoint
    i32.const 16
    i32.add
    f32.load)

  (func (export "er_ui_font_ref_glyph_advance") (param $codepoint i32) (param $size f32) (result f32)
    local.get $codepoint
    call $font_glyph_record_ptr
    local.tee $codepoint
    i32.eqz
    if
      f32.const 0
      return
    end
    local.get $codepoint
    i32.const 16
    i32.add
    f32.load
    local.get $size
    f32.mul
    call $font_units_per_em
    f32.div)

  (func $font_command_base (result i32)
    global.get $font_body_ptr
    i32.const 48
    i32.add
    call $font_glyph_count
    i32.const 20
    i32.mul
    i32.add
    call $font_kern_count
    i32.const 12
    i32.mul
    i32.add)

  (func $font_command_ptr (param $command_index i32) (result i32)
    call $font_command_base
    local.get $command_index
    i32.const 20
    i32.mul
    i32.add)
