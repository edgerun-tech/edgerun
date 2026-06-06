  (func (export "er_ui_gallery_catalog_intro_title_bounds") (param $intro i32) (param $out i32) (result i32)
    local.get $intro i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $intro f32.load local.get $intro i32.const 4 i32.add f32.load local.get $intro i32.const 8 i32.add f32.load f32.const 22 call $rect_store)

  (func (export "er_ui_gallery_catalog_intro_detail_bounds") (param $intro i32) (param $out i32) (result i32)
    local.get $intro i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $intro f32.load local.get $intro i32.const 4 i32.add f32.load f32.const 32 f32.add local.get $intro i32.const 8 i32.add f32.load f32.const 42 call $rect_store)

  (func (export "er_ui_gallery_catalog_selected_bounds") (param $section i32) (param $has_selected i32) (param $out i32) (result i32)
    (local $selected_h f32)
    local.get $section i32.eqz local.get $out i32.eqz i32.or local.get $has_selected i32.eqz i32.or
    if i32.const 0 return end
    local.get $section i32.const 8 i32.add f32.load call $er_ui_gallery_selected_component_height local.set $selected_h
    local.get $out local.get $section f32.load local.get $section i32.const 4 i32.add f32.load local.get $section i32.const 8 i32.add f32.load local.get $selected_h call $rect_store)

  (func (export "er_ui_gallery_catalog_intro_bounds") (param $section i32) (param $has_selected i32) (param $out i32) (result i32)
    (local $y f32)
    local.get $section i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $section i32.const 4 i32.add f32.load local.set $y
    local.get $has_selected
    if
      local.get $y local.get $section i32.const 8 i32.add f32.load call $er_ui_gallery_selected_component_height f32.add f32.const 32 f32.add local.set $y
    end
    local.get $out local.get $section f32.load local.get $y local.get $section i32.const 8 i32.add f32.load f32.const 86 call $rect_store)

  (func (export "er_ui_gallery_catalog_grid_bounds") (param $section i32) (param $has_selected i32) (param $out i32) (result i32)
    (local $y f32)
    local.get $section i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $section i32.const 4 i32.add f32.load f32.const 86 f32.add local.set $y
    local.get $has_selected
    if
      local.get $y local.get $section i32.const 8 i32.add f32.load call $er_ui_gallery_selected_component_height f32.add f32.const 32 f32.add local.set $y
    end
    local.get $out local.get $section f32.load local.get $y local.get $section i32.const 8 i32.add f32.load local.get $section i32.const 12 i32.add f32.load local.get $y local.get $section i32.const 4 i32.add f32.load f32.sub f32.sub f32.const 1 call $max_f32 call $rect_store)

  (func (export "er_ui_gallery_catalog_grid_item_bounds") (param $grid i32) (param $columns i32) (param $gap f32) (param $index i32) (param $out i32) (result i32)
    (local $cols i32) (local $col i32) (local $row i32) (local $item_w f32)
    local.get $grid i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $columns i32.const 1 call $layout_max_i32_u local.set $cols
    local.get $index local.get $cols i32.rem_u local.set $col
    local.get $index local.get $cols i32.div_u local.set $row
    local.get $grid i32.const 8 i32.add f32.load local.get $gap local.get $cols i32.const 1 i32.sub f32.convert_i32_u f32.mul f32.sub local.get $cols f32.convert_i32_u f32.div f32.const 1 call $max_f32 local.set $item_w
    local.get $out
    local.get $grid f32.load local.get $col f32.convert_i32_u local.get $item_w local.get $gap f32.add f32.mul f32.add
    local.get $grid i32.const 4 i32.add f32.load local.get $row f32.convert_i32_u f32.const 148 local.get $gap f32.add f32.mul f32.add
    local.get $item_w
    f32.const 148
    call $rect_store)

  (func (export "er_ui_gallery_catalog_card_inset_bounds") (param $card i32) (param $out i32) (result i32)
    local.get $card local.get $out f32.const 14 call $er_ui_rect_inset_uniform)

  (func (export "er_ui_gallery_catalog_card_source_bounds") (param $card i32) (param $selected i32) (param $out i32) (result i32)
    (local $in_x f32) (local $in_y f32) (local $in_w f32) (local $source_w f32)
    local.get $card i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $card f32.load f32.const 14 f32.add local.set $in_x
    local.get $card i32.const 4 i32.add f32.load f32.const 14 f32.add local.set $in_y
    local.get $card i32.const 8 i32.add f32.load f32.const 28 f32.sub local.set $in_w
    local.get $selected if (result f32) f32.const 94 else f32.const 86.5 end f32.const 82 call $max_f32 local.get $in_w call $min_f32 local.set $source_w
    local.get $out local.get $in_x local.get $in_w f32.add local.get $source_w f32.sub local.get $in_y f32.const 1 f32.sub local.get $source_w f32.const 24 call $rect_store)

  (func (export "er_ui_gallery_catalog_card_builder_bounds") (param $card i32) (param $out i32) (result i32)
    local.get $card i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $card f32.load f32.const 14 f32.add local.get $card i32.const 4 i32.add f32.load f32.const 62 f32.add local.get $card i32.const 8 i32.add f32.load f32.const 28 f32.sub f32.const 16 call $rect_store)

  (func (export "er_ui_gallery_catalog_card_preview_bounds") (param $card i32) (param $out i32) (result i32)
    local.get $card i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $card f32.load f32.const 14 f32.add local.get $card i32.const 4 i32.add f32.load f32.const 90 f32.add local.get $card i32.const 8 i32.add f32.load f32.const 28 f32.sub f32.const 38 call $rect_store)

  (func (export "er_ui_gallery_surface_variant_panel") (result i32) i32.const 0)
  (func (export "er_ui_gallery_surface_variant_elevated") (result i32) i32.const 1)

  (func $er_ui_gallery_rect_contains_exclusive_local (param $rect i32) (param $x f32) (param $y f32) (result i32)
    local.get $rect i32.eqz if i32.const 0 return end
    local.get $x local.get $rect f32.load f32.ge
    local.get $y local.get $rect i32.const 4 i32.add f32.load f32.ge i32.and
    local.get $x local.get $rect f32.load local.get $rect i32.const 8 i32.add f32.load f32.add f32.lt i32.and
    local.get $y local.get $rect i32.const 4 i32.add f32.load local.get $rect i32.const 12 i32.add f32.load f32.add f32.lt i32.and)

  (func $er_ui_gallery_hover_enabled (export "er_ui_gallery_hover_enabled") (param $hover_x f32) (param $hover_y f32) (result i32)
    local.get $hover_x f32.const 0 f32.ge local.get $hover_y f32.const 0 f32.ge i32.and)

  (func $er_ui_gallery_card_hovered (export "er_ui_gallery_card_hovered") (param $card i32) (param $hover_x f32) (param $hover_y f32) (result i32)
    local.get $hover_x local.get $hover_y call $er_ui_gallery_hover_enabled i32.eqz
    if i32.const 0 return end
    local.get $card local.get $hover_x local.get $hover_y call $er_ui_gallery_rect_contains_exclusive_local)

  (func (export "er_ui_gallery_catalog_card_surface_variant") (param $card i32) (param $selected i32) (param $hover_x f32) (param $hover_y f32) (result i32)
    local.get $selected
    if i32.const 1 return end
    local.get $card local.get $hover_x local.get $hover_y call $er_ui_gallery_card_hovered
    if i32.const 1 return end
    i32.const 0)
