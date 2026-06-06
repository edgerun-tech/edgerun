  (func $er_ui_font_ascent (export "er_ui_font_ascent") (param $size f32) (result f32)
    local.get $size
    f32.const 0.8
    f32.mul)

  (func $er_ui_font_descent (export "er_ui_font_descent") (param $size f32) (result f32)
    local.get $size
    f32.const 0.2
    f32.mul)

  (func $er_ui_font_line_height (export "er_ui_font_line_height") (param $size f32) (result f32)
    local.get $size
    f32.const 1.25
    f32.mul)

  (func (export "er_ui_font_cap_height") (param $size f32) (result f32)
    local.get $size
    f32.const 0.72
    f32.mul)

  (func (export "er_ui_font_x_height") (param $size f32) (result f32)
    local.get $size
    f32.const 0.52
    f32.mul)

  (func (export "er_ui_font_baseline") (param $size f32) (result f32)
    local.get $size
    call $er_ui_font_line_gap
    f32.const 0.5
    f32.mul
    local.get $size
    call $er_ui_font_ascent
    f32.add)

  (func $er_ui_font_line_gap (export "er_ui_font_line_gap") (param $size f32) (result f32)
    local.get $size
    call $er_ui_font_line_height
    local.get $size
    call $er_ui_font_ascent
    f32.sub
    local.get $size
    call $er_ui_font_descent
    f32.sub)

  (func (export "er_ui_font_underline_position") (param $size f32) (result f32)
    local.get $size
    f32.const 0.08
    f32.mul)

  (func (export "er_ui_font_underline_thickness") (param $size f32) (result f32)
    local.get $size
    f32.const 0.07
    f32.mul
    f32.const 1
    call $max_f32)

  (func $er_ui_font_glyph_advance (export "er_ui_font_glyph_advance") (param $codepoint i32) (param $size f32) (result f32)
    (local $glyph i32)
    global.get $font_body_ptr
    global.get $font_body_len
    call $font_body_valid_at
    if
      local.get $codepoint
      call $font_glyph_record_ptr
      local.tee $glyph
      i32.const 0
      i32.ne
      if
        local.get $glyph
        i32.const 16
        i32.add
        f32.load
        local.get $size
        f32.mul
        call $font_units_per_em
        f32.div
        return
      end
    end
    local.get $codepoint
    i32.const 32
    i32.eq
    if (result f32)
      f32.const 0.33
    else
      local.get $codepoint
      i32.const 105
      i32.eq
      local.get $codepoint
      i32.const 108
      i32.eq
      i32.or
      local.get $codepoint
      i32.const 73
      i32.eq
      i32.or
      if (result f32)
        f32.const 0.32
      else
        local.get $codepoint
        i32.const 77
        i32.eq
        local.get $codepoint
        i32.const 87
        i32.eq
        i32.or
        local.get $codepoint
        i32.const 109
        i32.eq
        i32.or
        local.get $codepoint
        i32.const 119
        i32.eq
        i32.or
        if (result f32)
          f32.const 0.9
        else
          local.get $codepoint
          i32.const 48
          i32.ge_u
          local.get $codepoint
          i32.const 57
          i32.le_u
          i32.and
          if (result f32)
            f32.const 0.58
          else
            local.get $codepoint
            i32.const 65
            i32.ge_u
            local.get $codepoint
            i32.const 90
            i32.le_u
            i32.and
            if (result f32)
              f32.const 0.64
            else
              local.get $codepoint
              i32.const 44
              i32.eq
              local.get $codepoint
              i32.const 46
              i32.eq
              i32.or
              local.get $codepoint
              i32.const 58
              i32.eq
              i32.or
              local.get $codepoint
              i32.const 59
              i32.eq
              i32.or
              if (result f32)
                f32.const 0.28
              else
                f32.const 0.56
              end
            end
          end
        end
      end
    end
    local.get $size
    f32.mul)

  (func $er_ui_font_text_width (export "er_ui_font_text_width") (param $ptr i32) (param $len i32) (param $size f32) (result f32)
    (local $i i32)
    (local $width f32)
    block $done
      loop $loop
        local.get $i
        local.get $len
        i32.ge_u
        br_if $done
        local.get $width
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        local.get $size
        call $er_ui_font_glyph_advance
        f32.add
        local.set $width
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $loop
      end
    end
    local.get $width)

  (func $er_ui_font_text_fit_len (export "er_ui_font_text_fit_len") (param $ptr i32) (param $len i32) (param $size f32) (param $max_w f32) (result i32)
    (local $i i32)
    (local $width f32)
    local.get $max_w
    f32.const 0
    f32.lt
    if
      i32.const 0
      return
    end
    block $done
      loop $loop
        local.get $i
        local.get $len
        i32.ge_u
        br_if $done
        local.get $width
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        local.get $size
        call $er_ui_font_glyph_advance
        f32.add
        local.tee $width
        local.get $max_w
        f32.gt
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
    local.get $len)

  (func $er_ui_font_text_fits (export "er_ui_font_text_fits") (param $ptr i32) (param $len i32) (param $size f32) (param $max_w f32) (result i32)
    local.get $ptr
    local.get $len
    local.get $size
    call $er_ui_font_text_width
    local.get $max_w
    f32.le)

  (func (export "er_ui_font_text_truncate_len") (param $ptr i32) (param $len i32) (param $size f32) (param $max_w f32) (param $ellipsis_ptr i32) (param $ellipsis_len i32) (result i32)
    (local $ellipsis_w f32)
    local.get $ptr
    local.get $len
    local.get $size
    local.get $max_w
    call $er_ui_font_text_fits
    if
      local.get $len
      return
    end
    local.get $ellipsis_ptr
    local.get $ellipsis_len
    local.get $size
    call $er_ui_font_text_width
    local.tee $ellipsis_w
    local.get $max_w
    f32.gt
    if
      i32.const 0
      return
    end
    local.get $ptr
    local.get $len
    local.get $size
    local.get $max_w
    local.get $ellipsis_w
    f32.sub
    call $er_ui_font_text_fit_len)

  (func (export "er_ui_font_measure_text") (param $ptr i32) (param $len i32) (param $size f32) (param $out i32) (result i32)
    local.get $out
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $out
    local.get $ptr
    local.get $len
    local.get $size
    call $er_ui_font_text_width
    f32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $size
    call $er_ui_font_ascent
    f32.store
    local.get $out
    i32.const 8
    i32.add
    local.get $size
    call $er_ui_font_descent
    f32.store
    local.get $out
    i32.const 12
    i32.add
    local.get $size
    call $er_ui_font_line_height
    f32.store
    i32.const 1)

  (func (export "er_ui_font_text_box") (param $ptr i32) (param $len i32) (param $size f32) (param $max_w f32) (param $out i32) (result i32)
    (local $fit_len i32)
    local.get $out
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $ptr
    local.get $len
    local.get $size
    local.get $max_w
    call $er_ui_font_text_fit_len
    local.set $fit_len
    local.get $out
    local.get $ptr
    local.get $fit_len
    local.get $size
    call $er_ui_font_text_width
    f32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $size
    call $er_ui_font_line_height
    f32.store
    local.get $out
    i32.const 8
    i32.add
    local.get $fit_len
    i32.store
    local.get $out
    i32.const 12
    i32.add
    local.get $fit_len
    local.get $len
    i32.eq
    i32.store
    i32.const 1)
