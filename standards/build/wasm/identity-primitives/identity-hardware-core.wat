(module
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
)
