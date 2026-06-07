
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

  (func $er_ui_snap_stroke_center  (param $v f32) (param $dpr f32) (result f32)
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
