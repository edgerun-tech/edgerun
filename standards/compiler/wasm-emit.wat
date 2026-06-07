  ;; ═════════════════════════════════════════════════════════════════════
  ;; WASM Binary Emitter — serializes decoded module back to .wasm binary
  ;;
  ;; Reads decoded structures from interpreter-core state and writes
  ;; a valid WASM binary to the output buffer.
  ;;
  ;; Exports: emit_wasm(out, max) -> i64 pack(status, size)
  ;; ═════════════════════════════════════════════════════════════════════

  (func $leb_u32_size (param $val i32) (result i32)
    (local $s i32)
    (local.set $s (i32.const 1))
    (block $done
      (loop $cont
        (if (i32.lt_u (local.get $val) (i32.const 128))
          (then (br $done))
        )
        (local.set $s (i32.add (local.get $s) (i32.const 1)))
        (local.set $val (i32.shr_u (local.get $val) (i32.const 7)))
        (br $cont)
      )
    )
    (local.get $s)
  )

  (func $leb_i32_size (param $val i32) (result i32)
    (local $s i32) (local $more i32)
    (local.set $s (i32.const 0))
    (block $done
      (loop $cont
        (local.set $more (i32.const 0))
        (if (i32.lt_s (local.get $val) (i32.const -64))
          (then (local.set $more (i32.const 1)))
        )
        (if (i32.ge_s (local.get $val) (i32.const 63))
          (then (local.set $more (i32.const 1)))
        )
        (local.set $val (i32.shr_s (local.get $val) (i32.const 7)))
        (local.set $s (i32.add (local.get $s) (i32.const 1)))
        (if (local.get $more)
          (then (br $cont))
          (else (br $done))
        )
        (br $cont)
      )
    )
    (local.get $s)
  )

  (func $write_leb_u32 (param $out i32) (param $val i32) (result i32)
    (local $p i32) (local $b i32)
    (local.set $p (local.get $out))
    (block $done
      (loop $cont
        (local.set $b (i32.and (local.get $val) (i32.const 0x7f)))
        (local.set $val (i32.shr_u (local.get $val) (i32.const 7)))
        (if (local.get $val)
          (then (local.set $b (i32.or (local.get $b) (i32.const 0x80))))
          (else (br $done))
        )
        (i32.store8 (local.get $p) (local.get $b))
        (local.set $p (i32.add (local.get $p) (i32.const 1)))
        (br $cont)
      )
    )
    (i32.store8 (local.get $p) (local.get $b))
    (i32.sub (i32.add (local.get $p) (i32.const 1)) (local.get $out))
  )

  (func $write_leb_i32 (param $out i32) (param $val i32) (result i32)
    (local $p i32) (local $b i32) (local $more i32)
    (local.set $p (local.get $out))
    (block $done
      (loop $cont
        (local.set $b (i32.and (local.get $val) (i32.const 0x7f)))
        (local.set $val (i32.shr_s (local.get $val) (i32.const 7)))
        (local.set $more (i32.const 0))
        (if (i32.and (i32.eqz (local.get $val)) (i32.eqz (i32.and (local.get $b) (i32.const 0x40))))
          (then)
          (else
            (if (i32.eq (local.get $val) (i32.const -1))
              (then (if (i32.and (local.get $b) (i32.const 0x40)) (then) (else (local.set $more (i32.const 1)))))
              (else (local.set $more (i32.const 1)))
            )
          )
        )
        (if (local.get $more)
          (then (local.set $b (i32.or (local.get $b) (i32.const 0x80))))
          (else (br $done))
        )
        (i32.store8 (local.get $p) (local.get $b))
        (local.set $p (i32.add (local.get $p) (i32.const 1)))
        (br $cont)
      )
    )
    (i32.store8 (local.get $p) (local.get $b))
    (i32.sub (i32.add (local.get $p) (i32.const 1)) (local.get $out))
  )

  (func $patch_leb_u32 (param $out i32) (param $val i32)
    (local $p i32) (local $b i32)
    (local.set $p (local.get $out))
    (block $done
      (loop $cont
        (local.set $b (i32.and (local.get $val) (i32.const 0x7f)))
        (local.set $val (i32.shr_u (local.get $val) (i32.const 7)))
        (if (local.get $val)
          (then (local.set $b (i32.or (local.get $b) (i32.const 0x80))))
          (else (br $done))
        )
        (i32.store8 (local.get $p) (local.get $b))
        (local.set $p (i32.add (local.get $p) (i32.const 1)))
        (br $cont)
      )
    )
    (i32.store8 (local.get $p) (local.get $b))
  )

  ;; ── Wasm Binary Writer ──

  (func $emit_wasm (export "emit_wasm") (param $out i32) (param $max_size i32) (result i64)
    (local $p i32) (local $i i32) (local $j i32) (local $k i32)
    (local $tc i32) (local $ic i32) (local $fc i32) (local $gc i32) (local $ec i32)
    (local $sc i32) (local $dc i32) (local $mc i32)
    (local $sec_size_p i32)

    (local.set $p (local.get $out))

    ;; Magic: \0asm
    (i32.store (local.get $p) (i32.const 0x6d736100))
    (local.set $p (i32.add (local.get $p) (i32.const 4)))

    ;; Version: 1
    (i32.store (local.get $p) (i32.const 1))
    (local.set $p (i32.add (local.get $p) (i32.const 4)))

    ;; ── Type section ──
    (local.set $tc (i32.load (global.get $OFF_TYPE_COUNT)))
    (if (local.get $tc)
      (then
        (i32.store8 (local.get $p) (global.get $SEC_TYPE))
        (local.set $p (i32.add (local.get $p) (i32.const 1)))
        (local.set $sec_size_p (local.get $p))
        (local.set $p (i32.add (local.get $p) (i32.const 5)))  ;; reserve 5 bytes for LEB128 size
        (local.set $p (i32.add (local.get $p) (call $write_leb_u32 (local.get $p) (local.get $tc))))
        (local.set $i (i32.const 0))
        (block $ts
          (loop $tl
            (if (i32.ge_u (local.get $i) (local.get $tc)) (then (br $ts)))
            (i32.store8 (local.get $p) (i32.const 0x60))
            (local.set $p (i32.add (local.get $p) (i32.const 1)))
            (local.set $j (i32.load16_u (i32.add (global.get $OFF_TYPES_BUF) (i32.add (i32.mul (local.get $i) (global.get $SZ_TYPE)) (i32.const 128)))))
            (local.set $p (i32.add (local.get $p) (call $write_leb_u32 (local.get $p) (local.get $j))))
            (local.set $k (i32.const 0))
            (block $pl
              (loop $pp
                (if (i32.ge_u (local.get $k) (local.get $j)) (then (br $pl)))
                (i32.store8 (local.get $p) (i32.load8_u (i32.add (global.get $OFF_TYPES_BUF) (i32.add (i32.mul (local.get $i) (global.get $SZ_TYPE)) (local.get $k)))))
                (local.set $p (i32.add (local.get $p) (i32.const 1)))
                (local.set $k (i32.add (local.get $k) (i32.const 1)))
                (br $pp)
              )
            )
            (local.set $j (i32.load16_u (i32.add (global.get $OFF_TYPES_BUF) (i32.add (i32.mul (local.get $i) (global.get $SZ_TYPE)) (i32.const 136)))))
            (local.set $p (i32.add (local.get $p) (call $write_leb_u32 (local.get $p) (local.get $j))))
            (local.set $k (i32.const 0))
            (block $rl
              (loop $rp
                (if (i32.ge_u (local.get $k) (local.get $j)) (then (br $rl)))
                (i32.store8 (local.get $p) (i32.load8_u (i32.add (global.get $OFF_TYPES_BUF) (i32.add (i32.add (i32.mul (local.get $i) (global.get $SZ_TYPE)) (i32.const 132)) (local.get $k)))))
                (local.set $p (i32.add (local.get $p) (i32.const 1)))
                (local.set $k (i32.add (local.get $k) (i32.const 1)))
                (br $rp)
              )
            )
            (local.set $i (i32.add (local.get $i) (i32.const 1)))
            (br $tl)
          )
        )
        (call $patch_leb_u32 (local.get $sec_size_p) (i32.sub (local.get $p) (i32.add (local.get $sec_size_p) (i32.const 5))))
      )
    )

    ;; ── Import section ──
    (local.set $ic (i32.load (global.get $OFF_IMPORT_COUNT)))
    (if (local.get $ic)
      (then
        (i32.store8 (local.get $p) (global.get $SEC_IMPORT))
        (local.set $p (i32.add (local.get $p) (i32.const 1)))
        (local.set $sec_size_p (local.get $p))
        (local.set $p (i32.add (local.get $p) (i32.const 5)))
        (local.set $p (i32.add (local.get $p) (call $write_leb_u32 (local.get $p) (local.get $ic))))
        (local.set $i (i32.const 0))
        (block $ims
          (loop $iml
            (if (i32.ge_u (local.get $i) (local.get $ic)) (then (br $ims)))
            ;; Module name: "env" (3 bytes)
            (local.set $p (i32.add (local.get $p) (call $write_leb_u32 (local.get $p) (i32.const 3))))
            (i32.store8 (local.get $p) (i32.const 0x65)) (local.set $p (i32.add (local.get $p) (i32.const 1)))
            (i32.store8 (local.get $p) (i32.const 0x6e)) (local.set $p (i32.add (local.get $p) (i32.const 1)))
            (i32.store8 (local.get $p) (i32.const 0x76)) (local.set $p (i32.add (local.get $p) (i32.const 1)))
            ;; Field name: "f0", "f1", etc.
            (local.set $p (i32.add (local.get $p) (call $write_leb_u32 (local.get $p) (i32.const 2))))
            (i32.store8 (local.get $p) (i32.const 0x66)) (local.set $p (i32.add (local.get $p) (i32.const 1)))
            (i32.store8 (local.get $p) (i32.or (i32.const 0x30) (local.get $i))) (local.set $p (i32.add (local.get $p) (i32.const 1)))
            ;; Kind: function (0)
            (i32.store8 (local.get $p) (global.get $EXT_FUNC))
            (local.set $p (i32.add (local.get $p) (i32.const 1)))
            ;; Type index: i (same as function index)
            (local.set $p (i32.add (local.get $p) (call $write_leb_u32 (local.get $p) (local.get $i))))
            (local.set $i (i32.add (local.get $i) (i32.const 1)))
            (br $iml)
          )
        )
        (call $patch_leb_u32 (local.get $sec_size_p) (i32.sub (local.get $p) (i32.add (local.get $sec_size_p) (i32.const 5))))
      )
    )

    ;; ── Function section ──
    (local.set $fc (i32.load (global.get $OFF_FUNCTION_COUNT)))
    (if (local.get $fc)
      (then
        (i32.store8 (local.get $p) (global.get $SEC_FUNCTION))
        (local.set $p (i32.add (local.get $p) (i32.const 1)))
        (local.set $sec_size_p (local.get $p))
        (local.set $p (i32.add (local.get $p) (i32.const 5)))
        (local.set $p (i32.add (local.get $p) (call $write_leb_u32 (local.get $p) (local.get $fc))))
        (local.set $i (i32.const 0))
        (block $fs
          (loop $fl
            (if (i32.ge_u (local.get $i) (local.get $fc)) (then (br $fs)))
            (local.set $p (i32.add (local.get $p)
              (call $write_leb_u32 (local.get $p)
                (i32.load (i32.add (global.get $OFF_FUNCTIONS_BUF) (i32.mul (local.get $i) (global.get $SZ_FUNC)))))))
            (local.set $i (i32.add (local.get $i) (i32.const 1)))
            (br $fl)
          )
        )
        (call $patch_leb_u32 (local.get $sec_size_p) (i32.sub (local.get $p) (i32.add (local.get $sec_size_p) (i32.const 5))))
      )
    )

    ;; ── Table section ──
    (local.set $mc (i32.load (global.get $OFF_TABLE_HAS)))
    (if (local.get $mc)
      (then
        (i32.store8 (local.get $p) (global.get $SEC_TABLE))
        (local.set $p (i32.add (local.get $p) (i32.const 1)))
        (local.set $sec_size_p (local.get $p))
        (local.set $p (i32.add (local.get $p) (i32.const 5)))
        (local.set $p (i32.add (local.get $p) (call $write_leb_u32 (local.get $p) (i32.const 1))))
        (i32.store8 (local.get $p) (i32.const 0x70))
        (local.set $p (i32.add (local.get $p) (i32.const 1)))
        (i32.store8 (local.get $p) (i32.const 0))
        (local.set $p (i32.add (local.get $p) (i32.const 1)))
        (local.set $p (i32.add (local.get $p) (call $write_leb_u32 (local.get $p) (i32.load (global.get $OFF_TABLE_MIN)))))
        (call $patch_leb_u32 (local.get $sec_size_p) (i32.sub (local.get $p) (i32.add (local.get $sec_size_p) (i32.const 5))))
      )
    )

    ;; ── Memory section ──
    (local.set $mc (i32.load (global.get $OFF_MEM_MIN)))
    (if (local.get $mc)
      (then
        (i32.store8 (local.get $p) (global.get $SEC_MEMORY))
        (local.set $p (i32.add (local.get $p) (i32.const 1)))
        (local.set $sec_size_p (local.get $p))
        (local.set $p (i32.add (local.get $p) (i32.const 5)))
        (local.set $p (i32.add (local.get $p) (call $write_leb_u32 (local.get $p) (i32.const 1))))
        (i32.store8 (local.get $p) (i32.const 0))
        (local.set $p (i32.add (local.get $p) (i32.const 1)))
        (local.set $p (i32.add (local.get $p) (call $write_leb_u32 (local.get $p) (local.get $mc))))
        (call $patch_leb_u32 (local.get $sec_size_p) (i32.sub (local.get $p) (i32.add (local.get $sec_size_p) (i32.const 5))))
      )
    )

    ;; ── Global section ──
    (local.set $gc (i32.load (global.get $OFF_GLOBAL_COUNT)))
    (if (local.get $gc)
      (then
        (i32.store8 (local.get $p) (global.get $SEC_GLOBAL))
        (local.set $p (i32.add (local.get $p) (i32.const 1)))
        (local.set $sec_size_p (local.get $p))
        (local.set $p (i32.add (local.get $p) (i32.const 5)))
        (local.set $p (i32.add (local.get $p) (call $write_leb_u32 (local.get $p) (local.get $gc))))
        (local.set $i (i32.const 0))
        (block $gs
          (loop $gl
            (if (i32.ge_u (local.get $i) (local.get $gc)) (then (br $gs)))
            (i32.store8 (local.get $p) (global.get $TI32))
            (local.set $p (i32.add (local.get $p) (i32.const 1)))
            (i32.store8 (local.get $p) (i32.const 0))
            (local.set $p (i32.add (local.get $p) (i32.const 1)))
            (i32.store8 (local.get $p) (i32.const 0x41))
            (local.set $p (i32.add (local.get $p) (i32.const 1)))
            (local.set $p (i32.add (local.get $p)
              (call $write_leb_i32 (local.get $p) (i32.load (i32.add (global.get $OFF_GLOBALS_BUF) (i32.shl (local.get $i) (i32.const 2)))))))
            (i32.store8 (local.get $p) (i32.const 0x0B))
            (local.set $p (i32.add (local.get $p) (i32.const 1)))
            (local.set $i (i32.add (local.get $i) (i32.const 1)))
            (br $gl)
          )
        )
        (call $patch_leb_u32 (local.get $sec_size_p) (i32.sub (local.get $p) (i32.add (local.get $sec_size_p) (i32.const 5))))
      )
    )

    ;; ── Export section ──
    (local.set $ec (i32.load (global.get $OFF_EXPORT_COUNT)))
    (if (local.get $ec)
      (then
        (i32.store8 (local.get $p) (global.get $SEC_EXPORT))
        (local.set $p (i32.add (local.get $p) (i32.const 1)))
        (local.set $sec_size_p (local.get $p))
        (local.set $p (i32.add (local.get $p) (i32.const 5)))
        (local.set $p (i32.add (local.get $p) (call $write_leb_u32 (local.get $p) (local.get $ec))))
        (local.set $i (i32.const 0))
        (block $es
          (loop $el
            (if (i32.ge_u (local.get $i) (local.get $ec)) (then (br $es)))
            (local.set $j (i32.load (i32.add (global.get $OFF_EXPORTS_BUF) (i32.add (i32.mul (local.get $i) (global.get $SZ_EXPORT)) (i32.const 8)))))
            (local.set $p (i32.add (local.get $p) (call $write_leb_u32 (local.get $p) (local.get $j))))
            (local.set $k (i32.const 0))
            (block $enl
              (loop $enp
                (if (i32.ge_u (local.get $k) (local.get $j)) (then (br $enl)))
                (i32.store8 (local.get $p)
                  (i32.load8_u (i32.add
                    (i32.load (i32.add (global.get $OFF_EXPORTS_BUF) (i32.add (i32.mul (local.get $i) (global.get $SZ_EXPORT)) (i32.const 0))))
                    (local.get $k))))
                (local.set $p (i32.add (local.get $p) (i32.const 1)))
                (local.set $k (i32.add (local.get $k) (i32.const 1)))
                (br $enp)
              )
            )
            (i32.store8 (local.get $p)
              (i32.load8_u (i32.add (global.get $OFF_EXPORTS_BUF) (i32.add (i32.mul (local.get $i) (global.get $SZ_EXPORT)) (i32.const 16)))))
            (local.set $p (i32.add (local.get $p) (i32.const 1)))
            (local.set $p (i32.add (local.get $p)
              (call $write_leb_u32 (local.get $p)
                (i32.load (i32.add (global.get $OFF_EXPORTS_BUF) (i32.add (i32.mul (local.get $i) (global.get $SZ_EXPORT)) (i32.const 24)))))))
            (local.set $i (i32.add (local.get $i) (i32.const 1)))
            (br $el)
          )
        )
        (call $patch_leb_u32 (local.get $sec_size_p) (i32.sub (local.get $p) (i32.add (local.get $sec_size_p) (i32.const 5))))
      )
    )

    ;; ── Start section ──
    (local.set $sc (i32.load (global.get $OFF_START_FUNC)))
    (if (i32.ne (local.get $sc) (i32.const -1))
      (then
        (i32.store8 (local.get $p) (global.get $SEC_START))
        (local.set $p (i32.add (local.get $p) (i32.const 1)))
        (local.set $sec_size_p (local.get $p))
        (local.set $p (i32.add (local.get $p) (i32.const 5)))
        (local.set $p (i32.add (local.get $p) (call $write_leb_u32 (local.get $p) (local.get $sc))))
        (call $patch_leb_u32 (local.get $sec_size_p) (i32.sub (local.get $p) (i32.add (local.get $sec_size_p) (i32.const 5))))
      )
    )

    ;; ── Code section ──
    (local.set $fc (i32.load (global.get $OFF_CODE_COUNT)))
    (if (local.get $fc)
      (then
        (i32.store8 (local.get $p) (global.get $SEC_CODE))
        (local.set $p (i32.add (local.get $p) (i32.const 1)))
        (local.set $sec_size_p (local.get $p))
        (local.set $p (i32.add (local.get $p) (i32.const 5)))
        (local.set $p (i32.add (local.get $p) (call $write_leb_u32 (local.get $p) (local.get $fc))))
        (local.set $i (i32.const 0))
        (block $cs
          (loop $cl
            (if (i32.ge_u (local.get $i) (local.get $fc)) (then (br $cs)))
            (local.set $j (i32.load (i32.add (global.get $OFF_CODE_BUF) (i32.add (i32.mul (local.get $i) (global.get $SZ_CODE)) (i32.const 8)))))
            (local.set $p (i32.add (local.get $p) (call $write_leb_u32 (local.get $p) (local.get $j))))
            (local.set $p (i32.add (local.get $p) (call $write_leb_u32 (local.get $p) (i32.const 0))))
            (call $memcpy (local.get $p)
              (i32.add (i32.load (global.get $OFF_WASM_PTR))
                (i32.load (i32.add (global.get $OFF_CODE_BUF) (i32.add (i32.mul (local.get $i) (global.get $SZ_CODE)) (i32.const 0)))))
              (local.get $j))
            (local.set $p (i32.add (local.get $p) (local.get $j)))
            (local.set $i (i32.add (local.get $i) (i32.const 1)))
            (br $cl)
          )
        )
        (call $patch_leb_u32 (local.get $sec_size_p) (i32.sub (local.get $p) (i32.add (local.get $sec_size_p) (i32.const 5))))
      )
    )

    ;; ── Data section ──
    (local.set $dc (i32.load (global.get $OFF_DATA_COUNT)))
    (if (local.get $dc)
      (then
        (i32.store8 (local.get $p) (global.get $SEC_DATA))
        (local.set $p (i32.add (local.get $p) (i32.const 1)))
        (local.set $sec_size_p (local.get $p))
        (local.set $p (i32.add (local.get $p) (i32.const 5)))
        (local.set $p (i32.add (local.get $p) (call $write_leb_u32 (local.get $p) (local.get $dc))))
        (local.set $i (i32.const 0))
        (block $ds
          (loop $dl
            (if (i32.ge_u (local.get $i) (local.get $dc)) (then (br $ds)))
            (i32.store8 (local.get $p) (i32.const 0))
            (local.set $p (i32.add (local.get $p) (i32.const 1)))
            (i32.store8 (local.get $p) (i32.const 0x41))
            (local.set $p (i32.add (local.get $p) (i32.const 1)))
            (local.set $p (i32.add (local.get $p)
              (call $write_leb_i32 (local.get $p)
                (i32.load (i32.add (global.get $OFF_DATA_BUF) (i32.add (i32.mul (local.get $i) (global.get $SZ_DATA)) (i32.const 4)))))))
            (i32.store8 (local.get $p) (i32.const 0x0B))
            (local.set $p (i32.add (local.get $p) (i32.const 1)))
            (local.set $j (i32.load (i32.add (global.get $OFF_DATA_BUF) (i32.add (i32.mul (local.get $i) (global.get $SZ_DATA)) (i32.const 8)))))
            (local.set $p (i32.add (local.get $p) (call $write_leb_u32 (local.get $p) (local.get $j))))
            (call $memcpy (local.get $p)
              (i32.add (i32.load (global.get $OFF_WASM_PTR))
                (i32.load (i32.add (global.get $OFF_DATA_BUF) (i32.add (i32.mul (local.get $i) (global.get $SZ_DATA)) (i32.const 0)))))
              (local.get $j))
            (local.set $p (i32.add (local.get $p) (local.get $j)))
            (local.set $i (i32.add (local.get $i) (i32.const 1)))
            (br $dl)
          )
        )
        (call $patch_leb_u32 (local.get $sec_size_p) (i32.sub (local.get $p) (i32.add (local.get $sec_size_p) (i32.const 5))))
      )
    )

    ;; Check bounds
    (if (i32.gt_u (i32.sub (local.get $p) (local.get $out)) (local.get $max_size))
      (then (return (call $pack (i32.const 12) (i32.sub (local.get $p) (local.get $out)))))
    )

    (return (call $pack (i32.const 0) (i32.sub (local.get $p) (local.get $out))))
  )
