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
  (import "edgerun-core" "memcpy" (func $memcpy (param i32 i32 i32)))

  (func (export "proto_standard_id") (result i32) i32.const 300508)

  ;; ════════════════════════════════════════════════════════════════
  ;; ws_encode — payload → WS frame (pure transform, batch)
  ;;
  ;; Config (optional, 4 bytes): [opcode:i32]
  ;;   +0: opcode (1=text, 2=binary, default=2). 0 defaults to binary.
  ;;
  ;; Reads payload from input pipe, wraps in WS server frame (FIN, no mask),
  ;; writes complete WS frame to output pipe.
  ;;
  ;; Signature: (input, output, cfg, clen, scratch, scap, state) → bytes_written
  ;; Batch stage: state is ignored (pass 0).
  ;; ════════════════════════════════════════════════════════════════

  (func (export "ws_encode")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $opcode i32) (local $n i32) (local $result i64)
    (local $status i32) (local $hdr_len i32)

    (local.set $opcode (i32.const 2))
    (if (i32.ge_u (local.get $clen) (i32.const 4))
      (then
        (local.set $opcode (i32.load (local.get $cfg)))
        (if (i32.eqz (local.get $opcode))
          (then (local.set $opcode (i32.const 2))))))

    (if (i32.lt_u (local.get $scap) (i32.const 17))
      (then (return (i32.const -1))))
    ;; Read payload into scratch+16 (leave 16 bytes for max header)
    (local.set $n (call $pipe_read (local.get $input)
      (i32.add (local.get $scratch) (i32.const 16))
      (i32.sub (local.get $scap) (i32.const 16))))
    (if (i32.le_s (local.get $n) (i32.const 0))
      (then (return (local.get $n))))

    ;; Write WS server frame header to scratch[0..9] (max 10 bytes, no mask)
    (local.set $result (call $ws_write_srv_hdr
      (local.get $opcode) (local.get $n) (i32.const 0)
      (local.get $scratch) (i32.const 10)))
    (local.set $status (i32.wrap_i64 (local.get $result)))
    (if (i32.ne (local.get $status) (i32.const 0))
      (then (return (i32.sub (i32.const 0) (local.get $status)))))
    (local.set $hdr_len (i32.wrap_i64 (i64.shr_u (local.get $result) (i64.const 32))))

    ;; Write header to output
    (drop (call $pipe_write (local.get $output) (local.get $scratch) (local.get $hdr_len)))
    ;; Write payload to output
    (drop (call $pipe_write (local.get $output)
      (i32.add (local.get $scratch) (i32.const 16)) (local.get $n)))
    (i32.add (local.get $hdr_len) (local.get $n)))

  ;; ════════════════════════════════════════════════════════════════
  ;; ws_decode — WS frame → payload (pure transform, streaming)
  ;;
  ;; Config: none (pass 0, 0).
  ;;
  ;; State layout (148 bytes):
  ;;   +0:   tick      i32 (RO)
  ;;   +4:   (reserved)
  ;;   +8:   dbuf_len  i32
  ;;   +12:  dbuf[136] decode buffer (partial frame data)
  ;;
  ;; Thin wrapper around $decode_frame_socket with dbuf_off=8 and no send_pipe.
  ;; ════════════════════════════════════════════════════════════════

  (func (export "ws_decode")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (call $decode_frame_socket
      (local.get $input) (local.get $scratch) (local.get $scap)
      (local.get $state) (i32.const 8) (local.get $output) (i32.const 0)))

  ;; ════════════════════════════════════════════════════════════════
  ;; process_ws_frame — bidirectional WS framing (socket mode)
  ;; Config: pointer to socket handle (4 bytes)
  ;; State layout (148 bytes):
  ;;   +0:   tick          i32 (RO)
  ;;   +4:   phase         i32 (0=idle, 1=awaiting)
  ;;   +8:   start_tick    i32
  ;;   +12:  timeout_ticks i32
  ;;   +16:  dbuf_len      i32
  ;;   +20:  dbuf[128]     decode buffer
  ;;
  ;; Reads payload from input, wraps in WS frame, writes to socket send pipe.
  ;; Reads raw bytes from socket recv pipe, decodes WS frames, writes payload
  ;; to output pipe. Ping→Pong, Close→Close echo.
  ;; ════════════════════════════════════════════════════════════════

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

    ;; Phase 1 (awaiting response)
    (if (i32.eq (local.get $phase) (i32.const 1))
      (then
        (local.set $elapsed
          (i32.sub (i32.load (local.get $state)) (i32.load offset=8 (local.get $state))))
        (if (i32.load offset=12 (local.get $state))
          (then
            (if (i32.ge_u (local.get $elapsed) (i32.load offset=12 (local.get $state)))
              (then (return (global.get $TIMEOUT))))))

        ;; Decode from recv pipe
        (local.set $n (call $decode_frame_socket
          (local.get $recv_pipe) (local.get $scratch) (local.get $scap)
          (local.get $state) (i32.const 20) (local.get $output) (local.get $send_pipe)))
        (if (i32.gt_s (local.get $n) (i32.const 0))
          (then
            (if (i32.ne (local.get $n) (global.get $MORE))
              (then
                (i32.store offset=4 (local.get $state) (i32.const 0))
                (return (local.get $n))))))
        (if (i32.lt_s (local.get $n) (i32.const 0))
          (then (return (local.get $n))))
        (return (global.get $MORE))))

    ;; Phase 0 (idle): encode + try decode

    ;; Encode: read payload, wrap in WS frame, send
    (if (i32.lt_u (local.get $scap) (i32.const 17))
      (then (return (i32.const -1))))
    (local.set $n (call $pipe_read (local.get $input)
      (i32.add (local.get $scratch) (i32.const 16))
      (i32.sub (local.get $scap) (i32.const 16))))
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

    ;; Decode from recv pipe
    (local.set $n (call $decode_frame_socket
      (local.get $recv_pipe) (local.get $scratch) (local.get $scap)
      (local.get $state) (i32.const 20) (local.get $output) (local.get $send_pipe)))
    (if (i32.gt_s (local.get $n) (i32.const 0))
      (then
        (if (i32.ne (local.get $n) (global.get $MORE))
          (then (return (local.get $n))))))
    (if (i32.lt_s (local.get $n) (i32.const 0))
      (then (return (local.get $n))))

    ;; Decode returned MORE
    (if (local.get $sent)
      (then
        (i32.store offset=4 (local.get $state) (i32.const 1))
        (i32.store offset=8 (local.get $state) (i32.load (local.get $state)))))
    (global.get $MORE))

  ;; ── $decode_frame_socket — same as ws_decode but reads from recv pipe ──
  ;; and handles control frames (ping→pong, close→close echo).
  ;; dbuf stored at state + dbuf_off (rather than fixed offset +8).
  (func $decode_frame_socket
    (param $recv i32) (param $scratch i32) (param $scap i32)
    (param $state i32) (param $dbuf_off i32)
    (param $output i32) (param $send_pipe i32) (result i32)
    (local $dbuf_len i32) (local $n i32) (local $total i32) (local $r i32)
    (local $hdr_out i32)
    (local $fin i32) (local $opcode i32) (local $masked i32)
    (local $hdr_len i32) (local $payload_len i32) (local $mask_key i32)
    (local $frame_size i32) (local $off i32)
    (local $result i64) (local $status i32) (local $written i32)
    (local $dbuf_cap i32)

    (if (i32.lt_u (local.get $scap) (i32.const 32))
      (then (return (i32.const -1))))
    (local.set $dbuf_cap (i32.sub (i32.const 144) (local.get $dbuf_off)))
    (local.set $dbuf_len (i32.load (i32.add (local.get $state) (local.get $dbuf_off))))

    (if (i32.gt_u (local.get $dbuf_len) (i32.const 0))
      (then
        (call $memcpy (local.get $scratch)
          (i32.add (local.get $state) (i32.add (local.get $dbuf_off) (i32.const 4)))
          (local.get $dbuf_len))))

    (local.set $n (call $pipe_read (local.get $recv)
      (i32.add (local.get $scratch) (local.get $dbuf_len))
      (i32.sub (local.get $scap) (local.get $dbuf_len))))
    (if (i32.lt_s (local.get $n) (i32.const 0))
      (then (local.set $n (i32.const 0))))
    (local.set $total (i32.add (local.get $dbuf_len) (local.get $n)))

    (if (i32.lt_u (local.get $total) (i32.const 2))
      (then
        (i32.store (i32.add (local.get $state) (local.get $dbuf_off)) (local.get $total))
        (return (global.get $MORE))))

    (local.set $hdr_out (i32.sub (local.get $scap) (i32.const 32)))
    (local.set $r (call $ws_parse_header
      (local.get $scratch) (local.get $total) (local.get $scap) (local.get $hdr_out)))

    (if (i32.eq (local.get $r) (i32.const 1))
      (then
        (if (i32.gt_u (local.get $total) (local.get $dbuf_cap))
          (then (return (i32.const -5))))
        (call $memcpy (i32.add (local.get $state) (i32.add (local.get $dbuf_off) (i32.const 4)))
          (local.get $scratch) (local.get $total))
        (i32.store (i32.add (local.get $state) (local.get $dbuf_off)) (local.get $total))
        (return (global.get $MORE))))

    (if (i32.eq (local.get $r) (i32.const 3)) (then (return (i32.const -3))))
    (if (i32.eq (local.get $r) (i32.const 4)) (then (return (i32.const -4))))

    (local.set $fin    (i32.load offset=0 (local.get $hdr_out)))
    (local.set $opcode (i32.load offset=8 (local.get $hdr_out)))
    (local.set $masked (i32.load offset=12 (local.get $hdr_out)))
    (local.set $payload_len (i32.load offset=16 (local.get $hdr_out)))
    (local.set $hdr_len (i32.load offset=24 (local.get $hdr_out)))
    (local.set $mask_key (i32.load offset=28 (local.get $hdr_out)))

    (local.set $frame_size (i32.add (local.get $hdr_len) (local.get $payload_len)))
    (if (i32.gt_u (local.get $frame_size) (local.get $total))
      (then
        (if (i32.gt_u (local.get $total) (local.get $dbuf_cap))
          (then (return (i32.const -5))))
        (call $memcpy (i32.add (local.get $state) (i32.add (local.get $dbuf_off) (i32.const 4)))
          (local.get $scratch) (local.get $total))
        (i32.store (i32.add (local.get $state) (local.get $dbuf_off)) (local.get $total))
        (return (global.get $MORE))))

    (local.set $off (local.get $hdr_len))

    (if (local.get $masked)
      (then
        (drop (call $ws_apply_mask
          (i32.add (local.get $scratch) (local.get $off))
          (local.get $payload_len) (local.get $mask_key)))))

    (block $ctrl
      (if (i32.and (i32.ge_u (local.get $opcode) (i32.const 1))
                   (i32.le_u (local.get $opcode) (i32.const 2)))
        (then
          (drop (call $pipe_write (local.get $output)
            (i32.add (local.get $scratch) (local.get $off))
            (local.get $payload_len)))
          (br $ctrl)))

      (if (i32.eq (local.get $opcode) (i32.const 0))
        (then
          (drop (call $pipe_write (local.get $output)
            (i32.add (local.get $scratch) (local.get $off))
            (local.get $payload_len)))
          (br $ctrl)))

      (if (i32.eq (local.get $opcode) (i32.const 9))
        (then
          (if (local.get $send_pipe)
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
                  (drop (call $pipe_write (local.get $send_pipe) (local.get $scratch) (local.get $written)))
                  (drop (call $pipe_write (local.get $send_pipe)
                    (i32.add (local.get $scratch) (local.get $off))
                    (local.get $payload_len)))))))
          (br $ctrl)))

      (if (i32.eq (local.get $opcode) (i32.const 8))
        (then
          (if (local.get $send_pipe)
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
                    (local.get $payload_len)))))))
          (br $ctrl))))

    ;; Save overflow
    (local.set $off (i32.add (local.get $off) (local.get $payload_len)))
    (local.set $n (i32.sub (local.get $total) (local.get $off)))
    (if (i32.gt_u (local.get $n) (i32.const 0))
      (then
        (if (i32.gt_u (local.get $n) (local.get $dbuf_cap))
          (then (return (i32.const -5))))
        (call $memcpy (i32.add (local.get $state) (i32.add (local.get $dbuf_off) (i32.const 4)))
          (i32.add (local.get $scratch) (local.get $off)) (local.get $n))
        (i32.store (i32.add (local.get $state) (local.get $dbuf_off)) (local.get $n)))
      (else
        (i32.store (i32.add (local.get $state) (local.get $dbuf_off)) (i32.const 0))))

    (if (i32.le_u (local.get $opcode) (i32.const 2))
      (then (return (local.get $payload_len))))
    (if (i32.eq (local.get $opcode) (i32.const 9))
      (then (return (i32.const 1))))
    (if (i32.eq (local.get $opcode) (i32.const 8))
      (then (return (i32.const 1))))
    (i32.const 1))

  ;; memcpy imported from edgerun-core as $memcpy(dst, src, len)
)
