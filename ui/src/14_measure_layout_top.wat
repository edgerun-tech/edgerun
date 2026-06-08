

  (func $er_ui_measure  (param $base i32) (param $len i32) (param $index i32) (result i64)
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

  (func $er_ui_measure_packed_width  (param $measure i64) (result f32)
    local.get $measure
    i64.const 32
    i64.shr_u
    i32.wrap_i64
    f32.reinterpret_i32)

  (func $er_ui_measure_packed_height  (param $measure i64) (result f32)
    local.get $measure
    i32.wrap_i64
    f32.reinterpret_i32)

  (func $er_ui_measure_width  (param $base i32) (param $len i32) (param $index i32) (result f32)
    local.get $base
    local.get $len
    local.get $index
    call $er_ui_measure
    call $er_ui_measure_packed_width)

  (func $er_ui_measure_height  (param $base i32) (param $len i32) (param $index i32) (result f32)
    local.get $base
    local.get $len
    local.get $index
    call $er_ui_measure
    call $er_ui_measure_packed_height)
  (func $er_ui_layout_set_buf (export "er_ui_layout_set_buf") (param $buf i32)
    local.get $buf
    global.set $ui_layout_buf)
  ;; Buffer entry: [x:f32, y:f32, w:f32, h:f32] — 16 bytes per node; w < 0 = invisible
  (func $er_ui_layout (export "er_ui_layout") (param $base i32) (param $len i32) (param $vp_w f32) (param $vp_h f32) (result i32)
    (local $node_count i32) (local $axis i32) (local $gap f32) (local $padding f32)
    (local $i i32) (local $record i32) (local $kind i32)
    (local $cw f32) (local $ch f32) (local $cx f32) (local $cy f32)
    (local $root_cw f32) (local $root_ch f32)
    (local $ancestor_ref i32) (local $ancestor_index i32)
    (local $is_descendant i32) (local $desc_depth i32)
    (local $child_i i32) (local $lb i32) (local $first_len i32) (local $first_ref i32)
    local.get $base
    local.get $len
    call $er_ui_validate_deep
    i32.eqz
    if
      i32.const -1
      return
    end
    local.get $base
    i32.load
    i32.const 0x49755245
    i32.ne
    if
      i32.const -1
      return
    end
    local.get $base
    i32.const 16
    i32.add
    call $load16
    local.set $node_count
    local.get $node_count
    i32.eqz
    if
      i32.const -1
      return
    end
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
    local.get $padding
    local.set $cx
    local.get $padding
    local.set $cy
    local.get $vp_w
    local.get $padding
    f32.const 2
    f32.mul
    f32.sub
    local.set $root_cw
    local.get $vp_h
    local.get $padding
    f32.const 2
    f32.mul
    f32.sub
    local.set $root_ch
    loop $loop
      local.get $i
      local.get $node_count
      i32.lt_u
      if
        local.get $base
        i32.const 20
        i32.add
        local.get $i
        i32.const 16
        i32.mul
        i32.add
        local.set $record
        local.get $record
        i32.const 2
        i32.add
        call $load16
        local.tee $first_ref
        i32.eqz
        i32.eqz
        if
          local.get $i
          i32.const 1
          i32.add
          local.set $i
          br $loop
        end
        local.get $record
        call $load16
        local.set $kind
        ;; Resolve first string length for preferred_w
        local.get $record
        i32.const 8
        i32.add
        call $load16
        local.get $record
        i32.const 10
        i32.add
        call $load16
        call $string_ref
        call $string_len
        local.set $first_len
        ;; Compute preferred dimensions from component kind
        local.get $kind
        local.get $first_len
        call $preferred_w
        local.set $cw
        local.get $kind
        call $preferred_h
        local.set $ch
        ;; Override based on layout axis
        local.get $axis
        i32.const 0
        i32.eq
        if
          local.get $root_cw
          local.set $cw
        else
          local.get $root_ch
          local.set $ch
        end
        local.get $cw
        local.get $ch
        call $valid_rect
        if
          local.get $i
          i32.const 4
          i32.shl
          global.get $ui_layout_buf
          i32.add
          local.set $lb
          local.get $lb
          local.get $cx
          f32.store offset=0
          local.get $lb
          local.get $cy
          f32.store offset=4
          local.get $lb
          local.get $cw
          f32.store offset=8
          local.get $lb
          local.get $ch
          f32.store offset=12
          local.get $cw
          local.set $root_cw
          local.get $ch
          local.set $root_ch
        end
        ;; Children loop
        i32.const 0
        local.set $child_i
        block $children_done
          loop $children
            local.get $child_i
            local.get $node_count
            i32.ge_u
            br_if $children_done
            local.get $base
            i32.const 20
            i32.add
            local.get $child_i
            i32.const 16
            i32.mul
            i32.add
            local.set $record
            i32.const 0
            local.set $is_descendant
            i32.const 0
            local.set $desc_depth
            local.get $record
            i32.const 2
            i32.add
            call $load16
            local.tee $ancestor_ref
            i32.eqz
            i32.eqz
            if
              block $ancestor_done
                loop $ancestor_loop
                  local.get $ancestor_ref
                  i32.const 1
                  i32.sub
                  local.tee $ancestor_index
                  local.get $i
                  i32.eq
                  if
                    i32.const 1
                    local.set $is_descendant
                    br $ancestor_done
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
                  br_if $ancestor_done
                  br $ancestor_loop
                end
              end
            end
            local.get $is_descendant
            if
              local.get $record
              call $load16
              local.set $kind
              local.get $root_cw
              f32.const 24
              local.get $desc_depth
              f32.convert_i32_u
              f32.const 16
              f32.mul
              f32.add
              f32.sub
              local.set $cw
              local.get $root_ch
              f32.const 16
              local.get $desc_depth
              f32.convert_i32_u
              f32.const 8
              f32.mul
              f32.add
              f32.sub
              local.set $ch
              local.get $cw
              local.get $ch
              call $valid_rect
              if
                local.get $child_i
                i32.const 4
                i32.shl
                global.get $ui_layout_buf
                i32.add
                local.set $lb
                local.get $lb
                local.get $cx
                f32.const 12
                f32.add
                local.get $desc_depth
                f32.convert_i32_u
                f32.const 8
                f32.mul
                f32.add
                f32.store offset=0
                local.get $lb
                local.get $cy
                f32.const 8
                f32.add
                local.get $desc_depth
                f32.convert_i32_u
                f32.const 4
                f32.mul
                f32.add
                f32.store offset=4
                local.get $lb
                local.get $cw
                f32.store offset=8
                local.get $lb
                local.get $ch
                f32.store offset=12
              end
            end
            local.get $child_i
            i32.const 1
            i32.add
            local.set $child_i
            br $children
          end
        end
        ;; Advance root position
        local.get $axis
        i32.const 0
        i32.eq
        if
          local.get $cy
          local.get $root_ch
          local.get $gap
          f32.add
          f32.add
          local.set $cy
        else
          local.get $cx
          local.get $root_cw
          local.get $gap
          f32.add
          f32.add
          local.set $cx
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $loop
      end
    end
    local.get $node_count)

  (func $er_ui_layout_get  (param $index i32) (param $out i32)
    (local $p i32)
    local.get $index
    i32.const 4
    i32.shl
    global.get $ui_layout_buf
    i32.add
    local.set $p
    local.get $out
    local.get $p
    f32.load offset=0
    f32.store offset=0
    local.get $out
    local.get $p
    f32.load offset=4
    f32.store offset=4
    local.get $out
    local.get $p
    f32.load offset=8
    f32.store offset=8
    local.get $out
    local.get $p
    f32.load offset=12
    f32.store offset=12)

  (func $er_ui_layout_hit_test (export "er_ui_layout_hit_test") (param $px f32) (param $py f32) (param $node_count i32) (result i32)
    (local $i i32) (local $p i32) (local $x f32) (local $y f32) (local $w f32) (local $h f32) (local $hit i32)
    i32.const -1
    local.set $hit
    block $done
      loop $loop
        local.get $i
        local.get $node_count
        i32.ge_u
        br_if $done
        local.get $i
        i32.const 4
        i32.shl
        global.get $ui_layout_buf
        i32.add
        local.set $p
        local.get $p
        f32.load offset=8
        local.tee $w
        f32.const 0
        f32.lt
        if
          local.get $i
          i32.const 1
          i32.add
          local.set $i
          br $loop
        end
        local.get $p
        f32.load offset=0
        local.set $x
        local.get $p
        f32.load offset=4
        local.set $y
        local.get $p
        f32.load offset=12
        local.set $h
        local.get $px
        local.get $x
        f32.ge
        local.get $px
        local.get $x
        local.get $w
        f32.add
        f32.lt
        i32.and
        local.get $py
        local.get $y
        f32.ge
        local.get $py
        local.get $y
        local.get $h
        f32.add
        f32.lt
        i32.and
        i32.and
        if
          local.get $i
          local.set $hit
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $loop
      end
    end
    local.get $hit)

  (func $er_ui_render (export "er_ui_render") (param $base i32) (param $len i32) (param $out i32) (param $cap i32) (param $x f32) (param $y f32) (param $w f32) (param $h f32) (result i32)
    (local $node_count i32)
    (local $axis i32)
    (local $gap f32)
    (local $padding f32)
    (local $i i32)
    (local $record i32)
    (local $kind i32)
    (local $id i32)
    (local $first_ref i32)
    (local $second_ref i32)
    (local $first_len i32)
    (local $second_len i32)
    (local $first_ptr i32)
    (local $second_ptr i32)
    (local $cw f32)
    (local $ch f32)
    (local $cx f32)
    (local $cy f32)
    (local $count i32)
    (local $prev_count i32)
    (local $child_i i32)
    (local $root_cw f32)
    (local $root_ch f32)
    (local $fill_w f32)
    (local $ancestor_ref i32)
    (local $ancestor_index i32)
    (local $is_descendant i32)
    (local $desc_depth i32)
    (local $state_value i32)
    local.get $base
    local.get $len
    call $er_ui_validate_deep
    i32.eqz
    if
      i32.const -1
      return
    end
    local.get $base
    i32.load
    i32.const 0x49755245
    i32.ne
    if
      i32.const -1
      return
    end
    local.get $base
    i32.const 16
    i32.add
    call $load16
    local.set $node_count
    local.get $node_count
    i32.eqz
    if
      i32.const -1
      return
    end
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
    local.get $x
    local.get $padding
    f32.add
    local.set $cx
    local.get $y
    local.get $padding
    f32.add
    local.set $cy
    loop $loop
      local.get $i
      local.get $node_count
      i32.lt_u
      if
        local.get $base
        i32.const 20
        i32.add
        local.get $i
        i32.const 16
        i32.mul
        i32.add
        local.set $record
        local.get $record
        i32.const 2
        i32.add
        call $load16
        i32.eqz
        i32.eqz
        if
          local.get $i
          i32.const 1
          i32.add
          local.set $i
          br $loop
        end
        local.get $record
        call $load16
        local.set $kind
        local.get $record
        i32.const 4
        i32.add
        i32.load
        local.set $id
        local.get $record
        i32.const 8
        i32.add
        call $load16
        local.get $record
        i32.const 10
        i32.add
        call $load16
        call $string_ref
        local.set $first_ref
        local.get $record
        i32.const 12
        i32.add
        call $load16
        local.get $record
        i32.const 14
        i32.add
        call $load16
        call $string_ref
        local.set $second_ref
        local.get $first_ref
        call $string_len
        local.set $first_len
        local.get $second_ref
        call $string_len
        local.set $second_len
        local.get $base
        local.get $node_count
        local.get $first_ref
        call $string_ptr
        local.set $first_ptr
        local.get $kind
        call $second_ref_is_packed
        i32.eqz
        local.get $second_len
        i32.eqz
        i32.eqz
        i32.and
        if
          local.get $base
          local.get $node_count
          local.get $second_ref
          call $string_ptr
          local.set $second_ptr
        end
        local.get $kind
        local.get $first_len
        call $preferred_w
        local.set $cw
        local.get $kind
        call $preferred_h
        local.set $ch
        local.get $axis
        i32.const 0
        i32.eq
        if
          local.get $w
          local.get $padding
          f32.const 2
          f32.mul
          f32.sub
          local.set $cw
        else
          local.get $h
          local.get $padding
          f32.const 2
          f32.mul
          f32.sub
          local.set $ch
        end
        local.get $cw
        local.get $ch
        call $valid_rect
        if
          local.get $cw
          local.set $root_cw
          local.get $ch
          local.set $root_ch
          local.get $kind
          i32.const 0
          i32.eq
          if
            local.get $count
            local.set $prev_count
            local.get $out
            local.get $cap
            local.get $count
            local.get $cx
            local.get $cy
            local.get $cw
            local.get $ch
            i32.const 0
            call $emit_rect
            local.set $count
            local.get $count
            local.get $prev_count
            i32.gt_s
            if
              local.get $out
              local.get $prev_count
              local.get $id
              local.get $kind
              i32.const 1
              call $command_set_owner_at
            end
          else
            local.get $kind
            i32.const 1
            i32.eq
            if
              local.get $count
              local.set $prev_count
              local.get $out
              local.get $cap
              local.get $count
              local.get $cx
              local.get $cy
              local.get $cw
              local.get $ch
              local.get $first_ptr
              local.get $first_len
              i32.const 0xfff7eee8
              call $emit_text
              local.set $count
              local.get $count
              local.get $prev_count
              i32.gt_s
              if
                local.get $out
                local.get $prev_count
                local.get $id
                local.get $kind
                i32.const 2
                call $command_set_owner_at
              end
            else
              local.get $count
              local.set $prev_count
              local.get $out
              local.get $cap
              local.get $count
              local.get $cx
              local.get $cy
              local.get $cw
              local.get $ch
              i32.const 0xff2a1f18
              call $emit_rect
              local.set $count
              local.get $count
              local.get $prev_count
              i32.gt_s
              if
                local.get $out
                local.get $prev_count
                local.get $id
                local.get $kind
                i32.const 1
                call $command_set_owner_at
              end
              local.get $count
              i32.const -1
              i32.ne
              if
                local.get $kind
                i32.const 46
                i32.eq
                local.get $kind
                i32.const 47
                i32.eq
                i32.or
                local.get $kind
                i32.const 52
                i32.eq
                i32.or
                local.get $kind
                i32.const 53
                i32.eq
                i32.or
                if
                  local.get $count
                  local.set $prev_count
                  local.get $out
                  local.get $cap
                  local.get $count
                  local.get $cx
                  f32.const 4
                  f32.add
                  local.get $cy
                  f32.const 4
                  f32.add
                  local.get $cw
                  f32.const 8
                  f32.sub
                  local.get $ch
                  f32.const 8
                  f32.sub
                  i32.const 0xaa000000
                  call $emit_rect
                  local.set $count
                  local.get $count
                  local.get $prev_count
                  i32.gt_s
                  if
                    local.get $out
                    local.get $prev_count
                    local.get $id
                    local.get $kind
                    i32.const 5
                    call $command_set_owner_at
                  end
                end
              end
              local.get $count
              i32.const -1
              i32.ne
              if
                local.get $kind
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
                if
                  local.get $count
                  local.set $prev_count
                  local.get $out
                  local.get $cap
                  local.get $count
                  local.get $cx
                  f32.const 4
                  f32.add
                  local.get $cy
                  f32.const 4
                  f32.add
                  f32.const 16
                  local.get $ch
                  f32.const 8
                  f32.sub
                  local.get $second_ref
                  i32.const 65535
                  i32.and
                  if (result i32)
                    i32.const 0xff20c878
                  else
                    i32.const 0xff6d625a
                  end
                  call $emit_rect
                  local.set $count
                  local.get $count
                  local.get $prev_count
                  i32.gt_s
                  if
                    local.get $out
                    local.get $prev_count
                    local.get $id
                    local.get $kind
                    i32.const 3
                    call $command_set_owner_at
                    local.get $out
                    local.get $prev_count
                    local.get $second_ref
                    i32.const 65535
                    i32.and
                    call $command_set_meta_at
                  end
                else
                  local.get $kind
                  i32.const 24
                  i32.eq
                  local.get $kind
                  i32.const 37
                  i32.eq
                  i32.or
                  local.get $kind
                  i32.const 57
                  i32.eq
                  i32.or
                  if
                    local.get $second_ref
                    i32.const 65535
                    i32.and
                    f32.convert_i32_u
                    f32.const 65535
                    f32.div
                    local.get $cw
                    f32.const 8
                    f32.sub
                    f32.mul
                    local.set $fill_w
                    local.get $fill_w
                    f32.const 0
                    f32.gt
                    if
                      local.get $count
                      local.set $prev_count
                      local.get $out
                      local.get $cap
                      local.get $count
                      local.get $cx
                      f32.const 4
                      f32.add
                      local.get $cy
                      f32.const 4
                      f32.add
                      local.get $fill_w
                      local.get $ch
                      f32.const 8
                      f32.sub
                      i32.const 0xff20a4f3
                      call $emit_rect
                      local.set $count
                      local.get $count
                      local.get $prev_count
                      i32.gt_s
                      if
                        local.get $out
                        local.get $prev_count
                        local.get $id
                        local.get $kind
                        i32.const 3
                        call $command_set_owner_at
                        local.get $out
                        local.get $prev_count
                        local.get $second_ref
                        i32.const 65535
                        i32.and
                        call $command_set_meta_at
                  end
                end
                local.get $kind
                call $second_ref_is_packed
                i32.eqz
                local.get $second_len
                i32.eqz
                i32.eqz
                i32.and
                local.get $count
                i32.const -1
                i32.ne
                i32.and
                if
                  local.get $count
                  local.set $prev_count
                  local.get $out
                  local.get $cap
                  local.get $count
                  local.get $cx
                  f32.const 12
                  f32.add
                  local.get $cy
                  local.get $ch
                  f32.const 0.5
                  f32.mul
                  f32.add
                  local.get $cw
                  f32.const 24
                  f32.sub
                  local.get $ch
                  f32.const 16
                  f32.sub
                  local.get $second_ptr
                  local.get $second_len
                  i32.const 0xffc9b8ad
                  call $emit_text
                  local.set $count
                  local.get $count
                  local.get $prev_count
                  i32.gt_s
                  if
                    local.get $out
                    local.get $prev_count
                    local.get $id
                    local.get $kind
                    i32.const 4
                    call $command_set_owner_at
                  end
                end
              end
            end
          end
              local.get $count
              i32.const -1
              i32.ne
                if
                  local.get $kind
                  i32.const 16
                i32.eq
                local.get $kind
                i32.const 20
                i32.eq
                i32.or
                local.get $kind
                i32.const 44
                i32.eq
                i32.or
                local.get $kind
                i32.const 55
                i32.eq
                i32.or
                if
                  local.get $count
                  local.set $prev_count
                  local.get $out
                  local.get $cap
                  local.get $count
                  local.get $cx
                  local.get $cw
                  f32.add
                  f32.const 20
                  f32.sub
                  local.get $cy
                  f32.const 6
                  f32.add
                  f32.const 14
                  f32.const 14
                  i32.const 0xffd6a84f
                  call $emit_rect
                  local.set $count
                  local.get $count
                  local.get $prev_count
                  i32.gt_s
                  if
                    local.get $out
                    local.get $prev_count
                    local.get $id
                    local.get $kind
                    i32.const 6
                    call $command_set_owner_at
                  end
                end
                local.get $kind
                i32.const 12
                i32.eq
                local.get $kind
                i32.const 16
                i32.eq
                i32.or
                local.get $kind
                i32.const 20
                i32.eq
                i32.or
                local.get $kind
                i32.const 44
                i32.eq
                i32.or
                local.get $kind
                i32.const 55
                i32.eq
                i32.or
                local.get $second_ref
                i32.const 65535
                i32.and
                i32.const 0
                i32.ne
                i32.and
                if
                  local.get $count
                  local.set $prev_count
                  local.get $out
                  local.get $cap
                  local.get $count
                  local.get $cx
                  f32.const 4
                  f32.add
                  local.get $cy
                  f32.const 6
                  f32.add
                  f32.const 12
                  f32.const 12
                  local.get $second_ref
                  i32.const 65535
                  i32.and
                  i32.const 0xff9fb6ff
                  call $emit_icon_or_rect
                  local.set $count
                  local.get $count
                  local.get $prev_count
                  i32.gt_s
                  if
                    local.get $out
                    local.get $prev_count
                    local.get $id
                    local.get $kind
                    i32.const 9
                    call $command_set_owner_at
                    local.get $out
                    local.get $prev_count
                    local.get $second_ref
                    i32.const 65535
                    i32.and
                    call $command_set_meta_at
                  end
                end
                local.get $kind
                local.get $id
                call $record_encoded_state
                local.tee $state_value
                i32.const 0
                i32.gt_s
                if
                  local.get $count
                  local.set $prev_count
                  local.get $out
                  local.get $cap
                  local.get $count
                  local.get $cx
                  f32.const 12
                  f32.add
                  local.get $state_value
                  f32.convert_i32_u
                  f32.const 20
                  f32.mul
                  f32.add
                  local.get $cy
                  local.get $ch
                  f32.add
                  f32.const 8
                  f32.sub
                  f32.const 16
                  f32.const 4
                  i32.const 0xffffcc55
                  call $emit_rect
                  local.set $count
                  local.get $count
                  local.get $prev_count
                  i32.gt_s
                  if
                    local.get $out
                    local.get $prev_count
                    local.get $id
                    local.get $kind
                    i32.const 10
                    call $command_set_owner_at
                    local.get $out
                    local.get $prev_count
                    local.get $state_value
                    call $command_set_meta_at
                  end
                end
                local.get $kind
                i32.const 32
                i32.eq
                if
                  local.get $count
                  local.set $prev_count
                  local.get $out
                  local.get $cap
                  local.get $count
                  local.get $cx
                  f32.const 8
                  f32.add
                  local.get $cy
                  f32.const 6
                  f32.add
                  local.get $cw
                  f32.const 16
                  f32.sub
                  f32.const 12
                  i32.const 0xff4d3f36
                  call $emit_rect
                  local.set $count
                  local.get $count
                  local.get $prev_count
                  i32.gt_s
                  if
                    local.get $out
                    local.get $prev_count
                    local.get $id
                    local.get $kind
                    i32.const 7
                    call $command_set_owner_at
                  end
                end
                local.get $kind
                i32.const 4
                i32.eq
                local.get $kind
                i32.const 11
                i32.eq
                i32.or
                local.get $kind
                i32.const 26
                i32.eq
                i32.or
                local.get $kind
                i32.const 27
                i32.eq
                i32.or
                if
                  local.get $count
                  local.set $prev_count
                  local.get $out
                  local.get $cap
                  local.get $count
                  local.get $cx
                  f32.const 4
                  f32.add
                  local.get $cy
                  f32.const 6
                  f32.add
                  f32.const 12
                  f32.const 12
                  i32.const 0xff9fb6ff
                  call $emit_rect
                  local.set $count
                  local.get $count
                  local.get $prev_count
                  i32.gt_s
                  if
                    local.get $out
                    local.get $prev_count
                    local.get $id
                    local.get $kind
                    i32.const 9
                    call $command_set_owner_at
                    local.get $out
                    local.get $prev_count
                    local.get $kind
                    i32.const 26
                    i32.eq
                    if (result i32)
                      local.get $id
                      i32.const 0x3fff
                      i32.and
                    else
                      local.get $kind
                      i32.const 4
                      i32.eq
                      local.get $kind
                      i32.const 11
                      i32.eq
                      i32.or
                      if (result i32)
                        local.get $id
                        i32.const 65535
                        i32.and
                      else
                        local.get $second_ref
                        i32.const 65535
                        i32.and
                      end
                    end
                    call $command_set_meta_at
                  end
                end
                local.get $kind
                i32.const 39
                i32.eq
                local.get $kind
                i32.const 40
                i32.eq
                i32.or
                local.get $kind
                i32.const 54
                i32.eq
                i32.or
                if
                  local.get $count
                  local.set $prev_count
                  local.get $out
                  local.get $cap
                  local.get $count
                  local.get $cx
                  f32.const 8
                  f32.add
                  local.get $cy
                  f32.const 6
                  f32.add
                  f32.const 20
                  local.get $ch
                  f32.const 12
                  f32.sub
                  i32.const 0xff5a463b
                  call $emit_rect
                  local.set $count
                  local.get $count
                  local.get $prev_count
                  i32.gt_s
                  if
                    local.get $out
                    local.get $prev_count
                    local.get $id
                    local.get $kind
                    i32.const 8
                    call $command_set_owner_at
                  end
                end
              end
              local.get $count
              i32.const -1
              i32.ne
              if
                local.get $count
                local.set $prev_count
                local.get $out
                local.get $cap
                local.get $count
                local.get $cx
                f32.const 12
                f32.add
                local.get $cy
                f32.const 8
                f32.add
                local.get $cw
                f32.const 24
                f32.sub
                local.get $ch
                f32.const 16
                f32.sub
                local.get $first_ptr
                local.get $first_len
                i32.const 0xfff7eee8
                call $emit_text
                local.set $count
                local.get $count
                local.get $prev_count
                i32.gt_s
                if
                  local.get $out
                  local.get $prev_count
                  local.get $id
                  local.get $kind
                  i32.const 2
                  call $command_set_owner_at
                end
                local.get $kind
                call $second_ref_is_packed
                i32.eqz
                local.get $second_len
                i32.eqz
                i32.eqz
                i32.and
                local.get $count
                i32.const -1
                i32.ne
                i32.and
                if
                  local.get $count
                  local.set $prev_count
                  local.get $out
                  local.get $cap
                  local.get $count
                  local.get $cx
                  f32.const 12
                  f32.add
                  local.get $cy
                  local.get $ch
                  f32.const 0.5
                  f32.mul
                  f32.add
                  local.get $cw
                  f32.const 24
                  f32.sub
                  local.get $ch
                  f32.const 16
                  f32.sub
                  local.get $second_ptr
                  local.get $second_len
                  i32.const 0xffc9b8ad
                  call $emit_text
                  local.set $count
                  local.get $count
                  local.get $prev_count
                  i32.gt_s
                  if
                    local.get $out
                    local.get $prev_count
                    local.get $id
                    local.get $kind
                    i32.const 4
                    call $command_set_owner_at
                  end
                end
              end
            end
          end
        end
        local.get $count
        i32.const -1
        i32.eq
        if
          i32.const -1
          return
        end
        i32.const 0
        local.set $child_i
        block $children_done
          loop $children
            local.get $child_i
            local.get $node_count
            i32.ge_u
            br_if $children_done
            local.get $base
            i32.const 20
            i32.add
            local.get $child_i
            i32.const 16
            i32.mul
            i32.add
            local.set $record
            global.get $ui_layout_buf
            if
              local.get $child_i
              i32.const 4
              i32.shl
              global.get $ui_layout_buf
              i32.add
              local.set $ancestor_ref
              local.get $ancestor_ref
              f32.load offset=8
              local.tee $cw
              f32.const 0
              f32.gt
              if
                i32.const 1
                local.set $is_descendant
                local.get $ancestor_ref
                f32.load offset=12
                local.set $ch
                local.get $root_cw
                f32.const 24
                f32.sub
                local.get $cw
                f32.sub
                f32.const 16
                f32.div
                f32.const 0.0001
                f32.add
                i32.trunc_f32_s
                local.set $desc_depth
              else
                i32.const 0
                local.set $is_descendant
              end
            else
              i32.const 0
              local.set $is_descendant
              i32.const 0
              local.set $desc_depth
              local.get $record
              i32.const 2
              i32.add
              call $load16
              local.tee $ancestor_ref
              i32.eqz
              i32.eqz
              if
                block $ancestor_done
                  loop $ancestor_loop
                    local.get $ancestor_ref
                    i32.const 1
                    i32.sub
                    local.tee $ancestor_index
                    local.get $i
                    i32.eq
                    if
                      i32.const 1
                      local.set $is_descendant
                      br $ancestor_done
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
                    br_if $ancestor_done
                    br $ancestor_loop
                  end
                end
              end
            end
            local.get $is_descendant
            if
              local.get $record
              call $load16
              local.set $kind
              local.get $record
              i32.const 4
              i32.add
              i32.load
              local.set $id
              local.get $record
              i32.const 8
              i32.add
              call $load16
              local.get $record
              i32.const 10
              i32.add
              call $load16
              call $string_ref
              local.set $first_ref
              local.get $first_ref
              call $string_len
              local.set $first_len
              local.get $base
              local.get $node_count
              local.get $first_ref
              call $string_ptr
              local.set $first_ptr
              local.get $record
              i32.const 12
              i32.add
              call $load16
              local.get $record
              i32.const 14
              i32.add
              call $load16
              call $string_ref
              local.set $second_ref
              local.get $second_ref
              call $string_len
              local.set $second_len
                local.get $kind
                call $second_ref_is_packed
                i32.eqz
                local.get $second_len
                i32.eqz
                i32.eqz
                i32.and
                if
                  local.get $base
                  local.get $node_count
                  local.get $second_ref
                  call $string_ptr
                  local.set $second_ptr
                end
                global.get $ui_layout_buf
                i32.eqz
                if
                  local.get $root_cw
                  f32.const 24
                  local.get $desc_depth
                  f32.convert_i32_u
                  f32.const 16
                  f32.mul
                  f32.add
                  f32.sub
                  local.set $cw
                  local.get $root_ch
                  f32.const 16
                  local.get $desc_depth
                  f32.convert_i32_u
                  f32.const 8
                  f32.mul
                  f32.add
                  f32.sub
                  local.set $ch
                end
              local.get $cw
              local.get $ch
              call $valid_rect
              if
                local.get $kind
                i32.const 1
                i32.eq
                if
                  local.get $count
                  local.set $prev_count
                  local.get $out
                  local.get $cap
                  local.get $count
                  local.get $cx
                  f32.const 12
                  local.get $desc_depth
                  f32.convert_i32_u
                  f32.const 8
                  f32.mul
                  f32.add
                  f32.add
                  local.get $cy
                  f32.const 8
                  local.get $desc_depth
                  f32.convert_i32_u
                  f32.const 4
                  f32.mul
                  f32.add
                  f32.add
                  local.get $cw
                  local.get $ch
                  local.get $first_ptr
                  local.get $first_len
                  i32.const 0xfff7eee8
                  call $emit_text
                  local.set $count
                  local.get $count
                  local.get $prev_count
                  i32.gt_s
                  if
                    local.get $out
                    local.get $prev_count
                    local.get $id
                    local.get $kind
                    i32.const 2
                    call $command_set_owner_at
                  end
                else
                  local.get $count
                  local.set $prev_count
                  local.get $out
                  local.get $cap
                  local.get $count
                  local.get $cx
                  f32.const 12
                  local.get $desc_depth
                  f32.convert_i32_u
                  f32.const 8
                  f32.mul
                  f32.add
                  f32.add
                  local.get $cy
                  f32.const 8
                  local.get $desc_depth
                  f32.convert_i32_u
                  f32.const 4
                  f32.mul
                  f32.add
                  f32.add
                  local.get $cw
                  local.get $ch
                  i32.const 0xff3a2d25
                  call $emit_rect
                  local.set $count
                  local.get $count
                  local.get $prev_count
                  i32.gt_s
                  if
                    local.get $out
                    local.get $prev_count
                    local.get $id
                    local.get $kind
                    i32.const 1
                    call $command_set_owner_at
                  end
                  local.get $count
                  i32.const -1
                  i32.ne
                  if
                    local.get $kind
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
                    if
                      local.get $count
                      local.set $prev_count
                      local.get $out
                      local.get $cap
                      local.get $count
                      local.get $cx
                      f32.const 16
                      local.get $desc_depth
                      f32.convert_i32_u
                      f32.const 8
                      f32.mul
                      f32.add
                      f32.add
                      local.get $cy
                      f32.const 12
                      local.get $desc_depth
                      f32.convert_i32_u
                      f32.const 4
                      f32.mul
                      f32.add
                      f32.add
                      f32.const 16
                      local.get $ch
                      f32.const 16
                      f32.sub
                      local.get $record
                      i32.const 12
                      i32.add
                      call $load16
                      if (result i32)
                        i32.const 0xff20c878
                      else
                        i32.const 0xff6d625a
                      end
                      call $emit_rect
                      local.set $count
                      local.get $count
                      local.get $prev_count
                      i32.gt_s
                      if
                        local.get $out
                        local.get $prev_count
                        local.get $id
                        local.get $kind
                        i32.const 3
                        call $command_set_owner_at
                        local.get $out
                        local.get $prev_count
                        local.get $record
                        i32.const 12
                        i32.add
                        call $load16
                        call $command_set_meta_at
                      end
                    else
                      local.get $kind
                      i32.const 24
                      i32.eq
                      local.get $kind
                      i32.const 37
                      i32.eq
                      i32.or
                      local.get $kind
                      i32.const 57
                      i32.eq
                      i32.or
                      if
                        local.get $record
                        i32.const 12
                        i32.add
                        call $load16
                        f32.convert_i32_u
                        f32.const 65535
                        f32.div
                        local.get $cw
                        f32.const 16
                        f32.sub
                        f32.mul
                        local.set $fill_w
                        local.get $fill_w
                        f32.const 0
                        f32.gt
                        if
                          local.get $count
                          local.set $prev_count
                          local.get $out
                          local.get $cap
                          local.get $count
                          local.get $cx
                          f32.const 16
                          local.get $desc_depth
                          f32.convert_i32_u
                          f32.const 8
                          f32.mul
                          f32.add
                          f32.add
                          local.get $cy
                          f32.const 12
                          local.get $desc_depth
                          f32.convert_i32_u
                          f32.const 4
                          f32.mul
                          f32.add
                          f32.add
                          local.get $fill_w
                          local.get $ch
                          f32.const 16
                          f32.sub
                          i32.const 0xff20a4f3
                          call $emit_rect
                          local.set $count
                          local.get $count
                          local.get $prev_count
                          i32.gt_s
                          if
                            local.get $out
                            local.get $prev_count
                            local.get $id
                            local.get $kind
                            i32.const 3
                            call $command_set_owner_at
                            local.get $out
                            local.get $prev_count
                            local.get $record
                            i32.const 12
                            i32.add
                            call $load16
                            call $command_set_meta_at
                          end
                        end
                      end
                      local.get $kind
                      i32.const 16
                      i32.eq
                      local.get $kind
                      i32.const 20
                      i32.eq
                      i32.or
                      local.get $kind
                      i32.const 44
                      i32.eq
                      i32.or
                      local.get $kind
                      i32.const 55
                      i32.eq
                      i32.or
                      if
                        local.get $count
                        local.set $prev_count
                        local.get $out
                        local.get $cap
                        local.get $count
                        local.get $cx
                        local.get $cw
                        f32.add
                        f32.const 24
                        f32.sub
                        local.get $cy
                        f32.const 12
                        local.get $desc_depth
                        f32.convert_i32_u
                        f32.const 4
                        f32.mul
                        f32.add
                        f32.add
                        f32.const 14
                        f32.const 14
                        i32.const 0xffd6a84f
                        call $emit_rect
                        local.set $count
                        local.get $count
                        local.get $prev_count
                        i32.gt_s
                        if
                          local.get $out
                          local.get $prev_count
                          local.get $id
                          local.get $kind
                          i32.const 6
                          call $command_set_owner_at
                        end
                      end
                    end
                  end
                  local.get $kind
                  i32.const 12
                  i32.eq
                  local.get $kind
                  i32.const 16
                  i32.eq
                  i32.or
                  local.get $kind
                  i32.const 20
                  i32.eq
                  i32.or
                  local.get $kind
                  i32.const 44
                  i32.eq
                  i32.or
                  local.get $kind
                  i32.const 55
                  i32.eq
                  i32.or
                  local.get $second_ref
                  i32.const 65535
                  i32.and
                  i32.const 0
                  i32.ne
                  i32.and
                  local.get $count
                  i32.const -1
                  i32.ne
                  i32.and
                  if
                    local.get $count
                    local.set $prev_count
                    local.get $out
                    local.get $cap
                    local.get $count
                    local.get $cx
                    f32.const 16
                    local.get $desc_depth
                    f32.convert_i32_u
                    f32.const 8
                    f32.mul
                    f32.add
                    f32.add
                    local.get $cy
                    f32.const 12
                    local.get $desc_depth
                    f32.convert_i32_u
                    f32.const 4
                    f32.mul
                    f32.add
                    f32.add
                    f32.const 12
                    f32.const 12
                    local.get $second_ref
                    i32.const 65535
                    i32.and
                    i32.const 0xff9fb6ff
                    call $emit_icon_or_rect
                    local.set $count
                    local.get $count
                    local.get $prev_count
                    i32.gt_s
                    if
                      local.get $out
                      local.get $prev_count
                      local.get $id
                      local.get $kind
                      i32.const 9
                      call $command_set_owner_at
                      local.get $out
                      local.get $prev_count
                      local.get $second_ref
                      i32.const 65535
                      i32.and
                      call $command_set_meta_at
                    end
                  end
                  local.get $kind
                  local.get $id
                  call $record_encoded_state
                  local.tee $state_value
                  i32.const 0
                  i32.gt_s
                  local.get $count
                  i32.const -1
                  i32.ne
                  i32.and
                  if
                    local.get $count
                    local.set $prev_count
                    local.get $out
                    local.get $cap
                    local.get $count
                    local.get $cx
                    f32.const 20
                    local.get $desc_depth
                    f32.convert_i32_u
                    f32.const 8
                    f32.mul
                    f32.add
                    f32.add
                    local.get $state_value
                    f32.convert_i32_u
                    f32.const 20
                    f32.mul
                    f32.add
                    local.get $cy
                    f32.const 12
                    local.get $desc_depth
                    f32.convert_i32_u
                    f32.const 4
                    f32.mul
                    f32.add
                    f32.add
                    local.get $ch
                    f32.add
                    f32.const 8
                    f32.sub
                    f32.const 16
                    f32.const 4
                    i32.const 0xffffcc55
                    call $emit_rect
                    local.set $count
                    local.get $count
                    local.get $prev_count
                    i32.gt_s
                    if
                      local.get $out
                      local.get $prev_count
                      local.get $id
                      local.get $kind
                      i32.const 10
                      call $command_set_owner_at
                      local.get $out
                      local.get $prev_count
                      local.get $state_value
                      call $command_set_meta_at
                    end
                  end
                  local.get $kind
                  i32.const 4
                  i32.eq
                  local.get $kind
                  i32.const 11
                  i32.eq
                  i32.or
                  local.get $kind
                  i32.const 26
                  i32.eq
                  i32.or
                  local.get $kind
                  i32.const 27
                  i32.eq
                  i32.or
                  local.get $count
                  i32.const -1
                  i32.ne
                  i32.and
                  if
                    local.get $count
                    local.set $prev_count
                    local.get $out
                    local.get $cap
                    local.get $count
                    local.get $cx
                    f32.const 16
                    local.get $desc_depth
                    f32.convert_i32_u
                    f32.const 8
                    f32.mul
                    f32.add
                    f32.add
                    local.get $cy
                    f32.const 12
                    local.get $desc_depth
                    f32.convert_i32_u
                    f32.const 4
                    f32.mul
                    f32.add
                    f32.add
                    f32.const 12
                    f32.const 12
                    i32.const 0xff9fb6ff
                    call $emit_rect
                    local.set $count
                    local.get $count
                    local.get $prev_count
                    i32.gt_s
                    if
                      local.get $out
                      local.get $prev_count
                      local.get $id
                      local.get $kind
                      i32.const 9
                      call $command_set_owner_at
                      local.get $out
                      local.get $prev_count
                      local.get $kind
                      i32.const 26
                      i32.eq
                      if (result i32)
                        local.get $id
                        i32.const 0x3fff
                        i32.and
                      else
                        local.get $kind
                        i32.const 4
                        i32.eq
                        local.get $kind
                        i32.const 11
                        i32.eq
                        i32.or
                        if (result i32)
                          local.get $id
                          i32.const 65535
                          i32.and
                        else
                          local.get $second_ref
                          i32.const 65535
                          i32.and
                        end
                      end
                      call $command_set_meta_at
                    end
                  end
                  local.get $count
                  i32.const -1
                  i32.ne
                  if
                    local.get $kind
                    i32.const 46
                    i32.eq
                    local.get $kind
                    i32.const 47
                    i32.eq
                    i32.or
                    local.get $kind
                    i32.const 52
                    i32.eq
                    i32.or
                    local.get $kind
                    i32.const 53
                    i32.eq
                    i32.or
                    if
                      local.get $count
                      local.set $prev_count
                      local.get $out
                      local.get $cap
                      local.get $count
                      local.get $cx
                      f32.const 16
                      local.get $desc_depth
                      f32.convert_i32_u
                      f32.const 8
                      f32.mul
                      f32.add
                      f32.add
                      local.get $cy
                      f32.const 12
                      local.get $desc_depth
                      f32.convert_i32_u
                      f32.const 4
                      f32.mul
                      f32.add
                      f32.add
                      local.get $cw
                      f32.const 24
                      f32.sub
                      local.get $ch
                      f32.const 16
                      f32.sub
                      i32.const 0xaa000000
                      call $emit_rect
                      local.set $count
                      local.get $count
                      local.get $prev_count
                      i32.gt_s
                      if
                        local.get $out
                        local.get $prev_count
                        local.get $id
                        local.get $kind
                        i32.const 5
                        call $command_set_owner_at
                      end
                    end
                    local.get $kind
                    i32.const 32
                    i32.eq
                    if
                      local.get $count
                      local.set $prev_count
                      local.get $out
                      local.get $cap
                      local.get $count
                      local.get $cx
                      f32.const 16
                      local.get $desc_depth
                      f32.convert_i32_u
                      f32.const 8
                      f32.mul
                      f32.add
                      f32.add
                      local.get $cy
                      f32.const 12
                      local.get $desc_depth
                      f32.convert_i32_u
                      f32.const 4
                      f32.mul
                      f32.add
                      f32.add
                      local.get $cw
                      f32.const 24
                      f32.sub
                      f32.const 12
                      i32.const 0xff4d3f36
                      call $emit_rect
                      local.set $count
                      local.get $count
                      local.get $prev_count
                      i32.gt_s
                      if
                        local.get $out
                        local.get $prev_count
                        local.get $id
                        local.get $kind
                        i32.const 7
                        call $command_set_owner_at
                      end
                    end
                    local.get $kind
                    i32.const 39
                    i32.eq
                    local.get $kind
                    i32.const 40
                    i32.eq
                    i32.or
                    local.get $kind
                    i32.const 54
                    i32.eq
                    i32.or
                    if
                      local.get $count
                      local.set $prev_count
                      local.get $out
                      local.get $cap
                      local.get $count
                      local.get $cx
                      f32.const 16
                      local.get $desc_depth
                      f32.convert_i32_u
                      f32.const 8
                      f32.mul
                      f32.add
                      f32.add
                      local.get $cy
                      f32.const 12
                      local.get $desc_depth
                      f32.convert_i32_u
                      f32.const 4
                      f32.mul
                      f32.add
                      f32.add
                      f32.const 20
                      local.get $ch
                      f32.const 16
                      f32.sub
                      i32.const 0xff5a463b
                      call $emit_rect
                      local.set $count
                      local.get $count
                      local.get $prev_count
                      i32.gt_s
                      if
                        local.get $out
                        local.get $prev_count
                        local.get $id
                        local.get $kind
                        i32.const 8
                        call $command_set_owner_at
                      end
                    end
                  end
                  local.get $count
                  i32.const -1
                  i32.ne
                  if
                    local.get $count
                    local.set $prev_count
                    local.get $out
                    local.get $cap
                    local.get $count
                    local.get $cx
                    f32.const 20
                    local.get $desc_depth
                    f32.convert_i32_u
                    f32.const 8
                    f32.mul
                    f32.add
                    f32.add
                    local.get $cy
                    f32.const 12
                    local.get $desc_depth
                    f32.convert_i32_u
                    f32.const 4
                    f32.mul
                    f32.add
                    f32.add
                    local.get $cw
                    f32.const 16
                    f32.sub
                    local.get $ch
                    f32.const 8
                    f32.sub
                    local.get $first_ptr
                    local.get $first_len
                    i32.const 0xfff7eee8
                    call $emit_text
                    local.set $count
                    local.get $count
                    local.get $prev_count
                    i32.gt_s
                    if
                      local.get $out
                      local.get $prev_count
                      local.get $id
                      local.get $kind
                      i32.const 2
                      call $command_set_owner_at
                    end
                  end
                  local.get $kind
                  call $second_ref_is_packed
                  i32.eqz
                  local.get $second_len
                  i32.eqz
                  i32.eqz
                  i32.and
                  local.get $count
                  i32.const -1
                  i32.ne
                  i32.and
                  if
                    local.get $count
                    local.set $prev_count
                    local.get $out
                    local.get $cap
                    local.get $count
                    local.get $cx
                    f32.const 20
                    local.get $desc_depth
                    f32.convert_i32_u
                    f32.const 8
                    f32.mul
                    f32.add
                    f32.add
                    local.get $cy
                    local.get $ch
                    f32.const 0.5
                    f32.mul
                    f32.add
                    local.get $desc_depth
                    f32.convert_i32_u
                    f32.const 4
                    f32.mul
                    f32.add
                    local.get $cw
                    f32.const 16
                    f32.sub
                    local.get $ch
                    f32.const 8
                    f32.sub
                    local.get $second_ptr
                    local.get $second_len
                    i32.const 0xffc9b8ad
                    call $emit_text
                    local.set $count
                    local.get $count
                    local.get $prev_count
                    i32.gt_s
                    if
                      local.get $out
                      local.get $prev_count
                      local.get $id
                      local.get $kind
                      i32.const 4
                      call $command_set_owner_at
                    end
                  end
                end
              end
              local.get $count
              i32.const -1
              i32.eq
              if
                i32.const -1
                return
              end
            end
            local.get $child_i
            i32.const 1
            i32.add
            local.set $child_i
            br $children
          end
        end
        local.get $root_cw
        local.set $cw
        local.get $root_ch
        local.set $ch
        local.get $axis
        i32.const 0
        i32.eq
        if
          local.get $cy
          local.get $ch
          local.get $gap
          f32.add
          f32.add
          local.set $cy
        else
          local.get $cx
          local.get $cw
          local.get $gap
          f32.add
          f32.add
          local.set $cx
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $loop
      end
    end
    local.get $count)

  (func $er_ui_validate  (param $base i32) (param $len i32) (result i32)
    (local $node_count i32)
    (local $root_count i32)
    (local $axis i32)
    (local $i i32)
    (local $roots i32)
    (local $parent_ref i32)
    (local $walk i32)
    (local $depth i32)
    (local $used_len i32)
    local.get $len
    i32.const 20
    i32.lt_u
    if
      i32.const 0
      return
    end
    local.get $base
    i32.load
    i32.const 0x49755245
    i32.ne
    if
      i32.const 0
      return
    end
    local.get $base
    i32.const 4
    i32.add
    i32.load
    local.tee $used_len
    i32.eqz
    i32.eqz
    if
      local.get $used_len
      local.get $len
      i32.ne
      local.get $base
      local.get $used_len
      i32.add
      local.get $len
      i32.ne
      i32.and
      if
        i32.const 0
        return
      end
    end
    local.get $base
    i32.const 16
    i32.add
    call $load16
    local.tee $node_count
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $base
    i32.const 18
    i32.add
    call $load16
    local.tee $root_count
    i32.eqz
    local.get $root_count
    local.get $node_count
    i32.gt_u
    i32.or
    if
      i32.const 0
      return
    end
    local.get $len
    i32.const 20
    local.get $node_count
    i32.const 16
    i32.mul
    i32.add
    i32.lt_u
    if
      i32.const 0
      return
    end
    local.get $base
    i32.const 10
    i32.add
    call $load16
    local.tee $axis
    i32.const 1
    i32.gt_u
    if
      i32.const 0
      return
    end
    block $done
      loop $loop
        local.get $i
        local.get $node_count
        i32.ge_u
        br_if $done
        local.get $base
        local.get $i
        call $record_at
        i32.const 2
        i32.add
        call $load16
        local.tee $parent_ref
        i32.eqz
        if
          local.get $roots
          i32.const 1
          i32.add
          local.set $roots
        else
          local.get $parent_ref
          local.get $node_count
          i32.gt_u
          local.get $parent_ref
          local.get $i
          i32.const 1
          i32.add
          i32.eq
          i32.or
          if
            i32.const 0
            return
          end
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $loop
      end
    end
    local.get $roots
    local.get $root_count
    i32.ne
    if
      i32.const 0
      return
    end
    i32.const 0
    local.set $i
    block $cycle_done
      loop $cycle_node
        local.get $i
        local.get $node_count
        i32.ge_u
        br_if $cycle_done
        local.get $i
        local.set $walk
        i32.const 0
        local.set $depth
        block $parent_done
          loop $parent_loop
            local.get $base
            local.get $walk
            call $record_at
            i32.const 2
            i32.add
            call $load16
            local.tee $parent_ref
            i32.eqz
            br_if $parent_done
            local.get $depth
            local.get $node_count
            i32.ge_u
            if
              i32.const 0
              return
            end
            local.get $parent_ref
            i32.const 1
            i32.sub
            local.set $walk
            local.get $depth
            i32.const 1
            i32.add
            local.set $depth
            br $parent_loop
          end
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $cycle_node
      end
    end
    i32.const 1)

  (func $er_ui_node_count  (param $base i32) (param $len i32) (result i32)
    local.get $base
    local.get $len
    call $er_ui_validate
    if (result i32)
      local.get $base
      i32.const 16
      i32.add
      call $load16
    else
      i32.const 0
    end)

  (func $er_ui_root_count  (param $base i32) (param $len i32) (result i32)
    local.get $base
    local.get $len
    call $er_ui_validate
    if (result i32)
      local.get $base
      i32.const 18
      i32.add
      call $load16
    else
      i32.const 0
    end)

  (func $er_ui_axis  (param $base i32) (param $len i32) (result i32)
    local.get $base
    local.get $len
    call $er_ui_validate
    if (result i32)
      local.get $base
      i32.const 10
      i32.add
      call $load16
    else
      i32.const -1
    end)

  (func $er_ui_gap  (param $base i32) (param $len i32) (result i32)
    local.get $base
    local.get $len
    call $er_ui_validate
    if (result i32)
      local.get $base
      i32.const 12
      i32.add
      call $load16
    else
      i32.const 0
    end)

  (func $er_ui_padding  (param $base i32) (param $len i32) (result i32)
    local.get $base
    local.get $len
    call $er_ui_validate
    if (result i32)
      local.get $base
      i32.const 14
      i32.add
      call $load16
    else
      i32.const 0
    end)

  (func $record_at (param $base i32) (param $index i32) (result i32)
    local.get $base
    i32.const 20
    i32.add
    local.get $index
    i32.const 16
    i32.mul
    i32.add)

  (func $er_ui_record_kind  (param $base i32) (param $len i32) (param $index i32) (result i32)
    (local $n i32)
    local.get $base
    local.get $len
    call $er_ui_validate
    i32.eqz
    if
      i32.const -1
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
      i32.const -1
      return
    end
    local.get $base
    local.get $index
    call $record_at
    call $load16)

  (func $er_ui_record_parent  (param $base i32) (param $len i32) (param $index i32) (result i32)
    (local $n i32)
    (local $parent_ref i32)
    local.get $base
    local.get $len
    call $er_ui_validate
    i32.eqz
    if
      i32.const -2
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
      i32.const -2
      return
    end
    local.get $base
    local.get $index
    call $record_at
    i32.const 2
    i32.add
    call $load16
    local.tee $parent_ref
    i32.eqz
    if
      i32.const -1
      return
    end
    local.get $parent_ref
    i32.const 1
    i32.sub)

  (func $er_ui_record_is_root  (param $base i32) (param $len i32) (param $index i32) (result i32)
    local.get $base
    local.get $len
    local.get $index
    call $er_ui_record_parent
    i32.const -1
    i32.eq)

  (func $er_ui_record_id  (param $base i32) (param $len i32) (param $index i32) (result i32)
    (local $n i32)
    local.get $base
    local.get $len
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
    local.get $base
    local.get $index
    call $record_at
    i32.const 4
    i32.add
    i32.load)

  (func $er_ui_record_state_value  (param $base i32) (param $len i32) (param $index i32) (result i32)
    (local $n i32)
    (local $record i32)
    (local $kind i32)
    (local $id i32)
    local.get $base
    local.get $len
    call $er_ui_validate
    i32.eqz
    if
      i32.const -1
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
      i32.const -1
      return
    end
    local.get $base
    local.get $index
    call $record_at
    local.set $record
    local.get $record
    call $load16
    local.set $kind
    local.get $record
    i32.const 4
    i32.add
    i32.load
    local.set $id
    local.get $kind
    local.get $id
    call $record_encoded_state)

  (func $er_ui_record_base_id  (param $base i32) (param $len i32) (param $index i32) (result i32)
    (local $n i32)
    (local $record i32)
    (local $kind i32)
    (local $id i32)
    local.get $base
    local.get $len
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
    local.get $base
    local.get $index
    call $record_at
    local.set $record
    local.get $record
    call $load16
    local.set $kind
    local.get $record
    i32.const 4
    i32.add
    i32.load
    local.set $id
    local.get $kind
    i32.const 26
    i32.eq
    if
      local.get $id
      i32.const 14
      i32.shr_u
      return
    end
    local.get $id
    local.get $kind
    call $record_encoded_mul
    i32.div_u)

  (func $er_ui_record_icon_value  (param $base i32) (param $len i32) (param $index i32) (result i32)
    (local $n i32)
    (local $record i32)
    (local $kind i32)
    (local $id i32)
    local.get $base
    local.get $len
    call $er_ui_validate
    i32.eqz
    if
      i32.const -1
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
      i32.const -1
      return
    end
    local.get $base
    local.get $index
    call $record_at
    local.set $record
    local.get $record
    call $load16
    local.set $kind
    local.get $record
    i32.const 4
    i32.add
    i32.load
    local.set $id
    local.get $kind
    i32.const 26
    i32.eq
    if
      local.get $id
      i32.const 0x3fff
      i32.and
      return
    end
    local.get $kind
    i32.const 4
    i32.eq
    local.get $kind
    i32.const 11
    i32.eq
    i32.or
    if
      local.get $id
      i32.const 65535
      i32.and
      return
    end
    local.get $kind
    i32.const 12
    i32.eq
    local.get $kind
    i32.const 13
    i32.eq
    i32.or
    local.get $kind
    i32.const 16
    i32.eq
    i32.or
    local.get $kind
    i32.const 20
    i32.eq
    i32.or
    local.get $kind
    i32.const 44
    i32.eq
    i32.or
    local.get $kind
    i32.const 55
    i32.eq
    i32.or
    i32.eqz
    if
      i32.const -1
      return
    end
    local.get $record
    i32.const 12
    i32.add
    call $load16
    i32.const 65535
    i32.and)

  (func $er_ui_record_variant_value  (param $base i32) (param $len i32) (param $index i32) (result i32)
    (local $n i32)
    (local $record i32)
    (local $kind i32)
    (local $id i32)
    local.get $base
    local.get $len
    call $er_ui_validate
    i32.eqz
    if
      i32.const -1
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
      i32.const -1
      return
    end
    local.get $base
    local.get $index
    call $record_at
    local.set $record
    local.get $record
    call $load16
    local.set $kind
    local.get $record
    i32.const 4
    i32.add
    i32.load
    local.set $id
    local.get $kind
    i32.const 4
    i32.eq
    if
      local.get $id
      i32.const 16
      i32.shr_u
      i32.const 65535
      i32.and
      return
    end
    local.get $kind
    i32.const 12
    i32.eq
    if
      local.get $record
      i32.const 14
      i32.add
      call $load16
      i32.const 255
      i32.and
      return
    end
    local.get $kind
    i32.const 27
    i32.eq
    if
      local.get $record
      i32.const 12
      i32.add
      call $load16
      return
    end
    i32.const -1)

  (func $er_ui_record_bool_value  (param $base i32) (param $len i32) (param $index i32) (result i32)
    (local $n i32)
    (local $record i32)
    (local $kind i32)
    local.get $base
    local.get $len
    call $er_ui_validate
    i32.eqz
    if
      i32.const -1
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
      i32.const -1
      return
    end
    local.get $base
    local.get $index
    call $record_at
    local.set $record
    local.get $record
    call $load16
    local.set $kind
    local.get $kind
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
      i32.const -1
      return
    end
    local.get $record
    i32.const 12
    i32.add
    call $load16
    i32.const 0
    i32.ne)

  (func $er_ui_record_unit_value  (param $base i32) (param $len i32) (param $index i32) (result i32)
    (local $n i32)
    (local $record i32)
    (local $kind i32)
    local.get $base
    local.get $len
    call $er_ui_validate
    i32.eqz
    if
      i32.const -1
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
      i32.const -1
      return
    end
    local.get $base
    local.get $index
    call $record_at
    local.set $record
    local.get $record
    call $load16
    local.set $kind
    local.get $kind
    i32.const 7
    i32.eq
    local.get $kind
    i32.const 24
    i32.eq
    i32.or
    local.get $kind
    i32.const 37
    i32.eq
    i32.or
    local.get $kind
    i32.const 57
    i32.eq
    i32.or
    i32.eqz
    if
      i32.const -1
      return
    end
    local.get $record
    i32.const 12
    i32.add
    call $load16)

  (func $er_ui_record_aspect_ratio_w  (param $base i32) (param $len i32) (param $index i32) (result i32)
    (local $id i32)
    local.get $base
    local.get $len
    local.get $index
    call $er_ui_record_kind
    i32.const 6
    i32.ne
    if
      i32.const -1
      return
    end
    local.get $base
    local.get $len
    local.get $index
    call $er_ui_record_id
    local.set $id
    local.get $id
    i32.const 16
    i32.shr_u
    i32.const 65535
    i32.and)

  (func $er_ui_record_aspect_ratio_h  (param $base i32) (param $len i32) (param $index i32) (result i32)
    local.get $base
    local.get $len
    local.get $index
    call $er_ui_record_kind
    i32.const 6
    i32.ne
    if
      i32.const -1
      return
    end
    local.get $base
    local.get $len
    local.get $index
    call $er_ui_record_id
    i32.const 65535
    i32.and)

  (func $er_ui_record_first_ref  (param $base i32) (param $len i32) (param $index i32) (result i32)
    (local $n i32)
    (local $r i32)
    local.get $base
    local.get $len
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
    local.get $base
    local.get $index
    call $record_at
    local.set $r
    local.get $r
    i32.const 8
    i32.add
    call $load16
    local.get $r
    i32.const 10
    i32.add
    call $load16
    call $string_ref)

  (func $er_ui_record_second_ref  (param $base i32) (param $len i32) (param $index i32) (result i32)
    (local $n i32)
    (local $r i32)
    local.get $base
    local.get $len
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
    local.get $base
    local.get $index
    call $record_at
    local.set $r
    local.get $r
    i32.const 12
    i32.add
    call $load16
    local.get $r
    i32.const 14
    i32.add
    call $load16
    call $string_ref)

  (func $er_ui_record_ref_valid  (param $base i32) (param $len i32) (param $ref i32) (result i32)
    (local $n i32)
    (local $strings_len i32)
    local.get $base
    local.get $len
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
    local.get $len
    i32.const 20
    local.get $n
    i32.const 16
    i32.mul
    i32.add
    i32.sub
    local.set $strings_len
    local.get $ref
    i32.const 65535
    i32.and
    local.get $strings_len
    i32.le_u
    local.get $ref
    i32.const 65535
    i32.and
    local.get $ref
    i32.const 16
    i32.shr_u
    i32.add
    local.get $strings_len
    i32.le_u
    i32.and)

  (func $er_ui_record_refs_valid  (param $base i32) (param $len i32) (param $index i32) (result i32)
    (local $first i32)
    (local $second i32)
    (local $kind i32)
    local.get $base
    local.get $len
    local.get $index
    call $er_ui_record_kind
    local.tee $kind
    i32.const -1
    i32.eq
    if
      i32.const -1
      return
    end
    local.get $base
    local.get $len
    local.get $index
    call $er_ui_record_first_ref
    local.set $first
    local.get $base
    local.get $len
    local.get $index
    call $er_ui_record_second_ref
    local.set $second
    local.get $base
    local.get $len
    local.get $first
    call $er_ui_record_ref_valid
    local.get $kind
    call $second_ref_is_packed
    if (result i32)
      local.get $kind
      local.get $second
      call $packed_second_ref_valid
    else
      local.get $base
      local.get $len
      local.get $second
      call $er_ui_record_ref_valid
    end
    i32.and)

  (func $er_ui_validate_deep  (param $base i32) (param $len i32) (result i32)
    (local $n i32)
    (local $i i32)
    (local $kind i32)
    local.get $base
    local.get $len
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
    block $done
      loop $loop
        local.get $i
        local.get $n
        i32.ge_u
        br_if $done
        local.get $base
        local.get $i
        call $record_at
        call $load16
        local.tee $kind
        i32.const 58
        i32.ge_u
        if
          i32.const 0
          return
        end
        local.get $base
        local.get $len
        local.get $i
        call $er_ui_record_refs_valid
        i32.eqz
        if
          i32.const 0
          return
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $loop
      end
    end
    i32.const 1)
  (func $er_ui_record_set_id  (param $base i32) (param $len i32) (param $index i32) (param $id i32) (result i32)
    (local $n i32)
    local.get $base
    local.get $len
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
    local.get $base
    local.get $index
    call $record_at
    i32.const 4
    i32.add
    local.get $id
    i32.store
    i32.const 1)

  (func $er_ui_record_set_parent  (param $base i32) (param $len i32) (param $index i32) (param $parent i32) (result i32)
    (local $n i32)
    local.get $base
    local.get $len
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
    local.get $parent
    i32.const -1
    i32.eq
    if
      local.get $base
      local.get $index
      call $record_at
      i32.const 2
      i32.add
      i32.const 0
      call $store16
      i32.const 1
      return
    end
    local.get $parent
    local.get $n
    i32.ge_u
    local.get $parent
    local.get $index
    i32.eq
    i32.or
    if
      i32.const 0
      return
    end
    local.get $base
    local.get $index
    call $record_at
    i32.const 2
    i32.add
    local.get $parent
    i32.const 1
    i32.add
    call $store16
    i32.const 1)

  (func $er_ui_record_set_first_ref  (param $base i32) (param $len i32) (param $index i32) (param $ref i32) (result i32)
    (local $n i32)
    (local $r i32)
    local.get $base
    local.get $len
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
    local.get $base
    local.get $index
    call $record_at
    local.set $r
    local.get $r
    i32.const 8
    i32.add
    local.get $ref
    i32.const 65535
    i32.and
    call $store16
    local.get $r
    i32.const 10
    i32.add
    local.get $ref
    i32.const 16
    i32.shr_u
    call $store16
    i32.const 1)

  (func $er_ui_record_set_second_ref  (param $base i32) (param $len i32) (param $index i32) (param $ref i32) (result i32)
    (local $n i32)
    (local $r i32)
    local.get $base
    local.get $len
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
    local.get $base
    local.get $index
    call $record_at
    local.set $r
    local.get $r
    i32.const 12
    i32.add
    local.get $ref
    i32.const 65535
    i32.and
    call $store16
    local.get $r
    i32.const 14
    i32.add
    local.get $ref
    i32.const 16
    i32.shr_u
    call $store16
    i32.const 1)
