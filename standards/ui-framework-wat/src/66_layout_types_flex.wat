  (func (export "er_ui_layout_axis_horizontal") (result i32) i32.const 0)
  (func (export "er_ui_layout_axis_vertical") (result i32) i32.const 1)
  (func (export "er_ui_layout_align_start") (result i32) i32.const 0)
  (func (export "er_ui_layout_align_stretch") (result i32) i32.const 1)
  (func (export "er_ui_layout_constraint_unconstrained") (result i32) i32.const 0)
  (func (export "er_ui_layout_constraint_at_most") (result i32) i32.const 1)
  (func (export "er_ui_layout_constraint_exact") (result i32) i32.const 2)
  (func (export "er_ui_layout_wrap_auto") (result i32) i32.const 0)
  (func (export "er_ui_layout_wrap_wrap") (result i32) i32.const 1)
  (func (export "er_ui_layout_wrap_nowrap") (result i32) i32.const 2)
  (func (export "er_ui_layout_wrap_truncate") (result i32) i32.const 3)
  (func (export "er_ui_layout_axis_constraint_size") (result i32) i32.const 8)
  (func (export "er_ui_layout_insets_size") (result i32) i32.const 16)
  (func (export "er_ui_layout_constraints_size") (result i32) i32.const 20)
  (func (export "er_ui_layout_measurement_size") (result i32) i32.const 24)
  (func (export "er_ui_layout_text_metrics_size") (result i32) i32.const 12)
  (func (export "er_ui_flex_options_size") (result i32) i32.const 28)
  (func (export "er_ui_flex_cursor_size") (result i32) i32.const 60)

  (func $layout_sanitize_size (param $value f32) (result f32)
    local.get $value
    call $finite_f32
    i32.eqz
    local.get $value
    f32.const 0
    f32.le
    i32.or
    if
      f32.const 0
      return
    end
    local.get $value)

  (func $layout_sanitize_positive (param $value f32) (param $fallback f32) (result f32)
    local.get $value
    call $finite_f32
    i32.eqz
    local.get $value
    f32.const 0
    f32.le
    i32.or
    if
      local.get $fallback
      return
    end
    local.get $value)

  (func $layout_min_i32_u (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.lt_u
    if (result i32)
      local.get $a
    else
      local.get $b
    end)

  (func $layout_max_i32_u (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.gt_u
    if (result i32)
      local.get $a
    else
      local.get $b
    end)

  (func $layout_store_size (param $out i32) (param $w f32) (param $h f32) (result i32)
    local.get $out
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $out
    local.get $w
    call $layout_sanitize_size
    f32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $h
    call $layout_sanitize_size
    f32.store
    i32.const 1)

  (func $layout_store_constraint (param $out i32) (param $tag i32) (param $value f32) (result i32)
    local.get $out
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $out
    local.get $tag
    i32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $value
    call $layout_sanitize_size
    f32.store
    i32.const 1)

  (func (export "er_ui_layout_axis_constraint_init") (param $out i32) (param $tag i32) (param $value f32) (result i32)
    local.get $out
    local.get $tag
    local.get $value
    call $layout_store_constraint)

  (func $er_ui_layout_axis_constraint_limit (export "er_ui_layout_axis_constraint_limit") (param $constraint i32) (param $fallback f32) (result f32)
    local.get $constraint
    i32.eqz
    if
      local.get $fallback
      return
    end
    local.get $constraint
    i32.load
    i32.eqz
    if
      local.get $fallback
      return
    end
    local.get $constraint
    i32.const 4
    i32.add
    f32.load
    call $layout_sanitize_size)

  (func (export "er_ui_layout_axis_constraint_exact_value") (param $constraint i32) (param $out i32) (result i32)
    local.get $constraint
    i32.eqz
    local.get $out
    i32.eqz
    i32.or
    if
      i32.const 0
      return
    end
    local.get $constraint
    i32.load
    i32.const 2
    i32.ne
    if
      i32.const 0
      return
    end
    local.get $out
    local.get $constraint
    i32.const 4
    i32.add
    f32.load
    call $layout_sanitize_size
    f32.store
    i32.const 1)

  (func $er_ui_layout_insets_uniform (export "er_ui_layout_insets_uniform") (param $value f32) (param $out i32) (result i32)
    (local $safe f32)
    local.get $out
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $value
    call $layout_sanitize_size
    local.set $safe
    local.get $out local.get $safe f32.store
    local.get $out i32.const 4 i32.add local.get $safe f32.store
    local.get $out i32.const 8 i32.add local.get $safe f32.store
    local.get $out i32.const 12 i32.add local.get $safe f32.store
    i32.const 1)

  (func $er_ui_layout_insets_horizontal (export "er_ui_layout_insets_horizontal") (param $insets i32) (result f32)
    local.get $insets
    i32.eqz
    if
      f32.const 0
      return
    end
    local.get $insets
    i32.const 12
    i32.add
    f32.load
    local.get $insets
    i32.const 4
    i32.add
    f32.load
    f32.add)

  (func $er_ui_layout_insets_vertical (export "er_ui_layout_insets_vertical") (param $insets i32) (result f32)
    local.get $insets
    i32.eqz
    if
      f32.const 0
      return
    end
    local.get $insets
    f32.load
    local.get $insets
    i32.const 8
    i32.add
    f32.load
    f32.add)

  (func $layout_shrink_constraint (param $constraint i32) (param $amount f32) (param $out i32) (result i32)
    (local $safe_amount f32)
    (local $tag i32)
    local.get $constraint i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $amount call $layout_sanitize_size local.set $safe_amount
    local.get $constraint i32.load local.set $tag
    local.get $tag i32.eqz
    if
      local.get $out i32.const 0 f32.const 0 call $layout_store_constraint return
    end
    local.get $out
    local.get $tag
    local.get $constraint i32.const 4 i32.add f32.load call $layout_sanitize_size
    local.get $safe_amount
    f32.sub
    f32.const 0
    call $max_f32
    call $layout_store_constraint)

  (func (export "er_ui_layout_constraints_init") (param $out i32) (param $width_tag i32) (param $width_value f32) (param $height_tag i32) (param $height_value f32) (param $text_wrap i32) (result i32)
    local.get $out
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $out local.get $width_tag local.get $width_value call $layout_store_constraint drop
    local.get $out i32.const 8 i32.add local.get $height_tag local.get $height_value call $layout_store_constraint drop
    local.get $out i32.const 16 i32.add local.get $text_wrap i32.store
    i32.const 1)

  (func $er_ui_layout_constraints_inner (export "er_ui_layout_constraints_inner") (param $constraints i32) (param $insets i32) (param $out i32) (result i32)
    local.get $constraints i32.eqz local.get $insets i32.eqz i32.or local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $constraints local.get $insets call $er_ui_layout_insets_horizontal local.get $out call $layout_shrink_constraint drop
    local.get $constraints i32.const 8 i32.add local.get $insets call $er_ui_layout_insets_vertical local.get $out i32.const 8 i32.add call $layout_shrink_constraint drop
    local.get $out i32.const 16 i32.add local.get $constraints i32.const 16 i32.add i32.load i32.store
    i32.const 1)

  (func $layout_measurement_store (param $out i32) (param $min_w f32) (param $min_h f32) (param $pref_w f32) (param $pref_h f32) (param $max_w f32) (param $max_h f32) (result i32)
    local.get $out i32.eqz
    if i32.const 0 return end
    local.get $out local.get $min_w local.get $min_h call $layout_store_size drop
    local.get $out i32.const 8 i32.add local.get $pref_w local.get $pref_h call $layout_store_size drop
    local.get $out i32.const 16 i32.add local.get $max_w local.get $max_h call $layout_store_size drop
    i32.const 1)

  (func $er_ui_layout_measurement_fixed (export "er_ui_layout_measurement_fixed") (param $w f32) (param $h f32) (param $out i32) (result i32)
    (local $sw f32) (local $sh f32)
    local.get $w call $layout_sanitize_size local.set $sw
    local.get $h call $layout_sanitize_size local.set $sh
    local.get $out local.get $sw local.get $sh local.get $sw local.get $sh local.get $sw local.get $sh call $layout_measurement_store)

  (func $er_ui_layout_measurement_flexible (export "er_ui_layout_measurement_flexible") (param $min_w f32) (param $min_h f32) (param $pref_w f32) (param $pref_h f32) (param $max_w f32) (param $max_h f32) (param $out i32) (result i32)
    (local $mnw f32) (local $mnh f32) (local $pfw f32) (local $pfh f32) (local $mxw f32) (local $mxh f32)
    local.get $min_w call $layout_sanitize_size local.set $mnw
    local.get $min_h call $layout_sanitize_size local.set $mnh
    local.get $pref_w call $layout_sanitize_size local.get $mnw call $max_f32 local.set $pfw
    local.get $pref_h call $layout_sanitize_size local.get $mnh call $max_f32 local.set $pfh
    local.get $max_w call $layout_sanitize_size local.get $pfw call $max_f32 local.set $mxw
    local.get $max_h call $layout_sanitize_size local.get $pfh call $max_f32 local.set $mxh
    local.get $out local.get $mnw local.get $mnh local.get $pfw local.get $pfh local.get $mxw local.get $mxh call $layout_measurement_store)

  (func $er_ui_layout_measurement_with_insets (export "er_ui_layout_measurement_with_insets") (param $measurement i32) (param $insets i32) (param $out i32) (result i32)
    (local $h f32) (local $v f32)
    local.get $measurement i32.eqz local.get $insets i32.eqz i32.or local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $insets call $er_ui_layout_insets_horizontal local.set $h
    local.get $insets call $er_ui_layout_insets_vertical local.set $v
    local.get $out
    local.get $measurement f32.load local.get $h f32.add
    local.get $measurement i32.const 4 i32.add f32.load local.get $v f32.add
    local.get $measurement i32.const 8 i32.add f32.load local.get $h f32.add
    local.get $measurement i32.const 12 i32.add f32.load local.get $v f32.add
    local.get $measurement i32.const 16 i32.add f32.load local.get $h f32.add
    local.get $measurement i32.const 20 i32.add f32.load local.get $v f32.add
    call $layout_measurement_store)

  (func $er_ui_layout_measurement_apply_exact (export "er_ui_layout_measurement_apply_exact") (param $measurement i32) (param $constraints i32) (param $out i32) (result i32)
    (local $pref_w f32) (local $pref_h f32)
    local.get $measurement i32.eqz local.get $constraints i32.eqz i32.or local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $measurement i32.const 8 i32.add f32.load local.set $pref_w
    local.get $measurement i32.const 12 i32.add f32.load local.set $pref_h
    local.get $constraints i32.load i32.const 2 i32.eq
    if local.get $constraints i32.const 4 i32.add f32.load call $layout_sanitize_size local.set $pref_w end
    local.get $constraints i32.const 8 i32.add i32.load i32.const 2 i32.eq
    if local.get $constraints i32.const 12 i32.add f32.load call $layout_sanitize_size local.set $pref_h end
    local.get $out
    local.get $measurement f32.load local.get $pref_w call $min_f32
    local.get $measurement i32.const 4 i32.add f32.load local.get $pref_h call $min_f32
    local.get $pref_w
    local.get $pref_h
    local.get $measurement i32.const 16 i32.add f32.load local.get $pref_w call $max_f32
    local.get $measurement i32.const 20 i32.add f32.load local.get $pref_h call $max_f32
    call $layout_measurement_store)

  (func $er_ui_layout_to_logical (export "er_ui_layout_to_logical") (param $axis i32) (param $w f32) (param $h f32) (param $out i32) (result i32)
    local.get $axis i32.const 1 i32.eq
    if
      local.get $out local.get $h local.get $w call $layout_store_size
      return
    end
    local.get $out local.get $w local.get $h call $layout_store_size)

  (func (export "er_ui_layout_from_logical") (param $axis i32) (param $w f32) (param $h f32) (param $out i32) (result i32)
    local.get $axis local.get $w local.get $h local.get $out call $er_ui_layout_to_logical)

  (func $er_ui_layout_text_metrics_init (export "er_ui_layout_text_metrics_init") (param $out i32) (param $line_height f32) (param $average_char_width f32) (param $max_lines i32) (result i32)
    local.get $out
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $out local.get $line_height f32.store
    local.get $out i32.const 4 i32.add local.get $average_char_width f32.store
    local.get $out i32.const 8 i32.add local.get $max_lines i32.store
    i32.const 1)

  (func $ui_utf8_sequence_len (param $first i32) (result i32)
    local.get $first
    i32.const 0x80
    i32.lt_u
    if
      i32.const 1
      return
    end
    local.get $first
    i32.const 0xc2
    i32.ge_u
    local.get $first
    i32.const 0xdf
    i32.le_u
    i32.and
    if
      i32.const 2
      return
    end
    local.get $first
    i32.const 0xe0
    i32.ge_u
    local.get $first
    i32.const 0xef
    i32.le_u
    i32.and
    if
      i32.const 3
      return
    end
    local.get $first
    i32.const 0xf0
    i32.ge_u
    local.get $first
    i32.const 0xf4
    i32.le_u
    i32.and
    if
      i32.const 4
      return
    end
    i32.const 0)

  (func $ui_utf8_valid_advance (param $ptr i32) (param $len i32) (param $index i32) (result i32)
    (local $seq i32)
    (local $end i32)
    (local $i i32)
    (local $codepoint i32)
    (local $byte i32)
    local.get $index
    local.get $len
    i32.ge_u
    if
      i32.const 0
      return
    end
    local.get $ptr
    local.get $index
    i32.add
    i32.load8_u
    call $ui_utf8_sequence_len
    local.tee $seq
    i32.eqz
    if
      i32.const 1
      return
    end
    local.get $index
    local.get $seq
    i32.add
    local.tee $end
    local.get $len
    i32.gt_u
    if
      local.get $len
      local.get $index
      i32.sub
      return
    end
    local.get $ptr
    local.get $index
    i32.add
    i32.load8_u
    local.set $byte
    local.get $seq
    i32.const 1
    i32.eq
    if
      i32.const 1
      return
    end
    local.get $seq
    i32.const 2
    i32.eq
    if
      local.get $byte
      i32.const 0x1f
      i32.and
      local.set $codepoint
    else
      local.get $seq
      i32.const 3
      i32.eq
      if
        local.get $byte
        i32.const 0x0f
        i32.and
        local.set $codepoint
      else
        local.get $byte
        i32.const 0x07
        i32.and
        local.set $codepoint
      end
    end
    i32.const 1 local.set $i
    block $done
      loop $cont
        local.get $i
        local.get $seq
        i32.ge_u
        br_if $done
        local.get $ptr
        local.get $index
        i32.add
        local.get $i
        i32.add
        i32.load8_u
        i32.const 0xc0
        i32.and
        i32.const 0x80
        i32.ne
        if
          i32.const 1
          return
        end
        local.get $codepoint
        i32.const 6
        i32.shl
        local.get $ptr
        local.get $index
        i32.add
        local.get $i
        i32.add
        i32.load8_u
        i32.const 0x3f
        i32.and
        i32.or
        local.set $codepoint
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $cont
      end
    end
    local.get $seq i32.const 2 i32.eq local.get $codepoint i32.const 0x80 i32.lt_u i32.and
    local.get $seq i32.const 3 i32.eq local.get $codepoint i32.const 0x800 i32.lt_u i32.and i32.or
    local.get $seq i32.const 4 i32.eq local.get $codepoint i32.const 0x10000 i32.lt_u i32.and i32.or
    local.get $codepoint i32.const 0x10ffff i32.gt_u i32.or
    local.get $codepoint i32.const 0xd800 i32.ge_u local.get $codepoint i32.const 0xdfff i32.le_u i32.and i32.or
    if
      i32.const 1
      return
    end
    local.get $seq)

  (func $ui_is_ascii_space_byte (param $byte i32) (result i32)
    local.get $byte i32.const 32 i32.eq
    local.get $byte i32.const 9 i32.eq i32.or
    local.get $byte i32.const 10 i32.eq i32.or
    local.get $byte i32.const 13 i32.eq i32.or)

  (func $er_ui_utf8_codepoint_count (export "er_ui_utf8_codepoint_count") (param $ptr i32) (param $len i32) (result i32)
    (local $index i32)
    (local $count i32)
    (local $seq i32)
    block $done
      loop $scan
        local.get $index
        local.get $len
        i32.ge_u
        br_if $done
        local.get $ptr
        local.get $index
        i32.add
        i32.load8_u
        call $ui_utf8_sequence_len
        local.tee $seq
        i32.eqz
        if
          i32.const 1
          local.set $seq
        end
        local.get $seq
        local.get $len
        local.get $index
        i32.sub
        i32.gt_u
        if
          local.get $len
          local.get $index
          i32.sub
          local.set $seq
        end
        local.get $index
        local.get $seq
        i32.add
        local.set $index
        local.get $count
        i32.const 1
        i32.add
        local.set $count
        br $scan
      end
    end
    local.get $count)

  (func $er_ui_text_skip_ascii_space (export "er_ui_text_skip_ascii_space") (param $ptr i32) (param $len i32) (param $start i32) (result i32)
    (local $index i32)
    local.get $start
    local.set $index
    block $done
      loop $spaces
        local.get $index
        local.get $len
        i32.ge_u
        br_if $done
        local.get $ptr
        local.get $index
        i32.add
        i32.load8_u
        i32.const 32
        i32.ne
        br_if $done
        local.get $index
        i32.const 1
        i32.add
        local.set $index
        br $spaces
      end
    end
    local.get $index)

  (func $er_ui_text_longest_utf8_run (export "er_ui_text_longest_utf8_run") (param $ptr i32) (param $len i32) (result i32)
    (local $index i32)
    (local $advance i32)
    (local $longest i32)
    (local $current i32)
    (local $byte i32)
    block $done
      loop $scan
        local.get $index
        local.get $len
        i32.ge_u
        br_if $done
        local.get $ptr
        local.get $index
        i32.add
        i32.load8_u
        local.set $byte
        local.get $byte
        call $ui_is_ascii_space_byte
        if
          local.get $longest
          local.get $current
          i32.lt_u
          if
            local.get $current
            local.set $longest
          end
          i32.const 0
          local.set $current
          i32.const 1
          local.set $advance
        else
          local.get $ptr
          local.get $len
          local.get $index
          call $ui_utf8_valid_advance
          local.set $advance
          local.get $current
          i32.const 1
          i32.add
          local.set $current
        end
        local.get $index
        local.get $advance
        i32.add
        local.set $index
        br $scan
      end
    end
    local.get $longest
    local.get $current
    i32.lt_u
    if (result i32)
      local.get $current
    else
      local.get $longest
    end)

  (func $er_ui_text_wrapped_line (export "er_ui_text_wrapped_line") (param $ptr i32) (param $len i32) (param $start i32) (param $char_capacity i32) (param $out i32) (result i32)
    (local $index i32)
    (local $chars i32)
    (local $last_space i32)
    local.get $out
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $start
    local.set $index
    i32.const -1
    local.set $last_space
    block $done
      loop $scan
        local.get $index
        local.get $len
        i32.ge_u
        br_if $done
        local.get $ptr
        local.get $index
        i32.add
        i32.load8_u
        i32.const 10
        i32.eq
        br_if $done
        local.get $chars
        local.get $char_capacity
        i32.ge_u
        br_if $done
        local.get $ptr
        local.get $index
        i32.add
        i32.load8_u
        i32.const 32
        i32.eq
        if
          local.get $index
          local.set $last_space
        end
        local.get $index
        i32.const 1
        i32.add
        local.set $index
        local.get $chars
        i32.const 1
        i32.add
        local.set $chars
        br $scan
      end
    end
    local.get $out
    local.get $start
    i32.store
    local.get $index
    local.get $len
    i32.ge_u
    local.get $index
    local.get $len
    i32.lt_u
    local.get $ptr
    local.get $index
    i32.add
    i32.load8_u
    i32.const 10
    i32.eq
    i32.and
    i32.or
    if
      local.get $out i32.const 4 i32.add local.get $index i32.store
      local.get $out i32.const 8 i32.add
      local.get $index
      i32.const 1
      i32.add
      local.get $len
      call $layout_min_i32_u
      i32.store
      i32.const 1
      return
    end
    local.get $last_space
    i32.const -1
    i32.ne
    if
      local.get $out i32.const 4 i32.add local.get $last_space i32.store
      local.get $out i32.const 8 i32.add local.get $last_space i32.const 1 i32.add i32.store
      i32.const 1
      return
    end
    local.get $out i32.const 4 i32.add local.get $index i32.store
    local.get $out i32.const 8 i32.add local.get $index i32.store
    i32.const 1)

  (func $er_ui_text_wrapped_line_count (export "er_ui_text_wrapped_line_count") (param $ptr i32) (param $len i32) (param $width f32) (param $average_char_width f32) (param $max_lines i32) (result i32)
    (local $byte_cursor i32)
    (local $line_count i32)
    (local $char_capacity i32)
    local.get $len
    i32.eqz
    local.get $max_lines
    i32.eqz
    i32.or
    if
      i32.const 0
      return
    end
    local.get $width
    local.get $average_char_width
    f32.div
    f32.const 1
    call $max_f32
    i32.trunc_f32_u
    local.tee $char_capacity
    i32.eqz
    if
      i32.const 1
      local.set $char_capacity
    end
    block $done
      loop $lines
        local.get $line_count
        local.get $max_lines
        i32.ge_u
        br_if $done
        local.get $ptr
        local.get $len
        local.get $byte_cursor
        call $er_ui_text_skip_ascii_space
        local.tee $byte_cursor
        local.get $len
        i32.ge_u
        if
          local.get $line_count
          return
        end
        local.get $ptr
        local.get $len
        local.get $byte_cursor
        local.get $char_capacity
        i32.const 120224
        call $er_ui_text_wrapped_line
        drop
        i32.const 120232
        i32.load
        local.set $byte_cursor
        local.get $line_count
        i32.const 1
        i32.add
        local.set $line_count
        br $lines
      end
    end
    local.get $line_count)

  (func $er_ui_layout_measure_text (export "er_ui_layout_measure_text") (param $text_ptr i32) (param $text_len i32) (param $constraints i32) (param $metrics i32) (param $out i32) (result i32)
    (local $line_height f32)
    (local $avg f32)
    (local $char_count i32)
    (local $longest i32)
    (local $natural_width f32)
    (local $min_width f32)
    (local $wrap_width f32)
    (local $measured_width f32)
    (local $line_count i32)
    (local $preferred_height f32)
    (local $pref_w f32)
    (local $pref_h f32)
    (local $max_width f32)
    (local $should_wrap i32)
    local.get $constraints i32.eqz local.get $metrics i32.eqz i32.or local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $text_len i32.eqz
    local.get $metrics i32.const 8 i32.add i32.load i32.eqz
    i32.or
    if
      f32.const 0 f32.const 0 local.get $out call $er_ui_layout_measurement_fixed
      return
    end
    local.get $metrics f32.load f32.const 16 call $layout_sanitize_positive local.set $line_height
    local.get $metrics i32.const 4 i32.add f32.load f32.const 8 call $layout_sanitize_positive local.set $avg
    local.get $text_ptr local.get $text_len call $er_ui_utf8_codepoint_count local.set $char_count
    local.get $text_ptr local.get $text_len call $er_ui_text_longest_utf8_run local.set $longest
    local.get $char_count f32.convert_i32_u local.get $avg f32.mul local.set $natural_width
    local.get $avg local.get $longest f32.convert_i32_u local.get $avg f32.mul call $max_f32 local.set $min_width
    local.get $constraints local.get $natural_width call $er_ui_layout_axis_constraint_limit local.set $wrap_width
    local.get $constraints i32.const 16 i32.add i32.load i32.const 1 i32.eq
    local.get $constraints i32.const 16 i32.add i32.load i32.eqz
    local.get $constraints i32.load i32.const 0 i32.ne
    i32.and
    i32.or
    local.set $should_wrap
    local.get $should_wrap
    if
      local.get $natural_width
      local.get $avg
      local.get $wrap_width
      call $max_f32
      call $min_f32
      local.set $measured_width
      local.get $text_ptr local.get $text_len local.get $measured_width local.get $avg local.get $metrics i32.const 8 i32.add i32.load call $er_ui_text_wrapped_line_count local.set $line_count
    else
      local.get $natural_width
      local.set $measured_width
      i32.const 1
      local.set $line_count
    end
    local.get $line_count
    i32.const 1
    call $layout_max_i32_u
    f32.convert_i32_u
    local.get $line_height
    f32.mul
    local.set $preferred_height
    local.get $measured_width local.set $pref_w
    local.get $preferred_height local.set $pref_h
    local.get $constraints i32.load i32.const 2 i32.eq
    if local.get $constraints i32.const 4 i32.add f32.load call $layout_sanitize_size local.set $pref_w end
    local.get $constraints i32.const 8 i32.add i32.load i32.const 2 i32.eq
    if local.get $constraints i32.const 12 i32.add f32.load call $layout_sanitize_size local.set $pref_h end
    local.get $constraints i32.load i32.eqz
    if
      local.get $natural_width local.set $max_width
    else
      local.get $constraints i32.const 4 i32.add f32.load call $layout_sanitize_size local.set $max_width
    end
    local.get $min_width local.get $pref_w call $min_f32
    local.get $line_height local.get $pref_h call $min_f32
    local.get $pref_w
    local.get $pref_h
    local.get $pref_w local.get $max_width call $max_f32
    local.get $pref_h local.get $preferred_height call $max_f32
    local.get $out
    call $er_ui_layout_measurement_flexible)

  (func $layout_measure_w (param $measurement i32) (param $slot i32) (result f32)
    local.get $measurement local.get $slot i32.add f32.load)

  (func $layout_measure_h (param $measurement i32) (param $slot i32) (result f32)
    local.get $measurement local.get $slot i32.add i32.const 4 i32.add f32.load)

  (func $layout_logical_main (param $measurement i32) (param $slot i32) (param $axis i32) (result f32)
    local.get $axis i32.const 1 i32.eq
    if (result f32)
      local.get $measurement local.get $slot call $layout_measure_h
    else
      local.get $measurement local.get $slot call $layout_measure_w
    end)

  (func $layout_logical_cross (param $measurement i32) (param $slot i32) (param $axis i32) (result f32)
    local.get $axis i32.const 1 i32.eq
    if (result f32)
      local.get $measurement local.get $slot call $layout_measure_w
    else
      local.get $measurement local.get $slot call $layout_measure_h
    end)

  (func (export "er_ui_flex_measure") (param $children i32) (param $child_count i32) (param $constraints i32) (param $axis i32) (param $gap f32) (param $insets i32) (param $out i32) (result i32)
    (local $i i32) (local $child i32) (local $gap_i f32)
    (local $min_main f32) (local $min_cross f32) (local $pref_main f32) (local $pref_cross f32) (local $max_main f32) (local $max_cross f32)
    local.get $children i32.eqz local.get $constraints i32.eqz i32.or local.get $insets i32.eqz i32.or local.get $out i32.eqz i32.or
    if i32.const 0 return end
    block $done
      loop $items
        local.get $i local.get $child_count i32.ge_u br_if $done
        local.get $children local.get $i i32.const 24 i32.mul i32.add local.set $child
        local.get $i i32.eqz if f32.const 0 local.set $gap_i else local.get $gap local.set $gap_i end
        local.get $min_main local.get $child i32.const 0 local.get $axis call $layout_logical_main local.get $gap_i f32.add f32.add local.set $min_main
        local.get $pref_main local.get $child i32.const 8 local.get $axis call $layout_logical_main local.get $gap_i f32.add f32.add local.set $pref_main
        local.get $max_main local.get $child i32.const 16 local.get $axis call $layout_logical_main local.get $gap_i f32.add f32.add local.set $max_main
        local.get $min_cross local.get $child i32.const 0 local.get $axis call $layout_logical_cross call $max_f32 local.set $min_cross
        local.get $pref_cross local.get $child i32.const 8 local.get $axis call $layout_logical_cross call $max_f32 local.set $pref_cross
        local.get $max_cross local.get $child i32.const 16 local.get $axis call $layout_logical_cross call $max_f32 local.set $max_cross
        local.get $i i32.const 1 i32.add local.set $i
        br $items
      end
    end
    local.get $axis i32.const 1 i32.eq
    if
      local.get $min_main local.get $min_cross local.set $min_main local.set $min_cross
      local.get $pref_main local.get $pref_cross local.set $pref_main local.set $pref_cross
      local.get $max_main local.get $max_cross local.set $max_main local.set $max_cross
    end
    local.get $min_main local.get $min_cross local.get $pref_main local.get $pref_cross local.get $max_main local.get $max_cross i32.const 120128 call $er_ui_layout_measurement_flexible drop
    i32.const 120128 local.get $insets i32.const 120160 call $er_ui_layout_measurement_with_insets drop
    i32.const 120160 local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)

  (func $flex_inner_rect (param $bounds i32) (param $insets i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $insets i32.eqz i32.or local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds
    local.get $out
    local.get $insets i32.const 12 i32.add f32.load
    local.get $insets f32.load
    local.get $insets i32.const 4 i32.add f32.load
    local.get $insets i32.const 8 i32.add f32.load
    call $er_ui_rect_inset_ltrb)

  (func $flex_inner_main (param $inner i32) (param $axis i32) (result f32)
    local.get $axis i32.const 1 i32.eq
    if (result f32)
      local.get $inner i32.const 12 i32.add f32.load
    else
      local.get $inner i32.const 8 i32.add f32.load
    end)

  (func $flex_inner_cross (param $inner i32) (param $axis i32) (result f32)
    local.get $axis i32.const 1 i32.eq
    if (result f32)
      local.get $inner i32.const 8 i32.add f32.load
    else
      local.get $inner i32.const 12 i32.add f32.load
    end)

  (func $er_ui_flex_resolve_main_sizes (export "er_ui_flex_resolve_main_sizes") (param $bounds i32) (param $children i32) (param $child_count i32) (param $axis i32) (param $gap f32) (param $insets i32) (param $out_sizes i32) (param $out_cap i32) (result i32)
    (local $count i32) (local $i i32) (local $child i32)
    (local $available f32) (local $available_children f32) (local $preferred_total f32) (local $min_total f32) (local $scale f32) (local $overflow f32) (local $shrink_capacity f32) (local $child_pref f32) (local $child_min f32) (local $child_shrink f32)
    local.get $bounds i32.eqz local.get $children i32.eqz i32.or local.get $insets i32.eqz i32.or local.get $out_sizes i32.eqz i32.or
    if i32.const 0 return end
    local.get $child_count local.get $out_cap i32.lt_u
    if local.get $child_count local.set $count else local.get $out_cap local.set $count end
    local.get $count i32.eqz if i32.const 0 return end
    local.get $bounds local.get $insets i32.const 120192 call $flex_inner_rect drop
    i32.const 120192 local.get $axis call $flex_inner_main local.set $available
    local.get $available local.get $gap local.get $count i32.const 1 i32.sub f32.convert_i32_u f32.mul f32.sub f32.const 0 call $max_f32 local.set $available_children
    block $scan_done
      loop $scan
        local.get $i local.get $count i32.ge_u br_if $scan_done
        local.get $children local.get $i i32.const 24 i32.mul i32.add local.set $child
        local.get $child i32.const 8 local.get $axis call $layout_logical_main local.set $child_pref
        local.get $child i32.const 0 local.get $axis call $layout_logical_main local.set $child_min
        local.get $out_sizes local.get $i i32.const 4 i32.mul i32.add local.get $child_pref f32.store
        local.get $preferred_total local.get $child_pref f32.add local.set $preferred_total
        local.get $min_total local.get $child_min f32.add local.set $min_total
        local.get $i i32.const 1 i32.add local.set $i
        br $scan
      end
    end
    local.get $preferred_total local.get $available_children f32.le
    if local.get $count return end
    local.get $min_total local.get $available_children f32.ge
    if
      local.get $min_total f32.const 0 f32.gt
      if local.get $available_children local.get $min_total f32.div local.set $scale else f32.const 0 local.set $scale end
      i32.const 0 local.set $i
      block $scale_done
        loop $scale_loop
          local.get $i local.get $count i32.ge_u br_if $scale_done
          local.get $children local.get $i i32.const 24 i32.mul i32.add i32.const 0 local.get $axis call $layout_logical_main local.get $scale f32.mul f32.const 1 call $max_f32 local.set $child_min
          local.get $out_sizes local.get $i i32.const 4 i32.mul i32.add local.get $child_min f32.store
          local.get $i i32.const 1 i32.add local.set $i
          br $scale_loop
        end
      end
      local.get $count
      return
    end
    local.get $preferred_total local.get $available_children f32.sub local.set $overflow
    local.get $preferred_total local.get $min_total f32.sub local.set $shrink_capacity
    i32.const 0 local.set $i
    block $shrink_done
      loop $shrink_loop
        local.get $i local.get $count i32.ge_u br_if $shrink_done
        local.get $children local.get $i i32.const 24 i32.mul i32.add local.set $child
        local.get $child i32.const 8 local.get $axis call $layout_logical_main local.set $child_pref
        local.get $child i32.const 0 local.get $axis call $layout_logical_main local.set $child_min
        local.get $child_pref local.get $child_min f32.sub local.set $child_shrink
        local.get $out_sizes local.get $i i32.const 4 i32.mul i32.add
        local.get $child_pref
        local.get $overflow
        local.get $child_shrink
        local.get $shrink_capacity
        f32.div
        f32.mul
        f32.sub
        f32.const 1
        call $max_f32
        f32.store
        local.get $i i32.const 1 i32.add local.set $i
        br $shrink_loop
      end
    end
    local.get $count)

  (func (export "er_ui_flex_place") (param $bounds i32) (param $children i32) (param $child_count i32) (param $axis i32) (param $gap f32) (param $insets i32) (param $cross_align i32) (param $out_rects i32) (param $out_cap i32) (param $scratch_sizes i32) (result i32)
    (local $count i32) (local $i i32) (local $child i32) (local $main_offset f32) (local $main_size f32) (local $cross_size f32) (local $inner_cross f32)
    local.get $bounds i32.eqz local.get $children i32.eqz i32.or local.get $insets i32.eqz i32.or local.get $out_rects i32.eqz i32.or local.get $scratch_sizes i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds local.get $children local.get $child_count local.get $axis local.get $gap local.get $insets local.get $scratch_sizes local.get $out_cap call $er_ui_flex_resolve_main_sizes local.set $count
    local.get $bounds local.get $insets i32.const 120192 call $flex_inner_rect drop
    i32.const 120192 local.get $axis call $flex_inner_cross local.set $inner_cross
    block $done
      loop $items
        local.get $i local.get $count i32.ge_u br_if $done
        local.get $i i32.eqz i32.eqz
        if local.get $main_offset local.get $gap f32.add local.set $main_offset end
        local.get $children local.get $i i32.const 24 i32.mul i32.add local.set $child
        local.get $scratch_sizes local.get $i i32.const 4 i32.mul i32.add f32.load local.set $main_size
        local.get $cross_align i32.eqz
        if
          local.get $child i32.const 8 local.get $axis call $layout_logical_cross local.get $inner_cross call $min_f32 local.set $cross_size
        else
          local.get $inner_cross local.set $cross_size
        end
        local.get $axis i32.const 1 i32.eq
        if
          local.get $out_rects local.get $i i32.const 16 i32.mul i32.add
          i32.const 120192 f32.load
          i32.const 120192 i32.const 4 i32.add f32.load local.get $main_offset f32.add
          local.get $cross_size
          local.get $main_size
          call $rect_store drop
        else
          local.get $out_rects local.get $i i32.const 16 i32.mul i32.add
          i32.const 120192 f32.load local.get $main_offset f32.add
          i32.const 120192 i32.const 4 i32.add f32.load
          local.get $main_size
          local.get $cross_size
          call $rect_store drop
        end
        local.get $main_offset local.get $main_size f32.add local.set $main_offset
        local.get $i i32.const 1 i32.add local.set $i
        br $items
      end
    end
    local.get $count)

  (func (export "er_ui_flex_options_init") (param $out i32) (param $axis i32) (param $gap f32) (param $insets i32) (param $cross_align i32) (result i32)
    local.get $out i32.eqz local.get $insets i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $axis i32.store
    local.get $out i32.const 4 i32.add local.get $gap f32.store
    local.get $out i32.const 8 i32.add local.get $insets f32.load f32.store
    local.get $out i32.const 12 i32.add local.get $insets i32.const 4 i32.add f32.load f32.store
    local.get $out i32.const 16 i32.add local.get $insets i32.const 8 i32.add f32.load f32.store
    local.get $out i32.const 20 i32.add local.get $insets i32.const 12 i32.add f32.load f32.store
    local.get $out i32.const 24 i32.add local.get $cross_align i32.store
    i32.const 1)

  (func (export "er_ui_flex_cursor_init") (param $cursor i32) (param $bounds i32) (param $options i32) (param $sizes_ptr i32) (param $sizes_len i32) (result i32)
    local.get $cursor i32.eqz local.get $bounds i32.eqz i32.or local.get $options i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds local.get $options i32.const 8 i32.add local.get $cursor call $flex_inner_rect drop
    local.get $cursor i32.const 16 i32.add local.get $options i32.load i32.store
    local.get $cursor i32.const 20 i32.add local.get $options i32.const 4 i32.add f32.load f32.store
    local.get $cursor i32.const 24 i32.add local.get $options i32.const 8 i32.add f32.load f32.store
    local.get $cursor i32.const 28 i32.add local.get $options i32.const 12 i32.add f32.load f32.store
    local.get $cursor i32.const 32 i32.add local.get $options i32.const 16 i32.add f32.load f32.store
    local.get $cursor i32.const 36 i32.add local.get $options i32.const 20 i32.add f32.load f32.store
    local.get $cursor i32.const 40 i32.add local.get $options i32.const 24 i32.add i32.load i32.store
    local.get $cursor i32.const 44 i32.add local.get $sizes_ptr i32.store
    local.get $cursor i32.const 48 i32.add local.get $sizes_len i32.store
    local.get $cursor i32.const 52 i32.add f32.const 0 f32.store
    local.get $cursor i32.const 56 i32.add i32.const 0 i32.store
    i32.const 1)

  (func $flex_cursor_axis (param $cursor i32) (result i32)
    local.get $cursor i32.const 16 i32.add i32.load)

  (func $flex_cursor_gap (param $cursor i32) (result f32)
    local.get $cursor i32.const 20 i32.add f32.load)

  (func $flex_cursor_cross_align (param $cursor i32) (result i32)
    local.get $cursor i32.const 40 i32.add i32.load)

  (func $flex_cursor_next_main_offset (param $cursor i32) (result f32)
    local.get $cursor i32.const 52 i32.add f32.load
    local.get $cursor i32.const 56 i32.add i32.load
    i32.eqz
    if (result f32)
      f32.const 0
    else
      local.get $cursor call $flex_cursor_gap
    end
    f32.add)

  (func $flex_cursor_child_main (param $cursor i32) (param $child i32) (result f32)
    (local $index i32)
    local.get $cursor i32.const 56 i32.add i32.load local.set $index
    local.get $cursor i32.const 44 i32.add i32.load
    i32.const 0
    i32.ne
    local.get $index
    local.get $cursor i32.const 48 i32.add i32.load
    i32.lt_u
    i32.and
    if (result f32)
      local.get $cursor i32.const 44 i32.add i32.load
      local.get $index i32.const 4 i32.mul i32.add
      f32.load
    else
      local.get $child i32.const 8 local.get $cursor call $flex_cursor_axis call $layout_logical_main
    end)

  (func $flex_cursor_child_cross (param $cursor i32) (param $child i32) (result f32)
    local.get $cursor call $flex_cursor_cross_align
    i32.eqz
    if (result f32)
      local.get $child i32.const 8 local.get $cursor call $flex_cursor_axis call $layout_logical_cross
      local.get $cursor local.get $cursor call $flex_cursor_axis call $flex_inner_cross
      call $min_f32
    else
      local.get $cursor local.get $cursor call $flex_cursor_axis call $flex_inner_cross
    end)

  (func $flex_cursor_rect_at (param $cursor i32) (param $main_offset f32) (param $child i32) (param $out i32) (result i32)
    (local $main_size f32) (local $cross_size f32)
    local.get $cursor local.get $child call $flex_cursor_child_main local.set $main_size
    local.get $cursor local.get $child call $flex_cursor_child_cross local.set $cross_size
    local.get $cursor call $flex_cursor_axis i32.const 1 i32.eq
    if
      local.get $out
      local.get $cursor f32.load
      local.get $cursor i32.const 4 i32.add f32.load local.get $main_offset f32.add
      local.get $cross_size
      local.get $main_size
      call $rect_store
      return
    end
    local.get $out
    local.get $cursor f32.load local.get $main_offset f32.add
    local.get $cursor i32.const 4 i32.add f32.load
    local.get $main_size
    local.get $cross_size
    call $rect_store)

  (func $flex_cursor_claim (param $cursor i32) (param $main_offset f32) (param $child i32)
    local.get $cursor i32.const 52 i32.add
    local.get $main_offset
    local.get $cursor local.get $child call $flex_cursor_child_main
    f32.add
    f32.store
    local.get $cursor i32.const 56 i32.add
    local.get $cursor i32.const 56 i32.add i32.load
    i32.const 1
    i32.add
    i32.store)

  (func (export "er_ui_flex_cursor_next") (param $cursor i32) (param $child i32) (param $out i32) (result i32)
    (local $main_offset f32)
    local.get $cursor i32.eqz local.get $child i32.eqz i32.or local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $cursor call $flex_cursor_next_main_offset local.set $main_offset
    local.get $cursor local.get $main_offset local.get $child local.get $out call $flex_cursor_rect_at drop
    local.get $cursor local.get $main_offset local.get $child call $flex_cursor_claim
    i32.const 1)

  (func (export "er_ui_flex_cursor_next_within_bounds") (param $cursor i32) (param $child i32) (param $out i32) (result i32)
    (local $main_offset f32)
    local.get $cursor i32.eqz local.get $child i32.eqz i32.or local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $cursor call $flex_cursor_next_main_offset local.set $main_offset
    local.get $main_offset
    local.get $cursor local.get $child call $flex_cursor_child_main
    f32.add
    local.get $cursor local.get $cursor call $flex_cursor_axis call $flex_inner_main
    f32.gt
    if
      i32.const 0
      return
    end
    local.get $cursor local.get $main_offset local.get $child local.get $out call $flex_cursor_rect_at drop
    local.get $cursor local.get $main_offset local.get $child call $flex_cursor_claim
    i32.const 1)
