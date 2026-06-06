  (func $svg_write_op1 (param $out i32) (param $cap i32) (param $count i32) (param $op f32) (result i32)
    local.get $count i32.const 1 i32.add i32.const 4 i32.mul local.get $cap i32.gt_u
    if i32.const -1 return end
    local.get $out local.get $count i32.const 4 i32.mul i32.add local.get $op f32.store
    local.get $count i32.const 1 i32.add)

  (func $svg_write_op2 (param $out i32) (param $cap i32) (param $count i32) (param $op f32) (param $a f32) (result i32)
    (local $p i32)
    local.get $count i32.const 2 i32.add i32.const 4 i32.mul local.get $cap i32.gt_u
    if i32.const -1 return end
    local.get $out local.get $count i32.const 4 i32.mul i32.add local.set $p
    local.get $p local.get $op f32.store
    local.get $p i32.const 4 i32.add local.get $a f32.store
    local.get $count i32.const 2 i32.add)

  (func $svg_write_op4 (param $out i32) (param $cap i32) (param $count i32) (param $op f32) (param $a f32) (param $b f32) (param $c f32) (result i32)
    (local $p i32)
    local.get $count i32.const 4 i32.add i32.const 4 i32.mul local.get $cap i32.gt_u
    if i32.const -1 return end
    local.get $out local.get $count i32.const 4 i32.mul i32.add local.set $p
    local.get $p local.get $op f32.store
    local.get $p i32.const 4 i32.add local.get $a f32.store
    local.get $p i32.const 8 i32.add local.get $b f32.store
    local.get $p i32.const 12 i32.add local.get $c f32.store
    local.get $count i32.const 4 i32.add)

  (func $svg_write_op3 (param $out i32) (param $cap i32) (param $count i32) (param $op f32) (param $x f32) (param $y f32) (result i32)
    (local $p i32)
    local.get $count i32.const 3 i32.add i32.const 4 i32.mul local.get $cap i32.gt_u
    if i32.const -1 return end
    local.get $out local.get $count i32.const 4 i32.mul i32.add local.set $p
    local.get $p local.get $op f32.store
    local.get $p i32.const 4 i32.add local.get $x f32.store
    local.get $p i32.const 8 i32.add local.get $y f32.store
    local.get $count i32.const 3 i32.add)

  (func $svg_write_op5 (param $out i32) (param $cap i32) (param $count i32) (param $op f32) (param $x1 f32) (param $y1 f32) (param $x2 f32) (param $y2 f32) (result i32)
    (local $p i32)
    local.get $count i32.const 5 i32.add i32.const 4 i32.mul local.get $cap i32.gt_u
    if i32.const -1 return end
    local.get $out local.get $count i32.const 4 i32.mul i32.add local.set $p
    local.get $p local.get $op f32.store
    local.get $p i32.const 4 i32.add local.get $x1 f32.store
    local.get $p i32.const 8 i32.add local.get $y1 f32.store
    local.get $p i32.const 12 i32.add local.get $x2 f32.store
    local.get $p i32.const 16 i32.add local.get $y2 f32.store
    local.get $count i32.const 5 i32.add)

  (func $svg_write_op6 (param $out i32) (param $cap i32) (param $count i32) (param $op f32) (param $a f32) (param $b f32) (param $c f32) (param $d f32) (param $e f32) (result i32)
    (local $p i32)
    local.get $count i32.const 6 i32.add i32.const 4 i32.mul local.get $cap i32.gt_u
    if i32.const -1 return end
    local.get $out local.get $count i32.const 4 i32.mul i32.add local.set $p
    local.get $p local.get $op f32.store
    local.get $p i32.const 4 i32.add local.get $a f32.store
    local.get $p i32.const 8 i32.add local.get $b f32.store
    local.get $p i32.const 12 i32.add local.get $c f32.store
    local.get $p i32.const 16 i32.add local.get $d f32.store
    local.get $p i32.const 20 i32.add local.get $e f32.store
    local.get $count i32.const 6 i32.add)

  (func $svg_write_op7 (param $out i32) (param $cap i32) (param $count i32) (param $op f32) (param $x1 f32) (param $y1 f32) (param $x2 f32) (param $y2 f32) (param $x3 f32) (param $y3 f32) (result i32)
    (local $p i32)
    local.get $count i32.const 7 i32.add i32.const 4 i32.mul local.get $cap i32.gt_u
    if i32.const -1 return end
    local.get $out local.get $count i32.const 4 i32.mul i32.add local.set $p
    local.get $p local.get $op f32.store
    local.get $p i32.const 4 i32.add local.get $x1 f32.store
    local.get $p i32.const 8 i32.add local.get $y1 f32.store
    local.get $p i32.const 12 i32.add local.get $x2 f32.store
    local.get $p i32.const 16 i32.add local.get $y2 f32.store
    local.get $p i32.const 20 i32.add local.get $x3 f32.store
    local.get $p i32.const 24 i32.add local.get $y3 f32.store
    local.get $count i32.const 7 i32.add)

  (func $svg_write_op8 (param $out i32) (param $cap i32) (param $count i32) (param $op f32) (param $a f32) (param $b f32) (param $c f32) (param $d f32) (param $e f32) (param $f f32) (param $g f32) (result i32)
    (local $p i32)
    local.get $count i32.const 8 i32.add i32.const 4 i32.mul local.get $cap i32.gt_u
    if i32.const -1 return end
    local.get $out local.get $count i32.const 4 i32.mul i32.add local.set $p
    local.get $p local.get $op f32.store
    local.get $p i32.const 4 i32.add local.get $a f32.store
    local.get $p i32.const 8 i32.add local.get $b f32.store
    local.get $p i32.const 12 i32.add local.get $c f32.store
    local.get $p i32.const 16 i32.add local.get $d f32.store
    local.get $p i32.const 20 i32.add local.get $e f32.store
    local.get $p i32.const 24 i32.add local.get $f f32.store
    local.get $p i32.const 28 i32.add local.get $g f32.store
    local.get $count i32.const 8 i32.add)

  (func $svg_style_emit_paint (param $style i32) (param $which i32) (param $out i32) (param $cap i32) (param $count i32) (result i32)
    (local $paint i32)
    local.get $style local.get $which call $er_ui_svg_style_resolve_paint local.tee $paint
    i32.const -2
    i32.eq
    if
      local.get $count
      return
    end
    local.get $paint i32.const -1 i32.eq
    if
      i32.const -1
      return
    end
    local.get $out local.get $cap local.get $count f32.const 17
      local.get $paint i32.const 255 i32.and f32.convert_i32_u
      local.get $paint i32.const 8 i32.shr_u i32.const 255 i32.and f32.convert_i32_u
      local.get $paint i32.const 16 i32.shr_u i32.const 255 i32.and f32.convert_i32_u
      local.get $paint i32.const 24 i32.shr_u i32.const 255 i32.and f32.convert_i32_u
      call $svg_write_op5)

  (func $er_ui_svg_path_style_parse_to_ir_transform_impl (export "er_ui_svg_path_style_parse_to_ir_transform") (param $ptr i32) (param $len i32) (param $out i32) (param $cap i32) (param $min_x f32) (param $min_y f32) (param $vw f32) (param $vh f32) (param $style i32) (param $a f32) (param $b f32) (param $c f32) (param $d f32) (param $tx f32) (param $ty f32) (result i32)
    (local $count i32) (local $body i32) (local $paint i32)
    local.get $ptr i32.eqz
    local.get $out i32.eqz i32.or
    local.get $style i32.eqz i32.or
    if i32.const -1 return end
    local.get $style i32.const 60 i32.add i32.load i32.eqz
    local.get $style i32.const 64 i32.add i32.load i32.eqz i32.or
    if i32.const 0 return end
    local.get $style i32.const 0 call $er_ui_svg_style_resolve_paint local.set $paint
    local.get $paint i32.const -1 i32.eq if i32.const -1 return end
    local.get $paint i32.const -2 i32.ne
    if
      local.get $style i32.const 0 local.get $out local.get $cap local.get $count call $svg_style_emit_paint local.tee $count i32.const -1 i32.eq if i32.const -1 return end
      local.get $out local.get $cap local.get $count
      local.get $style i32.const 36 i32.add i32.load
      if (result f32)
        f32.const 16
      else
        f32.const 14
      end
      call $svg_write_op1 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
      local.get $ptr local.get $len
      local.get $out local.get $count i32.const 4 i32.mul i32.add
      local.get $cap local.get $count i32.const 4 i32.mul i32.sub
      local.get $min_x local.get $min_y local.get $vw local.get $vh
      local.get $a local.get $b local.get $c local.get $d local.get $tx local.get $ty
      call $er_ui_svg_path_parse_to_ir_transform_impl local.tee $body i32.const -1 i32.eq if i32.const -1 return end
      local.get $count local.get $body i32.add local.set $count
      local.get $out local.get $cap local.get $count f32.const 15 call $svg_write_op1 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
    end
    local.get $style i32.const 1 call $er_ui_svg_style_resolve_paint local.set $paint
    local.get $paint i32.const -1 i32.eq if i32.const -1 return end
    local.get $paint i32.const -2 i32.ne
    if
      local.get $style i32.const 1 local.get $out local.get $cap local.get $count call $svg_style_emit_paint local.tee $count i32.const -1 i32.eq if i32.const -1 return end
      local.get $out local.get $cap local.get $count f32.const 22 local.get $style i32.const 12 i32.add f32.load call $svg_write_op2 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
      local.get $out local.get $cap local.get $count f32.const 23 local.get $style i32.const 28 i32.add i32.load f32.convert_i32_s call $svg_write_op2 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
      local.get $out local.get $cap local.get $count f32.const 24 local.get $style i32.const 32 i32.add i32.load f32.convert_i32_s call $svg_write_op2 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
      local.get $out local.get $cap local.get $count f32.const 25 local.get $style i32.const 40 i32.add f32.load call $svg_write_op2 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
      local.get $style i32.load i32.const 1024 i32.and
      if
        local.get $out local.get $cap local.get $count f32.const 30
          local.get $style i32.const 44 i32.add f32.load
          local.get $style i32.const 48 i32.add f32.load
          local.get $style i32.const 52 i32.add f32.load
          call $svg_write_op4 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
      end
      local.get $ptr local.get $len
      local.get $out local.get $count i32.const 4 i32.mul i32.add
      local.get $cap local.get $count i32.const 4 i32.mul i32.sub
      local.get $min_x local.get $min_y local.get $vw local.get $vh
      local.get $a local.get $b local.get $c local.get $d local.get $tx local.get $ty
      call $er_ui_svg_path_parse_to_ir_transform_impl local.tee $body i32.const -1 i32.eq if i32.const -1 return end
      local.get $count local.get $body i32.add local.set $count
    end
    local.get $count)

  (func (export "er_ui_svg_path_style_parse_to_ir") (param $ptr i32) (param $len i32) (param $out i32) (param $cap i32) (param $min_x f32) (param $min_y f32) (param $vw f32) (param $vh f32) (param $style i32) (result i32)
    local.get $ptr
    local.get $len
    local.get $out
    local.get $cap
    local.get $min_x
    local.get $min_y
    local.get $vw
    local.get $vh
    local.get $style
    f32.const 1
    f32.const 0
    f32.const 0
    f32.const 1
    f32.const 0
    f32.const 0
    call $er_ui_svg_path_style_parse_to_ir_transform_impl)

  (func $svg_style_emit_stroke_state (param $style i32) (param $out i32) (param $cap i32) (param $count i32) (result i32)
    local.get $out local.get $cap local.get $count f32.const 22 local.get $style i32.const 12 i32.add f32.load call $svg_write_op2 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
    local.get $out local.get $cap local.get $count f32.const 23 local.get $style i32.const 28 i32.add i32.load f32.convert_i32_s call $svg_write_op2 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
    local.get $out local.get $cap local.get $count f32.const 24 local.get $style i32.const 32 i32.add i32.load f32.convert_i32_s call $svg_write_op2 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
    local.get $out local.get $cap local.get $count f32.const 25 local.get $style i32.const 40 i32.add f32.load call $svg_write_op2)

  (func (export "er_ui_svg_circle_style_emit_ir") (param $out i32) (param $cap i32) (param $cx f32) (param $cy f32) (param $r f32) (param $style i32) (result i32)
    (local $count i32) (local $paint i32)
    local.get $out i32.eqz
    local.get $style i32.eqz i32.or
    local.get $r f32.const 0 f32.le i32.or
    if i32.const -1 return end
    local.get $style i32.const 60 i32.add i32.load i32.eqz
    local.get $style i32.const 64 i32.add i32.load i32.eqz i32.or
    if i32.const 0 return end
    local.get $style i32.const 0 call $er_ui_svg_style_resolve_paint local.set $paint
    local.get $paint i32.const -1 i32.eq if i32.const -1 return end
    local.get $paint i32.const -2 i32.ne
    if
      local.get $style i32.const 0 local.get $out local.get $cap local.get $count call $svg_style_emit_paint local.tee $count i32.const -1 i32.eq if i32.const -1 return end
      local.get $out local.get $cap local.get $count f32.const 5 local.get $cx local.get $cy local.get $r call $svg_write_op4 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
    end
    local.get $style i32.const 1 call $er_ui_svg_style_resolve_paint local.set $paint
    local.get $paint i32.const -1 i32.eq if i32.const -1 return end
    local.get $paint i32.const -2 i32.ne
    if
      local.get $style i32.const 1 local.get $out local.get $cap local.get $count call $svg_style_emit_paint local.tee $count i32.const -1 i32.eq if i32.const -1 return end
      local.get $style local.get $out local.get $cap local.get $count call $svg_style_emit_stroke_state local.tee $count i32.const -1 i32.eq if i32.const -1 return end
      local.get $out local.get $cap local.get $count f32.const 2 local.get $cx local.get $cy local.get $r call $svg_write_op4 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
    end
    local.get $count)

  (func (export "er_ui_svg_ellipse_style_emit_ir") (param $out i32) (param $cap i32) (param $cx f32) (param $cy f32) (param $rx f32) (param $ry f32) (param $style i32) (result i32)
    (local $count i32) (local $paint i32)
    local.get $out i32.eqz
    local.get $style i32.eqz i32.or
    local.get $rx f32.const 0 f32.le i32.or
    local.get $ry f32.const 0 f32.le i32.or
    if i32.const -1 return end
    local.get $style i32.const 60 i32.add i32.load i32.eqz
    local.get $style i32.const 64 i32.add i32.load i32.eqz i32.or
    if i32.const 0 return end
    local.get $style i32.const 0 call $er_ui_svg_style_resolve_paint local.set $paint
    local.get $paint i32.const -1 i32.eq if i32.const -1 return end
    local.get $paint i32.const -2 i32.ne
    if
      local.get $style i32.const 0 local.get $out local.get $cap local.get $count call $svg_style_emit_paint local.tee $count i32.const -1 i32.eq if i32.const -1 return end
      local.get $out local.get $cap local.get $count f32.const 12 local.get $cx local.get $cy local.get $rx local.get $ry call $svg_write_op5 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
    end
    local.get $style i32.const 1 call $er_ui_svg_style_resolve_paint local.set $paint
    local.get $paint i32.const -1 i32.eq if i32.const -1 return end
    local.get $paint i32.const -2 i32.ne
    if
      local.get $style i32.const 1 local.get $out local.get $cap local.get $count call $svg_style_emit_paint local.tee $count i32.const -1 i32.eq if i32.const -1 return end
      local.get $style local.get $out local.get $cap local.get $count call $svg_style_emit_stroke_state local.tee $count i32.const -1 i32.eq if i32.const -1 return end
      local.get $out local.get $cap local.get $count f32.const 3 local.get $cx local.get $cy local.get $rx local.get $ry call $svg_write_op5 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
    end
    local.get $count)

  (func (export "er_ui_svg_rect_style_emit_ir") (param $out i32) (param $cap i32) (param $x f32) (param $y f32) (param $w f32) (param $h f32) (param $r f32) (param $style i32) (result i32)
    (local $count i32) (local $paint i32)
    local.get $out i32.eqz
    local.get $style i32.eqz i32.or
    local.get $w f32.const 0 f32.le i32.or
    local.get $h f32.const 0 f32.le i32.or
    if i32.const -1 return end
    local.get $style i32.const 60 i32.add i32.load i32.eqz
    local.get $style i32.const 64 i32.add i32.load i32.eqz i32.or
    if i32.const 0 return end
    local.get $style i32.const 0 call $er_ui_svg_style_resolve_paint local.set $paint
    local.get $paint i32.const -1 i32.eq if i32.const -1 return end
    local.get $paint i32.const -2 i32.ne
    if
      local.get $style i32.const 0 local.get $out local.get $cap local.get $count call $svg_style_emit_paint local.tee $count i32.const -1 i32.eq if i32.const -1 return end
      local.get $out local.get $cap local.get $count f32.const 13 local.get $x local.get $y local.get $w local.get $h local.get $r call $svg_write_op6 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
    end
    local.get $style i32.const 1 call $er_ui_svg_style_resolve_paint local.set $paint
    local.get $paint i32.const -1 i32.eq if i32.const -1 return end
    local.get $paint i32.const -2 i32.ne
    if
      local.get $style i32.const 1 local.get $out local.get $cap local.get $count call $svg_style_emit_paint local.tee $count i32.const -1 i32.eq if i32.const -1 return end
      local.get $style local.get $out local.get $cap local.get $count call $svg_style_emit_stroke_state local.tee $count i32.const -1 i32.eq if i32.const -1 return end
      local.get $out local.get $cap local.get $count f32.const 4 local.get $x local.get $y local.get $w local.get $h local.get $r call $svg_write_op6 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
    end
    local.get $count)

  (func (export "er_ui_svg_line_style_emit_ir") (param $out i32) (param $cap i32) (param $x1 f32) (param $y1 f32) (param $x2 f32) (param $y2 f32) (param $style i32) (result i32)
    (local $count i32) (local $paint i32)
    local.get $out i32.eqz
    local.get $style i32.eqz i32.or
    if i32.const -1 return end
    local.get $style i32.const 60 i32.add i32.load i32.eqz
    local.get $style i32.const 64 i32.add i32.load i32.eqz i32.or
    if i32.const 0 return end
    local.get $style i32.const 1 call $er_ui_svg_style_resolve_paint local.set $paint
    local.get $paint i32.const -1 i32.eq if i32.const -1 return end
    local.get $paint i32.const -2 i32.eq if i32.const 0 return end
    local.get $style i32.const 1 local.get $out local.get $cap local.get $count call $svg_style_emit_paint local.tee $count i32.const -1 i32.eq if i32.const -1 return end
    local.get $style local.get $out local.get $cap local.get $count call $svg_style_emit_stroke_state local.tee $count i32.const -1 i32.eq if i32.const -1 return end
    local.get $style i32.load i32.const 1024 i32.and
    if
      local.get $out local.get $cap local.get $count f32.const 30
        local.get $style i32.const 44 i32.add f32.load
        local.get $style i32.const 48 i32.add f32.load
        local.get $style i32.const 52 i32.add f32.load
        call $svg_write_op4 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
    end
    local.get $out local.get $cap local.get $count f32.const 6 local.get $x1 local.get $y1 call $svg_write_op3 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
    local.get $out local.get $cap local.get $count f32.const 7 local.get $x2 local.get $y2 call $svg_write_op3 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
    local.get $count)

  (func (export "er_ui_svg_polyline_style_emit_ir") (param $points i32) (param $point_count i32) (param $out i32) (param $cap i32) (param $style i32) (result i32)
    (local $count i32) (local $paint i32) (local $i i32) (local $p i32) (local $needed i32)
    local.get $points i32.eqz
    local.get $out i32.eqz i32.or
    local.get $style i32.eqz i32.or
    local.get $point_count i32.const 2 i32.lt_u i32.or
    if i32.const -1 return end
    local.get $style i32.const 60 i32.add i32.load i32.eqz
    local.get $style i32.const 64 i32.add i32.load i32.eqz i32.or
    if i32.const 0 return end
    local.get $style i32.const 1 call $er_ui_svg_style_resolve_paint local.set $paint
    local.get $paint i32.const -1 i32.eq if i32.const -1 return end
    local.get $paint i32.const -2 i32.eq if i32.const 0 return end
    local.get $style i32.const 1 local.get $out local.get $cap local.get $count call $svg_style_emit_paint local.tee $count i32.const -1 i32.eq if i32.const -1 return end
    local.get $style local.get $out local.get $cap local.get $count call $svg_style_emit_stroke_state local.tee $count i32.const -1 i32.eq if i32.const -1 return end
    local.get $style i32.load i32.const 1024 i32.and
    if
      local.get $out local.get $cap local.get $count f32.const 30
        local.get $style i32.const 44 i32.add f32.load
        local.get $style i32.const 48 i32.add f32.load
        local.get $style i32.const 52 i32.add f32.load
        call $svg_write_op4 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
    end
    local.get $point_count i32.const 2 i32.mul i32.const 2 i32.add local.set $needed
    local.get $count local.get $needed i32.add i32.const 4 i32.mul local.get $cap i32.gt_u
    if i32.const -1 return end
    local.get $out local.get $count i32.const 4 i32.mul i32.add local.set $p
    local.get $p f32.const 1 f32.store
    local.get $p i32.const 4 i32.add local.get $point_count f32.convert_i32_u f32.store
    i32.const 0 local.set $i
    block $done
      loop $loop
        local.get $i local.get $point_count i32.ge_u br_if $done
        local.get $p i32.const 8 i32.add local.get $i i32.const 8 i32.mul i32.add
          local.get $points local.get $i i32.const 8 i32.mul i32.add f32.load
          f32.store
        local.get $p i32.const 12 i32.add local.get $i i32.const 8 i32.mul i32.add
          local.get $points local.get $i i32.const 8 i32.mul i32.add i32.const 4 i32.add f32.load
          f32.store
        local.get $i i32.const 1 i32.add local.set $i
        br $loop
      end
    end
    local.get $count local.get $needed i32.add)

  (func (export "er_ui_svg_polygon_style_emit_ir") (param $points i32) (param $point_count i32) (param $out i32) (param $cap i32) (param $style i32) (result i32)
    (local $count i32) (local $paint i32) (local $i i32)
    local.get $points i32.eqz
    local.get $out i32.eqz i32.or
    local.get $style i32.eqz i32.or
    local.get $point_count i32.const 3 i32.lt_u i32.or
    if i32.const -1 return end
    local.get $style i32.const 60 i32.add i32.load i32.eqz
    local.get $style i32.const 64 i32.add i32.load i32.eqz i32.or
    if i32.const 0 return end

    local.get $style i32.const 0 call $er_ui_svg_style_resolve_paint local.set $paint
    local.get $paint i32.const -1 i32.eq if i32.const -1 return end
    local.get $paint i32.const -2 i32.ne
    if
      local.get $style i32.const 0 local.get $out local.get $cap local.get $count call $svg_style_emit_paint local.tee $count i32.const -1 i32.eq if i32.const -1 return end
      local.get $out local.get $cap local.get $count
      local.get $style i32.const 36 i32.add i32.load
      if (result f32)
        f32.const 16
      else
        f32.const 14
      end
      call $svg_write_op1 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
      local.get $out local.get $cap local.get $count f32.const 6
        local.get $points f32.load
        local.get $points i32.const 4 i32.add f32.load
        call $svg_write_op3 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
      i32.const 1 local.set $i
      block $fill_done
        loop $fill_loop
          local.get $i local.get $point_count i32.ge_u br_if $fill_done
          local.get $out local.get $cap local.get $count f32.const 7
            local.get $points local.get $i i32.const 8 i32.mul i32.add f32.load
            local.get $points local.get $i i32.const 8 i32.mul i32.add i32.const 4 i32.add f32.load
            call $svg_write_op3 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
          local.get $i i32.const 1 i32.add local.set $i
          br $fill_loop
        end
      end
      local.get $out local.get $cap local.get $count f32.const 11 call $svg_write_op1 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
      local.get $out local.get $cap local.get $count f32.const 15 call $svg_write_op1 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
    end

    local.get $style i32.const 1 call $er_ui_svg_style_resolve_paint local.set $paint
    local.get $paint i32.const -1 i32.eq if i32.const -1 return end
    local.get $paint i32.const -2 i32.ne
    if
      local.get $style i32.const 1 local.get $out local.get $cap local.get $count call $svg_style_emit_paint local.tee $count i32.const -1 i32.eq if i32.const -1 return end
      local.get $style local.get $out local.get $cap local.get $count call $svg_style_emit_stroke_state local.tee $count i32.const -1 i32.eq if i32.const -1 return end
      local.get $style i32.load i32.const 1024 i32.and
      if
        local.get $out local.get $cap local.get $count f32.const 30
          local.get $style i32.const 44 i32.add f32.load
          local.get $style i32.const 48 i32.add f32.load
          local.get $style i32.const 52 i32.add f32.load
          call $svg_write_op4 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
      end
      local.get $out local.get $cap local.get $count f32.const 6
        local.get $points f32.load
        local.get $points i32.const 4 i32.add f32.load
        call $svg_write_op3 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
      i32.const 1 local.set $i
      block $stroke_done
        loop $stroke_loop
          local.get $i local.get $point_count i32.ge_u br_if $stroke_done
          local.get $out local.get $cap local.get $count f32.const 7
            local.get $points local.get $i i32.const 8 i32.mul i32.add f32.load
            local.get $points local.get $i i32.const 8 i32.mul i32.add i32.const 4 i32.add f32.load
            call $svg_write_op3 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
          local.get $i i32.const 1 i32.add local.set $i
          br $stroke_loop
        end
      end
      local.get $out local.get $cap local.get $count f32.const 11 call $svg_write_op1 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
    end
    local.get $count)
