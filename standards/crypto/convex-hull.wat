  ;; Convex hull — Jarvis march (gift wrapping) algorithm.
  ;; Points are (x, y) pairs stored as consecutive i32 values.
  ;; Writes hull point indices to output buffer.
  ;;
  ;; Exports:
  ;;   convex_hull(points_ptr, num_points, out_indices_ptr) -> i32
  ;;     Returns number of hull points (indices written to out).
  ;;     Returns 0 if < 3 points, -1 on error.
  (func (export "proto_standard_id") (result i32) i32.const 300534)

  (func $x (param $base i32) (param $idx i32) (result i32)
    local.get $base local.get $idx i32.const 3 i32.shl i32.add i32.load
  )

  (func $y (param $base i32) (param $idx i32) (result i32)
    local.get $base local.get $idx i32.const 3 i32.shl i32.add i32.const 4 i32.add i32.load
  )

  ;; Cross product of vectors (p1->p2) × (p1->p3)
  ;; Returns > 0 if counterclockwise, 0 if collinear, < 0 if clockwise
  (func $cross (param $base i32) (param $p1 i32) (param $p2 i32) (param $p3 i32) (result i32)
    (local $x1 i32) (local $y1 i32) (local $x2 i32) (local $y2 i32)
    local.get $base local.get $p2 call $x local.get $base local.get $p1 call $x i32.sub local.set $x1
    local.get $base local.get $p2 call $y local.get $base local.get $p1 call $y i32.sub local.set $y1
    local.get $base local.get $p3 call $x local.get $base local.get $p1 call $x i32.sub local.set $x2
    local.get $base local.get $p3 call $y local.get $base local.get $p1 call $y i32.sub local.set $y2
    local.get $x1 local.get $y2 i32.mul local.get $y1 local.get $x2 i32.mul i32.sub
  )

  (func (export "convex_hull") (param $pts i32) (param $n i32) (param $out i32) (result i32)
    (local $start i32) (local $cur i32) (local $next i32) (local $i i32)
    (local $count i32) (local $cx i32) (local $cy i32) (local $nx i32) (local $ny i32)
    (local $cross_val i32)

    local.get $n i32.const 3 i32.lt_s if
      i32.const 0 return
    end

    ;; Find leftmost (lowest x, tie-break lowest y)
    i32.const 0 local.set $start
    i32.const 1 local.set $i
    block $find_done
    loop $find
      local.get $i local.get $n i32.ge_s br_if $find_done
      local.get $pts local.get $i call $x
      local.get $pts local.get $start call $x
      i32.lt_s
      if
        local.get $i local.set $start
      else
        local.get $pts local.get $i call $x
        local.get $pts local.get $start call $x
        i32.eq
        if
          local.get $pts local.get $i call $y
          local.get $pts local.get $start call $y
          i32.lt_s
          if
            local.get $i local.set $start
          end
        end
      end
      local.get $i i32.const 1 i32.add local.set $i
      br $find
    end
    end

    local.get $start local.set $cur
    i32.const 0 local.set $count

    block $hull_done
    loop $hull
      local.get $out local.get $count i32.const 2 i32.shl i32.add local.get $cur i32.store
      local.get $count i32.const 1 i32.add local.set $count

      ;; Find next point: the most counterclockwise from $cur
      i32.const 0 local.set $next

      ;; Initial candidate: first point != cur
      i32.const 0 local.set $i
      block $find_next_init
      loop $init_loop
        local.get $i local.get $n i32.ge_s br_if $find_next_init
        local.get $i local.get $cur i32.ne if
          local.get $i local.set $next
          br $find_next_init
        end
        local.get $i i32.const 1 i32.add local.set $i
        br $init_loop
      end
      end

      i32.const 0 local.set $i
      block $next_done
      loop $next_loop
        local.get $i local.get $n i32.ge_s br_if $next_done
        local.get $i local.get $cur i32.eq if
          local.get $i i32.const 1 i32.add local.set $i
          br $next_loop
        end
        local.get $pts local.get $cur local.get $next local.get $i call $cross
        local.tee $cross_val
        i32.const 0 i32.gt_s
        if
          local.get $i local.set $next
        else
          local.get $cross_val i32.eqz
          if
            ;; Collinear: pick the farther point
            local.get $pts local.get $i call $x
            local.get $pts local.get $cur call $x i32.sub
            local.tee $cx
            local.get $cx i32.mul
            local.get $pts local.get $i call $y
            local.get $pts local.get $cur call $y i32.sub
            local.tee $cy
            local.get $cy i32.mul i32.add
            local.set $cx
            local.get $pts local.get $next call $x
            local.get $pts local.get $cur call $x i32.sub
            local.tee $nx
            local.get $nx i32.mul
            local.get $pts local.get $next call $y
            local.get $pts local.get $cur call $y i32.sub
            local.tee $ny
            local.get $ny i32.mul i32.add
            local.get $cx i32.lt_s
            if
              local.get $i local.set $next
            end
          end
        end
        local.get $i i32.const 1 i32.add local.set $i
        br $next_loop
      end
      end

      local.get $next local.set $cur
      local.get $cur local.get $start i32.eq br_if $hull_done
      br $hull
    end
    end

    local.get $count
  )
