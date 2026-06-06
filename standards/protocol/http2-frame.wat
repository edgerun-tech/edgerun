;; Known HTTP/2 frame types return their type byte. Unknown extension frames
  ;; return 255, matching the local Rust FrameType::Extension classifier.
  (func $http2_frame_type_classify (export "http2_frame_type_classify") (param $frame_type i32) (result i32)
    (if (result i32)
      (i32.le_u (local.get $frame_type) (i32.const 9))
      (then (local.get $frame_type))
      (else (i32.const 255))))

  ;; Decode output record, little-endian:
  ;; 0:u32 payload_length
  ;; 4:u32 frame_type_class
  ;; 8:u32 flags
  ;; 12:u32 stream_id_reserved_bit
  ;; 16:u32 stream_id_cleared
  ;; 20:u32 header_len
  ;; 24:u32 total_len
  (func (export "http2_frame_header_decode")
    (param $in_ptr i32) (param $in_len i32) (param $max_frame_size i32) (param $out_ptr i32)
    (result i32)
    (local $payload_len i32)
    (local $frame_type i32)
    (local $flags i32)
    (local $stream_raw i32)
    (local $stream_reserved i32)
    (local $stream_id i32)
    (local $total_len i32)
    (if (i32.lt_u (local.get $in_len) (i32.const 9))
      (then (return (i32.const 1))))
    (local.set $payload_len
      (i32.or
        (i32.or
          (i32.shl (i32.load8_u (local.get $in_ptr)) (i32.const 16))
          (i32.shl (i32.load8_u (i32.add (local.get $in_ptr) (i32.const 1))) (i32.const 8)))
        (i32.load8_u (i32.add (local.get $in_ptr) (i32.const 2)))))
    (if (i32.gt_u (local.get $payload_len) (local.get $max_frame_size))
      (then (return (i32.const 3))))
    (local.set $total_len (i32.add (local.get $payload_len) (i32.const 9)))
    (if (i32.lt_u (local.get $in_len) (local.get $total_len))
      (then (return (i32.const 1))))
    (local.set $frame_type (i32.load8_u (i32.add (local.get $in_ptr) (i32.const 3))))
    (local.set $flags (i32.load8_u (i32.add (local.get $in_ptr) (i32.const 4))))
    (local.set $stream_raw
      (i32.or
        (i32.or
          (i32.shl (i32.load8_u (i32.add (local.get $in_ptr) (i32.const 5))) (i32.const 24))
          (i32.shl (i32.load8_u (i32.add (local.get $in_ptr) (i32.const 6))) (i32.const 16)))
        (i32.or
          (i32.shl (i32.load8_u (i32.add (local.get $in_ptr) (i32.const 7))) (i32.const 8))
          (i32.load8_u (i32.add (local.get $in_ptr) (i32.const 8))))))
    (local.set $stream_reserved
      (i32.shr_u (local.get $stream_raw) (i32.const 31)))
    (local.set $stream_id
      (i32.and (local.get $stream_raw) (i32.const 0x7fffffff)))
    (i32.store (local.get $out_ptr) (local.get $payload_len))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 4))
      (if (result i32)
        (i32.le_u (local.get $frame_type) (i32.const 9))
        (then (local.get $frame_type))
        (else (i32.const 255))))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 8)) (local.get $flags))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 12)) (local.get $stream_reserved))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 16)) (local.get $stream_id))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 20)) (i32.const 9))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 24)) (local.get $total_len))
    (i32.const 0))

  ;; Return bits: low32=status, high32=written.
  (func (export "http2_frame_header_encode")
    (param $payload_len i32) (param $frame_type i32) (param $flags i32) (param $stream_id i32)
    (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (local $sid i32)
    (if (i32.lt_u (local.get $out_cap) (i32.const 9))
      (then
        (return
          (i64.or
            (i64.extend_i32_u (i32.const 2))
            (i64.shl (i64.extend_i32_u (i32.const 0)) (i64.const 32))))))
    (if
      (i32.or
        (i32.or
          (i32.gt_u (local.get $payload_len) (i32.const 0x00ffffff))
          (i32.gt_u (local.get $frame_type) (i32.const 255)))
        (i32.gt_u (local.get $flags) (i32.const 255)))
      (then
        (return
          (i64.or
            (i64.extend_i32_u (i32.const 4))
            (i64.shl (i64.extend_i32_u (i32.const 0)) (i64.const 32))))))
    (local.set $sid (i32.and (local.get $stream_id) (i32.const 0x7fffffff)))
    (i32.store8 (local.get $out_ptr) (i32.shr_u (local.get $payload_len) (i32.const 16)))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 1))
      (i32.shr_u (local.get $payload_len) (i32.const 8)))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 2)) (local.get $payload_len))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 3)) (local.get $frame_type))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 4)) (local.get $flags))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 5)) (i32.shr_u (local.get $sid) (i32.const 24)))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 6)) (i32.shr_u (local.get $sid) (i32.const 16)))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 7)) (i32.shr_u (local.get $sid) (i32.const 8)))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 8)) (local.get $sid))
    (i64.or
      (i64.extend_i32_u (i32.const 0))
      (i64.shl (i64.extend_i32_u (i32.const 9)) (i64.const 32))))
