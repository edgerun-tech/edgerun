;; ── EdgeRun Pipeline System — Comprehensive Demo ──
;; ─────────────────────────────────────────────────────
;; Demonstrates:
;;   1. Pipeline lifecycle (create, configure, run)
;;   2. Mux/Demux for data flow routing
;;   3. Frame-based messaging (stream_id + payload)
;;   4. Batch fusion optimization (consecutive stateless stages)
;;   5. UI rendering of pipeline state
;;   6. Bump allocator + heap snapshot management
;;
;; Compile:  wasm-tools print examples/pipeline-demo.wat -o examples/pipeline-demo.wasm
;; Validate: wasm-tools validate examples/pipeline-demo.wasm
;; Run:      node examples/pipeline-demo.test.mjs
;;
;; Memory layout:
;;   $BUF   0x2000000 — pipeline descriptor + pipes region (256 KB)
;;   $SCR   0x2040000 — scratch buffer (64 KB)
;;   $MUX   0x2050000 — mux config region (4 KB)
;;   $DATA  0x2051000 — data/cfg payloads (64 KB)
;;   $TREE  0x2060000 — UI component tree (128 KB)
;;   $CMD   0x2080000 — UI command output (128 KB)
;;   $UI_STR 0x20A0000 — UI string data (64 KB)
;;
;; ── Pipeline Architecture ──
;;
;; This demo builds TWO pipelines to show different data-flow patterns:
;;
;; Pipeline A: "Round-Robin Fan-Out"
;;   [Input Pipe] → [Mux Static: streams 0..2] → [Output Pipe]
;;   Creates 3 stream pipes, writes framed data through a static mux.
;;   Demonstrates: pipe_create, frame_write, mux_static_run
;;
;; Pipeline B: "Demux Route"
;;   [Input Pipe] → [Demux Static: routes by stream_id] → [Stream Pipes]
;;   Reads framed data, routes each frame to the correct stream pipe.
;;   Demonstrates: frame_read, demux_static_run, pipe routing
;;
;; Pipeline C: "Full Pipeline Run" (all stages chained)
;;   [Input] → [passthrough] → [mux_static] → [demux_static] → [Output]
;;   Uses pipeline_create/set_stage/run API.
;;   Demonstrates: pipeline lifecycle, batch fusion, intermediate pipes
;;
;; ── Batch Fusion ──
;; Consecutive stateless stages (state_ptr == NULL) share an intermediate pipe.
;; The stage reads from prev (draining it), writes to the same pipe as output.
;; This avoids allocating per-stage buffers for pure transform chains.
;;
;; ── Epoch Batching ──
;; Mux stages support epoch batching: set epoc_len on the state block to
;; accumulate data across N ticks before flushing. Reduces per-tick overhead
;; for low-throughput streams.
;; ─────────────────────────────────────────────────────

(module
  ;; ══════════════════════════════════════════════════════════════════════
  ;; Imports from "edgerun" (the compiled edgerun.wasm module)
  ;; ══════════════════════════════════════════════════════════════════════

  (import "edgerun" "memory" (memory 1))

  ;; ── Status codes (global constants) ──
  (import "edgerun" "STATUS_OK"    (global $OK    i32))
  (import "edgerun" "STATUS_MORE"  (global $MORE  i32))
  (import "edgerun" "STATUS_OVERFLOW" (global $OVERFLOW i32))

  ;; ── Pipe API ──
  (import "edgerun" "pipe_create"       (func $pipe_create       (param i32) (result i32)))
  (import "edgerun" "pipe_create_aligned" (func $pipe_create_align (param i32 i32) (result i32)))
  (import "edgerun" "pipe_write"        (func $pipe_write        (param i32 i32 i32) (result i32)))
  (import "edgerun" "pipe_read"         (func $pipe_read         (param i32 i32 i32) (result i32)))
  (import "edgerun" "pipe_available"    (func $pipe_available    (param i32) (result i32)))
  (import "edgerun" "pipe_alloc"        (func $pipe_alloc        (param i32) (result i32)))
  (import "edgerun" "pipe_close"        (func $pipe_close        (param i32)))
  (import "edgerun" "pipe_snapshot"     (func $pipe_snapshot     (result i32)))
  (import "edgerun" "pipe_restore"      (func $pipe_restore      (param i32)))

  ;; ── Pipeline API ──
  (import "edgerun" "pipeline_create"       (func $pl_create     (param i32 i32) (result i32)))
  (import "edgerun" "pipeline_set_stage"    (func $pl_set_stage  (param i32 i32 i32 i32 i32)))
  (import "edgerun" "pipeline_set_stage_state" (func $pl_set_state (param i32 i32 i32)))
  (import "edgerun" "pipeline_run"          (func $pl_run        (param i32 i32 i32 i32 i32) (result i32)))
  (import "edgerun" "pipeline_get_tick"     (func $pl_get_tick   (param i32) (result i32)))
  (import "edgerun" "pipeline_set_frame_size" (func $pl_set_frame (param i32 i32)))
  (import "edgerun" "STAGE_PASSTHROUGH"     (func $STAGE_PASS    (result i32)))
  (import "edgerun" "STAGE_MUX_STATIC"      (func $STAGE_MUX     (result i32)))
  (import "edgerun" "STAGE_DEMUX_STATIC"    (func $STAGE_DEMUX   (result i32)))

  ;; ── Frame API ──
  (import "edgerun" "frame_write" (func $frame_write (param i32 i32 i32 i32) (result i32)))
  (import "edgerun" "frame_read"  (func $frame_read  (param i32 i32 i32) (result i64)))
  (import "edgerun" "msg_write"   (func $msg_write   (param i32 i32 i32) (result i32)))
  (import "edgerun" "msg_read"    (func $msg_read    (param i32 i32 i32) (result i32)))

  ;; ── Mux API ──
  (import "edgerun" "mux_static_create" (func $mux_create (param i32) (result i32)))
  (import "edgerun" "mux_static_run"    (func $mux_run    (param i32 i32 i32) (result i32)))
  (import "edgerun" "demux_static_create" (func $demux_create (param i32) (result i32)))
  (import "edgerun" "demux_static_run"    (func $demux_run    (param i32 i32 i32) (result i32)))

  ;; ── Core helpers ──
  (import "edgerun" "pack"       (func $pack (param i32 i32) (result i64)))
  (import "edgerun" "min_u"      (func $min_u (param i32 i32) (result i32)))
  (import "edgerun" "memcpy"     (func $memcpy (param i32 i32 i32)))

  ;; ── UI Framework ──
  (import "edgerun" "er_ui_writer_begin"          (func $ui_begin    (param i32 i32 i32 i32 i32 i32 i32) (result i32)))
  (import "edgerun" "er_ui_writer_string"          (func $ui_string  (param i32 i32) (result i32)))
  (import "edgerun" "er_ui_writer_record"          (func $ui_record  (param i32 i32 i32 i32 i32) (result i32)))
  (import "edgerun" "er_ui_writer_record_child"    (func $ui_child   (param i32 i32 i32 i32 i32 i32) (result i32)))
  (import "edgerun" "er_ui_writer_cursor"          (func $ui_cursor  (result i32)))
  (import "edgerun" "er_ui_validate_deep"          (func $ui_validate (param i32 i32) (result i32)))
  (import "edgerun" "er_ui_render"                 (func $ui_render  (param i32 i32 i32 i32 f32 f32 f32 f32) (result i32)))
  (import "edgerun" "er_ui_wasm_new_card"          (func $ui_card    (param i32 i32 i32 i32 i32 i32 i32) (result i32)))
  (import "edgerun" "er_ui_wasm_new_row_item"      (func $ui_row     (param i32 i32 i32 i32 i32 i32 i32) (result i32)))
  (import "edgerun" "er_ui_wasm_new_badge"         (func $ui_badge   (param i32 i32 i32 i32 i32) (result i32)))
  (import "edgerun" "er_ui_wasm_new_separator"     (func $ui_sep     (param i32 i32) (result i32)))
  (import "edgerun" "er_ui_stack_axis_column"      (func $ui_col     (result i32)))
  (import "edgerun" "er_ui_stack_default_gap"      (func $ui_gap     (result i32)))
  (import "edgerun" "er_ui_stack_default_padding"  (func $ui_pad     (result i32)))

  ;; ══════════════════════════════════════════════════════════════════════
  ;; Memory Layout
  ;; ══════════════════════════════════════════════════════════════════════
  (global $BUF      i32 (i32.const 0x2000000))
  (global $BUF_CAP  i32 (i32.const 262144))
  (global $SCR      i32 (i32.const 0x2040000))
  (global $SCR_CAP  i32 (i32.const 65536))
  (global $MUX_CFG  i32 (i32.const 0x2050000))
  (global $DATA     i32 (i32.const 0x2051000))
  (global $TREE     i32 (i32.const 0x2060000))
  (global $TREE_CAP i32 (i32.const 131072))
  (global $CMD      i32 (i32.const 0x2080000))
  (global $CMD_CAP  i32 (i32.const 131072))
  (global $UI_STR   i32 (i32.const 0x20A0000))
  (global $UI_STR_CAP i32 (i32.const 65536))

  ;; ══════════════════════════════════════════════════════════════════════
  ;; String constants (UI labels, pipeline names)
  ;; ══════════════════════════════════════════════════════════════════════
  (data (i32.const 0x20A0000) "Pipeline Demo")
  (data (i32.const 0x20A0020) "Stage: Passthrough")
  (data (i32.const 0x20A0040) "Stage: Mux Static")
  (data (i32.const 0x20A0060) "Stage: Demux Static")
  (data (i32.const 0x20A0080) "Result: OK")
  (data (i32.const 0x20A00A0) "Result: MORE (yield)")
  (data (i32.const 0x20A00C0) "Result: ERROR")
  (data (i32.const 0x20A00E0) "Ticks:")
  (data (i32.const 0x20A0100) "Stream Pipe 0")
  (data (i32.const 0x20A0120) "Stream Pipe 1")
  (data (i32.const 0x20A0140) "Stream Pipe 2")
  (data (i32.const 0x20A0160) "Bytes Available:")
  (data (i32.const 0x20A0180) "Frames Sent:")
  (data (i32.const 0x20A01A0) "Frames Routed:")
  (data (i32.const 0x20A01C0) "Frame Pacer (32B)")
  (data (i32.const 0x20A01E0) "Pipeline Status")
  (data (i32.const 0x20A0200) "----------------")
  (data (i32.const 0x20A0220) "Running pipeline...")
  (data (i32.const 0x20A0240) "Pipeline complete")
  (data (i32.const 0x20A0260) "Buffer:")
  (data (i32.const 0x20A0280) "tick 0: mux(3 streams) wrote 3 frames")
  (data (i32.const 0x20A02C0) "tick 1: demux routed 3 frames")
  (data (i32.const 0x20A0300) "Use technique: batch fusion")
  (data (i32.const 0x20A0320) "Stream 0 data")
  (data (i32.const 0x20A0340) "Stream 1 data")
  (data (i32.const 0x20A0360) "Stream 2 data")
  (data (i32.const 0x20A0380) "tick 2: pipeline_run all stages")
  (data (i32.const 0x20A03C0) "0xF0F0F0F0")
  (data (i32.const 0x20A03E0) "Use technique: heap snapshot")
  (data (i32.const 0x20A0400) "Use technique: frame alignment")
  (data (i32.const 0x20A0420) "Use technique: zero-copy read")

  ;; ══════════════════════════════════════════════════════════════════════
  ;; Main entry point
  ;; ══════════════════════════════════════════════════════════════════════
  ;; build_pipeline_a: Round-robin mux demo (manual, no pipeline_run)
  ;; Writes data to 3 stream pipes, then muxes them to a single output.
  (func $build_pipeline_a (export "build_pipeline_a") (result i32)
    (local $s0 i32) (local $s1 i32) (local $s2 i32)
    (local $out i32) (local $mux_cfg i32) (local $mux i32)
    (local $n i32)

    ;; Create 3 stream pipes (256 bytes each)
    (local.set $s0 (call $pipe_create (i32.const 256)))
    (local.set $s1 (call $pipe_create (i32.const 256)))
    (local.set $s2 (call $pipe_create (i32.const 256)))

    ;; Create output pipe (1024 bytes)
    (local.set $out (call $pipe_create (i32.const 1024)))

    ;; Write test data to each stream
    ;; Each frame: [stream_id:4][len:4][payload]
    (call $frame_write (local.get $s0) (i32.const 0) (global.get $DATA) (i32.const 8))
    (call $frame_write (local.get $s1) (i32.const 1) (global.get $DATA) (i32.const 8))
    (call $frame_write (local.get $s2) (i32.const 2) (global.get $DATA) (i32.const 8))

    ;; Build mux config: [output_pipe, count, pipe_0, pipe_1, pipe_2]
    (i32.store offset=0 (global.get $MUX_CFG) (local.get $out))
    (i32.store offset=4 (global.get $MUX_CFG) (i32.const 3))
    (i32.store offset=8 (global.get $MUX_CFG) (local.get $s0))
    (i32.store offset=12 (global.get $MUX_CFG) (local.get $s1))
    (i32.store offset=16 (global.get $MUX_CFG) (local.get $s2))

    ;; Create mux and run it (round-robin drain)
    (local.set $mux (call $mux_create (global.get $MUX_CFG)))
    (local.set $n (call $mux_run (local.get $mux) (global.get $SCR) (global.get $SCR_CAP)))

    ;; Return frames written
    local.get $n
  )

  ;; build_pipeline_b: Demux route demo (manual, no pipeline_run)
  ;; Takes a framed input pipe, routes each frame to the correct stream.
  (func $build_pipeline_b (export "build_pipeline_b") (result i32)
    (local $in i32) (local $s0 i32) (local $s1 i32) (local $s2 i32)
    (local $demux_cfg i32) (local $demux i32)
    (local $n i32)

    ;; Create input pipe
    (local.set $in (call $pipe_create (i32.const 1024)))

    ;; Create 3 output stream pipes
    (local.set $s0 (call $pipe_create (i32.const 256)))
    (local.set $s1 (call $pipe_create (i32.const 256)))
    (local.set $s2 (call $pipe_create (i32.const 256)))

    ;; Write framed data to input
    (call $frame_write (local.get $in) (i32.const 0) (i32.const 0x20A0320) (i32.const 12))
    (call $frame_write (local.get $in) (i32.const 1) (i32.const 0x20A0340) (i32.const 12))
    (call $frame_write (local.get $in) (i32.const 2) (i32.const 0x20A0360) (i32.const 12))

    ;; Build demux config: [input_pipe, count, pipe_0, pipe_1, pipe_2]
    (i32.store offset=0 (global.get $MUX_CFG) (local.get $in))
    (i32.store offset=4 (global.get $MUX_CFG) (i32.const 3))
    (i32.store offset=8 (global.get $MUX_CFG) (local.get $s0))
    (i32.store offset=12 (global.get $MUX_CFG) (local.get $s1))
    (i32.store offset=16 (global.get $MUX_CFG) (local.get $s2))

    ;; Create demux and run it (route all frames)
    (local.set $demux (call $demux_create (global.get $MUX_CFG)))
    (block $loop
      (loop $continue
        (local.set $n (call $demux_run (local.get $demux) (global.get $SCR) (global.get $SCR_CAP)))
        (br_if $loop (i32.gt_s (local.get $n) (i32.const 0)))
        (br $continue)))
    local.get $n
  )

  ;; build_pipeline_c: Full pipeline_run with descriptor
  ;; Uses pipeline_create, pipeline_set_stage, pipeline_run.
  ;; Stages: [passthrough] → [mux_static] → [demux_static]
  ;; This demonstrates the complete pipeline lifecycle.
  (func $build_pipeline_c (export "build_pipeline_c") (result i32)
    (local $desc i32) (local $input i32) (local $output i32)
    (local $s0 i32) (local $s1 i32) (local $mux_cfg i32)
    (local $demux_cfg i32) (local $result i32)
    (local $snap i32)

    ;; ── Stage 0 setup: passthrough will pipe input directly ──

    ;; ── Stage 1 setup: mux_static needs stream pipes ──
    ;; We'll need: input → passthrough → mux → demux → output
    ;; The mux reads from pipes in its config, NOT from the pipeline prev pipe.
    ;; So we create stream pipes, write data to them, and the mux drains them.

    ;; Save heap snapshot for cleanup
    (local.set $snap (call $pipe_snapshot))

    ;; Create input pipe (written by caller)
    (local.set $input (call $pipe_create (i32.const 512)))

    ;; Create output pipe (read by caller after run)
    (local.set $output (call $pipe_create (i32.const 512)))

    ;; Create stream pipes (for mux to drain)
    (local.set $s0 (call $pipe_create (i32.const 256)))
    (local.set $s1 (call $pipe_create (i32.const 256)))

    ;; Write data to stream pipes
    (call $frame_write (local.get $s0) (i32.const 0) (global.get $DATA) (i32.const 8))
    (call $frame_write (local.get $s1) (i32.const 1) (global.get $DATA) (i32.const 8))

    ;; Build mux config: [output=intermediate, count=2, pipe_0, pipe_1]
    ;; The mux writes framed data to the intermediate pipe.
    ;; Demux then reads that intermediate pipe and routes frames.
    (local.set $mux_cfg (call $pipe_alloc (i32.const 16)))
    (i32.store offset=0 (local.get $mux_cfg) (i32.const 0))  ;; output: placeholder
    (i32.store offset=4 (local.get $mux_cfg) (i32.const 2))  ;; count
    (i32.store offset=8 (local.get $mux_cfg) (local.get $s0))
    (i32.store offset=12 (local.get $mux_cfg) (local.get $s1))

    ;; Build demux config: [input=intermediate, count=2, pipe_0, pipe_1]
    (local.set $demux_cfg (call $pipe_alloc (i32.const 16)))
    (i32.store offset=0 (local.get $demux_cfg) (i32.const 0))  ;; input: placeholder
    (i32.store offset=4 (local.get $demux_cfg) (i32.const 2))  ;; count
    (i32.store offset=8 (local.get $demux_cfg) (call $pipe_create (i32.const 256)))
    (i32.store offset=12 (local.get $demux_cfg) (call $pipe_create (i32.const 256)))

    ;; Create pipeline descriptor (3 stages, 1024-byte intermediate pipes)
    (local.set $desc (call $pl_create (i32.const 1024) (i32.const 3)))

    ;; Stage 0: passthrough (no config, no state — batch, fused)
    (call $pl_set_stage (local.get $desc) (i32.const 0)
      (call $STAGE_PASS) (i32.const 0) (i32.const 0))

    ;; Stage 1: mux_static (config = mux config, no state — batch, fused)
    (call $pl_set_stage (local.get $desc) (i32.const 1)
      (call $STAGE_MUX) (local.get $mux_cfg) (i32.const 12))

    ;; Stage 2: demux_static (config = demux config, no state — batch, fused)
    (call $pl_set_stage (local.get $desc) (i32.const 2)
      (call $STAGE_DEMUX) (local.get $demux_cfg) (i32.const 12))

    ;; Set aligned frame size for SIMD-friendly intermediate pipes
    (call $pl_set_frame (local.get $desc) (i32.const 64))

    ;; Run pipeline: passthrough (fused) → mux_static (fused) → demux_static
    (local.set $result
      (call $pl_run (local.get $desc) (local.get $input) (local.get $output)
        (global.get $SCR) (global.get $SCR_CAP)))

    ;; Cleanup intermediate pipes
    (call $pipe_restore (local.get $snap))

    local.get $result
  )

  ;; ── Pipeline D: Full streaming pipeline with frame pacer ──
  ;; Only available when using edgerun-full.wasm (has process_frame_pacer).
  ;; This shows the streaming yield pattern with STATE.
  (func $build_pipeline_d (export "build_pipeline_d") (result i32)
    ;; STAGE_FRAME_PACER = 15, requires edgerun-full.wat build
    ;; Config: [timeout_ticks: i32] — flush partial frame after N idle ticks
    ;; State:  80-byte block (tick + frame_size + buf_len + last_flush + buf[64])

    (local $desc i32) (local $input i32) (local $output i32)
    (local $state i32) (local $cfg i32) (local $result i32)
    (local $snap i32)

    (local.set $snap (call $pipe_snapshot))

    ;; Allocate 80-byte state block for frame pacer
    (local.set $state (call $pipe_alloc (i32.const 80)))
    (local.set $cfg (call $pipe_alloc (i32.const 4)))

    ;; Config: timeout = 0 ticks (never auto-flush partial)
    (i32.store (local.get $cfg) (i32.const 0))

    ;; Create pipes
    (local.set $input (call $pipe_create (i32.const 256)))
    (local.set $output (call $pipe_create (i32.const 256)))

    ;; Write partial frame (12 bytes, frame_size=32 → not enough yet)
    (call $pipe_write (local.get $input) (global.get $DATA) (i32.const 12))

    ;; Create single-stage pipeline with frame pacer
    (local.set $desc (call $pl_create (i32.const 256) (i32.const 1)))
    (call $pl_set_stage (local.get $desc) (i32.const 0)
      (i32.const 15) (local.get $cfg) (i32.const 4))
    (call $pl_set_state (local.get $desc) (i32.const 0) (local.get $state))

    ;; First run: should yield MORE (partial frame pending, timeout=0)
    (local.set $result
      (call $pl_run (local.get $desc) (local.get $input) (local.get $output)
        (global.get $SCR) (global.get $SCR_CAP)))

    ;; Add more data to complete a frame
    (call $pipe_write (local.get $input) (global.get $DATA) (i32.const 20))

    ;; Second run: should emit 32-byte frame (12+20=32) → OK
    (local.set $result
      (call $pl_run (local.get $desc) (local.get $input) (local.get $output)
        (global.get $SCR) (global.get $SCR_CAP)))

    (call $pipe_restore (local.get $snap))
    local.get $result
  )

  ;; ── Frame messaging primitive demo ──
  ;; Shows how to use frame_write/frame_read for message-oriented I/O.
  (func $build_frame_demo (export "build_frame_demo") (result i32)
    (local $pipe i32) (local $result i64)
    (local $status i32) (local $stream_id i32) (local $len i32)

    (local.set $pipe (call $pipe_create (i32.const 512)))

    ;; Write frame: stream_id=42, payload="Hello"
    (call $frame_write (local.get $pipe) (i32.const 42) (global.get $DATA) (i32.const 5))

    ;; Read frame back
    (local.set $result (call $frame_read (local.get $pipe) (global.get $SCR) (global.get $SCR_CAP)))
    (local.set $status (i32.wrap_i64 (i64.shr_u (local.get $result) (i64.const 32))))
    (local.set $stream_id (i32.wrap_i64 (local.get $result)))

    ;; Verify: status=OK (0), stream_id=42
    ;; payload_len at scratch[0], payload at scratch[4..]
    (local.set $len (i32.load (global.get $SCR)))

    ;; Return stream_id (should be 42) on success, -1 on error
    (if (i32.eq (local.get $status) (global.get $OK))
      (then (return (local.get $stream_id)))
      (else (return (i32.sub (i32.const 0) (local.get $status)))))
  )

  ;; ── Message queue demo ──
  ;; Shows msg_write/msg_read for simple length-prefixed messages.
  (func $build_msg_demo (export "build_msg_demo") (result i32)
    (local $pipe i32) (local $len i32)

    (local.set $pipe (call $pipe_create (i32.const 256)))

    ;; Write message
    (call $msg_write (local.get $pipe) (global.get $DATA) (i32.const 8))

    ;; Read message back
    (local.set $len (call $msg_read (local.get $pipe) (global.get $SCR) (global.get $SCR_CAP)))

    ;; Return payload length (should be 8)
    local.get $len
  )

  ;; ══════════════════════════════════════════════════════════════════════
  ;; UI rendering: visualize pipeline state
  ;; ══════════════════════════════════════════════════════════════════════
  ;; Builds a UI component tree showing pipeline configuration and status.
  ;; Uses the same pattern as examples/ui-card-demo.wat.
  (func $render_pipeline_ui (export "render_pipeline_ui") (result i32)
    (local $tree_len i32) (local $ok i32) (local $cmd_count i32)
    (local $title_ref i32) (local $detail_ref i32)
    (local $ref0 i32) (local $ref1 i32)

    ;; Store title and detail strings
    (global.get $UI_STR) (i32.const 12) call $ui_string local.set $title_ref
    (global.get $UI_STR) (i32.const 0x20) i32.add (i32.const 17) call $ui_string local.set $detail_ref

    ;; Build a card component showing pipeline status
    global.get $TREE
    global.get $TREE_CAP
    global.get $UI_STR
    i32.const 15
    global.get $UI_STR
    i32.const 0x1E0 i32.add
    i32.const 17
    i32.const 0
    call $ui_card
    local.set $tree_len
    local.get $tree_len i32.eqz if i32.const -1 return end

    ;; Validate
    global.get $TREE local.get $tree_len call $ui_validate local.set $ok
    local.get $ok i32.eqz if i32.const -2 return end

    ;; Render to command buffer
    global.get $TREE local.get $tree_len
    global.get $CMD global.get $CMD_CAP
    f32.const 0 f32.const 0 f32.const 400 f32.const 600
    call $ui_render
    local.set $cmd_count

    local.get $cmd_count
  )

  ;; ── Comprehensive pipeline visualization ──
  ;; Builds a detailed UI tree showing all pipeline stages and their state.
  (func $render_pipeline_detail (export "render_pipeline_detail") (param $tick i32) (param $result i32) (result i32)
    (local $cursor i32) (local $tree_len i32) (local $ok i32) (local $cmd_count i32)
    (local $r_title i32) (local $r_detail i32) (local $r_tick i32)
    (local $r_stage0 i32) (local $r_stage1 i32) (local $r_stage2 i32)
    (local $r_result i32) (local $r_tech1 i32) (local $r_tech2 i32)

    ;; Store string references
    global.get $UI_STR i32.const 12 call $ui_string  local.set $r_title
    global.get $UI_STR i32.const 0x20 i32.add i32.const 17 call $ui_string  local.set $r_detail
    global.get $UI_STR i32.const 0xE0 i32.add i32.const 6 call $ui_string  local.set $r_tick
    global.get $UI_STR i32.const 0x20 i32.add i32.const 17 call $ui_string  local.set $r_stage0
    global.get $UI_STR i32.const 0x40 i32.add i32.const 16 call $ui_string  local.set $r_stage1
    global.get $UI_STR i32.const 0x60 i32.add i32.const 18 call $ui_string  local.set $r_stage2
    global.get $UI_STR i32.const 0x300 i32.add i32.const 27 call $ui_string  local.set $r_tech1
    global.get $UI_STR i32.const 0x3E0 i32.add i32.const 20 call $ui_string  local.set $r_tech2

    ;; Begin tree: 1 root, column layout
    global.get $TREE
    global.get $TREE_CAP
    i32.const 8
    i32.const 1
    call $ui_col
    call $ui_gap
    call $ui_pad
    call $ui_begin
    local.set $cursor
    local.get $cursor i32.eqz if i32.const -1 return end

    ;; Root: Pipeline Status card
    i32.const 0  i32.const 28  i32.const 100
    local.get $r_title local.get $r_detail
    call $ui_record
    i32.eqz if i32.const -3 return end

    ;; Child 1: Tick count (row item)
    global.get $UI_STR i32.const 0x380 i32.add i32.const 35 call $ui_string  local.set $r_stage0
    i32.const 1  i32.const 0  i32.const 26  i32.const 101
    local.get $r_tick local.get $r_stage0
    call $ui_child
    i32.eqz if i32.const -3 return end

    ;; Child 2: Result
    global.get $UI_STR i32.const 0x240 i32.add i32.const 20 call $ui_string  local.set $r_stage0
    i32.const 2  i32.const 0  i32.const 26  i32.const 102
    local.get $r_stage0 i32.const 0
    call $ui_child
    i32.eqz if i32.const -3 return end

    ;; Child 3: Technique — batch fusion
    i32.const 3  i32.const 0  i32.const 26  i32.const 103
    local.get $r_tech1 i32.const 0
    call $ui_child
    i32.eqz if i32.const -3 return end

    ;; Child 4: Technique — heap snapshot
    i32.const 4  i32.const 0  i32.const 26  i32.const 104
    local.get $r_tech2 i32.const 0
    call $ui_child
    i32.eqz if i32.const -3 return end

    call $ui_cursor local.set $tree_len

    global.get $TREE local.get $tree_len call $ui_validate local.set $ok
    local.get $ok i32.eqz if i32.const -4 return end

    global.get $TREE local.get $tree_len
    global.get $CMD global.get $CMD_CAP
    f32.const 0 f32.const 0 f32.const 400 f32.const 500
    call $ui_render
    local.set $cmd_count

    local.get $cmd_count
  )

  ;; ══════════════════════════════════════════════════════════════════════
  ;; Run all demos and return combined status
  ;; ══════════════════════════════════════════════════════════════════════
  (func $run_all_demos (export "run_all_demos") (result i32)
    (local $a i32) (local $b i32) (local $c i32) (local $d i32)
    (local $frame i32) (local $msg i32) (local $ui i32)
    (local $result i32)

    (local.set $a (call $build_pipeline_a))
    (local.set $b (call $build_pipeline_b))
    (local.set $c (call $build_pipeline_c))
    (local.set $d (call $build_pipeline_d))
    (local.set $frame (call $build_frame_demo))
    (local.set $msg (call $build_msg_demo))
    (local.set $ui (call $render_pipeline_detail
      (i32.const 3) (i32.const 0)))

    ;; Pack results: low 16 bits = OK count, high 16 bits = error code
    (local.set $result (i32.const 0))
    (if (i32.eq (local.get $a) (i32.const 3)) (then (local.set $result (i32.add (local.get $result) (i32.const 1)))))
    (if (i32.eq (local.get $b) (i32.const 3)) (then (local.set $result (i32.add (local.get $result) (i32.const 2)))))
    (if (i32.eq (local.get $c) (global.get $OK)) (then (local.set $result (i32.add (local.get $result) (i32.const 4)))))
    (if (i32.eq (local.get $d) (global.get $OK)) (then (local.set $result (i32.add (local.get $result) (i32.const 8)))))
    (if (i32.eq (local.get $frame) (i32.const 42)) (then (local.set $result (i32.add (local.get $result) (i32.const 16)))))
    (if (i32.eq (local.get $msg) (i32.const 8)) (then (local.set $result (i32.add (local.get $result) (i32.const 32)))))
    (if (i32.gt_s (local.get $ui) (i32.const 0)) (then (local.set $result (i32.add (local.get $result) (i32.const 64)))))

    local.get $result
  )
)
