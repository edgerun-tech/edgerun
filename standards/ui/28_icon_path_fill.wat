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
