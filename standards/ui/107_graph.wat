(func (export "er_ui_graph_min_thickness") (result f32) f32.const 1)
  (func (export "er_ui_graph_elbow_min_mid_gap") (result f32) f32.const 10)
  (func (export "er_ui_graph_arrow_w") (result f32) f32.const 8)
  (func (export "er_ui_graph_arrow_h") (result f32) f32.const 8)
  (func (export "er_ui_graph_arrow_x_offset") (result f32) f32.const 5)
  (func (export "er_ui_graph_arrow_y_offset") (result f32) f32.const 4)

  (func $er_ui_graph_resolved_thickness (export "er_ui_graph_resolved_thickness") (param $thickness f32) (result f32)
    f32.const 1 local.get $thickness call $max_f32)

  (func $er_ui_graph_line_is_horizontal (export "er_ui_graph_line_is_horizontal") (param $x0 f32) (param $y0 f32) (param $x1 f32) (param $y1 f32) (result i32)
    local.get $x1 local.get $x0 f32.sub f32.abs
    local.get $y1 local.get $y0 f32.sub f32.abs
    f32.ge)

  (func $er_ui_graph_line_rect (export "er_ui_graph_line_rect") (param $x0 f32) (param $y0 f32) (param $x1 f32) (param $y1 f32) (param $thickness f32) (param $out i32) (result i32)
    (local $resolved f32)
    local.get $out i32.eqz
    if i32.const 0 return end
    local.get $thickness call $er_ui_graph_resolved_thickness local.set $resolved
    local.get $x0 local.get $y0 local.get $x1 local.get $y1 call $er_ui_graph_line_is_horizontal
    if
      local.get $out
      local.get $x0 local.get $x1 call $min_f32
      local.get $y0 local.get $resolved f32.const 0.5 f32.mul f32.sub
      local.get $resolved local.get $x1 local.get $x0 f32.sub f32.abs call $max_f32
      local.get $resolved
      call $rect_store
      return
    end
    local.get $out
    local.get $x0 local.get $resolved f32.const 0.5 f32.mul f32.sub
    local.get $y0 local.get $y1 call $min_f32
    local.get $resolved
    local.get $resolved local.get $y1 local.get $y0 f32.sub f32.abs call $max_f32
    call $rect_store)

  (func $er_ui_graph_elbow_points (export "er_ui_graph_elbow_points") (param $from i32) (param $to i32) (param $out i32) (result i32)
    (local $x0 f32) (local $y0 f32) (local $x1 f32) (local $y1 f32) (local $mid_x f32)
    local.get $from i32.eqz local.get $to i32.eqz i32.or local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $from f32.load local.get $from i32.const 8 i32.add f32.load f32.add local.set $x0
    local.get $from i32.const 4 i32.add f32.load local.get $from i32.const 12 i32.add f32.load f32.const 0.5 f32.mul f32.add local.set $y0
    local.get $to f32.load local.set $x1
    local.get $to i32.const 4 i32.add f32.load local.get $to i32.const 12 i32.add f32.load f32.const 0.5 f32.mul f32.add local.set $y1
    local.get $x0 local.get $x1 local.get $x0 f32.sub f32.const 0.5 f32.mul f32.const 10 call $max_f32 f32.add local.set $mid_x
    local.get $out local.get $x0 f32.store
    local.get $out i32.const 4 i32.add local.get $y0 f32.store
    local.get $out i32.const 8 i32.add local.get $mid_x f32.store
    local.get $out i32.const 12 i32.add local.get $y1 f32.store
    local.get $out i32.const 16 i32.add local.get $x1 f32.store
    i32.const 1)

  (func (export "er_ui_graph_elbow_segment_bounds") (param $from i32) (param $to i32) (param $segment i32) (param $thickness f32) (param $out i32) (result i32)
    local.get $from i32.eqz local.get $to i32.eqz i32.or local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $from local.get $to i32.const 124440 call $er_ui_graph_elbow_points drop
    local.get $segment i32.const 0 i32.eq
    if
      i32.const 124440 f32.load i32.const 124444 f32.load i32.const 124448 f32.load i32.const 124444 f32.load local.get $thickness local.get $out call $er_ui_graph_line_rect
      return
    end
    local.get $segment i32.const 1 i32.eq
    if
      i32.const 124448 f32.load i32.const 124444 f32.load i32.const 124448 f32.load i32.const 124452 f32.load local.get $thickness local.get $out call $er_ui_graph_line_rect
      return
    end
    i32.const 124448 f32.load i32.const 124452 f32.load i32.const 124456 f32.load i32.const 124452 f32.load local.get $thickness local.get $out call $er_ui_graph_line_rect)

  (func (export "er_ui_graph_elbow_arrow_bounds") (param $to i32) (param $out i32) (result i32)
    local.get $to i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $to f32.load f32.const 5 f32.sub
    local.get $to i32.const 4 i32.add f32.load local.get $to i32.const 12 i32.add f32.load f32.const 0.5 f32.mul f32.add f32.const 4 f32.sub
    f32.const 8
    f32.const 8
    call $rect_store)
