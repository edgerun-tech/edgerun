  (func (export "er_ui_gallery_preview_base_id") (result i32) i32.const 18000)
  (func (export "er_ui_gallery_first_catalog_card_id") (result i32) i32.const 20000)
  (func (export "er_ui_gallery_catalog_preview_id_base") (result i32) i32.const 23000)
  (func (export "er_ui_gallery_selected_preview_id_base") (result i32) i32.const 25000)
  (func (export "er_ui_gallery_preview_id_stride") (result i32) i32.const 32)
  (func (export "er_ui_gallery_component_count") (result i32) i32.const 60)
  (func (export "er_ui_gallery_header_h") (result f32) f32.const 56)
  (func (export "er_ui_gallery_page_top_pad") (result f32) f32.const 48)
  (func (export "er_ui_gallery_page_bottom_pad") (result f32) f32.const 120)
  (func (export "er_ui_gallery_card_content_x") (result f32) f32.const 18)
  (func (export "er_ui_gallery_min_column_width") (result f32) f32.const 300)
  (func (export "er_ui_gallery_max_columns") (result i32) i32.const 5)
  (func (export "er_ui_gallery_catalog_intro_h") (result f32) f32.const 86)
  (func (export "er_ui_gallery_catalog_card_h") (result f32) f32.const 148)
  (func (export "er_ui_gallery_catalog_source_min_w") (result f32) f32.const 82)
  (func (export "er_ui_gallery_selected_component_h") (result f32) f32.const 500)
  (func (export "er_ui_gallery_selected_component_compact_h") (result f32) f32.const 820)
  (func (export "er_ui_gallery_selected_component_gap") (result f32) f32.const 32)
  (func (export "er_ui_gallery_grid_gap_compact") (result f32) f32.const 28)
  (func (export "er_ui_gallery_grid_gap_default") (result f32) f32.const 40)
  (func (export "er_ui_gallery_grid_gap_wide") (result f32) f32.const 56)

  (func (export "er_ui_gallery_category_foundation") (result i32) i32.const 0)
  (func (export "er_ui_gallery_category_form") (result i32) i32.const 1)
  (func (export "er_ui_gallery_category_overlay") (result i32) i32.const 2)
  (func (export "er_ui_gallery_category_navigation") (result i32) i32.const 3)
  (func (export "er_ui_gallery_category_data_display") (result i32) i32.const 4)
  (func (export "er_ui_gallery_category_feedback") (result i32) i32.const 5)
  (func (export "er_ui_gallery_category_layout") (result i32) i32.const 6)
  (func (export "er_ui_gallery_category_media") (result i32) i32.const 7)

  (func (export "er_ui_gallery_count_by_category") (param $category i32) (result i32)
    local.get $category
    if (result i32)
      local.get $category i32.const 1 i32.eq
      if (result i32) i32.const 15 else
        local.get $category i32.const 2 i32.eq
        if (result i32) i32.const 10 else
          local.get $category i32.const 0 i32.eq
          if (result i32) i32.const 9 else
            local.get $category i32.const 5 i32.eq
            if (result i32) i32.const 7 else
              local.get $category i32.const 3 i32.eq local.get $category i32.const 6 i32.eq i32.or
              if (result i32) i32.const 6 else
                local.get $category i32.const 4 i32.eq
                if (result i32) i32.const 5 else
                  local.get $category i32.const 7 i32.eq
                  if (result i32) i32.const 2 else i32.const 0 end
                end
              end
            end
          end
        end
      end
    else
      i32.const 9
    end)

  (func $er_ui_gallery_normalized_grid_gap (export "er_ui_gallery_normalized_grid_gap") (param $value f32) (result f32)
    local.get $value f32.const 34 f32.le
    if f32.const 28 return end
    local.get $value f32.const 48 f32.ge
    if f32.const 56 return end
    f32.const 40)

  (func $er_ui_gallery_column_count (export "er_ui_gallery_column_count") (param $width f32) (param $gap f32) (result i32)
    (local $columns i32) (local $next i32) (local $required f32)
    i32.const 1 local.set $columns
    block $done
      loop $loop
        local.get $columns i32.const 5 i32.ge_u br_if $done
        local.get $columns i32.const 1 i32.add local.set $next
        f32.const 300 local.get $next f32.convert_i32_u f32.mul local.get $gap local.get $next i32.const 1 i32.sub f32.convert_i32_u f32.mul f32.add local.set $required
        local.get $required local.get $width f32.gt br_if $done
        local.get $next local.set $columns
        br $loop
      end
    end
    local.get $columns)

  (func $er_ui_gallery_catalog_section_height (export "er_ui_gallery_catalog_section_height") (param $columns i32) (param $gap f32) (result f32)
    (local $cols i32) (local $rows i32)
    local.get $columns i32.const 1 call $layout_max_i32_u local.set $cols
    i32.const 60 local.get $cols i32.add i32.const 1 i32.sub local.get $cols i32.div_u local.set $rows
    f32.const 86 local.get $rows f32.convert_i32_u f32.const 148 f32.mul f32.add local.get $rows i32.const 1 i32.sub f32.convert_i32_u local.get $gap f32.mul f32.add)

  (func $er_ui_gallery_selected_component_height (export "er_ui_gallery_selected_component_height") (param $width f32) (result f32)
    local.get $width f32.const 760 f32.lt
    if f32.const 820 return end
    f32.const 500)

  (func $er_ui_gallery_body_height (export "er_ui_gallery_body_height") (param $width f32) (param $columns i32) (param $gap f32) (param $has_selected i32) (result f32)
    local.get $has_selected
    if (result f32)
      local.get $width call $er_ui_gallery_selected_component_height f32.const 32 f32.add
    else
      f32.const 0
    end
    local.get $columns local.get $gap call $er_ui_gallery_catalog_section_height f32.add)

  (func (export "er_ui_gallery_docs_content_height") (param $width f32) (param $has_selected i32) (result f32)
    local.get $width local.get $width f32.const 40 call $er_ui_gallery_column_count f32.const 40 local.get $has_selected call $er_ui_gallery_body_height)

  (func (export "er_ui_gallery_content_height") (param $width f32) (param $scroll_y f32) (param $grid_gap f32) (param $has_selected i32) (result f32)
    (local $board_w f32) (local $gap f32) (local $columns i32)
    f32.const 1180 local.get $width f32.const 40 f32.sub f32.const 1 call $max_f32 call $min_f32 local.set $board_w
    local.get $grid_gap call $er_ui_gallery_normalized_grid_gap local.set $gap
    local.get $board_w local.get $gap call $er_ui_gallery_column_count local.set $columns
    f32.const 56 f32.const 48 f32.add local.get $board_w local.get $columns local.get $gap local.get $has_selected call $er_ui_gallery_body_height f32.add f32.const 120 f32.add)

  (func (export "er_ui_gallery_catalog_card_id") (param $index i32) (result i32)
    i32.const 20000 local.get $index i32.add)

  (func (export "er_ui_gallery_preview_hit_for_index") (param $index i32) (result i32)
    i32.const 23000 local.get $index i32.const 32 i32.mul i32.add)

  (func (export "er_ui_gallery_selected_preview_hit_for_index") (param $index i32) (result i32)
    i32.const 25000 local.get $index i32.const 32 i32.mul i32.add)

  (func (export "er_ui_gallery_index_by_catalog_hit") (param $hit_id i32) (result i32)
    local.get $hit_id i32.const 20000 i32.lt_u
    if i32.const -1 return end
    local.get $hit_id i32.const 20000 i32.sub local.tee $hit_id i32.const 60 i32.ge_u
    if i32.const -1 return end
    local.get $hit_id)

  (func (export "er_ui_gallery_index_by_preview_hit") (param $hit_id i32) (result i32)
    (local $relative i32) (local $index i32)
    local.get $hit_id i32.const 23000 i32.ge_u
    if
      local.get $hit_id i32.const 23000 i32.sub local.tee $relative i32.const 32 i32.rem_u drop
      local.get $relative i32.const 32 i32.div_u local.tee $index i32.const 60 i32.lt_u
      if local.get $index return end
    end
    local.get $hit_id i32.const 25000 i32.ge_u
    if
      local.get $hit_id i32.const 25000 i32.sub local.tee $relative i32.const 32 i32.div_u local.tee $index i32.const 60 i32.lt_u
      if local.get $index return end
    end
    i32.const -1)

  (func (export "er_ui_gallery_layout_board_bounds") (param $bounds i32) (param $scroll_y f32) (param $out i32) (result i32)
    (local $content_w f32) (local $content_x f32) (local $scroll f32) (local $board_y f32) (local $board_h f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    f32.const 1180 local.get $bounds i32.const 8 i32.add f32.load f32.const 40 f32.sub f32.const 1 call $max_f32 call $min_f32 local.set $content_w
    local.get $bounds f32.load local.get $bounds i32.const 8 i32.add f32.load local.get $content_w f32.sub f32.const 0.5 f32.mul f32.add local.set $content_x
    local.get $scroll_y f32.const 0 f32.const 4096 call $clamp_f32 local.set $scroll
    local.get $bounds i32.const 4 i32.add f32.load f32.const 104 f32.add local.get $scroll f32.sub local.set $board_y
    local.get $bounds i32.const 12 i32.add f32.load f32.const 104 f32.sub local.get $scroll f32.add f32.const 240 call $max_f32 local.set $board_h
    local.get $out local.get $content_x local.get $board_y local.get $content_w local.get $board_h call $rect_store)

  (func (export "er_ui_gallery_source_badge_bounds") (param $inset i32) (param $selected i32) (param $out i32) (result i32)
    (local $desired f32)
    local.get $inset i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $selected if (result f32) f32.const 94 else f32.const 86.5 end local.set $desired
    local.get $out local.get $inset f32.load local.get $inset i32.const 4 i32.add f32.load local.get $inset i32.const 8 i32.add f32.load f32.const 82 local.get $desired call $max_f32 call $min_f32 f32.const 24 call $rect_store)

  (func (export "er_ui_gallery_contract_badge_bounds") (param $bounds i32) (param $label_len i32) (param $out i32) (result i32)
    (local $desired f32) (local $width f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $label_len f32.convert_i32_u f32.const 7.5 f32.mul f32.const 34 f32.add local.set $desired
    local.get $bounds i32.const 8 i32.add f32.load f32.const 36 f32.sub f32.const 1 call $max_f32 f32.const 82 local.get $desired call $max_f32 call $min_f32 local.set $width
    local.get $out local.get $bounds f32.load f32.const 18 f32.add local.get $bounds i32.const 4 i32.add f32.load local.get $bounds i32.const 12 i32.add f32.load f32.add f32.const 32 f32.sub local.get $width f32.const 24 call $rect_store)
