;; EdgeRun Repo Dashboard — production UI component tree using er_ui_writer_* API
  ;;
  ;; Builds a dashboard UI tree with:
  ;;   1. Summary card (stats: files, lines, modules, stages, tests, build %)
  ;;   2. Module breakdown card (13 module entries)
  ;;   3. Pipeline stage status card (26 stage entries)
  ;;
  ;; Pipeline stage: slot 14 — reads metadata via input pipe, writes cmd count to output
  ;; Exports: dashboard_render → cmd count
  ;;
  ;; Metadata input (at DASH_META, written by host before calling render):
  ;;   +0:  i32  wat_files
  ;;   +4:  i32  wat_lines
  ;;   +8:  i32  all_lines
  ;;   +12: i32  mod_count
  ;;   +16: i32  wasm_kb
  ;;   +20: i32  stripped_kb
  ;;   +24: i32  stages_populated
  ;;   +28: i32  tests_passed
  ;;   +32: i32  tests_total
  ;;   +36: i32  pct  (0-100)
  ;;   +40: i32  mod_off  (relative to DASH_META)
  ;;   +44: i32  stage_off (relative to DASH_META)
  ;;
  ;; Module entry (28 bytes each, at DASH_META + mod_off):
  ;;   +0:  i32  files
  ;;   +4:  i32  lines
  ;;   +8:  i32  pct
  ;;   +12: char[16]  name (NUL-padded)
  ;;
  ;; Stage entry (16 bytes each, at DASH_META + stage_off):
  ;;   +0:  i32  populated (0/1)
  ;;   +4:  char[12]  name (NUL-padded)

  ;; ── Memory layout ──
  (global $DASH_TREE     i32 (i32.const 0x1100000))
  (global $DASH_TREE_CAP i32 (i32.const 131072))
  (global $DASH_STR      i32 (i32.const 0x1120000))
  (global $DASH_STR_CAP  i32 (i32.const 65536))
  (global $DASH_CMD      i32 (i32.const 0x1130000))
  (global $DASH_CMD_CAP  i32 (i32.const 131072))
  (global $DASH_TMP      i32 (i32.const 0x1150000))
  (global $DASH_META     i32 (i32.const 0x1151000))
  (global $LBL           i32 (i32.const 0x1160000))

  ;; ── Static label data ──
  (data (i32.const 0x1160000) "EdgeRun Standards")
  (data (i32.const 0x1160020) "WAT standards library")
  (data (i32.const 0x1160100) "Files")
  (data (i32.const 0x1160110) "WAT Lines")
  (data (i32.const 0x1160120) "Total")
  (data (i32.const 0x1160130) "Modules")
  (data (i32.const 0x1160140) "WASM")
  (data (i32.const 0x1160150) "Stripped")
  (data (i32.const 0x1160160) "Stages")
  (data (i32.const 0x1160170) "Tests")
  (data (i32.const 0x1160180) "Build")
  (data (i32.const 0x1160200) "Modules")
  (data (i32.const 0x1160210) "Module breakdown by files and lines")
  (data (i32.const 0x1160300) "Pipeline Stages")
  (data (i32.const 0x1160320) "Stage table population status")
  (data (i32.const 0x1160400) "POPULATED")
  (data (i32.const 0x1160410) "empty")
  (data (i32.const 0x1160500) "Production")
  (data (i32.const 0x1160510) " KB")
  (data (i32.const 0x1160520) "/")
  (data (i32.const 0x1160600) " KB")
  (data (i32.const 0x1160610) " / ")
  (data (i32.const 0x1160620) "%")

  ;; ── Format unsigned integer to buf, returns end pointer ──
  (func $fmt_u32 (param $val i32) (param $buf i32) (result i32)
    (local $i i32) (local $d i32) (local $tmp i32) (local $len i32)
    (local.set $tmp (global.get $DASH_TMP))
    (local.set $i (i32.const 0))
    (if (i32.eqz (local.get $val))
      (then
        (i32.store8 (local.get $buf) (i32.const 48))
        (return (i32.add (local.get $buf) (i32.const 1)))))
    (block $done
      (loop $digits
        (local.set $d (i32.rem_u (local.get $val) (i32.const 10)))
        (i32.store8 (i32.add (local.get $tmp) (local.get $i)) (i32.add (i32.const 48) (local.get $d)))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (local.set $val (i32.div_u (local.get $val) (i32.const 10)))
        (br_if $digits (i32.gt_u (local.get $val) (i32.const 0)))))
    (local.set $len (local.get $i))
    (block $copy
      (loop $rev
        (local.set $i (i32.sub (local.get $i) (i32.const 1)))
        (i32.store8 (local.get $buf) (i32.load8_u (i32.add (local.get $tmp) (local.get $i))))
        (local.set $buf (i32.add (local.get $buf) (i32.const 1)))
        (br_if $rev (i32.gt_u (local.get $i) (i32.const 0)))))
    (local.get $buf))

  ;; ── Copy NUL-terminated string from src to dst, returns end pointer ──
  (func $fmt_str (param $src i32) (param $dst i32) (result i32)
    (local $b i32)
    (block $done
      (loop $copy
        (local.set $b (i32.load8_u (local.get $src)))
        (if (i32.eqz (local.get $b)) (then (br $done)))
        (i32.store8 (local.get $dst) (local.get $b))
        (local.set $src (i32.add (local.get $src) (i32.const 1)))
        (local.set $dst (i32.add (local.get $dst) (i32.const 1)))
        (br $copy)))
    (local.get $dst))

  ;; ── dashboard_render: build UI tree, render to commands, return count ──
  (func $dashboard_render (export "dashboard_render") (result i32)
    (local $p i32) (local $tree_len i32) (local $ok i32) (local $cmd_count i32)
    (local $node i32) (local $parent i32)
    (local $meta i32) (local $mod_off i32) (local $stage_off i32)
    (local $i i32) (local $count i32)
    (local $v i32) (local $str_end i32) (local $ref i32) (local $ref2 i32)
    (local $r_title i32) (local $r_detail i32) (local $r_files i32) (local $r_lines i32)
    (local $r_total i32) (local $r_mods i32) (local $r_wasm i32)
    (local $r_stripped i32) (local $r_stages i32) (local $r_tests i32) (local $r_build i32)
    (local $r_mod_title i32) (local $r_mod_detail i32)
    (local $r_stage_title i32) (local $r_stage_detail i32)
    (local $r_populated i32) (local $r_empty i32) (local $r_prod i32)

    (local.set $meta (global.get $DASH_META))
    (local.set $mod_off (i32.load (i32.add (local.get $meta) (i32.const 40))))
    (local.set $stage_off (i32.load (i32.add (local.get $meta) (i32.const 44))))

    ;; ── Begin tree: up to 48 nodes, 3 roots, column layout ──
    (if (i32.eqz (call $er_ui_writer_begin
      (global.get $DASH_TREE) (global.get $DASH_TREE_CAP)
      (i32.const 48) (i32.const 3)
      (call $er_ui_stack_axis_column)
      (call $er_ui_stack_default_gap)
      (call $er_ui_stack_default_padding)))
      (then (return (i32.const -1))))

    ;; ── Register static label strings ──
    (local.set $r_title (call $er_ui_writer_string (global.get $LBL) (i32.const 17)))
    (local.set $r_detail (call $er_ui_writer_string (i32.add (global.get $LBL) (i32.const 0x20)) (i32.const 24)))
    (local.set $r_files (call $er_ui_writer_string (i32.add (global.get $LBL) (i32.const 0x100)) (i32.const 5)))
    (local.set $r_lines (call $er_ui_writer_string (i32.add (global.get $LBL) (i32.const 0x110)) (i32.const 9)))
    (local.set $r_total (call $er_ui_writer_string (i32.add (global.get $LBL) (i32.const 0x120)) (i32.const 5)))
    (local.set $r_mods (call $er_ui_writer_string (i32.add (global.get $LBL) (i32.const 0x130)) (i32.const 6)))
    (local.set $r_mod_title (call $er_ui_writer_string (i32.add (global.get $LBL) (i32.const 0x200)) (i32.const 6)))
    (local.set $r_mod_detail (call $er_ui_writer_string (i32.add (global.get $LBL) (i32.const 0x210)) (i32.const 36)))
    (local.set $r_stage_title (call $er_ui_writer_string (i32.add (global.get $LBL) (i32.const 0x300)) (i32.const 15)))
    (local.set $r_stage_detail (call $er_ui_writer_string (i32.add (global.get $LBL) (i32.const 0x320)) (i32.const 27)))
    (local.set $r_populated (call $er_ui_writer_string (i32.add (global.get $LBL) (i32.const 0x400)) (i32.const 9)))
    (local.set $r_empty (call $er_ui_writer_string (i32.add (global.get $LBL) (i32.const 0x410)) (i32.const 5)))
    (local.set $r_prod (call $er_ui_writer_string (i32.add (global.get $LBL) (i32.const 0x500)) (i32.const 10)))

    ;; ── Format dynamic strings into DASH_STR area ──
    ;; Each formatted string is written sequentially into DASH_STR + 256
    (local.set $str_end (i32.add (global.get $DASH_STR) (i32.const 256)))

    ;; Files value
    (local.set $str_end (call $fmt_u32 (i32.load (local.get $meta)) (local.get $str_end)))
    (i32.store8 (local.get $str_end) (i32.const 0))
    (local.set $v (i32.sub (local.get $str_end) (i32.add (global.get $DASH_STR) (i32.const 256))))
    (local.set $r_files (call $er_ui_writer_string (i32.add (global.get $DASH_STR) (i32.const 256)) (local.get $v)))
    (local.set $str_end (i32.add (local.get $str_end) (i32.const 1)))

    ;; WAT Lines value
    (local.set $str_end (call $fmt_u32 (i32.load (i32.add (local.get $meta) (i32.const 4))) (local.get $str_end)))
    (i32.store8 (local.get $str_end) (i32.const 0))
    (local.set $v (i32.sub (local.get $str_end) (i32.add (global.get $DASH_STR) (i32.const 256))))
    (local.set $r_lines (call $er_ui_writer_string (i32.add (global.get $DASH_STR) (i32.const 256)) (local.get $v)))
    (local.set $str_end (i32.add (local.get $str_end) (i32.const 1)))

    ;; Total Lines value
    (local.set $str_end (call $fmt_u32 (i32.load (i32.add (local.get $meta) (i32.const 8))) (local.get $str_end)))
    (i32.store8 (local.get $str_end) (i32.const 0))
    (local.set $v (i32.sub (local.get $str_end) (i32.add (global.get $DASH_STR) (i32.const 256))))
    (local.set $r_total (call $er_ui_writer_string (i32.add (global.get $DASH_STR) (i32.const 256)) (local.get $v)))
    (local.set $str_end (i32.add (local.get $str_end) (i32.const 1)))

    ;; Modules count value
    (local.set $str_end (call $fmt_u32 (i32.load (i32.add (local.get $meta) (i32.const 12))) (local.get $str_end)))
    (i32.store8 (local.get $str_end) (i32.const 0))
    (local.set $v (i32.sub (local.get $str_end) (i32.add (global.get $DASH_STR) (i32.const 256))))
    (local.set $r_mods (call $er_ui_writer_string (i32.add (global.get $DASH_STR) (i32.const 256)) (local.get $v)))
    (local.set $str_end (i32.add (local.get $str_end) (i32.const 1)))

    ;; WASM: format number + " KB"
    (local.set $str_end (call $fmt_u32 (i32.load (i32.add (local.get $meta) (i32.const 16))) (local.get $str_end)))
    (local.set $str_end (call $fmt_str (i32.add (global.get $LBL) (i32.const 0x510)) (local.get $str_end)))
    (i32.store8 (local.get $str_end) (i32.const 0))
    (local.set $v (i32.sub (local.get $str_end) (i32.add (global.get $DASH_STR) (i32.const 256))))
    (local.set $r_wasm (call $er_ui_writer_string (i32.add (global.get $DASH_STR) (i32.const 256)) (local.get $v)))
    (local.set $str_end (i32.add (local.get $str_end) (i32.const 1)))

    ;; Stripped: format number + " KB"
    (local.set $str_end (call $fmt_u32 (i32.load (i32.add (local.get $meta) (i32.const 20))) (local.get $str_end)))
    (local.set $str_end (call $fmt_str (i32.add (global.get $LBL) (i32.const 0x510)) (local.get $str_end)))
    (i32.store8 (local.get $str_end) (i32.const 0))
    (local.set $v (i32.sub (local.get $str_end) (i32.add (global.get $DASH_STR) (i32.const 256))))
    (local.set $r_stripped (call $er_ui_writer_string (i32.add (global.get $DASH_STR) (i32.const 256)) (local.get $v)))
    (local.set $str_end (i32.add (local.get $str_end) (i32.const 1)))

    ;; Stages: "N / 64"
    (local.set $str_end (call $fmt_u32 (i32.load (i32.add (local.get $meta) (i32.const 24))) (local.get $str_end)))
    (local.set $str_end (call $fmt_str (i32.add (global.get $LBL) (i32.const 0x610)) (local.get $str_end)))
    (local.set $str_end (call $fmt_u32 (i32.const 64) (local.get $str_end)))
    (i32.store8 (local.get $str_end) (i32.const 0))
    (local.set $v (i32.sub (local.get $str_end) (i32.add (global.get $DASH_STR) (i32.const 256))))
    (local.set $r_stages (call $er_ui_writer_string (i32.add (global.get $DASH_STR) (i32.const 256)) (local.get $v)))
    (local.set $str_end (i32.add (local.get $str_end) (i32.const 1)))

    ;; Tests: "N / M"
    (local.set $str_end (call $fmt_u32 (i32.load (i32.add (local.get $meta) (i32.const 28))) (local.get $str_end)))
    (local.set $str_end (call $fmt_str (i32.add (global.get $LBL) (i32.const 0x610)) (local.get $str_end)))
    (local.set $str_end (call $fmt_u32 (i32.load (i32.add (local.get $meta) (i32.const 32))) (local.get $str_end)))
    (i32.store8 (local.get $str_end) (i32.const 0))
    (local.set $v (i32.sub (local.get $str_end) (i32.add (global.get $DASH_STR) (i32.const 256))))
    (local.set $r_tests (call $er_ui_writer_string (i32.add (global.get $DASH_STR) (i32.const 256)) (local.get $v)))
    (local.set $str_end (i32.add (local.get $str_end) (i32.const 1)))

    ;; Build pct: "N%"
    (local.set $str_end (call $fmt_u32 (i32.load (i32.add (local.get $meta) (i32.const 36))) (local.get $str_end)))
    (local.set $str_end (call $fmt_str (i32.add (global.get $LBL) (i32.const 0x620)) (local.get $str_end)))
    (i32.store8 (local.get $str_end) (i32.const 0))
    (local.set $v (i32.sub (local.get $str_end) (i32.add (global.get $DASH_STR) (i32.const 256))))
    (local.set $r_build (call $er_ui_writer_string (i32.add (global.get $DASH_STR) (i32.const 256)) (local.get $v)))
    (local.set $str_end (i32.add (local.get $str_end) (i32.const 1)))

    ;; ── Node 0: Summary card ──
    (drop (call $er_ui_writer_record (i32.const 0) (i32.const 28) (i32.const 100)
      (local.get $r_title) (local.get $r_detail)))

    ;; Node 1: Files row
    (drop (call $er_ui_writer_record_child (i32.const 1) (i32.const 0) (i32.const 26) (i32.const 101)
      (local.get $r_files) (local.get $r_total)))

    ;; Node 2: WAT Lines row
    (drop (call $er_ui_writer_record_child (i32.const 2) (i32.const 0) (i32.const 26) (i32.const 102)
      (local.get $r_lines) (i32.const 0)))

    ;; Node 3: Modules row
    (drop (call $er_ui_writer_record_child (i32.const 3) (i32.const 0) (i32.const 26) (i32.const 103)
      (local.get $r_mods) (i32.const 0)))

    ;; Node 4: WASM size row
    (drop (call $er_ui_writer_record_child (i32.const 4) (i32.const 0) (i32.const 26) (i32.const 104)
      (local.get $r_wasm) (i32.const 0)))

    ;; Node 5: Stripped size row
    (drop (call $er_ui_writer_record_child (i32.const 5) (i32.const 0) (i32.const 26) (i32.const 105)
      (local.get $r_stripped) (i32.const 0)))

    ;; Node 6: Stages row
    (drop (call $er_ui_writer_record_child (i32.const 6) (i32.const 0) (i32.const 26) (i32.const 106)
      (local.get $r_stages) (i32.const 0)))

    ;; Node 7: Tests row
    (drop (call $er_ui_writer_record_child (i32.const 7) (i32.const 0) (i32.const 26) (i32.const 107)
      (local.get $r_tests) (i32.const 0)))

    ;; Node 8: Build progress bar
    (local.set $v (i32.mul (i32.load (i32.add (local.get $meta) (i32.const 36))) (i32.const 655)))
    (drop (call $er_ui_writer_record_child (i32.const 8) (i32.const 0) (i32.const 24) (i32.const 108)
      (local.get $r_build) (local.get $v)))

    ;; Node 9: Production badge
    (drop (call $er_ui_writer_record_child (i32.const 9) (i32.const 0) (i32.const 27) (i32.const 109)
      (local.get $r_prod) (i32.const 1)))

    ;; ── Node 10: Modules card ──
    (local.set $node (i32.const 10))
    (drop (call $er_ui_writer_record (local.get $node) (i32.const 28) (i32.const 200)
      (local.get $r_mod_title) (local.get $r_mod_detail)))
    (local.set $parent (local.get $node))

    ;; Module rows
    (local.set $count (i32.load (i32.add (local.get $meta) (i32.const 12))))
    (local.set $i (i32.const 0))
    (block $mod_done
      (loop $mod_loop
        (br_if $mod_done (i32.ge_u (local.get $i) (local.get $count)))
        (local.set $node (i32.add (i32.const 11) (local.get $i)))
        (local.set $p (i32.add (local.get $meta) (local.get $mod_off)))
        (local.set $p (i32.add (local.get $p) (i32.mul (local.get $i) (i32.const 28))))
        ;; Title = module name from metadata, Detail = "files / lines"
        (local.set $str_end (i32.add (global.get $DASH_STR) (i32.const 512)))
        (local.set $str_end (call $fmt_u32 (i32.load (local.get $p)) (local.get $str_end)))
        (local.set $str_end (call $fmt_str (i32.add (global.get $LBL) (i32.const 0x610)) (local.get $str_end)))
        (local.set $str_end (call $fmt_u32 (i32.load (i32.add (local.get $p) (i32.const 4))) (local.get $str_end)))
        (i32.store8 (local.get $str_end) (i32.const 0))
        (local.set $ref (call $er_ui_writer_string (i32.add (global.get $DASH_STR) (i32.const 512))
          (i32.sub (local.get $str_end) (i32.add (global.get $DASH_STR) (i32.const 512)))))
        ;; Format module name from metadata entry
        (local.set $str_end (i32.add (global.get $DASH_STR) (i32.const 768)))
        (local.set $str_end (call $fmt_str (i32.add (local.get $p) (i32.const 12)) (local.get $str_end)))
        (i32.store8 (local.get $str_end) (i32.const 0))
        (local.set $ref2 (call $er_ui_writer_string (i32.add (global.get $DASH_STR) (i32.const 768))
          (i32.sub (local.get $str_end) (i32.add (global.get $DASH_STR) (i32.const 768)))))
        (drop (call $er_ui_writer_record_child (local.get $node) (local.get $parent) (i32.const 26)
          (i32.add (i32.const 210) (local.get $i))
          (local.get $ref2) (local.get $ref)))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $mod_loop)))

    ;; ── Pipeline Stages card ──
    (local.set $node (i32.add (i32.const 11) (local.get $count)))
    (drop (call $er_ui_writer_record (local.get $node) (i32.const 28) (i32.const 300)
      (local.get $r_stage_title) (local.get $r_stage_detail)))
    (local.set $parent (local.get $node))

    ;; Stage rows (first 26 stages)
    (local.set $i (i32.const 0))
    (block $stage_done
      (loop $stage_loop
        (br_if $stage_done (i32.ge_u (local.get $i) (i32.const 26)))
        (local.set $node (i32.add (i32.add (i32.const 12) (local.get $count)) (local.get $i)))
        (local.set $p (i32.add (local.get $meta) (local.get $stage_off)))
        (local.set $p (i32.add (local.get $p) (i32.mul (local.get $i) (i32.const 16))))
        (local.set $v (i32.load (local.get $p)))
        (local.set $ref (if (result i32) (local.get $v)
          (then (local.get $r_populated))
          (else (local.get $r_empty))))
        ;; Format stage name from metadata
        (local.set $str_end (i32.add (global.get $DASH_STR) (i32.const 512)))
        (local.set $str_end (call $fmt_str (i32.add (local.get $p) (i32.const 4)) (local.get $str_end)))
        (i32.store8 (local.get $str_end) (i32.const 0))
        (local.set $ref2 (call $er_ui_writer_string (i32.add (global.get $DASH_STR) (i32.const 512))
          (i32.sub (local.get $str_end) (i32.add (global.get $DASH_STR) (i32.const 512)))))
        (drop (call $er_ui_writer_record_child (local.get $node) (local.get $parent) (i32.const 26)
          (i32.add (i32.const 400) (local.get $i))
          (local.get $ref2) (local.get $ref)))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $stage_loop)))

    ;; ── Get cursor (tree length) ──
    (local.set $tree_len (call $er_ui_writer_cursor))

    ;; ── Validate ──
    (local.set $ok (call $er_ui_validate_deep (global.get $DASH_TREE) (local.get $tree_len)))
    (if (i32.eqz (local.get $ok)) (then (return (i32.const -3))))

    ;; ── Render to command buffer ──
    (local.set $cmd_count (call $er_ui_render
      (global.get $DASH_TREE) (local.get $tree_len)
      (global.get $DASH_CMD) (global.get $DASH_CMD_CAP)
      (f32.const 0) (f32.const 0) (f32.const 800) (f32.const 1200)))
    (local.get $cmd_count))

  ;; ── Pipeline stage: process_dashboard (slot 14) ──
  (func $process_dashboard (export "process_dashboard")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $avail i32) (local $r i32) (local $cmd_count i32)
    (local.set $avail (call $pipe_available (local.get $input)))
    (if (i32.lt_u (local.get $avail) (i32.const 48))
      (then (return (i32.const 0))))
    (local.set $r (call $pipe_read (local.get $input) (global.get $DASH_META) (i32.const 48)))
    (if (i32.lt_s (local.get $r) (i32.const 0))
      (then (return (local.get $r))))
    (local.set $r (call $pipe_read (local.get $input)
      (i32.add (global.get $DASH_META) (i32.const 48))
      (i32.mul (i32.load (i32.add (global.get $DASH_META) (i32.const 12))) (i32.const 28))))
    (if (i32.lt_s (local.get $r) (i32.const 0))
      (then (return (local.get $r))))
    (local.set $r (call $pipe_read (local.get $input)
      (i32.add (global.get $DASH_META) (i32.const 48))
      (i32.mul (i32.const 26) (i32.const 16))))
    (if (i32.lt_s (local.get $r) (i32.const 0))
      (then (return (local.get $r))))
    (local.set $cmd_count (call $dashboard_render))
    (if (i32.lt_s (local.get $cmd_count) (i32.const 0))
      (then (return (local.get $cmd_count))))
    (i32.store (global.get $DASH_TMP) (local.get $cmd_count))
    (drop (call $pipe_write (local.get $output) (global.get $DASH_TMP) (i32.const 4)))
    (local.get $cmd_count))

  (elem (i32.const 14) $process_dashboard)
