;; Edgerun Resolve — placement resolution
  ;;
  ;; Walks an Edgerun IR graph and resolves @placement/@where/@near
  ;; annotations into a placement plan. Also handles @cross annotations
  ;; on edges for clock domain crossing.
  ;;
  ;; The placement plan is a flat array:
  ;;   +0: count (i32, number of entries)
  ;;   +4: entries[count] of { node, target_off, target_len, flags }
  ;;
  ;; Each entry is 16 bytes:
  ;;   +0: node_offset   (i32)
  ;;   +4: target_off    (i32, into graph str_buf)
  ;;   +8: target_len    (i32)
  ;;   +12: flags        (i32, bitmask: 1=private, 2=cross, 4=pinned)

  (global $ER_PLAN_ENTRY_SIZE i32 (i32.const 16))

  ;; ── Annotation string matching ──

  ;; Check if annotation at $s matches the given literal string.
  ;; Returns 1 if match, 0 otherwise.
  (func $er_anno_match (export "er_anno_match") (param $s i32) (param $lit i32) (param $lit_len i32) (result i32)
    (local $i i32) (local $b1 i32) (local $b2 i32)
    (block $done
      (loop $lp
        (if (i32.ge_u (local.get $i) (local.get $lit_len)) (then (br $done)))
        (local.set $b1 (i32.load8_u (i32.add (local.get $s) (local.get $i))))
        (local.set $b2 (i32.load8_u (i32.add (local.get $lit) (local.get $i))))
        (if (i32.ne (local.get $b1) (local.get $b2)) (then (return (i32.const 0))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $lp)
      )
    )
    (i32.const 1)
  )

  ;; Pre-defined keyword offsets for annotation matching.
  ;; These are in the keyword data area of edgerun-parse.wat.
  ;; "placement" at 0x8F1B0 (9 bytes)
  ;; "where" at 0x8F1C0 (5 bytes)
  ;; "private" at 0x8F1D0 (7 bytes)
  ;; "cross" at 0x8F1E0 (5 bytes)
  ;; "pin" at 0x8F1F0 (3 bytes)

  ;; Check if annotation starts with "placement"
  (func $er_anno_is_placement (export "er_anno_is_placement") (param $s i32) (result i32)
    (call $er_anno_match (local.get $s) (i32.const 0x8F1B0) (i32.const 9))
  )

  ;; Check if annotation starts with "where"
  (func $er_anno_is_where (export "er_anno_is_where") (param $s i32) (result i32)
    (call $er_anno_match (local.get $s) (i32.const 0x8F1C0) (i32.const 5))
  )

  ;; Check if annotation starts with "private"
  (func $er_anno_is_private (export "er_anno_is_private") (param $s i32) (result i32)
    (call $er_anno_match (local.get $s) (i32.const 0x8F1D0) (i32.const 7))
  )

  ;; Check if annotation starts with "cross"
  (func $er_anno_is_cross (export "er_anno_is_cross") (param $s i32) (result i32)
    (call $er_anno_match (local.get $s) (i32.const 0x8F1E0) (i32.const 5))
  )

  ;; Check if annotation starts with "pin"
  (func $er_anno_is_pin (export "er_anno_is_pin") (param $s i32) (result i32)
    (call $er_anno_match (local.get $s) (i32.const 0x8F1F0) (i32.const 3))
  )

  ;; Find the first '(' in an annotation string.
  ;; Returns offset from $s, or -1 if not found.
  (func $er_anno_find_paren (export "er_anno_find_paren") (param $s i32) (param $len i32) (result i32)
    (local $i i32)
    (block $done
      (loop $lp
        (if (i32.ge_u (local.get $i) (local.get $len)) (then (br $done)))
        (if (i32.eq (i32.load8_u (i32.add (local.get $s) (local.get $i))) (i32.const 0x28))
          (then (return (local.get $i)))
        )
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $lp)
      )
    )
    (i32.const -1)
  )

  ;; Skip past the parenthesized argument list.
  ;; $s = start of '(' character
  ;; $len = total length of annotation string from $s
  ;; Returns the character after ')', or the end of the string.
  (func $er_anno_skip_paren (export "er_anno_skip_paren") (param $s i32) (param $len i32) (result i32)
    (local $depth i32) (local $i i32)
    (local.set $depth (i32.const 1))
    (local.set $i (i32.const 1))
    (block $done
      (loop $lp
        (if (i32.ge_u (local.get $i) (local.get $len)) (then (br $done)))
        (if (i32.eq (i32.load8_u (i32.add (local.get $s) (local.get $i))) (i32.const 0x28))
          (then (local.set $depth (i32.add (local.get $depth) (i32.const 1))) (local.set $i (i32.add (local.get $i) (i32.const 1))) (br $lp))
        )
        (if (i32.eq (i32.load8_u (i32.add (local.get $s) (local.get $i))) (i32.const 0x29))
          (then
            (local.set $depth (i32.sub (local.get $depth) (i32.const 1)))
            (if (i32.eqz (local.get $depth)) (then (return (i32.add (local.get $i) (i32.const 1)))))
          )
        )
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $lp)
      )
    )
    local.get $len
  )

  ;; ── Placement plan ──

  ;; Allocate a placement plan. Returns plan offset.
  (func $er_plan_create (export "er_plan_create") (result i32)
    (local $plan i32)
    (local.set $plan (call $pipe_alloc (i32.const 4)))
    (if (i32.eq (local.get $plan) (i32.const -1)) (then (return (i32.const -1))))
    (i32.store (local.get $plan) (i32.const 0))
    local.get $plan
  )

  ;; Add a placement entry to the plan.
  ;; Returns 0 on success, -1 on alloc failure.
  (func $er_plan_add (export "er_plan_add") (param $plan i32) (param $node i32) (param $target_off i32) (param $target_len i32) (param $flags i32) (result i32)
    (local $count i32) (local $entry i32)
    (local.set $count (i32.load (local.get $plan)))
    (local.set $entry (call $pipe_alloc (global.get $ER_PLAN_ENTRY_SIZE)))
    (if (i32.eq (local.get $entry) (i32.const -1)) (then (return (i32.const -1))))
    (i32.store offset=0 (local.get $entry) (local.get $node))
    (i32.store offset=4 (local.get $entry) (local.get $target_off))
    (i32.store offset=8 (local.get $entry) (local.get $target_len))
    (i32.store offset=12 (local.get $entry) (local.get $flags))
    (i32.store (local.get $plan) (i32.add (local.get $count) (i32.const 1)))
    (i32.const 0)
  )

  ;; Resolve placement for a single node.
  ;; Reads node placement annotation, adds entry to plan.
  ;; Returns 0 on success, -1 on error.
  (func $er_resolve_node (export "er_resolve_node") (param $plan i32) (param $g i32) (param $node i32) (result i32)
    (local $p_off i32) (local $p i32) (local $len i32)
    (local $paren i32) (local $target_off i32) (local $target_len i32)
    (local $flags i32)
    (local.set $p_off (i32.load offset=20 (local.get $node)))
    (if (i32.eqz (local.get $p_off)) (then (return (i32.const 0))))

    (local.set $p (i32.add (i32.load offset=16 (local.get $g)) (local.get $p_off)))
    (local.set $len (call $strlen (local.get $p)))

    (if (call $er_anno_is_private (local.get $p))
      (then
        (return (call $er_plan_add (local.get $plan) (local.get $node) (i32.const 0) (i32.const 0) (i32.const 1)))
      )
    )

    (if (call $er_anno_is_placement (local.get $p))
      (then
        (local.set $paren (call $er_anno_find_paren (local.get $p) (local.get $len)))
        (if (i32.ge_s (local.get $paren) (i32.const 0))
          (then
            (local.set $target_off (i32.add (local.get $p_off) (i32.add (local.get $paren) (i32.const 1))))
            (local.set $target_len (i32.sub
              (call $er_anno_skip_paren (i32.add (local.get $p) (local.get $paren)) (i32.sub (local.get $len) (local.get $paren)))
              (i32.add (local.get $paren) (i32.const 2))))
            (return (call $er_plan_add (local.get $plan) (local.get $node) (local.get $target_off) (local.get $target_len) (i32.const 0)))
          )
        )
        (return (call $er_plan_add (local.get $plan) (local.get $node) (local.get $p_off) (local.get $len) (i32.const 0)))
      )
    )

    (if (call $er_anno_is_where (local.get $p))
      (then
        (local.set $paren (call $er_anno_find_paren (local.get $p) (local.get $len)))
        (if (i32.ge_s (local.get $paren) (i32.const 0))
          (then
            (local.set $target_off (i32.add (local.get $p_off) (i32.add (local.get $paren) (i32.const 1))))
            (local.set $target_len (i32.sub
              (call $er_anno_skip_paren (i32.add (local.get $p) (local.get $paren)) (i32.sub (local.get $len) (local.get $paren)))
              (i32.add (local.get $paren) (i32.const 2))))
            (return (call $er_plan_add (local.get $plan) (local.get $node) (local.get $target_off) (local.get $target_len) (i32.const 0)))
          )
        )
        (return (i32.const -1))
      )
    )

    (if (call $er_anno_is_pin (local.get $p))
      (then
        (return (call $er_plan_add (local.get $plan) (local.get $node) (local.get $p_off) (local.get $len) (i32.const 4)))
      )
    )

    ;; Unknown annotation — treat the whole string as a placement target
    (call $er_plan_add (local.get $plan) (local.get $node) (local.get $p_off) (local.get $len) (i32.const 0))
  )

  ;; Main entry point: resolve all placements in a graph.
  ;; $g = graph offset (from er_graph_create)
  ;; Returns plan offset, or -1 on error.
  (func $er_resolve (export "er_resolve") (param $g i32) (result i32)
    (local $plan i32) (local $n i32)
    (local.set $plan (call $er_plan_create))
    (if (i32.eq (local.get $plan) (i32.const -1)) (then (return (i32.const -1))))
    (local.set $n (i32.load offset=8 (local.get $g)))
    (block $done
      (loop $lp
        (if (i32.eqz (local.get $n)) (then (br $done)))
        (if (call $er_resolve_node (local.get $plan) (local.get $g) (local.get $n))
          (then (return (i32.const -1)))
        )
        (local.set $n (i32.load offset=16 (local.get $n)))
        (br $lp)
      )
    )
    local.get $plan
  )
