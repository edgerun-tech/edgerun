(module
  (import "edgerun-core" "memory" (memory 1))
  (import "edgerun-core" "is_alnum" (func $is_alnum (param i32) (result i32)))

(func (export "proto_standard_id") (result i32)
    i32.const 300100)

  (func $m140ascii_lower (param $c i32) (result i32)
    (if
      (i32.and
        (i32.ge_u (local.get $c) (i32.const 65))
        (i32.le_u (local.get $c) (i32.const 90)))
      (then (return (i32.add (local.get $c) (i32.const 32)))))
    local.get $c)

  (func $m140fnv_lower (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (local $h i32)
    (local.set $h (i32.const 0x811c9dc5))
    (block $done
      (loop $scan
        (br_if $done (i32.ge_u (local.get $i) (local.get $len)))
        (local.set $h
          (i32.mul
            (i32.xor
              (local.get $h)
              (call $m140ascii_lower (i32.load8_u (i32.add (local.get $ptr) (local.get $i)))))
            (i32.const 0x01000193)))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $scan)))
    local.get $h)

  (func (export "oci_config_hash_lower") (param $ptr i32) (param $len i32) (result i32)
    (call $m140fnv_lower (local.get $ptr) (local.get $len)))

  (func (export "oci_namespace_kind") (param $ptr i32) (param $len i32) (result i32)
    (local $h i32)
    (local.set $h (call $m140fnv_lower (local.get $ptr) (local.get $len)))
    (if (i32.eq (local.get $h) (i32.const 1813408442)) (then (return (i32.const 1)))) ;; mount
    (if (i32.eq (local.get $h) (i32.const 1765866786)) (then (return (i32.const 2)))) ;; pid
    (if (i32.eq (local.get $h) (i32.const 1377339077)) (then (return (i32.const 3)))) ;; network
    (if (i32.eq (local.get $h) (i32.const 2835456301)) (then (return (i32.const 4)))) ;; ipc
    (if (i32.eq (local.get $h) (i32.const 1301541837)) (then (return (i32.const 5)))) ;; uts
    (if (i32.eq (local.get $h) (i32.const 1618501362)) (then (return (i32.const 6)))) ;; user
    (if (i32.eq (local.get $h) (i32.const 3410062693)) (then (return (i32.const 7)))) ;; cgroup
    (if (i32.eq (local.get $h) (i32.const 1564253156)) (then (return (i32.const 8)))) ;; time
    i32.const 0)

  (func (export "oci_namespace_flag") (param $kind i32) (result i32)
    (if (i32.eq (local.get $kind) (i32.const 1)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $kind) (i32.const 2)) (then (return (i32.const 2))))
    (if (i32.eq (local.get $kind) (i32.const 3)) (then (return (i32.const 4))))
    (if (i32.eq (local.get $kind) (i32.const 4)) (then (return (i32.const 8))))
    (if (i32.eq (local.get $kind) (i32.const 5)) (then (return (i32.const 16))))
    (if (i32.eq (local.get $kind) (i32.const 6)) (then (return (i32.const 32))))
    (if (i32.eq (local.get $kind) (i32.const 7)) (then (return (i32.const 64))))
    (if (i32.eq (local.get $kind) (i32.const 8)) (then (return (i32.const 128))))
    i32.const 0)

  (func (export "oci_default_namespace_mask") (result i32)
    i32.const 31)

  (func (export "oci_pull_policy") (param $ptr i32) (param $len i32) (result i32)
    (local $h i32)
    (local.set $h (call $m140fnv_lower (local.get $ptr) (local.get $len)))
    (if (i32.eq (local.get $h) (i32.const 3069444615)) (then (return (i32.const 1)))) ;; missing
    (if (i32.eq (local.get $h) (i32.const 647213027)) (then (return (i32.const 1)))) ;; if-missing
    (if (i32.eq (local.get $h) (i32.const 1731637220)) (then (return (i32.const 2)))) ;; always
    (if (i32.eq (local.get $h) (i32.const 180965513)) (then (return (i32.const 3)))) ;; never
    i32.const 0)

  (func (export "oci_mount_option_flag") (param $ptr i32) (param $len i32) (result i32)
    (local $h i32)
    (local.set $h (call $m140fnv_lower (local.get $ptr) (local.get $len)))
    (if (i32.eq (local.get $h) (i32.const 1649706254)) (then (return (i32.const 1)))) ;; ro
    (if (i32.eq (local.get $h) (i32.const 1247043398)) (then (return (i32.const 2)))) ;; rw
    (if (i32.eq (local.get $h) (i32.const 3344129838)) (then (return (i32.const 4)))) ;; bind
    (if (i32.eq (local.get $h) (i32.const 7593790)) (then (return (i32.const 12)))) ;; rbind
    (if (i32.eq (local.get $h) (i32.const 3348235767)) (then (return (i32.const 16)))) ;; nosuid
    (if (i32.eq (local.get $h) (i32.const 1313770401)) (then (return (i32.const 32)))) ;; nodev
    (if (i32.eq (local.get $h) (i32.const 80650471)) (then (return (i32.const 64)))) ;; noexec
    (if (i32.eq (local.get $h) (i32.const 3127169598)) (then (return (i32.const 128)))) ;; strictatime
    (if (i32.eq (local.get $h) (i32.const 2767733972)) (then (return (i32.const 256)))) ;; shared
    (if (i32.eq (local.get $h) (i32.const 3162545472)) (then (return (i32.const 512)))) ;; slave
    (if (i32.eq (local.get $h) (i32.const 1657474316)) (then (return (i32.const 1024)))) ;; private
    (if (i32.eq (local.get $h) (i32.const 2373037001)) (then (return (i32.const 2048)))) ;; unbindable
    i32.const 0)

  (func (export "oci_path_absolute") (param $ptr i32) (param $len i32) (result i32)
    (if (i32.eqz (local.get $len)) (then (return (i32.const 0))))
    (i32.eq (i32.load8_u (local.get $ptr)) (i32.const 47)))

  (func (export "oci_rootfs_relative_path_valid") (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (local $seg_start i32)
    (local $seg_len i32)
    (block $done
      (loop $scan
        (if (i32.ge_u (local.get $i) (local.get $len)) (then (br $done)))
        (if (i32.eq (i32.load8_u (i32.add (local.get $ptr) (local.get $i))) (i32.const 47))
          (then
            (local.set $seg_len (i32.sub (local.get $i) (local.get $seg_start)))
            (if
              (i32.and
                (i32.eq (local.get $seg_len) (i32.const 2))
                (i32.and
                  (i32.eq (i32.load8_u (i32.add (local.get $ptr) (local.get $seg_start))) (i32.const 46))
                  (i32.eq (i32.load8_u (i32.add (i32.add (local.get $ptr) (local.get $seg_start)) (i32.const 1))) (i32.const 46))))
              (then (return (i32.const 0))))
            (local.set $seg_start (i32.add (local.get $i) (i32.const 1)))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $scan)))
    (local.set $seg_len (i32.sub (local.get $len) (local.get $seg_start)))
    (if
      (i32.and
        (i32.eq (local.get $seg_len) (i32.const 2))
        (i32.and
          (i32.eq (i32.load8_u (i32.add (local.get $ptr) (local.get $seg_start))) (i32.const 46))
          (i32.eq (i32.load8_u (i32.add (i32.add (local.get $ptr) (local.get $seg_start)) (i32.const 1))) (i32.const 46))))
      (then (return (i32.const 0))))
    i32.const 1)

  (func (export "oci_container_id_valid") (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (local $c i32)
    (if (i32.eqz (local.get $len)) (then (return (i32.const 0))))
    (if (i32.gt_u (local.get $len) (i32.const 255)) (then (return (i32.const 0))))
    (local.set $c (i32.load8_u (local.get $ptr)))
    (if (i32.eqz (call $is_alnum (local.get $c))) (then (return (i32.const 0))))
    (local.set $i (i32.const 1))
    (block $done
      (loop $scan
        (br_if $done (i32.ge_u (local.get $i) (local.get $len)))
        (local.set $c (i32.load8_u (i32.add (local.get $ptr) (local.get $i))))
        (if
          (i32.eqz
            (i32.or
              (call $is_alnum (local.get $c))
              (i32.or
                (i32.eq (local.get $c) (i32.const 45))
                (i32.or
                  (i32.eq (local.get $c) (i32.const 95))
                  (i32.eq (local.get $c) (i32.const 46))))))
          (then (return (i32.const 0))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $scan)))
    i32.const 1)

  (func (export "oci_hostname_valid") (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (local $c i32)
    (if (i32.eqz (local.get $len)) (then (return (i32.const 0))))
    (if (i32.gt_u (local.get $len) (i32.const 253)) (then (return (i32.const 0))))
    (block $done
      (loop $scan
        (br_if $done (i32.ge_u (local.get $i) (local.get $len)))
        (local.set $c (i32.load8_u (i32.add (local.get $ptr) (local.get $i))))
        (if
          (i32.eqz
            (i32.or
              (call $is_alnum (local.get $c))
              (i32.or
                (i32.eq (local.get $c) (i32.const 45))
                (i32.eq (local.get $c) (i32.const 46)))))
          (then (return (i32.const 0))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $scan)))
    i32.const 1)

  (func (export "oci_dns_server_valid") (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (local $c i32)
    (if (i32.eqz (local.get $len)) (then (return (i32.const 0))))
    (if (i32.gt_u (local.get $len) (i32.const 255)) (then (return (i32.const 0))))
    (block $done
      (loop $scan
        (br_if $done (i32.ge_u (local.get $i) (local.get $len)))
        (local.set $c (i32.load8_u (i32.add (local.get $ptr) (local.get $i))))
        (if
          (i32.eqz
            (i32.or
              (i32.and (i32.ge_u (local.get $c) (i32.const 48)) (i32.le_u (local.get $c) (i32.const 57)))
              (i32.or
                (i32.and (i32.ge_u (call $m140ascii_lower (local.get $c)) (i32.const 97)) (i32.le_u (call $m140ascii_lower (local.get $c)) (i32.const 102)))
                (i32.or
                  (i32.eq (local.get $c) (i32.const 46))
                  (i32.eq (local.get $c) (i32.const 58))))))
          (then (return (i32.const 0))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $scan)))
    i32.const 1)
)
