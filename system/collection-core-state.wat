;; Captures tinyvec/smallvec/slab/sharded-slab collection state semantics.
  ;; Vec kind: 1 inline, 2 heap/spilled.
  (func $coll_inline_or_heap (export "coll_inline_or_heap") (param $len i32) (param $inline_cap i32) (result i32)
    (select (i32.const 1) (i32.const 2) (i32.le_u (local.get $len) (local.get $inline_cap))))

  (func (export "coll_tinyvec_constructor") (param $elem_count i32) (param $inline_cap i32) (result i32)
    (call $coll_inline_or_heap (local.get $elem_count) (local.get $inline_cap)))

  ;; Reserve spills when inline len + additional exceeds inline capacity.
  (func (export "coll_reserve_spills")
    (param $is_heap i32) (param $len i32) (param $inline_cap i32) (param $additional i32)
    (result i32)
    (if (local.get $is_heap) (then (return (i32.const 1))))
    (select (i32.const 0) (i32.const 1)
      (i32.le_u (i32.add (local.get $len) (local.get $additional)) (local.get $inline_cap))))

  ;; SmallVec reserve_one grows to next_power_of_two(len + 1), at least inline size.
  (func $coll_next_power_two (export "coll_next_power_two") (param $n i32) (result i32)
    (local $cap i32)
    (local.set $cap (i32.const 1))
    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $cap) (local.get $n)))
        (if (i32.ge_u (local.get $cap) (i32.const 0x40000000)) (then (return (i32.const 0x80000000))))
        (local.set $cap (i32.shl (local.get $cap) (i32.const 1)))
        (br $loop)))
    (local.get $cap))

  (func (export "coll_smallvec_grow_cap") (param $len i32) (param $additional i32) (param $inline_cap i32) (result i32)
    (local $need i32) (local $cap i32)
    (local.set $need (i32.add (local.get $len) (local.get $additional)))
    (local.set $cap (call $coll_next_power_two (local.get $need)))
    (select (local.get $cap) (local.get $inline_cap) (i32.gt_u (local.get $cap) (local.get $inline_cap))))

  (func (export "coll_insert_result_len") (param $len i32) (param $index i32) (result i32)
    (if (i32.gt_u (local.get $index) (local.get $len)) (then (return (i32.const -1))))
    (i32.add (local.get $len) (i32.const 1)))

  (func (export "coll_remove_result_len") (param $len i32) (param $index i32) (result i32)
    (if (i32.ge_u (local.get $index) (local.get $len)) (then (return (i32.const -1))))
    (i32.sub (local.get $len) (i32.const 1)))

  ;; Drain removes [start,end) immediately; iterator consumption does not affect final len.
