  (func $er_ui_icon_render_alpha (export "er_ui_icon_render_alpha") (param $icon i32) (param $alpha i32) (param $width i32) (param $height i32) (param $x f32) (param $y f32) (param $w f32) (param $h f32) (result i32)
    (local $ptr i32) (local $end i32) (local $op f32) (local $stroke f32) (local $stroke_cap f32) (local $stroke_join f32) (local $stroke_miter_limit f32)
    (local $dash_on f32) (local $dash_off f32) (local $dash_base_offset f32) (local $dash_offset f32) (local $dash_dx f32) (local $dash_dy f32)
    (local $cx f32) (local $cy f32) (local $sx f32) (local $sy f32) (local $nx f32) (local $ny f32) (local $px0 f32) (local $py0 f32) (local $has_current i32) (local $has_segment i32)
    (local $poly_count i32) (local $poly_i i32) (local $poly_prev_x f32) (local $poly_prev_y f32)
    (local $grad_count i32) (local $grad_i i32) (local $grad_alpha f32)
    (local $fill_active i32) (local $fill_points i32) (local $fill_count i32) (local $fill_rule i32) (local $clip_capture i32)
    local.get $alpha
    i32.eqz
    local.get $width
    i32.eqz
    i32.or
    local.get $height
    i32.eqz
    i32.or
    local.get $w
    f32.const 0
    f32.le
    i32.or
    local.get $h
    f32.const 0
    f32.le
    i32.or
    local.get $icon
    call $icon_pack_entry_valid
    i32.eqz
    i32.or
    if
      i32.const -1
      return
    end
    local.get $alpha
    i32.const 0
    local.get $width
    local.get $height
    i32.mul
    memory.fill
    local.get $icon
    call $er_ui_icon_ir_ptr
    local.set $ptr
    local.get $ptr
    local.get $icon
    call $er_ui_icon_ir_byte_len
    i32.add
    local.set $end
    f32.const 0.083333336
    local.set $stroke
    f32.const 1
    local.set $stroke_cap
    f32.const 0
    local.set $stroke_join
    f32.const 4
    local.set $stroke_miter_limit
    f32.const 0
    local.set $dash_on
    f32.const 0
    local.set $dash_off
    f32.const 0
    local.set $dash_base_offset
    f32.const 0
    local.set $dash_offset
    f32.const 1
    global.set $icon_alpha_paint_scale
    f32.const 0
    global.set $icon_paint_r
    f32.const 0
    global.set $icon_paint_g
    f32.const 0
    global.set $icon_paint_b
    global.get $icon_rgba_out
    i32.eqz
    if
      f32.const 0
      global.set $icon_current_r
      f32.const 0
      global.set $icon_current_g
      f32.const 0
      global.set $icon_current_b
      f32.const 255
      global.set $icon_current_a
    end
    i32.const 0
    global.set $icon_paint_kind
    f32.const 1
    global.set $icon_paint_matrix_a
    f32.const 0
    global.set $icon_paint_matrix_b
    f32.const 0
    global.set $icon_paint_matrix_c
    f32.const 1
    global.set $icon_paint_matrix_d
    f32.const 0
    global.set $icon_paint_matrix_tx
    f32.const 0
    global.set $icon_paint_matrix_ty
    i32.const 0
    global.set $icon_gradient_stops_ptr
    i32.const 0
    global.set $icon_gradient_stop_count
    i32.const 0
    global.set $icon_gradient_spread
    i32.const 0
    global.set $icon_clip_enabled
    i32.const 0
    global.set $icon_clip2_enabled
    i32.const 0
    global.set $icon_clip3_enabled
    i32.const 0
    global.set $icon_clip4_enabled
    local.get $x
    global.set $icon_clip_bx
    local.get $y
    global.set $icon_clip_by
    local.get $w
    global.set $icon_clip_bw
    local.get $h
    global.set $icon_clip_bh
    i32.const 3900000
    local.set $fill_points
    block $done
      loop $loop
        local.get $ptr local.get $end i32.ge_u br_if $done
        local.get $ptr f32.load local.set $op
        local.get $op f32.const 22 f32.eq
        if
          local.get $ptr i32.const 4 i32.add f32.load local.set $stroke
          local.get $ptr i32.const 8 i32.add local.set $ptr
          br $loop
        end
        local.get $op f32.const 30 f32.eq
        if
          local.get $ptr i32.const 4 i32.add f32.load local.set $dash_on
          local.get $ptr i32.const 8 i32.add f32.load local.set $dash_off
          local.get $ptr i32.const 12 i32.add f32.load local.tee $dash_base_offset local.set $dash_offset
          local.get $ptr i32.const 16 i32.add local.set $ptr
          br $loop
        end
        local.get $op f32.const 6 f32.eq
        if
          local.get $has_segment
          if
            local.get $stroke_cap f32.const 1 f32.eq
            if
              local.get $alpha local.get $width local.get $height local.get $x local.get $y local.get $w local.get $h local.get $sx local.get $sy local.get $stroke call $icon_render_round_cap_alpha
              local.get $alpha local.get $width local.get $height local.get $x local.get $y local.get $w local.get $h local.get $cx local.get $cy local.get $stroke call $icon_render_round_cap_alpha
            end
          end
          local.get $ptr i32.const 4 i32.add f32.load local.set $cx
          local.get $ptr i32.const 8 i32.add f32.load local.set $cy
          local.get $cx local.set $sx
          local.get $cy local.set $sy
          local.get $cx local.set $px0
          local.get $cy local.set $py0
          local.get $dash_base_offset local.set $dash_offset
          i32.const 1 local.set $has_current
          i32.const 0 local.set $has_segment
          local.get $fill_active
          if
            local.get $fill_count
            if
              local.get $fill_points local.get $fill_count call $icon_fill_break_append local.set $fill_count
            end
            local.get $fill_points local.get $fill_count local.get $cx local.get $cy call $icon_fill_point_append local.set $fill_count
          end
          local.get $ptr i32.const 12 i32.add local.set $ptr
          br $loop
        end
        local.get $op f32.const 7 f32.eq
        if
          local.get $ptr i32.const 4 i32.add f32.load local.set $nx
          local.get $ptr i32.const 8 i32.add f32.load local.set $ny
          local.get $fill_active
          if
            local.get $nx local.set $cx
            local.get $ny local.set $cy
            local.get $fill_points local.get $fill_count local.get $cx local.get $cy call $icon_fill_point_append local.set $fill_count
            local.get $ptr i32.const 12 i32.add local.set $ptr
            br $loop
          end
          local.get $has_current
          if
            local.get $has_segment
            local.get $stroke_join f32.const 1 f32.eq
            i32.and
            if
              local.get $alpha local.get $width local.get $height local.get $x local.get $y local.get $w local.get $h local.get $cx local.get $cy local.get $stroke call $icon_render_round_cap_alpha
            end
            local.get $has_segment
            local.get $stroke_join f32.const 0 f32.eq
            i32.and
            if
              local.get $alpha local.get $width local.get $height local.get $x local.get $y local.get $w local.get $h local.get $px0 local.get $py0 local.get $cx local.get $cy local.get $nx local.get $ny local.get $stroke local.get $stroke_miter_limit call $icon_render_miter_join_alpha
            end
            local.get $has_segment
            local.get $stroke_join f32.const 2 f32.eq
            i32.and
            if
              local.get $alpha local.get $width local.get $height local.get $x local.get $y local.get $w local.get $h local.get $px0 local.get $py0 local.get $cx local.get $cy local.get $nx local.get $ny local.get $stroke call $icon_render_bevel_join_alpha
            end
            local.get $dash_on f32.const 0 f32.gt
            local.get $dash_off f32.const 0 f32.gt
            i32.and
            if
              local.get $alpha local.get $width local.get $height local.get $x local.get $y local.get $w local.get $h local.get $cx local.get $cy local.get $nx local.get $ny local.get $stroke local.get $stroke_cap local.get $dash_on local.get $dash_off local.get $dash_offset call $icon_render_dashed_line_alpha
              local.get $nx local.get $cx f32.sub local.set $dash_dx
              local.get $ny local.get $cy f32.sub local.set $dash_dy
              local.get $dash_offset
              local.get $dash_dx local.get $dash_dx f32.mul
              local.get $dash_dy local.get $dash_dy f32.mul
              f32.add
              f32.sqrt
              f32.add
              local.set $dash_offset
            else
              local.get $alpha local.get $width local.get $height local.get $x local.get $y local.get $w local.get $h local.get $cx local.get $cy local.get $nx local.get $ny local.get $stroke local.get $stroke_cap call $icon_render_line_alpha
            end
          end
          i32.const 1 local.set $has_segment
          local.get $cx local.set $px0
          local.get $cy local.set $py0
          local.get $nx local.set $cx
          local.get $ny local.set $cy
          i32.const 1 local.set $has_current
          local.get $fill_active
          if
            local.get $fill_points local.get $fill_count local.get $cx local.get $cy call $icon_fill_point_append local.set $fill_count
          end
          local.get $ptr i32.const 12 i32.add local.set $ptr
          br $loop
        end
        local.get $op f32.const 11 f32.eq
        if
          local.get $fill_active
          if
            local.get $sx local.set $cx
            local.get $sy local.set $cy
            local.get $fill_points local.get $fill_count local.get $cx local.get $cy call $icon_fill_point_append local.set $fill_count
            local.get $ptr i32.const 4 i32.add local.set $ptr
            br $loop
          end
          local.get $has_current
          if
            local.get $has_segment
            local.get $stroke_join f32.const 1 f32.eq
            i32.and
            if
              local.get $alpha local.get $width local.get $height local.get $x local.get $y local.get $w local.get $h local.get $cx local.get $cy local.get $stroke call $icon_render_round_cap_alpha
            end
            local.get $has_segment
            local.get $stroke_join f32.const 0 f32.eq
            i32.and
            if
              local.get $alpha local.get $width local.get $height local.get $x local.get $y local.get $w local.get $h local.get $px0 local.get $py0 local.get $cx local.get $cy local.get $sx local.get $sy local.get $stroke local.get $stroke_miter_limit call $icon_render_miter_join_alpha
            end
            local.get $has_segment
            local.get $stroke_join f32.const 2 f32.eq
            i32.and
            if
              local.get $alpha local.get $width local.get $height local.get $x local.get $y local.get $w local.get $h local.get $px0 local.get $py0 local.get $cx local.get $cy local.get $sx local.get $sy local.get $stroke call $icon_render_bevel_join_alpha
            end
            local.get $dash_on f32.const 0 f32.gt
            local.get $dash_off f32.const 0 f32.gt
            i32.and
            if
              local.get $alpha local.get $width local.get $height local.get $x local.get $y local.get $w local.get $h local.get $cx local.get $cy local.get $sx local.get $sy local.get $stroke local.get $stroke_cap local.get $dash_on local.get $dash_off local.get $dash_offset call $icon_render_dashed_line_alpha
              local.get $sx local.get $cx f32.sub local.set $dash_dx
              local.get $sy local.get $cy f32.sub local.set $dash_dy
              local.get $dash_offset
              local.get $dash_dx local.get $dash_dx f32.mul
              local.get $dash_dy local.get $dash_dy f32.mul
              f32.add
              f32.sqrt
              f32.add
              local.set $dash_offset
            else
              local.get $alpha local.get $width local.get $height local.get $x local.get $y local.get $w local.get $h local.get $cx local.get $cy local.get $sx local.get $sy local.get $stroke local.get $stroke_cap call $icon_render_line_alpha
            end
          end
          i32.const 0 local.set $has_segment
          local.get $sx local.set $cx
          local.get $sy local.set $cy
          local.get $cx local.set $px0
          local.get $cy local.set $py0
          local.get $fill_active
          if
            local.get $fill_points local.get $fill_count local.get $cx local.get $cy call $icon_fill_point_append local.set $fill_count
          end
          local.get $ptr i32.const 4 i32.add local.set $ptr
          br $loop
        end
        local.get $op f32.const 8 f32.eq
        if
          local.get $ptr i32.const 12 i32.add f32.load local.set $nx
          local.get $ptr i32.const 16 i32.add f32.load local.set $ny
          local.get $fill_active
          if
            local.get $fill_points local.get $fill_count
            local.get $cx local.get $cy
            local.get $ptr i32.const 4 i32.add f32.load
            local.get $ptr i32.const 8 i32.add f32.load
            local.get $nx local.get $ny
            call $icon_fill_quad_append local.set $fill_count
            local.get $nx local.set $cx
            local.get $ny local.set $cy
            local.get $ptr i32.const 20 i32.add local.set $ptr
            br $loop
          end
          local.get $has_current
          if
            local.get $has_segment
            local.get $stroke_join f32.const 1 f32.eq
            i32.and
            if
              local.get $alpha local.get $width local.get $height local.get $x local.get $y local.get $w local.get $h local.get $cx local.get $cy local.get $stroke call $icon_render_round_cap_alpha
            end
            local.get $dash_on f32.const 0 f32.gt
            local.get $dash_off f32.const 0 f32.gt
            i32.and
            if
              local.get $alpha local.get $width local.get $height local.get $x local.get $y local.get $w local.get $h
              local.get $cx local.get $cy
              local.get $ptr i32.const 4 i32.add f32.load
              local.get $ptr i32.const 8 i32.add f32.load
              local.get $nx local.get $ny local.get $stroke
              local.get $stroke_cap
              local.get $dash_on local.get $dash_off local.get $dash_offset
              call $icon_render_dashed_quad_alpha
              local.set $dash_offset
            else
              local.get $alpha local.get $width local.get $height local.get $x local.get $y local.get $w local.get $h
              local.get $cx local.get $cy
              local.get $ptr i32.const 4 i32.add f32.load
              local.get $ptr i32.const 8 i32.add f32.load
              local.get $nx local.get $ny local.get $stroke
              local.get $stroke_cap
              call $icon_render_quad_alpha
            end
          end
          i32.const 1 local.set $has_segment
          local.get $nx local.set $cx
          local.get $ny local.set $cy
          local.get $fill_active
          if
            local.get $fill_points local.get $fill_count local.get $cx local.get $cy call $icon_fill_point_append local.set $fill_count
          end
          local.get $ptr i32.const 20 i32.add local.set $ptr
          br $loop
        end
        local.get $op f32.const 9 f32.eq
        if
          local.get $ptr i32.const 20 i32.add f32.load local.set $nx
          local.get $ptr i32.const 24 i32.add f32.load local.set $ny
          local.get $fill_active
          if
            local.get $fill_points local.get $fill_count
            local.get $cx local.get $cy
            local.get $ptr i32.const 4 i32.add f32.load
            local.get $ptr i32.const 8 i32.add f32.load
            local.get $ptr i32.const 12 i32.add f32.load
            local.get $ptr i32.const 16 i32.add f32.load
            local.get $nx local.get $ny
            call $icon_fill_cubic_append local.set $fill_count
            local.get $nx local.set $cx
            local.get $ny local.set $cy
            local.get $ptr i32.const 28 i32.add local.set $ptr
            br $loop
          end
          local.get $has_current
          if
            local.get $has_segment
            local.get $stroke_join f32.const 1 f32.eq
            i32.and
            if
              local.get $alpha local.get $width local.get $height local.get $x local.get $y local.get $w local.get $h local.get $cx local.get $cy local.get $stroke call $icon_render_round_cap_alpha
            end
            local.get $dash_on f32.const 0 f32.gt
            local.get $dash_off f32.const 0 f32.gt
            i32.and
            if
              local.get $alpha local.get $width local.get $height local.get $x local.get $y local.get $w local.get $h
              local.get $cx local.get $cy
              local.get $ptr i32.const 4 i32.add f32.load
              local.get $ptr i32.const 8 i32.add f32.load
              local.get $ptr i32.const 12 i32.add f32.load
              local.get $ptr i32.const 16 i32.add f32.load
              local.get $nx local.get $ny local.get $stroke
              local.get $stroke_cap
              local.get $dash_on local.get $dash_off local.get $dash_offset
              call $icon_render_dashed_cubic_alpha
              local.set $dash_offset
            else
              local.get $alpha local.get $width local.get $height local.get $x local.get $y local.get $w local.get $h
              local.get $cx local.get $cy
              local.get $ptr i32.const 4 i32.add f32.load
              local.get $ptr i32.const 8 i32.add f32.load
              local.get $ptr i32.const 12 i32.add f32.load
              local.get $ptr i32.const 16 i32.add f32.load
              local.get $nx local.get $ny local.get $stroke
              local.get $stroke_cap
              call $icon_render_cubic_alpha
            end
          end
          i32.const 1 local.set $has_segment
          local.get $nx local.set $cx
          local.get $ny local.set $cy
          local.get $fill_active
          if
            local.get $fill_points local.get $fill_count local.get $cx local.get $cy call $icon_fill_point_append local.set $fill_count
          end
          local.get $ptr i32.const 28 i32.add local.set $ptr
          br $loop
        end
        local.get $op f32.const 10 f32.eq
        if
          local.get $ptr i32.const 24 i32.add f32.load local.set $nx
          local.get $ptr i32.const 28 i32.add f32.load local.set $ny
          local.get $fill_active
          if
            local.get $fill_points local.get $fill_count
            local.get $cx local.get $cy
            local.get $ptr i32.const 4 i32.add f32.load
            local.get $ptr i32.const 8 i32.add f32.load
            local.get $ptr i32.const 12 i32.add f32.load
            local.get $ptr i32.const 16 i32.add f32.load
            local.get $ptr i32.const 20 i32.add f32.load
            local.get $nx local.get $ny
            call $icon_fill_arc_append local.set $fill_count
            local.get $nx local.set $cx
            local.get $ny local.set $cy
            local.get $ptr i32.const 32 i32.add local.set $ptr
            br $loop
          end
          local.get $has_current
          if
            local.get $has_segment
            local.get $stroke_join f32.const 1 f32.eq
            i32.and
            if
              local.get $alpha local.get $width local.get $height local.get $x local.get $y local.get $w local.get $h local.get $cx local.get $cy local.get $stroke call $icon_render_round_cap_alpha
            end
            local.get $dash_on f32.const 0 f32.gt
            local.get $dash_off f32.const 0 f32.gt
            i32.and
            if
              local.get $alpha local.get $width local.get $height local.get $x local.get $y local.get $w local.get $h
              local.get $cx local.get $cy
              local.get $ptr i32.const 4 i32.add f32.load
              local.get $ptr i32.const 8 i32.add f32.load
              local.get $ptr i32.const 12 i32.add f32.load
              local.get $ptr i32.const 16 i32.add f32.load
              local.get $ptr i32.const 20 i32.add f32.load
              local.get $nx local.get $ny local.get $stroke
              local.get $stroke_cap
              local.get $dash_on local.get $dash_off local.get $dash_offset
              call $icon_render_dashed_arc_alpha
              local.set $dash_offset
            else
              local.get $alpha local.get $width local.get $height local.get $x local.get $y local.get $w local.get $h
              local.get $cx local.get $cy
              local.get $ptr i32.const 4 i32.add f32.load
              local.get $ptr i32.const 8 i32.add f32.load
              local.get $ptr i32.const 12 i32.add f32.load
              local.get $ptr i32.const 16 i32.add f32.load
              local.get $ptr i32.const 20 i32.add f32.load
              local.get $nx local.get $ny local.get $stroke
              local.get $stroke_cap
              call $icon_render_arc_alpha
            end
          end
          i32.const 1 local.set $has_segment
          local.get $nx local.set $cx
          local.get $ny local.set $cy
          local.get $fill_active
          if
            local.get $fill_points local.get $fill_count local.get $cx local.get $cy call $icon_fill_point_append local.set $fill_count
          end
          local.get $ptr i32.const 32 i32.add local.set $ptr
          br $loop
        end
        local.get $op f32.const 1 f32.eq
        if
          local.get $ptr i32.const 4 i32.add f32.load i32.trunc_f32_u local.set $poly_count
          local.get $poly_count i32.const 2 i32.lt_u
          if
            local.get $ptr i32.const 8 i32.add local.set $ptr
            br $loop
          end
          local.get $ptr i32.const 8 i32.add f32.load local.set $sx
          local.get $ptr i32.const 12 i32.add f32.load local.set $sy
          local.get $sx local.set $cx
          local.get $sy local.set $cy
          local.get $sx local.set $poly_prev_x
          local.get $sy local.set $poly_prev_y
          local.get $dash_base_offset local.set $dash_offset
          i32.const 1 local.set $poly_i
          block $poly_done
            loop $poly_loop
              local.get $poly_i local.get $poly_count i32.ge_u br_if $poly_done
              local.get $ptr i32.const 8 i32.add local.get $poly_i i32.const 8 i32.mul i32.add f32.load local.set $nx
              local.get $ptr i32.const 12 i32.add local.get $poly_i i32.const 8 i32.mul i32.add f32.load local.set $ny
              local.get $poly_i i32.const 1 i32.gt_u
              local.get $stroke_join f32.const 1 f32.eq
              i32.and
              if
                local.get $alpha local.get $width local.get $height local.get $x local.get $y local.get $w local.get $h local.get $cx local.get $cy local.get $stroke call $icon_render_round_cap_alpha
              end
              local.get $poly_i i32.const 1 i32.gt_u
              local.get $stroke_join f32.const 0 f32.eq
              i32.and
              if
                local.get $alpha local.get $width local.get $height local.get $x local.get $y local.get $w local.get $h local.get $poly_prev_x local.get $poly_prev_y local.get $cx local.get $cy local.get $nx local.get $ny local.get $stroke local.get $stroke_miter_limit call $icon_render_miter_join_alpha
              end
              local.get $poly_i i32.const 1 i32.gt_u
              local.get $stroke_join f32.const 2 f32.eq
              i32.and
              if
                local.get $alpha local.get $width local.get $height local.get $x local.get $y local.get $w local.get $h local.get $poly_prev_x local.get $poly_prev_y local.get $cx local.get $cy local.get $nx local.get $ny local.get $stroke call $icon_render_bevel_join_alpha
              end
              local.get $dash_on f32.const 0 f32.gt
              local.get $dash_off f32.const 0 f32.gt
              i32.and
              if
                local.get $alpha local.get $width local.get $height local.get $x local.get $y local.get $w local.get $h local.get $cx local.get $cy local.get $nx local.get $ny local.get $stroke local.get $stroke_cap local.get $dash_on local.get $dash_off local.get $dash_offset call $icon_render_dashed_line_alpha
                local.get $nx local.get $cx f32.sub local.set $dash_dx
                local.get $ny local.get $cy f32.sub local.set $dash_dy
                local.get $dash_offset
                local.get $dash_dx local.get $dash_dx f32.mul
                local.get $dash_dy local.get $dash_dy f32.mul
                f32.add
                f32.sqrt
                f32.add
                local.set $dash_offset
              else
                local.get $alpha local.get $width local.get $height local.get $x local.get $y local.get $w local.get $h local.get $cx local.get $cy local.get $nx local.get $ny local.get $stroke local.get $stroke_cap call $icon_render_line_alpha
              end
              local.get $cx local.set $poly_prev_x
              local.get $cy local.set $poly_prev_y
              local.get $nx local.set $cx
              local.get $ny local.set $cy
              local.get $poly_i i32.const 1 i32.add local.set $poly_i
              br $poly_loop
            end
          end
          local.get $stroke_cap f32.const 1 f32.eq
          local.get $dash_on f32.const 0 f32.gt
          local.get $dash_off f32.const 0 f32.gt
          i32.and
          i32.eqz
          i32.and
          if
            local.get $alpha local.get $width local.get $height local.get $x local.get $y local.get $w local.get $h local.get $sx local.get $sy local.get $stroke call $icon_render_round_cap_alpha
            local.get $alpha local.get $width local.get $height local.get $x local.get $y local.get $w local.get $h local.get $cx local.get $cy local.get $stroke call $icon_render_round_cap_alpha
          end
          local.get $ptr i32.const 8 i32.add local.get $poly_count i32.const 8 i32.mul i32.add local.set $ptr
          br $loop
        end
        local.get $op f32.const 2 f32.eq
        local.get $op f32.const 5 f32.eq i32.or
        if
          local.get $alpha local.get $width local.get $height local.get $x local.get $y local.get $w local.get $h
          local.get $ptr i32.const 4 i32.add f32.load
          local.get $ptr i32.const 8 i32.add f32.load
          local.get $ptr i32.const 12 i32.add f32.load
          local.get $ptr i32.const 12 i32.add f32.load
          local.get $stroke
          local.get $op f32.const 5 f32.eq
          call $icon_render_ellipse_alpha
          local.get $ptr i32.const 16 i32.add local.set $ptr
          br $loop
        end
        local.get $op f32.const 3 f32.eq
        local.get $op f32.const 12 f32.eq i32.or
        if
          local.get $alpha local.get $width local.get $height local.get $x local.get $y local.get $w local.get $h
          local.get $ptr i32.const 4 i32.add f32.load
          local.get $ptr i32.const 8 i32.add f32.load
          local.get $ptr i32.const 12 i32.add f32.load
          local.get $ptr i32.const 16 i32.add f32.load
          local.get $stroke
          local.get $op f32.const 12 f32.eq
          call $icon_render_ellipse_alpha
          local.get $ptr i32.const 24 i32.add local.set $ptr
          br $loop
        end
        local.get $op f32.const 4 f32.eq
        local.get $op f32.const 13 f32.eq i32.or
        if
          local.get $alpha local.get $width local.get $height local.get $x local.get $y local.get $w local.get $h
          local.get $ptr i32.const 4 i32.add f32.load
          local.get $ptr i32.const 8 i32.add f32.load
          local.get $ptr i32.const 12 i32.add f32.load
          local.get $ptr i32.const 16 i32.add f32.load
          local.get $ptr i32.const 20 i32.add f32.load
          local.get $stroke
          local.get $op f32.const 13 f32.eq
          call $icon_render_round_rect_alpha
          local.get $ptr i32.const 24 i32.add local.set $ptr
          br $loop
        end
        local.get $op f32.const 23 f32.eq
        if
          local.get $ptr i32.const 4 i32.add f32.load local.set $stroke_cap
          local.get $ptr i32.const 8 i32.add local.set $ptr
          br $loop
        end
        local.get $op f32.const 21 f32.eq
        if
          local.get $ptr i32.const 4 i32.add f32.load
          f32.const 255
          f32.div
          f32.const 0
          f32.const 1
          call $clamp_f32
          global.set $icon_alpha_paint_scale
          i32.const 3
          global.set $icon_paint_kind
          i32.const 0
          global.set $icon_gradient_stops_ptr
          i32.const 0
          global.set $icon_gradient_stop_count
          i32.const 0
          global.set $icon_gradient_spread
          local.get $ptr i32.const 8 i32.add local.set $ptr
          br $loop
        end
        local.get $op f32.const 24 f32.eq
        if
          local.get $ptr i32.const 4 i32.add f32.load local.set $stroke_join
          local.get $ptr i32.const 8 i32.add local.set $ptr
          br $loop
        end
        local.get $op f32.const 25 f32.eq
        if
          local.get $ptr i32.const 4 i32.add f32.load local.set $stroke_miter_limit
          local.get $ptr i32.const 8 i32.add local.set $ptr
          br $loop
        end
        local.get $op f32.const 31 f32.eq
        if
          local.get $ptr i32.const 4 i32.add f32.load global.set $icon_paint_matrix_a
          local.get $ptr i32.const 8 i32.add f32.load global.set $icon_paint_matrix_b
          local.get $ptr i32.const 12 i32.add f32.load global.set $icon_paint_matrix_c
          local.get $ptr i32.const 16 i32.add f32.load global.set $icon_paint_matrix_d
          local.get $ptr i32.const 20 i32.add f32.load global.set $icon_paint_matrix_tx
          local.get $ptr i32.const 24 i32.add f32.load global.set $icon_paint_matrix_ty
          local.get $ptr i32.const 28 i32.add local.set $ptr
          br $loop
        end
        local.get $op f32.const 19 f32.eq
        if
          local.get $ptr i32.const 28 i32.add f32.load i32.trunc_f32_u local.set $grad_count
          local.get $ptr i32.const 32 i32.add global.set $icon_gradient_stops_ptr
          local.get $grad_count global.set $icon_gradient_stop_count
          local.get $ptr i32.const 8 i32.add f32.load i32.trunc_f32_u global.set $icon_gradient_spread
          local.get $ptr i32.const 12 i32.add f32.load global.set $icon_linear_x1
          local.get $ptr i32.const 16 i32.add f32.load global.set $icon_linear_y1
          local.get $ptr i32.const 20 i32.add f32.load global.set $icon_linear_x2
          local.get $ptr i32.const 24 i32.add f32.load global.set $icon_linear_y2
          f32.const 1 global.set $icon_paint_matrix_a
          f32.const 0 global.set $icon_paint_matrix_b
          f32.const 0 global.set $icon_paint_matrix_c
          f32.const 1 global.set $icon_paint_matrix_d
          f32.const 0 global.set $icon_paint_matrix_tx
          f32.const 0 global.set $icon_paint_matrix_ty
          local.get $grad_count i32.const 0 i32.gt_u
          if
            local.get $ptr i32.const 48 i32.add f32.load f32.const 255 f32.div f32.const 0 f32.const 1 call $clamp_f32 global.set $icon_linear_a0
            local.get $ptr i32.const 48 i32.add local.get $grad_count i32.const 1 i32.sub i32.const 20 i32.mul i32.add f32.load f32.const 255 f32.div f32.const 0 f32.const 1 call $clamp_f32 global.set $icon_linear_a1
          end
          i32.const 1 global.set $icon_paint_kind
          f32.const 0 local.set $grad_alpha
          i32.const 0 local.set $grad_i
          block $grad_done
            loop $grad_loop
              local.get $grad_i local.get $grad_count i32.ge_u br_if $grad_done
              local.get $ptr i32.const 48 i32.add local.get $grad_i i32.const 20 i32.mul i32.add f32.load
              f32.const 255
              f32.div
              local.get $grad_alpha
              call $max_f32
              local.set $grad_alpha
              local.get $grad_i i32.const 1 i32.add local.set $grad_i
              br $grad_loop
            end
          end
          local.get $grad_alpha f32.const 0 f32.const 1 call $clamp_f32 global.set $icon_alpha_paint_scale
          local.get $ptr i32.const 32 i32.add local.get $grad_count i32.const 20 i32.mul i32.add local.set $ptr
          br $loop
        end
        local.get $op f32.const 20 f32.eq
        if
          local.get $ptr i32.const 36 i32.add f32.load i32.trunc_f32_u local.set $grad_count
          local.get $ptr i32.const 40 i32.add global.set $icon_gradient_stops_ptr
          local.get $grad_count global.set $icon_gradient_stop_count
          local.get $ptr i32.const 8 i32.add f32.load i32.trunc_f32_u global.set $icon_gradient_spread
          local.get $ptr i32.const 12 i32.add f32.load global.set $icon_radial_cx
          local.get $ptr i32.const 16 i32.add f32.load global.set $icon_radial_cy
          local.get $ptr i32.const 20 i32.add f32.load global.set $icon_radial_r
          local.get $ptr i32.const 24 i32.add f32.load global.set $icon_radial_fx
          local.get $ptr i32.const 28 i32.add f32.load global.set $icon_radial_fy
          local.get $ptr i32.const 32 i32.add f32.load global.set $icon_radial_fr
          f32.const 1 global.set $icon_paint_matrix_a
          f32.const 0 global.set $icon_paint_matrix_b
          f32.const 0 global.set $icon_paint_matrix_c
          f32.const 1 global.set $icon_paint_matrix_d
          f32.const 0 global.set $icon_paint_matrix_tx
          f32.const 0 global.set $icon_paint_matrix_ty
          local.get $grad_count i32.const 0 i32.gt_u
          if
            local.get $ptr i32.const 56 i32.add f32.load f32.const 255 f32.div f32.const 0 f32.const 1 call $clamp_f32 global.set $icon_radial_a0
            local.get $ptr i32.const 56 i32.add local.get $grad_count i32.const 1 i32.sub i32.const 20 i32.mul i32.add f32.load f32.const 255 f32.div f32.const 0 f32.const 1 call $clamp_f32 global.set $icon_radial_a1
          end
          i32.const 2 global.set $icon_paint_kind
          f32.const 0 local.set $grad_alpha
          i32.const 0 local.set $grad_i
          block $rad_grad_done
            loop $rad_grad_loop
              local.get $grad_i local.get $grad_count i32.ge_u br_if $rad_grad_done
              local.get $ptr i32.const 56 i32.add local.get $grad_i i32.const 20 i32.mul i32.add f32.load
              f32.const 255
              f32.div
              local.get $grad_alpha
              call $max_f32
              local.set $grad_alpha
              local.get $grad_i i32.const 1 i32.add local.set $grad_i
              br $rad_grad_loop
            end
          end
          local.get $grad_alpha f32.const 0 f32.const 1 call $clamp_f32 global.set $icon_alpha_paint_scale
          local.get $ptr i32.const 40 i32.add local.get $grad_count i32.const 20 i32.mul i32.add local.set $ptr
          br $loop
        end
        local.get $op f32.const 17 f32.eq
        if
          local.get $ptr i32.const 4 i32.add f32.load f32.const 0 f32.const 255 call $clamp_f32 global.set $icon_paint_r
          local.get $ptr i32.const 8 i32.add f32.load f32.const 0 f32.const 255 call $clamp_f32 global.set $icon_paint_g
          local.get $ptr i32.const 12 i32.add f32.load f32.const 0 f32.const 255 call $clamp_f32 global.set $icon_paint_b
          local.get $ptr i32.const 16 i32.add f32.load
          f32.const 255
          f32.div
          f32.const 0
          f32.const 1
          call $clamp_f32
          global.set $icon_alpha_paint_scale
          i32.const 0
          global.set $icon_paint_kind
          i32.const 0
          global.set $icon_gradient_stops_ptr
          i32.const 0
          global.set $icon_gradient_stop_count
          i32.const 0
          global.set $icon_gradient_spread
          local.get $ptr i32.const 20 i32.add local.set $ptr
          br $loop
        end
        local.get $op f32.const 14 f32.eq
        local.get $op f32.const 16 f32.eq i32.or
        if
          i32.const 3900000 local.set $fill_points
          i32.const 0 local.set $clip_capture
          i32.const 1 local.set $fill_active
          local.get $op f32.const 16 f32.eq local.set $fill_rule
          i32.const 0 local.set $fill_count
          i32.const 0 local.set $has_segment
          local.get $ptr i32.const 4 i32.add local.set $ptr
          br $loop
        end
        local.get $op f32.const 15 f32.eq
        if
          local.get $fill_active
          if
            local.get $alpha local.get $width local.get $height local.get $x local.get $y local.get $w local.get $h local.get $fill_points local.get $fill_count local.get $fill_rule call $icon_fill_polygon_alpha
          end
          i32.const 0 local.set $fill_active
          i32.const 0 local.set $fill_count
          local.get $ptr i32.const 4 i32.add local.set $ptr
          br $loop
        end
        local.get $op f32.const 26 f32.eq
        local.get $op f32.const 29 f32.eq i32.or
        if
          global.get $icon_clip3_enabled
          if
            i32.const 4150000 local.set $fill_points
          else
            global.get $icon_clip2_enabled
            if
              i32.const 4100000 local.set $fill_points
            else
              global.get $icon_clip_enabled
              if
                i32.const 4050000 local.set $fill_points
              else
                i32.const 4000000 local.set $fill_points
              end
            end
          end
          i32.const 1 local.set $clip_capture
          i32.const 1 local.set $fill_active
          local.get $op f32.const 29 f32.eq local.set $fill_rule
          i32.const 0 local.set $fill_count
          i32.const 0 local.set $has_segment
          local.get $ptr i32.const 4 i32.add local.set $ptr
          br $loop
        end
        local.get $op f32.const 27 f32.eq
        if
          local.get $clip_capture
          if
            local.get $fill_count i32.const 3 i32.ge_u
            if
              global.get $icon_clip3_enabled
              if
                i32.const 1 global.set $icon_clip4_enabled
                local.get $fill_points global.set $icon_clip4_points
                local.get $fill_count global.set $icon_clip4_count
                local.get $fill_rule global.set $icon_clip4_rule
              else
                global.get $icon_clip2_enabled
                if
                  i32.const 1 global.set $icon_clip3_enabled
                  local.get $fill_points global.set $icon_clip3_points
                  local.get $fill_count global.set $icon_clip3_count
                  local.get $fill_rule global.set $icon_clip3_rule
                else
                  global.get $icon_clip_enabled
                  if
                    i32.const 1 global.set $icon_clip2_enabled
                    local.get $fill_points global.set $icon_clip2_points
                    local.get $fill_count global.set $icon_clip2_count
                    local.get $fill_rule global.set $icon_clip2_rule
                  else
                    i32.const 1 global.set $icon_clip_enabled
                    local.get $fill_points global.set $icon_clip_points
                    local.get $fill_count global.set $icon_clip_count
                    local.get $fill_rule global.set $icon_clip_rule
                  end
                end
              end
            end
          end
          i32.const 0 local.set $clip_capture
          i32.const 0 local.set $fill_active
          i32.const 0 local.set $fill_count
          i32.const 3900000 local.set $fill_points
          local.get $ptr i32.const 4 i32.add local.set $ptr
          br $loop
        end
        local.get $op f32.const 28 f32.eq
        if
          i32.const 0 global.set $icon_clip_enabled
          i32.const 0 global.set $icon_clip2_enabled
          i32.const 0 global.set $icon_clip3_enabled
          i32.const 0 global.set $icon_clip4_enabled
          local.get $ptr i32.const 4 i32.add local.set $ptr
          br $loop
        end
        local.get $op f32.const 18 f32.eq
        if
          f32.const 1
          global.set $icon_alpha_paint_scale
          i32.const 3
          global.set $icon_paint_kind
          i32.const 0
          global.set $icon_gradient_stops_ptr
          i32.const 0
          global.set $icon_gradient_stop_count
          i32.const 0
          global.set $icon_gradient_spread
          local.get $ptr i32.const 4 i32.add local.set $ptr
          br $loop
        end
        br $done
      end
    end
    local.get $has_segment
    if
      local.get $stroke_cap f32.const 1 f32.eq
      if
        local.get $alpha local.get $width local.get $height local.get $x local.get $y local.get $w local.get $h local.get $sx local.get $sy local.get $stroke call $icon_render_round_cap_alpha
        local.get $alpha local.get $width local.get $height local.get $x local.get $y local.get $w local.get $h local.get $cx local.get $cy local.get $stroke call $icon_render_round_cap_alpha
      end
    end
    local.get $alpha local.get $width local.get $height call $icon_alpha_count)

  (func (export "er_ui_icon_render_rgba") (param $icon i32) (param $rgba i32) (param $width i32) (param $height i32) (param $x f32) (param $y f32) (param $w f32) (param $h f32) (param $current_rgba i32) (result i32)
    (local $count i32)
    local.get $rgba
    i32.eqz
    local.get $width
    i32.eqz
    i32.or
    local.get $height
    i32.eqz
    i32.or
    if
      i32.const -1
      return
    end
    local.get $rgba
    i32.const 0
    local.get $width
    local.get $height
    i32.mul
    i32.const 4
    i32.mul
    memory.fill
    local.get $rgba
    global.set $icon_rgba_out
    local.get $current_rgba i32.const 255 i32.and f32.convert_i32_u global.set $icon_current_r
    local.get $current_rgba i32.const 8 i32.shr_u i32.const 255 i32.and f32.convert_i32_u global.set $icon_current_g
    local.get $current_rgba i32.const 16 i32.shr_u i32.const 255 i32.and f32.convert_i32_u global.set $icon_current_b
    local.get $current_rgba i32.const 24 i32.shr_u i32.const 255 i32.and f32.convert_i32_u global.set $icon_current_a
    local.get $icon
    i32.const 3800000
    local.get $width
    local.get $height
    local.get $x
    local.get $y
    local.get $w
    local.get $h
    call $er_ui_icon_render_alpha
    local.set $count
    i32.const 0
    global.set $icon_rgba_out
    local.get $count)

  (func $er_ui_icon_stroke_width (export "er_ui_icon_stroke_width") (param $icon i32) (result f32)
    local.get $icon
    call $er_ui_icon_valid
    if (result f32)
      f32.const 2
    else
      f32.const 0
    end)

  (func (export "er_ui_icon_bounds") (param $icon i32) (param $out i32) (result i32)
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

  (func $er_ui_command_tag_icon (export "er_ui_command_tag_icon") (result i32)
    i32.const 3)

  (func $er_ui_icon_segment_count (export "er_ui_icon_segment_count") (param $icon i32) (result i32)
    local.get $icon
    i32.const 44
    i32.eq
    local.get $icon
    i32.const 2
    i32.eq
    i32.or
    if
      i32.const 2
      return
    end
    local.get $icon
    i32.const 55
    i32.eq
    if
      i32.const 5
      return
    end
    local.get $icon
    i32.const 99
    i32.eq
    if
      i32.const 4
      return
    end
    local.get $icon
    i32.const 1
    i32.eq
    local.get $icon
    i32.const 3
    i32.eq
    i32.or
    if
      i32.const 2
      return
    end
    i32.const 4)
