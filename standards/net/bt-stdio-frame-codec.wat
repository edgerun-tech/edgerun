(global $BISTDIO_ABI i32 (i32.const 1))

  ;; Frame types
  (global $FT_IDENTITY_REQUEST i32 (i32.const 0x01))
  (global $FT_IDENTITY_RESPONSE i32 (i32.const 0x02))
  (global $FT_IDENTITY_CONFIRM i32 (i32.const 0x03))
  (global $FT_SESSION_INIT i32 (i32.const 0x04))
  (global $FT_SESSION_READY i32 (i32.const 0x05))
  (global $FT_DATA i32 (i32.const 0x06))
  (global $FT_CLOSE i32 (i32.const 0x07))
  (global $FT_ERROR i32 (i32.const 0xFF))

  ;; Frame header size: 4 bytes length + 1 byte type
  (global $HEADER_SIZE i32 (i32.const 5))

  (func (export "bt_stdio_abi_version") (result i32)
    global.get $BISTDIO_ABI
  )

  ;; Encode a frame header at offset.
  ;; Returns total frame size (header + payload)
  (func (export "bt_stdio_encode_header")
    (param $buf_offset i32)
    (param $payload_len i32)
    (param $frame_type i32)
    (result i32)
    ;; Write payload length as big-endian u32
    (i32.store offset=0 (local.get $buf_offset)
      (i32.or
        (i32.shl (i32.and (local.get $payload_len) (i32.const 0xFF000000)) (i32.const 0))
        (i32.or
          (i32.shl (i32.and (local.get $payload_len) (i32.const 0x00FF0000)) (i32.const 0))
          (i32.or
            (i32.shl (i32.and (local.get $payload_len) (i32.const 0x0000FF00)) (i32.const 0))
            (i32.and (local.get $payload_len) (i32.const 0x000000FF))
          )
        )
      )
    )
    ;; Write frame type
    (i32.store8 offset=4 (local.get $buf_offset) (local.get $frame_type))
    ;; Return total size
    (i32.add (local.get $payload_len) (global.get $HEADER_SIZE))
  )

  ;; Decode frame header at offset. Returns frame type.
  ;; Stores payload length and payload offset at given pointers.
  (func (export "bt_stdio_decode_header")
    (param $buf_offset i32)
    (param $out_payload_len i32)
    (param $out_payload_offset i32)
    (result i32)
    (local $len_raw i32)
    (local $payload_len i32)
    (local $frame_type i32)
    ;; Read big-endian length
    (local.set $len_raw (i32.load offset=0 (local.get $buf_offset)))
    (local.set $payload_len
      (i32.or
        (i32.shl (i32.and (local.get $len_raw) (i32.const 0x000000FF)) (i32.const 24))
        (i32.or
          (i32.shl (i32.and (local.get $len_raw) (i32.const 0x0000FF00)) (i32.const 8))
          (i32.or
            (i32.shr_u (i32.and (local.get $len_raw) (i32.const 0x00FF0000)) (i32.const 8))
            (i32.shr_u (i32.and (local.get $len_raw) (i32.const 0xFF000000)) (i32.const 24))
          )
        )
      )
    )
    ;; Read frame type
    (local.set $frame_type (i32.load8_u offset=4 (local.get $buf_offset)))
    ;; Store outputs
    (i32.store (local.get $out_payload_len) (local.get $payload_len))
    (i32.store (local.get $out_payload_offset)
      (i32.add (local.get $buf_offset) (global.get $HEADER_SIZE)))
    ;; Return frame type
    (local.get $frame_type)
  )

  ;; Validate frame type
  (func (export "bt_stdio_valid_frame_type")
    (param $ft i32)
    (result i32)
    (if (i32.eq (local.get $ft) (global.get $FT_IDENTITY_REQUEST))
      (then (return (i32.const 1)))
    )
    (if (i32.eq (local.get $ft) (global.get $FT_IDENTITY_RESPONSE))
      (then (return (i32.const 1)))
    )
    (if (i32.eq (local.get $ft) (global.get $FT_IDENTITY_CONFIRM))
      (then (return (i32.const 1)))
    )
    (if (i32.eq (local.get $ft) (global.get $FT_SESSION_INIT))
      (then (return (i32.const 1)))
    )
    (if (i32.eq (local.get $ft) (global.get $FT_SESSION_READY))
      (then (return (i32.const 1)))
    )
    (if (i32.eq (local.get $ft) (global.get $FT_DATA))
      (then (return (i32.const 1)))
    )
    (if (i32.eq (local.get $ft) (global.get $FT_CLOSE))
      (then (return (i32.const 1)))
    )
    (if (i32.eq (local.get $ft) (global.get $FT_ERROR))
      (then (return (i32.const 1)))
    )
    (i32.const 0)
  )

  ;; Minimum buffer size for a valid frame
  (func (export "bt_stdio_min_frame_size") (result i32)
    global.get $HEADER_SIZE
  )

  ;; Maximum payload per data frame
  (func (export "bt_stdio_max_payload") (result i32)
    (i32.const 4096)
  )
