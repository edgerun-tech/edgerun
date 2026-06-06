(module
  (import "edgerun-core" "memory" (memory 1))
  (import "edgerun-core" "pack" (func $pack (param i32 i32) (result i64)))
  (import "edgerun-core" "STATUS_OK" (global $OK i32))
  (import "edgerun-core" "STATUS_OVERFLOW" (global $OVERFLOW i32))
  (import "edgerun-core" "STATUS_INPUT_SHORT" (global $INPUT_SHORT i32))
  (import "frame-core" "frame_write" (func $frame_write (param i32 i32 i32 i32) (result i32)))
  (import "frame-core" "frame_read" (func $frame_read (param i32 i32 i32) (result i64)))
  (import "frame-core" "frame_route" (func $frame_route (param i32 i32 i32) (result i64)))
  (import "pipe-core" "pipe_alloc" (func $pipe_alloc (param i32) (result i32)))
  (import "pipe-core" "pipe_read" (func $pipe_read (param i32 i32 i32) (result i32)))
  (import "pipe-core" "pipe_write" (func $pipe_write (param i32 i32 i32) (result i32)))
  (import "pipe-core" "pipe_available" (func $pipe_available (param i32) (result i32)))

  (func (export "proto_standard_id") (result i32) i32.const 300104)

  ;; ════════════════════════════════════════════════════════════════
  ;; Static Mux — fixed array of stream pipes → framed output
  ;; Config blob: [output_pipe:i32][count:i32][pipe_0:i32][pipe_1:i32]...
  ;; Mux descriptor: [type:i32=0][output:i32][count:i32][pipe_0:i32]...
  ;; ════════════════════════════════════════════════════════════════

  (func (export "mux_static_create") (param $cfg i32) (result i32)
    (local $count i32) (local $sz i32) (local $mux i32)
    (local.set $count (i32.load offset=4 (local.get $cfg)))
    (local.set $sz (i32.add (i32.const 12) (i32.mul (local.get $count) (i32.const 4))))
    (local.set $mux (call $pipe_alloc (local.get $sz)))
    (if (i32.eq (local.get $mux) (i32.const -1)) (then (return (i32.const -1))))
    (i32.store offset=0 (local.get $mux) (i32.const 0))
    (i32.store offset=4 (local.get $mux) (i32.load (local.get $cfg)))
    (i32.store offset=8 (local.get $mux) (local.get $count))
    (call $memcpy (local.get $mux) (i32.const 12) (local.get $cfg) (i32.const 8)
      (i32.mul (local.get $count) (i32.const 4)))
    local.get $mux)

  ;; mux_static_run(mux, scratch, scap) → frames_written | error
  ;; Round-robin: read from each stream pipe, frame and write to output
  (func (export "mux_static_run")
    (param $mux i32) (param $scratch i32) (param $scap i32) (result i32)
    (local $output i32) (local $count i32) (local $i i32)
    (local $pipe i32) (local $avail i32) (local $r i32) (local $total i32)
    (local.set $output (i32.load offset=4 (local.get $mux)))
    (local.set $count (i32.load offset=8 (local.get $mux)))
    (block $done
      (loop $streams
        (br_if $done (i32.ge_u (local.get $i) (local.get $count)))
        (local.set $pipe (i32.load (i32.add (local.get $mux)
          (i32.add (i32.const 12) (i32.mul (local.get $i) (i32.const 4))))))
        (local.set $avail (call $pipe_available (local.get $pipe)))
        (if (i32.gt_u (local.get $avail) (i32.const 0))
          (then
            (if (i32.gt_u (local.get $avail) (local.get $scap))
              (then (local.set $avail (local.get $scap))))
            (local.set $r (call $pipe_read (local.get $pipe) (local.get $scratch) (local.get $avail)))
            (if (i32.lt_s (local.get $r) (i32.const 0))
              (then (local.set $total (local.get $r)) (br $done)))
            (local.set $r (call $frame_write (local.get $output) (local.get $i) (local.get $scratch) (local.get $r)))
            (if (i32.ne (local.get $r) (global.get $OK))
              (then (local.set $total (i32.sub (i32.const 0) (local.get $r))) (br $done)))
            (local.set $total (i32.add (local.get $total) (i32.const 1)))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $streams)))
    local.get $total)

  ;; ════════════════════════════════════════════════════════════════
  ;; Static Demux — read framed input, route to output by stream_id
  ;; Config blob: [input_pipe:i32][count:i32][pipe_0:i32][pipe_1:i32]...
  ;; Demux descriptor: [type:i32=2][input:i32][count:i32][pipe_0:i32]...
  ;; ════════════════════════════════════════════════════════════════

  (func (export "demux_static_create") (param $cfg i32) (result i32)
    (local $count i32) (local $sz i32) (local $demux i32)
    (local.set $count (i32.load offset=4 (local.get $cfg)))
    (local.set $sz (i32.add (i32.const 12) (i32.mul (local.get $count) (i32.const 4))))
    (local.set $demux (call $pipe_alloc (local.get $sz)))
    (if (i32.eq (local.get $demux) (i32.const -1)) (then (return (i32.const -1))))
    (i32.store offset=0 (local.get $demux) (i32.const 2))
    (i32.store offset=4 (local.get $demux) (i32.load (local.get $cfg)))
    (i32.store offset=8 (local.get $demux) (local.get $count))
    (call $memcpy (local.get $demux) (i32.const 12) (local.get $cfg) (i32.const 8)
      (i32.mul (local.get $count) (i32.const 4)))
    local.get $demux)

  ;; demux_static_run(demux, scratch, scap) → 1 (routed) | 0 (no frame) | error
  ;; Read one frame from input, route payload to output pipe[stream_id]
  (func (export "demux_static_run")
    (param $demux i32) (param $scratch i32) (param $scap i32) (result i32)
    (local $input i32) (local $count i32) (local $result i64)
    (local $status i32) (local $stream_id i32) (local $pipe i32)
    (local $avail i32)
    (local.set $input (i32.load offset=4 (local.get $demux)))
    (local.set $count (i32.load offset=8 (local.get $demux)))
    ;; Peek available bytes — need at least 8 for header
    (local.set $avail (call $pipe_available (local.get $input)))
    (if (i32.lt_u (local.get $avail) (i32.const 8))
      (then (return (i32.const 0))))
    ;; Read header bytes to get stream_id
    (local.set $result (call $frame_read (local.get $input) (local.get $scratch) (local.get $scap)))
    (local.set $status (i32.wrap_i64 (i64.shr_u (local.get $result) (i64.const 32))))
    (if (i32.ne (local.get $status) (global.get $OK))
      (then
        ;; OVERFLOW means scratch too small — skip the frame
        (if (i32.eq (local.get $status) (global.get $OVERFLOW))
          (then
            ;; frame_read already skipped the payload, so this frame is lost
            (return (i32.const 0))))
        (return (i32.sub (i32.const 0) (local.get $status)))))
    (local.set $stream_id (i32.wrap_i64 (local.get $result)))
    ;; Route
    (if (i32.ge_u (local.get $stream_id) (local.get $count))
      (then (return (i32.const -1))))
    (local.set $pipe (i32.load (i32.add (local.get $demux)
      (i32.add (i32.const 12) (i32.mul (local.get $stream_id) (i32.const 4))))))
    ;; payload_len at scratch[0..4], payload at scratch[4..]
    (local.set $avail (i32.load (local.get $scratch)))
    (drop (call $pipe_write (local.get $pipe) (i32.add (local.get $scratch) (i32.const 4)) (local.get $avail)))
    (return (i32.const 1)))

  ;; ════════════════════════════════════════════════════════════════
  ;; Dynamic Mux — linked list of (stream_id, pipe) → framed output
  ;; ════════════════════════════════════════════════════════════════

  ;; stream_node: +0 stream_id, +4 pipe, +8 next

  (func (export "mux_dynamic_create") (param $output i32) (result i32)
    (local $mux i32)
    (local.set $mux (call $pipe_alloc (i32.const 16)))
    (if (i32.eq (local.get $mux) (i32.const -1)) (then (return (i32.const -1))))
    (i32.store offset=0 (local.get $mux) (i32.const 1))
    (i32.store offset=4 (local.get $mux) (local.get $output))
    (i32.store offset=8 (local.get $mux) (i32.const 0))
    (i32.store offset=12 (local.get $mux) (i32.const 0))
    local.get $mux)

  (func (export "mux_add_stream") (param $mux i32) (param $stream_id i32) (param $pipe i32) (result i32)
    (local $node i32)
    (local.set $node (call $pipe_alloc (i32.const 12)))
    (if (i32.eq (local.get $node) (i32.const -1)) (then (return (global.get $OVERFLOW))))
    (i32.store offset=0 (local.get $node) (local.get $stream_id))
    (i32.store offset=4 (local.get $node) (local.get $pipe))
    (i32.store offset=8 (local.get $node) (i32.load offset=8 (local.get $mux)))
    (i32.store offset=8 (local.get $mux) (local.get $node))
    (i32.store offset=12 (local.get $mux) (i32.add (i32.load offset=12 (local.get $mux)) (i32.const 1)))
    global.get $OK)

  (func (export "mux_remove_stream") (param $mux i32) (param $stream_id i32) (result i32)
    (local $prev i32) (local $curr i32) (local $next i32)
    (local.set $curr (i32.load offset=8 (local.get $mux)))
    (block $found
      (loop $walk
        (br_if $found (i32.eqz (local.get $curr)))
        (if (i32.eq (i32.load (local.get $curr)) (local.get $stream_id))
          (then
            (local.set $next (i32.load offset=8 (local.get $curr)))
            (if (local.get $prev)
              (then (i32.store offset=8 (local.get $prev) (local.get $next)))
              (else (i32.store offset=8 (local.get $mux) (local.get $next))))
            (i32.store offset=12 (local.get $mux)
              (i32.sub (i32.load offset=12 (local.get $mux)) (i32.const 1)))
            (br $found)))
        (local.set $prev (local.get $curr))
        (local.set $curr (i32.load offset=8 (local.get $curr)))
        (br $walk)))
    global.get $OK)

  ;; mux_dynamic_run(mux, scratch, scap) → frames_written | error
  (func $mux_dynamic_run (export "mux_dynamic_run")
    (param $mux i32) (param $scratch i32) (param $scap i32) (result i32)
    (local $output i32) (local $curr i32) (local $stream_id i32)
    (local $pipe i32) (local $avail i32) (local $r i32) (local $total i32)
    (local.set $output (i32.load offset=4 (local.get $mux)))
    (local.set $curr (i32.load offset=8 (local.get $mux)))
    (block $done
      (loop $walk
        (br_if $done (i32.eqz (local.get $curr)))
        (local.set $stream_id (i32.load (local.get $curr)))
        (local.set $pipe (i32.load offset=4 (local.get $curr)))
        (local.set $avail (call $pipe_available (local.get $pipe)))
        (if (i32.gt_u (local.get $avail) (i32.const 0))
          (then
            (if (i32.gt_u (local.get $avail) (local.get $scap))
              (then (local.set $avail (local.get $scap))))
            (local.set $r (call $pipe_read (local.get $pipe) (local.get $scratch) (local.get $avail)))
            (if (i32.lt_s (local.get $r) (i32.const 0))
              (then (local.set $total (local.get $r)) (br $done)))
            (local.set $r (call $frame_write (local.get $output) (local.get $stream_id) (local.get $scratch) (local.get $r)))
            (if (i32.ne (local.get $r) (global.get $OK))
              (then (local.set $total (i32.sub (i32.const 0) (local.get $r))) (br $done)))
            (local.set $total (i32.add (local.get $total) (i32.const 1)))))
        (local.set $curr (i32.load offset=8 (local.get $curr)))
        (br $walk)))
    local.get $total)

  ;; ════════════════════════════════════════════════════════════════
  ;; Dynamic Demux — hash table: stream_id → output_pipe
  ;; Descriptor: [type:i32=3][input:i32][slot_count:i32][slot_0:stream_id,pipe]...
  ;; slot = 8 bytes, slot_count must be power of 2
  ;; stream_id=0 = empty slot
  ;; ════════════════════════════════════════════════════════════════

  (func (export "demux_dynamic_create") (param $input i32) (param $slot_count i32) (result i32)
    (local $demux i32) (local $sz i32)
    (local.set $sz (i32.add (i32.const 12) (i32.mul (local.get $slot_count) (i32.const 8))))
    (local.set $demux (call $pipe_alloc (local.get $sz)))
    (if (i32.eq (local.get $demux) (i32.const -1)) (then (return (i32.const -1))))
    (i32.store offset=0 (local.get $demux) (i32.const 3))
    (i32.store offset=4 (local.get $demux) (local.get $input))
    (i32.store offset=8 (local.get $demux) (local.get $slot_count))
    local.get $demux)

  ;; Linear probe: hash(stream_id) = stream_id & mask
  ;; Returns slot address or 0 if not found
  (func $demux_lookup (param $demux i32) (param $stream_id i32) (result i32)
    (local $slot_count i32) (local $mask i32) (local $base i32)
    (local $sid i32) (local $i i32)
    (if (i32.eqz (local.get $stream_id)) (then (return (i32.const 0))))
    (local.set $slot_count (i32.load offset=8 (local.get $demux)))
    (local.set $mask (i32.sub (local.get $slot_count) (i32.const 1)))
    (local.set $base (i32.add (local.get $demux) (i32.const 12)))
    (local.set $i (i32.and (local.get $stream_id) (local.get $mask)))
    (block $found
      (loop $probe
        (local.set $sid (i32.load (i32.add (local.get $base) (i32.mul (local.get $i) (i32.const 8)))))
        (if (i32.eq (local.get $sid) (local.get $stream_id))
          (then (return (i32.add (local.get $base) (i32.mul (local.get $i) (i32.const 8))))))
        (if (i32.eqz (local.get $sid))
          (then (return (i32.const 0))))
        (local.set $i (i32.and (i32.add (local.get $i) (i32.const 1)) (local.get $mask)))
        (br $probe)))
    (i32.const 0))

  (func (export "demux_register_stream")
    (param $demux i32) (param $stream_id i32) (param $pipe i32) (result i32)
    (local $slot_count i32) (local $mask i32) (local $base i32)
    (local $i i32) (local $sid i32)
    (if (i32.eqz (local.get $stream_id)) (then (return (global.get $OVERFLOW))))
    (local.set $slot_count (i32.load offset=8 (local.get $demux)))
    (local.set $mask (i32.sub (local.get $slot_count) (i32.const 1)))
    (local.set $base (i32.add (local.get $demux) (i32.const 12)))
    (local.set $i (i32.and (local.get $stream_id) (local.get $mask)))
    (block $done
      (loop $probe
        (local.set $sid (i32.load (i32.add (local.get $base) (i32.mul (local.get $i) (i32.const 8)))))
        (if (i32.eqz (local.get $sid))
          (then
            (i32.store (i32.add (local.get $base) (i32.mul (local.get $i) (i32.const 8))) (local.get $stream_id))
            (i32.store offset=4 (i32.add (local.get $base) (i32.mul (local.get $i) (i32.const 8))) (local.get $pipe))
            (br $done)))
        (if (i32.eq (local.get $sid) (local.get $stream_id))
          (then
            (i32.store offset=4 (i32.add (local.get $base) (i32.mul (local.get $i) (i32.const 8))) (local.get $pipe))
            (br $done)))
        (local.set $i (i32.and (i32.add (local.get $i) (i32.const 1)) (local.get $mask)))
        (br $probe)))
    global.get $OK)

  (func (export "demux_unregister_stream") (param $demux i32) (param $stream_id i32) (result i32)
    (local $slot i32)
    (local.set $slot (call $demux_lookup (local.get $demux) (local.get $stream_id)))
    (if (local.get $slot)
      (then
        (i32.store (local.get $slot) (i32.const 0))
        (i32.store offset=4 (local.get $slot) (i32.const 0))))
    global.get $OK)

  ;; demux_dynamic_run(demux, scratch, scap) → 1 (routed) | 0 (no frame) | error
  (func $demux_dynamic_run (export "demux_dynamic_run")
    (param $demux i32) (param $scratch i32) (param $scap i32) (result i32)
    (local $input i32) (local $result i64) (local $status i32)
    (local $stream_id i32) (local $slot i32) (local $pipe i32)
    (local $avail i32)
    (local.set $input (i32.load offset=4 (local.get $demux)))
    ;; Need at least 8 bytes
    (local.set $avail (call $pipe_available (local.get $input)))
    (if (i32.lt_u (local.get $avail) (i32.const 8))
      (then (return (i32.const 0))))
    (local.set $result (call $frame_read (local.get $input) (local.get $scratch) (local.get $scap)))
    (local.set $status (i32.wrap_i64 (i64.shr_u (local.get $result) (i64.const 32))))
    (if (i32.ne (local.get $status) (global.get $OK))
      (then
        (if (i32.eq (local.get $status) (global.get $OVERFLOW))
          (then (return (i32.const 0))))
        (return (i32.sub (i32.const 0) (local.get $status)))))
    (local.set $stream_id (i32.wrap_i64 (local.get $result)))
    (local.set $slot (call $demux_lookup (local.get $demux) (local.get $stream_id)))
    (if (i32.eqz (local.get $slot))
      (then (return (i32.const -1))))
    (local.set $pipe (i32.load offset=4 (local.get $slot)))
    (local.set $avail (i32.load (local.get $scratch)))
    (drop (call $pipe_write (local.get $pipe) (i32.add (local.get $scratch) (i32.const 4)) (local.get $avail)))
    (i32.const 1))

  ;; ════════════════════════════════════════════════════════════════
  ;; Pipeline stage process functions
  ;; Signature: (input, output, config, clen, scratch, scap) → result
  ;; ════════════════════════════════════════════════════════════════

  ;; process_mux_static — reads from stream pipes in config, frame-writes to output
  ;; Config: [count][pipe_0][pipe_1]...
  ;; State layout (12 bytes):
  ;;   +0: tick          i32 (RO, written by pipeline_run)
  ;;   +4: epoch_len     i32 (0 = no batching, drain every call)
  ;;   +8: last_flush_tick i32 (last tick when drain occurred)
  (func (export "process_mux_static")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $count i32) (local $i i32) (local $pipe i32)
    (local $avail i32) (local $r i32) (local $total i32)
    (local $tick i32) (local $epoch_len i32) (local $last_flush i32)
    (local.set $count (i32.load (local.get $cfg)))

    ;; Epoch batching: if state != 0, check if it's time to drain
    (if (local.get $state)
      (then
        (local.set $tick (i32.load (local.get $state)))
        (local.set $epoch_len (i32.load offset=4 (local.get $state)))
        (local.set $last_flush (i32.load offset=8 (local.get $state)))
        (if (i32.lt_u (i32.sub (local.get $tick) (local.get $last_flush)) (local.get $epoch_len))
          (then (return (i32.const 0))))))

    (block $done
      (loop $streams
        (br_if $done (i32.ge_u (local.get $i) (local.get $count)))
        (local.set $pipe (i32.load (i32.add (local.get $cfg) (i32.add (i32.const 4) (i32.mul (local.get $i) (i32.const 4))))))
        (local.set $avail (call $pipe_available (local.get $pipe)))
        (if (i32.gt_u (local.get $avail) (i32.const 0))
          (then
            (if (i32.gt_u (local.get $avail) (local.get $scap))
              (then (local.set $avail (local.get $scap))))
            (local.set $r (call $pipe_read (local.get $pipe) (local.get $scratch) (local.get $avail)))
            (if (i32.lt_s (local.get $r) (i32.const 0))
              (then (local.set $total (local.get $r)) (br $done)))
            (local.set $r (call $frame_write (local.get $output) (local.get $i) (local.get $scratch) (local.get $r)))
            (if (i32.ne (local.get $r) (global.get $OK))
              (then (local.set $total (i32.sub (i32.const 0) (local.get $r))) (br $done)))
            (local.set $total (i32.add (local.get $total) (i32.const 1)))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $streams)))

    ;; Update last_flush_tick after drain (only if state != 0)
    (if (local.get $state)
      (then (i32.store offset=8 (local.get $state) (i32.load (local.get $state)))))
    local.get $total)

  ;; Helper: configure epoch batching on a mux_static state block
  ;; State must be non-zero and point to a 12-byte block
  (func (export "mux_static_set_epoch")
    (param $state i32) (param $epoch_len i32) (result i32)
    (if (i32.eqz (local.get $state))
      (then (return (i32.const -1))))
    (i32.store offset=4 (local.get $state) (local.get $epoch_len))
    (i32.store offset=8 (local.get $state) (i32.const 0))
    (i32.const 0))

  ;; process_demux_static — reads framed from input, routes to stream pipes in config
  ;; Config: [count][pipe_0][pipe_1]...
  ;; Drains all available frames in one call
  (func (export "process_demux_static")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $count i32) (local $avail i32) (local $result i64)
    (local $status i32) (local $stream_id i32) (local $pipe i32)
    (local $total i32)
    (local.set $count (i32.load (local.get $cfg)))
    (block $done
      (loop $frames
        (local.set $avail (call $pipe_available (local.get $input)))
        (br_if $done (i32.lt_u (local.get $avail) (i32.const 8)))
        (local.set $result (call $frame_read (local.get $input) (local.get $scratch) (local.get $scap)))
        (local.set $status (i32.wrap_i64 (i64.shr_u (local.get $result) (i64.const 32))))
        (if (i32.ne (local.get $status) (global.get $OK))
          (then
            (if (i32.eq (local.get $status) (global.get $OVERFLOW))
              (then (br $done)))
            (local.set $total (local.get $status)) (br $done)))
        (local.set $stream_id (i32.wrap_i64 (local.get $result)))
        (if (i32.ge_u (local.get $stream_id) (local.get $count))
          (then (local.set $total (i32.const -1)) (br $done)))
        (local.set $pipe (i32.load (i32.add (local.get $cfg) (i32.add (i32.const 4) (i32.mul (local.get $stream_id) (i32.const 4))))))
        (local.set $avail (i32.load (local.get $scratch)))
        (drop (call $pipe_write (local.get $pipe) (i32.add (local.get $scratch) (i32.const 4)) (local.get $avail)))
        (local.set $total (i32.add (local.get $total) (i32.const 1)))
        (br $frames)))
    (if (i32.lt_s (local.get $total) (i32.const 0))
      (then (return (local.get $total))))
    (global.get $OK))

  ;; process_mux_dynamic — calls mux_dynamic_run on pre-created mux handle
  ;; Config: [mux_handle:i32]
  (func (export "process_mux_dynamic")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $mux i32)
    (local.set $mux (i32.load (local.get $cfg)))
    (call $mux_dynamic_run (local.get $mux) (local.get $scratch) (local.get $scap)))

  ;; process_demux_dynamic — calls demux_dynamic_run on pre-created demux handle
  ;; Config: [demux_handle:i32]
  (func (export "process_demux_dynamic")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $demux i32)
    (local.set $demux (i32.load (local.get $cfg)))
    (call $demux_dynamic_run (local.get $demux) (local.get $scratch) (local.get $scap)))

  ;; ── memcpy ──
  (func $memcpy (param $dst i32) (param $doff i32) (param $src i32) (param $soff i32) (param $len i32)
    (local $i i32)
    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $i) (local.get $len)))
        (i32.store8
          (i32.add (i32.add (local.get $dst) (local.get $doff)) (local.get $i))
          (i32.load8_u
            (i32.add (i32.add (local.get $src) (local.get $soff)) (local.get $i))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop))))
)
