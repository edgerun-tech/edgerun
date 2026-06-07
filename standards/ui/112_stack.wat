(func (export "er_ui_stack_default_gap") (result i32) i32.const 8)
  (func (export "er_ui_stack_default_padding") (result i32) i32.const 0)
  (func $er_ui_stack_axis_column (export "er_ui_stack_axis_column") (result i32) i32.const 0)
  (func (export "er_ui_stack_axis_row") (result i32) i32.const 1)

  (func $er_ui_stack_layout_axis (export "er_ui_stack_layout_axis") (param $axis i32) (result i32)
    local.get $axis i32.const 1 i32.eq
    if (result i32)
      i32.const 0
    else
      i32.const 1
    end)

  (func (export "er_ui_stack_constraints_from_bounds") (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out i32.const 2 i32.store
    local.get $out i32.const 4 i32.add local.get $bounds i32.const 8 i32.add f32.load f32.store
    local.get $out i32.const 8 i32.add i32.const 2 i32.store
    local.get $out i32.const 12 i32.add local.get $bounds i32.const 12 i32.add f32.load f32.store
    local.get $out i32.const 16 i32.add i32.const 1 i32.store
    i32.const 1)

  (func (export "er_ui_stack_child_constraints_for") (param $axis i32) (param $padding f32) (param $constraints i32) (param $out i32) (result i32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $padding i32.const 124900 call $er_ui_layout_insets_uniform drop
    local.get $constraints i32.const 124900 i32.const 124920 call $er_ui_layout_constraints_inner drop
    local.get $axis i32.const 1 i32.eq
    if
      local.get $out i32.const 0 i32.store
      local.get $out i32.const 4 i32.add f32.const 0 f32.store
      local.get $out i32.const 8 i32.add i32.const 124928 i32.load i32.store
      local.get $out i32.const 12 i32.add i32.const 124932 f32.load f32.store
      local.get $out i32.const 16 i32.add local.get $constraints i32.const 16 i32.add i32.load i32.store
      i32.const 1
      return
    end
    local.get $out i32.const 124920 i32.load i32.store
    local.get $out i32.const 4 i32.add i32.const 124924 f32.load f32.store
    local.get $out i32.const 8 i32.add i32.const 0 i32.store
    local.get $out i32.const 12 i32.add f32.const 0 f32.store
    local.get $out i32.const 16 i32.add local.get $constraints i32.const 16 i32.add i32.load i32.store
    i32.const 1)

  (func (export "er_ui_stack_layout_options_for") (param $axis i32) (param $gap f32) (param $padding f32) (param $cross_align i32) (param $out i32) (result i32)
    local.get $out i32.eqz
    if i32.const 0 return end
    local.get $padding i32.const 124960 call $er_ui_layout_insets_uniform drop
    local.get $out local.get $axis call $er_ui_stack_layout_axis i32.store
    local.get $out i32.const 4 i32.add local.get $gap f32.store
    local.get $out i32.const 8 i32.add i32.const 124960 f32.load f32.store
    local.get $out i32.const 12 i32.add i32.const 124964 f32.load f32.store
    local.get $out i32.const 16 i32.add i32.const 124968 f32.load f32.store
    local.get $out i32.const 20 i32.add i32.const 124972 f32.load f32.store
    local.get $out i32.const 24 i32.add local.get $cross_align i32.store
    i32.const 1)
