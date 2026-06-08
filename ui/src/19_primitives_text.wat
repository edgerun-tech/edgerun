

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

  (func $er_ui_utf8_codepoint_count  (param $ptr i32) (param $len i32) (result i32)
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

  (func $er_ui_text_skip_ascii_space  (param $ptr i32) (param $len i32) (param $start i32) (result i32)
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

  (func $er_ui_text_longest_utf8_run  (param $ptr i32) (param $len i32) (result i32)
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

  (func $er_ui_text_wrapped_line  (param $ptr i32) (param $len i32) (param $start i32) (param $char_capacity i32) (param $out i32) (result i32)
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
      call $min_i32_u
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

  (func $er_ui_text_wrapped_line_count  (param $ptr i32) (param $len i32) (param $width f32) (param $average_char_width f32) (param $max_lines i32) (result i32)
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
  (func $er_ui_primitives_min_extent  (result f32) f32.const 1)
  (func $er_ui_primitives_side_panel_layout_size  (result i32) i32.const 16)
  (func $er_ui_primitives_menu_list_layout_size  (result i32) i32.const 24)

  (func $er_ui_primitives_constrain_preferred_size  (param $preferred_w f32) (param $preferred_h f32) (param $constraints i32) (param $out i32) (result i32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $constraints local.get $preferred_w call $er_ui_layout_axis_constraint_limit
    local.get $constraints i32.const 8 i32.add local.get $preferred_h call $er_ui_layout_axis_constraint_limit
    call $layout_store_size)

  (func $er_ui_primitives_max_measured_width  (param $constraints i32) (param $preferred_width f32) (result f32)
    local.get $constraints local.get $preferred_width call $er_ui_layout_axis_constraint_limit)

  (func $er_ui_primitives_max_measured_height  (param $constraints i32) (param $preferred_height f32) (result f32)
    local.get $constraints i32.const 8 i32.add local.get $preferred_height call $er_ui_layout_axis_constraint_limit)

  (func $er_ui_primitives_max_measured_size  (param $constraints i32) (param $preferred_w f32) (param $preferred_h f32) (param $out i32) (result i32)
    local.get $preferred_w local.get $preferred_h local.get $constraints local.get $out call $er_ui_primitives_constrain_preferred_size)

  (func $er_ui_primitives_content_inset  (param $bounds i32) (param $padding f32) (param $out i32) (result i32)
    (local $clamped f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $padding
    f32.const 0
    call $max_f32
    local.get $bounds i32.const 8 i32.add f32.load
    local.get $bounds i32.const 12 i32.add f32.load
    call $min_f32
    f32.const 0.5
    f32.mul
    call $min_f32
    local.set $clamped
    local.get $bounds
    local.get $out
    local.get $clamped
    call $er_ui_rect_inset_uniform
    drop
    local.get $out
    call $er_ui_rect_valid)

  (func $er_ui_primitives_side_panel_layout  (param $out i32) (param $trigger_y f32) (param $trigger_w f32) (param $trigger_h f32) (param $gap f32) (result i32)
    local.get $out i32.eqz
    if i32.const 0 return end
    local.get $out local.get $trigger_y f32.store
    local.get $out i32.const 4 i32.add local.get $trigger_w f32.store
    local.get $out i32.const 8 i32.add local.get $trigger_h f32.store
    local.get $out i32.const 12 i32.add local.get $gap f32.store
    i32.const 1)

  (func $er_ui_primitives_side_panel_trigger_bounds  (param $bounds i32) (param $spec i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $spec i32.eqz i32.or local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $bounds f32.load
    local.get $bounds i32.const 4 i32.add f32.load local.get $spec f32.load f32.add
    local.get $spec i32.const 4 i32.add f32.load
    local.get $spec i32.const 8 i32.add f32.load
    call $rect_store)

  (func $er_ui_primitives_side_panel_content_bounds  (param $bounds i32) (param $spec i32) (param $out i32) (result i32)
    (local $x f32)
    local.get $bounds i32.eqz local.get $spec i32.eqz i32.or local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds f32.load
    local.get $spec i32.const 4 i32.add f32.load
    f32.add
    local.get $spec i32.const 12 i32.add f32.load
    f32.add
    local.set $x
    local.get $out
    local.get $x
    local.get $bounds i32.const 4 i32.add f32.load
    local.get $bounds f32.load local.get $bounds i32.const 8 i32.add f32.load f32.add local.get $x f32.sub f32.const 1 call $max_f32
    local.get $bounds i32.const 12 i32.add f32.load
    call $rect_store)

  (func $er_ui_primitives_overlay_trigger_offset  (result i32) i32.const 0)
  (func $er_ui_primitives_overlay_primary_offset  (result i32) i32.const 1)
  (func $er_ui_primitives_overlay_secondary_offset  (result i32) i32.const 2)
  (func $er_ui_primitives_overlay_trigger_id  (param $id i32) (result i32) local.get $id)
  (func $er_ui_primitives_overlay_primary_id  (param $id i32) (result i32) local.get $id i32.const 1 i32.add)
  (func $er_ui_primitives_overlay_secondary_id  (param $id i32) (result i32) local.get $id i32.const 2 i32.add)
  (func $er_ui_primitives_overlay_indexed_id  (param $id i32) (param $index i32) (result i32)
    local.get $id local.get $index i32.add i32.const 1 i32.add)

  (func $er_ui_primitives_menu_list_layout  (param $out i32) (param $padding f32) (param $item_h f32) (param $item_pitch f32) (param $item_radius f32) (param $item_padding f32) (param $item_text_h f32) (result i32)
    local.get $out i32.eqz
    if i32.const 0 return end
    local.get $out local.get $padding f32.store
    local.get $out i32.const 4 i32.add local.get $item_h f32.store
    local.get $out i32.const 8 i32.add local.get $item_pitch f32.store
    local.get $out i32.const 12 i32.add local.get $item_radius f32.store
    local.get $out i32.const 16 i32.add local.get $item_padding f32.store
    local.get $out i32.const 20 i32.add local.get $item_text_h f32.store
    i32.const 1)

  (func $er_ui_primitives_menu_item_bounds  (param $content i32) (param $index i32) (param $spec i32) (param $out i32) (result i32)
    local.get $content i32.eqz local.get $spec i32.eqz i32.or local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $content f32.load local.get $spec f32.load f32.add
    local.get $content i32.const 4 i32.add f32.load local.get $spec f32.load f32.add local.get $index f32.convert_i32_u local.get $spec i32.const 8 i32.add f32.load f32.mul f32.add
    local.get $content i32.const 8 i32.add f32.load local.get $spec f32.load f32.const 2 f32.mul f32.sub f32.const 1 call $max_f32
    local.get $spec i32.const 4 i32.add f32.load
    call $rect_store)

  (func $primitives_average_width (param $text_ptr i32) (param $text_len i32) (param $line_height f32) (result f32)
    (local $count i32)
    local.get $text_len
    i32.eqz
    if
      i32.const 120248
      i32.const 110
      i32.store8
      i32.const 120248
      i32.const 1
      local.get $line_height
      call $er_ui_font_text_width
      return
    end
    local.get $text_ptr
    local.get $text_len
    call $er_ui_utf8_codepoint_count
    local.tee $count
    i32.eqz
    if
      f32.const 1
      return
    end
    local.get $text_ptr
    local.get $text_len
    local.get $line_height
    call $er_ui_font_text_width
    local.get $count
    f32.convert_i32_u
    f32.div
    f32.const 1
    call $max_f32)

  (func $primitives_text_measure (param $text_ptr i32) (param $text_len i32) (param $constraints i32) (param $line_height f32) (param $max_lines i32) (param $out i32) (result i32)
    i32.const 120256
    local.get $line_height
    local.get $text_ptr
    local.get $text_len
    local.get $line_height
    call $primitives_average_width
    local.get $max_lines
    call $er_ui_layout_text_metrics_init
    drop
    local.get $text_ptr
    local.get $text_len
    local.get $constraints
    i32.const 120256
    local.get $out
    call $er_ui_layout_measure_text)

  (func $er_ui_primitives_measured_label_width  (param $text_ptr i32) (param $text_len i32) (param $line_height f32) (param $max_lines i32) (param $padding f32) (result f32)
    local.get $text_ptr
    local.get $text_len
    local.get $line_height
    call $er_ui_font_text_width
    local.get $padding
    f32.const 2
    f32.mul
    f32.add)

  (func $er_ui_primitives_measure_two_item_menu_panel  (param $first_ptr i32) (param $first_len i32) (param $second_ptr i32) (param $second_len i32) (param $constraints i32) (param $spec i32) (param $out i32) (result i32)
    (local $pref_w f32) (local $pref_h f32) (local $min_w f32) (local $min_h f32) (local $max_w f32)
    local.get $constraints i32.eqz local.get $spec i32.eqz i32.or local.get $out i32.eqz i32.or
    if i32.const 0 return end
    i32.const 120272
    local.get $spec f32.load
    local.get $spec i32.const 16 i32.add f32.load
    f32.add
    f32.store
    i32.const 120276
    local.get $spec f32.load
    local.get $spec i32.const 16 i32.add f32.load
    f32.add
    f32.store
    i32.const 120280
    local.get $spec f32.load
    f32.store
    i32.const 120284
    local.get $spec f32.load
    f32.store
    local.get $constraints
    i32.const 120272
    i32.const 120288
    call $er_ui_layout_constraints_inner
    drop
    local.get $first_ptr local.get $first_len i32.const 120288 local.get $spec i32.const 20 i32.add f32.load i32.const 1 i32.const 120320 call $primitives_text_measure drop
    local.get $second_ptr local.get $second_len i32.const 120288 local.get $spec i32.const 20 i32.add f32.load i32.const 1 i32.const 120344 call $primitives_text_measure drop
    local.get $constraints
    i32.const 120328 f32.load i32.const 120352 f32.load call $max_f32
    local.get $spec f32.load local.get $spec i32.const 16 i32.add f32.load f32.add f32.const 2 f32.mul f32.add
    call $er_ui_layout_axis_constraint_limit
    local.set $pref_w
    local.get $constraints i32.const 8 i32.add
    local.get $spec f32.load f32.const 2 f32.mul local.get $spec i32.const 4 i32.add f32.load f32.add local.get $spec i32.const 8 i32.add f32.load f32.add
    call $er_ui_layout_axis_constraint_limit
    local.set $pref_h
    f32.const 1 local.get $spec f32.load local.get $spec i32.const 16 i32.add f32.load f32.add f32.const 2 f32.mul f32.add local.set $min_w
    local.get $spec f32.load f32.const 2 f32.mul local.get $spec i32.const 4 i32.add f32.load f32.const 2 f32.mul f32.add local.set $min_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit local.set $max_w
    local.get $min_w local.get $min_h local.get $pref_w local.get $pref_h local.get $max_w local.get $pref_h local.get $out call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)

  (func $er_ui_primitives_title_detail_panel_size  (result i32) i32.const 32)

  (func $er_ui_primitives_title_detail_panel  (param $out i32) (param $radius f32) (param $padding f32) (param $title_y f32) (param $title_h f32) (param $detail_y f32) (param $detail_h f32) (param $title_right_inset f32) (param $title_max_lines i32) (param $detail_max_lines i32) (result i32)
    local.get $out i32.eqz
    if i32.const 0 return end
    local.get $out local.get $radius f32.store
    local.get $out i32.const 4 i32.add local.get $padding f32.store
    local.get $out i32.const 8 i32.add local.get $title_y f32.store
    local.get $out i32.const 12 i32.add local.get $title_h f32.store
    local.get $out i32.const 16 i32.add local.get $detail_y f32.store
    local.get $out i32.const 20 i32.add local.get $detail_h f32.store
    local.get $out i32.const 24 i32.add local.get $title_right_inset f32.store
    local.get $out i32.const 28 i32.add local.get $title_max_lines i32.store16
    local.get $out i32.const 30 i32.add local.get $detail_max_lines i32.store16
    i32.const 1)

  (func $er_ui_primitives_measure_title_detail_panel  (param $title_ptr i32) (param $title_len i32) (param $detail_ptr i32) (param $detail_len i32) (param $constraints i32) (param $spec i32) (param $out i32) (result i32)
    (local $detail_gap f32) (local $pref_w f32) (local $pref_h f32) (local $min_w f32) (local $min_h f32) (local $max_w f32) (local $max_h f32)
    local.get $constraints i32.eqz local.get $spec i32.eqz i32.or local.get $out i32.eqz i32.or
    if i32.const 0 return end
    i32.const 120272 local.get $spec i32.const 4 i32.add f32.load f32.store
    i32.const 120276 local.get $spec i32.const 4 i32.add f32.load local.get $spec i32.const 24 i32.add f32.load f32.add f32.store
    i32.const 120280 f32.const 0 f32.store
    i32.const 120284 f32.const 0 f32.store
    local.get $constraints i32.const 120272 i32.const 120288 call $er_ui_layout_constraints_inner drop
    i32.const 120272 local.get $spec i32.const 4 i32.add f32.load f32.store
    i32.const 120276 local.get $spec i32.const 4 i32.add f32.load f32.store
    local.get $constraints i32.const 120272 i32.const 120360 call $er_ui_layout_constraints_inner drop
    local.get $title_ptr local.get $title_len i32.const 120288 local.get $spec i32.const 12 i32.add f32.load local.get $spec i32.const 28 i32.add i32.load16_u i32.const 120320 call $primitives_text_measure drop
    local.get $detail_ptr local.get $detail_len i32.const 120360 local.get $spec i32.const 20 i32.add f32.load local.get $spec i32.const 30 i32.add i32.load16_u i32.const 120384 call $primitives_text_measure drop
    local.get $spec i32.const 16 i32.add f32.load local.get $spec i32.const 8 i32.add f32.load f32.sub local.get $spec i32.const 12 i32.add f32.load f32.sub f32.const 0 call $max_f32 local.set $detail_gap
    local.get $constraints
    i32.const 120328 f32.load local.get $spec i32.const 24 i32.add f32.load f32.add i32.const 120392 f32.load call $max_f32 local.get $spec i32.const 4 i32.add f32.load f32.const 2 f32.mul f32.add
    call $er_ui_layout_axis_constraint_limit
    local.set $pref_w
    local.get $constraints i32.const 8 i32.add
    local.get $spec i32.const 8 i32.add f32.load i32.const 120332 f32.load f32.add local.get $detail_gap f32.add i32.const 120396 f32.load f32.add local.get $spec i32.const 4 i32.add f32.load f32.add
    call $er_ui_layout_axis_constraint_limit
    local.set $pref_h
    f32.const 1 local.get $spec i32.const 4 i32.add f32.load f32.const 2 f32.mul f32.add local.get $spec i32.const 24 i32.add f32.load f32.add local.set $min_w
    local.get $spec i32.const 8 i32.add f32.load local.get $spec i32.const 12 i32.add f32.load f32.add local.get $detail_gap f32.add local.get $spec i32.const 20 i32.add f32.load f32.add local.get $spec i32.const 4 i32.add f32.load f32.add local.set $min_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit local.set $max_w
    local.get $pref_h i32.const 120340 f32.load local.get $detail_gap f32.add i32.const 120412 f32.load f32.add local.get $spec i32.const 8 i32.add f32.load f32.add local.get $spec i32.const 4 i32.add f32.load f32.add call $max_f32 local.set $max_h
    local.get $min_w local.get $min_h local.get $pref_w local.get $pref_h local.get $max_w local.get $max_h local.get $out call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)

  (func $er_ui_primitives_measure_side_panel_menu  (param $trigger_ptr i32) (param $trigger_len i32) (param $first_ptr i32) (param $first_len i32) (param $second_ptr i32) (param $second_len i32) (param $constraints i32) (param $panel i32) (param $trigger_padding f32) (param $menu i32) (param $out i32) (result i32)
    (local $trigger_w f32) (local $pref_w f32) (local $pref_h f32) (local $min_w f32) (local $min_h f32) (local $max_w f32) (local $max_h f32)
    local.get $constraints i32.eqz local.get $panel i32.eqz i32.or local.get $menu i32.eqz i32.or local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $trigger_ptr local.get $trigger_len f32.const 16 i32.const 1 local.get $trigger_padding call $er_ui_primitives_measured_label_width local.set $trigger_w
    i32.const 120272 f32.const 0 f32.store
    i32.const 120276 f32.const 0 f32.store
    i32.const 120280 f32.const 0 f32.store
    i32.const 120284 local.get $trigger_w local.get $panel i32.const 12 i32.add f32.load f32.add f32.store
    local.get $constraints i32.const 120272 i32.const 120288 call $er_ui_layout_constraints_inner drop
    local.get $first_ptr local.get $first_len local.get $second_ptr local.get $second_len i32.const 120288 local.get $menu i32.const 120320 call $er_ui_primitives_measure_two_item_menu_panel drop
    local.get $constraints
    local.get $trigger_w local.get $panel i32.const 12 i32.add f32.load f32.add i32.const 120328 f32.load f32.add
    call $er_ui_layout_axis_constraint_limit
    local.set $pref_w
    local.get $constraints i32.const 8 i32.add
    local.get $panel f32.load local.get $panel i32.const 8 i32.add f32.load f32.const 16 local.get $trigger_padding f32.const 2 f32.mul f32.add call $max_f32 f32.add
    i32.const 120332 f32.load
    call $max_f32
    call $er_ui_layout_axis_constraint_limit
    local.set $pref_h
    f32.const 2 local.get $panel i32.const 12 i32.add f32.load f32.add local.set $min_w
    local.get $panel f32.load f32.const 16 local.get $trigger_padding f32.const 2 f32.mul f32.add f32.add i32.const 120324 f32.load call $max_f32 local.set $min_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit local.set $max_w
    local.get $pref_h i32.const 120340 f32.load call $max_f32 local.set $max_h
    local.get $min_w local.get $min_h local.get $pref_w local.get $pref_h local.get $max_w local.get $max_h local.get $out call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)
  (func $er_ui_text_component_line_height  (result f32) f32.const 18)
  (func $er_ui_text_component_max_lines  (result i32) i32.const 8)
  (func $er_ui_text_component_min_width  (result f32) f32.const 24)

  (func $er_ui_text_component_measure_value  (param $text_ptr i32) (param $text_len i32) (param $constraints i32) (param $metrics i32) (param $out i32) (result i32)
    local.get $constraints i32.eqz local.get $metrics i32.eqz i32.or local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $text_ptr local.get $text_len local.get $constraints local.get $metrics i32.const 120416 call $er_ui_layout_measure_text drop
    i32.const 120424 f32.load i32.const 120428 f32.load local.get $constraints i32.const 120448 call $er_ui_primitives_constrain_preferred_size drop
    f32.const 24 i32.const 120448 f32.load call $min_f32
    local.get $metrics f32.load i32.const 120452 f32.load call $min_f32
    i32.const 120448 f32.load
    i32.const 120452 f32.load
    i32.const 120432 f32.load
    i32.const 120436 f32.load
    local.get $out
    call $er_ui_layout_measurement_flexible
    drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)

  (func $er_ui_text_component_measure  (param $text_ptr i32) (param $text_len i32) (param $constraints i32) (param $out i32) (result i32)
    i32.const 120456
    f32.const 18
    local.get $text_ptr
    local.get $text_len
    f32.const 18
    call $primitives_average_width
    i32.const 8
    call $er_ui_layout_text_metrics_init
    drop
    local.get $text_ptr
    local.get $text_len
    local.get $constraints
    i32.const 120456
    local.get $out
    call $er_ui_text_component_measure_value)

  (func $textarea_line_start_at (param $ptr i32) (param $len i32) (param $target_line i32) (result i32)
    (local $line i32) (local $index i32)
    block $done
      loop $scan
        local.get $index local.get $len i32.ge_u
        local.get $line local.get $target_line i32.ge_u
        i32.or
        br_if $done
        local.get $ptr local.get $index i32.add i32.load8_u i32.const 10 i32.eq
        if local.get $line i32.const 1 i32.add local.set $line end
        local.get $index i32.const 1 i32.add local.set $index
        br $scan
      end
    end
    local.get $index)

  (func $textarea_line_end_at (param $ptr i32) (param $len i32) (param $start i32) (result i32)
    (local $index i32)
    local.get $start local.get $len call $min_i32_u local.set $index
    block $done
      loop $scan
        local.get $index local.get $len i32.ge_u br_if $done
        local.get $ptr local.get $index i32.add i32.load8_u i32.const 10 i32.eq br_if $done
        local.get $index i32.const 1 i32.add local.set $index
        br $scan
      end
    end
    local.get $index)

  (func $er_ui_empty_measure  (param $title_ptr i32) (param $title_len i32) (param $detail_ptr i32) (param $detail_len i32) (param $constraints i32) (param $out i32) (result i32)
    (local $content_w f32) (local $pref_w f32) (local $pref_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    i32.const 120624 f32.const 0 f32.store
    i32.const 120628 f32.const 24 f32.store
    i32.const 120632 f32.const 0 f32.store
    i32.const 120636 f32.const 24 f32.store
    local.get $constraints i32.const 120624 i32.const 120640 call $er_ui_layout_constraints_inner drop
    i32.const 120668 f32.const 20 f32.store
    i32.const 120672 local.get $title_ptr local.get $title_len f32.const 20 call $primitives_average_width f32.store
    i32.const 120676 i32.const 2 i32.store
    local.get $title_ptr local.get $title_len i32.const 120640 i32.const 120668 i32.const 120680 call $er_ui_text_component_measure_value drop
    i32.const 120728 f32.const 16 f32.store
    i32.const 120732 local.get $detail_ptr local.get $detail_len f32.const 16 call $primitives_average_width f32.store
    i32.const 120736 i32.const 2 i32.store
    local.get $detail_ptr local.get $detail_len i32.const 120640 i32.const 120728 i32.const 120704 call $er_ui_text_component_measure_value drop
    f32.const 40 i32.const 120688 f32.load i32.const 120712 f32.load call $max_f32 call $max_f32 local.set $content_w
    local.get $constraints f32.const 144 local.get $content_w f32.const 48 f32.add call $max_f32 call $er_ui_layout_axis_constraint_limit local.set $pref_w
    local.get $constraints i32.const 8 i32.add f32.const 48 f32.const 40 f32.add f32.const 10 f32.add i32.const 120692 f32.load f32.add f32.const 4 f32.add i32.const 120716 f32.load f32.add call $er_ui_layout_axis_constraint_limit local.set $pref_h
    f32.const 144 local.get $pref_w call $min_f32
    f32.const 96 local.get $pref_h call $min_f32
    local.get $pref_w local.get $pref_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit
    local.get $pref_h
    local.get $out
    call $er_ui_layout_measurement_flexible
    drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)

  (func $er_ui_measure_intrinsic (param $w f32) (param $h f32) (param $constraints i32) (param $out i32) (result i32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $w local.get $h local.get $constraints i32.const 120856 call $er_ui_primitives_constrain_preferred_size drop
    i32.const 120856 f32.load
    i32.const 120860 f32.load
    i32.const 120856 f32.load
    i32.const 120860 f32.load
    i32.const 120856 f32.load
    i32.const 120860 f32.load
    local.get $out
    call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)

  (func $er_ui_measure_flexible_line (param $min_w f32) (param $height f32) (param $constraints i32) (param $out i32) (result i32)
    (local $pref_w f32) (local $pref_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $min_w local.get $height local.get $constraints i32.const 120856 call $er_ui_primitives_constrain_preferred_size drop
    i32.const 120856 f32.load local.set $pref_w
    i32.const 120860 f32.load local.set $pref_h
    f32.const 1 local.get $pref_w call $min_f32
    local.get $height local.get $pref_h call $min_f32
    local.get $pref_w local.get $pref_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit
    local.get $pref_h
    local.get $out
    call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)
