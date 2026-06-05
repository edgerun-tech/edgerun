(module
  (memory (export "memory") 1)

  (func (export "proto_abi_version") (result i32)
    i32.const 2)

  (func (export "proto_standard_id") (result i32)
    i32.const 300088)

  ;; Status: 0 ok, 2 output/input short, 3 invalid.
  ;; Packed i64 emit result: low u32 status, high u32 bytes_written.

  (func $pack (param $status i32) (param $written i32) (result i64)
    (i64.or
      (i64.extend_i32_u (local.get $status))
      (i64.shl (i64.extend_i32_u (local.get $written)) (i64.const 32))))

  (func $is_digit (param $c i32) (result i32)
    (i32.and
      (i32.ge_u (local.get $c) (i32.const 48))
      (i32.le_u (local.get $c) (i32.const 57))))

  (func $copy (param $src i32) (param $len i32) (param $dst i32)
    (local $i i32)
    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $i) (local.get $len)))
        (i32.store8
          (i32.add (local.get $dst) (local.get $i))
          (i32.load8_u (i32.add (local.get $src) (local.get $i))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop))))

  (func $sdk_app_slug_valid (export "sdk_app_slug_valid") (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (local $c i32)
    (local $prev_hyphen i32)
    (if (i32.or (i32.eqz (local.get $len)) (i32.gt_u (local.get $len) (i32.const 64)))
      (then (return (i32.const 3))))
    (local.set $prev_hyphen (i32.const 0))
    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $i) (local.get $len)))
        (local.set $c (i32.load8_u (i32.add (local.get $ptr) (local.get $i))))
        (if (i32.eq (local.get $c) (i32.const 45))
          (then
            (if
              (i32.or
                (i32.or (i32.eqz (local.get $i)) (i32.eq (local.get $i) (i32.sub (local.get $len) (i32.const 1))))
                (local.get $prev_hyphen))
              (then (return (i32.const 3))))
            (local.set $prev_hyphen (i32.const 1)))
          (else
            (if
              (i32.eqz
                (i32.or
                  (i32.and
                    (i32.ge_u (local.get $c) (i32.const 97))
                    (i32.le_u (local.get $c) (i32.const 122)))
                  (call $is_digit (local.get $c))))
              (then (return (i32.const 3))))
            (local.set $prev_hyphen (i32.const 0))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop)))
    i32.const 0)

  (func $sdk_app_version_valid (export "sdk_app_version_valid") (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (local $c i32)
    (local $part i32)
    (local $digits i32)
    (local $suffix i32)
    (local $suffix_len i32)
    (if (i32.eqz (local.get $len))
      (then (return (i32.const 3))))
    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $i) (local.get $len)))
        (local.set $c (i32.load8_u (i32.add (local.get $ptr) (local.get $i))))
        (block $advance
          (if (local.get $suffix)
            (then
              (if
                (i32.or
                  (i32.lt_u (local.get $c) (i32.const 33))
                  (i32.gt_u (local.get $c) (i32.const 126)))
                (then (return (i32.const 3))))
              (local.set $suffix_len (i32.add (local.get $suffix_len) (i32.const 1)))
              (br $advance)))
          (if (call $is_digit (local.get $c))
            (then
              (local.set $digits (i32.add (local.get $digits) (i32.const 1)))
              (br $advance)))
          (if (i32.eq (local.get $c) (i32.const 46))
            (then
              (if (i32.or (i32.eqz (local.get $digits)) (i32.ge_u (local.get $part) (i32.const 2)))
                (then (return (i32.const 3))))
              (local.set $part (i32.add (local.get $part) (i32.const 1)))
              (local.set $digits (i32.const 0))
              (br $advance)))
          (if (i32.eq (local.get $c) (i32.const 45))
            (then
              (if (i32.or (i32.ne (local.get $part) (i32.const 2)) (i32.eqz (local.get $digits)))
                (then (return (i32.const 3))))
              (local.set $suffix (i32.const 1))
              (br $advance)))
          (return (i32.const 3)))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop)))
    (if
      (i32.or
        (i32.ne (local.get $part) (i32.const 2))
        (i32.eqz (local.get $digits)))
      (then (return (i32.const 3))))
    (if (i32.and (local.get $suffix) (i32.eqz (local.get $suffix_len)))
      (then (return (i32.const 3))))
    i32.const 0)

  (func (export "sdk_app_manifest_preimage")
    (param $slug_ptr i32) (param $slug_len i32)
    (param $dev_ptr i32) (param $dev_len i32)
    (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (local $prefix_len i32)
    (local $need i32)
    (local.set $prefix_len (i32.const 12))
    (if (i32.ne (call $sdk_app_slug_valid (local.get $slug_ptr) (local.get $slug_len)) (i32.const 0))
      (then (return (call $pack (i32.const 3) (i32.const 0)))))
    (if (i32.ne (local.get $dev_len) (i32.const 32))
      (then (return (call $pack (i32.const 3) (i32.const 0)))))
    (local.set $need (i32.add (i32.add (local.get $prefix_len) (local.get $slug_len)) (i32.const 33)))
    (if (i32.lt_u (local.get $out_cap) (local.get $need))
      (then (return (call $pack (i32.const 2) (i32.const 0)))))
    (i64.store (local.get $out_ptr) (i64.const 3273683113332925541))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 8)) (i32.const 980447329))
    (call $copy (local.get $slug_ptr) (local.get $slug_len) (i32.add (local.get $out_ptr) (local.get $prefix_len)))
    (i32.store8 (i32.add (i32.add (local.get $out_ptr) (local.get $prefix_len)) (local.get $slug_len)) (i32.const 0))
    (call $copy
      (local.get $dev_ptr)
      (i32.const 32)
      (i32.add
        (i32.add
          (i32.add (local.get $out_ptr) (local.get $prefix_len))
          (local.get $slug_len))
        (i32.const 1)))
    (call $pack (i32.const 0) (local.get $need)))

  (func (export "sdk_release_preimage")
    (param $app_id_ptr i32) (param $app_id_len i32)
    (param $version_ptr i32) (param $version_len i32)
    (param $manifest_hash_ptr i32) (param $manifest_hash_len i32)
    (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (local $need i32)
    (if
      (i32.or
        (i32.ne (local.get $app_id_len) (i32.const 32))
        (i32.ne (local.get $manifest_hash_len) (i32.const 32)))
      (then (return (call $pack (i32.const 3) (i32.const 0)))))
    (if (i32.ne (call $sdk_app_version_valid (local.get $version_ptr) (local.get $version_len)) (i32.const 0))
      (then (return (call $pack (i32.const 3) (i32.const 0)))))
    (local.set $need (i32.add (i32.add (i32.const 66) (local.get $version_len)) (i32.const 0)))
    (if (i32.lt_u (local.get $out_cap) (local.get $need))
      (then (return (call $pack (i32.const 2) (i32.const 0)))))
    (call $copy (local.get $app_id_ptr) (i32.const 32) (local.get $out_ptr))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 32)) (i32.const 0))
    (call $copy (local.get $version_ptr) (local.get $version_len) (i32.add (local.get $out_ptr) (i32.const 33)))
    (i32.store8 (i32.add (i32.add (local.get $out_ptr) (i32.const 33)) (local.get $version_len)) (i32.const 0))
    (call $copy
      (local.get $manifest_hash_ptr)
      (i32.const 32)
      (i32.add (i32.add (local.get $out_ptr) (i32.const 34)) (local.get $version_len)))
    (call $pack (i32.const 0) (local.get $need)))
)
