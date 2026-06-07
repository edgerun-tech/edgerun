  (func $er_ui_popover_trigger_y  (result f32) f32.const 6)
  (func $er_ui_popover_trigger_w  (result f32) f32.const 64)
  (func $er_ui_popover_trigger_h  (result f32) f32.const 30)
  (func $er_ui_popover_gap  (result f32) f32.const 10)
  (func $er_ui_popover_radius  (result f32) f32.const 8)
  (func $er_ui_popover_padding  (result f32) f32.const 10)
  (func $er_ui_popover_label_max_lines  (result i32) i32.const 1)

  (func $er_ui_popover_layout (param $out i32) (result i32)
    local.get $out f32.const 6 f32.const 64 f32.const 30 f32.const 10 call $er_ui_primitives_side_panel_layout)

  (func $er_ui_popover_trigger_id  (param $id i32) (result i32)
    local.get $id)

  (func $er_ui_popover_content_id  (param $id i32) (result i32)
    local.get $id i32.const 1 i32.add)

  (func $er_ui_popover_trigger_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    i32.const 122760 call $er_ui_popover_layout drop
    local.get $bounds i32.const 122760 local.get $out call $er_ui_primitives_side_panel_trigger_bounds)

  (func $er_ui_popover_content_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    i32.const 122760 call $er_ui_popover_layout drop
    local.get $bounds i32.const 122760 local.get $out call $er_ui_primitives_side_panel_content_bounds)

  (func $er_ui_popover_trigger_text_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 122776 call $er_ui_popover_trigger_bounds drop
    i32.const 122776 f32.const 12 i32.const 122792 call $er_ui_primitives_content_inset drop
    i32.const 122792 local.get $out f32.const 16 call $er_ui_rect_with_height_centered)

  (func $er_ui_popover_content_text_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 122808 call $er_ui_popover_content_bounds drop
    i32.const 122808 f32.const 10 i32.const 122824 call $er_ui_primitives_content_inset drop
    i32.const 122824 local.get $out f32.const 16 call $er_ui_rect_with_height_centered)

  (func $er_ui_popover_measure  (param $trigger_ptr i32) (param $trigger_len i32) (param $content_ptr i32) (param $content_len i32) (param $constraints i32) (param $out i32) (result i32)
    (local $trigger_w f32) (local $content_w f32) (local $content_h f32) (local $pref_w f32) (local $pref_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $trigger_ptr local.get $trigger_len f32.const 16 call $er_ui_font_text_width local.set $trigger_w
    i32.const 122840 f32.const 0 f32.store
    i32.const 122844 local.get $trigger_w f32.const 44 f32.add f32.store
    i32.const 122848 f32.const 10 f32.store
    i32.const 122852 f32.const 0 f32.store
    local.get $constraints i32.const 122840 i32.const 122856 call $er_ui_layout_constraints_inner drop
    i32.const 122880 f32.const 16 f32.store
    i32.const 122884 local.get $content_ptr local.get $content_len f32.const 16 call $primitives_average_width f32.store
    i32.const 122888 i32.const 1 i32.store
    local.get $content_ptr local.get $content_len i32.const 122856 i32.const 122880 i32.const 122892 call $er_ui_text_component_measure_value drop
    i32.const 122900 f32.load local.set $content_w
    i32.const 122904 f32.load local.set $content_h
    local.get $trigger_w local.get $content_w f32.add f32.const 54 f32.add local.set $pref_w
    f32.const 46 local.get $content_h f32.const 20 f32.add call $max_f32 local.set $pref_h
    local.get $pref_w local.get $pref_h local.get $constraints i32.const 122916 call $er_ui_primitives_constrain_preferred_size drop
    i32.const 122916 f32.load local.set $pref_w
    i32.const 122920 f32.load local.set $pref_h
    f32.const 12 local.get $pref_w call $min_f32
    f32.const 36 local.get $pref_h call $min_f32
    local.get $pref_w local.get $pref_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit
    local.get $pref_h
    local.get $out
    call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)
  (func $er_ui_context_menu_trigger_label_len  (result i32) i32.const 7)
  (func $er_ui_context_menu_item_count  (result i32) i32.const 2)
  (func $er_ui_context_menu_trigger_y  (result f32) f32.const 4)
  (func $er_ui_context_menu_trigger_w  (result f32) f32.const 64)
  (func $er_ui_context_menu_trigger_h  (result f32) f32.const 30)
  (func $er_ui_context_menu_gap  (result f32) f32.const 8)
  (func $er_ui_context_menu_radius  (result f32) f32.const 8)
  (func $er_ui_context_menu_trigger_padding  (result f32) f32.const 8)
  (func $er_ui_context_menu_list_padding  (result f32) f32.const 5)
  (func $er_ui_context_menu_item_h  (result f32) f32.const 14)
  (func $er_ui_context_menu_item_pitch  (result f32) f32.const 16)
  (func $er_ui_context_menu_item_radius  (result f32) f32.const 4)
  (func $er_ui_context_menu_item_padding  (result f32) f32.const 5)
  (func $er_ui_context_menu_item_text_h  (result f32) f32.const 12)
  (func $er_ui_context_menu_label_max_lines  (result i32) i32.const 1)

  (func $er_ui_context_menu_write_trigger_label (result i32)
    i32.const 122936 i32.const 67 i32.store8
    i32.const 122937 i32.const 111 i32.store8
    i32.const 122938 i32.const 110 i32.store8
    i32.const 122939 i32.const 116 i32.store8
    i32.const 122940 i32.const 101 i32.store8
    i32.const 122941 i32.const 120 i32.store8
    i32.const 122942 i32.const 116 i32.store8
    i32.const 122936)

  (func $er_ui_context_menu_panel_layout (param $out i32) (result i32)
    local.get $out f32.const 4 f32.const 64 f32.const 30 f32.const 8
    call $er_ui_primitives_side_panel_layout)

  (func $er_ui_context_menu_list_layout (param $out i32) (result i32)
    local.get $out f32.const 5 f32.const 14 f32.const 16 f32.const 4 f32.const 5 f32.const 12
    call $er_ui_primitives_menu_list_layout)

  (func $er_ui_context_menu_trigger_id  (param $id i32) (result i32)
    local.get $id call $er_ui_primitives_overlay_trigger_id)

  (func $er_ui_context_menu_item_id  (param $id i32) (param $index i32) (result i32)
    local.get $id local.get $index i32.const 0 i32.const 1 call $clamp_i32 call $er_ui_primitives_overlay_indexed_id)

  (func $er_ui_context_menu_trigger_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    i32.const 122948 call $er_ui_context_menu_panel_layout drop
    local.get $bounds i32.const 122948 local.get $out call $er_ui_primitives_side_panel_trigger_bounds)

  (func $er_ui_context_menu_content_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    i32.const 122948 call $er_ui_context_menu_panel_layout drop
    local.get $bounds i32.const 122948 local.get $out call $er_ui_primitives_side_panel_content_bounds)

  (func $er_ui_context_menu_item_bounds  (param $bounds i32) (param $index i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 122964 call $er_ui_context_menu_content_bounds drop
    i32.const 122980 call $er_ui_context_menu_list_layout drop
    i32.const 122964 local.get $index i32.const 0 i32.const 1 call $clamp_i32 i32.const 122980 local.get $out
    call $er_ui_primitives_menu_item_bounds)

  (func $er_ui_context_menu_trigger_text_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 123004 call $er_ui_context_menu_trigger_bounds drop
    i32.const 123004 f32.const 8 i32.const 123020 call $er_ui_primitives_content_inset drop
    i32.const 123020 local.get $out f32.const 16 call $er_ui_rect_with_height_centered)

  (func $er_ui_context_menu_item_text_bounds  (param $bounds i32) (param $index i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds local.get $index i32.const 123036 call $er_ui_context_menu_item_bounds drop
    i32.const 123036 f32.const 5 i32.const 123052 call $er_ui_primitives_content_inset drop
    i32.const 123052 local.get $out f32.const 12 call $er_ui_rect_with_height_centered)

  (func $er_ui_context_menu_measure_two_item_panel  (param $first_ptr i32) (param $first_len i32) (param $second_ptr i32) (param $second_len i32) (param $constraints i32) (param $out i32) (result i32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    i32.const 122980 call $er_ui_context_menu_list_layout drop
    local.get $first_ptr local.get $first_len local.get $second_ptr local.get $second_len local.get $constraints i32.const 122980 local.get $out
    call $er_ui_primitives_measure_two_item_menu_panel)

  (func $er_ui_context_menu_measure  (param $first_ptr i32) (param $first_len i32) (param $second_ptr i32) (param $second_len i32) (param $constraints i32) (param $out i32) (result i32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    i32.const 122948 call $er_ui_context_menu_panel_layout drop
    i32.const 122980 call $er_ui_context_menu_list_layout drop
    call $er_ui_context_menu_write_trigger_label i32.const 7
    local.get $first_ptr local.get $first_len
    local.get $second_ptr local.get $second_len
    local.get $constraints
    i32.const 122948
    f32.const 8
    i32.const 122980
    local.get $out
    call $er_ui_primitives_measure_side_panel_menu)
  (func $er_ui_hover_card_trigger_y  (result f32) f32.const 6)
  (func $er_ui_hover_card_trigger_w  (result f32) f32.const 66)
  (func $er_ui_hover_card_trigger_h  (result f32) f32.const 30)
  (func $er_ui_hover_card_gap  (result f32) f32.const 10)
  (func $er_ui_hover_card_radius  (result f32) f32.const 8)
  (func $er_ui_hover_card_padding  (result f32) f32.const 10)
  (func $er_ui_hover_card_panel_title_y  (result f32) f32.const 8)
  (func $er_ui_hover_card_panel_title_h  (result f32) f32.const 14)
  (func $er_ui_hover_card_panel_detail_y  (result f32) f32.const 25)
  (func $er_ui_hover_card_panel_detail_h  (result f32) f32.const 12)
  (func $er_ui_hover_card_detail_label_len  (result i32) i32.const 13)
  (func $er_ui_hover_card_text_max_lines  (result i32) i32.const 2)

  (func $er_ui_hover_card_layout (param $out i32) (result i32)
    local.get $out f32.const 6 f32.const 66 f32.const 30 f32.const 10 call $er_ui_primitives_side_panel_layout)

  (func $er_ui_hover_card_write_detail_label (result i32)
    i32.const 122940 i32.const 72 i32.store8
    i32.const 122941 i32.const 111 i32.store8
    i32.const 122942 i32.const 118 i32.store8
    i32.const 122943 i32.const 101 i32.store8
    i32.const 122944 i32.const 114 i32.store8
    i32.const 122945 i32.const 32 i32.store8
    i32.const 122946 i32.const 99 i32.store8
    i32.const 122947 i32.const 111 i32.store8
    i32.const 122948 i32.const 110 i32.store8
    i32.const 122949 i32.const 116 i32.store8
    i32.const 122950 i32.const 101 i32.store8
    i32.const 122951 i32.const 110 i32.store8
    i32.const 122952 i32.const 116 i32.store8
    i32.const 122940)

  (func $er_ui_hover_card_trigger_id  (param $id i32) (result i32)
    local.get $id)

  (func $er_ui_hover_card_content_id  (param $id i32) (result i32)
    local.get $id i32.const 1 i32.add)

  (func $er_ui_hover_card_trigger_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    i32.const 122956 call $er_ui_hover_card_layout drop
    local.get $bounds i32.const 122956 local.get $out call $er_ui_primitives_side_panel_trigger_bounds)

  (func $er_ui_hover_card_content_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    i32.const 122956 call $er_ui_hover_card_layout drop
    local.get $bounds i32.const 122956 local.get $out call $er_ui_primitives_side_panel_content_bounds)

  (func $er_ui_hover_card_trigger_text_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 122972 call $er_ui_hover_card_trigger_bounds drop
    i32.const 122972 f32.const 12 i32.const 122988 call $er_ui_primitives_content_inset drop
    i32.const 122988 local.get $out f32.const 16 call $er_ui_rect_with_height_centered)

  (func $er_ui_hover_card_title_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 123004 call $er_ui_hover_card_content_bounds drop
    local.get $out
    i32.const 123004 f32.load f32.const 10 f32.add
    i32.const 123004 i32.const 4 i32.add f32.load f32.const 8 f32.add
    i32.const 123004 i32.const 8 i32.add f32.load f32.const 20 f32.sub f32.const 1 call $max_f32
    f32.const 14
    call $rect_store)

  (func $er_ui_hover_card_detail_bounds  (param $bounds i32) (param $detail_ptr i32) (param $detail_len i32) (param $out i32) (result i32)
    (local $detail_h f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 123020 call $er_ui_hover_card_content_bounds drop
    local.get $detail_ptr local.get $detail_len i32.const 123020 i32.const 8 i32.add f32.load f32.const 20 f32.sub f32.const 1 call $max_f32 f32.const 12 i32.const 2 call $er_ui_alert_measured_text_height
    i32.const 123020 i32.const 12 i32.add f32.load f32.const 35 f32.sub f32.const 1 call $max_f32
    call $min_f32
    local.set $detail_h
    local.get $out
    i32.const 123020 f32.load f32.const 10 f32.add
    i32.const 123020 i32.const 4 i32.add f32.load f32.const 25 f32.add
    i32.const 123020 i32.const 8 i32.add f32.load f32.const 20 f32.sub f32.const 1 call $max_f32
    local.get $detail_h
    call $rect_store)

  (func $er_ui_hover_card_measure  (param $trigger_ptr i32) (param $trigger_len i32) (param $content_ptr i32) (param $content_len i32) (param $constraints i32) (param $out i32) (result i32)
    (local $trigger_w f32) (local $title_w f32) (local $detail_w f32) (local $panel_w f32) (local $pref_w f32) (local $pref_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $trigger_ptr local.get $trigger_len f32.const 16 call $er_ui_font_text_width f32.const 24 f32.add local.set $trigger_w
    call $er_ui_hover_card_write_detail_label i32.const 13 f32.const 14 call $er_ui_font_text_width local.set $title_w
    local.get $content_ptr local.get $content_len f32.const 12 call $er_ui_font_text_width local.set $detail_w
    local.get $title_w local.get $detail_w call $max_f32 f32.const 20 f32.add local.set $panel_w
    local.get $trigger_w f32.const 10 f32.add local.get $panel_w f32.add local.set $pref_w
    f32.const 47 local.set $pref_h
    local.get $pref_w local.get $pref_h local.get $constraints i32.const 123036 call $er_ui_primitives_constrain_preferred_size drop
    i32.const 123036 f32.load local.set $pref_w
    i32.const 123040 f32.load local.set $pref_h
    f32.const 12 local.get $pref_w call $min_f32
    f32.const 47 local.get $pref_h call $min_f32
    local.get $pref_w local.get $pref_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit
    local.get $pref_h
    local.get $out
    call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)
  (func $er_ui_chart_separator_height  (result f32) f32.const 1)
  (func $er_ui_chart_bar_count  (result i32) i32.const 5)
  (func $er_ui_chart_grid_count  (result i32) i32.const 3)
  (func $er_ui_chart_radius  (result f32) f32.const 8)
  (func $er_ui_chart_padding  (result f32) f32.const 8)
  (func $er_ui_chart_label_h  (result f32) f32.const 14)
  (func $er_ui_chart_label_max_lines  (result i32) i32.const 2)
  (func $er_ui_chart_label_gap  (result f32) f32.const 4)
  (func $er_ui_chart_bar_gap  (result f32) f32.const 5)
  (func $er_ui_chart_bar_radius  (result f32) f32.const 5)
  (func $er_ui_chart_plot_min_h  (result f32) f32.const 64)
  (func $er_ui_chart_min_width  (result f32) f32.const 120)
  (func $er_ui_chart_min_height  (result f32) f32.const 72)
  (func $er_ui_chart_grid_height  (result f32) f32.const 1)

  (func $er_ui_chart_bar_value  (param $index i32) (result f32)
    local.get $index i32.const 0 i32.const 4 call $clamp_i32
    if (result f32)
      local.get $index i32.const 1 i32.eq
      if (result f32)
        f32.const 0.72
      else
        local.get $index i32.const 2 i32.eq
        if (result f32)
          f32.const 0.38
        else
          local.get $index i32.const 3 i32.eq
          if (result f32)
            f32.const 0.86
          else
            local.get $index i32.const 4 i32.eq
            if (result f32)
              f32.const 0.62
            else
              f32.const 0.45
            end
          end
        end
      end
    else
      f32.const 0.45
    end)

  (func $er_ui_chart_label_height (param $label_ptr i32) (param $label_len i32) (param $width f32) (param $bounds_h f32) (result f32)
    local.get $label_len i32.eqz
    if (result f32)
      f32.const 14
    else
      local.get $label_ptr local.get $label_len local.get $width f32.const 14 i32.const 2 call $er_ui_alert_measured_text_height
    end
    local.get $bounds_h f32.const 16 f32.sub f32.const 1 call $max_f32
    call $min_f32)

  (func $er_ui_chart_label_bounds  (param $bounds i32) (param $label_ptr i32) (param $label_len i32) (param $out i32) (result i32)
    (local $w f32) (local $h f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 8 i32.add f32.load f32.const 16 f32.sub f32.const 1 call $max_f32 local.set $w
    local.get $label_ptr local.get $label_len local.get $w local.get $bounds i32.const 12 i32.add f32.load call $er_ui_chart_label_height local.set $h
    local.get $out
    local.get $bounds f32.load f32.const 8 f32.add
    local.get $bounds i32.const 4 i32.add f32.load f32.const 8 f32.add
    local.get $w
    local.get $h
    call $rect_store)

  (func $er_ui_chart_plot_bounds  (param $bounds i32) (param $label_ptr i32) (param $label_len i32) (param $out i32) (result i32)
    (local $label_h f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds local.get $label_ptr local.get $label_len i32.const 123200 call $er_ui_chart_label_bounds drop
    i32.const 123200 i32.const 12 i32.add f32.load local.set $label_h
    local.get $out
    local.get $bounds f32.load f32.const 8 f32.add
    local.get $bounds i32.const 4 i32.add f32.load f32.const 12 f32.add local.get $label_h f32.add
    local.get $bounds i32.const 8 i32.add f32.load f32.const 16 f32.sub f32.const 1 call $max_f32
    local.get $bounds i32.const 12 i32.add f32.load f32.const 20 f32.sub local.get $label_h f32.sub f32.const 1 call $max_f32
    call $rect_store)

  (func $er_ui_chart_grid_line_bounds  (param $bounds i32) (param $label_ptr i32) (param $label_len i32) (param $index i32) (param $out i32) (result i32)
    (local $grid_y f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds local.get $label_ptr local.get $label_len i32.const 123216 call $er_ui_chart_plot_bounds drop
    i32.const 123216 i32.const 4 i32.add f32.load
    i32.const 123216 i32.const 12 i32.add f32.load
    local.get $index i32.const 0 i32.const 2 call $clamp_i32 i32.const 1 i32.add f32.convert_i32_u
    f32.const 4
    f32.div
    f32.mul
    f32.add
    local.set $grid_y
    local.get $out
    i32.const 123216 f32.load
    local.get $grid_y
    i32.const 123216 i32.const 8 i32.add f32.load
    f32.const 1
    call $rect_store)

  (func $er_ui_chart_baseline_bounds  (param $bounds i32) (param $label_ptr i32) (param $label_len i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds local.get $label_ptr local.get $label_len i32.const 123232 call $er_ui_chart_plot_bounds drop
    local.get $out
    i32.const 123232 f32.load
    i32.const 123232 i32.const 4 i32.add f32.load i32.const 123232 i32.const 12 i32.add f32.load f32.add f32.const 1 f32.sub
    i32.const 123232 i32.const 8 i32.add f32.load
    f32.const 1
    call $rect_store)

  (func $er_ui_chart_bar_bounds  (param $bounds i32) (param $label_ptr i32) (param $label_len i32) (param $index i32) (param $out i32) (result i32)
    (local $bar_w f32) (local $h f32) (local $idx i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $index i32.const 0 i32.const 4 call $clamp_i32 local.set $idx
    local.get $bounds local.get $label_ptr local.get $label_len i32.const 123248 call $er_ui_chart_plot_bounds drop
    i32.const 123248 i32.const 8 i32.add f32.load f32.const 20 f32.sub f32.const 5 f32.div f32.const 1 call $max_f32 local.set $bar_w
    i32.const 123248 i32.const 12 i32.add f32.load local.get $idx call $er_ui_chart_bar_value f32.mul f32.const 1 call $max_f32 local.set $h
    local.get $out
    i32.const 123248 f32.load local.get $idx f32.convert_i32_u local.get $bar_w f32.const 5 f32.add f32.mul f32.add
    i32.const 123248 i32.const 4 i32.add f32.load i32.const 123248 i32.const 12 i32.add f32.load f32.add local.get $h f32.sub
    local.get $bar_w
    local.get $h
    call $rect_store)

  (func $er_ui_chart_measure  (param $label_ptr i32) (param $label_len i32) (param $constraints i32) (param $out i32) (result i32)
    (local $label_w f32) (local $label_h f32) (local $pref_w f32) (local $pref_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    i32.const 123264 f32.const 8 f32.store
    i32.const 123268 f32.const 8 f32.store
    i32.const 123272 f32.const 8 f32.store
    i32.const 123276 f32.const 8 f32.store
    local.get $constraints i32.const 123264 i32.const 123280 call $er_ui_layout_constraints_inner drop
    i32.const 123304 f32.const 14 f32.store
    i32.const 123308 local.get $label_ptr local.get $label_len f32.const 14 call $primitives_average_width f32.store
    i32.const 123312 i32.const 2 i32.store
    local.get $label_ptr local.get $label_len i32.const 123280 i32.const 123304 i32.const 123316 call $er_ui_text_component_measure_value drop
    i32.const 123324 f32.load local.set $label_w
    i32.const 123328 f32.load local.set $label_h
    f32.const 120 local.get $label_w f32.const 16 f32.add call $max_f32 local.set $pref_w
    f32.const 16 local.get $label_h f32.add f32.const 4 f32.add f32.const 64 f32.add local.set $pref_h
    local.get $pref_w local.get $pref_h local.get $constraints i32.const 123340 call $er_ui_primitives_constrain_preferred_size drop
    i32.const 123340 f32.load local.set $pref_w
    i32.const 123344 f32.load local.set $pref_h
    f32.const 120 local.get $pref_w call $min_f32
    f32.const 72 local.get $pref_h call $min_f32
    local.get $pref_w local.get $pref_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit
    local.get $pref_h
    local.get $out
    call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)
  (func $er_ui_calendar_day_count  (result i32) i32.const 28)
  (func $er_ui_calendar_day_id_offset  (result i32) i32.const 2)
  (func $er_ui_calendar_column_count  (result i32) i32.const 7)
  (func $er_ui_calendar_row_count  (result f32) f32.const 4)
  (func $er_ui_calendar_radius  (result f32) f32.const 8)
  (func $er_ui_calendar_padding  (result f32) f32.const 8)
  (func $er_ui_calendar_nav_size  (result f32) f32.const 24)
  (func $er_ui_calendar_caption_h  (result f32) f32.const 24)
  (func $er_ui_calendar_weekday_y  (result f32) f32.const 36)
  (func $er_ui_calendar_weekday_h  (result f32) f32.const 16)
  (func $er_ui_calendar_grid_y  (result f32) f32.const 56)
  (func $er_ui_calendar_cell_size  (result f32) f32.const 22)
  (func $er_ui_calendar_cell_gap  (result f32) f32.const 2)
  (func $er_ui_calendar_day_text_h  (result f32) f32.const 12)
  (func $er_ui_calendar_day_text_padding  (result f32) f32.const 2)

  (func $er_ui_calendar_intrinsic_width  (result f32)
    f32.const 170)

  (func $er_ui_calendar_intrinsic_height  (result f32)
    f32.const 152)

  (func $er_ui_calendar_nav_id  (param $id i32) (param $index i32) (result i32)
    local.get $id local.get $index i32.const 0 i32.const 1 call $clamp_i32 i32.add)

  (func $er_ui_calendar_day_id  (param $id i32) (param $index i32) (result i32)
    local.get $id i32.const 2 i32.add local.get $index i32.const 0 i32.const 27 call $clamp_i32 i32.add)

  (func $er_ui_calendar_nav_bounds  (param $bounds i32) (param $index i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $index i32.const 0 i32.eq
    if
      local.get $out
      local.get $bounds f32.load f32.const 8 f32.add
      local.get $bounds i32.const 4 i32.add f32.load f32.const 8 f32.add
      f32.const 24
      f32.const 24
      call $rect_store
      return
    end
    local.get $out
    local.get $bounds f32.load local.get $bounds i32.const 8 i32.add f32.load f32.add f32.const 32 f32.sub
    local.get $bounds i32.const 4 i32.add f32.load f32.const 8 f32.add
    f32.const 24
    f32.const 24
    call $rect_store)

  (func $er_ui_calendar_caption_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $bounds f32.load f32.const 40 f32.add
    local.get $bounds i32.const 4 i32.add f32.load f32.const 8 f32.add
    local.get $bounds i32.const 8 i32.add f32.load f32.const 80 f32.sub f32.const 1 call $max_f32
    f32.const 24
    call $rect_store)

  (func $er_ui_calendar_grid_bounds  (param $bounds i32) (param $out i32) (result i32)
    (local $grid_w f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    f32.const 166 local.set $grid_w
    local.get $out
    local.get $bounds f32.load local.get $bounds i32.const 8 i32.add f32.load local.get $grid_w f32.sub f32.const 0.5 f32.mul f32.add
    local.get $bounds i32.const 4 i32.add f32.load f32.const 56 f32.add
    local.get $grid_w
    local.get $bounds i32.const 12 i32.add f32.load f32.const 64 f32.sub f32.const 1 call $max_f32
    call $rect_store)

  (func $er_ui_calendar_weekday_bounds  (param $bounds i32) (param $index i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 123360 call $er_ui_calendar_grid_bounds drop
    local.get $out
    i32.const 123360 f32.load local.get $index i32.const 0 i32.const 6 call $clamp_i32 f32.convert_i32_u f32.const 24 f32.mul f32.add
    local.get $bounds i32.const 4 i32.add f32.load f32.const 36 f32.add
    f32.const 22
    f32.const 16
    call $rect_store)

  (func $er_ui_calendar_day_bounds  (param $bounds i32) (param $index i32) (param $out i32) (result i32)
    (local $idx i32) (local $col i32) (local $row i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $index i32.const 0 i32.const 27 call $clamp_i32 local.set $idx
    local.get $idx i32.const 7 i32.rem_u local.set $col
    local.get $idx i32.const 7 i32.div_u local.set $row
    local.get $bounds i32.const 123376 call $er_ui_calendar_grid_bounds drop
    local.get $out
    i32.const 123376 f32.load local.get $col f32.convert_i32_u f32.const 24 f32.mul f32.add
    i32.const 123376 i32.const 4 i32.add f32.load local.get $row f32.convert_i32_u f32.const 24 f32.mul f32.add
    f32.const 22
    f32.const 22
    call $rect_store)

  (func $er_ui_calendar_day_text_bounds  (param $bounds i32) (param $index i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds local.get $index i32.const 123392 call $er_ui_calendar_day_bounds drop
    i32.const 123392 f32.const 2 i32.const 123408 call $er_ui_primitives_content_inset drop
    i32.const 123408 local.get $out f32.const 12 call $er_ui_rect_with_height_centered)

  (func $er_ui_calendar_measure  (param $constraints i32) (param $out i32) (result i32)
    (local $pref_w f32) (local $pref_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    f32.const 170 f32.const 152 local.get $constraints i32.const 123424 call $er_ui_primitives_constrain_preferred_size drop
    i32.const 123424 f32.load local.set $pref_w
    i32.const 123428 f32.load local.set $pref_h
    local.get $pref_w local.get $pref_h local.get $pref_w local.get $pref_h local.get $pref_w local.get $pref_h local.get $out call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)
  (func $er_ui_dialog_trigger_y  (result f32) f32.const 6)
  (func $er_ui_dialog_trigger_w  (result f32) f32.const 66)
  (func $er_ui_dialog_trigger_h  (result f32) f32.const 30)
  (func $er_ui_dialog_gap  (result f32) f32.const 12)
  (func $er_ui_dialog_trigger_padding  (result f32) f32.const 8)
  (func $er_ui_dialog_panel_radius  (result f32) f32.const 10)
  (func $er_ui_dialog_panel_padding  (result f32) f32.const 10)
  (func $er_ui_dialog_panel_title_y  (result f32) f32.const 6)
  (func $er_ui_dialog_panel_title_h  (result f32) f32.const 14)
  (func $er_ui_dialog_panel_detail_y  (result f32) f32.const 22)
  (func $er_ui_dialog_panel_detail_h  (result f32) f32.const 12)
  (func $er_ui_dialog_open_label_len  (result i32) i32.const 4)

  (func $er_ui_dialog_open_label_ptr  (result i32)
    i32.const 123440 i32.const 79 i32.store8
    i32.const 123441 i32.const 112 i32.store8
    i32.const 123442 i32.const 101 i32.store8
    i32.const 123443 i32.const 110 i32.store8
    i32.const 123440)

  (func $er_ui_dialog_layout (param $out i32) (result i32)
    local.get $out f32.const 6 f32.const 66 f32.const 30 f32.const 12 call $er_ui_primitives_side_panel_layout)

  (func $er_ui_dialog_panel (param $out i32) (result i32)
    local.get $out f32.const 10 f32.const 10 f32.const 6 f32.const 14 f32.const 22 f32.const 12 f32.const 0 i32.const 1 i32.const 1 call $er_ui_primitives_title_detail_panel)

  (func $er_ui_dialog_trigger_id  (param $id i32) (result i32)
    local.get $id call $er_ui_primitives_overlay_trigger_id)

  (func $er_ui_dialog_content_id  (param $id i32) (result i32)
    local.get $id i32.const 1 call $er_ui_primitives_overlay_indexed_id)

  (func $er_ui_dialog_trigger_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    i32.const 123448 call $er_ui_dialog_layout drop
    local.get $bounds i32.const 123448 local.get $out call $er_ui_primitives_side_panel_trigger_bounds)

  (func $er_ui_dialog_content_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    i32.const 123448 call $er_ui_dialog_layout drop
    local.get $bounds i32.const 123448 local.get $out call $er_ui_primitives_side_panel_content_bounds)

  (func $er_ui_dialog_trigger_text_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 123464 call $er_ui_dialog_trigger_bounds drop
    i32.const 123464 f32.const 8 i32.const 123480 call $er_ui_primitives_content_inset drop
    i32.const 123480 local.get $out f32.const 16 call $er_ui_rect_with_height_centered)

  (func $er_ui_dialog_title_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 123496 call $er_ui_dialog_content_bounds drop
    local.get $out
    i32.const 123496 f32.load f32.const 10 f32.add
    i32.const 123500 f32.load f32.const 6 f32.add
    i32.const 123504 f32.load f32.const 20 f32.sub f32.const 1 call $max_f32
    f32.const 14
    call $rect_store)

  (func $er_ui_dialog_detail_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 123512 call $er_ui_dialog_content_bounds drop
    local.get $out
    i32.const 123512 f32.load f32.const 10 f32.add
    i32.const 123516 f32.load f32.const 22 f32.add
    i32.const 123520 f32.load f32.const 20 f32.sub f32.const 1 call $max_f32
    f32.const 12
    call $rect_store)

  (func $er_ui_dialog_measure  (param $title_ptr i32) (param $title_len i32) (param $detail_ptr i32) (param $detail_len i32) (param $constraints i32) (param $out i32) (result i32)
    (local $trigger_w f32) (local $pref_w f32) (local $pref_h f32) (local $min_h f32) (local $max_w f32) (local $max_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    call $er_ui_dialog_open_label_ptr i32.const 4 f32.const 16 i32.const 1 f32.const 8 call $er_ui_primitives_measured_label_width local.set $trigger_w
    i32.const 123528 f32.const 0 f32.store
    i32.const 123532 f32.const 0 f32.store
    i32.const 123536 f32.const 0 f32.store
    i32.const 123540 f32.const 78 f32.store
    local.get $constraints i32.const 123528 i32.const 123544 call $er_ui_layout_constraints_inner drop
    i32.const 123568 call $er_ui_dialog_panel drop
    local.get $title_ptr local.get $title_len local.get $detail_ptr local.get $detail_len i32.const 123544 i32.const 123568 i32.const 123600 call $er_ui_primitives_measure_title_detail_panel drop
    local.get $constraints local.get $trigger_w f32.const 12 f32.add i32.const 123608 f32.load f32.add call $er_ui_layout_axis_constraint_limit local.set $pref_w
    local.get $constraints i32.const 8 i32.add f32.const 38 i32.const 123612 f32.load call $max_f32 call $er_ui_layout_axis_constraint_limit local.set $pref_h
    f32.const 38 i32.const 123604 f32.load call $max_f32 local.set $min_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit local.set $max_w
    local.get $pref_h i32.const 123620 f32.load call $max_f32 local.set $max_h
    f32.const 14 local.get $min_h local.get $pref_w local.get $pref_h local.get $max_w local.get $max_h local.get $out call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)
  (func $er_ui_toast_radius  (result f32) f32.const 8)
  (func $er_ui_toast_padding  (result f32) f32.const 10)
  (func $er_ui_toast_icon_x  (result f32) f32.const 12)
  (func $er_ui_toast_icon_size  (result f32) f32.const 16)
  (func $er_ui_toast_text_x  (result f32) f32.const 38)
  (func $er_ui_toast_text_gap  (result f32) f32.const 3)
  (func $er_ui_toast_min_width  (result f32) f32.const 160)
  (func $er_ui_toast_min_height  (result f32) f32.const 40)
  (func $er_ui_toast_title_line_height  (result f32) f32.const 14)
  (func $er_ui_toast_detail_line_height  (result f32) f32.const 12)
  (func $er_ui_toast_text_max_lines  (result i32) i32.const 2)

  (func $er_ui_toast_id  (param $id i32) (result i32)
    local.get $id)

  (func $er_ui_toast_text_width  (param $bounds i32) (result f32)
    local.get $bounds i32.eqz
    if f32.const 0 return end
    local.get $bounds i32.const 8 i32.add f32.load f32.const 48 f32.sub f32.const 1 call $max_f32)

  (func $er_ui_toast_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $bounds f32.load
    local.get $bounds i32.const 4 i32.add f32.load
    local.get $bounds i32.const 8 i32.add f32.load
    local.get $bounds i32.const 12 i32.add f32.load
    call $rect_store)

  (func $er_ui_toast_icon_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $bounds f32.load f32.const 12 f32.add
    local.get $bounds i32.const 4 i32.add f32.load local.get $bounds i32.const 12 i32.add f32.load f32.const 16 f32.sub f32.const 0.5 f32.mul f32.add
    f32.const 16
    f32.const 16
    call $rect_store)

  (func $er_ui_toast_title_bounds  (param $bounds i32) (param $title_ptr i32) (param $title_len i32) (param $out i32) (result i32)
    (local $text_w f32) (local $title_h f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds call $er_ui_toast_text_width local.set $text_w
    local.get $title_ptr local.get $title_len local.get $text_w f32.const 14 i32.const 2 call $er_ui_alert_measured_text_height local.set $title_h
    local.get $out
    local.get $bounds f32.load f32.const 38 f32.add
    local.get $bounds i32.const 4 i32.add f32.load f32.const 10 f32.add
    local.get $text_w
    local.get $title_h
    call $rect_store)

  (func $er_ui_toast_detail_bounds  (param $bounds i32) (param $title_ptr i32) (param $title_len i32) (param $detail_ptr i32) (param $detail_len i32) (param $out i32) (result i32)
    (local $text_w f32) (local $title_h f32) (local $detail_y f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $detail_len i32.eqz
    if i32.const 0 return end
    local.get $bounds call $er_ui_toast_text_width local.set $text_w
    local.get $title_ptr local.get $title_len local.get $text_w f32.const 14 i32.const 2 call $er_ui_alert_measured_text_height local.set $title_h
    local.get $bounds i32.const 4 i32.add f32.load f32.const 10 f32.add local.get $title_h f32.add f32.const 3 f32.add local.set $detail_y
    local.get $out
    local.get $bounds f32.load f32.const 38 f32.add
    local.get $detail_y
    local.get $text_w
    local.get $bounds i32.const 4 i32.add f32.load local.get $bounds i32.const 12 i32.add f32.load f32.add local.get $detail_y f32.sub f32.const 10 f32.sub f32.const 1 call $max_f32
    call $rect_store)

  (func $er_ui_toast_measure  (param $title_ptr i32) (param $title_len i32) (param $detail_ptr i32) (param $detail_len i32) (param $constraints i32) (param $out i32) (result i32)
    (local $title_w f32) (local $title_h f32) (local $detail_w f32) (local $detail_h f32) (local $gap f32) (local $pref_w f32) (local $pref_h f32) (local $max_w f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    i32.const 123640 f32.const 0 f32.store
    i32.const 123644 f32.const 10 f32.store
    i32.const 123648 f32.const 0 f32.store
    i32.const 123652 f32.const 38 f32.store
    local.get $constraints i32.const 123640 i32.const 123656 call $er_ui_layout_constraints_inner drop
    local.get $title_ptr local.get $title_len i32.const 123656 f32.const 14 i32.const 2 i32.const 123680 call $primitives_text_measure drop
    i32.const 123688 f32.load local.set $title_w
    i32.const 123692 f32.load local.set $title_h
    local.get $detail_len i32.eqz
    if
      f32.const 0 local.set $detail_w
      f32.const 0 local.set $detail_h
      f32.const 0 local.set $gap
    else
      local.get $detail_ptr local.get $detail_len i32.const 123656 f32.const 12 i32.const 2 i32.const 123704 call $primitives_text_measure drop
      i32.const 123712 f32.load local.set $detail_w
      i32.const 123716 f32.load local.set $detail_h
      f32.const 3 local.set $gap
    end
    f32.const 160 local.get $title_w local.get $detail_w call $max_f32 f32.const 48 f32.add call $max_f32 local.set $pref_w
    f32.const 20 local.get $title_h f32.add local.get $gap f32.add local.get $detail_h f32.add local.set $pref_h
    local.get $pref_w local.get $pref_h local.get $constraints i32.const 123728 call $er_ui_primitives_constrain_preferred_size drop
    i32.const 123728 f32.load local.set $pref_w
    i32.const 123732 f32.load local.set $pref_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit local.set $max_w
    f32.const 160 local.get $pref_w call $min_f32
    f32.const 40 local.get $pref_h call $min_f32
    local.get $pref_w local.get $pref_h local.get $max_w local.get $pref_h local.get $out call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)
  (func $er_ui_drawer_trigger_y  (result f32) f32.const 4)
  (func $er_ui_drawer_trigger_w  (result f32) f32.const 62)
  (func $er_ui_drawer_trigger_h  (result f32) f32.const 30)
  (func $er_ui_drawer_trigger_padding  (result f32) f32.const 8)
  (func $er_ui_drawer_content_y  (result f32) f32.const 38)
  (func $er_ui_drawer_content_inset_x  (result f32) f32.const 10)
  (func $er_ui_drawer_radius  (result f32) f32.const 10)
  (func $er_ui_drawer_padding  (result f32) f32.const 12)
  (func $er_ui_drawer_handle_w  (result f32) f32.const 58)
  (func $er_ui_drawer_handle_h  (result f32) f32.const 4)
  (func $er_ui_drawer_handle_y  (result f32) f32.const 5)
  (func $er_ui_drawer_handle_radius  (result f32) f32.const 2)
  (func $er_ui_drawer_panel_title_y  (result f32) f32.const 14)
  (func $er_ui_drawer_panel_title_h  (result f32) f32.const 14)
  (func $er_ui_drawer_panel_detail_y  (result f32) f32.const 31)
  (func $er_ui_drawer_panel_detail_h  (result f32) f32.const 12)

  (func $er_ui_drawer_panel (param $out i32) (result i32)
    local.get $out f32.const 10 f32.const 12 f32.const 14 f32.const 14 f32.const 31 f32.const 12 f32.const 0 i32.const 1 i32.const 1 call $er_ui_primitives_title_detail_panel)

  (func $er_ui_drawer_trigger_id  (param $id i32) (result i32)
    local.get $id call $er_ui_primitives_overlay_trigger_id)

  (func $er_ui_drawer_content_id  (param $id i32) (result i32)
    local.get $id i32.const 1 call $er_ui_primitives_overlay_indexed_id)

  (func $er_ui_drawer_trigger_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $bounds f32.load
    local.get $bounds i32.const 4 i32.add f32.load f32.const 4 f32.add
    f32.const 62
    f32.const 30
    call $rect_store)

  (func $er_ui_drawer_content_bounds  (param $bounds i32) (param $out i32) (result i32)
    (local $y f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 4 i32.add f32.load f32.const 38 f32.add local.set $y
    local.get $out
    local.get $bounds f32.load f32.const 10 f32.add
    local.get $y
    local.get $bounds i32.const 8 i32.add f32.load f32.const 20 f32.sub f32.const 1 call $max_f32
    local.get $bounds i32.const 4 i32.add f32.load local.get $bounds i32.const 12 i32.add f32.load f32.add local.get $y f32.sub f32.const 1 call $max_f32
    call $rect_store)

  (func $er_ui_drawer_handle_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 123760 call $er_ui_drawer_content_bounds drop
    local.get $out
    i32.const 123760 f32.load i32.const 123768 f32.load f32.const 58 f32.sub f32.const 0.5 f32.mul f32.add
    i32.const 123764 f32.load f32.const 5 f32.add
    f32.const 58
    f32.const 4
    call $rect_store)

  (func $er_ui_drawer_trigger_text_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 123776 call $er_ui_drawer_trigger_bounds drop
    i32.const 123776 f32.const 8 i32.const 123792 call $er_ui_primitives_content_inset drop
    i32.const 123792 local.get $out f32.const 16 call $er_ui_rect_with_height_centered)

  (func $er_ui_drawer_title_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 123808 call $er_ui_drawer_content_bounds drop
    local.get $out
    i32.const 123808 f32.load f32.const 12 f32.add
    i32.const 123812 f32.load f32.const 14 f32.add
    i32.const 123816 f32.load f32.const 24 f32.sub f32.const 1 call $max_f32
    f32.const 14
    call $rect_store)

  (func $er_ui_drawer_detail_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 123824 call $er_ui_drawer_content_bounds drop
    local.get $out
    i32.const 123824 f32.load f32.const 12 f32.add
    i32.const 123828 f32.load f32.const 31 f32.add
    i32.const 123832 f32.load f32.const 24 f32.sub f32.const 1 call $max_f32
    f32.const 12
    call $rect_store)

  (func $er_ui_drawer_measure  (param $title_ptr i32) (param $title_len i32) (param $detail_ptr i32) (param $detail_len i32) (param $constraints i32) (param $out i32) (result i32)
    (local $trigger_w f32) (local $pref_w f32) (local $pref_h f32) (local $max_w f32) (local $max_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    call $er_ui_dialog_open_label_ptr i32.const 4 f32.const 16 i32.const 1 f32.const 8 call $er_ui_primitives_measured_label_width local.set $trigger_w
    i32.const 123840 f32.const 38 f32.store
    i32.const 123844 f32.const 10 f32.store
    i32.const 123848 f32.const 0 f32.store
    i32.const 123852 f32.const 10 f32.store
    local.get $constraints i32.const 123840 i32.const 123856 call $er_ui_layout_constraints_inner drop
    i32.const 123880 call $er_ui_drawer_panel drop
    local.get $title_ptr local.get $title_len local.get $detail_ptr local.get $detail_len i32.const 123856 i32.const 123880 i32.const 123912 call $er_ui_primitives_measure_title_detail_panel drop
    local.get $constraints local.get $trigger_w f32.const 16 f32.add i32.const 123920 f32.load f32.const 20 f32.add call $max_f32 call $er_ui_layout_axis_constraint_limit local.set $pref_w
    local.get $constraints i32.const 8 i32.add f32.const 38 i32.const 123924 f32.load f32.add call $er_ui_layout_axis_constraint_limit local.set $pref_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit local.set $max_w
    local.get $pref_h i32.const 123932 f32.load f32.const 38 f32.add call $max_f32 local.set $max_h
    f32.const 21 f32.const 93 local.get $pref_w local.get $pref_h local.get $max_w local.get $max_h local.get $out call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)
  (func $er_ui_sheet_trigger_y  (result f32) f32.const 4)
  (func $er_ui_sheet_trigger_w  (result f32) f32.const 62)
  (func $er_ui_sheet_trigger_h  (result f32) f32.const 30)
  (func $er_ui_sheet_trigger_padding  (result f32) f32.const 8)
  (func $er_ui_sheet_content_w  (result f32) f32.const 96)
  (func $er_ui_sheet_content_min_left  (result f32) f32.const 82)
  (func $er_ui_sheet_radius  (result f32) f32.const 8)
  (func $er_ui_sheet_padding  (result f32) f32.const 10)
  (func $er_ui_sheet_close_size  (result f32) f32.const 28)
  (func $er_ui_sheet_close_inset  (result f32) f32.const 8)
  (func $er_ui_sheet_close_space  (result f32) f32.const 34)
  (func $er_ui_sheet_panel_title_y  (result f32) f32.const 10)
  (func $er_ui_sheet_panel_title_h  (result f32) f32.const 14)
  (func $er_ui_sheet_panel_detail_y  (result f32) f32.const 29)
  (func $er_ui_sheet_panel_detail_h  (result f32) f32.const 12)

  (func $er_ui_sheet_panel (param $out i32) (result i32)
    local.get $out f32.const 8 f32.const 10 f32.const 10 f32.const 14 f32.const 29 f32.const 12 f32.const 34 i32.const 1 i32.const 1 call $er_ui_primitives_title_detail_panel)

  (func $er_ui_sheet_trigger_id  (param $id i32) (result i32)
    local.get $id call $er_ui_primitives_overlay_trigger_id)

  (func $er_ui_sheet_content_id  (param $id i32) (result i32)
    local.get $id i32.const 1 call $er_ui_primitives_overlay_indexed_id)

  (func $er_ui_sheet_close_id  (param $id i32) (result i32)
    local.get $id call $er_ui_primitives_overlay_secondary_id)

  (func $er_ui_sheet_trigger_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $bounds f32.load
    local.get $bounds i32.const 4 i32.add f32.load f32.const 4 f32.add
    f32.const 62
    f32.const 30
    call $rect_store)

  (func $er_ui_sheet_content_width_for_bounds  (param $bounds i32) (result f32)
    local.get $bounds i32.eqz
    if f32.const 1 return end
    f32.const 96
    local.get $bounds i32.const 8 i32.add f32.load f32.const 82 f32.sub f32.const 1 call $max_f32
    call $min_f32)

  (func $er_ui_sheet_content_bounds  (param $bounds i32) (param $out i32) (result i32)
    (local $w f32) (local $x f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds call $er_ui_sheet_content_width_for_bounds local.set $w
    local.get $bounds f32.load local.get $bounds i32.const 8 i32.add f32.load f32.add local.get $w f32.sub local.set $x
    local.get $out
    local.get $x
    local.get $bounds i32.const 4 i32.add f32.load
    local.get $bounds f32.load local.get $bounds i32.const 8 i32.add f32.load f32.add local.get $x f32.sub f32.const 1 call $max_f32
    local.get $bounds i32.const 12 i32.add f32.load
    call $rect_store)

  (func $er_ui_sheet_close_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 123960 call $er_ui_sheet_content_bounds drop
    local.get $out
    i32.const 123960 f32.load i32.const 123968 f32.load f32.add f32.const 36 f32.sub
    i32.const 123964 f32.load f32.const 8 f32.add
    f32.const 28
    f32.const 28
    call $rect_store)

  (func $er_ui_sheet_trigger_text_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 123976 call $er_ui_sheet_trigger_bounds drop
    i32.const 123976 f32.const 8 i32.const 123992 call $er_ui_primitives_content_inset drop
    i32.const 123992 local.get $out f32.const 16 call $er_ui_rect_with_height_centered)

  (func $er_ui_sheet_title_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 124008 call $er_ui_sheet_content_bounds drop
    local.get $out
    i32.const 124008 f32.load f32.const 10 f32.add
    i32.const 124012 f32.load f32.const 10 f32.add
    i32.const 124016 f32.load f32.const 54 f32.sub f32.const 1 call $max_f32
    f32.const 14
    call $rect_store)

  (func $er_ui_sheet_detail_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 124024 call $er_ui_sheet_content_bounds drop
    local.get $out
    i32.const 124024 f32.load f32.const 10 f32.add
    i32.const 124028 f32.load f32.const 29 f32.add
    i32.const 124032 f32.load f32.const 20 f32.sub f32.const 1 call $max_f32
    f32.const 12
    call $rect_store)

  (func $er_ui_sheet_measure  (param $title_ptr i32) (param $title_len i32) (param $detail_ptr i32) (param $detail_len i32) (param $constraints i32) (param $out i32) (result i32)
    (local $trigger_w f32) (local $pref_w f32) (local $pref_h f32) (local $max_w f32) (local $max_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    call $er_ui_dialog_open_label_ptr i32.const 4 f32.const 16 i32.const 1 f32.const 8 call $er_ui_primitives_measured_label_width local.set $trigger_w
    i32.const 124040 call $er_ui_sheet_panel drop
    local.get $title_ptr local.get $title_len local.get $detail_ptr local.get $detail_len local.get $constraints i32.const 124040 i32.const 124072 call $er_ui_primitives_measure_title_detail_panel drop
    local.get $constraints local.get $trigger_w f32.const 16 f32.add i32.const 124080 f32.load f32.const 82 f32.add call $max_f32 call $er_ui_layout_axis_constraint_limit local.set $pref_w
    local.get $constraints i32.const 8 i32.add f32.const 36 i32.const 124084 f32.load call $max_f32 call $er_ui_layout_axis_constraint_limit local.set $pref_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit local.set $max_w
    local.get $pref_h i32.const 124092 f32.load call $max_f32 local.set $max_h
    f32.const 83 f32.const 51 local.get $pref_w local.get $pref_h local.get $max_w local.get $max_h local.get $out call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)
  (func $er_ui_sidebar_rail_w  (result f32) f32.const 62)
  (func $er_ui_sidebar_content_gap  (result f32) f32.const 10)
  (func $er_ui_sidebar_radius  (result f32) f32.const 8)
  (func $er_ui_sidebar_trigger_x  (result f32) f32.const 17)
  (func $er_ui_sidebar_trigger_y  (result f32) f32.const 8)
  (func $er_ui_sidebar_trigger_size  (result f32) f32.const 28)
  (func $er_ui_sidebar_title_y  (result f32) f32.const 42)
  (func $er_ui_sidebar_title_h  (result f32) f32.const 12)
  (func $er_ui_sidebar_title_max_lines  (result i32) i32.const 2)
  (func $er_ui_sidebar_item_x  (result f32) f32.const 6)
  (func $er_ui_sidebar_item_y  (result f32) f32.const 66)
  (func $er_ui_sidebar_item_h  (result f32) f32.const 20)
  (func $er_ui_sidebar_item_bottom_padding  (result f32) f32.const 10)
  (func $er_ui_sidebar_item_radius  (result f32) f32.const 4)
  (func $er_ui_sidebar_item_padding  (result f32) f32.const 5)
  (func $er_ui_sidebar_item_text_h  (result f32) f32.const 12)
  (func $er_ui_sidebar_item_max_lines  (result i32) i32.const 2)
  (func $er_ui_sidebar_content_min_w  (result f32) f32.const 120)
  (func $er_ui_sidebar_min_width  (result f32) f32.const 160)
  (func $er_ui_sidebar_min_height  (result f32) f32.const 48)

  (func $er_ui_sidebar_trigger_id  (param $id i32) (result i32)
    local.get $id call $er_ui_primitives_overlay_trigger_id)

  (func $er_ui_sidebar_item_id  (param $id i32) (result i32)
    local.get $id i32.const 1 call $er_ui_primitives_overlay_indexed_id)

  (func $er_ui_sidebar_title_inner_width  (result f32)
    f32.const 50)

  (func $er_ui_sidebar_item_inner_width  (result f32)
    f32.const 40)

  (func $er_ui_sidebar_rail_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $bounds f32.load
    local.get $bounds i32.const 4 i32.add f32.load
    local.get $bounds i32.const 8 i32.add f32.load f32.const 62 call $min_f32
    local.get $bounds i32.const 12 i32.add f32.load
    call $rect_store)

  (func $er_ui_sidebar_trigger_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $bounds f32.load f32.const 17 f32.add
    local.get $bounds i32.const 4 i32.add f32.load f32.const 8 f32.add
    f32.const 28
    f32.const 28
    call $rect_store)

  (func $er_ui_sidebar_title_height  (param $title_ptr i32) (param $title_len i32) (result f32)
    local.get $title_ptr local.get $title_len f32.const 50 f32.const 12 i32.const 2 call $er_ui_alert_measured_text_height)

  (func $er_ui_sidebar_item_text_height  (param $item_ptr i32) (param $item_len i32) (result f32)
    local.get $item_ptr local.get $item_len f32.const 40 f32.const 12 i32.const 2 call $er_ui_alert_measured_text_height)

  (func $er_ui_sidebar_title_bounds  (param $bounds i32) (param $title_ptr i32) (param $title_len i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $bounds f32.load f32.const 6 f32.add
    local.get $bounds i32.const 4 i32.add f32.load f32.const 42 f32.add
    f32.const 50
    local.get $title_ptr local.get $title_len call $er_ui_sidebar_title_height
    call $rect_store)

  (func $er_ui_sidebar_item_bounds  (param $bounds i32) (param $title_ptr i32) (param $title_len i32) (param $item_ptr i32) (param $item_len i32) (param $out i32) (result i32)
    (local $title_h f32) (local $text_h f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $title_ptr local.get $title_len call $er_ui_sidebar_title_height local.set $title_h
    local.get $item_ptr local.get $item_len call $er_ui_sidebar_item_text_height local.set $text_h
    local.get $out
    local.get $bounds f32.load f32.const 6 f32.add
    local.get $bounds i32.const 4 i32.add f32.load f32.const 66 f32.add local.get $title_h f32.add f32.const 12 f32.sub
    f32.const 50
    f32.const 20 local.get $text_h f32.const 10 f32.add call $max_f32
    call $rect_store)

  (func $er_ui_sidebar_item_text_bounds  (param $item_bounds i32) (param $item_ptr i32) (param $item_len i32) (param $out i32) (result i32)
    (local $w f32) (local $h f32)
    local.get $item_bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $item_bounds i32.const 8 i32.add f32.load f32.const 10 f32.sub f32.const 1 call $max_f32 local.set $w
    local.get $item_ptr local.get $item_len local.get $w f32.const 12 i32.const 2 call $er_ui_alert_measured_text_height
    local.get $item_bounds i32.const 12 i32.add f32.load
    call $min_f32
    local.set $h
    local.get $out
    local.get $item_bounds f32.load f32.const 5 f32.add
    local.get $item_bounds i32.const 4 i32.add f32.load local.get $item_bounds i32.const 12 i32.add f32.load local.get $h f32.sub f32.const 0.5 f32.mul f32.add
    local.get $w
    local.get $h
    call $rect_store)

  (func $er_ui_sidebar_content_bounds  (param $bounds i32) (param $out i32) (result i32)
    (local $x f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds f32.load f32.const 72 f32.add local.set $x
    local.get $out
    local.get $x
    local.get $bounds i32.const 4 i32.add f32.load
    local.get $bounds f32.load local.get $bounds i32.const 8 i32.add f32.load f32.add local.get $x f32.sub f32.const 1 call $max_f32
    local.get $bounds i32.const 12 i32.add f32.load
    call $rect_store)

  (func $er_ui_sidebar_measure  (param $title_ptr i32) (param $title_len i32) (param $item_ptr i32) (param $item_len i32) (param $constraints i32) (param $out i32) (result i32)
    (local $title_h f32) (local $item_h f32) (local $rail_h f32) (local $pref_w f32) (local $pref_h f32) (local $max_w f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $title_ptr local.get $title_len call $er_ui_sidebar_title_height local.set $title_h
    local.get $item_ptr local.get $item_len call $er_ui_sidebar_item_text_height f32.const 10 f32.add f32.const 20 call $max_f32 local.set $item_h
    f32.const 66 local.get $title_h f32.add f32.const 12 f32.sub local.get $item_h f32.add f32.const 10 f32.add local.set $rail_h
    f32.const 192 local.get $rail_h f32.const 48 call $max_f32 local.get $constraints i32.const 124120 call $er_ui_primitives_constrain_preferred_size drop
    i32.const 124120 f32.load local.set $pref_w
    i32.const 124124 f32.load local.set $pref_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit local.set $max_w
    f32.const 160 local.get $pref_w call $min_f32
    f32.const 48 local.get $pref_h call $min_f32
    local.get $pref_w local.get $pref_h local.get $max_w local.get $pref_h local.get $out call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)
  (func $er_ui_dropdown_menu_trigger_label_len  (result i32) i32.const 4)
  (func $er_ui_dropdown_menu_item_count  (result i32) i32.const 2)
  (func $er_ui_dropdown_menu_trigger_y  (result f32) f32.const 4)
  (func $er_ui_dropdown_menu_trigger_w  (result f32) f32.const 64)
  (func $er_ui_dropdown_menu_trigger_h  (result f32) f32.const 30)
  (func $er_ui_dropdown_menu_gap  (result f32) f32.const 8)
  (func $er_ui_dropdown_menu_radius  (result f32) f32.const 8)
  (func $er_ui_dropdown_menu_trigger_padding  (result f32) f32.const 8)
  (func $er_ui_dropdown_menu_list_padding  (result f32) f32.const 5)
  (func $er_ui_dropdown_menu_item_h  (result f32) f32.const 14)
  (func $er_ui_dropdown_menu_item_pitch  (result f32) f32.const 16)
  (func $er_ui_dropdown_menu_item_radius  (result f32) f32.const 4)
  (func $er_ui_dropdown_menu_item_padding  (result f32) f32.const 5)
  (func $er_ui_dropdown_menu_item_text_h  (result f32) f32.const 12)
  (func $er_ui_dropdown_menu_label_max_lines  (result i32) i32.const 1)

  (func $er_ui_dropdown_menu_panel_layout (param $out i32) (result i32)
    local.get $out f32.const 4 f32.const 64 f32.const 30 f32.const 8
    call $er_ui_primitives_side_panel_layout)

  (func $er_ui_dropdown_menu_list_layout (param $out i32) (result i32)
    local.get $out f32.const 5 f32.const 14 f32.const 16 f32.const 4 f32.const 5 f32.const 12
    call $er_ui_primitives_menu_list_layout)

  (func $er_ui_dropdown_menu_trigger_id  (param $id i32) (result i32)
    local.get $id call $er_ui_primitives_overlay_trigger_id)

  (func $er_ui_dropdown_menu_item_id  (param $id i32) (param $index i32) (result i32)
    local.get $id local.get $index i32.const 0 i32.const 1 call $clamp_i32 call $er_ui_primitives_overlay_indexed_id)

  (func $er_ui_dropdown_menu_trigger_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    i32.const 124160 call $er_ui_dropdown_menu_panel_layout drop
    local.get $bounds i32.const 124160 local.get $out call $er_ui_primitives_side_panel_trigger_bounds)

  (func $er_ui_dropdown_menu_content_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    i32.const 124160 call $er_ui_dropdown_menu_panel_layout drop
    local.get $bounds i32.const 124160 local.get $out call $er_ui_primitives_side_panel_content_bounds)

  (func $er_ui_dropdown_menu_item_bounds  (param $bounds i32) (param $index i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 124176 call $er_ui_dropdown_menu_content_bounds drop
    i32.const 124192 call $er_ui_dropdown_menu_list_layout drop
    i32.const 124176 local.get $index i32.const 0 i32.const 1 call $clamp_i32 i32.const 124192 local.get $out
    call $er_ui_primitives_menu_item_bounds)

  (func $er_ui_dropdown_menu_trigger_text_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 124216 call $er_ui_dropdown_menu_trigger_bounds drop
    i32.const 124216 f32.const 8 i32.const 124232 call $er_ui_primitives_content_inset drop
    i32.const 124232 local.get $out f32.const 16 call $er_ui_rect_with_height_centered)

  (func $er_ui_dropdown_menu_item_text_bounds  (param $bounds i32) (param $index i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds local.get $index i32.const 124248 call $er_ui_dropdown_menu_item_bounds drop
    i32.const 124248 f32.const 5 i32.const 124264 call $er_ui_primitives_content_inset drop
    i32.const 124264 local.get $out f32.const 12 call $er_ui_rect_with_height_centered)

  (func $er_ui_dropdown_menu_measure_two_item_panel  (param $first_ptr i32) (param $first_len i32) (param $second_ptr i32) (param $second_len i32) (param $constraints i32) (param $out i32) (result i32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    i32.const 124192 call $er_ui_dropdown_menu_list_layout drop
    local.get $first_ptr local.get $first_len local.get $second_ptr local.get $second_len local.get $constraints i32.const 124192 local.get $out
    call $er_ui_primitives_measure_two_item_menu_panel)

  (func $er_ui_dropdown_menu_measure  (param $first_ptr i32) (param $first_len i32) (param $second_ptr i32) (param $second_len i32) (param $constraints i32) (param $out i32) (result i32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    i32.const 124160 call $er_ui_dropdown_menu_panel_layout drop
    i32.const 124192 call $er_ui_dropdown_menu_list_layout drop
    call $er_ui_dialog_open_label_ptr i32.const 4
    local.get $first_ptr local.get $first_len
    local.get $second_ptr local.get $second_len
    local.get $constraints
    i32.const 124160
    f32.const 8
    i32.const 124192
    local.get $out
    call $er_ui_primitives_measure_side_panel_menu)
