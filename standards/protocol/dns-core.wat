(module
  ;; ═════════════════════════════════════════════════════════════════════
  ;; DNS Core Primitives — shared across all DNS parsers
  ;; ═════════════════════════════════════════════════════════════════════

  (import "edgerun" "is_alpha" (func $is_alpha (param i32) (result i32)))
  (import "edgerun" "is_digit" (func $is_digit (param i32) (result i32)))

  (memory (export "memory") 1)

  ;; Valid DNS label byte: ALPHA, DIGIT, '-' (0x2D), or '_' (0x5F).
  (func (export "is_label_byte") (param $b i32) (result i32)
    (i32.or
      (i32.or
        (call $is_alpha (local.get $b))
        (call $is_digit (local.get $b)))
      (i32.or
        (i32.eq (local.get $b) (i32.const 45))
        (i32.eq (local.get $b) (i32.const 95)))))
