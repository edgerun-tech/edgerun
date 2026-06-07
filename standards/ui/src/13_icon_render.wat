  (func $icon_clip_points_contains (param $px i32) (param $py i32) (param $points i32) (param $count i32) (param $rule i32) (result i32)
    (local $pnx f32) (local $pny f32) (local $i i32) (local $j i32) (local $inside i32) (local $winding i32)
    (local $xi f32) (local $yi f32) (local $xj f32) (local $yj f32) (local $xhit f32)
    local.get $count
    i32.const 3
    i32.lt_u
    if
      i32.const 0
      return
    end
    local.get $px f32.convert_i32_u f32.const 0.5 f32.add global.get $icon_clip_bx f32.sub global.get $icon_clip_bw f32.div local.set $pnx
    local.get $py f32.convert_i32_u f32.const 0.5 f32.add global.get $icon_clip_by f32.sub global.get $icon_clip_bh f32.div local.set $pny
    i32.const 0 local.set $i
    block $done_edges
      loop $loop_edges
        local.get $i local.get $count i32.ge_u br_if $done_edges
        local.get $points local.get $i i32.const 8 i32.mul i32.add f32.load local.set $xi
        local.get $points local.get $i i32.const 8 i32.mul i32.add i32.const 4 i32.add f32.load local.set $yi
        local.get $xi
        f32.const -999999
        f32.lt
        if
          local.get $i i32.const 1 i32.add local.set $i
          br $loop_edges
        end
        local.get $i
        i32.eqz
        if
          local.get $i local.set $j
          block $contour_done
            loop $contour_loop
              local.get $j i32.const 1 i32.add local.get $count i32.ge_u br_if $contour_done
              local.get $points local.get $j i32.const 1 i32.add i32.const 8 i32.mul i32.add f32.load local.set $xj
              local.get $xj f32.const -999999 f32.lt br_if $contour_done
              local.get $j i32.const 1 i32.add local.set $j
              br $contour_loop
            end
          end
        else
          local.get $points local.get $i i32.const 1 i32.sub i32.const 8 i32.mul i32.add f32.load local.set $xj
          local.get $xj
          f32.const -999999
          f32.lt
          if
            local.get $i local.set $j
            block $contour_done
              loop $contour_loop
                local.get $j i32.const 1 i32.add local.get $count i32.ge_u br_if $contour_done
                local.get $points local.get $j i32.const 1 i32.add i32.const 8 i32.mul i32.add f32.load local.set $xj
                local.get $xj f32.const -999999 f32.lt br_if $contour_done
                local.get $j i32.const 1 i32.add local.set $j
                br $contour_loop
              end
            end
          else
            local.get $i i32.const 1 i32.sub local.set $j
          end
        end
        local.get $points local.get $j i32.const 8 i32.mul i32.add f32.load local.set $xj
        local.get $points local.get $j i32.const 8 i32.mul i32.add i32.const 4 i32.add f32.load local.set $yj
        local.get $yi local.get $pny f32.le
        local.get $yj local.get $pny f32.gt
        i32.and
        if
          local.get $xj local.get $xi f32.sub local.get $pny local.get $yi f32.sub f32.mul local.get $yj local.get $yi f32.sub f32.div local.get $xi f32.add local.set $xhit
          local.get $pnx local.get $xhit f32.lt
          if
            local.get $rule
            if
              local.get $inside i32.const 1 i32.xor local.set $inside
            else
              local.get $winding i32.const 1 i32.add local.set $winding
            end
          end
        end
        local.get $yj local.get $pny f32.le
        local.get $yi local.get $pny f32.gt
        i32.and
        if
          local.get $xj local.get $xi f32.sub local.get $pny local.get $yi f32.sub f32.mul local.get $yj local.get $yi f32.sub f32.div local.get $xi f32.add local.set $xhit
          local.get $pnx local.get $xhit f32.lt
          if
            local.get $rule
            if
              local.get $inside i32.const 1 i32.xor local.set $inside
            else
              local.get $winding i32.const 1 i32.sub local.set $winding
            end
          end
        end
        local.get $i i32.const 1 i32.add local.set $i
        br $loop_edges
      end
    end
    local.get $rule
    if (result i32)
      local.get $inside
    else
      local.get $winding i32.const 0 i32.ne
    end)

  (func $icon_clip_pixel_contains (param $px i32) (param $py i32) (result i32)
    global.get $icon_clip_enabled
    i32.eqz
    if
      i32.const 1
      return
    end
    local.get $px local.get $py global.get $icon_clip_points global.get $icon_clip_count global.get $icon_clip_rule call $icon_clip_points_contains
    i32.eqz
    if
      i32.const 0
      return
    end
    global.get $icon_clip2_enabled
    if
      local.get $px local.get $py global.get $icon_clip2_points global.get $icon_clip2_count global.get $icon_clip2_rule call $icon_clip_points_contains
      i32.eqz
      if
        i32.const 0
        return
      end
    end
    global.get $icon_clip3_enabled
    if
      local.get $px local.get $py global.get $icon_clip3_points global.get $icon_clip3_count global.get $icon_clip3_rule call $icon_clip_points_contains
      i32.eqz
      if
        i32.const 0
        return
      end
    end
    global.get $icon_clip4_enabled
    if
      local.get $px local.get $py global.get $icon_clip4_points global.get $icon_clip4_count global.get $icon_clip4_rule call $icon_clip_points_contains
      i32.eqz
      if
        i32.const 0
        return
      end
    end
    i32.const 1)

  (func $icon_gradient_alpha_at (param $t f32) (result f32)
    (local $i i32) (local $ptr i32)
    (local $whole i32) (local $has_lower i32) (local $has_upper i32)
    (local $lower_off f32) (local $lower_alpha f32) (local $upper_off f32) (local $upper_alpha f32)
    (local $off f32) (local $alpha f32) (local $mix f32)
    global.get $icon_gradient_stops_ptr
    i32.eqz
    global.get $icon_gradient_stop_count
    i32.eqz
    i32.or
    if
      global.get $icon_alpha_paint_scale
      return
    end
    global.get $icon_gradient_spread
    i32.const 1
    i32.eq
    if
      local.get $t f32.floor local.set $off
      local.get $t local.get $off f32.sub local.set $t
    end
    global.get $icon_gradient_spread
    i32.const 2
    i32.eq
    if
      local.get $t f32.floor local.set $off
      local.get $off i32.trunc_f32_s local.set $whole
      local.get $t local.get $off f32.sub local.set $t
      local.get $whole i32.const 1 i32.and
      if
        f32.const 1 local.get $t f32.sub local.set $t
      end
    end
    global.get $icon_gradient_spread
    i32.const 0
    i32.eq
    if
      local.get $t f32.const 0 f32.const 1 call $clamp_f32 local.set $t
    end
    global.get $icon_gradient_stop_count
    i32.const 1
    i32.eq
    if
      global.get $icon_gradient_stops_ptr i32.const 16 i32.add f32.load f32.const 255 f32.div f32.const 0 f32.const 1 call $clamp_f32
      return
    end
    i32.const 0 local.set $i
    block $done
      loop $loop
        local.get $i global.get $icon_gradient_stop_count i32.ge_u br_if $done
        global.get $icon_gradient_stops_ptr local.get $i i32.const 20 i32.mul i32.add local.set $ptr
        local.get $ptr f32.load local.set $off
        local.get $ptr i32.const 16 i32.add f32.load f32.const 255 f32.div f32.const 0 f32.const 1 call $clamp_f32 local.set $alpha
        local.get $off local.get $t f32.le
        if
          local.get $has_lower
          i32.eqz
          local.get $off local.get $lower_off f32.ge
          i32.or
          if
            i32.const 1 local.set $has_lower
            local.get $off local.set $lower_off
            local.get $alpha local.set $lower_alpha
          end
        end
        local.get $off local.get $t f32.ge
        if
          local.get $has_upper
          i32.eqz
          local.get $off local.get $upper_off f32.le
          i32.or
          if
            i32.const 1 local.set $has_upper
            local.get $off local.set $upper_off
            local.get $alpha local.set $upper_alpha
          end
        end
        local.get $i i32.const 1 i32.add local.set $i
        br $loop
      end
    end
    local.get $has_lower
    i32.eqz
    if
      local.get $upper_alpha
      return
    end
    local.get $has_upper
    i32.eqz
    if
      local.get $lower_alpha
      return
    end
    local.get $upper_off local.get $lower_off f32.sub f32.abs f32.const 0.000001 f32.le
    if
      local.get $upper_alpha
      return
    end
    local.get $upper_off local.get $lower_off f32.sub local.set $mix
    local.get $t local.get $lower_off f32.sub local.get $mix f32.div f32.const 0 f32.const 1 call $clamp_f32 local.set $mix
    local.get $lower_alpha f32.const 1 local.get $mix f32.sub f32.mul
    local.get $upper_alpha local.get $mix f32.mul
    f32.add)

  (func $icon_gradient_component_at (param $t f32) (param $component i32) (result f32)
    (local $i i32) (local $ptr i32)
    (local $whole i32) (local $has_lower i32) (local $has_upper i32)
    (local $lower_off f32) (local $lower_value f32) (local $upper_off f32) (local $upper_value f32)
    (local $off f32) (local $value f32) (local $mix f32)
    global.get $icon_gradient_stops_ptr
    i32.eqz
    global.get $icon_gradient_stop_count
    i32.eqz
    i32.or
    if
      local.get $component
      i32.const 4
      i32.eq
      if (result f32)
        global.get $icon_paint_r
      else
        local.get $component
        i32.const 8
        i32.eq
        if (result f32)
          global.get $icon_paint_g
        else
          local.get $component
          i32.const 12
          i32.eq
          if (result f32)
            global.get $icon_paint_b
          else
            global.get $icon_alpha_paint_scale f32.const 255 f32.mul
          end
        end
      end
      return
    end
    global.get $icon_gradient_spread
    i32.const 1
    i32.eq
    if
      local.get $t f32.floor local.set $off
      local.get $t local.get $off f32.sub local.set $t
    end
    global.get $icon_gradient_spread
    i32.const 2
    i32.eq
    if
      local.get $t f32.floor local.set $off
      local.get $off i32.trunc_f32_s local.set $whole
      local.get $t local.get $off f32.sub local.set $t
      local.get $whole i32.const 1 i32.and
      if
        f32.const 1 local.get $t f32.sub local.set $t
      end
    end
    global.get $icon_gradient_spread
    i32.const 0
    i32.eq
    if
      local.get $t f32.const 0 f32.const 1 call $clamp_f32 local.set $t
    end
    global.get $icon_gradient_stop_count
    i32.const 1
    i32.eq
    if
      global.get $icon_gradient_stops_ptr local.get $component i32.add f32.load
      f32.const 0 f32.const 255 call $clamp_f32
      return
    end
    i32.const 0 local.set $i
    block $done
      loop $loop
        local.get $i global.get $icon_gradient_stop_count i32.ge_u br_if $done
        global.get $icon_gradient_stops_ptr local.get $i i32.const 20 i32.mul i32.add local.set $ptr
        local.get $ptr f32.load local.set $off
        local.get $ptr local.get $component i32.add f32.load f32.const 0 f32.const 255 call $clamp_f32 local.set $value
        local.get $off local.get $t f32.le
        if
          local.get $has_lower
          i32.eqz
          local.get $off local.get $lower_off f32.ge
          i32.or
          if
            i32.const 1 local.set $has_lower
            local.get $off local.set $lower_off
            local.get $value local.set $lower_value
          end
        end
        local.get $off local.get $t f32.ge
        if
          local.get $has_upper
          i32.eqz
          local.get $off local.get $upper_off f32.le
          i32.or
          if
            i32.const 1 local.set $has_upper
            local.get $off local.set $upper_off
            local.get $value local.set $upper_value
          end
        end
        local.get $i i32.const 1 i32.add local.set $i
        br $loop
      end
    end
    local.get $has_lower
    i32.eqz
    if
      local.get $upper_value
      return
    end
    local.get $has_upper
    i32.eqz
    if
      local.get $lower_value
      return
    end
    local.get $upper_off local.get $lower_off f32.sub f32.abs f32.const 0.000001 f32.le
    if
      local.get $upper_value
      return
    end
    local.get $upper_off local.get $lower_off f32.sub local.set $mix
    local.get $t local.get $lower_off f32.sub local.get $mix f32.div f32.const 0 f32.const 1 call $clamp_f32 local.set $mix
    local.get $lower_value f32.const 1 local.get $mix f32.sub f32.mul
    local.get $upper_value local.get $mix f32.mul
    f32.add)

  (func $icon_alpha_store_max (param $alpha i32) (param $width i32) (param $height i32) (param $x i32) (param $y i32) (param $coverage f32)
    (local $addr i32)
    (local $rgba_addr i32)
    (local $value i32)
    (local $r i32) (local $g i32) (local $b i32)
    (local $paint_alpha f32)
    (local $pnx f32) (local $pny f32) (local $gx f32) (local $gy f32) (local $den f32) (local $t f32)
    (local $vx f32) (local $vy f32) (local $dr f32) (local $qa f32) (local $qb f32) (local $qc f32) (local $disc f32) (local $root1 f32) (local $root2 f32)
    local.get $x
    local.get $width
    i32.ge_u
    local.get $y
    local.get $height
    i32.ge_u
    i32.or
    local.get $coverage
    f32.const 0
    f32.le
    i32.or
    if
      return
    end
    local.get $x
    local.get $y
    call $icon_clip_pixel_contains
    i32.eqz
    if
      return
    end
    global.get $icon_alpha_paint_scale
    local.set $paint_alpha
    global.get $icon_paint_r f32.nearest i32.trunc_f32_u local.set $r
    global.get $icon_paint_g f32.nearest i32.trunc_f32_u local.set $g
    global.get $icon_paint_b f32.nearest i32.trunc_f32_u local.set $b
    global.get $icon_paint_kind
    i32.const 3
    i32.eq
    if
      global.get $icon_current_r f32.nearest i32.trunc_f32_u local.set $r
      global.get $icon_current_g f32.nearest i32.trunc_f32_u local.set $g
      global.get $icon_current_b f32.nearest i32.trunc_f32_u local.set $b
      global.get $icon_current_a f32.const 255 f32.div global.get $icon_alpha_paint_scale f32.mul local.set $paint_alpha
    end
    global.get $icon_paint_kind
    i32.const 1
    i32.eq
    if
      local.get $x f32.convert_i32_u f32.const 0.5 f32.add global.get $icon_clip_bx f32.sub global.get $icon_clip_bw f32.div local.set $pnx
      local.get $y f32.convert_i32_u f32.const 0.5 f32.add global.get $icon_clip_by f32.sub global.get $icon_clip_bh f32.div local.set $pny
      global.get $icon_paint_matrix_a local.get $pnx f32.mul
      global.get $icon_paint_matrix_c local.get $pny f32.mul
      f32.add
      global.get $icon_paint_matrix_tx
      f32.add
      local.set $gx
      global.get $icon_paint_matrix_b local.get $pnx f32.mul
      global.get $icon_paint_matrix_d local.get $pny f32.mul
      f32.add
      global.get $icon_paint_matrix_ty
      f32.add
      local.set $gy
      local.get $gx local.set $pnx
      local.get $gy local.set $pny
      global.get $icon_linear_x2 global.get $icon_linear_x1 f32.sub local.set $gx
      global.get $icon_linear_y2 global.get $icon_linear_y1 f32.sub local.set $gy
      local.get $gx local.get $gx f32.mul local.get $gy local.get $gy f32.mul f32.add local.set $den
      local.get $den f32.const 0.000001 f32.le
      if
        global.get $icon_linear_a1 local.set $paint_alpha
      else
        local.get $pnx global.get $icon_linear_x1 f32.sub local.get $gx f32.mul
        local.get $pny global.get $icon_linear_y1 f32.sub local.get $gy f32.mul
        f32.add
        local.get $den
        f32.div
        local.set $t
        local.get $t call $icon_gradient_alpha_at local.set $paint_alpha
        local.get $t i32.const 4 call $icon_gradient_component_at f32.nearest i32.trunc_f32_u local.set $r
        local.get $t i32.const 8 call $icon_gradient_component_at f32.nearest i32.trunc_f32_u local.set $g
        local.get $t i32.const 12 call $icon_gradient_component_at f32.nearest i32.trunc_f32_u local.set $b
      end
    end
    global.get $icon_paint_kind
    i32.const 2
    i32.eq
    if
      local.get $x f32.convert_i32_u f32.const 0.5 f32.add global.get $icon_clip_bx f32.sub global.get $icon_clip_bw f32.div local.set $pnx
      local.get $y f32.convert_i32_u f32.const 0.5 f32.add global.get $icon_clip_by f32.sub global.get $icon_clip_bh f32.div local.set $pny
      global.get $icon_paint_matrix_a local.get $pnx f32.mul
      global.get $icon_paint_matrix_c local.get $pny f32.mul
      f32.add
      global.get $icon_paint_matrix_tx
      f32.add
      local.set $gx
      global.get $icon_paint_matrix_b local.get $pnx f32.mul
      global.get $icon_paint_matrix_d local.get $pny f32.mul
      f32.add
      global.get $icon_paint_matrix_ty
      f32.add
      local.set $gy
      local.get $gx local.set $pnx
      local.get $gy local.set $pny
      global.get $icon_radial_cx global.get $icon_radial_fx f32.sub local.set $gx
      global.get $icon_radial_cy global.get $icon_radial_fy f32.sub local.set $gy
      global.get $icon_radial_r global.get $icon_radial_fr f32.sub local.set $dr
      global.get $icon_radial_fx local.get $pnx f32.sub local.set $vx
      global.get $icon_radial_fy local.get $pny f32.sub local.set $vy
      local.get $gx local.get $gx f32.mul local.get $gy local.get $gy f32.mul f32.add
      local.get $dr local.get $dr f32.mul f32.sub
      local.set $qa
      f32.const 2
      local.get $vx local.get $gx f32.mul local.get $vy local.get $gy f32.mul f32.add
      global.get $icon_radial_fr local.get $dr f32.mul f32.sub
      f32.mul
      local.set $qb
      local.get $vx local.get $vx f32.mul local.get $vy local.get $vy f32.mul f32.add
      global.get $icon_radial_fr global.get $icon_radial_fr f32.mul f32.sub
      local.set $qc
      local.get $qb local.get $qb f32.mul
      f32.const 4 local.get $qa f32.mul local.get $qc f32.mul
      f32.sub
      local.set $disc
      local.get $disc f32.const 0 f32.lt
      if
        f32.const 0 local.set $t
      else
        local.get $qa f32.abs f32.const 0.000001 f32.le
        if
          local.get $qb f32.abs f32.const 0.000001 f32.le
          if
            f32.const 0 local.set $t
          else
            local.get $qc f32.neg local.get $qb f32.div local.set $t
          end
        else
          local.get $qb f32.neg local.get $disc f32.sqrt f32.sub
          f32.const 2 local.get $qa f32.mul
          f32.div
          local.set $root1
          local.get $qb f32.neg local.get $disc f32.sqrt f32.add
          f32.const 2 local.get $qa f32.mul
          f32.div
          local.set $root2
          local.get $root1 f32.const 0 f32.ge
          local.get $root2 f32.const 0 f32.ge
          i32.and
          if
            local.get $root1 local.get $root2 call $min_f32 local.set $t
          else
            local.get $root1 f32.const 0 f32.ge
            if
              local.get $root1 local.set $t
            else
              local.get $root2 f32.const 0 f32.ge
              if
                local.get $root2 local.set $t
              else
                f32.const 0 local.set $t
              end
            end
          end
        end
      end
      local.get $t call $icon_gradient_alpha_at local.set $paint_alpha
      local.get $t i32.const 4 call $icon_gradient_component_at f32.nearest i32.trunc_f32_u local.set $r
      local.get $t i32.const 8 call $icon_gradient_component_at f32.nearest i32.trunc_f32_u local.set $g
      local.get $t i32.const 12 call $icon_gradient_component_at f32.nearest i32.trunc_f32_u local.set $b
    end
    local.get $coverage
    local.get $paint_alpha
    f32.mul
    f32.const 1
    call $min_f32
    f32.const 255
    f32.mul
    f32.nearest
    i32.trunc_f32_u
    local.set $value
    local.get $alpha
    local.get $y
    local.get $width
    i32.mul
    i32.add
    local.get $x
    i32.add
    local.set $addr
    local.get $value
    local.get $addr
    i32.load8_u
    i32.gt_u
    if
      local.get $addr
      local.get $value
      i32.store8
      global.get $icon_rgba_out
      if
        global.get $icon_rgba_out
        local.get $y
        local.get $width
        i32.mul
        local.get $x
        i32.add
        i32.const 4
        i32.mul
        i32.add
        local.set $rgba_addr
        local.get $rgba_addr local.get $r i32.const 0 i32.const 255 call $clamp_i32 i32.store8
        local.get $rgba_addr i32.const 1 i32.add local.get $g i32.const 0 i32.const 255 call $clamp_i32 i32.store8
        local.get $rgba_addr i32.const 2 i32.add local.get $b i32.const 0 i32.const 255 call $clamp_i32 i32.store8
        local.get $rgba_addr i32.const 3 i32.add local.get $value i32.store8
      end
    end)
  (func $icon_render_line_alpha (param $alpha i32) (param $width i32) (param $height i32) (param $bx f32) (param $by f32) (param $bw f32) (param $bh f32) (param $x0n f32) (param $y0n f32) (param $x1n f32) (param $y1n f32) (param $stroke_n f32) (param $cap f32)
    (local $x0 f32) (local $y0 f32) (local $x1 f32) (local $y1 f32)
    (local $dx f32) (local $dy f32) (local $len2 f32) (local $seg_len f32) (local $radius f32) (local $cap_lo f32) (local $cap_hi f32) (local $cap_extend f32)
    (local $minx i32) (local $maxx i32) (local $miny i32) (local $maxy i32) (local $px i32) (local $py i32)
    (local $pcx f32) (local $pcy f32) (local $t f32) (local $cx f32) (local $cy f32) (local $dist f32) (local $coverage f32)
    local.get $bx local.get $bw local.get $x0n f32.mul f32.add local.set $x0
    local.get $by local.get $bh local.get $y0n f32.mul f32.add local.set $y0
    local.get $bx local.get $bw local.get $x1n f32.mul f32.add local.set $x1
    local.get $by local.get $bh local.get $y1n f32.mul f32.add local.set $y1
    local.get $x1 local.get $x0 f32.sub local.set $dx
    local.get $y1 local.get $y0 f32.sub local.set $dy
    local.get $dx local.get $dx f32.mul local.get $dy local.get $dy f32.mul f32.add local.set $len2
    local.get $len2 f32.sqrt local.set $seg_len
    local.get $bw
    local.get $bh
    call $min_f32
    local.get $stroke_n
    f32.mul
    f32.const 0.5
    f32.mul
    local.set $radius
    local.get $radius
    f32.const 0
    f32.le
    local.get $len2
    f32.const 0
    f32.le
    i32.or
    if
      return
    end
    local.get $x0 local.get $x1 call $min_f32 local.get $radius f32.sub f32.const 1 f32.sub f32.floor i32.trunc_f32_s local.set $minx
    local.get $x0 local.get $x1 call $max_f32 local.get $radius f32.add f32.const 1 f32.add f32.ceil i32.trunc_f32_s local.set $maxx
    local.get $y0 local.get $y1 call $min_f32 local.get $radius f32.sub f32.const 1 f32.sub f32.floor i32.trunc_f32_s local.set $miny
    local.get $y0 local.get $y1 call $max_f32 local.get $radius f32.add f32.const 1 f32.add f32.ceil i32.trunc_f32_s local.set $maxy
    local.get $minx i32.const 0 i32.lt_s if i32.const 0 local.set $minx end
    local.get $miny i32.const 0 i32.lt_s if i32.const 0 local.set $miny end
    local.get $maxx local.get $width i32.gt_s if local.get $width local.set $maxx end
    local.get $maxy local.get $height i32.gt_s if local.get $height local.set $maxy end
    local.get $miny local.set $py
    block $done_y
      loop $loop_y
        local.get $py local.get $maxy i32.ge_s br_if $done_y
        local.get $minx local.set $px
        block $done_x
          loop $loop_x
            local.get $px local.get $maxx i32.ge_s br_if $done_x
            local.get $px f32.convert_i32_s f32.const 0.5 f32.add local.set $pcx
            local.get $py f32.convert_i32_s f32.const 0.5 f32.add local.set $pcy
            local.get $pcx local.get $x0 f32.sub local.get $dx f32.mul
            local.get $pcy local.get $y0 f32.sub local.get $dy f32.mul
            f32.add
            local.get $len2
            f32.div
            local.set $t
            f32.const 0
            local.set $cap_lo
            f32.const 1
            local.set $cap_hi
            local.get $cap f32.const 2 f32.eq
            if
              local.get $radius local.get $seg_len f32.div local.set $cap_extend
              local.get $cap_extend f32.neg local.set $cap_lo
              f32.const 1 local.get $cap_extend f32.add local.set $cap_hi
            end
            local.get $cap f32.const 1 f32.ne
            local.get $t local.get $cap_lo f32.lt
            local.get $t local.get $cap_hi f32.gt
            i32.or
            i32.and
            if
              f32.const 0
              local.set $coverage
              local.get $alpha local.get $width local.get $height local.get $px local.get $py local.get $coverage call $icon_alpha_store_max
              local.get $px i32.const 1 i32.add local.set $px
              br $loop_x
            end
            local.get $t
            local.get $cap_lo
            call $max_f32
            local.get $cap_hi
            call $min_f32
            local.set $t
            local.get $x0 local.get $dx local.get $t f32.mul f32.add local.set $cx
            local.get $y0 local.get $dy local.get $t f32.mul f32.add local.set $cy
            local.get $pcx local.get $cx f32.sub local.get $pcx local.get $cx f32.sub f32.mul
            local.get $pcy local.get $cy f32.sub local.get $pcy local.get $cy f32.sub f32.mul
            f32.add
            f32.sqrt
            local.set $dist
            local.get $radius
            f32.const 1
            f32.add
            local.get $dist
            f32.sub
            f32.const 0
            call $max_f32
            f32.const 1
            call $min_f32
            local.set $coverage
            local.get $alpha local.get $width local.get $height local.get $px local.get $py local.get $coverage call $icon_alpha_store_max
            local.get $px i32.const 1 i32.add local.set $px
            br $loop_x
          end
        end
        local.get $py i32.const 1 i32.add local.set $py
        br $loop_y
      end
    end)

  (func $icon_render_ellipse_alpha (param $alpha i32) (param $width i32) (param $height i32) (param $bx f32) (param $by f32) (param $bw f32) (param $bh f32) (param $cxn f32) (param $cyn f32) (param $rxn f32) (param $ryn f32) (param $stroke_n f32) (param $filled i32)
    (local $cx f32) (local $cy f32) (local $rx f32) (local $ry f32) (local $radius f32) (local $minr f32)
    (local $minx i32) (local $maxx i32) (local $miny i32) (local $maxy i32) (local $px i32) (local $py i32)
    (local $pcx f32) (local $pcy f32) (local $dx f32) (local $dy f32) (local $dist f32) (local $coverage f32)
    local.get $bx local.get $bw local.get $cxn f32.mul f32.add local.set $cx
    local.get $by local.get $bh local.get $cyn f32.mul f32.add local.set $cy
    local.get $bw local.get $rxn f32.mul f32.abs local.set $rx
    local.get $bh local.get $ryn f32.mul f32.abs local.set $ry
    local.get $bw local.get $bh call $min_f32 local.get $stroke_n f32.mul f32.const 0.5 f32.mul local.set $radius
    local.get $rx local.get $ry call $min_f32 local.set $minr
    local.get $rx f32.const 0 f32.le
    local.get $ry f32.const 0 f32.le
    i32.or
    if
      return
    end
    local.get $cx local.get $rx f32.sub local.get $radius f32.sub f32.const 1 f32.sub f32.floor i32.trunc_f32_s local.set $minx
    local.get $cx local.get $rx f32.add local.get $radius f32.add f32.const 1 f32.add f32.ceil i32.trunc_f32_s local.set $maxx
    local.get $cy local.get $ry f32.sub local.get $radius f32.sub f32.const 1 f32.sub f32.floor i32.trunc_f32_s local.set $miny
    local.get $cy local.get $ry f32.add local.get $radius f32.add f32.const 1 f32.add f32.ceil i32.trunc_f32_s local.set $maxy
    local.get $minx i32.const 0 i32.lt_s if i32.const 0 local.set $minx end
    local.get $miny i32.const 0 i32.lt_s if i32.const 0 local.set $miny end
    local.get $maxx local.get $width i32.gt_s if local.get $width local.set $maxx end
    local.get $maxy local.get $height i32.gt_s if local.get $height local.set $maxy end
    local.get $miny local.set $py
    block $done_y
      loop $loop_y
        local.get $py local.get $maxy i32.ge_s br_if $done_y
        local.get $minx local.set $px
        block $done_x
          loop $loop_x
            local.get $px local.get $maxx i32.ge_s br_if $done_x
            local.get $px f32.convert_i32_s f32.const 0.5 f32.add local.set $pcx
            local.get $py f32.convert_i32_s f32.const 0.5 f32.add local.set $pcy
            local.get $pcx local.get $cx f32.sub local.get $rx f32.div local.set $dx
            local.get $pcy local.get $cy f32.sub local.get $ry f32.div local.set $dy
            local.get $dx local.get $dx f32.mul local.get $dy local.get $dy f32.mul f32.add f32.sqrt local.set $dist
            local.get $filled
            if
              f32.const 1
              local.get $dist
              f32.sub
              local.get $minr
              f32.mul
              f32.const 1
              f32.add
              f32.const 0
              call $max_f32
              f32.const 1
              call $min_f32
              local.set $coverage
            else
              local.get $radius
              f32.const 1
              f32.add
              local.get $dist
              f32.const 1
              f32.sub
              f32.abs
              local.get $minr
              f32.mul
              f32.sub
              f32.const 0
              call $max_f32
              f32.const 1
              call $min_f32
              local.set $coverage
            end
            local.get $alpha local.get $width local.get $height local.get $px local.get $py local.get $coverage call $icon_alpha_store_max
            local.get $px i32.const 1 i32.add local.set $px
            br $loop_x
          end
        end
        local.get $py i32.const 1 i32.add local.set $py
        br $loop_y
      end
    end)

  (func $icon_render_round_rect_alpha (param $alpha i32) (param $width i32) (param $height i32) (param $bx f32) (param $by f32) (param $bw f32) (param $bh f32) (param $xn f32) (param $yn f32) (param $wn f32) (param $hn f32) (param $rn f32) (param $stroke_n f32) (param $filled i32)
    (local $x0 f32) (local $y0 f32) (local $rw f32) (local $rh f32) (local $rr f32) (local $radius f32)
    (local $cx f32) (local $cy f32) (local $half_w f32) (local $half_h f32)
    (local $minx i32) (local $maxx i32) (local $miny i32) (local $maxy i32) (local $px i32) (local $py i32)
    (local $pcx f32) (local $pcy f32) (local $qx f32) (local $qy f32) (local $ox f32) (local $oy f32) (local $outside f32) (local $inside f32) (local $sd f32) (local $coverage f32)
    local.get $bx local.get $bw local.get $xn f32.mul f32.add local.set $x0
    local.get $by local.get $bh local.get $yn f32.mul f32.add local.set $y0
    local.get $bw local.get $wn f32.mul f32.abs local.set $rw
    local.get $bh local.get $hn f32.mul f32.abs local.set $rh
    local.get $bw local.get $bh call $min_f32 local.get $rn f32.mul f32.abs local.set $rr
    local.get $bw local.get $bh call $min_f32 local.get $stroke_n f32.mul f32.const 0.5 f32.mul local.set $radius
    local.get $rw f32.const 0 f32.le
    local.get $rh f32.const 0 f32.le
    i32.or
    if
      return
    end
    local.get $rw f32.const 0.5 f32.mul local.set $half_w
    local.get $rh f32.const 0.5 f32.mul local.set $half_h
    local.get $rr local.get $half_w call $min_f32 local.get $half_h call $min_f32 local.set $rr
    local.get $x0 local.get $half_w f32.add local.set $cx
    local.get $y0 local.get $half_h f32.add local.set $cy
    local.get $x0 local.get $radius f32.sub f32.const 1 f32.sub f32.floor i32.trunc_f32_s local.set $minx
    local.get $x0 local.get $rw f32.add local.get $radius f32.add f32.const 1 f32.add f32.ceil i32.trunc_f32_s local.set $maxx
    local.get $y0 local.get $radius f32.sub f32.const 1 f32.sub f32.floor i32.trunc_f32_s local.set $miny
    local.get $y0 local.get $rh f32.add local.get $radius f32.add f32.const 1 f32.add f32.ceil i32.trunc_f32_s local.set $maxy
    local.get $minx i32.const 0 i32.lt_s if i32.const 0 local.set $minx end
    local.get $miny i32.const 0 i32.lt_s if i32.const 0 local.set $miny end
    local.get $maxx local.get $width i32.gt_s if local.get $width local.set $maxx end
    local.get $maxy local.get $height i32.gt_s if local.get $height local.set $maxy end
    local.get $miny local.set $py
    block $done_y
      loop $loop_y
        local.get $py local.get $maxy i32.ge_s br_if $done_y
        local.get $minx local.set $px
        block $done_x
          loop $loop_x
            local.get $px local.get $maxx i32.ge_s br_if $done_x
            local.get $px f32.convert_i32_s f32.const 0.5 f32.add local.set $pcx
            local.get $py f32.convert_i32_s f32.const 0.5 f32.add local.set $pcy
            local.get $pcx local.get $cx f32.sub f32.abs local.get $half_w f32.sub local.get $rr f32.add local.set $qx
            local.get $pcy local.get $cy f32.sub f32.abs local.get $half_h f32.sub local.get $rr f32.add local.set $qy
            local.get $qx f32.const 0 call $max_f32 local.set $ox
            local.get $qy f32.const 0 call $max_f32 local.set $oy
            local.get $ox local.get $ox f32.mul local.get $oy local.get $oy f32.mul f32.add f32.sqrt local.set $outside
            local.get $qx local.get $qy call $max_f32 f32.const 0 call $min_f32 local.set $inside
            local.get $outside local.get $inside f32.add local.get $rr f32.sub local.set $sd
            local.get $filled
            if
              f32.const 1
              local.get $sd
              f32.sub
              f32.const 0
              call $max_f32
              f32.const 1
              call $min_f32
              local.set $coverage
            else
              local.get $radius
              f32.const 1
              f32.add
              local.get $sd
              f32.abs
              f32.sub
              f32.const 0
              call $max_f32
              f32.const 1
              call $min_f32
              local.set $coverage
            end
            local.get $alpha local.get $width local.get $height local.get $px local.get $py local.get $coverage call $icon_alpha_store_max
            local.get $px i32.const 1 i32.add local.set $px
            br $loop_x
          end
        end
        local.get $py i32.const 1 i32.add local.set $py
        br $loop_y
      end
    end)

  (func $icon_alpha_count (param $alpha i32) (param $width i32) (param $height i32) (result i32)
    (local $i i32) (local $end i32) (local $count i32)
    local.get $width local.get $height i32.mul local.set $end
    block $done
      loop $loop
        local.get $i local.get $end i32.ge_u br_if $done
        local.get $alpha local.get $i i32.add i32.load8_u
        if
          local.get $count i32.const 1 i32.add local.set $count
        end
        local.get $i i32.const 1 i32.add local.set $i
        br $loop
      end
    end
    local.get $count)
  (func $icon_fill_point_append (param $points i32) (param $count i32) (param $x f32) (param $y f32) (result i32)
    local.get $count
    i32.const 256
    i32.ge_u
    if
      local.get $count
      return
    end
    local.get $points
    local.get $count
    i32.const 8
    i32.mul
    i32.add
    local.get $x
    f32.store
    local.get $points
    local.get $count
    i32.const 8
    i32.mul
    i32.add
    i32.const 4
    i32.add
    local.get $y
    f32.store
    local.get $count
    i32.const 1
    i32.add)

  (func $icon_fill_break_append (param $points i32) (param $count i32) (result i32)
    local.get $count
    i32.const 256
    i32.ge_u
    if
      local.get $count
      return
    end
    local.get $points
    local.get $count
    i32.const 8
    i32.mul
    i32.add
    f32.const -1000000
    f32.store
    local.get $points
    local.get $count
    i32.const 8
    i32.mul
    i32.add
    i32.const 4
    i32.add
    f32.const -1000000
    f32.store
    local.get $count
    i32.const 1
    i32.add)

  (func $icon_arc_vector_x (param $center_x f32) (param $cphi f32) (param $sphi f32) (param $rx f32) (param $ry f32) (param $vx f32) (param $vy f32) (result f32)
    (local $len f32) (local $raw_dx f32) (local $raw_dy f32)
    local.get $vx local.get $vx f32.mul local.get $vy local.get $vy f32.mul f32.add f32.sqrt f32.const 0.000001 call $max_f32 local.set $len
    local.get $rx local.get $vx f32.mul local.get $len f32.div local.set $raw_dx
    local.get $ry local.get $vy f32.mul local.get $len f32.div local.set $raw_dy
    local.get $center_x local.get $cphi local.get $raw_dx f32.mul local.get $sphi local.get $raw_dy f32.mul f32.sub f32.add)

  (func $icon_arc_vector_y (param $center_y f32) (param $cphi f32) (param $sphi f32) (param $rx f32) (param $ry f32) (param $vx f32) (param $vy f32) (result f32)
    (local $len f32) (local $raw_dx f32) (local $raw_dy f32)
    local.get $vx local.get $vx f32.mul local.get $vy local.get $vy f32.mul f32.add f32.sqrt f32.const 0.000001 call $max_f32 local.set $len
    local.get $rx local.get $vx f32.mul local.get $len f32.div local.set $raw_dx
    local.get $ry local.get $vy f32.mul local.get $len f32.div local.set $raw_dy
    local.get $center_y local.get $sphi local.get $raw_dx f32.mul local.get $cphi local.get $raw_dy f32.mul f32.add f32.add)

  (func $icon_fill_arc_vector_append (param $points i32) (param $count i32) (param $center_x f32) (param $center_y f32) (param $cphi f32) (param $sphi f32) (param $rx f32) (param $ry f32) (param $vx f32) (param $vy f32) (result i32)
    local.get $points
    local.get $count
    local.get $center_x local.get $cphi local.get $sphi local.get $rx local.get $ry local.get $vx local.get $vy call $icon_arc_vector_x
    local.get $center_y local.get $cphi local.get $sphi local.get $rx local.get $ry local.get $vx local.get $vy call $icon_arc_vector_y
    call $icon_fill_point_append)

  (func $icon_fill_quad_append (param $points i32) (param $count i32) (param $x0 f32) (param $y0 f32) (param $cx f32) (param $cy f32) (param $x1 f32) (param $y1 f32) (result i32)
    (local $step i32) (local $t f32) (local $mt f32) (local $px f32) (local $py f32)
    i32.const 1 local.set $step
    block $done
      loop $loop
        local.get $step i32.const 16 i32.gt_u br_if $done
        local.get $step f32.convert_i32_u f32.const 16 f32.div local.set $t
        f32.const 1 local.get $t f32.sub local.set $mt
        local.get $mt local.get $mt f32.mul local.get $x0 f32.mul
        f32.const 2 local.get $mt f32.mul local.get $t f32.mul local.get $cx f32.mul f32.add
        local.get $t local.get $t f32.mul local.get $x1 f32.mul f32.add
        local.set $px
        local.get $mt local.get $mt f32.mul local.get $y0 f32.mul
        f32.const 2 local.get $mt f32.mul local.get $t f32.mul local.get $cy f32.mul f32.add
        local.get $t local.get $t f32.mul local.get $y1 f32.mul f32.add
        local.set $py
        local.get $points local.get $count local.get $px local.get $py call $icon_fill_point_append local.set $count
        local.get $step i32.const 1 i32.add local.set $step
        br $loop
      end
    end
    local.get $count)

  (func $icon_fill_cubic_append (param $points i32) (param $count i32) (param $x0 f32) (param $y0 f32) (param $c0x f32) (param $c0y f32) (param $c1x f32) (param $c1y f32) (param $x1 f32) (param $y1 f32) (result i32)
    (local $step i32) (local $t f32) (local $mt f32) (local $px f32) (local $py f32)
    i32.const 1 local.set $step
    block $done
      loop $loop
        local.get $step i32.const 20 i32.gt_u br_if $done
        local.get $step f32.convert_i32_u f32.const 20 f32.div local.set $t
        f32.const 1 local.get $t f32.sub local.set $mt
        local.get $mt local.get $mt f32.mul local.get $mt f32.mul local.get $x0 f32.mul
        f32.const 3 local.get $mt f32.mul local.get $mt f32.mul local.get $t f32.mul local.get $c0x f32.mul f32.add
        f32.const 3 local.get $mt f32.mul local.get $t f32.mul local.get $t f32.mul local.get $c1x f32.mul f32.add
        local.get $t local.get $t f32.mul local.get $t f32.mul local.get $x1 f32.mul f32.add
        local.set $px
        local.get $mt local.get $mt f32.mul local.get $mt f32.mul local.get $y0 f32.mul
        f32.const 3 local.get $mt f32.mul local.get $mt f32.mul local.get $t f32.mul local.get $c0y f32.mul f32.add
        f32.const 3 local.get $mt f32.mul local.get $t f32.mul local.get $t f32.mul local.get $c1y f32.mul f32.add
        local.get $t local.get $t f32.mul local.get $t f32.mul local.get $y1 f32.mul f32.add
        local.set $py
        local.get $points local.get $count local.get $px local.get $py call $icon_fill_point_append local.set $count
        local.get $step i32.const 1 i32.add local.set $step
        br $loop
      end
    end
    local.get $count)

  (func $icon_fill_arc_append (param $points i32) (param $count i32) (param $x0 f32) (param $y0 f32) (param $rx_in f32) (param $ry_in f32) (param $rot f32) (param $large f32) (param $sweep f32) (param $x1 f32) (param $y1 f32) (result i32)
    (local $rx f32) (local $ry f32) (local $dx f32) (local $dy f32) (local $raw_dx f32) (local $raw_dy f32) (local $phi f32) (local $cphi f32) (local $sphi f32) (local $scale f32)
    (local $num f32) (local $den f32) (local $coef f32) (local $cxp f32) (local $cyp f32) (local $center_x f32) (local $center_y f32)
    (local $v0x f32) (local $v0y f32) (local $v1x f32) (local $v1y f32) (local $mx f32) (local $my f32) (local $mlen f32) (local $mid_x f32) (local $mid_y f32)
    (local $q0x f32) (local $q0y f32) (local $q1x f32) (local $q1y f32) (local $q0len f32) (local $q1len f32) (local $p0x f32) (local $p0y f32) (local $p1x f32) (local $p1y f32)
    (local $seg_x f32) (local $seg_y f32)
    local.get $rx_in f32.abs f32.const 0.00001 f32.le
    local.get $ry_in f32.abs f32.const 0.00001 f32.le i32.or
    if
      local.get $points local.get $count local.get $x1 local.get $y1 call $icon_fill_point_append
      return
    end
    local.get $rx_in f32.abs local.set $rx
    local.get $ry_in f32.abs local.set $ry
    local.get $rot f32.const 0.017453292 f32.mul local.set $phi
    local.get $phi call $cos_rad local.set $cphi
    local.get $phi call $sin_rad local.set $sphi
    local.get $x0 local.get $x1 f32.sub f32.const 0.5 f32.mul local.set $raw_dx
    local.get $y0 local.get $y1 f32.sub f32.const 0.5 f32.mul local.set $raw_dy
    local.get $cphi local.get $raw_dx f32.mul local.get $sphi local.get $raw_dy f32.mul f32.add local.set $dx
    local.get $sphi f32.neg local.get $raw_dx f32.mul local.get $cphi local.get $raw_dy f32.mul f32.add local.set $dy
    local.get $dx local.get $dx f32.mul local.get $rx local.get $rx f32.mul f32.div
    local.get $dy local.get $dy f32.mul local.get $ry local.get $ry f32.mul f32.div
    f32.add
    local.tee $scale
    f32.const 1
    f32.gt
    if
      local.get $scale f32.sqrt local.set $scale
      local.get $rx local.get $scale f32.mul local.set $rx
      local.get $ry local.get $scale f32.mul local.set $ry
    end
    local.get $rx local.get $rx f32.mul local.get $ry local.get $ry f32.mul f32.mul
    local.get $rx local.get $rx f32.mul local.get $dy local.get $dy f32.mul f32.mul f32.sub
    local.get $ry local.get $ry f32.mul local.get $dx local.get $dx f32.mul f32.mul f32.sub
    local.set $num
    local.get $rx local.get $rx f32.mul local.get $dy local.get $dy f32.mul f32.mul
    local.get $ry local.get $ry f32.mul local.get $dx local.get $dx f32.mul f32.mul f32.add
    local.set $den
    local.get $num f32.const 0 call $max_f32
    local.get $den f32.const 0.000001 call $max_f32
    f32.div
    f32.sqrt
    local.set $coef
    local.get $large local.get $sweep f32.eq
    if
      local.get $coef f32.neg local.set $coef
    end
    local.get $coef local.get $rx f32.mul local.get $dy f32.mul local.get $ry f32.div local.set $cxp
    local.get $coef f32.neg local.get $ry f32.mul local.get $dx f32.mul local.get $rx f32.div local.set $cyp
    local.get $x0 local.get $x1 f32.add f32.const 0.5 f32.mul
    local.get $cphi local.get $cxp f32.mul local.get $sphi local.get $cyp f32.mul f32.sub
    f32.add local.set $center_x
    local.get $y0 local.get $y1 f32.add f32.const 0.5 f32.mul
    local.get $sphi local.get $cxp f32.mul local.get $cphi local.get $cyp f32.mul f32.add
    f32.add local.set $center_y
    local.get $dx local.get $cxp f32.sub local.get $rx f32.div local.set $v0x
    local.get $dy local.get $cyp f32.sub local.get $ry f32.div local.set $v0y
    local.get $dx f32.neg local.get $cxp f32.sub local.get $rx f32.div local.set $v1x
    local.get $dy f32.neg local.get $cyp f32.sub local.get $ry f32.div local.set $v1y
    local.get $v0x local.get $v1x f32.add local.set $mx
    local.get $v0y local.get $v1y f32.add local.set $my
    local.get $large f32.const 0 f32.ne
    if
      local.get $mx f32.neg local.set $mx
      local.get $my f32.neg local.set $my
    end
    local.get $mx local.get $mx f32.mul local.get $my local.get $my f32.mul f32.add f32.sqrt local.set $mlen
    local.get $mlen f32.const 0.00001 f32.le
    if
      local.get $sweep f32.const 0 f32.ne
      if
        local.get $v0y f32.neg local.set $mx
        local.get $v0x local.set $my
      else
        local.get $v0y local.set $mx
        local.get $v0x f32.neg local.set $my
      end
      f32.const 1 local.set $mlen
    end
    local.get $rx local.get $mx f32.mul local.get $mlen f32.div local.set $raw_dx
    local.get $ry local.get $my f32.mul local.get $mlen f32.div local.set $raw_dy
    local.get $center_x local.get $cphi local.get $raw_dx f32.mul local.get $sphi local.get $raw_dy f32.mul f32.sub f32.add local.set $mid_x
    local.get $center_y local.get $sphi local.get $raw_dx f32.mul local.get $cphi local.get $raw_dy f32.mul f32.add f32.add local.set $mid_y
    local.get $v0x local.get $mx local.get $mlen f32.div f32.add local.set $q0x
    local.get $v0y local.get $my local.get $mlen f32.div f32.add local.set $q0y
    local.get $q0x local.get $q0x f32.mul local.get $q0y local.get $q0y f32.mul f32.add f32.sqrt local.tee $q0len f32.const 0.00001 f32.le
    if
      local.get $v0x local.set $q0x
      local.get $v0y local.set $q0y
      f32.const 1 local.set $q0len
    end
    local.get $mx local.get $mlen f32.div local.get $v1x f32.add local.set $q1x
    local.get $my local.get $mlen f32.div local.get $v1y f32.add local.set $q1y
    local.get $q1x local.get $q1x f32.mul local.get $q1y local.get $q1y f32.mul f32.add f32.sqrt local.tee $q1len f32.const 0.00001 f32.le
    if
      local.get $v1x local.set $q1x
      local.get $v1y local.set $q1y
      f32.const 1 local.set $q1len
    end
    local.get $points local.get $count local.get $center_x local.get $center_y local.get $cphi local.get $sphi local.get $rx local.get $ry local.get $v0x local.get $q0x f32.add local.get $v0y local.get $q0y f32.add call $icon_fill_arc_vector_append local.set $count
    local.get $points local.get $count local.get $center_x local.get $center_y local.get $cphi local.get $sphi local.get $rx local.get $ry local.get $q0x local.get $q0y call $icon_fill_arc_vector_append local.set $count
    local.get $points local.get $count local.get $center_x local.get $center_y local.get $cphi local.get $sphi local.get $rx local.get $ry local.get $q0x local.get $mx local.get $mlen f32.div f32.add local.get $q0y local.get $my local.get $mlen f32.div f32.add call $icon_fill_arc_vector_append local.set $count
    local.get $points local.get $count local.get $mid_x local.get $mid_y call $icon_fill_point_append local.set $count
    local.get $points local.get $count local.get $center_x local.get $center_y local.get $cphi local.get $sphi local.get $rx local.get $ry local.get $mx local.get $mlen f32.div local.get $q1x f32.add local.get $my local.get $mlen f32.div local.get $q1y f32.add call $icon_fill_arc_vector_append local.set $count
    local.get $points local.get $count local.get $center_x local.get $center_y local.get $cphi local.get $sphi local.get $rx local.get $ry local.get $q1x local.get $q1y call $icon_fill_arc_vector_append local.set $count
    local.get $points local.get $count local.get $center_x local.get $center_y local.get $cphi local.get $sphi local.get $rx local.get $ry local.get $q1x local.get $v1x f32.add local.get $q1y local.get $v1y f32.add call $icon_fill_arc_vector_append local.set $count
    local.get $points local.get $count local.get $x1 local.get $y1 call $icon_fill_point_append)

  (func $icon_fill_polygon_alpha (param $alpha i32) (param $width i32) (param $height i32) (param $bx f32) (param $by f32) (param $bw f32) (param $bh f32) (param $points i32) (param $count i32) (param $rule i32)
    (local $px i32) (local $py i32) (local $i i32) (local $j i32) (local $inside i32) (local $winding i32) (local $hit i32)
    (local $pnx f32) (local $pny f32) (local $xi f32) (local $yi f32) (local $xj f32) (local $yj f32) (local $xhit f32)
    (local $ex f32) (local $ey f32) (local $vx f32) (local $vy f32) (local $seg_len2 f32) (local $proj f32) (local $dist f32) (local $min_dist f32) (local $coverage f32)
    local.get $count
    i32.const 3
    i32.lt_u
    if
      return
    end
    i32.const 0
    local.set $py
    block $done_y
      loop $loop_y
        local.get $py
        local.get $height
        i32.ge_u
        br_if $done_y
        i32.const 0
        local.set $px
        block $done_x
          loop $loop_x
            local.get $px
            local.get $width
            i32.ge_u
            br_if $done_x
            local.get $px f32.convert_i32_u f32.const 0.5 f32.add local.get $bx f32.sub local.get $bw f32.div local.set $pnx
            local.get $py f32.convert_i32_u f32.const 0.5 f32.add local.get $by f32.sub local.get $bh f32.div local.set $pny
            i32.const 0
            local.set $inside
            i32.const 0
            local.set $winding
            f32.const 1000000
            local.set $min_dist
            i32.const 0
            local.set $i
            block $done_edges
              loop $loop_edges
                local.get $i
                local.get $count
                i32.ge_u
                br_if $done_edges
                local.get $points local.get $i i32.const 8 i32.mul i32.add f32.load local.set $xi
                local.get $points local.get $i i32.const 8 i32.mul i32.add i32.const 4 i32.add f32.load local.set $yi
                local.get $xi
                f32.const -999999
                f32.lt
                if
                  local.get $i
                  i32.const 1
                  i32.add
                  local.set $i
                  br $loop_edges
                end
                local.get $i
                i32.eqz
                if
                  local.get $i local.set $j
                  block $contour_done
                    loop $contour_loop
                      local.get $j i32.const 1 i32.add local.get $count i32.ge_u br_if $contour_done
                      local.get $points local.get $j i32.const 1 i32.add i32.const 8 i32.mul i32.add f32.load local.set $xj
                      local.get $xj f32.const -999999 f32.lt br_if $contour_done
                      local.get $j i32.const 1 i32.add local.set $j
                      br $contour_loop
                    end
                  end
                else
                  local.get $points local.get $i i32.const 1 i32.sub i32.const 8 i32.mul i32.add f32.load local.set $xj
                  local.get $xj
                  f32.const -999999
                  f32.lt
                  if
                    local.get $i local.set $j
                    block $contour_done
                      loop $contour_loop
                        local.get $j i32.const 1 i32.add local.get $count i32.ge_u br_if $contour_done
                        local.get $points local.get $j i32.const 1 i32.add i32.const 8 i32.mul i32.add f32.load local.set $xj
                        local.get $xj f32.const -999999 f32.lt br_if $contour_done
                        local.get $j i32.const 1 i32.add local.set $j
                        br $contour_loop
                      end
                    end
                  else
                    local.get $i i32.const 1 i32.sub local.set $j
                  end
                end
                local.get $points local.get $j i32.const 8 i32.mul i32.add f32.load local.set $xj
                local.get $points local.get $j i32.const 8 i32.mul i32.add i32.const 4 i32.add f32.load local.set $yj
                local.get $xj local.get $xi f32.sub local.set $ex
                local.get $yj local.get $yi f32.sub local.set $ey
                local.get $ex local.get $ex f32.mul local.get $ey local.get $ey f32.mul f32.add local.set $seg_len2
                local.get $seg_len2 f32.const 0.000001 f32.gt
                if
                  local.get $pnx local.get $xi f32.sub local.set $vx
                  local.get $pny local.get $yi f32.sub local.set $vy
                  local.get $vx local.get $ex f32.mul local.get $vy local.get $ey f32.mul f32.add local.get $seg_len2 f32.div f32.const 0 f32.const 1 call $clamp_f32 local.set $proj
                  local.get $xi local.get $ex local.get $proj f32.mul f32.add local.set $xhit
                  local.get $yi local.get $ey local.get $proj f32.mul f32.add local.set $dist
                  local.get $pnx local.get $xhit f32.sub local.get $pnx local.get $xhit f32.sub f32.mul
                  local.get $pny local.get $dist f32.sub local.get $pny local.get $dist f32.sub f32.mul
                  f32.add
                  f32.sqrt
                  local.get $min_dist
                  call $min_f32
                  local.set $min_dist
                end
                local.get $yi local.get $pny f32.le
                local.get $yj local.get $pny f32.gt
                i32.and
                if
                  local.get $xj local.get $xi f32.sub
                  local.get $pny local.get $yi f32.sub
                  f32.mul
                  local.get $yj local.get $yi f32.sub
                  f32.div
                  local.get $xi f32.add
                  local.set $xhit
                  local.get $pnx
                  local.get $xhit
                  f32.lt
                  if
                    local.get $rule
                    if
                      local.get $inside
                      i32.const 1
                      i32.xor
                      local.set $inside
                    else
                      local.get $winding
                      i32.const 1
                      i32.add
                      local.set $winding
                    end
                  end
                end
                local.get $yj local.get $pny f32.le
                local.get $yi local.get $pny f32.gt
                i32.and
                if
                  local.get $xj local.get $xi f32.sub
                  local.get $pny local.get $yi f32.sub
                  f32.mul
                  local.get $yj local.get $yi f32.sub
                  f32.div
                  local.get $xi f32.add
                  local.set $xhit
                  local.get $pnx
                  local.get $xhit
                  f32.lt
                  if
                    local.get $rule
                    if
                      local.get $inside
                      i32.const 1
                      i32.xor
                      local.set $inside
                    else
                      local.get $winding
                      i32.const 1
                      i32.sub
                      local.set $winding
                    end
                  end
                end
                local.get $i
                i32.const 1
                i32.add
                local.set $i
                br $loop_edges
              end
            end
            local.get $rule
            if (result i32)
              local.get $inside
            else
              local.get $winding
              i32.const 0
              i32.ne
            end
            local.set $hit
            local.get $hit
            if
              local.get $alpha local.get $width local.get $height local.get $px local.get $py f32.const 1 call $icon_alpha_store_max
            else
              local.get $rule
              i32.eqz
              if
              local.get $min_dist local.get $bw local.get $bh call $min_f32 f32.mul local.set $dist
              local.get $dist f32.const 1 f32.lt
              if
                f32.const 1 local.get $dist f32.sub f32.const 0 f32.const 1 call $clamp_f32 local.set $coverage
                local.get $alpha local.get $width local.get $height local.get $px local.get $py local.get $coverage call $icon_alpha_store_max
              end
              end
            end
            local.get $px
            i32.const 1
            i32.add
            local.set $px
            br $loop_x
          end
        end
        local.get $py
        i32.const 1
        i32.add
        local.set $py
        br $loop_y
      end
    end)
  (func $icon_render_round_cap_alpha (param $alpha i32) (param $width i32) (param $height i32) (param $bx f32) (param $by f32) (param $bw f32) (param $bh f32) (param $xn f32) (param $yn f32) (param $stroke_n f32)
    (local $cx f32) (local $cy f32) (local $radius f32)
    (local $minx i32) (local $maxx i32) (local $miny i32) (local $maxy i32) (local $px i32) (local $py i32)
    (local $pcx f32) (local $pcy f32) (local $dist f32) (local $coverage f32)
    local.get $bx local.get $bw local.get $xn f32.mul f32.add local.set $cx
    local.get $by local.get $bh local.get $yn f32.mul f32.add local.set $cy
    local.get $bw local.get $bh call $min_f32 local.get $stroke_n f32.mul f32.const 0.5 f32.mul local.set $radius
    local.get $radius f32.const 0 f32.le if return end
    local.get $cx local.get $radius f32.sub f32.const 1 f32.sub f32.floor i32.trunc_f32_s local.set $minx
    local.get $cx local.get $radius f32.add f32.const 1 f32.add f32.ceil i32.trunc_f32_s local.set $maxx
    local.get $cy local.get $radius f32.sub f32.const 1 f32.sub f32.floor i32.trunc_f32_s local.set $miny
    local.get $cy local.get $radius f32.add f32.const 1 f32.add f32.ceil i32.trunc_f32_s local.set $maxy
    local.get $minx i32.const 0 i32.lt_s if i32.const 0 local.set $minx end
    local.get $miny i32.const 0 i32.lt_s if i32.const 0 local.set $miny end
    local.get $maxx local.get $width i32.gt_s if local.get $width local.set $maxx end
    local.get $maxy local.get $height i32.gt_s if local.get $height local.set $maxy end
    local.get $miny local.set $py
    block $done_y
      loop $loop_y
        local.get $py local.get $maxy i32.ge_s br_if $done_y
        local.get $minx local.set $px
        block $done_x
          loop $loop_x
            local.get $px local.get $maxx i32.ge_s br_if $done_x
            local.get $px f32.convert_i32_s f32.const 0.5 f32.add local.set $pcx
            local.get $py f32.convert_i32_s f32.const 0.5 f32.add local.set $pcy
            local.get $pcx local.get $cx f32.sub local.get $pcx local.get $cx f32.sub f32.mul
            local.get $pcy local.get $cy f32.sub local.get $pcy local.get $cy f32.sub f32.mul
            f32.add f32.sqrt local.set $dist
            local.get $radius f32.const 1 f32.add local.get $dist f32.sub f32.const 0 call $max_f32 f32.const 1 call $min_f32 local.set $coverage
            local.get $alpha local.get $width local.get $height local.get $px local.get $py local.get $coverage call $icon_alpha_store_max
            local.get $px i32.const 1 i32.add local.set $px
            br $loop_x
          end
        end
        local.get $py i32.const 1 i32.add local.set $py
        br $loop_y
      end
    end)

  (func $icon_render_square_cap_alpha (param $alpha i32) (param $width i32) (param $height i32) (param $bx f32) (param $by f32) (param $bw f32) (param $bh f32) (param $xn f32) (param $yn f32) (param $dx f32) (param $dy f32) (param $stroke_n f32)
    (local $len f32) (local $ux f32) (local $uy f32) (local $nx f32) (local $ny f32) (local $r f32)
    local.get $stroke_n f32.const 0.5 f32.mul local.set $r
    local.get $dx local.get $dx f32.mul local.get $dy local.get $dy f32.mul f32.add f32.sqrt local.set $len
    local.get $r f32.const 0 f32.le
    local.get $len f32.const 0.000001 f32.le
    i32.or
    if
      return
    end
    local.get $dx local.get $len f32.div local.set $ux
    local.get $dy local.get $len f32.div local.set $uy
    local.get $uy f32.neg local.set $nx
    local.get $ux local.set $ny
    i32.const 4100064 local.get $xn local.get $nx local.get $r f32.mul f32.add f32.store
    i32.const 4100068 local.get $yn local.get $ny local.get $r f32.mul f32.add f32.store
    i32.const 4100072 local.get $xn local.get $ux local.get $r f32.mul f32.add local.get $nx local.get $r f32.mul f32.add f32.store
    i32.const 4100076 local.get $yn local.get $uy local.get $r f32.mul f32.add local.get $ny local.get $r f32.mul f32.add f32.store
    i32.const 4100080 local.get $xn local.get $ux local.get $r f32.mul f32.add local.get $nx local.get $r f32.mul f32.sub f32.store
    i32.const 4100084 local.get $yn local.get $uy local.get $r f32.mul f32.add local.get $ny local.get $r f32.mul f32.sub f32.store
    i32.const 4100088 local.get $xn local.get $nx local.get $r f32.mul f32.sub f32.store
    i32.const 4100092 local.get $yn local.get $ny local.get $r f32.mul f32.sub f32.store
    local.get $alpha local.get $width local.get $height local.get $bx local.get $by local.get $bw local.get $bh i32.const 4100064 i32.const 4 i32.const 0 call $icon_fill_polygon_alpha)

  (func $icon_render_miter_join_alpha (param $alpha i32) (param $width i32) (param $height i32) (param $bx f32) (param $by f32) (param $bw f32) (param $bh f32) (param $x0 f32) (param $y0 f32) (param $jx f32) (param $jy f32) (param $x1 f32) (param $y1 f32) (param $stroke_n f32) (param $miter_limit f32)
    (local $d0x f32) (local $d0y f32) (local $d1x f32) (local $d1y f32) (local $len0 f32) (local $len1 f32)
    (local $n0x f32) (local $n0y f32) (local $n1x f32) (local $n1y f32) (local $mx f32) (local $my f32) (local $mlen f32) (local $dot f32) (local $radius f32) (local $limit f32)
    local.get $stroke_n f32.const 0.5 f32.mul local.set $radius
    local.get $radius f32.const 0 f32.le if return end
    local.get $jx local.get $x0 f32.sub local.set $d0x
    local.get $jy local.get $y0 f32.sub local.set $d0y
    local.get $x1 local.get $jx f32.sub local.set $d1x
    local.get $y1 local.get $jy f32.sub local.set $d1y
    local.get $d0x local.get $d0x f32.mul local.get $d0y local.get $d0y f32.mul f32.add f32.sqrt local.set $len0
    local.get $d1x local.get $d1x f32.mul local.get $d1y local.get $d1y f32.mul f32.add f32.sqrt local.set $len1
    local.get $len0 f32.const 0.000001 f32.le
    local.get $len1 f32.const 0.000001 f32.le
    i32.or
    if
      return
    end
    local.get $d0x local.get $len0 f32.div local.set $d0x
    local.get $d0y local.get $len0 f32.div local.set $d0y
    local.get $d1x local.get $len1 f32.div local.set $d1x
    local.get $d1y local.get $len1 f32.div local.set $d1y
    local.get $d0x local.get $d1y f32.mul local.get $d0y local.get $d1x f32.mul f32.sub
    f32.const 0
    f32.gt
    if
      local.get $d0y local.set $n0x
      local.get $d0x f32.neg local.set $n0y
      local.get $d1y local.set $n1x
      local.get $d1x f32.neg local.set $n1y
    else
      local.get $d0y f32.neg local.set $n0x
      local.get $d0x local.set $n0y
      local.get $d1y f32.neg local.set $n1x
      local.get $d1x local.set $n1y
    end
    local.get $n0x local.get $n1x f32.add local.set $mx
    local.get $n0y local.get $n1y f32.add local.set $my
    local.get $mx local.get $mx f32.mul local.get $my local.get $my f32.mul f32.add f32.sqrt local.set $mlen
    local.get $mlen f32.const 0.000001 f32.le
    if
      return
    end
    local.get $mx local.get $mlen f32.div local.set $mx
    local.get $my local.get $mlen f32.div local.set $my
    local.get $mx local.get $n1x f32.mul local.get $my local.get $n1y f32.mul f32.add f32.abs f32.const 0.000001 call $max_f32 local.set $dot
    local.get $radius local.get $dot f32.div local.set $mlen
    local.get $miter_limit f32.const 1 call $max_f32 local.get $radius f32.mul local.set $limit
    local.get $mlen local.get $limit f32.gt
    if
      return
    end
    i32.const 4100000 local.get $jx local.get $n0x local.get $radius f32.mul f32.add f32.store
    i32.const 4100004 local.get $jy local.get $n0y local.get $radius f32.mul f32.add f32.store
    i32.const 4100008 local.get $jx local.get $mx local.get $mlen f32.mul f32.add f32.store
    i32.const 4100012 local.get $jy local.get $my local.get $mlen f32.mul f32.add f32.store
    i32.const 4100016 local.get $jx local.get $n1x local.get $radius f32.mul f32.add f32.store
    i32.const 4100020 local.get $jy local.get $n1y local.get $radius f32.mul f32.add f32.store
    local.get $alpha local.get $width local.get $height local.get $bx local.get $by local.get $bw local.get $bh i32.const 4100000 i32.const 3 i32.const 0 call $icon_fill_polygon_alpha)

  (func $icon_render_bevel_join_alpha (param $alpha i32) (param $width i32) (param $height i32) (param $bx f32) (param $by f32) (param $bw f32) (param $bh f32) (param $x0 f32) (param $y0 f32) (param $jx f32) (param $jy f32) (param $x1 f32) (param $y1 f32) (param $stroke_n f32)
    (local $d0x f32) (local $d0y f32) (local $d1x f32) (local $d1y f32) (local $len0 f32) (local $len1 f32)
    (local $n0x f32) (local $n0y f32) (local $n1x f32) (local $n1y f32) (local $radius f32)
    local.get $stroke_n f32.const 0.5 f32.mul local.set $radius
    local.get $radius f32.const 0 f32.le if return end
    local.get $jx local.get $x0 f32.sub local.set $d0x
    local.get $jy local.get $y0 f32.sub local.set $d0y
    local.get $x1 local.get $jx f32.sub local.set $d1x
    local.get $y1 local.get $jy f32.sub local.set $d1y
    local.get $d0x local.get $d0x f32.mul local.get $d0y local.get $d0y f32.mul f32.add f32.sqrt local.set $len0
    local.get $d1x local.get $d1x f32.mul local.get $d1y local.get $d1y f32.mul f32.add f32.sqrt local.set $len1
    local.get $len0 f32.const 0.000001 f32.le
    local.get $len1 f32.const 0.000001 f32.le
    i32.or
    if
      return
    end
    local.get $d0x local.get $len0 f32.div local.set $d0x
    local.get $d0y local.get $len0 f32.div local.set $d0y
    local.get $d1x local.get $len1 f32.div local.set $d1x
    local.get $d1y local.get $len1 f32.div local.set $d1y
    local.get $d0x local.get $d1y f32.mul local.get $d0y local.get $d1x f32.mul f32.sub
    f32.const 0
    f32.gt
    if
      local.get $d0y local.set $n0x
      local.get $d0x f32.neg local.set $n0y
      local.get $d1y local.set $n1x
      local.get $d1x f32.neg local.set $n1y
    else
      local.get $d0y f32.neg local.set $n0x
      local.get $d0x local.set $n0y
      local.get $d1y f32.neg local.set $n1x
      local.get $d1x local.set $n1y
    end
    i32.const 4100032 local.get $jx local.get $n0x local.get $radius f32.mul f32.add f32.store
    i32.const 4100036 local.get $jy local.get $n0y local.get $radius f32.mul f32.add f32.store
    i32.const 4100040 local.get $jx f32.store
    i32.const 4100044 local.get $jy f32.store
    i32.const 4100048 local.get $jx local.get $n1x local.get $radius f32.mul f32.add f32.store
    i32.const 4100052 local.get $jy local.get $n1y local.get $radius f32.mul f32.add f32.store
    local.get $alpha local.get $width local.get $height local.get $bx local.get $by local.get $bw local.get $bh i32.const 4100032 i32.const 3 i32.const 0 call $icon_fill_polygon_alpha)

  (func $icon_render_quad_alpha (param $alpha i32) (param $width i32) (param $height i32) (param $bx f32) (param $by f32) (param $bw f32) (param $bh f32) (param $x0 f32) (param $y0 f32) (param $cx f32) (param $cy f32) (param $x1 f32) (param $y1 f32) (param $stroke f32) (param $cap f32)
    (local $step i32) (local $t f32) (local $mt f32) (local $px0 f32) (local $py0 f32) (local $px1 f32) (local $py1 f32)
    (local $line_cap f32)
    local.get $x0 local.set $px0
    local.get $y0 local.set $py0
    local.get $cap f32.const 0 f32.ne
    if
      f32.const 0 local.set $line_cap
    else
      local.get $cap local.set $line_cap
    end
    i32.const 1 local.set $step
    block $done
      loop $loop
        local.get $step i32.const 16 i32.gt_u br_if $done
        local.get $step f32.convert_i32_u f32.const 16 f32.div local.set $t
        f32.const 1 local.get $t f32.sub local.set $mt
        local.get $mt local.get $mt f32.mul local.get $x0 f32.mul
        f32.const 2 local.get $mt f32.mul local.get $t f32.mul local.get $cx f32.mul f32.add
        local.get $t local.get $t f32.mul local.get $x1 f32.mul f32.add
        local.set $px1
        local.get $mt local.get $mt f32.mul local.get $y0 f32.mul
        f32.const 2 local.get $mt f32.mul local.get $t f32.mul local.get $cy f32.mul f32.add
        local.get $t local.get $t f32.mul local.get $y1 f32.mul f32.add
        local.set $py1
        local.get $alpha local.get $width local.get $height local.get $bx local.get $by local.get $bw local.get $bh local.get $px0 local.get $py0 local.get $px1 local.get $py1 local.get $stroke local.get $line_cap call $icon_render_line_alpha
        local.get $px1 local.set $px0
        local.get $py1 local.set $py0
        local.get $step i32.const 1 i32.add local.set $step
        br $loop
      end
    end
    local.get $cap f32.const 1 f32.eq
    if
      local.get $alpha local.get $width local.get $height local.get $bx local.get $by local.get $bw local.get $bh local.get $x0 local.get $y0 local.get $stroke call $icon_render_round_cap_alpha
      local.get $alpha local.get $width local.get $height local.get $bx local.get $by local.get $bw local.get $bh local.get $x1 local.get $y1 local.get $stroke call $icon_render_round_cap_alpha
    else
      local.get $cap f32.const 2 f32.eq
      if
        local.get $alpha local.get $width local.get $height local.get $bx local.get $by local.get $bw local.get $bh local.get $x0 local.get $y0 local.get $x0 local.get $cx f32.sub local.get $y0 local.get $cy f32.sub local.get $stroke call $icon_render_square_cap_alpha
        local.get $alpha local.get $width local.get $height local.get $bx local.get $by local.get $bw local.get $bh local.get $x1 local.get $y1 local.get $x1 local.get $cx f32.sub local.get $y1 local.get $cy f32.sub local.get $stroke call $icon_render_square_cap_alpha
      end
    end)

  (func $icon_render_cubic_alpha (param $alpha i32) (param $width i32) (param $height i32) (param $bx f32) (param $by f32) (param $bw f32) (param $bh f32) (param $x0 f32) (param $y0 f32) (param $c0x f32) (param $c0y f32) (param $c1x f32) (param $c1y f32) (param $x1 f32) (param $y1 f32) (param $stroke f32) (param $cap f32)
    (local $step i32) (local $t f32) (local $mt f32) (local $px0 f32) (local $py0 f32) (local $px1 f32) (local $py1 f32)
    (local $line_cap f32)
    local.get $x0 local.set $px0
    local.get $y0 local.set $py0
    local.get $cap f32.const 0 f32.ne
    if
      f32.const 0 local.set $line_cap
    else
      local.get $cap local.set $line_cap
    end
    i32.const 1 local.set $step
    block $done
      loop $loop
        local.get $step i32.const 20 i32.gt_u br_if $done
        local.get $step f32.convert_i32_u f32.const 20 f32.div local.set $t
        f32.const 1 local.get $t f32.sub local.set $mt
        local.get $mt local.get $mt f32.mul local.get $mt f32.mul local.get $x0 f32.mul
        f32.const 3 local.get $mt f32.mul local.get $mt f32.mul local.get $t f32.mul local.get $c0x f32.mul f32.add
        f32.const 3 local.get $mt f32.mul local.get $t f32.mul local.get $t f32.mul local.get $c1x f32.mul f32.add
        local.get $t local.get $t f32.mul local.get $t f32.mul local.get $x1 f32.mul f32.add
        local.set $px1
        local.get $mt local.get $mt f32.mul local.get $mt f32.mul local.get $y0 f32.mul
        f32.const 3 local.get $mt f32.mul local.get $mt f32.mul local.get $t f32.mul local.get $c0y f32.mul f32.add
        f32.const 3 local.get $mt f32.mul local.get $t f32.mul local.get $t f32.mul local.get $c1y f32.mul f32.add
        local.get $t local.get $t f32.mul local.get $t f32.mul local.get $y1 f32.mul f32.add
        local.set $py1
        local.get $alpha local.get $width local.get $height local.get $bx local.get $by local.get $bw local.get $bh local.get $px0 local.get $py0 local.get $px1 local.get $py1 local.get $stroke local.get $line_cap call $icon_render_line_alpha
        local.get $px1 local.set $px0
        local.get $py1 local.set $py0
        local.get $step i32.const 1 i32.add local.set $step
        br $loop
      end
    end
    local.get $cap f32.const 1 f32.eq
    if
      local.get $alpha local.get $width local.get $height local.get $bx local.get $by local.get $bw local.get $bh local.get $x0 local.get $y0 local.get $stroke call $icon_render_round_cap_alpha
      local.get $alpha local.get $width local.get $height local.get $bx local.get $by local.get $bw local.get $bh local.get $x1 local.get $y1 local.get $stroke call $icon_render_round_cap_alpha
    else
      local.get $cap f32.const 2 f32.eq
      if
        local.get $alpha local.get $width local.get $height local.get $bx local.get $by local.get $bw local.get $bh local.get $x0 local.get $y0 local.get $x0 local.get $c0x f32.sub local.get $y0 local.get $c0y f32.sub local.get $stroke call $icon_render_square_cap_alpha
        local.get $alpha local.get $width local.get $height local.get $bx local.get $by local.get $bw local.get $bh local.get $x1 local.get $y1 local.get $x1 local.get $c1x f32.sub local.get $y1 local.get $c1y f32.sub local.get $stroke call $icon_render_square_cap_alpha
      end
    end)

  (func $icon_render_arc_alpha (param $alpha i32) (param $width i32) (param $height i32) (param $bx f32) (param $by f32) (param $bw f32) (param $bh f32) (param $x0 f32) (param $y0 f32) (param $rx_in f32) (param $ry_in f32) (param $rot f32) (param $large f32) (param $sweep f32) (param $x1 f32) (param $y1 f32) (param $stroke f32) (param $cap f32)
    (local $rx f32) (local $ry f32) (local $dx f32) (local $dy f32) (local $raw_dx f32) (local $raw_dy f32) (local $phi f32) (local $cphi f32) (local $sphi f32) (local $scale f32)
    (local $num f32) (local $den f32) (local $coef f32) (local $cxp f32) (local $cyp f32) (local $center_x f32) (local $center_y f32)
    (local $v0x f32) (local $v0y f32) (local $v1x f32) (local $v1y f32) (local $mx f32) (local $my f32) (local $mlen f32) (local $mid_x f32) (local $mid_y f32)
    (local $q0x f32) (local $q0y f32) (local $q1x f32) (local $q1y f32) (local $q0len f32) (local $q1len f32) (local $p0x f32) (local $p0y f32) (local $p1x f32) (local $p1y f32)
    (local $seg_x f32) (local $seg_y f32)
    (local $line_cap f32)
    local.get $rx_in f32.abs f32.const 0.00001 f32.le
    local.get $ry_in f32.abs f32.const 0.00001 f32.le i32.or
    if
      local.get $alpha local.get $width local.get $height local.get $bx local.get $by local.get $bw local.get $bh local.get $x0 local.get $y0 local.get $x1 local.get $y1 local.get $stroke local.get $cap call $icon_render_line_alpha
      return
    end
    local.get $rx_in f32.abs local.set $rx
    local.get $ry_in f32.abs local.set $ry
    local.get $cap f32.const 0 f32.ne
    if
      f32.const 0 local.set $line_cap
    else
      local.get $cap local.set $line_cap
    end
    local.get $rot f32.const 0.017453292 f32.mul local.set $phi
    local.get $phi call $cos_rad local.set $cphi
    local.get $phi call $sin_rad local.set $sphi
    local.get $x0 local.get $x1 f32.sub f32.const 0.5 f32.mul local.set $raw_dx
    local.get $y0 local.get $y1 f32.sub f32.const 0.5 f32.mul local.set $raw_dy
    local.get $cphi local.get $raw_dx f32.mul local.get $sphi local.get $raw_dy f32.mul f32.add local.set $dx
    local.get $sphi f32.neg local.get $raw_dx f32.mul local.get $cphi local.get $raw_dy f32.mul f32.add local.set $dy
    local.get $dx local.get $dx f32.mul local.get $rx local.get $rx f32.mul f32.div
    local.get $dy local.get $dy f32.mul local.get $ry local.get $ry f32.mul f32.div
    f32.add
    local.tee $scale
    f32.const 1
    f32.gt
    if
      local.get $scale f32.sqrt local.set $scale
      local.get $rx local.get $scale f32.mul local.set $rx
      local.get $ry local.get $scale f32.mul local.set $ry
    end
    local.get $rx local.get $rx f32.mul local.get $ry local.get $ry f32.mul f32.mul
    local.get $rx local.get $rx f32.mul local.get $dy local.get $dy f32.mul f32.mul f32.sub
    local.get $ry local.get $ry f32.mul local.get $dx local.get $dx f32.mul f32.mul f32.sub
    local.set $num
    local.get $rx local.get $rx f32.mul local.get $dy local.get $dy f32.mul f32.mul
    local.get $ry local.get $ry f32.mul local.get $dx local.get $dx f32.mul f32.mul f32.add
    local.set $den
    local.get $num f32.const 0 call $max_f32
    local.get $den f32.const 0.000001 call $max_f32
    f32.div
    f32.sqrt
    local.set $coef
    local.get $large local.get $sweep f32.eq
    if
      local.get $coef f32.neg local.set $coef
    end
    local.get $coef local.get $rx f32.mul local.get $dy f32.mul local.get $ry f32.div local.set $cxp
    local.get $coef f32.neg local.get $ry f32.mul local.get $dx f32.mul local.get $rx f32.div local.set $cyp
    local.get $x0 local.get $x1 f32.add f32.const 0.5 f32.mul
    local.get $cphi local.get $cxp f32.mul local.get $sphi local.get $cyp f32.mul f32.sub
    f32.add local.set $center_x
    local.get $y0 local.get $y1 f32.add f32.const 0.5 f32.mul
    local.get $sphi local.get $cxp f32.mul local.get $cphi local.get $cyp f32.mul f32.add
    f32.add local.set $center_y
    local.get $dx local.get $cxp f32.sub local.get $rx f32.div local.set $v0x
    local.get $dy local.get $cyp f32.sub local.get $ry f32.div local.set $v0y
    local.get $dx f32.neg local.get $cxp f32.sub local.get $rx f32.div local.set $v1x
    local.get $dy f32.neg local.get $cyp f32.sub local.get $ry f32.div local.set $v1y
    local.get $v0x local.get $v1x f32.add local.set $mx
    local.get $v0y local.get $v1y f32.add local.set $my
    local.get $large f32.const 0 f32.ne
    if
      local.get $mx f32.neg local.set $mx
      local.get $my f32.neg local.set $my
    end
    local.get $mx local.get $mx f32.mul local.get $my local.get $my f32.mul f32.add f32.sqrt local.set $mlen
    local.get $mlen f32.const 0.00001 f32.le
    if
      local.get $sweep f32.const 0 f32.ne
      if
        local.get $v0y f32.neg local.set $mx
        local.get $v0x local.set $my
      else
        local.get $v0y local.set $mx
        local.get $v0x f32.neg local.set $my
      end
      f32.const 1 local.set $mlen
    end
    local.get $rx local.get $mx f32.mul local.get $mlen f32.div local.set $raw_dx
    local.get $ry local.get $my f32.mul local.get $mlen f32.div local.set $raw_dy
    local.get $center_x local.get $cphi local.get $raw_dx f32.mul local.get $sphi local.get $raw_dy f32.mul f32.sub f32.add local.set $mid_x
    local.get $center_y local.get $sphi local.get $raw_dx f32.mul local.get $cphi local.get $raw_dy f32.mul f32.add f32.add local.set $mid_y
    local.get $v0x local.get $mx local.get $mlen f32.div f32.add local.set $q0x
    local.get $v0y local.get $my local.get $mlen f32.div f32.add local.set $q0y
    local.get $q0x local.get $q0x f32.mul local.get $q0y local.get $q0y f32.mul f32.add f32.sqrt local.tee $q0len f32.const 0.00001 f32.le
    if
      local.get $v0x local.set $q0x
      local.get $v0y local.set $q0y
      f32.const 1 local.set $q0len
    end
    local.get $mx local.get $mlen f32.div local.get $v1x f32.add local.set $q1x
    local.get $my local.get $mlen f32.div local.get $v1y f32.add local.set $q1y
    local.get $q1x local.get $q1x f32.mul local.get $q1y local.get $q1y f32.mul f32.add f32.sqrt local.tee $q1len f32.const 0.00001 f32.le
    if
      local.get $v1x local.set $q1x
      local.get $v1y local.set $q1y
      f32.const 1 local.set $q1len
    end
    local.get $x0 local.set $p0x
    local.get $y0 local.set $p0y
    local.get $center_x local.get $cphi local.get $sphi local.get $rx local.get $ry local.get $v0x local.get $q0x f32.add local.get $v0y local.get $q0y f32.add call $icon_arc_vector_x local.set $seg_x
    local.get $center_y local.get $cphi local.get $sphi local.get $rx local.get $ry local.get $v0x local.get $q0x f32.add local.get $v0y local.get $q0y f32.add call $icon_arc_vector_y local.set $seg_y
    local.get $seg_x local.set $p1x
    local.get $seg_y local.set $p1y
    local.get $alpha local.get $width local.get $height local.get $bx local.get $by local.get $bw local.get $bh local.get $p0x local.get $p0y local.get $seg_x local.get $seg_y local.get $stroke local.get $line_cap call $icon_render_line_alpha
    local.get $seg_x local.set $p0x
    local.get $seg_y local.set $p0y
    local.get $center_x local.get $cphi local.get $sphi local.get $rx local.get $ry local.get $q0x local.get $q0y call $icon_arc_vector_x local.set $seg_x
    local.get $center_y local.get $cphi local.get $sphi local.get $rx local.get $ry local.get $q0x local.get $q0y call $icon_arc_vector_y local.set $seg_y
    local.get $alpha local.get $width local.get $height local.get $bx local.get $by local.get $bw local.get $bh local.get $p0x local.get $p0y local.get $seg_x local.get $seg_y local.get $stroke local.get $line_cap call $icon_render_line_alpha
    local.get $seg_x local.set $p0x
    local.get $seg_y local.set $p0y
    local.get $center_x local.get $cphi local.get $sphi local.get $rx local.get $ry local.get $q0x local.get $mx local.get $mlen f32.div f32.add local.get $q0y local.get $my local.get $mlen f32.div f32.add call $icon_arc_vector_x local.set $seg_x
    local.get $center_y local.get $cphi local.get $sphi local.get $rx local.get $ry local.get $q0x local.get $mx local.get $mlen f32.div f32.add local.get $q0y local.get $my local.get $mlen f32.div f32.add call $icon_arc_vector_y local.set $seg_y
    local.get $alpha local.get $width local.get $height local.get $bx local.get $by local.get $bw local.get $bh local.get $p0x local.get $p0y local.get $seg_x local.get $seg_y local.get $stroke local.get $line_cap call $icon_render_line_alpha
    local.get $seg_x local.set $p0x
    local.get $seg_y local.set $p0y
    local.get $alpha local.get $width local.get $height local.get $bx local.get $by local.get $bw local.get $bh local.get $p0x local.get $p0y local.get $mid_x local.get $mid_y local.get $stroke local.get $line_cap call $icon_render_line_alpha
    local.get $mid_x local.set $p0x
    local.get $mid_y local.set $p0y
    local.get $center_x local.get $cphi local.get $sphi local.get $rx local.get $ry local.get $mx local.get $mlen f32.div local.get $q1x f32.add local.get $my local.get $mlen f32.div local.get $q1y f32.add call $icon_arc_vector_x local.set $seg_x
    local.get $center_y local.get $cphi local.get $sphi local.get $rx local.get $ry local.get $mx local.get $mlen f32.div local.get $q1x f32.add local.get $my local.get $mlen f32.div local.get $q1y f32.add call $icon_arc_vector_y local.set $seg_y
    local.get $alpha local.get $width local.get $height local.get $bx local.get $by local.get $bw local.get $bh local.get $p0x local.get $p0y local.get $seg_x local.get $seg_y local.get $stroke local.get $line_cap call $icon_render_line_alpha
    local.get $seg_x local.set $p0x
    local.get $seg_y local.set $p0y
    local.get $center_x local.get $cphi local.get $sphi local.get $rx local.get $ry local.get $q1x local.get $q1y call $icon_arc_vector_x local.set $seg_x
    local.get $center_y local.get $cphi local.get $sphi local.get $rx local.get $ry local.get $q1x local.get $q1y call $icon_arc_vector_y local.set $seg_y
    local.get $alpha local.get $width local.get $height local.get $bx local.get $by local.get $bw local.get $bh local.get $p0x local.get $p0y local.get $seg_x local.get $seg_y local.get $stroke local.get $line_cap call $icon_render_line_alpha
    local.get $seg_x local.set $p0x
    local.get $seg_y local.set $p0y
    local.get $center_x local.get $cphi local.get $sphi local.get $rx local.get $ry local.get $q1x local.get $v1x f32.add local.get $q1y local.get $v1y f32.add call $icon_arc_vector_x local.set $seg_x
    local.get $center_y local.get $cphi local.get $sphi local.get $rx local.get $ry local.get $q1x local.get $v1x f32.add local.get $q1y local.get $v1y f32.add call $icon_arc_vector_y local.set $seg_y
    local.get $alpha local.get $width local.get $height local.get $bx local.get $by local.get $bw local.get $bh local.get $p0x local.get $p0y local.get $seg_x local.get $seg_y local.get $stroke local.get $line_cap call $icon_render_line_alpha
    local.get $seg_x local.set $p0x
    local.get $seg_y local.set $p0y
    local.get $alpha local.get $width local.get $height local.get $bx local.get $by local.get $bw local.get $bh local.get $p0x local.get $p0y local.get $x1 local.get $y1 local.get $stroke local.get $line_cap call $icon_render_line_alpha
    local.get $cap f32.const 1 f32.eq
    if
      local.get $alpha local.get $width local.get $height local.get $bx local.get $by local.get $bw local.get $bh local.get $x0 local.get $y0 local.get $stroke call $icon_render_round_cap_alpha
      local.get $alpha local.get $width local.get $height local.get $bx local.get $by local.get $bw local.get $bh local.get $x1 local.get $y1 local.get $stroke call $icon_render_round_cap_alpha
    else
      local.get $cap f32.const 2 f32.eq
      if
        local.get $alpha local.get $width local.get $height local.get $bx local.get $by local.get $bw local.get $bh local.get $x0 local.get $y0 local.get $x0 local.get $p1x f32.sub local.get $y0 local.get $p1y f32.sub local.get $stroke call $icon_render_square_cap_alpha
        local.get $alpha local.get $width local.get $height local.get $bx local.get $by local.get $bw local.get $bh local.get $x1 local.get $y1 local.get $x1 local.get $p0x f32.sub local.get $y1 local.get $p0y f32.sub local.get $stroke call $icon_render_square_cap_alpha
      end
    end)

  (func $icon_render_dashed_line_alpha (param $alpha i32) (param $width i32) (param $height i32) (param $bx f32) (param $by f32) (param $bw f32) (param $bh f32) (param $x0 f32) (param $y0 f32) (param $x1 f32) (param $y1 f32) (param $stroke f32) (param $cap f32) (param $dash_on f32) (param $dash_off f32) (param $dash_offset f32)
    (local $dx f32) (local $dy f32) (local $len f32) (local $pattern f32) (local $phase f32)
    (local $pos f32) (local $next f32) (local $remain f32) (local $draw i32)
    (local $sx f32) (local $sy f32) (local $ex f32) (local $ey f32)
    local.get $dash_on f32.const 0 f32.le
    local.get $dash_off f32.const 0 f32.le
    i32.or
    if
      local.get $alpha local.get $width local.get $height local.get $bx local.get $by local.get $bw local.get $bh local.get $x0 local.get $y0 local.get $x1 local.get $y1 local.get $stroke local.get $cap call $icon_render_line_alpha
      return
    end
    local.get $x1 local.get $x0 f32.sub local.set $dx
    local.get $y1 local.get $y0 f32.sub local.set $dy
    local.get $dx local.get $dx f32.mul
    local.get $dy local.get $dy f32.mul
    f32.add
    f32.sqrt
    local.tee $len
    f32.const 0.000001
    f32.le
    if
      return
    end
    local.get $dash_on local.get $dash_off f32.add local.set $pattern
    local.get $dash_offset local.set $phase
    block $phase_norm_done
      loop $phase_pos
        local.get $phase f32.const 0 f32.lt i32.eqz br_if $phase_norm_done
        local.get $phase local.get $pattern f32.add local.set $phase
        br $phase_pos
      end
    end
    block $phase_high_done
      loop $phase_high
        local.get $phase local.get $pattern f32.ge i32.eqz br_if $phase_high_done
        local.get $phase local.get $pattern f32.sub local.set $phase
        br $phase_high
      end
    end
    f32.const 0 local.set $pos
    block $done
      loop $loop
        local.get $pos local.get $len f32.ge br_if $done
        local.get $phase local.get $dash_on f32.lt local.set $draw
        local.get $draw
        if
          local.get $dash_on local.get $phase f32.sub local.set $remain
        else
          local.get $pattern local.get $phase f32.sub local.set $remain
        end
        local.get $remain f32.const 0.000001 f32.le
        if
          local.get $phase local.get $pattern f32.add local.set $phase
          br $loop
        end
        local.get $pos local.get $remain f32.add local.get $len call $min_f32 local.set $next
        local.get $draw
        if
          local.get $x0 local.get $dx local.get $pos local.get $len f32.div f32.mul f32.add local.set $sx
          local.get $y0 local.get $dy local.get $pos local.get $len f32.div f32.mul f32.add local.set $sy
          local.get $x0 local.get $dx local.get $next local.get $len f32.div f32.mul f32.add local.set $ex
          local.get $y0 local.get $dy local.get $next local.get $len f32.div f32.mul f32.add local.set $ey
          local.get $alpha local.get $width local.get $height local.get $bx local.get $by local.get $bw local.get $bh local.get $sx local.get $sy local.get $ex local.get $ey local.get $stroke local.get $cap call $icon_render_line_alpha
        end
        local.get $phase local.get $next local.get $pos f32.sub f32.add local.set $phase
        local.get $phase local.get $pattern f32.ge
        if
          local.get $phase local.get $pattern f32.sub local.set $phase
        end
        local.get $next local.set $pos
        br $loop
      end
    end)

  (func $icon_dash_phase_after_segment (param $phase f32) (param $x0 f32) (param $y0 f32) (param $x1 f32) (param $y1 f32) (result f32)
    local.get $phase
    local.get $x1 local.get $x0 f32.sub
    local.get $x1 local.get $x0 f32.sub
    f32.mul
    local.get $y1 local.get $y0 f32.sub
    local.get $y1 local.get $y0 f32.sub
    f32.mul
    f32.add
    f32.sqrt
    f32.add)

  (func $icon_render_dashed_quad_alpha (param $alpha i32) (param $width i32) (param $height i32) (param $bx f32) (param $by f32) (param $bw f32) (param $bh f32) (param $x0 f32) (param $y0 f32) (param $cx f32) (param $cy f32) (param $x1 f32) (param $y1 f32) (param $stroke f32) (param $cap f32) (param $dash_on f32) (param $dash_off f32) (param $dash_offset f32) (result f32)
    (local $step i32) (local $t f32) (local $mt f32) (local $px0 f32) (local $py0 f32) (local $px1 f32) (local $py1 f32)
    (local $dx f32) (local $dy f32) (local $line_cap f32) (local $phase f32)
    local.get $x0 local.set $px0
    local.get $y0 local.set $py0
    local.get $dash_offset local.set $phase
    local.get $cap f32.const 0 f32.ne
    if
      f32.const 0 local.set $line_cap
    else
      local.get $cap local.set $line_cap
    end
    i32.const 1 local.set $step
    block $done
      loop $loop
        local.get $step i32.const 16 i32.gt_u br_if $done
        local.get $step f32.convert_i32_u f32.const 16 f32.div local.set $t
        f32.const 1 local.get $t f32.sub local.set $mt
        local.get $mt local.get $mt f32.mul local.get $x0 f32.mul
        f32.const 2 local.get $mt f32.mul local.get $t f32.mul local.get $cx f32.mul f32.add
        local.get $t local.get $t f32.mul local.get $x1 f32.mul f32.add
        local.set $px1
        local.get $mt local.get $mt f32.mul local.get $y0 f32.mul
        f32.const 2 local.get $mt f32.mul local.get $t f32.mul local.get $cy f32.mul f32.add
        local.get $t local.get $t f32.mul local.get $y1 f32.mul f32.add
        local.set $py1
        local.get $alpha local.get $width local.get $height local.get $bx local.get $by local.get $bw local.get $bh local.get $px0 local.get $py0 local.get $px1 local.get $py1 local.get $stroke local.get $line_cap local.get $dash_on local.get $dash_off local.get $phase call $icon_render_dashed_line_alpha
        local.get $phase local.get $px0 local.get $py0 local.get $px1 local.get $py1 call $icon_dash_phase_after_segment local.set $phase
        local.get $px1 local.set $px0
        local.get $py1 local.set $py0
        local.get $step i32.const 1 i32.add local.set $step
        br $loop
      end
    end
    local.get $phase)

  (func $icon_render_dashed_cubic_alpha (param $alpha i32) (param $width i32) (param $height i32) (param $bx f32) (param $by f32) (param $bw f32) (param $bh f32) (param $x0 f32) (param $y0 f32) (param $c0x f32) (param $c0y f32) (param $c1x f32) (param $c1y f32) (param $x1 f32) (param $y1 f32) (param $stroke f32) (param $cap f32) (param $dash_on f32) (param $dash_off f32) (param $dash_offset f32) (result f32)
    (local $step i32) (local $t f32) (local $mt f32) (local $px0 f32) (local $py0 f32) (local $px1 f32) (local $py1 f32)
    (local $dx f32) (local $dy f32) (local $line_cap f32) (local $phase f32)
    local.get $x0 local.set $px0
    local.get $y0 local.set $py0
    local.get $dash_offset local.set $phase
    local.get $cap f32.const 0 f32.ne
    if
      f32.const 0 local.set $line_cap
    else
      local.get $cap local.set $line_cap
    end
    i32.const 1 local.set $step
    block $done
      loop $loop
        local.get $step i32.const 20 i32.gt_u br_if $done
        local.get $step f32.convert_i32_u f32.const 20 f32.div local.set $t
        f32.const 1 local.get $t f32.sub local.set $mt
        local.get $mt local.get $mt f32.mul local.get $mt f32.mul local.get $x0 f32.mul
        f32.const 3 local.get $mt f32.mul local.get $mt f32.mul local.get $t f32.mul local.get $c0x f32.mul f32.add
        f32.const 3 local.get $mt f32.mul local.get $t f32.mul local.get $t f32.mul local.get $c1x f32.mul f32.add
        local.get $t local.get $t f32.mul local.get $t f32.mul local.get $x1 f32.mul f32.add
        local.set $px1
        local.get $mt local.get $mt f32.mul local.get $mt f32.mul local.get $y0 f32.mul
        f32.const 3 local.get $mt f32.mul local.get $mt f32.mul local.get $t f32.mul local.get $c0y f32.mul f32.add
        f32.const 3 local.get $mt f32.mul local.get $t f32.mul local.get $t f32.mul local.get $c1y f32.mul f32.add
        local.get $t local.get $t f32.mul local.get $t f32.mul local.get $y1 f32.mul f32.add
        local.set $py1
        local.get $alpha local.get $width local.get $height local.get $bx local.get $by local.get $bw local.get $bh local.get $px0 local.get $py0 local.get $px1 local.get $py1 local.get $stroke local.get $line_cap local.get $dash_on local.get $dash_off local.get $phase call $icon_render_dashed_line_alpha
        local.get $phase local.get $px0 local.get $py0 local.get $px1 local.get $py1 call $icon_dash_phase_after_segment local.set $phase
        local.get $px1 local.set $px0
        local.get $py1 local.set $py0
        local.get $step i32.const 1 i32.add local.set $step
        br $loop
      end
    end
    local.get $phase)

  (func $icon_render_dashed_arc_alpha (param $alpha i32) (param $width i32) (param $height i32) (param $bx f32) (param $by f32) (param $bw f32) (param $bh f32) (param $x0 f32) (param $y0 f32) (param $rx_in f32) (param $ry_in f32) (param $rot f32) (param $large f32) (param $sweep f32) (param $x1 f32) (param $y1 f32) (param $stroke f32) (param $cap f32) (param $dash_on f32) (param $dash_off f32) (param $dash_offset f32) (result f32)
    (local $rx f32) (local $ry f32) (local $dx f32) (local $dy f32) (local $raw_dx f32) (local $raw_dy f32) (local $phi f32) (local $cphi f32) (local $sphi f32) (local $scale f32)
    (local $num f32) (local $den f32) (local $coef f32) (local $cxp f32) (local $cyp f32) (local $center_x f32) (local $center_y f32)
    (local $v0x f32) (local $v0y f32) (local $v1x f32) (local $v1y f32) (local $mx f32) (local $my f32) (local $mlen f32) (local $mid_x f32) (local $mid_y f32)
    (local $q0x f32) (local $q0y f32) (local $q1x f32) (local $q1y f32) (local $q0len f32) (local $q1len f32) (local $p0x f32) (local $p0y f32) (local $seg_x f32) (local $seg_y f32)
    (local $line_cap f32) (local $phase f32)
    local.get $dash_offset local.set $phase
    local.get $rx_in f32.abs f32.const 0.00001 f32.le
    local.get $ry_in f32.abs f32.const 0.00001 f32.le i32.or
    if
      local.get $alpha local.get $width local.get $height local.get $bx local.get $by local.get $bw local.get $bh local.get $x0 local.get $y0 local.get $x1 local.get $y1 local.get $stroke local.get $cap local.get $dash_on local.get $dash_off local.get $phase call $icon_render_dashed_line_alpha
      local.get $x1 local.get $x0 f32.sub local.set $dx
      local.get $y1 local.get $y0 f32.sub local.set $dy
      local.get $phase local.get $x0 local.get $y0 local.get $x1 local.get $y1 call $icon_dash_phase_after_segment return
    end
    local.get $rx_in f32.abs local.set $rx
    local.get $ry_in f32.abs local.set $ry
    local.get $cap f32.const 0 f32.ne
    if
      f32.const 0 local.set $line_cap
    else
      local.get $cap local.set $line_cap
    end
    local.get $rot f32.const 0.017453292 f32.mul local.set $phi
    local.get $phi call $cos_rad local.set $cphi
    local.get $phi call $sin_rad local.set $sphi
    local.get $x0 local.get $x1 f32.sub f32.const 0.5 f32.mul local.set $raw_dx
    local.get $y0 local.get $y1 f32.sub f32.const 0.5 f32.mul local.set $raw_dy
    local.get $cphi local.get $raw_dx f32.mul local.get $sphi local.get $raw_dy f32.mul f32.add local.set $dx
    local.get $sphi f32.neg local.get $raw_dx f32.mul local.get $cphi local.get $raw_dy f32.mul f32.add local.set $dy
    local.get $dx local.get $dx f32.mul local.get $rx local.get $rx f32.mul f32.div
    local.get $dy local.get $dy f32.mul local.get $ry local.get $ry f32.mul f32.div
    f32.add
    local.tee $scale
    f32.const 1
    f32.gt
    if
      local.get $scale f32.sqrt local.set $scale
      local.get $rx local.get $scale f32.mul local.set $rx
      local.get $ry local.get $scale f32.mul local.set $ry
    end
    local.get $rx local.get $rx f32.mul local.get $ry local.get $ry f32.mul f32.mul
    local.get $rx local.get $rx f32.mul local.get $dy local.get $dy f32.mul f32.mul f32.sub
    local.get $ry local.get $ry f32.mul local.get $dx local.get $dx f32.mul f32.mul f32.sub
    local.set $num
    local.get $rx local.get $rx f32.mul local.get $dy local.get $dy f32.mul f32.mul
    local.get $ry local.get $ry f32.mul local.get $dx local.get $dx f32.mul f32.mul f32.add
    local.set $den
    local.get $num f32.const 0 call $max_f32
    local.get $den f32.const 0.000001 call $max_f32
    f32.div
    f32.sqrt
    local.set $coef
    local.get $large local.get $sweep f32.eq
    if
      local.get $coef f32.neg local.set $coef
    end
    local.get $coef local.get $rx f32.mul local.get $dy f32.mul local.get $ry f32.div local.set $cxp
    local.get $coef f32.neg local.get $ry f32.mul local.get $dx f32.mul local.get $rx f32.div local.set $cyp
    local.get $x0 local.get $x1 f32.add f32.const 0.5 f32.mul
    local.get $cphi local.get $cxp f32.mul local.get $sphi local.get $cyp f32.mul f32.sub
    f32.add local.set $center_x
    local.get $y0 local.get $y1 f32.add f32.const 0.5 f32.mul
    local.get $sphi local.get $cxp f32.mul local.get $cphi local.get $cyp f32.mul f32.add
    f32.add local.set $center_y
    local.get $dx local.get $cxp f32.sub local.get $rx f32.div local.set $v0x
    local.get $dy local.get $cyp f32.sub local.get $ry f32.div local.set $v0y
    local.get $dx f32.neg local.get $cxp f32.sub local.get $rx f32.div local.set $v1x
    local.get $dy f32.neg local.get $cyp f32.sub local.get $ry f32.div local.set $v1y
    local.get $v0x local.get $v1x f32.add local.set $mx
    local.get $v0y local.get $v1y f32.add local.set $my
    local.get $large f32.const 0 f32.ne
    if
      local.get $mx f32.neg local.set $mx
      local.get $my f32.neg local.set $my
    end
    local.get $mx local.get $mx f32.mul local.get $my local.get $my f32.mul f32.add f32.sqrt local.set $mlen
    local.get $mlen f32.const 0.00001 f32.le
    if
      local.get $sweep f32.const 0 f32.ne
      if
        local.get $v0y f32.neg local.set $mx
        local.get $v0x local.set $my
      else
        local.get $v0y local.set $mx
        local.get $v0x f32.neg local.set $my
      end
      f32.const 1 local.set $mlen
    end
    local.get $rx local.get $mx f32.mul local.get $mlen f32.div local.set $raw_dx
    local.get $ry local.get $my f32.mul local.get $mlen f32.div local.set $raw_dy
    local.get $center_x local.get $cphi local.get $raw_dx f32.mul local.get $sphi local.get $raw_dy f32.mul f32.sub f32.add local.set $mid_x
    local.get $center_y local.get $sphi local.get $raw_dx f32.mul local.get $cphi local.get $raw_dy f32.mul f32.add f32.add local.set $mid_y
    local.get $v0x local.get $mx local.get $mlen f32.div f32.add local.set $q0x
    local.get $v0y local.get $my local.get $mlen f32.div f32.add local.set $q0y
    local.get $q0x local.get $q0x f32.mul local.get $q0y local.get $q0y f32.mul f32.add f32.sqrt local.tee $q0len f32.const 0.00001 f32.le
    if
      local.get $v0x local.set $q0x
      local.get $v0y local.set $q0y
      f32.const 1 local.set $q0len
    end
    local.get $mx local.get $mlen f32.div local.get $v1x f32.add local.set $q1x
    local.get $my local.get $mlen f32.div local.get $v1y f32.add local.set $q1y
    local.get $q1x local.get $q1x f32.mul local.get $q1y local.get $q1y f32.mul f32.add f32.sqrt local.tee $q1len f32.const 0.00001 f32.le
    if
      local.get $v1x local.set $q1x
      local.get $v1y local.set $q1y
      f32.const 1 local.set $q1len
    end
    local.get $x0 local.set $p0x
    local.get $y0 local.set $p0y
    local.get $center_x local.get $cphi local.get $sphi local.get $rx local.get $ry local.get $v0x local.get $q0x f32.add local.get $v0y local.get $q0y f32.add call $icon_arc_vector_x local.set $seg_x
    local.get $center_y local.get $cphi local.get $sphi local.get $rx local.get $ry local.get $v0x local.get $q0x f32.add local.get $v0y local.get $q0y f32.add call $icon_arc_vector_y local.set $seg_y
    local.get $alpha local.get $width local.get $height local.get $bx local.get $by local.get $bw local.get $bh local.get $p0x local.get $p0y local.get $seg_x local.get $seg_y local.get $stroke local.get $line_cap local.get $dash_on local.get $dash_off local.get $phase call $icon_render_dashed_line_alpha
    local.get $phase local.get $p0x local.get $p0y local.get $seg_x local.get $seg_y call $icon_dash_phase_after_segment local.set $phase
    local.get $seg_x local.set $p0x
    local.get $seg_y local.set $p0y
    local.get $center_x local.get $cphi local.get $sphi local.get $rx local.get $ry local.get $q0x local.get $q0y call $icon_arc_vector_x local.set $seg_x
    local.get $center_y local.get $cphi local.get $sphi local.get $rx local.get $ry local.get $q0x local.get $q0y call $icon_arc_vector_y local.set $seg_y
    local.get $alpha local.get $width local.get $height local.get $bx local.get $by local.get $bw local.get $bh local.get $p0x local.get $p0y local.get $seg_x local.get $seg_y local.get $stroke local.get $line_cap local.get $dash_on local.get $dash_off local.get $phase call $icon_render_dashed_line_alpha
    local.get $phase local.get $p0x local.get $p0y local.get $seg_x local.get $seg_y call $icon_dash_phase_after_segment local.set $phase
    local.get $seg_x local.set $p0x
    local.get $seg_y local.set $p0y
    local.get $center_x local.get $cphi local.get $sphi local.get $rx local.get $ry local.get $q0x local.get $mx local.get $mlen f32.div f32.add local.get $q0y local.get $my local.get $mlen f32.div f32.add call $icon_arc_vector_x local.set $seg_x
    local.get $center_y local.get $cphi local.get $sphi local.get $rx local.get $ry local.get $q0x local.get $mx local.get $mlen f32.div f32.add local.get $q0y local.get $my local.get $mlen f32.div f32.add call $icon_arc_vector_y local.set $seg_y
    local.get $alpha local.get $width local.get $height local.get $bx local.get $by local.get $bw local.get $bh local.get $p0x local.get $p0y local.get $seg_x local.get $seg_y local.get $stroke local.get $line_cap local.get $dash_on local.get $dash_off local.get $phase call $icon_render_dashed_line_alpha
    local.get $phase local.get $p0x local.get $p0y local.get $seg_x local.get $seg_y call $icon_dash_phase_after_segment local.set $phase
    local.get $seg_x local.set $p0x
    local.get $seg_y local.set $p0y
    local.get $alpha local.get $width local.get $height local.get $bx local.get $by local.get $bw local.get $bh local.get $p0x local.get $p0y local.get $mid_x local.get $mid_y local.get $stroke local.get $line_cap local.get $dash_on local.get $dash_off local.get $phase call $icon_render_dashed_line_alpha
    local.get $phase local.get $p0x local.get $p0y local.get $mid_x local.get $mid_y call $icon_dash_phase_after_segment local.set $phase
    local.get $mid_x local.set $p0x
    local.get $mid_y local.set $p0y
    local.get $center_x local.get $cphi local.get $sphi local.get $rx local.get $ry local.get $mx local.get $mlen f32.div local.get $q1x f32.add local.get $my local.get $mlen f32.div local.get $q1y f32.add call $icon_arc_vector_x local.set $seg_x
    local.get $center_y local.get $cphi local.get $sphi local.get $rx local.get $ry local.get $mx local.get $mlen f32.div local.get $q1x f32.add local.get $my local.get $mlen f32.div local.get $q1y f32.add call $icon_arc_vector_y local.set $seg_y
    local.get $alpha local.get $width local.get $height local.get $bx local.get $by local.get $bw local.get $bh local.get $p0x local.get $p0y local.get $seg_x local.get $seg_y local.get $stroke local.get $line_cap local.get $dash_on local.get $dash_off local.get $phase call $icon_render_dashed_line_alpha
    local.get $phase local.get $p0x local.get $p0y local.get $seg_x local.get $seg_y call $icon_dash_phase_after_segment local.set $phase
    local.get $seg_x local.set $p0x
    local.get $seg_y local.set $p0y
    local.get $center_x local.get $cphi local.get $sphi local.get $rx local.get $ry local.get $q1x local.get $q1y call $icon_arc_vector_x local.set $seg_x
    local.get $center_y local.get $cphi local.get $sphi local.get $rx local.get $ry local.get $q1x local.get $q1y call $icon_arc_vector_y local.set $seg_y
    local.get $alpha local.get $width local.get $height local.get $bx local.get $by local.get $bw local.get $bh local.get $p0x local.get $p0y local.get $seg_x local.get $seg_y local.get $stroke local.get $line_cap local.get $dash_on local.get $dash_off local.get $phase call $icon_render_dashed_line_alpha
    local.get $phase local.get $p0x local.get $p0y local.get $seg_x local.get $seg_y call $icon_dash_phase_after_segment local.set $phase
    local.get $seg_x local.set $p0x
    local.get $seg_y local.set $p0y
    local.get $center_x local.get $cphi local.get $sphi local.get $rx local.get $ry local.get $q1x local.get $v1x f32.add local.get $q1y local.get $v1y f32.add call $icon_arc_vector_x local.set $seg_x
    local.get $center_y local.get $cphi local.get $sphi local.get $rx local.get $ry local.get $q1x local.get $v1x f32.add local.get $q1y local.get $v1y f32.add call $icon_arc_vector_y local.set $seg_y
    local.get $alpha local.get $width local.get $height local.get $bx local.get $by local.get $bw local.get $bh local.get $p0x local.get $p0y local.get $seg_x local.get $seg_y local.get $stroke local.get $line_cap local.get $dash_on local.get $dash_off local.get $phase call $icon_render_dashed_line_alpha
    local.get $phase local.get $p0x local.get $p0y local.get $seg_x local.get $seg_y call $icon_dash_phase_after_segment local.set $phase
    local.get $seg_x local.set $p0x
    local.get $seg_y local.set $p0y
    local.get $alpha local.get $width local.get $height local.get $bx local.get $by local.get $bw local.get $bh local.get $p0x local.get $p0y local.get $x1 local.get $y1 local.get $stroke local.get $line_cap local.get $dash_on local.get $dash_off local.get $phase call $icon_render_dashed_line_alpha
    local.get $phase local.get $p0x local.get $p0y local.get $x1 local.get $y1 call $icon_dash_phase_after_segment)
  (func $er_ui_icon_render_alpha  (param $icon i32) (param $alpha i32) (param $width i32) (param $height i32) (param $x f32) (param $y f32) (param $w f32) (param $h f32) (result i32)
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

  (func $er_ui_icon_render_rgba  (param $icon i32) (param $rgba i32) (param $width i32) (param $height i32) (param $x f32) (param $y f32) (param $w f32) (param $h f32) (param $current_rgba i32) (result i32)
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

  (func $er_ui_icon_segment_count  (param $icon i32) (result i32)
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
  (func $icon_write_line (param $out i32) (param $cap i32) (param $count i32) (param $x1 f32) (param $y1 f32) (param $x2 f32) (param $y2 f32) (result i32)
    (local $p i32)
    local.get $count
    i32.const 1
    i32.add
    i32.const 20
    i32.mul
    local.get $cap
    i32.gt_u
    if
      i32.const -1
      return
    end
    local.get $out
    local.get $count
    i32.const 20
    i32.mul
    i32.add
    local.set $p
    local.get $p
    i32.const 1
    call $store32
    local.get $p
    i32.const 4
    i32.add
    local.get $x1
    f32.store
    local.get $p
    i32.const 8
    i32.add
    local.get $y1
    f32.store
    local.get $p
    i32.const 12
    i32.add
    local.get $x2
    f32.store
    local.get $p
    i32.const 16
    i32.add
    local.get $y2
    f32.store
    local.get $count
    i32.const 1
    i32.add)

  (func $icon_write_line_scaled (param $out i32) (param $cap i32) (param $count i32) (param $x f32) (param $y f32) (param $scale f32) (param $dpr f32) (param $x1 f32) (param $y1 f32) (param $x2 f32) (param $y2 f32) (result i32)
    local.get $out
    local.get $cap
    local.get $count
    local.get $x
    local.get $x1
    local.get $scale
    f32.mul
    f32.add
    local.get $dpr
    call $snap_pixel
    local.get $y
    local.get $y1
    local.get $scale
    f32.mul
    f32.add
    local.get $dpr
    call $snap_pixel
    local.get $x
    local.get $x2
    local.get $scale
    f32.mul
    f32.add
    local.get $dpr
    call $snap_pixel
    local.get $y
    local.get $y2
    local.get $scale
    f32.mul
    f32.add
    local.get $dpr
    call $snap_pixel
    call $icon_write_line)

  (func $er_ui_icon_scaled_stroke_width  (param $icon i32) (param $size f32) (result f32)
    local.get $icon
    call $er_ui_icon_valid
    local.get $size
    f32.const 0
    f32.gt
    i32.and
    if (result f32)
      local.get $icon
      call $er_ui_icon_stroke_width
      local.get $size
      f32.mul
      f32.const 24
      f32.div
    else
      f32.const 0
    end)

  (func $er_ui_icon_fit_rect  (param $x f32) (param $y f32) (param $w f32) (param $h f32) (param $dpr f32) (param $out i32) (result i32)
    (local $size f32)
    local.get $out
    i32.eqz
    local.get $w
    local.get $h
    call $valid_rect
    i32.eqz
    i32.or
    if
      i32.const 0
      return
    end
    local.get $w
    local.get $h
    call $min_f32
    local.set $size
    local.get $out
    local.get $x
    local.get $w
    local.get $size
    f32.sub
    f32.const 0.5
    f32.mul
    f32.add
    local.get $dpr
    call $snap_pixel
    f32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $y
    local.get $h
    local.get $size
    f32.sub
    f32.const 0.5
    f32.mul
    f32.add
    local.get $dpr
    call $snap_pixel
    f32.store
    local.get $out
    i32.const 8
    i32.add
    local.get $size
    local.get $dpr
    call $snap_pixel
    f32.store
    local.get $out
    i32.const 12
    i32.add
    local.get $size
    local.get $dpr
    call $snap_pixel
    f32.store
    i32.const 1)

  (func $er_ui_icon_fit_viewport  (param $x f32) (param $y f32) (param $w f32) (param $h f32) (param $dpr f32) (param $out i32) (result i32)
    local.get $x
    local.get $y
    local.get $w
    local.get $h
    local.get $dpr
    local.get $out
    call $er_ui_icon_fit_rect)

  (func $er_ui_icon_segments_write  (param $icon i32) (param $out i32) (param $cap i32) (result i32)
    (local $count i32)
    local.get $cap
    local.get $icon
    call $er_ui_icon_segment_count
    i32.const 20
    i32.mul
    i32.lt_u
    if
      i32.const -1
      return
    end
    local.get $icon
    i32.const 44
    i32.eq
    local.get $icon
    i32.const 2
    i32.eq
    i32.or
    if
      local.get $out
      local.get $cap
      local.get $count
      f32.const 6
      f32.const 9
      f32.const 12
      f32.const 15
      call $icon_write_line
      local.set $count
      local.get $out
      local.get $cap
      local.get $count
      f32.const 12
      f32.const 15
      f32.const 18
      f32.const 9
      call $icon_write_line
      return
    end
    local.get $icon
    i32.const 55
    i32.eq
    if
      local.get $out local.get $cap local.get $count f32.const 8 f32.const 5 f32.const 16 f32.const 5 call $icon_write_line local.set $count
      local.get $out local.get $cap local.get $count f32.const 16 f32.const 5 f32.const 16 f32.const 13 call $icon_write_line local.set $count
      local.get $out local.get $cap local.get $count f32.const 16 f32.const 13 f32.const 8 f32.const 13 call $icon_write_line local.set $count
      local.get $out local.get $cap local.get $count f32.const 8 f32.const 13 f32.const 8 f32.const 5 call $icon_write_line local.set $count
      local.get $out local.get $cap local.get $count f32.const 15 f32.const 14 f32.const 20 f32.const 19 call $icon_write_line
      return
    end
    local.get $icon
    i32.const 1
    i32.eq
    if
      local.get $out local.get $cap local.get $count f32.const 5 f32.const 12 f32.const 10 f32.const 17 call $icon_write_line local.set $count
      local.get $out local.get $cap local.get $count f32.const 10 f32.const 17 f32.const 19 f32.const 7 call $icon_write_line
      return
    end
    local.get $icon
    i32.const 3
    i32.eq
    if
      local.get $out local.get $cap local.get $count f32.const 6 f32.const 6 f32.const 18 f32.const 18 call $icon_write_line local.set $count
      local.get $out local.get $cap local.get $count f32.const 18 f32.const 6 f32.const 6 f32.const 18 call $icon_write_line
      return
    end
    local.get $out local.get $cap local.get $count f32.const 6 f32.const 6 f32.const 18 f32.const 6 call $icon_write_line local.set $count
    local.get $out local.get $cap local.get $count f32.const 18 f32.const 6 f32.const 18 f32.const 18 call $icon_write_line local.set $count
    local.get $out local.get $cap local.get $count f32.const 18 f32.const 18 f32.const 6 f32.const 18 call $icon_write_line local.set $count
    local.get $out local.get $cap local.get $count f32.const 6 f32.const 18 f32.const 6 f32.const 6 call $icon_write_line)

  (func $er_ui_icon_segments_write_scaled  (param $icon i32) (param $out i32) (param $cap i32) (param $x f32) (param $y f32) (param $w f32) (param $h f32) (param $dpr f32) (result i32)
    (local $count i32)
    (local $fit_x f32)
    (local $fit_y f32)
    (local $size f32)
    (local $scale f32)
    local.get $w
    local.get $h
    call $valid_rect
    i32.eqz
    if
      i32.const -1
      return
    end
    local.get $cap
    local.get $icon
    call $er_ui_icon_segment_count
    i32.const 20
    i32.mul
    i32.lt_u
    if
      i32.const -1
      return
    end
    local.get $w
    local.get $h
    call $min_f32
    local.get $dpr
    call $snap_pixel
    local.set $size
    local.get $x
    local.get $w
    local.get $size
    f32.sub
    f32.const 0.5
    f32.mul
    f32.add
    local.get $dpr
    call $snap_pixel
    local.set $fit_x
    local.get $y
    local.get $h
    local.get $size
    f32.sub
    f32.const 0.5
    f32.mul
    f32.add
    local.get $dpr
    call $snap_pixel
    local.set $fit_y
    local.get $size
    f32.const 24
    f32.div
    local.set $scale
    local.get $icon
    i32.const 44
    i32.eq
    local.get $icon
    i32.const 2
    i32.eq
    i32.or
    if
      local.get $out local.get $cap local.get $count local.get $fit_x local.get $fit_y local.get $scale local.get $dpr f32.const 6 f32.const 9 f32.const 12 f32.const 15 call $icon_write_line_scaled local.set $count
      local.get $out local.get $cap local.get $count local.get $fit_x local.get $fit_y local.get $scale local.get $dpr f32.const 12 f32.const 15 f32.const 18 f32.const 9 call $icon_write_line_scaled
      return
    end
    local.get $icon
    i32.const 55
    i32.eq
    if
      local.get $out local.get $cap local.get $count local.get $fit_x local.get $fit_y local.get $scale local.get $dpr f32.const 8 f32.const 5 f32.const 16 f32.const 5 call $icon_write_line_scaled local.set $count
      local.get $out local.get $cap local.get $count local.get $fit_x local.get $fit_y local.get $scale local.get $dpr f32.const 16 f32.const 5 f32.const 16 f32.const 13 call $icon_write_line_scaled local.set $count
      local.get $out local.get $cap local.get $count local.get $fit_x local.get $fit_y local.get $scale local.get $dpr f32.const 16 f32.const 13 f32.const 8 f32.const 13 call $icon_write_line_scaled local.set $count
      local.get $out local.get $cap local.get $count local.get $fit_x local.get $fit_y local.get $scale local.get $dpr f32.const 8 f32.const 13 f32.const 8 f32.const 5 call $icon_write_line_scaled local.set $count
      local.get $out local.get $cap local.get $count local.get $fit_x local.get $fit_y local.get $scale local.get $dpr f32.const 15 f32.const 14 f32.const 20 f32.const 19 call $icon_write_line_scaled
      return
    end
    local.get $icon
    i32.const 1
    i32.eq
    if
      local.get $out local.get $cap local.get $count local.get $fit_x local.get $fit_y local.get $scale local.get $dpr f32.const 5 f32.const 12 f32.const 10 f32.const 17 call $icon_write_line_scaled local.set $count
      local.get $out local.get $cap local.get $count local.get $fit_x local.get $fit_y local.get $scale local.get $dpr f32.const 10 f32.const 17 f32.const 19 f32.const 7 call $icon_write_line_scaled
      return
    end
    local.get $icon
    i32.const 3
    i32.eq
    if
      local.get $out local.get $cap local.get $count local.get $fit_x local.get $fit_y local.get $scale local.get $dpr f32.const 6 f32.const 6 f32.const 18 f32.const 18 call $icon_write_line_scaled local.set $count
      local.get $out local.get $cap local.get $count local.get $fit_x local.get $fit_y local.get $scale local.get $dpr f32.const 18 f32.const 6 f32.const 6 f32.const 18 call $icon_write_line_scaled
      return
    end
    local.get $out local.get $cap local.get $count local.get $fit_x local.get $fit_y local.get $scale local.get $dpr f32.const 6 f32.const 6 f32.const 18 f32.const 6 call $icon_write_line_scaled local.set $count
    local.get $out local.get $cap local.get $count local.get $fit_x local.get $fit_y local.get $scale local.get $dpr f32.const 18 f32.const 6 f32.const 18 f32.const 18 call $icon_write_line_scaled local.set $count
    local.get $out local.get $cap local.get $count local.get $fit_x local.get $fit_y local.get $scale local.get $dpr f32.const 18 f32.const 18 f32.const 6 f32.const 18 call $icon_write_line_scaled local.set $count
    local.get $out local.get $cap local.get $count local.get $fit_x local.get $fit_y local.get $scale local.get $dpr f32.const 6 f32.const 18 f32.const 6 f32.const 6 call $icon_write_line_scaled)
