(func (export "er_ui_patch_encode_bool") (param $out i32) (param $cap i32) (param $kind i32) (param $component_id i32) (param $value i32) (result i32)
    local.get $cap
    i32.const 3
    i32.lt_u
    if
      i32.const 0
      return
    end
    local.get $out
    local.get $kind
    i32.store8
    local.get $out
    i32.const 1
    i32.add
    local.get $component_id
    i32.store8
    local.get $out
    i32.const 2
    i32.add
    local.get $value
    i32.const 0
    i32.ne
    i32.store8
    i32.const 3)

  (func (export "er_ui_patch_encode_u16") (param $out i32) (param $cap i32) (param $kind i32) (param $component_id i32) (param $value i32) (result i32)
    local.get $cap
    i32.const 4
    i32.lt_u
    if
      i32.const 0
      return
    end
    local.get $out
    local.get $kind
    i32.store8
    local.get $out
    i32.const 1
    i32.add
    local.get $component_id
    i32.store8
    local.get $out
    i32.const 2
    i32.add
    local.get $value
    call $store16
    i32.const 4)

  (func (export "er_ui_patch_encode_f32") (param $out i32) (param $cap i32) (param $kind i32) (param $component_id i32) (param $value f32) (result i32)
    local.get $cap
    i32.const 6
    i32.lt_u
    if
      i32.const 0
      return
    end
    local.get $out
    local.get $kind
    i32.store8
    local.get $out
    i32.const 1
    i32.add
    local.get $component_id
    i32.store8
    local.get $out
    i32.const 2
    i32.add
    local.get $value
    f32.store
    i32.const 6)

  (func (export "er_ui_patch_encode_string") (param $out i32) (param $cap i32) (param $kind i32) (param $component_id i32) (param $src i32) (param $src_len i32) (result i32)
    local.get $src_len
    i32.const 255
    i32.gt_u
    if
      i32.const 0
      return
    end
    local.get $cap
    local.get $src_len
    i32.const 3
    i32.add
    i32.lt_u
    if
      i32.const 0
      return
    end
    local.get $out
    local.get $kind
    i32.store8
    local.get $out
    i32.const 1
    i32.add
    local.get $component_id
    i32.store8
    local.get $out
    i32.const 2
    i32.add
    local.get $src_len
    i32.store8
    local.get $out
    i32.const 3
    i32.add
    local.get $src
    local.get $src_len
    call $copy
    local.get $src_len
    i32.const 3
    i32.add)

  (func $er_ui_patch_encode_two_strings (export "er_ui_patch_encode_two_strings") (param $out i32) (param $cap i32) (param $kind i32) (param $component_id i32) (param $a_ptr i32) (param $a_len i32) (param $b_ptr i32) (param $b_len i32) (result i32)
    local.get $a_len
    i32.const 255
    i32.gt_u
    local.get $b_len
    i32.const 255
    i32.gt_u
    i32.or
    if
      i32.const 0
      return
    end
    local.get $cap
    local.get $a_len
    local.get $b_len
    i32.add
    i32.const 4
    i32.add
    i32.lt_u
    if
      i32.const 0
      return
    end
    local.get $out
    local.get $kind
    i32.store8
    local.get $out
    i32.const 1
    i32.add
    local.get $component_id
    i32.store8
    local.get $out
    i32.const 2
    i32.add
    local.get $a_len
    i32.store8
    local.get $out
    i32.const 3
    i32.add
    local.get $a_ptr
    local.get $a_len
    call $copy
    local.get $out
    i32.const 3
    i32.add
    local.get $a_len
    i32.add
    local.get $b_len
    i32.store8
    local.get $out
    i32.const 4
    i32.add
    local.get $a_len
    i32.add
    local.get $b_ptr
    local.get $b_len
    call $copy
    local.get $a_len
    local.get $b_len
    i32.add
    i32.const 4
    i32.add)

  (func (export "er_ui_patch_encode_two_strings_bool") (param $out i32) (param $cap i32) (param $kind i32) (param $component_id i32) (param $a_ptr i32) (param $a_len i32) (param $b_ptr i32) (param $b_len i32) (param $flag i32) (result i32)
    (local $n i32)
    local.get $cap
    local.get $a_len
    local.get $b_len
    i32.add
    i32.const 5
    i32.add
    i32.lt_u
    if
      i32.const 0
      return
    end
    local.get $out
    local.get $cap
    local.get $kind
    local.get $component_id
    local.get $a_ptr
    local.get $a_len
    local.get $b_ptr
    local.get $b_len
    call $er_ui_patch_encode_two_strings
    local.tee $n
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $cap
    local.get $n
    i32.const 1
    i32.add
    i32.lt_u
    if
      i32.const 0
      return
    end
    local.get $out
    local.get $n
    i32.add
    local.get $flag
    i32.const 0
    i32.ne
    i32.store8
    local.get $n
    i32.const 1
    i32.add)

  (func (export "er_ui_patch_encode_color") (param $out i32) (param $cap i32) (param $kind i32) (param $component_id i32) (param $rgba i32) (result i32)
    local.get $cap
    i32.const 6
    i32.lt_u
    if
      i32.const 0
      return
    end
    local.get $out
    local.get $kind
    i32.store8
    local.get $out
    i32.const 1
    i32.add
    local.get $component_id
    i32.store8
    local.get $out
    i32.const 2
    i32.add
    local.get $rgba
    call $store32
    i32.const 6)

  (func $er_ui_patch_kind (export "er_ui_patch_kind") (param $patch i32) (param $len i32) (result i32)
    local.get $len
    i32.const 2
    i32.lt_u
    if
      i32.const -1
      return
    end
    local.get $patch
    i32.load8_u)

  (func $er_ui_patch_component_id (export "er_ui_patch_component_id") (param $patch i32) (param $len i32) (result i32)
    local.get $len
    i32.const 2
    i32.lt_u
    if
      i32.const -1
      return
    end
    local.get $patch
    i32.const 1
    i32.add
    i32.load8_u)

  (func $patch_target_matches (param $base i32) (param $len i32) (param $index i32) (param $patch i32) (param $patch_len i32) (result i32)
    (local $kind i32)
    (local $id i32)
    local.get $patch_len
    i32.const 2
    i32.lt_u
    if
      i32.const 0
      return
    end
    local.get $base
    local.get $len
    call $er_ui_validate
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $base
    local.get $len
    local.get $index
    call $er_ui_record_kind
    local.tee $kind
    i32.const -1
    i32.eq
    if
      i32.const 0
      return
    end
    local.get $base
    local.get $len
    local.get $index
    call $er_ui_record_id
    local.set $id
    local.get $patch
    local.get $patch_len
    call $er_ui_patch_kind
    local.get $kind
    i32.eq
    local.get $patch
    local.get $patch_len
    call $er_ui_patch_component_id
    local.get $id
    i32.const 255
    i32.and
    i32.eq
    i32.and)

  (func (export "er_ui_patch_apply_one_string") (param $base i32) (param $cap i32) (param $current_end i32) (param $index i32) (param $src i32) (param $src_len i32) (result i32)
    (local $n i32)
    (local $offset i32)
    (local $r i32)
    local.get $current_end
    local.get $base
    i32.lt_u
    local.get $current_end
    local.get $base
    local.get $cap
    i32.add
    i32.gt_u
    i32.or
    if
      i32.const 0
      return
    end
    local.get $base
    local.get $current_end
    local.get $base
    i32.sub
    call $er_ui_validate
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $base
    i32.const 16
    i32.add
    call $load16
    local.set $n
    local.get $index
    local.get $n
    i32.ge_u
    if
      i32.const 0
      return
    end
    local.get $src_len
    i32.const 65535
    i32.gt_u
    if
      i32.const 0
      return
    end
    local.get $current_end
    local.get $src_len
    i32.add
    local.get $base
    local.get $cap
    i32.add
    i32.gt_u
    if
      i32.const 0
      return
    end
    local.get $current_end
    i32.const 20
    local.get $base
    i32.add
    local.get $n
    i32.const 16
    i32.mul
    i32.add
    i32.sub
    local.set $offset
    local.get $current_end
    local.get $src
    local.get $src_len
    call $copy
    local.get $base
    local.get $index
    call $record_at
    local.set $r
    local.get $r
    i32.const 8
    i32.add
    local.get $offset
    call $store16
    local.get $r
    i32.const 10
    i32.add
    local.get $src_len
    call $store16
    local.get $base
    i32.const 4
    i32.add
    local.get $current_end
    local.get $src_len
    i32.add
    local.get $base
    i32.sub
    call $store32
    local.get $current_end
    local.get $src_len
    i32.add)

  (func $er_ui_patch_apply_second_bool (export "er_ui_patch_apply_second_bool") (param $base i32) (param $len i32) (param $index i32) (param $patch i32) (param $patch_len i32) (result i32)
    local.get $patch
    local.get $patch_len
    call $er_ui_patch_bool_value
    local.tee $patch_len
    i32.const -1
    i32.eq
    if
      i32.const -1
      return
    end
    local.get $base
    local.get $len
    local.get $index
    local.get $patch_len
    i32.const 0
    call $string_ref
    call $er_ui_record_set_second_ref)

  (func (export "er_ui_patch_apply_second_bool_checked") (param $base i32) (param $len i32) (param $index i32) (param $patch i32) (param $patch_len i32) (result i32)
    (local $kind i32)
    local.get $base
    local.get $len
    local.get $index
    local.get $patch
    local.get $patch_len
    call $patch_target_matches
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $base
    local.get $len
    local.get $index
    call $er_ui_record_kind
    local.tee $kind
    i32.const 22
    i32.eq
    local.get $kind
    i32.const 23
    i32.eq
    i32.or
    local.get $kind
    i32.const 56
    i32.eq
    i32.or
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $base
    local.get $len
    local.get $index
    local.get $patch
    local.get $patch_len
    call $er_ui_patch_apply_second_bool
    i32.const 1
    i32.ne
    if
      i32.const 0
      return
    end
    local.get $base
    local.get $len
    call $er_ui_validate_deep)

  (func (export "er_ui_patch_apply_second_u16") (param $base i32) (param $len i32) (param $index i32) (param $patch i32) (param $patch_len i32) (result i32)
    local.get $patch
    local.get $patch_len
    call $er_ui_patch_u16_value
    local.tee $patch_len
    i32.const -1
    i32.eq
    if
      i32.const -1
      return
    end
    local.get $base
    local.get $len
    local.get $index
    local.get $patch_len
    i32.const 0
    call $string_ref
    call $er_ui_record_set_second_ref)

  (func (export "er_ui_patch_apply_id_u16") (param $base i32) (param $len i32) (param $index i32) (param $patch i32) (param $patch_len i32) (result i32)
    local.get $patch
    local.get $patch_len
    call $er_ui_patch_u16_value
    local.tee $patch_len
    i32.const -1
    i32.eq
    if
      i32.const -1
      return
    end
    local.get $base
    local.get $len
    local.get $index
    local.get $patch_len
    call $er_ui_record_set_id)

  (func (export "er_ui_patch_apply_color_to_id") (param $base i32) (param $len i32) (param $index i32) (param $patch i32) (param $patch_len i32) (result i32)
    local.get $patch_len
    i32.const 6
    i32.lt_u
    if
      i32.const 0
      return
    end
    local.get $base
    local.get $len
    local.get $index
    local.get $patch
    i32.const 2
    i32.add
    i32.load
    call $er_ui_record_set_id)

  (func (export "er_ui_patch_apply_id") (param $base i32) (param $len i32) (param $index i32) (param $id i32) (result i32)
    local.get $base
    local.get $len
    local.get $index
    local.get $id
    call $er_ui_record_set_id)

  (func (export "er_ui_patch_apply_second_ref") (param $base i32) (param $len i32) (param $index i32) (param $ref i32) (result i32)
    local.get $base
    local.get $len
    local.get $index
    local.get $ref
    call $er_ui_record_set_second_ref)

  (func (export "er_ui_patch_apply_bool_ref") (param $base i32) (param $len i32) (param $index i32) (param $value i32) (result i32)
    local.get $base
    local.get $len
    local.get $index
    local.get $value
    i32.const 0
    i32.ne
    i32.const 0
    call $string_ref
    call $er_ui_record_set_second_ref)

  (func (export "er_ui_patch_apply_u16_ref") (param $base i32) (param $len i32) (param $index i32) (param $value i32) (result i32)
    local.get $base
    local.get $len
    local.get $index
    local.get $value
    i32.const 0
    call $string_ref
    call $er_ui_record_set_second_ref)

  (func $er_ui_patch_apply_two_strings (export "er_ui_patch_apply_two_strings") (param $base i32) (param $cap i32) (param $current_end i32) (param $index i32) (param $a_ptr i32) (param $a_len i32) (param $b_ptr i32) (param $b_len i32) (result i32)
    (local $n i32)
    (local $a_offset i32)
    (local $b_offset i32)
    (local $r i32)
    local.get $current_end
    local.get $base
    i32.lt_u
    local.get $current_end
    local.get $base
    local.get $cap
    i32.add
    i32.gt_u
    i32.or
    if
      i32.const 0
      return
    end
    local.get $base
    local.get $current_end
    local.get $base
    i32.sub
    call $er_ui_validate
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $base
    i32.const 16
    i32.add
    call $load16
    local.set $n
    local.get $index
    local.get $n
    i32.ge_u
    if
      i32.const 0
      return
    end
    local.get $a_len
    i32.const 65535
    i32.gt_u
    local.get $b_len
    i32.const 65535
    i32.gt_u
    i32.or
    if
      i32.const 0
      return
    end
    local.get $current_end
    local.get $a_len
    i32.add
    local.get $b_len
    i32.add
    local.get $base
    local.get $cap
    i32.add
    i32.gt_u
    if
      i32.const 0
      return
    end
    local.get $current_end
    i32.const 20
    local.get $base
    i32.add
    local.get $n
    i32.const 16
    i32.mul
    i32.add
    i32.sub
    local.set $a_offset
    local.get $a_offset
    local.get $a_len
    i32.add
    local.set $b_offset
    local.get $current_end
    local.get $a_ptr
    local.get $a_len
    call $copy
    local.get $current_end
    local.get $a_len
    i32.add
    local.get $b_ptr
    local.get $b_len
    call $copy
    local.get $base
    local.get $index
    call $record_at
    local.set $r
    local.get $r
    i32.const 8
    i32.add
    local.get $a_offset
    call $store16
    local.get $r
    i32.const 10
    i32.add
    local.get $a_len
    call $store16
    local.get $r
    i32.const 12
    i32.add
    local.get $b_offset
    call $store16
    local.get $r
    i32.const 14
    i32.add
    local.get $b_len
    call $store16
    local.get $base
    i32.const 4
    i32.add
    local.get $current_end
    local.get $a_len
    i32.add
    local.get $b_len
    i32.add
    local.get $base
    i32.sub
    call $store32
    local.get $current_end
    local.get $a_len
    i32.add
    local.get $b_len
    i32.add)

  (func (export "er_ui_patch_apply_two_strings_checked") (param $base i32) (param $cap i32) (param $current_end i32) (param $index i32) (param $patch i32) (param $patch_len i32) (result i32)
    (local $doc_len i32)
    (local $kind i32)
    (local $next_end i32)
    local.get $current_end
    local.get $base
    i32.lt_u
    if
      i32.const 0
      return
    end
    local.get $current_end
    local.get $base
    i32.sub
    local.tee $doc_len
    drop
    local.get $base
    local.get $doc_len
    local.get $index
    local.get $patch
    local.get $patch_len
    call $patch_target_matches
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $base
    local.get $doc_len
    local.get $index
    call $er_ui_record_kind
    local.tee $kind
    call $second_ref_is_packed
    if
      i32.const 0
      return
    end
    local.get $patch
    local.get $patch_len
    call $er_ui_patch_string_ptr
    i32.eqz
    local.get $patch
    local.get $patch_len
    call $er_ui_patch_second_string_ptr
    i32.eqz
    i32.or
    if
      i32.const 0
      return
    end
    local.get $base
    local.get $cap
    local.get $current_end
    local.get $index
    local.get $patch
    local.get $patch_len
    call $er_ui_patch_string_ptr
    local.get $patch
    local.get $patch_len
    call $er_ui_patch_string_len
    local.get $patch
    local.get $patch_len
    call $er_ui_patch_second_string_ptr
    local.get $patch
    local.get $patch_len
    call $er_ui_patch_second_string_len
    call $er_ui_patch_apply_two_strings
    local.tee $next_end
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $base
    local.get $next_end
    local.get $base
    i32.sub
    call $er_ui_validate_deep
    if (result i32)
      local.get $next_end
    else
      i32.const 0
    end)
