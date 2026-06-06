  (data (i32.const 71600) "\03\04\05\06\1d\1b\26\0c\0e\07\1c\08\09\16\0a\2c\2d\20\10\2e\2b\2f\30\0b\15\31\37\0d\10\11\12\1a\1e\1f\27\14\28\29\32\25\19\39\22\14\21\35\36\23\24\18\34\17\20\2a\13\34\38\0f\33")
  (data (i32.const 71700) "\00\00\00\00\00\01\00\02\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00")
  (data (i32.const 71800) "\01\00\01\00\00\00\01\01\01\01\00\01\01\01\01\01\01\01\01\01\01\01\01\00\01\01\00\01\01\01\01\01\00\00\01\01\01\01\01\00\01\01\00\01\00\01\01\00\00\01\01\01\01\01\01\01\01\01\01")
  (data (i32.const 71900) "Is it accessible?")
  (data (i32.const 71924) "Heads up")
  (data (i32.const 71948) "Status message")
  (data (i32.const 71972) "Default")
  (data (i32.const 71996) "Sarah Chen")
  (data (i32.const 72020) "Engineer")
  (data (i32.const 72044) "Queued")
  (data (i32.const 72068) "Notification")
  (data (i32.const 72092) "Saved")
  (data (i32.const 72116) "Select")
  (data (i32.const 72140) "Native")
  (data (i32.const 72164) "Volume")
  (data (i32.const 72200) "Are you sure?")
  (data (i32.const 72224) "Modal content")
  (data (i32.const 72248) "Home")
  (data (i32.const 72272) "Button")
  (data (i32.const 72296) "Left")
  (data (i32.const 72320) "Right")
  (data (i32.const 72344) "May 2026")
  (data (i32.const 72368) "Slide")
  (data (i32.const 72392) "Card title")
  (data (i32.const 72416) "Description")
  (data (i32.const 72440) "Visitors")
  (data (i32.const 72464) "Accept terms")
  (data (i32.const 72488) "Search framework...")
  (data (i32.const 72512) "React")
  (data (i32.const 72536) "No results")
  (data (i32.const 72560) "Try another filter.")
  (data (i32.const 72584) "Email")
  (data (i32.const 72608) "m@example.com")
  (data (i32.const 72632) "Hover")
  (data (i32.const 72656) "@shadcn")
  (data (i32.const 72680) "Search")
  (data (i32.const 72704) "Type a command...")
  (data (i32.const 72728) "Profile")
  (data (i32.const 72752) "Settings")
  (data (i32.const 72776) "https://")
  (data (i32.const 72800) "example.com")
  (data (i32.const 72824) "123")
  (data (i32.const 72848) "Item title")
  (data (i32.const 72872) "Cmd K")
  (data (i32.const 72896) "File")
  (data (i32.const 72920) "Edit")
  (data (i32.const 72944) "Docs")
  (data (i32.const 72968) "Components")
  (data (i32.const 72992) "Open")
  (data (i32.const 73016) "Place content")
  (data (i32.const 73040) "Default")
  (data (i32.const 73064) "Comfortable")
  (data (i32.const 73088) "Airplane Mode")
  (data (i32.const 73112) "Type your message here.")
  (data (i32.const 73136) "Bold")
  (data (i32.const 73160) "Center")
  (data (i32.const 73184) "Account")
  (data (i32.const 73208) "Password")
  (data (i32.const 73232) "Edit profile")
  (data (i32.const 73256) "Drawer content")
  (data (i32.const 73280) "Sheet content")
  (data (i32.const 73304) "Workspace")
  (data (i32.const 73328) "Nav")
  (data (i32.const 73352) "Hover me")
  (data (i32.const 73376) "Add to library")
  (data (i32.const 73400) "ER")
  (data (i32.const 73424) "May 25, 2026")
  (data (i32.const 73448) "sparkles")
  (data (i32.const 73472) "Yes. It follows the pattern.")

  (func (export "er_ui_gallery_preview_strategy_primitive") (result i32) i32.const 0)
  (func (export "er_ui_gallery_preview_strategy_badge_variants") (result i32) i32.const 1)
  (func (export "er_ui_gallery_preview_strategy_button_variants") (result i32) i32.const 2)

  (func (export "er_ui_gallery_preview_kind_badge") (result i32) i32.const 5)
  (func (export "er_ui_gallery_preview_kind_button") (result i32) i32.const 7)
  (func (export "er_ui_gallery_preview_kind_data_table") (result i32) i32.const 17)
  (func (export "er_ui_gallery_preview_kind_table") (result i32) i32.const 52)
  (func (export "er_ui_gallery_preview_kind_sonner") (result i32) i32.const 50)
  (func (export "er_ui_gallery_preview_kind_native_select") (result i32) i32.const 35)

  (func $er_ui_gallery_preview_table_u8 (param $base i32) (param $kind i32) (result i32)
    local.get $kind i32.const 59 i32.ge_u
    if i32.const -1 return end
    local.get $base local.get $kind i32.add i32.load8_u)

  (func $er_ui_gallery_preview_strategy_for_kind (export "er_ui_gallery_preview_strategy_for_kind") (param $kind i32) (result i32)
    i32.const 71700 local.get $kind call $er_ui_gallery_preview_table_u8)

  (func (export "er_ui_gallery_preview_strategy_for_catalog_index") (param $index i32) (result i32)
    local.get $index i32.const 60 i32.ge_u
    if i32.const -1 return end
    i32.const 71700 i32.const 66100 local.get $index i32.add i32.load8_u call $er_ui_gallery_preview_table_u8)

  (func $er_ui_gallery_preview_component_kind_for_kind (export "er_ui_gallery_preview_component_kind_for_kind") (param $kind i32) (result i32)
    i32.const 71600 local.get $kind call $er_ui_gallery_preview_table_u8)

  (func (export "er_ui_gallery_preview_component_kind_for_catalog_index") (param $index i32) (result i32)
    local.get $index i32.const 60 i32.ge_u
    if i32.const -1 return end
    i32.const 71600 i32.const 66100 local.get $index i32.add i32.load8_u call $er_ui_gallery_preview_table_u8)

  (func $er_ui_gallery_preview_uses_control_id_for_kind (export "er_ui_gallery_preview_uses_control_id_for_kind") (param $kind i32) (result i32)
    i32.const 71800 local.get $kind call $er_ui_gallery_preview_table_u8)

  (func (export "er_ui_gallery_preview_uses_control_id_for_catalog_index") (param $index i32) (result i32)
    local.get $index i32.const 60 i32.ge_u
    if i32.const -1 return end
    i32.const 71800 i32.const 66100 local.get $index i32.add i32.load8_u call $er_ui_gallery_preview_table_u8)

  (func $er_ui_gallery_preview_variant_count_for_kind (export "er_ui_gallery_preview_variant_count_for_kind") (param $kind i32) (result i32)
    (local $strategy i32)
    local.get $kind call $er_ui_gallery_preview_strategy_for_kind local.tee $strategy i32.const 0 i32.lt_s
    if i32.const -1 return end
    local.get $strategy i32.eqz
    if i32.const 1 return end
    i32.const 3)

  (func (export "er_ui_gallery_preview_variant_count_for_catalog_index") (param $index i32) (result i32)
    (local $kind i32)
    local.get $index i32.const 60 i32.ge_u
    if i32.const -1 return end
    i32.const 66100 local.get $index i32.add i32.load8_u local.tee $kind call $er_ui_gallery_preview_variant_count_for_kind)

  (func (export "er_ui_gallery_preview_variant_gap") (result f32) f32.const 6)

  (func $er_ui_gallery_preview_variant_slot_width (export "er_ui_gallery_preview_variant_slot_width") (param $kind i32) (param $variant i32) (result f32)
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

  (func $er_ui_gallery_preview_variant_id (export "er_ui_gallery_preview_variant_id") (param $kind i32) (param $base_id i32) (param $variant i32) (result i32)
    local.get $kind i32.const 7 i32.eq
    if
      local.get $variant i32.const 3 i32.ge_u
      if i32.const -1 return end
      local.get $base_id local.get $variant i32.add return
    end
    local.get $variant i32.eqz local.get $kind call $er_ui_gallery_preview_uses_control_id_for_kind i32.const 1 i32.eq i32.and
    if local.get $base_id return end
    i32.const -1)

  (func $er_ui_gallery_preview_variant_tag (export "er_ui_gallery_preview_variant_tag") (param $kind i32) (param $variant i32) (result i32)
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

  (func (export "er_ui_gallery_preview_strategy") (param $kind i32) (result i32)
    local.get $kind call $er_ui_gallery_preview_strategy_for_kind)

  (func (export "er_ui_gallery_preview_component_kind") (param $kind i32) (result i32)
    local.get $kind call $er_ui_gallery_preview_component_kind_for_kind)

  (func (export "er_ui_gallery_badge_variant_count") (result i32) i32.const 3)
  (func (export "er_ui_gallery_button_variant_count") (result i32) i32.const 3)

  (func (export "er_ui_gallery_badge_variant_slot_w") (param $variant i32) (result f32)
    i32.const 5 local.get $variant call $er_ui_gallery_preview_variant_slot_width)

  (func (export "er_ui_gallery_button_variant_slot_w") (param $variant i32) (result f32)
    i32.const 7 local.get $variant call $er_ui_gallery_preview_variant_slot_width)

  (func (export "er_ui_gallery_badge_variant_tag") (param $variant i32) (result i32)
    i32.const 5 local.get $variant call $er_ui_gallery_preview_variant_tag)

  (func (export "er_ui_gallery_button_variant_tag") (param $variant i32) (result i32)
    i32.const 7 local.get $variant call $er_ui_gallery_preview_variant_tag)

  (func (export "er_ui_gallery_button_variant_id") (param $base_id i32) (param $variant i32) (result i32)
    i32.const 7 local.get $base_id local.get $variant call $er_ui_gallery_preview_variant_id)

  (func (export "er_ui_gallery_catalog_preview_slot_id") (param $index i32) (param $slot i32) (result i32)
    local.get $index i32.const 60 i32.ge_u local.get $slot i32.const 32 i32.ge_u i32.or
    if i32.const -1 return end
    i32.const 23000 local.get $index i32.const 32 i32.mul i32.add local.get $slot i32.add)

  (func (export "er_ui_gallery_selected_preview_slot_id") (param $index i32) (param $slot i32) (result i32)
    local.get $index i32.const 60 i32.ge_u local.get $slot i32.const 32 i32.ge_u i32.or
    if i32.const -1 return end
    i32.const 25000 local.get $index i32.const 32 i32.mul i32.add local.get $slot i32.add)

  (func (export "er_ui_gallery_preview_default_aspect_ratio_w") (result i32) i32.const 16)
  (func (export "er_ui_gallery_preview_default_aspect_ratio_h") (result i32) i32.const 9)
  (func (export "er_ui_gallery_preview_default_calendar_day") (result i32) i32.const 25)
  (func (export "er_ui_gallery_preview_default_pagination_page") (result i32) i32.const 1)
  (func (export "er_ui_gallery_preview_default_progress_value") (result f32) f32.const 0.62)
  (func (export "er_ui_gallery_preview_default_slider_value") (result f32) f32.const 0.68)
  (func (export "er_ui_gallery_preview_default_resizable_ratio") (result f32) f32.const 0.58)

  (func $er_ui_gallery_preview_build_basic (export "er_ui_gallery_preview_build_basic") (param $kind i32) (param $base i32) (param $cap i32) (param $id i32) (result i32)
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

  (func (export "er_ui_gallery_preview_build_basic_for_catalog_index") (param $index i32) (param $base i32) (param $cap i32) (result i32)
    local.get $index i32.const 60 i32.ge_u
    if i32.const 0 return end
    i32.const 66100 local.get $index i32.add i32.load8_u local.get $base local.get $cap i32.const 23000 local.get $index i32.const 32 i32.mul i32.add call $er_ui_gallery_preview_build_basic)
)
