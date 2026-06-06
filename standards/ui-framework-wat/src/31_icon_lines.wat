  (func $icon_write_line (param $out i32) (param $cap i32) (param $count i32) (param $x1 f32) (param $y1 f32) (param $x2 f32) (param $y2 f32) (result i32)
    (local $p i32)
    local.get $count
    i32.const 1
    i32.add
    i32.const 20
    i32.mul
    local.get $cap
    i32.gt_u
    if
      i32.const -1
      return
    end
    local.get $out
    local.get $count
    i32.const 20
    i32.mul
    i32.add
    local.set $p
    local.get $p
    i32.const 1
    call $store32
    local.get $p
    i32.const 4
    i32.add
    local.get $x1
    f32.store
    local.get $p
    i32.const 8
    i32.add
    local.get $y1
    f32.store
    local.get $p
    i32.const 12
    i32.add
    local.get $x2
    f32.store
    local.get $p
    i32.const 16
    i32.add
    local.get $y2
    f32.store
    local.get $count
    i32.const 1
    i32.add)

  (func $icon_write_line_scaled (param $out i32) (param $cap i32) (param $count i32) (param $x f32) (param $y f32) (param $scale f32) (param $dpr f32) (param $x1 f32) (param $y1 f32) (param $x2 f32) (param $y2 f32) (result i32)
    local.get $out
    local.get $cap
    local.get $count
    local.get $x
    local.get $x1
    local.get $scale
    f32.mul
    f32.add
    local.get $dpr
    call $snap_pixel
    local.get $y
    local.get $y1
    local.get $scale
    f32.mul
    f32.add
    local.get $dpr
    call $snap_pixel
    local.get $x
    local.get $x2
    local.get $scale
    f32.mul
    f32.add
    local.get $dpr
    call $snap_pixel
    local.get $y
    local.get $y2
    local.get $scale
    f32.mul
    f32.add
    local.get $dpr
    call $snap_pixel
    call $icon_write_line)

  (func (export "er_ui_icon_scaled_stroke_width") (param $icon i32) (param $size f32) (result f32)
    local.get $icon
    call $er_ui_icon_valid
    local.get $size
    f32.const 0
    f32.gt
    i32.and
    if (result f32)
      local.get $icon
      call $er_ui_icon_stroke_width
      local.get $size
      f32.mul
      f32.const 24
      f32.div
    else
      f32.const 0
    end)

  (func (export "er_ui_icon_stroke_width_scaled") (param $icon i32) (param $w f32) (param $h f32) (param $dpr f32) (result f32)
    (local $stroke f32)
    local.get $icon
    call $er_ui_icon_valid
    local.get $w
    local.get $h
    call $valid_rect
    i32.and
    if (result f32)
      local.get $icon
      call $er_ui_icon_stroke_width
      local.get $w
      local.get $h
      call $min_f32
      f32.mul
      f32.const 24
      f32.div
      local.tee $stroke
      local.get $dpr
      call $snap_pixel
      local.set $stroke
      local.get $dpr
      f32.const 0
      f32.gt
      if (result f32)
        local.get $stroke
        f32.const 1
        local.get $dpr
        f32.div
        call $max_f32
      else
        local.get $stroke
      end
    else
      f32.const 0
    end)

  (func $er_ui_icon_fit_rect (export "er_ui_icon_fit_rect") (param $x f32) (param $y f32) (param $w f32) (param $h f32) (param $dpr f32) (param $out i32) (result i32)
    (local $size f32)
    local.get $out
    i32.eqz
    local.get $w
    local.get $h
    call $valid_rect
    i32.eqz
    i32.or
    if
      i32.const 0
      return
    end
    local.get $w
    local.get $h
    call $min_f32
    local.set $size
    local.get $out
    local.get $x
    local.get $w
    local.get $size
    f32.sub
    f32.const 0.5
    f32.mul
    f32.add
    local.get $dpr
    call $snap_pixel
    f32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $y
    local.get $h
    local.get $size
    f32.sub
    f32.const 0.5
    f32.mul
    f32.add
    local.get $dpr
    call $snap_pixel
    f32.store
    local.get $out
    i32.const 8
    i32.add
    local.get $size
    local.get $dpr
    call $snap_pixel
    f32.store
    local.get $out
    i32.const 12
    i32.add
    local.get $size
    local.get $dpr
    call $snap_pixel
    f32.store
    i32.const 1)

  (func (export "er_ui_icon_fit_viewport") (param $x f32) (param $y f32) (param $w f32) (param $h f32) (param $dpr f32) (param $out i32) (result i32)
    local.get $x
    local.get $y
    local.get $w
    local.get $h
    local.get $dpr
    local.get $out
    call $er_ui_icon_fit_rect)

  (func (export "er_ui_icon_segments_write") (param $icon i32) (param $out i32) (param $cap i32) (result i32)
    (local $count i32)
    local.get $cap
    local.get $icon
    call $er_ui_icon_segment_count
    i32.const 20
    i32.mul
    i32.lt_u
    if
      i32.const -1
      return
    end
    local.get $icon
    i32.const 44
    i32.eq
    local.get $icon
    i32.const 2
    i32.eq
    i32.or
    if
      local.get $out
      local.get $cap
      local.get $count
      f32.const 6
      f32.const 9
      f32.const 12
      f32.const 15
      call $icon_write_line
      local.set $count
      local.get $out
      local.get $cap
      local.get $count
      f32.const 12
      f32.const 15
      f32.const 18
      f32.const 9
      call $icon_write_line
      return
    end
    local.get $icon
    i32.const 55
    i32.eq
    if
      local.get $out local.get $cap local.get $count f32.const 8 f32.const 5 f32.const 16 f32.const 5 call $icon_write_line local.set $count
      local.get $out local.get $cap local.get $count f32.const 16 f32.const 5 f32.const 16 f32.const 13 call $icon_write_line local.set $count
      local.get $out local.get $cap local.get $count f32.const 16 f32.const 13 f32.const 8 f32.const 13 call $icon_write_line local.set $count
      local.get $out local.get $cap local.get $count f32.const 8 f32.const 13 f32.const 8 f32.const 5 call $icon_write_line local.set $count
      local.get $out local.get $cap local.get $count f32.const 15 f32.const 14 f32.const 20 f32.const 19 call $icon_write_line
      return
    end
    local.get $icon
    i32.const 1
    i32.eq
    if
      local.get $out local.get $cap local.get $count f32.const 5 f32.const 12 f32.const 10 f32.const 17 call $icon_write_line local.set $count
      local.get $out local.get $cap local.get $count f32.const 10 f32.const 17 f32.const 19 f32.const 7 call $icon_write_line
      return
    end
    local.get $icon
    i32.const 3
    i32.eq
    if
      local.get $out local.get $cap local.get $count f32.const 6 f32.const 6 f32.const 18 f32.const 18 call $icon_write_line local.set $count
      local.get $out local.get $cap local.get $count f32.const 18 f32.const 6 f32.const 6 f32.const 18 call $icon_write_line
      return
    end
    local.get $out local.get $cap local.get $count f32.const 6 f32.const 6 f32.const 18 f32.const 6 call $icon_write_line local.set $count
    local.get $out local.get $cap local.get $count f32.const 18 f32.const 6 f32.const 18 f32.const 18 call $icon_write_line local.set $count
    local.get $out local.get $cap local.get $count f32.const 18 f32.const 18 f32.const 6 f32.const 18 call $icon_write_line local.set $count
    local.get $out local.get $cap local.get $count f32.const 6 f32.const 18 f32.const 6 f32.const 6 call $icon_write_line)

  (func (export "er_ui_icon_segments_write_scaled") (param $icon i32) (param $out i32) (param $cap i32) (param $x f32) (param $y f32) (param $w f32) (param $h f32) (param $dpr f32) (result i32)
    (local $count i32)
    (local $fit_x f32)
    (local $fit_y f32)
    (local $size f32)
    (local $scale f32)
    local.get $w
    local.get $h
    call $valid_rect
    i32.eqz
    if
      i32.const -1
      return
    end
    local.get $cap
    local.get $icon
    call $er_ui_icon_segment_count
    i32.const 20
    i32.mul
    i32.lt_u
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
    local.get $size
    f32.const 24
    f32.div
    local.set $scale
    local.get $icon
    i32.const 44
    i32.eq
    local.get $icon
    i32.const 2
    i32.eq
    i32.or
    if
      local.get $out local.get $cap local.get $count local.get $fit_x local.get $fit_y local.get $scale local.get $dpr f32.const 6 f32.const 9 f32.const 12 f32.const 15 call $icon_write_line_scaled local.set $count
      local.get $out local.get $cap local.get $count local.get $fit_x local.get $fit_y local.get $scale local.get $dpr f32.const 12 f32.const 15 f32.const 18 f32.const 9 call $icon_write_line_scaled
      return
    end
    local.get $icon
    i32.const 55
    i32.eq
    if
      local.get $out local.get $cap local.get $count local.get $fit_x local.get $fit_y local.get $scale local.get $dpr f32.const 8 f32.const 5 f32.const 16 f32.const 5 call $icon_write_line_scaled local.set $count
      local.get $out local.get $cap local.get $count local.get $fit_x local.get $fit_y local.get $scale local.get $dpr f32.const 16 f32.const 5 f32.const 16 f32.const 13 call $icon_write_line_scaled local.set $count
      local.get $out local.get $cap local.get $count local.get $fit_x local.get $fit_y local.get $scale local.get $dpr f32.const 16 f32.const 13 f32.const 8 f32.const 13 call $icon_write_line_scaled local.set $count
      local.get $out local.get $cap local.get $count local.get $fit_x local.get $fit_y local.get $scale local.get $dpr f32.const 8 f32.const 13 f32.const 8 f32.const 5 call $icon_write_line_scaled local.set $count
      local.get $out local.get $cap local.get $count local.get $fit_x local.get $fit_y local.get $scale local.get $dpr f32.const 15 f32.const 14 f32.const 20 f32.const 19 call $icon_write_line_scaled
      return
    end
    local.get $icon
    i32.const 1
    i32.eq
    if
      local.get $out local.get $cap local.get $count local.get $fit_x local.get $fit_y local.get $scale local.get $dpr f32.const 5 f32.const 12 f32.const 10 f32.const 17 call $icon_write_line_scaled local.set $count
      local.get $out local.get $cap local.get $count local.get $fit_x local.get $fit_y local.get $scale local.get $dpr f32.const 10 f32.const 17 f32.const 19 f32.const 7 call $icon_write_line_scaled
      return
    end
    local.get $icon
    i32.const 3
    i32.eq
    if
      local.get $out local.get $cap local.get $count local.get $fit_x local.get $fit_y local.get $scale local.get $dpr f32.const 6 f32.const 6 f32.const 18 f32.const 18 call $icon_write_line_scaled local.set $count
      local.get $out local.get $cap local.get $count local.get $fit_x local.get $fit_y local.get $scale local.get $dpr f32.const 18 f32.const 6 f32.const 6 f32.const 18 call $icon_write_line_scaled
      return
    end
    local.get $out local.get $cap local.get $count local.get $fit_x local.get $fit_y local.get $scale local.get $dpr f32.const 6 f32.const 6 f32.const 18 f32.const 6 call $icon_write_line_scaled local.set $count
    local.get $out local.get $cap local.get $count local.get $fit_x local.get $fit_y local.get $scale local.get $dpr f32.const 18 f32.const 6 f32.const 18 f32.const 18 call $icon_write_line_scaled local.set $count
    local.get $out local.get $cap local.get $count local.get $fit_x local.get $fit_y local.get $scale local.get $dpr f32.const 18 f32.const 18 f32.const 6 f32.const 18 call $icon_write_line_scaled local.set $count
    local.get $out local.get $cap local.get $count local.get $fit_x local.get $fit_y local.get $scale local.get $dpr f32.const 6 f32.const 18 f32.const 6 f32.const 6 call $icon_write_line_scaled)

  (func (export "er_ui_rect_contains") (param $rect i32) (param $x f32) (param $y f32) (result i32)
    local.get $x
    local.get $rect
    f32.load
    f32.ge
    local.get $y
    local.get $rect
    i32.const 4
    i32.add
    f32.load
    f32.ge
    i32.and
    local.get $x
    local.get $rect
    f32.load
    local.get $rect
    i32.const 8
    i32.add
    f32.load
    f32.add
    f32.lt
    i32.and
    local.get $y
    local.get $rect
    i32.const 4
    i32.add
    f32.load
    local.get $rect
    i32.const 12
    i32.add
    f32.load
    f32.add
    f32.lt
    i32.and)

  (func $er_ui_writer_begin (export "er_ui_writer_begin") (param $base i32) (param $cap i32) (param $node_count i32) (param $root_count i32) (param $axis i32) (param $gap i32) (param $padding i32) (result i32)
    (local $records_len i32)
    local.get $node_count
    i32.eqz
    local.get $root_count
    i32.eqz
    i32.or
    local.get $root_count
    local.get $node_count
    i32.gt_u
    i32.or
    if
      i32.const 0
      return
    end
    local.get $node_count
    i32.const 16
    i32.mul
    local.set $records_len
    local.get $cap
    i32.const 20
    local.get $records_len
    i32.add
    i32.lt_u
    if
      i32.const 0
      return
    end
    local.get $base
    local.get $cap
    call $zero
    local.get $base
    i32.const 0x49755245
    call $store32
    local.get $base
    i32.const 4
    i32.add
    i32.const 0
    call $store32
    local.get $base
    i32.const 8
    i32.add
    i32.const 1
    call $store16
    local.get $base
    i32.const 10
    i32.add
    local.get $axis
    call $store16
    local.get $base
    i32.const 12
    i32.add
    local.get $gap
    call $store16
    local.get $base
    i32.const 14
    i32.add
    local.get $padding
    call $store16
    local.get $base
    i32.const 16
    i32.add
    local.get $node_count
    call $store16
    local.get $base
    i32.const 18
    i32.add
    local.get $root_count
    call $store16
    local.get $node_count
    global.set $writer_node_count
    local.get $base
    global.set $writer_base
    local.get $cap
    global.set $writer_cap
    local.get $base
    i32.const 20
    i32.add
    local.get $records_len
    i32.add
    global.set $writer_cursor
    global.get $writer_cursor)

  (func (export "er_ui_writer_cursor") (result i32)
    global.get $writer_cursor)

  (func $writer_store_used_len
    global.get $writer_base
    i32.const 4
    i32.add
    global.get $writer_cursor
    global.get $writer_base
    i32.sub
    call $store32)

  (func $er_ui_writer_string (export "er_ui_writer_string") (param $src i32) (param $len i32) (result i32)
    (local $offset i32)
    (local $table i32)
    local.get $len
    i32.const 65535
    i32.gt_u
    if
      i32.const 0
      return
    end
    global.get $writer_base
    global.get $writer_node_count
    call $table_start
    local.set $table
    global.get $writer_cursor
    local.get $table
    i32.sub
    local.set $offset
    local.get $offset
    i32.const 65535
    i32.gt_u
    if
      i32.const 0
      return
    end
    global.get $writer_cursor
    local.get $len
    i32.add
    global.get $writer_base
    global.get $writer_cap
    i32.add
    i32.gt_u
    if
      i32.const 0
      return
    end
    global.get $writer_cursor
    local.get $src
    local.get $len
    call $copy
    global.get $writer_cursor
    local.get $len
    i32.add
    global.set $writer_cursor
    call $writer_store_used_len
    local.get $offset
    local.get $len
    call $string_ref)

  (func $er_ui_writer_record (export "er_ui_writer_record") (param $index i32) (param $kind i32) (param $id i32) (param $first_ref i32) (param $second_ref i32) (result i32)
    (local $p i32)
    local.get $index
    global.get $writer_node_count
    i32.ge_u
    if
      i32.const 0
      return
    end
    local.get $index
    call $record_ptr
    local.set $p
    local.get $p
    local.get $kind
    call $store16
    local.get $p
    i32.const 4
    i32.add
    local.get $id
    call $store32
    local.get $p
    i32.const 8
    i32.add
    local.get $first_ref
    i32.const 65535
    i32.and
    call $store16
    local.get $p
    i32.const 10
    i32.add
    local.get $first_ref
    i32.const 16
    i32.shr_u
    call $store16
    local.get $p
    i32.const 12
    i32.add
    local.get $second_ref
    i32.const 65535
    i32.and
    call $store16
    local.get $p
    i32.const 14
    i32.add
    local.get $second_ref
    i32.const 16
    i32.shr_u
    call $store16
    call $writer_store_used_len
    i32.const 1)

  (func (export "er_ui_writer_record_child") (param $index i32) (param $parent i32) (param $kind i32) (param $id i32) (param $first_ref i32) (param $second_ref i32) (result i32)
    (local $p i32)
    local.get $index
    global.get $writer_node_count
    i32.ge_u
    local.get $parent
    global.get $writer_node_count
    i32.ge_u
    i32.or
    local.get $index
    local.get $parent
    i32.eq
    i32.or
    if
      i32.const 0
      return
    end
    local.get $index
    local.get $kind
    local.get $id
    local.get $first_ref
    local.get $second_ref
    call $er_ui_writer_record
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $index
    call $record_ptr
    local.set $p
    local.get $p
    i32.const 2
    i32.add
    local.get $parent
    i32.const 1
    i32.add
    call $store16
    i32.const 1)

  (func (export "er_ui_write_one_string") (param $base i32) (param $cap i32) (param $kind i32) (param $id i32) (param $src i32) (param $len i32) (result i32)
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

  (func $er_ui_measure (export "er_ui_measure") (param $base i32) (param $len i32) (param $index i32) (result i64)
    (local $node_count i32)
    (local $axis i32)
    (local $gap f32)
    (local $padding f32)
    (local $record i32)
    (local $kind i32)
    (local $first_len i32)
    (local $mw f32)
    (local $mh f32)
    (local $child_i i32)
    (local $child_record i32)
    (local $child_kind i32)
    (local $child_first_len i32)
    (local $child_w f32)
    (local $child_h f32)
    (local $ancestor_ref i32)
    (local $ancestor_index i32)
    (local $is_descendant i32)
    (local $desc_depth i32)
    (local $direct_count i32)
    (local $direct_main f32)
    (local $direct_cross f32)
    (local $wb i32)
    (local $hb i32)
    local.get $base
    local.get $len
    call $er_ui_validate_deep
    i32.eqz
    if
      i64.const 0
      return
    end
    local.get $base
    i32.const 16
    i32.add
    call $load16
    local.set $node_count
    local.get $base
    i32.const 10
    i32.add
    call $load16
    local.set $axis
    local.get $base
    i32.const 12
    i32.add
    call $load16
    f32.convert_i32_u
    local.set $gap
    local.get $base
    i32.const 14
    i32.add
    call $load16
    f32.convert_i32_u
    local.set $padding
    local.get $index
    local.get $node_count
    i32.ge_u
    if
      i64.const 0
      return
    end
    local.get $base
    i32.const 20
    i32.add
    local.get $index
    i32.const 16
    i32.mul
    i32.add
    local.set $record
    local.get $record
    call $load16
    local.set $kind
    local.get $record
    i32.const 10
    i32.add
    call $load16
    local.set $first_len
    local.get $kind
    local.get $first_len
    call $preferred_w
    local.set $mw
    local.get $kind
    call $preferred_h
    local.set $mh
    i32.const 0
    local.set $child_i
    block $measure_children_done
      loop $measure_children
        local.get $child_i
        local.get $node_count
        i32.ge_u
        br_if $measure_children_done
        local.get $child_i
        local.get $index
        i32.eq
        if
          local.get $child_i
          i32.const 1
          i32.add
          local.set $child_i
          br $measure_children
        end
        local.get $base
        i32.const 20
        i32.add
        local.get $child_i
        i32.const 16
        i32.mul
        i32.add
        local.set $child_record
        i32.const 0
        local.set $is_descendant
        i32.const 0
        local.set $desc_depth
        local.get $child_record
        i32.const 2
        i32.add
        call $load16
        local.tee $ancestor_ref
        i32.eqz
        i32.eqz
        if
          block $measure_ancestor_done
            loop $measure_ancestor_loop
              local.get $ancestor_ref
              i32.const 1
              i32.sub
              local.tee $ancestor_index
              local.get $index
              i32.eq
              if
                i32.const 1
                local.set $is_descendant
                br $measure_ancestor_done
              end
              local.get $desc_depth
              i32.const 1
              i32.add
              local.set $desc_depth
              local.get $base
              local.get $ancestor_index
              call $record_at
              i32.const 2
              i32.add
              call $load16
              local.tee $ancestor_ref
              i32.eqz
              br_if $measure_ancestor_done
              br $measure_ancestor_loop
            end
          end
        end
        local.get $is_descendant
        if
          local.get $child_record
          call $load16
          local.set $child_kind
          local.get $child_record
          i32.const 10
          i32.add
          call $load16
          local.set $child_first_len
          local.get $child_kind
          local.get $child_first_len
          call $preferred_w
          f32.const 24
          local.get $desc_depth
          f32.convert_i32_u
          f32.const 16
          f32.mul
          f32.add
          f32.add
          local.set $child_w
          local.get $child_kind
          call $preferred_h
          f32.const 16
          local.get $desc_depth
          f32.convert_i32_u
          f32.const 8
          f32.mul
          f32.add
          f32.add
          local.set $child_h
          local.get $desc_depth
          i32.eqz
          if
            local.get $axis
            i32.eqz
            if
              local.get $direct_main
              local.get $child_h
              f32.add
              local.set $direct_main
              local.get $direct_cross
              local.get $child_w
              call $max_f32
              local.set $direct_cross
            else
              local.get $direct_main
              local.get $child_w
              f32.add
              local.set $direct_main
              local.get $direct_cross
              local.get $child_h
              call $max_f32
              local.set $direct_cross
            end
            local.get $direct_count
            i32.const 1
            i32.add
            local.set $direct_count
          end
          local.get $mw
          local.get $child_w
          call $max_f32
          local.set $mw
          local.get $mh
          local.get $child_h
          call $max_f32
          local.set $mh
        end
        local.get $child_i
        i32.const 1
        i32.add
        local.set $child_i
        br $measure_children
      end
    end
    local.get $direct_count
    if
      local.get $direct_main
      local.get $direct_count
      i32.const 1
      i32.sub
      f32.convert_i32_u
      local.get $gap
      f32.mul
      f32.add
      local.get $padding
      f32.const 2
      f32.mul
      f32.add
      local.set $direct_main
      local.get $direct_cross
      local.get $padding
      f32.const 2
      f32.mul
      f32.add
      local.set $direct_cross
      local.get $axis
      i32.eqz
      if
        local.get $mw
        local.get $direct_cross
        call $max_f32
        local.set $mw
        local.get $mh
        local.get $direct_main
        call $max_f32
        local.set $mh
      else
        local.get $mw
        local.get $direct_main
        call $max_f32
        local.set $mw
        local.get $mh
        local.get $direct_cross
        call $max_f32
        local.set $mh
      end
    end
    local.get $mw
    i32.reinterpret_f32
    local.set $wb
    local.get $mh
    i32.reinterpret_f32
    local.set $hb
    local.get $wb
    i64.extend_i32_u
    i64.const 32
    i64.shl
    local.get $hb
    i64.extend_i32_u
    i64.or)

  (func $er_ui_measure_packed_width (export "er_ui_measure_packed_width") (param $measure i64) (result f32)
    local.get $measure
    i64.const 32
    i64.shr_u
    i32.wrap_i64
    f32.reinterpret_i32)

  (func $er_ui_measure_packed_height (export "er_ui_measure_packed_height") (param $measure i64) (result f32)
    local.get $measure
    i32.wrap_i64
    f32.reinterpret_i32)

  (func (export "er_ui_measure_width") (param $base i32) (param $len i32) (param $index i32) (result f32)
    local.get $base
    local.get $len
    local.get $index
    call $er_ui_measure
    call $er_ui_measure_packed_width)

  (func (export "er_ui_measure_height") (param $base i32) (param $len i32) (param $index i32) (result f32)
    local.get $base
    local.get $len
    local.get $index
    call $er_ui_measure
    call $er_ui_measure_packed_height)
