  (func (export "er_ui_semantic_kind_identity") (result i32) i32.const 0)
  (func (export "er_ui_semantic_kind_metric") (result i32) i32.const 1)
  (func (export "er_ui_semantic_kind_resource") (result i32) i32.const 2)
  (func (export "er_ui_semantic_kind_path") (result i32) i32.const 3)
  (func (export "er_ui_semantic_kind_event") (result i32) i32.const 4)
  (func (export "er_ui_semantic_kind_action") (result i32) i32.const 5)
  (func (export "er_ui_semantic_kind_artifact") (result i32) i32.const 6)
  (func (export "er_ui_semantic_kind_warning") (result i32) i32.const 7)
  (func (export "er_ui_semantic_kind_dependency") (result i32) i32.const 8)
  (func (export "er_ui_semantic_kind_timeline") (result i32) i32.const 9)
  (func (export "er_ui_semantic_importance_primary") (result i32) i32.const 0)
  (func (export "er_ui_semantic_importance_normal") (result i32) i32.const 1)
  (func (export "er_ui_semantic_importance_support") (result i32) i32.const 2)
  (func (export "er_ui_semantic_importance_background") (result i32) i32.const 3)
  (func (export "er_ui_semantic_state_neutral") (result i32) i32.const 0)
  (func (export "er_ui_semantic_state_active") (result i32) i32.const 1)
  (func (export "er_ui_semantic_state_good") (result i32) i32.const 2)
  (func (export "er_ui_semantic_state_warning") (result i32) i32.const 3)
  (func (export "er_ui_semantic_state_bad") (result i32) i32.const 4)
  (func (export "er_ui_semantic_state_blocked") (result i32) i32.const 5)
  (func (export "er_ui_semantic_state_private") (result i32) i32.const 6)
  (func (export "er_ui_semantic_state_pending") (result i32) i32.const 7)
  (func (export "er_ui_semantic_mode_overview") (result i32) i32.const 0)
  (func (export "er_ui_semantic_mode_schedule") (result i32) i32.const 3)
  (func (export "er_ui_semantic_focus_general") (result i32) i32.const 0)
  (func (export "er_ui_semantic_focus_resources") (result i32) i32.const 1)
  (func (export "er_ui_semantic_focus_paths") (result i32) i32.const 2)
  (func (export "er_ui_semantic_focus_dependencies") (result i32) i32.const 3)
  (func (export "er_ui_semantic_focus_privacy") (result i32) i32.const 4)
  (func (export "er_ui_semantic_focus_errors") (result i32) i32.const 5)
  (func (export "er_ui_semantic_density_compact") (result i32) i32.const 0)
  (func (export "er_ui_semantic_density_normal") (result i32) i32.const 1)
  (func (export "er_ui_semantic_density_expanded") (result i32) i32.const 2)

  (func (export "er_ui_semantic_control_id") (param $id i32) (result i32)
    local.get $id i32.eqz
    if i32.const -1 return end
    local.get $id)

  (func (export "er_ui_semantic_promotes") (param $kind i32) (param $importance i32) (param $state i32) (param $focus i32) (result i32)
    local.get $importance i32.eqz
    if i32.const 1 return end
    local.get $importance i32.const 3 i32.eq
    if i32.const 0 return end
    local.get $focus i32.const 1 i32.eq local.get $kind i32.const 2 i32.eq i32.and
    local.get $focus i32.const 2 i32.eq local.get $kind i32.const 3 i32.eq i32.and i32.or
    local.get $focus i32.const 3 i32.eq local.get $kind i32.const 8 i32.eq i32.and i32.or
    local.get $focus i32.const 4 i32.eq local.get $state i32.const 6 i32.eq i32.and i32.or
    local.get $focus i32.const 5 i32.eq local.get $state i32.const 4 i32.eq local.get $state i32.const 5 i32.eq i32.or local.get $kind i32.const 7 i32.eq i32.or i32.and i32.or)

  (func (export "er_ui_semantic_primary_height") (param $density i32) (result f32)
    local.get $density i32.eqz
    if f32.const 88 return end
    local.get $density i32.const 2 i32.eq
    if f32.const 122 return end
    f32.const 104)

  (func (export "er_ui_semantic_row_height") (param $density i32) (result f32)
    local.get $density i32.eqz
    if f32.const 36 return end
    local.get $density i32.const 2 i32.eq
    if f32.const 54 return end
    f32.const 44)

  (func (export "er_ui_semantic_gap") (param $density i32) (result f32)
    local.get $density i32.eqz
    if f32.const 6 return end
    local.get $density i32.const 2 i32.eq
    if f32.const 14 return end
    f32.const 10)

  (func (export "er_ui_semantic_header_gap") (param $density i32) (result f32)
    local.get $density i32.eqz
    if f32.const 8 return end
    local.get $density i32.const 2 i32.eq
    if f32.const 16 return end
    f32.const 12)

  (func (export "er_ui_semantic_badge_variant") (param $state i32) (result i32)
    local.get $state i32.const 1 i32.eq local.get $state i32.const 2 i32.eq i32.or
    if i32.const 1 return end
    local.get $state i32.const 3 i32.eq local.get $state i32.const 4 i32.eq i32.or local.get $state i32.const 5 i32.eq i32.or
    if i32.const 0 return end
    i32.const 3)

  (func (export "er_ui_semantic_badge_bounds") (param $bounds i32) (param $label_len i32) (param $out i32) (result i32)
    (local $desired f32) (local $width f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $label_len f32.convert_i32_u f32.const 7.4 f32.mul f32.const 28 f32.add local.set $desired
    f32.const 54 local.get $desired call $max_f32 f32.const 54 local.get $bounds i32.const 8 i32.add f32.load f32.const 20 f32.sub call $max_f32 call $min_f32 local.set $width
    local.get $out
    local.get $bounds f32.load local.get $bounds i32.const 8 i32.add f32.load f32.add local.get $width f32.sub f32.const 12 f32.sub
    local.get $bounds i32.const 4 i32.add f32.load f32.const 12 f32.add
    local.get $width
    f32.const 22
    call $rect_store)

  (func (export "er_ui_semantic_row_progress_bounds") (param $bounds i32) (param $out i32) (result i32)
    (local $bar_w f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    f32.const 70 f32.const 30 local.get $bounds i32.const 8 i32.add f32.load f32.const 0.22 f32.mul call $max_f32 call $min_f32 local.set $bar_w
    local.get $out
    local.get $bounds f32.load local.get $bounds i32.const 8 i32.add f32.load f32.add local.get $bar_w f32.sub f32.const 10 f32.sub
    local.get $bounds i32.const 4 i32.add f32.load local.get $bounds i32.const 12 i32.add f32.load f32.add f32.const 13 f32.sub
    local.get $bar_w
    f32.const 6
    call $rect_store)

  (func (export "er_ui_semantic_action_button_row") (param $kind i32) (param $id i32) (param $mode i32) (result i32)
    local.get $kind i32.const 5 i32.eq local.get $id i32.const 0 i32.ne i32.and local.get $mode i32.const 3 i32.eq i32.and)

  (func (export "er_ui_semantic_primary_slots_for_count") (param $bounds i32) (param $promoted_count i32) (result i32)
    (local $max_slots i32)
    local.get $bounds i32.eqz local.get $promoted_count i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 12 i32.add f32.load f32.const 150 f32.lt
    if i32.const 0 return end
    local.get $bounds i32.const 8 i32.add f32.load f32.const 720 f32.ge
    if
      i32.const 3 local.set $max_slots
    else
      local.get $bounds i32.const 8 i32.add f32.load f32.const 440 f32.ge
      if i32.const 2 local.set $max_slots else i32.const 1 local.set $max_slots end
    end
    local.get $promoted_count local.get $max_slots i32.lt_u
    if (result i32) local.get $promoted_count else local.get $max_slots end)
