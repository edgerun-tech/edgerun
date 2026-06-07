  (memory 64)
  (data (i32.const 65000) "edgerun:ui-hit:v1")
  (data (i32.const 65100) "M5 12l4 4L19 6")
  (data (i32.const 65132) "M6 9l6 6 6-6")
  (data (i32.const 65164) "M6 6l12 12M18 6L6 18")

  (global $writer_base (mut i32) (i32.const 0))
  (global $writer_cap (mut i32) (i32.const 0))
  (global $writer_node_count (mut i32) (i32.const 0))
  (global $writer_cursor (mut i32) (i32.const 0))
  (global $runtime_hover (mut i32) (i32.const 0))
  (global $runtime_focus (mut i32) (i32.const 0))
  (global $runtime_active (mut i32) (i32.const 0))
  (global $runtime_key (mut i32) (i32.const 0))
  (global $runtime_pointer_x (mut f32) (f32.const 0))
  (global $runtime_pointer_y (mut f32) (f32.const 0))
  (global $runtime_overlay (mut i32) (i32.const 0))
  (global $font_body_ptr (mut i32) (i32.const 0))
  (global $font_body_len (mut i32) (i32.const 0))
  (global $font_weight (mut i32) (i32.const 0))
  (global $font_regular_ptr (mut i32) (i32.const 0))
  (global $font_regular_len (mut i32) (i32.const 0))
  (global $font_semibold_ptr (mut i32) (i32.const 0))
  (global $font_semibold_len (mut i32) (i32.const 0))
  (global $font_bold_ptr (mut i32) (i32.const 0))
  (global $font_bold_len (mut i32) (i32.const 0))
  (global $icon_index_ptr (mut i32) (i32.const 0))
  (global $icon_index_len (mut i32) (i32.const 0))
  (global $icon_ir_ptr (mut i32) (i32.const 0))
  (global $icon_ir_len (mut i32) (i32.const 0))
  (global $icon_names_ptr (mut i32) (i32.const 0))
  (global $icon_names_len (mut i32) (i32.const 0))
  (global $icon_alpha_paint_scale (mut f32) (f32.const 1))
  (global $icon_paint_kind (mut i32) (i32.const 0))
  (global $icon_rgba_out (mut i32) (i32.const 0))
  (global $icon_paint_r (mut f32) (f32.const 0))
  (global $icon_paint_g (mut f32) (f32.const 0))
  (global $icon_paint_b (mut f32) (f32.const 0))
  (global $icon_current_r (mut f32) (f32.const 0))
  (global $icon_current_g (mut f32) (f32.const 0))
  (global $icon_current_b (mut f32) (f32.const 0))
  (global $icon_current_a (mut f32) (f32.const 255))
  (global $icon_paint_matrix_a (mut f32) (f32.const 1))
  (global $icon_paint_matrix_b (mut f32) (f32.const 0))
  (global $icon_paint_matrix_c (mut f32) (f32.const 0))
  (global $icon_paint_matrix_d (mut f32) (f32.const 1))
  (global $icon_paint_matrix_tx (mut f32) (f32.const 0))
  (global $icon_paint_matrix_ty (mut f32) (f32.const 0))
  (global $icon_linear_x1 (mut f32) (f32.const 0))
  (global $icon_linear_y1 (mut f32) (f32.const 0))
  (global $icon_linear_x2 (mut f32) (f32.const 1))
  (global $icon_linear_y2 (mut f32) (f32.const 0))
  (global $icon_linear_a0 (mut f32) (f32.const 1))
  (global $icon_linear_a1 (mut f32) (f32.const 1))
  (global $icon_gradient_spread (mut i32) (i32.const 0))
  (global $icon_radial_cx (mut f32) (f32.const 0.5))
  (global $icon_radial_cy (mut f32) (f32.const 0.5))
  (global $icon_radial_r (mut f32) (f32.const 0.5))
  (global $icon_radial_fx (mut f32) (f32.const 0.5))
  (global $icon_radial_fy (mut f32) (f32.const 0.5))
  (global $icon_radial_fr (mut f32) (f32.const 0))
  (global $icon_radial_a0 (mut f32) (f32.const 1))
  (global $icon_radial_a1 (mut f32) (f32.const 1))
  (global $icon_gradient_stops_ptr (mut i32) (i32.const 0))
  (global $icon_gradient_stop_count (mut i32) (i32.const 0))
  (global $icon_clip_enabled (mut i32) (i32.const 0))
  (global $icon_clip_points (mut i32) (i32.const 0))
  (global $icon_clip_count (mut i32) (i32.const 0))
  (global $icon_clip_rule (mut i32) (i32.const 0))
  (global $icon_clip2_enabled (mut i32) (i32.const 0))
  (global $icon_clip2_points (mut i32) (i32.const 0))
  (global $icon_clip2_count (mut i32) (i32.const 0))
  (global $icon_clip2_rule (mut i32) (i32.const 0))
  (global $icon_clip3_enabled (mut i32) (i32.const 0))
  (global $icon_clip3_points (mut i32) (i32.const 0))
  (global $icon_clip3_count (mut i32) (i32.const 0))
  (global $icon_clip3_rule (mut i32) (i32.const 0))
  (global $icon_clip4_enabled (mut i32) (i32.const 0))
  (global $icon_clip4_points (mut i32) (i32.const 0))
  (global $icon_clip4_count (mut i32) (i32.const 0))
  (global $icon_clip4_rule (mut i32) (i32.const 0))
  (global $icon_clip_bx (mut f32) (f32.const 0))
  (global $icon_clip_by (mut f32) (f32.const 0))
  (global $icon_clip_bw (mut f32) (f32.const 1))
  (global $icon_clip_bh (mut f32) (f32.const 1))

  (func $min_f32 (param $a f32) (param $b f32) (result f32)
    local.get $a
    local.get $b
    f32.lt
    if (result f32)
      local.get $a
    else
      local.get $b
    end)

  (func $max_f32 (param $a f32) (param $b f32) (result f32)
    local.get $a
    local.get $b
    f32.gt
    if (result f32)
      local.get $a
    else
      local.get $b
    end)

  (func $clamp_f32 (param $v f32) (param $lo f32) (param $hi f32) (result f32)
    local.get $v
    local.get $lo
    call $max_f32
    local.get $hi
    call $min_f32)

  (func $clamp_i32 (param $v i32) (param $lo i32) (param $hi i32) (result i32)
    local.get $v
    local.get $lo
    i32.lt_s
    if (result i32)
      local.get $lo
    else
      local.get $v
      local.get $hi
      i32.gt_s
      if (result i32)
        local.get $hi
      else
        local.get $v
      end
    end)

  (func $sin_rad (param $x f32) (result f32)
    (local $x2 f32)
    block $hi_done
      loop $hi_loop
        local.get $x
        f32.const 3.1415927
        f32.gt
        i32.eqz
        br_if $hi_done
        local.get $x
        f32.const 6.2831855
        f32.sub
        local.set $x
        br $hi_loop
      end
    end
    block $lo_done
      loop $lo_loop
        local.get $x
        f32.const -3.1415927
        f32.lt
        i32.eqz
        br_if $lo_done
        local.get $x
        f32.const 6.2831855
        f32.add
        local.set $x
        br $lo_loop
      end
    end
    local.get $x local.get $x f32.mul local.set $x2
    local.get $x
    local.get $x local.get $x2 f32.mul f32.const 0.16666667 f32.mul f32.sub
    local.get $x local.get $x2 f32.mul local.get $x2 f32.mul f32.const 0.008333334 f32.mul f32.add
    local.get $x local.get $x2 f32.mul local.get $x2 f32.mul local.get $x2 f32.mul f32.const 0.0001984127 f32.mul f32.sub)

  (func $cos_rad (param $x f32) (result f32)
    local.get $x
    f32.const 1.5707964
    f32.add
    call $sin_rad)

  (func $snap_pixel (param $v f32) (param $dpr f32) (result f32)
    local.get $dpr
    f32.const 0
    f32.le
    if (result f32)
      local.get $v
    else
      local.get $v
      local.get $dpr
      f32.mul
      f32.nearest
      local.get $dpr
      f32.div
    end)

  (func (export "er_ui_snap_pixel") (param $v f32) (param $dpr f32) (result f32)
    local.get $v
    local.get $dpr
    call $snap_pixel)

  (func (export "er_ui_snap_px") (param $v f32) (param $dpr f32) (result f32)
    local.get $v
    local.get $dpr
    call $snap_pixel)

  (func (export "er_ui_pixel_snap") (param $v f32) (param $dpr f32) (result f32)
    local.get $v
    local.get $dpr
    call $snap_pixel)

  (func (export "er_ui_snap_stroke_center") (param $v f32) (param $dpr f32) (result f32)
    local.get $dpr
    f32.const 0
    f32.le
    if (result f32)
      local.get $v
    else
      local.get $v
      local.get $dpr
      f32.mul
      f32.floor
      f32.const 0.5
      f32.add
      local.get $dpr
      f32.div
    end)

  (func $min_i32_u (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.lt_u
    if (result i32)
      local.get $a
    else
      local.get $b
    end)

  (func $store16 (param $p i32) (param $v i32)
    local.get $p
    local.get $v
    i32.store16)

  (func $store32 (param $p i32) (param $v i32)
    local.get $p
    local.get $v
    i32.store)

  (func $load16 (param $p i32) (result i32)
    local.get $p
    i32.load16_u)

  (func $copy (param $dst i32) (param $src i32) (param $len i32)
    local.get $dst
    local.get $src
    local.get $len
    memory.copy)

  (func $zero (param $dst i32) (param $len i32)
    local.get $dst
    i32.const 0
    local.get $len
    memory.fill)

  (func $store_runtime_state (param $out i32)
    local.get $out
    global.get $runtime_hover
    i32.store
    local.get $out
    i32.const 4
    i32.add
    global.get $runtime_focus
    i32.store
    local.get $out
    i32.const 8
    i32.add
    global.get $runtime_active
    i32.store
    local.get $out
    i32.const 12
    i32.add
    global.get $runtime_key
    i32.store
    local.get $out
    i32.const 16
    i32.add
    global.get $runtime_pointer_x
    f32.store
    local.get $out
    i32.const 20
    i32.add
    global.get $runtime_pointer_y
    f32.store
    local.get $out
    i32.const 24
    i32.add
    global.get $runtime_overlay
    i32.store
    local.get $out
    i32.const 28
    i32.add
    i32.const 0
    i32.store)

  (func $fnv1a_byte (param $hash i32) (param $byte i32) (result i32)
    local.get $hash
    local.get $byte
    i32.xor
    i32.const 16777619
    i32.mul)

  (func $fnv1a_range (param $hash i32) (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    block $done
      loop $loop
        local.get $i
        local.get $len
        i32.ge_u
        br_if $done
        local.get $hash
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        call $fnv1a_byte
        local.set $hash
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $loop
      end
    end
    local.get $hash)

  (func $record_ptr (param $index i32) (result i32)
    global.get $writer_base
    i32.const 20
    i32.add
    local.get $index
    i32.const 16
    i32.mul
    i32.add)

  (func $table_start (param $base i32) (param $node_count i32) (result i32)
    local.get $base
    i32.const 20
    i32.add
    local.get $node_count
    i32.const 16
    i32.mul
    i32.add)

  (func $string_ptr (param $base i32) (param $node_count i32) (param $ref i32) (result i32)
    local.get $base
    local.get $node_count
    call $table_start
    local.get $ref
    i32.const 65535
    i32.and
    i32.add)

  (func $string_len (param $ref i32) (result i32)
    local.get $ref
    i32.const 16
    i32.shr_u
    i32.const 65535
    i32.and)

  (func $string_ref (param $offset i32) (param $len i32) (result i32)
    local.get $len
    i32.const 16
    i32.shl
    local.get $offset
    i32.const 65535
    i32.and
    i32.or)

  (func $second_ref_is_packed (param $kind i32) (result i32)
    local.get $kind
    i32.const 7
    i32.eq
    local.get $kind
    i32.const 12
    i32.eq
    i32.or
    local.get $kind
    i32.const 13
    i32.eq
    i32.or
    local.get $kind
    i32.const 16
    i32.eq
    i32.or
    local.get $kind
    i32.const 20
    i32.eq
    i32.or
    local.get $kind
    i32.const 22
    i32.eq
    i32.or
    local.get $kind
    i32.const 23
    i32.eq
    i32.or
    local.get $kind
    i32.const 24
    i32.eq
    i32.or
    local.get $kind
    i32.const 27
    i32.eq
    i32.or
    local.get $kind
    i32.const 37
    i32.eq
    i32.or
    local.get $kind
    i32.const 44
    i32.eq
    i32.or
    local.get $kind
    i32.const 55
    i32.eq
    i32.or
    local.get $kind
    i32.const 56
    i32.eq
    i32.or
    local.get $kind
    i32.const 57
    i32.eq
    i32.or)

  (func $packed_second_ref_valid (param $kind i32) (param $ref i32) (result i32)
    local.get $kind
    i32.const 22
    i32.eq
    local.get $kind
    i32.const 23
    i32.eq
    i32.or
    local.get $kind
    i32.const 56
    i32.eq
    i32.or
    if (result i32)
      local.get $ref
      i32.const 16
      i32.shr_u
      i32.eqz
      local.get $ref
      i32.const 65535
      i32.and
      i32.const 1
      i32.le_u
      i32.and
    else
        local.get $kind
        i32.const 7
        i32.eq
        local.get $kind
        i32.const 20
        i32.eq
        i32.or
        local.get $kind
        i32.const 24
        i32.eq
        i32.or
        local.get $kind
        i32.const 37
        i32.eq
      i32.or
      local.get $kind
      i32.const 16
      i32.eq
      i32.or
      local.get $kind
      i32.const 27
      i32.eq
      i32.or
      local.get $kind
      i32.const 44
      i32.eq
      i32.or
      local.get $kind
      i32.const 57
      i32.eq
      i32.or
      if (result i32)
        local.get $ref
        i32.const 16
        i32.shr_u
        i32.eqz
      else
        local.get $kind
        i32.const 12
        i32.eq
        if (result i32)
          local.get $ref
          i32.const 16
          i32.shr_u
          i32.const 511
          i32.le_u
        else
          local.get $kind
          i32.const 13
          i32.eq
          if (result i32)
            local.get $ref
            i32.const 65535
            i32.and
            i32.const 255
            i32.le_u
          else
            local.get $kind
            i32.const 55
            i32.eq
            if (result i32)
              local.get $ref
              i32.const 65535
              i32.and
              i32.eqz
            else
              i32.const 1
            end
          end
        end
      end
    end)

  (func $valid_rect (param $w f32) (param $h f32) (result i32)
    local.get $w
    f32.const 0
    f32.gt
    local.get $h
    f32.const 0
    f32.gt
    i32.and)

  (func $preferred_w (param $kind i32) (param $first_len i32) (result f32)
    local.get $kind
    i32.const 1
    i32.eq
    if (result f32)
      local.get $first_len
      f32.convert_i32_u
      f32.const 8
      f32.mul
    else
      local.get $kind
      i32.const 0
      i32.eq
      if (result f32)
        f32.const 1
      else
        local.get $kind
        i32.const 44
        i32.eq
        local.get $kind
        i32.const 27
        i32.eq
        i32.or
        local.get $kind
        i32.const 56
        i32.eq
        i32.or
        if (result f32)
          f32.const 64
        else
          local.get $kind
          i32.const 12
          i32.eq
          local.get $kind
          i32.const 13
          i32.eq
          i32.or
          local.get $kind
          i32.const 22
          i32.eq
          i32.or
          local.get $kind
          i32.const 23
          i32.eq
          i32.or
          local.get $kind
          i32.const 36
          i32.eq
          i32.or
          local.get $kind
          i32.const 39
          i32.eq
          i32.or
          local.get $kind
          i32.const 42
          i32.eq
          i32.or
          local.get $kind
          i32.const 57
          i32.eq
          i32.or
          if (result f32)
            f32.const 120
          else
            local.get $kind
            i32.const 16
            i32.eq
            local.get $kind
            i32.const 18
            i32.eq
            i32.or
            local.get $kind
            i32.const 19
            i32.eq
            i32.or
            local.get $kind
            i32.const 24
            i32.eq
            i32.or
            local.get $kind
            i32.const 25
            i32.eq
            i32.or
            local.get $kind
            i32.const 37
            i32.eq
            i32.or
            local.get $kind
            i32.const 57
            i32.eq
            i32.or
            if (result f32)
              f32.const 180
            else
              local.get $kind
              i32.const 6
              i32.eq
              if (result f32)
                f32.const 160
              else
                f32.const 220
              end
            end
          end
        end
      end
    end)

  (func $preferred_h (param $kind i32) (result f32)
    local.get $kind
    i32.const 0
    i32.eq
    if (result f32)
      f32.const 1
    else
      local.get $kind
      i32.const 1
      i32.eq
      if (result f32)
        f32.const 18
      else
        local.get $kind
        i32.const 33
        i32.eq
        if (result f32)
          f32.const 1
        else
          local.get $kind
          i32.const 6
          i32.eq
          if (result f32)
            f32.const 90
          else
            local.get $kind
            i32.const 27
            i32.eq
            local.get $kind
            i32.const 56
            i32.eq
            i32.or
            if (result f32)
              f32.const 24
            else
              local.get $kind
              i32.const 3
              i32.ge_u
              local.get $kind
              i32.const 58
              i32.lt_u
              i32.and
              if (result f32)
                f32.const 36
              else
                f32.const 32
              end
            end
          end
        end
      end
    end)

  (func $emit_rect (param $out i32) (param $cap i32) (param $count i32) (param $x f32) (param $y f32) (param $w f32) (param $h f32) (param $color i32) (result i32)
    (local $p i32)
    local.get $count
    i32.const 48
    i32.mul
    i32.const 48
    i32.add
    local.get $cap
    i32.gt_u
    if (result i32)
      i32.const -1
    else
      local.get $out
      local.get $count
      i32.const 48
      i32.mul
      i32.add
      local.set $p
      local.get $p
      i32.const 48
      call $zero
      local.get $p
      i32.const 1
      call $store32
      local.get $p
      i32.const 4
      i32.add
      local.get $color
      call $store32
      local.get $p
      i32.const 8
      i32.add
      local.get $x
      f32.store
      local.get $p
      i32.const 12
      i32.add
      local.get $y
      f32.store
      local.get $p
      i32.const 16
      i32.add
      local.get $w
      f32.store
      local.get $p
      i32.const 20
      i32.add
      local.get $h
      f32.store
      local.get $count
      i32.const 1
      i32.add
    end)

  (func $emit_text (param $out i32) (param $cap i32) (param $count i32) (param $x f32) (param $y f32) (param $w f32) (param $h f32) (param $ptr i32) (param $len i32) (param $color i32) (result i32)
    (local $p i32)
    local.get $len
    i32.eqz
    if (result i32)
      local.get $count
    else
      local.get $count
      i32.const 48
      i32.mul
      i32.const 48
      i32.add
      local.get $cap
      i32.gt_u
      if (result i32)
        i32.const -1
      else
        local.get $out
        local.get $count
        i32.const 48
        i32.mul
        i32.add
        local.set $p
        local.get $p
        i32.const 48
        call $zero
        local.get $p
        i32.const 2
        call $store32
        local.get $p
        i32.const 4
        i32.add
        local.get $color
        call $store32
        local.get $p
        i32.const 8
        i32.add
        local.get $x
        f32.store
        local.get $p
        i32.const 12
        i32.add
        local.get $y
        f32.store
        local.get $p
        i32.const 16
        i32.add
        local.get $w
        f32.store
        local.get $p
        i32.const 20
        i32.add
        local.get $h
        f32.store
        local.get $p
        i32.const 24
        i32.add
        local.get $ptr
        call $store32
        local.get $p
        i32.const 28
        i32.add
        local.get $len
        call $store32
        local.get $count
        i32.const 1
        i32.add
      end
    end)

  (func $emit_icon_or_rect (param $out i32) (param $cap i32) (param $count i32) (param $x f32) (param $y f32) (param $w f32) (param $h f32) (param $icon i32) (param $color i32) (result i32)
    local.get $icon
    call $er_ui_icon_valid
    if (result i32)
      local.get $out
      local.get $cap
      local.get $count
      local.get $x
      local.get $y
      local.get $w
      local.get $h
      local.get $icon
      local.get $color
      call $er_ui_command_write_icon
    else
      local.get $out
      local.get $cap
      local.get $count
      local.get $x
      local.get $y
      local.get $w
      local.get $h
      local.get $color
      call $emit_rect
    end)

  (func $command_set_owner_at (param $commands i32) (param $index i32) (param $id i32) (param $kind i32) (param $role i32)
    (local $p i32)
    local.get $commands
    local.get $index
    i32.const 48
    i32.mul
    i32.add
    local.set $p
    local.get $p
    i32.const 32
    i32.add
    local.get $id
    call $store32
    local.get $p
    i32.const 36
    i32.add
    local.get $kind
    call $store32
    local.get $p
    i32.const 40
    i32.add
    local.get $role
    call $store32)

  (func $command_set_meta_at (param $commands i32) (param $index i32) (param $meta i32)
    local.get $commands
    local.get $index
    i32.const 48
    i32.mul
    i32.add
    i32.const 44
    i32.add
    local.get $meta
    call $store32)

  (func $record_encoded_state (param $kind i32) (param $id i32) (result i32)
    local.get $kind
    i32.const 3
    i32.eq
    local.get $kind
    i32.const 14
    i32.eq
    i32.or
    local.get $kind
    i32.const 25
    i32.eq
    i32.or
    local.get $kind
    i32.const 42
    i32.eq
    i32.or
    local.get $kind
    i32.const 43
    i32.eq
    i32.or
    if
      local.get $id
      i32.const 2
      i32.rem_u
      return
    end
    local.get $kind
    i32.const 15
    i32.eq
    local.get $kind
    i32.const 39
    i32.eq
    i32.or
    local.get $kind
    i32.const 40
    i32.eq
    i32.or
    local.get $kind
    i32.const 41
    i32.eq
    i32.or
    if
      local.get $id
      i32.const 3
      i32.rem_u
      return
    end
    i32.const -1)

  (func $record_encoded_mul (param $kind i32) (result i32)
    local.get $kind
    i32.const 3
    i32.eq
    local.get $kind
    i32.const 14
    i32.eq
    i32.or
    local.get $kind
    i32.const 25
    i32.eq
    i32.or
    local.get $kind
    i32.const 42
    i32.eq
    i32.or
    local.get $kind
    i32.const 43
    i32.eq
    i32.or
    if
      i32.const 2
      return
    end
    local.get $kind
    i32.const 15
    i32.eq
    local.get $kind
    i32.const 39
    i32.eq
    i32.or
    local.get $kind
    i32.const 40
    i32.eq
    i32.or
    local.get $kind
    i32.const 41
    i32.eq
    i32.or
    if
      i32.const 3
      return
    end
    i32.const 1)
