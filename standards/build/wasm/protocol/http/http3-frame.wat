(module
  (import "edgerun-core" "memory" (memory 1))
  (import "edgerun-core" "pack" (func $pack (param i32 i32) (result i64)))

(func (export "proto_standard_id") (result i32)
    (i32.const 300012))

  ;; Classifications:
  ;; 0 DATA, 1 HEADERS, 2 SETTINGS, 3 CANCEL_PUSH, 4 PUSH_PROMISE,
  ;; 5 MAX_PUSH_ID, 6 GOAWAY, 7 STREAMS_BLOCKED, 8 reserved/greasing, 9 unknown.
  (func $http3_frame_type_classify (export "http3_frame_type_classify") (param $frame_type i64) (result i32)
    (if (i64.eq (local.get $frame_type) (i64.const 0))
      (then (return (i32.const 0))))
    (if (i64.eq (local.get $frame_type) (i64.const 1))
      (then (return (i32.const 1))))
    (if (i64.eq (local.get $frame_type) (i64.const 3))
      (then (return (i32.const 3))))
    (if (i64.eq (local.get $frame_type) (i64.const 4))
      (then (return (i32.const 2))))
    (if (i64.eq (local.get $frame_type) (i64.const 5))
      (then (return (i32.const 4))))
    (if (i64.eq (local.get $frame_type) (i64.const 7))
      (then (return (i32.const 5))))
    (if (i64.eq (local.get $frame_type) (i64.const 8))
      (then (return (i32.const 6))))
    (if (i64.eq (local.get $frame_type) (i64.const 9))
      (then (return (i32.const 7))))
    (if
      (i32.or
        (i64.eq (i64.rem_u (local.get $frame_type) (i64.const 31)) (i64.const 2))
        (i64.eq (i64.rem_u (local.get $frame_type) (i64.const 31)) (i64.const 6)))
      (then (return (i32.const 8))))
    (i32.const 9))

  (func $quic_varint_decode_at
    (param $in_ptr i32) (param $in_len i32) (param $offset i32) (param $out_ptr i32)
    (result i64)
    (local $first i32)
    (local $need i32)
    (local $i i32)
    (local $value i64)
    (if (i32.ge_u (local.get $offset) (local.get $in_len))
      (then (return (call $pack (i32.const 1) (i32.const 0)))))
    (local.set $first (i32.load8_u (i32.add (local.get $in_ptr) (local.get $offset))))
    (local.set $need
      (i32.shl
        (i32.const 1)
        (i32.shr_u (local.get $first) (i32.const 6))))
    (if
      (i32.or
        (i32.gt_u (local.get $need) (local.get $in_len))
        (i32.gt_u (local.get $offset) (i32.sub (local.get $in_len) (local.get $need))))
      (then (return (call $pack (i32.const 5) (i32.const 0)))))
    (local.set $value (i64.extend_i32_u (i32.and (local.get $first) (i32.const 63))))
    (local.set $i (i32.const 1))
    (loop $again
      (if (i32.lt_u (local.get $i) (local.get $need))
        (then
          (local.set $value
            (i64.or
              (i64.shl (local.get $value) (i64.const 8))
              (i64.extend_i32_u
                (i32.load8_u
                  (i32.add
                    (i32.add (local.get $in_ptr) (local.get $offset))
                    (local.get $i))))))
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
          (br $again))))
    (i64.store (local.get $out_ptr) (local.get $value))
    (call $pack (i32.const 0) (local.get $need)))

  (func $quic_varint_encode
    (param $value i64) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (if (i64.gt_u (local.get $value) (i64.const 4611686018427387903))
      (then (return (call $pack (i32.const 4) (i32.const 0)))))
    (if (i64.le_u (local.get $value) (i64.const 63))
      (then
        (if (i32.lt_u (local.get $out_cap) (i32.const 1))
          (then (return (call $pack (i32.const 2) (i32.const 0)))))
        (i32.store8 (local.get $out_ptr) (i32.wrap_i64 (local.get $value)))
        (return (call $pack (i32.const 0) (i32.const 1)))))
    (if (i64.le_u (local.get $value) (i64.const 16383))
      (then
        (if (i32.lt_u (local.get $out_cap) (i32.const 2))
          (then (return (call $pack (i32.const 2) (i32.const 0)))))
        (i32.store8
          (local.get $out_ptr)
          (i32.or
            (i32.const 64)
            (i32.and
              (i32.wrap_i64 (i64.shr_u (local.get $value) (i64.const 8)))
              (i32.const 63))))
        (i32.store8
          (i32.add (local.get $out_ptr) (i32.const 1))
          (i32.wrap_i64 (local.get $value)))
        (return (call $pack (i32.const 0) (i32.const 2)))))
    (if (i64.le_u (local.get $value) (i64.const 1073741823))
      (then
        (if (i32.lt_u (local.get $out_cap) (i32.const 4))
          (then (return (call $pack (i32.const 2) (i32.const 0)))))
        (i32.store8
          (local.get $out_ptr)
          (i32.or
            (i32.const 128)
            (i32.and
              (i32.wrap_i64 (i64.shr_u (local.get $value) (i64.const 24)))
              (i32.const 63))))
        (i32.store8
          (i32.add (local.get $out_ptr) (i32.const 1))
          (i32.wrap_i64 (i64.shr_u (local.get $value) (i64.const 16))))
        (i32.store8
          (i32.add (local.get $out_ptr) (i32.const 2))
          (i32.wrap_i64 (i64.shr_u (local.get $value) (i64.const 8))))
        (i32.store8
          (i32.add (local.get $out_ptr) (i32.const 3))
          (i32.wrap_i64 (local.get $value))
        )
        (return (call $pack (i32.const 0) (i32.const 4)))))
    (if (i32.lt_u (local.get $out_cap) (i32.const 8))
      (then (return (call $pack (i32.const 2) (i32.const 0)))))
    (i32.store8
      (local.get $out_ptr)
      (i32.or
        (i32.const 192)
        (i32.and
          (i32.wrap_i64 (i64.shr_u (local.get $value) (i64.const 56)))
          (i32.const 63))))
    (i32.store8
      (i32.add (local.get $out_ptr) (i32.const 1))
      (i32.wrap_i64 (i64.shr_u (local.get $value) (i64.const 48))))
    (i32.store8
      (i32.add (local.get $out_ptr) (i32.const 2))
      (i32.wrap_i64 (i64.shr_u (local.get $value) (i64.const 40))))
    (i32.store8
      (i32.add (local.get $out_ptr) (i32.const 3))
      (i32.wrap_i64 (i64.shr_u (local.get $value) (i64.const 32))))
    (i32.store8
      (i32.add (local.get $out_ptr) (i32.const 4))
      (i32.wrap_i64 (i64.shr_u (local.get $value) (i64.const 24))))
    (i32.store8
      (i32.add (local.get $out_ptr) (i32.const 5))
      (i32.wrap_i64 (i64.shr_u (local.get $value) (i64.const 16))))
    (i32.store8
      (i32.add (local.get $out_ptr) (i32.const 6))
      (i32.wrap_i64 (i64.shr_u (local.get $value) (i64.const 8))))
    (i32.store8
      (i32.add (local.get $out_ptr) (i32.const 7))
      (i32.wrap_i64 (local.get $value)))
    (call $pack (i32.const 0) (i32.const 8)))

  ;; Decode an HTTP/3 frame header and verify the declared payload is present.
  ;; out record u32 fields:
  ;; 0 type_low, 4 type_high, 8 payload_len_low, 12 payload_len_high,
  ;; 16 header_len, 20 classification.
  (func (export "http3_frame_header_decode")
    (param $in_ptr i32) (param $in_len i32) (param $out_ptr i32)
    (result i32)
    (local $packed i64)
    (local $status i32)
    (local $type_len i32)
    (local $payload_len_bytes i32)
    (local $header_len i32)
    (local $frame_type i64)
    (local $payload_len i64)
    (local.set $packed
      (call $quic_varint_decode_at
        (local.get $in_ptr)
        (local.get $in_len)
        (i32.const 0)
        (local.get $out_ptr)))
    (local.set $status (i32.wrap_i64 (local.get $packed)))
    (if (local.get $status)
      (then (return (local.get $status))))
    (local.set $type_len
      (i32.wrap_i64 (i64.shr_u (local.get $packed) (i64.const 32))))
    (local.set $frame_type (i64.load (local.get $out_ptr)))
    (local.set $packed
      (call $quic_varint_decode_at
        (local.get $in_ptr)
        (local.get $in_len)
        (local.get $type_len)
        (i32.add (local.get $out_ptr) (i32.const 8))))
    (local.set $status (i32.wrap_i64 (local.get $packed)))
    (if (local.get $status)
      (then (return (local.get $status))))
    (local.set $payload_len_bytes
      (i32.wrap_i64 (i64.shr_u (local.get $packed) (i64.const 32))))
    (local.set $payload_len (i64.load (i32.add (local.get $out_ptr) (i32.const 8))))
    (local.set $header_len (i32.add (local.get $type_len) (local.get $payload_len_bytes)))
    (if
      (i64.gt_u
        (local.get $payload_len)
        (i64.extend_i32_u (i32.sub (local.get $in_len) (local.get $header_len))))
      (then (return (i32.const 5))))
    (i32.store (local.get $out_ptr) (i32.wrap_i64 (local.get $frame_type)))
    (i32.store
      (i32.add (local.get $out_ptr) (i32.const 4))
      (i32.wrap_i64 (i64.shr_u (local.get $frame_type) (i64.const 32))))
    (i32.store
      (i32.add (local.get $out_ptr) (i32.const 8))
      (i32.wrap_i64 (local.get $payload_len)))
    (i32.store
      (i32.add (local.get $out_ptr) (i32.const 12))
      (i32.wrap_i64 (i64.shr_u (local.get $payload_len) (i64.const 32))))
    (i32.store
      (i32.add (local.get $out_ptr) (i32.const 16))
      (local.get $header_len))
    (i32.store
      (i32.add (local.get $out_ptr) (i32.const 20))
      (call $http3_frame_type_classify (local.get $frame_type)))
    (i32.const 0))

  ;; Return bits: low32=status, high32=written.
  (func (export "http3_frame_header_encode")
    (param $frame_type i64) (param $payload_len i64) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (local $packed i64)
    (local $status i32)
    (local $type_written i32)
    (if
      (i32.or
        (i64.gt_u (local.get $frame_type) (i64.const 4611686018427387903))
        (i64.gt_u (local.get $payload_len) (i64.const 4611686018427387903)))
      (then (return (call $pack (i32.const 4) (i32.const 0)))))
    (local.set $packed
      (call $quic_varint_encode
        (local.get $frame_type)
        (local.get $out_ptr)
        (local.get $out_cap)))
    (local.set $status (i32.wrap_i64 (local.get $packed)))
    (if (local.get $status)
      (then (return (local.get $packed))))
    (local.set $type_written
      (i32.wrap_i64 (i64.shr_u (local.get $packed) (i64.const 32))))
    (local.set $packed
      (call $quic_varint_encode
        (local.get $payload_len)
        (i32.add (local.get $out_ptr) (local.get $type_written))
        (i32.sub (local.get $out_cap) (local.get $type_written))))
    (local.set $status (i32.wrap_i64 (local.get $packed)))
    (if (local.get $status)
      (then (return (local.get $packed))))
    (call $pack
      (i32.const 0)
      (i32.add
        (local.get $type_written)
        (i32.wrap_i64 (i64.shr_u (local.get $packed) (i64.const 32))))))
)
