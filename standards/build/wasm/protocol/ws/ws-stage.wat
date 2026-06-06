(module
  (import "edgerun-core" "memory" (memory 1))
  (import "edgerun-core" "STATUS_OK" (global $OK i32))
  (import "edgerun-core" "STATUS_MORE" (global $MORE i32))
  (import "edgerun-core" "STATUS_TIMEOUT" (global $TIMEOUT i32))
  (import "edgerun-core" "pack" (func $pack (param i32 i32) (result i64)))

  (import "pipe-core" "pipe_read" (func $pipe_read (param i32 i32 i32) (result i32)))
  (import "pipe-core" "pipe_write" (func $pipe_write (param i32 i32 i32) (result i32)))
  (import "pipe-core" "pipe_available" (func $pipe_available (param i32) (result i32)))

  (import "ws-frame" "ws_parse_header" (func $ws_parse_header (param i32 i32 i32 i32) (result i32)))
  (import "ws-frame" "ws_apply_mask_in_place" (func $ws_apply_mask (param i32 i32 i32) (result i32)))
  (import "ws-frame" "ws_write_server_frame_header" (func $ws_write_srv_hdr (param i32 i32 i32 i32 i32) (result i64)))
  (import "ws-frame" "ws_write_frame_header" (func $ws_write_hdr (param i32 i32 i32 i32 i32 i32 i32 i32) (result i64)))

  (func (export "proto_standard_id") (result i32) i32.const 300508)

  ;; ─────────────────────────────────────────────────────────────
  ;; WebSocket frame pipeline stage — bidirectional WS framing
  ;;
  ;; Config: pointer to a socket handle (4 bytes i32)
  ;;   Socket +0 = send_pipe, +4 = recv_pipe
  ;;
  ;; State layout (148 bytes):
  ;;   +0:   tick          i32 (RO, written by pipeline_run)
  ;;   +4:   phase         i32 (0=idle, 1=awaiting)
  ;;   +8:   start_tick    i32
  ;;   +12:  timeout_ticks i32
  ;;   +16:  dbuf_len      i32 (decode buffer fill)
  ;;   +20:  dbuf[128]     decode buffer (partial frame data)
  ;;
  ;; Encode: reads payload from input pipe, wraps in WS binary frame,
  ;;         writes to socket send pipe.
  ;; Decode: reads raw bytes from socket recv pipe, parses WS frames,
  ;;         writes payload to output pipe.
  ;; Control frames: auto-respond Pong to Ping, Close→Close echo.
  ;; ─────────────────────────────────────────────────────────────

  (func (export "process_ws_frame")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $sock i32) (local $send_pipe i32) (local $recv_pipe i32)
    (local $phase i32) (local $elapsed i32) (local $sent i32)
    (local $n i32) (local $result i64) (local $status i32)

    (if (i32.lt_u (local.get $clen) (i32.const 4))
      (then (return (i32.const -1))))
    (local.set $sock (i32.load (local.get $cfg)))
    (local.set $send_pipe (i32.load (local.get $sock)))
    (local.set $recv_pipe (i32.load offset=4 (local.get $sock)))
    (local.set $phase (i32.load offset=4 (local.get $state)))

    ;; ── Phase 1 (awaiting response) ──
    (if (i32.eq (local.get $phase) (i32.const 1))
      (then
        (local.set $elapsed
          (i32.sub (i32.load (local.get $state)) (i32.load offset=8 (local.get $state))))
        (if (i32.load offset=12 (local.get $state))
          (then
            (if (i32.ge_u (local.get $elapsed) (i32.load offset=12 (local.get $state)))
              (then (return (global.get $TIMEOUT))))))

        (local.set $n (call $decode_frame
          (local.get $recv_pipe) (local.get $scratch) (local.get $scap)
          (local.get $state) (local.get $output) (local.get $send_pipe)))
        (if (i32.gt_s (local.get $n) (i32.const 0))
          (then
            (if (i32.ne (local.get $n) (global.get $MORE))
              (then
                (i32.store offset=4 (local.get $state) (i32.const 0))
                (return (local.get $n))))))
        (if (i32.lt_s (local.get $n) (i32.const 0))
          (then (return (local.get $n))))
        (return (global.get $MORE))))

    ;; ── Phase 0 (idle): encode + try decode ──

    ;; Encode: read payload from input, wrap in WS binary frame, send
    (local.set $n (call $pipe_read (local.get $input)
      (i32.add (local.get $scratch) (i32.const 16))
      (local.get $scap)))
    (if (i32.gt_s (local.get $n) (i32.const 0))
      (then
        (local.set $sent (local.get $n))
        (local.set $result (call $ws_write_srv_hdr
          (i32.const 2) (local.get $n) (i32.const 0)
          (local.get $scratch) (i32.const 10)))
        (local.set $status (i32.wrap_i64 (local.get $result)))
        (if (i32.eqz (local.get $status))
          (then
            (local.set $n (i32.wrap_i64 (i64.shr_u (local.get $result) (i64.const 32))))
            (drop (call $pipe_write (local.get $send_pipe) (local.get $scratch) (local.get $n)))
            (drop (call $pipe_write (local.get $send_pipe)
              (i32.add (local.get $scratch) (i32.const 16)) (local.get $sent)))))))

    ;; Decode: try to read and parse a WS frame from recv pipe
    (local.set $n (call $decode_frame
      (local.get $recv_pipe) (local.get $scratch) (local.get $scap)
      (local.get $state) (local.get $output) (local.get $send_pipe)))
    (if (i32.gt_s (local.get $n) (i32.const 0))
      (then
        (if (i32.ne (local.get $n) (global.get $MORE))
          (then (return (local.get $n))))))
    (if (i32.lt_s (local.get $n) (i32.const 0))
      (then (return (local.get $n))))

    ;; Decode returned MORE (need more data)
    (if (local.get $sent)
      (then
        ;; We sent data — enter phase 1 (awaiting) with timeout tracking
        (i32.store offset=4 (local.get $state) (i32.const 1))
        (i32.store offset=8 (local.get $state) (i32.load (local.get $state)))))
    (global.get $MORE))

  ;; ─────────────────────────────────────────────────────────────
  ;; $decode_frame — read from recv pipe, parse one WS frame
  ;;
  ;; Returns: >0 = payload bytes written to output,
  ;;           0 = no new data available (but need more bytes)
  ;;          $MORE = incomplete frame, buffered for next call
  ;;          <0 = protocol error
  ;;
  ;; Control frames (ping/pong/close) are handled internally:
  ;; - Ping → echoes Pong with same payload to send_pipe
  ;; - Close → echoes Close frame to send_pipe
  ;; - Pong → silently consumed
  ;; ─────────────────────────────────────────────────────────────

  (func $decode_frame
    (param $recv i32) (param $scratch i32) (param $scap i32)
    (param $state i32) (param $output i32) (param $send_pipe i32) (result i32)
    (local $dbuf_len i32) (local $n i32) (local $total i32) (local $r i32)
    (local $hdr_out i32)
    (local $fin i32) (local $opcode i32) (local $masked i32)
    (local $hdr_len i32) (local $payload_len i32) (local $mask_key i32)
    (local $frame_size i32) (local $off i32)
    (local $result i64) (local $status i32) (local $written i32)

    (local.set $dbuf_len (i32.load offset=16 (local.get $state)))

    ;; Copy dbuf to scratch if present
    (if (i32.gt_u (local.get $dbuf_len) (i32.const 0))
      (then
        (call $memcpy (local.get $scratch)
          (i32.add (local.get $state) (i32.const 20))
          (local.get $dbuf_len))))

    ;; Read available bytes from recv pipe
    (local.set $n (call $pipe_read (local.get $recv)
      (i32.add (local.get $scratch) (local.get $dbuf_len))
      (i32.sub (local.get $scap) (local.get $dbuf_len))))
    (if (i32.lt_s (local.get $n) (i32.const 0))
      (then (local.set $n (i32.const 0))))
    (local.set $total (i32.add (local.get $dbuf_len) (local.get $n)))

    ;; Need at least 2 bytes for basic WS header
    (if (i32.lt_u (local.get $total) (i32.const 2))
      (then
        (i32.store offset=16 (local.get $state) (local.get $total))
        (return (global.get $MORE))))

    ;; Parse header — use scratch[scap-32..scap-1] as hdr_out
    (local.set $hdr_out (i32.sub (local.get $scap) (i32.const 32)))
    (local.set $r (call $ws_parse_header
      (local.get $scratch) (local.get $total) (local.get $scap) (local.get $hdr_out)))

    ;; r=1: incomplete header, need more bytes
    (if (i32.eq (local.get $r) (i32.const 1))
      (then
        (call $memcpy (i32.add (local.get $state) (i32.const 20))
          (local.get $scratch) (local.get $total))
        (i32.store offset=16 (local.get $state) (local.get $total))
        (return (global.get $MORE))))

    ;; r=3: protocol error
    (if (i32.eq (local.get $r) (i32.const 3))
      (then (return (i32.const -3))))
    ;; r=4: payload too large for max_len
    (if (i32.eq (local.get $r) (i32.const 4))
      (then (return (i32.const -4))))

    ;; Parse header fields
    (local.set $fin    (i32.load offset=0 (local.get $hdr_out)))
    (local.set $opcode (i32.load offset=8 (local.get $hdr_out)))
    (local.set $masked (i32.load offset=12 (local.get $hdr_out)))
    (local.set $payload_len (i32.load offset=16 (local.get $hdr_out)))
    (local.set $hdr_len (i32.load offset=24 (local.get $hdr_out)))
    (local.set $mask_key (i32.load offset=28 (local.get $hdr_out)))

    ;; Check complete frame available
    (local.set $frame_size (i32.add (local.get $hdr_len) (local.get $payload_len)))
    (if (i32.gt_u (local.get $frame_size) (local.get $total))
      (then
        (call $memcpy (i32.add (local.get $state) (i32.const 20))
          (local.get $scratch) (local.get $total))
        (i32.store offset=16 (local.get $state) (local.get $total))
        (return (global.get $MORE))))

    ;; Complete frame — payload starts at scratch[hdr_len]
    (local.set $off (local.get $hdr_len))

    ;; Unmask if needed (server receives masked frames from client)
    (if (local.get $masked)
      (then
        (drop (call $ws_apply_mask
          (i32.add (local.get $scratch) (local.get $off))
          (local.get $payload_len) (local.get $mask_key)))))

    ;; Dispatch by opcode
    (block $ctrl
      ;; Text (1) / Binary (2) — write payload to output pipe
      (if (i32.and (i32.ge_u (local.get $opcode) (i32.const 1))
                   (i32.le_u (local.get $opcode) (i32.const 2)))
        (then
          (drop (call $pipe_write (local.get $output)
            (i32.add (local.get $scratch) (local.get $off))
            (local.get $payload_len)))
          (br $ctrl)))

      ;; Continuation (0) — same as text/binary, write to output
      (if (i32.eq (local.get $opcode) (i32.const 0))
        (then
          (drop (call $pipe_write (local.get $output)
            (i32.add (local.get $scratch) (local.get $off))
            (local.get $payload_len)))
          (br $ctrl)))

      ;; Ping (9) — echo payload back as Pong
      (if (i32.eq (local.get $opcode) (i32.const 9))
        (then
          (local.set $result (call $ws_write_hdr
            (i32.const 10) (i32.const 128)
            (local.get $payload_len) (i32.const 0)
            (i32.const 0) (i32.const 0)
            (local.get $scratch) (i32.const 10)))
          (local.set $status (i32.wrap_i64 (local.get $result)))
          (if (i32.eqz (local.get $status))
            (then
              (local.set $written (i32.wrap_i64 (i64.shr_u (local.get $result) (i64.const 32))))
              ;; Write pong header + echo payload directly (no overlap: header < off)
              (drop (call $pipe_write (local.get $send_pipe) (local.get $scratch) (local.get $written)))
              (drop (call $pipe_write (local.get $send_pipe)
                (i32.add (local.get $scratch) (local.get $off))
                (local.get $payload_len)))))
          (br $ctrl)))

      ;; Close (8) — echo close frame back
      (if (i32.eq (local.get $opcode) (i32.const 8))
        (then
          (local.set $result (call $ws_write_hdr
            (i32.const 8) (i32.const 128)
            (local.get $payload_len) (i32.const 0)
            (i32.const 0) (i32.const 0)
            (local.get $scratch) (i32.const 10)))
          (local.set $status (i32.wrap_i64 (local.get $result)))
          (if (i32.eqz (local.get $status))
            (then
              (local.set $written (i32.wrap_i64 (i64.shr_u (local.get $result) (i64.const 32))))
              (drop (call $pipe_write (local.get $send_pipe) (local.get $scratch) (local.get $written)))
              (drop (call $pipe_write (local.get $send_pipe)
                (i32.add (local.get $scratch) (local.get $off))
                (local.get $payload_len)))))
          (br $ctrl)))

      ;; Pong (10) or unknown — silently consume
    )
    ;; end block

    ;; Save overflow data back to dbuf
    (local.set $off (i32.add (local.get $off) (local.get $payload_len)))
    (local.set $n (i32.sub (local.get $total) (local.get $off)))
    (if (i32.gt_u (local.get $n) (i32.const 0))
      (then
        (call $memcpy (i32.add (local.get $state) (i32.const 20))
          (i32.add (local.get $scratch) (local.get $off)) (local.get $n))
        (i32.store offset=16 (local.get $state) (local.get $n)))
      (else
        (i32.store offset=16 (local.get $state) (i32.const 0))))

    ;; Return payload length for data frames, 1 for handled control frames
    (if (i32.le_u (local.get $opcode) (i32.const 2))
      (then (return (local.get $payload_len))))
    (if (i32.eq (local.get $opcode) (i32.const 9))
      (then (return (i32.const 1))))
    (if (i32.eq (local.get $opcode) (i32.const 8))
      (then (return (i32.const 1))))
    (i32.const 1))

  ;; ── memcpy ──
  (func $memcpy (param $dst i32) (param $src i32) (param $len i32)
    (local $i i32)
    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $i) (local.get $len)))
        (i32.store8
          (i32.add (local.get $dst) (local.get $i))
          (i32.load8_u (i32.add (local.get $src) (local.get $i))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop))))
)
