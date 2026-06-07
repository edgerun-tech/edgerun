  (func $er_ui_gallery_preview_base_id  (result i32) i32.const 18000)
  (func $er_ui_gallery_first_catalog_card_id  (result i32) i32.const 20000)
  (func $er_ui_gallery_catalog_preview_id_base  (result i32) i32.const 23000)
  (func $er_ui_gallery_selected_preview_id_base  (result i32) i32.const 25000)
  (func $er_ui_gallery_preview_id_stride  (result i32) i32.const 32)
  (func $er_ui_gallery_component_count  (result i32) i32.const 60)
  (func $er_ui_gallery_header_h  (result f32) f32.const 56)
  (func $er_ui_gallery_page_top_pad  (result f32) f32.const 48)
  (func $er_ui_gallery_page_bottom_pad  (result f32) f32.const 120)
  (func $er_ui_gallery_card_content_x  (result f32) f32.const 18)
  (func $er_ui_gallery_min_column_width  (result f32) f32.const 300)
  (func $er_ui_gallery_max_columns  (result i32) i32.const 5)
  (func $er_ui_gallery_catalog_intro_h  (result f32) f32.const 86)
  (func $er_ui_gallery_catalog_card_h  (result f32) f32.const 148)
  (func $er_ui_gallery_catalog_source_min_w  (result f32) f32.const 82)
  (func $er_ui_gallery_selected_component_h  (result f32) f32.const 500)
  (func $er_ui_gallery_selected_component_compact_h  (result f32) f32.const 820)
  (func $er_ui_gallery_selected_component_gap  (result f32) f32.const 32)
  (func $er_ui_gallery_grid_gap_compact  (result f32) f32.const 28)
  (func $er_ui_gallery_grid_gap_default  (result f32) f32.const 40)
  (func $er_ui_gallery_grid_gap_wide  (result f32) f32.const 56)

  (func $er_ui_gallery_category_foundation  (result i32) i32.const 0)
  (func $er_ui_gallery_category_form  (result i32) i32.const 1)
  (func $er_ui_gallery_category_overlay  (result i32) i32.const 2)
  (func $er_ui_gallery_category_navigation  (result i32) i32.const 3)
  (func $er_ui_gallery_category_data_display  (result i32) i32.const 4)
  (func $er_ui_gallery_category_feedback  (result i32) i32.const 5)
  (func $er_ui_gallery_category_layout  (result i32) i32.const 6)
  (func $er_ui_gallery_category_media  (result i32) i32.const 7)

  (func $er_ui_gallery_count_by_category  (param $category i32) (result i32)
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

  (func $er_ui_gallery_normalized_grid_gap  (param $value f32) (result f32)
    local.get $value f32.const 34 f32.le
    if f32.const 28 return end
    local.get $value f32.const 48 f32.ge
    if f32.const 56 return end
    f32.const 40)

  (func $er_ui_gallery_column_count  (param $width f32) (param $gap f32) (result i32)
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

  (func $er_ui_gallery_catalog_section_height  (param $columns i32) (param $gap f32) (result f32)
    (local $cols i32) (local $rows i32)
    local.get $columns i32.const 1 call $layout_max_i32_u local.set $cols
    i32.const 60 local.get $cols i32.add i32.const 1 i32.sub local.get $cols i32.div_u local.set $rows
    f32.const 86 local.get $rows f32.convert_i32_u f32.const 148 f32.mul f32.add local.get $rows i32.const 1 i32.sub f32.convert_i32_u local.get $gap f32.mul f32.add)

  (func $er_ui_gallery_selected_component_height  (param $width f32) (result f32)
    local.get $width f32.const 760 f32.lt
    if f32.const 820 return end
    f32.const 500)

  (func $er_ui_gallery_body_height  (param $width f32) (param $columns i32) (param $gap f32) (param $has_selected i32) (result f32)
    local.get $has_selected
    if (result f32)
      local.get $width call $er_ui_gallery_selected_component_height f32.const 32 f32.add
    else
      f32.const 0
    end
    local.get $columns local.get $gap call $er_ui_gallery_catalog_section_height f32.add)

  (func $er_ui_gallery_docs_content_height  (param $width f32) (param $has_selected i32) (result f32)
    local.get $width local.get $width f32.const 40 call $er_ui_gallery_column_count f32.const 40 local.get $has_selected call $er_ui_gallery_body_height)

  (func $er_ui_gallery_content_height  (param $width f32) (param $scroll_y f32) (param $grid_gap f32) (param $has_selected i32) (result f32)
    (local $board_w f32) (local $gap f32) (local $columns i32)
    f32.const 1180 local.get $width f32.const 40 f32.sub f32.const 1 call $max_f32 call $min_f32 local.set $board_w
    local.get $grid_gap call $er_ui_gallery_normalized_grid_gap local.set $gap
    local.get $board_w local.get $gap call $er_ui_gallery_column_count local.set $columns
    f32.const 56 f32.const 48 f32.add local.get $board_w local.get $columns local.get $gap local.get $has_selected call $er_ui_gallery_body_height f32.add f32.const 120 f32.add)

  (func $er_ui_gallery_catalog_card_id  (param $index i32) (result i32)
    i32.const 20000 local.get $index i32.add)

  (func $er_ui_gallery_preview_hit_for_index  (param $index i32) (result i32)
    i32.const 23000 local.get $index i32.const 32 i32.mul i32.add)

  (func $er_ui_gallery_selected_preview_hit_for_index  (param $index i32) (result i32)
    i32.const 25000 local.get $index i32.const 32 i32.mul i32.add)

  (func $er_ui_gallery_index_by_catalog_hit  (param $hit_id i32) (result i32)
    local.get $hit_id i32.const 20000 i32.lt_u
    if i32.const -1 return end
    local.get $hit_id i32.const 20000 i32.sub local.tee $hit_id i32.const 60 i32.ge_u
    if i32.const -1 return end
    local.get $hit_id)

  (func $er_ui_gallery_index_by_preview_hit  (param $hit_id i32) (result i32)
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

  (func $er_ui_gallery_layout_board_bounds  (param $bounds i32) (param $scroll_y f32) (param $out i32) (result i32)
    (local $content_w f32) (local $content_x f32) (local $scroll f32) (local $board_y f32) (local $board_h f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    f32.const 1180 local.get $bounds i32.const 8 i32.add f32.load f32.const 40 f32.sub f32.const 1 call $max_f32 call $min_f32 local.set $content_w
    local.get $bounds f32.load local.get $bounds i32.const 8 i32.add f32.load local.get $content_w f32.sub f32.const 0.5 f32.mul f32.add local.set $content_x
    local.get $scroll_y f32.const 0 f32.const 4096 call $clamp_f32 local.set $scroll
    local.get $bounds i32.const 4 i32.add f32.load f32.const 104 f32.add local.get $scroll f32.sub local.set $board_y
    local.get $bounds i32.const 12 i32.add f32.load f32.const 104 f32.sub local.get $scroll f32.add f32.const 240 call $max_f32 local.set $board_h
    local.get $out local.get $content_x local.get $board_y local.get $content_w local.get $board_h call $rect_store)

  (func $er_ui_gallery_source_badge_bounds  (param $inset i32) (param $selected i32) (param $out i32) (result i32)
    (local $desired f32)
    local.get $inset i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $selected if (result f32) f32.const 94 else f32.const 86.5 end local.set $desired
    local.get $out local.get $inset f32.load local.get $inset i32.const 4 i32.add f32.load local.get $inset i32.const 8 i32.add f32.load f32.const 82 local.get $desired call $max_f32 call $min_f32 f32.const 24 call $rect_store)

  (func $er_ui_gallery_contract_badge_bounds  (param $bounds i32) (param $label_len i32) (param $out i32) (result i32)
    (local $desired f32) (local $width f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $label_len f32.convert_i32_u f32.const 7.5 f32.mul f32.const 34 f32.add local.set $desired
    local.get $bounds i32.const 8 i32.add f32.load f32.const 36 f32.sub f32.const 1 call $max_f32 f32.const 82 local.get $desired call $max_f32 call $min_f32 local.set $width
    local.get $out local.get $bounds f32.load f32.const 18 f32.add local.get $bounds i32.const 4 i32.add f32.load local.get $bounds i32.const 12 i32.add f32.load f32.add f32.const 32 f32.sub local.get $width f32.const 24 call $rect_store)

  (func $er_ui_gallery_table_u8 (param $base i32) (param $index i32) (result i32)
    local.get $index i32.const 60 i32.ge_u
    if i32.const -1 return end
    local.get $base local.get $index i32.add i32.load8_u)

  (func $er_ui_gallery_copy_table_string (param $data_base i32) (param $off_base i32) (param $len_base i32) (param $index i32) (param $out i32) (param $cap i32) (result i32)
    (local $len i32) (local $off i32)
    local.get $index i32.const 60 i32.ge_u local.get $out i32.eqz i32.or
    if i32.const -1 return end
    local.get $len_base local.get $index i32.add i32.load8_u local.tee $len local.get $cap i32.gt_u
    if i32.const -1 return end
    local.get $off_base local.get $index i32.const 2 i32.mul i32.add i32.load16_u local.set $off
    local.get $out local.get $data_base local.get $off i32.add local.get $len memory.copy
    local.get $len)

  (func $er_ui_gallery_table_string_eq (param $data_base i32) (param $off_base i32) (param $len_base i32) (param $index i32) (param $ptr i32) (param $len i32) (result i32)
    (local $table_len i32) (local $table_ptr i32) (local $i i32)
    local.get $index i32.const 60 i32.ge_u local.get $ptr i32.eqz i32.or
    if i32.const 0 return end
    local.get $len_base local.get $index i32.add i32.load8_u local.tee $table_len local.get $len i32.ne
    if i32.const 0 return end
    local.get $data_base local.get $off_base local.get $index i32.const 2 i32.mul i32.add i32.load16_u i32.add local.set $table_ptr
    i32.const 0 local.set $i
    block $done
      loop $loop
        local.get $i local.get $len i32.ge_u br_if $done
        local.get $table_ptr local.get $i i32.add i32.load8_u local.get $ptr local.get $i i32.add i32.load8_u i32.ne
        if i32.const 0 return end
        local.get $i i32.const 1 i32.add local.set $i
        br $loop
      end
    end
    i32.const 1)

  (func $er_ui_gallery_index_by_table_string (param $data_base i32) (param $off_base i32) (param $len_base i32) (param $ptr i32) (param $len i32) (result i32)
    (local $index i32)
    local.get $ptr i32.eqz
    if i32.const -1 return end
    i32.const 0 local.set $index
    block $done
      loop $loop
        local.get $index i32.const 60 i32.ge_u br_if $done
        local.get $data_base local.get $off_base local.get $len_base local.get $index local.get $ptr local.get $len call $er_ui_gallery_table_string_eq
        if local.get $index return end
        local.get $index i32.const 1 i32.add local.set $index
        br $loop
      end
    end
    i32.const -1)

  (func $er_ui_gallery_category_count  (result i32) i32.const 8)
  (func $er_ui_gallery_preview_kind_count  (result i32) i32.const 59)
  (func $er_ui_gallery_component_path_count  (result i32) i32.const 58)

  (func $er_ui_gallery_category_label_len  (param $category i32) (result i32)
    local.get $category i32.const 0 i32.eq
    if i32.const 10 return end
    local.get $category i32.const 1 i32.eq
    if i32.const 4 return end
    local.get $category i32.const 2 i32.eq
    if i32.const 7 return end
    local.get $category i32.const 3 i32.eq
    if i32.const 10 return end
    local.get $category i32.const 4 i32.eq
    if i32.const 12 return end
    local.get $category i32.const 5 i32.eq
    if i32.const 8 return end
    local.get $category i32.const 6 i32.eq
    if i32.const 6 return end
    local.get $category i32.const 7 i32.eq
    if i32.const 5 return end
    i32.const -1)

  (func $er_ui_gallery_category_for_index  (param $index i32) (result i32)
    i32.const 66000 local.get $index call $er_ui_gallery_table_u8)

  (func $er_ui_gallery_category_at  (param $index i32) (result i32)
    i32.const 66000 local.get $index call $er_ui_gallery_table_u8)

  (func $er_ui_gallery_preview_kind_for_index  (param $index i32) (result i32)
    i32.const 66100 local.get $index call $er_ui_gallery_table_u8)

  (func $er_ui_gallery_preview_kind_at  (param $index i32) (result i32)
    i32.const 66100 local.get $index call $er_ui_gallery_table_u8)

  (func $er_ui_gallery_name_len_for_index  (param $index i32) (result i32)
    i32.const 66200 local.get $index call $er_ui_gallery_table_u8)

  (func $er_ui_gallery_name_len_at  (param $index i32) (result i32)
    i32.const 66200 local.get $index call $er_ui_gallery_table_u8)

  (func $er_ui_gallery_slug_len_for_index  (param $index i32) (result i32)
    i32.const 66300 local.get $index call $er_ui_gallery_table_u8)

  (func $er_ui_gallery_slug_len_at  (param $index i32) (result i32)
    i32.const 66300 local.get $index call $er_ui_gallery_table_u8)

  (func $er_ui_gallery_source_component_len_for_index  (param $index i32) (result i32)
    i32.const 66400 local.get $index call $er_ui_gallery_table_u8)

  (func $er_ui_gallery_source_component_len_at  (param $index i32) (result i32)
    i32.const 66400 local.get $index call $er_ui_gallery_table_u8)

  (func $er_ui_gallery_edge_builder_len_for_index  (param $index i32) (result i32)
    i32.const 66500 local.get $index call $er_ui_gallery_table_u8)

  (func $er_ui_gallery_edge_builder_len_at  (param $index i32) (result i32)
    i32.const 66500 local.get $index call $er_ui_gallery_table_u8)

  (func $er_ui_gallery_component_path_len_for_index  (param $index i32) (result i32)
    i32.const 66600 local.get $index call $er_ui_gallery_table_u8)

  (func $er_ui_gallery_component_path_len_at  (param $index i32) (result i32)
    i32.const 66600 local.get $index call $er_ui_gallery_table_u8)

  (func $er_ui_gallery_source_path_len_for_index  (param $index i32) (result i32)
    (local $source_len i32)
    i32.const 66400 local.get $index call $er_ui_gallery_table_u8 local.tee $source_len i32.const 0 i32.lt_s
    if i32.const -1 return end
    i32.const 22 local.get $source_len i32.add)

  (func $er_ui_gallery_source_path_len_at  (param $index i32) (result i32)
    (local $source_len i32)
    i32.const 66400 local.get $index call $er_ui_gallery_table_u8 local.tee $source_len i32.const 0 i32.lt_s
    if i32.const -1 return end
    i32.const 22 local.get $source_len i32.add)

  (func $er_ui_gallery_name_copy  (param $index i32) (param $out i32) (param $cap i32) (result i32)
    i32.const 66700 i32.const 67200 i32.const 66200 local.get $index local.get $out local.get $cap call $er_ui_gallery_copy_table_string)

  (func $er_ui_gallery_slug_copy  (param $index i32) (param $out i32) (param $cap i32) (result i32)
    i32.const 67400 i32.const 67900 i32.const 66300 local.get $index local.get $out local.get $cap call $er_ui_gallery_copy_table_string)

  (func $er_ui_gallery_source_component_copy  (param $index i32) (param $out i32) (param $cap i32) (result i32)
    i32.const 68100 i32.const 68600 i32.const 66400 local.get $index local.get $out local.get $cap call $er_ui_gallery_copy_table_string)

  (func $er_ui_gallery_edge_builder_copy  (param $index i32) (param $out i32) (param $cap i32) (result i32)
    i32.const 68800 i32.const 69600 i32.const 66500 local.get $index local.get $out local.get $cap call $er_ui_gallery_copy_table_string)

  (func $er_ui_gallery_component_path_copy  (param $index i32) (param $out i32) (param $cap i32) (result i32)
    i32.const 69800 i32.const 71300 i32.const 66600 local.get $index local.get $out local.get $cap call $er_ui_gallery_copy_table_string)

  (func $er_ui_gallery_source_path_copy  (param $index i32) (param $out i32) (param $cap i32) (result i32)
    (local $source_len i32) (local $total i32) (local $source_off i32)
    local.get $index i32.const 60 i32.ge_u local.get $out i32.eqz i32.or
    if i32.const -1 return end
    i32.const 66400 local.get $index i32.add i32.load8_u local.set $source_len
    i32.const 22 local.get $source_len i32.add local.tee $total local.get $cap i32.gt_u
    if i32.const -1 return end
    i32.const 68600 local.get $index i32.const 2 i32.mul i32.add i32.load16_u local.set $source_off
    local.get $out i32.const 71500 i32.const 18 memory.copy
    local.get $out i32.const 18 i32.add i32.const 68100 local.get $source_off i32.add local.get $source_len memory.copy
    local.get $out i32.const 18 i32.add local.get $source_len i32.add i32.const 71532 i32.const 4 memory.copy
    local.get $total)

  (func $er_ui_gallery_index_by_slug_bytes  (param $ptr i32) (param $len i32) (result i32)
    i32.const 67400 i32.const 67900 i32.const 66300 local.get $ptr local.get $len call $er_ui_gallery_index_by_table_string)

  (func $er_ui_gallery_index_by_slug  (param $ptr i32) (param $len i32) (result i32)
    i32.const 67400 i32.const 67900 i32.const 66300 local.get $ptr local.get $len call $er_ui_gallery_index_by_table_string)

  (func $er_ui_gallery_index_by_source_component_bytes  (param $ptr i32) (param $len i32) (result i32)
    i32.const 68100 i32.const 68600 i32.const 66400 local.get $ptr local.get $len call $er_ui_gallery_index_by_table_string)

  (func $er_ui_gallery_index_by_source_component  (param $ptr i32) (param $len i32) (result i32)
    i32.const 68100 i32.const 68600 i32.const 66400 local.get $ptr local.get $len call $er_ui_gallery_index_by_table_string)
  (func $er_ui_gallery_selected_preview_surface_h  (result f32) f32.const 266)
  (func $er_ui_gallery_selected_preview_compact_surface_h  (result f32) f32.const 320)
  (func $er_ui_gallery_catalog_preview_h  (result f32) f32.const 38)
  (func $er_ui_gallery_catalog_card_pad  (result f32) f32.const 14)

  (func $er_ui_gallery_split_left_rects (param $bounds i32) (param $width f32) (param $gap f32) (param $out i32) (result i32)
    (local $first_w f32) (local $rest_x f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $width f32.const 1 call $max_f32 local.get $bounds i32.const 8 i32.add f32.load call $min_f32 local.set $first_w
    local.get $bounds f32.load local.get $first_w f32.add local.get $gap f32.add local.set $rest_x
    local.get $out local.get $bounds f32.load local.get $bounds i32.const 4 i32.add f32.load local.get $first_w local.get $bounds i32.const 12 i32.add f32.load call $rect_store drop
    local.get $out i32.const 16 i32.add local.get $rest_x local.get $bounds i32.const 4 i32.add f32.load local.get $bounds f32.load local.get $bounds i32.const 8 i32.add f32.load f32.add local.get $rest_x f32.sub f32.const 1 call $max_f32 local.get $bounds i32.const 12 i32.add f32.load call $rect_store)

  (func $er_ui_gallery_selected_is_compact  (param $bounds i32) (result i32)
    local.get $bounds i32.eqz
    if i32.const 0 return end
    local.get $bounds i32.const 8 i32.add f32.load f32.const 36 f32.sub f32.const 720 f32.lt)

  (func $er_ui_gallery_selected_inset_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds local.get $out f32.const 18 call $er_ui_rect_inset_uniform)

  (func $er_ui_gallery_selected_part_bounds  (param $bounds i32) (param $part i32) (param $out i32) (result i32)
    (local $in_x f32) (local $in_y f32) (local $in_w f32) (local $in_h f32)
    (local $first_w f32) (local $gap f32) (local $second_x f32) (local $second_w f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds f32.load f32.const 18 f32.add local.set $in_x
    local.get $bounds i32.const 4 i32.add f32.load f32.const 18 f32.add local.set $in_y
    local.get $bounds i32.const 8 i32.add f32.load f32.const 36 f32.sub local.set $in_w
    local.get $bounds i32.const 12 i32.add f32.load f32.const 36 f32.sub local.set $in_h
    local.get $in_w f32.const 720 f32.lt
    if
      local.get $part i32.const 1 i32.eq
      if local.get $out local.get $in_x local.get $in_y f32.const 38 f32.add local.get $in_w f32.const 26 call $rect_store return end
      local.get $part i32.const 2 i32.eq
      if local.get $out local.get $in_x local.get $in_y f32.const 76 f32.add local.get $in_w f32.const 54 call $rect_store return end
      local.get $part i32.const 3 i32.eq
      if local.get $out local.get $in_x local.get $in_y f32.const 148 f32.add local.get $in_w f32.const 320 call $rect_store return end
      local.get $part i32.const 4 i32.eq
      if local.get $out local.get $in_x local.get $in_y f32.const 490 f32.add local.get $in_w f32.const 210 call $rect_store return end
      local.get $part i32.const 5 i32.eq
      if local.get $out local.get $in_x local.get $in_y f32.const 718 f32.add local.get $in_w f32.const 64 call $rect_store return end
      i32.const 0 return
    end
    local.get $in_w f32.const 0.42 f32.mul f32.const 1 call $max_f32 local.get $in_w call $min_f32 local.set $first_w
    local.get $in_w f32.const 0.06 f32.mul local.set $gap
    local.get $in_x local.get $first_w f32.add local.get $gap f32.add local.set $second_x
    local.get $in_x local.get $in_w f32.add local.get $second_x f32.sub f32.const 1 call $max_f32 local.set $second_w
    local.get $part i32.const 1 i32.eq
    if local.get $out local.get $in_x local.get $in_y f32.const 38 f32.add local.get $first_w f32.const 26 call $rect_store return end
    local.get $part i32.const 2 i32.eq
    if local.get $out local.get $in_x local.get $in_y f32.const 76 f32.add local.get $first_w f32.const 54 call $rect_store return end
    local.get $part i32.const 3 i32.eq
    if local.get $out local.get $second_x local.get $in_y f32.const 10 f32.add local.get $second_w f32.const 266 call $rect_store return end
    local.get $part i32.const 4 i32.eq
    if local.get $out local.get $in_x local.get $in_y f32.const 154 f32.add local.get $first_w local.get $in_h f32.const 154 f32.sub call $rect_store return end
    local.get $part i32.const 5 i32.eq
    if local.get $out local.get $second_x local.get $in_y f32.const 298 f32.add local.get $second_w local.get $in_h f32.const 298 f32.sub call $rect_store return end
    i32.const 0)

  (func $er_ui_gallery_opened_preview_inner_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $bounds f32.load f32.const 18 f32.add local.get $bounds i32.const 4 i32.add f32.load f32.const 72 f32.add local.get $bounds i32.const 8 i32.add f32.load f32.const 36 f32.sub local.get $bounds i32.const 12 i32.add f32.load f32.const 92 f32.sub call $rect_store)

  (func $er_ui_gallery_opened_preview_slot_bounds  (param $bounds i32) (param $slot i32) (param $out i32) (result i32)
    (local $x f32) (local $y f32) (local $w f32) (local $h f32)
    (local $main_h f32) (local $second_h f32) (local $first_w f32) (local $rest_x f32) (local $rest_w f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or local.get $slot i32.const 2 i32.gt_u i32.or
    if i32.const 0 return end
    local.get $bounds f32.load f32.const 18 f32.add local.set $x
    local.get $bounds i32.const 4 i32.add f32.load f32.const 72 f32.add local.set $y
    local.get $bounds i32.const 8 i32.add f32.load f32.const 36 f32.sub local.set $w
    local.get $bounds i32.const 12 i32.add f32.load f32.const 92 f32.sub local.set $h
    local.get $w f32.const 360 f32.lt
    if
      f32.const 112 local.get $h f32.const 0.45 f32.mul call $min_f32 local.set $main_h
      f32.const 88 local.get $h f32.const 0.32 f32.mul call $min_f32 local.set $second_h
      local.get $slot i32.eqz
      if local.get $out local.get $x local.get $y local.get $w local.get $main_h call $rect_store return end
      local.get $slot i32.const 1 i32.eq
      if local.get $out local.get $x local.get $y local.get $main_h f32.add f32.const 14 f32.add local.get $w local.get $second_h call $rect_store return end
      local.get $out local.get $x local.get $y local.get $main_h f32.add f32.const 14 f32.add local.get $second_h f32.add f32.const 14 f32.add local.get $w local.get $second_h call $rect_store return
    end
    local.get $w f32.const 0.58 f32.mul f32.const 1 call $max_f32 local.get $w call $min_f32 local.set $first_w
    local.get $x local.get $first_w f32.add f32.const 14 f32.add local.set $rest_x
    local.get $x local.get $w f32.add local.get $rest_x f32.sub f32.const 1 call $max_f32 local.set $rest_w
    local.get $slot i32.eqz
    if local.get $out local.get $x local.get $y local.get $first_w local.get $h call $rect_store return end
    local.get $h f32.const 14 f32.sub f32.const 0.5 f32.mul local.set $second_h
    local.get $slot i32.const 1 i32.eq
    if local.get $out local.get $rest_x local.get $y local.get $rest_w local.get $second_h call $rect_store return end
    local.get $out local.get $rest_x local.get $y local.get $second_h f32.add f32.const 14 f32.add local.get $rest_w local.get $second_h call $rect_store)

  (func $er_ui_gallery_preview_slot_content_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds local.get $out f32.const 12 f32.const 32 f32.const 12 f32.const 12 call $er_ui_rect_inset_ltrb)

  (func $er_ui_gallery_api_label_h  (result f32) f32.const 14)
  (func $er_ui_gallery_api_value_line_h  (result f32) f32.const 16)
  (func $er_ui_gallery_api_value_avg_w  (result f32) f32.const 7.8)
  (func $er_ui_gallery_api_value_max_lines  (result i32) i32.const 2)

  (func $er_ui_gallery_api_len_for_field (param $index i32) (param $field i32) (result i32)
    local.get $index i32.const 60 i32.ge_u
    if i32.const -1 return end
    local.get $field i32.const 0 i32.eq
    if i32.const 66600 local.get $index i32.add i32.load8_u return end
    local.get $field i32.const 1 i32.eq
    if i32.const 66400 local.get $index i32.add i32.load8_u return end
    local.get $field i32.const 2 i32.eq
    if i32.const 66500 local.get $index i32.add i32.load8_u return end
    i32.const -1)

  (func $er_ui_gallery_api_wrapped_line_count_for_len  (param $value_len i32) (param $width f32) (result i32)
    (local $cap i32) (local $lines i32)
    local.get $value_len i32.eqz
    if i32.const 0 return end
    local.get $width f32.const 7.8 f32.div f32.const 1 call $max_f32 i32.trunc_f32_u local.set $cap
    local.get $value_len local.get $cap i32.add i32.const 1 i32.sub local.get $cap i32.div_u local.set $lines
    local.get $lines i32.const 2 i32.gt_u
    if i32.const 2 return end
    local.get $lines)

  (func $er_ui_gallery_api_value_height_for_len  (param $value_len i32) (param $width f32) (result f32)
    local.get $value_len local.get $width call $er_ui_gallery_api_wrapped_line_count_for_len f32.convert_i32_u f32.const 16 f32.mul f32.const 16 call $max_f32)

  (func $er_ui_gallery_api_field_height_for_len  (param $value_len i32) (param $width f32) (result f32)
    f32.const 20 local.get $value_len local.get $width call $er_ui_gallery_api_value_height_for_len f32.add)

  (func $er_ui_gallery_api_field_height_for_index  (param $index i32) (param $field i32) (param $width f32) (result f32)
    (local $value_len i32)
    local.get $index local.get $field call $er_ui_gallery_api_len_for_field local.tee $value_len i32.const 0 i32.lt_s
    if f32.const -1 return end
    local.get $value_len local.get $width call $er_ui_gallery_api_field_height_for_len)

  (func $er_ui_gallery_api_content_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $bounds f32.load f32.const 18 f32.add local.get $bounds i32.const 4 i32.add f32.load f32.const 50 f32.add local.get $bounds i32.const 8 i32.add f32.load f32.const 36 f32.sub f32.const 1 call $max_f32 local.get $bounds i32.const 12 i32.add f32.load f32.const 50 f32.sub f32.const 1 call $max_f32 call $rect_store)

  (func $er_ui_gallery_api_field_bounds  (param $bounds i32) (param $index i32) (param $field i32) (param $out i32) (result i32)
    (local $x f32) (local $y f32) (local $w f32) (local $h f32)
    (local $h0 f32) (local $h1 f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or local.get $field i32.const 2 i32.gt_u i32.or
    if i32.const 0 return end
    local.get $bounds f32.load f32.const 18 f32.add local.set $x
    local.get $bounds i32.const 4 i32.add f32.load f32.const 50 f32.add local.set $y
    local.get $bounds i32.const 8 i32.add f32.load f32.const 36 f32.sub f32.const 1 call $max_f32 local.set $w
    local.get $index local.get $field local.get $w call $er_ui_gallery_api_field_height_for_index local.tee $h f32.const 0 f32.lt
    if i32.const 0 return end
    local.get $field i32.const 1 i32.ge_u
    if
      local.get $index i32.const 0 local.get $w call $er_ui_gallery_api_field_height_for_index local.set $h0
      local.get $y local.get $h0 f32.add f32.const 12 f32.add local.set $y
    end
    local.get $field i32.const 2 i32.eq
    if
      local.get $index i32.const 1 local.get $w call $er_ui_gallery_api_field_height_for_index local.set $h1
      local.get $y local.get $h1 f32.add f32.const 12 f32.add local.set $y
    end
    local.get $out local.get $x local.get $y local.get $w local.get $h call $rect_store)
  (func $er_ui_gallery_catalog_intro_title_bounds  (param $intro i32) (param $out i32) (result i32)
    local.get $intro i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $intro f32.load local.get $intro i32.const 4 i32.add f32.load local.get $intro i32.const 8 i32.add f32.load f32.const 22 call $rect_store)

  (func $er_ui_gallery_catalog_intro_detail_bounds  (param $intro i32) (param $out i32) (result i32)
    local.get $intro i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $intro f32.load local.get $intro i32.const 4 i32.add f32.load f32.const 32 f32.add local.get $intro i32.const 8 i32.add f32.load f32.const 42 call $rect_store)

  (func $er_ui_gallery_catalog_selected_bounds  (param $section i32) (param $has_selected i32) (param $out i32) (result i32)
    (local $selected_h f32)
    local.get $section i32.eqz local.get $out i32.eqz i32.or local.get $has_selected i32.eqz i32.or
    if i32.const 0 return end
    local.get $section i32.const 8 i32.add f32.load call $er_ui_gallery_selected_component_height local.set $selected_h
    local.get $out local.get $section f32.load local.get $section i32.const 4 i32.add f32.load local.get $section i32.const 8 i32.add f32.load local.get $selected_h call $rect_store)

  (func $er_ui_gallery_catalog_intro_bounds  (param $section i32) (param $has_selected i32) (param $out i32) (result i32)
    (local $y f32)
    local.get $section i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $section i32.const 4 i32.add f32.load local.set $y
    local.get $has_selected
    if
      local.get $y local.get $section i32.const 8 i32.add f32.load call $er_ui_gallery_selected_component_height f32.add f32.const 32 f32.add local.set $y
    end
    local.get $out local.get $section f32.load local.get $y local.get $section i32.const 8 i32.add f32.load f32.const 86 call $rect_store)

  (func $er_ui_gallery_catalog_grid_bounds  (param $section i32) (param $has_selected i32) (param $out i32) (result i32)
    (local $y f32)
    local.get $section i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $section i32.const 4 i32.add f32.load f32.const 86 f32.add local.set $y
    local.get $has_selected
    if
      local.get $y local.get $section i32.const 8 i32.add f32.load call $er_ui_gallery_selected_component_height f32.add f32.const 32 f32.add local.set $y
    end
    local.get $out local.get $section f32.load local.get $y local.get $section i32.const 8 i32.add f32.load local.get $section i32.const 12 i32.add f32.load local.get $y local.get $section i32.const 4 i32.add f32.load f32.sub f32.sub f32.const 1 call $max_f32 call $rect_store)

  (func $er_ui_gallery_catalog_grid_item_bounds  (param $grid i32) (param $columns i32) (param $gap f32) (param $index i32) (param $out i32) (result i32)
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

  (func $er_ui_gallery_catalog_card_inset_bounds  (param $card i32) (param $out i32) (result i32)
    local.get $card local.get $out f32.const 14 call $er_ui_rect_inset_uniform)

  (func $er_ui_gallery_catalog_card_source_bounds  (param $card i32) (param $selected i32) (param $out i32) (result i32)
    (local $in_x f32) (local $in_y f32) (local $in_w f32) (local $source_w f32)
    local.get $card i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $card f32.load f32.const 14 f32.add local.set $in_x
    local.get $card i32.const 4 i32.add f32.load f32.const 14 f32.add local.set $in_y
    local.get $card i32.const 8 i32.add f32.load f32.const 28 f32.sub local.set $in_w
    local.get $selected if (result f32) f32.const 94 else f32.const 86.5 end f32.const 82 call $max_f32 local.get $in_w call $min_f32 local.set $source_w
    local.get $out local.get $in_x local.get $in_w f32.add local.get $source_w f32.sub local.get $in_y f32.const 1 f32.sub local.get $source_w f32.const 24 call $rect_store)

  (func $er_ui_gallery_catalog_card_builder_bounds  (param $card i32) (param $out i32) (result i32)
    local.get $card i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $card f32.load f32.const 14 f32.add local.get $card i32.const 4 i32.add f32.load f32.const 62 f32.add local.get $card i32.const 8 i32.add f32.load f32.const 28 f32.sub f32.const 16 call $rect_store)

  (func $er_ui_gallery_catalog_card_preview_bounds  (param $card i32) (param $out i32) (result i32)
    local.get $card i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out local.get $card f32.load f32.const 14 f32.add local.get $card i32.const 4 i32.add f32.load f32.const 90 f32.add local.get $card i32.const 8 i32.add f32.load f32.const 28 f32.sub f32.const 38 call $rect_store)

  (func $er_ui_gallery_surface_variant_panel  (result i32) i32.const 0)
  (func $er_ui_gallery_surface_variant_elevated  (result i32) i32.const 1)

  (func $er_ui_gallery_rect_contains_exclusive_local (param $rect i32) (param $x f32) (param $y f32) (result i32)
    local.get $rect i32.eqz if i32.const 0 return end
    local.get $x local.get $rect f32.load f32.ge
    local.get $y local.get $rect i32.const 4 i32.add f32.load f32.ge i32.and
    local.get $x local.get $rect f32.load local.get $rect i32.const 8 i32.add f32.load f32.add f32.lt i32.and
    local.get $y local.get $rect i32.const 4 i32.add f32.load local.get $rect i32.const 12 i32.add f32.load f32.add f32.lt i32.and)

  (func $er_ui_gallery_hover_enabled  (param $hover_x f32) (param $hover_y f32) (result i32)
    local.get $hover_x f32.const 0 f32.ge local.get $hover_y f32.const 0 f32.ge i32.and)

  (func $er_ui_gallery_card_hovered  (param $card i32) (param $hover_x f32) (param $hover_y f32) (result i32)
    local.get $hover_x local.get $hover_y call $er_ui_gallery_hover_enabled i32.eqz
    if i32.const 0 return end
    local.get $card local.get $hover_x local.get $hover_y call $er_ui_gallery_rect_contains_exclusive_local)

  (func $er_ui_gallery_catalog_card_surface_variant  (param $card i32) (param $selected i32) (param $hover_x f32) (param $hover_y f32) (result i32)
    local.get $selected
    if i32.const 1 return end
    local.get $card local.get $hover_x local.get $hover_y call $er_ui_gallery_card_hovered
    if i32.const 1 return end
    i32.const 0)

  (func $er_ui_gallery_preview_strategy_primitive  (result i32) i32.const 0)
  (func $er_ui_gallery_preview_strategy_badge_variants  (result i32) i32.const 1)
  (func $er_ui_gallery_preview_strategy_button_variants  (result i32) i32.const 2)

  (func $er_ui_gallery_preview_kind_badge  (result i32) i32.const 5)
  (func $er_ui_gallery_preview_kind_button  (result i32) i32.const 7)
  (func $er_ui_gallery_preview_kind_data_table  (result i32) i32.const 17)
  (func $er_ui_gallery_preview_kind_table  (result i32) i32.const 52)
  (func $er_ui_gallery_preview_kind_sonner  (result i32) i32.const 50)
  (func $er_ui_gallery_preview_kind_native_select  (result i32) i32.const 35)

  (func $er_ui_gallery_preview_table_u8 (param $base i32) (param $kind i32) (result i32)
    local.get $kind i32.const 59 i32.ge_u
    if i32.const -1 return end
    local.get $base local.get $kind i32.add i32.load8_u)

  (func $er_ui_gallery_preview_strategy_for_kind  (param $kind i32) (result i32)
    i32.const 71700 local.get $kind call $er_ui_gallery_preview_table_u8)

  (func $er_ui_gallery_preview_strategy_for_catalog_index  (param $index i32) (result i32)
    local.get $index i32.const 60 i32.ge_u
    if i32.const -1 return end
    i32.const 71700 i32.const 66100 local.get $index i32.add i32.load8_u call $er_ui_gallery_preview_table_u8)

  (func $er_ui_gallery_preview_component_kind_for_kind  (param $kind i32) (result i32)
    i32.const 71600 local.get $kind call $er_ui_gallery_preview_table_u8)

  (func $er_ui_gallery_preview_component_kind_for_catalog_index  (param $index i32) (result i32)
    local.get $index i32.const 60 i32.ge_u
    if i32.const -1 return end
    i32.const 71600 i32.const 66100 local.get $index i32.add i32.load8_u call $er_ui_gallery_preview_table_u8)

  (func $er_ui_gallery_preview_uses_control_id_for_kind  (param $kind i32) (result i32)
    i32.const 71800 local.get $kind call $er_ui_gallery_preview_table_u8)

  (func $er_ui_gallery_preview_uses_control_id_for_catalog_index  (param $index i32) (result i32)
    local.get $index i32.const 60 i32.ge_u
    if i32.const -1 return end
    i32.const 71800 i32.const 66100 local.get $index i32.add i32.load8_u call $er_ui_gallery_preview_table_u8)

  (func $er_ui_gallery_preview_variant_count_for_kind  (param $kind i32) (result i32)
    (local $strategy i32)
    local.get $kind call $er_ui_gallery_preview_strategy_for_kind local.tee $strategy i32.const 0 i32.lt_s
    if i32.const -1 return end
    local.get $strategy i32.eqz
    if i32.const 1 return end
    i32.const 3)

  (func $er_ui_gallery_preview_variant_count_for_catalog_index  (param $index i32) (result i32)
    (local $kind i32)
    local.get $index i32.const 60 i32.ge_u
    if i32.const -1 return end
    i32.const 66100 local.get $index i32.add i32.load8_u local.tee $kind call $er_ui_gallery_preview_variant_count_for_kind)

  (func $er_ui_gallery_preview_variant_gap  (result f32) f32.const 6)

  (func $er_ui_gallery_preview_variant_slot_width  (param $kind i32) (param $variant i32) (result f32)
    local.get $kind i32.const 5 i32.eq
    if
      local.get $variant i32.eqz
      if f32.const 64 return end
      local.get $variant i32.const 1 i32.eq
      if f32.const 72 return end
      local.get $variant i32.const 2 i32.eq
      if f32.const 44 return end
      f32.const -1 return
    end
    local.get $kind i32.const 7 i32.eq
    if
      local.get $variant i32.eqz
      if f32.const 70 return end
      local.get $variant i32.const 1 i32.eq
      if f32.const 76 return end
      local.get $variant i32.const 2 i32.eq
      if f32.const 54 return end
      f32.const -1 return
    end
    f32.const -1)

  (func $er_ui_gallery_preview_variant_id  (param $kind i32) (param $base_id i32) (param $variant i32) (result i32)
    local.get $kind i32.const 7 i32.eq
    if
      local.get $variant i32.const 3 i32.ge_u
      if i32.const -1 return end
      local.get $base_id local.get $variant i32.add return
    end
    local.get $variant i32.eqz local.get $kind call $er_ui_gallery_preview_uses_control_id_for_kind i32.const 1 i32.eq i32.and
    if local.get $base_id return end
    i32.const -1)

  (func $er_ui_gallery_preview_variant_tag  (param $kind i32) (param $variant i32) (result i32)
    local.get $kind i32.const 5 i32.eq
    if
      local.get $variant i32.eqz
      if i32.const 0 return end
      local.get $variant i32.const 1 i32.eq
      if i32.const 3 return end
      local.get $variant i32.const 2 i32.eq
      if i32.const 5 return end
      i32.const -1 return
    end
    local.get $kind i32.const 7 i32.eq
    if
      local.get $variant i32.eqz
      if i32.const 0 return end
      local.get $variant i32.const 1 i32.eq
      if i32.const 1 return end
      local.get $variant i32.const 2 i32.eq
      if i32.const 5 return end
      i32.const -1 return
    end
    i32.const -1)

  (func $er_ui_gallery_preview_strategy  (param $kind i32) (result i32)
    local.get $kind call $er_ui_gallery_preview_strategy_for_kind)

  (func $er_ui_gallery_preview_component_kind  (param $kind i32) (result i32)
    local.get $kind call $er_ui_gallery_preview_component_kind_for_kind)

  (func $er_ui_gallery_badge_variant_count  (result i32) i32.const 3)
  (func $er_ui_gallery_button_variant_count  (result i32) i32.const 3)

  (func $er_ui_gallery_badge_variant_slot_w  (param $variant i32) (result f32)
    i32.const 5 local.get $variant call $er_ui_gallery_preview_variant_slot_width)

  (func $er_ui_gallery_button_variant_slot_w  (param $variant i32) (result f32)
    i32.const 7 local.get $variant call $er_ui_gallery_preview_variant_slot_width)

  (func $er_ui_gallery_badge_variant_tag  (param $variant i32) (result i32)
    i32.const 5 local.get $variant call $er_ui_gallery_preview_variant_tag)

  (func $er_ui_gallery_button_variant_tag  (param $variant i32) (result i32)
    i32.const 7 local.get $variant call $er_ui_gallery_preview_variant_tag)

  (func $er_ui_gallery_button_variant_id  (param $base_id i32) (param $variant i32) (result i32)
    i32.const 7 local.get $base_id local.get $variant call $er_ui_gallery_preview_variant_id)

  (func $er_ui_gallery_catalog_preview_slot_id  (param $index i32) (param $slot i32) (result i32)
    local.get $index i32.const 60 i32.ge_u local.get $slot i32.const 32 i32.ge_u i32.or
    if i32.const -1 return end
    i32.const 23000 local.get $index i32.const 32 i32.mul i32.add local.get $slot i32.add)

  (func $er_ui_gallery_selected_preview_slot_id  (param $index i32) (param $slot i32) (result i32)
    local.get $index i32.const 60 i32.ge_u local.get $slot i32.const 32 i32.ge_u i32.or
    if i32.const -1 return end
    i32.const 25000 local.get $index i32.const 32 i32.mul i32.add local.get $slot i32.add)

  (func $er_ui_gallery_preview_default_aspect_ratio_w  (result i32) i32.const 16)
  (func $er_ui_gallery_preview_default_aspect_ratio_h  (result i32) i32.const 9)
  (func $er_ui_gallery_preview_default_calendar_day  (result i32) i32.const 25)
  (func $er_ui_gallery_preview_default_pagination_page  (result i32) i32.const 1)
  (func $er_ui_gallery_preview_default_progress_value  (result f32) f32.const 0.62)
  (func $er_ui_gallery_preview_default_slider_value  (result f32) f32.const 0.68)
  (func $er_ui_gallery_preview_default_resizable_ratio  (result f32) f32.const 0.58)

  (func $er_ui_gallery_preview_build_basic  (param $kind i32) (param $base i32) (param $cap i32) (param $id i32) (result i32)
    local.get $kind i32.const 0 i32.eq
    if local.get $base local.get $cap local.get $id i32.const 71900 i32.const 17 i32.const 73472 i32.const 28 i32.const 1 call $er_ui_wasm_new_accordion_full return end
    local.get $kind i32.const 1 i32.eq
    if local.get $base local.get $cap i32.const 71924 i32.const 8 i32.const 71948 i32.const 14 i32.const 0 i32.const 0 call $er_ui_wasm_new_alert return end
    local.get $kind i32.const 2 i32.eq
    if local.get $base local.get $cap local.get $id i32.const 72200 i32.const 13 i32.const 72224 i32.const 13 call $er_ui_wasm_new_alert_dialog return end
    local.get $kind i32.const 3 i32.eq
    if local.get $base local.get $cap i32.const 16 i32.const 9 call $er_ui_wasm_new_aspect_ratio return end
    local.get $kind i32.const 4 i32.eq
    if local.get $base local.get $cap i32.const 73400 i32.const 2 call $er_ui_wasm_new_avatar return end
    local.get $kind i32.const 5 i32.eq
    if local.get $base local.get $cap i32.const 71972 i32.const 7 i32.const 0 call $er_ui_wasm_new_badge return end
    local.get $kind i32.const 7 i32.eq
    if local.get $base local.get $cap local.get $id i32.const 71972 i32.const 7 i32.const 0 i32.const 0 i32.const 0 call $er_ui_wasm_new_button return end
    local.get $kind i32.const 6 i32.eq
    if local.get $base local.get $cap local.get $id i32.const 72248 i32.const 4 i32.const 72272 i32.const 6 call $er_ui_wasm_new_breadcrumb return end
    local.get $kind i32.const 8 i32.eq
    if local.get $base local.get $cap local.get $id i32.const 72296 i32.const 4 i32.const 72320 i32.const 5 i32.const 1 call $er_ui_wasm_new_button_group_full return end
    local.get $kind i32.const 9 i32.eq
    if local.get $base local.get $cap local.get $id i32.const 72344 i32.const 8 i32.const 25 call $er_ui_wasm_new_calendar return end
    local.get $kind i32.const 10 i32.eq
    if local.get $base local.get $cap i32.const 72392 i32.const 10 i32.const 72416 i32.const 11 i32.const 0 call $er_ui_wasm_new_card return end
    local.get $kind i32.const 11 i32.eq
    if local.get $base local.get $cap local.get $id i32.const 72368 i32.const 5 call $er_ui_wasm_new_carousel return end
    local.get $kind i32.const 12 i32.eq
    if local.get $base local.get $cap local.get $id i32.const 72440 i32.const 8 call $er_ui_wasm_new_chart return end
    local.get $kind i32.const 13 i32.eq
    if local.get $base local.get $cap local.get $id i32.const 72464 i32.const 12 i32.const 1 call $er_ui_wasm_new_checkbox return end
    local.get $kind i32.const 14 i32.eq
    if local.get $base local.get $cap local.get $id i32.const 72488 i32.const 19 i32.const 72512 i32.const 5 call $er_ui_wasm_new_combobox return end
    local.get $kind i32.const 15 i32.eq
    if local.get $base local.get $cap local.get $id i32.const 72704 i32.const 17 i32.const 4194 call $er_ui_wasm_new_command return end
    local.get $kind i32.const 16 i32.eq
    if local.get $base local.get $cap local.get $id i32.const 72728 i32.const 7 i32.const 72752 i32.const 8 call $er_ui_wasm_new_context_menu return end
    local.get $kind i32.const 17 i32.eq local.get $kind i32.const 52 i32.eq i32.or
    if local.get $base local.get $cap local.get $id i32.const 71996 i32.const 10 i32.const 72020 i32.const 8 call $er_ui_wasm_new_table return end
    local.get $kind i32.const 18 i32.eq
    if local.get $base local.get $cap local.get $id i32.const 73424 i32.const 12 i32.const 0 call $er_ui_wasm_new_input return end
    local.get $kind i32.const 19 i32.eq
    if local.get $base local.get $cap local.get $id i32.const 73232 i32.const 12 i32.const 72224 i32.const 13 call $er_ui_wasm_new_dialog return end
    local.get $kind i32.const 20 i32.eq
    if local.get $base local.get $cap local.get $id i32.const 72188 i32.const 0 i32.const 0 call $er_ui_wasm_new_direction return end
    local.get $kind i32.const 21 i32.eq
    if local.get $base local.get $cap local.get $id i32.const 73232 i32.const 12 i32.const 73256 i32.const 14 call $er_ui_wasm_new_drawer return end
    local.get $kind i32.const 22 i32.eq
    if local.get $base local.get $cap local.get $id i32.const 72728 i32.const 7 i32.const 72752 i32.const 8 call $er_ui_wasm_new_dropdown_menu return end
    local.get $kind i32.const 23 i32.eq
    if local.get $base local.get $cap i32.const 72536 i32.const 10 i32.const 72560 i32.const 19 i32.const 4399 call $er_ui_wasm_new_empty_state return end
    local.get $kind i32.const 24 i32.eq
    if local.get $base local.get $cap local.get $id i32.const 72584 i32.const 5 i32.const 72608 i32.const 13 call $er_ui_wasm_new_field return end
    local.get $kind i32.const 25 i32.eq
    if local.get $base local.get $cap local.get $id i32.const 72632 i32.const 5 i32.const 72656 i32.const 7 call $er_ui_wasm_new_hover_card return end
    local.get $kind i32.const 26 i32.eq
    if local.get $base local.get $cap i32.const 73448 i32.const 8 i32.const 4399 call $er_ui_wasm_new_icon return end
    local.get $kind i32.const 27 i32.eq
    if local.get $base local.get $cap local.get $id i32.const 72680 i32.const 6 i32.const 4194 i32.const 0 call $er_ui_wasm_new_icon_button_named return end
    local.get $kind i32.const 28 i32.eq
    if local.get $base local.get $cap local.get $id i32.const 72584 i32.const 5 i32.const 0 call $er_ui_wasm_new_input return end
    local.get $kind i32.const 29 i32.eq
    if local.get $base local.get $cap local.get $id i32.const 72776 i32.const 8 i32.const 72800 i32.const 11 call $er_ui_wasm_new_input_group return end
    local.get $kind i32.const 30 i32.eq
    if local.get $base local.get $cap local.get $id i32.const 72824 i32.const 3 call $er_ui_wasm_new_input_otp return end
    local.get $kind i32.const 31 i32.eq
    if local.get $base local.get $cap local.get $id i32.const 72848 i32.const 10 i32.const 72416 i32.const 11 i32.const 0 call $er_ui_wasm_new_row_item return end
    local.get $kind i32.const 32 i32.eq
    if local.get $base local.get $cap i32.const 72872 i32.const 5 call $er_ui_wasm_new_kbd return end
    local.get $kind i32.const 33 i32.eq
    if local.get $base local.get $cap i32.const 72584 i32.const 5 call $er_ui_wasm_new_label return end
    local.get $kind i32.const 34 i32.eq
    if local.get $base local.get $cap local.get $id i32.const 72896 i32.const 4 i32.const 72920 i32.const 4 i32.const 0 call $er_ui_wasm_new_menubar_full return end
    local.get $kind i32.const 43 i32.eq local.get $kind i32.const 35 i32.eq i32.or
    if
      local.get $base local.get $cap local.get $id
      local.get $kind i32.const 35 i32.eq
      if (result i32) i32.const 72140 else i32.const 72116 end
      i32.const 6 i32.const 1300 call $er_ui_wasm_new_select return
    end
    local.get $kind i32.const 36 i32.eq
    if local.get $base local.get $cap local.get $id i32.const 72944 i32.const 4 i32.const 72968 i32.const 10 i32.const 1 call $er_ui_wasm_new_navigation_menu_full return end
    local.get $kind i32.const 37 i32.eq
    if local.get $base local.get $cap local.get $id i32.const 72188 i32.const 0 i32.const 1 call $er_ui_wasm_new_pagination return end
    local.get $kind i32.const 38 i32.eq
    if local.get $base local.get $cap local.get $id i32.const 72992 i32.const 4 i32.const 73016 i32.const 13 call $er_ui_wasm_new_popover return end
    local.get $kind i32.const 50 i32.eq
    if local.get $base local.get $cap local.get $id i32.const 72092 i32.const 5 i32.const 72068 i32.const 12 call $er_ui_wasm_new_toast return end
    local.get $kind i32.const 55 i32.eq
    if local.get $base local.get $cap local.get $id i32.const 72044 i32.const 6 i32.const 72068 i32.const 12 call $er_ui_wasm_new_toast return end
    local.get $kind i32.const 39 i32.eq
    if local.get $base local.get $cap i32.const 0 i32.const 72188 i32.const 0 i32.const 40632 call $er_ui_wasm_new_progress return end
    local.get $kind i32.const 49 i32.eq
    if local.get $base local.get $cap local.get $id i32.const 72164 i32.const 6 i32.const 44564 call $er_ui_wasm_new_slider return end
    local.get $kind i32.const 41 i32.eq
    if local.get $base local.get $cap local.get $id i32.const 72188 i32.const 0 i32.const 38010 call $er_ui_wasm_new_resizable return end
    local.get $kind i32.const 40 i32.eq
    if local.get $base local.get $cap local.get $id i32.const 73040 i32.const 7 i32.const 73064 i32.const 11 i32.const 1 call $er_ui_wasm_new_radio_group_full return end
    local.get $kind i32.const 42 i32.eq
    if local.get $base local.get $cap call $er_ui_wasm_new_scroll_area return end
    local.get $kind i32.const 44 i32.eq
    if local.get $base local.get $cap call $er_ui_wasm_new_separator return end
    local.get $kind i32.const 45 i32.eq
    if local.get $base local.get $cap local.get $id i32.const 73232 i32.const 12 i32.const 73280 i32.const 13 call $er_ui_wasm_new_sheet return end
    local.get $kind i32.const 46 i32.eq
    if local.get $base local.get $cap local.get $id i32.const 73304 i32.const 9 i32.const 73328 i32.const 3 call $er_ui_wasm_new_sidebar return end
    local.get $kind i32.const 47 i32.eq
    if local.get $base local.get $cap call $er_ui_wasm_new_skeleton return end
    local.get $kind i32.const 48 i32.eq
    if local.get $base local.get $cap call $er_ui_wasm_new_spinner return end
    local.get $kind i32.const 51 i32.eq
    if local.get $base local.get $cap local.get $id i32.const 73088 i32.const 13 i32.const 1 call $er_ui_wasm_new_switch return end
    local.get $kind i32.const 53 i32.eq
    if local.get $base local.get $cap local.get $id i32.const 73184 i32.const 7 i32.const 73208 i32.const 8 i32.const 0 call $er_ui_wasm_new_tabs_full return end
    local.get $kind i32.const 54 i32.eq
    if local.get $base local.get $cap local.get $id i32.const 73112 i32.const 23 call $er_ui_wasm_new_textarea return end
    local.get $kind i32.const 56 i32.eq
    if local.get $base local.get $cap local.get $id i32.const 73136 i32.const 4 i32.const 1 call $er_ui_wasm_new_toggle return end
    local.get $kind i32.const 57 i32.eq
    if local.get $base local.get $cap local.get $id i32.const 72296 i32.const 4 i32.const 73160 i32.const 6 i32.const 1 call $er_ui_wasm_new_toggle_group_full return end
    local.get $kind i32.const 58 i32.eq
    if local.get $base local.get $cap local.get $id i32.const 73352 i32.const 8 i32.const 73376 i32.const 14 call $er_ui_wasm_new_tooltip_full return end
    i32.const 0)

  (func $er_ui_gallery_preview_build_basic_for_catalog_index  (param $index i32) (param $base i32) (param $cap i32) (result i32)
    local.get $index i32.const 60 i32.ge_u
    if i32.const 0 return end
    i32.const 66100 local.get $index i32.add i32.load8_u local.get $base local.get $cap i32.const 23000 local.get $index i32.const 32 i32.mul i32.add call $er_ui_gallery_preview_build_basic)
