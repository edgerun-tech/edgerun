;; Filesystem walking and identity semantics captured from local walkdir,
  ;; same-file, and tempfile compatibility crates.
  ;;
  ;; Node kinds: file=0 dir=1 symlink=2 error=3.
  ;; Order: preorder=0 contents_first=1.
  ;; Decisions: skip=0 yield=1 descend=2 yield_and_descend=3 defer=4 error=5.

  (func $m94bool (param $x i32) (result i32)
    local.get $x
    i32.const 0
    i32.ne)

  (func $max1 (param $n i32) (result i32)
    local.get $n
    i32.const 1
    i32.lt_s
    if (result i32)
      i32.const 1
    else
      local.get $n
    end)

  (export "walk_default_flags" (func $walk_default_flags))
  (func $walk_default_flags (result i32)
    ;; follow_links=false, follow_root_links=true, contents_first=false,
    ;; same_file_system=false, sorter=false.
    i32.const 2)

  (export "walk_default_max_open" (func $walk_default_max_open))
  (func $walk_default_max_open (result i32)
    i32.const 10)

  (export "walk_min_depth_clamp" (func $walk_min_depth_clamp))
  (func $walk_min_depth_clamp
    (param $requested_min i32)
    (param $current_max i32)
    (result i32)
    local.get $requested_min
    local.get $current_max
    i32.gt_s
    if (result i32)
      local.get $current_max
    else
      local.get $requested_min
    end)

  (export "walk_max_depth_clamp" (func $walk_max_depth_clamp))
  (func $walk_max_depth_clamp
    (param $requested_max i32)
    (param $current_min i32)
    (result i32)
    local.get $requested_max
    local.get $current_min
    i32.lt_s
    if (result i32)
      local.get $current_min
    else
      local.get $requested_max
    end)

  (export "walk_max_open_clamp" (func $walk_max_open_clamp))
  (func $walk_max_open_clamp
    (param $requested i32)
    (result i32)
    local.get $requested
    call $max1)

  (export "walk_depth_visible" (func $walk_depth_visible))
  (func $walk_depth_visible
    (param $depth i32)
    (param $min_depth i32)
    (param $max_depth i32)
    (result i32)
    local.get $depth
    local.get $min_depth
    i32.ge_s
    local.get $depth
    local.get $max_depth
    i32.le_s
    i32.and)

  (export "walk_should_descend" (func $walk_should_descend))
  (func $walk_should_descend
    (param $node_kind i32)
    (param $depth i32)
    (param $max_depth i32)
    (param $filter_accepts i32)
    (result i32)
    local.get $node_kind
    i32.const 1
    i32.eq
    local.get $depth
    local.get $max_depth
    i32.lt_s
    i32.and
    local.get $filter_accepts
    call $m94bool
    i32.and)

  (export "walk_entry_decision" (func $walk_entry_decision))
  (func $walk_entry_decision
    (param $node_kind i32)
    (param $depth i32)
    (param $min_depth i32)
    (param $max_depth i32)
    (param $contents_first i32)
    (param $filter_accepts i32)
    (result i32)
    (local $visible i32)
    (local $descend i32)
    local.get $depth
    local.get $min_depth
    local.get $max_depth
    call $walk_depth_visible
    local.set $visible
    local.get $node_kind
    local.get $depth
    local.get $max_depth
    local.get $filter_accepts
    call $walk_should_descend
    local.set $descend
    local.get $node_kind
    i32.const 3
    i32.eq
    if (result i32)
      i32.const 5
    else
      local.get $visible
      i32.eqz
      if (result i32)
        local.get $descend
        if (result i32)
          i32.const 2
        else
          i32.const 0
        end
      else
        local.get $descend
        if (result i32)
          local.get $contents_first
          call $m94bool
          if (result i32)
            i32.const 4
          else
            i32.const 3
          end
        else
          i32.const 1
        end
      end
    end)

  (export "walk_symlink_kind" (func $walk_symlink_kind))
  (func $walk_symlink_kind
    (param $is_root i32)
    (param $is_symlink i32)
    (param $target_is_dir i32)
    (param $follow_links i32)
    (param $follow_root_links i32)
    (result i32)
    local.get $is_symlink
    call $m94bool
    i32.eqz
    if (result i32)
      local.get $target_is_dir
      if (result i32)
        i32.const 1
      else
        i32.const 0
      end
    else
      local.get $follow_links
      call $m94bool
      local.get $is_root
      call $m94bool
      local.get $follow_root_links
      call $m94bool
      i32.and
      i32.or
      if (result i32)
        local.get $target_is_dir
        if (result i32)
          i32.const 1
        else
          i32.const 0
        end
      else
        i32.const 2
      end
    end)

  (export "walk_loop_error" (func $walk_loop_error))
  (func $walk_loop_error
    (param $follow_links i32)
    (param $candidate_matches_ancestor i32)
    (result i32)
    local.get $follow_links
    call $m94bool
    local.get $candidate_matches_ancestor
    call $m94bool
    i32.and)

  (export "walk_same_file_system_descend" (func $walk_same_file_system_descend))
  (func $walk_same_file_system_descend
    (param $same_file_system i32)
    (param $root_dev i64)
    (param $entry_dev i64)
    (result i32)
    local.get $same_file_system
    call $m94bool
    if (result i32)
      local.get $root_dev
      local.get $entry_dev
      i64.eq
    else
      i32.const 1
    end)

  (export "walk_fd_spill" (func $walk_fd_spill))
  (func $walk_fd_spill
    (param $open_count i32)
    (param $max_open i32)
    (result i32)
    ;; When opening another directory at the limit, close the oldest handle
    ;; and retain its remaining entries in memory.
    local.get $open_count
    local.get $max_open
    call $max1
    i32.ge_s)

  (export "walk_filter_skip_descendants" (func $walk_filter_skip_descendants))
  (func $walk_filter_skip_descendants
    (param $is_dir i32)
    (param $predicate_accepts i32)
    (param $contents_first i32)
    (result i32)
    ;; filter_entry can prune descendants only for pre-order directory entries.
    local.get $is_dir
    call $m94bool
    local.get $predicate_accepts
    call $m94bool
    i32.eqz
    i32.and
    local.get $contents_first
    call $m94bool
    i32.eqz
    i32.and)

  (export "same_file_unix_equal" (func $same_file_unix_equal))
  (func $same_file_unix_equal
    (param $dev_a i64)
    (param $ino_a i64)
    (param $dev_b i64)
    (param $ino_b i64)
    (result i32)
    local.get $dev_a
    local.get $dev_b
    i64.eq
    local.get $ino_a
    local.get $ino_b
    i64.eq
    i32.and)

  (export "same_file_std_drop_closes" (func $same_file_std_drop_closes))
  (func $same_file_std_drop_closes
    (param $is_std_handle i32)
    (result i32)
    ;; std stream handles intentionally leak the fd back out on drop.
    local.get $is_std_handle
    call $m94bool
    i32.eqz)

  (export "tempfile_retry_allowed" (func $tempfile_retry_allowed))
  (func $tempfile_retry_allowed
    (param $attempt i32)
    (param $already_exists i32)
    (result i32)
    local.get $attempt
    i32.const 128
    i32.lt_s
    local.get $already_exists
    call $m94bool
    i32.and)

  (export "tempfile_terminal_status" (func $tempfile_terminal_status))
  (func $tempfile_terminal_status
    (param $attempt i32)
    (param $create_ok i32)
    (param $already_exists i32)
    (result i32)
    ;; 0 continue, 1 success, 2 collision exhausted, 3 io error.
    local.get $create_ok
    call $m94bool
    if (result i32)
      i32.const 1
    else
      local.get $already_exists
      call $m94bool
      if (result i32)
        local.get $attempt
        i32.const 127
        i32.lt_s
        if (result i32)
          i32.const 0
        else
          i32.const 2
        end
      else
        i32.const 3
      end
    end)

  (export "tempfile_drop_action" (func $tempfile_drop_action))
  (func $tempfile_drop_action
    (param $is_dir i32)
    (result i32)
    ;; file remove=1, recursive dir remove=2.
    local.get $is_dir
    call $m94bool
    if (result i32)
      i32.const 2
    else
      i32.const 1
    end)