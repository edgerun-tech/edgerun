;; ── Example: Build UI using edgerun UI APIs ──
;;
;; Compile: wasm-tools print examples/ui-card-demo.wat -o ui-card-demo.wasm
;; Validate: wasm-tools validate ui-card-demo.wasm
;;
;; Pipeline: writer_begin → writer_string → writer_record → validate → measure → render
;;
;; Memory layout (above FB @ 0x800000-0x1000000, within 128 MB):
;;   $BUF  0x2000000 — tree buffer (256 KB)
;;   $STR  0x2100000 — string data
;;   $CMD  0x2200000 — command output (256 KB)
;;   $S    0x2300000 — layout scratch

(module
  (import "edgerun" "memory" (memory 1))

  ;; ── Writer API ──
  (import "edgerun" "er_ui_writer_begin"
    (func $writer_begin (param i32 i32 i32 i32 i32 i32 i32) (result i32)))
  (import "edgerun" "er_ui_writer_string"
    (func $writer_string (param i32 i32) (result i32)))
  (import "edgerun" "er_ui_writer_record"
    (func $writer_record (param i32 i32 i32 i32 i32) (result i32)))
  (import "edgerun" "er_ui_writer_record_child"
    (func $writer_record_child (param i32 i32 i32 i32 i32 i32) (result i32)))
  (import "edgerun" "er_ui_writer_cursor"
    (func $writer_cursor (result i32)))

  ;; ── Validation ──
  (import "edgerun" "er_ui_validate_deep"
    (func $validate_deep (param i32 i32) (result i32)))

  ;; ── Measure ──
  (import "edgerun" "er_ui_measure"
    (func $measure (param i32 i32 i32) (result i64)))
  (import "edgerun" "er_ui_measure_packed_width"
    (func $measure_w (param i64) (result f32)))
  (import "edgerun" "er_ui_measure_packed_height"
    (func $measure_h (param i64) (result f32)))

  ;; ── Render ──
  (import "edgerun" "er_ui_render"
    (func $render (param i32 i32 i32 i32 f32 f32 f32 f32) (result i32)))

  ;; ── Convenience constructors ──
  (import "edgerun" "er_ui_wasm_new_card"
    (func $new_card (param i32 i32 i32 i32 i32 i32 i32) (result i32)))
  (import "edgerun" "er_ui_wasm_new_button"
    (func $new_button (param i32 i32 i32 i32 i32 i32 i32 i32) (result i32)))

  ;; ── Constants ──
  (import "edgerun" "er_ui_stack_axis_column"
    (func $axis_col (result i32)))
  (import "edgerun" "er_ui_stack_default_gap"
    (func $default_gap (result i32)))
  (import "edgerun" "er_ui_stack_default_padding"
    (func $default_padding (result i32)))

  ;; ── Memory layout ──
  (global $BUF      i32 (i32.const 0x2000000))
  (global $BUF_CAP  i32 (i32.const 262144))
  (global $STR      i32 (i32.const 0x2100000))
  (global $CMD      i32 (i32.const 0x2200000))
  (global $CMD_CAP  i32 (i32.const 262144))
  (global $S        i32 (i32.const 0x2300000))

  ;; ── String data ──
  (data (i32.const 0x2100000) "Welcome to EdgeRun")
  (data (i32.const 0x2100020) "A decentralized edge computing platform")
  (data (i32.const 0x2100050) "Get Started")
  (data (i32.const 0x2100060) "Enable dark mode")

  ;; ── build_card: simple card using convenience constructor ──
  (func (export "build_card") (result i32)
    (local $tree_len i32) (local $ok i32) (local $cmd_count i32)

    global.get $BUF
    global.get $BUF_CAP
    global.get $STR
    i32.const 20
    global.get $STR
    i32.const 0x20
    i32.add
    i32.const 38
    i32.const 0
    call $new_card
    local.set $tree_len
    local.get $tree_len i32.eqz if i32.const -1 return end

    global.get $BUF
    local.get $tree_len
    call $validate_deep
    local.set $ok
    local.get $ok i32.eqz if i32.const -2 return end

    global.get $BUF
    local.get $tree_len
    global.get $CMD
    global.get $CMD_CAP
    f32.const 0
    f32.const 0
    f32.const 320
    f32.const 200
    call $render
    local.set $cmd_count

    local.get $cmd_count
  )

  ;; ── build_dialog: composed tree (card + button + switch) using writer API ──
  (func (export "build_dialog") (result i32)
    (local $cursor i32) (local $title_ref i32) (local $detail_ref i32)
    (local $switch_ref i32) (local $btn_ref i32)
    (local $tree_len i32) (local $ok i32) (local $cmd_count i32)

    global.get $BUF
    global.get $BUF_CAP
    i32.const 5
    i32.const 1
    call $axis_col
    call $default_gap
    call $default_padding
    call $writer_begin
    local.set $cursor
    local.get $cursor i32.eqz if i32.const -1 return end

    global.get $STR i32.const 20 call $writer_string  local.set $title_ref
    global.get $STR i32.const 0x20 i32.add i32.const 38 call $writer_string  local.set $detail_ref
    global.get $STR i32.const 0x60 i32.add i32.const 16 call $writer_string  local.set $switch_ref
    global.get $STR i32.const 0x50 i32.add i32.const 11 call $writer_string  local.set $btn_ref

    i32.const 0  i32.const 11  i32.const 100
    local.get $title_ref local.get $detail_ref
    call $writer_record
    i32.eqz if i32.const -3 return end

    i32.const 1  i32.const 0  i32.const 1  i32.const 101
    local.get $detail_ref  i32.const 0
    call $writer_record_child
    i32.eqz if i32.const -3 return end

    i32.const 2  i32.const 0  i32.const 15  i32.const 102
    i32.const 0  i32.const 0
    call $writer_record_child
    i32.eqz if i32.const -3 return end

    i32.const 3  i32.const 0  i32.const 5  i32.const 103
    local.get $switch_ref  i32.const 1
    call $writer_record_child
    i32.eqz if i32.const -3 return end

    i32.const 4  i32.const 0  i32.const 12  i32.const 104
    local.get $btn_ref  i32.const 0
    call $writer_record_child
    i32.eqz if i32.const -3 return end

    call $writer_cursor  local.set $tree_len

    global.get $BUF  local.get $tree_len  call $validate_deep  local.set $ok
    local.get $ok i32.eqz if i32.const -4 return end

    global.get $BUF  local.get $tree_len
    global.get $CMD  global.get $CMD_CAP
    f32.const 0  f32.const 0  f32.const 320  f32.const 400
    call $render  local.set $cmd_count

    local.get $cmd_count
  )
)
