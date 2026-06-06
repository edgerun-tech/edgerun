  ;; ═══════════════════════════════════════════════════════════════════════════
  ;; WAT Parse Pipeline Stage — SIMD-accelerated WAT→WASM compiler
  ;; Stage type: batch (state=0)
  ;; Input:  WAT source bytes
  ;; Output: 4-byte status (0 = OK, err < 0)
  ;; ═══════════════════════════════════════════════════════════════════════════

  (func $wat_skip_ws (param $pos i32) (result i32)
    (local $p i32) (local $end i32) (local $b i32)
    (local $v v128) (local $ws v128)
    (local.set $p (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos)))
    (local.set $end (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.load (global.get $OFF_WAT_LEN))))
    (block $scalar
      (loop $simd
        (if (i32.ge_u (i32.add (local.get $p) (i32.const 16)) (local.get $end))
          (then (br $scalar)))
        (local.set $v (v128.load (local.get $p)))
        (local.set $ws
          (v128.or (i8x16.eq (local.get $v) (i8x16.splat (i32.const 0x20)))
            (v128.or (i8x16.eq (local.get $v) (i8x16.splat (i32.const 0x09)))
              (v128.or (i8x16.eq (local.get $v) (i8x16.splat (i32.const 0x0A)))
                (i8x16.eq (local.get $v) (i8x16.splat (i32.const 0x0D)))))))
        (if (i32.eqz (v128.any_true (v128.not (local.get $ws))))
          (then
            (local.set $p (i32.add (local.get $p) (i32.const 16)))
            (br $simd)))
      )
    )
    (block $done
      (loop $lp
        (if (i32.ge_u (local.get $p) (local.get $end)) (then (br $done)))
        (local.set $b (i32.load8_u (local.get $p)))
        (if (i32.eq (local.get $b) (i32.const 0x20)) (then (local.set $p (i32.add (local.get $p) (i32.const 1))) (br $lp)))
        (if (i32.eq (local.get $b) (i32.const 0x09)) (then (local.set $p (i32.add (local.get $p) (i32.const 1))) (br $lp)))
        (if (i32.eq (local.get $b) (i32.const 0x0A)) (then (local.set $p (i32.add (local.get $p) (i32.const 1))) (br $lp)))
        (if (i32.eq (local.get $b) (i32.const 0x0D)) (then (local.set $p (i32.add (local.get $p) (i32.const 1))) (br $lp)))
        (if (i32.eq (local.get $b) (i32.const 0x3B))
          (then
            (local.set $p (i32.add (local.get $p) (i32.const 1)))
            (if (i32.lt_u (local.get $p) (local.get $end))
              (then
                (local.set $b (i32.load8_u (local.get $p)))
                (local.set $p (i32.add (local.get $p) (i32.const 1)))
                (if (i32.eq (local.get $b) (i32.const 0x3B))
                  (then
                    (block $eol
                      (loop $eol_lp
                        (if (i32.ge_u (local.get $p) (local.get $end)) (then (br $eol)))
                        (local.set $b (i32.load8_u (local.get $p)))
                        (local.set $p (i32.add (local.get $p) (i32.const 1)))
                        (if (i32.or (i32.eq (local.get $b) (i32.const 0x0A)) (i32.eq (local.get $b) (i32.const 0x0D)))
                          (then (br $eol))
                          (else (br $eol_lp)))
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
        (if (i32.eq (local.get $b) (i32.const 0x28))
          (then
            (local.set $p (i32.add (local.get $p) (i32.const 1)))
            (if (i32.lt_u (local.get $p) (local.get $end))
              (then
                (local.set $b (i32.load8_u (local.get $p)))
                (if (i32.eq (local.get $b) (i32.const 0x3B))
                  (then
                    (local.set $p (i32.add (local.get $p) (i32.const 1)))
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
    (return (i32.const 0))
  )

  ;; Read keyword (alphanumeric + _ + . + $)
  ;; Output: scratch0=offset, scratch1=len, scratch2=new_pos
  (func $wat_read_kw (param $pos i32) (result i32)
    (local $p i32) (local $end i32) (local $start i32) (local $b i32)
    (local.set $p (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos)))
    (local.set $end (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.load (global.get $OFF_WAT_LEN))))
    (if (i32.ge_u (local.get $p) (local.get $end)) (then (return (global.get $ERR_PARSE))))
    (local.set $start (local.get $p))
    (block $done
      (loop $lp
        (if (i32.ge_u (local.get $p) (local.get $end)) (then (br $done)))
        (local.set $b (i32.load8_u (local.get $p)))
        (if (i32.and (i32.ge_u (local.get $b) (i32.const 0x30)) (i32.le_u (local.get $b) (i32.const 0x39)))
          (then (local.set $p (i32.add (local.get $p) (i32.const 1))) (br $lp)))
        (if (i32.and (i32.ge_u (local.get $b) (i32.const 0x41)) (i32.le_u (local.get $b) (i32.const 0x5A)))
          (then (local.set $p (i32.add (local.get $p) (i32.const 1))) (br $lp)))
        (if (i32.and (i32.ge_u (local.get $b) (i32.const 0x61)) (i32.le_u (local.get $b) (i32.const 0x7A)))
          (then (local.set $p (i32.add (local.get $p) (i32.const 1))) (br $lp)))
        (if (i32.or (i32.eq (local.get $b) (i32.const 0x2E)) (i32.or (i32.eq (local.get $b) (i32.const 0x5F)) (i32.eq (local.get $b) (i32.const 0x24))))
          (then (local.set $p (i32.add (local.get $p) (i32.const 1))) (br $lp)))
        (br $done)
      )
    )
    (if (i32.eq (local.get $p) (local.get $start)) (then (return (global.get $ERR_PARSE))))
    (i32.store (global.get $OFF_SCRATCH0) (i32.sub (local.get $start) (i32.load (global.get $OFF_WAT_PTR))))
    (i32.store (global.get $OFF_SCRATCH1) (i32.sub (local.get $p) (local.get $start)))
    (i32.store (global.get $OFF_SCRATCH2) (i32.sub (local.get $p) (i32.load (global.get $OFF_WAT_PTR))))
    (return (i32.const 0))
  )

  ;; Read unsigned decimal integer
  ;; Output: scratch0=value, scratch1=new_pos
  (func $wat_read_uint (param $pos i32) (result i32)
    (local $p i32) (local $end i32) (local $val i32) (local $b i32)
    (local.set $p (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos)))
    (local.set $end (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.load (global.get $OFF_WAT_LEN))))
    (if (i32.ge_u (local.get $p) (local.get $end)) (then (return (global.get $ERR_PARSE))))
    (block $done
      (loop $lp
        (if (i32.ge_u (local.get $p) (local.get $end)) (then (br $done)))
        (local.set $b (i32.load8_u (local.get $p)))
        (if (i32.and (i32.ge_u (local.get $b) (i32.const 0x30)) (i32.le_u (local.get $b) (i32.const 0x39)))
          (then
            (local.set $val (i32.add (i32.mul (local.get $val) (i32.const 10)) (i32.sub (local.get $b) (i32.const 48))))
            (local.set $p (i32.add (local.get $p) (i32.const 1)))
            (br $lp))
          (else (br $done))
        )
      )
    )
    (i32.store (global.get $OFF_SCRATCH0) (local.get $val))
    (i32.store (global.get $OFF_SCRATCH1) (i32.sub (local.get $p) (i32.load (global.get $OFF_WAT_PTR))))
    (return (i32.const 0))
  )

  ;; Read signed decimal integer
  ;; Output: scratch0=value, scratch1=new_pos
  (func $wat_read_sint (param $pos i32) (result i32)
    (local $p i32) (local $end i32) (local $val i32) (local $b i32) (local $neg i32)
    (local.set $p (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos)))
    (local.set $end (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.load (global.get $OFF_WAT_LEN))))
    (if (i32.ge_u (local.get $p) (local.get $end)) (then (return (global.get $ERR_PARSE))))
    (local.set $b (i32.load8_u (local.get $p)))
    (if (i32.eq (local.get $b) (i32.const 0x2D))
      (then (local.set $neg (i32.const 1)) (local.set $p (i32.add (local.get $p) (i32.const 1))))
      (else (local.set $neg (i32.const 0)))
    )
    (if (i32.ge_u (local.get $p) (local.get $end)) (then (return (global.get $ERR_PARSE))))
    (block $done
      (loop $lp
        (if (i32.ge_u (local.get $p) (local.get $end)) (then (br $done)))
        (local.set $b (i32.load8_u (local.get $p)))
        (if (i32.and (i32.ge_u (local.get $b) (i32.const 0x30)) (i32.le_u (local.get $b) (i32.const 0x39)))
          (then
            (local.set $val (i32.add (i32.mul (local.get $val) (i32.const 10)) (i32.sub (local.get $b) (i32.const 48))))
            (local.set $p (i32.add (local.get $p) (i32.const 1)))
            (br $lp))
          (else (br $done))
        )
      )
    )
    (if (local.get $neg) (then (local.set $val (i32.sub (i32.const 0) (local.get $val)))))
    (i32.store (global.get $OFF_SCRATCH0) (local.get $val))
    (i32.store (global.get $OFF_SCRATCH1) (i32.sub (local.get $p) (i32.load (global.get $OFF_WAT_PTR))))
    (return (i32.const 0))
  )

  ;; Read quoted string
  ;; Output: scratch0=str_offset, scratch1=str_len, scratch2=new_pos
  (func $wat_read_string (param $pos i32) (result i32)
    (local $p i32) (local $end i32) (local $start i32) (local $b i32)
    (local.set $p (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos)))
    (local.set $end (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.load (global.get $OFF_WAT_LEN))))
    (if (i32.ge_u (local.get $p) (local.get $end)) (then (return (global.get $ERR_PARSE))))
    (local.set $b (i32.load8_u (local.get $p)))
    (if (i32.ne (local.get $b) (i32.const 0x22)) (then (return (global.get $ERR_PARSE))))  ;; "
    (local.set $p (i32.add (local.get $p) (i32.const 1)))
    (local.set $start (local.get $p))
    (block $done
      (loop $lp
        (if (i32.ge_u (local.get $p) (local.get $end)) (then (return (global.get $ERR_PARSE))))
        (local.set $b (i32.load8_u (local.get $p)))
        (if (i32.eq (local.get $b) (i32.const 0x5C))  ;; backslash
          (then
            (local.set $p (i32.add (local.get $p) (i32.const 1)))
            (if (i32.ge_u (local.get $p) (local.get $end)) (then (return (global.get $ERR_PARSE))))
            (local.set $p (i32.add (local.get $p) (i32.const 1)))
            (br $lp)))
        (if (i32.eq (local.get $b) (i32.const 0x22))  ;; "
          (then
            (i32.store (global.get $OFF_SCRATCH0) (i32.sub (local.get $start) (i32.load (global.get $OFF_WAT_PTR))))
            (i32.store (global.get $OFF_SCRATCH1) (i32.sub (local.get $p) (local.get $start)))
            (i32.store (global.get $OFF_SCRATCH2) (i32.add (i32.sub (local.get $p) (i32.load (global.get $OFF_WAT_PTR))) (i32.const 1)))
            (return (i32.const 0))))
        (local.set $p (i32.add (local.get $p) (i32.const 1)))
        (br $lp)
      )
    )
    (return (global.get $ERR_PARSE))
  )

  ;; Compare rest of keyword bytes
  (func $wat_kw_match_rest (param $kw_off i32) (param $start i32) (param $count i32)
    (param $p0 i32) (param $p1 i32) (param $p2 i32) (param $p3 i32)
    (param $p4 i32) (param $p5 i32) (param $p6 i32) (param $p7 i32)
    (param $p8 i32) (param $p9 i32) (result i32)
    (local $base i32) (local $i i32) (local $b i32)
    (local.set $base (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $kw_off)))
    (local.set $i (i32.const 0))
    (block $done
      (loop $lp
        (if (i32.ge_u (local.get $i) (local.get $count)) (then (br $done)))
        (local.set $b (i32.load8_u (i32.add (local.get $base) (i32.add (local.get $start) (local.get $i)))))
        (if (i32.eq (local.get $i) (i32.const 0))
          (then (if (i32.ne (local.get $b) (local.get $p0)) (then (return (i32.const 1))))))
        (if (i32.eq (local.get $i) (i32.const 1))
          (then (if (i32.ne (local.get $b) (local.get $p1)) (then (return (i32.const 1))))))
        (if (i32.eq (local.get $i) (i32.const 2))
          (then (if (i32.ne (local.get $b) (local.get $p2)) (then (return (i32.const 1))))))
        (if (i32.eq (local.get $i) (i32.const 3))
          (then (if (i32.ne (local.get $b) (local.get $p3)) (then (return (i32.const 1))))))
        (if (i32.eq (local.get $i) (i32.const 4))
          (then (if (i32.ne (local.get $b) (local.get $p4)) (then (return (i32.const 1))))))
        (if (i32.eq (local.get $i) (i32.const 5))
          (then (if (i32.ne (local.get $b) (local.get $p5)) (then (return (i32.const 1))))))
        (if (i32.eq (local.get $i) (i32.const 6))
          (then (if (i32.ne (local.get $b) (local.get $p6)) (then (return (i32.const 1))))))
        (if (i32.eq (local.get $i) (i32.const 7))
          (then (if (i32.ne (local.get $b) (local.get $p7)) (then (return (i32.const 1))))))
        (if (i32.eq (local.get $i) (i32.const 8))
          (then (if (i32.ne (local.get $b) (local.get $p8)) (then (return (i32.const 1))))))
        (if (i32.eq (local.get $i) (i32.const 9))
          (then (if (i32.ne (local.get $b) (local.get $p9)) (then (return (i32.const 1))))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $lp)
      )
    )
    (return (i32.const 0))
  )

  ;; Value type helper: convert type keyword → WASM valtype byte
  ;; Input: scratch0=kw_off, scratch1=kw_len
  ;; Output: scratch0 = valtype byte, error if unknown
  (func $wat_valtype (param $kw_off i32) (param $kw_len i32) (result i32)
    (local $p i32) (local $b0 i32) (local $b1 i32) (local $b2 i32) (local $b3 i32)
    (local.set $p (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $kw_off)))
    (local.set $b0 (i32.load8_u (local.get $p)))
    (if (i32.gt_u (local.get $kw_len) (i32.const 1))
      (then (local.set $b1 (i32.load8_u (i32.add (local.get $p) (i32.const 1))))))
    (if (i32.gt_u (local.get $kw_len) (i32.const 2))
      (then (local.set $b2 (i32.load8_u (i32.add (local.get $p) (i32.const 2))))))
    (block $done
      (if (i32.eq (local.get $kw_len) (i32.const 3))
        (then
          (if (i32.and (i32.eq (local.get $b0) (i32.const 0x69)) (i32.and (i32.eq (local.get $b1) (i32.const 0x33)) (i32.eq (local.get $b2) (i32.const 0x32))))
            (then (i32.store (global.get $OFF_SCRATCH0) (i32.const 0x7F)) (return (i32.const 0))))
          (if (i32.and (i32.eq (local.get $b0) (i32.const 0x69)) (i32.and (i32.eq (local.get $b1) (i32.const 0x36)) (i32.eq (local.get $b2) (i32.const 0x34))))
            (then (i32.store (global.get $OFF_SCRATCH0) (i32.const 0x7E)) (return (i32.const 0))))
          (if (i32.and (i32.eq (local.get $b0) (i32.const 0x66)) (i32.and (i32.eq (local.get $b1) (i32.const 0x33)) (i32.eq (local.get $b2) (i32.const 0x32))))
            (then (i32.store (global.get $OFF_SCRATCH0) (i32.const 0x7D)) (return (i32.const 0))))
          (if (i32.and (i32.eq (local.get $b0) (i32.const 0x66)) (i32.and (i32.eq (local.get $b1) (i32.const 0x36)) (i32.eq (local.get $b2) (i32.const 0x34))))
            (then (i32.store (global.get $OFF_SCRATCH0) (i32.const 0x7C)) (return (i32.const 0))))
        )
      )
      (if (i32.eq (local.get $kw_len) (i32.const 4))
        (then
          (local.set $b3 (i32.load8_u (i32.add (local.get $p) (i32.const 3))))
          (if (i32.and (i32.eq (local.get $b0) (i32.const 0x76)) (i32.and (i32.eq (local.get $b1) (i32.const 0x31)) (i32.and (i32.eq (local.get $b2) (i32.const 0x32)) (i32.eq (local.get $b3) (i32.const 0x38)))))
            (then (i32.store (global.get $OFF_SCRATCH0) (i32.const 0x7B)) (return (i32.const 0))))
        )
      )
    )
    (return (global.get $ERR_PARSE))
  )

  ;; Emit one byte to body buffer
  ;; Input: off, b; Output: scratch0=new_off or error
  (func $wat_emit_byte (param $off i32) (param $b i32) (result i32)
    (if (i32.ge_u (local.get $off) (global.get $WAT_BODY_SZ))
      (then (return (global.get $ERR_NO_MEM))))
    (i32.store8 (i32.add (global.get $OFF_WAT_BODY) (local.get $off)) (local.get $b))
    (i32.store (global.get $OFF_SCRATCH0) (i32.add (local.get $off) (i32.const 1)))
    (return (i32.const 0))
  )

  ;; Emit LEB128 unsigned
  (func $wat_emit_leb_u32 (param $off i32) (param $val i32) (result i32)
    (local $b i32)
    (block $done
      (loop $lp
        (local.set $b (i32.and (local.get $val) (i32.const 0x7F)))
        (local.set $val (i32.shr_u (local.get $val) (i32.const 7)))
        (if (i32.ne (local.get $val) (i32.const 0))
          (then (local.set $b (i32.or (local.get $b) (i32.const 0x80)))))
        (if (call $wat_emit_byte (local.get $off) (local.get $b)) (then (return (global.get $ERR_NO_MEM))))
        (local.set $off (i32.load (global.get $OFF_SCRATCH0)))
        (if (i32.eqz (local.get $val)) (then (br $done)))
        (br $lp)
      )
    )
    (i32.store (global.get $OFF_SCRATCH0) (local.get $off))
    (return (i32.const 0))
  )

  ;; Emit LEB128 signed
  (func $wat_emit_leb_i32 (param $off i32) (param $val i32) (result i32)
    (local $b i32) (local $more i32) (local $sign i32)
    (local.set $sign (i32.and (local.get $val) (i32.const 0x40)))
    (block $done
      (loop $lp
        (local.set $b (i32.and (local.get $val) (i32.const 0x7F)))
        (local.set $val (i32.shr_s (local.get $val) (i32.const 7)))
        (block $check_more
          (if (i32.eq (local.get $val) (i32.const 0))
            (then
              (if (i32.eqz (i32.and (local.get $b) (i32.const 0x40))) (then (br $check_more)))))
          (if (i32.eq (local.get $val) (i32.const -1))
            (then
              (if (i32.and (local.get $b) (i32.const 0x40)) (then (br $check_more)))))
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
    (return (i32.const 0))
  )

  ;; ─────────────────────────────────────────────────────────────
  ;; Body opcode parser
  ;; ─────────────────────────────────────────────────────────────

  (func $wat_parse_body (param $pos i32) (param $body_off i32) (result i32)
    (local $err i32) (local $b i32) (local $kw_off i32)
    (local $kw_len i32) (local $b0 i32) (local $b1 i32) (local $b2 i32) (local $b3 i32)
    (local $imm i32)

    (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))

    (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
    (if (i32.eq (local.get $b) (i32.const 0x29))  ;; )
      (then
        (if (call $wat_emit_byte (local.get $body_off) (i32.const 0x0B))
          (then (return (global.get $ERR_NO_MEM))))
        (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
        (i32.store (global.get $OFF_WASM_PTR) (global.get $OFF_WAT_BODY))
        (i32.store (global.get $OFF_WASM_LEN) (local.get $body_off))
        (i32.store (global.get $OFF_SCRATCH0) (local.get $body_off))
        (i32.store (global.get $OFF_SCRATCH1) (i32.const 0))
        (return (i32.const -1))  ;; DONE
      )
    )

    (if (i32.ge_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))
                  (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.load (global.get $OFF_WAT_LEN))))
      (then (return (global.get $ERR_PARSE))))

    (local.set $err (call $wat_read_kw (local.get $pos)))
    (if (local.get $err) (then (return (global.get $ERR_PARSE))))
    (local.set $kw_off (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $kw_len (i32.load (global.get $OFF_SCRATCH1)))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))

    (local.set $b0 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $kw_off))))

    (block $not_unreachable
      (if (i32.ne (local.get $kw_len) (i32.const 11)) (then (br $not_unreachable)))
      (if (i32.ne (local.get $b0) (i32.const 0x75)) (then (br $not_unreachable)))
      (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 10)
            (i32.const 0x6e) (i32.const 0x72) (i32.const 0x65) (i32.const 0x61)
            (i32.const 0x63) (i32.const 0x68) (i32.const 0x61) (i32.const 0x62)
            (i32.const 0x6c) (i32.const 0x65))
        (then (br $not_unreachable)))
      (if (call $wat_emit_byte (local.get $body_off) (i32.const 0x00)) (then (return (global.get $ERR_NO_MEM))))
      (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
      (i32.store (global.get $OFF_SCRATCH0) (local.get $body_off))
      (i32.store (global.get $OFF_SCRATCH1) (local.get $pos))
      (return (i32.const 0))
    )
    (block $not_nop
      (if (i32.ne (local.get $kw_len) (i32.const 3)) (then (br $not_nop)))
      (if (i32.ne (local.get $b0) (i32.const 0x6e)) (then (br $not_nop)))
      (local.set $b1 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.add (local.get $kw_off) (i32.const 1)))))
      (local.set $b2 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.add (local.get $kw_off) (i32.const 2)))))
      (if (i32.ne (local.get $b1) (i32.const 0x6f)) (then (br $not_nop)))
      (if (i32.ne (local.get $b2) (i32.const 0x70)) (then (br $not_nop)))
      (if (call $wat_emit_byte (local.get $body_off) (i32.const 0x01)) (then (return (global.get $ERR_NO_MEM))))
      (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
      (i32.store (global.get $OFF_SCRATCH0) (local.get $body_off))
      (i32.store (global.get $OFF_SCRATCH1) (local.get $pos))
      (return (i32.const 0))
    )
    (block $not_i32add
      (if (i32.ne (local.get $kw_len) (i32.const 7)) (then (br $not_i32add)))
      (if (i32.ne (local.get $b0) (i32.const 0x69)) (then (br $not_i32add)))
      (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 6)
            (i32.const 0x33) (i32.const 0x32) (i32.const 0x2e) (i32.const 0x61)
            (i32.const 0x64) (i32.const 0x64) (i32.const 0) (i32.const 0)
            (i32.const 0) (i32.const 0))
        (then (br $not_i32add)))
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
            (i32.const 0x33) (i32.const 0x32) (i32.const 0x2e) (i32.const 0x73)
            (i32.const 0x75) (i32.const 0x62) (i32.const 0) (i32.const 0)
            (i32.const 0) (i32.const 0))
        (then (br $not_i32sub)))
      (if (call $wat_emit_byte (local.get $body_off) (i32.const 0x6b)) (then (return (global.get $ERR_NO_MEM))))
      (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
      (i32.store (global.get $OFF_SCRATCH0) (local.get $body_off))
      (i32.store (global.get $OFF_SCRATCH1) (local.get $pos))
      (return (i32.const 0))
    )
    (block $not_i32mul
      (if (i32.ne (local.get $kw_len) (i32.const 7)) (then (br $not_i32mul)))
      (if (i32.ne (local.get $b0) (i32.const 0x69)) (then (br $not_i32mul)))
      (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 6)
            (i32.const 0x33) (i32.const 0x32) (i32.const 0x2e) (i32.const 0x6d)
            (i32.const 0x75) (i32.const 0x6c) (i32.const 0) (i32.const 0)
            (i32.const 0) (i32.const 0))
        (then (br $not_i32mul)))
      (if (call $wat_emit_byte (local.get $body_off) (i32.const 0x6c)) (then (return (global.get $ERR_NO_MEM))))
      (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
      (i32.store (global.get $OFF_SCRATCH0) (local.get $body_off))
      (i32.store (global.get $OFF_SCRATCH1) (local.get $pos))
      (return (i32.const 0))
    )
    (block $not_i32div_s
      (if (i32.ne (local.get $kw_len) (i32.const 9)) (then (br $not_i32div_s)))
      (if (i32.ne (local.get $b0) (i32.const 0x69)) (then (br $not_i32div_s)))
      (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 8)
            (i32.const 0x33) (i32.const 0x32) (i32.const 0x2e) (i32.const 0x64)
            (i32.const 0x69) (i32.const 0x76) (i32.const 0x5f) (i32.const 0x73)
            (i32.const 0) (i32.const 0))
        (then (br $not_i32div_s)))
      (if (call $wat_emit_byte (local.get $body_off) (i32.const 0x6d)) (then (return (global.get $ERR_NO_MEM))))
      (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
      (i32.store (global.get $OFF_SCRATCH0) (local.get $body_off))
      (i32.store (global.get $OFF_SCRATCH1) (local.get $pos))
      (return (i32.const 0))
    )
    (block $not_i32const
      (if (i32.ne (local.get $kw_len) (i32.const 9)) (then (br $not_i32const)))
      (if (i32.ne (local.get $b0) (i32.const 0x69)) (then (br $not_i32const)))
      (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 8)
            (i32.const 0x33) (i32.const 0x32) (i32.const 0x2e) (i32.const 0x63)
            (i32.const 0x6f) (i32.const 0x6e) (i32.const 0x73) (i32.const 0x74)
            (i32.const 0) (i32.const 0))
        (then (br $not_i32const)))
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
      (if (i32.ne (local.get $b0) (i32.const 0x6c)) (then (br $not_localget)))
      (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 8)
            (i32.const 0x6f) (i32.const 0x63) (i32.const 0x61) (i32.const 0x6c)
            (i32.const 0x2e) (i32.const 0x67) (i32.const 0x65) (i32.const 0x74)
            (i32.const 0) (i32.const 0))
        (then (br $not_localget)))
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
            (i32.const 0x6f) (i32.const 0x63) (i32.const 0x61) (i32.const 0x6c)
            (i32.const 0x2e) (i32.const 0x73) (i32.const 0x65) (i32.const 0x74)
            (i32.const 0) (i32.const 0))
        (then (br $not_localset)))
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
            (i32.const 0x6f) (i32.const 0x63) (i32.const 0x61) (i32.const 0x6c)
            (i32.const 0x2e) (i32.const 0x74) (i32.const 0x65) (i32.const 0x65)
            (i32.const 0) (i32.const 0))
        (then (br $not_localtee)))
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
      (if (i32.ne (local.get $b0) (i32.const 0x72)) (then (br $not_return)))
      (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 5)
            (i32.const 0x65) (i32.const 0x74) (i32.const 0x75) (i32.const 0x72)
            (i32.const 0x6e) (i32.const 0) (i32.const 0) (i32.const 0)
            (i32.const 0) (i32.const 0))
        (then (br $not_return)))
      (if (call $wat_emit_byte (local.get $body_off) (i32.const 0x0F)) (then (return (global.get $ERR_NO_MEM))))
      (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
      (i32.store (global.get $OFF_SCRATCH0) (local.get $body_off))
      (i32.store (global.get $OFF_SCRATCH1) (local.get $pos))
      (return (i32.const 0))
    )
    (block $not_drop
      (if (i32.ne (local.get $kw_len) (i32.const 4)) (then (br $not_drop)))
      (if (i32.ne (local.get $b0) (i32.const 0x64)) (then (br $not_drop)))
      (local.set $b1 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.add (local.get $kw_off) (i32.const 1)))))
      (local.set $b2 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.add (local.get $kw_off) (i32.const 2)))))
      (local.set $b3 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.add (local.get $kw_off) (i32.const 3)))))
      (if (i32.ne (local.get $b1) (i32.const 0x72)) (then (br $not_drop)))
      (if (i32.ne (local.get $b2) (i32.const 0x6f)) (then (br $not_drop)))
      (if (i32.ne (local.get $b3) (i32.const 0x70)) (then (br $not_drop)))
      (if (call $wat_emit_byte (local.get $body_off) (i32.const 0x1A)) (then (return (global.get $ERR_NO_MEM))))
      (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
      (i32.store (global.get $OFF_SCRATCH0) (local.get $body_off))
      (i32.store (global.get $OFF_SCRATCH1) (local.get $pos))
      (return (i32.const 0))
    )
    (block $not_end
      (if (i32.ne (local.get $kw_len) (i32.const 3)) (then (br $not_end)))
      (if (i32.ne (local.get $b0) (i32.const 0x65)) (then (br $not_end)))
      (local.set $b1 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.add (local.get $kw_off) (i32.const 1)))))
      (local.set $b2 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.add (local.get $kw_off) (i32.const 2)))))
      (if (i32.ne (local.get $b1) (i32.const 0x6e)) (then (br $not_end)))
      (if (i32.ne (local.get $b2) (i32.const 0x64)) (then (br $not_end)))
      (if (call $wat_emit_byte (local.get $body_off) (i32.const 0x0B)) (then (return (global.get $ERR_NO_MEM))))
      (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
      (i32.store (global.get $OFF_SCRATCH0) (local.get $body_off))
      (i32.store (global.get $OFF_SCRATCH1) (local.get $pos))
      (return (i32.const 0))
    )
    (block $not_if
      (if (i32.ne (local.get $kw_len) (i32.const 2)) (then (br $not_if)))
      (if (i32.ne (local.get $b0) (i32.const 0x69)) (then (br $not_if)))
      (local.set $b1 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.add (local.get $kw_off) (i32.const 1)))))
      (if (i32.ne (local.get $b1) (i32.const 0x66)) (then (br $not_if)))
      (if (call $wat_emit_byte (local.get $body_off) (i32.const 0x04)) (then (return (global.get $ERR_NO_MEM))))
      (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
      (i32.store (global.get $OFF_SCRATCH0) (local.get $body_off))
      (i32.store (global.get $OFF_SCRATCH1) (local.get $pos))
      (return (i32.const 0))
    )
    (block $not_else
      (if (i32.ne (local.get $kw_len) (i32.const 4)) (then (br $not_else)))
      (if (i32.ne (local.get $b0) (i32.const 0x65)) (then (br $not_else)))
      (local.set $b1 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.add (local.get $kw_off) (i32.const 1)))))
      (local.set $b2 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.add (local.get $kw_off) (i32.const 2)))))
      (local.set $b3 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.add (local.get $kw_off) (i32.const 3)))))
      (if (i32.ne (local.get $b1) (i32.const 0x6c)) (then (br $not_else)))
      (if (i32.ne (local.get $b2) (i32.const 0x73)) (then (br $not_else)))
      (if (i32.ne (local.get $b3) (i32.const 0x65)) (then (br $not_else)))
      (if (call $wat_emit_byte (local.get $body_off) (i32.const 0x05)) (then (return (global.get $ERR_NO_MEM))))
      (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
      (i32.store (global.get $OFF_SCRATCH0) (local.get $body_off))
      (i32.store (global.get $OFF_SCRATCH1) (local.get $pos))
      (return (i32.const 0))
    )
    (block $not_block
      (if (i32.ne (local.get $kw_len) (i32.const 5)) (then (br $not_block)))
      (if (i32.ne (local.get $b0) (i32.const 0x62)) (then (br $not_block)))
      (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 4)
            (i32.const 0x6c) (i32.const 0x6f) (i32.const 0x63) (i32.const 0x6b)
            (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 0)
            (i32.const 0) (i32.const 0))
        (then (br $not_block)))
      (if (call $wat_emit_byte (local.get $body_off) (i32.const 0x02)) (then (return (global.get $ERR_NO_MEM))))
      (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
      (i32.store (global.get $OFF_SCRATCH0) (local.get $body_off))
      (i32.store (global.get $OFF_SCRATCH1) (local.get $pos))
      (return (i32.const 0))
    )
    (block $not_loop
      (if (i32.ne (local.get $kw_len) (i32.const 4)) (then (br $not_loop)))
      (if (i32.ne (local.get $b0) (i32.const 0x6c)) (then (br $not_loop)))
      (local.set $b1 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.add (local.get $kw_off) (i32.const 1)))))
      (local.set $b2 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.add (local.get $kw_off) (i32.const 2)))))
      (local.set $b3 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.add (local.get $kw_off) (i32.const 3)))))
      (if (i32.ne (local.get $b1) (i32.const 0x6f)) (then (br $not_loop)))
      (if (i32.ne (local.get $b2) (i32.const 0x6f)) (then (br $not_loop)))
      (if (i32.ne (local.get $b3) (i32.const 0x70)) (then (br $not_loop)))
      (if (call $wat_emit_byte (local.get $body_off) (i32.const 0x03)) (then (return (global.get $ERR_NO_MEM))))
      (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
      (i32.store (global.get $OFF_SCRATCH0) (local.get $body_off))
      (i32.store (global.get $OFF_SCRATCH1) (local.get $pos))
      (return (i32.const 0))
    )
    (block $not_br
      (if (i32.ne (local.get $kw_len) (i32.const 2)) (then (br $not_br)))
      (if (i32.ne (local.get $b0) (i32.const 0x62)) (then (br $not_br)))
      (local.set $b1 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.add (local.get $kw_off) (i32.const 1)))))
      (if (i32.ne (local.get $b1) (i32.const 0x72)) (then (br $not_br)))
      (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
      (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
      (if (call $wat_read_uint (local.get $pos)) (then (return (global.get $ERR_PARSE))))
      (local.set $imm (i32.load (global.get $OFF_SCRATCH0)))
      (local.set $pos (i32.load (global.get $OFF_SCRATCH1)))
      (if (call $wat_emit_byte (local.get $body_off) (i32.const 0x0C)) (then (return (global.get $ERR_NO_MEM))))
      (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
      (if (call $wat_emit_leb_u32 (local.get $body_off) (local.get $imm)) (then (return (global.get $ERR_NO_MEM))))
      (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
      (i32.store (global.get $OFF_SCRATCH0) (local.get $body_off))
      (i32.store (global.get $OFF_SCRATCH1) (local.get $pos))
      (return (i32.const 0))
    )
    (block $not_br_if
      (if (i32.ne (local.get $kw_len) (i32.const 5)) (then (br $not_br_if)))
      (if (i32.ne (local.get $b0) (i32.const 0x62)) (then (br $not_br_if)))
      (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 4)
            (i32.const 0x72) (i32.const 0x5f) (i32.const 0x69) (i32.const 0x66)
            (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 0)
            (i32.const 0) (i32.const 0))
        (then (br $not_br_if)))
      (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
      (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
      (if (call $wat_read_uint (local.get $pos)) (then (return (global.get $ERR_PARSE))))
      (local.set $imm (i32.load (global.get $OFF_SCRATCH0)))
      (local.set $pos (i32.load (global.get $OFF_SCRATCH1)))
      (if (call $wat_emit_byte (local.get $body_off) (i32.const 0x0D)) (then (return (global.get $ERR_NO_MEM))))
      (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
      (if (call $wat_emit_leb_u32 (local.get $body_off) (local.get $imm)) (then (return (global.get $ERR_NO_MEM))))
      (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
      (i32.store (global.get $OFF_SCRATCH0) (local.get $body_off))
      (i32.store (global.get $OFF_SCRATCH1) (local.get $pos))
      (return (i32.const 0))
    )
    (block $not_call
      (if (i32.ne (local.get $kw_len) (i32.const 4)) (then (br $not_call)))
      (if (i32.ne (local.get $b0) (i32.const 0x63)) (then (br $not_call)))
      (local.set $b1 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.add (local.get $kw_off) (i32.const 1)))))
      (local.set $b2 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.add (local.get $kw_off) (i32.const 2)))))
      (local.set $b3 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.add (local.get $kw_off) (i32.const 3)))))
      (if (i32.ne (local.get $b1) (i32.const 0x61)) (then (br $not_call)))
      (if (i32.ne (local.get $b2) (i32.const 0x6c)) (then (br $not_call)))
      (if (i32.ne (local.get $b3) (i32.const 0x6c)) (then (br $not_call)))
      (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
      (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
      (if (call $wat_read_uint (local.get $pos)) (then (return (global.get $ERR_PARSE))))
      (local.set $imm (i32.load (global.get $OFF_SCRATCH0)))
      (local.set $pos (i32.load (global.get $OFF_SCRATCH1)))
      (if (call $wat_emit_byte (local.get $body_off) (i32.const 0x10)) (then (return (global.get $ERR_NO_MEM))))
      (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
      (if (call $wat_emit_leb_u32 (local.get $body_off) (local.get $imm)) (then (return (global.get $ERR_NO_MEM))))
      (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
      (i32.store (global.get $OFF_SCRATCH0) (local.get $body_off))
      (i32.store (global.get $OFF_SCRATCH1) (local.get $pos))
      (return (i32.const 0))
    )

    ;; Unknown keyword
    (return (global.get $ERR_PARSE))
  )

  ;; ─────────────────────────────────────────────────────────────
  ;; Parse (result ... ) type annotation
  ;; At pos: we've consumed "(" and "result"
  ;; Output: scratch0=valtype_byte, scratch1=new_pos
  ;; ─────────────────────────────────────────────────────────────

  (func $wat_parse_result (param $pos i32) (result i32)
    (local $b i32) (local $vt i32)
    (local.set $vt (i32.const 0x40))  ;; default empty
    (block $lp
      (loop $cont
        (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
        (if (i32.eq (local.get $b) (i32.const 0x29))  ;; )
          (then (local.set $pos (i32.add (local.get $pos) (i32.const 1))) (br $lp)))
        (if (call $wat_read_kw (local.get $pos)) (then (return (global.get $ERR_PARSE))))
        (if (call $wat_valtype (i32.load (global.get $OFF_SCRATCH0)) (i32.load (global.get $OFF_SCRATCH1)))
          (then (return (global.get $ERR_PARSE))))
        (local.set $vt (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
        (br $lp)
      )
    )
    (i32.store (global.get $OFF_SCRATCH0) (local.get $vt))
    (i32.store (global.get $OFF_SCRATCH1) (local.get $pos))
    (return (i32.const 0))
  )

  ;; ─────────────────────────────────────────────────────────────
  ;; Module parser — entry point
  ;; Input: WAT bytes already set up (OFF_WAT_PTR, OFF_WAT_LEN)
  ;; Output: 0=OK, error code otherwise
  ;; ─────────────────────────────────────────────────────────────

  (func $wat_parse_module (result i32)
    (local $pos i32) (local $err i32) (local $b i32)
    (local $kw_off i32) (local $kw_len i32)
    (local $b0 i32) (local $b1 i32) (local $b2 i32) (local $b3 i32)
    (local $type_idx i32) (local $func_idx i32)

    ;; Initialize interpreter state (like $load does)
    (i32.store (global.get $OFF_TYPE_COUNT) (i32.const 0))
    (i32.store (global.get $OFF_FUNCTION_COUNT) (i32.const 0))
    (i32.store (global.get $OFF_CODE_COUNT) (i32.const 0))
    (i32.store (global.get $OFF_EXPORT_COUNT) (i32.const 0))
    (i32.store (global.get $OFF_WAT_SYM) (i32.const 0))

    (local.set $pos (i32.const 0))

    ;; Expect "(" followed by "module"
    (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
    (if (i32.ne (local.get $b) (i32.const 0x28)) (then (return (global.get $ERR_PARSE))))  ;; (
    (local.set $pos (i32.add (local.get $pos) (i32.const 1)))

    (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
    (if (call $wat_read_kw (local.get $pos)) (then (return (global.get $ERR_PARSE))))
    (local.set $kw_off (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $kw_len (i32.load (global.get $OFF_SCRATCH1)))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
    (if (i32.ne (local.get $kw_len) (i32.const 6)) (then (return (global.get $ERR_PARSE))))
    (local.set $b0 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $kw_off))))
    (if (i32.ne (local.get $b0) (i32.const 0x6d)) (then (return (global.get $ERR_PARSE))))  ;; 'm'
    (block $not_module
      (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 5)
            (i32.const 0x6f) (i32.const 0x64) (i32.const 0x75) (i32.const 0x6c)
            (i32.const 0x65) (i32.const 0) (i32.const 0) (i32.const 0)
            (i32.const 0) (i32.const 0))
        (then (br $not_module)))
      ;; Process module body — loop parsing S-expressions
      (block $module_done
        (loop $module_loop
          (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
          (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
          (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
          (if (i32.eq (local.get $b) (i32.const 0x29))  ;; closing )
            (then
              (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
              (br $module_done)))
          (if (i32.ge_u (local.get $p) (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.load (global.get $OFF_WAT_LEN))))
            (then (return (global.get $ERR_PARSE))))

          ;; Expect a sub-expression starting with "("
          (if (i32.ne (local.get $b) (i32.const 0x28))
            (then (return (global.get $ERR_PARSE))))
          (local.set $pos (i32.add (local.get $pos) (i32.const 1)))

          ;; Read keyword after "("
          (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
          (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
          (if (call $wat_read_kw (local.get $pos)) (then (return (global.get $ERR_PARSE))))
          (local.set $kw_off (i32.load (global.get $OFF_SCRATCH0)))
          (local.set $kw_len (i32.load (global.get $OFF_SCRATCH1)))
          (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))

          (local.set $b0 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $kw_off))))

          ;; Dispatch on keyword

          ;; (func ...
          (block $not_func
            (if (i32.ne (local.get $kw_len) (i32.const 4)) (then (br $not_func)))
            (if (i32.ne (local.get $b0) (i32.const 0x66)) (then (br $not_func)))
            (local.set $b1 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.add (local.get $kw_off) (i32.const 1)))))
            (local.set $b2 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.add (local.get $kw_off) (i32.const 2)))))
            (local.set $b3 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.add (local.get $kw_off) (i32.const 3)))))
            (if (i32.ne (local.get $b1) (i32.const 0x75)) (then (br $not_func)))
            (if (i32.ne (local.get $b2) (i32.const 0x6e)) (then (br $not_func)))
            (if (i32.ne (local.get $b3) (i32.const 0x63)) (then (br $not_func)))

            ;; ── Parse (func ...) ──

            ;; Step 1: parse (export "name") if present
            (block $func_export_done
              (loop $func_export_loop
                ;; Look for (export "name")
                ;; First skip ws and peek
                (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
                (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
                (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
                (if (i32.ne (local.get $b) (i32.const 0x28)) (then (br $func_export_done)))
                ;; Peek at what follows — if it's "export", parse it
                (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.add (local.get $pos) (i32.const 1)))))
                (if (i32.ne (local.get $b) (i32.const 0x65))
                  (then
                    (br $func_export_done)))
                (block $try_export
                  (local.set $kw_off (i32.add (local.get $pos) (i32.const 1)))
                  (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 5)
                        (i32.const 0x78) (i32.const 0x70) (i32.const 0x6f) (i32.const 0x72)
                        (i32.const 0x74) (i32.const 0) (i32.const 0) (i32.const 0)
                        (i32.const 0) (i32.const 0))
                    (then (br $try_export)))
                  (local.set $pos (i32.add (local.get $pos) (i32.const 1)))  ;; skip (
                  (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
                  (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
                  (if (call $wat_read_kw (local.get $pos)) (then (return (global.get $ERR_PARSE))))
                  (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
                  ;; Read string
                  (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
                  (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
                  (if (call $wat_read_string (local.get $pos)) (then (return (global.get $ERR_PARSE))))
                  (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
                  ;; Expect )
                  (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
                  (if (i32.ne (local.get $b) (i32.const 0x29)) (then (return (global.get $ERR_PARSE))))
                  (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
                )
                (br $func_export_loop)
              )
            )

            ;; Step 2: parse (param ...) if present
            (local.set $type_idx (i32.load (global.get $OFF_TYPE_COUNT)))
            (local.set $err (call $wat_skip_ws (local.get $pos)))
            (if (local.get $err) (then (return (global.get $ERR_PARSE))))
            (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))

            ;; Build type record at OFF_TYPES_BUF + type_idx * SZ_TYPE
            (local.set $b0 (i32.mul (local.get $type_idx) (global.get $SZ_TYPE)))
            ;; Write func type marker 0x60
            (i32.store8 (i32.add (global.get $OFF_TYPES_BUF) (local.get $b0)) (i32.const 0x60))
            (local.set $b1 (i32.add (local.get $b0) (i32.const 1)))

            ;; Parse param types
            (block $param_done
              (loop $param_loop
                (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
                (if (i32.ne (local.get $b) (i32.const 0x28))
                  (then (br $param_done)))
                (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
                (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
                (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
                (if (call $wat_read_kw (local.get $pos)) (then (return (global.get $ERR_PARSE))))
                (local.set $kw_off (i32.load (global.get $OFF_SCRATCH0)))
                (local.set $kw_len (i32.load (global.get $OFF_SCRATCH1)))
                (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
                (local.set $b0 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $kw_off))))
                (if (i32.ne (local.get $kw_len) (i32.const 5)) (then (br $param_done)))
                (if (i32.ne (local.get $b0) (i32.const 0x70)) (then (br $param_done)))
                ;; match "param"
                (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 4)
                      (i32.const 0x61) (i32.const 0x72) (i32.const 0x61) (i32.const 0x6d)
                      (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 0)
                      (i32.const 0) (i32.const 0))
                  (then (br $param_done)))
                ;; Parse value types until )
                (loop $ptypes
                  (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
                  (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
                  (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
                  (if (i32.eq (local.get $b) (i32.const 0x29))
                    (then
                      (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
                      (br $ptypes)))
                  (if (call $wat_read_kw (local.get $pos)) (then (return (global.get $ERR_PARSE))))
                  (if (call $wat_valtype (i32.load (global.get $OFF_SCRATCH0)) (i32.load (global.get $OFF_SCRATCH1)))
                    (then (return (global.get $ERR_PARSE))))
                  (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
                  (i32.store8 (local.get $b1) (i32.load (global.get $OFF_SCRATCH0)))
                  (local.set $b1 (i32.add (local.get $b1) (i32.const 1)))
                  (br $ptypes)
                )
                (br $param_loop)
              )
            )

            ;; Store param count and parse result
            (local.set $b2 (i32.sub (local.get $b1) (i32.add (local.get $b0) (i32.const 1))))
            ;; Write param count as byte (assuming < 256 params)
            (i32.store8 (i32.add (local.get $b0) (i32.const 1)) (local.get $b2))

            ;; Parse (result ...) if present
            (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
            (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
            (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
            (if (i32.eq (local.get $b) (i32.const 0x28))
              (then
                (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
                (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
                (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
                (if (call $wat_read_kw (local.get $pos)) (then (return (global.get $ERR_PARSE))))
                (local.set $kw_off (i32.load (global.get $OFF_SCRATCH0)))
                (local.set $kw_len (i32.load (global.get $OFF_SCRATCH1)))
                (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
                (local.set $b0 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $kw_off))))
                (if (i32.and (i32.eq (local.get $kw_len) (i32.const 6)) (i32.eq (local.get $b0) (i32.const 0x72)))
                  (then
                    ;; match "result"
                    (block $try_result
                      (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 5)
                            (i32.const 0x65) (i32.const 0x73) (i32.const 0x75) (i32.const 0x6c)
                            (i32.const 0x74) (i32.const 0) (i32.const 0) (i32.const 0)
                            (i32.const 0) (i32.const 0))
                        (then (br $try_result)))
                      (if (call $wat_parse_result (local.get $pos)) (then (return (global.get $ERR_PARSE))))
                      (local.set $pos (i32.load (global.get $OFF_SCRATCH1)))
                      ;; Write result type at b1 (now = pos in type record body)
                      (i32.store8 (local.get $b1) (i32.load (global.get $OFF_SCRATCH0)))
                      (local.set $b1 (i32.add (local.get $b1) (i32.const 1)))
                      ;; Expect closing )
                      (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
                      (if (i32.ne (local.get $b) (i32.const 0x29)) (then (return (global.get $ERR_PARSE))))
                      (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
                    )
                  )
                )
              )
            )

            ;; Type record is complete. Set type_count.
            (i32.store (global.get $OFF_TYPE_COUNT) (i32.add (local.get $type_idx) (i32.const 1)))

            ;; Write function record at OFF_FUNCTIONS_BUF + func_idx * SZ_FUNC
            (local.set $func_idx (i32.load (global.get $OFF_FUNCTION_COUNT)))
            (i32.store8 (i32.add (global.get $OFF_FUNCTIONS_BUF) (i32.mul (local.get $func_idx) (global.get $SZ_FUNC)))
              (local.get $type_idx))
            (i32.store (global.get $OFF_FUNCTION_COUNT) (i32.add (local.get $func_idx) (i32.const 1)))

            ;; Parse function body — read body opcodes
            (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
            (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))

            ;; Emit code section header for this function at OFF_CODE_BUF + code_count * SZ_CODE
            (local.set $code_off (i32.mul (i32.load (global.get $OFF_CODE_COUNT)) (global.get $SZ_CODE)))
            (local.set $b0 (i32.add (global.get $OFF_CODE_BUF) (local.get $code_off)))
            ;; Format: [LEB128 body_size][LEB128 local_count][locals...][opcodes...]
            ;; Skip body_size for now — we'll fill it after body parsing
            ;; Reserve first 5 bytes for size info
            (local.set $body_off (i32.add (local.get $b0) (i32.const 5)))
            (local.set $b0_ptr (local.get $b0))

            ;; Parse body opcodes
            (block $body_end
              (loop $body_loop
                (local.set $err (call $wat_parse_body (local.get $pos) (local.get $body_start)))
                (if (i32.eq (local.get $err) (i32.const -1))  ;; DONE
                  (then
                    (local.set $body_len (i32.load (global.get $OFF_SCRATCH0)))
                    (br $body_end)))
                (if (local.get $err) (then (return (global.get $ERR_PARSE))))
                (local.set $pos (i32.load (global.get $OFF_SCRATCH1)))
                (local.set $body_len (i32.load (global.get $OFF_SCRATCH0)))
                (br $body_loop)
              )
            )
            (local.set $body_len (i32.load (global.get $OFF_SCRATCH0)))

            (local.set $body_start (local.get $body_off))
            WAIT I need locals for these variables.

            (br $not_func)
          )

          ;; (export ... — handled inside (func ...) above
          ;; Other top-level forms — skip for now

          ;; Skip to matching )
          (local.set $depth (i32.const 1))
          (block $skip_done
            (loop $skip_loop
              (if (i32.ge_u (local.get $pos)
                    (i32.load (global.get $OFF_WAT_LEN)))
                (then (return (global.get $ERR_PARSE))))
              (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
              (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
              (if (i32.eq (local.get $b) (i32.const 0x28))
                (then (local.set $depth (i32.add (local.get $depth) (i32.const 1)))))
              (if (i32.eq (local.get $b) (i32.const 0x29))
                (then
                  (local.set $depth (i32.sub (local.get $depth) (i32.const 1)))
                  (if (i32.eqz (local.get $depth)) (then (br $skip_done)))))
              (br $skip_loop)
            )
          )
          (br $module_loop)
        )
      )

      ;; End of module
      ;; Set code_count
      ;; Run decode_opcodes + compute_end_targets for each function
      ;; Set exec_mod_valid = 1
      ;; Return OK

      (return (i32.const 0))
    )
  )

  ;; ═════════════════════════════════════════════════════════════════════════
  ;; Pipeline stage: process_wat_parse
  ;; Reads WAT from input pipe, parses into interpreter state,
  ;; writes 4-byte status to output pipe.
  ;; ═════════════════════════════════════════════════════════════════════════

  (func (export "process_wat_parse")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $len i32) (local $err i32)
    (local $len_slot i32) (local $in_ptr i32)

    ;; Read input via zero-copy
    (local.set $len_slot (i32.sub (i32.add (local.get $scratch) (local.get $scap)) (i32.const 4)))
    (local.set $in_ptr (call $pipe_read_ptr (local.get $input) (local.get $len_slot)))
    (local.set $len (i32.load (local.get $len_slot)))
    (if (i32.eqz (local.get $len)) (then (return (i32.const 0))))

    ;; Set up WAT parser globals to point at input
    (i32.store (global.get $OFF_WAT_PTR) (local.get $in_ptr))
    (i32.store (global.get $OFF_WAT_LEN) (local.get $len))

    ;; Parse
    (local.set $err (call $wat_parse_module))
    (if (local.get $err)
      (then
        (call $pipe_advance (local.get $input) (local.get $len))
        (i32.store (local.get $scratch) (i32.sub (i32.const 0) (local.get $err)))
        (drop (call $pipe_write (local.get $output) (local.get $scratch) (i32.const 4)))
        (return (i32.const 4))))

    ;; Success
    (call $pipe_advance (local.get $input) (local.get $len))
    (i32.store (local.get $scratch) (i32.const 0))
    (drop (call $pipe_write (local.get $output) (local.get $scratch) (i32.const 4)))
    (return (i32.const 4))
  )
