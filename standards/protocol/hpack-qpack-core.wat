  Appended qpack_prefix_decode and prefix_encode to hpack-qpack-core.wat
(module
  ;; ═════════════════════════════════════════════════════════════════════
  ;; HPACK/QPACK Core Primitives — shared Huffman tables and decode
  ;; ═════════════════════════════════════════════════════════════════════

  (memory (export "memory") 1)

  ;; Huffman code table (257 entries × 4 bytes = 1028 bytes at 60000)
  (data (i32.const 60000) "\f8\1f\00\00\d8\ff\7f\00\e2\ff\ff\0f\e3\ff\ff\0f\e4\ff\ff\0f\e5\ff\ff\0f\e6\ff\ff\0f\e7\ff\ff\0f\e8\ff\ff\0f\ea\ff\ff\00\fc\ff\ff\3f\e9\ff\ff\0f\ea\ff\ff\0f\fd\ff\ff\3f\eb\ff\ff\0f\ec\ff\ff\0f\ed\ff\ff\0f\ee\ff\ff\0f\ef\ff\ff\0f\f0\ff\ff\0f\f1\ff\ff\0f\f2\ff\ff\0f\fe\ff\ff\3f\f3\ff\ff\0f\f4\ff\ff\0f\f5\ff\ff\0f\f6\ff\ff\0f\f7\ff\ff\0f\f8\ff\ff\0f\f9\ff\ff\0f\fa\ff\ff\0f\fb\ff\ff\0f\14\00\00\00\f8\03\00\00\f9\03\00\00\fa\0f\00\00\f9\1f\00\00\15\00\00\00\f8\00\00\00\fa\07\00\00\fa\03\00\00\fb\03\00\00\f9\00\00\00\fb\07\00\00\fa\00\00\00\16\00\00\00\17\00\00\00\18\00\00\00\00\00\00\00\01\00\00\00\02\00\00\00\19\00\00\00\1a\00\00\00\1b\00\00\00\1c\00\00\00\1d\00\00\00\1e\00\00\00\1f\00\00\00\5c\00\00\00\fb\00\00\00\fc\7f\00\00\20\00\00\00\fb\0f\00\00\fc\03\00\00\fa\1f\00\00\21\00\00\00\5d\00\00\00\5e\00\00\00\5f\00\00\00\60\00\00\00\61\00\00\00\62\00\00\00\63\00\00\00\64\00\00\00\65\00\00\00\66\00\00\00\67\00\00\00\68\00\00\00\69\00\00\00\6a\00\00\00\6b\00\00\00\6c\00\00\00\6d\00\00\00\6e\00\00\00\6f\00\00\00\70\00\00\00\71\00\00\00\72\00\00\00\fc\00\00\00\73\00\00\00\fd\00\00\00\fb\1f\00\00\f0\ff\07\00\fc\1f\00\00\fc\3f\00\00\22\00\00\00\fd\7f\00\00\03\00\00\00\23\00\00\00\04\00\00\00\24\00\00\00\05\00\00\00\25\00\00\00\26\00\00\00\27\00\00\00\06\00\00\00\74\00\00\00\75\00\00\00\28\00\00\00\29\00\00\00\2a\00\00\00\07\00\00\00\2b\00\00\00\76\00\00\00\2c\00\00\00\08\00\00\00\09\00\00\00\2d\00\00\00\77\00\00\00\78\00\00\00\79\00\00\00\7a\00\00\00\7b\00\00\00\fe\7f\00\00\fc\07\00\00\fd\3f\00\00\fd\1f\00\00\fc\ff\ff\0f\e6\ff\0f\00\d2\ff\3f\00\e7\ff\0f\00\e8\ff\0f\00\d3\ff\3f\00\d4\ff\3f\00\d5\ff\3f\00\d9\ff\7f\00\d6\ff\3f\00\da\ff\7f\00\db\ff\7f\00\dc\ff\7f\00\dd\ff\7f\00\de\ff\7f\00\eb\ff\ff\00\df\ff\7f\00\ec\ff\ff\00\ed\ff\ff\00\d7\ff\3f\00\e0\ff\7f\00\ee\ff\ff\00\e1\ff\7f\00\e2\ff\7f\00\e3\ff\7f\00\e4\ff\7f\00\dc\ff\1f\00\d8\ff\3f\00\e5\ff\7f\00\d9\ff\3f\00\e6\ff\7f\00\e7\ff\7f\00\ef\ff\ff\00\da\ff\3f\00\dd\ff\1f\00\e9\ff\0f\00\db\ff\3f\00\dc\ff\0f\00\e8\ff\0f\00\e9\ff\0f\00\e0\ff\0f\00\dd\ff\0f\00\ed\ff\0f\00\ea\ff\0f\00\eb\ff\0f\00\ee\ff\0f\00\ec\ff\0f\00\ef\ff\0f\00")

  ;; Huffman bit-length table (257 bytes at 61024)
  (data (i32.const 61024) "\0d\17\1c\1c\1c\1c\1c\1c\1c\18\1e\1c\1c\1e\1c\1c\1c\1c\1c\1c\1c\1c\1e\1c\1c\1c\1c\1c\1c\1c\1c\1c\06\0a\0a\0c\0d\06\08\0b\0a\0a\08\0b\08\06\06\06\05\05\05\06\06\06\06\06\06\06\07\08\0f\06\0c\0a\0d\06\07\07\07\07\07\07\07\07\07\07\07\07\07\07\07\07\07\07\07\07\07\07\08\07\08\0d\13\0d\0e\06\0f\05\06\05\06\05\06\06\06\05\07\07\06\06\06\05\06\07\06\05\05\06\07\07\07\07\07\0f\0b\0e\0d\1c\14\16\14\14\16\16\16\17\16\17\17\17\17\17\18\17\18\18\16\17\18\17\17\17\17\15\16\17\16\17\17\18\16\15\14\16\16\17\17\15\17\16\16\18\15\16\17\17\15\15\16\15\17\16\17\17\14\16\16\16\17\16\16\17\1a\1a\14\13\16\17\16\19\1a\1a\1a\1b\1b\1a\18\19\13\15\1a\1b\1b\1a\1b\18\15\15\1a\1a\1c\1b\1b\1b\14\18\14\15\16\15\15\17\16\16\19\19\18\18\1a\17\1a\1b\1a\1a\1b\1b\1b\1b\1b\1c\1b\1b\1b\1b\1b\1a\1e")

  ;; Look up a Huffman code value in the table.
  ;; Returns symbol index (0-256) or -1 if not found.
  (func $huff_lookup (export "huff_lookup") (param $code i32) (param $bits i32) (result i32)
    (local $i i32)
    (loop $again
      (if (i32.gt_u (local.get $i) (i32.const 256))
        (then (return (i32.const -1))))
      (if
        (i32.and
          (i32.eq
            (i32.load8_u (i32.add (i32.const 61024) (local.get $i)))
            (local.get $bits))
          (i32.eq
            (i32.load (i32.add (i32.const 60000) (i32.mul (local.get $i) (i32.const 4))))
            (local.get $code)))
        (then (return (local.get $i))))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br $again))
    (i32.const -1))

  ;; Decode a Huffman-encoded byte stream.
  ;; If write=1, writes decoded bytes to out_ptr (up to out_cap).
  ;; Returns i64: low=status, high=written count.
  (func $huffman_decode_internal (export "huffman_decode_internal")
    (param $in_ptr i32) (param $in_len i32) (param $out_ptr i32) (param $out_cap i32) (param $write i32)
    (result i64)
    (local $pos i32)
    (local $bit i32)
    (local $byte i32)
    (local $current i32)
    (local $current_len i32)
    (local $sym i32)
    (local $written i32)
    (loop $bytes
      (if (i32.ge_u (local.get $pos) (local.get $in_len))
        (then
          (if (i32.gt_u (local.get $current_len) (i32.const 0))
            (then
              (if (i32.gt_u (local.get $current_len) (i32.const 7))
                (then (return (call $pack (i32.const 3) (local.get $written)))))
              (if
                (i32.ne
                  (local.get $current)
                  (i32.sub (i32.shl (i32.const 1) (local.get $current_len)) (i32.const 1)))
                (then (return (call $pack (i32.const 3) (local.get $written)))))))
          (return (call $pack (i32.const 0) (local.get $written)))))
      (local.set $byte (i32.load8_u (i32.add (local.get $in_ptr) (local.get $pos))))
      (local.set $bit (i32.const 7))
      (loop $bits
        (local.set $current
          (i32.or
            (i32.shl (local.get $current) (i32.const 1))
            (i32.and (i32.shr_u (local.get $byte) (local.get $bit)) (i32.const 1))))
        (local.set $current_len (i32.add (local.get $current_len) (i32.const 1)))
        (if (i32.le_u (local.get $current_len) (i32.const 30))
          (then
            (local.set $sym (call $huff_lookup (local.get $current) (local.get $current_len)))
            (if (i32.ge_s (local.get $sym) (i32.const 0))
              (then
                (if (i32.eq (local.get $sym) (i32.const 256))
                  (then (return (call $pack (i32.const 3) (local.get $written)))))
                (if (local.get $write)
                  (then
                    (if (i32.ge_u (local.get $written) (local.get $out_cap))
                      (then (return (call $pack (i32.const 2) (local.get $written)))))
                    (i32.store8 (i32.add (local.get $out_ptr) (local.get $written)) (local.get $sym))))
                (local.set $written (i32.add (local.get $written) (i32.const 1)))
                (local.set $current (i32.const 0))
                (local.set $current_len (i32.const 0))))))
        (if (i32.eqz (local.get $bit))
          (then)
          (else
            (local.set $bit (i32.sub (local.get $bit) (i32.const 1)))
            (br $bits))))
      (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
      (br $bytes))
    (call $pack (i32.const 3) (local.get $written)))

  (import "math" "prefix_mask" (func $prefix_mask (param i32) (result i32)))

  ;; Decode a QPACK prefix-encoded integer.
  ;; Prefix output record, little-endian:
  ;; 0:u32 flags_high_bits, 4:u32 consumed, 8:u64 value.
  (func $qpack_prefix_decode (export "qpack_prefix_decode")
    (param $in_ptr i32) (param $in_len i32) (param $prefix_bits i32) (param $out_ptr i32)
    (result i32)
    (local $first i32)
    (local $mask i32)
    (local $flags i32)
    (local $value i64)
    (local $consumed i32)
    (local $shift i32)
    (local $byte i32)
    (if
      (i32.or
        (i32.lt_u (local.get $prefix_bits) (i32.const 1))
        (i32.gt_u (local.get $prefix_bits) (i32.const 8)))
      (then (return (i32.const 3))))
    (if (i32.eqz (local.get $in_len))
      (then (return (i32.const 1))))
    (local.set $mask (call $prefix_mask (local.get $prefix_bits)))
    (local.set $first (i32.load8_u (local.get $in_ptr)))
    (local.set $flags (i32.shr_u (local.get $first) (local.get $prefix_bits)))
    (local.set $value (i64.extend_i32_u (i32.and (local.get $first) (local.get $mask))))
    (if (i64.lt_u (local.get $value) (i64.extend_i32_u (local.get $mask)))
      (then
        (i32.store (local.get $out_ptr) (local.get $flags))
        (i32.store (i32.add (local.get $out_ptr) (i32.const 4)) (i32.const 1))
        (i64.store (i32.add (local.get $out_ptr) (i32.const 8)) (local.get $value))
        (return (i32.const 0))))
    (local.set $consumed (i32.const 1))
    (loop $again
      (if (i32.ge_u (local.get $consumed) (i32.const 10))
        (then (return (i32.const 4))))
      (if (i32.ge_u (local.get $consumed) (local.get $in_len))
        (then (return (i32.const 1))))
      (local.set $byte (i32.load8_u (i32.add (local.get $in_ptr) (local.get $consumed))))
      (if
        (i32.and
          (i32.ge_u (local.get $shift) (i32.const 63))
          (i32.ne (i32.and (local.get $byte) (i32.const 127)) (i32.const 0)))
        (then (return (i32.const 4))))
      (local.set $value
        (i64.add
          (local.get $value)
          (i64.shl
            (i64.extend_i32_u (i32.and (local.get $byte) (i32.const 127)))
            (i64.extend_i32_u (local.get $shift)))))
      (local.set $consumed (i32.add (local.get $consumed) (i32.const 1)))
      (if (i32.eqz (i32.and (local.get $byte) (i32.const 128)))
        (then
          (i32.store (local.get $out_ptr) (local.get $flags))
          (i32.store (i32.add (local.get $out_ptr) (i32.const 4)) (local.get $consumed))
          (i64.store (i32.add (local.get $out_ptr) (i32.const 8)) (local.get $value))
          (return (i32.const 0))))
      (local.set $shift (i32.add (local.get $shift) (i32.const 7)))
      (br $again))
    (i32.const 1))

  ;; Encode a prefix-encoded integer.
  (func $prefix_encode (export "prefix_encode")
    (param $value i64) (param $prefix_bits i32) (param $prefix_high_bits i32)
    (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (local $mask i32)
    (local $flags i32)
    (local $remaining i64)
    (local $written i32)
    (local $byte i32)
    (if
      (i32.or
        (i32.lt_u (local.get $prefix_bits) (i32.const 1))
        (i32.gt_u (local.get $prefix_bits) (i32.const 8)))
      (then (return (call $pack (i32.const 3) (i32.const 0)))))
    (if (i32.eqz (local.get $out_cap))
      (then (return (call $pack (i32.const 2) (i32.const 0)))))
    (local.set $mask (call $prefix_mask (local.get $prefix_bits)))
    (if
      (i32.ge_u
        (local.get $prefix_high_bits)
        (if (result i32)
          (i32.eq (local.get $prefix_bits) (i32.const 8))
          (then (i32.const 1))
          (else (i32.shl (i32.const 1) (i32.sub (i32.const 8) (local.get $prefix_bits))))))
      (then (return (call $pack (i32.const 3) (i32.const 0)))))
    (local.set $flags (i32.shl (local.get $prefix_high_bits) (local.get $prefix_bits)))
    (if (i64.lt_u (local.get $value) (i64.extend_i32_u (local.get $mask)))
      (then
        (i32.store8
          (local.get $out_ptr)
          (i32.or (local.get $flags) (i32.wrap_i64 (local.get $value))))
        (return (call $pack (i32.const 0) (i32.const 1)))))
    (i32.store8 (local.get $out_ptr) (i32.or (local.get $flags) (local.get $mask)))
    (local.set $remaining (i64.sub (local.get $value) (i64.extend_i32_u (local.get $mask))))
    (local.set $written (i32.const 1))
    (loop $again
      (if (i32.ge_u (local.get $written) (local.get $out_cap))
        (then (return (call $pack (i32.const 2) (local.get $written)))))
      (local.set $byte (i32.and (i32.wrap_i64 (local.get $remaining)) (i32.const 127)))
      (local.set $remaining (i64.shr_u (local.get $remaining) (i64.const 7)))
      (if (i64.ne (local.get $remaining) (i64.const 0))
        (then (local.set $byte (i32.or (local.get $byte) (i32.const 128)))))
      (i32.store8 (i32.add (local.get $out_ptr) (local.get $written)) (local.get $byte))
      (local.set $written (i32.add (local.get $written) (i32.const 1)))
      (br_if $again (i64.ne (local.get $remaining) (i64.const 0))))
    (call $pack (i32.const 0) (local.get $written)))



  ;; Table entry size: name_len + value_len + 32.
  ;; Returns packed low32=status, high32=size.
  (func $table_entry_size (export "table_entry_size")
    (param $name_len i32) (param $value_len i32)
    (result i64)
    (local $sum i32)
    (local.set $sum (i32.add (local.get $name_len) (local.get $value_len)))
    (if (i32.lt_u (local.get $sum) (local.get $name_len))
      (then (return (call $pack (i32.const 4) (i32.const 0)))))
    (local.set $sum (i32.add (local.get $sum) (i32.const 32)))
    (if (i32.lt_u (local.get $sum) (i32.const 32))
      (then (return (call $pack (i32.const 4) (i32.const 0)))))
    (call $pack (i32.const 0) (local.get $sum)))

  ;; ── Header block record helpers ────────────────────────────────────
  ;; Shared between hpack-header-block.wat and qpack-encoder-stream.wat.

  ;; Clear a 48-byte header block record at out_ptr (zeros all 6 i64 slots).
  (func $clear_record (export "clear_record") (param $out_ptr i32)
    (i64.store (local.get $out_ptr) (i64.const 0))
    (i64.store (i32.add (local.get $out_ptr) (i32.const 8)) (i64.const 0))
    (i64.store (i32.add (local.get $out_ptr) (i32.const 16)) (i64.const 0))
    (i64.store (i32.add (local.get $out_ptr) (i32.const 24)) (i64.const 0))
    (i64.store (i32.add (local.get $out_ptr) (i32.const 32)) (i64.const 0))
    (i64.store (i32.add (local.get $out_ptr) (i32.const 40)) (i64.const 0)))

  ;; Write a prefix record: kind, offset, then copy 3 i32 + 1 i64 from prefix_ptr.
  ;; Output layout: 0:u32 kind, 4:u32 offset, 8:u32, 12:u32 from prefix, 16:i64 from prefix.
  (func $write_prefix_record (export "write_prefix_record")
    (param $out_ptr i32) (param $kind i32) (param $offset i32) (param $prefix_ptr i32)
    (i32.store (local.get $out_ptr) (local.get $kind))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 4)) (local.get $offset))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 8)) (i32.load (i32.add (local.get $prefix_ptr) (i32.const 4))))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 12)) (i32.load (local.get $prefix_ptr)))
    (i64.store (i32.add (local.get $out_ptr) (i32.const 16)) (i64.load (i32.add (local.get $prefix_ptr) (i32.const 8)))))

  ;; Copy name metadata from meta_ptr to out_ptr+24.
  ;; Uses base_offset to relocate the name offset.
  (func $copy_name_meta (export "copy_name_meta")
    (param $out_ptr i32) (param $base_offset i32) (param $meta_ptr i32)
    (i32.store (i32.add (local.get $out_ptr) (i32.const 24))
      (i32.add (local.get $base_offset) (i32.load (i32.add (local.get $meta_ptr) (i32.const 12)))))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 28))
      (i32.load (i32.add (local.get $meta_ptr) (i32.const 16))))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 32))
      (i32.load (i32.add (local.get $meta_ptr) (i32.const 4)))))

  ;; Copy value metadata from meta_ptr to out_ptr+36.
  ;; Uses base_offset to relocate the value offset.
  (func $copy_value_meta (export "copy_value_meta")
    (param $out_ptr i32) (param $base_offset i32) (param $meta_ptr i32)
    (i32.store (i32.add (local.get $out_ptr) (i32.const 36))
      (i32.add (local.get $base_offset) (i32.load (i32.add (local.get $meta_ptr) (i32.const 12)))))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 40))
      (i32.load (i32.add (local.get $meta_ptr) (i32.const 16))))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 44))
      (i32.load (i32.add (local.get $meta_ptr) (i32.const 4)))))
)
