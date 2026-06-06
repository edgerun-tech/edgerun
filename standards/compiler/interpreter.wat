  ;; ===================================================================
  ;; EdgeRun WASM Interpreter — standalone (WAT parser + syscall init)
  ;;
  ;; The interpreter core (load/validate/decode/execute) lives in
  ;; compiler/interpreter-core.wat and is shared with pipeline/.
  ;; This file adds the WAT lexer, syscall init, and standalone exports.
  ;; ===================================================================


  ;; ── Shared constants (from edgerun-core) ─────────────────────────────

  ;; ── Memory offsets ──────────────────────────────────────────────────
  ;; ── Shared core ────────────────────────────────────────────────────
  ;; The interpreter engine (globals, decode, execute, load) is in
  ;; compiler/interpreter-core.wat, shared with pipeline/.
  ;; Include that fragment BEFORE this file in your (module ...) block.
  ;; ───────────────────────────────────────────────────────────────────

  ;; (All shared globals, decode, execute, and load functions are in
  ;; compiler/interpreter-core.wat — included before this file.)


  ;; ═════════════════════════════════════════════════════════════════════
  ;; WAT Parser — parse WAT text format directly into state buffers
  ;; ═════════════════════════════════════════════════════════════════════
  ;;
  ;; Memory layout for WAT parser:
  ;;   0x8C000: WAT source pointer
  ;;   0x8C004: WAT source length
  ;;   0x8C008: Current position
  ;;   0x8C00C: Symbol table count
  ;;   0x8C010: Saved wasm_ptr (for restoration)
  ;;   0x8C014: Saved wasm_len
  ;;   0x8C018: Error position
  ;;   0x8C01C: Body offset (temp)
  ;;   0x8B000: Symbol table (256 entries × 16 bytes)
  ;;   0x8D000: WASM bytecode emission buffer (4KB)

  (global $OFF_WAT_PTR  i32 (i32.const 0x8C000))
  (global $OFF_WAT_LEN  i32 (i32.const 0x8C004))
  (global $OFF_WAT_POS  i32 (i32.const 0x8C008))
  (global $OFF_WAT_SYM  i32 (i32.const 0x8C00C))
  (global $OFF_WAT_SAV_PTR i32 (i32.const 0x8C010))
  (global $OFF_WAT_SAV_LEN i32 (i32.const 0x8C014))
  (global $OFF_WAT_ERR    i32 (i32.const 0x8C018))
  (global $OFF_WAT_TMP    i32 (i32.const 0x8C01C))
  (global $OFF_WAT_SYMS   i32 (i32.const 0x8B000))
  (global $WAT_SYM_SZ     i32 (i32.const 16))
  (global $WAT_MAX_SYMS   i32 (i32.const 256))
  (global $OFF_WAT_BODY   i32 (i32.const 0x8D000))
  (global $WAT_BODY_SZ    i32 (i32.const 4096))
  (global $OFF_WAT_TMP0   i32 (i32.const 0x8C040))
  (global $OFF_WAT_TMP1   i32 (i32.const 0x8C044))

  ;; ═════════════════════════════════════════════════════════════════════
  ;; WAT Lexer
  ;; ═════════════════════════════════════════════════════════════════════

  ;; Advance past whitespace and comments starting from pos.
  ;; Returns new position in scratch0, error in return value.
  (func $wat_skip_ws (param $pos i32) (result i32)
    (local $p i32) (local $l i32) (local $b i32) (local $end i32)
    (i32.store (i32.const 0x8C074) (i32.load (global.get $OFF_WAT_PTR)))
    (i32.store (i32.const 0x8C078) (local.get $pos))
    (local.set $p (i32.load (global.get $OFF_WAT_PTR)))
    (local.set $l (i32.load (global.get $OFF_WAT_LEN)))
    (local.set $end (i32.add (local.get $p) (local.get $l)))
    (local.set $p (i32.add (local.get $p) (local.get $pos)))
    (i32.store (i32.const 0x8C07C) (local.get $p))
    (block $done
      (loop $lp
        (if (i32.ge_u (local.get $p) (local.get $end)) (then (br $done)))
        (local.set $b (i32.load8_u (local.get $p)))
        ;; space, tab, lf, cr
        (if (i32.eq (local.get $b) (i32.const 0x20)) (then (local.set $p (i32.add (local.get $p) (i32.const 1))) (br $lp)))
        (if (i32.eq (local.get $b) (i32.const 0x09)) (then (local.set $p (i32.add (local.get $p) (i32.const 1))) (br $lp)))
        (if (i32.eq (local.get $b) (i32.const 0x0A)) (then (local.set $p (i32.add (local.get $p) (i32.const 1))) (br $lp)))
        (if (i32.eq (local.get $b) (i32.const 0x0D)) (then (local.set $p (i32.add (local.get $p) (i32.const 1))) (br $lp)))
        ;; Line comment: ;;
        (if (i32.eq (local.get $b) (i32.const 0x3B))
          (then
            (local.set $p (i32.add (local.get $p) (i32.const 1)))
            (if (i32.lt_u (local.get $p) (local.get $end))
              (then
                (local.set $b (i32.load8_u (local.get $p)))
                (local.set $p (i32.add (local.get $p) (i32.const 1)))
                (if (i32.eq (local.get $b) (i32.const 0x3B))
                  (then
                    ;; Skip to end of line
                    (block $eol
                      (loop $eol_lp
                        (if (i32.ge_u (local.get $p) (local.get $end)) (then (br $eol)))
                        (local.set $b (i32.load8_u (local.get $p)))
                        (local.set $p (i32.add (local.get $p) (i32.const 1)))
                        (if (i32.or (i32.eq (local.get $b) (i32.const 0x0A)) (i32.eq (local.get $b) (i32.const 0x0D)))
                          (then (br $eol))
                          (else (br $eol_lp))
                        )
                      )
                    )
                    (br $lp)
                  )
                )
              )
            )
            (br $done)
          )
        )
        ;; Block comment: (;
        (if (i32.eq (local.get $b) (i32.const 0x28))
          (then
            (local.set $p (i32.add (local.get $p) (i32.const 1)))
            (if (i32.lt_u (local.get $p) (local.get $end))
              (then
                (local.set $b (i32.load8_u (local.get $p)))
                (if (i32.eq (local.get $b) (i32.const 0x3B))
                  (then
                    (local.set $p (i32.add (local.get $p) (i32.const 1)))
                    ;; Skip to ;)
                    (block $bce
                      (loop $bcl
                        (if (i32.ge_u (local.get $p) (local.get $end)) (then (br $bce)))
                        (local.set $b (i32.load8_u (local.get $p)))
                        (local.set $p (i32.add (local.get $p) (i32.const 1)))
                        (if (i32.eq (local.get $b) (i32.const 0x3B))
                          (then
                            (if (i32.lt_u (local.get $p) (local.get $end))
                              (then
                                (local.set $b (i32.load8_u (local.get $p)))
                                (if (i32.eq (local.get $b) (i32.const 0x29))
                                  (then
                                    (local.set $p (i32.add (local.get $p) (i32.const 1)))
                                    (br $bce)
                                  )
                                )
                              )
                            )
                          )
                        )
                        (br $bcl)
                      )
                    )
                    (br $lp)
                  )
                  (else
                    ;; Not a block comment, back up
                    (local.set $p (i32.sub (local.get $p) (i32.const 1)))
                  )
                )
              )
            )
            (br $done)
          )
        )
        (br $done)
      )
    )
    (i32.store (global.get $OFF_SCRATCH0) (i32.sub (local.get $p) (i32.load (global.get $OFF_WAT_PTR))))
    (return (global.get $OK))
  )

  ;; Check if keyword at pos matches given kw_ptr/kw_len (case-insensitive).
  ;; Returns 1 on match, 0 on no match. Does NOT advance position.
  (func $wat_match_kw (param $pos i32) (param $kw_ptr i32) (param $kw_len i32) (result i32)
    (local $i i32) (local $p i32) (local $end i32) (local $b1 i32) (local $b2 i32)
    (local.set $p (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos)))
    (local.set $end (i32.add (local.get $p) (local.get $kw_len)))
    ;; Check bounds
    (if (i32.gt_u (local.get $end) (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.load (global.get $OFF_WAT_LEN))))
      (then (return (i32.const 0)))
    )
    (local.set $i (i32.const 0))
    (block $done
      (loop $lp
        (if (i32.ge_u (local.get $i) (local.get $kw_len)) (then (br $done)))
        (local.set $b1 (i32.load8_u (i32.add (local.get $p) (local.get $i))))
        (local.set $b2 (i32.load8_u (i32.add (local.get $kw_ptr) (local.get $i))))
        ;; Case-insensitive compare: convert $b1 to lowercase
        (if (i32.and (i32.ge_u (local.get $b1) (i32.const 0x41)) (i32.le_u (local.get $b1) (i32.const 0x5A)))
          (then (local.set $b1 (i32.or (local.get $b1) (i32.const 32))))
        )
        (if (i32.and (i32.ge_u (local.get $b2) (i32.const 0x41)) (i32.le_u (local.get $b2) (i32.const 0x5A)))
          (then (local.set $b2 (i32.or (local.get $b2) (i32.const 32))))
        )
        (if (i32.ne (local.get $b1) (local.get $b2)) (then (return (i32.const 0))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $lp)
      )
    )
    (return (i32.const 1))
  )

  ;; Read a keyword at pos (identifier characters: a-z A-Z 0-9 . _ + - * / < > ! ~)
  ;; Stores in WAT source: the keyword starts at pos, with given length.
  ;; Output: scratch0=keyword_offset, scratch1=keyword_len, scratch2=new_pos
  ;; Returns error code.
  (func $wat_read_kw (param $pos i32) (result i32)
    (local $p i32) (local $end i32) (local $b i32) (local $start i32)
    (local.set $p (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos)))
    (local.set $end (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.load (global.get $OFF_WAT_LEN))))
    (if (i32.ge_u (local.get $p) (local.get $end))
      (then (return (global.get $ERR_PARSE)))
    )
    (local.set $start (local.get $p))
    (block $done
      (loop $lp
        (if (i32.ge_u (local.get $p) (local.get $end)) (then (br $done)))
        (local.set $b (i32.load8_u (local.get $p)))
        ;; Check if identifier character
        (block $is_id
          (if (i32.and (i32.ge_u (local.get $b) (i32.const 0x61)) (i32.le_u (local.get $b) (i32.const 0x7A))) (then (br $is_id)))
          (if (i32.and (i32.ge_u (local.get $b) (i32.const 0x41)) (i32.le_u (local.get $b) (i32.const 0x5A))) (then (br $is_id)))
          (if (i32.and (i32.ge_u (local.get $b) (i32.const 0x30)) (i32.le_u (local.get $b) (i32.const 0x39))) (then (br $is_id)))
          (if (i32.eq (local.get $b) (i32.const 0x2E)) (then (br $is_id)))  ;; .
          (if (i32.eq (local.get $b) (i32.const 0x5F)) (then (br $is_id)))  ;; _
          (if (i32.eq (local.get $b) (i32.const 0x2B)) (then (br $is_id)))  ;; +
          (if (i32.eq (local.get $b) (i32.const 0x2D)) (then (br $is_id)))  ;; -
          (if (i32.eq (local.get $b) (i32.const 0x2A)) (then (br $is_id)))  ;; *
          (if (i32.eq (local.get $b) (i32.const 0x2F)) (then (br $is_id)))  ;; /
          (if (i32.eq (local.get $b) (i32.const 0x3C)) (then (br $is_id)))  ;; <
          (if (i32.eq (local.get $b) (i32.const 0x3E)) (then (br $is_id)))  ;; >
          (if (i32.eq (local.get $b) (i32.const 0x21)) (then (br $is_id)))  ;; !
          (if (i32.eq (local.get $b) (i32.const 0x7E)) (then (br $is_id)))  ;; ~
          (if (i32.eq (local.get $b) (i32.const 0x27)) (then (br $is_id)))  ;; '
          (br $done)
        )
        (local.set $p (i32.add (local.get $p) (i32.const 1)))
        (br $lp)
      )
    )
    (if (i32.eq (local.get $start) (local.get $p))
      (then (return (global.get $ERR_PARSE)))
    )
    (i32.store (global.get $OFF_SCRATCH0) (local.get $pos))
    (i32.store (global.get $OFF_SCRATCH1) (i32.sub (local.get $p) (local.get $start)))
    (i32.store (global.get $OFF_SCRATCH2) (i32.add (local.get $pos) (i32.sub (local.get $p) (local.get $start))))
    (return (global.get $OK))
  )

  ;; Read a $identifier starting at pos. Store the name (without $) in names buffer.
  ;; Output: scratch0=name_offset (in names buf), scratch1=name_len, scratch2=new_pos
  ;; Returns error code.
  (func $wat_read_id (param $pos i32) (result i32)
    (local $p i32) (local $end i32) (local $b i32) (local $start i32) (local $name_start i32)
    (local $dst i32) (local $i i32) (local $len i32)
    (local.set $p (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos)))
    (local.set $end (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.load (global.get $OFF_WAT_LEN))))
    (if (i32.ge_u (local.get $p) (local.get $end))
      (then (return (global.get $ERR_PARSE)))
    )
    (local.set $b (i32.load8_u (local.get $p)))
    (if (i32.ne (local.get $b) (i32.const 0x24))  ;; '$'
      (then (return (global.get $ERR_PARSE)))
    )
    (local.set $p (i32.add (local.get $p) (i32.const 1)))
    (local.set $start (local.get $p))
    (block $done
      (loop $lp
        (if (i32.ge_u (local.get $p) (local.get $end)) (then (br $done)))
        (local.set $b (i32.load8_u (local.get $p)))
        (block $is_idc
          (if (i32.and (i32.ge_u (local.get $b) (i32.const 0x61)) (i32.le_u (local.get $b) (i32.const 0x7A))) (then (br $is_idc)))
          (if (i32.and (i32.ge_u (local.get $b) (i32.const 0x41)) (i32.le_u (local.get $b) (i32.const 0x5A))) (then (br $is_idc)))
          (if (i32.and (i32.ge_u (local.get $b) (i32.const 0x30)) (i32.le_u (local.get $b) (i32.const 0x39))) (then (br $is_idc)))
          (if (i32.eq (local.get $b) (i32.const 0x2E)) (then (br $is_idc)))  ;; .
          (if (i32.eq (local.get $b) (i32.const 0x5F)) (then (br $is_idc)))  ;; _
          (if (i32.eq (local.get $b) (i32.const 0x2D)) (then (br $is_idc)))  ;; -
          (if (i32.eq (local.get $b) (i32.const 0x2B)) (then (br $is_idc)))  ;; +
          (if (i32.eq (local.get $b) (i32.const 0x27)) (then (br $is_idc)))  ;; '
          (br $done)
        )
        (local.set $p (i32.add (local.get $p) (i32.const 1)))
        (br $lp)
      )
    )
    (local.set $len (i32.sub (local.get $p) (local.get $start)))
    (if (i32.eqz (local.get $len)) (then (return (global.get $ERR_PARSE))))
    ;; Copy name to names buffer
    (local.set $dst (i32.load (global.get $OFF_NAMES_PTR)))
    (local.set $i (i32.const 0))
    (block $clp
      (loop $cl
        (if (i32.ge_u (local.get $i) (local.get $len)) (then (br $clp)))
        (i32.store8 (i32.add (local.get $dst) (local.get $i))
          (i32.load8_u (i32.add (local.get $start) (local.get $i)))
        )
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $cl)
      )
    )
    (i32.store (global.get $OFF_NAMES_PTR) (i32.add (local.get $dst) (local.get $len)))
    (i32.store (global.get $OFF_SCRATCH0) (local.get $dst))
    (i32.store (global.get $OFF_SCRATCH1) (local.get $len))
    (i32.store (global.get $OFF_SCRATCH2) (i32.add (local.get $pos) (i32.add (i32.const 1) (local.get $len))))
    (return (global.get $OK))
  )

  ;; Read an unsigned integer (decimal or 0x hex).
  ;; Output: scratch0=value, scratch1=new_pos
  ;; Returns error code.
  (func $wat_read_uint (param $pos i32) (result i32)
    (local $p i32) (local $end i32) (local $b i32) (local $val i32) (local $start i32)
    (local $neg i32)
    (local.set $p (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos)))
    (local.set $end (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.load (global.get $OFF_WAT_LEN))))
    (if (i32.ge_u (local.get $p) (local.get $end))
      (then (return (global.get $ERR_PARSE)))
    )
    (local.set $start (local.get $p))
    (local.set $b (i32.load8_u (local.get $p)))
    ;; Optional leading sign for unsigned (will be treated as positive)
    (if (i32.eq (local.get $b) (i32.const 0x2D))
      (then (local.set $p (i32.add (local.get $p) (i32.const 1))) (local.set $neg (i32.const 1)))
    )
    (if (i32.eq (local.get $b) (i32.const 0x2B))
      (then (local.set $p (i32.add (local.get $p) (i32.const 1))))
    )
    ;; Check for hex prefix 0x or 0X
    (local.set $b (i32.load8_u (local.get $p)))
    (if (i32.eq (local.get $b) (i32.const 0x30))
      (then
        (local.set $p (i32.add (local.get $p) (i32.const 1)))
        (if (i32.lt_u (local.get $p) (local.get $end))
          (then
            (local.set $b (i32.load8_u (local.get $p)))
            (if (i32.or (i32.eq (local.get $b) (i32.const 0x78)) (i32.eq (local.get $b) (i32.const 0x58)))
              (then
                ;; Hex number
                (local.set $p (i32.add (local.get $p) (i32.const 1)))
                (local.set $val (i32.const 0))
                (block $hd
                  (loop $hl
                    (if (i32.ge_u (local.get $p) (local.get $end)) (then (br $hd)))
                    (local.set $b (i32.load8_u (local.get $p)))
                    (block $hdig
                      (if (i32.and (i32.ge_u (local.get $b) (i32.const 0x30)) (i32.le_u (local.get $b) (i32.const 0x39)))
                        (then (local.set $b (i32.sub (local.get $b) (i32.const 0x30))) (br $hdig)))
                      (if (i32.and (i32.ge_u (local.get $b) (i32.const 0x61)) (i32.le_u (local.get $b) (i32.const 0x66)))
                        (then (local.set $b (i32.sub (local.get $b) (i32.sub (i32.const 0x61) (i32.const 10)))) (br $hdig)))
                      (if (i32.and (i32.ge_u (local.get $b) (i32.const 0x41)) (i32.le_u (local.get $b) (i32.const 0x46)))
                        (then (local.set $b (i32.sub (local.get $b) (i32.sub (i32.const 0x41) (i32.const 10)))) (br $hdig)))
                      (br $hd)
                    )
                    (local.set $val (i32.add (i32.shl (local.get $val) (i32.const 4)) (local.get $b)))
                    (local.set $p (i32.add (local.get $p) (i32.const 1)))
                    (br $hl)
                  )
                )
                (if (i32.eq (local.get $start) (i32.sub (local.get $p) (i32.const 2)))
                  (then (return (global.get $ERR_PARSE)))  ;; "0x" with no digits
                )
                (i32.store (global.get $OFF_SCRATCH0) (local.get $val))
                (i32.store (global.get $OFF_SCRATCH1) (i32.sub (local.get $p) (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
                (return (global.get $OK))
              )
            )
            ;; Not hex, it's a decimal 0
            (local.set $val (i32.const 0))
            (i32.store (global.get $OFF_SCRATCH0) (local.get $val))
            (i32.store (global.get $OFF_SCRATCH1) (i32.sub (local.get $p) (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
            (return (global.get $OK))
          )
        )
      )
    )
    ;; Decimal
    (local.set $val (i32.const 0))
    (block $dd
      (loop $dl
        (if (i32.ge_u (local.get $p) (local.get $end)) (then (br $dd)))
        (local.set $b (i32.load8_u (local.get $p)))
        (if (i32.or (i32.lt_u (local.get $b) (i32.const 0x30)) (i32.gt_u (local.get $b) (i32.const 0x39)))
          (then (br $dd))
        )
        (local.set $val (i32.add (i32.mul (local.get $val) (i32.const 10)) (i32.sub (local.get $b) (i32.const 0x30))))
        (local.set $p (i32.add (local.get $p) (i32.const 1)))
        (br $dl)
      )
    )
    (i32.store (global.get $OFF_SCRATCH0) (local.get $val))
    (i32.store (global.get $OFF_SCRATCH1) (i32.sub (local.get $p) (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
    (return (global.get $OK))
  )

  ;; Read a signed integer (decimal or 0x hex).
  ;; Output: scratch0=value, scratch1=new_pos
  ;; Returns error code.
  (func $wat_read_sint (param $pos i32) (result i32)
    (local $p i32) (local $end i32) (local $b i32) (local $val i32) (local $neg i32)
    (local.set $p (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos)))
    (local.set $end (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.load (global.get $OFF_WAT_LEN))))
    (if (i32.ge_u (local.get $p) (local.get $end))
      (then (return (global.get $ERR_PARSE)))
    )
    (local.set $neg (i32.const 0))
    (local.set $b (i32.load8_u (local.get $p)))
    (if (i32.eq (local.get $b) (i32.const 0x2D))
      (then (local.set $neg (i32.const 1)) (local.set $p (i32.add (local.get $p) (i32.const 1))))
    )
    (if (i32.eq (local.get $b) (i32.const 0x2B))
      (then (local.set $p (i32.add (local.get $p) (i32.const 1))))
    )
    ;; Save relative offset of number start (after sign)
    (local.set $pos (i32.add (local.get $pos) (i32.sub (local.get $p) (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos)))))
    ;; Use unsigned reader on the rest
    (if (call $wat_read_uint (local.get $pos))
      (then (return (global.get $ERR_PARSE)))
    )
    (local.set $val (i32.load (global.get $OFF_SCRATCH0)))
    (if (local.get $neg)
      (then
        (local.set $val (i32.sub (i32.const 0) (local.get $val)))
        (i32.store (global.get $OFF_SCRATCH0) (local.get $val))
      )
    )
    ;; Add sign chars to position advance
    (i32.store (global.get $OFF_SCRATCH1)
      (i32.add
        (i32.sub (local.get $p) (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos)))
        (i32.load (global.get $OFF_SCRATCH1))
      )
    )
    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; WAT Symbol Table
  ;; ═════════════════════════════════════════════════════════════════════

  ;; Register a name in the symbol table.
  ;; name_off = offset in names buffer, name_len = length, kind = entity kind, index = entity index
  ;; Returns error (ERR_PARSE on overflow).
  (func $wat_sym_register (param $name_off i32) (param $name_len i32) (param $kind i32) (param $index i32) (result i32)
    (local $sym_cnt i32) (local $base i32)
    (local.set $sym_cnt (i32.load (global.get $OFF_WAT_SYM)))
    (if (i32.ge_u (local.get $sym_cnt) (global.get $WAT_MAX_SYMS))
      (then (return (global.get $ERR_PARSE)))
    )
    (local.set $base (i32.add (global.get $OFF_WAT_SYMS) (i32.mul (local.get $sym_cnt) (global.get $WAT_SYM_SZ))))
    (i32.store (i32.add (local.get $base) (i32.const 0)) (local.get $name_off))
    (i32.store (i32.add (local.get $base) (i32.const 4)) (local.get $name_len))
    (i32.store (i32.add (local.get $base) (i32.const 8)) (local.get $kind))
    (i32.store (i32.add (local.get $base) (i32.const 12)) (local.get $index))
    (i32.store (global.get $OFF_WAT_SYM) (i32.add (local.get $sym_cnt) (i32.const 1)))
    (return (global.get $OK))
  )

  ;; Look up a name in the symbol table.
  ;; kind = entity kind to search for (-1 = any kind).
  ;; Output: scratch0=index, scratch1=0 if found, 1 if not found.
  (func $wat_sym_lookup (param $name_off i32) (param $name_len i32) (param $kind i32) (result i32)
    (local $sym_cnt i32) (local $i i32) (local $base i32) (local $n_off i32) (local $n_len i32) (local $k i32)
    (local $j i32) (local $b1 i32) (local $b2 i32)
    (local.set $sym_cnt (i32.load (global.get $OFF_WAT_SYM)))
    (local.set $i (i32.const 0))
    (block $done
      (loop $lp
        (if (i32.ge_u (local.get $i) (local.get $sym_cnt)) (then (br $done)))
        (local.set $base (i32.add (global.get $OFF_WAT_SYMS) (i32.mul (local.get $i) (global.get $WAT_SYM_SZ))))
        (local.set $k (i32.load (i32.add (local.get $base) (i32.const 8))))
        (if (i32.and (i32.ne (local.get $kind) (i32.const -1)) (i32.ne (local.get $k) (local.get $kind)))
          (then (local.set $i (i32.add (local.get $i) (i32.const 1))) (br $lp))
        )
        (local.set $n_len (i32.load (i32.add (local.get $base) (i32.const 4))))
        (if (i32.ne (local.get $n_len) (local.get $name_len))
          (then (local.set $i (i32.add (local.get $i) (i32.const 1))) (br $lp))
        )
        (local.set $n_off (i32.load (i32.add (local.get $base) (i32.const 0))))
        ;; Compare names byte-by-byte
        (local.set $j (i32.const 0))
        (block $cmp_done
          (loop $cmp
            (if (i32.ge_u (local.get $j) (local.get $n_len)) (then (br $cmp_done)))
            (local.set $b1 (i32.load8_u (i32.add (local.get $n_off) (local.get $j))))
            (local.set $b2 (i32.load8_u (i32.add (local.get $name_off) (local.get $j))))
            (if (i32.ne (local.get $b1) (local.get $b2))
              (then (local.set $i (i32.add (local.get $i) (i32.const 1))) (br $lp))
            )
            (local.set $j (i32.add (local.get $j) (i32.const 1)))
            (br $cmp)
          )
        )
        ;; Found!
        (i32.store (global.get $OFF_SCRATCH0) (i32.load (i32.add (local.get $base) (i32.const 12))))
        (i32.store (global.get $OFF_SCRATCH1) (i32.const 0))
        (return (global.get $OK))
      )
    )
    (i32.store (global.get $OFF_SCRATCH1) (i32.const 1))
    (return (global.get $OK))
  )

  ;; Clear the symbol table (for a new function body scope).
  (func $wat_sym_clear (result i32)
    (i32.store (global.get $OFF_WAT_SYM) (i32.const 0))
    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Value type helper: convert WAT type keyword → WASM type byte
  ;; Input: kw_offset, kw_len (from wat_read_kw)
  ;; Output: scratch0 = type byte (0x7F, etc.) or -1 if unknown
  ;; ═════════════════════════════════════════════════════════════════════
  (func $wat_valtype (param $kw_off i32) (param $kw_len i32) (result i32)
    (local $p i32) (local $b0 i32) (local $b1 i32) (local $b2 i32) (local $b3 i32)
    (local.set $p (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $kw_off)))
    (local.set $b0 (i32.load8_u (local.get $p)))
    (if (i32.gt_u (local.get $kw_len) (i32.const 1))
      (then (local.set $b1 (i32.load8_u (i32.add (local.get $p) (i32.const 1)))))
    )
    (if (i32.gt_u (local.get $kw_len) (i32.const 2))
      (then (local.set $b2 (i32.load8_u (i32.add (local.get $p) (i32.const 2)))))
    )
    (block $done
      (if (i32.eq (local.get $kw_len) (i32.const 3))
        (then
          (block $try3
            ;; "i32" = 0x69 0x33 0x32
            (if (i32.and (i32.eq (local.get $b0) (i32.const 0x69)) (i32.and (i32.eq (local.get $b1) (i32.const 0x33)) (i32.eq (local.get $b2) (i32.const 0x32))))
              (then (i32.store (global.get $OFF_SCRATCH0) (i32.const 0x7F)) (return (global.get $OK)))
            )
            ;; "i64" = 0x69 0x36 0x34
            (if (i32.and (i32.eq (local.get $b0) (i32.const 0x69)) (i32.and (i32.eq (local.get $b1) (i32.const 0x36)) (i32.eq (local.get $b2) (i32.const 0x34))))
              (then (i32.store (global.get $OFF_SCRATCH0) (i32.const 0x7E)) (return (global.get $OK)))
            )
            ;; "f32" = 0x66 0x33 0x32
            (if (i32.and (i32.eq (local.get $b0) (i32.const 0x66)) (i32.and (i32.eq (local.get $b1) (i32.const 0x33)) (i32.eq (local.get $b2) (i32.const 0x32))))
              (then (i32.store (global.get $OFF_SCRATCH0) (i32.const 0x7D)) (return (global.get $OK)))
            )
            ;; "f64" = 0x66 0x36 0x34
            (if (i32.and (i32.eq (local.get $b0) (i32.const 0x66)) (i32.and (i32.eq (local.get $b1) (i32.const 0x36)) (i32.eq (local.get $b2) (i32.const 0x34))))
              (then (i32.store (global.get $OFF_SCRATCH0) (i32.const 0x7C)) (return (global.get $OK)))
            )
          )
        )
      )
      (if (i32.eq (local.get $kw_len) (i32.const 4))
        (then
          (local.set $b3 (i32.load8_u (i32.add (local.get $p) (i32.const 3))))
          ;; "v128" = 0x76 ('v') 0x31 ('1') 0x32 ('2') 0x38 ('8')
          (if (i32.and (i32.eq (local.get $b0) (i32.const 0x76)) (i32.and (i32.eq (local.get $b1) (i32.const 0x31)) (i32.and (i32.eq (local.get $b2) (i32.const 0x32)) (i32.eq (local.get $b3) (i32.const 0x38)))))
            (then (i32.store (global.get $OFF_SCRATCH0) (i32.const 0x7B)) (return (global.get $OK)))
          )
        )
      )
    )
    (i32.store (global.get $OFF_SCRATCH0) (i32.const -1))
    (return (global.get $OK))
  )

  ;; Parse a (result ...) type annotation.
  ;; Expects: at pos, we've already consumed "(" and "result".
  ;; Reads value types until ")".
  ;; Output: scratch0=first_valtype_byte (0x40 if no results), scratch1=new_pos
  (func $wat_parse_result (param $pos i32) (result i32)
    (local $b i32) (local $vt i32)
    (local.set $vt (i32.const 0x40))  ;; default empty
    (block $lp
      (loop $cont
        (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
        (if (i32.eq (local.get $b) (i32.const 0x29))  ;; )
          (then (local.set $pos (i32.add (local.get $pos) (i32.const 1))) (br $lp))
        )
        ;; Read value type keyword
        (if (call $wat_read_kw (local.get $pos)) (then (return (global.get $ERR_PARSE))))
        (local.set $vt (i32.load (global.get $OFF_SCRATCH0)))
        (if (call $wat_valtype (i32.load (global.get $OFF_SCRATCH0)) (i32.load (global.get $OFF_SCRATCH1)))
          (then (return (global.get $ERR_PARSE)))
        )
        (local.set $vt (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
        ;; Only store first result type for now
        (br $lp)  ;; only support single result for now
      )
    )
    (i32.store (global.get $OFF_SCRATCH0) (local.get $vt))
    (i32.store (global.get $OFF_SCRATCH1) (local.get $pos))
    (return (global.get $OK))
