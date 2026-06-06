(module
  (import "edgerun-core" "memory" (memory 1))
;; Captures tinyvec/smallvec/slab/sharded-slab collection state semantics.
  (func (export "proto_standard_id") (result i32) (i32.const 300139))

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
  (func (export "coll_drain_result_len") (param $len i32) (param $start i32) (param $end i32) (result i32)
    (if (i32.or (i32.gt_u (local.get $start) (local.get $end)) (i32.gt_u (local.get $end) (local.get $len)))
      (then (return (i32.const -1))))
    (i32.sub (local.get $len) (i32.sub (local.get $end) (local.get $start))))

  ;; retain/drain_filter keeps order; result len is original minus removed.
  (func (export "coll_filter_result_len") (param $len i32) (param $removed i32) (result i32)
    (if (i32.gt_u (local.get $removed) (local.get $len)) (then (return (i32.const -1))))
    (i32.sub (local.get $len) (local.get $removed)))

  ;; Slab insert uses next vacant if available, otherwise appends at capacity.
  ;; packed low16=key high16=new_len.
  (func (export "coll_slab_insert_result") (param $len i32) (param $capacity i32) (param $next i32) (result i32)
    (local $key i32)
    (local.set $key (select (local.get $next) (local.get $capacity) (i32.lt_u (local.get $next) (local.get $capacity))))
    (i32.or (i32.and (local.get $key) (i32.const 0xffff))
            (i32.shl (i32.and (i32.add (local.get $len) (i32.const 1)) (i32.const 0xffff)) (i32.const 16))))

  ;; Removing occupied slot pushes it onto vacant stack and decrements len.
  ;; packed low16=new_next high16=new_len; invalid => -1.
  (func (export "coll_slab_remove_result")
    (param $len i32) (param $capacity i32) (param $key i32) (param $occupied i32)
    (result i32)
    (if (i32.or (i32.eqz (local.get $occupied)) (i32.ge_u (local.get $key) (local.get $capacity)))
      (then (return (i32.const -1))))
    (i32.or (i32.and (local.get $key) (i32.const 0xffff))
            (i32.shl (i32.and (i32.sub (local.get $len) (i32.const 1)) (i32.const 0xffff)) (i32.const 16))))

  ;; get_disjoint_mut error codes: 0 ok, 1 vacant, 2 out-of-bounds, 3 overlap.
  (func (export "coll_slab_disjoint_error") (param $in_bounds i32) (param $occupied i32) (param $overlap i32) (result i32)
    (if (i32.eqz (local.get $in_bounds)) (then (return (i32.const 2))))
    (if (i32.eqz (local.get $occupied)) (then (return (i32.const 1))))
    (if (local.get $overlap) (then (return (i32.const 3))))
    (i32.const 0))

  ;; Sharded slab lifecycle states: present=0, marked=1, removing=3.
  ;; get succeeds only when generation matches and state is present; it increments refs.
  (func (export "coll_sharded_get_result")
    (param $requested_gen i32) (param $current_gen i32) (param $state i32) (param $refs i32)
    (result i32)
    (if (i32.or (i32.ne (local.get $requested_gen) (local.get $current_gen)) (i32.ne (local.get $state) (i32.const 0)))
      (then (return (i32.const -1))))
    (i32.add (local.get $refs) (i32.const 1)))

  ;; mark_release returns 0 absent, 1 can remove now, 2 deferred until refs drop.
  (func (export "coll_sharded_mark_release")
    (param $requested_gen i32) (param $current_gen i32) (param $state i32) (param $refs i32)
    (result i32)
    (if (i32.or (i32.ne (local.get $requested_gen) (local.get $current_gen)) (i32.eq (local.get $state) (i32.const 3)))
      (then (return (i32.const 0))))
    (select (i32.const 1) (i32.const 2) (i32.eqz (local.get $refs))))

  ;; Generation advances and wraps within configured bit mask.
  (func (export "coll_generation_advance") (param $gen i32) (param $mask i32) (result i32)
    (i32.and (i32.add (local.get $gen) (i32.const 1)) (local.get $mask)))

  ;; Key packing: low slot bits plus high generation bits.
  (func (export "coll_pack_key") (param $slot i32) (param $gen i32) (param $slot_bits i32) (result i32)
    (i32.or (local.get $slot) (i32.shl (local.get $gen) (local.get $slot_bits))))

  (func (export "coll_unpack_slot") (param $key i32) (param $slot_mask i32) (result i32)
    (i32.and (local.get $key) (local.get $slot_mask)))

  (func (export "coll_unpack_generation") (param $key i32) (param $slot_bits i32) (param $gen_mask i32) (result i32)
    (i32.and (i32.shr_u (local.get $key) (local.get $slot_bits)) (local.get $gen_mask)))

  ;; Pool clear preserves storage allocation but resets logical occupancy.
  (func (export "coll_pool_clear_len") (param $len i32) (result i32)
    (drop (local.get $len))
    (i32.const 0))
)
