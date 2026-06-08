;; Edgerun Lower — IR graph to pipeline descriptor converter
  ;;
  ;; Walks an Edgerun IR graph and produces pipeline descriptors.
  ;; Each flow is a pipeline; inside each flow, nodes/actors/kernels
  ;; become stages, clocks configure timers, queues configure pipes.
  ;;
  ;; Node-to-stage mapping stored at 0x8F100 (256 entries x 4 bytes each).
  ;; Maps node offset → stage index in the pipeline descriptor.
  ;;
  ;; Pipeline descriptor stage config layout for Edgerun nodes:
  ;;   +0:  name_off  (into IR string buffer)
  ;;   +4:  name_len
  ;;   +8:  kind      (ER_NODE_KIND_*)
  ;;   +12: placement (offset into str_buf, 0 = none)
  ;;   total: 16 bytes

  (global $ER_CFG_SIZE i32 (i32.const 16))

  ;; ── Node-to-stage mapping table ──
  (global $ER_LOWER_MAP i32 (i32.const 0x8F100))
  (global $ER_LOWER_MAP_ENTRIES i32 (i32.const 256))
  (global $ER_LOWER_MAP_STRIDE i32 (i32.const 4))

  ;; Map a node offset to its stage index.
  (func $er_map_store (param $node i32) (param $stage_idx i32)
    (local $slot i32)
    (local.set $slot (i32.add (global.get $ER_LOWER_MAP)
      (i32.mul (i32.and (local.get $node) (i32.const 0xFF)) (i32.const 4))))
    (i32.store (local.get $slot) (local.get $stage_idx))
  )

  ;; Look up a node's stage index. Returns -1 if not mapped.
  (func $er_map_lookup (param $node i32) (result i32)
    (local $slot i32)
    (local.set $slot (i32.add (global.get $ER_LOWER_MAP)
      (i32.mul (i32.and (local.get $node) (i32.const 0xFF)) (i32.const 4))))
    (i32.load (local.get $slot))
  )

  ;; ── Stage type mapping ──
  ;; Maps ER_NODE_KIND_* to pipeline stage type indices.
  (func $er_kind_to_stage (export "er_kind_to_stage") (param $kind i32) (result i32)
    (if (i32.eq (local.get $kind) (global.get $ER_NODE_KIND_NODE))
      (then (return (i32.const 47)))
    )
    (if (i32.eq (local.get $kind) (global.get $ER_NODE_KIND_ACTOR))
      (then (return (i32.const 47)))
    )
    (if (i32.eq (local.get $kind) (global.get $ER_NODE_KIND_KERNEL))
      (then (return (i32.const 47)))
    )
    (if (i32.eq (local.get $kind) (global.get $ER_NODE_KIND_MUX))
      (then (return (i32.const 6)))
    )
    (if (i32.eq (local.get $kind) (global.get $ER_NODE_KIND_DEMUX))
      (then (return (i32.const 7)))
    )
    (if (i32.eq (local.get $kind) (global.get $ER_NODE_KIND_GATE))
      (then (return (i32.const 47)))
    )
    (if (i32.eq (local.get $kind) (global.get $ER_NODE_KIND_BATCH))
      (then (return (i32.const 47)))
    )
    (if (i32.eq (local.get $kind) (global.get $ER_NODE_KIND_REDUCE))
      (then (return (i32.const 47)))
    )
    (if (i32.eq (local.get $kind) (global.get $ER_NODE_KIND_MAP))
      (then (return (i32.const 47)))
    )
    (if (i32.eq (local.get $kind) (global.get $ER_NODE_KIND_ARBITER))
      (then (return (i32.const 47)))
    )
    (if (i32.eq (local.get $kind) (global.get $ER_NODE_KIND_FEEDBACK))
      (then (return (i32.const 47)))
    )
    (if (i32.eq (local.get $kind) (global.get $ER_NODE_KIND_SPLIT))
      (then (return (i32.const 9)))
    )
    (if (i32.eq (local.get $kind) (global.get $ER_NODE_KIND_QUEUE))
      (then (return (i32.const 48)))
    )
    (if (i32.eq (local.get $kind) (global.get $ER_NODE_KIND_BUFFER))
      (then (return (i32.const 49)))
    )
    (if (i32.eq (local.get $kind) (global.get $ER_NODE_KIND_CDC))
      (then (return (i32.const 50)))
    )
    (i32.const -1)
  )

  ;; ── Counting ──

  ;; Count nodes that map to stages (skip flow/clock/queue/buffer).
  (func $er_count_stages (export "er_count_stages") (param $g i32) (result i32)
    (local $n i32) (local $kind i32) (local $count i32)
    (local.set $n (i32.load offset=8 (local.get $g)))
    (block $done
      (loop $lp
        (if (i32.eqz (local.get $n)) (then (br $done)))
        (local.set $kind (i32.load offset=0 (local.get $n)))
        (if (i32.ge_s (call $er_kind_to_stage (local.get $kind)) (i32.const 0))
          (then (local.set $count (i32.add (local.get $count) (i32.const 1))))
        )
        (local.set $n (i32.load offset=16 (local.get $n)))
        (br $lp)
      )
    )
    local.get $count
  )

  ;; ── Config allocation ──

  ;; Allocate and populate a stage config block from a node.
  ;; Returns config offset, or -1 on alloc failure.
  (func $er_alloc_config (param $node i32) (result i32)
    (local $cfg i32)
    (local.set $cfg (call $pipe_alloc (global.get $ER_CFG_SIZE)))
    (if (i32.eq (local.get $cfg) (i32.const -1)) (then (return (i32.const -1))))
    (i32.store offset=0 (local.get $cfg) (i32.load offset=4 (local.get $node)))  ;; name_off
    (i32.store offset=4 (local.get $cfg) (i32.load offset=8 (local.get $node)))  ;; name_len
    (i32.store offset=8 (local.get $cfg) (i32.load offset=0 (local.get $node)))  ;; kind
    (i32.store offset=12 (local.get $cfg) (i32.load offset=20 (local.get $node))) ;; placement
    local.get $cfg
  )

  ;; ── Main lowering entry point ──

  ;; Lower an IR graph to a pipeline descriptor.
  ;; $g = graph offset (from er_graph_create)
  ;; $pcap = intermediate pipe capacity
  ;; Returns pipeline descriptor offset, or -1 on error.
  (func $er_lower (export "er_lower") (param $g i32) (param $pcap i32) (result i32)
    (local $desc i32) (local $count i32) (local $n i32) (local $kind i32)
    (local $stage_idx i32) (local $stype i32) (local $cfg i32)

    ;; Count stages
    (local.set $count (call $er_count_stages (local.get $g)))
    (if (i32.eqz (local.get $count)) (then (return (i32.const -1))))

    ;; Create pipeline descriptor
    (local.set $desc (call $pipeline_create (local.get $pcap) (local.get $count)))
    (if (i32.eq (local.get $desc) (i32.const -1)) (then (return (i32.const -1))))

    ;; Walk nodes, map each to a stage
    (local.set $n (i32.load offset=8 (local.get $g)))
    (local.set $stage_idx (i32.const 0))
    (block $done
      (loop $lp
        (if (i32.eqz (local.get $n)) (then (br $done)))
        (local.set $kind (i32.load offset=0 (local.get $n)))
        (local.set $stype (call $er_kind_to_stage (local.get $kind)))
        (if (i32.ge_s (local.get $stype) (i32.const 0))
          (then
            (call $er_map_store (local.get $n) (local.get $stage_idx))
            (local.set $cfg (call $er_alloc_config (local.get $n)))
            (if (i32.eq (local.get $cfg) (i32.const -1)) (then (return (i32.const -1))))
            (call $pipeline_set_stage
              (local.get $desc) (local.get $stage_idx) (local.get $stype)
              (local.get $cfg) (global.get $ER_CFG_SIZE))
            (local.set $stage_idx (i32.add (local.get $stage_idx) (i32.const 1)))
          )
        )
        (local.set $n (i32.load offset=16 (local.get $n)))
        (br $lp)
      )
    )

    local.get $desc
  )

  ;; ── Kernel lowering ──
  ;; Convert a kernel's pipe subgraph into a pipeline descriptor.
  ;; Walks sub-nodes (parent=kernel), orders by edges, creates pipeline.
  ;; Returns the pipeline descriptor offset, or -1 on error.

  ;; Count sub-nodes belonging to a kernel
  (func $er_count_kernel_stages (export "er_count_kernel_stages") (param $g i32) (param $kernel i32) (result i32)
    (local $n i32) (local $count i32)
    (local.set $n (i32.load offset=8 (local.get $g)))
    (block $done
      (loop $lp
        (if (i32.eqz (local.get $n)) (then (br $done)))
        (if (i32.eq (i32.load offset=24 (local.get $n)) (local.get $kernel))
          (then (local.set $count (i32.add (local.get $count) (i32.const 1))))
        )
        (local.set $n (i32.load offset=16 (local.get $n)))
        (br $lp)
      )
    )
    local.get $count
  )

  ;; Find the first edge where from_node equals a given node.
  ;; Returns edge offset, or 0 if not found.
  (func $er_find_outgoing_edge (export "er_find_outgoing_edge") (param $g i32) (param $from_node i32) (result i32)
    (local $e i32)
    (local.set $e (i32.load offset=12 (local.get $g)))
    (block $done
      (loop $lp
        (if (i32.eqz (local.get $e)) (then (br $done)))
        (if (i32.eq (i32.load offset=0 (local.get $e)) (local.get $from_node))
          (then (return (local.get $e)))
        )
        (local.set $e (i32.load offset=32 (local.get $e)))
        (br $lp)
      )
    )
    (i32.const 0)
  )

  ;; Find the entry node of a kernel pipe (node with parent=kernel but no incoming edge from kernel sub-nodes).
  ;; Returns node offset, or 0 if not found.
  (func $er_find_kernel_entry (export "er_find_kernel_entry") (param $g i32) (param $kernel i32) (result i32)
    (local $n i32) (local $e i32) (local $has_incoming i32)
    (local.set $n (i32.load offset=8 (local.get $g)))
    (block $done
      (loop $nlp
        (if (i32.eqz (local.get $n)) (then (br $done)))
        (if (i32.eq (i32.load offset=24 (local.get $n)) (local.get $kernel))
          (then
            ;; Check if any edge from a kernel sub-node points to this node
            (local.set $has_incoming (i32.const 0))
            (local.set $e (i32.load offset=12 (local.get $g)))
            (block $echeck
              (loop $elp
                (if (i32.eqz (local.get $e)) (then (br $echeck)))
                (if (i32.and
                      (i32.eq (i32.load offset=12 (local.get $e)) (local.get $n))
                      (i32.eq (i32.load offset=24 (i32.load offset=0 (local.get $e))) (local.get $kernel)))
                  (then (local.set $has_incoming (i32.const 1)) (br $echeck))
                )
                (local.set $e (i32.load offset=32 (local.get $e)))
                (br $elp)
              )
            )
            (if (i32.eqz (local.get $has_incoming))
              (then (return (local.get $n)))
            )
          )
        )
        (local.set $n (i32.load offset=16 (local.get $n)))
        (br $nlp)
      )
    )
    (i32.const 0)
  )

  ;; Lower a kernel's subgraph to a pipeline descriptor.
  ;; $g = graph, $kernel = kernel node offset, $pcap = intermediate pipe capacity
  ;; Returns pipeline descriptor offset, or -1 on error.
  (func $er_kernel_lower (export "er_kernel_lower") (param $g i32) (param $kernel i32) (param $pcap i32) (result i32)
    (local $count i32) (local $desc i32) (local $cur i32) (local $e i32)
    (local $stage_idx i32) (local $cfg i32)
    ;; Count sub-nodes
    (local.set $count (call $er_count_kernel_stages (local.get $g) (local.get $kernel)))
    (if (i32.eqz (local.get $count)) (then (return (i32.const -1))))
    ;; Create pipeline descriptor
    (local.set $desc (call $pipeline_create (local.get $pcap) (local.get $count)))
    (if (i32.eq (local.get $desc) (i32.const -1)) (then (return (i32.const -1))))
    ;; Find entry node
    (local.set $cur (call $er_find_kernel_entry (local.get $g) (local.get $kernel)))
    (if (i32.eqz (local.get $cur)) (then (return (i32.const -1))))
    ;; Walk chain, setting stages
    (local.set $stage_idx (i32.const 0))
    (block $done
      (loop $lp
        (if (i32.eqz (local.get $cur)) (then (br $done)))
        (call $er_map_store (local.get $cur) (local.get $stage_idx))
        (local.set $cfg (call $er_alloc_config (local.get $cur)))
        (if (i32.eq (local.get $cfg) (i32.const -1)) (then (return (i32.const -1))))
        (call $pipeline_set_stage
          (local.get $desc) (local.get $stage_idx) (i32.const 47)
          (local.get $cfg) (global.get $ER_CFG_SIZE))
        (local.set $stage_idx (i32.add (local.get $stage_idx) (i32.const 1)))
        ;; Find next node via outgoing edge
        (local.set $e (call $er_find_outgoing_edge (local.get $g) (local.get $cur)))
        (if (i32.eqz (local.get $e))
          (then (local.set $cur (i32.const 0)))
          (else (local.set $cur (i32.load offset=12 (local.get $e))))
        )
        (br $lp)
      )
    )
    local.get $desc
  )

  ;; ── Edge walker for future use ──

  ;; Walk edges and return the stage indices of connected nodes.
  ;; Output: scratch0=from_stage_idx, scratch1=to_stage_idx
  ;; Returns -1 if either end is unmapped.
  (func $er_edge_stages (export "er_edge_stages") (param $e i32) (result i32)
    (local $from_s i32) (local $to_s i32)
    (local.set $from_s (call $er_map_lookup (i32.load offset=0 (local.get $e))))
    (local.set $to_s (call $er_map_lookup (i32.load offset=12 (local.get $e))))
    (if (i32.or (i32.lt_s (local.get $from_s) (i32.const 0)) (i32.lt_s (local.get $to_s) (i32.const 0)))
      (then (return (i32.const -1)))
    )
    (i32.store (global.get $OFF_SCRATCH0) (local.get $from_s))
    (i32.store (global.get $OFF_SCRATCH1) (local.get $to_s))
    (i32.const 0)
  )
