  ;; EdgeRun shared runtime core — owns linear memory, exports shared helpers.
  ;; All other modules import memory + helpers from here.

  (memory (export "memory") 256)

  ;; Character classification LUT at 0x1000 (256 bytes)
  ;; bit 0: digit, bit 1: uppercase, bit 2: lowercase, bit 3: tchar,
  ;; bit 4: hex, bit 5: ws, bit 6: scheme, bit 7: dns-label
  (data (i32.const 0x1000) "\00\00\00\00\00\00\00\00\00\20\20\00\00\20\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\20\08\00\08\08\08\08\08\00\00\08\48\00\c8\48\00\d9\d9\d9\d9\d9\d9\d9\d9\d9\d9\00\00\00\00\00\00\00\da\da\da\da\da\da\ca\ca\ca\ca\ca\ca\ca\ca\ca\ca\ca\ca\ca\ca\ca\ca\ca\ca\ca\ca\00\00\00\08\08\08\dc\dc\dc\dc\dc\dc\cc\cc\cc\cc\cc\cc\cc\cc\cc\cc\cc\cc\cc\cc\cc\cc\cc\cc\cc\cc\00\08\00\08\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00")

  ;; Lowercase mapping LUT at 0x2000 (256 bytes)
  (data (i32.const 0x2000) "\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\61\62\63\64\65\66\67\68\69\6a\6b\6c\6d\6e\6f\70\71\72\73\74\75\76\77\78\79\7a\00\00\00\00\00\00\61\62\63\64\65\66\67\68\69\6a\6b\6c\6d\6e\6f\70\71\72\73\74\75\76\77\78\79\7a\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00")

  ;; ── Shared memory layout constants (imported by interpreter + compiler) ──
  (global $OFF_TYPES_BUF (export "OFF_TYPES_BUF") i32 (i32.const 264))
  (global $OFF_CODE_BUF (export "OFF_CODE_BUF") i32 (i32.const 21792))
  (global $OFF_FUNCTIONS_BUF (export "OFF_FUNCTIONS_BUF") i32 (i32.const 17688))
  (global $OFF_DECODED_OPS (export "OFF_DECODED_OPS") i32 (i32.const 0xA0000))
  (global $OFF_DECODED_COUNT (export "OFF_DECODED_COUNT") i32 (i32.const 89864))
  (global $DEC_SZ (export "DEC_SZ") i32 (i32.const 32))
  (global $SZ_TYPE (export "SZ_TYPE") i32 (i32.const 256))
  (global $SZ_FUNC (export "SZ_FUNC") i32 (i32.const 16))
  (global $SZ_CODE (export "SZ_CODE") i32 (i32.const 64))

  ;; ── Character classification helpers ──

  (func $char_class (export "char_class") (param $b i32) (result i32)
    (i32.load8_u offset=0x1000 (local.get $b)))

  (func $is_digit (export "is_digit") (param $b i32) (result i32)
    (i32.and (call $char_class (local.get $b)) (i32.const 1)))

  (func $is_upper (export "is_upper") (param $b i32) (result i32)
    (i32.and (call $char_class (local.get $b)) (i32.const 2)))

  (func $is_lower (export "is_lower") (param $b i32) (result i32)
    (i32.and (call $char_class (local.get $b)) (i32.const 4)))

  (func $is_alpha (export "is_alpha") (param $b i32) (result i32)
    (i32.and (call $char_class (local.get $b)) (i32.const 6)))

  (func $is_tchar (export "is_tchar") (param $b i32) (result i32)
    (i32.and (call $char_class (local.get $b)) (i32.const 8)))

  (func $is_hex (export "is_hex") (param $b i32) (result i32)
    (i32.and (call $char_class (local.get $b)) (i32.const 16)))

  (func $is_ws (export "is_ws") (param $b i32) (result i32)
    (i32.and (call $char_class (local.get $b)) (i32.const 32)))

  (func $is_scheme_byte (export "is_scheme_byte") (param $b i32) (result i32)
    (i32.and (call $char_class (local.get $b)) (i32.const 64)))

  (func $is_label_byte (export "is_label_byte") (param $b i32) (result i32)
    (i32.and (call $char_class (local.get $b)) (i32.const 128)))

  (func $to_lower (export "to_lower") (param $b i32) (result i32)
    (local $cl i32)
    (local.set $cl (call $char_class (local.get $b)))
    (if (i32.and (local.get $cl) (i32.const 2))
      (then (return (i32.or (local.get $b) (i32.const 32)))))
    (if (i32.and (local.get $cl) (i32.const 4))
      (then (return (local.get $b))))
    (i32.const 0))

  (func $to_upper (export "to_upper") (param $b i32) (result i32)
    (if (i32.and (call $char_class (local.get $b)) (i32.const 4))
      (then (return (i32.sub (local.get $b) (i32.const 32)))))
    (local.get $b))

  (func $is_alnum (export "is_alnum") (param $b i32) (result i32)
    (i32.and (call $char_class (local.get $b)) (i32.const 7)))

  (func $is_print (export "is_print") (param $b i32) (result i32)
    (i32.and
      (i32.ge_u (local.get $b) (i32.const 32))
      (i32.le_u (local.get $b) (i32.const 126))))

  (func $is_cont (export "is_cont") (param $b i32) (result i32)
    (i32.eq (i32.and (local.get $b) (i32.const 0xC0)) (i32.const 0x80)))

  ;; ── Pack/unpack helpers ──

  (func $pack (export "pack") (param $status i32) (param $value i32) (result i64)
    (i64.or
      (i64.extend_i32_u (local.get $value))
      (i64.shl (i64.extend_i32_u (local.get $status)) (i64.const 32))))

  (func $pack_u16 (export "pack_u16") (param $a i32) (param $b i32) (result i32)
    (i32.or
      (local.get $b)
      (i32.shl (local.get $a) (i32.const 8))))

  (func $byte (export "byte") (param $v i32) (param $i i32) (result i32)
    (i32.and
      (i32.shr_u (local.get $v) (i32.shl (local.get $i) (i32.const 3)))
      (i32.const 0xFF)))

  (func $has (export "has") (param $v i32) (param $mask i32) (result i32)
    (i32.ne (i32.and (local.get $v) (local.get $mask)) (i32.const 0)))

  ;; ── Shared memcpy ──
  (func $memcpy (export "memcpy") (param $dst i32) (param $src i32) (param $len i32)
    (local $i i32)
    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $i) (local.get $len)))
        (i32.store8
          (i32.add (local.get $dst) (local.get $i))
          (i32.load8_u (i32.add (local.get $src) (local.get $i))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop))))

  (func $memcpy_off (export "memcpy_off") (param $dst i32) (param $doff i32) (param $src i32) (param $soff i32) (param $len i32)
    (local $i i32)
    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $i) (local.get $len)))
        (i32.store8
          (i32.add (i32.add (local.get $dst) (local.get $doff)) (local.get $i))
          (i32.load8_u
            (i32.add (i32.add (local.get $src) (local.get $soff)) (local.get $i))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop))))

  ;; ── SIMD stubs ──

  (func $simd_memchr (export "simd_memchr") (param $ptr i32) (param $len i32) (param $byte i32) (result i32)
    (local $i i32) (local $vec v128) (local $cmp v128) (local $mask i32)
    (local.set $i (local.get $ptr))
    (block $done
      (loop $loop
        (br_if $done (i32.lt_u (local.get $len) (i32.const 16)))
        (local.set $vec (v128.load (local.get $i)))
        (local.set $cmp (i8x16.eq (local.get $vec) (i8x16.splat (i32.wrap_i64 (i64.extend_i32_u (local.get $byte))))))
        (local.set $mask (i32x4.bitmask (local.get $cmp)))
        (if (local.get $mask)
          (then (return (i32.add (local.get $i) (i32.ctz (local.get $mask))))))
        (local.set $i (i32.add (local.get $i) (i32.const 16)))
        (local.set $len (i32.sub (local.get $len) (i32.const 16)))
        (br $loop)))
    (block $r_done
      (loop $r_loop
        (br_if $r_done (i32.eqz (local.get $len)))
        (if (i32.eq (i32.load8_u (local.get $i)) (local.get $byte))
          (then (return (local.get $i))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (local.set $len (i32.sub (local.get $len) (i32.const 1)))
        (br $r_loop)))
    (i32.const -1))

  (func $simd_memrchr (export "simd_memrchr") (param $ptr i32) (param $len i32) (param $byte i32) (result i32)
    (local $i i32) (local $end i32) (local $vec v128) (local $cmp v128) (local $mask i32)
    (local.set $end (i32.add (local.get $ptr) (local.get $len)))
    (local.set $i (i32.sub (local.get $end) (i32.const 16)))
    (block $done
      (loop $loop
        (br_if $done (i32.lt_u (local.get $i) (local.get $ptr)))
        (local.set $vec (v128.load (local.get $i)))
        (local.set $cmp (i8x16.eq (local.get $vec) (i8x16.splat (i32.wrap_i64 (i64.extend_i32_u (local.get $byte))))))
        (local.set $mask (i32x4.bitmask (local.get $cmp)))
        (if (local.get $mask)
          (then (return (i32.add (local.get $i) (i32.ctz (local.get $mask))))))
        (local.set $i (i32.sub (local.get $i) (i32.const 16)))
        (br $loop)))
    (local.set $i (i32.sub (local.get $end) (i32.const 1)))
    (block $r_done
      (loop $r_loop
        (br_if $r_done (i32.lt_u (local.get $i) (local.get $ptr)))
        (if (i32.eq (i32.load8_u (local.get $i)) (local.get $byte))
          (then (return (local.get $i))))
        (local.set $i (i32.sub (local.get $i) (i32.const 1)))
        (br $r_loop)))
    (i32.const -1))

  ;; ── Shared status codes (all modules should use these) ──
  (global $STATUS_OK           (export "STATUS_OK")           i32 (i32.const 0))
  (global $STATUS_INPUT_SHORT  (export "STATUS_INPUT_SHORT")  i32 (i32.const 1))
  (global $STATUS_OUTPUT_SHORT (export "STATUS_OUTPUT_SHORT") i32 (i32.const 2))
  (global $STATUS_INVALID      (export "STATUS_INVALID")      i32 (i32.const 3))
  (global $STATUS_OVERFLOW     (export "STATUS_OVERFLOW")     i32 (i32.const 4))
  (global $STATUS_TRUNCATED    (export "STATUS_TRUNCATED")    i32 (i32.const 5))
  (global $STATUS_TOO_LONG     (export "STATUS_TOO_LONG")     i32 (i32.const 6))
  (global $STATUS_MORE         (export "STATUS_MORE")         i32 (i32.const 7))
  (global $STATUS_TIMEOUT      (export "STATUS_TIMEOUT")      i32 (i32.const 8))

  ;; ── Global epoch — shared tick counter across all modules ──
  ;; Schedulers increment this before driving pipelines. Stages read it
  ;; for cross-pipeline synchronization and tick-based timeouts.
  (global $epoch (export "epoch") (mut i32) (i32.const 0))

  ;; ── Bounds check ──
  ;; Returns 1 if offset + need <= len, 0 otherwise.
  (func $bounds_check (export "bounds_check")
    (param $len i32) (param $offset i32) (param $need i32) (result i32)
    (if (result i32)
      (i32.lt_u (local.get $len) (local.get $need))
      (then (i32.const 0))
      (else
        (i32.le_u
          (local.get $offset)
          (i32.sub (local.get $len) (local.get $need))))))

  ;; ── Shared big-endian read helpers (returns i64: status<<32 | value) ──

  (func $read_u16_be (export "read_u16_be")
    (param $ptr i32) (param $len i32) (param $offset i32) (result i64)
    (if (result i64)
      (call $bounds_check (local.get $len) (local.get $offset) (i32.const 2))
      (then
        (call $pack (i32.const 0)
          (i32.or
            (i32.shl (i32.load8_u (i32.add (local.get $ptr) (local.get $offset))) (i32.const 8))
            (i32.load8_u (i32.add (local.get $ptr) (i32.add (local.get $offset) (i32.const 1)))))))
      (else (call $pack (i32.const 1) (i32.const 0)))))

  (func $read_u24_be (export "read_u24_be")
    (param $ptr i32) (param $len i32) (param $offset i32) (result i64)
    (if (result i64)
      (call $bounds_check (local.get $len) (local.get $offset) (i32.const 3))
      (then
        (call $pack (i32.const 0)
          (i32.or
            (i32.or
              (i32.shl (i32.load8_u (i32.add (local.get $ptr) (local.get $offset))) (i32.const 16))
              (i32.shl (i32.load8_u (i32.add (local.get $ptr) (i32.add (local.get $offset) (i32.const 1)))) (i32.const 8)))
            (i32.load8_u (i32.add (local.get $ptr) (i32.add (local.get $offset) (i32.const 2)))))))
      (else (call $pack (i32.const 1) (i32.const 0)))))

  (func $read_u32_be (export "read_u32_be")
    (param $ptr i32) (param $len i32) (param $offset i32) (result i64)
    (if (result i64)
      (call $bounds_check (local.get $len) (local.get $offset) (i32.const 4))
      (then
        (call $pack (i32.const 0)
          (i32.or
            (i32.or
              (i32.or
                (i32.shl (i32.load8_u (i32.add (local.get $ptr) (local.get $offset))) (i32.const 24))
                (i32.shl (i32.load8_u (i32.add (local.get $ptr) (i32.add (local.get $offset) (i32.const 1)))) (i32.const 16)))
              (i32.shl (i32.load8_u (i32.add (local.get $ptr) (i32.add (local.get $offset) (i32.const 2)))) (i32.const 8)))
            (i32.load8_u (i32.add (local.get $ptr) (i32.add (local.get $offset) (i32.const 3)))))))
      (else (call $pack (i32.const 1) (i32.const 0)))))

  ;; ── LEB128 decoders ──

  (func $read_leb128_u (export "read_leb128_u")
    (param $ptr i32) (param $len i32) (param $offset i32) (result i64)
    (local $pos i32) (local $val i64) (local $b i32) (local $shift i32)
    (local.set $pos (local.get $offset))
    (local.set $val (i64.const 0))
    (local.set $shift (i32.const 0))
    (block $done
      (loop $loop
        (if (i32.ge_u (local.get $pos) (local.get $len))
          (then
            (return (call $pack (i32.const 1) (i32.wrap_i64 (local.get $val))))))
        (local.set $b (i32.load8_u (i32.add (local.get $ptr) (local.get $pos))))
        (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
        (local.set $val
          (i64.or (local.get $val)
            (i64.shl (i64.extend_i32_u (i32.and (local.get $b) (i32.const 0x7f))) (i64.extend_i32_u (local.get $shift)))))
        (local.set $shift (i32.add (local.get $shift) (i32.const 7)))
        (if (i32.eqz (i32.and (local.get $b) (i32.const 0x80)))
          (then
            (return (call $pack (i32.const 0)
              (i32.or (local.get $pos) (i32.shl (i32.wrap_i64 (local.get $val)) (i32.const 8)))))))
        (br $loop)))
    (call $pack (i32.const 5) (i32.const 0)))

  (func $read_leb128_s (export "read_leb128_s")
    (param $ptr i32) (param $len i32) (param $offset i32) (result i64)
    (local $pos i32) (local $val i64) (local $b i32) (local $shift i32)
    (local.set $pos (local.get $offset))
    (local.set $val (i64.const 0))
    (local.set $shift (i32.const 0))
    (block $done
      (loop $loop
        (if (i32.ge_u (local.get $pos) (local.get $len))
          (then
            (return (call $pack (i32.const 1) (i32.wrap_i64 (local.get $val))))))
        (local.set $b (i32.load8_u (i32.add (local.get $ptr) (local.get $pos))))
        (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
        (local.set $val
          (i64.or (local.get $val)
            (i64.shl (i64.extend_i32_u (i32.and (local.get $b) (i32.const 0x7f))) (i64.extend_i32_u (local.get $shift)))))
        (local.set $shift (i32.add (local.get $shift) (i32.const 7)))
        (if (i32.eqz (i32.and (local.get $b) (i32.const 0x80)))
          (then
            (if (i32.and (local.get $b) (i32.const 0x40))
              (then
                (local.set $val
                  (i64.or (local.get $val)
                    (i64.shl (i64.const -1) (i64.extend_i32_u (local.get $shift)))))))
            (return (call $pack (i32.const 0)
              (i32.or (local.get $pos) (i32.shl (i32.wrap_i64 (local.get $val)) (i32.const 8)))))))
        (br $loop)))
    (call $pack (i32.const 5) (i32.const 0)))

  ;; ── Protocol exports ──

  (func $proto_abi_version (export "proto_abi_version") (result i32)
    (i32.const 2))

  (func $proto_standard_id (export "proto_standard_id") (result i32)
    (i32.const 0))

  (func $simd_capabilities (export "simd_capabilities") (result i32)
    (i32.const 1))
