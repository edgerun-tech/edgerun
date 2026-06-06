  ;; ═════════════════════════════════════════════════════════════════════
  ;; Decode opcodes: translate raw bytecode to DecodedOp entries
  ;; Input: offset (in wasm bytes), len (bytecode length)
  ;; Output: decoded_start (index into decoded_ops cache), decoded_count
  ;; Stored to scratch0/scratch1
  ;; ═════════════════════════════════════════════════════════════════════
  (func $decode_opcodes (export "decode_opcodes") (param $offset i32) (param $len i32) (result i32)
    (local $end i32) (local $start_idx i32) (local $dc i32)
    (local $op i32) (local $p i32) (local $imm0 i32) (local $imm1 i32)
    (local $adv i32) (local $base i32)

    (local.set $end (i32.add (local.get $offset) (local.get $len)))
    (local.set $start_idx (i32.load (global.get $OFF_DECODED_COUNT)))  ;; running decoded op count
    (local.set $dc (local.get $start_idx))

    (block $lp
      (loop $cont
        (if (i32.ge_u (local.get $offset) (local.get $end)) (then (br $lp)))

        (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
        (local.set $op (i32.load8_u (i32.add (local.get $p) (local.get $offset))))
        (local.set $offset (i32.add (local.get $offset) (i32.const 1)))

        ;; Compute decoded op base in cache
        (local.set $base (i32.add (global.get $OFF_DECODED_OPS) (i32.mul (local.get $dc) (global.get $DEC_SZ))))
        ;; Store opcode
        (i32.store8 (local.get $base) (local.get $op))

        ;; Decode immediates based on opcode
        (block $op_handled
          ;; ── No immediate ops ──
          (if (i32.le_u (local.get $op) (i32.const 0x01))   ;; unreachable, nop
            (then (br $op_handled))
          )
          (if (i32.eq (local.get $op) (i32.const 0x0B))     ;; end
            (then (br $op_handled))
          )
          (if (i32.eq (local.get $op) (i32.const 0x0F))     ;; return
            (then (br $op_handled))
          )
          (if (i32.eq (local.get $op) (i32.const 0x1A))     ;; drop
            (then (br $op_handled))
          )
          (if (i32.eq (local.get $op) (i32.const 0x45))     ;; i32.eqz
            (then (br $op_handled))
          )
          ;; All comparison, arithmetic, conversion ops (0x46-0xC4 excl block/loop/if)
          (if (i32.and (i32.ge_u (local.get $op) (i32.const 0x46)) (i32.le_u (local.get $op) (i32.const 0xC4)))
            (then
              (if (i32.or (i32.eq (local.get $op) (i32.const 0x02)) (i32.eq (local.get $op) (i32.const 0x03)))
                (then)  ;; block/loop handled below
                (else (br $op_handled))
              )
            )
          )

          ;; ── LEB128 immediate ops ──
          (if (i32.or (i32.eq (local.get $op) (i32.const 0x0C)) (i32.eq (local.get $op) (i32.const 0x0D))) ;; br, br_if
            (then
              (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
              (i32.store (i32.add (local.get $base) (i32.const 4)) (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
              (br $op_handled)
            )
          )
          (if (i32.eq (local.get $op) (i32.const 0x10))     ;; call
            (then
              (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
              (i32.store (i32.add (local.get $base) (i32.const 4)) (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
              (br $op_handled)
            )
          )
          (if (i32.eq (local.get $op) (i32.const 0x11))     ;; call_indirect
            (then
              (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
              (i32.store (i32.add (local.get $base) (i32.const 4)) (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
              (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
              (i32.store (i32.add (local.get $base) (i32.const 8)) (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
              (br $op_handled)
            )
          )
          (if (i32.and (i32.ge_u (local.get $op) (i32.const 0x20)) (i32.le_u (local.get $op) (i32.const 0x26))) ;; local.get/set/tee/global.get/set, table.get/set
            (then
              (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
              (i32.store (i32.add (local.get $base) (i32.const 4)) (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
              (br $op_handled)
            )
          )
          (if (i32.eq (local.get $op) (i32.const 0x41))     ;; i32.const (signed LEB128)
            (then
              (if (call $leb_i32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
              (i32.store (i32.add (local.get $base) (i32.const 4)) (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
              (br $op_handled)
            )
          )
          (if (i32.eq (local.get $op) (i32.const 0x42))     ;; i64.const (signed LEB128)
            (then
              (if (call $leb_i64 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
              (i64.store (i32.add (local.get $base) (i32.const 4)) (i64.load (global.get $OFF_SCRATCH0)))
              (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH2))))
              (br $op_handled)
            )
          )
          (if (i32.eq (local.get $op) (i32.const 0x43))     ;; f32.const
            (then
              (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
              (i32.store (i32.add (local.get $base) (i32.const 4))
                (i32.load (i32.add (local.get $p) (local.get $offset)))
              )
              (local.set $offset (i32.add (local.get $offset) (i32.const 4)))
              (br $op_handled)
            )
          )
          (if (i32.eq (local.get $op) (i32.const 0x44))     ;; f64.const
            (then
              (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
              (i64.store (i32.add (local.get $base) (i32.const 4))
                (i64.load (i32.add (local.get $p) (local.get $offset)))
              )
              (local.set $offset (i32.add (local.get $offset) (i32.const 8)))
              (br $op_handled)
            )
          )

          ;; ── Memory ops (load/store): align + offset ──
          (if (i32.and (i32.ge_u (local.get $op) (i32.const 0x28)) (i32.le_u (local.get $op) (i32.const 0x3E)))
            (then
              ;; align (LEB128)
              (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
              (local.set $imm0 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
              ;; offset (LEB128)
              (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
              (local.set $imm1 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
              (i32.store (i32.add (local.get $base) (i32.const 4)) (local.get $imm0))  ;; align
              (i32.store (i32.add (local.get $base) (i32.const 8)) (local.get $imm1))  ;; mem offset
              (br $op_handled)
            )
          )

          ;; ── Block/loop/if: block type ──
          (if (i32.or (i32.eq (local.get $op) (i32.const 0x02)) (i32.eq (local.get $op) (i32.const 0x03)))
            (then
              ;; Block type: either empty(0x40), a value type byte, or a signed LEB128 type index
              (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
              (local.set $imm0 (i32.load8_u (i32.add (local.get $p) (local.get $offset))))
              (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
              (i32.store (i32.add (local.get $base) (i32.const 4)) (local.get $imm0))
              (br $op_handled)
            )
          )
          (if (i32.eq (local.get $op) (i32.const 0x04))     ;; if
            (then
              ;; Same as block
              (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
              (local.set $imm0 (i32.load8_u (i32.add (local.get $p) (local.get $offset))))
              (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
              (i32.store (i32.add (local.get $base) (i32.const 4)) (local.get $imm0))
              (br $op_handled)
            )
          )

          ;; ── br_table: count + labels + default ──
          (if (i32.eq (local.get $op) (i32.const 0x0E))
            (then
              ;; Read label count
              (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
              (i32.store (i32.add (local.get $base) (i32.const 4)) (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
              ;; Skip labels + default
              (local.set $imm0 (i32.load (i32.add (local.get $base) (i32.const 4))))
              (local.set $imm1 (i32.const 0))
              (block $bt_lp
                (loop $bt_cont
                  (if (i32.ge_u (local.get $imm1) (local.get $imm0)) (then (br $bt_lp)))
                  (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
                  (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
                  (local.set $imm1 (i32.add (local.get $imm1) (i32.const 1)))
                  (br $bt_cont)
                )
              )
              ;; Read default label and store as imm1 (at base+8)
              (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
              (i32.store (i32.add (local.get $base) (i32.const 8)) (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
              (br $op_handled)
            )
          )

          ;; ── else (0x05) ──
          (if (i32.eq (local.get $op) (i32.const 0x05))
            (then (br $op_handled))
          )

          ;; ── select (0x1B, 0x1C) ──
          (if (i32.eq (local.get $op) (i32.const 0x1B))
            (then (br $op_handled))
          )
          (if (i32.eq (local.get $op) (i32.const 0x1C))
            (then
              ;; typed select: skip result types
              (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
              (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
              (br $op_handled)
            )
          )

          ;; ── memory.size (0x3F), memory.grow (0x40): 1 byte immediate (0x00) ──
          (if (i32.or (i32.eq (local.get $op) (i32.const 0x3F)) (i32.eq (local.get $op) (i32.const 0x40)))
            (then
              (local.set $offset (i32.add (local.get $offset) (i32.const 1)))  ;; skip 0x00 byte
              (br $op_handled)
            )
          )

          ;; ── ref.null (0xD0): 1 byte immediate (reftype) ──
          (if (i32.eq (local.get $op) (i32.const 0xD0))
            (then
              (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
              (i32.store (i32.add (local.get $base) (i32.const 4))
                (i32.load8_u (i32.add (local.get $p) (local.get $offset)))
              )
              (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
              (br $op_handled)
            )
          )
          (if (i32.eq (local.get $op) (i32.const 0xD1))     ;; ref.is_null
            (then (br $op_handled))
          )
          (if (i32.eq (local.get $op) (i32.const 0xD2))     ;; ref.func
            (then
              (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
              (i32.store (i32.add (local.get $base) (i32.const 4)) (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
              (br $op_handled)
            )
          )

          ;; ── Extended prefix (0xFC) ──
          (if (i32.eq (local.get $op) (i32.const 0xFC))
            (then
              ;; Read sub-opcode
              (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
              (i32.store (i32.add (local.get $base) (i32.const 4)) (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
              ;; Consume reserved immediates based on sub-opcode
              (local.set $imm0 (i32.load (i32.add (local.get $base) (i32.const 4))))
              (if (i32.eq (local.get $imm0) (i32.const 0x0A))     ;; memory.copy: 2 reserved bytes
                (then (local.set $offset (i32.add (local.get $offset) (i32.const 2))))
              )
              (if (i32.eq (local.get $imm0) (i32.const 0x0B))     ;; memory.fill: 1 reserved byte
                (then (local.set $offset (i32.add (local.get $offset) (i32.const 1))))
              )
              (if (i32.eq (local.get $imm0) (i32.const 0x08))     ;; memory.init: data_seg idx + 1 reserved byte
                (then
                  (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
                  (i32.store (i32.add (local.get $base) (i32.const 8)) (i32.load (global.get $OFF_SCRATCH0)))
                  (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
                  (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
                )
              )
              (if (i32.eq (local.get $imm0) (i32.const 0x09))     ;; data.drop: data_seg idx
                (then
                  (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
                  (i32.store (i32.add (local.get $base) (i32.const 8)) (i32.load (global.get $OFF_SCRATCH0)))
                  (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
                )
              )
              (if (i32.eq (local.get $imm0) (i32.const 0x0C))     ;; table.init: elem_idx + table_idx
                (then
                  (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
                  (i32.store (i32.add (local.get $base) (i32.const 8)) (i32.load (global.get $OFF_SCRATCH0)))
                  (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
                  (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
                  (i32.store (i32.add (local.get $base) (i32.const 12)) (i32.load (global.get $OFF_SCRATCH0)))
                  (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
                )
              )
              (if (i32.eq (local.get $imm0) (i32.const 0x0D))     ;; elem.drop: elem_idx
                (then
                  (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
                  (i32.store (i32.add (local.get $base) (i32.const 8)) (i32.load (global.get $OFF_SCRATCH0)))
                  (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
                )
              )
              (if (i32.eq (local.get $imm0) (i32.const 0x0E))     ;; table.copy: dst + src table idx
                (then
                  (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
                  (i32.store (i32.add (local.get $base) (i32.const 8)) (i32.load (global.get $OFF_SCRATCH0)))
                  (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
                  (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
                  (i32.store (i32.add (local.get $base) (i32.const 12)) (i32.load (global.get $OFF_SCRATCH0)))
                  (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
                )
              )
              (if (i32.eq (local.get $imm0) (i32.const 0x0F))     ;; table.grow: table_idx
                (then
                  (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
                  (i32.store (i32.add (local.get $base) (i32.const 8)) (i32.load (global.get $OFF_SCRATCH0)))
                  (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
                )
              )
              (if (i32.eq (local.get $imm0) (i32.const 0x10))     ;; table.size: table_idx
                (then
                  (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
                  (i32.store (i32.add (local.get $base) (i32.const 8)) (i32.load (global.get $OFF_SCRATCH0)))
                  (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
                )
              )
              (if (i32.eq (local.get $imm0) (i32.const 0x11))     ;; table.fill: table_idx
                (then
                  (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
                  (i32.store (i32.add (local.get $base) (i32.const 8)) (i32.load (global.get $OFF_SCRATCH0)))
                  (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
                )
              )
              (br $op_handled)
            )
          )

          ;; ── SIMD prefix (0xFD) ──
          (if (i32.eq (local.get $op) (i32.const 0xFD))
            (then
              ;; Read sub-opcode (LEB128 u32)
              (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
              (i32.store (i32.add (local.get $base) (i32.const 4)) (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
              (local.set $imm0 (i32.load (i32.add (local.get $base) (i32.const 4))))

              ;; Memory ops (raw sub-opcodes 0x00-0x1F except 0x0C): align + offset
              ;; Uses raw $imm0 BEFORE translation to capture both old and modern encodings
              (if (i32.and (i32.le_u (local.get $imm0) (i32.const 0x1F))
                           (i32.ne (local.get $imm0) (i32.const 0x0C)))
                (then
                  (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
                  (local.set $imm1 (i32.load (global.get $OFF_SCRATCH0)))
                  (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
                  (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
                  (i32.store (i32.add (local.get $base) (i32.const 8)) (i32.load (global.get $OFF_SCRATCH0)))
                  (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
                  (i32.store (i32.add (local.get $base) (i32.const 12)) (local.get $imm1))  ;; align
                )
              )

              ;; v128.const (sub-opcode 0x0C): read 16 bytes into base+16
              (if (i32.eq (local.get $imm0) (i32.const 0x0C))
                (then
                  (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
                  (i64.store (i32.add (local.get $base) (i32.const 16))
                    (i64.load (i32.add (local.get $p) (local.get $offset))))
                  (i64.store (i32.add (local.get $base) (i32.const 24))
                    (i64.load (i32.add (local.get $p) (i32.add (local.get $offset) (i32.const 8)))))
                  (local.set $offset (i32.add (local.get $offset) (i32.const 16)))
                )
              )

              ;; ── Canonicalize sub-opcode (modern wabt → old encoding) ──
              ;; Only applied when $SIMD_MODERN_ENCODING is set to 1 (default 0).
              ;; Old-encoding input passes through unchanged.
              (if (global.get $SIMD_MODERN_ENCODING)
                (then
                  ;; Memory ops (modern store at 0x0B → canonical 0x1B)
                  (if (i32.eq (local.get $imm0) (i32.const 0x0B)) (then (local.set $imm0 (i32.const 0x1B))))
                  ;; Load-splat extends (modern only)
                  (if (i32.eq (local.get $imm0) (i32.const 0x07)) (then (local.set $imm0 (i32.const 0xA6))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x08)) (then (local.set $imm0 (i32.const 0xA7))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x09)) (then (local.set $imm0 (i32.const 0xA8))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x0A)) (then (local.set $imm0 (i32.const 0xA9))))
                  ;; Swizzle (modern only)
                  (if (i32.eq (local.get $imm0) (i32.const 0x0E)) (then (local.set $imm0 (i32.const 0xAA))))
                  ;; Splats
                  (if (i32.eq (local.get $imm0) (i32.const 0x0F)) (then (local.set $imm0 (i32.const 0x2D))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x10)) (then (local.set $imm0 (i32.const 0x31))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x11)) (then (local.set $imm0 (i32.const 0x35))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x12)) (then (local.set $imm0 (i32.const 0x39))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x13)) (then (local.set $imm0 (i32.const 0x3A))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x14)) (then (local.set $imm0 (i32.const 0x3D))))
                  ;; Extract/replace lanes
                  (if (i32.eq (local.get $imm0) (i32.const 0x15)) (then (local.set $imm0 (i32.const 0x2E))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x16)) (then (local.set $imm0 (i32.const 0x2F))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x17)) (then (local.set $imm0 (i32.const 0x30))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x18)) (then (local.set $imm0 (i32.const 0x32))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x19)) (then (local.set $imm0 (i32.const 0x33))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x1A)) (then (local.set $imm0 (i32.const 0x34))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x1B)) (then (local.set $imm0 (i32.const 0x36))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x1C)) (then (local.set $imm0 (i32.const 0x37))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x1D)) (then (local.set $imm0 (i32.const 0x38))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x1E)) (then (local.set $imm0 (i32.const 0xAB))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x1F)) (then (local.set $imm0 (i32.const 0x3B))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x20)) (then (local.set $imm0 (i32.const 0x3C))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x21)) (then (local.set $imm0 (i32.const 0x3E))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x22)) (then (local.set $imm0 (i32.const 0x3F))))
                  ;; Integer comparisons (modern 0x23-0x40 → canonical)
                  (if (i32.eq (local.get $imm0) (i32.const 0x23)) (then (local.set $imm0 (i32.const 0x47))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x24)) (then (local.set $imm0 (i32.const 0x48))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x25)) (then (local.set $imm0 (i32.const 0x4B))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x26)) (then (local.set $imm0 (i32.const 0x4A))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x27)) (then (local.set $imm0 (i32.const 0x4D))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x28)) (then (local.set $imm0 (i32.const 0x4C))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x29)) (then (local.set $imm0 (i32.const 0x4F))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x2A)) (then (local.set $imm0 (i32.const 0x4E))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x2B)) (then (local.set $imm0 (i32.const 0x49))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x2C)) (then (local.set $imm0 (i32.const 0xAE))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x2D)) (then (local.set $imm0 (i32.const 0x58))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x2E)) (then (local.set $imm0 (i32.const 0x59))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x2F)) (then (local.set $imm0 (i32.const 0x5C))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x30)) (then (local.set $imm0 (i32.const 0x5B))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x31)) (then (local.set $imm0 (i32.const 0x5E))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x32)) (then (local.set $imm0 (i32.const 0x5D))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x33)) (then (local.set $imm0 (i32.const 0x60))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x34)) (then (local.set $imm0 (i32.const 0x5F))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x35)) (then (local.set $imm0 (i32.const 0x5A))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x36)) (then (local.set $imm0 (i32.const 0xAF))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x37)) (then (local.set $imm0 (i32.const 0x69))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x38)) (then (local.set $imm0 (i32.const 0x6A))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x39)) (then (local.set $imm0 (i32.const 0x6D))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x3A)) (then (local.set $imm0 (i32.const 0x6C))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x3B)) (then (local.set $imm0 (i32.const 0x6F))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x3C)) (then (local.set $imm0 (i32.const 0x6E))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x3D)) (then (local.set $imm0 (i32.const 0x71))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x3E)) (then (local.set $imm0 (i32.const 0x70))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x3F)) (then (local.set $imm0 (i32.const 0x6B))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x40)) (then (local.set $imm0 (i32.const 0xB0))))
                  ;; Float comparisons (modern 0x41-0x4C → canonical)
                  (if (i32.eq (local.get $imm0) (i32.const 0x41)) (then (local.set $imm0 (i32.const 0x8C))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x42)) (then (local.set $imm0 (i32.const 0x8B))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x43)) (then (local.set $imm0 (i32.const 0x8E))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x44)) (then (local.set $imm0 (i32.const 0x90))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x45)) (then (local.set $imm0 (i32.const 0x92))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x46)) (then (local.set $imm0 (i32.const 0x94))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x47)) (then (local.set $imm0 (i32.const 0x9D))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x48)) (then (local.set $imm0 (i32.const 0x9C))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x49)) (then (local.set $imm0 (i32.const 0x9F))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x4A)) (then (local.set $imm0 (i32.const 0xA1))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x4B)) (then (local.set $imm0 (i32.const 0xA3))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x4C)) (then (local.set $imm0 (i32.const 0xA5))))
                  ;; v128 bitwise (modern → canonical)
                  (if (i32.eq (local.get $imm0) (i32.const 0x4D)) (then (local.set $imm0 (i32.const 0xAC))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x4E)) (then (local.set $imm0 (i32.const 0x43))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x50)) (then (local.set $imm0 (i32.const 0x44))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x51)) (then (local.set $imm0 (i32.const 0x45))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x52)) (then (local.set $imm0 (i32.const 0xAD))))
                  ;; i8x16 ops (modern → canonical)
                  (if (i32.eq (local.get $imm0) (i32.const 0x61)) (then (local.set $imm0 (i32.const 0x46))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x6E)) (then (local.set $imm0 (i32.const 0x40))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x71)) (then (local.set $imm0 (i32.const 0x41))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x77)) (then (local.set $imm0 (i32.const 0xB8))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x79)) (then (local.set $imm0 (i32.const 0xBA))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x7B)) (then (local.set $imm0 (i32.const 0xBB))))
                  ;; i16x8 ops (modern → canonical)
                  (if (i32.eq (local.get $imm0) (i32.const 0x81)) (then (local.set $imm0 (i32.const 0x57))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x8E)) (then (local.set $imm0 (i32.const 0x51))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x91)) (then (local.set $imm0 (i32.const 0x52))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x95)) (then (local.set $imm0 (i32.const 0x61))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x97)) (then (local.set $imm0 (i32.const 0xBC))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x99)) (then (local.set $imm0 (i32.const 0xBE))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x9B)) (then (local.set $imm0 (i32.const 0xBF))))
                  ;; i32x4 ops (modern → canonical)
                  (if (i32.eq (local.get $imm0) (i32.const 0xA1)) (then (local.set $imm0 (i32.const 0x68))))
                  (if (i32.eq (local.get $imm0) (i32.const 0xAE)) (then (local.set $imm0 (i32.const 0x62))))
                  (if (i32.eq (local.get $imm0) (i32.const 0xB1)) (then (local.set $imm0 (i32.const 0x63))))
                  (if (i32.eq (local.get $imm0) (i32.const 0xB5)) (then (local.set $imm0 (i32.const 0x72))))
                  ;; i64x2 ops (modern → canonical)
                  (if (i32.eq (local.get $imm0) (i32.const 0xC1)) (then (local.set $imm0 (i32.const 0x79))))
                  (if (i32.eq (local.get $imm0) (i32.const 0xCE)) (then (local.set $imm0 (i32.const 0x73))))
                  (if (i32.eq (local.get $imm0) (i32.const 0xD1)) (then (local.set $imm0 (i32.const 0x74))))
                  ;; f32x4 ops (modern → canonical)
                  (if (i32.eq (local.get $imm0) (i32.const 0xE0)) (then (local.set $imm0 (i32.const 0x88))))
                  (if (i32.eq (local.get $imm0) (i32.const 0xE1)) (then (local.set $imm0 (i32.const 0x8A))))
                  (if (i32.eq (local.get $imm0) (i32.const 0xE4)) (then (local.set $imm0 (i32.const 0x84))))
                  (if (i32.eq (local.get $imm0) (i32.const 0xE5)) (then (local.set $imm0 (i32.const 0x85))))
                  (if (i32.eq (local.get $imm0) (i32.const 0xE6)) (then (local.set $imm0 (i32.const 0x87))))
                  (if (i32.eq (local.get $imm0) (i32.const 0xE7)) (then (local.set $imm0 (i32.const 0x89))))
                  (if (i32.eq (local.get $imm0) (i32.const 0xE8)) (then (local.set $imm0 (i32.const 0xE1))))
                  (if (i32.eq (local.get $imm0) (i32.const 0xE9)) (then (local.set $imm0 (i32.const 0xE2))))
                  ;; f64x2 ops (modern → canonical)
                  (if (i32.eq (local.get $imm0) (i32.const 0xEC)) (then (local.set $imm0 (i32.const 0x9A))))
                  (if (i32.eq (local.get $imm0) (i32.const 0xED)) (then (local.set $imm0 (i32.const 0x9B))))
                  (if (i32.eq (local.get $imm0) (i32.const 0xF0)) (then (local.set $imm0 (i32.const 0x95))))
                  (if (i32.eq (local.get $imm0) (i32.const 0xF1)) (then (local.set $imm0 (i32.const 0x96))))
                  (if (i32.eq (local.get $imm0) (i32.const 0xF2)) (then (local.set $imm0 (i32.const 0x98))))
                  (if (i32.eq (local.get $imm0) (i32.const 0xF3)) (then (local.set $imm0 (i32.const 0x99))))
                  (if (i32.eq (local.get $imm0) (i32.const 0xF4)) (then (local.set $imm0 (i32.const 0xE3))))
                  (if (i32.eq (local.get $imm0) (i32.const 0xF5)) (then (local.set $imm0 (i32.const 0xE4))))
                )
              )

              ;; Store canonical sub-opcode (translated or pass-through)
              (i32.store (i32.add (local.get $base) (i32.const 4)) (local.get $imm0))

              ;; Lane index ops (extract_lane, replace_lane only): 1-byte lane index
              ;; Note: splat ops (0x2D, 0x31, 0x35, 0x39, 0x3A, 0x3D) do NOT have a lane index
              ;; Check specific extract/replace ranges: [0x2E-0x30], [0x32-0x34], [0x36-0x38], [0x3B-0x3C], [0x3E-0x3F]
              (local.set $adv (i32.const 0))
              (if (i32.and (i32.ge_u (local.get $imm0) (i32.const 0x2E)) (i32.le_u (local.get $imm0) (i32.const 0x30)))
                (then (local.set $adv (i32.const 1))))
              (if (i32.and (i32.ge_u (local.get $imm0) (i32.const 0x32)) (i32.le_u (local.get $imm0) (i32.const 0x34)))
                (then (local.set $adv (i32.const 1))))
              (if (i32.and (i32.ge_u (local.get $imm0) (i32.const 0x36)) (i32.le_u (local.get $imm0) (i32.const 0x38)))
                (then (local.set $adv (i32.const 1))))
              (if (i32.and (i32.ge_u (local.get $imm0) (i32.const 0x3B)) (i32.le_u (local.get $imm0) (i32.const 0x3C)))
                (then (local.set $adv (i32.const 1))))
              (if (i32.and (i32.ge_u (local.get $imm0) (i32.const 0x3E)) (i32.le_u (local.get $imm0) (i32.const 0x3F)))
                (then (local.set $adv (i32.const 1))))
              (if (i32.eq (local.get $imm0) (i32.const 0xAB))
                (then (local.set $adv (i32.const 1))))  ;; i64x2.replace_lane (new canonical)
              (if (local.get $adv)
                (then
                  (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
                  (i32.store (i32.add (local.get $base) (i32.const 8))
                    (i32.load8_u (i32.add (local.get $p) (local.get $offset))))
                  (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
                )
              )

              ;; All other 0xFD ops: no extra immediate
              (br $op_handled)
            )
          )

          ;; Unhandled opcode
          (return (global.get $ERR_UNSUP))
        )

        ;; Store decoded op marker
        (local.set $dc (i32.add (local.get $dc) (i32.const 1)))
        (br $cont)
      )
    )

    ;; Store start index and count
    (i32.store (global.get $OFF_SCRATCH0) (local.get $start_idx))
    (i32.store (global.get $OFF_SCRATCH1) (i32.sub (local.get $dc) (local.get $start_idx)))
    (i32.store (global.get $OFF_DECODED_COUNT) (local.get $dc))

    (return (global.get $OK))
  )

