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
