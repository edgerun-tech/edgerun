  (func (export "er_ui_tree_codec_tree_layout_size") (result i32) i32.const 16)
  (func (export "er_ui_tree_codec_slot_layout_size") (result i32) i32.const 16)
  (func (export "er_ui_tree_codec_max_children") (result i32) i32.const 64)
  (func (export "er_ui_tree_codec_id_size") (result i32) i32.const 32)

  (func $tree_codec_magic_tree (param $ptr i32)
    local.get $ptr i32.const 0x4c555245 i32.store
    local.get $ptr i32.const 4 i32.add i32.const 0x00313030 i32.store)

  (func $tree_codec_magic_slot (param $ptr i32)
    local.get $ptr i32.const 0x53555245 i32.store
    local.get $ptr i32.const 4 i32.add i32.const 0x00313030 i32.store)

  (func $tree_codec_has_tree_magic (param $ptr i32) (param $len i32) (result i32)
    local.get $ptr i32.eqz local.get $len i32.const 16 i32.ne i32.or
    if i32.const 0 return end
    local.get $ptr i32.load i32.const 0x4c555245 i32.eq
    local.get $ptr i32.const 4 i32.add i32.load i32.const 0x00313030 i32.eq
    i32.and)

  (func $tree_codec_has_slot_magic (param $ptr i32) (param $len i32) (result i32)
    local.get $ptr i32.eqz local.get $len i32.const 16 i32.ne i32.or
    if i32.const 0 return end
    local.get $ptr i32.load i32.const 0x53555245 i32.eq
    local.get $ptr i32.const 4 i32.add i32.load i32.const 0x00313030 i32.eq
    i32.and)

  (func $er_ui_tree_codec_encode_tree_layout (export "er_ui_tree_codec_encode_tree_layout") (param $out i32) (param $cap i32) (param $axis i32) (param $gap i32) (param $padding i32) (param $child_count i32) (result i32)
    local.get $out i32.eqz local.get $cap i32.const 16 i32.lt_u i32.or
    if i32.const 0 return end
    local.get $axis i32.const 0 i32.ne local.get $axis i32.const 1 i32.ne i32.and
    if i32.const 0 return end
    local.get $child_count i32.const 64 i32.gt_u
    if i32.const 0 return end
    local.get $out i32.const 0 i32.const 16 memory.fill
    local.get $out call $tree_codec_magic_tree
    local.get $out i32.const 8 i32.add local.get $axis i32.store8
    local.get $out i32.const 10 i32.add local.get $gap call $store16
    local.get $out i32.const 12 i32.add local.get $padding call $store16
    local.get $out i32.const 14 i32.add local.get $child_count call $store16
    i32.const 16)

  (func $er_ui_tree_codec_is_tree_layout_body (export "er_ui_tree_codec_is_tree_layout_body") (param $ptr i32) (param $len i32) (result i32)
    local.get $ptr local.get $len call $tree_codec_has_tree_magic
    local.get $ptr i32.const 9 i32.add i32.load8_u i32.eqz
    i32.and
    local.get $ptr i32.const 8 i32.add i32.load8_u i32.const 1 i32.le_u
    i32.and)

  (func (export "er_ui_tree_codec_tree_axis") (param $ptr i32) (param $len i32) (result i32)
    local.get $ptr local.get $len call $er_ui_tree_codec_is_tree_layout_body
    i32.eqz
    if i32.const -1 return end
    local.get $ptr i32.const 8 i32.add i32.load8_u)

  (func (export "er_ui_tree_codec_tree_gap") (param $ptr i32) (param $len i32) (result i32)
    local.get $ptr local.get $len call $er_ui_tree_codec_is_tree_layout_body
    i32.eqz
    if i32.const -1 return end
    local.get $ptr i32.const 10 i32.add call $load16)

  (func (export "er_ui_tree_codec_tree_padding") (param $ptr i32) (param $len i32) (result i32)
    local.get $ptr local.get $len call $er_ui_tree_codec_is_tree_layout_body
    i32.eqz
    if i32.const -1 return end
    local.get $ptr i32.const 12 i32.add call $load16)

  (func $er_ui_tree_codec_tree_child_count (export "er_ui_tree_codec_tree_child_count") (param $ptr i32) (param $len i32) (result i32)
    local.get $ptr local.get $len call $er_ui_tree_codec_is_tree_layout_body
    i32.eqz
    if i32.const -1 return end
    local.get $ptr i32.const 14 i32.add call $load16)

  (func (export "er_ui_tree_codec_decode_tree_layout") (param $ptr i32) (param $len i32) (param $out i32) (result i32)
    local.get $out i32.eqz
    if i32.const 0 return end
    local.get $ptr local.get $len call $er_ui_tree_codec_is_tree_layout_body
    i32.eqz
    if i32.const 0 return end
    local.get $out local.get $ptr i32.const 8 i32.add i32.load8_u i32.store
    local.get $out i32.const 4 i32.add local.get $ptr i32.const 10 i32.add call $load16 i32.store
    local.get $out i32.const 8 i32.add local.get $ptr i32.const 12 i32.add call $load16 i32.store
    local.get $out i32.const 12 i32.add local.get $ptr i32.const 14 i32.add call $load16 i32.store
    i32.const 1)

  (func $er_ui_tree_codec_encode_slot_layout (export "er_ui_tree_codec_encode_slot_layout") (param $out i32) (param $cap i32) (param $id i32) (result i32)
    local.get $out i32.eqz local.get $cap i32.const 16 i32.lt_u i32.or
    if i32.const 0 return end
    local.get $out i32.const 0 i32.const 16 memory.fill
    local.get $out call $tree_codec_magic_slot
    local.get $out i32.const 8 i32.add local.get $id call $store32
    i32.const 16)

  (func $er_ui_tree_codec_is_slot_layout_body (export "er_ui_tree_codec_is_slot_layout_body") (param $ptr i32) (param $len i32) (result i32)
    local.get $ptr local.get $len call $tree_codec_has_slot_magic)

  (func (export "er_ui_tree_codec_decode_slot_layout") (param $ptr i32) (param $len i32) (result i32)
    local.get $ptr local.get $len call $er_ui_tree_codec_is_slot_layout_body
    i32.eqz
    if i32.const -1 return end
    local.get $ptr i32.const 8 i32.add i32.load)

  (func $er_ui_tree_codec_same_id (export "er_ui_tree_codec_same_id") (param $left i32) (param $right i32) (result i32)
    (local $i i32)
    local.get $left i32.eqz local.get $right i32.eqz i32.or
    if i32.const 0 return end
    block $done
      loop $loop
        local.get $i i32.const 32 i32.ge_u
        br_if $done
        local.get $left local.get $i i32.add i32.load8_u
        local.get $right local.get $i i32.add i32.load8_u
        i32.ne
        if i32.const 0 return end
        local.get $i i32.const 1 i32.add local.set $i
        br $loop
      end
    end
    i32.const 1)

  (func (export "er_ui_object_id_size") (result i32) i32.const 32)
  (func (export "er_ui_object_child_size") (result i32) i32.const 84)
  (func (export "er_ui_object_kind_bytes") (result i32) i32.const 1)
  (func (export "er_ui_object_kind_tree") (result i32) i32.const 2)
  (func (export "er_ui_object_kind_receipt") (result i32) i32.const 4)

  (func $er_ui_object_id_nonzero (param $ptr i32) (result i32)
    (local $i i32)
    local.get $ptr i32.eqz
    if i32.const 0 return end
    block $done
      loop $loop
        local.get $i i32.const 32 i32.ge_u
        br_if $done
        local.get $ptr local.get $i i32.add i32.load8_u
        i32.eqz
        i32.eqz
        if i32.const 1 return end
        local.get $i i32.const 1 i32.add local.set $i
        br $loop
      end
    end
    i32.const 0)

  (func $er_ui_object_kind_valid (param $kind i32) (result i32)
    local.get $kind i32.const 1 i32.eq
    local.get $kind i32.const 2 i32.eq
    i32.or
    local.get $kind i32.const 4 i32.eq
    i32.or)

  (func (export "er_ui_object_child_encode") (param $out i32) (param $cap i32) (param $object_id i32) (param $logical_offset i64) (param $logical_len i64) (param $kind i32) (param $requirements_hash i32) (result i32)
    local.get $out i32.eqz
    local.get $cap i32.const 84 i32.lt_u i32.or
    local.get $object_id call $er_ui_object_id_nonzero i32.eqz i32.or
    local.get $requirements_hash call $er_ui_object_id_nonzero i32.eqz i32.or
    local.get $logical_len i64.eqz i32.or
    local.get $kind call $er_ui_object_kind_valid i32.eqz i32.or
    if i32.const 0 return end
    local.get $out i32.const 0 i32.const 84 memory.fill
    local.get $out local.get $object_id i32.const 32 memory.copy
    local.get $out i32.const 32 i32.add local.get $logical_offset i64.store
    local.get $out i32.const 40 i32.add local.get $logical_len i64.store
    local.get $out i32.const 48 i32.add local.get $kind call $store16
    local.get $out i32.const 52 i32.add local.get $requirements_hash i32.const 32 memory.copy
    i32.const 84)

  (func (export "er_ui_object_child_logical_offset") (param $child i32) (result i64)
    local.get $child i32.eqz
    if i64.const -1 return end
    local.get $child i32.const 32 i32.add i64.load)

  (func (export "er_ui_object_child_logical_len") (param $child i32) (result i64)
    local.get $child i32.eqz
    if i64.const -1 return end
    local.get $child i32.const 40 i32.add i64.load)

  (func (export "er_ui_object_child_kind") (param $child i32) (result i32)
    local.get $child i32.eqz
    if i32.const -1 return end
    local.get $child i32.const 48 i32.add call $load16)

  (func (export "er_ui_object_child_reserved") (param $child i32) (result i32)
    local.get $child i32.eqz
    if i32.const -1 return end
    local.get $child i32.const 50 i32.add call $load16)

  (func $er_ui_object_child_valid (export "er_ui_object_child_valid") (param $child i32) (param $expected_offset i64) (result i32)
    local.get $child i32.eqz
    if i32.const 0 return end
    local.get $child call $er_ui_object_id_nonzero
    local.get $child i32.const 52 i32.add call $er_ui_object_id_nonzero
    i32.and
    local.get $child i32.const 40 i32.add i64.load i64.eqz i32.eqz
    i32.and
    local.get $child i32.const 32 i32.add i64.load local.get $expected_offset i64.eq
    i32.and
    local.get $child i32.const 50 i32.add call $load16 i32.eqz
    i32.and
    local.get $child i32.const 48 i32.add call $load16 call $er_ui_object_kind_valid
    i32.and)

  (func (export "er_ui_object_header_size") (result i32) i32.const 148)
  (func (export "er_ui_object_requirements_size") (result i32) i32.const 28)
  (func (export "er_ui_object_owner_size") (result i32) i32.const 36)
  (func (export "er_ui_object_envelope_size") (result i32) i32.const 76)
  (func (export "er_ui_object_max_owners") (result i32) i32.const 16)
  (func (export "er_ui_object_max_envelopes") (result i32) i32.const 16)
  (func (export "er_ui_object_max_children") (result i32) i32.const 65536)
  (func (export "er_ui_object_header_reserved_start") (result i32) i32.const 132)
  (func (export "er_ui_object_header_reserved_size") (result i32) i32.const 16)

  (func $er_ui_object_has_magic (param $ptr i32) (result i32)
    local.get $ptr i32.eqz
    if i32.const 0 return end
    local.get $ptr i32.load i32.const 0x424f5245 i32.eq
    local.get $ptr i32.const 4 i32.add i32.load i32.const 0x3130304a i32.eq
    i32.and)

  (func $er_ui_object_reserved_zero (param $header i32) (result i32)
    (local $i i32)
    local.get $header i32.eqz
    if i32.const 0 return end
    block $done
      loop $loop
        local.get $i i32.const 16 i32.ge_u
        br_if $done
        local.get $header i32.const 132 i32.add local.get $i i32.add i32.load8_u
        if i32.const 0 return end
        local.get $i i32.const 1 i32.add local.set $i
        br $loop
      end
    end
    i32.const 1)

  (func $er_ui_object_canonical_size (export "er_ui_object_canonical_size") (param $kind i32) (param $body_len i32) (param $owners i32) (param $envelopes i32) (param $children i32) (result i32)
    local.get $kind call $er_ui_object_kind_valid i32.eqz
    local.get $owners i32.const 16 i32.gt_u i32.or
    local.get $envelopes i32.const 16 i32.gt_u i32.or
    local.get $children i32.const 65536 i32.gt_u i32.or
    if i32.const -1 return end
    local.get $kind i32.const 2 i32.ne local.get $children i32.const 0 i32.ne i32.and
    if i32.const -1 return end
    local.get $kind i32.const 2 i32.eq local.get $body_len i32.const 0 i32.ne i32.and
    if i32.const -1 return end
    i32.const 148
    local.get $owners i32.const 36 i32.mul i32.add
    local.get $envelopes i32.const 76 i32.mul i32.add
    local.get $children i32.const 84 i32.mul i32.add
    local.get $body_len i32.add)

  (func $er_ui_object_header_kind (export "er_ui_object_header_kind") (param $header i32) (result i32)
    local.get $header i32.eqz
    if i32.const -1 return end
    local.get $header i32.const 10 i32.add call $load16)

  (func (export "er_ui_object_header_logical_len") (param $header i32) (result i64)
    local.get $header i32.eqz
    if i64.const -1 return end
    local.get $header i32.const 16 i32.add i64.load)

  (func (export "er_ui_object_header_owner_count") (param $header i32) (result i32)
    local.get $header i32.eqz
    if i32.const -1 return end
    local.get $header i32.const 24 i32.add call $load16)

  (func (export "er_ui_object_header_envelope_count") (param $header i32) (result i32)
    local.get $header i32.eqz
    if i32.const -1 return end
    local.get $header i32.const 26 i32.add call $load16)

  (func $er_ui_object_header_child_count (export "er_ui_object_header_child_count") (param $header i32) (result i32)
    local.get $header i32.eqz
    if i32.const -1 return end
    local.get $header i32.const 28 i32.add i32.load)

  (func (export "er_ui_object_header_body_len") (param $header i32) (result i64)
    local.get $header i32.eqz
    if i64.const -1 return end
    local.get $header i32.const 32 i32.add i64.load)

  (func (export "er_ui_object_owners_offset") (result i32) i32.const 148)

  (func (export "er_ui_object_envelopes_offset") (param $owner_count i32) (result i32)
    i32.const 148 local.get $owner_count i32.const 36 i32.mul i32.add)

  (func $er_ui_object_children_offset (export "er_ui_object_children_offset") (param $owner_count i32) (param $envelope_count i32) (result i32)
    i32.const 148
    local.get $owner_count i32.const 36 i32.mul i32.add
    local.get $envelope_count i32.const 76 i32.mul i32.add)

  (func $er_ui_object_body_offset (export "er_ui_object_body_offset") (param $owner_count i32) (param $envelope_count i32) (param $child_count i32) (result i32)
    i32.const 148
    local.get $owner_count i32.const 36 i32.mul i32.add
    local.get $envelope_count i32.const 76 i32.mul i32.add
    local.get $child_count i32.const 84 i32.mul i32.add)

  (func $er_ui_object_header_lite_valid (export "er_ui_object_header_lite_valid") (param $canonical i32) (param $len i32) (result i32)
    (local $kind i32) (local $body_len_i32 i32) (local $total i32)
    local.get $canonical i32.eqz local.get $len i32.const 148 i32.lt_u i32.or
    if i32.const 0 return end
    local.get $canonical call $er_ui_object_has_magic
    local.get $canonical i32.const 8 i32.add call $load16 i32.const 1 i32.eq
    i32.and
    local.get $canonical call $er_ui_object_reserved_zero
    i32.and
    i32.eqz
    if i32.const 0 return end
    local.get $canonical i32.const 10 i32.add call $load16 local.tee $kind call $er_ui_object_kind_valid i32.eqz
    if i32.const 0 return end
    local.get $canonical i32.const 32 i32.add i64.load i32.wrap_i64 local.set $body_len_i32
    local.get $kind local.get $body_len_i32 local.get $canonical i32.const 24 i32.add call $load16 local.get $canonical i32.const 26 i32.add call $load16 local.get $canonical i32.const 28 i32.add i32.load call $er_ui_object_canonical_size local.tee $total
    local.get $len i32.eq
    local.get $total i32.const 0 i32.ge_s
    i32.and)

  (func (export "er_ui_object_body_ptr") (param $canonical i32) (param $len i32) (result i32)
    (local $off i32)
    local.get $canonical local.get $len call $er_ui_object_header_lite_valid
    i32.eqz
    if i32.const 0 return end
    local.get $canonical i32.const 24 i32.add call $load16
    local.get $canonical i32.const 26 i32.add call $load16
    local.get $canonical i32.const 28 i32.add i32.load
    call $er_ui_object_body_offset local.set $off
    local.get $canonical local.get $off i32.add)

  (func $er_ui_object_children_ptr (export "er_ui_object_children_ptr") (param $canonical i32) (param $len i32) (result i32)
    local.get $canonical local.get $len call $er_ui_object_header_lite_valid
    i32.eqz
    if i32.const 0 return end
    local.get $canonical
    local.get $canonical i32.const 24 i32.add call $load16
    local.get $canonical i32.const 26 i32.add call $load16
    call $er_ui_object_children_offset
    i32.add)

  (func $er_ui_object_child_ptr (export "er_ui_object_child_ptr") (param $canonical i32) (param $len i32) (param $index i32) (result i32)
    (local $children i32) (local $count i32) (local $i i32) (local $expected i64) (local $child i32)
    local.get $canonical local.get $len call $er_ui_object_header_lite_valid
    i32.eqz
    if i32.const 0 return end
    local.get $canonical i32.const 28 i32.add i32.load local.tee $count
    local.get $index i32.le_u
    if i32.const 0 return end
    local.get $canonical local.get $len call $er_ui_object_children_ptr local.set $children
    block $done
      loop $loop
        local.get $i local.get $index i32.ge_u
        br_if $done
        local.get $children local.get $i i32.const 84 i32.mul i32.add local.tee $child local.get $expected call $er_ui_object_child_valid i32.eqz
        if i32.const 0 return end
        local.get $expected local.get $child i32.const 40 i32.add i64.load i64.add local.set $expected
        local.get $i i32.const 1 i32.add local.set $i
        br $loop
      end
    end
    local.get $children local.get $index i32.const 84 i32.mul i32.add local.tee $child local.get $expected call $er_ui_object_child_valid
    if local.get $child return end
    i32.const 0)
