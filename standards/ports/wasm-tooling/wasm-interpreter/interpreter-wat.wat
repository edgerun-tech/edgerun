
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
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; WASM bytecode emission helpers (to scratch buffer)
  ;; ═════════════════════════════════════════════════════════════════════

  ;; Emit one byte to the body buffer at given offset.
  ;; Output: scratch0 = new offset
  (func $wat_emit_byte (param $off i32) (param $b i32) (result i32)
    (if (i32.ge_u (local.get $off) (global.get $WAT_BODY_SZ))
      (then (return (global.get $ERR_NO_MEM)))
    )
    (i32.store8 (i32.add (global.get $OFF_WAT_BODY) (local.get $off)) (local.get $b))
    (i32.store (global.get $OFF_SCRATCH0) (i32.add (local.get $off) (i32.const 1)))
    (return (global.get $OK))
  )

  ;; Emit a LEB128 unsigned integer.
  ;; Output: scratch0 = new offset
  (func $wat_emit_leb_u32 (param $off i32) (param $val i32) (result i32)
    (local $b i32)
    (block $done
      (loop $lp
        (local.set $b (i32.and (local.get $val) (i32.const 0x7F)))
        (local.set $val (i32.shr_u (local.get $val) (i32.const 7)))
        (if (i32.ne (local.get $val) (i32.const 0))
          (then (local.set $b (i32.or (local.get $b) (i32.const 0x80))))
        )
        (if (call $wat_emit_byte (local.get $off) (local.get $b)) (then (return (global.get $ERR_NO_MEM))))
        (local.set $off (i32.load (global.get $OFF_SCRATCH0)))
        (if (i32.eqz (local.get $val)) (then (br $done)))
        (br $lp)
      )
    )
    (i32.store (global.get $OFF_SCRATCH0) (local.get $off))
    (return (global.get $OK))
  )

  ;; Emit a LEB128 signed integer (32-bit).
  ;; Output: scratch0 = new offset
  (func $wat_emit_leb_i32 (param $off i32) (param $val i32) (result i32)
    (local $b i32) (local $more i32) (local $sign i32)
    (local.set $sign (i32.and (local.get $val) (i32.const 0x40)))  ;; bit 6 of original
    (block $done
      (loop $lp
        (local.set $b (i32.and (local.get $val) (i32.const 0x7F)))
        (local.set $val (i32.shr_s (local.get $val) (i32.const 7)))
        ;; Check if more bytes needed
        (block $check_more
          (if (i32.eq (local.get $val) (i32.const 0))
            (then
              (if (i32.eqz (i32.and (local.get $b) (i32.const 0x40))) (then (br $check_more)))
            )
            (else
              (if (i32.eq (local.get $val) (i32.const -1))
                (then
                  (if (i32.and (local.get $b) (i32.const 0x40)) (then (br $check_more)))
                )
                (else (br $check_more))
              )
            )
          )
          (local.set $more (i32.const 0))
          (br $done)
        )
        (local.set $more (i32.const 1))
        (local.set $b (i32.or (local.get $b) (i32.const 0x80)))
        (if (call $wat_emit_byte (local.get $off) (local.get $b)) (then (return (global.get $ERR_NO_MEM))))
        (local.set $off (i32.load (global.get $OFF_SCRATCH0)))
        (br $lp)
      )
    )
    (if (call $wat_emit_byte (local.get $off) (local.get $b)) (then (return (global.get $ERR_NO_MEM))))
    (local.set $off (i32.load (global.get $OFF_SCRATCH0)))
    (i32.store (global.get $OFF_SCRATCH0) (local.get $off))
    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Compare rest of keyword bytes (after first byte already matched)
  ;; Input: kw_off, start offset (usually 1), count of bytes to compare,
  ;;        11 expected byte values (unused ones should be 0)
  ;; Returns 1 if mismatch, 0 if match
  ;; ═════════════════════════════════════════════════════════════════════
  (func $wat_kw_match_rest (param $kw_off i32) (param $start i32) (param $count i32)
    (param $p0 i32) (param $p1 i32) (param $p2 i32) (param $p3 i32)
    (param $p4 i32) (param $p5 i32) (param $p6 i32) (param $p7 i32)
    (param $p8 i32) (param $p9 i32)
    (result i32)
    (local $base i32) (local $i i32) (local $b i32)
    (local.set $base (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $kw_off)))
    (local.set $i (i32.const 0))
    (block $done
      (loop $lp
        (if (i32.ge_u (local.get $i) (local.get $count)) (then (br $done)))
        (local.set $b (i32.load8_u (i32.add (local.get $base) (i32.add (local.get $start) (local.get $i)))))
        (if (i32.eq (local.get $i) (i32.const 0))
          (then (if (i32.ne (local.get $b) (local.get $p0)) (then (return (i32.const 1)))))
        )
        (if (i32.eq (local.get $i) (i32.const 1))
          (then (if (i32.ne (local.get $b) (local.get $p1)) (then (return (i32.const 1)))))
        )
        (if (i32.eq (local.get $i) (i32.const 2))
          (then (if (i32.ne (local.get $b) (local.get $p2)) (then (return (i32.const 1)))))
        )
        (if (i32.eq (local.get $i) (i32.const 3))
          (then (if (i32.ne (local.get $b) (local.get $p3)) (then (return (i32.const 1)))))
        )
        (if (i32.eq (local.get $i) (i32.const 4))
          (then (if (i32.ne (local.get $b) (local.get $p4)) (then (return (i32.const 1)))))
        )
        (if (i32.eq (local.get $i) (i32.const 5))
          (then (if (i32.ne (local.get $b) (local.get $p5)) (then (return (i32.const 1)))))
        )
        (if (i32.eq (local.get $i) (i32.const 6))
          (then (if (i32.ne (local.get $b) (local.get $p6)) (then (return (i32.const 1)))))
        )
        (if (i32.eq (local.get $i) (i32.const 7))
          (then (if (i32.ne (local.get $b) (local.get $p7)) (then (return (i32.const 1)))))
        )
        (if (i32.eq (local.get $i) (i32.const 8))
          (then (if (i32.ne (local.get $b) (local.get $p8)) (then (return (i32.const 1)))))
        )
        (if (i32.eq (local.get $i) (i32.const 9))
          (then (if (i32.ne (local.get $b) (local.get $p9)) (then (return (i32.const 1)))))
        )
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $lp)
      )
    )
    (return (i32.const 0))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Function body opcode parser: reads WAT opcodes, emits WASM bytecodes
  ;; Input: pos = position in WAT source after func header
  ;; Output: scratch0 = decoded_start, scratch1 = decoded_count
  ;; Returns error code.
  ;; ═════════════════════════════════════════════════════════════════════

  ;; Parse one opcode. Returns: 0=continue, -1=done(')'), positive=error
  (func $wat_parse_body (param $pos i32) (param $body_off i32) (result i32)
    (local $err i32) (local $b i32) (local $kw_off i32)
    (local $kw_len i32) (local $b0 i32) (local $b1 i32) (local $b2 i32) (local $b3 i32)
    (local $imm i32)

    (i32.store (i32.const 0x8C060) (local.get $pos))
    (i32.store (i32.const 0x8C080) (local.get $body_off))

    ;; Skip whitespace
    (i32.store (i32.const 0x8C048) (i32.const 0x7001))
    (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
    (i32.store (i32.const 0x8C048) (i32.const 0x7002))
    (i32.store (i32.const 0x8C084) (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))

    ;; Check for end of function: closing paren at top level
    (i32.store (i32.const 0x8C048) (i32.const 0x7003))
    (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
    (if (i32.eq (local.get $b) (i32.const 0x29))  ;; )
      (then
        (i32.store (i32.const 0x8C048) (i32.const 0x7010))
        ;; Emit end opcode
        (if (call $wat_emit_byte (local.get $body_off) (i32.const 0x0B))
          (then (return (global.get $ERR_NO_MEM)))
        )
        (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
        ;; Decode the emitted bytecodes
        (i32.store (global.get $OFF_WAT_TMP) (local.get $body_off))
        (i32.store (global.get $OFF_WAT_SAV_PTR) (i32.load (global.get $OFF_WASM_PTR)))
        (i32.store (global.get $OFF_WAT_SAV_LEN) (i32.load (global.get $OFF_WASM_LEN)))
        (i32.store (global.get $OFF_WASM_PTR) (global.get $OFF_WAT_BODY))
        (i32.store (global.get $OFF_WASM_LEN) (local.get $body_off))
        (local.set $err (call $decode_opcodes (i32.const 0) (local.get $body_off)))
        (if (local.get $err) (then (return (local.get $err))))
        (i32.store (global.get $OFF_WASM_PTR) (i32.load (global.get $OFF_WAT_SAV_PTR)))
        (i32.store (global.get $OFF_WASM_LEN) (i32.load (global.get $OFF_WAT_SAV_LEN)))
        (i32.store (global.get $OFF_SCRATCH0) (local.get $body_off))
        (i32.store (global.get $OFF_SCRATCH1) (i32.load (global.get $OFF_DECODED_COUNT)))
        (local.set $err (call $compute_end_targets
          (local.get $body_off)
          (i32.load (global.get $OFF_DECODED_COUNT))
        ))
        (if (local.get $err) (then (return (local.get $err))))
        (i32.store (global.get $OFF_SCRATCH2) (i32.add (local.get $pos) (i32.const 1)))
        (return (i32.const -1))  ;; DONE
      )
    )

    ;; Check for end of input
    (if (i32.ge_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))
                  (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.load (global.get $OFF_WAT_LEN))))
      (then (return (global.get $ERR_PARSE)))
    )

    ;; Read opcode keyword
    (i32.store (i32.const 0x8C048) (i32.const 0x7040))
    (i32.store (i32.const 0x8C088) (local.get $pos))
    (local.set $err (call $wat_read_kw (local.get $pos)))
    (i32.store (i32.const 0x8C048) (i32.const 0x7041))
    (if (local.get $err) (then (return (global.get $ERR_PARSE))))
    (local.set $kw_off (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $kw_len (i32.load (global.get $OFF_SCRATCH1)))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))

      ;; ─── Opcode dispatch ───
      ;; Inline byte comparison: load first few bytes and compare
      (local.set $b0 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $kw_off))))
      (i32.store (i32.const 0x8C04C) (local.get $kw_len))
      (i32.store (i32.const 0x8C050) (local.get $b0))
      (i32.store (i32.const 0x8C054) (local.get $kw_off))
      (i32.store (i32.const 0x8C058) (i32.load (global.get $OFF_WAT_PTR)))
        (block $not_unreachable
          (if (i32.ne (local.get $kw_len) (i32.const 11)) (then (br $not_unreachable)))
          (if (i32.ne (local.get $b0) (i32.const 0x75)) (then (br $not_unreachable)))  ;; 'u'
          (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 10)
                (i32.const 0x6e) (i32.const 0x72) (i32.const 0x65) (i32.const 0x61)  ;; nrea
                (i32.const 0x63) (i32.const 0x68) (i32.const 0x61) (i32.const 0x62)  ;; chab
                (i32.const 0x6c) (i32.const 0x65))  ;; le
            (then (br $not_unreachable))
          )
          ;; unreachable
          (local.set $b (i32.const 0x00))
          (if (call $wat_emit_byte (local.get $body_off) (local.get $b)) (then (return (global.get $ERR_NO_MEM))))
          (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
          (i32.store (global.get $OFF_SCRATCH0) (local.get $body_off))
          (i32.store (global.get $OFF_SCRATCH1) (local.get $pos))
          (return (i32.const 0))
        )
        (block $not_nop
          (if (i32.ne (local.get $kw_len) (i32.const 3)) (then (br $not_nop)))
          (if (i32.ne (local.get $b0) (i32.const 0x6e)) (then (br $not_nop)))  ;; 'n'
          (local.set $b1 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.add (local.get $kw_off) (i32.const 1)))))
          (local.set $b2 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.add (local.get $kw_off) (i32.const 2)))))
          (if (i32.ne (local.get $b1) (i32.const 0x6f)) (then (br $not_nop)))  ;; 'o'
          (if (i32.ne (local.get $b2) (i32.const 0x70)) (then (br $not_nop)))  ;; 'p'
          ;; nop
          (if (call $wat_emit_byte (local.get $body_off) (i32.const 0x01)) (then (return (global.get $ERR_NO_MEM))))
          (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
          (i32.store (global.get $OFF_SCRATCH0) (local.get $body_off))
          (i32.store (global.get $OFF_SCRATCH1) (local.get $pos))
          (return (i32.const 0))
        )
        (block $not_i32add
          (if (i32.ne (local.get $kw_len) (i32.const 7)) (then (br $not_i32add)))
          (if (i32.ne (local.get $b0) (i32.const 0x69)) (then (br $not_i32add)))  ;; 'i'
          (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 6)
                (i32.const 0x33) (i32.const 0x32) (i32.const 0x2e) (i32.const 0x61)  ;; 32.a
                (i32.const 0x64) (i32.const 0x64) (i32.const 0) (i32.const 0)  ;; dd
                (i32.const 0) (i32.const 0))
            (then (br $not_i32add))
          )
          ;; i32.add
          (if (call $wat_emit_byte (local.get $body_off) (i32.const 0x6a)) (then (return (global.get $ERR_NO_MEM))))
          (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
          (i32.store (global.get $OFF_SCRATCH0) (local.get $body_off))
          (i32.store (global.get $OFF_SCRATCH1) (local.get $pos))
          (return (i32.const 0))
        )
        (block $not_i32sub
          (if (i32.ne (local.get $kw_len) (i32.const 7)) (then (br $not_i32sub)))
          (if (i32.ne (local.get $b0) (i32.const 0x69)) (then (br $not_i32sub)))
          (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 6)
                (i32.const 0x33) (i32.const 0x32) (i32.const 0x2e) (i32.const 0x73)  ;; 32.s
                (i32.const 0x75) (i32.const 0x62) (i32.const 0) (i32.const 0)  ;; ub
                (i32.const 0) (i32.const 0))
            (then (br $not_i32sub))
          )
          ;; i32.sub
          (if (call $wat_emit_byte (local.get $body_off) (i32.const 0x6b)) (then (return (global.get $ERR_NO_MEM))))
          (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
          (i32.store (global.get $OFF_SCRATCH0) (local.get $body_off))
          (i32.store (global.get $OFF_SCRATCH1) (local.get $pos))
          (return (i32.const 0))
        )
        (block $not_i32const
          (if (i32.ne (local.get $kw_len) (i32.const 9)) (then (br $not_i32const)))
          (if (i32.ne (local.get $b0) (i32.const 0x69)) (then (br $not_i32const)))
          (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 8)
                (i32.const 0x33) (i32.const 0x32) (i32.const 0x2e) (i32.const 0x63)  ;; 32.c
                (i32.const 0x6f) (i32.const 0x6e) (i32.const 0x73) (i32.const 0x74)  ;; onst
                (i32.const 0) (i32.const 0))
            (then (br $not_i32const))
          )
          ;; i32.const — read immediate value
          (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
          (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
          (if (call $wat_read_sint (local.get $pos)) (then (return (global.get $ERR_PARSE))))
          (local.set $imm (i32.load (global.get $OFF_SCRATCH0)))
          (local.set $pos (i32.load (global.get $OFF_SCRATCH1)))
          (if (call $wat_emit_byte (local.get $body_off) (i32.const 0x41)) (then (return (global.get $ERR_NO_MEM))))
          (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
          (if (call $wat_emit_leb_i32 (local.get $body_off) (local.get $imm)) (then (return (global.get $ERR_NO_MEM))))
          (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
          (i32.store (global.get $OFF_SCRATCH0) (local.get $body_off))
          (i32.store (global.get $OFF_SCRATCH1) (local.get $pos))
          (return (i32.const 0))
        )
        (block $not_localget
          (if (i32.ne (local.get $kw_len) (i32.const 9)) (then (br $not_localget)))
          (if (i32.ne (local.get $b0) (i32.const 0x6c)) (then (br $not_localget)))  ;; 'l'
          (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 8)
                (i32.const 0x6f) (i32.const 0x63) (i32.const 0x61) (i32.const 0x6c)  ;; ocal
                (i32.const 0x2e) (i32.const 0x67) (i32.const 0x65) (i32.const 0x74)  ;; .get
                (i32.const 0) (i32.const 0))
            (then (br $not_localget))
          )
          ;; local.get — read immediate local index
          (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
          (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
          (if (call $wat_read_uint (local.get $pos)) (then (return (global.get $ERR_PARSE))))
          (local.set $imm (i32.load (global.get $OFF_SCRATCH0)))
          (local.set $pos (i32.load (global.get $OFF_SCRATCH1)))
          (if (call $wat_emit_byte (local.get $body_off) (i32.const 0x20)) (then (return (global.get $ERR_NO_MEM))))
          (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
          (if (call $wat_emit_leb_u32 (local.get $body_off) (local.get $imm)) (then (return (global.get $ERR_NO_MEM))))
          (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
          (i32.store (global.get $OFF_SCRATCH0) (local.get $body_off))
          (i32.store (global.get $OFF_SCRATCH1) (local.get $pos))
          (return (i32.const 0))
        )
        (block $not_localset
          (if (i32.ne (local.get $kw_len) (i32.const 9)) (then (br $not_localset)))
          (if (i32.ne (local.get $b0) (i32.const 0x6c)) (then (br $not_localset)))
          (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 8)
                (i32.const 0x6f) (i32.const 0x63) (i32.const 0x61) (i32.const 0x6c)  ;; ocal
                (i32.const 0x2e) (i32.const 0x73) (i32.const 0x65) (i32.const 0x74)  ;; .set
                (i32.const 0) (i32.const 0))
            (then (br $not_localset))
          )
          ;; local.set — read immediate local index
          (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
          (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
          (if (call $wat_read_uint (local.get $pos)) (then (return (global.get $ERR_PARSE))))
          (local.set $imm (i32.load (global.get $OFF_SCRATCH0)))
          (local.set $pos (i32.load (global.get $OFF_SCRATCH1)))
          (if (call $wat_emit_byte (local.get $body_off) (i32.const 0x21)) (then (return (global.get $ERR_NO_MEM))))
          (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
          (if (call $wat_emit_leb_u32 (local.get $body_off) (local.get $imm)) (then (return (global.get $ERR_NO_MEM))))
          (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
          (i32.store (global.get $OFF_SCRATCH0) (local.get $body_off))
          (i32.store (global.get $OFF_SCRATCH1) (local.get $pos))
          (return (i32.const 0))
        )
        (block $not_localtee
          (if (i32.ne (local.get $kw_len) (i32.const 9)) (then (br $not_localtee)))
          (if (i32.ne (local.get $b0) (i32.const 0x6c)) (then (br $not_localtee)))
          (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 8)
                (i32.const 0x6f) (i32.const 0x63) (i32.const 0x61) (i32.const 0x6c)  ;; ocal
                (i32.const 0x2e) (i32.const 0x74) (i32.const 0x65) (i32.const 0x65)  ;; .tee
                (i32.const 0) (i32.const 0))
            (then (br $not_localtee))
          )
          ;; local.tee — read immediate local index
          (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
          (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
          (if (call $wat_read_uint (local.get $pos)) (then (return (global.get $ERR_PARSE))))
          (local.set $imm (i32.load (global.get $OFF_SCRATCH0)))
          (local.set $pos (i32.load (global.get $OFF_SCRATCH1)))
          (if (call $wat_emit_byte (local.get $body_off) (i32.const 0x22)) (then (return (global.get $ERR_NO_MEM))))
          (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
          (if (call $wat_emit_leb_u32 (local.get $body_off) (local.get $imm)) (then (return (global.get $ERR_NO_MEM))))
          (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
          (i32.store (global.get $OFF_SCRATCH0) (local.get $body_off))
          (i32.store (global.get $OFF_SCRATCH1) (local.get $pos))
          (return (i32.const 0))
        )
        (block $not_return
          (if (i32.ne (local.get $kw_len) (i32.const 6)) (then (br $not_return)))
          (if (i32.ne (local.get $b0) (i32.const 0x72)) (then (br $not_return)))  ;; 'r'
          (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 5)
                (i32.const 0x65) (i32.const 0x74) (i32.const 0x75) (i32.const 0x72)  ;; etur
                (i32.const 0x6e) (i32.const 0) (i32.const 0) (i32.const 0)  ;; n
                (i32.const 0) (i32.const 0))
            (then (br $not_return))
          )
          ;; return
          (if (call $wat_emit_byte (local.get $body_off) (i32.const 0x0F)) (then (return (global.get $ERR_NO_MEM))))
          (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
          (i32.store (global.get $OFF_SCRATCH0) (local.get $body_off))
          (i32.store (global.get $OFF_SCRATCH1) (local.get $pos))
          (return (i32.const 0))
        )
        (block $not_drop
          (if (i32.ne (local.get $kw_len) (i32.const 4)) (then (br $not_drop)))
          (if (i32.ne (local.get $b0) (i32.const 0x64)) (then (br $not_drop)))  ;; 'd'
          (local.set $b1 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.add (local.get $kw_off) (i32.const 1)))))
          (local.set $b2 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.add (local.get $kw_off) (i32.const 2)))))
          (local.set $b3 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.add (local.get $kw_off) (i32.const 3)))))
          (if (i32.ne (local.get $b1) (i32.const 0x72)) (then (br $not_drop)))  ;; 'r'
          (if (i32.ne (local.get $b2) (i32.const 0x6f)) (then (br $not_drop)))  ;; 'o'
          (if (i32.ne (local.get $b3) (i32.const 0x70)) (then (br $not_drop)))  ;; 'p'
          ;; drop
          (if (call $wat_emit_byte (local.get $body_off) (i32.const 0x1A)) (then (return (global.get $ERR_NO_MEM))))
          (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
          (i32.store (global.get $OFF_SCRATCH0) (local.get $body_off))
          (i32.store (global.get $OFF_SCRATCH1) (local.get $pos))
          (return (i32.const 0))
        )
        (block $not_end
          (if (i32.ne (local.get $kw_len) (i32.const 3)) (then (br $not_end)))
          (if (i32.ne (local.get $b0) (i32.const 0x65)) (then (br $not_end)))  ;; 'e'
          (local.set $b1 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.add (local.get $kw_off) (i32.const 1)))))
          (local.set $b2 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.add (local.get $kw_off) (i32.const 2)))))
          (if (i32.ne (local.get $b1) (i32.const 0x6e)) (then (br $not_end)))  ;; 'n'
          (if (i32.ne (local.get $b2) (i32.const 0x64)) (then (br $not_end)))  ;; 'd'
          ;; end — emit end opcode
          (if (call $wat_emit_byte (local.get $body_off) (i32.const 0x0B)) (then (return (global.get $ERR_NO_MEM))))
          (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
          (i32.store (global.get $OFF_SCRATCH0) (local.get $body_off))
          (i32.store (global.get $OFF_SCRATCH1) (local.get $pos))
          (return (i32.const 0))
        )

        ;; Unknown opcode
        (i32.store (i32.const 0x8C048) (i32.const 0x3100))
        (return (global.get $ERR_PARSE))
      )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Top-level declaration parsers
  ;; ═════════════════════════════════════════════════════════════════════

  ;; Parse (type ...) declaration.
  ;; Input: pos after "(type"
  ;; Output: scratch0 = new_pos, scratch1 = type_index
  (func $wat_parse_type_decl (param $pos i32) (result i32)
    (local $err i32) (local $tc i32) (local $base i32) (local $rc i32)
    (local $i i32) (local $vt i32) (local $b i32) (local $kw_off i32) (local $kw_len i32)
    (local $p0 i32) (local $p1 i32) (local $p2 i32) (local $p3 i32)

    ;; Skip optional $name identifier
    (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
    (if (i32.eq (local.get $b) (i32.const 0x24))  ;; '$'
      (then
        (if (call $wat_read_id (local.get $pos)) (then (return (global.get $ERR_PARSE))))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
      )
    )

    ;; Expect "("
    (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
    (if (i32.ne (local.get $b) (i32.const 0x28))  ;; '('
      (then (return (global.get $ERR_PARSE)))
    )
    (local.set $pos (i32.add (local.get $pos) (i32.const 1)))

    ;; Expect "func"
    (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
    (if (call $wat_read_kw (local.get $pos)) (then (return (global.get $ERR_PARSE))))
    (local.set $kw_off (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $kw_len (i32.load (global.get $OFF_SCRATCH1)))
    (local.set $p0 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $kw_off))))
    (if (i32.ne (local.get $kw_len) (i32.const 4))
      (then (return (global.get $ERR_PARSE)))
    )
    (if (i32.ne (local.get $p0) (i32.const 0x66))  ;; 'f'
      (then (return (global.get $ERR_PARSE)))
    )
    (local.set $p1 (i32.load8_u (i32.add (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $kw_off)) (i32.const 1))))
    (local.set $p2 (i32.load8_u (i32.add (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $kw_off)) (i32.const 2))))
    (local.set $p3 (i32.load8_u (i32.add (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $kw_off)) (i32.const 3))))
    (if (i32.ne (local.get $p1) (i32.const 0x75))  ;; 'u'
      (then (return (global.get $ERR_PARSE)))
    )
    (if (i32.ne (local.get $p2) (i32.const 0x6e))  ;; 'n'
      (then (return (global.get $ERR_PARSE)))
    )
    (if (i32.ne (local.get $p3) (i32.const 0x63))  ;; 'c'
      (then (return (global.get $ERR_PARSE)))
    )
    (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))

    ;; Get type count
    (local.set $tc (i32.load (global.get $OFF_TYPE_COUNT)))
    (if (i32.ge_u (local.get $tc) (global.get $MAX_TYPES)) (then (return (global.get $ERR_PARSE))))
    (local.set $base (i32.add (global.get $OFF_TYPES_BUF) (i32.mul (local.get $tc) (global.get $SZ_TYPE))))

    ;; Store functype marker at offset 0 (not strictly needed but follows binary pattern)
    ;; The interpreter uses SZ_TYPE = 256 with structured fields, not this marker.

    ;; Parse (param ...) and (result ...)
    (local.set $rc (i32.const 0))
    (block $tlp
      (loop $tcont
        (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
        (if (i32.eq (local.get $b) (i32.const 0x29))  ;; ')'
          (then (local.set $pos (i32.add (local.get $pos) (i32.const 1))) (br $tlp))
        )
        ;; Expect "("
        (if (i32.ne (local.get $b) (i32.const 0x28))  ;; '('
          (then (return (global.get $ERR_PARSE)))
        )
        (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
        ;; Read keyword: "param" or "result"
        (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
        (if (call $wat_read_kw (local.get $pos)) (then (return (global.get $ERR_PARSE))))
        (local.set $kw_off (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $kw_len (i32.load (global.get $OFF_SCRATCH1)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))

        ;; Check if keyword is "param" (5 bytes)
        (local.set $p0 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $kw_off))))
        (block $not_param
          (if (i32.ne (local.get $kw_len) (i32.const 5)) (then (br $not_param)))
          (if (i32.ne (local.get $p0) (i32.const 0x70)) (then (br $not_param)))  ;; 'p'
          (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 4)
                (i32.const 0x61) (i32.const 0x72) (i32.const 0x61) (i32.const 0x6d)  ;; aram
                (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 0)
                (i32.const 0) (i32.const 0))
            (then (br $not_param))
          )
          ;; Read param types until ')'
          (block $plp
              (loop $pcont
                (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
                (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
                (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
                (if (i32.eq (local.get $b) (i32.const 0x29))  ;; ')'
                  (then (local.set $pos (i32.add (local.get $pos) (i32.const 1))) (br $plp))
                )
                ;; Skip optional $name
                (if (i32.eq (local.get $b) (i32.const 0x24))
                  (then
                    (if (call $wat_read_id (local.get $pos)) (then (return (global.get $ERR_PARSE))))
                    (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
                    (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
                    (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
                  )
                )
                ;; Read value type
                (if (call $wat_read_kw (local.get $pos)) (then (return (global.get $ERR_PARSE))))
                (if (call $wat_valtype (i32.load (global.get $OFF_SCRATCH0)) (i32.load (global.get $OFF_SCRATCH1)))
                  (then (return (global.get $ERR_PARSE)))
                )
                (local.set $vt (i32.load (global.get $OFF_SCRATCH0)))
                ;; Store param type
                (if (i32.gt_u (local.get $rc) (i32.const 31)) (then (return (global.get $ERR_PARSE))))
                (i32.store8 (i32.add (local.get $base) (local.get $rc)) (local.get $vt))
                (local.set $rc (i32.add (local.get $rc) (i32.const 1)))
                (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
                (br $pcont)
              )
            )
            (br $tcont)
          )

        ;; Check if keyword is "result" (6 bytes)
        (block $not_result
          (if (i32.ne (local.get $kw_len) (i32.const 6)) (then (br $not_result)))
          (if (i32.ne (local.get $p0) (i32.const 0x72)) (then (br $not_result)))  ;; 'r'
          (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 5)
                (i32.const 0x65) (i32.const 0x73) (i32.const 0x75) (i32.const 0x6c)  ;; esul
                (i32.const 0x74) (i32.const 0) (i32.const 0) (i32.const 0)  ;; t
                (i32.const 0) (i32.const 0))
            (then (br $not_result))
          )
          ;; Read result types until ')'
          (local.set $i (i32.const 0))
            (block $rlp
              (loop $rcont
                (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
                (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
                (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
                (if (i32.eq (local.get $b) (i32.const 0x29))  ;; ')'
                  (then (local.set $pos (i32.add (local.get $pos) (i32.const 1))) (br $rlp))
                )
                ;; Read value type
                (if (call $wat_read_kw (local.get $pos)) (then (return (global.get $ERR_PARSE))))
                (if (call $wat_valtype (i32.load (global.get $OFF_SCRATCH0)) (i32.load (global.get $OFF_SCRATCH1)))
                  (then (return (global.get $ERR_PARSE)))
                )
                (local.set $vt (i32.load (global.get $OFF_SCRATCH0)))
                ;; Store result type
                (if (i32.gt_u (local.get $i) (i32.const 3)) (then (return (global.get $ERR_PARSE))))
                (i32.store8 (i32.add (local.get $base) (i32.const 132)) (local.get $vt))  ;; result_types at +132
                (local.set $i (i32.add (local.get $i) (i32.const 1)))
                (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
                (br $rcont)
              )
            )
            ;; Store result_count at +136
            (i32.store16 (i32.add (local.get $base) (i32.const 136)) (local.get $i))
            ;; Expect closing ')'
            (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
            (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
            (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
            (if (i32.ne (local.get $b) (i32.const 0x29)) (then (return (global.get $ERR_PARSE))))
            (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
            (br $tlp)
        )
        (return (global.get $ERR_PARSE))
      )
    )

    ;; Store param_count at +128
    (i32.store16 (i32.add (local.get $base) (i32.const 128)) (local.get $rc))

    ;; Increment type count
    (i32.store (global.get $OFF_TYPE_COUNT) (i32.add (local.get $tc) (i32.const 1)))

    (i32.store (global.get $OFF_SCRATCH0) (local.get $pos))
    (i32.store (global.get $OFF_SCRATCH1) (local.get $tc))
    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Parse (func ...) declaration (inline)
  ;; Input: pos after "(func"
  ;; Output: scratch0 = new_pos
  ;; ═════════════════════════════════════════════════════════════════════
  (func $wat_parse_func_decl (param $pos i32) (result i32)
    (local $err i32) (local $b i32) (local $kw_off i32) (local $kw_len i32)
    (local $tc i32) (local $base i32) (local $rc i32) (local $i i32) (local $vt i32)
    (local $fc i32) (local $code_i i32)
    (local $export_name_ptr i32) (local $export_name_len i32)
    (local $dst i32) (local $body_len i32)
    (local $has_export i32)
    (local $p0 i32) (local $p1 i32) (local $p2 i32) (local $p3 i32)
    (local $result_count i32)
    (local $decoded_start i32) (local $decoded_count i32)
    (local $body_off i32)

    ;; ── Skip optional $name ──
    (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
    (if (i32.eq (local.get $b) (i32.const 0x24))  ;; '$'
      (then
        (if (call $wat_read_id (local.get $pos)) (then (return (global.get $ERR_PARSE))))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
      )
    )

    ;; ── Check for (export "name") ──
    (local.set $has_export (i32.const 0))
    (block $export_check_done
      ;; Look for '('
      (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
      (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
      (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
      (if (i32.ne (local.get $b) (i32.const 0x28)) (then (br $export_check_done)))  ;; not '('
      (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
      ;; Read keyword
      (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
      (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
      (if (call $wat_read_kw (local.get $pos)) (then (return (global.get $ERR_PARSE))))
      (local.set $kw_off (i32.load (global.get $OFF_SCRATCH0)))
      (local.set $kw_len (i32.load (global.get $OFF_SCRATCH1)))
      (local.set $p0 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $kw_off))))
      ;; Check "export" (6 bytes)
      (if (i32.ne (local.get $kw_len) (i32.const 6)) (then (return (global.get $ERR_PARSE))))
      (if (i32.ne (local.get $p0) (i32.const 0x65)) (then (return (global.get $ERR_PARSE))))
      (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 5)
            (i32.const 0x78) (i32.const 0x70) (i32.const 0x6f) (i32.const 0x72)
            (i32.const 0x74) (i32.const 0) (i32.const 0) (i32.const 0)
            (i32.const 0) (i32.const 0))
        (then (return (global.get $ERR_PARSE)))
      )
      (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))

      ;; Parse export name string: "name"
      (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
      (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
      (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
      (if (i32.ne (local.get $b) (i32.const 0x22)) (then (return (global.get $ERR_PARSE))))  ;; '"'
      (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
      ;; Read name bytes into names buffer
      (local.set $export_name_ptr (i32.load (global.get $OFF_NAMES_PTR)))
      (local.set $export_name_len (i32.const 0))
      (block $ename_done
        (loop $ename_lp
          (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
          (if (i32.eq (local.get $b) (i32.const 0x22))  ;; closing '"'
            (then
              (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
              (br $ename_done)
            )
          )
          (i32.store8 (i32.add (local.get $export_name_ptr) (local.get $export_name_len)) (local.get $b))
          (local.set $export_name_len (i32.add (local.get $export_name_len) (i32.const 1)))
          (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
          (br $ename_lp)
        )
      )
      ;; Advance names buffer pointer
      (i32.store (global.get $OFF_NAMES_PTR) (i32.add (local.get $export_name_ptr) (local.get $export_name_len)))
      ;; Expect ')'
      (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
      (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
      (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
      (if (i32.ne (local.get $b) (i32.const 0x29)) (then (return (global.get $ERR_PARSE))))  ;; ')'
      (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
      (local.set $has_export (i32.const 1))
    )

    ;; ── Parse inline type: (param ...) and (result ...) ──
    (local.set $tc (i32.load (global.get $OFF_TYPE_COUNT)))
    (if (i32.ge_u (local.get $tc) (global.get $MAX_TYPES)) (then (return (global.get $ERR_PARSE))))
    (local.set $base (i32.add (global.get $OFF_TYPES_BUF) (i32.mul (local.get $tc) (global.get $SZ_TYPE))))
    (local.set $rc (i32.const 0))

    (block $type_done
      (loop $type_lp
        (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
        ;; If we see `)` we're done with type declarations, go to body
        (if (i32.eq (local.get $b) (i32.const 0x29)) (then (local.set $pos (i32.add (local.get $pos) (i32.const 1))) (br $type_done)))
        ;; Must be '('
        (if (i32.ne (local.get $b) (i32.const 0x28)) (then (br $type_done)))  ;; not '(', assume body starts
        (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
        ;; Read keyword
        (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
        (if (call $wat_read_kw (local.get $pos)) (then (return (global.get $ERR_PARSE))))
        (local.set $kw_off (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $kw_len (i32.load (global.get $OFF_SCRATCH1)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
        (local.set $p0 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $kw_off))))

        ;; "param" (5 bytes)
        (block $not_p
          (if (i32.ne (local.get $kw_len) (i32.const 5)) (then (br $not_p)))
          (if (i32.ne (local.get $p0) (i32.const 0x70)) (then (br $not_p)))
          (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 4)
                (i32.const 0x61) (i32.const 0x72) (i32.const 0x61) (i32.const 0x6d)
                (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 0)
                (i32.const 0) (i32.const 0))
            (then (br $not_p))
          )
          ;; Param types until ')'
          (block $param_done
            (loop $param_lp
              (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
              (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
              (if (i32.eq (local.get $b) (i32.const 0x29)) (then (local.set $pos (i32.add (local.get $pos) (i32.const 1))) (br $param_done)))  ;; ')'
              ;; Read value type
              (if (call $wat_read_kw (local.get $pos)) (then (return (global.get $ERR_PARSE))))
              (if (call $wat_valtype (i32.load (global.get $OFF_SCRATCH0)) (i32.load (global.get $OFF_SCRATCH1)))
                (then (return (global.get $ERR_PARSE)))
              )
              (local.set $vt (i32.load (global.get $OFF_SCRATCH0)))
              (i32.store8 (i32.add (local.get $base) (local.get $rc)) (local.get $vt))
              (local.set $rc (i32.add (local.get $rc) (i32.const 1)))
              (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
              (br $param_lp)
            )
          )
          (br $type_lp)
        )

        ;; "result" (6 bytes)
        (block $not_r
          (if (i32.ne (local.get $kw_len) (i32.const 6)) (then (br $not_r)))
          (if (i32.ne (local.get $p0) (i32.const 0x72)) (then (br $not_r)))
          (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 5)
                (i32.const 0x65) (i32.const 0x73) (i32.const 0x75) (i32.const 0x6c)
                (i32.const 0x74) (i32.const 0) (i32.const 0) (i32.const 0)
                (i32.const 0) (i32.const 0))
            (then (br $not_r))
          )
          ;; Result types until ')'
          (local.set $result_count (i32.const 0))
          (block $res_done
            (loop $res_lp
              (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
              (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
              (if (i32.eq (local.get $b) (i32.const 0x29)) (then (local.set $pos (i32.add (local.get $pos) (i32.const 1))) (br $res_done)))
              ;; Read value type
              (if (call $wat_read_kw (local.get $pos)) (then (return (global.get $ERR_PARSE))))
              (if (call $wat_valtype (i32.load (global.get $OFF_SCRATCH0)) (i32.load (global.get $OFF_SCRATCH1)))
                (then (return (global.get $ERR_PARSE)))
              )
              (local.set $vt (i32.load (global.get $OFF_SCRATCH0)))
              (i32.store8 (i32.add (local.get $base) (i32.const 132)) (local.get $vt))
              (local.set $result_count (i32.add (local.get $result_count) (i32.const 1)))
              (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
              (br $res_lp)
            )
          )
          (i32.store16 (i32.add (local.get $base) (i32.const 136)) (local.get $result_count))
          (br $type_lp)
        )

        ;; Not param or result — unread the open paren and go to body
        (local.set $pos (i32.sub (local.get $pos) (i32.const 1)))
        (br $type_done)
      )
    )

    ;; Store param_count at +128
    (i32.store16 (i32.add (local.get $base) (i32.const 128)) (local.get $rc))
    ;; Increment type count
    (i32.store (global.get $OFF_TYPE_COUNT) (i32.add (local.get $tc) (i32.const 1)))

    ;; ── Store function entry ──
    (local.set $fc (i32.load (global.get $OFF_FUNCTION_COUNT)))
    (i32.store (i32.add (global.get $OFF_FUNCTIONS_BUF) (i32.mul (local.get $fc) (global.get $SZ_FUNC))) (local.get $tc))

    ;; ── Parse body ──
    (local.set $body_off (i32.const 0))
    (block $body_done
      (loop $body_loop
        (local.set $err (call $wat_parse_body (local.get $pos) (local.get $body_off)))
        (if (i32.lt_s (local.get $err) (i32.const 0)) (then (br $body_done)))  ;; DONE
        (if (local.get $err)
          (then
            (i32.store (global.get $OFF_WAT_DBG) (i32.const 0x2001))
            (i32.store (global.get $OFF_WAT_TMP) (local.get $err))
            (return (global.get $ERR_PARSE))
          )
        )
        ;; CONTINUE
        (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH1)))
        (br $body_loop)
      )
    )
    ;; After $wat_parse_body: scratch0=decoded_start, scratch1=decoded_count
    (local.set $decoded_start (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $decoded_count (i32.load (global.get $OFF_SCRATCH1)))
    (local.set $body_len (i32.load (global.get $OFF_WAT_TMP)))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))

    ;; ── Store code entry ──
    (local.set $code_i (i32.load (global.get $OFF_CODE_COUNT)))
    (local.set $base (i32.add (global.get $OFF_CODE_BUF) (i32.mul (local.get $code_i) (global.get $SZ_CODE))))
    (i32.store (i32.add (local.get $base) (i32.const 0)) (i32.const 0))  ;; body_offset = 0
    (i32.store (i32.add (local.get $base) (i32.const 8)) (local.get $body_len))  ;; body_len
    (i32.store (i32.add (local.get $base) (i32.const 16)) (i32.const 0))  ;; local_count = 0
    (i32.store (i32.add (local.get $base) (i32.const 24)) (local.get $decoded_start))  ;; decoded_start
    (i32.store (i32.add (local.get $base) (i32.const 32)) (local.get $decoded_count))  ;; decoded_count

    ;; ── Store export entry (if export clause present) ──
    (if (local.get $has_export)
      (then
        (local.set $i (i32.load (global.get $OFF_EXPORT_COUNT)))
        (local.set $base (i32.add (global.get $OFF_EXPORTS_BUF) (i32.mul (local.get $i) (global.get $SZ_EXPORT))))
        (i32.store (i32.add (local.get $base) (i32.const 0)) (local.get $export_name_ptr))  ;; name_ptr
        (i32.store (i32.add (local.get $base) (i32.const 8)) (local.get $export_name_len))  ;; name_len
        (i32.store8 (i32.add (local.get $base) (i32.const 16)) (i32.const 0x00))  ;; kind = func
        (i32.store (i32.add (local.get $base) (i32.const 24)) (local.get $fc))  ;; index
        (i32.store (global.get $OFF_EXPORT_COUNT) (i32.add (local.get $i) (i32.const 1)))
      )
    )

    ;; ── Update counters ──
    (i32.store (global.get $OFF_FUNCTION_COUNT) (i32.add (local.get $fc) (i32.const 1)))
    (i32.store (global.get $OFF_CODE_COUNT) (i32.add (local.get $code_i) (i32.const 1)))

    ;; ── Skip closing ')' of func — $wat_parse_body already consumed it ──
    (i32.store (global.get $OFF_SCRATCH0) (local.get $pos))
    (return (global.get $OK))
  )

  (global $OFF_WAT_DBG i32 (i32.const 0x8C020))

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Main WAT module parser
  ;; Input: WAT source at wasm_ptr (we'll save it), length = wasm_len
  ;; ═════════════════════════════════════════════════════════════════════

  (func $wat_parse_module (param $wat_ptr i32) (param $wat_len i32) (result i32)
    (local $pos i32) (local $err i32) (local $b i32)
    (local $func_i i32) (local $func_cnt i32) (local $type_idx i32)
    (local $code_i i32) (local $code_cnt i32)
    (local $export_i i32) (local $export_cnt i32)
    (local $import_i i32) (local $import_cnt i32)
    (local $global_i i32) (local $global_cnt i32)
    (local $start_func i32)
    (local $kw_off i32) (local $kw_len i32) (local $open_parens i32)
    (local $b0 i32) (local $b1 i32) (local $b2 i32) (local $b3 i32)

    (i32.store (global.get $OFF_WAT_DBG) (i32.const 1))  ;; debug: entered

    ;; Save original wasm_ptr/len
    (i32.store (global.get $OFF_WAT_SAV_PTR) (i32.load (global.get $OFF_WASM_PTR)))
    (i32.store (global.get $OFF_WAT_SAV_LEN) (i32.load (global.get $OFF_WASM_LEN)))

    ;; Store WAT source as the "wasm" pointer temporarily for lexer access
    (i32.store (global.get $OFF_WAT_PTR) (local.get $wat_ptr))
    (i32.store (global.get $OFF_WAT_LEN) (local.get $wat_len))
    (local.set $pos (i32.const 0))

    ;; Clear symbol table
    (i32.store (global.get $OFF_WAT_SYM) (i32.const 0))

    ;; Clear state counters (reset module state fully)
    (i32.store (global.get $OFF_TYPE_COUNT) (i32.const 0))
    (i32.store (global.get $OFF_IMPORT_COUNT) (i32.const 0))
    (i32.store (global.get $OFF_FUNCTION_COUNT) (i32.const 0))
    (i32.store (global.get $OFF_CODE_COUNT) (i32.const 0))
    (i32.store (global.get $OFF_EXPORT_COUNT) (i32.const 0))
    (i32.store (global.get $OFF_GLOBAL_COUNT) (i32.const 0))
    (i32.store (global.get $OFF_TABLE_HAS) (i32.const 0))
    (i32.store (global.get $OFF_MEM_MIN) (i32.const 0))
    (i32.store (global.get $OFF_START_FUNC) (i32.const -1))
    (i32.store (global.get $OFF_ELEM_COUNT) (i32.const 0))
    (i32.store (global.get $OFF_DATA_COUNT) (i32.const 0))
    (i32.store (global.get $OFF_DECODED_COUNT) (i32.const 0))
    (i32.store8 (global.get $OFF_EXEC_MOD_VALID) (i32.const 0))

    ;; Reset names pointer
    (i32.store (global.get $OFF_NAMES_PTR) (global.get $OFF_NAMES_BUF))

    (i32.store (global.get $OFF_WAT_DBG) (i32.const 2))

    ;; Skip whitespace
    (if (call $wat_skip_ws (local.get $pos)) (then (i32.store (global.get $OFF_WAT_DBG) (i32.const 10)) (return (global.get $ERR_PARSE))))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))

    (i32.store (global.get $OFF_WAT_DBG) (i32.const 3))

    ;; Expect "("
    (i32.store (global.get $OFF_WAT_TMP0) (i32.load (global.get $OFF_WAT_PTR)))
    (i32.store (global.get $OFF_WAT_TMP1) (local.get $pos))
    (i32.store (global.get $OFF_WAT_TMP1) (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos)))
    (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
    (i32.store8 (global.get $OFF_WAT_DBG) (local.get $b))
    (i32.store8 (global.get $OFF_WAT_TMP0) (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
    (if (i32.ne (local.get $b) (i32.const 0x28))
      (then (i32.store (global.get $OFF_WAT_DBG) (i32.const 0x1100)) (return (global.get $ERR_PARSE)))
    )
    (local.set $pos (i32.add (local.get $pos) (i32.const 1)))

    (i32.store (global.get $OFF_WAT_DBG) (i32.const 4))

    ;; Expect "module"
    (if (call $wat_skip_ws (local.get $pos)) (then (i32.store (global.get $OFF_WAT_DBG) (i32.const 12)) (return (global.get $ERR_PARSE))))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))

    (i32.store (global.get $OFF_WAT_DBG) (i32.const 5))

    (if (call $wat_read_kw (local.get $pos)) (then (i32.store (global.get $OFF_WAT_DBG) (i32.const 13)) (return (global.get $ERR_PARSE))))

    (i32.store (global.get $OFF_WAT_DBG) (i32.const 6))

    (local.set $kw_off (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $kw_len (i32.load (global.get $OFF_SCRATCH1)))
    (if (i32.ne (local.get $kw_len) (i32.const 6))
      (then (i32.store (global.get $OFF_WAT_DBG) (i32.const 14)) (return (global.get $ERR_PARSE)))
    )

    (i32.store (global.get $OFF_WAT_DBG) (i32.const 7))

    (if (i32.ne (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $kw_off))) (i32.const 0x6D))
      (then (i32.store (global.get $OFF_WAT_DBG) (i32.const 15)) (return (global.get $ERR_PARSE)))  ;; 'm'
    )

    (i32.store (global.get $OFF_WAT_DBG) (i32.const 8))

    (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 5)
          (i32.const 0x6F) (i32.const 0x64) (i32.const 0x75) (i32.const 0x6C)
          (i32.const 0x65) (i32.const 0) (i32.const 0) (i32.const 0)
          (i32.const 0) (i32.const 0))
      (then (i32.store (global.get $OFF_WAT_DBG) (i32.const 16)) (return (global.get $ERR_PARSE)))
    )

    (i32.store (global.get $OFF_WAT_DBG) (i32.const 9))

    (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))

    (i32.store (global.get $OFF_WAT_DBG) (i32.const 20))

    ;; ═══ Pass 1: Scan all declarations, register names, count entities ═══
    (block $pass1_done
      (loop $pass1
        (if (call $wat_skip_ws (local.get $pos)) (then (i32.store (global.get $OFF_WAT_DBG) (i32.const 21)) (return (global.get $ERR_PARSE))))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
        ;; Closing paren = end of module
        (if (i32.eq (local.get $b) (i32.const 0x29))  ;; ')'
          (then
            (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
            (br $pass1_done)
          )
        )

        ;; Must be "("
        (if (i32.ne (local.get $b) (i32.const 0x28))  ;; '('
          (then (i32.store (global.get $OFF_WAT_DBG) (i32.const 22)) (return (global.get $ERR_PARSE)))
        )
        (local.set $pos (i32.add (local.get $pos) (i32.const 1)))

        ;; Read keyword
        (if (call $wat_skip_ws (local.get $pos)) (then (i32.store (global.get $OFF_WAT_DBG) (i32.const 23)) (return (global.get $ERR_PARSE))))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
        (if (call $wat_read_kw (local.get $pos)) (then (i32.store (global.get $OFF_WAT_DBG) (i32.const 24)) (return (global.get $ERR_PARSE))))
        (local.set $kw_off (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $kw_len (i32.load (global.get $OFF_SCRATCH1)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
        (local.set $b0 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $kw_off))))

        ;; ── Count declarations by keyword ──
        (block $kw_matched
          (block $not_type
            (if (i32.ne (local.get $kw_len) (i32.const 4)) (then (br $not_type)))
            (if (i32.ne (local.get $b0) (i32.const 0x74)) (then (br $not_type)))  ;; 't'
            (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 3)
                  (i32.const 0x79) (i32.const 0x70) (i32.const 0x65) (i32.const 0)
                  (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 0)
                  (i32.const 0) (i32.const 0))
              (then (br $not_type))
            )
            (i32.store (global.get $OFF_TYPE_COUNT) (i32.add (i32.load (global.get $OFF_TYPE_COUNT)) (i32.const 1)))
            (br $kw_matched)
          )
          (block $not_func
            (if (i32.ne (local.get $kw_len) (i32.const 4)) (then (br $not_func)))
            (if (i32.ne (local.get $b0) (i32.const 0x66)) (then (br $not_func)))  ;; 'f'
            (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 3)
                  (i32.const 0x75) (i32.const 0x6e) (i32.const 0x63) (i32.const 0)
                  (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 0)
                  (i32.const 0) (i32.const 0))
              (then (br $not_func))
            )
            (i32.store (global.get $OFF_FUNCTION_COUNT) (i32.add (i32.load (global.get $OFF_FUNCTION_COUNT)) (i32.const 1)))
            (i32.store (global.get $OFF_CODE_COUNT) (i32.add (i32.load (global.get $OFF_CODE_COUNT)) (i32.const 1)))
            (br $kw_matched)
          )
          (block $not_export
            (if (i32.ne (local.get $kw_len) (i32.const 6)) (then (br $not_export)))
            (if (i32.ne (local.get $b0) (i32.const 0x65)) (then (br $not_export)))  ;; 'e'
            (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 5)
                  (i32.const 0x78) (i32.const 0x70) (i32.const 0x6f) (i32.const 0x72)  ;; xpor
                  (i32.const 0x74) (i32.const 0) (i32.const 0) (i32.const 0)
                  (i32.const 0) (i32.const 0))
              (then (br $not_export))
            )
            (i32.store (global.get $OFF_EXPORT_COUNT) (i32.add (i32.load (global.get $OFF_EXPORT_COUNT)) (i32.const 1)))
            (br $kw_matched)
          )
          (block $not_memory
            (if (i32.ne (local.get $kw_len) (i32.const 6)) (then (br $not_memory)))
            (if (i32.ne (local.get $b0) (i32.const 0x6d)) (then (br $not_memory)))  ;; 'm'
            (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 5)
                  (i32.const 0x65) (i32.const 0x6d) (i32.const 0x6f) (i32.const 0x72)  ;; emor
                  (i32.const 0x79) (i32.const 0) (i32.const 0) (i32.const 0)
                  (i32.const 0) (i32.const 0))
              (then (br $not_memory))
            )
            (i32.store (global.get $OFF_MEM_MIN) (i32.const 1))
            (br $kw_matched)
          )
          (block $not_start
            (if (i32.ne (local.get $kw_len) (i32.const 5)) (then (br $not_start)))
            (if (i32.ne (local.get $b0) (i32.const 0x73)) (then (br $not_start)))  ;; 's'
            (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 4)
                  (i32.const 0x74) (i32.const 0x61) (i32.const 0x72) (i32.const 0x74)  ;; tart
                  (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 0)
                  (i32.const 0) (i32.const 0))
              (then (br $not_start))
            )
            (br $kw_matched)
          )
          (block $not_data
            (if (i32.ne (local.get $kw_len) (i32.const 4)) (then (br $not_data)))
            (if (i32.ne (local.get $b0) (i32.const 0x64)) (then (br $not_data)))  ;; 'd'
            (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 3)
                  (i32.const 0x61) (i32.const 0x74) (i32.const 0x61) (i32.const 0)
                  (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 0)
                  (i32.const 0) (i32.const 0))
              (then (br $not_data))
            )
            (i32.store (global.get $OFF_DATA_COUNT) (i32.add (i32.load (global.get $OFF_DATA_COUNT)) (i32.const 1)))
            (br $kw_matched)
          )
        )

        ;; Find matching closing paren and skip entire declaration
        (local.set $open_parens (i32.const 1))
        (block $skip_decl
          (loop $skip_lp
            (if (i32.ge_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))
                          (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.load (global.get $OFF_WAT_LEN))))
              (then (i32.store (global.get $OFF_WAT_DBG) (i32.const 25)) (return (global.get $ERR_PARSE)))
            )
            (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
            (if (i32.eq (local.get $b) (i32.const 0x28))  ;; '('
              (then (local.set $open_parens (i32.add (local.get $open_parens) (i32.const 1))))
            )
            (if (i32.eq (local.get $b) (i32.const 0x29))  ;; ')'
              (then
                (local.set $open_parens (i32.sub (local.get $open_parens) (i32.const 1)))
                (if (i32.eqz (local.get $open_parens))
                  (then
                    (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
                    (br $skip_decl)
                  )
                )
              )
            )
            (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
            (br $skip_lp)
          )
        )
        (br $pass1)
      )
    )

    (i32.store (global.get $OFF_WAT_DBG) (i32.const 30))

    ;; ═══ Pass 2: Parse declarations into state buffers ═══
    (local.set $pos (i32.const 0))
    (if (call $wat_skip_ws (local.get $pos)) (then (i32.store (global.get $OFF_WAT_DBG) (i32.const 31)) (return (global.get $ERR_PARSE))))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))

    ;; skip "(module"
    (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
    (if (i32.ne (local.get $b) (i32.const 0x28)) (then (i32.store (global.get $OFF_WAT_DBG) (i32.const 32)) (return (global.get $ERR_PARSE))))
    (local.set $pos (i32.add (local.get $pos) (i32.const 1)))

    (if (call $wat_skip_ws (local.get $pos)) (then (i32.store (global.get $OFF_WAT_DBG) (i32.const 33)) (return (global.get $ERR_PARSE))))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
    (if (call $wat_read_kw (local.get $pos)) (then (i32.store (global.get $OFF_WAT_DBG) (i32.const 34)) (return (global.get $ERR_PARSE))))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))

    ;; Reset decl counters for pass 2
    (i32.store (global.get $OFF_TYPE_COUNT) (i32.const 0))
    (i32.store (global.get $OFF_FUNCTION_COUNT) (i32.const 0))
    (i32.store (global.get $OFF_CODE_COUNT) (i32.const 0))
    (i32.store (global.get $OFF_EXPORT_COUNT) (i32.const 0))

    (block $pass2_done
      (loop $pass2
        (if (call $wat_skip_ws (local.get $pos)) (then (i32.store (global.get $OFF_WAT_DBG) (i32.const 35)) (return (global.get $ERR_PARSE))))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
        (if (i32.eq (local.get $b) (i32.const 0x29))  ;; ')'
          (then (br $pass2_done))
        )
        ;; Must be "("
        (if (i32.ne (local.get $b) (i32.const 0x28))  ;; '('
          (then (i32.store (global.get $OFF_WAT_DBG) (i32.const 36)) (return (global.get $ERR_PARSE)))
        )
        (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
        ;; Read keyword
        (if (call $wat_skip_ws (local.get $pos)) (then (i32.store (global.get $OFF_WAT_DBG) (i32.const 37)) (return (global.get $ERR_PARSE))))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
        (if (call $wat_read_kw (local.get $pos)) (then (i32.store (global.get $OFF_WAT_DBG) (i32.const 38)) (return (global.get $ERR_PARSE))))
        (local.set $kw_off (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $kw_len (i32.load (global.get $OFF_SCRATCH1)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
        (local.set $b0 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $kw_off))))

        (block $kw2_fallthrough
          (if (i32.eq (local.get $kw_len) (i32.const 4))
            (then
              (if (i32.eq (local.get $b0) (i32.const 0x74))  ;; 't'
                (then
                  (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 3)
                        (i32.const 0x79) (i32.const 0x70) (i32.const 0x65) (i32.const 0)
                        (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 0)
                        (i32.const 0) (i32.const 0))
                    (then (br $kw2_fallthrough))
                  )
                  (if (call $wat_parse_type_decl (local.get $pos))
                    (then (return (global.get $ERR_PARSE)))
                  )
                  (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
                  (br $pass2)
                )
              )
              (if (i32.eq (local.get $b0) (i32.const 0x66))  ;; 'f'
                (then
                  (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 3)
                        (i32.const 0x75) (i32.const 0x6e) (i32.const 0x63) (i32.const 0)
                        (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 0)
                        (i32.const 0) (i32.const 0))
                    (then (br $kw2_fallthrough))
                  )
                  ;; Parse function decl inline
                  (if (call $wat_parse_func_decl (local.get $pos))
                    (then (return (global.get $ERR_PARSE)))
                  )
                  (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
                  (br $pass2)
                )
              )
            )
          )
        )

        ;; Unknown declaration — skip it
        (local.set $open_parens (i32.const 1))
        (block $skip2
          (loop $skip2_lp
            (if (i32.ge_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))
                          (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.load (global.get $OFF_WAT_LEN))))
              (then (return (global.get $ERR_PARSE)))
            )
            (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
            (if (i32.eq (local.get $b) (i32.const 0x28)) (then (local.set $open_parens (i32.add (local.get $open_parens) (i32.const 1)))))
            (if (i32.eq (local.get $b) (i32.const 0x29))
              (then
                (local.set $open_parens (i32.sub (local.get $open_parens) (i32.const 1)))
                (if (i32.eqz (local.get $open_parens))
                  (then (local.set $pos (i32.add (local.get $pos) (i32.const 1))) (br $skip2))
                )
              )
            )
            (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
            (br $skip2_lp)
          )
        )
        (br $pass2)
      )
    )

    ;; Restore wasm ptr/len
    (i32.store (global.get $OFF_WASM_PTR) (i32.load (global.get $OFF_WAT_SAV_PTR)))
    (i32.store (global.get $OFF_WASM_LEN) (i32.load (global.get $OFF_WAT_SAV_LEN)))

    (i32.store (global.get $OFF_WAT_DBG) (i32.const 99))
    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Exported WAT loader: load_wat(wat_ptr, wat_len) -> error_code
  ;; ═════════════════════════════════════════════════════════════════════

  (func (export "load_wat") (param $wat_ptr i32) (param $wat_len i32) (result i32)
    (return (call $wat_parse_module (local.get $wat_ptr) (local.get $wat_len)))
  )
