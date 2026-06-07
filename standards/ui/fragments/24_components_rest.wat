(module
  ;; Imports from other UI fragments
  (import "ui" "clamp_f32" (func $clamp_f32 (param f32) (param f32) (param f32) (result f32)))
  (import "ui" "clamp_i32" (func $clamp_i32 (param i32) (param i32) (param i32) (result i32)))
  (import "ui" "er_ui_font_text_width" (func $er_ui_font_text_width (param i32) (param i32) (param f32) (result f32)))
  (import "ui" "er_ui_primitives_content_inset" (func $er_ui_primitives_content_inset (param i32) (param f32) (param i32) (result i32)))
  (import "ui" "er_ui_rect_inset_uniform" (func $er_ui_rect_inset_uniform (param i32) (param i32) (param f32) (result i32)))
  (import "ui" "er_ui_text_wrapped_line_count" (func $er_ui_text_wrapped_line_count (param i32) (param i32) (param f32) (param f32) (param i32) (result i32)))
  (import "ui" "layout_max_i32_u" (func $layout_max_i32_u (param i32) (param i32) (result i32)))
  (import "ui" "max_f32" (func $max_f32 (param f32) (param f32) (result f32)))
  (import "ui" "min_f32" (func $min_f32 (param f32) (param f32) (result f32)))
  (import "ui" "rect_store" (func $rect_store (param i32) (param f32) (param f32) (param f32) (param f32) (result i32)))

  (func $er_ui_timeline_min_scale  (result f32) f32.const 0.05)
  (func $er_ui_timeline_min_window_w  (result f32) f32.const 0.01)
  (func $er_ui_timeline_pan_factor  (result f32) f32.const 0.25)
  (func $er_ui_timeline_min_zoom  (result f32) f32.const 1)
  (func $er_ui_timeline_max_zoom  (result f32) f32.const 6)
  (func $er_ui_timeline_zoom_out_factor  (result f32) f32.const 0.75)
  (func $er_ui_timeline_zoom_in_factor  (result f32) f32.const 1.35)
  (func $er_ui_timeline_default_label_w  (result f32) f32.const 82)
  (func $er_ui_timeline_default_inset  (result f32) f32.const 14)
  (func $er_ui_timeline_default_radius  (result f32) f32.const 8)
  (func $er_ui_timeline_header_h  (result f32) f32.const 24)
  (func $er_ui_timeline_controls_max_w  (result f32) f32.const 194)
  (func $er_ui_timeline_controls_gap  (result f32) f32.const 10)
  (func $er_ui_timeline_axis_top_gap  (result f32) f32.const 12)
  (func $er_ui_timeline_axis_y_offset  (result f32) f32.const 14)
  (func $er_ui_timeline_axis_bottom_gap  (result f32) f32.const 18)
  (func $er_ui_timeline_lane_top  (result f32) f32.const 18)
  (func $er_ui_timeline_lane_reserved_h  (result f32) f32.const 16)
  (func $er_ui_timeline_lane_label_x_offset  (result f32) f32.const 78)
  (func $er_ui_timeline_lane_label_w  (result f32) f32.const 68)
  (func $er_ui_timeline_lane_label_h  (result f32) f32.const 16)
  (func $er_ui_timeline_lane_separator_bottom  (result f32) f32.const 6)
  (func $er_ui_timeline_block_min_w  (result f32) f32.const 4)
  (func $er_ui_timeline_block_min_h  (result f32) f32.const 5)
  (func $er_ui_timeline_block_bottom  (result f32) f32.const 8)
  (func $er_ui_timeline_axis_line_y  (result f32) f32.const 12)
  (func $er_ui_timeline_mark_tick_y  (result f32) f32.const 7)
  (func $er_ui_timeline_mark_tick_h  (result f32) f32.const 11)
  (func $er_ui_timeline_mark_label_x_offset  (result f32) f32.const 22)
  (func $er_ui_timeline_mark_label_y_offset  (result f32) f32.const 7)
  (func $er_ui_timeline_mark_label_w  (result f32) f32.const 64)
  (func $er_ui_timeline_mark_label_h  (result f32) f32.const 14)

  (func $er_ui_timeline_clamp_unit  (param $value f32) (result f32)
    local.get $value f32.const 0 call $max_f32 f32.const 1 call $min_f32)

  (func $er_ui_timeline_window  (param $offset f32) (param $scale f32) (param $out i32) (result i32)
    (local $start f32) (local $width f32)
    local.get $out i32.eqz
    if i32.const 0 return end
    local.get $offset call $er_ui_timeline_clamp_unit local.set $start
    f32.const 1 local.get $scale f32.const 0.05 call $max_f32 f32.div local.set $width
    local.get $out local.get $start f32.store
    local.get $out i32.const 4 i32.add local.get $start f32.const 0.01 f32.add local.get $start local.get $width f32.add call $max_f32 f32.store
    i32.const 1)

  (func $er_ui_timeline_pan_offset  (param $offset f32) (param $scale f32) (param $direction f32) (result f32)
    (local $window_w f32) (local $max_offset f32)
    f32.const 1 local.get $scale f32.const 0.05 call $max_f32 f32.div local.set $window_w
    f32.const 0 f32.const 1 local.get $window_w f32.sub call $max_f32 local.set $max_offset
    local.get $offset local.get $direction local.get $window_w f32.mul f32.const 0.25 f32.mul f32.add
    f32.const 0 call $max_f32
    local.get $max_offset call $min_f32)

  (func $er_ui_timeline_zoom_state  (param $offset f32) (param $scale f32) (param $factor f32) (param $out i32) (result i32)
    (local $previous_scale f32) (local $previous_w f32) (local $center f32) (local $next_scale f32) (local $next_w f32) (local $max_offset f32)
    local.get $out i32.eqz
    if i32.const 0 return end
    local.get $scale f32.const 0.05 call $max_f32 local.set $previous_scale
    f32.const 1 local.get $previous_scale f32.div local.set $previous_w
    local.get $offset local.get $previous_w f32.const 0.5 f32.mul f32.add local.set $center
    f32.const 1 local.get $scale local.get $factor f32.mul call $max_f32 f32.const 6 call $min_f32 local.set $next_scale
    f32.const 1 local.get $next_scale f32.div local.set $next_w
    f32.const 0 f32.const 1 local.get $next_w f32.sub call $max_f32 local.set $max_offset
    local.get $out local.get $center local.get $next_w f32.const 0.5 f32.mul f32.sub f32.const 0 call $max_f32 local.get $max_offset call $min_f32 f32.store
    local.get $out i32.const 4 i32.add local.get $next_scale f32.store
    i32.const 1)

  (func $er_ui_timeline_unit_in_window  (param $value f32) (param $start f32) (param $end f32) (param $out i32) (result i32)
    local.get $out i32.eqz
    if i32.const 0 return end
    local.get $end local.get $start f32.le
    if i32.const 0 return end
    local.get $value local.get $start f32.lt
    local.get $value local.get $end f32.gt
    i32.or
    if i32.const 0 return end
    local.get $out local.get $value local.get $start f32.sub local.get $end local.get $start f32.sub f32.div call $er_ui_timeline_clamp_unit f32.store
    i32.const 1)

  (func $er_ui_timeline_block_in_window  (param $block_start f32) (param $block_end f32) (param $block_value f32) (param $start f32) (param $end f32) (param $out i32) (result i32)
    (local $mapped_start f32) (local $mapped_end f32) (local $denom f32)
    local.get $out i32.eqz
    if i32.const 0 return end
    local.get $end local.get $start f32.le
    if i32.const 0 return end
    local.get $start local.get $block_start call $max_f32 local.set $mapped_start
    local.get $end local.get $block_start local.get $block_end call $max_f32 call $min_f32 local.set $mapped_end
    local.get $mapped_end local.get $mapped_start f32.le
    if i32.const 0 return end
    local.get $end local.get $start f32.sub local.set $denom
    local.get $out local.get $mapped_start local.get $start f32.sub local.get $denom f32.div call $er_ui_timeline_clamp_unit f32.store
    local.get $out i32.const 4 i32.add local.get $mapped_end local.get $start f32.sub local.get $denom f32.div call $er_ui_timeline_clamp_unit f32.store
    local.get $out i32.const 8 i32.add local.get $block_value f32.store
    i32.const 1)

  (func $er_ui_timeline_lane_h  (param $axis i32) (param $lane_count i32) (result f32)
    local.get $axis i32.eqz local.get $lane_count i32.eqz i32.or
    if f32.const 0 return end
    local.get $axis i32.const 12 i32.add f32.load f32.const 16 f32.sub local.get $lane_count f32.convert_i32_u f32.div f32.const 1 call $max_f32)

  (func $er_ui_timeline_lane_label_bounds  (param $axis i32) (param $lane_index i32) (param $lane_count i32) (param $out i32) (result i32)
    (local $lane_h f32) (local $y f32)
    local.get $axis i32.eqz local.get $out i32.eqz i32.or local.get $lane_count i32.eqz i32.or
    if i32.const 0 return end
    local.get $axis local.get $lane_count call $er_ui_timeline_lane_h local.set $lane_h
    local.get $axis i32.const 4 i32.add f32.load f32.const 18 f32.add local.get $lane_index i32.const 0 local.get $lane_count i32.const 1 i32.sub call $clamp_i32 f32.convert_i32_u local.get $lane_h f32.mul f32.add local.set $y
    local.get $out
    local.get $axis f32.load f32.const 78 f32.sub
    local.get $y f32.const 4 f32.add
    f32.const 68
    f32.const 16
    call $rect_store)

  (func $er_ui_timeline_lane_separator_bounds  (param $axis i32) (param $lane_index i32) (param $lane_count i32) (param $out i32) (result i32)
    (local $lane_h f32) (local $y f32)
    local.get $axis i32.eqz local.get $out i32.eqz i32.or local.get $lane_count i32.eqz i32.or
    if i32.const 0 return end
    local.get $axis local.get $lane_count call $er_ui_timeline_lane_h local.set $lane_h
    local.get $axis i32.const 4 i32.add f32.load f32.const 18 f32.add local.get $lane_index i32.const 0 local.get $lane_count i32.const 1 i32.sub call $clamp_i32 f32.convert_i32_u local.get $lane_h f32.mul f32.add local.set $y
    local.get $out
    local.get $axis f32.load
    local.get $y local.get $lane_h f32.add f32.const 6 f32.sub
    local.get $axis i32.const 8 i32.add f32.load
    f32.const 1
    call $rect_store)

  (func $er_ui_timeline_lane_block_bounds  (param $axis i32) (param $lane_index i32) (param $lane_count i32) (param $block_start f32) (param $block_end f32) (param $block_value f32) (param $out i32) (result i32)
    (local $lane_h f32) (local $y f32) (local $start_x f32) (local $end_x f32) (local $w f32) (local $h f32)
    local.get $axis i32.eqz local.get $out i32.eqz i32.or local.get $lane_count i32.eqz i32.or
    if i32.const 0 return end
    local.get $axis local.get $lane_count call $er_ui_timeline_lane_h local.set $lane_h
    local.get $axis i32.const 4 i32.add f32.load f32.const 18 f32.add local.get $lane_index i32.const 0 local.get $lane_count i32.const 1 i32.sub call $clamp_i32 f32.convert_i32_u local.get $lane_h f32.mul f32.add local.set $y
    local.get $axis f32.load local.get $axis i32.const 8 i32.add f32.load local.get $block_start call $er_ui_timeline_clamp_unit f32.mul f32.add local.set $start_x
    local.get $axis f32.load local.get $axis i32.const 8 i32.add f32.load local.get $block_end call $er_ui_timeline_clamp_unit f32.mul f32.add local.set $end_x
    f32.const 4 f32.const 0 local.get $end_x local.get $start_x f32.sub call $max_f32 call $max_f32 local.set $w
    f32.const 5 local.get $lane_h f32.const 18 f32.sub local.get $block_value call $er_ui_timeline_clamp_unit f32.mul call $max_f32 local.set $h
    local.get $out local.get $start_x local.get $y local.get $lane_h f32.add f32.const 8 f32.sub local.get $h f32.sub local.get $w local.get $h call $rect_store)

  (func $er_ui_timeline_axis_line_bounds  (param $axis i32) (param $out i32) (result i32)
    local.get $axis i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $axis f32.load local.get $axis i32.const 4 i32.add f32.load f32.const 12 f32.add local.get $axis i32.const 8 i32.add f32.load f32.const 1 call $rect_store)

  (func $er_ui_timeline_mark_tick_bounds  (param $axis i32) (param $x_unit f32) (param $out i32) (result i32)
    (local $x f32)
    local.get $axis i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $axis f32.load local.get $axis i32.const 8 i32.add f32.load local.get $x_unit call $er_ui_timeline_clamp_unit f32.mul f32.add local.set $x
    local.get $out local.get $x local.get $axis i32.const 4 i32.add f32.load f32.const 7 f32.add f32.const 1 f32.const 11 call $rect_store)

  (func $er_ui_timeline_mark_label_bounds  (param $axis i32) (param $x_unit f32) (param $out i32) (result i32)
    (local $x f32)
    local.get $axis i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $axis f32.load local.get $axis i32.const 8 i32.add f32.load local.get $x_unit call $er_ui_timeline_clamp_unit f32.mul f32.add local.set $x
    local.get $out local.get $x f32.const 22 f32.sub local.get $axis i32.const 4 i32.add f32.load f32.const 7 f32.sub f32.const 64 f32.const 14 call $rect_store)

  (func $er_ui_timeline_inner_bounds  (param $bounds i32) (param $inset f32) (param $out i32) (result i32)
    local.get $bounds local.get $inset local.get $out call $er_ui_primitives_content_inset)

  (func $er_ui_timeline_header_text_bounds  (param $bounds i32) (param $has_header i32) (param $has_controls i32) (param $label_w f32) (param $inset f32) (param $out i32) (result i32)
    (local $controls_w f32) (local $controls_gap f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds local.get $inset i32.const 124300 call $er_ui_timeline_inner_bounds drop
    local.get $has_controls
    if
      i32.const 124308 f32.load f32.const 194 call $min_f32 local.set $controls_w
      f32.const 10 local.set $controls_gap
    else
      f32.const 0 local.set $controls_w
      f32.const 0 local.set $controls_gap
    end
    local.get $out i32.const 124300 f32.load i32.const 124304 f32.load i32.const 124308 f32.load local.get $controls_w f32.sub local.get $controls_gap f32.sub f32.const 1 call $max_f32 local.get $has_header if (result f32) f32.const 24 else f32.const 0 end call $rect_store)

  (func $er_ui_timeline_controls_bounds  (param $bounds i32) (param $has_controls i32) (param $inset f32) (param $out i32) (result i32)
    (local $controls_w f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds local.get $inset i32.const 124320 call $er_ui_timeline_inner_bounds drop
    local.get $has_controls i32.eqz
    if i32.const 0 return end
    i32.const 124328 f32.load f32.const 194 call $min_f32 local.set $controls_w
    local.get $out i32.const 124320 f32.load i32.const 124328 f32.load f32.add local.get $controls_w f32.sub i32.const 124324 f32.load f32.const 2 f32.sub local.get $controls_w f32.const 28 call $rect_store)

  (func $er_ui_timeline_axis_bounds  (param $bounds i32) (param $has_header i32) (param $label_w f32) (param $inset f32) (param $out i32) (result i32)
    (local $header_h f32) (local $resolved_label_w f32) (local $axis_y f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds local.get $inset i32.const 124340 call $er_ui_timeline_inner_bounds drop
    local.get $has_header if (result f32) f32.const 24 else f32.const 0 end local.set $header_h
    i32.const 124348 f32.load f32.const 0.42 f32.mul local.get $label_w f32.const 0 call $max_f32 call $min_f32 local.set $resolved_label_w
    i32.const 124344 f32.load local.get $header_h f32.add f32.const 12 f32.add local.set $axis_y
    local.get $out
    i32.const 124340 f32.load local.get $resolved_label_w f32.add
    local.get $axis_y f32.const 14 f32.add
    i32.const 124348 f32.load local.get $resolved_label_w f32.sub f32.const 1 call $max_f32
    i32.const 124344 f32.load i32.const 124352 f32.load f32.add local.get $axis_y f32.sub f32.const 18 f32.sub f32.const 1 call $max_f32
    call $rect_store)
  (func $er_ui_workspace_default_rail_w  (result f32) f32.const 48)
  (func $er_ui_workspace_default_sidebar_w  (result f32) f32.const 260)
  (func $er_ui_workspace_default_top_h  (result f32) f32.const 56)
  (func $er_ui_workspace_default_status_h  (result f32) f32.const 24)
  (func $er_ui_workspace_top_trailing_default_w  (result f32) f32.const 210)
  (func $er_ui_workspace_top_inset_x  (result f32) f32.const 16)
  (func $er_ui_workspace_top_title_y  (result f32) f32.const 13)
  (func $er_ui_workspace_top_title_h  (result f32) f32.const 18)
  (func $er_ui_workspace_top_detail_y  (result f32) f32.const 34)
  (func $er_ui_workspace_top_detail_h  (result f32) f32.const 14)
  (func $er_ui_workspace_top_trailing_gap  (result f32) f32.const 20)
  (func $er_ui_workspace_top_trailing_top_y  (result f32) f32.const 13)
  (func $er_ui_workspace_top_trailing_bottom_y  (result f32) f32.const 32)
  (func $er_ui_workspace_status_inset_x  (result f32) f32.const 12)
  (func $er_ui_workspace_status_text_y  (result f32) f32.const 5)
  (func $er_ui_workspace_status_text_h  (result f32) f32.const 14)
  (func $er_ui_workspace_responsive_default_breakpoint  (result f32) f32.const 980)
  (func $er_ui_workspace_responsive_default_gap  (result f32) f32.const 14)

  (func $er_ui_workspace_shell_metrics (param $bounds i32) (param $rail_w f32) (param $sidebar_w f32) (param $top_h f32) (param $status_h f32) (param $out i32) (result i32)
    (local $rail_w_res f32) (local $status_h_res f32) (local $top_h_res f32) (local $body_h f32) (local $body_y f32) (local $rail_h f32) (local $content_x f32) (local $content_w f32) (local $sidebar_w_res f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 8 i32.add f32.load local.get $rail_w f32.const 0 call $max_f32 call $min_f32 local.set $rail_w_res
    local.get $bounds i32.const 12 i32.add f32.load local.get $status_h f32.const 0 call $max_f32 call $min_f32 local.set $status_h_res
    local.get $bounds i32.const 12 i32.add f32.load local.get $status_h_res f32.sub f32.const 0 call $max_f32 local.get $top_h f32.const 0 call $max_f32 call $min_f32 local.set $top_h_res
    local.get $bounds i32.const 12 i32.add f32.load local.get $top_h_res f32.sub local.get $status_h_res f32.sub f32.const 1 call $max_f32 local.set $body_h
    local.get $bounds i32.const 4 i32.add f32.load local.get $top_h_res f32.add local.set $body_y
    local.get $bounds i32.const 12 i32.add f32.load local.get $status_h_res f32.sub f32.const 1 call $max_f32 local.set $rail_h
    local.get $bounds f32.load local.get $rail_w_res f32.add local.set $content_x
    local.get $bounds i32.const 8 i32.add f32.load local.get $rail_w_res f32.sub f32.const 1 call $max_f32 local.set $content_w
    local.get $content_w local.get $sidebar_w f32.const 0 call $max_f32 call $min_f32 local.set $sidebar_w_res
    local.get $out local.get $rail_w_res f32.store
    local.get $out i32.const 4 i32.add local.get $status_h_res f32.store
    local.get $out i32.const 8 i32.add local.get $top_h_res f32.store
    local.get $out i32.const 12 i32.add local.get $body_h f32.store
    local.get $out i32.const 16 i32.add local.get $body_y f32.store
    local.get $out i32.const 20 i32.add local.get $rail_h f32.store
    local.get $out i32.const 24 i32.add local.get $content_x f32.store
    local.get $out i32.const 28 i32.add local.get $content_w f32.store
    local.get $out i32.const 32 i32.add local.get $sidebar_w_res f32.store
    i32.const 1)

  (func $er_ui_workspace_shell_rail_bounds  (param $bounds i32) (param $rail_w f32) (param $sidebar_w f32) (param $top_h f32) (param $status_h f32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds local.get $rail_w local.get $sidebar_w local.get $top_h local.get $status_h i32.const 124380 call $er_ui_workspace_shell_metrics drop
    local.get $out local.get $bounds f32.load local.get $bounds i32.const 4 i32.add f32.load i32.const 124380 f32.load i32.const 124400 f32.load call $rect_store)

  (func $er_ui_workspace_shell_top_bounds  (param $bounds i32) (param $rail_w f32) (param $sidebar_w f32) (param $top_h f32) (param $status_h f32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds local.get $rail_w local.get $sidebar_w local.get $top_h local.get $status_h i32.const 124380 call $er_ui_workspace_shell_metrics drop
    local.get $out i32.const 124404 f32.load local.get $bounds i32.const 4 i32.add f32.load i32.const 124408 f32.load i32.const 124388 f32.load call $rect_store)

  (func $er_ui_workspace_shell_sidebar_bounds  (param $bounds i32) (param $rail_w f32) (param $sidebar_w f32) (param $top_h f32) (param $status_h f32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds local.get $rail_w local.get $sidebar_w local.get $top_h local.get $status_h i32.const 124380 call $er_ui_workspace_shell_metrics drop
    local.get $out i32.const 124404 f32.load i32.const 124396 f32.load i32.const 124412 f32.load i32.const 124392 f32.load call $rect_store)

  (func $er_ui_workspace_shell_main_bounds  (param $bounds i32) (param $rail_w f32) (param $sidebar_w f32) (param $top_h f32) (param $status_h f32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds local.get $rail_w local.get $sidebar_w local.get $top_h local.get $status_h i32.const 124380 call $er_ui_workspace_shell_metrics drop
    local.get $out i32.const 124404 f32.load i32.const 124412 f32.load f32.add i32.const 124396 f32.load i32.const 124408 f32.load i32.const 124412 f32.load f32.sub f32.const 1 call $max_f32 i32.const 124392 f32.load call $rect_store)

  (func $er_ui_workspace_shell_status_bounds  (param $bounds i32) (param $rail_w f32) (param $sidebar_w f32) (param $top_h f32) (param $status_h f32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds local.get $rail_w local.get $sidebar_w local.get $top_h local.get $status_h i32.const 124380 call $er_ui_workspace_shell_metrics drop
    local.get $out local.get $bounds f32.load local.get $bounds i32.const 4 i32.add f32.load local.get $bounds i32.const 12 i32.add f32.load f32.add i32.const 124384 f32.load f32.sub local.get $bounds i32.const 8 i32.add f32.load i32.const 124384 f32.load call $rect_store)

  (func $er_ui_workspace_top_trailing_width  (param $bounds i32) (param $has_trailing i32) (param $trailing_w f32) (result f32)
    local.get $bounds i32.eqz local.get $has_trailing i32.eqz i32.or
    if f32.const 0 return end
    local.get $bounds i32.const 8 i32.add f32.load local.get $trailing_w f32.const 1 call $max_f32 call $min_f32)

  (func $er_ui_workspace_top_title_bounds  (param $bounds i32) (param $has_trailing i32) (param $trailing_w f32) (param $inset_x f32) (param $out i32) (result i32)
    (local $tw f32) (local $gap f32) (local $text_w f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds local.get $has_trailing local.get $trailing_w call $er_ui_workspace_top_trailing_width local.set $tw
    local.get $tw f32.const 0 f32.gt
    if f32.const 20 local.set $gap else f32.const 0 local.set $gap end
    local.get $bounds i32.const 8 i32.add f32.load local.get $tw f32.sub local.get $inset_x f32.const 2 f32.mul f32.sub local.get $gap f32.sub f32.const 1 call $max_f32 local.set $text_w
    local.get $out local.get $bounds f32.load local.get $inset_x f32.add local.get $bounds i32.const 4 i32.add f32.load f32.const 13 f32.add local.get $text_w f32.const 18 call $rect_store)

  (func $er_ui_workspace_top_detail_bounds  (param $bounds i32) (param $has_trailing i32) (param $trailing_w f32) (param $inset_x f32) (param $out i32) (result i32)
    local.get $bounds local.get $has_trailing local.get $trailing_w local.get $inset_x local.get $out call $er_ui_workspace_top_title_bounds drop
    local.get $out i32.eqz
    if i32.const 0 return end
    local.get $out i32.const 4 i32.add local.get $bounds i32.const 4 i32.add f32.load f32.const 34 f32.add f32.store
    local.get $out i32.const 12 i32.add f32.const 14 f32.store
    i32.const 1)

  (func $er_ui_workspace_top_trailing_top_bounds  (param $bounds i32) (param $trailing_w f32) (param $inset_x f32) (param $out i32) (result i32)
    (local $tw f32) (local $x f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 8 i32.add f32.load local.get $trailing_w f32.const 1 call $max_f32 call $min_f32 local.set $tw
    local.get $bounds f32.load local.get $bounds i32.const 8 i32.add f32.load f32.add local.get $tw f32.sub local.get $inset_x f32.sub local.set $x
    local.get $out local.get $x local.get $bounds i32.const 4 i32.add f32.load f32.const 13 f32.add local.get $tw f32.const 14 call $rect_store)

  (func $er_ui_workspace_top_trailing_bottom_bounds  (param $bounds i32) (param $trailing_w f32) (param $inset_x f32) (param $out i32) (result i32)
    local.get $bounds local.get $trailing_w local.get $inset_x local.get $out call $er_ui_workspace_top_trailing_top_bounds drop
    local.get $out i32.eqz
    if i32.const 0 return end
    local.get $out i32.const 4 i32.add local.get $bounds i32.const 4 i32.add f32.load f32.const 32 f32.add f32.store
    i32.const 1)

  (func $er_ui_workspace_status_text_bounds  (param $bounds i32) (param $inset_x f32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $bounds f32.load local.get $inset_x f32.add local.get $bounds i32.const 4 i32.add f32.load f32.const 5 f32.add local.get $bounds i32.const 8 i32.add f32.load local.get $inset_x f32.const 2 f32.mul f32.sub f32.const 1 call $max_f32 f32.const 14 call $rect_store)

  (func $er_ui_workspace_responsive_stacked  (param $bounds i32) (param $breakpoint f32) (result i32)
    local.get $bounds i32.eqz
    if i32.const 0 return end
    local.get $bounds i32.const 8 i32.add f32.load local.get $breakpoint f32.lt)

  (func $er_ui_workspace_responsive_panes  (param $bounds i32) (param $breakpoint f32) (param $gap f32) (param $first_w f32) (param $third_w f32) (param $first_stack_h f32) (param $second_stack_h f32) (param $out i32) (result i32)
    (local $g f32) (local $fw f32) (local $tw f32) (local $second_x f32) (local $third_x f32) (local $fh f32) (local $second_y f32) (local $sh f32) (local $third_y f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $gap f32.const 0 call $max_f32 local.set $g
    local.get $bounds i32.const 8 i32.add f32.load local.get $breakpoint f32.ge
    if
      local.get $bounds i32.const 8 i32.add f32.load local.get $first_w f32.const 1 call $max_f32 call $min_f32 local.set $fw
      local.get $bounds i32.const 8 i32.add f32.load local.get $third_w f32.const 1 call $max_f32 call $min_f32 local.set $tw
      local.get $bounds f32.load local.get $fw f32.add local.get $g f32.add local.set $second_x
      local.get $bounds f32.load local.get $bounds i32.const 8 i32.add f32.load f32.add local.get $tw f32.sub local.set $third_x
      local.get $out local.get $bounds f32.load local.get $bounds i32.const 4 i32.add f32.load local.get $fw local.get $bounds i32.const 12 i32.add f32.load call $rect_store drop
      local.get $out i32.const 16 i32.add local.get $second_x local.get $bounds i32.const 4 i32.add f32.load local.get $third_x local.get $g f32.sub local.get $second_x f32.sub f32.const 1 call $max_f32 local.get $bounds i32.const 12 i32.add f32.load call $rect_store drop
      local.get $out i32.const 32 i32.add local.get $third_x local.get $bounds i32.const 4 i32.add f32.load local.get $tw local.get $bounds i32.const 12 i32.add f32.load call $rect_store drop
      local.get $out i32.const 48 i32.add i32.const 0 i32.store
      i32.const 1
      return
    end
    local.get $bounds i32.const 12 i32.add f32.load local.get $first_stack_h f32.const 1 call $max_f32 call $min_f32 local.set $fh
    local.get $bounds i32.const 4 i32.add f32.load local.get $fh f32.add local.get $g f32.add local.set $second_y
    local.get $bounds i32.const 4 i32.add f32.load local.get $bounds i32.const 12 i32.add f32.load f32.add local.get $second_y f32.sub f32.const 1 call $max_f32 local.get $second_stack_h f32.const 1 call $max_f32 call $min_f32 local.set $sh
    local.get $second_y local.get $sh f32.add local.get $g f32.add local.set $third_y
    local.get $out local.get $bounds f32.load local.get $bounds i32.const 4 i32.add f32.load local.get $bounds i32.const 8 i32.add f32.load local.get $fh call $rect_store drop
    local.get $out i32.const 16 i32.add local.get $bounds f32.load local.get $second_y local.get $bounds i32.const 8 i32.add f32.load local.get $sh call $rect_store drop
    local.get $out i32.const 32 i32.add local.get $bounds f32.load local.get $third_y local.get $bounds i32.const 8 i32.add f32.load local.get $bounds i32.const 4 i32.add f32.load local.get $bounds i32.const 12 i32.add f32.load f32.add local.get $third_y f32.sub f32.const 1 call $max_f32 call $rect_store drop
    local.get $out i32.const 48 i32.add i32.const 1 i32.store
    i32.const 1)
  (func $er_ui_graph_min_thickness  (result f32) f32.const 1)
  (func $er_ui_graph_elbow_min_mid_gap  (result f32) f32.const 10)
  (func $er_ui_graph_arrow_w  (result f32) f32.const 8)
  (func $er_ui_graph_arrow_h  (result f32) f32.const 8)
  (func $er_ui_graph_arrow_x_offset  (result f32) f32.const 5)
  (func $er_ui_graph_arrow_y_offset  (result f32) f32.const 4)

  (func $er_ui_graph_resolved_thickness  (param $thickness f32) (result f32)
    f32.const 1 local.get $thickness call $max_f32)

  (func $er_ui_graph_line_is_horizontal  (param $x0 f32) (param $y0 f32) (param $x1 f32) (param $y1 f32) (result i32)
    local.get $x1 local.get $x0 f32.sub f32.abs
    local.get $y1 local.get $y0 f32.sub f32.abs
    f32.ge)

  (func $er_ui_graph_line_rect  (param $x0 f32) (param $y0 f32) (param $x1 f32) (param $y1 f32) (param $thickness f32) (param $out i32) (result i32)
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

  (func $er_ui_graph_elbow_points  (param $from i32) (param $to i32) (param $out i32) (result i32)
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

  (func $er_ui_graph_elbow_segment_bounds  (param $from i32) (param $to i32) (param $segment i32) (param $thickness f32) (param $out i32) (result i32)
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

  (func $er_ui_graph_elbow_arrow_bounds  (param $to i32) (param $out i32) (result i32)
    local.get $to i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $to f32.load f32.const 5 f32.sub
    local.get $to i32.const 4 i32.add f32.load local.get $to i32.const 12 i32.add f32.load f32.const 0.5 f32.mul f32.add f32.const 4 f32.sub
    f32.const 8
    f32.const 8
    call $rect_store)
  (func $er_ui_app_panel_scaffold_default_inset  (result f32) f32.const 16)
  (func $er_ui_app_panel_scaffold_default_header_h  (result f32) f32.const 42)
  (func $er_ui_app_panel_scaffold_default_header_gap  (result f32) f32.const 16)
  (func $er_ui_app_panel_list_default_row_h  (result f32) f32.const 42)
  (func $er_ui_app_panel_list_default_gap  (result f32) f32.const 4)
  (func $er_ui_app_action_toolbar_button_w  (result f32) f32.const 34)
  (func $er_ui_app_action_toolbar_button_h  (result f32) f32.const 36)
  (func $er_ui_app_action_toolbar_gap  (result f32) f32.const 8)
  (func $er_ui_app_workspace_rail_pad_x  (result f32) f32.const 6)
  (func $er_ui_app_workspace_rail_pad_top  (result f32) f32.const 12)
  (func $er_ui_app_workspace_sidebar_inset_x  (result f32) f32.const 16)
  (func $er_ui_app_workspace_sidebar_title_y  (result f32) f32.const 14)
  (func $er_ui_app_workspace_sidebar_detail_y  (result f32) f32.const 36)
  (func $er_ui_app_workspace_sidebar_body_y  (result f32) f32.const 68)
  (func $er_ui_app_workspace_sidebar_right_border_w  (result f32) f32.const 1)
  (func $er_ui_app_compose_bar_height  (result f32) f32.const 74)
  (func $er_ui_app_compose_bar_inset_x  (result f32) f32.const 18)
  (func $er_ui_app_compose_bar_toolbar_w  (result f32) f32.const 120)
  (func $er_ui_app_compose_bar_toolbar_button_w  (result f32) f32.const 34)
  (func $er_ui_app_compose_bar_toolbar_gap  (result f32) f32.const 6)
  (func $er_ui_app_compose_bar_textarea_gap  (result f32) f32.const 6)
  (func $er_ui_app_compose_bar_send_w  (result f32) f32.const 44)
  (func $er_ui_app_floating_panel_radius  (result f32) f32.const 12)
  (func $er_ui_app_floating_panel_shadow_size  (result f32) f32.const 8)
  (func $er_ui_app_floating_panel_shadow_outset  (result f32) f32.const 2)
  (func $er_ui_app_floating_panel_inset  (result f32) f32.const 16)
  (func $er_ui_app_segment_map_gap  (result f32) f32.const 5)
  (func $er_ui_app_segment_map_radius  (result f32) f32.const 8)

  (func $er_ui_app_header_badges_width  (param $count i32) (param $badge_w f32) (result f32)
    local.get $count i32.eqz
    if f32.const 0 return end
    local.get $count f32.convert_i32_u local.get $badge_w f32.const 10 f32.add f32.mul)

  (func $er_ui_app_panel_scaffold_header_bounds  (param $bounds i32) (param $inset f32) (param $header_h f32) (param $out i32) (result i32)
    (local $header f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $header_h f32.const 1 call $max_f32 local.set $header
    local.get $bounds local.get $inset i32.const 124500 call $er_ui_primitives_content_inset drop
    local.get $out i32.const 124500 f32.load i32.const 124504 f32.load i32.const 124508 f32.load local.get $header call $rect_store)

  (func $er_ui_app_panel_scaffold_body_bounds  (param $bounds i32) (param $inset f32) (param $header_h f32) (param $header_gap f32) (param $out i32) (result i32)
    (local $header f32) (local $gap f32) (local $body_y f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $header_h f32.const 1 call $max_f32 local.set $header
    local.get $header_gap f32.const 0 call $max_f32 local.set $gap
    local.get $bounds local.get $inset i32.const 124516 call $er_ui_primitives_content_inset drop
    i32.const 124520 f32.load local.get $header f32.add local.get $gap f32.add local.set $body_y
    local.get $out
    i32.const 124516 f32.load
    local.get $body_y
    i32.const 124524 f32.load
    i32.const 124520 f32.load i32.const 124528 f32.load f32.add local.get $body_y f32.sub f32.const 1 call $max_f32
    call $rect_store)

  (func $er_ui_app_action_toolbar_item_bounds  (param $bounds i32) (param $index i32) (param $direction i32) (param $button_w f32) (param $button_h f32) (param $gap f32) (param $out i32) (result i32)
    (local $idx f32) (local $g f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $index i32.const 0 i32.lt_s
    if
      f32.const 0 local.set $idx
    else
      local.get $index f32.convert_i32_s local.set $idx
    end
    local.get $gap f32.const 0 call $max_f32 local.set $g
    local.get $direction i32.eqz
    if
      local.get $out
      local.get $bounds f32.load local.get $idx local.get $button_w local.get $g f32.add f32.mul f32.add
      local.get $bounds i32.const 4 i32.add f32.load
      local.get $button_w f32.const 1 call $max_f32
      local.get $bounds i32.const 12 i32.add f32.load
      call $rect_store
      return
    end
    local.get $out
    local.get $bounds f32.load
    local.get $bounds i32.const 4 i32.add f32.load local.get $idx local.get $button_h local.get $g f32.add f32.mul f32.add
    local.get $bounds i32.const 8 i32.add f32.load
    local.get $button_h f32.const 1 call $max_f32
    call $rect_store)

  (func $er_ui_app_workspace_rail_toolbar_bounds  (param $bounds i32) (param $pad_x f32) (param $pad_top f32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $bounds f32.load local.get $pad_x f32.add
    local.get $bounds i32.const 4 i32.add f32.load local.get $pad_top f32.add
    local.get $bounds i32.const 8 i32.add f32.load local.get $pad_x f32.const 2 f32.mul f32.sub f32.const 1 call $max_f32
    local.get $bounds i32.const 12 i32.add f32.load local.get $pad_top f32.sub f32.const 1 call $max_f32
    call $rect_store)

  (func $er_ui_app_workspace_sidebar_right_border_bounds  (param $bounds i32) (param $right_border_w f32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $right_border_w f32.const 0 f32.le
    if i32.const 0 return end
    local.get $out local.get $bounds f32.load local.get $bounds i32.const 8 i32.add f32.load f32.add local.get $right_border_w f32.sub local.get $bounds i32.const 4 i32.add f32.load local.get $right_border_w local.get $bounds i32.const 12 i32.add f32.load call $rect_store)

  (func $er_ui_app_workspace_sidebar_title_bounds  (param $bounds i32) (param $inset_x f32) (param $title_y f32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $bounds f32.load local.get $inset_x f32.add local.get $bounds i32.const 4 i32.add f32.load local.get $title_y f32.add local.get $bounds i32.const 8 i32.add f32.load local.get $inset_x f32.const 2 f32.mul f32.sub f32.const 1 call $max_f32 f32.const 16 call $rect_store)

  (func $er_ui_app_workspace_sidebar_detail_bounds  (param $bounds i32) (param $inset_x f32) (param $detail_y f32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $bounds f32.load local.get $inset_x f32.add local.get $bounds i32.const 4 i32.add f32.load local.get $detail_y f32.add local.get $bounds i32.const 8 i32.add f32.load local.get $inset_x f32.const 2 f32.mul f32.sub f32.const 1 call $max_f32 f32.const 14 call $rect_store)

  (func $er_ui_app_workspace_sidebar_body_bounds  (param $bounds i32) (param $inset_x f32) (param $body_y f32) (param $out i32) (result i32)
    (local $x_pad f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $inset_x f32.const 6 f32.sub local.set $x_pad
    local.get $out
    local.get $bounds f32.load local.get $x_pad f32.add
    local.get $bounds i32.const 4 i32.add f32.load local.get $body_y f32.add
    local.get $bounds i32.const 8 i32.add f32.load local.get $x_pad f32.const 2 f32.mul f32.sub f32.const 1 call $max_f32
    local.get $bounds i32.const 12 i32.add f32.load local.get $body_y f32.sub f32.const 1 call $max_f32
    call $rect_store)

  (func $er_ui_app_compose_toolbar_bounds  (param $bounds i32) (param $inset_x f32) (param $toolbar_w f32) (param $out i32) (result i32)
    (local $tool_y f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 4 i32.add f32.load local.get $bounds i32.const 12 i32.add f32.load f32.const 38 f32.sub f32.const 0 call $max_f32 f32.const 0.5 f32.mul f32.add local.set $tool_y
    local.get $out local.get $bounds f32.load local.get $inset_x f32.add local.get $tool_y local.get $toolbar_w f32.const 38 call $rect_store)

  (func $er_ui_app_compose_textarea_bounds  (param $bounds i32) (param $inset_x f32) (param $toolbar_w f32) (param $textarea_gap f32) (param $send_w f32) (param $out i32) (result i32)
    (local $textarea_x f32) (local $send_x f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds f32.load local.get $inset_x f32.add local.get $toolbar_w f32.add local.get $textarea_gap f32.add local.set $textarea_x
    local.get $bounds f32.load local.get $bounds i32.const 8 i32.add f32.load f32.add local.get $inset_x f32.sub local.get $send_w f32.sub local.set $send_x
    local.get $out local.get $textarea_x local.get $bounds i32.const 4 i32.add f32.load f32.const 15 f32.add local.get $send_x local.get $textarea_x f32.sub local.get $textarea_gap f32.sub f32.const 1 call $max_f32 f32.const 44 call $rect_store)

  (func $er_ui_app_compose_send_bounds  (param $bounds i32) (param $inset_x f32) (param $send_w f32) (param $out i32) (result i32)
    (local $tool_y f32) (local $send_x f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 4 i32.add f32.load local.get $bounds i32.const 12 i32.add f32.load f32.const 38 f32.sub f32.const 0 call $max_f32 f32.const 0.5 f32.mul f32.add local.set $tool_y
    local.get $bounds f32.load local.get $bounds i32.const 8 i32.add f32.load f32.add local.get $inset_x f32.sub local.get $send_w f32.sub local.set $send_x
    local.get $out local.get $send_x local.get $tool_y local.get $send_w f32.const 38 call $rect_store)

  (func $er_ui_app_compose_footer_bounds  (param $bounds i32) (param $inset_x f32) (param $toolbar_w f32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $bounds f32.load local.get $inset_x f32.add local.get $bounds i32.const 4 i32.add f32.load local.get $bounds i32.const 12 i32.add f32.load f32.add f32.const 18 f32.sub local.get $toolbar_w f32.const 40 f32.add f32.const 1 call $max_f32 f32.const 14 call $rect_store)

  (func $er_ui_app_floating_panel_inner_bounds  (param $bounds i32) (param $inset f32) (param $out i32) (result i32)
    local.get $bounds local.get $inset local.get $out call $er_ui_primitives_content_inset)

  (func $er_ui_app_floating_panel_shadow_bounds  (param $bounds i32) (param $shadow_outset f32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds local.get $out local.get $shadow_outset f32.neg call $er_ui_rect_inset_uniform)

  (func $er_ui_app_floating_panel_scrim_bounds  (param $bounds i32) (param $scrim_h f32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $scrim_h f32.const 0 f32.le
    if i32.const 0 return end
    local.get $out local.get $bounds f32.load local.get $bounds i32.const 4 i32.add f32.load local.get $bounds i32.const 8 i32.add f32.load local.get $scrim_h call $rect_store)

  (func $er_ui_app_message_bubble_body_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds f32.const 11 local.get $out call $er_ui_primitives_content_inset)

  (func $er_ui_app_message_bubble_media_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $bounds f32.load f32.const 10 f32.add local.get $bounds i32.const 4 i32.add f32.load f32.const 42 f32.add local.get $bounds i32.const 8 i32.add f32.load f32.const 20 f32.sub f32.const 1 call $max_f32 f32.const 66 call $rect_store)

  (func $er_ui_app_message_bubble_media_icon_bounds  (param $media i32) (param $out i32) (result i32)
    local.get $media i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $media f32.load local.get $media i32.const 8 i32.add f32.load f32.add f32.const 28 f32.sub local.get $media i32.const 4 i32.add f32.load f32.const 10 f32.add f32.const 18 f32.const 18 call $rect_store)

  (func $er_ui_app_segment_map_inner_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds f32.const 8 local.get $out call $er_ui_primitives_content_inset)

  (func $er_ui_app_segment_map_block_bounds  (param $bounds i32) (param $segment_index i32) (param $segment_count i32) (param $cursor_x f32) (param $total_weight f32) (param $weight f32) (param $height_unit f32) (param $out i32) (result i32)
    (local $remaining f32) (local $normalized f32) (local $width f32) (local $block_w f32) (local $block_h f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or local.get $segment_count i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 124532 call $er_ui_app_segment_map_inner_bounds drop
    i32.const 124532 f32.load i32.const 124540 f32.load f32.add local.get $cursor_x f32.sub f32.const 1 call $max_f32 local.set $remaining
    local.get $total_weight f32.const 0 f32.le
    if i32.const 0 return end
    local.get $weight f32.const 0 call $max_f32 local.get $total_weight f32.div local.set $normalized
    local.get $segment_index local.get $segment_count i32.const 1 i32.sub i32.eq
    if
      local.get $remaining local.set $width
    else
      f32.const 18 i32.const 124540 f32.load local.get $normalized f32.mul call $max_f32 local.set $width
    end
    f32.const 1 local.get $width local.get $remaining call $min_f32 call $max_f32 local.set $block_w
    f32.const 18 i32.const 124544 f32.load local.get $height_unit f32.const 0.05 f32.const 1 call $clamp_f32 f32.mul call $max_f32 local.set $block_h
    local.get $out local.get $cursor_x i32.const 124536 f32.load i32.const 124544 f32.load f32.add local.get $block_h f32.sub local.get $block_w local.get $block_h call $rect_store)

  (func $er_ui_app_context_panel_bounds  (param $container i32) (param $x f32) (param $y f32) (param $w f32) (param $h f32) (param $out i32) (result i32)
    local.get $container i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $x local.get $container f32.load f32.const 8 f32.add local.get $container f32.load local.get $container i32.const 8 i32.add f32.load f32.add local.get $w f32.sub f32.const 8 f32.sub call $clamp_f32
    local.get $y local.get $container i32.const 4 i32.add f32.load f32.const 8 f32.add local.get $container i32.const 4 i32.add f32.load local.get $container i32.const 12 i32.add f32.load f32.add local.get $h f32.sub f32.const 8 f32.sub call $clamp_f32
    local.get $w
    local.get $h
    call $rect_store)

  (func $er_ui_app_context_panel_title_bounds  (param $panel i32) (param $out i32) (result i32)
    local.get $panel i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $panel f32.load f32.const 14 f32.add local.get $panel i32.const 4 i32.add f32.load f32.const 12 f32.add local.get $panel i32.const 8 i32.add f32.load f32.const 28 f32.sub f32.const 18 call $rect_store)

  (func $er_ui_app_context_panel_detail_bounds  (param $panel i32) (param $out i32) (result i32)
    local.get $panel i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $panel f32.load f32.const 14 f32.add local.get $panel i32.const 4 i32.add f32.load f32.const 36 f32.add local.get $panel i32.const 8 i32.add f32.load f32.const 28 f32.sub f32.const 16 call $rect_store)

  (func $er_ui_app_context_panel_primary_bounds  (param $panel i32) (param $out i32) (result i32)
    local.get $panel i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $panel f32.load f32.const 12 f32.add local.get $panel i32.const 4 i32.add f32.load f32.const 66 f32.add f32.const 118 f32.const 34 call $rect_store)

  (func $er_ui_app_context_panel_secondary_bounds  (param $panel i32) (param $out i32) (result i32)
    local.get $panel i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $panel f32.load f32.const 138 f32.add local.get $panel i32.const 4 i32.add f32.load f32.const 66 f32.add f32.const 70 f32.const 34 call $rect_store)

  (func $er_ui_app_overlay_motion_dy  (param $progress f32) (result f32)
    f32.const 1 local.get $progress f32.sub f32.const 8 f32.mul)

  (func $er_ui_app_overlay_motion_dx  (param $progress f32) (result f32)
    f32.const 1 local.get $progress f32.sub f32.const 18 f32.mul)

  (func $er_ui_app_property_editor_panel_width  (param $container i32) (param $panel_w f32) (result f32)
    local.get $container i32.eqz
    if f32.const 0 return end
    local.get $panel_w local.get $container i32.const 8 i32.add f32.load f32.const 0.24 f32.mul f32.const 300 call $max_f32 call $min_f32)

  (func $er_ui_app_property_editor_panel_bounds  (param $container i32) (param $panel_w f32) (param $out i32) (result i32)
    (local $w f32)
    local.get $container i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $container local.get $panel_w call $er_ui_app_property_editor_panel_width local.set $w
    local.get $out
    local.get $container f32.load local.get $container i32.const 8 i32.add f32.load f32.add local.get $w f32.sub f32.const 18 f32.sub
    local.get $container i32.const 4 i32.add f32.load f32.const 18 f32.add
    local.get $w
    f32.const 398 local.get $container i32.const 12 i32.add f32.load f32.const 36 f32.sub call $min_f32
    call $rect_store)

  (func $er_ui_app_property_editor_inner_bounds  (param $container i32) (param $panel_w f32) (param $out i32) (result i32)
    local.get $container i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $container local.get $panel_w i32.const 124560 call $er_ui_app_property_editor_panel_bounds drop
    i32.const 124560 f32.const 16 local.get $out call $er_ui_primitives_content_inset)

  (func $er_ui_app_property_editor_title_bounds  (param $inner i32) (param $out i32) (result i32)
    local.get $inner i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $inner f32.load local.get $inner i32.const 4 i32.add f32.load local.get $inner i32.const 8 i32.add f32.load f32.const 42 f32.sub f32.const 24 call $rect_store)

  (func $er_ui_app_property_editor_close_bounds  (param $inner i32) (param $out i32) (result i32)
    local.get $inner i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $inner f32.load local.get $inner i32.const 8 i32.add f32.load f32.add f32.const 34 f32.sub local.get $inner i32.const 4 i32.add f32.load f32.const 2 f32.sub f32.const 32 f32.const 32 call $rect_store)

  (func $er_ui_app_property_editor_preview_bounds  (param $inner i32) (param $out i32) (result i32)
    local.get $inner i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $inner f32.load local.get $inner i32.const 4 i32.add f32.load f32.const 86 f32.add local.get $inner i32.const 8 i32.add f32.load f32.const 74 call $rect_store)

  (func $er_ui_app_property_editor_section_title_bounds  (param $inner i32) (param $out i32) (result i32)
    local.get $inner i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $inner f32.load local.get $inner i32.const 4 i32.add f32.load f32.const 184 f32.add local.get $inner i32.const 8 i32.add f32.load f32.const 20 call $rect_store)

  (func $er_ui_app_property_editor_button_bounds  (param $inner i32) (param $index i32) (param $out i32) (result i32)
    (local $button_w f32) (local $x f32) (local $y f32)
    local.get $inner i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $inner i32.const 8 i32.add f32.load f32.const 10 f32.sub f32.const 0.5 f32.mul local.set $button_w
    local.get $inner f32.load local.set $x
    local.get $index i32.const 0 i32.ne
    if local.get $x local.get $button_w f32.add f32.const 10 f32.add local.set $x end
    local.get $inner i32.const 4 i32.add f32.load f32.const 236 f32.add local.set $y
    local.get $out local.get $x local.get $y local.get $button_w f32.const 34 call $rect_store)

  (func $er_ui_app_property_editor_switch_bounds  (param $inner i32) (param $index i32) (param $out i32) (result i32)
    (local $idx f32)
    local.get $inner i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $index i32.const 0 i32.lt_s
    if
      f32.const 0 local.set $idx
    else
      local.get $index f32.convert_i32_s local.set $idx
    end
    local.get $out
    local.get $inner f32.load
    local.get $inner i32.const 4 i32.add f32.load f32.const 306 f32.add local.get $idx f32.const 40 f32.mul f32.add
    local.get $inner i32.const 8 i32.add f32.load
    f32.const 32
    call $rect_store)

  (func $er_ui_app_section_text_bounds  (param $bounds i32) (param $has_icon i32) (param $detail i32) (param $out i32) (result i32)
    (local $text_x f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds f32.load local.set $text_x
    local.get $has_icon
    if local.get $bounds f32.load f32.const 42 f32.add local.set $text_x end
    local.get $out local.get $text_x local.get $bounds i32.const 4 i32.add f32.load local.get $detail if (result f32) f32.const 23 else f32.const 0 end f32.add local.get $bounds f32.load local.get $bounds i32.const 8 i32.add f32.load f32.add local.get $text_x f32.sub f32.const 1 call $max_f32 local.get $detail if (result f32) f32.const 15 else f32.const 18 end call $rect_store)

  (func $er_ui_app_section_icon_chip_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $bounds f32.load local.get $bounds i32.const 4 i32.add f32.load f32.const 2 f32.add f32.const 28 f32.const 28 call $rect_store)

  (func $er_ui_app_label_value_label_bounds  (param $bounds i32) (param $label_w f32) (param $out i32) (result i32)
    (local $lw f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 8 i32.add f32.load local.get $label_w f32.const 1 call $max_f32 call $min_f32 local.set $lw
    local.get $out local.get $bounds f32.load local.get $bounds i32.const 4 i32.add f32.load local.get $lw local.get $bounds i32.const 12 i32.add f32.load call $rect_store)

  (func $er_ui_app_label_value_value_bounds  (param $bounds i32) (param $label_w f32) (param $out i32) (result i32)
    (local $lw f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 8 i32.add f32.load local.get $label_w f32.const 1 call $max_f32 call $min_f32 local.set $lw
    local.get $bounds i32.const 8 i32.add f32.load local.get $lw f32.le
    if i32.const 0 return end
    local.get $out local.get $bounds f32.load local.get $lw f32.add local.get $bounds i32.const 4 i32.add f32.load local.get $bounds i32.const 8 i32.add f32.load local.get $lw f32.sub f32.const 1 call $max_f32 local.get $bounds i32.const 12 i32.add f32.load call $rect_store)

  (func $er_ui_app_metric_card_inner_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds f32.const 14 local.get $out call $er_ui_primitives_content_inset)

  (func $er_ui_app_metric_card_title_bounds  (param $bounds i32) (param $has_icon i32) (param $out i32) (result i32)
    (local $text_x f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 124576 call $er_ui_app_metric_card_inner_bounds drop
    i32.const 124576 f32.load local.set $text_x
    local.get $has_icon
    if i32.const 124576 f32.load f32.const 40 f32.add local.set $text_x end
    local.get $out local.get $text_x i32.const 124580 f32.load f32.const 1 f32.sub i32.const 124576 f32.load i32.const 124584 f32.load f32.add local.get $text_x f32.sub f32.const 1 call $max_f32 f32.const 17 call $rect_store)

  (func $er_ui_app_metric_card_value_bounds  (param $bounds i32) (param $has_detail i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 124592 call $er_ui_app_metric_card_inner_bounds drop
    local.get $out i32.const 124592 f32.load i32.const 124596 f32.load local.get $has_detail if (result f32) f32.const 58 else f32.const 36 end f32.add i32.const 124600 f32.load f32.const 20 call $rect_store)

  (func $er_ui_app_metric_card_progress_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 124608 call $er_ui_app_metric_card_inner_bounds drop
    local.get $out i32.const 124608 f32.load i32.const 124612 f32.load i32.const 124620 f32.load f32.add f32.const 24 f32.sub i32.const 124616 f32.load f32.const 18 call $rect_store)

  (func $er_ui_app_path_row_marker_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $bounds f32.load f32.const 9 f32.add local.get $bounds i32.const 4 i32.add f32.load f32.const 9 f32.add f32.const 7 local.get $bounds i32.const 12 i32.add f32.load f32.const 18 f32.sub f32.const 1 call $max_f32 call $rect_store)

  (func $er_ui_app_path_row_title_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $bounds f32.load f32.const 24 f32.add local.get $bounds i32.const 4 i32.add f32.load f32.const 7 f32.add local.get $bounds i32.const 8 i32.add f32.load f32.const 92 f32.sub f32.const 1 call $max_f32 f32.const 16 call $rect_store)

  (func $er_ui_app_path_row_detail_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $bounds f32.load f32.const 24 f32.add local.get $bounds i32.const 4 i32.add f32.load f32.const 27 f32.add local.get $bounds i32.const 8 i32.add f32.load f32.const 92 f32.sub f32.const 1 call $max_f32 f32.const 14 call $rect_store)

  (func $er_ui_app_path_row_trailing_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $bounds f32.load local.get $bounds i32.const 8 i32.add f32.load f32.add f32.const 62 f32.sub local.get $bounds i32.const 4 i32.add f32.load f32.const 8 f32.add f32.const 56 f32.const 14 call $rect_store)

  (func $er_ui_app_path_row_progress_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $bounds f32.load local.get $bounds i32.const 8 i32.add f32.load f32.add f32.const 62 f32.sub local.get $bounds i32.const 4 i32.add f32.load local.get $bounds i32.const 12 i32.add f32.load f32.add f32.const 16 f32.sub f32.const 50 f32.const 6 call $rect_store)

  (func $er_ui_app_pipeline_node_marker_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $bounds f32.load f32.const 10 f32.add local.get $bounds i32.const 4 i32.add f32.load f32.const 10 f32.add f32.const 8 local.get $bounds i32.const 12 i32.add f32.load f32.const 20 f32.sub f32.const 1 call $max_f32 call $rect_store)

  (func $er_ui_app_pipeline_node_title_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $bounds f32.load f32.const 28 f32.add local.get $bounds i32.const 4 i32.add f32.load f32.const 10 f32.add local.get $bounds i32.const 8 i32.add f32.load f32.const 36 f32.sub f32.const 1 call $max_f32 f32.const 18 call $rect_store)

  (func $er_ui_app_pipeline_node_detail_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $bounds f32.load f32.const 28 f32.add local.get $bounds i32.const 4 i32.add f32.load f32.const 32 f32.add local.get $bounds i32.const 8 i32.add f32.load f32.const 36 f32.sub f32.const 1 call $max_f32 f32.const 16 call $rect_store)

  (func $er_ui_app_panel_list_body_bounds  (param $bounds i32) (param $inset f32) (param $header_h f32) (param $header_gap f32) (param $out i32) (result i32)
    (local $inner_y f32) (local $inner_h f32) (local $body_y f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds local.get $inset i32.const 124624 call $er_ui_primitives_content_inset drop
    i32.const 124628 f32.load local.set $inner_y
    i32.const 124636 f32.load local.set $inner_h
    local.get $inner_y local.get $header_h f32.const 1 call $max_f32 f32.add local.get $header_gap f32.const 0 call $max_f32 f32.add local.set $body_y
    local.get $out
    i32.const 124624 f32.load
    local.get $body_y
    i32.const 124632 f32.load
    local.get $inner_y local.get $inner_h f32.add local.get $body_y f32.sub f32.const 1 call $max_f32
    call $rect_store)

  (func $er_ui_app_panel_list_row_bounds  (param $body i32) (param $index i32) (param $row_h f32) (param $gap f32) (param $out i32) (result i32)
    (local $idx f32) (local $y f32)
    local.get $body i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $index i32.const 0 i32.lt_s
    if f32.const 0 local.set $idx else local.get $index f32.convert_i32_s local.set $idx end
    local.get $body i32.const 4 i32.add f32.load local.get $idx local.get $row_h local.get $gap f32.const 0 call $max_f32 f32.add f32.mul f32.add local.set $y
    local.get $y local.get $row_h f32.add local.get $body i32.const 4 i32.add f32.load local.get $body i32.const 12 i32.add f32.load f32.add f32.gt
    if i32.const 0 return end
    local.get $out local.get $body f32.load local.get $y local.get $body i32.const 8 i32.add f32.load local.get $row_h call $rect_store)

  (func $er_ui_app_page_header_inner_bounds  (param $bounds i32) (param $inset f32) (param $out i32) (result i32)
    local.get $bounds local.get $inset local.get $out call $er_ui_primitives_content_inset)

  (func $er_ui_app_page_header_icon_chip_bounds  (param $inner i32) (param $out i32) (result i32)
    local.get $inner i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $inner f32.load local.get $inner i32.const 4 i32.add f32.load f32.const 36 f32.const 36 call $rect_store)

  (func $er_ui_app_page_header_text_bounds  (param $inner i32) (param $has_icon i32) (param $badges_w f32) (param $has_action i32) (param $detail i32) (param $out i32) (result i32)
    (local $text_x f32) (local $action_w f32) (local $reserved f32)
    local.get $inner i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $inner f32.load local.set $text_x
    local.get $has_icon
    if local.get $text_x f32.const 52 f32.add local.set $text_x end
    local.get $has_action
    if f32.const 44 local.set $action_w else f32.const 0 local.set $action_w end
    local.get $action_w local.get $badges_w f32.add local.set $reserved
    local.get $badges_w f32.const 0 f32.gt local.get $has_action i32.const 0 i32.ne i32.and
    if local.get $reserved f32.const 10 f32.add local.set $reserved end
    local.get $out
    local.get $text_x
    local.get $inner i32.const 4 i32.add f32.load local.get $detail if (result f32) f32.const 28 else f32.const -2 end f32.add
    local.get $inner f32.load local.get $inner i32.const 8 i32.add f32.load f32.add local.get $text_x f32.sub local.get $reserved f32.sub f32.const 1 call $max_f32
    local.get $detail if (result f32) f32.const 18 else f32.const 24 end
    call $rect_store)

  (func $er_ui_app_page_header_action_bounds  (param $inner i32) (param $out i32) (result i32)
    local.get $inner i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $inner f32.load local.get $inner i32.const 8 i32.add f32.load f32.add f32.const 44 f32.sub local.get $inner i32.const 4 i32.add f32.load f32.const 1 f32.add f32.const 34 f32.const 34 call $rect_store)

  (func $er_ui_app_page_header_badge_cursor_start  (param $inner i32) (param $has_action i32) (result f32)
    (local $x f32)
    local.get $inner i32.eqz
    if f32.const 0 return end
    local.get $inner f32.load local.get $inner i32.const 8 i32.add f32.load f32.add local.set $x
    local.get $has_action
    if local.get $x f32.const 44 f32.sub f32.const 10 f32.sub local.set $x end
    local.get $x)

  (func $er_ui_app_page_header_badge_bounds  (param $inner i32) (param $cursor_x f32) (param $badge_w f32) (param $out i32) (result i32)
    local.get $inner i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $cursor_x local.get $badge_w f32.sub local.get $inner i32.const 4 i32.add f32.load f32.const 4 f32.add local.get $badge_w f32.const 28 call $rect_store)

  (func $er_ui_app_workspace_rail_value_item_bounds  (param $bounds i32) (param $index i32) (param $pad_x f32) (param $pad_top f32) (param $button_h f32) (param $gap f32) (param $out i32) (result i32)
    (local $idx f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $index i32.const 0 i32.lt_s
    if f32.const 0 local.set $idx else local.get $index f32.convert_i32_s local.set $idx end
    local.get $out
    local.get $bounds f32.load local.get $pad_x f32.add
    local.get $bounds i32.const 4 i32.add f32.load local.get $pad_top f32.add local.get $idx local.get $button_h local.get $gap f32.add f32.mul f32.add
    local.get $bounds i32.const 8 i32.add f32.load local.get $pad_x f32.const 2 f32.mul f32.sub f32.const 1 call $max_f32
    local.get $button_h
    call $rect_store)

  (func $er_ui_app_control_group_inner_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds f32.const 14 local.get $out call $er_ui_primitives_content_inset)

  (func $er_ui_app_control_group_title_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 124640 call $er_ui_app_control_group_inner_bounds drop
    local.get $out i32.const 124640 f32.load i32.const 124644 f32.load i32.const 124648 f32.load f32.const 0.55 f32.mul f32.const 18 call $rect_store)

  (func $er_ui_app_control_group_value_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 124656 call $er_ui_app_control_group_inner_bounds drop
    local.get $out i32.const 124656 f32.load i32.const 124664 f32.load f32.const 0.55 f32.mul f32.add i32.const 124660 f32.load f32.const 1 f32.add i32.const 124664 f32.load f32.const 0.45 f32.mul f32.const 15 call $rect_store)

  (func $er_ui_app_control_group_slider_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 124672 call $er_ui_app_control_group_inner_bounds drop
    local.get $out i32.const 124672 f32.load i32.const 124676 f32.load f32.const 27 f32.add i32.const 124680 f32.load f32.const 26 call $rect_store)

  (func $er_ui_app_control_group_button_bounds  (param $bounds i32) (param $index i32) (param $out i32) (result i32)
    (local $half_w f32) (local $x f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 124688 call $er_ui_app_control_group_inner_bounds drop
    i32.const 124696 f32.load f32.const 10 f32.sub f32.const 0.5 f32.mul local.set $half_w
    i32.const 124688 f32.load local.set $x
    local.get $index i32.const 0 i32.ne
    if local.get $x local.get $half_w f32.add f32.const 10 f32.add local.set $x end
    local.get $out local.get $x i32.const 124692 f32.load f32.const 66 f32.add local.get $half_w f32.const 32 call $rect_store)
  (func $er_ui_table_radius  (result f32) f32.const 6)
  (func $er_ui_table_padding_x  (result f32) f32.const 8)
  (func $er_ui_table_header_h  (result f32) f32.const 24)
  (func $er_ui_table_header_y  (result f32) f32.const 5)
  (func $er_ui_table_header_text_h  (result f32) f32.const 14)
  (func $er_ui_table_body_y  (result f32) f32.const 35)
  (func $er_ui_table_body_text_h  (result f32) f32.const 14)
  (func $er_ui_table_name_column_ratio  (result f32) f32.const 0.55)
  (func $er_ui_table_row_inset  (result f32) f32.const 4)
  (func $er_ui_table_row_radius  (result f32) f32.const 4)
  (func $er_ui_table_min_width  (result f32) f32.const 160)
  (func $er_ui_table_min_height  (result f32) f32.const 48)
  (func $er_ui_table_separator_height  (result f32) f32.const 1)

  (func $er_ui_table_row_id  (param $id i32) (result i32)
    local.get $id)

  (func $er_ui_table_name_header_id  (param $id i32) (result i32)
    local.get $id i32.const 1 i32.add)

  (func $er_ui_table_role_header_id  (param $id i32) (result i32)
    local.get $id i32.const 2 i32.add)

  (func $er_ui_table_column_index  (param $column i32) (result i32)
    local.get $column i32.eqz
    if i32.const 0 return end
    i32.const 1)

  (func $er_ui_table_cell_bounds  (param $bounds i32) (param $column i32) (param $y_offset f32) (param $height f32) (param $out i32) (result i32)
    (local $left_w f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 8 i32.add f32.load f32.const 0.55 f32.mul local.set $left_w
    local.get $column i32.eqz
    if
      local.get $out
      local.get $bounds f32.load f32.const 8 f32.add
      local.get $bounds i32.const 4 i32.add f32.load local.get $y_offset f32.add
      local.get $left_w f32.const 8 f32.sub f32.const 1 call $max_f32
      local.get $height
      call $rect_store
      return
    end
    local.get $out
    local.get $bounds f32.load local.get $left_w f32.add
    local.get $bounds i32.const 4 i32.add f32.load local.get $y_offset f32.add
    local.get $bounds i32.const 8 i32.add f32.load local.get $left_w f32.sub f32.const 8 f32.sub f32.const 1 call $max_f32
    local.get $height
    call $rect_store)

  (func $er_ui_table_row_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $bounds f32.load
    local.get $bounds i32.const 4 i32.add f32.load f32.const 25 f32.add
    local.get $bounds i32.const 8 i32.add f32.load
    local.get $bounds i32.const 12 i32.add f32.load f32.const 25 f32.sub f32.const 1 call $max_f32
    call $rect_store)

  (func $er_ui_table_row_fill_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 124736 call $er_ui_table_row_bounds drop
    i32.const 124736 f32.const 4 local.get $out call $er_ui_primitives_content_inset)

  (func $er_ui_table_text_height  (param $bounds i32) (param $column i32) (param $ptr i32) (param $len i32) (param $line_h f32) (param $max_lines i32) (result f32)
    (local $text_w f32) (local $avg f32) (local $lines i32)
    local.get $bounds i32.eqz local.get $ptr i32.eqz i32.or local.get $len i32.eqz i32.or local.get $max_lines i32.eqz i32.or
    if f32.const 0 return end
    local.get $bounds local.get $column f32.const 0 local.get $line_h i32.const 124752 call $er_ui_table_cell_bounds drop
    i32.const 124760 f32.load local.set $text_w
    local.get $ptr local.get $len local.get $line_h call $er_ui_font_text_width
    local.get $len f32.convert_i32_u f32.div
    f32.const 1 call $max_f32
    local.set $avg
    local.get $ptr local.get $len local.get $text_w local.get $avg local.get $max_lines call $er_ui_text_wrapped_line_count local.set $lines
    local.get $bounds i32.const 12 i32.add f32.load
    local.get $lines i32.const 1 call $layout_max_i32_u f32.convert_i32_u local.get $line_h f32.mul
    call $min_f32)

  (func $er_ui_table_header_bounds  (param $bounds i32) (param $column i32) (param $label_ptr i32) (param $label_len i32) (param $out i32) (result i32)
    (local $h f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds local.get $column local.get $label_ptr local.get $label_len f32.const 14 i32.const 1 call $er_ui_table_text_height local.set $h
    local.get $bounds local.get $column f32.const 5 local.get $h local.get $out call $er_ui_table_cell_bounds)

  (func $er_ui_table_body_cell_bounds  (param $bounds i32) (param $column i32) (param $value_ptr i32) (param $value_len i32) (param $out i32) (result i32)
    (local $h f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds local.get $column local.get $value_ptr local.get $value_len f32.const 14 i32.const 2 call $er_ui_table_text_height local.set $h
    local.get $bounds local.get $column f32.const 35 local.get $h local.get $out call $er_ui_table_cell_bounds)

  (func $er_ui_table_measure_preferred  (param $name_ptr i32) (param $name_len i32) (param $role_ptr i32) (param $role_len i32) (param $width f32) (param $out i32) (result i32)
    (local $w f32) (local $name_w f32) (local $role_w f32) (local $name_h f32) (local $role_h f32) (local $row_h f32) (local $pref_w f32)
    local.get $out i32.eqz
    if i32.const 0 return end
    local.get $width f32.const 160 call $max_f32 local.set $w
    local.get $w f32.const 0.55 f32.mul f32.const 8 f32.sub f32.const 1 call $max_f32 local.set $name_w
    local.get $w f32.const 0.45 f32.mul f32.const 8 f32.sub f32.const 1 call $max_f32 local.set $role_w
    local.get $name_ptr local.get $name_len local.get $name_w
    local.get $name_ptr local.get $name_len f32.const 14 call $er_ui_font_text_width local.get $name_len f32.convert_i32_u f32.div f32.const 1 call $max_f32
    i32.const 2 call $er_ui_text_wrapped_line_count i32.const 1 call $layout_max_i32_u f32.convert_i32_u f32.const 14 f32.mul local.set $name_h
    local.get $role_ptr local.get $role_len local.get $role_w
    local.get $role_ptr local.get $role_len f32.const 14 call $er_ui_font_text_width local.get $role_len f32.convert_i32_u f32.div f32.const 1 call $max_f32
    i32.const 2 call $er_ui_text_wrapped_line_count i32.const 1 call $layout_max_i32_u f32.convert_i32_u f32.const 14 f32.mul local.set $role_h
    f32.const 14 local.get $name_h local.get $role_h call $max_f32 call $max_f32 f32.const 8 f32.add local.set $row_h
    f32.const 160
    local.get $name_ptr local.get $name_len f32.const 14 call $er_ui_font_text_width
    local.get $role_ptr local.get $role_len f32.const 14 call $er_ui_font_text_width
    f32.add f32.const 16 f32.add
    call $max_f32 local.set $pref_w
    local.get $out local.get $pref_w f32.store
    local.get $out i32.const 4 i32.add f32.const 25 local.get $row_h f32.add f32.store
    i32.const 1)

)
