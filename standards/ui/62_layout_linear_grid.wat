(func $er_ui_layout_linear_child (export "er_ui_layout_linear_child") (param $out i32) (param $x f32) (param $y f32) (param $w f32) (param $h f32) (param $children i32) (param $index i32) (param $axis i32) (param $gap f32) (param $padding f32) (result i32)
    (local $content_x f32)
    (local $content_y f32)
    (local $content_w f32)
    (local $content_h f32)
    (local $step f32)
    local.get $children
    i32.eqz
    local.get $index
    local.get $children
    i32.ge_u
    i32.or
    if
      i32.const 0
      return
    end
    local.get $x
    local.get $padding
    f32.add
    local.set $content_x
    local.get $y
    local.get $padding
    f32.add
    local.set $content_y
    local.get $w
    local.get $padding
    f32.const 2
    f32.mul
    f32.sub
    local.set $content_w
    local.get $h
    local.get $padding
    f32.const 2
    f32.mul
    f32.sub
    local.set $content_h
    local.get $axis
    i32.const 1
    i32.gt_u
    local.get $content_w
    f32.const 0
    f32.le
    i32.or
    local.get $content_h
    f32.const 0
    f32.le
    i32.or
    if
      i32.const 0
      return
    end
    local.get $axis
    i32.eqz
    if
      local.get $content_w
      local.get $gap
      local.get $children
      i32.const 1
      i32.sub
      f32.convert_i32_u
      f32.mul
      f32.sub
      local.get $children
      f32.convert_i32_u
      f32.div
      local.set $step
      local.get $step
      f32.const 0
      f32.le
      if
        i32.const 0
        return
      end
      local.get $out
      local.get $content_x
      local.get $step
      local.get $gap
      f32.add
      local.get $index
      f32.convert_i32_u
      f32.mul
      f32.add
      f32.store
      local.get $out
      i32.const 4
      i32.add
      local.get $content_y
      f32.store
      local.get $out
      i32.const 8
      i32.add
      local.get $step
      f32.store
      local.get $out
      i32.const 12
      i32.add
      local.get $content_h
      f32.store
    else
      local.get $content_h
      local.get $gap
      local.get $children
      i32.const 1
      i32.sub
      f32.convert_i32_u
      f32.mul
      f32.sub
      local.get $children
      f32.convert_i32_u
      f32.div
      local.set $step
      local.get $step
      f32.const 0
      f32.le
      if
        i32.const 0
        return
      end
      local.get $out
      local.get $content_x
      f32.store
      local.get $out
      i32.const 4
      i32.add
      local.get $content_y
      local.get $step
      local.get $gap
      f32.add
      local.get $index
      f32.convert_i32_u
      f32.mul
      f32.add
      f32.store
      local.get $out
      i32.const 8
      i32.add
      local.get $content_w
      f32.store
      local.get $out
      i32.const 12
      i32.add
      local.get $step
      f32.store
    end
    i32.const 1)

  (func (export "er_ui_layout_scrolled_child") (param $out i32) (param $x f32) (param $y f32) (param $w f32) (param $h f32) (param $children i32) (param $index i32) (param $gap f32) (param $padding f32) (param $scroll_y f32) (result i32)
    local.get $out
    local.get $x
    local.get $y
    local.get $scroll_y
    f32.sub
    local.get $w
    local.get $h
    local.get $children
    local.get $index
    i32.const 1
    local.get $gap
    local.get $padding
    call $er_ui_layout_linear_child)

  (func (export "er_ui_layout_scratch_base") (result i32)
    i32.const 120000)

  (func (export "er_ui_layout_scratch_size") (result i32)
    i32.const 1024)

  (func (export "er_ui_layout_grid_child") (param $out i32) (param $x f32) (param $y f32) (param $w f32) (param $h f32) (param $children i32) (param $index i32) (param $columns i32) (param $gap f32) (param $padding f32) (result i32)
    (local $rows i32)
    (local $col i32)
    (local $row i32)
    (local $cw f32)
    (local $ch f32)
    local.get $children
    i32.eqz
    local.get $index
    local.get $children
    i32.ge_u
    i32.or
    if
      i32.const 0
      return
    end
    local.get $columns
    i32.eqz
    if
      i32.const 1
      local.set $columns
    end
    local.get $columns
    local.get $children
    i32.gt_u
    if
      local.get $children
      local.set $columns
    end
    local.get $children
    local.get $columns
    i32.add
    i32.const 1
    i32.sub
    local.get $columns
    i32.div_u
    local.set $rows
    local.get $index
    local.get $columns
    i32.rem_u
    local.set $col
    local.get $index
    local.get $columns
    i32.div_u
    local.set $row
    local.get $w
    local.get $padding
    f32.const 2
    f32.mul
    f32.sub
    local.get $gap
    local.get $columns
    i32.const 1
    i32.sub
    f32.convert_i32_u
    f32.mul
    f32.sub
    local.get $columns
    f32.convert_i32_u
    f32.div
    local.set $cw
    local.get $cw
    f32.const 0
    f32.le
    if
      i32.const 0
      return
    end
    local.get $h
    local.get $padding
    f32.const 2
    f32.mul
    f32.sub
    local.get $gap
    local.get $rows
    i32.const 1
    i32.sub
    f32.convert_i32_u
    f32.mul
    f32.sub
    local.get $rows
    f32.convert_i32_u
    f32.div
    local.set $ch
    local.get $ch
    f32.const 0
    f32.le
    if
      i32.const 0
      return
    end
    local.get $out
    local.get $x
    local.get $padding
    f32.add
    local.get $cw
    local.get $gap
    f32.add
    local.get $col
    f32.convert_i32_u
    f32.mul
    f32.add
    f32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $y
    local.get $padding
    f32.add
    local.get $ch
    local.get $gap
    f32.add
    local.get $row
    f32.convert_i32_u
    f32.mul
    f32.add
    f32.store
    local.get $out
    i32.const 8
    i32.add
    local.get $cw
    f32.store
    local.get $out
    i32.const 12
    i32.add
    local.get $ch
    f32.store
    i32.const 1)
