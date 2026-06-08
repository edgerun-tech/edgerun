;; EdgeRun Privacy Lattice — annotation-based privacy policy enforcement
  ;;
  ;; Privacy levels (ordered from least to most restrictive):
  ;;   0 = public
  ;;   1 = internal
  ;;   2 = private
  ;;   3 = confidential
  ;;   4 = encrypted
  ;;
  ;; Rules:
  ;;   - Data may flow from level N to level M iff N <= M (ascending only).
  ;;   - @encrypted on edge annotation bypasses all restrictions.
  ;;   - Unannotated nodes default to internal (level 1).
  ;;   - Node-level privacy comes from placement annotation keywords.
  ;;   - Return value: number of violations found (0 = clean).

  ;; Privacy level constants
  (global $PRIV_PUBLIC       i32 (i32.const 0))
  (global $PRIV_INTERNAL     i32 (i32.const 1))
  (global $PRIV_PRIVATE      i32 (i32.const 2))
  (global $PRIV_CONFIDENTIAL i32 (i32.const 3))
  (global $PRIV_ENCRYPTED    i32 (i32.const 4))

  ;; Keyword data region for privacy checks
  (global $ER_PRIV_KW_REGION i32 (i32.const 0x8F400))

  (data (i32.const 0x8F400) "public")
  (data (i32.const 0x8F410) "internal")
  (data (i32.const 0x8F420) "private")
  (data (i32.const 0x8F430) "confidential")
  (data (i32.const 0x8F440) "encrypted")

  ;; Check if annotation string contains a given keyword.
  ;; $src = source buffer, $len = length, $kw = keyword data offset, $kw_len = keyword length
  (func $er_anno_has_kw (param $src i32) (param $len i32) (param $kw i32) (param $kw_len i32) (result i32)
    (local $i i32) (local $j i32) (local $b1 i32) (local $b2 i32)
    (block $done
      (loop $outer
        (if (i32.ge_u (local.get $i) (local.get $len)) (then (br $done)))
        (local.set $b1 (i32.load8_u (i32.add (local.get $src) (local.get $i))))
        (if (i32.eq (local.get $b1) (i32.load8_u (local.get $kw)))
          (then
            (local.set $j (i32.const 0))
            (block $inner_done
              (loop $inner
                (if (i32.ge_u (local.get $j) (local.get $kw_len)) (then (br $inner_done)))
                (local.set $b1 (i32.load8_u (i32.add (local.get $src) (i32.add (local.get $i) (local.get $j)))))
                (local.set $b2 (i32.load8_u (i32.add (local.get $kw) (local.get $j))))
                (if (i32.ne (local.get $b1) (local.get $b2)) (then (local.set $j (i32.const -1)) (br $inner_done)))
                (local.set $j (i32.add (local.get $j) (i32.const 1)))
              )
            )
            (if (i32.ge_u (local.get $j) (local.get $kw_len)) (then (return (i32.const 1))))
          )
        )
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $outer)
      )
    )
    (i32.const 0)
  )

  ;; Determine privacy level of a node from its placement annotation.
  ;; Returns PRIV level (defaults to PRIV_INTERNAL if no keyword found).
  (func $er_node_privacy_level (param $node i32) (result i32)
    (local $p_off i32) (local $p_len i32) (local $src i32)
    (local.set $p_off (i32.load offset=20 (local.get $node)))  ;; placement
    (if (i32.eqz (local.get $p_off)) (then (return (global.get $PRIV_INTERNAL))))
    ;; Get str_len from graph's str_buf — not stored per string.
    ;; Instead: walk from p_off until null/end-of-heap.
    ;; For now: scan for known privacy keywords by searching the string buffer.
    ;; Use a fixed max scan length.
    (local.set $src (local.get $p_off))
    ;; Scan for "encrypted" (9 chars) at 0x8F440
    (if (call $er_anno_has_kw (local.get $src) (i32.const 64) (global.get $ER_PRIV_KW_REGION) (i32.const 7))  ;; "public"
      (then (return (global.get $PRIV_PUBLIC)))
    )
    (if (call $er_anno_has_kw (local.get $src) (i32.const 64) (i32.const 0x8F410) (i32.const 8))  ;; "internal"
      (then (return (global.get $PRIV_INTERNAL)))
    )
    (if (call $er_anno_has_kw (local.get $src) (i32.const 64) (i32.const 0x8F420) (i32.const 7))  ;; "private"
      (then (return (global.get $PRIV_PRIVATE)))
    )
    (if (call $er_anno_has_kw (local.get $src) (i32.const 64) (i32.const 0x8F430) (i32.const 11))  ;; "confidential"
      (then (return (global.get $PRIV_CONFIDENTIAL)))
    )
    (if (call $er_anno_has_kw (local.get $src) (i32.const 64) (i32.const 0x8F440) (i32.const 9))  ;; "encrypted"
      (then (return (global.get $PRIV_ENCRYPTED)))
    )
    (global.get $PRIV_INTERNAL)
  )

  ;; Check if an edge has @encrypted annotation.
  (func $er_edge_is_encrypted (param $e i32) (result i32)
    (local $anno_off i32) (local $anno_len i32)
    (local.set $anno_off (i32.load offset=24 (local.get $e)))
    (if (i32.eqz (local.get $anno_off)) (then (return (i32.const 0))))
    (local.set $anno_len (i32.load offset=28 (local.get $e)))
    (call $er_anno_has_kw (local.get $anno_off) (local.get $anno_len) (i32.const 0x8F440) (i32.const 9))  ;; "encrypted"
  )

  ;; Check port annotation for privacy keywords.
  ;; Returns privacy level or -1 if no privacy annotation found.
  (func $er_port_privacy_level (param $port i32) (result i32)
    (local $anno_off i32)
    (local.set $anno_off (i32.load offset=24 (local.get $port)))
    (if (i32.eqz (local.get $anno_off)) (then (return (i32.const -1))))
    (if (call $er_anno_has_kw (local.get $anno_off) (i32.const 64) (global.get $ER_PRIV_KW_REGION) (i32.const 7))
      (then (return (global.get $PRIV_PUBLIC)))
    )
    (if (call $er_anno_has_kw (local.get $anno_off) (i32.const 64) (i32.const 0x8F420) (i32.const 7))
      (then (return (global.get $PRIV_PRIVATE)))
    )
    (if (call $er_anno_has_kw (local.get $anno_off) (i32.const 64) (i32.const 0x8F430) (i32.const 11))
      (then (return (global.get $PRIV_CONFIDENTIAL)))
    )
    (if (call $er_anno_has_kw (local.get $anno_off) (i32.const 64) (i32.const 0x8F440) (i32.const 9))
      (then (return (global.get $PRIV_ENCRYPTED)))
    )
    (i32.const -1)
  )

  ;; Find a port on a node by name.
  ;; Returns port offset or 0.
  (func $er_find_port (param $node i32) (param $port_name i32) (param $port_len i32) (result i32)
    (local $p i32) (local $pn_off i32) (local $pn_len i32) (local $i i32) (local $b1 i32) (local $b2 i32)
    (local.set $p (i32.load offset=12 (local.get $node)))
    (block $done
      (loop $lp
        (if (i32.eqz (local.get $p)) (then (br $done)))
        (local.set $pn_len (i32.load offset=8 (local.get $p)))
        (if (i32.eq (local.get $pn_len) (local.get $port_len))
          (then
            (local.set $pn_off (i32.load offset=4 (local.get $p)))
            (local.set $i (i32.const 0))
            (block $cmp_done
              (loop $cmp_lp
                (if (i32.ge_u (local.get $i) (local.get $pn_len)) (then (br $cmp_done)))
                (local.set $b1 (i32.load8_u (i32.add (local.get $pn_off) (local.get $i))))
                (local.set $b2 (i32.load8_u (i32.add (local.get $port_name) (local.get $i))))
                (if (i32.ne (local.get $b1) (local.get $b2)) (then (local.set $i (i32.const -1)) (br $cmp_done)))
                (local.set $i (i32.add (local.get $i) (i32.const 1)))
                (br $cmp_lp)
              )
            )
            (if (i32.ge_u (local.get $i) (local.get $pn_len)) (then (return (local.get $p))))
          )
        )
        (local.set $p (i32.load offset=20 (local.get $p)))
        (br $lp)
      )
    )
    (i32.const 0)
  )

  ;; Get the effective privacy level for a given end of an edge.
  ;; $e = edge, $from_or_to = 0 for from, 1 for to
  ;; Returns privacy level (0-4).
  (func $er_edge_end_privacy (param $e i32) (param $from_or_to i32) (result i32)
    (local $node i32) (local $port_name i32) (local $port_len i32) (local $port i32) (local $plevel i32)
    ;; Get node
    (if (i32.eqz (local.get $from_or_to))
      (then
        (local.set $node (i32.load offset=0 (local.get $e)))
        (local.set $port_name (i32.load offset=4 (local.get $e)))
        (local.set $port_len (i32.load offset=8 (local.get $e)))
      )
      (else
        (local.set $node (i32.load offset=12 (local.get $e)))
        (local.set $port_name (i32.load offset=16 (local.get $e)))
        (local.set $port_len (i32.load offset=20 (local.get $e)))
      )
    )
    ;; If port name is given, check port-level privacy
    (if (local.get $port_name)
      (then
        (local.set $port (call $er_find_port (local.get $node) (local.get $port_name) (local.get $port_len)))
        (if (local.get $port)
          (then
            (local.set $plevel (call $er_port_privacy_level (local.get $port)))
            (if (i32.ge_s (local.get $plevel) (i32.const 0))
              (then (return (local.get $plevel)))
            )
          )
        )
      )
    )
    ;; Fall back to node-level privacy
    (call $er_node_privacy_level (local.get $node))
  )

  ;; Check all edges in the graph for privacy violations.
  ;; Returns number of violations (0 = clean).
  (func $er_check_privacy (export "er_check_privacy") (param $g i32) (result i32)
    (local $e i32) (local $from_priv i32) (local $to_priv i32) (local $violations i32)
    (local.set $e (i32.load offset=12 (local.get $g)))
    (block $done
      (loop $lp
        (if (i32.eqz (local.get $e)) (then (br $done)))
        ;; Check if edge is @encrypted (bypasses all checks)
        (if (i32.eqz (call $er_edge_is_encrypted (local.get $e)))
          (then
            (local.set $from_priv (call $er_edge_end_privacy (local.get $e) (i32.const 0)))
            (local.set $to_priv (call $er_edge_end_privacy (local.get $e) (i32.const 1)))
            ;; Violation if from_priv > to_priv (data flows to less restrictive level)
            (if (i32.gt_u (local.get $from_priv) (local.get $to_priv))
              (then (local.set $violations (i32.add (local.get $violations) (i32.const 1))))
            )
          )
        )
        (local.set $e (i32.load offset=32 (local.get $e)))
        (br $lp)
      )
    )
    local.get $violations
  )
