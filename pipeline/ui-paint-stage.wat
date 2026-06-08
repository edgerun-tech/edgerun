;; UI Paint Stage — slot 142
  ;; Renders UI tree to command buffer.
  ;; Tree must have been written to the fixed address 0x1110000 (UI_PAINT_TREE).
  ;; Input:  [vp_w: f32][vp_h: f32][tree_data...] — written to UI_PAINT_TREE
  ;; Output: [cmd_count: i32][command data...]
  ;;   First 4 bytes: cmd_count (i32)
  ;;   Then cmd_count * 48 bytes of command records
  ;; Each command (48 bytes):
  ;;   +0: kind (i32) — 1=rect, 2=text, 3=icon_path
  ;;   +4: color (i32) — RGBA packed
  ;;   +8: x (f32)
  ;;  +12: y (f32)
  ;;  +16: w (f32)
  ;;  +20: h (f32)
  ;;  +24: text_ptr (i32) — text string pointer (only for text cmds)
  ;;  +28: text_len (i32) — text string length
  ;;  +32: owner_id (i32) — set by post-processing
  ;;  +36: owner_kind (i32)
  ;;  +40: owner_role (i32)
  ;;  +44: reserved
  ;; Error codes:
  ;;   0: insufficient input (< 12 bytes)
  ;;   -1: pipe_read or er_ui_render returned error
  ;; Memory: reads at 0x1110000, writes at 0x1120000

  (global $UI_PAINT_TREE_CAP i32 (i32.const 131072))
  ;; $UI_PAINT_TREE and $UI_PAINT_CMD come from config.wat
  (global $UI_PAINT_CMD_CAP i32 (i32.const 131072))

  (func $process_ui_paint (export "process_ui_paint")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $vp_w i32) (local $vp_h i32)
    (local $avail i32) (local $read i32) (local $vp_w f32) (local $vp_h f32)
    (local $cmd_count i32) (local $cmd_bytes i32)
    (local.set $avail (call $pipe_available (local.get $input)))
    (if (i32.lt_u (local.get $avail) (i32.const 12))
      (then (return (i32.const 0))))
    (local.set $read (call $pipe_read (local.get $input)
      (global.get $UI_PAINT_TREE) (local.get $avail)))
    (if (i32.lt_s (local.get $read) (i32.const 0))
      (then (return (local.get $read))))
    (local.set $vp_w (f32.load (global.get $UI_PAINT_TREE)))
    (local.set $vp_h (f32.load (i32.add (global.get $UI_PAINT_TREE) (i32.const 4))))
    (local.set $cmd_count (call $er_ui_render
      (i32.add (global.get $UI_PAINT_TREE) (i32.const 8))
      (i32.sub (local.get $read) (i32.const 8))
      (global.get $UI_PAINT_CMD) (global.get $UI_PAINT_CMD_CAP)
      (f32.const 0) (f32.const 0) (local.get $vp_w) (local.get $vp_h)))
    (if (i32.lt_s (local.get $cmd_count) (i32.const 0))
      (then (return (local.get $cmd_count))))
    (local.set $cmd_bytes (i32.mul (local.get $cmd_count) (i32.const 48)))
    (i32.store (global.get $UI_PAINT_TREE) (local.get $cmd_count))
    (drop (call $pipe_write (local.get $output)
      (global.get $UI_PAINT_TREE) (i32.const 4)))
    (drop (call $pipe_write (local.get $output)
      (global.get $UI_PAINT_CMD) (local.get $cmd_bytes)))
    (local.get $cmd_count))
