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
