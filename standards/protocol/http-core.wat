(module
  ;; ═════════════════════════════════════════════════════════════════════
  ;; HTTP Core Primitives — shared across all HTTP parsers
  ;; ═════════════════════════════════════════════════════════════════════

  (import "edgerun" "to_lower" (func $to_lower (param i32) (result i32)))
  (import "edgerun" "is_alpha" (func $is_alpha (param i32) (result i32)))
  (import "edgerun" "is_digit" (func $is_digit (param i32) (result i32)))

  (memory (export "memory") 1)

  ;; ── Character classification ──────────────────────────────────────

  ;; RFC 7230 tchar: ALPHA / DIGIT / "!" / "#" / "$" / "%" / "&" / "'" /
  ;; "*" / "+" / "-" / "." / "^" / "_" / "`" / "|" / "~"
  (func $is_tchar (export "is_tchar") (param $b i32) (result i32)
    (i32.or
      (i32.or
        (i32.or
          (i32.and (i32.ge_u (local.get $b) (i32.const 65)) (i32.le_u (local.get $b) (i32.const 90)))
          (i32.and (i32.ge_u (local.get $b) (i32.const 97)) (i32.le_u (local.get $b) (i32.const 122))))
        (i32.and (i32.ge_u (local.get $b) (i32.const 48)) (i32.le_u (local.get $b) (i32.const 57))))
      (i32.or
        (i32.or
          (i32.or
            (i32.or (i32.eq (local.get $b) (i32.const 33)) (i32.eq (local.get $b) (i32.const 35)))
            (i32.or (i32.eq (local.get $b) (i32.const 36)) (i32.eq (local.get $b) (i32.const 37))))
          (i32.or
            (i32.or (i32.eq (local.get $b) (i32.const 38)) (i32.eq (local.get $b) (i32.const 39)))
            (i32.or (i32.eq (local.get $b) (i32.const 42)) (i32.eq (local.get $b) (i32.const 43)))))
        (i32.or
          (i32.or
            (i32.or (i32.eq (local.get $b) (i32.const 45)) (i32.eq (local.get $b) (i32.const 46)))
            (i32.or (i32.eq (local.get $b) (i32.const 94)) (i32.eq (local.get $b) (i32.const 95))))
          (i32.or
            (i32.or (i32.eq (local.get $b) (i32.const 96)) (i32.eq (local.get $b) (i32.const 124)))
            (i32.eq (local.get $b) (i32.const 126)))))))

  ;; HTTP whitespace: space (0x20) or horizontal tab (0x09).
  ;; Narrower than general is_ws (which also accepts CR, LF, etc.).
  (func $is_space (export "is_space") (param $b i32) (result i32)
    (i32.or (i32.eq (local.get $b) (i32.const 32)) (i32.eq (local.get $b) (i32.const 9))))

  ;; Valid header field value byte: tab (0x09) or visible ASCII [32,126].
  (func $is_header_value_byte (export "is_header_value_byte") (param $b i32) (result i32)
    (i32.or
      (i32.eq (local.get $b) (i32.const 9))
      (i32.and (i32.ge_u (local.get $b) (i32.const 32)) (i32.le_u (local.get $b) (i32.const 126)))))

  ;; ── Case-insensitive comparison ───────────────────────────────────

  ;; Compare two ASCII byte sequences case-insensitively.
  ;; Returns 1 if equal, 0 otherwise.
  (func $ascii_eq_ci (export "ascii_eq_ci") (param $aptr i32) (param $alen i32) (param $bptr i32) (param $blen i32) (result i32)
    (local $i i32)
    (if (i32.ne (local.get $alen) (local.get $blen)) (then (return (i32.const 0))))
    (loop $scan
      (if (i32.ge_u (local.get $i) (local.get $alen)) (then (return (i32.const 1))))
      (if (i32.ne
            (call $to_lower (i32.load8_u (i32.add (local.get $aptr) (local.get $i))))
            (call $to_lower (i32.load8_u (i32.add (local.get $bptr) (local.get $i)))))
        (then (return (i32.const 0))))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br $scan))
    i32.const 1))
