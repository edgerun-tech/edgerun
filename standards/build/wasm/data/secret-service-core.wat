(module
  (import "edgerun-core" "memory" (memory 1))
  ;; Portable scalar model extracted from crates/apps/edgerun-secret-service/src.
  ;; Codes:
  ;; interface: 1 Service, 2 Collection, 3 Item, 4 Session, 5 Introspectable, 6 Properties.
  ;; action: 1 OpenSession, 2 CreateCollection, 3 SearchItems, 4 Unlock, 5 Lock,
  ;;         6 GetSecrets, 7 ReadAlias, 8 SetAlias, 9 ListItems, 10 CreateItem,
  ;;         11 DeleteCollection, 12 GetSecret, 13 DeleteItem, 14 CloseSession,
  ;;         15 Introspect, 16 PropertiesGetAll.
  ;; path: 1 service root, 2 collection, 3 item, 4 session, 5 prompt root.
  ;; status/error values are intentionally small integers for host ABI use.

  (data (i32.const 4096) "/org/freedesktop/secrets")
  (data (i32.const 4128) "/org/freedesktop/secrets/collections/")
  (data (i32.const 4176) "/org/freedesktop/secrets/session/")


  (func (export "proto_standard_id") (result i32)
    i32.const 300109)

  (func $fnv (param $ptr i32) (param $len i32) (result i32)
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
              (i32.load8_u (i32.add (local.get $ptr) (local.get $i))))
            (i32.const 0x01000193)))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $scan)))
    local.get $h)

  (func $m163mem_eq (param $a i32) (param $b i32) (param $len i32) (result i32)
    (local $i i32)
    (block $done
      (loop $scan
        (br_if $done (i32.ge_u (local.get $i) (local.get $len)))
        (if
          (i32.ne
            (i32.load8_u (i32.add (local.get $a) (local.get $i)))
            (i32.load8_u (i32.add (local.get $b) (local.get $i))))
          (then (return (i32.const 0))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $scan)))
    i32.const 1)

  (func $starts_with (param $ptr i32) (param $len i32) (param $prefix i32) (param $prefix_len i32) (result i32)
    (if (i32.lt_u (local.get $len) (local.get $prefix_len))
      (then (return (i32.const 0))))
    (call $m163mem_eq (local.get $ptr) (local.get $prefix) (local.get $prefix_len)))

  (func $is_component_byte (param $c i32) (result i32)
    (if
      (i32.and
        (i32.ge_u (local.get $c) (i32.const 48))
        (i32.le_u (local.get $c) (i32.const 57)))
      (then (return (i32.const 1))))
    (if
      (i32.and
        (i32.ge_u (local.get $c) (i32.const 65))
        (i32.le_u (local.get $c) (i32.const 90)))
      (then (return (i32.const 1))))
    (if
      (i32.and
        (i32.ge_u (local.get $c) (i32.const 97))
        (i32.le_u (local.get $c) (i32.const 122)))
      (then (return (i32.const 1))))
    (i32.eq (local.get $c) (i32.const 95)))

  (func $secret_component_id_valid (export "secret_component_id_valid") (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (if (i32.eqz (local.get $len)) (then (return (i32.const 0))))
    (block $done
      (loop $scan
        (br_if $done (i32.ge_u (local.get $i) (local.get $len)))
        (if
          (i32.eqz
            (call $is_component_byte
              (i32.load8_u (i32.add (local.get $ptr) (local.get $i)))))
          (then (return (i32.const 0))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $scan)))
    i32.const 1)

  (func $secret_session_id_valid (export "secret_session_id_valid") (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (local $c i32)
    (if (i32.lt_u (local.get $len) (i32.const 2)) (then (return (i32.const 0))))
    (if (i32.ne (i32.load8_u (local.get $ptr)) (i32.const 115)) (then (return (i32.const 0))))
    (local.set $i (i32.const 1))
    (block $done
      (loop $scan
        (br_if $done (i32.ge_u (local.get $i) (local.get $len)))
        (local.set $c (i32.load8_u (i32.add (local.get $ptr) (local.get $i))))
        (if
          (i32.or
            (i32.lt_u (local.get $c) (i32.const 48))
            (i32.gt_u (local.get $c) (i32.const 57)))
          (then (return (i32.const 0))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $scan)))
    i32.const 1)

  (func $slash_index (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (block $done
      (loop $scan
        (br_if $done (i32.ge_u (local.get $i) (local.get $len)))
        (if (i32.eq (i32.load8_u (i32.add (local.get $ptr) (local.get $i))) (i32.const 47))
          (then (return (local.get $i))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $scan)))
    i32.const -1)

  (func (export "secret_path_classify") (param $ptr i32) (param $len i32) (result i32)
    (local $rest_ptr i32)
    (local $rest_len i32)
    (local $slash i32)
    (if
      (i32.and
        (i32.eq (local.get $len) (i32.const 1))
        (i32.eq (i32.load8_u (local.get $ptr)) (i32.const 47)))
      (then (return (i32.const 5))))
    (if
      (i32.and
        (i32.eq (local.get $len) (i32.const 24))
        (call $m163mem_eq (local.get $ptr) (i32.const 4096) (i32.const 24)))
      (then (return (i32.const 1))))
    (if (call $starts_with (local.get $ptr) (local.get $len) (i32.const 4176) (i32.const 33))
      (then
        (local.set $rest_ptr (i32.add (local.get $ptr) (i32.const 33)))
        (local.set $rest_len (i32.sub (local.get $len) (i32.const 33)))
        (if (call $secret_session_id_valid (local.get $rest_ptr) (local.get $rest_len))
          (then (return (i32.const 4))))
        (return (i32.const 0))))
    (if (call $starts_with (local.get $ptr) (local.get $len) (i32.const 4128) (i32.const 37))
      (then
        (local.set $rest_ptr (i32.add (local.get $ptr) (i32.const 37)))
        (local.set $rest_len (i32.sub (local.get $len) (i32.const 37)))
        (local.set $slash (call $slash_index (local.get $rest_ptr) (local.get $rest_len)))
        (if (i32.eq (local.get $slash) (i32.const -1))
          (then
            (if (call $secret_component_id_valid (local.get $rest_ptr) (local.get $rest_len))
              (then (return (i32.const 2))))
            (return (i32.const 0))))
        (if
          (i32.and
            (call $secret_component_id_valid (local.get $rest_ptr) (local.get $slash))
            (call $secret_component_id_valid
              (i32.add (i32.add (local.get $rest_ptr) (local.get $slash)) (i32.const 1))
              (i32.sub (i32.sub (local.get $rest_len) (local.get $slash)) (i32.const 1))))
          (then (return (i32.const 3))))
        (return (i32.const 0))))
    i32.const 0)

  (func $secret_interface_code (export "secret_interface_code") (param $ptr i32) (param $len i32) (result i32)
    (local $h i32)
    (local.set $h (call $fnv (local.get $ptr) (local.get $len)))
    (if (i32.eq (local.get $h) (i32.const 257647820)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $h) (i32.const 316200729)) (then (return (i32.const 2))))
    (if (i32.eq (local.get $h) (i32.const 962541646)) (then (return (i32.const 3))))
    (if (i32.eq (local.get $h) (i32.const 4196407711)) (then (return (i32.const 4))))
    (if (i32.eq (local.get $h) (i32.const 101288616)) (then (return (i32.const 5))))
    (if (i32.eq (local.get $h) (i32.const 447584974)) (then (return (i32.const 6))))
    i32.const 0)

  (func $secret_member_code (export "secret_member_code") (param $iface i32) (param $ptr i32) (param $len i32) (result i32)
    (local $h i32)
    (local.set $h (call $fnv (local.get $ptr) (local.get $len)))
    (if (i32.eq (local.get $iface) (i32.const 1))
      (then
        (if (i32.eq (local.get $h) (i32.const 837357579)) (then (return (i32.const 1))))
        (if (i32.eq (local.get $h) (i32.const 774603337)) (then (return (i32.const 2))))
        (if (i32.eq (local.get $h) (i32.const 4265671683)) (then (return (i32.const 3))))
        (if (i32.eq (local.get $h) (i32.const 2625214837)) (then (return (i32.const 4))))
        (if (i32.eq (local.get $h) (i32.const 1907633506)) (then (return (i32.const 5))))
        (if (i32.eq (local.get $h) (i32.const 4180915764)) (then (return (i32.const 6))))
        (if (i32.eq (local.get $h) (i32.const 942667607)) (then (return (i32.const 7))))
        (if (i32.eq (local.get $h) (i32.const 3117815037)) (then (return (i32.const 8))))))
    (if (i32.eq (local.get $iface) (i32.const 2))
      (then
        (if (i32.eq (local.get $h) (i32.const 1906482251)) (then (return (i32.const 9))))
        (if (i32.eq (local.get $h) (i32.const 3880205310)) (then (return (i32.const 10))))
        (if (i32.eq (local.get $h) (i32.const 1469573738)) (then (return (i32.const 11))))))
    (if (i32.eq (local.get $iface) (i32.const 3))
      (then
        (if (i32.eq (local.get $h) (i32.const 1785012495)) (then (return (i32.const 12))))
        (if (i32.eq (local.get $h) (i32.const 1469573738)) (then (return (i32.const 13))))))
    (if (i32.eq (local.get $iface) (i32.const 4))
      (then
        (if (i32.eq (local.get $h) (i32.const 3448155331)) (then (return (i32.const 14))))))
    (if (i32.eq (local.get $iface) (i32.const 5))
      (then
        (if (i32.eq (local.get $h) (i32.const 546088306)) (then (return (i32.const 15))))))
    (if (i32.eq (local.get $iface) (i32.const 6))
      (then
        (if (i32.eq (local.get $h) (i32.const 3484971442)) (then (return (i32.const 16))))))
    i32.const 0)

  (func (export "secret_dbus_action_code")
    (param $iface_ptr i32)
    (param $iface_len i32)
    (param $member_ptr i32)
    (param $member_len i32)
    (result i32)
    (call $secret_member_code
      (call $secret_interface_code (local.get $iface_ptr) (local.get $iface_len))
      (local.get $member_ptr)
      (local.get $member_len)))

  (func (export "secret_action_valid_for_path") (param $path_class i32) (param $action i32) (result i32)
    (if (i32.eq (local.get $path_class) (i32.const 1))
      (then (return (i32.and (i32.ge_u (local.get $action) (i32.const 1)) (i32.le_u (local.get $action) (i32.const 8))))))
    (if (i32.eq (local.get $path_class) (i32.const 2))
      (then (return (i32.and (i32.ge_u (local.get $action) (i32.const 9)) (i32.le_u (local.get $action) (i32.const 11))))))
    (if (i32.eq (local.get $path_class) (i32.const 3))
      (then (return (i32.and (i32.ge_u (local.get $action) (i32.const 12)) (i32.le_u (local.get $action) (i32.const 13))))))
    (if (i32.eq (local.get $path_class) (i32.const 4))
      (then (return (i32.eq (local.get $action) (i32.const 14)))))
    i32.const 0)

  (func (export "secret_request_code_valid") (param $request i32) (result i32)
    (i32.and (i32.ge_u (local.get $request) (i32.const 1)) (i32.le_u (local.get $request) (i32.const 9))))

  (func (export "secret_response_code_valid") (param $response i32) (result i32)
    (i32.and (i32.ge_u (local.get $response) (i32.const 1)) (i32.le_u (local.get $response) (i32.const 7))))

  (func (export "secret_backend_item_status")
    (param $collection_exists i32)
    (param $index_entry_exists i32)
    (param $blob_exists i32)
    (param $decrypt_ok i32)
    (result i32)
    (if (i32.eqz (local.get $collection_exists)) (then (return (i32.const 1))))
    (if (i32.eqz (local.get $index_entry_exists)) (then (return (i32.const 2))))
    (if (i32.eqz (local.get $blob_exists)) (then (return (i32.const 3))))
    (if (i32.eqz (local.get $decrypt_ok)) (then (return (i32.const 4))))
    i32.const 0)

  (func (export "secret_session_access_status")
    (param $exists i32)
    (param $closed i32)
    (param $verified i32)
    (param $has_last_verified i32)
    (param $now_us i64)
    (param $last_verified_us i64)
    (param $idle_timeout_us i64)
    (result i32)
    (if (i32.eqz (local.get $exists)) (then (return (i32.const 1))))
    (if (local.get $closed) (then (return (i32.const 2))))
    (if (i32.eqz (local.get $verified)) (then (return (i32.const 3))))
    (if (i32.eqz (local.get $has_last_verified)) (then (return (i32.const 3))))
    (if (i64.gt_u (i64.sub (local.get $now_us) (local.get $last_verified_us)) (local.get $idle_timeout_us))
      (then (return (i32.const 4))))
    i32.const 0)

  (func (export "secret_unlock_status") (param $has_biometrics i32) (param $verified i32) (result i32)
    (if (i32.eqz (local.get $has_biometrics)) (then (return (i32.const 3))))
    (if (i32.eqz (local.get $verified)) (then (return (i32.const 3))))
    i32.const 0)

  (func (export "secret_lock_status") (param $session_exists i32) (param $closed i32) (result i32)
    (if (i32.eqz (local.get $session_exists)) (then (return (i32.const 1))))
    (if (local.get $closed) (then (return (i32.const 2))))
    i32.const 0)

  (func (export "secret_error_code") (param $ptr i32) (param $len i32) (result i32)
    (local $h i32)
    (local.set $h (call $fnv (local.get $ptr) (local.get $len)))
    (if (i32.eq (local.get $h) (i32.const 4122576587)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $h) (i32.const 2677189898)) (then (return (i32.const 2))))
    (if (i32.eq (local.get $h) (i32.const 3217075387)) (then (return (i32.const 3))))
    (if (i32.eq (local.get $h) (i32.const 3014949139)) (then (return (i32.const 4))))
    (if (i32.eq (local.get $h) (i32.const 6262436)) (then (return (i32.const 5))))
    (if (i32.eq (local.get $h) (i32.const 3542615378)) (then (return (i32.const 6))))
    i32.const 0)

)