(module
  (import "binary" "read_u16_be" (func $read_u16_be (param $ptr i32) (result i32)))
  (import "binary" "read_u24_be" (func $read_u24_be (param $ptr i32) (result i32)))
  (import "edgerun" "to_lower" (func $m180ascii_lower (param i32) (result i32)))
  (import "edgerun" "is_digit" (func $is_digit (param i32) (result i32)))
  (import "edgerun" "is_alnum" (func $is_alnum (param i32) (result i32)))
  (import "edgerun" "pack" (func $pack (param i32 i32) (result i64)))
  (memory (export "memory") 1)

;; Status values: 0 ok, 1 input_short, 2 output_short, 3 invalid, 4 overflow, 5 truncated.
  ;; Scan a TLS 1.3 Certificate handshake body.
  ;; Output record: context_offset:u32, context_len:u32, list_offset:u32, list_len:u32, next_offset:u32.
  (func (export "tls_certificate_list_scan") (param $ptr i32) (param $len i32) (param $out_ptr i32) (result i32)
    (local $context_len i32)
    (local $list_offset i32)
    (local $list_len i32)
    (if (i32.lt_u (local.get $len) (i32.const 4))
      (then (return (i32.const 1))))
    (local.set $context_len (i32.load8_u (local.get $ptr)))
    (if (i32.gt_u (local.get $context_len) (i32.sub (local.get $len) (i32.const 1)))
      (then (return (i32.const 5))))
    (if (i32.lt_u (i32.sub (local.get $len) (i32.add (i32.const 1) (local.get $context_len))) (i32.const 3))
      (then (return (i32.const 1))))
    (local.set $list_offset (i32.add (i32.const 4) (local.get $context_len)))
    (local.set $list_len (call $read_u24_be (i32.add (i32.add (local.get $ptr) (i32.const 1)) (local.get $context_len))))
    (if (i32.gt_u (local.get $list_len) (i32.sub (local.get $len) (local.get $list_offset)))
      (then (return (i32.const 5))))
    (if (i32.ne (i32.add (local.get $list_offset) (local.get $list_len)) (local.get $len))
      (then (return (i32.const 3))))
    (i32.store (local.get $out_ptr) (i32.const 1))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 4)) (local.get $context_len))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 8)) (local.get $list_offset))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 12)) (local.get $list_len))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 16)) (local.get $len))
    (i32.const 0))

  ;; Iterate CertificateEntry records inside the certificate_list vector payload.
  ;; Output record: cert_offset:u32, cert_len:u32, extensions_offset:u32, extensions_len:u32, next_offset:u32.
  (func (export "tls_certificate_entry_next") (param $ptr i32) (param $len i32) (param $start i32) (param $out_ptr i32) (result i32)
    (local $pos i32)
    (local $cert_len i32)
    (local $ext_offset i32)
    (local $ext_len i32)
    (local $next_offset i32)
    (if (i32.gt_u (local.get $start) (local.get $len))
      (then (return (i32.const 3))))
    (if (i32.eq (local.get $start) (local.get $len))
      (then (return (i32.const 1))))
    (if (i32.lt_u (i32.sub (local.get $len) (local.get $start)) (i32.const 5))
      (then (return (i32.const 1))))
    (local.set $pos (local.get $start))
    (local.set $cert_len (call $read_u24_be (i32.add (local.get $ptr) (local.get $pos))))
    (local.set $pos (i32.add (local.get $pos) (i32.const 3)))
    (if (i32.eqz (local.get $cert_len))
      (then (return (i32.const 3))))
    (if (i32.gt_u (local.get $cert_len) (i32.sub (local.get $len) (local.get $pos)))
      (then (return (i32.const 5))))
    (local.set $ext_offset (i32.add (local.get $pos) (local.get $cert_len)))
    (if (i32.lt_u (i32.sub (local.get $len) (local.get $ext_offset)) (i32.const 2))
      (then (return (i32.const 5))))
    (local.set $ext_len (call $read_u16_be (i32.add (local.get $ptr) (local.get $ext_offset))))
    (local.set $ext_offset (i32.add (local.get $ext_offset) (i32.const 2)))
    (if (i32.gt_u (local.get $ext_len) (i32.sub (local.get $len) (local.get $ext_offset)))
      (then (return (i32.const 5))))
    (local.set $next_offset (i32.add (local.get $ext_offset) (local.get $ext_len)))
    (i32.store (local.get $out_ptr) (local.get $pos))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 4)) (local.get $cert_len))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 8)) (local.get $ext_offset))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 12)) (local.get $ext_len))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 16)) (local.get $next_offset))
    (i32.const 0))


;; Status values: 0 ok, 1 input_short, 2 output_short, 3 invalid, 4 overflow, 5 truncated.
  ;; Output record:
  ;; legacy_version:u32, random_offset:u32, random_len:u32,
  ;; session_offset:u32, session_len:u32,
  ;; cipher_suites_offset:u32, cipher_suites_len:u32, cipher_suite_count:u32,
  ;; compression_offset:u32, compression_len:u32,
  ;; extensions_offset:u32, extensions_len:u32.
  (func (export "tls_clienthello_scan") (param $ptr i32) (param $len i32) (param $out_ptr i32) (result i32)
    (local $pos i32)
    (local $legacy_version i32)
    (local $session_len i32)
    (local $cipher_len i32)
    (local $cipher_offset i32)
    (local $comp_len i32)
    (local $ext_len i32)
    (if (i32.lt_u (local.get $len) (i32.const 34))
      (then (return (i32.const 1))))
    (local.set $legacy_version (call $read_u16_be (local.get $ptr)))
    (local.set $pos (i32.const 34))

    (if (i32.ge_u (local.get $pos) (local.get $len))
      (then (return (i32.const 1))))
    (local.set $session_len (i32.load8_u (i32.add (local.get $ptr) (local.get $pos))))
    (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
    (if (i32.gt_u (local.get $session_len) (i32.const 32))
      (then (return (i32.const 3))))
    (if (i32.gt_u (local.get $session_len) (i32.sub (local.get $len) (local.get $pos)))
      (then (return (i32.const 5))))
    (local.set $pos (i32.add (local.get $pos) (local.get $session_len)))

    (if (i32.lt_u (i32.sub (local.get $len) (local.get $pos)) (i32.const 2))
      (then (return (i32.const 1))))
    (local.set $cipher_len (call $read_u16_be (i32.add (local.get $ptr) (local.get $pos))))
    (local.set $pos (i32.add (local.get $pos) (i32.const 2)))
    (local.set $cipher_offset (local.get $pos))
    (if (i32.or (i32.eqz (local.get $cipher_len)) (i32.ne (i32.and (local.get $cipher_len) (i32.const 1)) (i32.const 0)))
      (then (return (i32.const 3))))
    (if (i32.gt_u (local.get $cipher_len) (i32.sub (local.get $len) (local.get $pos)))
      (then (return (i32.const 5))))
    (local.set $pos (i32.add (local.get $pos) (local.get $cipher_len)))

    (if (i32.ge_u (local.get $pos) (local.get $len))
      (then (return (i32.const 1))))
    (local.set $comp_len (i32.load8_u (i32.add (local.get $ptr) (local.get $pos))))
    (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
    (if (i32.eqz (local.get $comp_len))
      (then (return (i32.const 3))))
    (if (i32.gt_u (local.get $comp_len) (i32.sub (local.get $len) (local.get $pos)))
      (then (return (i32.const 5))))
    (local.set $pos (i32.add (local.get $pos) (local.get $comp_len)))

    (i32.store (local.get $out_ptr) (local.get $legacy_version))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 4)) (i32.const 2))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 8)) (i32.const 32))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 12)) (i32.const 35))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 16)) (local.get $session_len))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 20)) (local.get $cipher_offset))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 24)) (local.get $cipher_len))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 28)) (i32.shr_u (local.get $cipher_len) (i32.const 1)))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 32)) (i32.sub (local.get $pos) (local.get $comp_len)))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 36)) (local.get $comp_len))

    (if (i32.eq (local.get $pos) (local.get $len))
      (then
        (i32.store (i32.add (local.get $out_ptr) (i32.const 40)) (i32.const 0))
        (i32.store (i32.add (local.get $out_ptr) (i32.const 44)) (i32.const 0))
        (return (i32.const 0))))

    (if (i32.lt_u (i32.sub (local.get $len) (local.get $pos)) (i32.const 2))
      (then (return (i32.const 1))))
    (local.set $ext_len (call $read_u16_be (i32.add (local.get $ptr) (local.get $pos))))
    (local.set $pos (i32.add (local.get $pos) (i32.const 2)))
    (if (i32.gt_u (local.get $ext_len) (i32.sub (local.get $len) (local.get $pos)))
      (then (return (i32.const 5))))
    (if (i32.ne (i32.add (local.get $pos) (local.get $ext_len)) (local.get $len))
      (then (return (i32.const 3))))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 40)) (local.get $pos))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 44)) (local.get $ext_len))
    (i32.const 0))

  ;; Find an extension in an ExtensionList payload.
  ;; Output record: data_offset:u32, data_len:u32, next_offset:u32, extension_type:u32.
  (func (export "tls_clienthello_find_extension") (param $ptr i32) (param $len i32) (param $ext_type i32) (param $out_ptr i32) (result i32)
    (local $pos i32)
    (local $typ i32)
    (local $data_len i32)
    (loop $scan
      (if (i32.eq (local.get $pos) (local.get $len))
        (then (return (i32.const 3))))
      (if (i32.lt_u (i32.sub (local.get $len) (local.get $pos)) (i32.const 4))
        (then (return (i32.const 5))))
      (local.set $typ (call $read_u16_be (i32.add (local.get $ptr) (local.get $pos))))
      (local.set $data_len (call $read_u16_be (i32.add (i32.add (local.get $ptr) (local.get $pos)) (i32.const 2))))
      (local.set $pos (i32.add (local.get $pos) (i32.const 4)))
      (if (i32.gt_u (local.get $data_len) (i32.sub (local.get $len) (local.get $pos)))
        (then (return (i32.const 5))))
      (if (i32.eq (local.get $typ) (local.get $ext_type))
        (then
          (i32.store (local.get $out_ptr) (local.get $pos))
          (i32.store (i32.add (local.get $out_ptr) (i32.const 4)) (local.get $data_len))
          (i32.store (i32.add (local.get $out_ptr) (i32.const 8)) (i32.add (local.get $pos) (local.get $data_len)))
          (i32.store (i32.add (local.get $out_ptr) (i32.const 12)) (local.get $typ))
          (return (i32.const 0))))
      (local.set $pos (i32.add (local.get $pos) (local.get $data_len)))
      br $scan)
    (i32.const 3))

  ;; Extract the first host_name entry from an SNI extension payload.
  ;; Output record: host_offset:u32, host_len:u32.
  (func (export "tls_clienthello_sni_host") (param $ptr i32) (param $len i32) (param $out_ptr i32) (result i32)
    (local $list_len i32)
    (local $name_type i32)
    (local $name_len i32)
    (if (i32.lt_u (local.get $len) (i32.const 5))
      (then (return (i32.const 1))))
    (local.set $list_len (call $read_u16_be (local.get $ptr)))
    (if (i32.ne (i32.add (local.get $list_len) (i32.const 2)) (local.get $len))
      (then (return (i32.const 5))))
    (local.set $name_type (i32.load8_u (i32.add (local.get $ptr) (i32.const 2))))
    (if (i32.ne (local.get $name_type) (i32.const 0))
      (then (return (i32.const 3))))
    (local.set $name_len (call $read_u16_be (i32.add (local.get $ptr) (i32.const 3))))
    (if (i32.eqz (local.get $name_len))
      (then (return (i32.const 3))))
    (if (i32.ne (i32.add (local.get $name_len) (i32.const 5)) (local.get $len))
      (then (return (i32.const 5))))
    (i32.store (local.get $out_ptr) (i32.const 5))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 4)) (local.get $name_len))
    (i32.const 0))

  ;; Iterate ALPN protocol names from an ALPN extension payload.
  ;; start=0 validates the u16 protocol_name_list length and yields the first name.
  ;; Output record: proto_offset:u32, proto_len:u32, next_offset:u32, list_len:u32.
  (func (export "tls_clienthello_alpn_next") (param $ptr i32) (param $len i32) (param $start i32) (param $out_ptr i32) (result i32)
    (local $list_len i32)
    (local $pos i32)
    (local $proto_len i32)
    (if (i32.lt_u (local.get $len) (i32.const 2))
      (then (return (i32.const 1))))
    (local.set $list_len (call $read_u16_be (local.get $ptr)))
    (if (i32.ne (i32.add (local.get $list_len) (i32.const 2)) (local.get $len))
      (then (return (i32.const 5))))
    (if (i32.eqz (local.get $list_len))
      (then (return (i32.const 3))))
    (if (i32.eqz (local.get $start))
      (then (local.set $pos (i32.const 2)))
      (else
        (local.set $pos (local.get $start))
        (if (i32.or (i32.lt_u (local.get $pos) (i32.const 2)) (i32.ge_u (local.get $pos) (local.get $len)))
          (then (return (i32.const 3))))))
    (local.set $proto_len (i32.load8_u (i32.add (local.get $ptr) (local.get $pos))))
    (if (i32.eqz (local.get $proto_len))
      (then (return (i32.const 3))))
    (if (i32.gt_u (local.get $proto_len) (i32.sub (local.get $len) (i32.add (local.get $pos) (i32.const 1))))
      (then (return (i32.const 5))))
    (i32.store (local.get $out_ptr) (i32.add (local.get $pos) (i32.const 1)))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 4)) (local.get $proto_len))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 8)) (i32.add (i32.add (local.get $pos) (i32.const 1)) (local.get $proto_len)))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 12)) (local.get $list_len))
    (i32.const 0))


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


;; Status values: 0 ok, 1 input_short, 2 output_short, 3 invalid, 4 overflow, 5 truncated.
  ;; Walk a TLS ExtensionList body: repeated type:u16, len:u16, payload bytes.
  ;;
  ;; Output record:
  ;; extension_count:u32,
  ;; first_sni_data_offset:u32, first_sni_data_len:u32,
  ;; alpn_present:u32,
  ;; final_offset:u32.
  ;;
  ;; Offsets are relative to ptr. first_sni_data_offset/len are zero when SNI is absent.
  (func (export "tls_extension_walk") (param $ptr i32) (param $len i32) (param $out_ptr i32) (result i32)
    (local $pos i32)
    (local $remaining i32)
    (local $ext_type i32)
    (local $data_len i32)
    (local $data_offset i32)
    (local $next_offset i32)
    (local $count i32)
    (local $sni_offset i32)
    (local $sni_len i32)
    (local $alpn_present i32)

    (loop $scan
      (if (i32.eq (local.get $pos) (local.get $len))
        (then
          (i32.store (local.get $out_ptr) (local.get $count))
          (i32.store (i32.add (local.get $out_ptr) (i32.const 4)) (local.get $sni_offset))
          (i32.store (i32.add (local.get $out_ptr) (i32.const 8)) (local.get $sni_len))
          (i32.store (i32.add (local.get $out_ptr) (i32.const 12)) (local.get $alpn_present))
          (i32.store (i32.add (local.get $out_ptr) (i32.const 16)) (local.get $pos))
          (return (i32.const 0))))

      (local.set $remaining (i32.sub (local.get $len) (local.get $pos)))
      (if (i32.lt_u (local.get $remaining) (i32.const 4))
        (then (return (i32.const 5))))

      (local.set $ext_type (call $read_u16_be (i32.add (local.get $ptr) (local.get $pos))))
      (local.set $data_len (call $read_u16_be (i32.add (i32.add (local.get $ptr) (local.get $pos)) (i32.const 2))))
      (local.set $data_offset (i32.add (local.get $pos) (i32.const 4)))
      (if (i32.gt_u (local.get $data_len) (i32.sub (local.get $len) (local.get $data_offset)))
        (then (return (i32.const 5))))

      (local.set $next_offset (i32.add (local.get $data_offset) (local.get $data_len)))
      (local.set $count (i32.add (local.get $count) (i32.const 1)))

      ;; server_name extension type 0. Keep only the first SNI payload span.
      (if (i32.and
            (i32.eq (local.get $ext_type) (i32.const 0))
            (i32.eqz (local.get $sni_offset)))
        (then
          (local.set $sni_offset (local.get $data_offset))
          (local.set $sni_len (local.get $data_len))))

      ;; application_layer_protocol_negotiation extension type 16.
      (if (i32.eq (local.get $ext_type) (i32.const 16))
        (then (local.set $alpn_present (i32.const 1))))

      (local.set $pos (local.get $next_offset))
      br $scan)
    (i32.const 3))

;; Status values: 0 ok, 1 input_short, 2 output_short, 3 invalid, 4 overflow.
  (func $valid_record_content_type (param $content_type i32) (result i32)
    (i32.or
      (i32.or
        (i32.eq (local.get $content_type) (i32.const 20))
        (i32.eq (local.get $content_type) (i32.const 21)))
      (i32.or
        (i32.eq (local.get $content_type) (i32.const 22))
        (i32.eq (local.get $content_type) (i32.const 23)))))

  (func $valid_record_version (param $version i32) (result i32)
    (i32.and
      (i32.ge_u (local.get $version) (i32.const 768))
      (i32.le_u (local.get $version) (i32.const 772))))

  (func $valid_record_fragment_len (param $fragment_len i32) (result i32)
    (i32.le_u (local.get $fragment_len) (i32.const 16384)))
  ;;
  ;; tls_record_header_decode return bits:
  ;; low16=status, next8=content_type, next16=version, next16=fragment_len.
  (func (export "tls_record_header_decode") (param $in_ptr i32) (param $in_len i32) (result i64)
    (local $content_type i32)
    (local $version i32)
    (local $fragment_len i32)
    (if (i32.lt_u (local.get $in_len) (i32.const 5))
      (then (return (i64.const 1))))
    (local.set $content_type (i32.load8_u (local.get $in_ptr)))
    (local.set $version
      (i32.or
        (i32.shl (i32.load8_u (i32.add (local.get $in_ptr) (i32.const 1))) (i32.const 8))
        (i32.load8_u (i32.add (local.get $in_ptr) (i32.const 2)))))
    (local.set $fragment_len
      (i32.or
        (i32.shl (i32.load8_u (i32.add (local.get $in_ptr) (i32.const 3))) (i32.const 8))
        (i32.load8_u (i32.add (local.get $in_ptr) (i32.const 4)))))
    (if
      (i32.eqz
        (i32.and
          (i32.and
            (call $valid_record_content_type (local.get $content_type))
            (call $valid_record_version (local.get $version)))
          (call $valid_record_fragment_len (local.get $fragment_len))))
      (then (return (i64.const 3))))
    (i64.or
      (i64.or
        (i64.shl (i64.extend_i32_u (local.get $fragment_len)) (i64.const 40))
        (i64.shl (i64.extend_i32_u (local.get $version)) (i64.const 24)))
      (i64.shl (i64.extend_i32_u (local.get $content_type)) (i64.const 16))))

  ;; Return bits: low32=status, high32=written.
  (func (export "tls_record_header_encode")
    (param $content_type i32) (param $version i32) (param $fragment_len i32)
    (param $out_ptr i32) (param $out_cap i32) (result i64)
    (if (i32.lt_u (local.get $out_cap) (i32.const 5))
      (then (return (i64.const 2))))
    (if
      (i32.or
        (i32.or
          (i32.gt_u (local.get $content_type) (i32.const 255))
          (i32.gt_u (local.get $version) (i32.const 65535)))
        (i32.gt_u (local.get $fragment_len) (i32.const 65535)))
      (then (return (i64.const 4))))
    (if
      (i32.eqz
        (i32.and
          (i32.and
            (call $valid_record_content_type (local.get $content_type))
            (call $valid_record_version (local.get $version)))
          (call $valid_record_fragment_len (local.get $fragment_len))))
      (then (return (i64.const 3))))
    (i32.store8 (local.get $out_ptr) (local.get $content_type))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 1)) (i32.shr_u (local.get $version) (i32.const 8)))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 2)) (local.get $version))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 3)) (i32.shr_u (local.get $fragment_len) (i32.const 8)))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 4)) (local.get $fragment_len))
    (i64.const 21474836480))

  ;; tls_handshake_header_decode return bits:
  ;; low16=status, next8=handshake_type, next24=body_len.
  (func (export "tls_handshake_header_decode") (param $in_ptr i32) (param $in_len i32) (result i64)
    (local $handshake_type i32)
    (local $body_len i32)
    (if (i32.lt_u (local.get $in_len) (i32.const 4))
      (then (return (i64.const 1))))
    (local.set $handshake_type (i32.load8_u (local.get $in_ptr)))
    (local.set $body_len
      (i32.or
        (i32.or
          (i32.shl (i32.load8_u (i32.add (local.get $in_ptr) (i32.const 1))) (i32.const 16))
          (i32.shl (i32.load8_u (i32.add (local.get $in_ptr) (i32.const 2))) (i32.const 8)))
        (i32.load8_u (i32.add (local.get $in_ptr) (i32.const 3)))))
    (i64.or
      (i64.shl (i64.extend_i32_u (local.get $body_len)) (i64.const 24))
      (i64.shl (i64.extend_i32_u (local.get $handshake_type)) (i64.const 16))))

  ;; Return bits: low32=status, high32=written.
  (func (export "tls_handshake_header_encode")
    (param $handshake_type i32) (param $body_len i32)
    (param $out_ptr i32) (param $out_cap i32) (result i64)
    (if (i32.lt_u (local.get $out_cap) (i32.const 4))
      (then (return (i64.const 2))))
    (if
      (i32.or
        (i32.gt_u (local.get $handshake_type) (i32.const 255))
        (i32.gt_u (local.get $body_len) (i32.const 16777215)))
      (then (return (i64.const 4))))
    (i32.store8 (local.get $out_ptr) (local.get $handshake_type))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 1)) (i32.shr_u (local.get $body_len) (i32.const 16)))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 2)) (i32.shr_u (local.get $body_len) (i32.const 8)))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 3)) (local.get $body_len))
    (i64.const 17179869184))


;; Status values: 0 ok, 1 input_short, 2 output_short, 3 invalid, 4 overflow.

  (func $m183is_space (param $b i32) (result i32)
    (i32.or
      (i32.eq (local.get $b) (i32.const 32))
      (i32.and
        (i32.ge_u (local.get $b) (i32.const 9))
        (i32.le_u (local.get $b) (i32.const 13)))))

  (func $validate_normalized (param $ptr i32) (param $len i32) (param $allow_wildcard i32) (result i32)
    (local $i i32)
    (local $b i32)
    (local $label_len i32)
    (local $labels i32)
    (local $last i32)
    (local $all_digit_dot i32)
    (if (i32.or (i32.eqz (local.get $len)) (i32.gt_u (local.get $len) (i32.const 253)))
      (then (return (i32.const 0))))
    (local.set $all_digit_dot (i32.const 1))
    (loop $scan
      (if (i32.lt_u (local.get $i) (local.get $len))
        (then
          (local.set $b (i32.load8_u (i32.add (local.get $ptr) (local.get $i))))
          (if
            (i32.or
              (i32.or (i32.eqz (local.get $b)) (call $m183is_space (local.get $b)))
              (i32.eq (local.get $b) (i32.const 58)))
            (then (return (i32.const 0))))
          (if
            (i32.eqz
              (i32.or
                (call $is_digit (local.get $b))
                (i32.eq (local.get $b) (i32.const 46))))
            (then (local.set $all_digit_dot (i32.const 0))))
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
          (br $scan))))
    (if (local.get $all_digit_dot)
      (then (return (i32.const 0))))
    (local.set $i (i32.const 0))
    (local.set $label_len (i32.const 0))
    (loop $labels_loop
      (if (i32.lt_u (local.get $i) (local.get $len))
        (then
          (local.set $b (i32.load8_u (i32.add (local.get $ptr) (local.get $i))))
          (if (i32.eq (local.get $b) (i32.const 46))
            (then
              (if
                (i32.or
                  (i32.or (i32.eqz (local.get $label_len)) (i32.gt_u (local.get $label_len) (i32.const 63)))
                  (i32.eq (local.get $last) (i32.const 45)))
                (then (return (i32.const 0))))
              (local.set $labels (i32.add (local.get $labels) (i32.const 1)))
              (local.set $label_len (i32.const 0)))
            (else
              (if
                (i32.and
                  (local.get $allow_wildcard)
                  (i32.and
                    (i32.eqz (local.get $i))
                    (i32.eq (local.get $b) (i32.const 42))))
                (then
                  (if (i32.ne (local.get $len) (i32.const 1))
                    (then
                      (if (i32.ne (i32.load8_u (i32.add (local.get $ptr) (i32.const 1))) (i32.const 46))
                        (then (return (i32.const 0)))))
                    (else (return (i32.const 0))))
                  (local.set $label_len (i32.add (local.get $label_len) (i32.const 1))))
                (else
                  (if
                    (i32.eqz
                      (i32.or
                        (call $is_alnum (local.get $b))
                        (i32.eq (local.get $b) (i32.const 45))))
                    (then (return (i32.const 0))))
                  (if
                    (i32.and
                      (i32.eqz (local.get $label_len))
                      (i32.eq (local.get $b) (i32.const 45)))
                    (then (return (i32.const 0))))
                  (local.set $label_len (i32.add (local.get $label_len) (i32.const 1)))))
              (local.set $last (local.get $b))))
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
          (br $labels_loop))))
    (if
      (i32.or
        (i32.or (i32.eqz (local.get $label_len)) (i32.gt_u (local.get $label_len) (i32.const 63)))
        (i32.eq (local.get $last) (i32.const 45)))
      (then (return (i32.const 0))))
    (local.set $labels (i32.add (local.get $labels) (i32.const 1)))
    (i32.ge_u (local.get $labels) (i32.const 2)))

  (func $validate_label (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (local $b i32)
    (if (i32.or (i32.eqz (local.get $len)) (i32.gt_u (local.get $len) (i32.const 63)))
      (then (return (i32.const 0))))
    (loop $scan
      (if (i32.lt_u (local.get $i) (local.get $len))
        (then
          (local.set $b (i32.load8_u (i32.add (local.get $ptr) (local.get $i))))
          (if
            (i32.eqz
              (i32.or
                (call $is_alnum (local.get $b))
                (i32.eq (local.get $b) (i32.const 45))))
            (then (return (i32.const 0))))
          (if
            (i32.and
              (i32.eqz (local.get $i))
              (i32.eq (local.get $b) (i32.const 45)))
            (then (return (i32.const 0))))
          (if
            (i32.and
              (i32.eq (i32.add (local.get $i) (i32.const 1)) (local.get $len))
              (i32.eq (local.get $b) (i32.const 45)))
            (then (return (i32.const 0))))
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
          (br $scan))))
    (i32.const 1))

  ;; Return bits: low32=status, high32=written. Writes lowercase normalized DNS name.
  (func $tls_dns_name_normalize (export "tls_dns_name_normalize")
    (param $input_ptr i32) (param $input_len i32) (param $out_ptr i32) (param $out_cap i32) (result i64)
    (local $start i32)
    (local $end i32)
    (local $written i32)
    (local $b i32)
    (local $valid i32)
    (local.set $start (i32.const 0))
    (local.set $end (local.get $input_len))
    (loop $trim_start
      (if
        (i32.and
          (i32.lt_u (local.get $start) (local.get $end))
          (call $m183is_space (i32.load8_u (i32.add (local.get $input_ptr) (local.get $start)))))
        (then
          (local.set $start (i32.add (local.get $start) (i32.const 1)))
          (br $trim_start))))
    (loop $trim_end
      (if
        (i32.and
          (i32.gt_u (local.get $end) (local.get $start))
          (call $m183is_space (i32.load8_u (i32.add (local.get $input_ptr) (i32.sub (local.get $end) (i32.const 1))))))
        (then
          (local.set $end (i32.sub (local.get $end) (i32.const 1)))
          (br $trim_end))))
    (loop $trim_dot
      (if
        (i32.and
          (i32.gt_u (local.get $end) (local.get $start))
          (i32.eq (i32.load8_u (i32.add (local.get $input_ptr) (i32.sub (local.get $end) (i32.const 1)))) (i32.const 46)))
        (then
          (local.set $end (i32.sub (local.get $end) (i32.const 1)))
          (br $trim_dot))))
    (if (i32.lt_u (local.get $out_cap) (i32.sub (local.get $end) (local.get $start)))
      (then (return (call $pack (i32.const 2) (i32.const 0)))))
    (loop $copy
      (if (i32.lt_u (i32.add (local.get $start) (local.get $written)) (local.get $end))
        (then
          (local.set $b
            (call $m180ascii_lower
              (i32.load8_u
                (i32.add
                  (local.get $input_ptr)
                  (i32.add (local.get $start) (local.get $written))))))
          (i32.store8 (i32.add (local.get $out_ptr) (local.get $written)) (local.get $b))
          (local.set $written (i32.add (local.get $written) (i32.const 1)))
          (br $copy))))
    (local.set $valid (call $validate_normalized (local.get $out_ptr) (local.get $written) (i32.const 0)))
    (if (i32.eqz (local.get $valid))
      (then (return (call $pack (i32.const 3) (i32.const 0)))))
    (call $pack (i32.const 0) (local.get $written)))

  (func $normalized_match (param $a_ptr i32) (param $a_len i32) (param $b_ptr i32) (param $b_len i32) (result i32)
    (local $i i32)
    (if (i32.ne (local.get $a_len) (local.get $b_len))
      (then (return (i32.const 0))))
    (loop $cmp
      (if (i32.lt_u (local.get $i) (local.get $a_len))
        (then
          (if
            (i32.ne
              (i32.load8_u (i32.add (local.get $a_ptr) (local.get $i)))
              (i32.load8_u (i32.add (local.get $b_ptr) (local.get $i))))
            (then (return (i32.const 0))))
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
          (br $cmp))))
    (i32.const 1))

  (func $strip_and_lower_pattern (param $ptr i32) (param $len i32) (param $out_ptr i32) (result i32)
    (local $start i32)
    (local $end i32)
    (local $written i32)
    (local.set $end (local.get $len))
    (loop $trim_start
      (if
        (i32.and
          (i32.lt_u (local.get $start) (local.get $end))
          (call $m183is_space (i32.load8_u (i32.add (local.get $ptr) (local.get $start)))))
        (then
          (local.set $start (i32.add (local.get $start) (i32.const 1)))
          (br $trim_start))))
    (loop $trim_end
      (if
        (i32.and
          (i32.gt_u (local.get $end) (local.get $start))
          (call $m183is_space (i32.load8_u (i32.add (local.get $ptr) (i32.sub (local.get $end) (i32.const 1))))))
        (then
          (local.set $end (i32.sub (local.get $end) (i32.const 1)))
          (br $trim_end))))
    (loop $trim_dot
      (if
        (i32.and
          (i32.gt_u (local.get $end) (local.get $start))
          (i32.eq (i32.load8_u (i32.add (local.get $ptr) (i32.sub (local.get $end) (i32.const 1)))) (i32.const 46)))
        (then
          (local.set $end (i32.sub (local.get $end) (i32.const 1)))
          (br $trim_dot))))
    (loop $copy
      (if (i32.lt_u (i32.add (local.get $start) (local.get $written)) (local.get $end))
        (then
          (i32.store8
            (i32.add (local.get $out_ptr) (local.get $written))
            (call $m180ascii_lower
              (i32.load8_u
                (i32.add
                  (local.get $ptr)
                  (i32.add (local.get $start) (local.get $written))))))
          (local.set $written (i32.add (local.get $written) (i32.const 1)))
          (br $copy))))
    (local.get $written))

  ;; Scratch areas 4096..4351 for host normalization and 4352..4607 for pattern normalization.
  ;; Return 0 for no match/invalid, 1 for exact or single-label wildcard match.
  (func (export "tls_dns_name_matches")
    (param $pattern_ptr i32) (param $pattern_len i32) (param $host_ptr i32) (param $host_len i32) (result i32)
    (local $host_pack i64)
    (local $host_status i32)
    (local $host_norm_len i32)
    (local $pattern_norm_len i32)
    (local $suffix_ptr i32)
    (local $suffix_len i32)
    (local $prefix_len i32)
    (local $i i32)
    (local $b i32)
    (local.set $host_pack
      (call $tls_dns_name_normalize
        (local.get $host_ptr)
        (local.get $host_len)
        (i32.const 4096)
        (i32.const 256)))
    (local.set $host_status (i32.wrap_i64 (local.get $host_pack)))
    (if (i32.ne (local.get $host_status) (i32.const 0))
      (then (return (i32.const 0))))
    (local.set $host_norm_len
      (i32.wrap_i64
        (i64.shr_u (local.get $host_pack) (i64.const 32))))
    (local.set $pattern_norm_len
      (call $strip_and_lower_pattern
        (local.get $pattern_ptr)
        (local.get $pattern_len)
        (i32.const 4352)))
    (if (call $normalized_match (i32.const 4352) (local.get $pattern_norm_len) (i32.const 4096) (local.get $host_norm_len))
      (then (return (i32.const 1))))
    (if
      (i32.or
        (i32.lt_u (local.get $pattern_norm_len) (i32.const 3))
        (i32.or
          (i32.ne (i32.load8_u (i32.const 4352)) (i32.const 42))
          (i32.ne (i32.load8_u (i32.const 4353)) (i32.const 46))))
      (then (return (i32.const 0))))
    (local.set $suffix_ptr (i32.const 4354))
    (local.set $suffix_len (i32.sub (local.get $pattern_norm_len) (i32.const 2)))
    (if (i32.eqz (call $validate_normalized (local.get $suffix_ptr) (local.get $suffix_len) (i32.const 0)))
      (then (return (i32.const 0))))
    (if (i32.le_u (local.get $host_norm_len) (local.get $suffix_len))
      (then (return (i32.const 0))))
    (local.set $prefix_len (i32.sub (i32.sub (local.get $host_norm_len) (local.get $suffix_len)) (i32.const 1)))
    (if (i32.eqz (local.get $prefix_len))
      (then (return (i32.const 0))))
    (if (i32.ne (i32.load8_u (i32.add (i32.const 4096) (local.get $prefix_len))) (i32.const 46))
      (then (return (i32.const 0))))
    (if (i32.eqz (call $normalized_match (local.get $suffix_ptr) (local.get $suffix_len) (i32.add (i32.const 4096) (i32.add (local.get $prefix_len) (i32.const 1))) (local.get $suffix_len)))
      (then (return (i32.const 0))))
    (loop $prefix_scan
      (if (i32.lt_u (local.get $i) (local.get $prefix_len))
        (then
          (local.set $b (i32.load8_u (i32.add (i32.const 4096) (local.get $i))))
          (if (i32.eq (local.get $b) (i32.const 46))
            (then (return (i32.const 0))))
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
          (br $prefix_scan))))
    (call $validate_label (i32.const 4096) (local.get $prefix_len)))


;; Status values: 0 ok, 1 input_short, 2 output_short, 3 invalid, 4 overflow, 5 truncated.
  (func $store_span3 (param $out_ptr i32) (param $data_offset i32) (param $data_len i32) (param $next_offset i32)
    (i32.store (local.get $out_ptr) (local.get $data_offset))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 4)) (local.get $data_len))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 8)) (local.get $next_offset)))

  ;; Output record: data_offset:u32, data_len:u32, next_offset:u32.
  (func (export "tls_vector_u8_decode") (param $ptr i32) (param $len i32) (param $out_ptr i32) (result i32)
    (local $data_len i32)
    (if (i32.lt_u (local.get $len) (i32.const 1))
      (then (return (i32.const 1))))
    (local.set $data_len (i32.load8_u (local.get $ptr)))
    (if (i32.gt_u (local.get $data_len) (i32.sub (local.get $len) (i32.const 1)))
      (then (return (i32.const 5))))
    (call $store_span3
      (local.get $out_ptr)
      (i32.const 1)
      (local.get $data_len)
      (i32.add (i32.const 1) (local.get $data_len)))
    (i32.const 0))

  ;; Output record: data_offset:u32, data_len:u32, next_offset:u32.
  (func (export "tls_vector_u16_decode") (param $ptr i32) (param $len i32) (param $out_ptr i32) (result i32)
    (local $data_len i32)
    (if (i32.lt_u (local.get $len) (i32.const 2))
      (then (return (i32.const 1))))
    (local.set $data_len (call $read_u16_be (local.get $ptr)))
    (if (i32.gt_u (local.get $data_len) (i32.sub (local.get $len) (i32.const 2)))
      (then (return (i32.const 5))))
    (call $store_span3
      (local.get $out_ptr)
      (i32.const 2)
      (local.get $data_len)
      (i32.add (i32.const 2) (local.get $data_len)))
    (i32.const 0))

  ;; Output record: data_offset:u32, data_len:u32, next_offset:u32.
  (func (export "tls_vector_u24_decode") (param $ptr i32) (param $len i32) (param $out_ptr i32) (result i32)
    (local $data_len i32)
    (if (i32.lt_u (local.get $len) (i32.const 3))
      (then (return (i32.const 1))))
    (local.set $data_len (call $read_u24_be (local.get $ptr)))
    (if (i32.gt_u (local.get $data_len) (i32.sub (local.get $len) (i32.const 3)))
      (then (return (i32.const 5))))
    (call $store_span3
      (local.get $out_ptr)
      (i32.const 3)
      (local.get $data_len)
      (i32.add (i32.const 3) (local.get $data_len)))
    (i32.const 0))

  ;; Iterate a TLS ExtensionList payload.
  ;; Output record: extension_type:u32, data_offset:u32, data_len:u32, next_offset:u32.
  (func (export "tls_extension_next") (param $ptr i32) (param $len i32) (param $start i32) (param $out_ptr i32) (result i32)
    (local $ext_type i32)
    (local $data_len i32)
    (local $data_offset i32)
    (local $next_offset i32)
    (if (i32.gt_u (local.get $start) (local.get $len))
      (then (return (i32.const 3))))
    (if (i32.lt_u (i32.sub (local.get $len) (local.get $start)) (i32.const 4))
      (then (return (i32.const 1))))
    (local.set $ext_type (call $read_u16_be (i32.add (local.get $ptr) (local.get $start))))
    (local.set $data_len (call $read_u16_be (i32.add (i32.add (local.get $ptr) (local.get $start)) (i32.const 2))))
    (local.set $data_offset (i32.add (local.get $start) (i32.const 4)))
    (if (i32.gt_u (local.get $data_len) (i32.sub (local.get $len) (local.get $data_offset)))
      (then (return (i32.const 5))))
    (local.set $next_offset (i32.add (local.get $data_offset) (local.get $data_len)))
    (i32.store (local.get $out_ptr) (local.get $ext_type))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 4)) (local.get $data_offset))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 8)) (local.get $data_len))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 12)) (local.get $next_offset))
    (i32.const 0))

  ;; Iterate an RFC 7301 ALPN extension payload.
  ;; start=0 validates the u16 protocol_name_list length and yields the first name.
  ;; For subsequent calls, pass the previous next_offset.
  ;; Output record: proto_offset:u32, proto_len:u32, next_offset:u32, list_len:u32.
  (func (export "tls_alpn_next") (param $ptr i32) (param $len i32) (param $start i32) (param $out_ptr i32) (result i32)
    (local $list_len i32)
    (local $pos i32)
    (local $proto_len i32)
    (local $next_offset i32)
    (if (i32.lt_u (local.get $len) (i32.const 2))
      (then (return (i32.const 1))))
    (local.set $list_len (call $read_u16_be (local.get $ptr)))
    (if (i32.ne (i32.add (local.get $list_len) (i32.const 2)) (local.get $len))
      (then (return (i32.const 5))))
    (if (i32.eqz (local.get $list_len))
      (then (return (i32.const 3))))
    (if (i32.eqz (local.get $start))
      (then (local.set $pos (i32.const 2)))
      (else
        (local.set $pos (local.get $start))
        (if (i32.or (i32.lt_u (local.get $pos) (i32.const 2)) (i32.ge_u (local.get $pos) (local.get $len)))
          (then (return (i32.const 3))))))
    (local.set $proto_len (i32.load8_u (i32.add (local.get $ptr) (local.get $pos))))
    (if (i32.eqz (local.get $proto_len))
      (then (return (i32.const 3))))
    (if (i32.gt_u (local.get $proto_len) (i32.sub (local.get $len) (i32.add (local.get $pos) (i32.const 1))))
      (then (return (i32.const 5))))
    (local.set $next_offset (i32.add (i32.add (local.get $pos) (i32.const 1)) (local.get $proto_len)))
    (i32.store (local.get $out_ptr) (i32.add (local.get $pos) (i32.const 1)))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 4)) (local.get $proto_len))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 8)) (local.get $next_offset))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 12)) (local.get $list_len))
    (i32.const 0))

)
