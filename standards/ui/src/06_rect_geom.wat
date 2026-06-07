

  (func $rect_store (param $out i32) (param $x f32) (param $y f32) (param $w f32) (param $h f32) (result i32)
    local.get $out
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $out
    local.get $x
    f32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $y
    f32.store
    local.get $out
    i32.const 8
    i32.add
    local.get $w
    f32.store
    local.get $out
    i32.const 12
    i32.add
    local.get $h
    f32.store
    i32.const 1)

  (func $er_ui_rect_valid  (param $rect i32) (result i32)
    local.get $rect
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $rect f32.load call $finite_f32
    local.get $rect i32.const 4 i32.add f32.load call $finite_f32 i32.and
    local.get $rect i32.const 8 i32.add f32.load call $finite_f32 i32.and
    local.get $rect i32.const 12 i32.add f32.load call $finite_f32 i32.and
    local.get $rect i32.const 8 i32.add f32.load f32.const 0 f32.gt i32.and
    local.get $rect i32.const 12 i32.add f32.load f32.const 0 f32.gt i32.and)

  (func $er_ui_rect_usable  (param $rect i32) (result i32)
    local.get $rect
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $rect f32.load call $finite_f32
    local.get $rect i32.const 4 i32.add f32.load call $finite_f32 i32.and
    local.get $rect i32.const 8 i32.add f32.load call $finite_f32 i32.and
    local.get $rect i32.const 12 i32.add f32.load call $finite_f32 i32.and
    local.get $rect i32.const 8 i32.add f32.load f32.const 0 f32.ge i32.and
    local.get $rect i32.const 12 i32.add f32.load f32.const 0 f32.ge i32.and)

  (func $er_ui_rect_inset  (param $rect i32) (param $out i32) (param $dx f32) (param $dy f32) (result i32)
    local.get $rect
    i32.eqz
    local.get $out
    i32.eqz
    i32.or
    if
      i32.const 0
      return
    end
    local.get $out
    local.get $rect f32.load local.get $dx f32.add
    local.get $rect i32.const 4 i32.add f32.load local.get $dy f32.add
    local.get $rect i32.const 8 i32.add f32.load local.get $dx f32.const 2 f32.mul f32.sub f32.const 0 call $max_f32
    local.get $rect i32.const 12 i32.add f32.load local.get $dy f32.const 2 f32.mul f32.sub f32.const 0 call $max_f32
    call $rect_store)

  (func $er_ui_rect_inset_uniform  (param $rect i32) (param $out i32) (param $amount f32) (result i32)
    local.get $rect
    local.get $out
    local.get $amount
    local.get $amount
    call $er_ui_rect_inset)

  (func $er_ui_rect_inset_ltrb  (param $rect i32) (param $out i32) (param $left f32) (param $top f32) (param $right f32) (param $bottom f32) (result i32)
    local.get $rect
    i32.eqz
    local.get $out
    i32.eqz
    i32.or
    if
      i32.const 0
      return
    end
    local.get $out
    local.get $rect f32.load local.get $left f32.add
    local.get $rect i32.const 4 i32.add f32.load local.get $top f32.add
    local.get $rect i32.const 8 i32.add f32.load local.get $left f32.sub local.get $right f32.sub f32.const 0 call $max_f32
    local.get $rect i32.const 12 i32.add f32.load local.get $top f32.sub local.get $bottom f32.sub f32.const 0 call $max_f32
    call $rect_store)

  (func $er_ui_rect_with_height_centered  (param $rect i32) (param $out i32) (param $height f32) (result i32)
    (local $h f32)
    local.get $rect i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $height f32.const 0 local.get $rect i32.const 12 i32.add f32.load call $clamp_f32 local.set $h
    local.get $out
    local.get $rect f32.load
    local.get $rect i32.const 4 i32.add f32.load local.get $rect i32.const 12 i32.add f32.load local.get $h f32.sub f32.const 0.5 f32.mul f32.add
    local.get $rect i32.const 8 i32.add f32.load
    local.get $h
    call $rect_store)

  (func $er_ui_rect_with_width_centered  (param $rect i32) (param $out i32) (param $width f32) (result i32)
    (local $w f32)
    local.get $rect i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $width f32.const 0 local.get $rect i32.const 8 i32.add f32.load call $clamp_f32 local.set $w
    local.get $out
    local.get $rect f32.load local.get $rect i32.const 8 i32.add f32.load local.get $w f32.sub f32.const 0.5 f32.mul f32.add
    local.get $rect i32.const 4 i32.add f32.load
    local.get $w
    local.get $rect i32.const 12 i32.add f32.load
    call $rect_store)

  (func $er_ui_rect_right  (param $rect i32) (param $out i32) (param $width f32) (result i32)
    local.get $rect i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $rect f32.load local.get $rect i32.const 8 i32.add f32.load f32.add local.get $width f32.sub
    local.get $rect i32.const 4 i32.add f32.load
    local.get $width
    local.get $rect i32.const 12 i32.add f32.load
    call $rect_store)

  (func $er_ui_rect_bottom  (param $rect i32) (param $out i32) (param $height f32) (result i32)
    local.get $rect i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $rect f32.load
    local.get $rect i32.const 4 i32.add f32.load local.get $rect i32.const 12 i32.add f32.load f32.add local.get $height f32.sub
    local.get $rect i32.const 8 i32.add f32.load
    local.get $height
    call $rect_store)

  (func $er_ui_rect_contains_inclusive  (param $rect i32) (param $x f32) (param $y f32) (result i32)
    local.get $rect i32.eqz if i32.const 0 return end
    local.get $x local.get $rect f32.load f32.ge
    local.get $y local.get $rect i32.const 4 i32.add f32.load f32.ge i32.and
    local.get $x local.get $rect f32.load local.get $rect i32.const 8 i32.add f32.load f32.add f32.le i32.and
    local.get $y local.get $rect i32.const 4 i32.add f32.load local.get $rect i32.const 12 i32.add f32.load f32.add f32.le i32.and)

  (func $er_ui_rect_contains_exclusive  (param $rect i32) (param $x f32) (param $y f32) (result i32)
    local.get $rect i32.eqz if i32.const 0 return end
    local.get $x local.get $rect f32.load f32.ge
    local.get $y local.get $rect i32.const 4 i32.add f32.load f32.ge i32.and
    local.get $x local.get $rect f32.load local.get $rect i32.const 8 i32.add f32.load f32.add f32.lt i32.and
    local.get $y local.get $rect i32.const 4 i32.add f32.load local.get $rect i32.const 12 i32.add f32.load f32.add f32.lt i32.and)

  (func $er_ui_rect_contains  (param $rect i32) (param $x f32) (param $y f32) (result i32)
    local.get $rect i32.eqz if i32.const 0 return end
    local.get $x
    local.get $rect
    f32.load
    f32.ge
    local.get $y
    local.get $rect
    i32.const 4
    i32.add
    f32.load
    f32.ge
    i32.and
    local.get $x
    local.get $rect
    f32.load
    local.get $rect
    i32.const 8
    i32.add
    f32.load
    f32.add
    f32.lt
    i32.and
    local.get $y
    local.get $rect
    i32.const 4
    i32.add
    f32.load
    local.get $rect
    i32.const 12
    i32.add
    f32.load
    f32.add
    f32.lt
    i32.and)

  (func $er_ui_rect_intersect  (param $a i32) (param $b i32) (param $out i32) (result i32)
    (local $x0 f32)
    (local $y0 f32)
    (local $x1 f32)
    (local $y1 f32)
    local.get $a i32.eqz local.get $b i32.eqz i32.or local.get $out i32.eqz i32.or if i32.const 0 return end
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
