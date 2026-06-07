;; UI Layout Stage — slot 141
  ;; Computes node positions and stores them in the shared layout buffer.
  ;; Tree must have been written to the fixed address 0x1110000 (UI_LAYOUT_TREE).
  ;; Input:  [vp_w: f32][vp_h: f32][tree_data...] — written to UI_LAYOUT_TREE
  ;; Output: [node_count: i32]  (4 bytes, -1 on error)
  ;; Prerequisite: call er_ui_layout_set_buf before pipeline_run to set
  ;;   the layout buffer address (where per-node [x,y,w,h] are stored).
  ;; Wire format (input):
  ;;   offset 0: vp_w (f32) — viewport width
  ;;   offset 4: vp_h (f32) — viewport height
  ;;   offset 8: start of serialized tree (ERUI header + records)
  ;; Error codes:
  ;;   0: insufficient input (< 12 bytes)
  ;;   -1: pipe_read or er_ui_layout returned error
  ;; Memory: reads/writes at 0x1110000

  (global $UI_LAYOUT_TREE i32 (i32.const 0x1110000))
  (global $UI_LAYOUT_TREE_CAP i32 (i32.const 131072))

  (func $process_ui_layout (export "process_ui_layout")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $avail i32) (local $read i32) (local $vp_w f32) (local $vp_h f32)
    (local $result i32)
    (local.set $avail (call $pipe_available (local.get $input)))
    (if (i32.lt_u (local.get $avail) (i32.const 12))
      (then (return (i32.const 0))))
    (local.set $read (call $pipe_read (local.get $input)
      (global.get $UI_LAYOUT_TREE) (local.get $avail)))
    (if (i32.lt_s (local.get $read) (i32.const 0))
      (then (return (local.get $read))))
    (local.set $vp_w (f32.load (global.get $UI_LAYOUT_TREE)))
    (local.set $vp_h (f32.load (i32.add (global.get $UI_LAYOUT_TREE) (i32.const 4))))
    (local.set $result (call $er_ui_layout
      (i32.add (global.get $UI_LAYOUT_TREE) (i32.const 8))
      (i32.sub (local.get $read) (i32.const 8))
      (local.get $vp_w) (local.get $vp_h)))
    (if (i32.lt_s (local.get $result) (i32.const 0))
      (then (return (local.get $result))))
    (i32.store (global.get $UI_LAYOUT_TREE) (local.get $result))
    (drop (call $pipe_write (local.get $output)
      (global.get $UI_LAYOUT_TREE) (i32.const 4)))
    (local.get $result))
