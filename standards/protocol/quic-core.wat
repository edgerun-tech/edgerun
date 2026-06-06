(module
  ;; ═════════════════════════════════════════════════════════════════════
  ;; QUIC Core Primitives — shared across QUIC parsers
  ;; ═════════════════════════════════════════════════════════════════════

  (import "edgerun" "pack" (func $pack (param i32 i32) (result i64)))

  (memory (export "memory") 1)

  ;; QUIC varint decode status: 0 ok, 1 empty/out-of-range offset, 5 truncated.
  ;; Returns pack(status, bytes_read) and stores the decoded u64 at out_ptr on ok.
  (func $quic_varint_decode_at (export "quic_varint_decode_at")
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
