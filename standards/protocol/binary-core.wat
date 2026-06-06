(module
  ;; ═════════════════════════════════════════════════════════════════════
  ;; Binary Core Primitives — shared big-endian integer read/write helpers
  ;; ═════════════════════════════════════════════════════════════════════

  (memory (export "memory") 1)

  ;; Raw big-endian u16 read from ptr (no bounds checking).
  (func $read_u16_be (export "read_u16_be") (param $ptr i32) (result i32)
    (i32.or
      (i32.shl (i32.load8_u (local.get $ptr)) (i32.const 8))
      (i32.load8_u (i32.add (local.get $ptr) (i32.const 1)))))

  ;; Raw big-endian u24 read from ptr (no bounds checking).
  (func $read_u24_be (export "read_u24_be") (param $ptr i32) (result i32)
    (i32.or
      (i32.or
        (i32.shl (i32.load8_u (local.get $ptr)) (i32.const 16))
        (i32.shl (i32.load8_u (i32.add (local.get $ptr) (i32.const 1))) (i32.const 8)))
      (i32.load8_u (i32.add (local.get $ptr) (i32.const 2)))))

  ;; Raw big-endian u32 read from ptr (no bounds checking).
  (func $read_u32_be (export "read_u32_be") (param $ptr i32) (result i32)
    (i32.or
      (i32.or
        (i32.or
          (i32.shl (i32.load8_u (local.get $ptr)) (i32.const 24))
          (i32.shl (i32.load8_u (i32.add (local.get $ptr) (i32.const 1))) (i32.const 16)))
        (i32.shl (i32.load8_u (i32.add (local.get $ptr) (i32.const 2))) (i32.const 8)))
      (i32.load8_u (i32.add (local.get $ptr) (i32.const 3)))))

  ;; Big-endian u16 write to ptr.
  (func $write_u16_be (export "write_u16_be") (param $ptr i32) (param $v i32)
    (i32.store8 (local.get $ptr) (i32.shr_u (local.get $v) (i32.const 8)))
    (i32.store8 (i32.add (local.get $ptr) (i32.const 1)) (local.get $v)))

  ;; Big-endian u32 write to ptr.
  (func $write_u32_be (export "write_u32_be") (param $ptr i32) (param $v i32)
    (i32.store8 (local.get $ptr) (i32.shr_u (local.get $v) (i32.const 24)))
    (i32.store8 (i32.add (local.get $ptr) (i32.const 1)) (i32.shr_u (local.get $v) (i32.const 16)))
    (i32.store8 (i32.add (local.get $ptr) (i32.const 2)) (i32.shr_u (local.get $v) (i32.const 8)))
    (i32.store8 (i32.add (local.get $ptr) (i32.const 3)) (local.get $v)))
)
