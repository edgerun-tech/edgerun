  (func (export "proto_abi_version") (result i32)
    i32.const 2)

  (func (export "proto_standard_id") (result i32)
    i32.const 300090)

  ;; Status values: 0 ok, 1 invalid, 2 too large.
  ;; oci_reference_scan writes nine little-endian u32 slots:
  ;; registry_start,registry_len,repo_start,repo_len,tag_start,tag_len,digest_start,digest_len,kind.
  ;; kind: 1 tag, 2 digest, 3 both. Missing spans are written as zero length.

  (func $m142b (param $ptr i32) (param $off i32) (result i32)
    (i32.load8_u (i32.add (local.get $ptr) (local.get $off))))

  (func $is_lower_hex (param $c i32) (result i32)
    (i32.or
      (i32.and (i32.ge_u (local.get $c) (i32.const 48)) (i32.le_u (local.get $c) (i32.const 57)))
      (i32.and (i32.ge_u (local.get $c) (i32.const 97)) (i32.le_u (local.get $c) (i32.const 102)))))

  (func $is_tag_char (param $c i32) (result i32)
    (i32.or
      (i32.or
        (i32.and (i32.ge_u (local.get $c) (i32.const 48)) (i32.le_u (local.get $c) (i32.const 57)))
        (i32.and (i32.ge_u (local.get $c) (i32.const 65)) (i32.le_u (local.get $c) (i32.const 90))))
      (i32.or
        (i32.and (i32.ge_u (local.get $c) (i32.const 97)) (i32.le_u (local.get $c) (i32.const 122)))
        (i32.or
          (i32.eq (local.get $c) (i32.const 95))
          (i32.or (i32.eq (local.get $c) (i32.const 46)) (i32.eq (local.get $c) (i32.const 45)))))))

  (func $is_repo_char (param $c i32) (result i32)
    (i32.or
      (i32.and (i32.ge_u (local.get $c) (i32.const 48)) (i32.le_u (local.get $c) (i32.const 57)))
      (i32.or
        (i32.and (i32.ge_u (local.get $c) (i32.const 97)) (i32.le_u (local.get $c) (i32.const 122)))
        (i32.or
          (i32.eq (local.get $c) (i32.const 46))
          (i32.or (i32.eq (local.get $c) (i32.const 95)) (i32.eq (local.get $c) (i32.const 45)))))))

  (func $is_registry_char (param $c i32) (result i32)
    (i32.or
      (i32.or
        (i32.and (i32.ge_u (local.get $c) (i32.const 48)) (i32.le_u (local.get $c) (i32.const 57)))
        (i32.and (i32.ge_u (local.get $c) (i32.const 65)) (i32.le_u (local.get $c) (i32.const 90))))
      (i32.or
        (i32.and (i32.ge_u (local.get $c) (i32.const 97)) (i32.le_u (local.get $c) (i32.const 122)))
        (i32.or
          (i32.eq (local.get $c) (i32.const 46))
          (i32.or
            (i32.eq (local.get $c) (i32.const 45))
            (i32.or
              (i32.eq (local.get $c) (i32.const 58))
              (i32.or (i32.eq (local.get $c) (i32.const 91)) (i32.eq (local.get $c) (i32.const 93)))))))))

  (func $m142eq_lit (param $ptr i32) (param $len i32) (param $lit i32) (param $lit_len i32) (result i32)
    (local $i i32)
    (if (i32.ne (local.get $len) (local.get $lit_len)) (then (return (i32.const 0))))
    (block $done
      (loop $scan
        (br_if $done (i32.ge_u (local.get $i) (local.get $len)))
        (if (i32.ne
              (i32.load8_u (i32.add (local.get $ptr) (local.get $i)))
              (i32.load8_u (i32.add (local.get $lit) (local.get $i))))
          (then (return (i32.const 0))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $scan)))
    i32.const 1)

  (func $validate_tag (param $ptr i32) (param $start i32) (param $len i32) (result i32)
    (local $i i32)
    (local $c i32)
    (if (i32.or (i32.eqz (local.get $len)) (i32.gt_u (local.get $len) (i32.const 128)))
      (then (return (i32.const 0))))
    (local.set $c (call $m142b (local.get $ptr) (local.get $start)))
    (if (i32.eqz
          (i32.or
            (i32.eq (local.get $c) (i32.const 95))
            (i32.or
              (i32.and (i32.ge_u (local.get $c) (i32.const 48)) (i32.le_u (local.get $c) (i32.const 57)))
              (i32.or
                (i32.and (i32.ge_u (local.get $c) (i32.const 65)) (i32.le_u (local.get $c) (i32.const 90)))
                (i32.and (i32.ge_u (local.get $c) (i32.const 97)) (i32.le_u (local.get $c) (i32.const 122)))))))
      (then (return (i32.const 0))))
    (local.set $i (i32.const 0))
    (block $done
      (loop $scan
        (br_if $done (i32.ge_u (local.get $i) (local.get $len)))
        (if (i32.eqz (call $is_tag_char (call $m142b (local.get $ptr) (i32.add (local.get $start) (local.get $i)))))
          (then (return (i32.const 0))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $scan)))
    i32.const 1)

  (func $validate_registry (param $ptr i32) (param $start i32) (param $len i32) (result i32)
    (local $i i32)
    (local $c i32)
    (if (i32.eqz (local.get $len)) (then (return (i32.const 1))))
    (local.set $c (call $m142b (local.get $ptr) (local.get $start)))
    (if (i32.eq (local.get $c) (i32.const 46)) (then (return (i32.const 0))))
    (local.set $c (call $m142b (local.get $ptr) (i32.sub (i32.add (local.get $start) (local.get $len)) (i32.const 1))))
    (if (i32.eq (local.get $c) (i32.const 46)) (then (return (i32.const 0))))
    (block $done
      (loop $scan
        (br_if $done (i32.ge_u (local.get $i) (local.get $len)))
        (if (i32.eqz (call $is_registry_char (call $m142b (local.get $ptr) (i32.add (local.get $start) (local.get $i)))))
          (then (return (i32.const 0))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $scan)))
    i32.const 1)

  (func $validate_repo (param $ptr i32) (param $start i32) (param $len i32) (result i32)
    (local $i i32)
    (local $prev_slash i32)
    (local $c i32)
    (if (i32.eqz (local.get $len)) (then (return (i32.const 0))))
    (if (i32.eq (call $m142b (local.get $ptr) (local.get $start)) (i32.const 47)) (then (return (i32.const 0))))
    (if (i32.eq (call $m142b (local.get $ptr) (i32.sub (i32.add (local.get $start) (local.get $len)) (i32.const 1))) (i32.const 47))
      (then (return (i32.const 0))))
    (local.set $prev_slash (i32.const 0))
    (block $done
      (loop $scan
        (br_if $done (i32.ge_u (local.get $i) (local.get $len)))
        (local.set $c (call $m142b (local.get $ptr) (i32.add (local.get $start) (local.get $i))))
        (if (i32.eq (local.get $c) (i32.const 47))
          (then
            (if (local.get $prev_slash) (then (return (i32.const 0))))
            (local.set $prev_slash (i32.const 1)))
          (else
            (if (i32.eqz (call $is_repo_char (local.get $c))) (then (return (i32.const 0))))
            (local.set $prev_slash (i32.const 0))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $scan)))
    i32.const 1)

  (func $oci_digest_validate (export "oci_digest_validate") (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (local $c i32)
    (if (i32.ne (local.get $len) (i32.const 71)) (then (return (i32.const 1))))
    (if (i32.ne (i32.load8_u (local.get $ptr)) (i32.const 115)) (then (return (i32.const 1))))
    (if (i32.ne (i32.load8_u (i32.add (local.get $ptr) (i32.const 1))) (i32.const 104)) (then (return (i32.const 1))))
    (if (i32.ne (i32.load8_u (i32.add (local.get $ptr) (i32.const 2))) (i32.const 97)) (then (return (i32.const 1))))
    (if (i32.ne (i32.load8_u (i32.add (local.get $ptr) (i32.const 3))) (i32.const 50)) (then (return (i32.const 1))))
    (if (i32.ne (i32.load8_u (i32.add (local.get $ptr) (i32.const 4))) (i32.const 53)) (then (return (i32.const 1))))
    (if (i32.ne (i32.load8_u (i32.add (local.get $ptr) (i32.const 5))) (i32.const 54)) (then (return (i32.const 1))))
    (if (i32.ne (i32.load8_u (i32.add (local.get $ptr) (i32.const 6))) (i32.const 58)) (then (return (i32.const 1))))
    (local.set $i (i32.const 7))
    (block $done
      (loop $scan
        (br_if $done (i32.ge_u (local.get $i) (local.get $len)))
        (local.set $c (i32.load8_u (i32.add (local.get $ptr) (local.get $i))))
        (if (i32.eqz (call $is_lower_hex (local.get $c))) (then (return (i32.const 1))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $scan)))
    i32.const 0)

  (func $store_out
    (param $out i32) (param $rs i32) (param $rl i32) (param $ps i32) (param $pl i32)
    (param $ts i32) (param $tl i32) (param $ds i32) (param $dl i32) (param $kind i32)
    (i32.store (local.get $out) (local.get $rs))
    (i32.store (i32.add (local.get $out) (i32.const 4)) (local.get $rl))
    (i32.store (i32.add (local.get $out) (i32.const 8)) (local.get $ps))
    (i32.store (i32.add (local.get $out) (i32.const 12)) (local.get $pl))
    (i32.store (i32.add (local.get $out) (i32.const 16)) (local.get $ts))
    (i32.store (i32.add (local.get $out) (i32.const 20)) (local.get $tl))
    (i32.store (i32.add (local.get $out) (i32.const 24)) (local.get $ds))
    (i32.store (i32.add (local.get $out) (i32.const 28)) (local.get $dl))
    (i32.store (i32.add (local.get $out) (i32.const 32)) (local.get $kind)))

  (func (export "oci_reference_scan") (param $ptr i32) (param $len i32) (param $out i32) (result i32)
    (local $i i32)
    (local $first_slash i32)
    (local $last_slash i32)
    (local $last_colon i32)
    (local $at i32)
    (local $registry_start i32)
    (local $registry_len i32)
    (local $repo_start i32)
    (local $repo_end i32)
    (local $tag_start i32)
    (local $tag_len i32)
    (local $digest_start i32)
    (local $digest_len i32)
    (local $kind i32)
    (if (i32.eqz (local.get $len)) (then (return (i32.const 1))))
    (if (i32.gt_u (local.get $len) (i32.const 512)) (then (return (i32.const 2))))
    (call $store_out (local.get $out) (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 0))
    (local.set $first_slash (i32.const -1))
    (local.set $last_slash (i32.const -1))
    (local.set $last_colon (i32.const -1))
    (local.set $at (i32.const -1))
    (block $done
      (loop $scan
        (br_if $done (i32.ge_u (local.get $i) (local.get $len)))
        (if (i32.eq (call $m142b (local.get $ptr) (local.get $i)) (i32.const 47))
          (then
            (if (i32.eq (local.get $first_slash) (i32.const -1)) (then (local.set $first_slash (local.get $i))))
            (local.set $last_slash (local.get $i))))
        (if (i32.and
              (i32.eq (local.get $at) (i32.const -1))
              (i32.eq (call $m142b (local.get $ptr) (local.get $i)) (i32.const 58)))
          (then (local.set $last_colon (local.get $i))))
        (if (i32.eq (call $m142b (local.get $ptr) (local.get $i)) (i32.const 64))
          (then
            (if (i32.ne (local.get $at) (i32.const -1)) (then (return (i32.const 1))))
            (local.set $at (local.get $i))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $scan)))
    (if (i32.ne (local.get $at) (i32.const -1))
      (then
        (local.set $digest_start (i32.add (local.get $at) (i32.const 1)))
        (local.set $digest_len (i32.sub (local.get $len) (local.get $digest_start)))
        (if (call $oci_digest_validate (i32.add (local.get $ptr) (local.get $digest_start)) (local.get $digest_len))
          (then (return (i32.const 1))))
        (local.set $repo_end (local.get $at))
        (local.set $kind (i32.const 2)))
      (else
        (local.set $repo_end (local.get $len))))
    (if (i32.and
          (i32.and
            (i32.ne (local.get $last_colon) (i32.const -1))
            (i32.or
              (i32.eq (local.get $last_slash) (i32.const -1))
              (i32.gt_u (local.get $last_colon) (local.get $last_slash))))
          (i32.lt_u (local.get $last_colon) (local.get $repo_end)))
      (then
        (local.set $tag_start (i32.add (local.get $last_colon) (i32.const 1)))
        (local.set $tag_len (i32.sub (local.get $repo_end) (local.get $tag_start)))
        (if (i32.eqz (call $validate_tag (local.get $ptr) (local.get $tag_start) (local.get $tag_len)))
          (then (return (i32.const 1))))
        (local.set $repo_end (local.get $last_colon))
        (local.set $kind (i32.or (local.get $kind) (i32.const 1)))))
    (if (i32.eqz (local.get $kind)) (then (return (i32.const 1))))
    (if (i32.ne (local.get $first_slash) (i32.const -1))
      (then
        (local.set $registry_start (i32.const 0))
        (local.set $registry_len (local.get $first_slash))
        (local.set $repo_start (i32.add (local.get $first_slash) (i32.const 1))))
      (else
        (local.set $repo_start (i32.const 0))))
    (if (i32.eq (local.get $registry_len) (i32.const 0))
      (then (local.set $registry_start (i32.const 0))))
    (local.set $repo_end (local.get $repo_end))
    (if (i32.eqz (call $validate_registry (local.get $ptr) (local.get $registry_start) (local.get $registry_len)))
      (then (return (i32.const 1))))
    (if (i32.eqz (call $validate_repo (local.get $ptr) (local.get $repo_start) (i32.sub (local.get $repo_end) (local.get $repo_start))))
      (then (return (i32.const 1))))
    (call $store_out
      (local.get $out)
      (local.get $registry_start) (local.get $registry_len)
      (local.get $repo_start) (i32.sub (local.get $repo_end) (local.get $repo_start))
      (local.get $tag_start) (local.get $tag_len)
      (local.get $digest_start) (local.get $digest_len)
      (local.get $kind))
    i32.const 0)

  (func (export "oci_media_layer_compression") (param $ptr i32) (param $len i32) (result i32)
    (if (call $m142eq_lit (local.get $ptr) (local.get $len) (i32.const 4096) (i32.const 38)) (then (return (i32.const 0))))
    (if (call $m142eq_lit (local.get $ptr) (local.get $len) (i32.const 4160) (i32.const 55)) (then (return (i32.const 0))))
    (if (call $m142eq_lit (local.get $ptr) (local.get $len) (i32.const 4224) (i32.const 44)) (then (return (i32.const 0))))
    (if (call $m142eq_lit (local.get $ptr) (local.get $len) (i32.const 4288) (i32.const 43)) (then (return (i32.const 1))))
    (if (call $m142eq_lit (local.get $ptr) (local.get $len) (i32.const 4352) (i32.const 60)) (then (return (i32.const 1))))
    (if (call $m142eq_lit (local.get $ptr) (local.get $len) (i32.const 4416) (i32.const 49)) (then (return (i32.const 1))))
    (if (call $m142eq_lit (local.get $ptr) (local.get $len) (i32.const 4480) (i32.const 43)) (then (return (i32.const 2))))
    (if (call $m142eq_lit (local.get $ptr) (local.get $len) (i32.const 4544) (i32.const 60)) (then (return (i32.const 2))))
    i32.const 3)

  (data (i32.const 4096) "application/vnd.oci.image.layer.v1.tar")
  (data (i32.const 4160) "application/vnd.oci.image.layer.nondistributable.v1.tar")
  (data (i32.const 4224) "application/vnd.docker.image.rootfs.diff.tar")
  (data (i32.const 4288) "application/vnd.oci.image.layer.v1.tar+gzip")
  (data (i32.const 4352) "application/vnd.oci.image.layer.nondistributable.v1.tar+gzip")
  (data (i32.const 4416) "application/vnd.docker.image.rootfs.diff.tar.gzip")
  (data (i32.const 4480) "application/vnd.oci.image.layer.v1.tar+zstd")
  (data (i32.const 4544) "application/vnd.oci.image.layer.nondistributable.v1.tar+zstd")