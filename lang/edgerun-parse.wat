;; Parser scratch globals (offsets in Edgerun reserved region)
  (global $OFF_SCRATCH4 i32 (i32.const 0x8E100))
  (global $OFF_SCRATCH5 i32 (i32.const 0x8E104))
  (global $OFF_SCRATCH6 i32 (i32.const 0x8E108))
  (global $OFF_SCRATCH7 i32 (i32.const 0x8E10C))
  (global $OFF_SCRATCH8 i32 (i32.const 0x8E110))
  (global $OFF_SCRATCH9 i32 (i32.const 0x8E114))

  ;; Edgerun Parser — reads .edgerun source (compact arrow syntax), builds IR graph
  ;;
  ;; Compact syntax:
  ;;   flow Name {
  ;;     clocks name=type, ...
  ;;     lanes by placement|node
  ;;     object name: Type @placement { ... }
  ;;     buffer name: Type @placement { ... }
  ;;     node.port -> stage -> node.port
  ;;     mux Name: Type<...> { ... }
  ;;     demux Name: Type<...> { ... }
  ;;     gate Name { ... }
  ;;     kernel Name { pipe { ... } }
  ;;   }
  ;;
  ;; Reuses interpreter.wat lexer: $wat_skip_ws, $wat_match_kw, $wat_read_kw
  ;; Output: $OFF_SCRATCH0=anno_off, $OFF_SCRATCH1=anno_len, $OFF_SCRATCH2=new_pos

  ;; ── Edgerun-specific skip whitespace ──
  ;; Handles space, tab, CR, LF, // and ;; line comments, /* */ block comments.
  ;; Returns 0; new position in scratch0.
  (func $er_skip_ws (param $pos i32) (result i32)
    (local $p i32) (local $end i32) (local $b i32) (local $b2 i32)
    (local.set $p (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos)))
    (local.set $end (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.load (global.get $OFF_WAT_LEN))))
    (block $done
      (loop $lp
        (if (i32.ge_u (local.get $p) (local.get $end)) (then (br $done)))
        (local.set $b (i32.load8_u (local.get $p)))
        (if (i32.eq (local.get $b) (i32.const 0x20)) (then (local.set $p (i32.add (local.get $p) (i32.const 1))) (br $lp)))
        (if (i32.eq (local.get $b) (i32.const 0x09)) (then (local.set $p (i32.add (local.get $p) (i32.const 1))) (br $lp)))
        (if (i32.eq (local.get $b) (i32.const 0x0A)) (then (local.set $p (i32.add (local.get $p) (i32.const 1))) (br $lp)))
        (if (i32.eq (local.get $b) (i32.const 0x0D)) (then (local.set $p (i32.add (local.get $p) (i32.const 1))) (br $lp)))
        ;; Line comment: ;; or //
        (if (i32.eq (local.get $b) (i32.const 0x3B))
          (then
            (local.set $b2 (i32.load8_u (i32.add (local.get $p) (i32.const 1))))
            (if (i32.eq (local.get $b2) (i32.const 0x3B))
              (then
                (local.set $p (i32.add (local.get $p) (i32.const 2)))
                (block $eol
                  (loop $eol_lp
                    (if (i32.ge_u (local.get $p) (local.get $end)) (then (br $eol)))
                    (local.set $b (i32.load8_u (local.get $p)))
                    (local.set $p (i32.add (local.get $p) (i32.const 1)))
                    (if (i32.eq (local.get $b) (i32.const 0x0A)) (then (br $eol)))
                    (br $eol_lp)
                  )
                )
                (br $lp)
              )
            )
            (br $done)
          )
        )
        (if (i32.eq (local.get $b) (i32.const 0x2F))
          (then
            (local.set $b2 (i32.load8_u (i32.add (local.get $p) (i32.const 1))))
            (if (i32.eq (local.get $b2) (i32.const 0x2F))
              (then
                (local.set $p (i32.add (local.get $p) (i32.const 2)))
                (block $eol2
                  (loop $eol2_lp
                    (if (i32.ge_u (local.get $p) (local.get $end)) (then (br $eol2)))
                    (local.set $b (i32.load8_u (local.get $p)))
                    (local.set $p (i32.add (local.get $p) (i32.const 1)))
                    (if (i32.eq (local.get $b) (i32.const 0x0A)) (then (br $eol2)))
                    (br $eol2_lp)
                  )
                )
                (br $lp)
              )
            )
            (if (i32.eq (local.get $b2) (i32.const 0x2A))
              (then
                (local.set $p (i32.add (local.get $p) (i32.const 2)))
                (block $bce
                  (loop $bcl
                    (if (i32.ge_u (local.get $p) (local.get $end)) (then (br $bce)))
                    (local.set $b (i32.load8_u (local.get $p)))
                    (local.set $p (i32.add (local.get $p) (i32.const 1)))
                    (if (i32.eq (local.get $b) (i32.const 0x2A))
                      (then
                        (if (i32.lt_u (local.get $p) (local.get $end))
                          (then
                            (local.set $b (i32.load8_u (local.get $p)))
                            (if (i32.eq (local.get $b) (i32.const 0x2F))
                              (then (local.set $p (i32.add (local.get $p) (i32.const 1))) (br $bce))
                            )
                          )
                        )
                        (br $bcl)
                      )
                    )
                    (br $bcl)
                  )
                )
                (br $lp)
              )
            )
            (br $done)
          )
        )
        (br $done)
      )
    )
    (i32.store (global.get $OFF_SCRATCH0) (i32.sub (local.get $p) (i32.load (global.get $OFF_WAT_PTR))))
    (i32.const 0)
  )

  ;; ── Arrow check ──
  (func $er_match_arrow (param $pos i32) (result i32)
    (local $p i32) (local $end i32)
    (local.set $p (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos)))
    (local.set $end (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.load (global.get $OFF_WAT_LEN))))
    (if (i32.le_u (i32.add (local.get $p) (i32.const 2)) (local.get $end))
      (then
        (if (i32.and
              (i32.eq (i32.load8_u (local.get $p)) (i32.const 0x2D))
              (i32.eq (i32.load8_u (i32.add (local.get $p) (i32.const 1))) (i32.const 0x3E)))
          (then (return (i32.const 1)))
        )
      )
    )
    (i32.const 0)
  )

  ;; ── Annotation reader ──
  ;; Reads @annotation at pos. Output: scratch0=off, scratch1=len, scratch2=new_pos
  (func $er_read_annotation (param $pos i32) (result i32)
    (local $p i32) (local $end i32) (local $start i32) (local $b i32)
    (local.set $p (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos)))
    (local.set $end (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.load (global.get $OFF_WAT_LEN))))
    (if (i32.ge_u (local.get $p) (local.get $end)) (then (return (i32.const 1))))
    (if (i32.ne (i32.load8_u (local.get $p)) (i32.const 0x40)) (then (return (i32.const 1))))
    (local.set $p (i32.add (local.get $p) (i32.const 1)))
    (local.set $start (local.get $p))
    (block $done
      (loop $lp
        (if (i32.ge_u (local.get $p) (local.get $end)) (then (br $done)))
        (local.set $b (i32.load8_u (local.get $p)))
        (block $is_id
          (if (i32.and (i32.ge_u (local.get $b) (i32.const 0x61)) (i32.le_u (local.get $b) (i32.const 0x7A))) (then (br $is_id)))
          (if (i32.and (i32.ge_u (local.get $b) (i32.const 0x41)) (i32.le_u (local.get $b) (i32.const 0x5A))) (then (br $is_id)))
          (if (i32.and (i32.ge_u (local.get $b) (i32.const 0x30)) (i32.le_u (local.get $b) (i32.const 0x39))) (then (br $is_id)))
          (if (i32.eq (local.get $b) (i32.const 0x5F)) (then (br $is_id)))
          (if (i32.eq (local.get $b) (i32.const 0x2E)) (then (br $is_id)))
          (if (i32.eq (local.get $b) (i32.const 0x28)) (then (br $is_id)))  ;; (
          (if (i32.eq (local.get $b) (i32.const 0x29)) (then (br $is_id)))  ;; )
          (if (i32.eq (local.get $b) (i32.const 0x3C)) (then (br $is_id)))  ;; <
          (if (i32.eq (local.get $b) (i32.const 0x3E)) (then (br $is_id)))  ;; >
          (if (i32.eq (local.get $b) (i32.const 0x2C)) (then (br $is_id)))  ;; ,
          (if (i32.eq (local.get $b) (i32.const 0x2B)) (then (br $is_id)))  ;; +
          (br $done)
        )
        (local.set $p (i32.add (local.get $p) (i32.const 1)))
        (br $lp)
      )
    )
    (if (i32.eq (local.get $start) (local.get $p)) (then (return (i32.const 1))))
    (i32.store (global.get $OFF_SCRATCH0) (i32.sub (local.get $start) (i32.load (global.get $OFF_WAT_PTR))))
    (i32.store (global.get $OFF_SCRATCH1) (i32.sub (local.get $p) (local.get $start)))
    (i32.store (global.get $OFF_SCRATCH2) (i32.sub (local.get $p) (i32.load (global.get $OFF_WAT_PTR))))
    (i32.const 0)
  )

  ;; ── String reader ──
  (func $er_read_string (param $pos i32) (result i32)
    (local $p i32) (local $end i32) (local $quote i32) (local $start i32) (local $b i32)
    (local.set $p (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos)))
    (local.set $end (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.load (global.get $OFF_WAT_LEN))))
    (if (i32.ge_u (local.get $p) (local.get $end)) (then (return (i32.const 1))))
    (local.set $quote (i32.load8_u (local.get $p)))
    (if (i32.and (i32.ne (local.get $quote) (i32.const 0x22)) (i32.ne (local.get $quote) (i32.const 0x27)))
      (then (return (i32.const 1)))
    )
    (local.set $p (i32.add (local.get $p) (i32.const 1)))
    (local.set $start (local.get $p))
    (block $done
      (loop $lp
        (if (i32.ge_u (local.get $p) (local.get $end)) (then (br $done)))
        (local.set $b (i32.load8_u (local.get $p)))
        (if (i32.eq (local.get $b) (local.get $quote))
          (then
            (i32.store (global.get $OFF_SCRATCH0) (i32.sub (local.get $start) (i32.load (global.get $OFF_WAT_PTR))))
            (i32.store (global.get $OFF_SCRATCH1) (i32.sub (local.get $p) (local.get $start)))
            (i32.store (global.get $OFF_SCRATCH2) (i32.sub (i32.add (local.get $p) (i32.const 1)) (i32.load (global.get $OFF_WAT_PTR))))
            (return (i32.const 0))
          )
        )
        (if (i32.eq (local.get $b) (i32.const 0x5C))
          (then (local.set $p (i32.add (local.get $p) (i32.const 2))) (br $lp))
        )
        (local.set $p (i32.add (local.get $p) (i32.const 1)))
        (br $lp)
      )
    )
    (i32.store (global.get $OFF_SCRATCH0) (i32.sub (local.get $start) (i32.load (global.get $OFF_WAT_PTR))))
    (i32.store (global.get $OFF_SCRATCH1) (i32.sub (local.get $p) (local.get $start)))
    (i32.store (global.get $OFF_SCRATCH2) (local.get $pos))
    (i32.const 1)
  )

  ;; ── Number reader (with optional unit suffix) ──
  (func $er_read_number (param $pos i32) (result i32)
    (local $p i32) (local $end i32) (local $start i32) (local $b i32)
    (local.set $p (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos)))
    (local.set $end (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.load (global.get $OFF_WAT_LEN))))
    (if (i32.ge_u (local.get $p) (local.get $end)) (then (return (i32.const 1))))
    (local.set $start (local.get $p))
    (if (call $wat_read_uint (local.get $pos)) (then (return (i32.const 1))))
    (local.set $p (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.add (local.get $pos) (i32.load (global.get $OFF_SCRATCH1)))))
    (block $unit_done
      (loop $unit_lp
        (if (i32.ge_u (local.get $p) (local.get $end)) (then (br $unit_done)))
        (local.set $b (i32.load8_u (local.get $p)))
        (if (i32.lt_u (local.get $b) (i32.const 0x41)) (then (br $unit_done)))
        (if (i32.and (i32.le_u (local.get $b) (i32.const 0x5A)) (i32.ge_u (local.get $b) (i32.const 0x41)))
          (then (local.set $p (i32.add (local.get $p) (i32.const 1))) (br $unit_lp))
        )
        (if (i32.lt_u (local.get $b) (i32.const 0x61)) (then (br $unit_done)))
        (if (i32.le_u (local.get $b) (i32.const 0x7A))
          (then (local.set $p (i32.add (local.get $p) (i32.const 1))) (br $unit_lp))
        )
        (br $unit_done)
      )
    )
    (i32.store (global.get $OFF_SCRATCH0) (local.get $pos))
    (i32.store (global.get $OFF_SCRATCH1) (i32.sub (local.get $p) (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
    (i32.store (global.get $OFF_SCRATCH2)
      (i32.add (local.get $pos) (i32.sub (local.get $p) (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos)))))
    (i32.const 0)
  )

  ;; ── Keyword string data ──
  (data (i32.const 0x8F000) "flow")
  (data (i32.const 0x8F010) "node")
  (data (i32.const 0x8F020) "actor")
  (data (i32.const 0x8F030) "wire")
  (data (i32.const 0x8F040) "clock")
  (data (i32.const 0x8F050) "queue")
  (data (i32.const 0x8F060) "in")
  (data (i32.const 0x8F070) "out")
  (data (i32.const 0x8F080) "clocks")
  (data (i32.const 0x8F090) "lanes")
  (data (i32.const 0x8F0A0) "object")
  (data (i32.const 0x8F0B0) "buffer")
  (data (i32.const 0x8F0C0) "mux")
  (data (i32.const 0x8F0D0) "demux")
  (data (i32.const 0x8F0E0) "join")
  (data (i32.const 0x8F0F0) "gate")
  (data (i32.const 0x8F100) "kernel")
  (data (i32.const 0x8F110) "batch")
  (data (i32.const 0x8F120) "reduce")
  (data (i32.const 0x8F130) "map")
  (data (i32.const 0x8F140) "arbiter")
  (data (i32.const 0x8F150) "feedback")
  (data (i32.const 0x8F160) "pipe")
  (data (i32.const 0x8F170) "by")
  (data (i32.const 0x8F180) "Queue")
  (data (i32.const 0x8F190) "Stream")
  (data (i32.const 0x8F1A0) "cdc")
  (data (i32.const 0x8F1B0) "placement")
  (data (i32.const 0x8F1C0) "where")
  (data (i32.const 0x8F1D0) "private")
  (data (i32.const 0x8F1E0) "cross")
  (data (i32.const 0x8F1F0) "pin")
  (data (i32.const 0x8F1F4) "state")
  (data (i32.const 0x8F200) "inout")
  (data (i32.const 0x8F210) "split")
  (data (i32.const 0x8F220) "max_items")
  (data (i32.const 0x8F230) "max_delay")
  (data (i32.const 0x8F240) "max_bytes")
  (data (i32.const 0x8F250) "group_by")
  (data (i32.const 0x8F260) "op")
  (data (i32.const 0x8F270) "window")
  (data (i32.const 0x8F280) "resource")
  (data (i32.const 0x8F290) "select")
  (data (i32.const 0x8F2A0) "pass")
  (data (i32.const 0x8F2B0) "delay")
  (data (i32.const 0x8F2C0) "policy")
  (data (i32.const 0x8F2D0) "priority")
  (data (i32.const 0x8F2E0) "retry")
  (data (i32.const 0x8F2F0) "workers")
  (data (i32.const 0x8F300) "partition")

  ;; ── Brace skipper ──
  (func $skip_braces (param $pos i32) (result i32)
    (local $depth i32) (local $b i32)
    (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
    (local.set $depth (i32.const 1))
    (block $done
      (loop $lp
        (if (i32.ge_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))
                      (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.load (global.get $OFF_WAT_LEN))))
          (then (br $done))
        )
        (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
        (if (i32.eq (local.get $b) (i32.const 0x7B)) (then (local.set $depth (i32.add (local.get $depth) (i32.const 1)))))
        (if (i32.eq (local.get $b) (i32.const 0x7D))
          (then
            (local.set $depth (i32.sub (local.get $depth) (i32.const 1)))
            (if (i32.eqz (local.get $depth))
              (then (local.set $pos (i32.add (local.get $pos) (i32.const 1))) (br $done))
            )
          )
        )
        (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
        (br $lp)
      )
    )
    local.get $pos
  )

  ;; ── Read dotted identifier (node.port) ──
  ;; Output: scratch0=full_off, scratch1=full_len, scratch2=new_pos
  ;; If dot found: scratch0=node_part_offset, scratch1=node_part_len, 
  ;;               scratch2=dot_offset, scratch3=port_part_off, scratch4=port_part_len
  ;; Uses scratch3 and scratch4 for the port part.
  (func $er_read_dotted (param $pos i32) (result i32)
    (local $p i32) (local $end i32) (local $start i32) (local $b i32)
    (local $dot i32)
    (local.set $p (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos)))
    (local.set $end (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.load (global.get $OFF_WAT_LEN))))
    (local.set $start (local.get $p))
    (local.set $dot (i32.const 0))
    (block $done
      (loop $lp
        (if (i32.ge_u (local.get $p) (local.get $end)) (then (br $done)))
        (local.set $b (i32.load8_u (local.get $p)))
        (if (i32.eq (local.get $b) (i32.const 0x2E))  ;; .
          (then
            (if (i32.eqz (local.get $dot)) (then (local.set $dot (local.get $p))))
            (local.set $p (i32.add (local.get $p) (i32.const 1)))
            (br $lp)
          )
        )
        (if (i32.and (i32.ge_u (local.get $b) (i32.const 0x61)) (i32.le_u (local.get $b) (i32.const 0x7A))) (then (local.set $p (i32.add (local.get $p) (i32.const 1))) (br $lp)))
        (if (i32.and (i32.ge_u (local.get $b) (i32.const 0x41)) (i32.le_u (local.get $b) (i32.const 0x5A))) (then (local.set $p (i32.add (local.get $p) (i32.const 1))) (br $lp)))
        (if (i32.and (i32.ge_u (local.get $b) (i32.const 0x30)) (i32.le_u (local.get $b) (i32.const 0x39))) (then (local.set $p (i32.add (local.get $p) (i32.const 1))) (br $lp)))
        (if (i32.eq (local.get $b) (i32.const 0x5F)) (then (local.set $p (i32.add (local.get $p) (i32.const 1))) (br $lp)))
        (br $done)
      )
    )
    (if (i32.eq (local.get $start) (local.get $p)) (then (return (i32.const 1))))
    (i32.store (global.get $OFF_SCRATCH0) (i32.sub (local.get $start) (i32.load (global.get $OFF_WAT_PTR))))
    (i32.store (global.get $OFF_SCRATCH1) (i32.sub (local.get $p) (local.get $start)))
    (i32.store (global.get $OFF_SCRATCH2) (i32.sub (local.get $p) (i32.load (global.get $OFF_WAT_PTR))))
    (if (local.get $dot)
      (then
        (i32.store (global.get $OFF_SCRATCH3) (i32.sub (local.get $start) (i32.load (global.get $OFF_WAT_PTR))))
        (i32.store (global.get $OFF_SCRATCH4) (i32.sub (local.get $dot) (local.get $start)))
        (i32.store (global.get $OFF_SCRATCH5) (i32.sub (i32.add (local.get $dot) (i32.const 1)) (i32.load (global.get $OFF_WAT_PTR))))
        (i32.store (global.get $OFF_SCRATCH6) (i32.sub (local.get $p) (i32.add (local.get $dot) (i32.const 1))))
      )
      (else
        (i32.store (global.get $OFF_SCRATCH3) (i32.const 0))
        (i32.store (global.get $OFF_SCRATCH4) (i32.const 0))
        (i32.store (global.get $OFF_SCRATCH5) (i32.const 0))
        (i32.store (global.get $OFF_SCRATCH6) (i32.const 0))
      )
    )
    (i32.const 0)
  )

  ;; ── Parse a dotted port reference: "node.port" ──
  ;; Finds or creates the node, returns (node_offset, port_offset) in scratch3/4.
  ;; Returns 0 on success, non-zero on error.
  (func $er_parse_port_ref (param $g i32) (param $pos i32) (result i32)
    (local $node i32) (local $node_name i32)
    (if (call $er_read_dotted (local.get $pos)) (then (return (i32.const 1))))
    ;; Check if we have a dotted identifier
    (if (i32.load (global.get $OFF_SCRATCH3))  ;; node part exists
      (then
        ;; Find or create node by name
        (local.set $node (call $er_find_node (local.get $g)
          (i32.load (global.get $OFF_SCRATCH3))
          (i32.load (global.get $OFF_SCRATCH4))))
        (if (i32.eqz (local.get $node))
          (then
            (local.set $node (call $er_node_alloc (local.get $g) (global.get $ER_NODE_KIND_NODE)
              (i32.load (global.get $OFF_SCRATCH3))
              (i32.load (global.get $OFF_SCRATCH4))))
            (if (i32.eq (local.get $node) (i32.const -1)) (then (return (i32.const 1))))
          )
        )
        ;; Return node offset in scratch3, port name in scratch5/6
        (i32.store (global.get $OFF_SCRATCH3) (local.get $node))
        ;; port name is the part after the dot
        ;; Keep scratch5/6 from read_dotted
      )
      (else
        ;; Not dotted -- it's just a plain identifier. Create an anonymous node.
        (local.set $node (call $er_node_alloc (local.get $g) (global.get $ER_NODE_KIND_NODE)
          (i32.load (global.get $OFF_SCRATCH0))
          (i32.load (global.get $OFF_SCRATCH1))))
        (if (i32.eq (local.get $node) (i32.const -1)) (then (return (i32.const 1))))
        (i32.store (global.get $OFF_SCRATCH3) (local.get $node))
        (i32.store (global.get $OFF_SCRATCH5) (i32.const 0))
        (i32.store (global.get $OFF_SCRATCH6) (i32.const 0))
      )
    )
    (i32.store (global.get $OFF_SCRATCH7) (i32.load (global.get $OFF_SCRATCH2)))  ;; save new pos
    (i32.const 0)
  )

  ;; ── Main parse entry point ──
  (func $er_parse (export "er_parse") (param $src i32) (param $len i32) (result i32)
    (local $g i32) (local $pos i32) (local $b i32)
    (i32.store (global.get $OFF_WAT_PTR) (local.get $src))
    (i32.store (global.get $OFF_WAT_LEN) (local.get $len))
    (local.set $g (call $er_graph_create))
    (if (i32.eq (local.get $g) (i32.const -1)) (then (return (i32.const -1))))
    (local.set $pos (i32.const 0))
    (block $parse_done
      (loop $parse_lp
        (drop (call $er_skip_ws (local.get $pos)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
        (if (i32.ge_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))
                      (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.load (global.get $OFF_WAT_LEN))))
          (then (br $parse_done))
        )
        (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
        ;; flow Name { ... }
        (if (call $wat_match_kw (local.get $pos) (i32.const 0x8F000) (i32.const 4))
          (then
            (local.set $pos (call $er_parse_flow (local.get $g) (local.get $pos)))
            (if (i32.eq (local.get $pos) (i32.const -1)) (then (return (i32.const -1))))
            (br $parse_lp)
          )
        )
        ;; Unknown top-level token — skip it
        (if (call $wat_read_kw (local.get $pos))
          (then (br $parse_done))
        )
        (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
        (br $parse_lp)
      )
    )
    local.get $g
  )

  ;; ── Parse flow declaration ──
  ;; flow Name { decls... }
  (func $er_parse_flow (param $g i32) (param $pos i32) (result i32)
    (local $kw_off i32) (local $kw_len i32) (local $b i32)
    (local.set $pos (i32.add (local.get $pos) (i32.const 4)))  ;; skip "flow"
    (drop (call $er_skip_ws (local.get $pos)))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
    ;; Read flow name
    (if (call $wat_read_kw (local.get $pos)) (then (return (i32.const -1))))
    (local.set $kw_off (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $kw_len (i32.load (global.get $OFF_SCRATCH1)))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
    ;; Allocate flow node
    (drop (call $er_node_alloc (local.get $g) (global.get $ER_NODE_KIND_FLOW)
      (local.get $kw_off) (local.get $kw_len)))
    (drop (call $er_skip_ws (local.get $pos)))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
    ;; Expect '{'
    (if (i32.ne (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))) (i32.const 0x7B))
      (then (return (i32.const -1)))
    )
    (local.set $pos (i32.add (local.get $pos) (i32.const 1)))  ;; skip {
    ;; Parse flow body
    (block $flow_done
      (loop $flow_lp
        (drop (call $er_skip_ws (local.get $pos)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
        ;; Check for EOF
        (if (i32.ge_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))
                      (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.load (global.get $OFF_WAT_LEN))))
          (then (br $flow_done))
        )
        ;; Check for }
        (if (i32.eq (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))) (i32.const 0x7D))
          (then (local.set $pos (i32.add (local.get $pos) (i32.const 1))) (br $flow_done))
        )
        ;; clocks name=type, ...
        (if (call $wat_match_kw (local.get $pos) (i32.const 0x8F080) (i32.const 6))
          (then (local.set $pos (call $er_parse_clocks (local.get $g) (local.get $pos))) (br $flow_lp))
        )
        ;; clock name = type (individual)
        (if (call $wat_match_kw (local.get $pos) (i32.const 0x8F040) (i32.const 5))
          (then (local.set $pos (call $er_parse_clock_single (local.get $g) (local.get $pos))) (br $flow_lp))
        )
        ;; lanes by placement|node
        (if (call $wat_match_kw (local.get $pos) (i32.const 0x8F090) (i32.const 5))
          (then (local.set $pos (call $er_parse_layout (local.get $g) (local.get $pos))) (br $flow_lp))
        )
        ;; object name: Type @placement { ... }
        (if (call $wat_match_kw (local.get $pos) (i32.const 0x8F0A0) (i32.const 6))
          (then (local.set $pos (call $er_parse_object (local.get $g) (i32.const 5) (local.get $pos))) (br $flow_lp))
        )
        ;; buffer name: Type @placement { ... }
        (if (call $wat_match_kw (local.get $pos) (i32.const 0x8F0B0) (i32.const 6))
          (then (local.set $pos (call $er_parse_object (local.get $g) (i32.const 5) (local.get $pos))) (br $flow_lp))
        )
        ;; mux/demux/join/gate/kernel/batch/reduce/map/arbiter/feedback/cdc blocks
        (if (call $wat_match_kw (local.get $pos) (i32.const 0x8F0C0) (i32.const 3))  ;; "mux"
          (then (local.set $pos (call $er_parse_block_node (local.get $g) (global.get $ER_NODE_KIND_MUX) (local.get $pos))) (br $flow_lp))
        )
        (if (call $wat_match_kw (local.get $pos) (i32.const 0x8F0D0) (i32.const 5))  ;; "demux"
          (then (local.set $pos (call $er_parse_block_node (local.get $g) (global.get $ER_NODE_KIND_DEMUX) (local.get $pos))) (br $flow_lp))
        )
        (if (call $wat_match_kw (local.get $pos) (i32.const 0x8F0E0) (i32.const 4))  ;; "join"
          (then (local.set $pos (call $er_parse_block_node (local.get $g) (global.get $ER_NODE_KIND_JOIN) (local.get $pos))) (br $flow_lp))
        )
        (if (call $wat_match_kw (local.get $pos) (i32.const 0x8F0F0) (i32.const 4))  ;; "gate"
          (then (local.set $pos (call $er_parse_block_node (local.get $g) (global.get $ER_NODE_KIND_GATE) (local.get $pos))) (br $flow_lp))
        )
        (if (call $wat_match_kw (local.get $pos) (i32.const 0x8F100) (i32.const 6))  ;; "kernel"
          (then (local.set $pos (call $er_parse_kernel (local.get $g) (local.get $pos))) (br $flow_lp))
        )
        (if (call $wat_match_kw (local.get $pos) (i32.const 0x8F110) (i32.const 5))  ;; "batch"
          (then (local.set $pos (call $er_parse_block_node (local.get $g) (global.get $ER_NODE_KIND_BATCH) (local.get $pos))) (br $flow_lp))
        )
        (if (call $wat_match_kw (local.get $pos) (i32.const 0x8F120) (i32.const 6))  ;; "reduce"
          (then (local.set $pos (call $er_parse_block_node (local.get $g) (global.get $ER_NODE_KIND_REDUCE) (local.get $pos))) (br $flow_lp))
        )
        (if (call $wat_match_kw (local.get $pos) (i32.const 0x8F130) (i32.const 3))  ;; "map"
          (then (local.set $pos (call $er_parse_block_node (local.get $g) (global.get $ER_NODE_KIND_MAP) (local.get $pos))) (br $flow_lp))
        )
        (if (call $wat_match_kw (local.get $pos) (i32.const 0x8F140) (i32.const 7))  ;; "arbiter"
          (then (local.set $pos (call $er_parse_block_node (local.get $g) (global.get $ER_NODE_KIND_ARBITER) (local.get $pos))) (br $flow_lp))
        )
        (if (call $wat_match_kw (local.get $pos) (i32.const 0x8F150) (i32.const 8))  ;; "feedback"
          (then (local.set $pos (call $er_parse_block_node (local.get $g) (global.get $ER_NODE_KIND_FEEDBACK) (local.get $pos))) (br $flow_lp))
        )
        (if (call $wat_match_kw (local.get $pos) (i32.const 0x8F1A0) (i32.const 3))  ;; "cdc"
          (then (local.set $pos (call $er_parse_cdc (local.get $g) (local.get $pos))) (br $flow_lp))
        )
        ;; wire decl: wire node.port -> node.port -> ...
        (if (call $wat_match_kw (local.get $pos) (i32.const 0x8F030) (i32.const 4))
          (then
            (local.set $pos (i32.add (local.get $pos) (i32.const 4)))  ;; skip "wire"
            (local.set $pos (call $er_parse_arrow_chain (local.get $g) (local.get $pos)))
            (if (i32.eq (local.get $pos) (i32.const -1)) (then (return (i32.const -1))))
            (br $flow_lp)
          )
        )
        ;; node Name @placement { ... }
        (if (call $wat_match_kw (local.get $pos) (i32.const 0x8F010) (i32.const 4))
          (then (local.set $pos (call $er_parse_node (local.get $g) (i32.const 0) (local.get $pos))) (br $flow_lp))
        )
        ;; actor Name @placement { ... }
        (if (call $wat_match_kw (local.get $pos) (i32.const 0x8F020) (i32.const 5))
          (then (local.set $pos (call $er_parse_node (local.get $g) (i32.const 1) (local.get $pos))) (br $flow_lp))
        )
        ;; split block
        (if (call $wat_match_kw (local.get $pos) (i32.const 0x8F210) (i32.const 5))
          (then (local.set $pos (call $er_parse_block_node (local.get $g) (global.get $ER_NODE_KIND_SPLIT) (local.get $pos))) (br $flow_lp))
        )
        ;; Try to parse an arrow chain (node.port -> ... -> node.port)
        ;; Check if current position starts with an identifier followed by '.'
        (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
        (if (i32.or (i32.and (i32.ge_u (local.get $b) (i32.const 0x61)) (i32.le_u (local.get $b) (i32.const 0x7A)))
                    (i32.and (i32.ge_u (local.get $b) (i32.const 0x41)) (i32.le_u (local.get $b) (i32.const 0x5A))))
          (then
            ;; Could be start of an arrow chain — try it
            (local.set $pos (call $er_parse_arrow_chain (local.get $g) (local.get $pos)))
            (if (i32.eq (local.get $pos) (i32.const -1))
              (then
                ;; If arrow chain parsing fails, this is just a skipped token
                (if (call $wat_read_kw (local.get $pos)) (then (br $flow_done)))
                (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
              )
            )
            (br $flow_lp)
          )
        )
        ;; Fallback: skip unknown token
        (if (call $wat_read_kw (local.get $pos)) (then (br $flow_done)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
        (br $flow_lp)
      )
    )
    local.get $pos
  )

  ;; ── Parse clocks declaration ──
  ;; clocks name=type, name=type, ...
  (func $er_skip_spaces (param $pos i32) (result i32)
    (local $p i32) (local $end i32) (local $b i32)
    (local.set $p (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos)))
    (local.set $end (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.load (global.get $OFF_WAT_LEN))))
    (block $done
      (loop $lp
        (if (i32.ge_u (local.get $p) (local.get $end)) (then (br $done)))
        (local.set $b (i32.load8_u (local.get $p)))
        (if (i32.eq (local.get $b) (i32.const 0x20)) (then (local.set $p (i32.add (local.get $p) (i32.const 1))) (br $lp)))
        (if (i32.eq (local.get $b) (i32.const 0x09)) (then (local.set $p (i32.add (local.get $p) (i32.const 1))) (br $lp)))
        (br $done)
      )
    )
    (i32.store (global.get $OFF_SCRATCH0) (i32.sub (local.get $p) (i32.load (global.get $OFF_WAT_PTR))))
    (i32.const 0)
  )

  ;; ── Parse clocks declaration ──
  ;; clocks name=type, name=type, ...
  (func $er_parse_clocks (param $g i32) (param $pos i32) (result i32)
    (local $kw_off i32) (local $kw_len i32) (local $b i32) (local $p i32)
    (local.set $pos (i32.add (local.get $pos) (i32.const 6)))  ;; skip "clocks"
    (block $done
      (loop $lp
        ;; Skip spaces/tabs only — stop at newline so we stay on one line
        (drop (call $er_skip_spaces (local.get $pos)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
        (if (i32.ge_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))
                      (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.load (global.get $OFF_WAT_LEN))))
          (then (br $done))
        )
        (local.set $p (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos)))
        (local.set $b (i32.load8_u (local.get $p)))
        ;; End on newline, }, or comment
        (if (i32.eq (local.get $b) (i32.const 0x7D)) (then (br $done)))  ;; }
        (if (i32.eq (local.get $b) (i32.const 0x0A)) (then (br $done)))  ;; \n
        (if (i32.eq (local.get $b) (i32.const 0x0D)) (then (br $done)))  ;; \r
        (if (i32.and (i32.eq (local.get $b) (i32.const 0x2F))
                     (i32.lt_u (i32.add (local.get $p) (i32.const 1))
                               (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.load (global.get $OFF_WAT_LEN)))))
          (then
            (if (i32.eq (i32.load8_u (i32.add (local.get $p) (i32.const 1))) (i32.const 0x2F))
              (then (br $done))  ;; // comment — end of clocks
            )
          )
        )
        ;; Read clock name
        (if (call $wat_read_kw (local.get $pos)) (then (br $done)))
        (local.set $kw_off (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $kw_len (i32.load (global.get $OFF_SCRATCH1)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
        ;; Allocate clock node
        (drop (call $er_node_alloc (local.get $g) (global.get $ER_NODE_KIND_CLOCK)
          (local.get $kw_off) (local.get $kw_len)))
        ;; Skip = type
        (drop (call $er_skip_spaces (local.get $pos)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
        (if (i32.eq (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))) (i32.const 0x3D))  ;; =
          (then
            (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
            (if (call $wat_read_kw (local.get $pos)) (then (br $done)))
            (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
          )
        )
        ;; Skip optional comma
        (drop (call $er_skip_spaces (local.get $pos)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
        (if (i32.eq (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))) (i32.const 0x2C))
          (then (local.set $pos (i32.add (local.get $pos) (i32.const 1))))
        )
        (br $lp)
      )
    )
    local.get $pos
  )

  ;; ── Parse layout hint ──
  ;; lanes by placement|node
  (func $er_parse_layout (param $g i32) (param $pos i32) (result i32)
    (local.set $pos (i32.add (local.get $pos) (i32.const 5)))  ;; skip "lanes"
    (drop (call $er_skip_ws (local.get $pos)))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
    (if (call $wat_match_kw (local.get $pos) (i32.const 0x8F170) (i32.const 2))  ;; "by"
      (then
        (local.set $pos (i32.add (local.get $pos) (i32.const 2)))
        (if (call $wat_read_kw (local.get $pos)) (then (return local.get $pos)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
      )
    )
    local.get $pos
  )

  ;; ── Parse individual clock declaration ──
  ;; clock name = type
  (func $er_parse_clock_single (export "er_parse_clock_single") (param $g i32) (param $pos i32) (result i32)
    (local $name_off i32) (local $name_len i32)
    (local.set $pos (i32.add (local.get $pos) (i32.const 5)))  ;; skip "clock"
    (drop (call $er_skip_ws (local.get $pos)))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
    ;; Read clock name
    (if (call $wat_read_kw (local.get $pos)) (then (return local.get $pos)))
    (local.set $name_off (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $name_len (i32.load (global.get $OFF_SCRATCH1)))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
    ;; Allocate clock node
    (drop (call $er_node_alloc (local.get $g) (global.get $ER_NODE_KIND_CLOCK)
      (local.get $name_off) (local.get $name_len)))
    ;; Skip = type
    (drop (call $er_skip_ws (local.get $pos)))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
    (if (i32.eq (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))) (i32.const 0x3D))  ;; =
      (then
        (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
        (if (call $wat_read_kw (local.get $pos)) (then (return local.get $pos)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
      )
    )
    local.get $pos
  )

  ;; ── Parse object/buffer declaration ──
  ;; object name: Type @placement { ... }
  ;; buffer name: Type @placement { ... }
  (func $er_parse_object (param $g i32) (param $kind i32) (param $pos i32) (result i32)
    (local $node i32) (local $kw_off i32) (local $kw_len i32) (local $b i32)
    (local $anno_off i32) (local $anno_len i32)
    (if (i32.eq (local.get $kind) (i32.const 5))  ;; buffer
      (then (local.set $pos (i32.add (local.get $pos) (i32.const 6))))  ;; skip "buffer"
      (else (local.set $pos (i32.add (local.get $pos) (i32.const 6))))  ;; skip "object"
    )
    (drop (call $er_skip_ws (local.get $pos)))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
    ;; Read object name
    (if (call $wat_read_kw (local.get $pos)) (then (return local.get $pos)))
    (local.set $kw_off (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $kw_len (i32.load (global.get $OFF_SCRATCH1)))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
    ;; Allocate node
    (local.set $node (call $er_node_alloc (local.get $g) (global.get $ER_NODE_KIND_BUFFER)
      (local.get $kw_off) (local.get $kw_len)))
    (if (i32.eq (local.get $node) (i32.const -1)) (then (return (i32.const -1))))
    ;; Skip : Type annotation
    (drop (call $er_skip_ws (local.get $pos)))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
    (if (i32.eq (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))) (i32.const 0x3A))  ;; :
      (then
        (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
        (if (call $wat_read_kw (local.get $pos)) (then (return local.get $pos)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
      )
    )
    ;; Parse and store @placement annotation
    (drop (call $er_skip_ws (local.get $pos)))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
    (if (i32.eq (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))) (i32.const 0x40))  ;; @
      (then
        (if (call $er_read_annotation (local.get $pos)) (then (return local.get $pos)))
        (local.set $anno_off (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $anno_len (i32.load (global.get $OFF_SCRATCH1)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
        (call $er_node_set_placement (local.get $node)
          (call $er_str_store (local.get $g) (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $anno_off)) (local.get $anno_len)))
      )
    )
    ;; Skip optional { ... } block
    (drop (call $er_skip_ws (local.get $pos)))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
    (if (i32.eq (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))) (i32.const 0x7B))
      (then (local.set $pos (call $skip_braces (local.get $pos))))
    )
    local.get $pos
  )

  ;; ── Parse a block-style node: mux/demux/join/gate/batch/reduce/map/arbiter/feedback ──
  ;; keyword Name { ... }
  (func $er_parse_block_node (param $g i32) (param $kind i32) (param $pos i32) (result i32)
    (local $kw_len i32) (local $name_off i32) (local $name_len i32)
    (local $node i32) (local $anno_off i32) (local $anno_len i32)
    ;; Determine keyword length based on kind
    (block $done
      (if (i32.eq (local.get $kind) (global.get $ER_NODE_KIND_MUX))
        (then (local.set $kw_len (i32.const 3)) (br $done))
      )
      (if (i32.eq (local.get $kind) (global.get $ER_NODE_KIND_MAP))
        (then (local.set $kw_len (i32.const 3)) (br $done))
      )
      (if (i32.eq (local.get $kind) (global.get $ER_NODE_KIND_DEMUX))
        (then (local.set $kw_len (i32.const 5)) (br $done))
      )
      (if (i32.eq (local.get $kind) (global.get $ER_NODE_KIND_BATCH))
        (then (local.set $kw_len (i32.const 5)) (br $done))
      )
      (if (i32.eq (local.get $kind) (global.get $ER_NODE_KIND_REDUCE))
        (then (local.set $kw_len (i32.const 6)) (br $done))
      )
      (if (i32.eq (local.get $kind) (global.get $ER_NODE_KIND_ARBITER))
        (then (local.set $kw_len (i32.const 7)) (br $done))
      )
      (if (i32.eq (local.get $kind) (global.get $ER_NODE_KIND_FEEDBACK))
        (then (local.set $kw_len (i32.const 8)) (br $done))
      )
      (local.set $kw_len (i32.const 4))  ;; join, gate
    )
    (local.set $pos (i32.add (local.get $pos) (local.get $kw_len)))  ;; skip keyword
    (drop (call $er_skip_ws (local.get $pos)))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
    ;; Read name
    (if (call $wat_read_kw (local.get $pos)) (then (return local.get $pos)))
    (local.set $name_off (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $name_len (i32.load (global.get $OFF_SCRATCH1)))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
    ;; Allocate node
    (local.set $node (call $er_node_alloc (local.get $g) (local.get $kind)
      (local.get $name_off) (local.get $name_len)))
    (if (i32.eq (local.get $node) (i32.const -1)) (then (return (i32.const -1))))
    ;; Skip optional : Type<...> annotation
    (drop (call $er_skip_ws (local.get $pos)))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
    (if (i32.eq (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))) (i32.const 0x3A))  ;; :
      (then
        (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
        (if (call $wat_read_kw (local.get $pos)) (then (return local.get $pos)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
      )
    )
    ;; Parse optional @placement annotation
    (drop (call $er_skip_ws (local.get $pos)))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
    (if (i32.eq (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))) (i32.const 0x40))
      (then
        (if (call $er_read_annotation (local.get $pos)) (then (return local.get $pos)))
        (local.set $anno_off (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $anno_len (i32.load (global.get $OFF_SCRATCH1)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
        (call $er_node_set_placement (local.get $node)
          (call $er_str_store (local.get $g) (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $anno_off)) (local.get $anno_len)))
      )
    )
    ;; Parse kind-specific { ... } body
    (drop (call $er_skip_ws (local.get $pos)))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
    (if (i32.eq (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))) (i32.const 0x7B))
      (then
        (block $body_dispatch
          ;; batch { max_items, max_delay, ... }
          (if (i32.eq (local.get $kind) (global.get $ER_NODE_KIND_BATCH))
            (then (local.set $pos (call $er_parse_batch_body (local.get $g) (local.get $node) (local.get $pos))) (br $body_dispatch))
          )
          ;; arbiter { resource, policy, ... }
          (if (i32.eq (local.get $kind) (global.get $ER_NODE_KIND_ARBITER))
            (then (local.set $pos (call $er_parse_arbiter_body (local.get $g) (local.get $node) (local.get $pos))) (br $body_dispatch))
          )
          ;; gate { pass if ... }
          (if (i32.eq (local.get $kind) (global.get $ER_NODE_KIND_GATE))
            (then (local.set $pos (call $er_parse_gate_body (local.get $g) (local.get $node) (local.get $pos))) (br $body_dispatch))
          )
          ;; Fallback: skip unknown body
          (local.set $pos (call $skip_braces (local.get $pos)))
        )
      )
    )
    local.get $pos
  )

  ;; ── Parse a keyword-terminated body ──
  ;; Reads key-value pairs inside { ... } body.
  ;; Matches each keyword from a list, reads its value token.
  ;; For each match, calls a callback function.
  ;; Returns new pos, or -1 on error.
  (func $er_parse_body_kvs (param $pos i32) (result i32)
    (local $b i32)
    ;; Expect '{'
    (if (i32.ne (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))) (i32.const 0x7B))
      (then (return local.get $pos))  ;; no body
    )
    (local.set $pos (i32.add (local.get $pos) (i32.const 1)))  ;; skip {
    (block $done
      (loop $lp
        (drop (call $er_skip_ws (local.get $pos)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
        (if (i32.ge_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))
                      (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.load (global.get $OFF_WAT_LEN))))
          (then (br $done))
        )
        (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
        ;; '}' or ',' or ';'
        (block $check_end
          (if (i32.eq (local.get $b) (i32.const 0x7D))
            (then (local.set $pos (i32.add (local.get $pos) (i32.const 1))) (br $done))
          )
          (if (i32.eq (local.get $b) (i32.const 0x2C))
            (then (local.set $pos (i32.add (local.get $pos) (i32.const 1))) (br $check_end))
          )
          (if (i32.eq (local.get $b) (i32.const 0x3B))
            (then (local.set $pos (i32.add (local.get $pos) (i32.const 1))) (br $check_end))
          )
        )
        ;; Read the next key or value token
        (if (call $wat_read_kw (local.get $pos)) (then (br $done)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
        (br $lp)
      )
    )
    local.get $pos
  )

  ;; Read an i32 value after a keyword.
  ;; Expects a uint literal. Stores in scratch1.
  (func $er_read_kv_int (param $pos i32) (result i32)
    (drop (call $er_skip_ws (local.get $pos)))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
    (if (call $wat_read_uint (local.get $pos)) (then (return (i32.const -1))))
    local.get $pos
  )

  ;; Read a string value after a keyword.
  ;; Expects an identifier. Stores in scratch0/scratch1.
  (func $er_read_kv_str (param $pos i32) (result i32)
    (drop (call $er_skip_ws (local.get $pos)))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
    (if (call $wat_read_kw (local.get $pos)) (then (return (i32.const -1))))
    local.get $pos
  )

  ;; ── Batch body parser ──
  ;; { max_items N, max_delay D, max_bytes B, group_by S }
  ;; BatchConfig: +0 max_items, +4 max_delay, +8 max_bytes, +12 group_by_off, +16 group_by_len
  (func $er_parse_batch_body (param $g i32) (param $node i32) (param $pos i32) (result i32)
    (local $cfg i32) (local $max_items i32) (local $max_delay i32) (local $max_bytes i32)
    (local $group_off i32) (local $group_len i32)
    (local $pos2 i32) (local $kw_off i32) (local $kw_len i32)
    (if (i32.ne (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))) (i32.const 0x7B))
      (then (return local.get $pos))
    )
    (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
    (block $done
      (loop $lp
        (drop (call $er_skip_ws (local.get $pos)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
        (if (i32.ge_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))
                      (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.load (global.get $OFF_WAT_LEN))))
          (then (br $done))
        )
        (local.set $kw_off (i32.load (global.get $OFF_WAT_PTR)))
        (local.set $kw_len (i32.load (global.get $OFF_WAT_LEN)))
        ;; Check end
        (if (i32.eq (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))) (i32.const 0x7D))
          (then (local.set $pos (i32.add (local.get $pos) (i32.const 1))) (br $done))
        )
        ;; Skip , or ;
        (block $skip_sep
          (if (i32.eq (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))) (i32.const 0x2C))
            (then (local.set $pos (i32.add (local.get $pos) (i32.const 1))) (drop (call $er_skip_ws (local.get $pos))) (local.set $pos (i32.load (global.get $OFF_SCRATCH0))) )
          )
          (if (i32.eq (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))) (i32.const 0x3B))
            (then (local.set $pos (i32.add (local.get $pos) (i32.const 1))) (drop (call $er_skip_ws (local.get $pos))) (local.set $pos (i32.load (global.get $OFF_SCRATCH0))) )
          )
        )
        ;; max_items
        (if (call $wat_match_kw (local.get $pos) (i32.const 0x8F220) (i32.const 9))
          (then
            (local.set $pos (call $er_read_kv_int (i32.add (local.get $pos) (i32.const 9))))
            (local.set $max_items (i32.load (global.get $OFF_SCRATCH1)))
            (br $lp)
          )
        )
        ;; max_delay
        (if (call $wat_match_kw (local.get $pos) (i32.const 0x8F230) (i32.const 9))
          (then
            (local.set $pos (call $er_read_kv_int (i32.add (local.get $pos) (i32.const 9))))
            (local.set $max_delay (i32.load (global.get $OFF_SCRATCH1)))
            (br $lp)
          )
        )
        ;; max_bytes
        (if (call $wat_match_kw (local.get $pos) (i32.const 0x8F240) (i32.const 9))
          (then
            (local.set $pos (call $er_read_kv_int (i32.add (local.get $pos) (i32.const 9))))
            (local.set $max_bytes (i32.load (global.get $OFF_SCRATCH1)))
            (br $lp)
          )
        )
        ;; group_by
        (if (call $wat_match_kw (local.get $pos) (i32.const 0x8F250) (i32.const 8))
          (then
            (local.set $pos (call $er_read_kv_str (i32.add (local.get $pos) (i32.const 8))))
            (local.set $group_off (i32.load (global.get $OFF_SCRATCH0)))
            (local.set $group_len (i32.load (global.get $OFF_SCRATCH1)))
            (br $lp)
          )
        )
        ;; Skip unknown token
        (if (call $wat_read_kw (local.get $pos)) (then (br $done)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
        (br $lp)
      )
    )
    ;; Allocate BatchConfig (20 bytes)
    (local.set $cfg (call $pipe_alloc (i32.const 20)))
    (if (i32.eq (local.get $cfg) (i32.const -1)) (then (return (i32.const -1))))
    (i32.store offset=0 (local.get $cfg) (local.get $max_items))
    (i32.store offset=4 (local.get $cfg) (local.get $max_delay))
    (i32.store offset=8 (local.get $cfg) (local.get $max_bytes))
    (i32.store offset=12 (local.get $cfg) (local.get $group_off))
    (i32.store offset=16 (local.get $cfg) (local.get $group_len))
    (call $er_node_set_config (local.get $node) (local.get $cfg))
    local.get $pos
  )

  ;; ── Arbiter body parser ──
  ;; { resource S, policy S, ... }
  ;; ArbiterConfig: +0 resource_off, +4 resource_len, +8 policy_off, +12 policy_len
  (func $er_parse_arbiter_body (param $g i32) (param $node i32) (param $pos i32) (result i32)
    (local $cfg i32) (local $res_off i32) (local $res_len i32)
    (local $pol_off i32) (local $pol_len i32)
    (if (i32.ne (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))) (i32.const 0x7B))
      (then (return local.get $pos))
    )
    (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
    (block $done
      (loop $lp
        (drop (call $er_skip_ws (local.get $pos)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
        (if (i32.ge_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))
                      (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.load (global.get $OFF_WAT_LEN))))
          (then (br $done))
        )
        (if (i32.eq (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))) (i32.const 0x7D))
          (then (local.set $pos (i32.add (local.get $pos) (i32.const 1))) (br $done))
        )
        (block $skip_sep
          (if (i32.eq (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))) (i32.const 0x2C))
            (then (local.set $pos (i32.add (local.get $pos) (i32.const 1))) (drop (call $er_skip_ws (local.get $pos))) (local.set $pos (i32.load (global.get $OFF_SCRATCH0))) )
          )
          (if (i32.eq (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))) (i32.const 0x3B))
            (then (local.set $pos (i32.add (local.get $pos) (i32.const 1))) (drop (call $er_skip_ws (local.get $pos))) (local.set $pos (i32.load (global.get $OFF_SCRATCH0))) )
          )
        )
        ;; resource
        (if (call $wat_match_kw (local.get $pos) (i32.const 0x8F280) (i32.const 8))
          (then
            (local.set $pos (call $er_read_kv_str (i32.add (local.get $pos) (i32.const 8))))
            (local.set $res_off (i32.load (global.get $OFF_SCRATCH0)))
            (local.set $res_len (i32.load (global.get $OFF_SCRATCH1)))
            (br $lp)
          )
        )
        ;; policy
        (if (call $wat_match_kw (local.get $pos) (i32.const 0x8F2C0) (i32.const 6))
          (then
            (local.set $pos (call $er_read_kv_str (i32.add (local.get $pos) (i32.const 6))))
            (local.set $pol_off (i32.load (global.get $OFF_SCRATCH0)))
            (local.set $pol_len (i32.load (global.get $OFF_SCRATCH1)))
            (br $lp)
          )
        )
        (if (call $wat_read_kw (local.get $pos)) (then (br $done)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
        (br $lp)
      )
    )
    (local.set $cfg (call $pipe_alloc (i32.const 16)))
    (if (i32.eq (local.get $cfg) (i32.const -1)) (then (return (i32.const -1))))
    (i32.store offset=0 (local.get $cfg) (local.get $res_off))
    (i32.store offset=4 (local.get $cfg) (local.get $res_len))
    (i32.store offset=8 (local.get $cfg) (local.get $pol_off))
    (i32.store offset=12 (local.get $cfg) (local.get $pol_len))
    (call $er_node_set_config (local.get $node) (local.get $cfg))
    local.get $pos
  )

  ;; ── Gate body parser ──
  ;; { pass if condition }
  ;; GateConfig: +0 cond_off, +4 cond_len
  (func $er_parse_gate_body (param $g i32) (param $node i32) (param $pos i32) (result i32)
    (local $cfg i32) (local $cond_off i32) (local $cond_len i32)
    (if (i32.ne (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))) (i32.const 0x7B))
      (then (return local.get $pos))
    )
    (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
    ;; pass if EXPR
    (drop (call $er_skip_ws (local.get $pos)))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
    (if (call $wat_match_kw (local.get $pos) (i32.const 0x8F2A0) (i32.const 4))
      (then
        (local.set $pos (i32.add (local.get $pos) (i32.const 4)))  ;; skip "pass"
        ;; skip "if"
        (drop (call $er_skip_ws (local.get $pos)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $pos (i32.add (local.get $pos) (i32.const 2)))  ;; skip "if"
        ;; Read rest of condition until }
        (drop (call $er_skip_ws (local.get $pos)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $cond_off (local.get $pos))  ;; start of condition
        ;; Find }
        (block $find_close
          (loop $scan
            (if (i32.eq (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))) (i32.const 0x7D))
              (then (br $find_close))
            )
            (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
            (br $scan)
          )
        )
        (local.set $cond_len (i32.sub (local.get $pos) (local.get $cond_off)))
      )
    )
    ;; Skip }
    (drop (call $er_skip_ws (local.get $pos)))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
    (if (i32.eq (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))) (i32.const 0x7D))
      (then (local.set $pos (i32.add (local.get $pos) (i32.const 1))))
    )
    (local.set $cfg (call $pipe_alloc (i32.const 8)))
    (if (i32.eq (local.get $cfg) (i32.const -1)) (then (return (i32.const -1))))
    (i32.store offset=0 (local.get $cfg) (local.get $cond_off))
    (i32.store offset=4 (local.get $cfg) (local.get $cond_len))
    (call $er_node_set_config (local.get $node) (local.get $cfg))
    local.get $pos
  )

  ;; ── Port declaration parser ──
  ;; [in|out|inout] name: Type @clock(clk) @rate(max N) @size(~8mb) @private
  ;; Allocates port attached to node. Returns new pos.
  (func $er_parse_port_decl (param $g i32) (param $node i32) (param $dir i32) (param $pos i32) (result i32)
    (local $name_off i32) (local $name_len i32) (local $type_off i32) (local $type_len i32)
    (local $anno_off i32) (local $anno_len i32) (local $port i32) (local $b i32)
    ;; Read port name
    (if (call $wat_read_kw (local.get $pos)) (then (return (i32.const -1))))
    (local.set $name_off (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $name_len (i32.load (global.get $OFF_SCRATCH1)))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
    ;; Check for :Type
    (drop (call $er_skip_ws (local.get $pos)))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
    (if (i32.eq (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))) (i32.const 0x3A))
      (then
        (local.set $pos (i32.add (local.get $pos) (i32.const 1)))  ;; skip :
        (if (call $wat_read_kw (local.get $pos)) (then (return (i32.const -1))))
        (local.set $type_off (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $type_len (i32.load (global.get $OFF_SCRATCH1)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
        ;; Skip <...> generic args
        (drop (call $er_skip_ws (local.get $pos)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
        (if (i32.eq (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))) (i32.const 0x3C))
          (then
            (local.set $pos (call $skip_balanced (local.get $pos) (i32.const 0x3C) (i32.const 0x3E)))
            (if (i32.eq (local.get $pos) (i32.const -1)) (then (return (i32.const -1))))
          )
        )
        ;; Skip skip optional (...) ctor args
        (drop (call $er_skip_ws (local.get $pos)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
        (if (i32.eq (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))) (i32.const 0x28))
          (then
            (local.set $pos (call $skip_balanced (local.get $pos) (i32.const 0x28) (i32.const 0x29)))
            (if (i32.eq (local.get $pos) (i32.const -1)) (then (return (i32.const -1))))
          )
        )
      )
    )
    ;; Read @annotations (clock, rate, size, private, etc.)
    (block $anno_lp
      (loop $anno_loop
        (drop (call $er_skip_ws (local.get $pos)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
        (if (i32.ne (local.get $b) (i32.const 0x40)) (then (br $anno_lp)))  ;; not @
        (if (call $er_read_annotation (local.get $pos)) (then (br $anno_lp)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
        (br $anno_loop)
      )
    )
    ;; Allocate port
    (local.set $port (call $er_port_alloc (local.get $node) (local.get $dir)
      (local.get $name_off) (local.get $name_len)
      (local.get $type_off) (local.get $type_len)))
    (if (i32.eq (local.get $port) (i32.const -1)) (then (return (i32.const -1))))
    local.get $pos
  )

  ;; ── Parse a node/actor declaration ──
  ;; node|actor Name @placement { in|out|inout name: Type @clock(clk) ... }
  ;; For actors also: state name: Buffer<Type> @placement { ... }
  (func $er_parse_node (param $g i32) (param $is_actor i32) (param $pos i32) (result i32)
    (local $kw_len i32) (local $name_off i32) (local $name_len i32)
    (local $node i32) (local $anno_off i32) (local $anno_len i32)
    (local $kind i32) (local $b i32) (local $dir i32)
    (local.set $kw_len
      (if (result i32) (local.get $is_actor)
        (then (i32.const 5))
        (else (i32.const 4))
      )
    )
    (local.set $pos (i32.add (local.get $pos) (local.get $kw_len)))  ;; skip keyword
    ;; Read name
    (if (call $wat_read_kw (local.get $pos)) (then (return (i32.const -1))))
    (local.set $name_off (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $name_len (i32.load (global.get $OFF_SCRATCH1)))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
    ;; Determine kind
    (local.set $kind
      (if (result i32) (local.get $is_actor)
        (then (global.get $ER_NODE_KIND_ACTOR))
        (else (global.get $ER_NODE_KIND_NODE))
      )
    )
    ;; Allocate node
    (local.set $node (call $er_node_alloc (local.get $g) (local.get $kind)
      (local.get $name_off) (local.get $name_len)))
    (if (i32.eq (local.get $node) (i32.const -1)) (then (return (i32.const -1))))
    ;; Parse optional @placement annotation
    (drop (call $er_skip_ws (local.get $pos)))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
    (if (i32.eq (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))) (i32.const 0x40))
      (then
        (if (call $er_read_annotation (local.get $pos)) (then (return local.get $pos)))
        (local.set $anno_off (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $anno_len (i32.load (global.get $OFF_SCRATCH1)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
        (call $er_node_set_placement (local.get $node)
          (call $er_str_store (local.get $g) (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $anno_off)) (local.get $anno_len)))
      )
    )
    ;; Parse optional { body } with port declarations
    (drop (call $er_skip_ws (local.get $pos)))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
    (if (i32.ne (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))) (i32.const 0x7B))
      (then (return local.get $pos))  ;; no body
    )
    (local.set $pos (i32.add (local.get $pos) (i32.const 1)))  ;; skip {
    (block $body_done
      (loop $body_lp
        (drop (call $er_skip_ws (local.get $pos)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
        (if (i32.ge_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))
                      (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.load (global.get $OFF_WAT_LEN))))
          (then (br $body_done))
        )
        (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
        (if (i32.eq (local.get $b) (i32.const 0x7D))
          (then (local.set $pos (i32.add (local.get $pos) (i32.const 1))) (br $body_done))
        )
        ;; in port
        (if (call $wat_match_kw (local.get $pos) (i32.const 0x8F060) (i32.const 2))
          (then
            (local.set $pos (i32.add (local.get $pos) (i32.const 2)))
            (local.set $pos (call $er_parse_port_decl (local.get $g) (local.get $node) (i32.const 0) (local.get $pos)))
            (if (i32.eq (local.get $pos) (i32.const -1)) (then (return (i32.const -1))))
            (br $body_lp)
          )
        )
        ;; out port
        (if (call $wat_match_kw (local.get $pos) (i32.const 0x8F070) (i32.const 3))
          (then
            (local.set $pos (i32.add (local.get $pos) (i32.const 3)))
            (local.set $pos (call $er_parse_port_decl (local.get $g) (local.get $node) (i32.const 1) (local.get $pos)))
            (if (i32.eq (local.get $pos) (i32.const -1)) (then (return (i32.const -1))))
            (br $body_lp)
          )
        )
        ;; inout port
        (if (call $wat_match_kw (local.get $pos) (i32.const 0x8F200) (i32.const 5))
          (then
            (local.set $pos (i32.add (local.get $pos) (i32.const 5)))
            (local.set $pos (call $er_parse_port_decl (local.get $g) (local.get $node) (i32.const 2) (local.get $pos)))
            (if (i32.eq (local.get $pos) (i32.const -1)) (then (return (i32.const -1))))
            (br $body_lp)
          )
        )
        ;; state declaration (actor only, but accept for any for now)
        (if (call $wat_match_kw (local.get $pos) (i32.const 0x8F1F4) (i32.const 5))
          (then
            (local.set $pos (i32.add (local.get $pos) (i32.const 5)))
            ;; state name: Type @placement { ... }
            (local.set $pos (call $er_parse_object (local.get $g) (i32.const 5) (local.get $pos)))
            (if (i32.eq (local.get $pos) (i32.const -1)) (then (return (i32.const -1))))
            (br $body_lp)
          )
        )
        ;; Unknown token — skip
        (if (call $wat_read_kw (local.get $pos)) (then (br $body_done)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
        (br $body_lp)
      )
    )
    local.get $pos
  )

  ;; ── Parse kernel pipe body ──
  ;; pipe { stage -> stage -> ... }
  ;; Creates nodes for each stage, edges between consecutive stages.
  (func $er_parse_kernel_pipe (param $g i32) (param $kernel i32) (param $pos i32) (result i32)
    (local $prev_node i32) (local $cur_node i32) (local $b i32)
    (local $id_off i32) (local $id_len i32) (local $arrow_found i32)
    (local.set $pos (i32.add (local.get $pos) (i32.const 4)))  ;; skip "pipe"
    (drop (call $er_skip_ws (local.get $pos)))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
    ;; Expect '{'
    (if (i32.ne (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))) (i32.const 0x7B))
      (then (return (i32.const -1)))
    )
    (local.set $pos (i32.add (local.get $pos) (i32.const 1)))  ;; skip {
    (local.set $arrow_found (i32.const 1))
    ;; Parse pipe body
    (block $pipe_done
      (loop $pipe_lp
        (drop (call $er_skip_ws (local.get $pos)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
        ;; Check for end
        (if (i32.ge_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))
                      (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.load (global.get $OFF_WAT_LEN))))
          (then (br $pipe_done))
        )
        (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
        (if (i32.eq (local.get $b) (i32.const 0x7D))   ;; }
          (then (local.set $pos (i32.add (local.get $pos) (i32.const 1))) (br $pipe_done))
        )
        ;; Skip -> arrow
        (if (call $er_match_arrow (local.get $pos))
          (then
            (local.set $pos (i32.add (local.get $pos) (i32.const 2)))
            (local.set $arrow_found (i32.const 1))
            (br $pipe_lp)
          )
        )
        ;; Read stage identifier
        (if (call $wat_read_kw (local.get $pos)) (then (br $pipe_done)))
        (local.set $id_off (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $id_len (i32.load (global.get $OFF_SCRATCH1)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
        ;; Create node for this stage, set parent to kernel
        (local.set $cur_node (call $er_node_alloc (local.get $g) (global.get $ER_NODE_KIND_NODE)
          (local.get $id_off) (local.get $id_len)))
        (if (i32.eq (local.get $cur_node) (i32.const -1)) (then (return (i32.const -1))))
        (call $er_node_set_parent (local.get $cur_node) (local.get $kernel))
        ;; If previous node exists, create edge
        (if (i32.and (local.get $prev_node) (local.get $arrow_found))
          (then
            (drop (call $er_edge_alloc (local.get $g)
              (local.get $prev_node) (i32.const 0) (i32.const 0)
              (local.get $cur_node) (i32.const 0) (i32.const 0)
              (i32.const 0) (i32.const 0)))
          )
        )
        (local.set $prev_node (local.get $cur_node))
        (local.set $arrow_found (i32.const 0))
        (br $pipe_lp)
      )
    )
    local.get $pos
  )

  ;; ── Parse kernel body ──
  ;; kernel body: { clock* buffer* pipe* }
  (func $er_parse_kernel_body (param $g i32) (param $node i32) (param $pos i32) (result i32)
    (local $b i32)
    ;; Expect '{'
    (drop (call $er_skip_ws (local.get $pos)))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
    (if (i32.ne (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))) (i32.const 0x7B))
      (then (return local.get $pos))  ;; no body, ok
    )
    (local.set $pos (i32.add (local.get $pos) (i32.const 1)))  ;; skip {
    (block $body_done
      (loop $body_lp
        (drop (call $er_skip_ws (local.get $pos)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
        (if (i32.ge_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))
                      (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.load (global.get $OFF_WAT_LEN))))
          (then (br $body_done))
        )
        (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
        (if (i32.eq (local.get $b) (i32.const 0x7D))
          (then (local.set $pos (i32.add (local.get $pos) (i32.const 1))) (br $body_done))
        )
        ;; clock name = type
        (if (call $wat_match_kw (local.get $pos) (i32.const 0x8F040) (i32.const 5))
          (then (local.set $pos (call $er_parse_clock_single (local.get $g) (local.get $pos))) (br $body_lp))
        )
        ;; buffer name: Type @placement { ... }
        (if (call $wat_match_kw (local.get $pos) (i32.const 0x8F0B0) (i32.const 6))
          (then (local.set $pos (call $er_parse_object (local.get $g) (i32.const 5) (local.get $pos))) (br $body_lp))
        )
        ;; pipe { ... }
        (if (call $wat_match_kw (local.get $pos) (i32.const 0x8F160) (i32.const 4))
          (then (local.set $pos (call $er_parse_kernel_pipe (local.get $g) (local.get $node) (local.get $pos))) (br $body_lp))
        )
        ;; Skip unknown token
        (if (call $wat_read_kw (local.get $pos)) (then (br $body_done)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
        (br $body_lp)
      )
    )
    local.get $pos
  )

  ;; ── Parse kernel declaration ──
  ;; kernel Name @placement(target) { clock* buffer* pipe* }
  (func $er_parse_kernel (param $g i32) (param $pos i32) (result i32)
    (local $node i32) (local $name_off i32) (local $name_len i32)
    (local $anno_off i32) (local $anno_len i32)
    (local.set $pos (i32.add (local.get $pos) (i32.const 6)))  ;; skip "kernel"
    (drop (call $er_skip_ws (local.get $pos)))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
    ;; Read kernel name
    (if (call $wat_read_kw (local.get $pos)) (then (return (i32.const -1))))
    (local.set $name_off (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $name_len (i32.load (global.get $OFF_SCRATCH1)))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
    ;; Allocate kernel node
    (local.set $node (call $er_node_alloc (local.get $g) (global.get $ER_NODE_KIND_KERNEL)
      (local.get $name_off) (local.get $name_len)))
    (if (i32.eq (local.get $node) (i32.const -1)) (then (return (i32.const -1))))
    ;; Parse optional @placement annotation
    (drop (call $er_skip_ws (local.get $pos)))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
    (if (i32.eq (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))) (i32.const 0x40))
      (then
        (if (call $er_read_annotation (local.get $pos)) (then (return local.get $pos)))
        (local.set $anno_off (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $anno_len (i32.load (global.get $OFF_SCRATCH1)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
        (call $er_node_set_placement (local.get $node)
          (call $er_str_store (local.get $g) (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $anno_off)) (local.get $anno_len)))
      )
    )
    ;; Parse kernel body
    (local.set $pos (call $er_parse_kernel_body (local.get $g) (local.get $node) (local.get $pos)))
    local.get $pos
  )

  ;; ── Parse CDC declaration ──
  ;; cdc name {
  ;;     from src_clock
  ;;     to dst_clock
  ;;     queue qname { capacity N; policy backpressure }
  ;; }
  (func $er_parse_cdc (param $g i32) (param $pos i32) (result i32)
    (local $node i32) (local $name_off i32) (local $name_len i32) (local $b i32)
    (local $anno_off i32) (local $anno_len i32)
    (local.set $pos (i32.add (local.get $pos) (i32.const 3)))  ;; skip "cdc"
    (drop (call $er_skip_ws (local.get $pos)))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
    ;; Read name
    (if (call $wat_read_kw (local.get $pos)) (then (return (i32.const -1))))
    (local.set $name_off (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $name_len (i32.load (global.get $OFF_SCRATCH1)))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
    ;; Allocate CDC node
    (local.set $node (call $er_node_alloc (local.get $g) (global.get $ER_NODE_KIND_CDC)
      (local.get $name_off) (local.get $name_len)))
    (if (i32.eq (local.get $node) (i32.const -1)) (then (return (i32.const -1))))
    ;; Parse optional @placement annotation
    (drop (call $er_skip_ws (local.get $pos)))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
    (if (i32.eq (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))) (i32.const 0x40))
      (then
        (if (call $er_read_annotation (local.get $pos)) (then (return local.get $pos)))
        (local.set $anno_off (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $anno_len (i32.load (global.get $OFF_SCRATCH1)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
        (call $er_node_set_placement (local.get $node)
          (call $er_str_store (local.get $g) (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $anno_off)) (local.get $anno_len)))
      )
    )
    ;; Expect '{'
    (drop (call $er_skip_ws (local.get $pos)))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
    (if (i32.ne (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))) (i32.const 0x7B))
      (then (return (i32.const -1)))
    )
    (local.set $pos (i32.add (local.get $pos) (i32.const 1)))  ;; skip {
    ;; Parse body
    (block $cdc_done
      (loop $cdc_lp
        (drop (call $er_skip_ws (local.get $pos)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
        (if (i32.ge_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))
                      (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.load (global.get $OFF_WAT_LEN))))
          (then (br $cdc_done))
        )
        (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
        (if (i32.eq (local.get $b) (i32.const 0x7D))
          (then (local.set $pos (i32.add (local.get $pos) (i32.const 1))) (br $cdc_done))
        )
        ;; Skip "from" / "to" / "queue" keywords and their arguments
        (if (call $wat_read_kw (local.get $pos)) (then (br $cdc_done)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
        ;; Skip next identifier (clock/queue name)
        (drop (call $er_skip_ws (local.get $pos)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
        (if (call $wat_read_kw (local.get $pos)) (then (br $cdc_done)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
        ;; Skip optional { ... } block for queue
        (drop (call $er_skip_ws (local.get $pos)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
        (if (i32.eq (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))) (i32.const 0x7B))
          (then (local.set $pos (call $skip_braces (local.get $pos))))
        )
        (br $cdc_lp)
      )
    )
    local.get $pos
  )

  ;; Skip past a balanced pair: '(' ')', or '<' '>', or '{' '}'.
  ;; $pos = position of opening bracket
  ;; $open = open byte, $close = close byte
  ;; Returns position after matching close, or -1 on error.
  (func $skip_balanced (export "skip_balanced") (param $pos i32) (param $open i32) (param $close i32) (result i32)
    (local $depth i32) (local $b i32) (local $end i32)
    (local.set $depth (i32.const 1))
    (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
    (local.set $end (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.load (global.get $OFF_WAT_LEN))))
    (block $done
      (loop $lp
        (if (i32.ge_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos)) (local.get $end))
          (then (br $done))
        )
        (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
        (if (i32.eq (local.get $b) (local.get $open))
          (then (local.set $depth (i32.add (local.get $depth) (i32.const 1))))
        )
        (if (i32.eq (local.get $b) (local.get $close))
          (then
            (local.set $depth (i32.sub (local.get $depth) (i32.const 1)))
            (if (i32.eqz (local.get $depth)) (then (return (i32.add (local.get $pos) (i32.const 1)))))
          )
        )
        (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
        (br $lp)
      )
    )
    (i32.const -1)
  )

  ;; ── Parse a stage in an arrow chain ──
  ;; Stages can be:
  ;;   node.port           — dotted port reference
  ;;   name @placement     — inline node with placement
  ;;   Queue<T>(N, policy) — inline queue (uppercase + < + ())
  ;;   name:Type<T> @place — named node with type
  ;;   name                — simple node reference
  ;;
  ;; Returns: node in scratch3, port_off in scratch5, port_len in scratch6, 
  ;;          annotation in scratch8/9, new pos in scratch7.
  ;; Returns 0 on success, non-zero on error.
  (func $er_parse_chain_stage (export "er_parse_chain_stage") (param $g i32) (param $pos i32) (result i32)
    (local $id_off i32) (local $id_len i32) (local $node i32)
    (local $anno_off i32) (local $anno_len i32) (local $b i32)
    (local $p i32)
    ;; First try dotted port ref (fast path for common case)
    (if (i32.eqz (call $er_parse_port_ref (local.get $g) (local.get $pos)))
      (then (return (i32.const 0)))
    )
    ;; Not a port ref — parse inline stage
    ;; Read identifier
    (if (call $wat_read_kw (local.get $pos)) (then (return (i32.const 1))))
    (local.set $id_off (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $id_len (i32.load (global.get $OFF_SCRATCH1)))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
    ;; Check for generic args <...>
    (drop (call $er_skip_ws (local.get $pos)))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
    (if (i32.eq (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))) (i32.const 0x3C))  ;; <
      (then
        (local.set $pos (call $skip_balanced (local.get $pos) (i32.const 0x3C) (i32.const 0x3E)))
        (if (i32.eq (local.get $pos) (i32.const -1)) (then (return (i32.const 1))))
        (drop (call $er_skip_ws (local.get $pos)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
      )
    )
    ;; Check for ctor args (...)
    (if (i32.eq (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))) (i32.const 0x28))  ;; (
      (then
        (local.set $pos (call $skip_balanced (local.get $pos) (i32.const 0x28) (i32.const 0x29)))
        (if (i32.eq (local.get $pos) (i32.const -1)) (then (return (i32.const 1))))
        (drop (call $er_skip_ws (local.get $pos)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
      )
    )
    ;; Check for :Type annotation
    (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
    (if (i32.eq (local.get $b) (i32.const 0x3A))  ;; :
      (then
        ;; name:Type form — id_off/id_len is the NAME, read TYPE next
        (local.set $pos (i32.add (local.get $pos) (i32.const 1)))  ;; skip :
        (drop (call $er_skip_ws (local.get $pos)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
        ;; Save name before reading type
        (i32.store (global.get $OFF_SCRATCH4) (local.get $id_off))
        (i32.store (global.get $OFF_SCRATCH5) (local.get $id_len))
        ;; Read type identifier
        (if (call $wat_read_kw (local.get $pos)) (then (return (i32.const 1))))
        (local.set $id_off (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $id_len (i32.load (global.get $OFF_SCRATCH1)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
        ;; Skip optional <...> after type
        (drop (call $er_skip_ws (local.get $pos)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
        (if (i32.eq (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))) (i32.const 0x3C))
          (then
            (local.set $pos (call $skip_balanced (local.get $pos) (i32.const 0x3C) (i32.const 0x3E)))
            (if (i32.eq (local.get $pos) (i32.const -1)) (then (return (i32.const 1))))
            (drop (call $er_skip_ws (local.get $pos)))
            (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
          )
        )
        ;; Skip optional (...) ctor args after type
        (if (i32.eq (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))) (i32.const 0x28))
          (then
            (local.set $pos (call $skip_balanced (local.get $pos) (i32.const 0x28) (i32.const 0x29)))
            (if (i32.eq (local.get $pos) (i32.const -1)) (then (return (i32.const 1))))
            (drop (call $er_skip_ws (local.get $pos)))
            (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
          )
        )
      )
      (else
        ;; No :Type — id_off/id_len is the node name
        (i32.store (global.get $OFF_SCRATCH4) (i32.const 0))  ;; no separate name
      )
    )
    ;; Parse optional @placement annotation
    (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
    (if (i32.eq (local.get $b) (i32.const 0x40))  ;; @
      (then
        (if (call $er_read_annotation (local.get $pos)) (then (return (i32.const 1))))
        (local.set $anno_off (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $anno_len (i32.load (global.get $OFF_SCRATCH1)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
      )
    )
    ;; Create node
    (local.set $node (call $er_node_alloc (local.get $g) (global.get $ER_NODE_KIND_NODE)
      (local.get $id_off) (local.get $id_len)))
    (if (i32.eq (local.get $node) (i32.const -1)) (then (return (i32.const 1))))
    ;; Store annotation if present
    (if (local.get $anno_off)
      (then
        (call $er_node_set_placement (local.get $node)
          (call $er_str_store (local.get $g)
            (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $anno_off))
            (local.get $anno_len)))
      )
    )
    ;; Return results
    (i32.store (global.get $OFF_SCRATCH3) (local.get $node))
    (i32.store (global.get $OFF_SCRATCH5) (i32.const 0))  ;; no port
    (i32.store (global.get $OFF_SCRATCH6) (i32.const 0))
    (i32.store (global.get $OFF_SCRATCH7) (local.get $pos))
    (i32.store (global.get $OFF_SCRATCH8) (local.get $anno_off))  ;; save for edge annotations
    (i32.store (global.get $OFF_SCRATCH9) (local.get $anno_len))
    (i32.const 0)
  )

  ;; ── Parse an arrow chain ──
  ;; node.port -> stage -> node.port -> ...
  ;; Each stage can be a port ref, inline node/queue, or named stage.
  ;; Returns new pos, or -1 on error.
  (func $er_parse_arrow_chain (param $g i32) (param $pos i32) (result i32)
    (local $prev_node i32) (local $prev_port_off i32) (local $prev_port_len i32)
    (local $cur_node i32) (local $cur_port_off i32) (local $cur_port_len i32)
    (local $edge_anno_off i32) (local $edge_anno_len i32)
    ;; Parse the first endpoint
    (if (call $er_parse_chain_stage (local.get $g) (local.get $pos)) (then (return (i32.const -1))))
    (local.set $prev_node (i32.load (global.get $OFF_SCRATCH3)))
    (local.set $prev_port_off (i32.load (global.get $OFF_SCRATCH5)))
    (local.set $prev_port_len (i32.load (global.get $OFF_SCRATCH6)))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH7)))
    (drop (call $er_skip_ws (local.get $pos)))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
    ;; Check for -> arrow
    (block $chain_done
      (loop $chain_lp
        (if (i32.eqz (call $er_match_arrow (local.get $pos)))
          (then (br $chain_done))
        )
        (local.set $pos (i32.add (local.get $pos) (i32.const 2)))  ;; skip ->
        (drop (call $er_skip_ws (local.get $pos)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
        ;; Parse the next endpoint
        (if (call $er_parse_chain_stage (local.get $g) (local.get $pos)) (then (return (i32.const -1))))
        (local.set $cur_node (i32.load (global.get $OFF_SCRATCH3)))
        (local.set $cur_port_off (i32.load (global.get $OFF_SCRATCH5)))
        (local.set $cur_port_len (i32.load (global.get $OFF_SCRATCH6)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH7)))
        ;; Check if this stage has @cross or other edge annotations
        (local.set $edge_anno_off (i32.load (global.get $OFF_SCRATCH8)))
        (local.set $edge_anno_len (i32.load (global.get $OFF_SCRATCH9)))
        ;; Create edge from prev to cur
        (drop (call $er_edge_alloc (local.get $g)
          (local.get $prev_node) (local.get $prev_port_off) (local.get $prev_port_len)
          (local.get $cur_node) (local.get $cur_port_off) (local.get $cur_port_len)
          (local.get $edge_anno_off) (local.get $edge_anno_len)))
        ;; Advance: cur becomes prev for next iteration
        (local.set $prev_node (local.get $cur_node))
        (local.set $prev_port_off (local.get $cur_port_off))
        (local.set $prev_port_len (local.get $cur_port_len))
        (drop (call $er_skip_ws (local.get $pos)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
        (br $chain_lp)
      )
    )
    local.get $pos
  )
