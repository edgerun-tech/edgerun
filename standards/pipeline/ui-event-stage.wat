;; UI Event Stage — slot 143
  ;; Hit-tests a point against the cached layout buffer.
  ;; Requires that er_ui_layout_set_buf was called and layout stage ran first.

  (global $UI_EVENT_TMP i32 (i32.const 0x1110000))
  ;; Input:  [px: f32][py: f32][node_count: i32] (12 bytes total, at UI_EVENT_TMP)
  ;; Output: [hit_index: i32]  (-1 = no hit, >=0 = node index hit)
  ;; Wire format (input):
  ;;   offset 0: px (f32) — pointer x
  ;;   offset 4: py (f32) — pointer y
  ;;   offset 8: node_count (i32) — number of nodes in the layout buffer
  ;; Error codes:
  ;;   0: insufficient input (< 12 bytes)
  ;; Prerequisite: call er_ui_layout_set_buf before pipeline_run,
  ;;               and the layout stage must have been run first.
  ;; Memory: reads/writes at 0x1110000

  (func $process_ui_event (export "process_ui_event")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $px i32) (local $py i32)
    (local $avail i32) (local $px f32) (local $py f32) (local $node_count i32)
    (local $hit i32)
    (local.set $avail (call $pipe_available (local.get $input)))
    (if (i32.lt_u (local.get $avail) (i32.const 12))
      (then (return (i32.const 0))))
    (drop (call $pipe_read (local.get $input) (global.get $UI_EVENT_TMP) (i32.const 12)))
    (local.set $px (f32.load (global.get $UI_EVENT_TMP)))
    (local.set $py (f32.load (i32.add (global.get $UI_EVENT_TMP) (i32.const 4))))
    (local.set $node_count (i32.load (i32.add (global.get $UI_EVENT_TMP) (i32.const 8))))
    (local.set $hit (call $er_ui_layout_hit_test
      (local.get $px) (local.get $py) (local.get $node_count)))
    (i32.store (global.get $UI_EVENT_TMP) (local.get $hit))
    (drop (call $pipe_write (local.get $output)
      (global.get $UI_EVENT_TMP) (i32.const 4)))
    (local.get $hit))
