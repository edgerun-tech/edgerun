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
