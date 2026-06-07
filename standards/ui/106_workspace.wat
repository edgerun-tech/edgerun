(func (export "er_ui_workspace_default_rail_w") (result f32) f32.const 48)
  (func (export "er_ui_workspace_default_sidebar_w") (result f32) f32.const 260)
  (func (export "er_ui_workspace_default_top_h") (result f32) f32.const 56)
  (func (export "er_ui_workspace_default_status_h") (result f32) f32.const 24)
  (func (export "er_ui_workspace_top_trailing_default_w") (result f32) f32.const 210)
  (func (export "er_ui_workspace_top_inset_x") (result f32) f32.const 16)
  (func (export "er_ui_workspace_top_title_y") (result f32) f32.const 13)
  (func (export "er_ui_workspace_top_title_h") (result f32) f32.const 18)
  (func (export "er_ui_workspace_top_detail_y") (result f32) f32.const 34)
  (func (export "er_ui_workspace_top_detail_h") (result f32) f32.const 14)
  (func (export "er_ui_workspace_top_trailing_gap") (result f32) f32.const 20)
  (func (export "er_ui_workspace_top_trailing_top_y") (result f32) f32.const 13)
  (func (export "er_ui_workspace_top_trailing_bottom_y") (result f32) f32.const 32)
  (func (export "er_ui_workspace_status_inset_x") (result f32) f32.const 12)
  (func (export "er_ui_workspace_status_text_y") (result f32) f32.const 5)
  (func (export "er_ui_workspace_status_text_h") (result f32) f32.const 14)
  (func (export "er_ui_workspace_responsive_default_breakpoint") (result f32) f32.const 980)
  (func (export "er_ui_workspace_responsive_default_gap") (result f32) f32.const 14)

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

  (func (export "er_ui_workspace_shell_rail_bounds") (param $bounds i32) (param $rail_w f32) (param $sidebar_w f32) (param $top_h f32) (param $status_h f32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds local.get $rail_w local.get $sidebar_w local.get $top_h local.get $status_h i32.const 124380 call $er_ui_workspace_shell_metrics drop
    local.get $out local.get $bounds f32.load local.get $bounds i32.const 4 i32.add f32.load i32.const 124380 f32.load i32.const 124400 f32.load call $rect_store)

  (func (export "er_ui_workspace_shell_top_bounds") (param $bounds i32) (param $rail_w f32) (param $sidebar_w f32) (param $top_h f32) (param $status_h f32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds local.get $rail_w local.get $sidebar_w local.get $top_h local.get $status_h i32.const 124380 call $er_ui_workspace_shell_metrics drop
    local.get $out i32.const 124404 f32.load local.get $bounds i32.const 4 i32.add f32.load i32.const 124408 f32.load i32.const 124388 f32.load call $rect_store)

  (func (export "er_ui_workspace_shell_sidebar_bounds") (param $bounds i32) (param $rail_w f32) (param $sidebar_w f32) (param $top_h f32) (param $status_h f32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds local.get $rail_w local.get $sidebar_w local.get $top_h local.get $status_h i32.const 124380 call $er_ui_workspace_shell_metrics drop
    local.get $out i32.const 124404 f32.load i32.const 124396 f32.load i32.const 124412 f32.load i32.const 124392 f32.load call $rect_store)

  (func (export "er_ui_workspace_shell_main_bounds") (param $bounds i32) (param $rail_w f32) (param $sidebar_w f32) (param $top_h f32) (param $status_h f32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds local.get $rail_w local.get $sidebar_w local.get $top_h local.get $status_h i32.const 124380 call $er_ui_workspace_shell_metrics drop
    local.get $out i32.const 124404 f32.load i32.const 124412 f32.load f32.add i32.const 124396 f32.load i32.const 124408 f32.load i32.const 124412 f32.load f32.sub f32.const 1 call $max_f32 i32.const 124392 f32.load call $rect_store)

  (func (export "er_ui_workspace_shell_status_bounds") (param $bounds i32) (param $rail_w f32) (param $sidebar_w f32) (param $top_h f32) (param $status_h f32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds local.get $rail_w local.get $sidebar_w local.get $top_h local.get $status_h i32.const 124380 call $er_ui_workspace_shell_metrics drop
    local.get $out local.get $bounds f32.load local.get $bounds i32.const 4 i32.add f32.load local.get $bounds i32.const 12 i32.add f32.load f32.add i32.const 124384 f32.load f32.sub local.get $bounds i32.const 8 i32.add f32.load i32.const 124384 f32.load call $rect_store)

  (func $er_ui_workspace_top_trailing_width (export "er_ui_workspace_top_trailing_width") (param $bounds i32) (param $has_trailing i32) (param $trailing_w f32) (result f32)
    local.get $bounds i32.eqz local.get $has_trailing i32.eqz i32.or
    if f32.const 0 return end
    local.get $bounds i32.const 8 i32.add f32.load local.get $trailing_w f32.const 1 call $max_f32 call $min_f32)

  (func $er_ui_workspace_top_title_bounds (export "er_ui_workspace_top_title_bounds") (param $bounds i32) (param $has_trailing i32) (param $trailing_w f32) (param $inset_x f32) (param $out i32) (result i32)
    (local $tw f32) (local $gap f32) (local $text_w f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds local.get $has_trailing local.get $trailing_w call $er_ui_workspace_top_trailing_width local.set $tw
    local.get $tw f32.const 0 f32.gt
    if f32.const 20 local.set $gap else f32.const 0 local.set $gap end
    local.get $bounds i32.const 8 i32.add f32.load local.get $tw f32.sub local.get $inset_x f32.const 2 f32.mul f32.sub local.get $gap f32.sub f32.const 1 call $max_f32 local.set $text_w
    local.get $out local.get $bounds f32.load local.get $inset_x f32.add local.get $bounds i32.const 4 i32.add f32.load f32.const 13 f32.add local.get $text_w f32.const 18 call $rect_store)

  (func (export "er_ui_workspace_top_detail_bounds") (param $bounds i32) (param $has_trailing i32) (param $trailing_w f32) (param $inset_x f32) (param $out i32) (result i32)
    local.get $bounds local.get $has_trailing local.get $trailing_w local.get $inset_x local.get $out call $er_ui_workspace_top_title_bounds drop
    local.get $out i32.eqz
    if i32.const 0 return end
    local.get $out i32.const 4 i32.add local.get $bounds i32.const 4 i32.add f32.load f32.const 34 f32.add f32.store
    local.get $out i32.const 12 i32.add f32.const 14 f32.store
    i32.const 1)

  (func $er_ui_workspace_top_trailing_top_bounds (export "er_ui_workspace_top_trailing_top_bounds") (param $bounds i32) (param $trailing_w f32) (param $inset_x f32) (param $out i32) (result i32)
    (local $tw f32) (local $x f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 8 i32.add f32.load local.get $trailing_w f32.const 1 call $max_f32 call $min_f32 local.set $tw
    local.get $bounds f32.load local.get $bounds i32.const 8 i32.add f32.load f32.add local.get $tw f32.sub local.get $inset_x f32.sub local.set $x
    local.get $out local.get $x local.get $bounds i32.const 4 i32.add f32.load f32.const 13 f32.add local.get $tw f32.const 14 call $rect_store)

  (func (export "er_ui_workspace_top_trailing_bottom_bounds") (param $bounds i32) (param $trailing_w f32) (param $inset_x f32) (param $out i32) (result i32)
    local.get $bounds local.get $trailing_w local.get $inset_x local.get $out call $er_ui_workspace_top_trailing_top_bounds drop
    local.get $out i32.eqz
    if i32.const 0 return end
    local.get $out i32.const 4 i32.add local.get $bounds i32.const 4 i32.add f32.load f32.const 32 f32.add f32.store
    i32.const 1)

  (func (export "er_ui_workspace_status_text_bounds") (param $bounds i32) (param $inset_x f32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $bounds f32.load local.get $inset_x f32.add local.get $bounds i32.const 4 i32.add f32.load f32.const 5 f32.add local.get $bounds i32.const 8 i32.add f32.load local.get $inset_x f32.const 2 f32.mul f32.sub f32.const 1 call $max_f32 f32.const 14 call $rect_store)

  (func (export "er_ui_workspace_responsive_stacked") (param $bounds i32) (param $breakpoint f32) (result i32)
    local.get $bounds i32.eqz
    if i32.const 0 return end
    local.get $bounds i32.const 8 i32.add f32.load local.get $breakpoint f32.lt)

  (func (export "er_ui_workspace_responsive_panes") (param $bounds i32) (param $breakpoint f32) (param $gap f32) (param $first_w f32) (param $third_w f32) (param $first_stack_h f32) (param $second_stack_h f32) (param $out i32) (result i32)
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
