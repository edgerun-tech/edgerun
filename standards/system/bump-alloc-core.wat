(func $next_power2 (param $n i32) (result i32)
    (local $p i32)
    i32.const 1
    local.set $p
    loop $again
      local.get $p
      local.get $n
      i32.lt_u
      if
        local.get $p
        i32.const 1
        i32.shl
        local.set $p
        br $again
      end
    end
    local.get $p)

  ;; Buf::advance either consumes exactly cnt bytes or reports the bounds error.
  (func (export "bytes_buf_advance_remaining") (param $remaining i32) (param $cnt i32) (result i32)
    local.get $cnt
    local.get $remaining
    i32.gt_u
    if (result i32)
      i32.const -1
    else
      local.get $remaining
      local.get $cnt
      i32.sub
    end)

  ;; Buf copy/get methods are checked: enough remaining bytes advances by the request.
  (func (export "bytes_buf_checked_read_remaining") (param $remaining i32) (param $requested i32) (result i32)
    local.get $requested
    local.get $remaining
    i32.gt_u
    if (result i32)
      i32.const -1
    else
      local.get $remaining
      local.get $requested
      i32.sub
    end)

  ;; try_copy_to_slice copies from chunk-sized pieces, but succeeds only when the total fits.
  (func (export "bytes_buf_copy_first_chunk") (param $remaining i32) (param $dst_len i32) (param $chunk_len i32) (result i32)
    local.get $dst_len
    local.get $remaining
    i32.gt_u
    if (result i32)
      i32.const -1
    else
      local.get $dst_len
      local.get $chunk_len
      call $m37min
    end)

  ;; field: 0 status, 1 self_len, 2 self_cap, 3 other_len, 4 other_cap.
  (func (export "bytes_mut_split_to_field") (param $len i32) (param $cap i32) (param $at i32) (param $field i32) (result i32)
    local.get $at
    local.get $len
    i32.gt_u
    if
      i32.const -1
      return
    end
    local.get $field
    i32.const 0
    i32.eq
    if (result i32)
      i32.const 0
    else
      local.get $field
      i32.const 1
      i32.eq
      if (result i32)
        local.get $len
        local.get $at
        i32.sub
      else
        local.get $field
        i32.const 2
        i32.eq
        if (result i32)
          local.get $cap
          local.get $at
          i32.sub
        else
          local.get $field
          i32.const 3
          i32.eq
          if (result i32)
            local.get $at
          else
            local.get $field
            i32.const 4
            i32.eq
            if (result i32)
              local.get $at
            else
              i32.const -1
            end
          end
        end
      end
    end)

  ;; field: 0 status, 1 self_len, 2 self_cap, 3 other_len, 4 other_cap.
  (func (export "bytes_mut_split_off_field") (param $len i32) (param $cap i32) (param $at i32) (param $field i32) (result i32)
    local.get $at
    local.get $cap
    i32.gt_u
    if
      i32.const -1
      return
    end
    local.get $field
    i32.const 0
    i32.eq
    if (result i32)
      i32.const 0
    else
      local.get $field
      i32.const 1
      i32.eq
      if (result i32)
        local.get $len
        local.get $at
        call $m37min
      else
        local.get $field
        i32.const 2
        i32.eq
        if (result i32)
          local.get $at
        else
          local.get $field
          i32.const 3
          i32.eq
          if (result i32)
            local.get $len
            local.get $at
            call $sat_sub
          else
            local.get $field
            i32.const 4
            i32.eq
            if (result i32)
              local.get $cap
              local.get $at
              i32.sub
            else
              i32.const -1
            end
          end
        end
      end
    end)

  ;; 0 enough spare, 1 reclaim/shift, 2 allocate Vec, 3 copy out of shared, 4 no-allocate failure.
  (func (export "bytes_mut_reserve_action")
    (param $kind i32) (param $len i32) (param $cap i32) (param $offset i32)
    (param $additional i32) (param $unique i32) (param $backing_cap i32) (param $allocate i32)
    (result i32)
    (local $rem i32)
    (local $need i32)
    local.get $cap
    local.get $len
    i32.sub
    local.set $rem
    local.get $additional
    local.get $rem
    i32.le_u
    if
      i32.const 0
      return
    end
    local.get $kind
    i32.const 1
    i32.eq
    if
      local.get $rem
      local.get $offset
      i32.add
      local.get $additional
      i32.ge_u
      local.get $offset
      local.get $len
      i32.ge_u
      i32.and
      if
        i32.const 1
        return
      end
      local.get $allocate
      if (result i32) i32.const 2 else i32.const 4 end
      return
    end
    local.get $len
    local.get $additional
    i32.add
    local.set $need
    local.get $unique
    if
      local.get $backing_cap
      local.get $need
      local.get $offset
      i32.add
      i32.ge_u
      if
        i32.const 1
        return
      end
      local.get $backing_cap
      local.get $need
      i32.ge_u
      local.get $offset
      local.get $len
      i32.ge_u
      i32.and
      if
        i32.const 1
        return
      end
      local.get $allocate
      if (result i32) i32.const 2 else i32.const 4 end
      return
    end
    local.get $allocate
    if (result i32) i32.const 3 else i32.const 4 end)

  ;; field: 0 status, 1 len, 2 cap. Fast unsplit requires contiguous ARC views with same shared state.
  (func (export "bytes_mut_unsplit_field")
    (param $self_len i32) (param $self_cap i32) (param $other_len i32) (param $other_cap i32)
    (param $contiguous i32) (param $same_shared i32) (param $field i32)
    (result i32)
    local.get $other_cap
    i32.eqz
    if
      local.get $field
      i32.const 0
      i32.eq
      if (result i32) i32.const 0 else i32.const -1 end
      return
    end
    local.get $contiguous
    local.get $same_shared
    i32.and
    i32.eqz
    if
      i32.const -1
      return
    end
    local.get $field
    i32.const 0
    i32.eq
    if (result i32)
      i32.const 0
    else
      local.get $field
      i32.const 1
      i32.eq
      if (result i32)
        local.get $self_len
        local.get $other_len
        i32.add
      else
        local.get $field
        i32.const 2
        i32.eq
        if (result i32)
          local.get $self_cap
          local.get $other_cap
          i32.add
        else
          i32.const -1
        end
      end
    end)

  ;; field: 0 status, 1 new_capacity, 2 aligned_size, 3 align_padding, 4 effective_align.
  (func $bump_fast_alloc_field (export "bump_fast_alloc_field")
    (param $capacity i32) (param $size i32) (param $layout_align i32) (param $m37min_align i32)
    (param $ptr_mod_layout_align i32) (param $field i32)
    (result i32)
    (local $effective_align i32)
    (local $aligned_size i32)
    (local $padding i32)
    (local $usable i32)
    local.get $layout_align
    local.get $m37min_align
    call $max
    local.set $effective_align
    local.get $size
    local.get $effective_align
    call $round_up
    local.set $aligned_size
    local.get $layout_align
    local.get $m37min_align
    i32.gt_u
    if
      local.get $ptr_mod_layout_align
      local.set $padding
    else
      i32.const 0
      local.set $padding
    end
    local.get $capacity
    local.get $padding
    i32.lt_u
    if
      i32.const -1
      return
    end
    local.get $capacity
    local.get $padding
    i32.sub
    local.set $usable
    local.get $aligned_size
    local.get $usable
    i32.gt_u
    if
      i32.const -1
      return
    end
    local.get $field
    i32.const 0
    i32.eq
    if (result i32)
      i32.const 0
    else
      local.get $field
      i32.const 1
      i32.eq
      if (result i32)
        local.get $usable
        local.get $aligned_size
        i32.sub
      else
        local.get $field
        i32.const 2
        i32.eq
        if (result i32)
          local.get $aligned_size
        else
          local.get $field
          i32.const 3
          i32.eq
          if (result i32)
            local.get $padding
          else
            local.get $field
            i32.const 4
            i32.eq
            if (result i32)
              local.get $effective_align
            else
              i32.const -1
            end
          end
        end
      end
    end)

  ;; Models NewChunkMemoryDetails. field: 1 usable-without-footer, 2 allocation-align, 3 total-size.
  (func $bump_new_chunk_field (export "bump_new_chunk_field")
    (param $requested_without_footer i32) (param $request_size i32) (param $request_align i32) (param $m37min_align i32) (param $field i32)
    (result i32)
    (local $align i32)
    (local $usable i32)
    i32.const 16
    local.get $m37min_align
    call $max
    local.get $request_align
    call $max
    local.set $align
    local.get $requested_without_footer
    i32.eqz
    if
      i32.const 448
      local.set $usable
    else
      local.get $requested_without_footer
      local.set $usable
    end
    local.get $usable
    local.get $request_size
    local.get $align
    call $round_up
    call $max
    local.set $usable
    local.get $usable
    i32.const 4096
    i32.lt_u
    if
      local.get $usable
      i32.const 64
      i32.add
      call $next_power2
      i32.const 64
      i32.sub
      local.set $usable
    else
      local.get $usable
      i32.const 64
      i32.add
      i32.const 4096
      call $round_up
      i32.const 64
      i32.sub
      local.set $usable
    end
    local.get $field
    i32.const 1
    i32.eq
    if (result i32)
      local.get $usable
    else
      local.get $field
      i32.const 2
      i32.eq
      if (result i32)
        local.get $align
      else
        local.get $field
        i32.const 3
        i32.eq
        if (result i32)
          local.get $usable
          i32.const 48
          i32.add
        else
          i32.const -1
        end
      end
    end)

  (func $bump_limit_fits (export "bump_limit_fits") (param $has_limit i32) (param $limit_left i32) (param $new_usable_size i32) (result i32)
    local.get $has_limit
    i32.eqz
    if
      i32.const 1
      return
    end
    local.get $limit_left
    local.get $new_usable_size
    i32.ge_u)

  ;; 1 fast current chunk, 2 new chunk, -1 allocation limit/oom before new chunk.
  (func (export "bump_allocation_path")
    (param $current_capacity i32) (param $size i32) (param $layout_align i32) (param $m37min_align i32)
    (param $ptr_mod_layout_align i32) (param $has_limit i32) (param $limit_left i32) (param $current_without_footer i32)
    (result i32)
    (local $base i32)
    (local $new_usable i32)
    local.get $current_capacity
    local.get $size
    local.get $layout_align
    local.get $m37min_align
    local.get $ptr_mod_layout_align
    i32.const 0
    call $bump_fast_alloc_field
    i32.const 0
    i32.eq
    if
      i32.const 1
      return
    end
    local.get $current_without_footer
    i32.const 1
    i32.shl
    local.get $size
    i32.const 448
    call $max
    call $max
    local.set $base
    local.get $base
    local.get $size
    local.get $layout_align
    local.get $m37min_align
    i32.const 1
    call $bump_new_chunk_field
    local.set $new_usable
    local.get $has_limit
    local.get $limit_left
    local.get $new_usable
    call $bump_limit_fits
    if (result i32)
      i32.const 2
    else
      i32.const -1
    end)

  ;; field 1 retained_chunks, 2 capacity_reset. Reset keeps one current chunk and frees older chunks.
  (func (export "bump_reset_field") (param $has_current_chunk i32) (param $current_without_footer i32) (param $field i32) (result i32)
    local.get $has_current_chunk
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $field
    i32.const 1
    i32.eq
    if (result i32)
      i32.const 1
    else
      local.get $field
      i32.const 2
      i32.eq
      if (result i32)
        local.get $current_without_footer
      else
        i32.const -1
      end
    end)
