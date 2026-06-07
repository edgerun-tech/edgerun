
  (func $er_ui_command_tag_icon  (result i32)
    i32.const 3)

  (func $er_ui_write_one_string  (param $base i32) (param $cap i32) (param $kind i32) (param $id i32) (param $src i32) (param $len i32) (result i32)
    (local $ref i32)
    local.get $base
    local.get $cap
    i32.const 1
    i32.const 1
    i32.const 0
    i32.const 0
    i32.const 0
    call $er_ui_writer_begin
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $src
    local.get $len
    call $er_ui_writer_string
    local.set $ref
    local.get $len
    i32.eqz
    i32.eqz
    local.get $ref
    i32.eqz
    i32.and
    if
      i32.const 0
      return
    end
    i32.const 0
    local.get $kind
    local.get $id
    local.get $ref
    i32.const 0
    call $er_ui_writer_record
    drop
    global.get $writer_cursor)

  (func $er_ui_write_empty  (param $base i32) (param $cap i32) (param $kind i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 1
    i32.const 1
    i32.const 0
    i32.const 0
    i32.const 0
    call $er_ui_writer_begin
    i32.eqz
    if
      i32.const 0
      return
    end
    i32.const 0
    local.get $kind
    i32.const 0
    i32.const 0
    i32.const 0
    call $er_ui_writer_record
    drop
    global.get $writer_cursor)

  (func $er_ui_write_ref  (param $base i32) (param $cap i32) (param $kind i32) (param $id i32) (param $ref i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 1
    i32.const 1
    i32.const 0
    i32.const 0
    i32.const 0
    call $er_ui_writer_begin
    i32.eqz
    if
      i32.const 0
      return
    end
    i32.const 0
    local.get $kind
    local.get $id
    i32.const 0
    local.get $ref
    call $er_ui_writer_record
    drop
    global.get $writer_cursor)

  (func $er_ui_write_two_strings  (param $base i32) (param $cap i32) (param $kind i32) (param $id i32) (param $a_ptr i32) (param $a_len i32) (param $b_ptr i32) (param $b_len i32) (result i32)
    (local $a_ref i32)
    (local $b_ref i32)
    local.get $base
    local.get $cap
    i32.const 1
    i32.const 1
    i32.const 0
    i32.const 0
    i32.const 0
    call $er_ui_writer_begin
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $a_ptr
    local.get $a_len
    call $er_ui_writer_string
    local.set $a_ref
    local.get $a_len
    i32.eqz
    i32.eqz
    local.get $a_ref
    i32.eqz
    i32.and
    if
      i32.const 0
      return
    end
    local.get $b_ptr
    local.get $b_len
    call $er_ui_writer_string
    local.set $b_ref
    local.get $b_len
    i32.eqz
    i32.eqz
    local.get $b_ref
    i32.eqz
    i32.and
    if
      i32.const 0
      return
    end
    i32.const 0
    local.get $kind
    local.get $id
    local.get $a_ref
    local.get $b_ref
    call $er_ui_writer_record
    drop
    global.get $writer_cursor)
  (func $er_ui_patch_encode_bool  (param $out i32) (param $cap i32) (param $kind i32) (param $component_id i32) (param $value i32) (result i32)
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

  (func $er_ui_patch_encode_u16  (param $out i32) (param $cap i32) (param $kind i32) (param $component_id i32) (param $value i32) (result i32)
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

  (func $er_ui_patch_encode_f32  (param $out i32) (param $cap i32) (param $kind i32) (param $component_id i32) (param $value f32) (result i32)
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

  (func $er_ui_patch_encode_string  (param $out i32) (param $cap i32) (param $kind i32) (param $component_id i32) (param $src i32) (param $src_len i32) (result i32)
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

  (func $er_ui_patch_encode_two_strings  (param $out i32) (param $cap i32) (param $kind i32) (param $component_id i32) (param $a_ptr i32) (param $a_len i32) (param $b_ptr i32) (param $b_len i32) (result i32)
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

  (func $er_ui_patch_encode_two_strings_bool  (param $out i32) (param $cap i32) (param $kind i32) (param $component_id i32) (param $a_ptr i32) (param $a_len i32) (param $b_ptr i32) (param $b_len i32) (param $flag i32) (result i32)
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

  (func $er_ui_patch_encode_color  (param $out i32) (param $cap i32) (param $kind i32) (param $component_id i32) (param $rgba i32) (result i32)
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

  (func $er_ui_patch_kind  (param $patch i32) (param $len i32) (result i32)
    local.get $len
    i32.const 2
    i32.lt_u
    if
      i32.const -1
      return
    end
    local.get $patch
    i32.load8_u)

  (func $er_ui_patch_component_id  (param $patch i32) (param $len i32) (result i32)
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

  (func $er_ui_patch_apply_one_string  (param $base i32) (param $cap i32) (param $current_end i32) (param $index i32) (param $src i32) (param $src_len i32) (result i32)
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

  (func $er_ui_patch_apply_second_bool  (param $base i32) (param $len i32) (param $index i32) (param $patch i32) (param $patch_len i32) (result i32)
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

  (func $er_ui_patch_apply_second_bool_checked  (param $base i32) (param $len i32) (param $index i32) (param $patch i32) (param $patch_len i32) (result i32)
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

  (func $er_ui_patch_apply_second_u16  (param $base i32) (param $len i32) (param $index i32) (param $patch i32) (param $patch_len i32) (result i32)
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

  (func $er_ui_patch_apply_id_u16  (param $base i32) (param $len i32) (param $index i32) (param $patch i32) (param $patch_len i32) (result i32)
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

  (func $er_ui_patch_apply_color_to_id  (param $base i32) (param $len i32) (param $index i32) (param $patch i32) (param $patch_len i32) (result i32)
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

  (func $er_ui_patch_apply_id  (param $base i32) (param $len i32) (param $index i32) (param $id i32) (result i32)
    local.get $base
    local.get $len
    local.get $index
    local.get $id
    call $er_ui_record_set_id)

  (func $er_ui_patch_apply_second_ref  (param $base i32) (param $len i32) (param $index i32) (param $ref i32) (result i32)
    local.get $base
    local.get $len
    local.get $index
    local.get $ref
    call $er_ui_record_set_second_ref)

  (func $er_ui_patch_apply_bool_ref  (param $base i32) (param $len i32) (param $index i32) (param $value i32) (result i32)
    local.get $base
    local.get $len
    local.get $index
    local.get $value
    i32.const 0
    i32.ne
    i32.const 0
    call $string_ref
    call $er_ui_record_set_second_ref)

  (func $er_ui_patch_apply_u16_ref  (param $base i32) (param $len i32) (param $index i32) (param $value i32) (result i32)
    local.get $base
    local.get $len
    local.get $index
    local.get $value
    i32.const 0
    call $string_ref
    call $er_ui_record_set_second_ref)

  (func $er_ui_patch_apply_two_strings  (param $base i32) (param $cap i32) (param $current_end i32) (param $index i32) (param $a_ptr i32) (param $a_len i32) (param $b_ptr i32) (param $b_len i32) (result i32)
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

  (func $er_ui_patch_apply_two_strings_checked  (param $base i32) (param $cap i32) (param $current_end i32) (param $index i32) (param $patch i32) (param $patch_len i32) (result i32)
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
  (func $er_ui_ble_encode_frame  (param $out i32) (param $cap i32) (param $stream_id i32) (param $sequence i32) (param $kind i32) (param $body i32) (param $body_len i32) (result i32)
    local.get $body_len
    i32.const 19
    i32.gt_u
    local.get $kind
    i32.const 1
    i32.lt_u
    i32.or
    local.get $kind
    i32.const 4
    i32.gt_u
    i32.or
    local.get $cap
    local.get $body_len
    i32.const 8
    i32.add
    i32.lt_u
    i32.or
    if
      i32.const 0
      return
    end
    local.get $out
    i32.const 0x49555245
    call $store32
    local.get $out
    i32.const 4
    i32.add
    i32.const 1
    i32.store8
    local.get $out
    i32.const 5
    i32.add
    local.get $stream_id
    i32.store8
    local.get $out
    i32.const 6
    i32.add
    local.get $sequence
    i32.store8
    local.get $out
    i32.const 7
    i32.add
    local.get $kind
    i32.store8
    local.get $out
    i32.const 8
    i32.add
    local.get $body
    local.get $body_len
    call $copy
    local.get $body_len
    i32.const 8
    i32.add)

  (func $er_ui_ble_decode_frame  (param $frame i32) (param $len i32) (param $out i32) (param $cap i32) (result i32)
    local.get $len
    i32.const 8
    i32.lt_u
    local.get $len
    i32.const 27
    i32.gt_u
    i32.or
    local.get $cap
    i32.const 16
    i32.lt_u
    i32.or
    if
      i32.const 0
      return
    end
    local.get $frame
    i32.load
    i32.const 0x49555245
    i32.ne
    local.get $frame
    i32.const 4
    i32.add
    i32.load8_u
    i32.const 1
    i32.ne
    i32.or
    local.get $frame
    i32.const 7
    i32.add
    i32.load8_u
    i32.const 1
    i32.lt_u
    i32.or
    local.get $frame
    i32.const 7
    i32.add
    i32.load8_u
    i32.const 4
    i32.gt_u
    i32.or
    if
      i32.const 0
      return
    end
    local.get $out
    local.get $frame
    i32.const 5
    i32.add
    i32.load8_u
    i32.store8
    local.get $out
    i32.const 1
    i32.add
    local.get $frame
    i32.const 6
    i32.add
    i32.load8_u
    i32.store8
    local.get $out
    i32.const 2
    i32.add
    local.get $frame
    i32.const 7
    i32.add
    i32.load8_u
    i32.store8
    local.get $out
    i32.const 4
    i32.add
    local.get $frame
    i32.const 8
    i32.add
    call $store32
    local.get $out
    i32.const 8
    i32.add
    local.get $len
    i32.const 8
    i32.sub
    call $store32
    i32.const 12)

  (func $er_ui_ble_encode_route  (param $out i32) (param $cap i32) (param $route_id i32) (param $flags i32) (param $name i32) (param $name_len i32) (result i32)
    local.get $name_len
    i32.const 255
    i32.gt_u
    local.get $cap
    local.get $name_len
    i32.const 3
    i32.add
    i32.lt_u
    i32.or
    if
      i32.const 0
      return
    end
    local.get $out
    local.get $route_id
    i32.store8
    local.get $out
    i32.const 1
    i32.add
    local.get $flags
    i32.store8
    local.get $out
    i32.const 2
    i32.add
    local.get $name_len
    i32.store8
    local.get $out
    i32.const 3
    i32.add
    local.get $name
    local.get $name_len
    call $copy
    local.get $name_len
    i32.const 3
    i32.add)

  (func $er_ui_ble_encode_manufacturer_ad  (param $out i32) (param $cap i32) (param $frame i32) (param $frame_len i32) (result i32)
    local.get $frame_len
    i32.const 8
    i32.lt_u
    local.get $frame_len
    i32.const 27
    i32.gt_u
    i32.or
    local.get $cap
    local.get $frame_len
    i32.const 4
    i32.add
    i32.lt_u
    i32.or
    if
      i32.const 0
      return
    end
    local.get $frame
    i32.load
    i32.const 0x49555245
    i32.ne
    local.get $frame
    i32.const 4
    i32.add
    i32.load8_u
    i32.const 1
    i32.ne
    i32.or
    local.get $frame
    i32.const 7
    i32.add
    i32.load8_u
    i32.const 1
    i32.lt_u
    i32.or
    local.get $frame
    i32.const 7
    i32.add
    i32.load8_u
    i32.const 4
    i32.gt_u
    i32.or
    if
      i32.const 0
      return
    end
    local.get $out
    local.get $frame_len
    i32.const 3
    i32.add
    i32.store8
    local.get $out
    i32.const 1
    i32.add
    i32.const 0xff
    i32.store8
    local.get $out
    i32.const 2
    i32.add
    i32.const 0xffff
    call $store16
    local.get $out
    i32.const 4
    i32.add
    local.get $frame
    local.get $frame_len
    call $copy
    local.get $frame_len
    i32.const 4
    i32.add)
  (func $er_ui_patch_bool_value  (param $patch i32) (param $len i32) (result i32)
    local.get $len
    i32.const 3
    i32.lt_u
    if
      i32.const -1
      return
    end
    local.get $patch
    i32.const 2
    i32.add
    i32.load8_u
    i32.const 0
    i32.ne)

  (func $er_ui_patch_u16_value  (param $patch i32) (param $len i32) (result i32)
    local.get $len
    i32.const 4
    i32.lt_u
    if
      i32.const -1
      return
    end
    local.get $patch
    i32.const 2
    i32.add
    call $load16)

  (func $er_ui_patch_f32_value  (param $patch i32) (param $len i32) (result f32)
    local.get $len
    i32.const 6
    i32.lt_u
    if
      f32.const nan
      return
    end
    local.get $patch
    i32.const 2
    i32.add
    f32.load)

  (func $er_ui_patch_string_ptr  (param $patch i32) (param $len i32) (result i32)
    local.get $len
    i32.const 3
    i32.lt_u
    if
      i32.const 0
      return
    end
    local.get $patch
    i32.const 3
    i32.add)

  (func $er_ui_patch_string_len  (param $patch i32) (param $len i32) (result i32)
    (local $n i32)
    local.get $len
    i32.const 3
    i32.lt_u
    if
      i32.const 0
      return
    end
    local.get $patch
    i32.const 2
    i32.add
    i32.load8_u
    local.tee $n
    i32.const 3
    i32.add
    local.get $len
    i32.gt_u
    if
      i32.const 0
      return
    end
    local.get $n)

  (func $er_ui_patch_second_string_ptr  (param $patch i32) (param $len i32) (result i32)
    (local $a i32)
    local.get $len
    i32.const 4
    i32.lt_u
    if
      i32.const 0
      return
    end
    local.get $patch
    i32.const 2
    i32.add
    i32.load8_u
    local.set $a
    local.get $len
    i32.const 4
    local.get $a
    i32.add
    i32.lt_u
    if
      i32.const 0
      return
    end
    local.get $patch
    i32.const 4
    i32.add
    local.get $a
    i32.add)

  (func $er_ui_patch_second_string_len  (param $patch i32) (param $len i32) (result i32)
    (local $a i32)
    (local $b i32)
    local.get $len
    i32.const 4
    i32.lt_u
    if
      i32.const 0
      return
    end
    local.get $patch
    i32.const 2
    i32.add
    i32.load8_u
    local.set $a
    local.get $len
    i32.const 4
    local.get $a
    i32.add
    i32.lt_u
    if
      i32.const 0
      return
    end
    local.get $patch
    i32.const 3
    i32.add
    local.get $a
    i32.add
    i32.load8_u
    local.tee $b
    i32.const 4
    local.get $a
    i32.add
    i32.add
    local.get $len
    i32.gt_u
    if
      i32.const 0
      return
    end
    local.get $b)

  (func $er_ui_patch_color_value  (param $patch i32) (param $len i32) (result i32)
    local.get $len
    i32.const 6
    i32.lt_u
    if
      i32.const 0
      return
    end
    local.get $patch
    i32.const 2
    i32.add
    i32.load)
  (func $er_ui_ble_decode_route  (param $route i32) (param $len i32) (param $out i32) (param $cap i32) (result i32)
    (local $name_len i32)
    local.get $len
    i32.const 3
    i32.lt_u
    local.get $cap
    i32.const 12
    i32.lt_u
    i32.or
    if
      i32.const 0
      return
    end
    local.get $route
    i32.const 2
    i32.add
    i32.load8_u
    local.set $name_len
    local.get $len
    i32.const 3
    local.get $name_len
    i32.add
    i32.ne
    if
      i32.const 0
      return
    end
    local.get $out
    local.get $route
    i32.load8_u
    i32.store8
    local.get $out
    i32.const 1
    i32.add
    local.get $route
    i32.const 1
    i32.add
    i32.load8_u
    i32.store8
    local.get $out
    i32.const 4
    i32.add
    local.get $route
    i32.const 3
    i32.add
    call $store32
    local.get $out
    i32.const 8
    i32.add
    local.get $name_len
    call $store32
    i32.const 12)

  (func $er_ui_ble_decode_manufacturer_ad  (param $scan i32) (param $len i32) (param $out i32) (param $cap i32) (result i32)
    (local $index i32)
    (local $ad_len i32)
    (local $ad i32)
    (local $next i32)
    loop $loop
      local.get $index
      local.get $len
      i32.lt_u
      if
        local.get $scan
        local.get $index
        i32.add
        i32.load8_u
        local.tee $ad_len
        i32.eqz
        if
          i32.const 0
          return
        end
        local.get $index
        i32.const 1
        i32.add
        local.get $ad_len
        i32.add
        local.tee $next
        local.get $len
        i32.gt_u
        if
          i32.const 0
          return
        end
        local.get $scan
        local.get $index
        i32.const 1
        i32.add
        i32.add
        local.set $ad
        local.get $ad_len
        i32.const 3
        i32.ge_u
        local.get $ad
        i32.load8_u
        i32.const 0xff
        i32.eq
        i32.and
        local.get $ad
        i32.const 1
        i32.add
        call $load16
        i32.const 0xffff
        i32.eq
        i32.and
        if
          local.get $ad
          i32.const 3
          i32.add
          local.get $ad_len
          i32.const 3
          i32.sub
          local.get $out
          local.get $cap
          call $er_ui_ble_decode_frame
          return
        end
        local.get $next
        local.set $index
        br $loop
      end
    end
    i32.const 0)
  (func $er_ui_command_hit_test  (param $commands i32) (param $count i32) (param $x f32) (param $y f32) (result i32)
    (local $i i32)
    (local $p i32)
    (local $hit i32)
    (local $rx f32)
    (local $ry f32)
    (local $rw f32)
    (local $rh f32)
    i32.const -1
    local.set $hit
    loop $loop
      local.get $i
      local.get $count
      i32.lt_u
      if
        local.get $commands
        local.get $i
        i32.const 48
        i32.mul
        i32.add
        local.set $p
        local.get $p
        i32.load
        i32.const 0
        i32.ne
        if
          local.get $p
          i32.const 8
          i32.add
          f32.load
          local.set $rx
          local.get $p
          i32.const 12
          i32.add
          f32.load
          local.set $ry
          local.get $p
          i32.const 16
          i32.add
          f32.load
          local.set $rw
          local.get $p
          i32.const 20
          i32.add
          f32.load
          local.set $rh
          local.get $x
          local.get $rx
          f32.ge
          local.get $y
          local.get $ry
          f32.ge
          i32.and
          local.get $x
          local.get $rx
          local.get $rw
          f32.add
          f32.lt
          i32.and
          local.get $y
          local.get $ry
          local.get $rh
          f32.add
          f32.lt
          i32.and
          if
            local.get $i
            local.set $hit
          end
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $loop
      end
    end
    local.get $hit)

  (func $er_ui_command_hit_id  (param $commands i32) (param $count i32) (param $x f32) (param $y f32) (result i32)
    (local $index i32)
    local.get $commands
    local.get $count
    local.get $x
    local.get $y
    call $er_ui_command_hit_test
    local.tee $index
    i32.const -1
    i32.eq
    if
      i32.const -1
      return
    end
    local.get $commands
    local.get $count
    local.get $index
    call $er_ui_command_owner_id_at)

  (func $er_ui_command_tag  (param $command i32) (result i32)
    local.get $command
    i32.load)

  (func $er_ui_command_color  (param $command i32) (result i32)
    local.get $command
    i32.const 4
    i32.add
    i32.load)

  (func $er_ui_command_x  (param $command i32) (result f32)
    local.get $command
    i32.const 8
    i32.add
    f32.load)

  (func $er_ui_command_y  (param $command i32) (result f32)
    local.get $command
    i32.const 12
    i32.add
    f32.load)

  (func $er_ui_command_w  (param $command i32) (result f32)
    local.get $command
    i32.const 16
    i32.add
    f32.load)

  (func $er_ui_command_h  (param $command i32) (result f32)
    local.get $command
    i32.const 20
    i32.add
    f32.load)

  (func $er_ui_command_text_ptr  (param $command i32) (result i32)
    local.get $command
    i32.const 24
    i32.add
    i32.load)

  (func $er_ui_command_text_len  (param $command i32) (result i32)
    local.get $command
    i32.const 28
    i32.add
    i32.load)

  (func $er_ui_command_owner_id  (param $command i32) (result i32)
    local.get $command
    i32.const 32
    i32.add
    i32.load)

  (func $er_ui_command_owner_kind  (param $command i32) (result i32)
    local.get $command
    i32.const 36
    i32.add
    i32.load)

  (func $er_ui_command_role  (param $command i32) (result i32)
    local.get $command
    i32.const 40
    i32.add
    i32.load)

  (func $er_ui_command_meta  (param $command i32) (result i32)
    local.get $command
    i32.const 44
    i32.add
    i32.load)

  (func $er_ui_command_set_owner  (param $command i32) (param $id i32) (param $kind i32) (param $role i32) (result i32)
    local.get $command
    i32.const 32
    i32.add
    local.get $id
    call $store32
    local.get $command
    i32.const 36
    i32.add
    local.get $kind
    call $store32
    local.get $command
    i32.const 40
    i32.add
    local.get $role
    call $store32
    i32.const 1)

  (func $er_ui_command_set_meta  (param $command i32) (param $meta i32) (result i32)
    local.get $command
    i32.const 44
    i32.add
    local.get $meta
    call $store32
    i32.const 1)

  (func $er_ui_command_ptr  (param $commands i32) (param $count i32) (param $index i32) (result i32)
    local.get $index
    local.get $count
    i32.ge_u
    if
      i32.const -1
      return
    end
    local.get $commands
    local.get $index
    i32.const 48
    i32.mul
    i32.add)

  (func $er_ui_command_valid_at  (param $commands i32) (param $count i32) (param $index i32) (result i32)
    local.get $index
    local.get $count
    i32.lt_u)

  (func $er_ui_command_copy  (param $commands i32) (param $count i32) (param $index i32) (param $out i32) (result i32)
    local.get $commands
    local.get $count
    local.get $index
    call $er_ui_command_ptr
    local.tee $commands
    i32.const -1
    i32.eq
    if
      i32.const 0
      return
    end
    local.get $out
    local.get $commands
    i32.const 48
    call $copy
    i32.const 1)

  (func $er_ui_command_tag_at  (param $commands i32) (param $count i32) (param $index i32) (result i32)
    local.get $commands
    local.get $count
    local.get $index
    call $er_ui_command_ptr
    local.tee $commands
    i32.const -1
    i32.eq
    if
      i32.const -1
      return
    end
    local.get $commands
    call $er_ui_command_tag)

  (func $er_ui_command_color_at  (param $commands i32) (param $count i32) (param $index i32) (result i32)
    local.get $commands
    local.get $count
    local.get $index
    call $er_ui_command_ptr
    local.tee $commands
    i32.const -1
    i32.eq
    if
      i32.const -1
      return
    end
    local.get $commands
    call $er_ui_command_color)

  (func $er_ui_command_x_at  (param $commands i32) (param $count i32) (param $index i32) (result f32)
    local.get $commands
    local.get $count
    local.get $index
    call $er_ui_command_ptr
    local.tee $commands
    i32.const -1
    i32.eq
    if
      f32.const nan
      return
    end
    local.get $commands
    call $er_ui_command_x)

  (func $er_ui_command_y_at  (param $commands i32) (param $count i32) (param $index i32) (result f32)
    local.get $commands
    local.get $count
    local.get $index
    call $er_ui_command_ptr
    local.tee $commands
    i32.const -1
    i32.eq
    if
      f32.const nan
      return
    end
    local.get $commands
    call $er_ui_command_y)

  (func $er_ui_command_w_at  (param $commands i32) (param $count i32) (param $index i32) (result f32)
    local.get $commands
    local.get $count
    local.get $index
    call $er_ui_command_ptr
    local.tee $commands
    i32.const -1
    i32.eq
    if
      f32.const nan
      return
    end
    local.get $commands
    call $er_ui_command_w)

  (func $er_ui_command_h_at  (param $commands i32) (param $count i32) (param $index i32) (result f32)
    local.get $commands
    local.get $count
    local.get $index
    call $er_ui_command_ptr
    local.tee $commands
    i32.const -1
    i32.eq
    if
      f32.const nan
      return
    end
    local.get $commands
    call $er_ui_command_h)

  (func $er_ui_command_text_ptr_at  (param $commands i32) (param $count i32) (param $index i32) (result i32)
    local.get $commands
    local.get $count
    local.get $index
    call $er_ui_command_ptr
    local.tee $commands
    i32.const -1
    i32.eq
    if
      i32.const -1
      return
    end
    local.get $commands
    call $er_ui_command_text_ptr)

  (func $er_ui_command_text_len_at  (param $commands i32) (param $count i32) (param $index i32) (result i32)
    local.get $commands
    local.get $count
    local.get $index
    call $er_ui_command_ptr
    local.tee $commands
    i32.const -1
    i32.eq
    if
      i32.const -1
      return
    end
    local.get $commands
    call $er_ui_command_text_len)

  (func $er_ui_command_owner_id_at  (param $commands i32) (param $count i32) (param $index i32) (result i32)
    local.get $commands
    local.get $count
    local.get $index
    call $er_ui_command_ptr
    local.tee $commands
    i32.const -1
    i32.eq
    if
      i32.const -1
      return
    end
    local.get $commands
    call $er_ui_command_owner_id)

  (func $er_ui_command_owner_kind_at  (param $commands i32) (param $count i32) (param $index i32) (result i32)
    local.get $commands
    local.get $count
    local.get $index
    call $er_ui_command_ptr
    local.tee $commands
    i32.const -1
    i32.eq
    if
      i32.const -1
      return
    end
    local.get $commands
    call $er_ui_command_owner_kind)

  (func $er_ui_command_role_at  (param $commands i32) (param $count i32) (param $index i32) (result i32)
    local.get $commands
    local.get $count
    local.get $index
    call $er_ui_command_ptr
    local.tee $commands
    i32.const -1
    i32.eq
    if
      i32.const -1
      return
    end
    local.get $commands
    call $er_ui_command_role)

  (func $er_ui_command_meta_at  (param $commands i32) (param $count i32) (param $index i32) (result i32)
    local.get $commands
    local.get $count
    local.get $index
    call $er_ui_command_ptr
    local.tee $commands
    i32.const -1
    i32.eq
    if
      i32.const -1
      return
    end
    local.get $commands
    call $er_ui_command_meta)

  (func $er_ui_command_set_owner_at  (param $commands i32) (param $count i32) (param $index i32) (param $id i32) (param $kind i32) (param $role i32) (result i32)
    local.get $commands
    local.get $count
    local.get $index
    call $er_ui_command_ptr
    local.tee $commands
    i32.const -1
    i32.eq
    if
      i32.const 0
      return
    end
    local.get $commands
    i32.const 32
    i32.add
    local.get $id
    call $store32
    local.get $commands
    i32.const 36
    i32.add
    local.get $kind
    call $store32
    local.get $commands
    i32.const 40
    i32.add
    local.get $role
    call $store32
    i32.const 1)

  (func $er_ui_command_set_meta_at  (param $commands i32) (param $count i32) (param $index i32) (param $meta i32) (result i32)
    local.get $commands
    local.get $count
    local.get $index
    call $er_ui_command_ptr
    local.tee $commands
    i32.const -1
    i32.eq
    if
      i32.const 0
      return
    end
    local.get $commands
    i32.const 44
    i32.add
    local.get $meta
    call $store32
    i32.const 1)

  (func $er_ui_command_write_rect  (param $out i32) (param $cap i32) (param $count i32) (param $x f32) (param $y f32) (param $w f32) (param $h f32) (param $color i32) (result i32)
    local.get $w
    local.get $h
    call $valid_rect
    i32.eqz
    if
      i32.const -1
      return
    end
    local.get $out
    local.get $cap
    local.get $count
    local.get $x
    local.get $y
    local.get $w
    local.get $h
    local.get $color
    call $emit_rect)

  (func $er_ui_command_write_text  (param $out i32) (param $cap i32) (param $count i32) (param $x f32) (param $y f32) (param $w f32) (param $h f32) (param $text_ptr i32) (param $text_len i32) (param $color i32) (result i32)
    (local $p i32)
    local.get $count
    i32.const 48
    i32.mul
    i32.const 48
    i32.add
    local.get $cap
    i32.gt_u
    if
      i32.const -1
      return
    end
    local.get $w
    local.get $h
    call $valid_rect
    i32.eqz
    if
      i32.const -1
      return
    end
    local.get $out
    local.get $count
    i32.const 48
    i32.mul
    i32.add
    local.set $p
    local.get $p
    i32.const 48
    call $zero
    local.get $p
    i32.const 2
    call $store32
    local.get $p
    i32.const 4
    i32.add
    local.get $color
    call $store32
    local.get $p
    i32.const 8
    i32.add
    local.get $x
    f32.store
    local.get $p
    i32.const 12
    i32.add
    local.get $y
    f32.store
    local.get $p
    i32.const 16
    i32.add
    local.get $w
    f32.store
    local.get $p
    i32.const 20
    i32.add
    local.get $h
    f32.store
    local.get $p
    i32.const 24
    i32.add
    local.get $text_ptr
    call $store32
    local.get $p
    i32.const 28
    i32.add
    local.get $text_len
    call $store32
    local.get $count
    i32.const 1
    i32.add)

  (func $er_ui_command_write_icon  (param $out i32) (param $cap i32) (param $count i32) (param $x f32) (param $y f32) (param $w f32) (param $h f32) (param $icon i32) (param $color i32) (result i32)
    (local $p i32)
    local.get $icon
    call $er_ui_icon_valid
    i32.eqz
    if
      i32.const -1
      return
    end
    local.get $count
    i32.const 48
    i32.mul
    i32.const 48
    i32.add
    local.get $cap
    i32.gt_u
    if
      i32.const -1
      return
    end
    local.get $w
    local.get $h
    call $valid_rect
    i32.eqz
    if
      i32.const -1
      return
    end
    local.get $out
    local.get $count
    i32.const 48
    i32.mul
    i32.add
    local.set $p
    local.get $p
    i32.const 48
    call $zero
    local.get $p
    call $er_ui_command_tag_icon
    call $store32
    local.get $p
    i32.const 4
    i32.add
    local.get $color
    call $store32
    local.get $p
    i32.const 8
    i32.add
    local.get $x
    f32.store
    local.get $p
    i32.const 12
    i32.add
    local.get $y
    f32.store
    local.get $p
    i32.const 16
    i32.add
    local.get $w
    f32.store
    local.get $p
    i32.const 20
    i32.add
    local.get $h
    f32.store
    local.get $p
    i32.const 24
    i32.add
    local.get $icon
    call $er_ui_icon_path_ptr
    call $store32
    local.get $p
    i32.const 28
    i32.add
    local.get $icon
    call $er_ui_icon_path_len
    call $store32
    local.get $p
    i32.const 44
    i32.add
    local.get $icon
    call $store32
    local.get $count
    i32.const 1
    i32.add)

  (func $er_ui_command_write_icon_fit  (param $out i32) (param $cap i32) (param $count i32) (param $x f32) (param $y f32) (param $w f32) (param $h f32) (param $icon i32) (param $color i32) (param $dpr f32) (result i32)
    (local $fit_x f32)
    (local $fit_y f32)
    (local $size f32)
    local.get $w
    local.get $h
    call $valid_rect
    i32.eqz
    if
      i32.const -1
      return
    end
    local.get $w
    local.get $h
    call $min_f32
    local.get $dpr
    call $snap_pixel
    local.set $size
    local.get $x
    local.get $w
    local.get $size
    f32.sub
    f32.const 0.5
    f32.mul
    f32.add
    local.get $dpr
    call $snap_pixel
    local.set $fit_x
    local.get $y
    local.get $h
    local.get $size
    f32.sub
    f32.const 0.5
    f32.mul
    f32.add
    local.get $dpr
    call $snap_pixel
    local.set $fit_y
    local.get $out
    local.get $cap
    local.get $count
    local.get $fit_x
    local.get $fit_y
    local.get $size
    local.get $size
    local.get $icon
    local.get $color
    call $er_ui_command_write_icon)

  (func $er_ui_command_hit_find  (param $commands i32) (param $count i32) (param $x f32) (param $y f32) (param $out_command i32) (param $out_index i32) (result i32)
    (local $i i32)
    (local $p i32)
    (local $rx f32)
    (local $ry f32)
    (local $rw f32)
    (local $rh f32)
    local.get $count
    local.set $i
    block $done
      loop $loop
        local.get $i
        i32.eqz
        br_if $done
        local.get $i
        i32.const 1
        i32.sub
        local.set $i
        local.get $commands
        local.get $i
        i32.const 48
        i32.mul
        i32.add
        local.set $p
        local.get $p
        i32.load
        i32.const 0
        i32.ne
        if
          local.get $p
          i32.const 8
          i32.add
          f32.load
          local.set $rx
          local.get $p
          i32.const 12
          i32.add
          f32.load
          local.set $ry
          local.get $p
          i32.const 16
          i32.add
          f32.load
          local.set $rw
          local.get $p
          i32.const 20
          i32.add
          f32.load
          local.set $rh
          local.get $x
          local.get $rx
          f32.ge
          local.get $y
          local.get $ry
          f32.ge
          i32.and
          local.get $x
          local.get $rx
          local.get $rw
          f32.add
          f32.lt
          i32.and
          local.get $y
          local.get $ry
          local.get $rh
          f32.add
          f32.lt
          i32.and
          if
            local.get $out_command
            i32.eqz
            i32.eqz
            if
              local.get $out_command
              local.get $p
              i32.const 48
              call $copy
            end
            local.get $out_index
            i32.eqz
            i32.eqz
            if
              local.get $out_index
              local.get $i
              i32.store
            end
            i32.const 1
            return
          end
        end
        br $loop
      end
    end
    i32.const 0)
  (func $er_ui_command_radius  (result f32) f32.const 8)
  (func $er_ui_command_input_h  (result f32) f32.const 36)
  (func $er_ui_command_icon_x  (result f32) f32.const 8)
  (func $er_ui_command_icon_size  (result f32) f32.const 14)
  (func $er_ui_command_text_x  (result f32) f32.const 28)
  (func $er_ui_command_padding_x  (result f32) f32.const 8)
  (func $er_ui_command_text_h  (result f32) f32.const 13)
  (func $er_ui_command_item_id_offset  (result i32) i32.const 1)
  (func $er_ui_command_list_gap  (result f32) f32.const 6)
  (func $er_ui_command_list_padding  (result f32) f32.const 4)
  (func $er_ui_command_item_h  (result f32) f32.const 24)
  (func $er_ui_command_item_gap  (result f32) f32.const 4)
  (func $er_ui_command_item_padding_x  (result f32) f32.const 8)
  (func $er_ui_command_shortcut_gap  (result f32) f32.const 12)
  (func $er_ui_command_max_visible_items  (result i32) i32.const 3)
  (func $er_ui_command_empty_text_h  (result f32) f32.const 14)

  (func $er_ui_command_input_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $bounds f32.load
    local.get $bounds i32.const 4 i32.add f32.load
    local.get $bounds i32.const 8 i32.add f32.load
    local.get $bounds i32.const 12 i32.add f32.load f32.const 36 call $min_f32
    call $rect_store)

  (func $er_ui_command_list_bounds  (param $bounds i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 12 i32.add f32.load f32.const 42 f32.le
    if i32.const 0 return end
    local.get $out
    local.get $bounds f32.load
    local.get $bounds i32.const 4 i32.add f32.load f32.const 42 f32.add
    local.get $bounds i32.const 8 i32.add f32.load
    local.get $bounds i32.const 12 i32.add f32.load f32.const 42 f32.sub f32.const 1 call $max_f32
    call $rect_store)

  (func $er_ui_command_icon_bounds  (param $input i32) (param $out i32) (result i32)
    local.get $input i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $input f32.load f32.const 8 f32.add
    local.get $input i32.const 4 i32.add f32.load local.get $input i32.const 12 i32.add f32.load f32.const 14 f32.sub f32.const 0.5 f32.mul f32.add
    f32.const 14
    f32.const 14
    call $rect_store)

  (func $er_ui_command_input_text_bounds  (param $input i32) (param $out i32) (result i32)
    local.get $input i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $input f32.load f32.const 28 f32.add
    local.get $input i32.const 4 i32.add f32.load local.get $input i32.const 12 i32.add f32.load f32.const 13 f32.sub f32.const 0.5 f32.mul f32.add
    local.get $input i32.const 8 i32.add f32.load f32.const 28 f32.sub f32.const 8 f32.sub f32.const 1 call $max_f32
    f32.const 13
    call $rect_store)

  (func $er_ui_command_visible_item_capacity  (param $bounds i32) (result i32)
    (local $available f32) (local $raw i32)
    local.get $bounds i32.eqz
    if i32.const 0 return end
    local.get $bounds i32.const 124704 call $er_ui_command_list_bounds
    i32.eqz
    if i32.const 0 return end
    i32.const 124716 f32.load f32.const 8 f32.sub f32.const 0 call $max_f32 local.set $available
    local.get $available f32.const 4 f32.add f32.const 28 f32.div f32.floor i32.trunc_f32_u local.set $raw
    local.get $raw i32.const 3 i32.gt_u
    if i32.const 3 return end
    local.get $raw)

  (func $er_ui_command_item_bounds  (param $bounds i32) (param $index i32) (param $out i32) (result i32)
    (local $idx f32) (local $y f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $index i32.const 0 i32.lt_s local.get $index i32.const 3 i32.ge_s i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 124720 call $er_ui_command_list_bounds
    i32.eqz
    if i32.const 0 return end
    local.get $index f32.convert_i32_s local.set $idx
    i32.const 124724 f32.load f32.const 4 f32.add local.get $idx f32.const 28 f32.mul f32.add local.set $y
    local.get $y f32.const 24 f32.add i32.const 124724 f32.load i32.const 124732 f32.load f32.add f32.const 4 f32.sub f32.gt
    if i32.const 0 return end
    local.get $out
    i32.const 124720 f32.load f32.const 4 f32.add
    local.get $y
    i32.const 124728 f32.load f32.const 8 f32.sub f32.const 1 call $max_f32
    f32.const 24
    call $rect_store)

  (func $er_ui_command_item_id  (param $id i32) (param $index i32) (result i32)
    local.get $id local.get $index i32.add i32.const 1 i32.add)

  (func $er_ui_command_empty_text_bounds  (param $list i32) (param $out i32) (result i32)
    local.get $list i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $list f32.load f32.const 4 f32.add
    local.get $list i32.const 4 i32.add f32.load f32.const 4 f32.add
    local.get $list i32.const 8 i32.add f32.load f32.const 8 f32.sub f32.const 1 call $max_f32
    f32.const 14
    call $rect_store)

  (func $er_ui_command_item_detail_width  (param $shortcut_ptr i32) (param $shortcut_len i32) (result f32)
    local.get $shortcut_ptr i32.eqz local.get $shortcut_len i32.eqz i32.or
    if f32.const 0 return end
    local.get $shortcut_ptr local.get $shortcut_len f32.const 16 call $er_ui_font_text_width f32.const 16 f32.add)

  (func $er_ui_command_item_label_bounds  (param $item i32) (param $detail_w f32) (param $out i32) (result i32)
    local.get $item i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $out
    local.get $item f32.load
    local.get $item i32.const 4 i32.add f32.load
    local.get $item i32.const 8 i32.add f32.load local.get $detail_w f32.sub f32.const 1 call $max_f32
    local.get $item i32.const 12 i32.add f32.load
    call $rect_store)

  (func $er_ui_command_item_shortcut_bounds  (param $item i32) (param $detail_w f32) (param $out i32) (result i32)
    local.get $item i32.eqz local.get $out i32.eqz i32.or local.get $detail_w f32.const 0 f32.le i32.or
    if i32.const 0 return end
    local.get $out
    local.get $item f32.load local.get $item i32.const 8 i32.add f32.load f32.add local.get $detail_w f32.sub
    local.get $item i32.const 4 i32.add f32.load
    local.get $detail_w
    local.get $item i32.const 12 i32.add f32.load
    call $rect_store)
