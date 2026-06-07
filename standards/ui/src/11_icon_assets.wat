  (func $icon_pack_loaded (result i32)
    global.get $icon_index_ptr
    i32.const 0
    i32.ne
    global.get $icon_ir_ptr
    i32.const 0
    i32.ne
    i32.and
    global.get $icon_index_len
    i32.const 8
    i32.ge_u
    i32.and)

  (func $icon_pack_count (result i32)
    call $icon_pack_loaded
    if (result i32)
      global.get $icon_index_ptr
      i32.const 4
      i32.add
      i32.load
    else
      i32.const 3
    end)

  (func $icon_pack_entry_offset (param $icon i32) (result i32)
    global.get $icon_index_ptr
    i32.const 8
    i32.add
    local.get $icon
    i32.const 1
    i32.sub
    i32.const 8
    i32.mul
    i32.add)

  (func $icon_pack_entry_valid (param $icon i32) (result i32)
    (local $entry i32)
    (local $off i32)
    (local $len i32)
    call $icon_pack_loaded
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $icon
    i32.const 1
    i32.ge_u
    local.get $icon
    call $icon_pack_count
    i32.le_u
    i32.and
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $icon
    call $icon_pack_entry_offset
    local.set $entry
    local.get $entry
    i32.load
    local.set $off
    local.get $entry
    i32.const 4
    i32.add
    i32.load
    local.set $len
    local.get $len
    i32.const 4
    i32.rem_u
    i32.eqz
    local.get $off
    local.get $len
    i32.add
    global.get $icon_ir_len
    i32.le_u
    i32.and)

  (func $er_ui_icon_asset_pack_set  (param $index_ptr i32) (param $index_len i32) (param $ir_ptr i32) (param $ir_len i32) (param $names_ptr i32) (param $names_len i32) (result i32)
    (local $count i32)
    local.get $index_len
    i32.const 8
    i32.lt_u
    local.get $ir_len
    i32.eqz
    i32.or
    if
      i32.const 0
      return
    end
    local.get $index_ptr
    i32.load
    i32.const 0x4e435849
    i32.ne
    if
      i32.const 0
      return
    end
    local.get $index_ptr
    i32.const 4
    i32.add
    i32.load
    local.set $count
    local.get $count
    i32.eqz
    local.get $index_len
    i32.const 8
    local.get $count
    i32.const 8
    i32.mul
    i32.add
    i32.ne
    i32.or
    if
      i32.const 0
      return
    end
    local.get $index_ptr
    global.set $icon_index_ptr
    local.get $index_len
    global.set $icon_index_len
    local.get $ir_ptr
    global.set $icon_ir_ptr
    local.get $ir_len
    global.set $icon_ir_len
    local.get $names_ptr
    global.set $icon_names_ptr
    local.get $names_len
    global.set $icon_names_len
    i32.const 1)

  (func $er_ui_icon_asset_pack_loaded  (result i32)
    call $icon_pack_loaded)

  (func $er_ui_icon_segment_size  (result i32)
    i32.const 20)

  (func $er_ui_icon_viewbox  (result f32)
    f32.const 24)

  (func $er_ui_icon_count  (result i32)
    call $icon_pack_count)

  (func $er_ui_icon_valid  (param $icon i32) (result i32)
    call $icon_pack_loaded
    if
      local.get $icon
      call $icon_pack_entry_valid
      return
    end
    local.get $icon
    i32.const 1
    i32.ge_u
    local.get $icon
    i32.const 3
    i32.le_u
    i32.and)

  (func $er_ui_icon_path_ptr  (param $icon i32) (result i32)
    local.get $icon
    i32.const 1
    i32.eq
    if
      i32.const 65100
      return
    end
    local.get $icon
    i32.const 2
    i32.eq
    if
      i32.const 65132
      return
    end
    local.get $icon
    i32.const 3
    i32.eq
    if
      i32.const 65164
      return
    end
    i32.const 0)

  (func $er_ui_icon_path_len  (param $icon i32) (result i32)
    local.get $icon
    i32.const 1
    i32.eq
    if
      i32.const 14
      return
    end
    local.get $icon
    i32.const 2
    i32.eq
    if
      i32.const 12
      return
    end
    local.get $icon
    i32.const 3
    i32.eq
    if
      i32.const 20
      return
    end
    i32.const 0)

  (func $er_ui_icon_ir_byte_offset  (param $icon i32) (result i32)
    local.get $icon
    call $icon_pack_entry_valid
    i32.eqz
    if
      i32.const -1
      return
    end
    local.get $icon
    call $icon_pack_entry_offset
    i32.load)

  (func $er_ui_icon_ir_byte_len  (param $icon i32) (result i32)
    local.get $icon
    call $icon_pack_entry_valid
    i32.eqz
    if
      i32.const -1
      return
    end
    local.get $icon
    call $icon_pack_entry_offset
    i32.const 4
    i32.add
    i32.load)

  (func $er_ui_icon_ir_ptr  (param $icon i32) (result i32)
    local.get $icon
    call $icon_pack_entry_valid
    i32.eqz
    if
      i32.const 0
      return
    end
    global.get $icon_ir_ptr
    local.get $icon
    call $icon_pack_entry_offset
    i32.load
    i32.add)

  (func $er_ui_icon_ir_float_count  (param $icon i32) (result i32)
    local.get $icon
    call $icon_pack_entry_valid
    i32.eqz
    if
      i32.const -1
      return
    end
    local.get $icon
    call $icon_pack_entry_offset
    i32.const 4
    i32.add
    i32.load
    i32.const 2
    i32.shr_u)

  (func $er_ui_icon_ir_copy  (param $icon i32) (param $out i32) (param $cap i32) (result i32)
    (local $src i32)
    (local $len i32)
    local.get $icon
    call $icon_pack_entry_valid
    i32.eqz
    local.get $out
    i32.eqz
    i32.or
    if
      i32.const -1
      return
    end
    local.get $icon
    call $er_ui_icon_ir_byte_len
    local.set $len
    local.get $cap
    local.get $len
    i32.lt_u
    if
      i32.const -1
      return
    end
    local.get $icon
    call $er_ui_icon_ir_ptr
    local.set $src
    local.get $out
    local.get $src
    local.get $len
    memory.copy
    local.get $len
    i32.const 2
    i32.shr_u)

  (func $er_ui_icon_name_ptr  (param $icon i32) (result i32)
    (local $pos i32)
    (local $ptr i32)
    (local $end i32)
    call $icon_pack_loaded
    i32.eqz
    global.get $icon_names_ptr
    i32.eqz
    i32.or
    if
      i32.const 0
      return
    end
    local.get $icon
    i32.const 1
    i32.lt_u
    local.get $icon
    call $icon_pack_count
    i32.gt_u
    i32.or
    if
      i32.const 0
      return
    end
    i32.const 1
    local.set $pos
    global.get $icon_names_ptr
    local.set $ptr
    global.get $icon_names_ptr
    global.get $icon_names_len
    i32.add
    local.set $end
    block $done
      loop $loop
        local.get $ptr
        local.get $end
        i32.ge_u
        br_if $done
        local.get $pos
        local.get $icon
        i32.eq
        if
          local.get $ptr
          return
        end
        block $name_done
          loop $name_loop
            local.get $ptr
            local.get $end
            i32.ge_u
            br_if $done
            local.get $ptr
            i32.load8_u
            i32.eqz
            br_if $name_done
            local.get $ptr
            i32.const 1
            i32.add
            local.set $ptr
            br $name_loop
          end
        end
        local.get $ptr
        i32.const 1
        i32.add
        local.set $ptr
        local.get $pos
        i32.const 1
        i32.add
        local.set $pos
        br $loop
      end
    end
    i32.const 0)

  (func $er_ui_icon_name_len  (param $icon i32) (result i32)
    (local $ptr i32)
    (local $end i32)
    (local $start i32)
    local.get $icon
    call $er_ui_icon_name_ptr
    local.tee $ptr
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $ptr
    local.set $start
    global.get $icon_names_ptr
    global.get $icon_names_len
    i32.add
    local.set $end
    block $done
      loop $loop
        local.get $ptr
        local.get $end
        i32.ge_u
        br_if $done
        local.get $ptr
        i32.load8_u
        i32.eqz
        br_if $done
        local.get $ptr
        i32.const 1
        i32.add
        local.set $ptr
        br $loop
      end
    end
    local.get $ptr
    local.get $start
    i32.sub)
  (func $er_ui_icon_op_polyline  (result f32) f32.const 1)
  (func $er_ui_icon_op_circle  (result f32) f32.const 2)
  (func $er_ui_icon_op_ellipse  (result f32) f32.const 3)
  (func $er_ui_icon_op_round_rect  (result f32) f32.const 4)
  (func $er_ui_icon_op_filled_circle  (result f32) f32.const 5)
  (func $er_ui_icon_op_move_to  (result f32) f32.const 6)
  (func $er_ui_icon_op_line_to  (result f32) f32.const 7)
  (func $er_ui_icon_op_quad_to  (result f32) f32.const 8)
  (func $er_ui_icon_op_cubic_to  (result f32) f32.const 9)
  (func $er_ui_icon_op_arc_to  (result f32) f32.const 10)
  (func $er_ui_icon_op_close_path  (result f32) f32.const 11)
  (func $er_ui_icon_op_filled_ellipse  (result f32) f32.const 12)
  (func $er_ui_icon_op_filled_round_rect  (result f32) f32.const 13)
  (func $er_ui_icon_op_begin_fill_path  (result f32) f32.const 14)
  (func $er_ui_icon_op_end_fill_path  (result f32) f32.const 15)
  (func $er_ui_icon_op_begin_evenodd_fill_path  (result f32) f32.const 16)
  (func $er_ui_icon_op_paint_rgba  (result f32) f32.const 17)
  (func $er_ui_icon_op_paint_current_color  (result f32) f32.const 18)
  (func $er_ui_icon_op_paint_linear_gradient  (result f32) f32.const 19)
  (func $er_ui_icon_op_paint_radial_gradient  (result f32) f32.const 20)
  (func $er_ui_icon_op_paint_current_color_alpha  (result f32) f32.const 21)
  (func $er_ui_icon_op_stroke_width  (result f32) f32.const 22)
  (func $er_ui_icon_op_stroke_cap  (result f32) f32.const 23)
  (func $er_ui_icon_op_stroke_join  (result f32) f32.const 24)
  (func $er_ui_icon_op_stroke_miter_limit  (result f32) f32.const 25)
  (func $er_ui_icon_op_begin_clip_path  (result f32) f32.const 26)
  (func $er_ui_icon_op_end_clip_path  (result f32) f32.const 27)
  (func $er_ui_icon_op_clear_clip_path  (result f32) f32.const 28)
  (func $er_ui_icon_op_begin_evenodd_clip_path  (result f32) f32.const 29)
  (func $er_ui_icon_op_stroke_dash  (result f32) f32.const 30)
  (func $er_ui_icon_op_paint_matrix  (result f32) f32.const 31)

  (func $er_ui_icon_stroke_cap_butt  (result f32) f32.const 0)
  (func $er_ui_icon_stroke_cap_round  (result f32) f32.const 1)
  (func $er_ui_icon_stroke_cap_square  (result f32) f32.const 2)
  (func $er_ui_icon_stroke_join_miter  (result f32) f32.const 0)
  (func $er_ui_icon_stroke_join_round  (result f32) f32.const 1)
  (func $er_ui_icon_stroke_join_bevel  (result f32) f32.const 2)
  (func $er_ui_icon_default_stroke_width  (result f32) f32.const 0.083333336)
  (func $er_ui_icon_default_miter_limit  (result f32) f32.const 4)

  (func $er_ui_icon_stroke_width  (param $icon i32) (result f32)
    local.get $icon
    call $er_ui_icon_valid
    if (result f32)
      f32.const 2
    else
      f32.const 0
    end)

  (func $er_ui_icon_bounds  (param $icon i32) (param $out i32) (result i32)
    local.get $icon
    call $er_ui_icon_valid
    i32.eqz
    local.get $out
    i32.eqz
    i32.or
    if
      i32.const 0
      return
    end
    local.get $out
    f32.const 0
    f32.store
    local.get $out
    i32.const 4
    i32.add
    f32.const 0
    f32.store
    local.get $out
    i32.const 8
    i32.add
    f32.const 24
    f32.store
    local.get $out
    i32.const 12
    i32.add
    f32.const 24
    f32.store
    i32.const 1)

  (func $er_ui_icon_stroke_width_scaled  (param $icon i32) (param $w f32) (param $h f32) (param $dpr f32) (result f32)
    (local $stroke f32)
    local.get $icon
    call $er_ui_icon_valid
    local.get $w
    local.get $h
    call $valid_rect
    i32.and
    if (result f32)
      local.get $icon
      call $er_ui_icon_stroke_width
      local.get $w
      local.get $h
      call $min_f32
      f32.mul
      f32.const 24
      f32.div
      local.tee $stroke
      local.get $dpr
      call $snap_pixel
      local.set $stroke
      local.get $dpr
      f32.const 0
      f32.gt
      if (result f32)
        local.get $stroke
        f32.const 1
        local.get $dpr
        f32.div
        call $max_f32
      else
        local.get $stroke
      end
    else
      f32.const 0
    end)
