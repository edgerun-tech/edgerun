;; ──────────────────────────────────────────────────────────────────────────────
;; er-tools — self-hosting view/edit/commit embedded source in EdgeRun .wasm
;;
;; Imports memory from host (via "host" "memory" — same convention as all
;; EdgeRun ui_framework fragments).  NO data sections: strings are stored
;; inline or built at runtime, so there's no fixed-address overlap with the
;; wasm binary loaded into memory.
;;
;; Custom section binary format (NO gzip, NO JSON):
;;   [file_count: u32 LEB128]
;;   for each file:
;;     [name_len: u32 LEB128][name_bytes...]
;;     [content_len: u32 LEB128][content_bytes...]
;; ──────────────────────────────────────────────────────────────────────────────

  ;; ── Import memory from host (like all EdgeRun modules) ────────────────────

  ;; ── Fixed address regions ──────────────────────────────────────────────────
  (global $FILE_TABLE    i32 (i32.const 0x3200000))  ;; entry table: 1024 × 16 B
  (global $FILE_TABLE_END i32 (i32.const 0x3204000))
  (global $ARENA        i32 (i32.const 0x3210000))  ;; bump arena for edited content
  (global $SCRATCH      i32 (i32.const 0x3300000))  ;; scratch for I/O
  (global $MAX_FILES    i32 (i32.const 1024))

  ;; ── Scratch globals (return-value convention) ──────────────────────────────
  (global $R0 (mut i32) (i32.const 0))
  (global $R1 (mut i32) (i32.const 0))
  (global $R2 (mut i32) (i32.const 0))

  ;; ── Module state ───────────────────────────────────────────────────────────
  (global $file_count   (mut i32) (i32.const 0))
  (global $arena_ptr    (mut i32) (i32.const 0))

  ;; ═══════════════════════════════════════════════════════════════════════════
  ;; Host interface
  ;; ═══════════════════════════════════════════════════════════════════════════

  ;; ── init_source(data_ptr, data_len) → file_count or -1
  (func $init_source (export "init_source") (param $data_ptr i32) (param $data_len i32) (result i32)
    (global.set $file_count (i32.const 0))
    (global.set $arena_ptr (global.get $ARENA))
    (call $leb_u32_read (local.get $data_ptr))
    (local.set $data_ptr (i32.add (local.get $data_ptr) (global.get $R1)))
    (call $parse_file_entries (local.get $data_ptr) (global.get $R0))
    (global.get $file_count)
  )

  ;; ── list_files(output_ptr, max_len) → actual_len
  (func (export "list_files") (param $out i32) (param $max i32) (result i32)
    (local $pos i32) (local $i i32)
    (local $np i32) (local $nl i32) (local $cl i32)
    (local $line_count i32)

    (local.set $pos (local.get $out))

    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $i) (global.get $file_count)))
        (call $get_file_info (local.get $i))
        (local.set $np (global.get $R0))
        (local.set $nl (global.get $R1))
        (local.set $cl (global.get $R2))

        (local.set $line_count (call $count_lines
          (call $file_content_ptr (local.get $i))
          (local.get $cl)))

        ;; size (10), space, lines (6), space, name, newline
        (local.set $pos (call $write_dec_right (local.get $pos) (local.get $cl) (i32.const 10)))
        (i32.store8 (local.get $pos) (i32.const 0x20))
        (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
        (local.set $pos (call $write_dec_right (local.get $pos) (local.get $line_count) (i32.const 6)))
        (i32.store8 (local.get $pos) (i32.const 0x20))
        (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
        (call $er_memcpy (local.get $pos) (local.get $np) (local.get $nl))
        (local.set $pos (i32.add (local.get $pos) (local.get $nl)))
        (i32.store8 (local.get $pos) (i32.const 0x0a))
        (local.set $pos (i32.add (local.get $pos) (i32.const 1)))

        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br_if $loop (i32.lt_u (local.get $pos) (i32.add (local.get $out) (local.get $max))))
      ))

    ;; Summary line: \n<N> file(s)\n
    (i32.store8 (local.get $pos) (i32.const 0x0a))
    (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
    (local.set $pos (call $write_dec_right (local.get $pos) (global.get $file_count) (i32.const 8)))
    (i32.store8 (local.get $pos) (i32.const 0x20))
    (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
    (if (i32.eq (global.get $file_count) (i32.const 1))
      (then (call $er_memcpy_str (local.get $pos) (i32.const 0x66696c65))  ;; "file"
            (local.set $pos (i32.add (local.get $pos) (i32.const 4))))
      (else (call $er_memcpy_str (local.get $pos) (i32.const 0x66696c65))  ;; "file"
            (i32.store8 (i32.add (local.get $pos) (i32.const 4)) (i32.const 0x73))  ;; "s"
            (local.set $pos (i32.add (local.get $pos) (i32.const 5)))))
    (i32.store8 (local.get $pos) (i32.const 0x0a))
    (local.set $pos (i32.add (local.get $pos) (i32.const 1)))

    (i32.sub (local.get $pos) (local.get $out))
  )

  ;; ── cat_file(name_ptr, name_len, output_ptr, max_len) → actual_len or -1
  (func (export "cat_file") (param $np i32) (param $nl i32) (param $out i32) (param $max i32) (result i32)
    (local $idx i32) (local $cp i32) (local $cl i32)
    (local.set $idx (call $find_file (local.get $np) (local.get $nl)))
    (if (i32.lt_s (local.get $idx) (i32.const 0))
      (then (return (i32.const -1))))
    (local.set $cp (call $file_content_ptr (local.get $idx)))
    (call $get_file_info (local.get $idx))
    (local.set $cl (global.get $R2))
    (call $copy_to_out (local.get $cp) (local.get $cl) (local.get $out) (local.get $max))
  )

  ;; ── edit_file(name_ptr, name_len, new_content_ptr, new_content_len) → 0 ok, -1 not found
  (func (export "edit_file") (param $np i32) (param $nl i32)
                              (param $cp i32) (param $cl i32) (result i32)
    (local $idx i32) (local $new_arena i32)

    (local.set $idx (call $find_file (local.get $np) (local.get $nl)))
    (if (i32.lt_s (local.get $idx) (i32.const 0))
      (then (return (i32.const -1))))

    ;; Allocate from bump arena
    (local.set $new_arena (global.get $arena_ptr))
    (global.set $arena_ptr
      (i32.add (global.get $arena_ptr) (local.get $cl)))

    (call $er_memcpy (local.get $new_arena) (local.get $cp) (local.get $cl))
    (call $set_file_content (local.get $idx) (local.get $new_arena) (local.get $cl))
    (i32.const 0)
  )

  ;; ═══════════════════════════════════════════════════════════════════════════
  ;; Internal helpers
  ;; ═══════════════════════════════════════════════════════════════════════════

  ;; ── $parse_source_section — scans WASM sections, populates file table

  ;; ── $is_source_section(section_id, payload_ptr, payload_len) → bool

  ;; ── $cstr_len(ptr, max) → len (without NUL)

  ;; ── $parse_file_entries(ptr, count) — reads entries from payload into file table
  (func $parse_file_entries (param $ptr i32) (param $count i32)
    (local $i i32) (local $name_len i32) (local $name_ptr i32)
    (local $content_len i32) (local $content_ptr i32)
    (local $entry_off i32)

    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $i) (local.get $count)))

        (call $leb_u32_read (local.get $ptr))
        (local.set $name_len (global.get $R0))
        (local.set $name_ptr (i32.add (local.get $ptr) (global.get $R1)))
        (local.set $ptr (i32.add (local.get $name_ptr) (local.get $name_len)))

        (call $leb_u32_read (local.get $ptr))
        (local.set $content_len (global.get $R0))
        (local.set $content_ptr (i32.add (local.get $ptr) (global.get $R1)))
        (local.set $ptr (i32.add (local.get $content_ptr) (local.get $content_len)))

        (local.set $entry_off
          (i32.add (global.get $FILE_TABLE)
            (i32.mul (local.get $i) (i32.const 16))))
        (i32.store (local.get $entry_off) (local.get $name_ptr))
        (i32.store (i32.add (local.get $entry_off) (i32.const 4)) (local.get $name_len))
        (i32.store (i32.add (local.get $entry_off) (i32.const 8)) (local.get $content_ptr))
        (i32.store (i32.add (local.get $entry_off) (i32.const 12)) (local.get $content_len))

        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop)))
    (global.set $file_count (local.get $count))
  )

  ;; ── $get_file_info(index) → R0=name_ptr, R1=name_len, R2=content_len
  (func $get_file_info (param $idx i32)
    (local $entry_off i32)
    (local.set $entry_off
      (i32.add (global.get $FILE_TABLE)
        (i32.mul (local.get $idx) (i32.const 16))))
    (global.set $R0 (i32.load (local.get $entry_off)))
    (global.set $R1 (i32.load (i32.add (local.get $entry_off) (i32.const 4))))
    (global.set $R2 (i32.load (i32.add (local.get $entry_off) (i32.const 12))))
  )

  ;; ── $file_content_ptr(index) → ptr
  (func $file_content_ptr (param $idx i32) (result i32)
    (i32.load
      (i32.add
        (i32.add (global.get $FILE_TABLE)
          (i32.mul (local.get $idx) (i32.const 16)))
        (i32.const 8)))
  )

  ;; ── $set_file_content(index, content_ptr, content_len)
  (func $set_file_content (param $idx i32) (param $cp i32) (param $cl i32)
    (local $entry_off i32)
    (local.set $entry_off
      (i32.add (global.get $FILE_TABLE)
        (i32.mul (local.get $idx) (i32.const 16))))
    (i32.store (i32.add (local.get $entry_off) (i32.const 8)) (local.get $cp))
    (i32.store (i32.add (local.get $entry_off) (i32.const 12)) (local.get $cl))
  )

  ;; ── $find_file(name_ptr, name_len) → index or -1
  (func $find_file (param $np i32) (param $nl i32) (result i32)
    (local $i i32) (local $name_ptr i32) (local $name_len i32)
    (local $entry_off i32)

    (block $found
      (loop $loop
        (br_if $found (i32.ge_u (local.get $i) (global.get $file_count)))
        (local.set $entry_off
          (i32.add (global.get $FILE_TABLE)
            (i32.mul (local.get $i) (i32.const 16))))
        (local.set $name_ptr (i32.load (local.get $entry_off)))
        (local.set $name_len (i32.load (i32.add (local.get $entry_off) (i32.const 4))))
        (if (i32.and
              (i32.eq (local.get $name_len) (local.get $nl))
              (call $memcmp (local.get $name_ptr) (local.get $np) (local.get $nl)))
          (then (return (local.get $i))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop)))
    (i32.const -1)
  )

  ;; ── $leb_u32_read(ptr) → R0=value, R1=bytes_consumed
  (func $leb_u32_read (param $ptr i32)
    (local $val i32) (local $shift i32) (local $b i32) (local $pos i32)
    (local.set $pos (local.get $ptr))
    (block $done
      (loop $loop
        (local.set $b (i32.load8_u (local.get $pos)))
        (local.set $val (i32.or (local.get $val)
          (i32.shl (i32.and (local.get $b) (i32.const 127)) (local.get $shift))))
        (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
        (local.set $shift (i32.add (local.get $shift) (i32.const 7)))
        (br_if $done (i32.eqz (i32.and (local.get $b) (i32.const 128))))
        (br $loop)))
    (global.set $R0 (local.get $val))
    (global.set $R1 (i32.sub (local.get $pos) (local.get $ptr)))
  )

  ;; ── $er_memcpy(dst, src, len)
  (func $er_memcpy (param $dst i32) (param $src i32) (param $len i32)
    (local $i i32)
    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $i) (local.get $len)))
        (i32.store8
          (i32.add (local.get $dst) (local.get $i))
          (i32.load8_u (i32.add (local.get $src) (local.get $i))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop)))
  )

  ;; ── $er_memcpy_str(dst, packed32) — write 4 bytes of a string literal
  (func $er_memcpy_str (param $dst i32) (param $val i32)
    (i32.store8 (local.get $dst) (i32.and (local.get $val) (i32.const 0xff)))
    (i32.store8 (i32.add (local.get $dst) (i32.const 1))
      (i32.and (i32.shr_u (local.get $val) (i32.const 8)) (i32.const 0xff)))
    (i32.store8 (i32.add (local.get $dst) (i32.const 2))
      (i32.and (i32.shr_u (local.get $val) (i32.const 16)) (i32.const 0xff)))
    (i32.store8 (i32.add (local.get $dst) (i32.const 3))
      (i32.and (i32.shr_u (local.get $val) (i32.const 24)) (i32.const 0xff)))
  )

  ;; ── $memcmp(a_ptr, b_ptr, len) → bool (true = equal)
  (func $memcmp (param $a i32) (param $b i32) (param $len i32) (result i32)
    (local $i i32)
    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $i) (local.get $len)))
        (if (i32.ne
              (i32.load8_u (i32.add (local.get $a) (local.get $i)))
              (i32.load8_u (i32.add (local.get $b) (local.get $i))))
          (then (return (i32.const 0))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop)))
    (i32.const 1)
  )

  ;; ── $count_lines(ptr, len) → line count
  (func $count_lines (param $ptr i32) (param $len i32) (result i32)
    (local $i i32) (local $count i32)
    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $i) (local.get $len)))
        (if (i32.eq (i32.load8_u (i32.add (local.get $ptr) (local.get $i))) (i32.const 0x0a))
          (then (local.set $count (i32.add (local.get $count) (i32.const 1)))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop)))
    (local.get $count)
  )

  ;; ── $write_dec_right(ptr, value, width) → ptr + width
  (func $write_dec_right (param $ptr i32) (param $val i32) (param $width i32) (result i32)
    (local $tmp i32) (local $i i32) (local $end i32)
    (local.set $end (i32.add (local.get $ptr) (local.get $width)))
    (local.set $tmp (local.get $val))
    (block $wr_done
      (loop $wr_loop
        (br_if $wr_done (i32.eqz (local.get $tmp)))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (i32.store8
          (i32.sub (local.get $end) (local.get $i))
          (i32.add (i32.rem_u (local.get $tmp) (i32.const 10)) (i32.const 0x30)))
        (local.set $tmp (i32.div_u (local.get $tmp) (i32.const 10)))
        (br $wr_loop)))
    (if (i32.eqz (local.get $i))
      (then (i32.store8 (i32.sub (local.get $end) (i32.const 1)) (i32.const 0x30))
            (local.set $i (i32.const 1))))
    (block $sp_done
      (loop $sp_loop
        (br_if $sp_done (i32.ge_u (local.get $i) (local.get $width)))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (i32.store8 (i32.sub (local.get $end) (local.get $i)) (i32.const 0x20))
        (br $sp_loop)))
    (local.get $end))

  ;; ── $copy_to_out(src_ptr, src_len, out_ptr, max_len) → actual_len
  (func $copy_to_out (param $sp i32) (param $sl i32) (param $op i32) (param $ml i32) (result i32)
    (local $copy_len i32)
    (local.set $copy_len (local.get $sl))
    (if (i32.gt_u (local.get $copy_len) (local.get $ml))
      (then (local.set $copy_len (local.get $ml))))
    (call $er_memcpy (local.get $op) (local.get $sp) (local.get $copy_len))
    (local.get $copy_len)
  )


