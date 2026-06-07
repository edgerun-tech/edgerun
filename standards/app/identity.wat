;; Identity hardware semantics plundered from edgerun-hardware-signing,
  ;; edgerun-tpm, and edgerun-yubikey.

  (func (export "identity_mesh_public_key_len") (result i32)
    i32.const 64)

  (func (export "identity_mesh_signature_len") (result i32)
    i32.const 64)

  (func (export "identity_algorithm_is_mesh") (param $algorithm i32) (result i32)
    (i32.eq (local.get $algorithm) (i32.const 3))) ;; EcdsaP256Sha256

  (func (export "identity_assurance_strength") (param $level i32) (result i32)
    ;; Unknown=0, Software=1, IsolatedHardware=2, StrongBox=3, Certified=4
    (if (result i32) (i32.le_u (local.get $level) (i32.const 4))
      (then local.get $level)
      (else i32.const 0)))

  (func (export "identity_assurance_at_least") (param $actual i32) (param $minimum i32) (result i32)
    (i32.ge_u (call $identity_assurance_strength_impl (local.get $actual)) (local.get $minimum)))

  (func $identity_assurance_strength_impl (param $level i32) (result i32)
    (if (result i32) (i32.le_u (local.get $level) (i32.const 4))
      (then local.get $level)
      (else i32.const 0)))

  (func (export "identity_node_id_result") (param $algorithm i32) (param $public_key_len i32) (result i32)
    (if (i32.ne (local.get $algorithm) (i32.const 3)) (then (return (i32.const 1))))
    (if (i32.ne (local.get $public_key_len) (i32.const 64)) (then (return (i32.const 2))))
    i32.const 0)

  (func (export "identity_validate_key_info")
    (param $provider_allowed i32) (param $algorithm_allowed i32)
    (param $has_min_assurance i32) (param $assurance_ok i32)
    (param $require_attestation i32) (param $attestation_len i32)
    (param $require_public_key i32) (param $public_key_len i32)
    (param $has_min_biometric i32) (param $biometric_ok i32)
    (result i32)
    (if (i32.eqz (local.get $provider_allowed)) (then (return (i32.const 1))))
    (if (i32.eqz (local.get $algorithm_allowed)) (then (return (i32.const 2))))
    (if (i32.and (local.get $has_min_assurance) (i32.eqz (local.get $assurance_ok))) (then (return (i32.const 3))))
    (if (i32.and (local.get $require_attestation) (i32.eqz (local.get $attestation_len))) (then (return (i32.const 4))))
    (if (i32.and (local.get $require_public_key) (i32.eqz (local.get $public_key_len))) (then (return (i32.const 5))))
    (if (i32.and (local.get $has_min_biometric) (i32.eqz (local.get $biometric_ok))) (then (return (i32.const 6))))
    i32.const 0)

  (func (export "identity_signature_input_len") (param $domain_len i32) (param $record_hash_len i32) (result i32)
    ;; domain + separator/length overhead is delegated to protocol signature_input;
    ;; this keeps the caller-visible invariant: input grows with both fields.
    (i32.add (i32.add (local.get $domain_len) (local.get $record_hash_len)) (i32.const 1)))

  (func (export "identity_provider_code") (param $provider i32) (result i32)
    (if (result i32) (i32.and (i32.ge_u (local.get $provider) (i32.const 1)) (i32.le_u (local.get $provider) (i32.const 3)))
      (then local.get $provider) ;; 1 TPM, 2 Android, 3 YubiKey
      (else i32.const 0)))

  (func (export "identity_map_tpm_algorithm") (param $algorithm i32) (result i32)
    (if (result i32) (i32.and (i32.ge_u (local.get $algorithm) (i32.const 1)) (i32.le_u (local.get $algorithm) (i32.const 6)))
      (then local.get $algorithm)
      (else i32.const 7))) ;; opaque

  (func (export "identity_map_tpm_assurance") (param $level i32) (result i32)
    (if (result i32) (i32.eq (local.get $level) (i32.const 1))
      (then i32.const 1) ;; software simulated
      (else
        (if (result i32) (i32.or (i32.eq (local.get $level) (i32.const 2)) (i32.eq (local.get $level) (i32.const 3)))
          (then i32.const 2) ;; discrete/integrated TPM
          (else
            (if (result i32) (i32.eq (local.get $level) (i32.const 4))
              (then i32.const 4) ;; certified
              (else i32.const 0)))))))

  (func (export "identity_map_yubikey_assurance") (param $level i32) (result i32)
    (if (result i32) (i32.eq (local.get $level) (i32.const 1))
      (then i32.const 1) ;; simulator
      (else
        (if (result i32) (i32.or (i32.eq (local.get $level) (i32.const 2)) (i32.eq (local.get $level) (i32.const 3)))
          (then i32.const 2) ;; hardware/PIV attested
          (else
            (if (result i32) (i32.or (i32.eq (local.get $level) (i32.const 4)) (i32.eq (local.get $level) (i32.const 5)))
              (then i32.const 4) ;; FIPS/certified
              (else i32.const 0)))))))

  (func (export "identity_yubikey_default_algorithm_allowed_count") (param $explicit_allowed_count i32) (result i32)
    (if (result i32) (i32.eqz (local.get $explicit_allowed_count))
      (then i32.const 5)
      (else local.get $explicit_allowed_count)))

  (func (export "identity_yubikey_hardware_algorithm_result") (param $hardware_algorithm i32) (result i32)
    (if (i32.eq (local.get $hardware_algorithm) (i32.const 5)) (then (return (i32.const -1)))) ;; EcSchnorr unsupported
    (if (result i32) (i32.and (i32.ge_u (local.get $hardware_algorithm) (i32.const 1)) (i32.le_u (local.get $hardware_algorithm) (i32.const 6)))
      (then local.get $hardware_algorithm)
      (else i32.const 7)))

  (func (export "identity_tpm_command_code") (param $which i32) (result i32)
    (if (result i32) (i32.eq (local.get $which) (i32.const 1))
      (then i32.const 0x15d) ;; Sign
      (else
        (if (result i32) (i32.eq (local.get $which) (i32.const 2))
          (then i32.const 0x173) ;; ReadPublic
          (else
            (if (result i32) (i32.eq (local.get $which) (i32.const 3))
              (then i32.const 0x17d) ;; Hash
              (else
                (if (result i32) (i32.eq (local.get $which) (i32.const 4))
                  (then i32.const 0x17b) ;; GetRandom
                  (else
                    (if (result i32) (i32.eq (local.get $which) (i32.const 5))
                      (then i32.const 0x177) ;; VerifySignature
                      (else
                        (if (result i32) (i32.eq (local.get $which) (i32.const 6))
                          (then i32.const 0x176) ;; StartAuthSession
                          (else i32.const 0)))))))))))))

  (func (export "identity_tpm_name_algorithm") (param $alg i32) (result i32)
    (if (result i32) (i32.eq (local.get $alg) (i32.const 1))
      (then i32.const 0x0004)
      (else
        (if (result i32) (i32.eq (local.get $alg) (i32.const 2))
          (then i32.const 0x000b)
          (else
            (if (result i32) (i32.eq (local.get $alg) (i32.const 3))
              (then i32.const 0x000c)
              (else
                (if (result i32) (i32.eq (local.get $alg) (i32.const 4))
                  (then i32.const 0x000d)
                  (else i32.const 0x0010)))))))))

  (func (export "identity_tpm_map_public_type") (param $wire i32) (result i32)
    (if (result i32) (i32.eq (local.get $wire) (i32.const 0x0001))
      (then i32.const 1)
      (else
        (if (result i32) (i32.eq (local.get $wire) (i32.const 0x0023))
          (then i32.const 2)
          (else
            (if (result i32) (i32.eq (local.get $wire) (i32.const 0x0008))
              (then i32.const 3)
              (else
                (if (result i32) (i32.eq (local.get $wire) (i32.const 0x0025))
                  (then i32.const 4)
                  (else i32.const 0)))))))))

  (func (export "identity_tpm_map_ecc_curve") (param $wire i32) (result i32)
    (if (result i32) (i32.eq (local.get $wire) (i32.const 0x0003))
      (then i32.const 1)
      (else
        (if (result i32) (i32.eq (local.get $wire) (i32.const 0x0004))
          (then i32.const 2)
          (else
            (if (result i32) (i32.eq (local.get $wire) (i32.const 0x0040))
              (then i32.const 3)
              (else i32.const 0)))))))

  (func (export "identity_tpm_auth_command_len") (param $nonce_len i32) (param $hmac_len i32) (result i32)
    (i32.add (i32.const 9) (i32.add (local.get $nonce_len) (local.get $hmac_len))))

  (func (export "identity_tpm_auth_value_area_len") (param $auth_value_len i32) (result i32)
    (i32.add (i32.const 9) (local.get $auth_value_len)))

  (func (export "identity_tpm_sign_command_len") (param $digest_len i32) (param $ticket_digest_len i32) (param $auth_area_len i32) (result i32)
    (local $params_len i32)
    (local.set $params_len (i32.add (i32.const 18) (i32.add (local.get $digest_len) (local.get $ticket_digest_len))))
    (if (result i32) (i32.eqz (local.get $auth_area_len))
      (then (i32.add (i32.const 10) (local.get $params_len)))
      (else (i32.add (i32.const 14) (i32.add (local.get $auth_area_len) (local.get $params_len))))))

  (func (export "identity_tpm_hash_command_len") (param $data_len i32) (result i32)
    (i32.add (i32.const 18) (local.get $data_len))) ;; header + TPM2B(data) + alg + hierarchy

  (func (export "identity_tpm_fixed_command_len") (param $which i32) (result i32)
    (if (result i32) (i32.eq (local.get $which) (i32.const 1))
      (then i32.const 12) ;; GetRandom / Startup
      (else
        (if (result i32) (i32.eq (local.get $which) (i32.const 2))
          (then i32.const 14) ;; ReadPublic
          (else i32.const 0)))))

  (func (export "identity_tpm_response_header_result") (param $response_len i32) (param $header_size i32) (param $rc i32) (result i32)
    (if (i32.lt_u (local.get $response_len) (i32.const 10)) (then (return (i32.const 1))))
    (if (i32.ne (local.get $header_size) (local.get $response_len)) (then (return (i32.const 2))))
    (if (i32.ne (local.get $rc) (i32.const 0)) (then (return (i32.const 3))))
    i32.const 0)

  (func (export "identity_tpm_parse_sign_result")
    (param $header_result i32) (param $scheme i32) (param $trailing_bytes i32)
    (result i32)
    (if (i32.ne (local.get $header_result) (i32.const 0)) (then (return (local.get $header_result))))
    (if (local.get $trailing_bytes) (then (return (i32.const 4))))
    (if (result i32)
      (i32.or (i32.or (i32.eq (local.get $scheme) (i32.const 0x0014)) (i32.eq (local.get $scheme) (i32.const 0x0016)))
              (i32.or (i32.or (i32.eq (local.get $scheme) (i32.const 0x0018)) (i32.eq (local.get $scheme) (i32.const 0x001a))) (i32.eq (local.get $scheme) (i32.const 0x001c))))
      (then i32.const 0)
      (else i32.const 5))) ;; opaque

  (func (export "identity_tpm_public_area_result")
    (param $object_type i32) (param $unique_len i32) (param $curve i32)
    (result i32)
    (if (result i32) (i32.eq (local.get $object_type) (i32.const 0x0023))
      (then
        (if (result i32) (i32.and (i32.eq (local.get $curve) (i32.const 0x0003)) (i32.eq (local.get $unique_len) (i32.const 64)))
          (then i32.const 1) ;; ECC P-256 key
          (else i32.const 2)))
      (else
        (if (result i32) (i32.eq (local.get $object_type) (i32.const 0x0001))
          (then i32.const 3) ;; RSA key
          (else i32.const 0)))))

  (func (export "identity_tpm_strip_ecdsa_result") (param $sig_len i32) (param $r_size i32) (param $s_size i32) (param $s_end_matches i32) (result i32)
    (if (i32.lt_u (local.get $sig_len) (i32.const 8)) (then (return (i32.const -1))))
    (if (i32.eqz (local.get $s_end_matches)) (then (return (i32.const -2))))
    (i32.add (local.get $r_size) (local.get $s_size)))

  (func (export "identity_yubikey_product_known") (param $pid i32) (result i32)
    (if (result i32)
      (i32.or (i32.or (i32.eq (local.get $pid) (i32.const 0x0407)) (i32.eq (local.get $pid) (i32.const 0x0406)))
              (i32.or (i32.eq (local.get $pid) (i32.const 0x0410)) (i32.eq (local.get $pid) (i32.const 0x0405))))
      (then i32.const 1)
      (else i32.const 0)))

  (func (export "identity_yubikey_slot_key_ref") (param $slot i32) (result i32)
    (if (result i32) (i32.eq (local.get $slot) (i32.const 1))
      (then i32.const 0x9a)
      (else
        (if (result i32) (i32.eq (local.get $slot) (i32.const 2))
          (then i32.const 0x9c)
          (else
            (if (result i32) (i32.eq (local.get $slot) (i32.const 3))
              (then i32.const 0x9d)
              (else
                (if (result i32) (i32.eq (local.get $slot) (i32.const 4))
                  (then i32.const 0x9e)
                  (else i32.const 0)))))))))

  (func (export "identity_yubikey_apdu_len") (param $kind i32) (param $payload_len i32) (result i32)
    (if (result i32) (i32.eq (local.get $kind) (i32.const 1))
      (then (i32.add (i32.const 5) (local.get $payload_len))) ;; SELECT
      (else
        (if (result i32) (i32.eq (local.get $kind) (i32.const 2))
          (then i32.const 4) ;; metadata/attestation
          (else
            (if (result i32) (i32.eq (local.get $kind) (i32.const 3))
              (then (i32.add (i32.const 7) (local.get $payload_len))) ;; GET DATA tag
              (else
                (if (result i32) (i32.eq (local.get $kind) (i32.const 4))
                  (then i32.const 13) ;; VERIFY PIN
                  (else i32.const 0)))))))))

  (func (export "identity_yubikey_pin_result") (param $pin_len i32) (result i32)
    (if (result i32) (i32.and (i32.ge_u (local.get $pin_len) (i32.const 6)) (i32.le_u (local.get $pin_len) (i32.const 8)))
      (then i32.const 0)
      (else i32.const 1)))

  (func (export "identity_yubikey_piv_algorithm_id") (param $algorithm i32) (result i32)
    (if (result i32) (i32.or (i32.eq (local.get $algorithm) (i32.const 1)) (i32.eq (local.get $algorithm) (i32.const 2)))
      (then i32.const 0x07)
      (else
        (if (result i32) (i32.eq (local.get $algorithm) (i32.const 3))
          (then i32.const 0x11)
          (else
            (if (result i32) (i32.eq (local.get $algorithm) (i32.const 4))
              (then i32.const 0x14)
              (else i32.const -1)))))))

  (func (export "identity_yubikey_digest_len") (param $algorithm i32) (result i32)
    (if (result i32) (i32.or (i32.or (i32.eq (local.get $algorithm) (i32.const 1)) (i32.eq (local.get $algorithm) (i32.const 2))) (i32.eq (local.get $algorithm) (i32.const 3)))
      (then i32.const 32)
      (else
        (if (result i32) (i32.eq (local.get $algorithm) (i32.const 4))
          (then i32.const 48)
          (else i32.const -1)))))

  (func (export "identity_yubikey_general_auth_apdu_len") (param $digest_len i32) (result i32)
    ;; 5-byte APDU header + outer TLV around witness TLV(2 bytes) and challenge TLV(2+digest)
    (i32.add (i32.const 11) (local.get $digest_len)))

  (func (export "identity_yubikey_parse_apdu_response") (param $len i32) (param $status_word i32) (result i32)
    (if (i32.lt_u (local.get $len) (i32.const 2)) (then (return (i32.const 1))))
    (if (result i32) (i32.eq (local.get $status_word) (i32.const 0x9000))
      (then i32.const 0)
      (else i32.const 2)))

  (func (export "identity_yubikey_parse_version") (param $len i32) (result i32)
    (if (result i32) (i32.ge_u (local.get $len) (i32.const 3))
      (then i32.const 0)
      (else i32.const 1)))

  (func (export "identity_yubikey_parse_piv_algorithm") (param $id i32) (result i32)
    (if (result i32)
      (i32.or (i32.or (i32.eq (local.get $id) (i32.const 0x06)) (i32.eq (local.get $id) (i32.const 0x07)))
              (i32.or (i32.eq (local.get $id) (i32.const 0x05)) (i32.eq (local.get $id) (i32.const 0x16))))
      (then i32.const 1)
      (else
        (if (result i32) (i32.eq (local.get $id) (i32.const 0x11))
          (then i32.const 3)
          (else
            (if (result i32) (i32.eq (local.get $id) (i32.const 0x14))
              (then i32.const 4)
              (else i32.const 0))))))
)



  ;; Status: 0 ok, 2 output/input short, 3 invalid.
  ;; Packed i64 emit result: low u32 status, high u32 bytes_written.

  (func $m161copy (param $src i32) (param $len i32) (param $dst i32)
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
    (call $m161copy (local.get $slug_ptr) (local.get $slug_len) (i32.add (local.get $out_ptr) (local.get $prefix_len)))
    (i32.store8 (i32.add (i32.add (local.get $out_ptr) (local.get $prefix_len)) (local.get $slug_len)) (i32.const 0))
    (call $m161copy
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
    (call $m161copy (local.get $app_id_ptr) (i32.const 32) (local.get $out_ptr))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 32)) (i32.const 0))
    (call $m161copy (local.get $version_ptr) (local.get $version_len) (i32.add (local.get $out_ptr) (i32.const 33)))
    (i32.store8 (i32.add (i32.add (local.get $out_ptr) (i32.const 33)) (local.get $version_len)) (i32.const 0))
    (call $m161copy
      (local.get $manifest_hash_ptr)
      (i32.const 32)
      (i32.add (i32.add (local.get $out_ptr) (i32.const 34)) (local.get $version_len)))
    (call $pack (i32.const 0) (local.get $need)))


  ;; Status: 0 pass/found/ok, 1 not found, 2 output short or reject,
  ;; 3 invalid input. Packed i64: low u32 status, high u32 bytes_written.


  (func $m162is_lower (param $c i32) (result i32)
    (i32.and
      (i32.ge_u (local.get $c) (i32.const 97))
      (i32.le_u (local.get $c) (i32.const 122))))

  (func $is_id_char (param $c i32) (result i32)
    (i32.or
      (i32.or (call $m162is_lower (local.get $c)) (call $is_digit (local.get $c)))
      (i32.eq (local.get $c) (i32.const 45))))

  (func $m162copy (param $src i32) (param $len i32) (param $dst i32)
    (local $i i32)
    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $i) (local.get $len)))
        (i32.store8
          (i32.add (local.get $dst) (local.get $i))
          (i32.load8_u (i32.add (local.get $src) (local.get $i))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop))))

  (func $unit_index (param $ptr i32) (param $len i32) (result i32)
    (if (call $string_eq (local.get $ptr) (local.get $len) (i32.const 32800) (i32.const 23))
      (then (return (i32.const 0))))
    (if (call $string_eq (local.get $ptr) (local.get $len) (i32.const 32832) (i32.const 23))
      (then (return (i32.const 1))))
    (if (call $string_eq (local.get $ptr) (local.get $len) (i32.const 32864) (i32.const 22))
      (then (return (i32.const 2))))
    (if (call $string_eq (local.get $ptr) (local.get $len) (i32.const 32900) (i32.const 24))
      (then (return (i32.const 3))))
    (if (call $string_eq (local.get $ptr) (local.get $len) (i32.const 32932) (i32.const 28))
      (then (return (i32.const 4))))
    (if (call $string_eq (local.get $ptr) (local.get $len) (i32.const 32968) (i32.const 29))
      (then (return (i32.const 5))))
    i32.const -1)

  (func $unit_kind (param $index i32) (result i32)
    (if (i32.le_u (local.get $index) (i32.const 1))
      (then (return (i32.const 0))))
    i32.const 1)

  (func $unit_standard_code (param $index i32) (result i32)
    (if
      (i32.or
        (i32.eq (local.get $index) (i32.const 0))
        (i32.eq (local.get $index) (i32.const 2)))
      (then (return (i32.const 1))))
    i32.const 2)

  (func $unit_wasm_export_code (param $index i32) (result i32)
    (if (i32.le_u (local.get $index) (i32.const 1))
      (then (return (i32.const 1))))
    i32.const 2)

  (func $unit_requirement_mask (param $index i32) (result i32)
    (if (i32.eq (local.get $index) (i32.const 2)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $index) (i32.const 3)) (then (return (i32.const 2))))
    (if (i32.eq (local.get $index) (i32.const 4)) (then (return (i32.const 4))))
    (if (i32.eq (local.get $index) (i32.const 5)) (then (return (i32.const 8))))
    i32.const 0)

  (func $sdk_seed_id_valid (export "sdk_seed_id_valid") (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (local $c i32)
    (local $prev_hyphen i32)
    (if
      (i32.or
        (i32.eqz (local.get $len))
        (i32.gt_u (local.get $len) (i32.const 96)))
      (then (return (i32.const 3))))
    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $i) (local.get $len)))
        (local.set $c (i32.load8_u (i32.add (local.get $ptr) (local.get $i))))
        (if (i32.eqz (call $is_id_char (local.get $c)))
          (then (return (i32.const 3))))
        (if (i32.eq (local.get $c) (i32.const 45))
          (then
            (if
              (i32.or
                (i32.or
                  (i32.eqz (local.get $i))
                  (i32.eq (local.get $i) (i32.sub (local.get $len) (i32.const 1))))
                (local.get $prev_hyphen))
              (then (return (i32.const 3))))
            (local.set $prev_hyphen (i32.const 1)))
          (else
            (local.set $prev_hyphen (i32.const 0))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop)))
    i32.const 0)

  (func $sdk_seed_namespace_valid (export "sdk_seed_namespace_valid") (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (local $c i32)
    (local $prev_sep i32)
    (if
      (i32.or
        (i32.eqz (local.get $len))
        (i32.gt_u (local.get $len) (i32.const 128)))
      (then (return (i32.const 3))))
    (local.set $prev_sep (i32.const 1))
    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $i) (local.get $len)))
        (local.set $c (i32.load8_u (i32.add (local.get $ptr) (local.get $i))))
        (if
          (i32.or (i32.eq (local.get $c) (i32.const 46)) (i32.eq (local.get $c) (i32.const 58)))
          (then
            (if (local.get $prev_sep) (then (return (i32.const 3))))
            (local.set $prev_sep (i32.const 1)))
          (else
            (if
              (i32.eqz
                (i32.or
                  (i32.or (call $is_id_char (local.get $c)) (i32.eq (local.get $c) (i32.const 95)))
                  (i32.eq (local.get $c) (i32.const 64))))
              (then (return (i32.const 3))))
            (local.set $prev_sep (i32.const 0))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop)))
    (if (local.get $prev_sep) (then (return (i32.const 3))))
    i32.const 0)

  (func (export "sdk_seed_unit_lookup") (param $ptr i32) (param $len i32) (result i32)
    (local $index i32)
    (if (i32.ne (call $sdk_seed_id_valid (local.get $ptr) (local.get $len)) (i32.const 0))
      (then (return (i32.const 3))))
    (local.set $index (call $unit_index (local.get $ptr) (local.get $len)))
    (if (i32.lt_s (local.get $index) (i32.const 0))
      (then (return (i32.const 1))))
    i32.const 0)

  (func (export "sdk_seed_unit_kind") (param $ptr i32) (param $len i32) (result i32)
    (local $index i32)
    (if (i32.ne (call $sdk_seed_id_valid (local.get $ptr) (local.get $len)) (i32.const 0))
      (then (return (i32.const -2))))
    (local.set $index (call $unit_index (local.get $ptr) (local.get $len)))
    (if (i32.lt_s (local.get $index) (i32.const 0))
      (then (return (i32.const -1))))
    (call $unit_kind (local.get $index)))

  (func (export "sdk_seed_graph_member") (param $ptr i32) (param $len i32) (result i32)
    (local $index i32)
    (if (i32.ne (call $sdk_seed_id_valid (local.get $ptr) (local.get $len)) (i32.const 0))
      (then (return (i32.const 3))))
    (local.set $index (call $unit_index (local.get $ptr) (local.get $len)))
    (if
      (i32.or
        (i32.eq (local.get $index) (i32.const 1))
        (i32.or
          (i32.eq (local.get $index) (i32.const 3))
          (i32.or
            (i32.eq (local.get $index) (i32.const 4))
            (i32.eq (local.get $index) (i32.const 5)))))
      (then (return (i32.const 0))))
    i32.const 1)

  (func (export "sdk_seed_clause_table_status")
    (param $id_ptr i32) (param $id_len i32)
    (param $byte_len i32) (param $opcode i32)
    (param $udp_length i32) (param $udp_payload_len i32)
    (result i32)
    (local $index i32)
    (if (i32.ne (call $sdk_seed_id_valid (local.get $id_ptr) (local.get $id_len)) (i32.const 0))
      (then (return (i32.const 3))))
    (local.set $index (call $unit_index (local.get $id_ptr) (local.get $id_len)))
    (if (i32.lt_s (local.get $index) (i32.const 0))
      (then (return (i32.const 1))))
    (if (i32.eq (local.get $index) (i32.const 2))
      (then
        (if
          (i32.and
            (i32.ge_u (local.get $udp_length) (i32.const 8))
            (i32.eq (local.get $udp_length) (i32.add (local.get $udp_payload_len) (i32.const 8))))
          (then (return (i32.const 0))))
        (return (i32.const 2))))
    (if (i32.eq (local.get $index) (i32.const 3))
      (then
        (if
          (i32.and
            (i32.ge_u (local.get $byte_len) (i32.const 2))
            (i32.and
              (i32.ge_u (local.get $opcode) (i32.const 1))
              (i32.le_u (local.get $opcode) (i32.const 6))))
          (then (return (i32.const 0))))
        (return (i32.const 2))))
    (if (i32.eq (local.get $index) (i32.const 4))
      (then
        (if
          (i32.or
            (i32.ne (local.get $opcode) (i32.const 4))
            (i32.eq (local.get $byte_len) (i32.const 4)))
          (then (return (i32.const 0))))
        (return (i32.const 2))))
    (if (i32.eq (local.get $index) (i32.const 5))
      (then
        (if
          (i32.or
            (i32.ne (local.get $opcode) (i32.const 3))
            (i32.ge_u (local.get $byte_len) (i32.const 4)))
          (then (return (i32.const 0))))
        (return (i32.const 2))))
    i32.const 1)

  (func $sdk_seed_unit_preimage (export "sdk_seed_unit_preimage")
    (param $id_ptr i32) (param $id_len i32)
    (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (local $index i32)
    (local $need i32)
    (if (i32.ne (call $sdk_seed_id_valid (local.get $id_ptr) (local.get $id_len)) (i32.const 0))
      (then (return (call $pack (i32.const 3) (i32.const 0)))))
    (local.set $index (call $unit_index (local.get $id_ptr) (local.get $id_len)))
    (if (i32.lt_s (local.get $index) (i32.const 0))
      (then (return (call $pack (i32.const 1) (i32.const 0)))))
    (local.set $need (i32.add (i32.const 39) (local.get $id_len)))
    (if (i32.lt_u (local.get $out_cap) (local.get $need))
      (then (return (call $pack (i32.const 2) (i32.const 0)))))
    (call $m162copy (i32.const 32768) (i32.const 30) (local.get $out_ptr))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 30)) (local.get $index))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 31)) (call $unit_kind (local.get $index)))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 32)) (call $unit_standard_code (local.get $index)))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 33)) (call $unit_wasm_export_code (local.get $index)))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 34)) (call $unit_requirement_mask (local.get $index)))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 38)) (local.get $id_len))
    (call $m162copy (local.get $id_ptr) (local.get $id_len) (i32.add (local.get $out_ptr) (i32.const 39)))
    (call $pack (i32.const 0) (local.get $need)))

  (func $sdk_seed_shape32 (export "sdk_seed_shape32")
    (param $in_ptr i32) (param $in_len i32)
    (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (local $i i32)
    (local $b i32)
    (local $s0 i32)
    (local $s1 i32)
    (local $s2 i32)
    (local $s3 i32)
    (if (i32.lt_u (local.get $out_cap) (i32.const 32))
      (then (return (call $pack (i32.const 2) (i32.const 0)))))
    (local.set $s0 (i32.const 0x811c9dc5))
    (local.set $s1 (i32.const 0x9e3779b9))
    (local.set $s2 (i32.const 0x85ebca6b))
    (local.set $s3 (i32.const 0xc2b2ae35))
    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $i) (local.get $in_len)))
        (local.set $b (i32.load8_u (i32.add (local.get $in_ptr) (local.get $i))))
        (local.set $s0
          (i32.add
            (i32.mul (i32.rotl (i32.xor (local.get $s0) (local.get $b)) (i32.const 5)) (i32.const 16777619))
            (local.get $i)))
        (local.set $s1
          (i32.add
            (i32.rotl (i32.add (local.get $s1) (local.get $b)) (i32.const 7))
            (i32.const 0x7f4a7c15)))
        (local.set $s2
          (i32.xor
            (i32.rotl (local.get $s2) (i32.const 11))
            (i32.add (i32.shl (local.get $b) (i32.const 16)) (local.get $i))))
        (local.set $s3
          (i32.add
            (i32.xor (local.get $s3) (i32.mul (local.get $b) (i32.const 0x45d9f3b)))
            (i32.rotl (local.get $s0) (i32.const 13))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop)))
    (i32.store (local.get $out_ptr) (local.get $s0))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 4)) (local.get $s1))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 8)) (local.get $s2))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 12)) (local.get $s3))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 16)) (i32.xor (local.get $s0) (local.get $s2)))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 20)) (i32.xor (local.get $s1) (local.get $s3)))
    (i32.store
      (i32.add (local.get $out_ptr) (i32.const 24))
      (i32.add (i32.rotl (local.get $s0) (i32.const 17)) (local.get $s3)))
    (i32.store
      (i32.add (local.get $out_ptr) (i32.const 28))
      (i32.xor (i32.rotl (local.get $s1) (i32.const 3)) (local.get $s2)))
    (call $pack (i32.const 0) (i32.const 32)))

  (func (export "sdk_seed_unit_shape32")
    (param $id_ptr i32) (param $id_len i32)
    (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (local $r i64)
    (local $status i32)
    (local $written i32)
    (local.set $r
      (call $sdk_seed_unit_preimage
        (local.get $id_ptr)
        (local.get $id_len)
        (i32.const 8192)
        (i32.const 512)))
    (local.set $status (i32.wrap_i64 (local.get $r)))
    (local.set $written (i32.wrap_i64 (i64.shr_u (local.get $r) (i64.const 32))))
    (if (i32.ne (local.get $status) (i32.const 0))
      (then (return (call $pack (local.get $status) (i32.const 0)))))
    (call $sdk_seed_shape32
      (i32.const 8192)
      (local.get $written)
      (local.get $out_ptr)
      (local.get $out_cap)))

  (data (i32.const 32768) "edgerun-sdk-standards-seed/v1\00")
  (data (i32.const 32800) "udp-datagram-definition")
  (data (i32.const 32832) "tftp-message-definition")
  (data (i32.const 32864) "udp-rfc768-length-0001")
  (data (i32.const 32900) "tftp-rfc1350-opcode-0001")
  (data (i32.const 32932) "tftp-rfc1350-ack-length-0001")
  (data (i32.const 32968) "tftp-rfc1350-data-length-0001")
