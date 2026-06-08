;; Frame Core — framed I/O (8-byte header)

    ;; Standard ID removed — merged into single module

  ;; ── Shared header scratch ──
  ;; $HDR comes from config.wat / package.json

  ;; ── Frame: [stream_id:u32_le][payload_len:u32_le][payload] ──

  ;; frame_write(pipe, stream_id, data, len) → status
  (func $frame_write (export "frame_write")
    (param $pipe i32) (param $stream_id i32) (param $data i32) (param $len i32) (result i32)
    (local $r i32)
    (i32.store (global.get $HDR) (local.get $stream_id))
    (i32.store offset=4 (global.get $HDR) (local.get $len))
    (local.set $r (call $pipe_write (local.get $pipe) (global.get $HDR) (i32.const 8)))
    (if (i32.ne (local.get $r) (global.get $STATUS_OK)) (then (return (local.get $r))))
    (call $pipe_write (local.get $pipe) (local.get $data) (local.get $len)))

  ;; frame_read(pipe, scratch, scap) → pack(status, stream_id)
  ;; On success: stores payload_len at scratch[0..4], payload at scratch[4..4+payload_len)
  ;; scratch must be at least payload_len + 4 bytes
  (func $frame_read (export "frame_read")
    (param $pipe i32) (param $scratch i32) (param $scap i32) (result i64)
    (local $r i32) (local $avail i32) (local $stream_id i32) (local $payload_len i32)
    (local $remaining i32) (local $chunk i32)
    ;; Read 8-byte header
    (local.set $r (call $pipe_read (local.get $pipe) (local.get $scratch) (i32.const 8)))
    (if (i32.lt_s (local.get $r) (i32.const 8))
      (then (return (call $pack (global.get $STATUS_INPUT_SHORT) (i32.const 0)))))
    (local.set $stream_id (i32.load (local.get $scratch)))
    (local.set $payload_len (i32.load offset=4 (local.get $scratch)))
    ;; Check space
    (if (i32.gt_u (i32.add (local.get $payload_len) (i32.const 4)) (local.get $scap))
      (then
        ;; Skip payload in chunks using scratch as temp buffer (avoids HDR overflow)
        (local.set $remaining (local.get $payload_len))
        (block $skip_done
          (loop $skip_loop
            (br_if $skip_done (i32.eqz (local.get $remaining)))
            (local.set $chunk (local.get $remaining))
            (if (i32.gt_u (local.get $chunk) (local.get $scap))
              (then (local.set $chunk (local.get $scap))))
            (drop (call $pipe_read (local.get $pipe) (local.get $scratch) (local.get $chunk)))
            (local.set $remaining (i32.sub (local.get $remaining) (local.get $chunk)))
            (br $skip_loop)))
        (return (call $pack (global.get $STATUS_OVERFLOW) (local.get $stream_id)))))
    ;; Write payload_len to scratch[0..4], payload to scratch[4..]
    (i32.store (local.get $scratch) (local.get $payload_len))
    (local.set $r (call $pipe_read (local.get $pipe)
      (i32.add (local.get $scratch) (i32.const 4)) (local.get $payload_len)))
    (if (i32.lt_s (local.get $r) (i32.const 0))
      (then (return (call $pack (i32.sub (i32.const 0) (local.get $r)) (i32.const 0)))))
    (call $pack (global.get $STATUS_OK) (local.get $stream_id)))

  ;; ── Convenience: read frame payload directly to output pipe ──
  ;; frame_route(pipe, stream_dst, scratch) → pack(status, stream_id)
  ;; Reads header from pipe, then reads payload directly to stream_dst
  ;; scratch[0..7] used for header only
  (func $frame_route (export "frame_route")
    (param $pipe i32) (param $stream_dst i32) (param $scratch i32) (result i64)
    (local $r i32) (local $stream_id i32) (local $payload_len i32)
    (local.set $r (call $pipe_read (local.get $pipe) (local.get $scratch) (i32.const 8)))
    (if (i32.lt_s (local.get $r) (i32.const 8))
      (then (return (call $pack (global.get $STATUS_INPUT_SHORT) (i32.const 0)))))
    (local.set $stream_id (i32.load (local.get $scratch)))
    (local.set $payload_len (i32.load offset=4 (local.get $scratch)))
    ;; Read payload directly to output pipe via scratch in chunks
    (local.set $r (call $pipe_read (local.get $pipe) (local.get $scratch) (local.get $payload_len)))
    (if (i32.lt_s (local.get $r) (i32.const 0))
      (then (return (call $pack (i32.sub (i32.const 0) (local.get $r)) (i32.const 0)))))
    (drop (call $pipe_write (local.get $stream_dst) (local.get $scratch) (local.get $payload_len)))
    (return (call $pack (global.get $STATUS_OK) (local.get $stream_id))))

  ;; ── Message queue: [len:u32_le][payload] ──

  (func $msg_write (export "msg_write")
    (param $pipe i32) (param $data i32) (param $len i32) (result i32)
    (local $r i32)
    (i32.store (global.get $HDR) (local.get $len))
    (local.set $r (call $pipe_write (local.get $pipe) (global.get $HDR) (i32.const 4)))
    (if (i32.ne (local.get $r) (global.get $STATUS_OK)) (then (return (local.get $r))))
    (call $pipe_write (local.get $pipe) (local.get $data) (local.get $len)))

  ;; msg_read(pipe, scratch, scap) → payload_len (bytes in scratch) | 0 | negative error
  ;; On success: stores payload_len at scratch[0..4], payload at scratch[4..4+payload_len)
  (func $msg_read (export "msg_read")
    (param $pipe i32) (param $scratch i32) (param $scap i32) (result i32)
    (local $r i32) (local $payload_len i32)
    (local.set $r (call $pipe_read (local.get $pipe) (local.get $scratch) (i32.const 4)))
    (if (i32.lt_s (local.get $r) (i32.const 4))
      (then (return (i32.const 0))))
    (local.set $payload_len (i32.load (local.get $scratch)))
    (if (i32.gt_u (i32.add (local.get $payload_len) (i32.const 4)) (local.get $scap))
      (then (return (i32.sub (i32.const 0) (global.get $STATUS_OVERFLOW)))))
    (i32.store (local.get $scratch) (local.get $payload_len))
    (local.set $r (call $pipe_read (local.get $pipe)
      (i32.add (local.get $scratch) (i32.const 4)) (local.get $payload_len)))
    (if (i32.lt_s (local.get $r) (i32.const 0))
      (then (return (local.get $r))))
    local.get $payload_len)
