;; Edgerun IR Graph UI Renderer
  ;;
  ;; Takes an Edgerun IR graph and renders it as a visual dataflow diagram
  ;; using the UI framework's command buffer format.
  ;;
  ;; Layout: simple top-down flow with auto-layout
  ;;   - Nodes are arranged in a grid
  ;;   - Each node is a labeled rectangle
  ;;   - Edges are lines connecting output ports to input ports
  ;;
  ;; Output: UI commands (rect, text) written to the command buffer
  ;; compatible with er_ui_render output format.

  ;; ── Node kind labels (16-byte aligned, null-terminated) ──
  (data (i32.const 0x8F400) "flow\00\00\00\00\00\00\00\00\00\00\00")
  (data (i32.const 0x8F410) "node\00\00\00\00\00\00\00\00\00\00\00")
  (data (i32.const 0x8F420) "actor\00\00\00\00\00\00\00\00\00\00")
  (data (i32.const 0x8F430) "clock\00\00\00\00\00\00\00\00\00\00")
  (data (i32.const 0x8F440) "queue\00\00\00\00\00\00\00\00\00\00")
  (data (i32.const 0x8F450) "buffer\00\00\00\00\00\00\00\00\00")
  (data (i32.const 0x8F460) "mux\00\00\00\00\00\00\00\00\00\00\00")
  (data (i32.const 0x8F470) "demux\00\00\00\00\00\00\00\00\00\00")
  (data (i32.const 0x8F480) "join\00\00\00\00\00\00\00\00\00\00\00")
  (data (i32.const 0x8F490) "kernel\00\00\00\00\00\00\00\00\00")
  (data (i32.const 0x8F4A0) "cdc\00\00\00\00\00\00\00\00\00\00\00\00")
  (data (i32.const 0x8F4B0) "gate\00\00\00\00\00\00\00\00\00\00\00")
  (data (i32.const 0x8F4C0) "batch\00\00\00\00\00\00\00\00\00\00")
  (data (i32.const 0x8F4D0) "reduce\00\00\00\00\00\00\00\00\00")
  (data (i32.const 0x8F4E0) "map\00\00\00\00\00\00\00\00\00\00\00\00")
  (data (i32.const 0x8F4F0) "arbiter\00\00\00\00\00\00\00\00\00")
  (data (i32.const 0x8F500) "feedback\00\00\00\00\00\00\00")
  (data (i32.const 0x8F510) "split\00\00\00\00\00\00\00\00\00\00")
  (data (i32.const 0x8F520) "?\00\00\00\00\00\00\00\00\00\00\00\00\00\00")

  ;; ── Kind name lengths table (18 entries, 4 bytes each at 0x8F600) ──
  (data (i32.const 0x8F600)
    "\04\00\00\00"  ;; 0: flow
    "\04\00\00\00"  ;; 1: node
    "\05\00\00\00"  ;; 2: actor
    "\05\00\00\00"  ;; 3: clock
    "\05\00\00\00"  ;; 4: queue
    "\06\00\00\00"  ;; 5: buffer
    "\03\00\00\00"  ;; 6: mux
    "\05\00\00\00"  ;; 7: demux
    "\04\00\00\00"  ;; 8: join
    "\06\00\00\00"  ;; 9: kernel
    "\03\00\00\00"  ;; 10: cdc
    "\04\00\00\00"  ;; 11: gate
    "\05\00\00\00"  ;; 12: batch
    "\06\00\00\00"  ;; 13: reduce
    "\03\00\00\00"  ;; 14: map
    "\07\00\00\00"  ;; 15: arbiter
    "\08\00\00\00"  ;; 16: feedback
    "\05\00\00\00"  ;; 17: split
  )

  ;; Returns (name_ptr, name_len) in scratch0/scratch1 for a given ER_NODE_KIND
  (func $er_kind_name (param $kind i32)
    (if (i32.gt_u (local.get $kind) (i32.const 17))
      (then (local.set $kind (i32.const 17)))
    )
    (i32.store (global.get $OFF_SCRATCH0)
      (i32.add (i32.const 0x8F400) (i32.mul (local.get $kind) (i32.const 16))))
    (i32.store (global.get $OFF_SCRATCH1)
      (i32.load (i32.add (i32.const 0x8F600) (i32.mul (local.get $kind) (i32.const 4)))))
  )

  ;; ── Graph renderer ──
  ;; Walks an IR graph and emits UI commands (rect, text) to the output buffer.
  ;;
  ;; $graph = graph offset from er_parse
  ;; $out   = command buffer
  ;; $cap   = command buffer capacity in bytes
  ;; $x, $y = viewport origin
  ;; $w, $h = viewport size
  ;;
  ;; Returns command count, or -1 on error.
  ;;
  (func $er_graph_render (export "er_graph_render")
    (param $graph i32) (param $out i32) (param $cap i32)
    (param $x f32) (param $y f32) (param $w f32) (param $h f32)
    (result i32)
    (local $count i32)
    (local $n i32) (local $kind i32) (local $idx i32)
    (local $cmd_idx i32)
    (local $nx f32) (local $ny f32) (local $cols i32)
    (local $nw f32) (local $nh f32)

    ;; Count nodes in graph (walk linked list)
    (local.set $n (i32.load offset=8 (local.get $graph)))
    (local.set $count (i32.const 0))
    (block $count_done
      (loop $count_lp
        (if (i32.eqz (local.get $n)) (then (br $count_done)))
        (local.set $count (i32.add (local.get $count) (i32.const 1)))
        (local.set $n (i32.load offset=16 (local.get $n)))
        (br $count_lp)
      )
    )

    (if (i32.eqz (local.get $count)) (then (return (i32.const 0))))

    ;; Grid layout: 4 columns, 52px rows
    (local.set $cols (i32.const 4))
    (local.set $nw (f32.div (local.get $w) (f32.convert_i32_s (local.get $cols))))
    (local.set $nh (f32.const 48))

    ;; Walk nodes and emit rect + text for each
    (local.set $n (i32.load offset=8 (local.get $graph)))
    (local.set $idx (i32.const 0))
    (block $done
      (loop $lp
        (if (i32.eqz (local.get $n)) (then (br $done)))

        (local.set $kind (i32.load offset=0 (local.get $n)))
        (local.set $nx (f32.add (local.get $x)
          (f32.mul (f32.const 4)
            (f32.add (f32.convert_i32_s (i32.rem_u (local.get $idx) (local.get $cols)))
                     (f32.const 0.5)))))
        (local.set $ny (f32.add (local.get $y)
          (f32.mul (f32.const 52)
            (f32.convert_i32_s (i32.div_u (local.get $idx) (local.get $cols))))))

        ;; Look up kind name via $er_kind_name
        (call $er_kind_name (local.get $kind))

        ;; Emit background rect (dark panel)
        (local.set $cmd_idx (call $emit_rect_raw (local.get $out) (local.get $cap)
          (local.get $cmd_idx) (local.get $nx) (local.get $ny)
          (f32.sub (local.get $nw) (f32.const 8))
          (f32.sub (local.get $nh) (f32.const 4))
          (i32.const 0xff2a1f18)))

        ;; Emit kind label
        (local.set $cmd_idx (call $emit_text_raw (local.get $out) (local.get $cap)
          (local.get $cmd_idx) (local.get $nx) (local.get $ny)
          (f32.sub (local.get $nw) (f32.const 8))
          (f32.const 14)
          (i32.load (global.get $OFF_SCRATCH0))
          (i32.load (global.get $OFF_SCRATCH1))
          (i32.const 0xffc9b8ad)))

        (local.set $n (i32.load offset=16 (local.get $n)))
        (local.set $idx (i32.add (local.get $idx) (i32.const 1)))
        (br $lp)
      )
    )

    ;; Walk edges and emit line commands
    (local.set $n (i32.load offset=12 (local.get $graph)))  ;; first_edge
    (block $edge_done
      (loop $edge_lp
        (if (i32.eqz (local.get $n)) (then (br $edge_done)))

        ;; Edge struct: +0:src_node, +4:src_port_off, +8:src_port_len,
        ;; +12:dst_node, +16:dst_port_off, +20:dst_port_len,
        ;; +24:anno_off, +28:anno_len, +32:next_edge
        ;; For now, just count edges (future: draw connection lines)
        (local.set $n (i32.load offset=32 (local.get $n)))
        (br $edge_lp)
      )
    )

    local.get $cmd_idx
  )

  ;; ── Low-level emit helpers ──
  ;; These write directly to the UI command buffer (48-byte commands).

  ;; Emit a rect command. Returns new count.
  (func $emit_rect_raw (param $out i32) (param $cap i32)
    (param $count i32) (param $rx f32) (param $ry f32)
    (param $rw f32) (param $rh f32) (param $color i32) (result i32)
    (local $cmd i32)
    (local.set $cmd (i32.add (local.get $out)
      (i32.mul (local.get $count) (i32.const 48))))
    (if (i32.gt_u (i32.add (i32.add (local.get $cmd) (i32.const 48)) (i32.const -1))
                  (i32.add (local.get $out) (local.get $cap)))
      (then (return (local.get $count)))
    )
    (i32.store offset=0 (local.get $cmd) (i32.const 1))  ;; tag=1 (rect)
    (i32.store offset=4 (local.get $cmd) (local.get $color))
    (f32.store offset=8 (local.get $cmd) (local.get $rx))
    (f32.store offset=12 (local.get $cmd) (local.get $ry))
    (f32.store offset=16 (local.get $cmd) (local.get $rw))
    (f32.store offset=20 (local.get $cmd) (local.get $rh))
    (i32.add (local.get $count) (i32.const 1))
  )

  ;; Emit a text command. Returns new count.
  (func $emit_text_raw (param $out i32) (param $cap i32)
    (param $count i32) (param $tx f32) (param $ty f32)
    (param $tw f32) (param $th f32)
    (param $text_ptr i32) (param $text_len i32) (param $color i32) (result i32)
    (local $cmd i32)
    (local.set $cmd (i32.add (local.get $out)
      (i32.mul (local.get $count) (i32.const 48))))
    (if (i32.gt_u (i32.add (i32.add (local.get $cmd) (i32.const 48)) (i32.const -1))
                  (i32.add (local.get $out) (local.get $cap)))
      (then (return (local.get $count)))
    )
    (i32.store offset=0 (local.get $cmd) (i32.const 2))  ;; tag=2 (text)
    (i32.store offset=4 (local.get $cmd) (local.get $color))
    (f32.store offset=8 (local.get $cmd) (local.get $tx))
    (f32.store offset=12 (local.get $cmd) (local.get $ty))
    (f32.store offset=16 (local.get $cmd) (local.get $tw))
    (f32.store offset=20 (local.get $cmd) (local.get $th))
    (i32.store offset=24 (local.get $cmd) (local.get $text_ptr))
    (i32.store offset=28 (local.get $cmd) (local.get $text_len))
    (i32.add (local.get $count) (i32.const 1))
  )
