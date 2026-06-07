(func (export "er_ui_app_panel_scaffold_default_inset") (result f32) f32.const 16)
  (func (export "er_ui_app_panel_scaffold_default_header_h") (result f32) f32.const 42)
  (func (export "er_ui_app_panel_scaffold_default_header_gap") (result f32) f32.const 16)
  (func (export "er_ui_app_panel_list_default_row_h") (result f32) f32.const 42)
  (func (export "er_ui_app_panel_list_default_gap") (result f32) f32.const 4)
  (func (export "er_ui_app_action_toolbar_button_w") (result f32) f32.const 34)
  (func (export "er_ui_app_action_toolbar_button_h") (result f32) f32.const 36)
  (func (export "er_ui_app_action_toolbar_gap") (result f32) f32.const 8)
  (func (export "er_ui_app_workspace_rail_pad_x") (result f32) f32.const 6)
  (func (export "er_ui_app_workspace_rail_pad_top") (result f32) f32.const 12)
  (func (export "er_ui_app_workspace_sidebar_inset_x") (result f32) f32.const 16)
  (func (export "er_ui_app_workspace_sidebar_title_y") (result f32) f32.const 14)
  (func (export "er_ui_app_workspace_sidebar_detail_y") (result f32) f32.const 36)
  (func (export "er_ui_app_workspace_sidebar_body_y") (result f32) f32.const 68)
  (func (export "er_ui_app_workspace_sidebar_right_border_w") (result f32) f32.const 1)
  (func (export "er_ui_app_compose_bar_height") (result f32) f32.const 74)
  (func (export "er_ui_app_compose_bar_inset_x") (result f32) f32.const 18)
  (func (export "er_ui_app_compose_bar_toolbar_w") (result f32) f32.const 120)
  (func (export "er_ui_app_compose_bar_toolbar_button_w") (result f32) f32.const 34)
  (func (export "er_ui_app_compose_bar_toolbar_gap") (result f32) f32.const 6)
  (func (export "er_ui_app_compose_bar_textarea_gap") (result f32) f32.const 6)
  (func (export "er_ui_app_compose_bar_send_w") (result f32) f32.const 44)
  (func (export "er_ui_app_floating_panel_radius") (result f32) f32.const 12)
  (func (export "er_ui_app_floating_panel_shadow_size") (result f32) f32.const 8)
  (func (export "er_ui_app_floating_panel_shadow_outset") (result f32) f32.const 2)
  (func (export "er_ui_app_floating_panel_inset") (result f32) f32.const 16)
  (func (export "er_ui_app_segment_map_gap") (result f32) f32.const 5)
  (func (export "er_ui_app_segment_map_radius") (result f32) f32.const 8)

  (func (export "er_ui_app_header_badges_width") (param $count i32) (param $badge_w f32) (result f32)
    local.get $count i32.eqz
    if f32.const 0 return end
    local.get $count f32.convert_i32_u local.get $badge_w f32.const 10 f32.add f32.mul)

  (func (export "er_ui_app_panel_scaffold_header_bounds") (param $bounds i32) (param $inset f32) (param $header_h f32) (param $out i32) (result i32)
    (local $header f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $header_h f32.const 1 call $max_f32 local.set $header
    local.get $bounds local.get $inset i32.const 124500 call $er_ui_primitives_content_inset drop
    local.get $out i32.const 124500 f32.load i32.const 124504 f32.load i32.const 124508 f32.load local.get $header call $rect_store)

  (func (export "er_ui_app_panel_scaffold_body_bounds") (param $bounds i32) (param $inset f32) (param $header_h f32) (param $header_gap f32) (param $out i32) (result i32)
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

  (func (export "er_ui_app_action_toolbar_item_bounds") (param $bounds i32) (param $index i32) (param $direction i32) (param $button_w f32) (param $button_h f32) (param $gap f32) (param $out i32) (result i32)
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

  (func (export "er_ui_app_workspace_rail_toolbar_bounds") (param $bounds i32) (param $pad_x f32) (param $pad_top f32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $bounds f32.load local.get $pad_x f32.add
    local.get $bounds i32.const 4 i32.add f32.load local.get $pad_top f32.add
    local.get $bounds i32.const 8 i32.add f32.load local.get $pad_x f32.const 2 f32.mul f32.sub f32.const 1 call $max_f32
    local.get $bounds i32.const 12 i32.add f32.load local.get $pad_top f32.sub f32.const 1 call $max_f32
    call $rect_store)

  (func (export "er_ui_app_workspace_sidebar_right_border_bounds") (param $bounds i32) (param $right_border_w f32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $right_border_w f32.const 0 f32.le
    if i32.const 0 return end
    local.get $out local.get $bounds f32.load local.get $bounds i32.const 8 i32.add f32.load f32.add local.get $right_border_w f32.sub local.get $bounds i32.const 4 i32.add f32.load local.get $right_border_w local.get $bounds i32.const 12 i32.add f32.load call $rect_store)

  (func (export "er_ui_app_workspace_sidebar_title_bounds") (param $bounds i32) (param $inset_x f32) (param $title_y f32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $bounds f32.load local.get $inset_x f32.add local.get $bounds i32.const 4 i32.add f32.load local.get $title_y f32.add local.get $bounds i32.const 8 i32.add f32.load local.get $inset_x f32.const 2 f32.mul f32.sub f32.const 1 call $max_f32 f32.const 16 call $rect_store)

  (func (export "er_ui_app_workspace_sidebar_detail_bounds") (param $bounds i32) (param $inset_x f32) (param $detail_y f32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $bounds f32.load local.get $inset_x f32.add local.get $bounds i32.const 4 i32.add f32.load local.get $detail_y f32.add local.get $bounds i32.const 8 i32.add f32.load local.get $inset_x f32.const 2 f32.mul f32.sub f32.const 1 call $max_f32 f32.const 14 call $rect_store)

  (func (export "er_ui_app_workspace_sidebar_body_bounds") (param $bounds i32) (param $inset_x f32) (param $body_y f32) (param $out i32) (result i32)
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

  (func (export "er_ui_app_compose_toolbar_bounds") (param $bounds i32) (param $inset_x f32) (param $toolbar_w f32) (param $out i32) (result i32)
    (local $tool_y f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 4 i32.add f32.load local.get $bounds i32.const 12 i32.add f32.load f32.const 38 f32.sub f32.const 0 call $max_f32 f32.const 0.5 f32.mul f32.add local.set $tool_y
    local.get $out local.get $bounds f32.load local.get $inset_x f32.add local.get $tool_y local.get $toolbar_w f32.const 38 call $rect_store)

  (func (export "er_ui_app_compose_textarea_bounds") (param $bounds i32) (param $inset_x f32) (param $toolbar_w f32) (param $textarea_gap f32) (param $send_w f32) (param $out i32) (result i32)
    (local $textarea_x f32) (local $send_x f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds f32.load local.get $inset_x f32.add local.get $toolbar_w f32.add local.get $textarea_gap f32.add local.set $textarea_x
    local.get $bounds f32.load local.get $bounds i32.const 8 i32.add f32.load f32.add local.get $inset_x f32.sub local.get $send_w f32.sub local.set $send_x
    local.get $out local.get $textarea_x local.get $bounds i32.const 4 i32.add f32.load f32.const 15 f32.add local.get $send_x local.get $textarea_x f32.sub local.get $textarea_gap f32.sub f32.const 1 call $max_f32 f32.const 44 call $rect_store)

  (func (export "er_ui_app_compose_send_bounds") (param $bounds i32) (param $inset_x f32) (param $send_w f32) (param $out i32) (result i32)
    (local $tool_y f32) (local $send_x f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 4 i32.add f32.load local.get $bounds i32.const 12 i32.add f32.load f32.const 38 f32.sub f32.const 0 call $max_f32 f32.const 0.5 f32.mul f32.add local.set $tool_y
    local.get $bounds f32.load local.get $bounds i32.const 8 i32.add f32.load f32.add local.get $inset_x f32.sub local.get $send_w f32.sub local.set $send_x
    local.get $out local.get $send_x local.get $tool_y local.get $send_w f32.const 38 call $rect_store)

  (func (export "er_ui_app_compose_footer_bounds") (param $bounds i32) (param $inset_x f32) (param $toolbar_w f32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $bounds f32.load local.get $inset_x f32.add local.get $bounds i32.const 4 i32.add f32.load local.get $bounds i32.const 12 i32.add f32.load f32.add f32.const 18 f32.sub local.get $toolbar_w f32.const 40 f32.add f32.const 1 call $max_f32 f32.const 14 call $rect_store)

  (func (export "er_ui_app_floating_panel_inner_bounds") (param $bounds i32) (param $inset f32) (param $out i32) (result i32)
    local.get $bounds local.get $inset local.get $out call $er_ui_primitives_content_inset)

  (func (export "er_ui_app_floating_panel_shadow_bounds") (param $bounds i32) (param $shadow_outset f32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds local.get $out local.get $shadow_outset f32.neg call $er_ui_rect_inset_uniform)

  (func (export "er_ui_app_floating_panel_scrim_bounds") (param $bounds i32) (param $scrim_h f32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $scrim_h f32.const 0 f32.le
    if i32.const 0 return end
    local.get $out local.get $bounds f32.load local.get $bounds i32.const 4 i32.add f32.load local.get $bounds i32.const 8 i32.add f32.load local.get $scrim_h call $rect_store)

  (func (export "er_ui_app_message_bubble_body_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds f32.const 11 local.get $out call $er_ui_primitives_content_inset)

  (func (export "er_ui_app_message_bubble_media_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $bounds f32.load f32.const 10 f32.add local.get $bounds i32.const 4 i32.add f32.load f32.const 42 f32.add local.get $bounds i32.const 8 i32.add f32.load f32.const 20 f32.sub f32.const 1 call $max_f32 f32.const 66 call $rect_store)

  (func (export "er_ui_app_message_bubble_media_icon_bounds") (param $media i32) (param $out i32) (result i32)
    local.get $media i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $media f32.load local.get $media i32.const 8 i32.add f32.load f32.add f32.const 28 f32.sub local.get $media i32.const 4 i32.add f32.load f32.const 10 f32.add f32.const 18 f32.const 18 call $rect_store)

  (func $er_ui_app_segment_map_inner_bounds (export "er_ui_app_segment_map_inner_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds f32.const 8 local.get $out call $er_ui_primitives_content_inset)

  (func (export "er_ui_app_segment_map_block_bounds") (param $bounds i32) (param $segment_index i32) (param $segment_count i32) (param $cursor_x f32) (param $total_weight f32) (param $weight f32) (param $height_unit f32) (param $out i32) (result i32)
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

  (func (export "er_ui_app_context_panel_bounds") (param $container i32) (param $x f32) (param $y f32) (param $w f32) (param $h f32) (param $out i32) (result i32)
    local.get $container i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $x local.get $container f32.load f32.const 8 f32.add local.get $container f32.load local.get $container i32.const 8 i32.add f32.load f32.add local.get $w f32.sub f32.const 8 f32.sub call $clamp_f32
    local.get $y local.get $container i32.const 4 i32.add f32.load f32.const 8 f32.add local.get $container i32.const 4 i32.add f32.load local.get $container i32.const 12 i32.add f32.load f32.add local.get $h f32.sub f32.const 8 f32.sub call $clamp_f32
    local.get $w
    local.get $h
    call $rect_store)

  (func (export "er_ui_app_context_panel_title_bounds") (param $panel i32) (param $out i32) (result i32)
    local.get $panel i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $panel f32.load f32.const 14 f32.add local.get $panel i32.const 4 i32.add f32.load f32.const 12 f32.add local.get $panel i32.const 8 i32.add f32.load f32.const 28 f32.sub f32.const 18 call $rect_store)

  (func (export "er_ui_app_context_panel_detail_bounds") (param $panel i32) (param $out i32) (result i32)
    local.get $panel i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $panel f32.load f32.const 14 f32.add local.get $panel i32.const 4 i32.add f32.load f32.const 36 f32.add local.get $panel i32.const 8 i32.add f32.load f32.const 28 f32.sub f32.const 16 call $rect_store)

  (func (export "er_ui_app_context_panel_primary_bounds") (param $panel i32) (param $out i32) (result i32)
    local.get $panel i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $panel f32.load f32.const 12 f32.add local.get $panel i32.const 4 i32.add f32.load f32.const 66 f32.add f32.const 118 f32.const 34 call $rect_store)

  (func (export "er_ui_app_context_panel_secondary_bounds") (param $panel i32) (param $out i32) (result i32)
    local.get $panel i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $panel f32.load f32.const 138 f32.add local.get $panel i32.const 4 i32.add f32.load f32.const 66 f32.add f32.const 70 f32.const 34 call $rect_store)

  (func (export "er_ui_app_overlay_motion_dy") (param $progress f32) (result f32)
    f32.const 1 local.get $progress f32.sub f32.const 8 f32.mul)

  (func (export "er_ui_app_overlay_motion_dx") (param $progress f32) (result f32)
    f32.const 1 local.get $progress f32.sub f32.const 18 f32.mul)

  (func $er_ui_app_property_editor_panel_width (export "er_ui_app_property_editor_panel_width") (param $container i32) (param $panel_w f32) (result f32)
    local.get $container i32.eqz
    if f32.const 0 return end
    local.get $panel_w local.get $container i32.const 8 i32.add f32.load f32.const 0.24 f32.mul f32.const 300 call $max_f32 call $min_f32)

  (func $er_ui_app_property_editor_panel_bounds (export "er_ui_app_property_editor_panel_bounds") (param $container i32) (param $panel_w f32) (param $out i32) (result i32)
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

  (func $er_ui_app_property_editor_inner_bounds (export "er_ui_app_property_editor_inner_bounds") (param $container i32) (param $panel_w f32) (param $out i32) (result i32)
    local.get $container i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $container local.get $panel_w i32.const 124560 call $er_ui_app_property_editor_panel_bounds drop
    i32.const 124560 f32.const 16 local.get $out call $er_ui_primitives_content_inset)

  (func (export "er_ui_app_property_editor_title_bounds") (param $inner i32) (param $out i32) (result i32)
    local.get $inner i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $inner f32.load local.get $inner i32.const 4 i32.add f32.load local.get $inner i32.const 8 i32.add f32.load f32.const 42 f32.sub f32.const 24 call $rect_store)

  (func (export "er_ui_app_property_editor_close_bounds") (param $inner i32) (param $out i32) (result i32)
    local.get $inner i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $inner f32.load local.get $inner i32.const 8 i32.add f32.load f32.add f32.const 34 f32.sub local.get $inner i32.const 4 i32.add f32.load f32.const 2 f32.sub f32.const 32 f32.const 32 call $rect_store)

  (func (export "er_ui_app_property_editor_preview_bounds") (param $inner i32) (param $out i32) (result i32)
    local.get $inner i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $inner f32.load local.get $inner i32.const 4 i32.add f32.load f32.const 86 f32.add local.get $inner i32.const 8 i32.add f32.load f32.const 74 call $rect_store)

  (func (export "er_ui_app_property_editor_section_title_bounds") (param $inner i32) (param $out i32) (result i32)
    local.get $inner i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $inner f32.load local.get $inner i32.const 4 i32.add f32.load f32.const 184 f32.add local.get $inner i32.const 8 i32.add f32.load f32.const 20 call $rect_store)

  (func (export "er_ui_app_property_editor_button_bounds") (param $inner i32) (param $index i32) (param $out i32) (result i32)
    (local $button_w f32) (local $x f32) (local $y f32)
    local.get $inner i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $inner i32.const 8 i32.add f32.load f32.const 10 f32.sub f32.const 0.5 f32.mul local.set $button_w
    local.get $inner f32.load local.set $x
    local.get $index i32.const 0 i32.ne
    if local.get $x local.get $button_w f32.add f32.const 10 f32.add local.set $x end
    local.get $inner i32.const 4 i32.add f32.load f32.const 236 f32.add local.set $y
    local.get $out local.get $x local.get $y local.get $button_w f32.const 34 call $rect_store)

  (func (export "er_ui_app_property_editor_switch_bounds") (param $inner i32) (param $index i32) (param $out i32) (result i32)
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

  (func (export "er_ui_app_section_text_bounds") (param $bounds i32) (param $has_icon i32) (param $detail i32) (param $out i32) (result i32)
    (local $text_x f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds f32.load local.set $text_x
    local.get $has_icon
    if local.get $bounds f32.load f32.const 42 f32.add local.set $text_x end
    local.get $out local.get $text_x local.get $bounds i32.const 4 i32.add f32.load local.get $detail if (result f32) f32.const 23 else f32.const 0 end f32.add local.get $bounds f32.load local.get $bounds i32.const 8 i32.add f32.load f32.add local.get $text_x f32.sub f32.const 1 call $max_f32 local.get $detail if (result f32) f32.const 15 else f32.const 18 end call $rect_store)

  (func (export "er_ui_app_section_icon_chip_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $bounds f32.load local.get $bounds i32.const 4 i32.add f32.load f32.const 2 f32.add f32.const 28 f32.const 28 call $rect_store)

  (func (export "er_ui_app_label_value_label_bounds") (param $bounds i32) (param $label_w f32) (param $out i32) (result i32)
    (local $lw f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 8 i32.add f32.load local.get $label_w f32.const 1 call $max_f32 call $min_f32 local.set $lw
    local.get $out local.get $bounds f32.load local.get $bounds i32.const 4 i32.add f32.load local.get $lw local.get $bounds i32.const 12 i32.add f32.load call $rect_store)

  (func (export "er_ui_app_label_value_value_bounds") (param $bounds i32) (param $label_w f32) (param $out i32) (result i32)
    (local $lw f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 8 i32.add f32.load local.get $label_w f32.const 1 call $max_f32 call $min_f32 local.set $lw
    local.get $bounds i32.const 8 i32.add f32.load local.get $lw f32.le
    if i32.const 0 return end
    local.get $out local.get $bounds f32.load local.get $lw f32.add local.get $bounds i32.const 4 i32.add f32.load local.get $bounds i32.const 8 i32.add f32.load local.get $lw f32.sub f32.const 1 call $max_f32 local.get $bounds i32.const 12 i32.add f32.load call $rect_store)

  (func $er_ui_app_metric_card_inner_bounds (export "er_ui_app_metric_card_inner_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds f32.const 14 local.get $out call $er_ui_primitives_content_inset)

  (func (export "er_ui_app_metric_card_title_bounds") (param $bounds i32) (param $has_icon i32) (param $out i32) (result i32)
    (local $text_x f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 124576 call $er_ui_app_metric_card_inner_bounds drop
    i32.const 124576 f32.load local.set $text_x
    local.get $has_icon
    if i32.const 124576 f32.load f32.const 40 f32.add local.set $text_x end
    local.get $out local.get $text_x i32.const 124580 f32.load f32.const 1 f32.sub i32.const 124576 f32.load i32.const 124584 f32.load f32.add local.get $text_x f32.sub f32.const 1 call $max_f32 f32.const 17 call $rect_store)

  (func (export "er_ui_app_metric_card_value_bounds") (param $bounds i32) (param $has_detail i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 124592 call $er_ui_app_metric_card_inner_bounds drop
    local.get $out i32.const 124592 f32.load i32.const 124596 f32.load local.get $has_detail if (result f32) f32.const 58 else f32.const 36 end f32.add i32.const 124600 f32.load f32.const 20 call $rect_store)

  (func (export "er_ui_app_metric_card_progress_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 124608 call $er_ui_app_metric_card_inner_bounds drop
    local.get $out i32.const 124608 f32.load i32.const 124612 f32.load i32.const 124620 f32.load f32.add f32.const 24 f32.sub i32.const 124616 f32.load f32.const 18 call $rect_store)

  (func (export "er_ui_app_path_row_marker_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $bounds f32.load f32.const 9 f32.add local.get $bounds i32.const 4 i32.add f32.load f32.const 9 f32.add f32.const 7 local.get $bounds i32.const 12 i32.add f32.load f32.const 18 f32.sub f32.const 1 call $max_f32 call $rect_store)

  (func (export "er_ui_app_path_row_title_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $bounds f32.load f32.const 24 f32.add local.get $bounds i32.const 4 i32.add f32.load f32.const 7 f32.add local.get $bounds i32.const 8 i32.add f32.load f32.const 92 f32.sub f32.const 1 call $max_f32 f32.const 16 call $rect_store)

  (func (export "er_ui_app_path_row_detail_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $bounds f32.load f32.const 24 f32.add local.get $bounds i32.const 4 i32.add f32.load f32.const 27 f32.add local.get $bounds i32.const 8 i32.add f32.load f32.const 92 f32.sub f32.const 1 call $max_f32 f32.const 14 call $rect_store)

  (func (export "er_ui_app_path_row_trailing_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $bounds f32.load local.get $bounds i32.const 8 i32.add f32.load f32.add f32.const 62 f32.sub local.get $bounds i32.const 4 i32.add f32.load f32.const 8 f32.add f32.const 56 f32.const 14 call $rect_store)

  (func (export "er_ui_app_path_row_progress_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $bounds f32.load local.get $bounds i32.const 8 i32.add f32.load f32.add f32.const 62 f32.sub local.get $bounds i32.const 4 i32.add f32.load local.get $bounds i32.const 12 i32.add f32.load f32.add f32.const 16 f32.sub f32.const 50 f32.const 6 call $rect_store)

  (func (export "er_ui_app_pipeline_node_marker_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $bounds f32.load f32.const 10 f32.add local.get $bounds i32.const 4 i32.add f32.load f32.const 10 f32.add f32.const 8 local.get $bounds i32.const 12 i32.add f32.load f32.const 20 f32.sub f32.const 1 call $max_f32 call $rect_store)

  (func (export "er_ui_app_pipeline_node_title_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $bounds f32.load f32.const 28 f32.add local.get $bounds i32.const 4 i32.add f32.load f32.const 10 f32.add local.get $bounds i32.const 8 i32.add f32.load f32.const 36 f32.sub f32.const 1 call $max_f32 f32.const 18 call $rect_store)

  (func (export "er_ui_app_pipeline_node_detail_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $bounds f32.load f32.const 28 f32.add local.get $bounds i32.const 4 i32.add f32.load f32.const 32 f32.add local.get $bounds i32.const 8 i32.add f32.load f32.const 36 f32.sub f32.const 1 call $max_f32 f32.const 16 call $rect_store)

  (func (export "er_ui_app_panel_list_body_bounds") (param $bounds i32) (param $inset f32) (param $header_h f32) (param $header_gap f32) (param $out i32) (result i32)
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

  (func (export "er_ui_app_panel_list_row_bounds") (param $body i32) (param $index i32) (param $row_h f32) (param $gap f32) (param $out i32) (result i32)
    (local $idx f32) (local $y f32)
    local.get $body i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $index i32.const 0 i32.lt_s
    if f32.const 0 local.set $idx else local.get $index f32.convert_i32_s local.set $idx end
    local.get $body i32.const 4 i32.add f32.load local.get $idx local.get $row_h local.get $gap f32.const 0 call $max_f32 f32.add f32.mul f32.add local.set $y
    local.get $y local.get $row_h f32.add local.get $body i32.const 4 i32.add f32.load local.get $body i32.const 12 i32.add f32.load f32.add f32.gt
    if i32.const 0 return end
    local.get $out local.get $body f32.load local.get $y local.get $body i32.const 8 i32.add f32.load local.get $row_h call $rect_store)

  (func (export "er_ui_app_page_header_inner_bounds") (param $bounds i32) (param $inset f32) (param $out i32) (result i32)
    local.get $bounds local.get $inset local.get $out call $er_ui_primitives_content_inset)

  (func (export "er_ui_app_page_header_icon_chip_bounds") (param $inner i32) (param $out i32) (result i32)
    local.get $inner i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $inner f32.load local.get $inner i32.const 4 i32.add f32.load f32.const 36 f32.const 36 call $rect_store)

  (func (export "er_ui_app_page_header_text_bounds") (param $inner i32) (param $has_icon i32) (param $badges_w f32) (param $has_action i32) (param $detail i32) (param $out i32) (result i32)
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

  (func (export "er_ui_app_page_header_action_bounds") (param $inner i32) (param $out i32) (result i32)
    local.get $inner i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $inner f32.load local.get $inner i32.const 8 i32.add f32.load f32.add f32.const 44 f32.sub local.get $inner i32.const 4 i32.add f32.load f32.const 1 f32.add f32.const 34 f32.const 34 call $rect_store)

  (func (export "er_ui_app_page_header_badge_cursor_start") (param $inner i32) (param $has_action i32) (result f32)
    (local $x f32)
    local.get $inner i32.eqz
    if f32.const 0 return end
    local.get $inner f32.load local.get $inner i32.const 8 i32.add f32.load f32.add local.set $x
    local.get $has_action
    if local.get $x f32.const 44 f32.sub f32.const 10 f32.sub local.set $x end
    local.get $x)

  (func (export "er_ui_app_page_header_badge_bounds") (param $inner i32) (param $cursor_x f32) (param $badge_w f32) (param $out i32) (result i32)
    local.get $inner i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $cursor_x local.get $badge_w f32.sub local.get $inner i32.const 4 i32.add f32.load f32.const 4 f32.add local.get $badge_w f32.const 28 call $rect_store)

  (func (export "er_ui_app_workspace_rail_value_item_bounds") (param $bounds i32) (param $index i32) (param $pad_x f32) (param $pad_top f32) (param $button_h f32) (param $gap f32) (param $out i32) (result i32)
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

  (func $er_ui_app_control_group_inner_bounds (export "er_ui_app_control_group_inner_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds f32.const 14 local.get $out call $er_ui_primitives_content_inset)

  (func (export "er_ui_app_control_group_title_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 124640 call $er_ui_app_control_group_inner_bounds drop
    local.get $out i32.const 124640 f32.load i32.const 124644 f32.load i32.const 124648 f32.load f32.const 0.55 f32.mul f32.const 18 call $rect_store)

  (func (export "er_ui_app_control_group_value_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 124656 call $er_ui_app_control_group_inner_bounds drop
    local.get $out i32.const 124656 f32.load i32.const 124664 f32.load f32.const 0.55 f32.mul f32.add i32.const 124660 f32.load f32.const 1 f32.add i32.const 124664 f32.load f32.const 0.45 f32.mul f32.const 15 call $rect_store)

  (func (export "er_ui_app_control_group_slider_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 124672 call $er_ui_app_control_group_inner_bounds drop
    local.get $out i32.const 124672 f32.load i32.const 124676 f32.load f32.const 27 f32.add i32.const 124680 f32.load f32.const 26 call $rect_store)

  (func (export "er_ui_app_control_group_button_bounds") (param $bounds i32) (param $index i32) (param $out i32) (result i32)
    (local $half_w f32) (local $x f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 124688 call $er_ui_app_control_group_inner_bounds drop
    i32.const 124696 f32.load f32.const 10 f32.sub f32.const 0.5 f32.mul local.set $half_w
    i32.const 124688 f32.load local.set $x
    local.get $index i32.const 0 i32.ne
    if local.get $x local.get $half_w f32.add f32.const 10 f32.add local.set $x end
    local.get $out local.get $x i32.const 124692 f32.load f32.const 66 f32.add local.get $half_w f32.const 32 call $rect_store)
