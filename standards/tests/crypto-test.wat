;; Crypto Test — validates SHA-256 K constants data table
;; Tests that the data table at 0x10100 contains correct values.
;; Must be run in the same module as crypto-sha256.wat.

(module
  (memory (export "memory") 4)

  ;; SHA-256 K constants data table (same as crypto-sha256.wat)
  (data (i32.const 0x10100) "\98\2f\8a\42\91\44\37\71\cf\fb\c0\b5\a5\db\b5\e9\5b\c2\56\39\f1\11\f1\59\a4\82\3f\92\d5\5e\1c\ab\98\aa\07\d8\01\5b\83\12\be\85\31\24\c3\7d\0c\55\74\5d\be\72\fe\b1\de\80\a7\06\dc\9b\74\f1\9b\c1\c1\69\9b\e4\86\47\be\ef\c6\9d\c1\0f\cc\a1\0c\24\6f\2c\e9\2d\aa\84\74\4a\dc\a9\b0\5c\da\88\f9\76\52\51\3e\98\6d\c6\31\a8\c8\27\03\b0\c7\7f\59\bf\f3\0b\e0\c6\47\91\a7\d5\51\63\ca\06\67\29\29\14\85\0a\b7\27\38\21\1b\2e\fc\6d\2c\4d\13\0d\38\53\54\73\0a\65\bb\0a\6a\76\2e\c9\c2\81\85\2c\72\92\a1\e8\bf\a2\4b\66\1a\a8\70\8b\4b\c2\a3\51\6c\c7\19\e8\92\d1\24\06\99\d6\85\35\0e\f4\70\a0\6a\10\16\c1\a4\19\08\6c\37\1e\4c\77\48\27\b5\bc\b0\34\b3\0c\1c\39\4a\aa\d8\4e\4f\ca\9c\5b\f3\6f\2e\68\ee\82\8f\74\6f\63\a5\78\14\78\c8\84\08\02\c7\8c\fa\ff\be\90\eb\6c\50\a4\f7\a3\f9\be\f2\78\71\c6")

  ;; Known SHA-256 K constants (first 6)
  (global $K0 i32 (i32.const 0x428a2f98))
  (global $K1 i32 (i32.const 0x71374491))
  (global $K2 i32 (i32.const 0xb5c0fbcf))
  (global $K3 i32 (i32.const 0xe9b5dba5))
  (global $K4 i32 (i32.const 0x3956c25b))
  (global $K63 i32 (i32.const 0xc67178f2))

  ;; load_be32: read big-endian i32 from memory (data table stores in LE)
  (func $load_i32_le (param $p i32) (result i32)
    local.get $p i32.load8_u
    local.get $p i32.const 1 i32.add i32.load8_u i32.const 8 i32.shl i32.or
    local.get $p i32.const 2 i32.add i32.load8_u i32.const 16 i32.shl i32.or
    local.get $p i32.const 3 i32.add i32.load8_u i32.const 24 i32.shl i32.or)

  (func (export "test_crypto") (result i32)
    (local $v i32)

    ;; Test K[0] = 0x428a2f98
    (local.set $v (call $load_i32_le (i32.const 0x10100)))
    (if (i32.ne (local.get $v) (global.get $K0)) (then (return (i32.const 1))))

    ;; Test K[1] = 0x71374491
    (local.set $v (call $load_i32_le (i32.const 0x10104)))
    (if (i32.ne (local.get $v) (global.get $K1)) (then (return (i32.const 2))))

    ;; Test K[2] = 0xb5c0fbcf
    (local.set $v (call $load_i32_le (i32.const 0x10108)))
    (if (i32.ne (local.get $v) (global.get $K2)) (then (return (i32.const 3))))

    ;; Test K[3] = 0xe9b5dba5
    (local.set $v (call $load_i32_le (i32.const 0x1010c)))
    (if (i32.ne (local.get $v) (global.get $K3)) (then (return (i32.const 4))))

    ;; Test K[4] = 0x3956c25b
    (local.set $v (call $load_i32_le (i32.const 0x10110)))
    (if (i32.ne (local.get $v) (global.get $K4)) (then (return (i32.const 5))))

    ;; Test K[63] = 0xc67178f2 (last entry at offset 63*4 = 0xFC)
    (local.set $v (call $load_i32_le (i32.const 0x101FC)))
    (if (i32.ne (local.get $v) (global.get $K63)) (then (return (i32.const 6))))

    ;; All passed
    i32.const 0)
)
