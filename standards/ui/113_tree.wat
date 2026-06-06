  (func (export "er_ui_tree_descriptor_unknown") (result i32) i32.const 0)
  (func (export "er_ui_tree_descriptor_stack") (result i32) i32.const 1)
  (func (export "er_ui_tree_descriptor_slot") (result i32) i32.const 2)

  (func $er_ui_tree_descriptor_kind (export "er_ui_tree_descriptor_kind") (param $body i32) (param $body_len i32) (result i32)
    local.get $body local.get $body_len call $er_ui_tree_codec_is_tree_layout_body
    if i32.const 1 return end
    local.get $body local.get $body_len call $er_ui_tree_codec_is_slot_layout_body
    if i32.const 2 return end
    i32.const 0)

  (func (export "er_ui_tree_resolved_count_valid") (param $descriptor_body i32) (param $descriptor_len i32) (param $resolved_count i32) (result i32)
    (local $kind i32)
    local.get $descriptor_body local.get $descriptor_len call $er_ui_tree_descriptor_kind
    local.tee $kind
    i32.const 1 i32.eq
    if
      local.get $descriptor_body local.get $descriptor_len call $er_ui_tree_codec_tree_child_count i32.const 1 i32.add local.get $resolved_count i32.eq
      return
    end
    local.get $kind i32.const 2 i32.eq
    if
      local.get $resolved_count i32.const 2 i32.eq
      return
    end
    i32.const 0)

  (func (export "er_ui_slot_tree_validate_shape") (param $tree i32) (param $tree_len i32) (param $descriptor_id i32) (param $child_id i32) (result i32)
    (local $child0 i32) (local $child1 i32)
    local.get $tree local.get $tree_len call $er_ui_object_header_lite_valid
    i32.eqz
    if i32.const 0 return end
    local.get $tree call $er_ui_object_header_kind i32.const 2 i32.ne
    local.get $tree call $er_ui_object_header_child_count i32.const 2 i32.ne i32.or
    if i32.const 0 return end
    local.get $tree local.get $tree_len i32.const 0 call $er_ui_object_child_ptr local.tee $child0 i32.eqz
    if i32.const 0 return end
    local.get $tree local.get $tree_len i32.const 1 call $er_ui_object_child_ptr local.tee $child1 i32.eqz
    if i32.const 0 return end
    local.get $child0 local.get $descriptor_id call $er_ui_tree_codec_same_id
    local.get $child1 local.get $child_id call $er_ui_tree_codec_same_id
    i32.and)

  (func (export "er_ui_stack_tree_validate_shape") (param $tree i32) (param $tree_len i32) (param $descriptor_body i32) (param $descriptor_len i32) (param $descriptor_id i32) (result i32)
    (local $expected_count i32) (local $child0 i32)
    local.get $descriptor_body local.get $descriptor_len call $er_ui_tree_codec_is_tree_layout_body
    i32.eqz
    if i32.const 0 return end
    local.get $descriptor_body local.get $descriptor_len call $er_ui_tree_codec_tree_child_count i32.const 1 i32.add local.set $expected_count
    local.get $tree local.get $tree_len call $er_ui_object_header_lite_valid
    i32.eqz
    if i32.const 0 return end
    local.get $tree call $er_ui_object_header_kind i32.const 2 i32.ne
    local.get $tree call $er_ui_object_header_child_count local.get $expected_count i32.ne i32.or
    if i32.const 0 return end
    local.get $tree local.get $tree_len i32.const 0 call $er_ui_object_child_ptr local.tee $child0 i32.eqz
    if i32.const 0 return end
    local.get $child0 local.get $descriptor_id call $er_ui_tree_codec_same_id)
