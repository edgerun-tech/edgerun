;; ── Canonical crypto helpers ──
;; Extracted from duplicate definitions across crypto-aes128-gcm, crypto-aes-ctr,
;; crypto-hmac-sha256, crypto-sha1, crypto-sha256, crypto-aes-block, crypto-bigint-limb, crypto-xtea.
;; Include BEFORE any other crypto fragment.

;; Load big-endian 32-bit word from ptr
(func $load_be32 (export "load_be32") (param $p i32) (result i32)
  local.get $p i32.load8_u i32.const 24 i32.shl
  local.get $p i32.const 1 i32.add i32.load8_u i32.const 16 i32.shl i32.or
  local.get $p i32.const 2 i32.add i32.load8_u i32.const 8 i32.shl i32.or
  local.get $p i32.const 3 i32.add i32.load8_u i32.or)

;; Store big-endian 32-bit word to ptr
(func $store_be32 (export "store_be32") (param $p i32) (param $v i32)
  local.get $p i32.const 0 i32.add local.get $v i32.const 24 i32.shr_u i32.store8
  local.get $p i32.const 1 i32.add local.get $v i32.const 16 i32.shr_u i32.const 0xFF i32.and i32.store8
  local.get $p i32.const 2 i32.add local.get $v i32.const 8 i32.shr_u i32.const 0xFF i32.and i32.store8
  local.get $p i32.const 3 i32.add local.get $v i32.const 0xFF i32.and i32.store8)

;; Store big-endian 64-bit value to ptr
(func $store_be64 (export "store_be64") (param $p i32) (param $v i64)
  local.get $p i32.const 0 i32.add local.get $v i64.const 56 i64.shr_u i64.store8
  local.get $p i32.const 1 i32.add local.get $v i64.const 48 i64.shr_u i64.const 0xFF i64.and i64.store8
  local.get $p i32.const 2 i32.add local.get $v i64.const 40 i64.shr_u i64.const 0xFF i64.and i64.store8
  local.get $p i32.const 3 i32.add local.get $v i64.const 32 i64.shr_u i64.const 0xFF i64.and i64.store8
  local.get $p i32.const 4 i32.add local.get $v i64.const 24 i64.shr_u i64.const 0xFF i64.and i64.store8
  local.get $p i32.const 5 i32.add local.get $v i64.const 16 i64.shr_u i64.const 0xFF i64.and i64.store8
  local.get $p i32.const 6 i32.add local.get $v i64.const 8 i64.shr_u i64.const 0xFF i64.and i64.store8
  local.get $p i32.const 7 i32.add local.get $v i64.const 0xFF i64.and i64.store8)

;; Range check: ptr + len does not overflow and fits within max_addr
(func $range_ok (export "range_ok") (param $ptr i32) (param $len i32) (param $max_addr i32) (result i32)
  (local $end i32)
  local.get $ptr local.get $len i32.add local.set $end
  local.get $end local.get $ptr i32.lt_u if i32.const 0 return end
  local.get $end local.get $max_addr i32.le_u)
