;; ──────────────────────────────────────────────────────────────────────────────
;; er-tools compile — compile files from the file table to WASM or ELF
;; ──────────────────────────────────────────────────────────────────────────────

  ;; ── $name_matches_suffix(ptr, len, b0, b1, b2, b3) → bool
  ;;    Checks the last 4 bytes of name (or fewer if name len < 4).
  ;;    b3 is first char of suffix (e.g. '.'), b0 is last char.
  (func $name_matches_suffix (param $ptr i32) (param $len i32)
                             (param $b0 i32) (param $b1 i32) (param $b2 i32) (param $b3 i32) (result i32)
    (local $off i32)
    (if (i32.lt_u (local.get $len) (i32.const 4))
      (then (return (i32.const 0)))
    )
    (local.set $off (i32.sub (local.get $len) (i32.const 1)))
    (if (i32.ne (i32.load8_u (i32.add (local.get $ptr) (local.get $off))) (local.get $b0))
      (then (return (i32.const 0)))
    )
    (local.set $off (i32.sub (local.get $len) (i32.const 2)))
    (if (i32.ne (i32.load8_u (i32.add (local.get $ptr) (local.get $off))) (local.get $b1))
      (then (return (i32.const 0)))
    )
    (local.set $off (i32.sub (local.get $len) (i32.const 3)))
    (if (i32.ne (i32.load8_u (i32.add (local.get $ptr) (local.get $off))) (local.get $b2))
      (then (return (i32.const 0)))
    )
    (local.set $off (i32.sub (local.get $len) (i32.const 4)))
    (i32.eq (i32.load8_u (i32.add (local.get $ptr) (local.get $off))) (local.get $b3))
  )

  ;; ── compile_file_to_wasm(name_ptr, name_len) → (output_ptr, output_len)
  (func (export "compile_file_to_wasm") (param $name_ptr i32) (param $name_len i32) (result i32 i32)
    (local $idx i32) (local $cp i32) (local $cl i32)
    (local $err i32)

    (local.set $idx (call $find_file (local.get $name_ptr) (local.get $name_len)))
    (if (i32.lt_s (local.get $idx) (i32.const 0))
      (then (return (i32.const -1) (i32.const 0)))
    )

    (local.set $cp (call $file_content_ptr (local.get $idx)))
    (call $get_file_info (local.get $idx))
    (local.set $cl (global.get $R2))

    ;; ".wasm" = 0x6D 0x73 0x61 0x77 0x2E  (reversed: . m s a w → char order)
    ;;  name_matches_suffix checks: last bytes = 'm'(0x6D) 's'(0x73) 'a'(0x61) 'w'(0x77)
    (if (call $name_matches_suffix (local.get $name_ptr) (local.get $name_len)
          (i32.const 0x6D) (i32.const 0x73) (i32.const 0x61) (i32.const 0x2E))
      (then
        (return (local.get $cp) (local.get $cl))
      )
    )

    ;; ".wat" = 0x74 0x61 0x77 0x2E  (reversed: . w a t → char order)
    (if (call $name_matches_suffix (local.get $name_ptr) (local.get $name_len)
          (i32.const 0x74) (i32.const 0x61) (i32.const 0x77) (i32.const 0x2E))
      (then
        (local.set $err (call $load_wat (local.get $cp) (local.get $cl)))
        (if (local.get $err)
          (then (return (i32.const -2) (i32.const 0)))
        )
        (return
          (global.get $WAT_EMIT_BUF)
          (i32.load (global.get $OFF_WAT_BODY_OFF)))
      )
    )

    (i32.const -3) (i32.const 0)
  )

  ;; ── compile_file_to_elf(name_ptr, name_len, backend) → (output_ptr, output_len)
  (func (export "compile_file_to_elf") (param $name_ptr i32) (param $name_len i32) (param $backend i32) (result i32 i32)
    (local $idx i32) (local $cp i32) (local $cl i32)
    (local $err i32) (local $import_count i32)

    (local.set $idx (call $find_file (local.get $name_ptr) (local.get $name_len)))
    (if (i32.lt_s (local.get $idx) (i32.const 0))
      (then (return (i32.const -1) (i32.const 0)))
    )

    (local.set $cp (call $file_content_ptr (local.get $idx)))
    (call $get_file_info (local.get $idx))
    (local.set $cl (global.get $R2))

    ;; ".wasm" → $load
    (if (call $name_matches_suffix (local.get $name_ptr) (local.get $name_len)
          (i32.const 0x6D) (i32.const 0x73) (i32.const 0x61) (i32.const 0x2E))
      (then
        (local.set $err (call $load (local.get $cp) (local.get $cl)))
        (if (local.get $err)
          (then (return (i32.const -2) (i32.const 0)))
        )
      )
      (else
        ;; ".wat" → $load_wat
        (if (call $name_matches_suffix (local.get $name_ptr) (local.get $name_len)
              (i32.const 0x74) (i32.const 0x61) (i32.const 0x77) (i32.const 0x2E))
          (then
            (local.set $err (call $load_wat (local.get $cp) (local.get $cl)))
            (if (local.get $err)
              (then (return (i32.const -2) (i32.const 0)))
            )
          )
          (else
            (return (i32.const -3) (i32.const 0))
          )
        )
      )
    )

    (local.set $import_count (i32.load (global.get $OFF_IMPORT_COUNT)))
    (return (call $compile_to_elf (local.get $import_count) (local.get $backend)))
  )
