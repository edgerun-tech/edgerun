  ;; Status values: 0 ok, 1 input_short, 3 invalid.
  ;; Derived DB: magic ERDB0001, format 2, schema 1, flags 0.
  ;; FileIndex strings are u64-len-prefixed UTF-8 fields in the Rust storage.
  ;; EdgeFS path validation mirrors normalize_path_checked enough to reject NUL
  ;; and root escapes while classifying root/current as a normalized empty path.

  (data (i32.const 60000)
    "events.bin\00stream_heads.bin\00replay_cache.bin\00peers.bin\00snapshots.bin\00delegations.bin\00revocations.bin\00credentials.bin\00object_presence.bin\00controller_changes.bin\00fetch_queue.bin\00pending\00done\00failed\00discovered\00status_changed\00unreachable\00")

  (func $load8 (param $ptr i32) (param $off i32) (result i32)
    (i32.load8_u (i32.add (local.get $ptr) (local.get $off))))

  (func $str_eq (param $ptr i32) (param $len i32) (param $lit i32) (param $lit_len i32) (result i32)
    (local $i i32)
    (if (i32.ne (local.get $len) (local.get $lit_len)) (then (return (i32.const 0))))
    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $i) (local.get $len)))
        (if
          (i32.ne
            (i32.load8_u (i32.add (local.get $ptr) (local.get $i)))
            (i32.load8_u (i32.add (local.get $lit) (local.get $i))))
          (then (return (i32.const 0))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        br $loop))
    i32.const 1)

  (func $fnv_update (param $hash i32) (param $byte i32) (result i32)
    (i32.mul
      (i32.xor (local.get $hash) (local.get $byte))
      (i32.const 16777619)))

  (func $fnv_bytes (param $ptr i32) (param $len i32) (param $hash i32) (result i32)
    (local $i i32)
    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $i) (local.get $len)))
        (local.set $hash (call $fnv_update (local.get $hash) (call $load8 (local.get $ptr) (local.get $i))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        br $loop))
    local.get $hash)

  (func (export "storage_object_kind_valid") (param $kind i32) (result i32)
    (i32.le_u (local.get $kind) (i32.const 10)))

  (func (export "storage_object_kind_portable") (param $kind i32) (result i32)
    (i32.and
      (i32.le_u (local.get $kind) (i32.const 10))
      (i32.ne (local.get $kind) (i32.const 0))))

  (func (export "storage_action_status_valid") (param $status i32) (result i32)
    (i32.le_u (local.get $status) (i32.const 3)))

  (func (export "storage_op_event_type_valid") (param $event_type i32) (result i32)
    (if
      (i32.and
        (i32.ge_s (local.get $event_type) (i32.const 100))
        (i32.le_s (local.get $event_type) (i32.const 112)))
      (then (return (i32.const 1))))
    (if
      (i32.and
        (i32.ge_s (local.get $event_type) (i32.const 114))
        (i32.le_s (local.get $event_type) (i32.const 119)))
      (then (return (i32.const 1))))
    i32.const 0)

  (func (export "storage_fetch_status_code") (param $ptr i32) (param $len i32) (result i32)
    ;; 1 pending, 2 done, 3 failed, 0 unknown.
    (if (call $str_eq (local.get $ptr) (local.get $len) (i32.const 60176) (i32.const 7))
      (then (return (i32.const 1))))
    (if (call $str_eq (local.get $ptr) (local.get $len) (i32.const 60184) (i32.const 4))
      (then (return (i32.const 2))))
    (if (call $str_eq (local.get $ptr) (local.get $len) (i32.const 60189) (i32.const 6))
      (then (return (i32.const 3))))
    i32.const 0)

  (func (export "storage_peer_status_code") (param $ptr i32) (param $len i32) (result i32)
    ;; 1 discovered, 2 status_changed, 3 unreachable, 0 unknown.
    (if (call $str_eq (local.get $ptr) (local.get $len) (i32.const 60196) (i32.const 10))
      (then (return (i32.const 1))))
    (if (call $str_eq (local.get $ptr) (local.get $len) (i32.const 60207) (i32.const 14))
      (then (return (i32.const 2))))
    (if (call $str_eq (local.get $ptr) (local.get $len) (i32.const 60222) (i32.const 11))
      (then (return (i32.const 3))))
    i32.const 0)

  (func (export "derived_db_header_checksum") (param $ptr i32) (param $len i32) (result i32)
    (call $fnv_bytes (local.get $ptr) (local.get $len) (i32.const -2128831035)))

  (func (export "derived_db_record_checksum") (param $kind i32) (param $ptr i32) (param $len i32) (result i32)
    (call $fnv_bytes
      (local.get $ptr)
      (local.get $len)
      (call $fnv_update (i32.const -2128831035) (local.get $kind))))

  (func (export "derived_db_record_kind_valid") (param $kind i32) (result i32)
    (i32.and
      (i32.ge_u (local.get $kind) (i32.const 1))
      (i32.le_u (local.get $kind) (i32.const 5))))

  (func (export "derived_db_header_validate") (param $ptr i32) (param $len i32) (result i32)
    (local $expected i32)
    (if (i32.lt_u (local.get $len) (i32.const 20)) (then (return (i32.const 1))))
    (if (i32.eqz (call $str_eq (local.get $ptr) (i32.const 8) (i32.const 60243) (i32.const 8)))
      (then (return (i32.const 3))))
    (if (i32.ne (i32.load16_u (i32.add (local.get $ptr) (i32.const 8))) (i32.const 2))
      (then (return (i32.const 3))))
    (if (i32.ne (i32.load16_u (i32.add (local.get $ptr) (i32.const 10))) (i32.const 1))
      (then (return (i32.const 3))))
    (if (i32.ne (i32.load (i32.add (local.get $ptr) (i32.const 12))) (i32.const 0))
      (then (return (i32.const 3))))
    (local.set $expected (call $fnv_bytes (local.get $ptr) (i32.const 16) (i32.const -2128831035)))
    (if (i32.ne (i32.load (i32.add (local.get $ptr) (i32.const 16))) (local.get $expected))
      (then (return (i32.const 3))))
    i32.const 0)

  (func (export "derived_db_access_path")
    (param $table i32) (param $has_key i32) (param $has_object_id i32) (param $has_stream_id i32) (param $has_seq i32) (param $has_id i32)
    (result i32)
    ;; Tables: 1 runtime_meta, 2 derived_object_index, 3 admin_audit.
    ;; Return: 0 full_scan, 1 runtime_meta_key, 2 object_id,
    ;; 3 stream, 4 stream_seq, 5 admin_audit_id.
    (if (i32.and (i32.eq (local.get $table) (i32.const 1)) (local.get $has_key))
      (then (return (i32.const 1))))
    (if (i32.eq (local.get $table) (i32.const 2))
      (then
        (if (local.get $has_object_id) (then (return (i32.const 2))))
        (if (i32.and (local.get $has_stream_id) (local.get $has_seq))
          (then (return (i32.const 4))))
        (if (local.get $has_stream_id) (then (return (i32.const 3))))))
    (if (i32.and (i32.eq (local.get $table) (i32.const 3)) (local.get $has_id))
      (then (return (i32.const 5))))
    i32.const 0)

  (func (export "file_index_filename_kind") (param $ptr i32) (param $len i32) (result i32)
    (if (call $str_eq (local.get $ptr) (local.get $len) (i32.const 60000) (i32.const 10)) (then (return (i32.const 2))))
    (if (call $str_eq (local.get $ptr) (local.get $len) (i32.const 60011) (i32.const 16)) (then (return (i32.const 1))))
    (if (call $str_eq (local.get $ptr) (local.get $len) (i32.const 60028) (i32.const 16)) (then (return (i32.const 3))))
    (if (call $str_eq (local.get $ptr) (local.get $len) (i32.const 60045) (i32.const 9)) (then (return (i32.const 4))))
    (if (call $str_eq (local.get $ptr) (local.get $len) (i32.const 60055) (i32.const 13)) (then (return (i32.const 5))))
    (if (call $str_eq (local.get $ptr) (local.get $len) (i32.const 60069) (i32.const 15)) (then (return (i32.const 6))))
    (if (call $str_eq (local.get $ptr) (local.get $len) (i32.const 60085) (i32.const 15)) (then (return (i32.const 7))))
    (if (call $str_eq (local.get $ptr) (local.get $len) (i32.const 60101) (i32.const 15)) (then (return (i32.const 8))))
    (if (call $str_eq (local.get $ptr) (local.get $len) (i32.const 60117) (i32.const 19)) (then (return (i32.const 9))))
    (if (call $str_eq (local.get $ptr) (local.get $len) (i32.const 60137) (i32.const 22)) (then (return (i32.const 10))))
    (if (call $str_eq (local.get $ptr) (local.get $len) (i32.const 60160) (i32.const 15)) (then (return (i32.const 11))))
    i32.const 0)

  (func (export "file_index_record_shape") (param $kind i32) (result i32)
    ;; Stable field counts for the accepted .bin files. Returns 0 for unknown.
    (if (i32.eq (local.get $kind) (i32.const 1)) (then (return (i32.const 4))))
    (if (i32.eq (local.get $kind) (i32.const 2)) (then (return (i32.const 6))))
    (if (i32.eq (local.get $kind) (i32.const 3)) (then (return (i32.const 4))))
    (if (i32.eq (local.get $kind) (i32.const 4)) (then (return (i32.const 6))))
    (if (i32.eq (local.get $kind) (i32.const 5)) (then (return (i32.const 2))))
    (if (i32.eq (local.get $kind) (i32.const 6)) (then (return (i32.const 7))))
    (if (i32.eq (local.get $kind) (i32.const 7)) (then (return (i32.const 6))))
    (if (i32.eq (local.get $kind) (i32.const 8)) (then (return (i32.const 4))))
    (if (i32.eq (local.get $kind) (i32.const 9)) (then (return (i32.const 4))))
    (if (i32.eq (local.get $kind) (i32.const 10)) (then (return (i32.const 3))))
    (if (i32.eq (local.get $kind) (i32.const 11)) (then (return (i32.const 6))))
    i32.const 0)

  (func (export "file_index_credential_key_valid") (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (local $slash i32)
    (local $b i32)
    (if (i32.eqz (local.get $len)) (then (return (i32.const 0))))
    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $i) (local.get $len)))
        (local.set $b (call $load8 (local.get $ptr) (local.get $i)))
        (if (i32.or (i32.eqz (local.get $b)) (i32.eq (local.get $b) (i32.const 47)))
          (then
            (if (i32.eqz (local.get $b)) (then (return (i32.const 0))))
            (if
              (i32.or
                (i32.eqz (local.get $i))
                (i32.eq (local.get $i) (i32.sub (local.get $len) (i32.const 1))))
              (then (return (i32.const 0))))
            (local.set $slash (i32.add (local.get $slash) (i32.const 1)))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        br $loop))
    (i32.eq (local.get $slash) (i32.const 1)))

  (func (export "block_event_log_header_validate") (param $ptr i32) (param $len i32) (param $sector_size i32) (param $sectors i64) (result i32)
    (local $cursor i64)
    (local $region i64)
    (if (i32.lt_u (local.get $len) (i32.const 16)) (then (return (i32.const 1))))
    (if (i32.eqz (call $str_eq (local.get $ptr) (i32.const 4) (i32.const 60251) (i32.const 4)))
      (then (return (i32.const 3))))
    (if (i32.lt_u (local.get $sector_size) (i32.const 16)) (then (return (i32.const 3))))
    (if (i64.eqz (local.get $sectors)) (then (return (i32.const 3))))
    (local.set $cursor (i64.load (i32.add (local.get $ptr) (i32.const 4))))
    (local.set $region
      (i64.mul
        (i64.sub (local.get $sectors) (i64.const 1))
        (i64.extend_i32_u (local.get $sector_size))))
    (if (i64.gt_u (local.get $cursor) (local.get $region)) (then (return (i32.const 3))))
    i32.const 0)

  (func (export "block_event_stream_append_status")
    (param $head_present i32) (param $head_seq i64) (param $event_seq i64) (param $has_prev i32) (param $prev_alg i32) (param $prev_matches i32)
    (result i32)
    (if (i32.eqz (local.get $head_present))
      (then
        (if (i64.ne (local.get $event_seq) (i64.const 0)) (then (return (i32.const 3))))
        (if (local.get $has_prev) (then (return (i32.const 3))))
        (return (i32.const 0))))
    (if (i64.ne (local.get $event_seq) (i64.add (local.get $head_seq) (i64.const 1)))
      (then (return (i32.const 3))))
    (if (i32.eqz (local.get $has_prev)) (then (return (i32.const 3))))
    (if (i32.ne (local.get $prev_alg) (i32.const 1)) (then (return (i32.const 3))))
    (if (i32.eqz (local.get $prev_matches)) (then (return (i32.const 3))))
    i32.const 0)

  (func (export "edgefs_entry_kind_valid") (param $kind i32) (result i32)
    (i32.and
      (i32.ge_u (local.get $kind) (i32.const 1))
      (i32.le_u (local.get $kind) (i32.const 7))))

  (func (export "edgefs_entry_kind_class") (param $kind i32) (result i32)
    ;; 0 invalid, 1 file, 2 directory, 3 link, 4 device, 5 fifo.
    (if (i32.eq (local.get $kind) (i32.const 1)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $kind) (i32.const 2)) (then (return (i32.const 2))))
    (if
      (i32.or (i32.eq (local.get $kind) (i32.const 3)) (i32.eq (local.get $kind) (i32.const 4)))
      (then (return (i32.const 3))))
    (if
      (i32.or (i32.eq (local.get $kind) (i32.const 5)) (i32.eq (local.get $kind) (i32.const 6)))
      (then (return (i32.const 4))))
    (if (i32.eq (local.get $kind) (i32.const 7)) (then (return (i32.const 5))))
    i32.const 0)

  (func (export "edgefs_create_node_kind_valid") (param $kind i32) (result i32)
    (i32.or
      (i32.eq (local.get $kind) (i32.const 2))
      (i32.or
        (i32.eq (local.get $kind) (i32.const 5))
        (i32.or
          (i32.eq (local.get $kind) (i32.const 6))
          (i32.eq (local.get $kind) (i32.const 7))))))

  (func $pack_status_depth (param $status i32) (param $depth i32) (result i64)
    (i64.or
      (i64.extend_i32_u (local.get $status))
      (i64.shl (i64.extend_i32_u (local.get $depth)) (i64.const 32))))

  (func $edgefs_process_segment (param $ptr i32) (param $start i32) (param $len i32) (param $depth i32) (result i64)
    (if (i32.eqz (local.get $len))
      (then (return (call $pack_status_depth (i32.const 0) (local.get $depth)))))
    (if
      (i32.and
        (i32.eq (local.get $len) (i32.const 1))
        (i32.eq (call $load8 (local.get $ptr) (local.get $start)) (i32.const 46)))
      (then (return (call $pack_status_depth (i32.const 0) (local.get $depth)))))
    (if
      (i32.and
        (i32.eq (local.get $len) (i32.const 2))
        (i32.and
          (i32.eq (call $load8 (local.get $ptr) (local.get $start)) (i32.const 46))
          (i32.eq (call $load8 (local.get $ptr) (i32.add (local.get $start) (i32.const 1))) (i32.const 46))))
      (then
        (if (i32.eqz (local.get $depth))
          (then (return (call $pack_status_depth (i32.const 2) (i32.const 0)))))
        (return (call $pack_status_depth
          (i32.const 0)
          (i32.sub (local.get $depth) (i32.const 1))))))
    (call $pack_status_depth (i32.const 0) (i32.add (local.get $depth) (i32.const 1))))

  (func (export "edgefs_path_classify") (param $ptr i32) (param $len i32) (param $allow_empty i32) (result i32)
    ;; 0 valid nonempty, 1 valid empty/root/current, 2 invalid path/root escape,
    ;; 3 contains NUL.
    (local $i i32)
    (local $seg_start i32)
    (local $seg_len i32)
    (local $depth i32)
    (local $b i32)
    (local $packed i64)
    (block $finish
      (loop $loop
        (br_if $finish (i32.ge_u (local.get $i) (local.get $len)))
        (local.set $b (call $load8 (local.get $ptr) (local.get $i)))
        (if (i32.eqz (local.get $b)) (then (return (i32.const 3))))
        (if (i32.eq (local.get $b) (i32.const 47))
          (then
            (local.set $packed (call $edgefs_process_segment (local.get $ptr) (local.get $seg_start) (local.get $seg_len) (local.get $depth)))
            (if (i32.ne (i32.wrap_i64 (local.get $packed)) (i32.const 0))
              (then (return (i32.wrap_i64 (local.get $packed)))))
            (local.set $depth (i32.wrap_i64 (i64.shr_u (local.get $packed) (i64.const 32))))
            (local.set $seg_len (i32.const 0))
            (local.set $seg_start (i32.add (local.get $i) (i32.const 1))))
          (else
            (local.set $seg_len (i32.add (local.get $seg_len) (i32.const 1)))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        br $loop))
    (local.set $packed (call $edgefs_process_segment (local.get $ptr) (local.get $seg_start) (local.get $seg_len) (local.get $depth)))
    (if (i32.ne (i32.wrap_i64 (local.get $packed)) (i32.const 0))
      (then (return (i32.wrap_i64 (local.get $packed)))))
    (local.set $depth (i32.wrap_i64 (i64.shr_u (local.get $packed) (i64.const 32))))
    (if (i32.eqz (local.get $depth))
      (then
        (if (local.get $allow_empty) (then (return (i32.const 1))))
        (return (i32.const 2))))
    i32.const 0)

  ;; Literal copies for Rust storage headers.
  (data (i32.const 60243) "ERDB0001")
  (data (i32.const 60251) "ERLG")
