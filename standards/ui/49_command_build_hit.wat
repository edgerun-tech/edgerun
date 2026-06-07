(func (export "er_ui_command_write_rect") (param $out i32) (param $cap i32) (param $count i32) (param $x f32) (param $y f32) (param $w f32) (param $h f32) (param $color i32) (result i32)
    local.get $w
    local.get $h
    call $valid_rect
    i32.eqz
    if
      i32.const -1
      return
    end
    local.get $out
    local.get $cap
    local.get $count
    local.get $x
    local.get $y
    local.get $w
    local.get $h
    local.get $color
    call $emit_rect)

  (func (export "er_ui_command_write_text") (param $out i32) (param $cap i32) (param $count i32) (param $x f32) (param $y f32) (param $w f32) (param $h f32) (param $text_ptr i32) (param $text_len i32) (param $color i32) (result i32)
    (local $p i32)
    local.get $count
    i32.const 48
    i32.mul
    i32.const 48
    i32.add
    local.get $cap
    i32.gt_u
    if
      i32.const -1
      return
    end
    local.get $w
    local.get $h
    call $valid_rect
    i32.eqz
    if
      i32.const -1
      return
    end
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
    local.get $text_ptr
    call $store32
    local.get $p
    i32.const 28
    i32.add
    local.get $text_len
    call $store32
    local.get $count
    i32.const 1
    i32.add)

  (func $er_ui_command_write_icon (export "er_ui_command_write_icon") (param $out i32) (param $cap i32) (param $count i32) (param $x f32) (param $y f32) (param $w f32) (param $h f32) (param $icon i32) (param $color i32) (result i32)
    (local $p i32)
    local.get $icon
    call $er_ui_icon_valid
    i32.eqz
    if
      i32.const -1
      return
    end
    local.get $count
    i32.const 48
    i32.mul
    i32.const 48
    i32.add
    local.get $cap
    i32.gt_u
    if
      i32.const -1
      return
    end
    local.get $w
    local.get $h
    call $valid_rect
    i32.eqz
    if
      i32.const -1
      return
    end
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
    call $er_ui_command_tag_icon
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
    local.get $icon
    call $er_ui_icon_path_ptr
    call $store32
    local.get $p
    i32.const 28
    i32.add
    local.get $icon
    call $er_ui_icon_path_len
    call $store32
    local.get $p
    i32.const 44
    i32.add
    local.get $icon
    call $store32
    local.get $count
    i32.const 1
    i32.add)

  (func (export "er_ui_command_write_icon_fit") (param $out i32) (param $cap i32) (param $count i32) (param $x f32) (param $y f32) (param $w f32) (param $h f32) (param $icon i32) (param $color i32) (param $dpr f32) (result i32)
    (local $fit_x f32)
    (local $fit_y f32)
    (local $size f32)
    local.get $w
    local.get $h
    call $valid_rect
    i32.eqz
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
    local.get $out
    local.get $cap
    local.get $count
    local.get $fit_x
    local.get $fit_y
    local.get $size
    local.get $size
    local.get $icon
    local.get $color
    call $er_ui_command_write_icon)

  (func (export "er_ui_command_hit_find") (param $commands i32) (param $count i32) (param $x f32) (param $y f32) (param $out_command i32) (param $out_index i32) (result i32)
    (local $i i32)
    (local $p i32)
    (local $rx f32)
    (local $ry f32)
    (local $rw f32)
    (local $rh f32)
    local.get $count
    local.set $i
    block $done
      loop $loop
        local.get $i
        i32.eqz
        br_if $done
        local.get $i
        i32.const 1
        i32.sub
        local.set $i
        local.get $commands
        local.get $i
        i32.const 48
        i32.mul
        i32.add
        local.set $p
        local.get $p
        i32.load
        i32.const 0
        i32.ne
        if
          local.get $p
          i32.const 8
          i32.add
          f32.load
          local.set $rx
          local.get $p
          i32.const 12
          i32.add
          f32.load
          local.set $ry
          local.get $p
          i32.const 16
          i32.add
          f32.load
          local.set $rw
          local.get $p
          i32.const 20
          i32.add
          f32.load
          local.set $rh
          local.get $x
          local.get $rx
          f32.ge
          local.get $y
          local.get $ry
          f32.ge
          i32.and
          local.get $x
          local.get $rx
          local.get $rw
          f32.add
          f32.lt
          i32.and
          local.get $y
          local.get $ry
          local.get $rh
          f32.add
          f32.lt
          i32.and
          if
            local.get $out_command
            i32.eqz
            i32.eqz
            if
              local.get $out_command
              local.get $p
              i32.const 48
              call $copy
            end
            local.get $out_index
            i32.eqz
            i32.eqz
            if
              local.get $out_index
              local.get $i
              i32.store
            end
            i32.const 1
            return
          end
        end
        br $loop
      end
    end
    i32.const 0)

  (func (export "er_ui_rect_intersect") (param $a i32) (param $b i32) (param $out i32) (result i32)
    (local $x0 f32)
    (local $y0 f32)
    (local $x1 f32)
    (local $y1 f32)
    local.get $a
    f32.load
    local.get $b
    f32.load
    call $max_f32
    local.set $x0
    local.get $a
    i32.const 4
    i32.add
    f32.load
    local.get $b
    i32.const 4
    i32.add
    f32.load
    call $max_f32
    local.set $y0
    local.get $a
    f32.load
    local.get $a
    i32.const 8
    i32.add
    f32.load
    f32.add
    local.get $b
    f32.load
    local.get $b
    i32.const 8
    i32.add
    f32.load
    f32.add
    call $min_f32
    local.set $x1
    local.get $a
    i32.const 4
    i32.add
    f32.load
    local.get $a
    i32.const 12
    i32.add
    f32.load
    f32.add
    local.get $b
    i32.const 4
    i32.add
    f32.load
    local.get $b
    i32.const 12
    i32.add
    f32.load
    f32.add
    call $min_f32
    local.set $y1
    local.get $x1
    local.get $x0
    f32.sub
    f32.const 0
    f32.le
    local.get $y1
    local.get $y0
    f32.sub
    f32.const 0
    f32.le
    i32.or
    if
      i32.const 0
      return
    end
    local.get $out
    local.get $x0
    f32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $y0
    f32.store
    local.get $out
    i32.const 8
    i32.add
    local.get $x1
    local.get $x0
    f32.sub
    f32.store
    local.get $out
    i32.const 12
    i32.add
    local.get $y1
    local.get $y0
    f32.sub
    f32.store
    i32.const 1)
