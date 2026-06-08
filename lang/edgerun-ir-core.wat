;; Edgerun IR Core — graph node structs + heap allocation
  ;; Builds on pipeline/pipe-core.wat ($pipe_alloc) and runtime/edgerun-core.wat.
  ;;
  ;; IR memory layout (all offsets into the bump heap at 0x40000):
  ;;
  ;; Graph (root stored in global)
  ;;   +0: node_count  i32
  ;;   +4: edge_count  i32
  ;;   +8: first_node  i32 (0 = empty)
  ;;   +12: first_edge i32 (0 = empty)
  ;;   +16: str_buf    i32 (next free byte in string area)
  ;;   total: 20 bytes
  ;;
  ;; GraphNode
  ;;   +0: kind        i32   (ER_NODE_* constant)
  ;;   +4: name_off    i32   (offset into str_buf)
  ;;   +8: name_len    i32
  ;;   +12: first_port i32   (0 = no ports)
  ;;   +16: next_node  i32   (linked list, 0 = end)
  ;;   +20: placement  i32   (offset into str_buf, 0 = none)
  ;;   +24: parent     i32   (0 = top-level, otherwise offset of parent kernel/flow node)
  ;;   +28: config     i32   (offset of kind-specific config block, 0 = none)
  ;;   total: 32 bytes
  ;;
  ;; Port
  ;;   +0: direction   i32   (0=in, 1=out, 2=inout)
  ;;   +4: name_off    i32
  ;;   +8: name_len    i32
  ;;   +12: type_off   i32   (type string offset, 0 = untyped)
  ;;   +16: type_len   i32
  ;;   +20: next_port  i32
  ;;   +24: annotation i32   (offset into str_buf, 0 = none)
  ;;   total: 28 bytes
  ;;
  ;; Edge
  ;;   +0: from_node   i32   (offset of GraphNode)
  ;;   +4: from_port   i32   (port name offset)
  ;;   +8: from_port_len i32
  ;;   +12: to_node    i32
  ;;   +16: to_port    i32
  ;;   +20: to_port_len i32
  ;;   +24: annotation i32   (offset into str_buf, 0 = none)
  ;;   +28: anno_len   i32
  ;;   +32: next_edge  i32
  ;;   total: 36 bytes

  ;; ── IR struct sizes ──
  (global $ER_GRAPH_SIZE   i32 (i32.const 20))
  (global $ER_NODE_SIZE    i32 (i32.const 32))
  (global $ER_PORT_SIZE    i32 (i32.const 28))
  (global $ER_EDGE_SIZE    i32 (i32.const 36))

  ;; ── Node kind constants ──
  (global $ER_NODE_KIND_FLOW    i32 (i32.const 0))
  (global $ER_NODE_KIND_NODE    i32 (i32.const 1))
  (global $ER_NODE_KIND_ACTOR   i32 (i32.const 2))
  (global $ER_NODE_KIND_CLOCK   i32 (i32.const 3))
  (global $ER_NODE_KIND_QUEUE   i32 (i32.const 4))
  (global $ER_NODE_KIND_BUFFER  i32 (i32.const 5))
  (global $ER_NODE_KIND_MUX     i32 (i32.const 6))
  (global $ER_NODE_KIND_DEMUX   i32 (i32.const 7))
  (global $ER_NODE_KIND_JOIN    i32 (i32.const 8))
  (global $ER_NODE_KIND_KERNEL  i32 (i32.const 9))
  (global $ER_NODE_KIND_CDC       i32 (i32.const 10))
  (global $ER_NODE_KIND_GATE      i32 (i32.const 11))
  (global $ER_NODE_KIND_BATCH     i32 (i32.const 12))
  (global $ER_NODE_KIND_REDUCE    i32 (i32.const 13))
  (global $ER_NODE_KIND_MAP       i32 (i32.const 14))
  (global $ER_NODE_KIND_ARBITER   i32 (i32.const 15))
  (global $ER_NODE_KIND_FEEDBACK  i32 (i32.const 16))
  (global $ER_NODE_KIND_SPLIT     i32 (i32.const 17))

  ;; ── Graph root (global pointer, 0 = uninitialized) ──
  (global $er_graph (mut i32) (i32.const 0))

  ;; ── String buffer state ──
  ;; Strings are stored in the heap after all IR structs.
  ;; $er_str_ptr points to the next free byte in the string area.
  (global $er_str_ptr (mut i32) (i32.const 0))

  ;; ── Graph lifecycle ──

  ;; Allocate a new empty graph. Returns graph offset.
  (func $er_graph_create (export "er_graph_create") (result i32)
    (local $g i32)
    (local.set $g (call $pipe_alloc (global.get $ER_GRAPH_SIZE)))
    (if (i32.eq (local.get $g) (i32.const -1))
      (then (return (i32.const -1)))
    )
    (i32.store offset=0 (local.get $g) (i32.const 0))  ;; node_count
    (i32.store offset=4 (local.get $g) (i32.const 0))  ;; edge_count
    (i32.store offset=8 (local.get $g) (i32.const 0))  ;; first_node
    (i32.store offset=12 (local.get $g) (i32.const 0)) ;; first_edge
    ;; str_buf starts after the graph header + room for growth
    (i32.store offset=16 (local.get $g) (i32.add (local.get $g) (global.get $ER_GRAPH_SIZE)))
    (global.set $er_graph (local.get $g))
    (global.set $er_str_ptr (i32.add (local.get $g) (global.get $ER_GRAPH_SIZE)))
    local.get $g
  )

  ;; ── Node operations ──

  ;; Allocate a node in the graph.
  ;; $g = graph offset, $kind = ER_NODE_KIND_*, $name_off/len = name in str_buf
  ;; Returns node offset, or -1 on alloc failure.
  (func $er_node_alloc (export "er_node_alloc") (param $g i32) (param $kind i32) (param $name_off i32) (param $name_len i32) (result i32)
    (local $n i32) (local $first i32)
    (local.set $n (call $pipe_alloc (global.get $ER_NODE_SIZE)))
    (if (i32.eq (local.get $n) (i32.const -1))
      (then (return (i32.const -1)))
    )
    (i32.store offset=0 (local.get $n) (local.get $kind))
    (i32.store offset=4 (local.get $n) (local.get $name_off))
    (i32.store offset=8 (local.get $n) (local.get $name_len))
    (i32.store offset=12 (local.get $n) (i32.const 0))  ;; first_port
    (i32.store offset=16 (local.get $n) (i32.const 0))  ;; next_node
    (i32.store offset=20 (local.get $n) (i32.const 0))  ;; placement
    (i32.store offset=24 (local.get $n) (i32.const 0))  ;; parent
    (i32.store offset=28 (local.get $n) (i32.const 0))  ;; config
    ;; Prepend to node list
    (local.set $first (i32.load offset=8 (local.get $g)))
    (i32.store offset=16 (local.get $n) (local.get $first))
    (i32.store offset=8 (local.get $g) (local.get $n))
    ;; Increment count
    (i32.store offset=0 (local.get $g)
      (i32.add (i32.load offset=0 (local.get $g)) (i32.const 1)))
    local.get $n
  )

  ;; Set placement on a node.
  (func $er_node_set_placement (export "er_node_set_placement") (param $node i32) (param $p_off i32)
    (i32.store offset=20 (local.get $node) (local.get $p_off))
  )

  ;; Set parent on a node (for kernel/flow sub-nodes).
  (func $er_node_set_parent (export "er_node_set_parent") (param $node i32) (param $parent i32)
    (i32.store offset=24 (local.get $node) (local.get $parent))
  )

  ;; Get parent of a node (0 = top-level).
  (func $er_node_get_parent (export "er_node_get_parent") (param $node i32) (result i32)
    (i32.load offset=24 (local.get $node))
  )

  ;; Set config on a node.
  (func $er_node_set_config (export "er_node_set_config") (param $node i32) (param $cfg i32)
    (i32.store offset=28 (local.get $node) (local.get $cfg))
  )

  ;; Get config on a node (0 = none).
  (func $er_node_get_config (export "er_node_get_config") (param $node i32) (result i32)
    (i32.load offset=28 (local.get $node))
  )

  ;; ── Port operations ──

  ;; Allocate a port on a node.
  ;; $node = node offset, $dir = direction, $name_off/len, $type_off/len
  ;; Returns port offset, or -1.
  (func $er_port_alloc (export "er_port_alloc") (param $node i32) (param $dir i32)
    (param $name_off i32) (param $name_len i32)
    (param $type_off i32) (param $type_len i32) (result i32)
    (local $p i32) (local $first i32)
    (local.set $p (call $pipe_alloc (global.get $ER_PORT_SIZE)))
    (if (i32.eq (local.get $p) (i32.const -1))
      (then (return (i32.const -1)))
    )
    (i32.store offset=0 (local.get $p) (local.get $dir))
    (i32.store offset=4 (local.get $p) (local.get $name_off))
    (i32.store offset=8 (local.get $p) (local.get $name_len))
    (i32.store offset=12 (local.get $p) (local.get $type_off))
    (i32.store offset=16 (local.get $p) (local.get $type_len))
    (i32.store offset=20 (local.get $p) (i32.const 0))  ;; next_port
    (i32.store offset=24 (local.get $p) (i32.const 0))  ;; annotation
    ;; Prepend to port list on node
    (local.set $first (i32.load offset=12 (local.get $node)))
    (i32.store offset=20 (local.get $p) (local.get $first))
    (i32.store offset=12 (local.get $node) (local.get $p))
    local.get $p
  )

  ;; Set annotation on a port (clock binding, rate, etc.)
  (func $er_port_set_annotation (export "er_port_set_annotation") (param $port i32) (param $anno_off i32)
    (i32.store offset=24 (local.get $port) (local.get $anno_off))
  )

  ;; Get annotation on a port (0 = none).
  (func $er_port_get_annotation (export "er_port_get_annotation") (param $port i32) (result i32)
    (i32.load offset=24 (local.get $port))
  )

  ;; ── Edge operations ──

  ;; Allocate an edge between two ports.
  (func $er_edge_alloc (export "er_edge_alloc") (param $g i32)
    (param $from_node i32) (param $from_port i32) (param $from_port_len i32)
    (param $to_node i32)   (param $to_port i32)   (param $to_port_len i32)
    (param $anno_off i32)  (param $anno_len i32)  (result i32)
    (local $e i32) (local $first i32)
    (local.set $e (call $pipe_alloc (global.get $ER_EDGE_SIZE)))
    (if (i32.eq (local.get $e) (i32.const -1))
      (then (return (i32.const -1)))
    )
    (i32.store offset=0 (local.get $e) (local.get $from_node))
    (i32.store offset=4 (local.get $e) (local.get $from_port))
    (i32.store offset=8 (local.get $e) (local.get $from_port_len))
    (i32.store offset=12 (local.get $e) (local.get $to_node))
    (i32.store offset=16 (local.get $e) (local.get $to_port))
    (i32.store offset=20 (local.get $e) (local.get $to_port_len))
    (i32.store offset=24 (local.get $e) (local.get $anno_off))
    (i32.store offset=28 (local.get $e) (local.get $anno_len))
    (i32.store offset=32 (local.get $e) (i32.const 0))  ;; next_edge
    ;; Prepend to edge list
    (local.set $first (i32.load offset=12 (local.get $g)))
    (i32.store offset=32 (local.get $e) (local.get $first))
    (i32.store offset=12 (local.get $g) (local.get $e))
    (i32.store offset=4 (local.get $g)
      (i32.add (i32.load offset=4 (local.get $g)) (i32.const 1)))
    local.get $e
  )

  ;; ── String storage ──

  ;; Store a string in the graph's string buffer.
  ;; $g = graph, $src = source pointer, $len = length
  ;; Returns offset of stored string (in the str_buf area).
  (func $er_str_store (export "er_str_store") (param $g i32) (param $src i32) (param $len i32) (result i32)
    (local $dst i32) (local $i i32)
    (local.set $dst (global.get $er_str_ptr))
    (global.set $er_str_ptr (i32.add (local.get $dst) (local.get $len)))
    (local.set $i (i32.const 0))
    (block $done
      (loop $lp
        (if (i32.ge_u (local.get $i) (local.get $len)) (then (br $done)))
        (i32.store8 (i32.add (local.get $dst) (local.get $i))
          (i32.load8_u (i32.add (local.get $src) (local.get $i)))
        )
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $lp)
      )
    )
    local.get $dst
  )

  ;; ── Graph traversal ──

  (type $node_walker (func (param i32 i32) (result i32)))

  ;; Walk graph nodes, calling $callback(node_offset, user_data) for each.
  ;; callback should return 0 to continue, non-zero to stop.
  (func $er_walk_nodes (export "er_walk_nodes") (param $g i32) (param $callback i32) (param $user i32) (result i32)
    (local $n i32)
    (local.set $n (i32.load offset=8 (local.get $g)))
    (block $done
      (loop $lp
        (if (i32.eqz (local.get $n)) (then (br $done)))
        (if (call_indirect (type $node_walker) (local.get $n) (local.get $user) (local.get $callback))
          (then (return (local.get $n)))
        )
        (local.set $n (i32.load offset=16 (local.get $n)))
        (br $lp)
      )
    )
    (i32.const 0)
  )

  ;; Find a node by name. Returns offset or 0 if not found.
  (func $er_find_node (export "er_find_node") (param $g i32) (param $name_off i32) (param $name_len i32) (result i32)
    (local $n i32) (local $n_off i32) (local $n_len i32) (local $i i32) (local $b1 i32) (local $b2 i32)
    (local.set $n (i32.load offset=8 (local.get $g)))
    (block $done
      (loop $lp
        (if (i32.eqz (local.get $n)) (then (br $done)))
        (local.set $n_len (i32.load offset=8 (local.get $n)))
        (if (i32.eq (local.get $n_len) (local.get $name_len))
          (then
            (local.set $n_off (i32.load offset=4 (local.get $n)))
            (local.set $i (i32.const 0))
            (block $cmp_done
              (loop $cmp_lp
                (if (i32.ge_u (local.get $i) (local.get $n_len)) (then (br $cmp_done)))
                (local.set $b1 (i32.load8_u (i32.add (local.get $n_off) (local.get $i))))
                (local.set $b2 (i32.load8_u (i32.add (local.get $name_off) (local.get $i))))
                (if (i32.ne (local.get $b1) (local.get $b2)) (then (local.set $i (i32.const -1)) (br $cmp_done)))
                (local.set $i (i32.add (local.get $i) (i32.const 1)))
                (br $cmp_lp)
              )
            )
            (if (i32.ge_u (local.get $i) (local.get $n_len))
              (then (return (local.get $n)))
            )
          )
        )
        (local.set $n (i32.load offset=16 (local.get $n)))
        (br $lp)
      )
    )
    (i32.const 0)
  )
