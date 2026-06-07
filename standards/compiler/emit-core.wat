  ;; ── Shared emit core — parameterized byte/dword/qword helpers ──────
  ;;   $code_ptr: mutable i32 global storing the current code offset
  ;;
  ;;   Each arch file defines $emit_{arch}_byte / _dword / _qword wrappers
  ;;   that call these core functions with the arch-specific global.

  (func $emit_byte (param $b i32) (param $code_ptr i32)
    (local $p i32)
    (local.set $p (i32.add (global.get $JIT_CACHE) (i32.load (local.get $code_ptr))))
    (i32.store8 (local.get $p) (local.get $b))
    (i32.store (local.get $code_ptr) (i32.add (i32.load (local.get $code_ptr)) (i32.const 1))))

  (func $emit_dword (param $v i32) (param $code_ptr i32)
    (call $emit_byte (i32.and (local.get $v) (i32.const 0xFF)) (local.get $code_ptr))
    (call $emit_byte (i32.and (i32.shr_u (local.get $v) (i32.const 8)) (i32.const 0xFF)) (local.get $code_ptr))
    (call $emit_byte (i32.and (i32.shr_u (local.get $v) (i32.const 16)) (i32.const 0xFF)) (local.get $code_ptr))
    (call $emit_byte (i32.and (i32.shr_u (local.get $v) (i32.const 24)) (i32.const 0xFF)) (local.get $code_ptr)))

  (func $emit_qword (param $v i64) (param $code_ptr i32)
    (call $emit_dword (i32.wrap_i64 (local.get $v)) (local.get $code_ptr))
    (call $emit_dword (i32.wrap_i64 (i64.shr_u (local.get $v) (i64.const 32))) (local.get $code_ptr)))
