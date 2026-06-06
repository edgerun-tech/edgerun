  ;; ═════════════════════════════════════════════════════════════════════
  ;; LEB128 Decoding
  ;; ═════════════════════════════════════════════════════════════════════

  (func $leb_u32 (param $offset i32) (result i32)
    (local $r i32) (local $s i32) (local $b i32) (local $c i32) (local $p i32) (local $l i32)
    (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
    (local.set $l (i32.load (global.get $OFF_WASM_LEN)))
    (local.set $c (local.get $offset))
    (block $end
      (loop $lp
        (if (i32.ge_u (local.get $c) (local.get $l)) (then (return (global.get $ERR_PARSE))))
        (local.set $b (i32.load8_u (i32.add (local.get $p) (local.get $c))))
        (local.set $c (i32.add (local.get $c) (i32.const 1)))
        (local.set $r (i32.or (local.get $r) (i32.shl (i32.and (local.get $b) (i32.const 0x7f)) (local.get $s))))
        (local.set $s (i32.add (local.get $s) (i32.const 7)))
        (if (i32.gt_u (local.get $s) (i32.const 35)) (then (return (global.get $ERR_PARSE))))
        (if (i32.eqz (i32.and (local.get $b) (i32.const 0x80))) (then (br $end)))
        (br $lp)
      )
    )
    (i32.store (global.get $OFF_SCRATCH0) (local.get $r))
    (i32.store (global.get $OFF_SCRATCH1) (i32.sub (local.get $c) (local.get $offset)))
    (return (global.get $OK))
  )

  (func $leb_i32 (param $offset i32) (result i32)
    (local $r i32) (local $s i32) (local $b i32) (local $c i32) (local $p i32) (local $l i32)
    (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
    (local.set $l (i32.load (global.get $OFF_WASM_LEN)))
    (local.set $c (local.get $offset))
    (block $end
      (loop $lp
        (if (i32.ge_u (local.get $c) (local.get $l)) (then (return (global.get $ERR_PARSE))))
        (local.set $b (i32.load8_u (i32.add (local.get $p) (local.get $c))))
        (local.set $c (i32.add (local.get $c) (i32.const 1)))
        (local.set $r (i32.or (local.get $r) (i32.shl (i32.and (local.get $b) (i32.const 0x7f)) (local.get $s))))
        (local.set $s (i32.add (local.get $s) (i32.const 7)))
        (if (i32.gt_u (local.get $s) (i32.const 35)) (then (return (global.get $ERR_PARSE))))
        (if (i32.eqz (i32.and (local.get $b) (i32.const 0x80))) (then (br $end)))
        (br $lp)
      )
    )
    (if (i32.and (local.get $b) (i32.const 0x40))
      (then (local.set $r (i32.or (local.get $r) (i32.shl (i32.const -1) (local.get $s)))))
    )
    (i32.store (global.get $OFF_SCRATCH0) (local.get $r))
    (i32.store (global.get $OFF_SCRATCH1) (i32.sub (local.get $c) (local.get $offset)))
    (return (global.get $OK))
  )

  (func $leb_i64 (param $offset i32) (result i32)
    (local $r_lo i32) (local $r_hi i32) (local $s i32) (local $b i32) (local $c i32) (local $p i32) (local $l i32)
    (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
    (local.set $l (i32.load (global.get $OFF_WASM_LEN)))
    (local.set $c (local.get $offset))
    (block $end
      (loop $lp
        (if (i32.ge_u (local.get $c) (local.get $l)) (then (return (global.get $ERR_PARSE))))
        (local.set $b (i32.load8_u (i32.add (local.get $p) (local.get $c))))
        (local.set $c (i32.add (local.get $c) (i32.const 1)))
        (if (i32.lt_s (local.get $s) (i32.const 32))
          (then
            (local.set $r_lo (i32.or (local.get $r_lo) (i32.shl (i32.and (local.get $b) (i32.const 0x7f)) (local.get $s))))
          )
          (else
            (local.set $r_hi (i32.or (local.get $r_hi) (i32.shl (i32.and (local.get $b) (i32.const 0x7f)) (i32.sub (local.get $s) (i32.const 32)))))
          )
        )
        (local.set $s (i32.add (local.get $s) (i32.const 7)))
        (if (i32.gt_u (local.get $s) (i32.const 63)) (then (return (global.get $ERR_PARSE))))
        (if (i32.eqz (i32.and (local.get $b) (i32.const 0x80))) (then (br $end)))
        (br $lp)
      )
    )
    (if (i32.and (local.get $b) (i32.const 0x40))
      (then
        (if (i32.lt_s (local.get $s) (i32.const 32))
          (then (local.set $r_lo (i32.or (local.get $r_lo) (i32.shl (i32.const -1) (local.get $s)))))
          (else (local.set $r_hi (i32.or (local.get $r_hi) (i32.shl (i32.const -1) (i32.sub (local.get $s) (i32.const 32))))))
        )
      )
    )
    (i64.store (global.get $OFF_SCRATCH0)
      (i64.or (i64.extend_i32_u (local.get $r_lo)) (i64.shl (i64.extend_i32_u (local.get $r_hi)) (i64.const 32)))
    )
    (i32.store (global.get $OFF_SCRATCH2) (i32.sub (local.get $c) (local.get $offset)))
    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; String / name reader (length-prefixed UTF-8 byte sequence)
  ;; stores name bytes to OFFSET_NAMES_BUF, returns (name_offset, bytes_consumed)
  ;; in scratch0/scratch1
  ;; ═════════════════════════════════════════════════════════════════════
  (func $read_name (param $offset i32) (result i32)
    (local $len i32) (local $adv i32) (local $dst i32) (local $p i32) (local $i i32)
    (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
    (local.set $len (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $adv (i32.load (global.get $OFF_SCRATCH1)))
    (local.set $offset (i32.add (local.get $offset) (local.get $adv)))
    (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
    (local.set $dst (i32.load (global.get $OFF_NAMES_PTR)))
    (local.set $i (i32.const 0))
    (block $copy_end
      (loop $copy_lp
        (if (i32.ge_u (local.get $i) (local.get $len)) (then (br $copy_end)))
        (i32.store8 (i32.add (local.get $dst) (local.get $i))
          (i32.load8_u (i32.add (local.get $p) (local.get $offset) (local.get $i)))
        )
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $copy_lp)
      )
    )
    (i32.store (global.get $OFF_SCRATCH0) (local.get $dst))
    (i32.store (global.get $OFF_SCRATCH1) (i32.add (local.get $adv) (local.get $len)))
    ;; Advance names buffer
    (i32.store (global.get $OFF_NAMES_PTR) (i32.add (local.get $dst) (local.get $len)))
    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Section header reader
  ;; Output: scratch0=sec_id, scratch1=sec_len, scratch2=content_offset, scratch3=end_offset
  ;; ═════════════════════════════════════════════════════════════════════
  (func $read_section_header (param $offset i32) (result i32)
    (local $id i32) (local $p i32) (local $l i32) (local $leb_adv i32)
    (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
    (local.set $l (i32.load (global.get $OFF_WASM_LEN)))
    (if (i32.ge_u (local.get $offset) (local.get $l)) (then (return (global.get $ERR_PARSE))))

    ;; Read section ID (1 byte)
    (local.set $id (i32.load8_u (i32.add (local.get $p) (local.get $offset))))
    (local.set $offset (i32.add (local.get $offset) (i32.const 1)))

    ;; Read section size (LEB128) - result in scratch0, advance in scratch1
    (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
    (local.set $leb_adv (i32.load (global.get $OFF_SCRATCH1)))

    ;; Save section_size before overwriting scratch0
    (i32.store (global.get $OFF_SCRATCH2) (i32.load (global.get $OFF_SCRATCH0)))  ;; sec_size saved in scratch2 temporarily

    ;; Store results
    (i32.store (global.get $OFF_SCRATCH0) (local.get $id))                    ;; sec_id -> scratch0
    ;; scratch1 = sec_size (moved from scratch2 to scratch1)
    (i32.store (global.get $OFF_SCRATCH1) (i32.load (global.get $OFF_SCRATCH2)))  ;; sec_size -> scratch1
    ;; scratch2 = content_offset = offset + leb_adv
    (i32.store (global.get $OFF_SCRATCH2) (i32.add (local.get $offset) (local.get $leb_adv)))
    ;; scratch3 = end_offset = content_offset + sec_size
    (i32.store (global.get $OFF_SCRATCH3)
      (i32.add
        (i32.add (local.get $offset) (local.get $leb_adv))
        (i32.load (global.get $OFF_SCRATCH1))
      )
    )
    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Module header: magic + version
  ;; ═════════════════════════════════════════════════════════════════════
  (func $parse_header (result i32)
    (local $p i32) (local $l i32)
    (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
    (local.set $l (i32.load (global.get $OFF_WASM_LEN)))
    (if (i32.lt_u (local.get $l) (i32.const 8)) (then (return (global.get $ERR_PARSE))))
    (if (i32.ne (i32.load (local.get $p)) (i32.const 0x6d736100)) (then (return (global.get $ERR_PARSE))))
    (if (i32.ne (i32.load (i32.add (local.get $p) (i32.const 4))) (i32.const 1)) (then (return (global.get $ERR_PARSE))))
    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Parse one type entry (functype)
  ;; Reads at offset, writes to scratch0 = new_offset after consuming type
  ;; Returns error
  ;; ═════════════════════════════════════════════════════════════════════
  (func $parse_one_type (param $offset i32) (result i32)
    (local $tc i32) (local $rc i32) (local $base i32) (local $i i32) (local $wp i32)

    (local.set $tc (i32.load (global.get $OFF_TYPE_COUNT)))
    (if (i32.ge_u (local.get $tc) (global.get $MAX_TYPES)) (then (return (global.get $ERR_PARSE))))

    ;; Type entry starts with 0x60 (functype)
    (local.set $wp (i32.load (global.get $OFF_WASM_PTR)))
    (if (i32.ne (i32.load8_u (i32.add (local.get $wp) (local.get $offset))) (i32.const 0x60))
      (then (return (global.get $ERR_PARSE)))
    )
    (local.set $offset (i32.add (local.get $offset) (i32.const 1)))

    ;; Read param count
    (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
    (local.set $rc (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
    (if (i32.gt_u (local.get $rc) (i32.const 32)) (then (return (global.get $ERR_PARSE))))

    ;; Compute type entry base
    (local.set $base (i32.add (global.get $OFF_TYPES_BUF) (i32.mul (local.get $tc) (global.get $SZ_TYPE))))

    ;; Write param types (up to 32 entries, each 1 byte)
    (local.set $i (i32.const 0))
    (block $pl
      (loop $pl_lp
        (if (i32.ge_u (local.get $i) (local.get $rc)) (then (br $pl)))
        (local.set $wp (i32.load (global.get $OFF_WASM_PTR)))
        (i32.store8 (i32.add (local.get $base) (local.get $i))
          (i32.load8_u (i32.add (local.get $wp) (local.get $offset)))
        )
        (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $pl_lp)
      )
    )

    ;; Write param_count (16-bit at offset 128 within type entry)
    (i32.store16 (i32.add (local.get $base) (i32.const 128)) (local.get $rc))

    ;; Read result count
    (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
    (local.set $rc (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
    (if (i32.gt_u (local.get $rc) (i32.const 4)) (then (return (global.get $ERR_PARSE))))

    ;; Write result types (start at offset 132 = 128 + 4)
    (local.set $i (i32.const 0))
    (block $rl
      (loop $rl_lp
        (if (i32.ge_u (local.get $i) (local.get $rc)) (then (br $rl)))
        (local.set $wp (i32.load (global.get $OFF_WASM_PTR)))
        (i32.store8 (i32.add (local.get $base) (i32.const 132) (local.get $i))
          (i32.load8_u (i32.add (local.get $wp) (local.get $offset)))
        )
        (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $rl_lp)
      )
    )

    ;; Write result_count (16-bit at offset 132 + 4 = 136)
    (i32.store16 (i32.add (local.get $base) (i32.const 136)) (local.get $rc))

    ;; Increment type count
    (i32.store (global.get $OFF_TYPE_COUNT) (i32.add (i32.load (global.get $OFF_TYPE_COUNT)) (i32.const 1)))

    ;; Return new offset
    (i32.store (global.get $OFF_SCRATCH0) (local.get $offset))
    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Parse type section
  ;; ═════════════════════════════════════════════════════════════════════
  (func $parse_type_section (param $offset i32) (param $size i32) (result i32)
    (local $count i32) (local $err i32) (local $end i32)

    (local.set $end (i32.add (local.get $offset) (local.get $size)))

    ;; Read type count
    (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
    (local.set $count (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))

    (block $lp
      (loop $continue
        (if (i32.eqz (local.get $count)) (then (br $lp)))
        (local.set $err (call $parse_one_type (local.get $offset)))
        (if (local.get $err) (then (return (local.get $err))))
        (local.set $offset (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $count (i32.sub (local.get $count) (i32.const 1)))
        (br $continue)
      )
    )

    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Parse limits: reads either "min" or "min, max" pair
  ;; Input: offset, returns: scratch0=min, scratch1=max(-1=unset), scratch2=advance
  ;; ═════════════════════════════════════════════════════════════════════
  (func $parse_limits (param $offset i32) (result i32)
    (local $flags i32) (local $min i32) (local $adv i32) (local $p i32)
    (local.set $p (i32.load (global.get $OFF_WASM_PTR)))

    ;; Read flags byte: 0 = min only, 1 = min + max
    (local.set $flags (i32.load8_u (i32.add (local.get $p) (local.get $offset))))
    (local.set $offset (i32.add (local.get $offset) (i32.const 1)))

    ;; Read min
    (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
    (local.set $min (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $adv (i32.load (global.get $OFF_SCRATCH1)))
    (local.set $offset (i32.add (local.get $offset) (local.get $adv)))
    (i32.store (global.get $OFF_SCRATCH0) (local.get $min))

    (if (local.get $flags)
      (then
        ;; Read max
        (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
        (i32.store (global.get $OFF_SCRATCH1) (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
      )
      (else
        ;; max = -1 (unlimited)
        (i32.store (global.get $OFF_SCRATCH1) (i32.const -1))
      )
    )

    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Parse function section (list of type indices)
  ;; ═════════════════════════════════════════════════════════════════════
  (func $parse_function_section (param $offset i32) (param $size i32) (result i32)
    (local $count i32) (local $i i32) (local $fc i32)

    (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
    (local.set $count (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
    (if (i32.gt_u (local.get $count) (global.get $MAX_FUNCTIONS)) (then (return (global.get $ERR_PARSE))))

    (local.set $i (i32.const 0))
    (local.set $fc (i32.load (global.get $OFF_FUNCTION_COUNT)))
    (block $lp
      (loop $cont
        (if (i32.ge_u (local.get $i) (local.get $count)) (then (br $lp)))
        ;; Read type index
        (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
        ;; Store to functions_buf[fc + i] = type_index (first 8 bytes of 16-byte entry)
        (i32.store
          (i32.add (global.get $OFF_FUNCTIONS_BUF) (i32.mul (i32.add (local.get $fc) (local.get $i)) (global.get $SZ_FUNC)))
          (i32.load (global.get $OFF_SCRATCH0))
        )
        (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $cont)
      )
    )
    (i32.store (global.get $OFF_FUNCTION_COUNT) (i32.add (local.get $fc) (local.get $count)))
    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Parse import section
  ;; Each import: module_name(string) + field_name(string) + kind(1 byte) + kind_data
  ;; For function: kind_data = type_index(LEB128)
  ;; For table: kind_data = elem_type(1) + limits
  ;; For memory: kind_data = limits
  ;; For global: kind_data = val_type(1) + mutability(1)
  ;; ═════════════════════════════════════════════════════════════════════
  (func $parse_import_section (param $offset i32) (param $size i32) (result i32)
    (local $count i32) (local $i i32)
    (local $kind i32) (local $p i32) (local $end i32) (local $flags i32)

    (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
    (local.set $count (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
    (if (i32.gt_u (local.get $count) (global.get $MAX_IMPORTS)) (then (return (global.get $ERR_PARSE))))

    (local.set $end (i32.add (local.get $offset) (local.get $size)))
    (local.set $i (i32.const 0))
    (block $lp
      (loop $cont
        (if (i32.ge_u (local.get $i) (local.get $count)) (then (br $lp)))
        (if (i32.ge_u (local.get $offset) (local.get $end)) (then (return (global.get $ERR_PARSE))))

        ;; Skip module name
        (if (call $read_name (local.get $offset)) (then (return (global.get $ERR_PARSE))))
        (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))

        ;; Skip field name
        (if (call $read_name (local.get $offset)) (then (return (global.get $ERR_PARSE))))
        (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))

        ;; Read import kind
        (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
        (local.set $kind (i32.load8_u (i32.add (local.get $p) (local.get $offset))))
        (local.set $offset (i32.add (local.get $offset) (i32.const 1)))

        (if (i32.eq (local.get $kind) (global.get $EXT_FUNC))
          (then
            ;; Function import: skip type index
            (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
            (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
          )
        )
        (if (i32.eq (local.get $kind) (global.get $EXT_TABLE))
          (then
            ;; Table import: elem_type (1 byte) + limits
            (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
            ;; flags (1 byte)
            (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
            (local.set $flags (i32.load8_u (i32.add (local.get $p) (local.get $offset))))
            (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
            ;; min
            (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
            (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
            ;; optional max
            (if (local.get $flags)
              (then
                (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
                (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
              )
            )
          )
        )
        (if (i32.eq (local.get $kind) (global.get $EXT_MEM))
          (then
            ;; Memory import: limits
            (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
            (local.set $flags (i32.load8_u (i32.add (local.get $p) (local.get $offset))))
            (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
            (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
            (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
            (if (local.get $flags)
              (then
                (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
                (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
              )
            )
          )
        )
        (if (i32.eq (local.get $kind) (global.get $EXT_GLOBAL))
          (then
            ;; Global import: val_type (1 byte) + mutability (1 byte)
            (local.set $offset (i32.add (local.get $offset) (i32.const 2)))
          )
        )

        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $cont)
      )
    )
    ;; Store import count
    (i32.store (global.get $OFF_IMPORT_COUNT) (local.get $count))
    ;; Initialize syscall map: sysno[i] = i for each import
    (local.set $i (i32.const 0))
    (loop $sysinit
      (if (i32.lt_u (local.get $i) (local.get $count))
        (then
          (i32.store (i32.add (i32.const 0x90000) (i32.shl (local.get $i) (i32.const 2))) (local.get $i))
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
          (br $sysinit)
        )
      )
    )
    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Parse global section
  ;; Each global: val_type(1) + mutability(1) + init_expr(opcodes ending with end)
  ;; ═════════════════════════════════════════════════════════════════════
  (func $parse_global_section (param $offset i32) (param $size i32) (result i32)
    (local $count i32) (local $i i32) (local $gc i32) (local $end i32)
    (local $op i32) (local $p i32)

    (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
    (local.set $count (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
    (if (i32.gt_u (local.get $count) (global.get $MAX_GLOBALS)) (then (return (global.get $ERR_PARSE))))

    (local.set $end (i32.add (local.get $offset) (local.get $size)))
    (local.set $gc (i32.load (global.get $OFF_GLOBAL_COUNT)))
    (local.set $i (i32.const 0))
    (block $lp
      (loop $cont
        (if (i32.ge_u (local.get $i) (local.get $count)) (then (br $lp)))
        (if (i32.ge_u (local.get $offset) (local.get $end)) (then (return (global.get $ERR_PARSE))))

        ;; Skip val_type (1 byte) + mutability (1 byte)
        (local.set $offset (i32.add (local.get $offset) (i32.const 2)))

        ;; Parse init expression: read opcodes until end (0x0B)
        (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
        (local.set $op (i32.load8_u (i32.add (local.get $p) (local.get $offset))))
        (if (i32.eq (local.get $op) (i32.const 0x41))  ;; i32.const
          (then
            (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
            (if (call $leb_i32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
            (i32.store
              (i32.add (global.get $OFF_GLOBALS_BUF) (i32.shl (i32.add (local.get $gc) (local.get $i)) (i32.const 2)))
              (i32.load (global.get $OFF_SCRATCH0))
            )
            (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
          )
          (else
            ;; Unsupported init expr: skip to end (0x0B) silently, value stays 0
            (block $skip_expr
              (loop $skip_cont
                (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
                (local.set $op (i32.load8_u (i32.add (local.get $p) (local.get $offset))))
                (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
                (if (i32.eq (local.get $op) (i32.const 0x0B)) (then (br $skip_expr)))
                (br $skip_cont)
              )
            )
            ;; Default to 0
            (i32.store
              (i32.add (global.get $OFF_GLOBALS_BUF) (i32.shl (i32.add (local.get $gc) (local.get $i)) (i32.const 2)))
              (i32.const 0)
            )
          )
        )

        ;; Skip end
        (if (i32.ne (local.get $op) (i32.const 0x0B))
          (then
            ;; Skip remaining opcodes until end (0x0B)
            (block $skip_end
              (loop $skip_end_cont
                (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
                (local.set $op (i32.load8_u (i32.add (local.get $p) (local.get $offset))))
                (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
                (if (i32.eq (local.get $op) (i32.const 0x0B)) (then (br $skip_end)))
                (br $skip_end_cont)
              )
            )
          )
        )

        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $cont)
      )
    )
    (i32.store (global.get $OFF_GLOBAL_COUNT) (i32.add (local.get $gc) (local.get $count)))
    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Parse memory section (memory type + limits)
  ;; ═════════════════════════════════════════════════════════════════════
  (func $parse_memory_section (param $offset i32) (param $size i32) (result i32)
    (local $count i32)

    (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
    (local.set $count (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
    (if (i32.ne (local.get $count) (i32.const 1)) (then (return (global.get $ERR_UNSUP))))

    ;; Parse limits
    (if (call $parse_limits (local.get $offset)) (then (return (global.get $ERR_PARSE))))
    (i32.store (global.get $OFF_MEM_MIN) (i32.load (global.get $OFF_SCRATCH0)))
    (i32.store (global.get $OFF_MEM_MAX) (i32.load (global.get $OFF_SCRATCH1)))
    (i32.store (global.get $OFF_GUEST_MEM_PAGES) (i32.load (global.get $OFF_SCRATCH0)))
    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Parse table section (elem_type + limits)
  ;; MVP: at most 1 table, elem_type must be 0x70 (funcref)
  ;; ═════════════════════════════════════════════════════════════════════
  (func $parse_table_section (param $offset i32) (param $size i32) (result i32)
    (local $count i32) (local $elem_type i32) (local $flags i32)
    (local $min i32) (local $max i32) (local $p i32) (local $i i32)

    (i32.store (global.get $OFF_DBG) (i32.const 50))  ;; dbg: entered parse_table_section

    (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
    (i32.store (global.get $OFF_DBG) (i32.const 51))  ;; dbg: after leb count
    (local.set $count (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
    (if (i32.gt_u (local.get $count) (i32.const 1)) (then (return (global.get $ERR_UNSUP))))
    (if (i32.eqz (local.get $count)) (then (return (global.get $OK))))

    (i32.store (global.get $OFF_DBG) (i32.const 52))  ;; dbg: count is 1, reading elem_type

    ;; elem_type (1 byte) must be 0x70 (funcref)
    (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
    (local.set $elem_type (i32.load8_u (i32.add (local.get $p) (local.get $offset))))
    (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
    (if (i32.ne (local.get $elem_type) (i32.const 0x70)) (then (return (global.get $ERR_UNSUP))))

    (i32.store (global.get $OFF_DBG) (i32.const 53))  ;; dbg: elem_type is 0x70

    ;; Parse limits: flags (1 byte) + min (LEB128) + [max (LEB128)]
    (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
    (local.set $flags (i32.load8_u (i32.add (local.get $p) (local.get $offset))))
    (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
    (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
    (local.set $min (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
    (local.set $max (i32.const 0))
    (if (i32.and (local.get $flags) (i32.const 1))
      (then
        (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
        (local.set $max (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
      )
    )
    (i32.store (global.get $OFF_DBG) (i32.const 54))  ;; dbg: limits read, checking max
    (if (i32.gt_u (local.get $min) (global.get $MAX_TABLE)) (then (return (global.get $ERR_PARSE))))
    (if (i32.and (local.get $flags) (i32.const 1))
      (then (if (i32.gt_u (local.get $max) (global.get $MAX_TABLE)) (then (return (global.get $ERR_PARSE)))))
    )

    ;; Store table info
    (i32.store8 (global.get $OFF_TABLE_HAS) (i32.const 1))
    (i32.store (global.get $OFF_TABLE_MIN) (local.get $min))
    (i32.store (global.get $OFF_TABLE_MAX) (local.get $max))

    ;; Initialize all table entries to -1 (null funcref)
    (local.set $i (i32.const 0))
    (block $init_lp
      (loop $init_cont
        (if (i32.ge_u (local.get $i) (local.get $min)) (then (br $init_lp)))
        (i32.store
          (i32.add (global.get $OFF_TABLE_DATA) (i32.shl (local.get $i) (i32.const 2)))
          (i32.const -1)
        )
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $init_cont)
      )
    )

    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Parse export section (list of exports)
  ;; Each export: name(string) + kind(1 byte) + index(LEB128)
  ;; ═════════════════════════════════════════════════════════════════════
  (func $parse_export_section (param $offset i32) (param $size i32) (result i32)
    (local $count i32) (local $i i32) (local $ec i32) (local $base i32)
    (local $name_off i32) (local $name_len i32) (local $kind i32) (local $idx i32) (local $p i32)

    (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
    (local.set $count (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
    (if (i32.gt_u (local.get $count) (global.get $MAX_EXPORTS)) (then (return (global.get $ERR_PARSE))))

    (local.set $ec (i32.load (global.get $OFF_EXPORT_COUNT)))
    (local.set $i (i32.const 0))
    (block $lp
      (loop $cont
        (if (i32.ge_u (local.get $i) (local.get $count)) (then (br $lp)))

        (local.set $base (i32.add (global.get $OFF_EXPORTS_BUF) (i32.mul (i32.add (local.get $ec) (local.get $i)) (global.get $SZ_EXPORT))))

        ;; Read name string
        (if (call $read_name (local.get $offset)) (then (return (global.get $ERR_PARSE))))
        (local.set $name_off (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $name_len (i32.load (global.get $OFF_SCRATCH1)))
        (local.set $offset (i32.add (local.get $offset) (local.get $name_len)))

        ;; Read kind (1 byte)
        (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
        (local.set $kind (i32.load8_u (i32.add (local.get $p) (local.get $offset))))
        (local.set $offset (i32.add (local.get $offset) (i32.const 1)))

        ;; Read index (LEB128)
        (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
        (local.set $idx (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))

        ;; Store to export entry
        (i32.store (local.get $base) (local.get $name_off))           ;; name_ptr
        (i32.store (i32.add (local.get $base) (i32.const 8)) (local.get $name_len))  ;; name_len
        (i32.store8 (i32.add (local.get $base) (i32.const 16)) (local.get $kind))    ;; kind
        (i32.store (i32.add (local.get $base) (i32.const 24)) (local.get $idx))      ;; index

        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $cont)
      )
    )
    (i32.store (global.get $OFF_EXPORT_COUNT) (i32.add (local.get $ec) (local.get $count)))
    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Parse start section (single function index)
  ;; ═════════════════════════════════════════════════════════════════════
  (func $parse_start_section (param $offset i32) (param $size i32) (result i32)
    (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
    (i32.store (global.get $OFF_START_FUNC) (i32.load (global.get $OFF_SCRATCH0)))
    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Parse code section (function bodies)
  ;; Each body: size(LEB128) + local_count(LEB128) + (locals: count + type)* + bytecode
  ;; We store: body_offset, body_len, local_count
  ;; ═════════════════════════════════════════════════════════════════════
  (func $parse_code_section (param $offset i32) (param $size i32) (result i32)
    (local $count i32) (local $i i32) (local $cc i32) (local $base i32)
    (local $body_size i32) (local $body_end i32)
    (local $local_count i32) (local $lc i32)
    (local $num_locals i32) (local $type i32) (local $j i32)
    (local $p i32)

    (i32.store (global.get $OFF_DBG) (i32.const 90))  ;; dbg: entered parse_code_section
    (local.set $cc (i32.load (global.get $OFF_CODE_COUNT)))

    (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
    (local.set $count (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
    (if (i32.gt_u (local.get $count) (global.get $MAX_FUNCTIONS)) (then (return (global.get $ERR_PARSE))))

    (local.set $i (i32.const 0))
    (block $lp
      (loop $cont
        (if (i32.ge_u (local.get $i) (local.get $count)) (then (br $lp)))

        ;; Read body size
        (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
        (local.set $body_size (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))

        ;; body_end = offset + body_size (offset is after the size LEB, body_size is the content size)
        (local.set $body_end (i32.add (local.get $offset) (local.get $body_size)))

        ;; Store body_offset (relative to wasm_base), body_len
        (local.set $base (i32.add (global.get $OFF_CODE_BUF) (i32.mul (i32.add (local.get $cc) (local.get $i)) (global.get $SZ_CODE))))
        (i32.store (i32.add (local.get $base) (i32.const 0)) (local.get $offset))     ;; body_offset
        (i32.store (i32.add (local.get $base) (i32.const 8)) (local.get $body_size))  ;; body_len

        ;; Read local count
        (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
        (local.set $local_count (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))

        ;; Parse local declarations
        (local.set $lc (i32.const 0))
        (local.set $j (i32.const 0))
        (block $llp
          (loop $llcont
            (if (i32.ge_u (local.get $j) (local.get $local_count)) (then (br $llp)))
            (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
            (local.set $num_locals (i32.load (global.get $OFF_SCRATCH0)))
            (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
            (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
            (local.set $type (i32.load8_u (i32.add (local.get $p) (local.get $offset))))
            (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
            (local.set $lc (i32.add (local.get $lc) (local.get $num_locals)))
            (local.set $j (i32.add (local.get $j) (i32.const 1)))
            (br $llcont)
          )
        )
        (i32.store (i32.add (local.get $base) (i32.const 16)) (local.get $lc))  ;; local_count

        ;; Decode opcodes
        (i32.store (global.get $OFF_DBG) (i32.const 80))  ;; dbg: before decode
        (if (call $decode_opcodes (local.get $offset) (i32.sub (local.get $body_end) (local.get $offset)))
          (then (i32.store (global.get $OFF_DBG) (i32.const 81)) (return (global.get $ERR_PARSE)))  ;; dbg: decode fail
        )
        (i32.store (global.get $OFF_DBG) (i32.const 82))  ;; dbg: after decode
        ;; Compute end targets for structured control flow
        (if (call $compute_end_targets
              (i32.load (global.get $OFF_SCRATCH0))
              (i32.load (global.get $OFF_SCRATCH1))
            )
          (then (i32.store (global.get $OFF_DBG) (i32.const 83)) (return (global.get $ERR_PARSE)))  ;; dbg: compute fail
        )
        (i32.store (global.get $OFF_DBG) (i32.const 84))  ;; dbg: after compute
        ;; Store decoded_start (scratch0) and decoded_count (scratch1) in code entry
        (i32.store (i32.add (local.get $base) (i32.const 24)) (i32.load (global.get $OFF_SCRATCH0)))  ;; decoded_start
        (i32.store (i32.add (local.get $base) (i32.const 32)) (i32.load (global.get $OFF_SCRATCH1)))  ;; decoded_count

        ;; Skip to body_end
        (local.set $offset (local.get $body_end))

        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $cont)
      )
    )
    (i32.store (global.get $OFF_CODE_COUNT) (i32.add (local.get $cc) (local.get $count)))
    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Parse data section (list of data segments)
  ;; ═════════════════════════════════════════════════════════════════════
  (func $parse_data_section (param $offset i32) (param $size i32) (result i32)
    (local $count i32) (local $i i32) (local $dc i32) (local $base i32)
    (local $mode i32) (local $seg_size i32) (local $p i32) (local $j i32)
    (local $dest_addr i32) (local $wasm_base i32)

    (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
    (local.set $count (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
    (if (i32.gt_u (local.get $count) (global.get $MAX_DATAS)) (then (return (global.get $ERR_PARSE))))

    (local.set $dc (i32.load (global.get $OFF_DATA_COUNT)))
    (local.set $wasm_base (i32.load (global.get $OFF_WASM_PTR)))
    (local.set $i (i32.const 0))
    (block $lp
      (loop $cont
        (if (i32.ge_u (local.get $i) (local.get $count)) (then (br $lp)))
        (local.set $base (i32.add (global.get $OFF_DATA_BUF) (i32.mul (i32.add (local.get $dc) (local.get $i)) (global.get $SZ_DATA))))

        ;; Read mode (0=active, 1=passive, 2=active with memory index)
        (local.set $mode (i32.load8_u (i32.add (local.get $wasm_base) (local.get $offset))))
        (local.set $offset (i32.add (local.get $offset) (i32.const 1)))

        (local.set $dest_addr (i32.const 0))
        (if (i32.or (i32.eq (local.get $mode) (i32.const 2)) (i32.eq (local.get $mode) (i32.const 0)))
          (then
            ;; Active: parse i32.const <value> 0x0B
            (if (i32.eq (i32.load8_u (i32.add (local.get $wasm_base) (local.get $offset))) (i32.const 0x41))
              (then
                (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
                (if (call $leb_i32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
                (local.set $dest_addr (i32.load (global.get $OFF_SCRATCH0)))
                (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
              )
            )
            ;; Skip remaining init expression until 0x0B
            (block $ie
              (loop $iel
                (if (i32.eq (i32.load8_u (i32.add (local.get $wasm_base) (local.get $offset))) (i32.const 0x0B))
                  (then
                    (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
                    (br $ie)
                  )
                )
                (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
                (br $iel)
              )
            )
          )
        )

        ;; Read segment data size
        (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
        (local.set $seg_size (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))

        ;; Store data segment info
        (i32.store (local.get $base) (local.get $offset))      ;; data_offset (in guest wasm)
        (i32.store (i32.add (local.get $base) (i32.const 4)) (local.get $dest_addr))  ;; dest_addr (in guest memory)
        (i32.store (i32.add (local.get $base) (i32.const 8)) (local.get $seg_size))  ;; data_len

        ;; Copy data bytes to guest memory (only for active segments)
        (if (i32.eq (local.get $mode) (i32.const 1))
          (then)  ;; passive segment: skip copy to memory
          (else
            (if (i32.and (i32.eqz (local.get $dest_addr)) (i32.eqz (local.get $seg_size)))
              (then)
              (else
            ;; Copy seg_size bytes from wasm_base+data_offset to guest_mem_base+dest_addr
            (local.set $p (i32.const 0))
            (block $copy_lp
              (loop $copy_cont
                (if (i32.ge_u (local.get $p) (local.get $seg_size)) (then (br $copy_lp)))
                (i32.store8
                  (i32.add (global.get $OFF_GUEST_MEM_BASE) (i32.add (local.get $dest_addr) (local.get $p)))
                  (i32.load8_u (i32.add (local.get $wasm_base) (i32.add (local.get $offset) (local.get $p))))
                )
                (local.set $p (i32.add (local.get $p) (i32.const 1)))
                (br $copy_cont)
              )
            )
            )
          )
        )
      )

        ;; Skip data bytes (advance offset past the data)
        (local.set $offset (i32.add (local.get $offset) (local.get $seg_size)))

        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $cont)
      )
    )
    (i32.store (global.get $OFF_DATA_COUNT) (i32.add (local.get $dc) (local.get $count)))
    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Parse element section (segment data for table initialization)
  ;; Each element: mode + [table_idx] + offset_expr + count + func_indices
  ;; ═════════════════════════════════════════════════════════════════════
  (func $parse_element_section (param $offset i32) (param $size i32) (result i32)
    (local $count i32) (local $i i32) (local $mode i32) (local $table_idx i32)
    (local $dest_offset i32) (local $elem_count i32) (local $j i32)
    (local $func_idx i32) (local $p i32) (local $end i32)
    (local $elem_base i32) (local $dc i32)

    (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
    (local.set $count (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
    (if (i32.gt_u (local.get $count) (global.get $MAX_ELEMS)) (then (return (global.get $ERR_PARSE))))

    (local.set $end (i32.add (local.get $offset) (local.get $size)))
    (local.set $dc (i32.load (global.get $OFF_ELEM_COUNT)))
    (local.set $i (i32.const 0))
    (block $lp
      (loop $cont
        (if (i32.ge_u (local.get $i) (local.get $count)) (then (br $lp)))
        (if (i32.ge_u (local.get $offset) (local.get $end)) (then (return (global.get $ERR_PARSE))))

        (local.set $elem_base (i32.add (global.get $OFF_ELEM_BUF) (i32.mul (i32.add (local.get $dc) (local.get $i)) (global.get $SZ_ELEM))))

        ;; Read mode
        (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
        (local.set $mode (i32.load8_u (i32.add (local.get $p) (local.get $offset))))
        (local.set $offset (i32.add (local.get $offset) (i32.const 1)))

        (local.set $table_idx (i32.const 0))
        (local.set $dest_offset (i32.const 0))

        (if (i32.eq (local.get $mode) (i32.const 0))
          (then
            ;; Mode 0: active, implicit table 0. Parse init_expr (i32.const <value> 0x0B)
            (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
            (if (i32.eq (i32.load8_u (i32.add (local.get $p) (local.get $offset))) (i32.const 0x41))
              (then
                (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
                (if (call $leb_i32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
                (local.set $dest_offset (i32.load (global.get $OFF_SCRATCH0)))
                (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
              )
            )
            ;; Skip to end (0x0B)
            (block $ie
              (loop $iel
                (if (i32.ge_u (local.get $offset) (local.get $end)) (then (return (global.get $ERR_PARSE))))
                (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
                (if (i32.eq (i32.load8_u (i32.add (local.get $p) (local.get $offset))) (i32.const 0x0B))
                  (then
                    (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
                    (br $ie)
                  )
                )
                (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
                (br $iel)
              )
            )
          )
        )
        (if (i32.eq (local.get $mode) (i32.const 1))
          (then
            ;; Mode 1: passive segment - stored for table.init
          )
        )
        (if (i32.eq (local.get $mode) (i32.const 2))
          (then
            ;; Mode 2: active with explicit table index
            (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
            (local.set $table_idx (i32.load (global.get $OFF_SCRATCH0)))
            (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
            (if (i32.ne (local.get $table_idx) (i32.const 0)) (then (return (global.get $ERR_UNSUP))))

            ;; Parse init_expr
            (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
            (if (i32.eq (i32.load8_u (i32.add (local.get $p) (local.get $offset))) (i32.const 0x41))
              (then
                (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
                (if (call $leb_i32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
                (local.set $dest_offset (i32.load (global.get $OFF_SCRATCH0)))
                (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
              )
            )
            (block $ie2
              (loop $iel2
                (if (i32.ge_u (local.get $offset) (local.get $end)) (then (return (global.get $ERR_PARSE))))
                (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
                (if (i32.eq (i32.load8_u (i32.add (local.get $p) (local.get $offset))) (i32.const 0x0B))
                  (then
                    (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
                    (br $ie2)
                  )
                )
                (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
                (br $iel2)
              )
            )
          )
        )

        ;; Store mode to element buffer (0=dropped flag, then mode, dest_offset)
        (i32.store (local.get $elem_base) (i32.const 0))           ;; dropped = 0 (active)
        (i32.store (i32.add (local.get $elem_base) (i32.const 4)) (local.get $dest_offset))  ;; dest_offset

        ;; Read element count
        (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
        (local.set $elem_count (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))

        ;; Store element count
        (i32.store (i32.add (local.get $elem_base) (i32.const 8)) (local.get $elem_count))

        ;; Read and store each func idx in the table and element buffer
        (local.set $j (i32.const 0))
        (block $elem_lp
          (loop $elem_cont
            (if (i32.ge_u (local.get $j) (local.get $elem_count)) (then (br $elem_lp)))

            (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
            (local.set $func_idx (i32.load (global.get $OFF_SCRATCH0)))
            (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))

            ;; Store func_idx in element buffer for table.init
            (i32.store
              (i32.add (local.get $elem_base) (i32.add (i32.const 12) (i32.shl (local.get $j) (i32.const 2))))
              (local.get $func_idx)
            )

            ;; For active segments (mode 0/2), also write to table immediately
            (if (i32.le_u (local.get $mode) (i32.const 2))
              (then
                (if (i32.ne (local.get $mode) (i32.const 1))
                  (then
                    (i32.store
                      (i32.add (global.get $OFF_TABLE_DATA) (i32.shl (i32.add (local.get $dest_offset) (local.get $j)) (i32.const 2)))
                      (local.get $func_idx)
                    )
                  )
                )
              )
            )

            (local.set $j (i32.add (local.get $j) (i32.const 1)))
            (br $elem_cont)
          )
        )

        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $cont)
      )
    )
    (i32.store (global.get $OFF_ELEM_COUNT) (i32.add (local.get $dc) (local.get $count)))
    (return (global.get $OK))
  )

