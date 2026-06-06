  (import "edgerun" "to_lower" (func $m180ascii_lower (param i32) (result i32)))

(func $m180fnv_lower (param $ptr i32) (param $len i32) (result i32)
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
              (call $m180ascii_lower (i32.load8_u (i32.add (local.get $ptr) (local.get $i)))))
            (i32.const 0x01000193)))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $scan)))
    local.get $h)

  (func (export "tls_hash_lower") (param $ptr i32) (param $len i32) (result i32)
    (call $m180fnv_lower (local.get $ptr) (local.get $len)))

  (func $tls_record_content_type (export "tls_record_content_type") (param $id i32) (result i32)
    (if (i32.eq (local.get $id) (i32.const 20)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $id) (i32.const 21)) (then (return (i32.const 2))))
    (if (i32.eq (local.get $id) (i32.const 22)) (then (return (i32.const 3))))
    (if (i32.eq (local.get $id) (i32.const 23)) (then (return (i32.const 4))))
    i32.const 0)

  (func (export "tls_record_header_status")
    (param $content_type i32)
    (param $version i32)
    (param $fragment_len i32)
    (param $available_len i32)
    (result i32)
    (if (i32.eqz (call $tls_record_content_type (local.get $content_type))) (then (return (i32.const 1))))
    (if (i32.ne (local.get $version) (i32.const 0x0303)) (then (return (i32.const 2))))
    (if (i32.gt_u (local.get $fragment_len) (i32.const 16384)) (then (return (i32.const 3))))
    (if (i32.lt_u (local.get $available_len) (i32.add (local.get $fragment_len) (i32.const 5))) (then (return (i32.const 4))))
    i32.const 0)

  (func $tls_alert_level (export "tls_alert_level") (param $id i32) (result i32)
    (if (i32.eq (local.get $id) (i32.const 1)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $id) (i32.const 2)) (then (return (i32.const 2))))
    i32.const 0)

  (func $tls_alert_description (export "tls_alert_description") (param $id i32) (result i32)
    (if (i32.eq (local.get $id) (i32.const 0)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $id) (i32.const 40)) (then (return (i32.const 2))))
    (if (i32.and (i32.ge_u (local.get $id) (i32.const 42)) (i32.le_u (local.get $id) (i32.const 51))) (then (return (i32.const 3))))
    (if (i32.eq (local.get $id) (i32.const 70)) (then (return (i32.const 4))))
    (if (i32.eq (local.get $id) (i32.const 71)) (then (return (i32.const 5))))
    (if (i32.eq (local.get $id) (i32.const 80)) (then (return (i32.const 6))))
    (if (i32.eq (local.get $id) (i32.const 86)) (then (return (i32.const 7))))
    (if (i32.eq (local.get $id) (i32.const 90)) (then (return (i32.const 8))))
    (if (i32.and (i32.ge_u (local.get $id) (i32.const 109)) (i32.le_u (local.get $id) (i32.const 120))) (then (return (i32.const 9))))
    i32.const 0)

  (func (export "tls_alert_message_status") (param $level i32) (param $description i32) (param $len i32) (result i32)
    (if (i32.lt_u (local.get $len) (i32.const 2)) (then (return (i32.const 1))))
    (if (i32.eqz (call $tls_alert_level (local.get $level))) (then (return (i32.const 2))))
    (if (i32.eqz (call $tls_alert_description (local.get $description))) (then (return (i32.const 3))))
    i32.const 0)

  (func (export "tls_handshake_type") (param $id i32) (result i32)
    (if (i32.eq (local.get $id) (i32.const 1)) (then (return (i32.const 1)))) ;; ClientHello
    (if (i32.eq (local.get $id) (i32.const 2)) (then (return (i32.const 2)))) ;; ServerHello
    (if (i32.eq (local.get $id) (i32.const 4)) (then (return (i32.const 3)))) ;; NewSessionTicket
    (if (i32.eq (local.get $id) (i32.const 8)) (then (return (i32.const 4)))) ;; EncryptedExtensions
    (if (i32.eq (local.get $id) (i32.const 11)) (then (return (i32.const 5)))) ;; Certificate
    (if (i32.eq (local.get $id) (i32.const 15)) (then (return (i32.const 6)))) ;; CertificateVerify
    (if (i32.eq (local.get $id) (i32.const 20)) (then (return (i32.const 7)))) ;; Finished
    i32.const 0)

  (func (export "tls_extension_type") (param $id i32) (result i32)
    (if (i32.eq (local.get $id) (i32.const 0)) (then (return (i32.const 1)))) ;; server_name
    (if (i32.eq (local.get $id) (i32.const 10)) (then (return (i32.const 2)))) ;; groups
    (if (i32.eq (local.get $id) (i32.const 13)) (then (return (i32.const 3)))) ;; sig algs
    (if (i32.eq (local.get $id) (i32.const 16)) (then (return (i32.const 4)))) ;; alpn
    (if (i32.eq (local.get $id) (i32.const 41)) (then (return (i32.const 5)))) ;; psk
    (if (i32.eq (local.get $id) (i32.const 43)) (then (return (i32.const 6)))) ;; supported versions
    (if (i32.eq (local.get $id) (i32.const 44)) (then (return (i32.const 7)))) ;; cookie
    (if (i32.eq (local.get $id) (i32.const 45)) (then (return (i32.const 8)))) ;; psk modes
    (if (i32.eq (local.get $id) (i32.const 51)) (then (return (i32.const 9)))) ;; key share
    i32.const 0)

  (func (export "tls_named_group") (param $id i32) (result i32)
    (if (i32.eq (local.get $id) (i32.const 23)) (then (return (i32.const 1)))) ;; secp256r1
    (if (i32.eq (local.get $id) (i32.const 24)) (then (return (i32.const 2)))) ;; secp384r1
    (if (i32.eq (local.get $id) (i32.const 29)) (then (return (i32.const 3)))) ;; x25519
    (if (i32.eq (local.get $id) (i32.const 30)) (then (return (i32.const 4)))) ;; x448
    i32.const 0)

  (func (export "tls_hkdf_label") (param $ptr i32) (param $len i32) (result i32)
    (local $h i32)
    (local.set $h (call $m180fnv_lower (local.get $ptr) (local.get $len)))
    (if (i32.eq (local.get $h) (i32.const 1746258028)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $h) (i32.const 1228441398)) (then (return (i32.const 2))))
    (if (i32.eq (local.get $h) (i32.const 35917078)) (then (return (i32.const 3))))
    (if (i32.eq (local.get $h) (i32.const 1003379576)) (then (return (i32.const 4))))
    (if (i32.eq (local.get $h) (i32.const 1347746370)) (then (return (i32.const 5))))
    (if (i32.eq (local.get $h) (i32.const 4053860466)) (then (return (i32.const 6))))
    (if (i32.eq (local.get $h) (i32.const 4031646414)) (then (return (i32.const 7))))
    (if (i32.eq (local.get $h) (i32.const 551008766)) (then (return (i32.const 8))))
    (if (i32.eq (local.get $h) (i32.const 1001159214)) (then (return (i32.const 9))))
    (if (i32.eq (local.get $h) (i32.const 3589666249)) (then (return (i32.const 10))))
    i32.const 0)

  (func (export "tls_alpn_protocol") (param $ptr i32) (param $len i32) (result i32)
    (local $h i32)
    (local.set $h (call $m180fnv_lower (local.get $ptr) (local.get $len)))
    (if (i32.eq (local.get $h) (i32.const 2386244204)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $h) (i32.const 3079932075)) (then (return (i32.const 2))))
    i32.const 0)

  (func (export "tls_cert_list_status")
    (param $context_len i32)
    (param $list_len i32)
    (param $available_len i32)
    (result i32)
    (local $start i32)
    (local.set $start (i32.add (i32.const 4) (local.get $context_len)))
    (if (i32.lt_u (local.get $available_len) (local.get $start)) (then (return (i32.const 1))))
    (if (i32.lt_u (local.get $available_len) (i32.add (local.get $start) (local.get $list_len))) (then (return (i32.const 2))))
    i32.const 0)

  (func (export "tls_cert_valid_at") (param $not_before i64) (param $not_after i64) (param $now i64) (result i32)
    (i32.and
      (i64.ge_u (local.get $now) (local.get $not_before))
      (i64.le_u (local.get $now) (local.get $not_after))))

  (func (export "tls_hostname_wildcard_shape") (param $ptr i32) (param $len i32) (result i32)
    (if (i32.lt_u (local.get $len) (i32.const 3)) (then (return (i32.const 0))))
    (if (i32.ne (i32.load8_u (local.get $ptr)) (i32.const 42)) (then (return (i32.const 0))))
    (if (i32.ne (i32.load8_u (i32.add (local.get $ptr) (i32.const 1))) (i32.const 46)) (then (return (i32.const 0))))
    i32.const 1)
