(module
  (import "edgerun" "to_lower" (func $m93ascii_lower (param i32) (result i32)))
  (import "edgerun" "fnv1a_lower" (func $m93fnv_lower (param i32 i32) (result i32)))
  (import "edgerun" "pack" (func $pack (param i32 i32) (result i64)))
  (import "edgerun" "lo" (func $lo (param i64) (result i32)))
  (import "edgerun" "hi" (func $hi (param i64) (result i32)))
  (import "edgerun" "is_digit" (func $is_digit (param i32) (result i32)))
  (memory (export "memory") 1)
;; EdgeRun capability policy and remote capability semantics plundered from
  ;; edgerun-capabilities and edgerun-remote-capability.

  (func (export "cap_descriptor_version") (result i32)
    i32.const 1)

  (func (export "cap_validate_descriptor")
    (param $provider_name_present i32) (param $instance_present i32)
    (param $modalities_len i32) (param $event_kinds_len i32) (param $operations_len i32)
    (result i32)
    (if (i32.eqz (local.get $provider_name_present)) (then (return (i32.const 1))))
    (if (i32.eqz (local.get $instance_present)) (then (return (i32.const 2))))
    (if (i32.eqz (local.get $modalities_len)) (then (return (i32.const 3))))
    (if (i32.eqz (local.get $event_kinds_len)) (then (return (i32.const 4))))
    (if (i32.eqz (local.get $operations_len)) (then (return (i32.const 5))))
    i32.const 0)

  (func (export "cap_validate_grant")
    (param $has_grantee i32) (param $has_selector i32) (param $operation_count i32)
    (result i32)
    (if (i32.eqz (local.get $has_grantee)) (then (return (i32.const 1))))
    (if (i32.eqz (local.get $has_selector)) (then (return (i32.const 2))))
    (if (i32.eqz (local.get $operation_count)) (then (return (i32.const 3))))
    i32.const 0)

  (func (export "cap_policy_default_secs") (param $which i32) (result i64)
    (if (result i64) (i32.eq (local.get $which) (i32.const 1))
      (then i64.const 300)
      (else
        (if (result i64) (i32.eq (local.get $which) (i32.const 2))
          (then i64.const 3600)
          (else i64.const 0)))))

  (func (export "cap_context_default") (param $which i32) (result i32)
    (if (result i32) (i32.eq (local.get $which) (i32.const 1))
      (then i32.const 1) ;; is_local
      (else i32.const 0))) ;; user/biometric/hardware false

  (func (export "cap_revocation_reason_code") (param $reason i32) (result i32)
    (if (result i32) (i32.le_u (local.get $reason) (i32.const 5))
      (then local.get $reason)
      (else i32.const 5))) ;; Other string

  (func (export "cap_effective_access_class") (param $selector_access i32) (result i32)
    (if (result i32) (i32.eqz (local.get $selector_access))
      (then i32.const 1) ;; Derived
      (else local.get $selector_access)))

  (func (export "cap_effective_operations_source")
    (param $request_ops_len i32) (param $selector_ops_len i32) (param $descriptor_ops_len i32)
    (result i32)
    (if (i32.gt_u (local.get $request_ops_len) (i32.const 0)) (then (return (i32.const 1))))
    (if (i32.gt_u (local.get $selector_ops_len) (i32.const 0)) (then (return (i32.const 2))))
    (if (result i32) (i32.gt_u (local.get $descriptor_ops_len) (i32.const 0))
      (then i32.const 3)
      (else i32.const 0)))

  (func (export "cap_dedupe_append") (param $already_seen i32) (result i32)
    (i32.eqz (local.get $already_seen)))

  (func (export "cap_selector_match_result")
    (param $provider_instance_mismatch i32) (param $role_mismatch i32)
    (param $capability_id_mismatch i32) (param $unsupported_modality i32)
    (param $unsupported_event_kind i32)
    (result i32)
    (if (local.get $provider_instance_mismatch) (then (return (i32.const 1))))
    (if (local.get $role_mismatch) (then (return (i32.const 2))))
    (if (local.get $capability_id_mismatch) (then (return (i32.const 3))))
    (if (local.get $unsupported_modality) (then (return (i32.const 4))))
    (if (local.get $unsupported_event_kind) (then (return (i32.const 5))))
    i32.const 0)

  (func (export "cap_requires_user_presence")
    (param $explicit i32) (param $sensitive_modality i32) (param $input_or_secure_role i32)
    (param $sensitive_operation i32)
    (result i32)
    (i32.or
      (local.get $explicit)
      (i32.and
        (i32.or (local.get $sensitive_modality) (local.get $input_or_secure_role))
        (local.get $sensitive_operation))))

  (func (export "cap_evaluate_constraints")
    (param $access_raw i32) (param $is_local i32)
    (param $require_local i32) (param $require_hardware i32) (param $hardware_present i32)
    (param $require_biometric i32) (param $biometric_present i32)
    (param $requires_user_presence i32) (param $user_present i32)
    (result i32)
    (if (i32.and (local.get $access_raw) (i32.eqz (local.get $is_local)))
      (then (return (i32.const 1)))) ;; deny raw remote
    (if (i32.and (local.get $require_local) (i32.eqz (local.get $is_local)))
      (then (return (i32.const 2)))) ;; deny local-only remote
    (if (i32.and (local.get $require_hardware) (i32.eqz (local.get $hardware_present)))
      (then (return (i32.const 3)))) ;; deny missing hardware protection
    (if (i32.and (local.get $require_biometric) (i32.eqz (local.get $biometric_present)))
      (then (return (i32.const 4)))) ;; require interaction: biometric
    (if (i32.and (local.get $requires_user_presence) (i32.eqz (local.get $user_present)))
      (then (return (i32.const 5)))) ;; require interaction: user presence
    i32.const 0)

  (func (export "cap_requested_expiry_secs")
    (param $requested_valid i32) (param $requested_secs i64) (param $default_secs i64) (param $max_secs i64)
    (result i64)
    (local $duration i64)
    (local.set $duration
      (if (result i64) (local.get $requested_valid)
        (then local.get $requested_secs)
        (else local.get $default_secs)))
    (if (result i64) (i64.gt_u (local.get $duration) (local.get $max_secs))
      (then local.get $max_secs)
      (else local.get $duration)))

  (func (export "cap_evaluate_request_result")
    (param $descriptor_valid i32) (param $has_selector i32) (param $selector_result i32)
    (param $supported_operation_count i32) (param $constraints_result i32) (param $grant_valid i32)
    (result i32)
    (if (i32.ne (local.get $descriptor_valid) (i32.const 0)) (then (return (i32.const 1))))
    (if (i32.eqz (local.get $has_selector)) (then (return (i32.const 2))))
    (if (i32.ne (local.get $selector_result) (i32.const 0)) (then (return (i32.const 3))))
    (if (i32.eqz (local.get $supported_operation_count)) (then (return (i32.const 4))))
    (if (i32.ne (local.get $constraints_result) (i32.const 0)) (then (return (local.get $constraints_result))))
    (if (i32.ne (local.get $grant_valid) (i32.const 0)) (then (return (i32.const 6))))
    i32.const 0)

  (func (export "cap_authorize_invocation_result")
    (param $grant_known i32) (param $revoked i32) (param $expired i32)
    (param $grant_derived i32) (param $request_raw i32)
    (param $operation_granted i32) (param $grant_has_selector i32)
    (param $constraints_result i32) (param $one_shot i32) (param $invocation_count i64)
    (param $rate_limited i32)
    (result i32)
    (if (i32.eqz (local.get $grant_known)) (then (return (i32.const 1))))
    (if (local.get $revoked) (then (return (i32.const 2))))
    (if (local.get $expired) (then (return (i32.const 3))))
    (if (i32.and (local.get $grant_derived) (local.get $request_raw)) (then (return (i32.const 4))))
    (if (i32.eqz (local.get $operation_granted)) (then (return (i32.const 5))))
    (if (i32.eqz (local.get $grant_has_selector)) (then (return (i32.const 6))))
    (if (i32.ne (local.get $constraints_result) (i32.const 0)) (then (return (i32.const 7))))
    (if (i32.and (local.get $one_shot) (i64.gt_u (local.get $invocation_count) (i64.const 0))) (then (return (i32.const 8))))
    (if (local.get $rate_limited) (then (return (i32.const 9))))
    i32.const 0)

  (func (export "cap_rate_limit_allows") (param $window_count i64) (param $max_operations i64) (result i32)
    (i64.lt_u (local.get $window_count) (local.get $max_operations)))

  (func (export "cap_timestamp_valid") (param $seconds i64) (param $nanos i32) (result i32)
    (i32.and (i64.ge_s (local.get $seconds) (i64.const 0)) (i32.ge_s (local.get $nanos) (i32.const 0))))

  (func (export "cap_duration_valid") (param $seconds i64) (param $nanos i32) (result i32)
    (call $cap_timestamp_valid_impl (local.get $seconds) (local.get $nanos)))

  (func $cap_timestamp_valid_impl (param $seconds i64) (param $nanos i32) (result i32)
    (i32.and (i64.ge_s (local.get $seconds) (i64.const 0)) (i32.ge_s (local.get $nanos) (i32.const 0))))

  (func (export "cap_grant_id_preimage_len")
    (param $request_id_len i32) (param $provider_name_len i32) (param $instance_len i32)
    (result i32)
    ;; four length prefixes: domain, request id, provider name, provider instance;
    ;; plus domain bytes(37), request/provider bytes, nonce(8), secs(8), nanos(4)
    (i32.add
      (i32.const 85)
      (i32.add (local.get $request_id_len) (i32.add (local.get $provider_name_len) (local.get $instance_len)))))

  (func (export "cap_revocation_id_domain") (result i32)
    i32.const 1) ;; sha256(grant_id || reason.as_str())

  (func (export "remote_session_open_request_selector_source") (param $open_has_selector i32) (result i32)
    (if (result i32) (local.get $open_has_selector)
      (then i32.const 1)
      (else i32.const 2))) ;; descriptor-derived selector

  (func (export "remote_session_accept_kind") (param $policy_grant i32) (param $accepted i32) (result i32)
    (if (i32.eqz (local.get $policy_grant)) (then (return (i32.const 2)))) ;; reject
    (if (result i32) (local.get $accepted)
      (then i32.const 1) ;; grant-backed accept
      (else i32.const 3))) ;; inner rejected; revoke grant

  (func (export "remote_accept_unchecked_fields") (param $requested_ops_len i32) (result i32)
    (local.get $requested_ops_len)) ;; accepted=true, grant_id=session_id, access=requested_access

  (func (export "remote_serve_action") (param $message_kind i32) (param $frame_has_invocation i32) (param $provider_ok i32) (param $has_grant i32) (result i32)
    (if (i32.eq (local.get $message_kind) (i32.const 1)) (then (return (i32.const 1)))) ;; SessionOpen -> SessionAccept
    (if (i32.eq (local.get $message_kind) (i32.const 2))
      (then (return (if (result i32) (local.get $provider_ok) (then i32.const 2) (else i32.const 3))))) ;; Invocation -> Result
    (if (i32.eq (local.get $message_kind) (i32.const 3))
      (then
        (if (i32.eqz (local.get $frame_has_invocation)) (then (return (i32.const 4))))
        (return (if (result i32) (local.get $provider_ok) (then i32.const 5) (else i32.const 3))))) ;; InvocationFrame -> ResultFrame/Result
    (if (i32.eq (local.get $message_kind) (i32.const 4)) (then (return (i32.const 6)))) ;; SessionClose fire-and-forget
    (if (i32.eq (local.get $message_kind) (i32.const 5))
      (then (return (if (result i32) (local.get $has_grant) (then i32.const 7) (else i32.const 8))))) ;; Request -> Grant or no response
    (if (i32.eq (local.get $message_kind) (i32.const 6)) (then (return (i32.const 9)))) ;; standalone Grant rejected
    (if (i32.eq (local.get $message_kind) (i32.const 7)) (then (return (i32.const 10)))) ;; Revocation
    i32.const 0)

  (func (export "remote_result_message_kind") (param $inline_payload_len i32) (result i32)
    (if (result i32) (i32.eqz (local.get $inline_payload_len))
      (then i32.const 1) ;; Result
      (else i32.const 2))) ;; ResultFrame

  (func (export "remote_error_result_success") (result i32)
    i32.const 0)

  (func (export "remote_pump_event_action") (param $has_event i32) (result i32)
    (if (result i32) (local.get $has_event)
      (then i32.const 1)
      (else i32.const 0)))

  (func (export "remote_memory_transport_action") (param $action i32) (param $queue_len i32) (result i32)
    (if (i32.eq (local.get $action) (i32.const 1)) (then (return (i32.add (local.get $queue_len) (i32.const 1))))) ;; send push_back
    (if (i32.eq (local.get $action) (i32.const 2))
      (then (return (if (result i32) (i32.gt_u (local.get $queue_len) (i32.const 0)) (then i32.const 1) (else i32.const 0))))) ;; recv pop_front some?
    i32.const 0)

  (func (export "remote_framed_recv_result") (param $first_read_len i32) (param $payload_decode_ok i32) (result i32)
    (if (i32.eqz (local.get $first_read_len)) (then (return (i32.const 0)))) ;; EOF none
    (if (i32.gt_u (local.get $first_read_len) (i32.const 4)) (then (return (i32.const 2))))
    (if (result i32) (local.get $payload_decode_ok)
      (then i32.const 1)
      (else i32.const 3))) ;; provider decode error

  (func (export "remote_policy_invoke_result") (param $binding_exists i32) (param $auth_result i32) (param $inner_ok i32) (result i32)
    (if (i32.eqz (local.get $binding_exists)) (then (return (i32.const 1))))
    (if (i32.ne (local.get $auth_result) (i32.const 0)) (then (return (i32.const 2))))
    (if (result i32) (local.get $inner_ok) (then i32.const 0) (else i32.const 3)))

  (func (export "remote_close_session_action") (param $has_grant_binding i32) (result i32)
    (if (result i32) (local.get $has_grant_binding)
      (then i32.const 1) ;; revoke Superseded then close inner
      (else i32.const 0)))

  (func (export "remote_stream_event_sequence") (param $events_available i32) (param $current_seq i64) (result i64)
    (if (result i64) (local.get $events_available)
      (then local.get $current_seq)
      (else i64.const 0)))

  (func (export "remote_next_sequence") (param $current_seq i64) (result i64)
    (i64.add (local.get $current_seq) (i64.const 1)))

  (func (export "remote_stream_oriented_invoke") (result i32)
    i32.const 1) ;; returns unsupported stream-oriented error result

  (func (export "remote_input_kind_to_wire") (param $kind i32) (result i32)
    (if (result i32) (i32.and (i32.ge_u (local.get $kind) (i32.const 1)) (i32.le_u (local.get $kind) (i32.const 6)))
      (then local.get $kind)
      (else (i32.or (local.get $kind) (i32.const 0x8000)))))

  (func (export "remote_input_kind_from_wire") (param $wire i32) (result i32)
    (if (result i32) (i32.and (i32.ge_u (local.get $wire) (i32.const 1)) (i32.le_u (local.get $wire) (i32.const 6)))
      (then local.get $wire)
      (else
        (if (result i32) (i32.ne (i32.and (local.get $wire) (i32.const 0x8000)) (i32.const 0))
          (then (i32.and (local.get $wire) (i32.const 0x7fff)))
          (else local.get $wire)))))

  (func (export "remote_audio_capture_format") (param $format i32) (result i32)
    (if (result i32) (i32.and (i32.ge_u (local.get $format) (i32.const 1)) (i32.le_u (local.get $format) (i32.const 4)))
      (then local.get $format)
      (else (i32.or (local.get $format) (i32.const 0x80000000)))))

  (func (export "remote_speaker_format_result") (param $format i32) (result i32)
    (if (result i32) (i32.and (i32.ge_u (local.get $format) (i32.const 1)) (i32.le_u (local.get $format) (i32.const 3)))
      (then local.get $format)
      (else i32.const -1)))

  (func (export "remote_speaker_invoke_result") (param $has_inline i32) (param $operation i32) (param $has_target_level i32) (result i32)
    (if (i32.eqz (local.get $has_inline)) (then (return (i32.const 1))))
    (if (result i32) (i32.eq (local.get $operation) (i32.const 3)) ;; Control
      (then
        (if (result i32) (local.get $has_target_level)
          (then i32.const 2)
          (else i32.const 3)))
      (else
        (if (result i32) (i32.eq (local.get $operation) (i32.const 4)) ;; Render
          (then i32.const 4)
          (else i32.const 0)))))

  (func (export "remote_bluetooth_address_kind") (param $kind i32) (result i32)
    (if (result i32) (i32.le_u (local.get $kind) (i32.const 2)) (then local.get $kind) (else i32.const -1)))

  (func (export "remote_bluetooth_transport_kind") (param $kind i32) (result i32)
    (if (result i32) (i32.le_u (local.get $kind) (i32.const 3)) (then local.get $kind) (else i32.const -1)))

  (func (export "remote_bluetooth_profile_kind") (param $profile i32) (result i32)
    (if (result i32)
      (i32.or (i32.and (i32.ge_u (local.get $profile) (i32.const 1)) (i32.le_u (local.get $profile) (i32.const 12))) (i32.eq (local.get $profile) (i32.const 255)))
      (then local.get $profile)
      (else i32.const -1)))

  (func (export "remote_bluetooth_link_kind") (param $kind i32) (result i32)
    (if (result i32) (i32.le_u (local.get $kind) (i32.const 3)) (then local.get $kind) (else i32.const -1)))

  (func (export "remote_wifi_power_state") (param $state i32) (result i32)
    (if (result i32) (i32.le_u (local.get $state) (i32.const 3)) (then local.get $state) (else i32.const -1)))

  (func (export "remote_wifi_interface_mode") (param $mode i32) (result i32)
    (if (result i32) (i32.le_u (local.get $mode) (i32.const 4)) (then local.get $mode) (else i32.const -1)))

  (func (export "remote_display_content_kind") (param $kind i32) (result i32)
    (if (result i32) (i32.and (i32.ge_u (local.get $kind) (i32.const 1)) (i32.le_u (local.get $kind) (i32.const 4)))
      (then local.get $kind)
      (else (i32.or (local.get $kind) (i32.const 0x80000000)))))

  (func (export "remote_camera_pixel_format") (param $format i32) (result i32)
    (if (result i32) (i32.and (i32.ge_u (local.get $format) (i32.const 1)) (i32.le_u (local.get $format) (i32.const 5)))
      (then local.get $format)
      (else (i32.or (local.get $format) (i32.const 0x80000000)))))

  (func (export "remote_camera_quality_result") (param $quality i32) (result i32)
    (if (result i32) (i32.and (i32.ge_u (local.get $quality) (i32.const 1)) (i32.le_u (local.get $quality) (i32.const 4)))
      (then local.get $quality)
      (else i32.const -1)))

  (func (export "remote_biometric_modality") (param $modality i32) (result i32)
    (if (result i32) (i32.le_u (local.get $modality) (i32.const 5))
      (then local.get $modality)
      (else i32.const 0)))

;; Validator semantics plundered from edgerun-core.
  ;; Result code 0 is accept unless a function says otherwise.

  (func (export "core_validator_abi_version") (result i32) i32.const 1)
  (func (export "core_validator_record_version_result") (param $version i32) (result i32)
    ;; 0 ok, 1 unsupported version.
    (if (i32.ne (local.get $version) (i32.const 1)) (then (return (i32.const 1))))
    i32.const 0)

  (func (export "core_validator_timestamp_result") (param $nanos i32) (result i32)
    ;; 0 ok, 1 structural invalid.
    (if (i32.ge_u (local.get $nanos) (i32.const 1000000000)) (then (return (i32.const 1))))
    i32.const 0)

  (func (export "core_validator_digest_result")
    (param $has_hash i32) (param $algorithm i32) (param $len i32) (result i32)
    ;; 0 ok, 1 missing hash, 2 unsupported algorithm, 3 wrong length. SHA-256 is algorithm 1.
    (if (i32.eqz (local.get $has_hash)) (then (return (i32.const 1))))
    (if (i32.ne (local.get $algorithm) (i32.const 1)) (then (return (i32.const 2))))
    (if (i32.ne (local.get $len) (i32.const 32)) (then (return (i32.const 3))))
    i32.const 0)

  (func (export "core_validator_object_ref_result")
    (param $has_ref i32) (param $object_id_len i32) (param $kind_valid_or_absent i32) (result i32)
    ;; 0 ok, 1 missing ref, 2 empty object id, 3 invalid kind.
    (if (i32.eqz (local.get $has_ref)) (then (return (i32.const 1))))
    (if (i32.eqz (local.get $object_id_len)) (then (return (i32.const 2))))
    (if (i32.eqz (local.get $kind_valid_or_absent)) (then (return (i32.const 3))))
    i32.const 0)

  (func (export "core_validator_identity_ref_result")
    (param $identity_id_len i32) (param $kind_valid i32) (result i32)
    ;; 0 ok, 1 empty identity id, 2 invalid identity kind.
    (if (i32.eqz (local.get $identity_id_len)) (then (return (i32.const 1))))
    (if (i32.eqz (local.get $kind_valid)) (then (return (i32.const 2))))
    i32.const 0)

  (func (export "core_validator_identity_record_result")
    (param $version i32) (param $id_len i32) (param $kind_valid i32) (param $key_alg i32)
    (param $public_key_len i32) (param $has_created_at i32) (param $created_nanos i32)
    (param $supersedes_ok i32) (param $assurance_refs_ok i32) (param $metadata_ok i32)
    (param $has_signature i32) (param $signature_alg i32) (param $signature_len i32)
    (param $public_key_decodes i32) (param $signature_ok i32)
    (result i32)
    ;; 0 ok, 1 bad version, 2 empty id, 3 bad kind, 4 bad key, 5 missing/bad created_at,
    ;; 6 bad nested ref, 7 missing/bad signature, 8 public key decode failed, 9 signature failed.
    (if (i32.ne (local.get $version) (i32.const 1)) (then (return (i32.const 1))))
    (if (i32.eqz (local.get $id_len)) (then (return (i32.const 2))))
    (if (i32.eqz (local.get $kind_valid)) (then (return (i32.const 3))))
    (if (i32.or (i32.ne (local.get $key_alg) (i32.const 1)) (i32.ne (local.get $public_key_len) (i32.const 64)))
      (then (return (i32.const 4))))
    (if (i32.or (i32.eqz (local.get $has_created_at)) (i32.ge_u (local.get $created_nanos) (i32.const 1000000000)))
      (then (return (i32.const 5))))
    (if (i32.eqz (i32.and (i32.and (local.get $supersedes_ok) (local.get $assurance_refs_ok)) (local.get $metadata_ok)))
      (then (return (i32.const 6))))
    (if (i32.or (i32.eqz (local.get $has_signature))
        (i32.or (i32.ne (local.get $signature_alg) (i32.const 1)) (i32.ne (local.get $signature_len) (i32.const 64))))
      (then (return (i32.const 7))))
    (if (i32.eqz (local.get $public_key_decodes)) (then (return (i32.const 8))))
    (if (i32.eqz (local.get $signature_ok)) (then (return (i32.const 9))))
    i32.const 0)

  (func (export "core_validator_stream_heads_proof_result")
    (param $source_query_len i32) (param $heads_len i32) (param $all_heads_valid i32) (result i32)
    ;; 0 ok, 1 missing source query, 2 empty head set, 3 bad stream/event hash.
    (if (i32.eqz (local.get $source_query_len)) (then (return (i32.const 1))))
    (if (i32.eqz (local.get $heads_len)) (then (return (i32.const 2))))
    (if (i32.eqz (local.get $all_heads_valid)) (then (return (i32.const 3))))
    i32.const 0)

  (func (export "core_validator_snapshot_set_proof_result")
    (param $source_query_len i32) (param $snapshot_count i32) (param $all_snapshots_valid i32) (result i32)
    ;; 0 ok, 1 missing source query, 2 empty snapshot set, 3 bad snapshot/object id.
    (if (i32.eqz (local.get $source_query_len)) (then (return (i32.const 1))))
    (if (i32.eqz (local.get $snapshot_count)) (then (return (i32.const 2))))
    (if (i32.eqz (local.get $all_snapshots_valid)) (then (return (i32.const 3))))
    i32.const 0)

  (func (export "core_validator_event_set_proof_result")
    (param $source_query_len i32) (param $event_count i32) (param $events_valid i32) (param $related_objects_ok i32) (result i32)
    ;; 0 ok, 1 missing source query, 2 empty event set, 3 bad event hash/stream, 4 bad related object.
    (if (i32.eqz (local.get $source_query_len)) (then (return (i32.const 1))))
    (if (i32.eqz (local.get $event_count)) (then (return (i32.const 2))))
    (if (i32.eqz (local.get $events_valid)) (then (return (i32.const 3))))
    (if (i32.eqz (local.get $related_objects_ok)) (then (return (i32.const 4))))
    i32.const 0)

  (func (export "core_validator_object_assertion_proof_result")
    (param $source_query_len i32) (param $has_object_ref i32) (param $object_ref_ok i32) (param $bundled_result_ok i32) (result i32)
    ;; 0 ok, 1 missing source query, 2 missing object ref, 3 bad object ref, 4 bad bundled result object.
    (if (i32.eqz (local.get $source_query_len)) (then (return (i32.const 1))))
    (if (i32.eqz (local.get $has_object_ref)) (then (return (i32.const 2))))
    (if (i32.eqz (local.get $object_ref_ok)) (then (return (i32.const 3))))
    (if (i32.eqz (local.get $bundled_result_ok)) (then (return (i32.const 4))))
    i32.const 0)

  (func (export "core_validator_aggregate_summary_result")
    (param $source_query_len i32) (param $responders_ok i32) (param $responder_overlap i32) (param $trust_policy_ok i32) (result i32)
    ;; 0 ok, 1 missing source query, 2 bad responder ref, 3 included/excluded overlap, 4 bad policy object.
    (if (i32.eqz (local.get $source_query_len)) (then (return (i32.const 1))))
    (if (i32.eqz (local.get $responders_ok)) (then (return (i32.const 2))))
    (if (local.get $responder_overlap) (then (return (i32.const 3))))
    (if (i32.eqz (local.get $trust_policy_ok)) (then (return (i32.const 4))))
    i32.const 0)

  (func (export "core_validator_trust_policy_proof_result")
    (param $source_query_len i32) (param $has_policy_object i32) (param $has_assignments_object i32)
    (param $policy_ok i32) (param $assignments_ok i32) (result i32)
    ;; 0 ok, 1 missing source query, 2 no policy or assignments object, 3 bad policy, 4 bad assignments.
    (if (i32.eqz (local.get $source_query_len)) (then (return (i32.const 1))))
    (if (i32.eqz (i32.or (local.get $has_policy_object) (local.get $has_assignments_object))) (then (return (i32.const 2))))
    (if (i32.eqz (local.get $policy_ok)) (then (return (i32.const 3))))
    (if (i32.eqz (local.get $assignments_ok)) (then (return (i32.const 4))))
    i32.const 0)

  (func (export "core_validator_proof_bundle_result")
    (param $version i32) (param $payload_type_valid i32) (param $payload_unspecified i32)
    (param $allowed_count i32) (param $type_allowed i32) (param $source_query_len i32)
    (param $expected_matches i32) (param $has_payload_object i32) (param $payload_object_ok i32)
    (param $supporting_ok i32) (param $availability_checked i32) (param $payload_available i32)
    (param $supporting_available i32) (result i32)
    ;; 0 ok, 1 bad version, 2 bad payload type, 3 type not allowed, 4 missing source query,
    ;; 5 query mismatch, 6 bad payload object, 7 bad supporting object, 8 defer missing payload, 9 defer missing support.
    (if (i32.ne (local.get $version) (i32.const 1)) (then (return (i32.const 1))))
    (if (i32.or (i32.eqz (local.get $payload_type_valid)) (local.get $payload_unspecified)) (then (return (i32.const 2))))
    (if (i32.and (i32.gt_u (local.get $allowed_count) (i32.const 0)) (i32.eqz (local.get $type_allowed))) (then (return (i32.const 3))))
    (if (i32.eqz (local.get $source_query_len)) (then (return (i32.const 4))))
    (if (i32.eqz (local.get $expected_matches)) (then (return (i32.const 5))))
    (if (i32.eqz (i32.and (local.get $has_payload_object) (local.get $payload_object_ok))) (then (return (i32.const 6))))
    (if (i32.eqz (local.get $supporting_ok)) (then (return (i32.const 7))))
    (if (i32.and (local.get $availability_checked) (i32.eqz (local.get $payload_available))) (then (return (i32.const 8))))
    (if (i32.and (local.get $availability_checked) (i32.eqz (local.get $supporting_available))) (then (return (i32.const 9))))
    i32.const 0)

  (func (export "core_validator_federated_aggregate_result")
    (param $version i32) (param $aggregate_id_len i32) (param $source_query_len i32)
    (param $expected_matches i32) (param $aggregator_ok i32) (param $has_aggregated_at i32)
    (param $timestamp_ok i32) (param $input_fragments_len i32) (param $input_fragments_ok i32)
    (param $policy_object_ok i32) (param $has_payload_object i32) (param $payload_object_ok i32)
    (result i32)
    ;; 0 ok, 1 bad version, 2 empty aggregate id, 3 missing source query, 4 query mismatch,
    ;; 5 bad aggregator, 6 bad timestamp, 7 empty/bad fragments, 8 bad policy, 9 bad payload.
    (if (i32.ne (local.get $version) (i32.const 1)) (then (return (i32.const 1))))
    (if (i32.eqz (local.get $aggregate_id_len)) (then (return (i32.const 2))))
    (if (i32.eqz (local.get $source_query_len)) (then (return (i32.const 3))))
    (if (i32.eqz (local.get $expected_matches)) (then (return (i32.const 4))))
    (if (i32.eqz (local.get $aggregator_ok)) (then (return (i32.const 5))))
    (if (i32.eqz (i32.and (local.get $has_aggregated_at) (local.get $timestamp_ok))) (then (return (i32.const 6))))
    (if (i32.or (i32.eqz (local.get $input_fragments_len)) (i32.eqz (local.get $input_fragments_ok))) (then (return (i32.const 7))))
    (if (i32.eqz (local.get $policy_object_ok)) (then (return (i32.const 8))))
    (if (i32.eqz (i32.and (local.get $has_payload_object) (local.get $payload_object_ok))) (then (return (i32.const 9))))
    i32.const 0)

  (func (export "core_validator_route_trust_assignments_result")
    (param $issuer_len i32) (param $issuer_trusted i32) (param $assignment_count i32) (result i32)
    ;; 0 accepted, 1 missing issuer, 2 issuer untrusted, 3 no assignments.
    (if (i32.eqz (local.get $issuer_len)) (then (return (i32.const 1))))
    (if (i32.eqz (local.get $issuer_trusted)) (then (return (i32.const 2))))
    (if (i32.eqz (local.get $assignment_count)) (then (return (i32.const 3))))
    i32.const 0)

  (func (export "core_validator_aggregate_trust_policy_result")
    (param $issuer_len i32) (param $issuer_trusted i32) (param $min_score_present i32) (param $min_score i32) (result i32)
    ;; 0 accepted, 1 missing issuer, 2 issuer untrusted, 3 negative minimum score.
    (if (i32.eqz (local.get $issuer_len)) (then (return (i32.const 1))))
    (if (i32.eqz (local.get $issuer_trusted)) (then (return (i32.const 2))))
    (if (i32.and (local.get $min_score_present) (i32.lt_s (local.get $min_score) (i32.const 0))) (then (return (i32.const 3))))
    i32.const 0)

  (func (export "core_validator_route_candidate_allowed")
    (param $quality i32) (param $min_quality i32) (param $cost i32) (param $max_cost i32)
    (param $require_active i32) (param $has_active i32) (param $allowed_responders_enabled i32)
    (param $responder_allowed i32) (param $score i32) (param $minimum_trust_score i32) (result i32)
    ;; 1 allowed, 0 filtered.
    (if (i32.lt_s (local.get $quality) (local.get $min_quality)) (then (return (i32.const 0))))
    (if (i32.gt_s (local.get $cost) (local.get $max_cost)) (then (return (i32.const 0))))
    (if (i32.and (local.get $require_active) (i32.eqz (local.get $has_active))) (then (return (i32.const 0))))
    (if (i32.and (local.get $allowed_responders_enabled) (i32.eqz (local.get $responder_allowed))) (then (return (i32.const 0))))
    (if (i32.lt_s (local.get $score) (local.get $minimum_trust_score)) (then (return (i32.const 0))))
    i32.const 1)

  (func (export "core_validator_route_candidate_score")
    (param $assignment_score i32) (param $quality i32) (param $cost i32) (param $preferred_aggregator i32) (result i32)
    ;; score = trust assignment + quality - cost + preferred aggregator bonus.
    (i32.add
      (i32.sub (i32.add (local.get $assignment_score) (local.get $quality)) (local.get $cost))
      (if (result i32) (local.get $preferred_aggregator) (then i32.const 1000) (else i32.const 0))))

  (func (export "core_validator_route_replace_best")
    (param $candidate_score i32) (param $best_exists i32) (param $best_score i32)
    (param $candidate_preferred_advertiser i32) (param $best_preferred_advertiser i32)
    (param $candidate_preferred_next_hop i32) (param $best_preferred_next_hop i32)
    (param $candidate_earlier_advertised_at i32) (param $candidate_next_hop_lex_less i32) (result i32)
    ;; 1 replace best. Tie-break: score, preferred advertiser, preferred next hop, earlier time, lexicographic next hop.
    (if (i32.eqz (local.get $best_exists)) (then (return (i32.const 1))))
    (if (i32.gt_s (local.get $candidate_score) (local.get $best_score)) (then (return (i32.const 1))))
    (if (i32.lt_s (local.get $candidate_score) (local.get $best_score)) (then (return (i32.const 0))))
    (if (i32.and (local.get $candidate_preferred_advertiser) (i32.eqz (local.get $best_preferred_advertiser))) (then (return (i32.const 1))))
    (if (i32.and (i32.eq (local.get $candidate_preferred_advertiser) (local.get $best_preferred_advertiser))
        (i32.and (local.get $candidate_preferred_next_hop) (i32.eqz (local.get $best_preferred_next_hop)))) (then (return (i32.const 1))))
    (if (local.get $candidate_earlier_advertised_at) (then (return (i32.const 1))))
    (if (local.get $candidate_next_hop_lex_less) (then (return (i32.const 1))))
    i32.const 0)

  (func (export "core_validator_session_hello_result")
    (param $initiator_len i32) (param $target_matches_local i32) (param $metadata_ok i32)
    (param $feature_lists_ok i32) (param $nonce_len i32) (param $supported_count i32)
    (param $contains_zero_version i32) (param $has_protocol_overlap i32)
    (param $signature_ok i32) (param $locators_ok i32) (result i32)
    ;; 0 ok, 1 missing initiator, 2 target mismatch, 3 bad metadata/features,
    ;; 4 missing nonce, 5 no usable protocol version, 6 invalid signature, 7 bad locators.
    (if (i32.eqz (local.get $initiator_len)) (then (return (i32.const 1))))
    (if (i32.eqz (local.get $target_matches_local)) (then (return (i32.const 2))))
    (if (i32.eqz (i32.and (local.get $metadata_ok) (local.get $feature_lists_ok))) (then (return (i32.const 3))))
    (if (i32.eqz (local.get $nonce_len)) (then (return (i32.const 4))))
    (if (i32.or (i32.eqz (local.get $supported_count))
        (i32.or (local.get $contains_zero_version) (i32.eqz (local.get $has_protocol_overlap)))) (then (return (i32.const 5))))
    (if (i32.eqz (local.get $signature_ok)) (then (return (i32.const 6))))
    (if (i32.eqz (local.get $locators_ok)) (then (return (i32.const 7))))
    i32.const 0)

  (func (export "core_validator_session_accept_result")
    (param $responder_len i32) (param $metadata_ok i32) (param $features_ok i32)
    (param $echoed_nonce_len i32) (param $nonce_matches i32) (param $version_supported i32)
    (param $signature_ok i32) (param $locators_ok i32) (result i32)
    ;; 0 ok, 1 missing responder, 2 bad metadata/features, 3 nonce missing/mismatch,
    ;; 4 unsupported version, 5 invalid signature, 6 bad locators.
    (if (i32.eqz (local.get $responder_len)) (then (return (i32.const 1))))
    (if (i32.eqz (i32.and (local.get $metadata_ok) (local.get $features_ok))) (then (return (i32.const 2))))
    (if (i32.or (i32.eqz (local.get $echoed_nonce_len)) (i32.eqz (local.get $nonce_matches))) (then (return (i32.const 3))))
    (if (i32.eqz (local.get $version_supported)) (then (return (i32.const 4))))
    (if (i32.eqz (local.get $signature_ok)) (then (return (i32.const 5))))
    (if (i32.eqz (local.get $locators_ok)) (then (return (i32.const 6))))
    i32.const 0)

  (func (export "core_validator_route_advertisement_result")
    (param $target_len i32) (param $advertiser_len i32) (param $advertised_at_ok i32)
    (param $next_hop_ok i32) (param $objects_ok i32) (param $expiry_order_ok i32)
    (param $not_expired i32) (param $reachability_count i32) (param $reachability_subjects_ok i32)
    (param $reachability_hints_ok i32) (param $signature_ok i32) (result i32)
    ;; 0 ok, 1 missing target/advertiser, 2 bad time, 3 bad next hop/object, 4 expired/bad expiry,
    ;; 5 no reachability, 6 reachability invalid, 7 signature invalid.
    (if (i32.or (i32.eqz (local.get $target_len)) (i32.eqz (local.get $advertiser_len))) (then (return (i32.const 1))))
    (if (i32.eqz (local.get $advertised_at_ok)) (then (return (i32.const 2))))
    (if (i32.eqz (i32.and (local.get $next_hop_ok) (local.get $objects_ok))) (then (return (i32.const 3))))
    (if (i32.eqz (i32.and (local.get $expiry_order_ok) (local.get $not_expired))) (then (return (i32.const 4))))
    (if (i32.eqz (local.get $reachability_count)) (then (return (i32.const 5))))
    (if (i32.eqz (i32.and (local.get $reachability_subjects_ok) (local.get $reachability_hints_ok))) (then (return (i32.const 6))))
    (if (i32.eqz (local.get $signature_ok)) (then (return (i32.const 7))))
    i32.const 0)

  (func (export "core_validator_relay_envelope_result")
    (param $sender_len i32) (param $message_id_len i32) (param $target_matches_local i32)
    (param $payload_kind_ok i32) (param $relay_chain_ok i32) (param $has_payload_object i32)
    (param $has_inline_payload i32) (param $refs_ok i32) (param $inline_nonempty i32)
    (param $not_expired i32) (param $signature_ok i32) (result i32)
    ;; 0 ok, 1 missing sender/message id, 2 wrong recipient, 3 bad payload kind, 4 bad relay chain,
    ;; 5 payload object xor inline violation, 6 bad ref/inline, 7 expired, 8 signature invalid.
    (if (i32.or (i32.eqz (local.get $sender_len)) (i32.eqz (local.get $message_id_len))) (then (return (i32.const 1))))
    (if (i32.eqz (local.get $target_matches_local)) (then (return (i32.const 2))))
    (if (i32.eqz (local.get $payload_kind_ok)) (then (return (i32.const 3))))
    (if (i32.eqz (local.get $relay_chain_ok)) (then (return (i32.const 4))))
    (if (i32.eq (local.get $has_payload_object) (local.get $has_inline_payload)) (then (return (i32.const 5))))
    (if (i32.eqz (local.get $refs_ok)) (then (return (i32.const 6))))
    (if (i32.and (local.get $has_inline_payload) (i32.eqz (local.get $inline_nonempty))) (then (return (i32.const 6))))
    (if (i32.eqz (local.get $not_expired)) (then (return (i32.const 7))))
    (if (i32.eqz (local.get $signature_ok)) (then (return (i32.const 8))))
    i32.const 0)

  (func (export "core_validator_object_descriptor_result")
    (param $object_id_len i32) (param $version i32) (param $kind_ok i32) (param $schema_version i32)
    (param $canonicalization_len i32) (param $canonical_digest_len i32) (param $has_canonical_size i32)
    (param $canonical_size i64) (param $describes_ok i32) (param $metadata_ok i32) (result i32)
    ;; 0 ok, 1 missing id, 2 bad version, 3 bad kind/schema, 4 bad canonical fields, 5 bad refs.
    (if (i32.eqz (local.get $object_id_len)) (then (return (i32.const 1))))
    (if (i32.ne (local.get $version) (i32.const 1)) (then (return (i32.const 2))))
    (if (i32.or (i32.eqz (local.get $kind_ok)) (i32.eqz (local.get $schema_version))) (then (return (i32.const 3))))
    (if (i32.or (i32.eqz (local.get $canonicalization_len))
        (i32.or (i32.eqz (local.get $canonical_digest_len))
          (i32.or (i32.eqz (local.get $has_canonical_size)) (i64.lt_s (local.get $canonical_size) (i64.const 0)))))
      (then (return (i32.const 4))))
    (if (i32.eqz (i32.and (local.get $describes_ok) (local.get $metadata_ok))) (then (return (i32.const 5))))
    i32.const 0)

  (func (export "core_validator_object_header_result")
    (param $version_ok i32) (param $representation_id_len i32) (param $object_id_len i32)
    (param $descriptor_matches i32) (param $refs_ok i32) (param $has_representation_digest i32)
    (param $chunking_mode_ok i32) (param $stored_size i64) (param $manifest_required i32)
    (param $has_manifest i32) (result i32)
    ;; 0 ok, 1 bad version, 2 missing representation/object id, 3 descriptor mismatch,
    ;; 4 bad refs/digest/mode, 5 invalid stored size, 6 defer missing chunks.
    (if (i32.eqz (local.get $version_ok)) (then (return (i32.const 1))))
    (if (i32.or (i32.eqz (local.get $representation_id_len)) (i32.eqz (local.get $object_id_len))) (then (return (i32.const 2))))
    (if (i32.eqz (local.get $descriptor_matches)) (then (return (i32.const 3))))
    (if (i32.eqz (i32.and (i32.and (local.get $refs_ok) (local.get $has_representation_digest)) (local.get $chunking_mode_ok))) (then (return (i32.const 4))))
    (if (i64.le_s (local.get $stored_size) (i64.const 0)) (then (return (i32.const 5))))
    (if (i32.and (local.get $manifest_required) (i32.eqz (local.get $has_manifest))) (then (return (i32.const 6))))
    i32.const 0)

  (func (export "core_validator_chunk_manifest_result")
    (param $version_ok i32) (param $object_id_len i32) (param $descriptor_matches i32)
    (param $metadata_ok i32) (param $entries_len i32) (param $claimed_count i32)
    (param $total_stored_size i64) (param $entries_valid i32) (param $sum_lengths i64)
    (param $header_representation_matches i32) (param $header_size_matches i32) (result i32)
    ;; 0 ok, 1 bad version, 2 missing/mismatched object, 3 bad metadata, 4 empty/count mismatch,
    ;; 5 invalid total, 6 bad entry, 7 sum mismatch, 8 header mismatch.
    (if (i32.eqz (local.get $version_ok)) (then (return (i32.const 1))))
    (if (i32.or (i32.eqz (local.get $object_id_len)) (i32.eqz (local.get $descriptor_matches))) (then (return (i32.const 2))))
    (if (i32.eqz (local.get $metadata_ok)) (then (return (i32.const 3))))
    (if (i32.or (i32.eqz (local.get $entries_len)) (i32.ne (local.get $claimed_count) (local.get $entries_len))) (then (return (i32.const 4))))
    (if (i64.le_s (local.get $total_stored_size) (i64.const 0)) (then (return (i32.const 5))))
    (if (i32.eqz (local.get $entries_valid)) (then (return (i32.const 6))))
    (if (i64.ne (local.get $sum_lengths) (local.get $total_stored_size)) (then (return (i32.const 7))))
    (if (i32.eqz (i32.and (local.get $header_representation_matches) (local.get $header_size_matches))) (then (return (i32.const 8))))
    i32.const 0)

  (func (export "core_validator_representation_result")
    (param $uses_manifest i32) (param $all_chunks_available i32) (param $all_chunk_digests_match i32)
    (param $has_stored_bytes i32) (param $computed_object_id_matches i32) (result i32)
    ;; 0 ok, 1 defer missing chunks, 2 chunk digest mismatch, 3 missing bytes, 4 object id mismatch.
    (if (i32.and (local.get $uses_manifest) (i32.eqz (local.get $all_chunks_available))) (then (return (i32.const 1))))
    (if (i32.and (local.get $uses_manifest) (i32.eqz (local.get $all_chunk_digests_match))) (then (return (i32.const 2))))
    (if (i32.and (i32.eqz (local.get $uses_manifest)) (i32.eqz (local.get $has_stored_bytes))) (then (return (i32.const 3))))
    (if (i32.eqz (local.get $computed_object_id_matches)) (then (return (i32.const 4))))
    i32.const 0)

  (func (export "core_validator_command_result")
    (param $has_command i32) (param $version_ok i32) (param $ids_present i32) (param $command_type_ok i32)
    (param $target_matches_local i32) (param $issued_at_ok i32) (param $not_before_ok i32)
    (param $expires_ok i32) (param $signature_ok i32) (param $nested_refs_ok i32)
    (param $replay_duplicate i32) (param $issuer_authorized i32) (result i32)
    ;; 0 ok, 1 missing command, 2 bad version, 3 missing ids, 4 bad command type,
    ;; 5 wrong target, 6 bad time window, 7 signature invalid, 8 bad nested refs,
    ;; 9 duplicate replay, 10 authority denied.
    (if (i32.eqz (local.get $has_command)) (then (return (i32.const 1))))
    (if (i32.eqz (local.get $version_ok)) (then (return (i32.const 2))))
    (if (i32.eqz (local.get $ids_present)) (then (return (i32.const 3))))
    (if (i32.eqz (local.get $command_type_ok)) (then (return (i32.const 4))))
    (if (i32.eqz (local.get $target_matches_local)) (then (return (i32.const 5))))
    (if (i32.eqz (i32.and (i32.and (local.get $issued_at_ok) (local.get $not_before_ok)) (local.get $expires_ok))) (then (return (i32.const 6))))
    (if (i32.eqz (local.get $signature_ok)) (then (return (i32.const 7))))
    (if (i32.eqz (local.get $nested_refs_ok)) (then (return (i32.const 8))))
    (if (local.get $replay_duplicate) (then (return (i32.const 9))))
    (if (i32.eqz (local.get $issuer_authorized)) (then (return (i32.const 10))))
    i32.const 0)

  (func (export "core_validator_fixture_signature_result")
    (param $hex_len i32) (param $all_zero i32) (param $starts_der_sequence i32) (result i32)
    ;; Conformance verifier accepts nonempty DER-looking fixture signatures: hex length >= 100 and starts with 30.
    ;; 0 ok, 1 empty/short, 2 zero signature, 3 not DER sequence.
    (if (i32.lt_u (local.get $hex_len) (i32.const 100)) (then (return (i32.const 1))))
    (if (local.get $all_zero) (then (return (i32.const 2))))
    (if (i32.eqz (local.get $starts_der_sequence)) (then (return (i32.const 3))))
    i32.const 0)

  (func (export "core_native_generated_record_code") (param $kind i32) (result i32)
    ;; 1 common, 2 appabi, 3 stream, 4 wallet, 5 capability, 6 trust, 7 access,
    ;; 8 server resources, 9 app, 10 network, 11 object, 12 identity, 13 capability runtime.
    (if (i32.and (i32.ge_u (local.get $kind) (i32.const 1)) (i32.le_u (local.get $kind) (i32.const 13)))
      (then (return (local.get $kind))))
    i32.const 0)


  (global $q0 (mut i64) (i64.const 0))
  (global $q1 (mut i64) (i64.const 0))
  (global $q2 (mut i64) (i64.const 0))
  (global $q3 (mut i64) (i64.const 0))

  (func (export "marketplace_listing_status_valid") (param $id i32) (result i32)
    (i32.le_u (local.get $id) (i32.const 3)))

  (func (export "marketplace_checkout_status_valid") (param $id i32) (result i32)
    (i32.le_u (local.get $id) (i32.const 3)))

  (func $marketplace_listing_live (export "marketplace_listing_live") (param $id i32) (result i32)
    (i32.and
      (i32.ge_u (local.get $id) (i32.const 1))
      (i32.le_u (local.get $id) (i32.const 2))))

  (func $marketplace_checkout_live (export "marketplace_checkout_live") (param $id i32) (result i32)
    (i32.and
      (i32.ge_u (local.get $id) (i32.const 1))
      (i32.le_u (local.get $id) (i32.const 2))))

  (func $marketplace_create_listing_offer (export "marketplace_create_listing_offer") (param $merchant_id i64) (param $price_lo i64) (param $price_hi i64) (param $min_ship_days i32) (param $max_ship_days i32) (param $expires_s i64) (param $out i32) (result i32)
    (local $ts i64)
    (local $head_len i32)
    (local $varint_len i32)
    (local $boff i32)
    (local.set $head_len (i32.const 24))
    (local.set $boff (i32.add (local.get $out) (local.get $head_len)))
    (local.set $ts (i64.load (i32.const 3000)))
    (i64.store (local.get $out) (local.get $merchant_id))
    (i64.store (i32.add (local.get $out) (i32.const 8)) (local.get $price_lo))
    (i64.store (i32.add (local.get $out) (i32.const 16)) (local.get $price_hi))
    (i32.store8 (i32.add (local.get $out) (i32.const 24)) (i32.const 0))
    (local.set $varint_len (call $write_varint64 (local.get $boff) (i32.add (local.get $out) (i32.const 3048)) (local.get $ts)))
    (i32.store8 (i32.add (local.get $boff) (local.get $varint_len)) (i32.const 44))
    (local.set $varint_len (i32.add (local.get $varint_len) (i32.const 1)))
    (local.set $boff (i32.add (local.get $boff) (local.get $varint_len)))
    (i32.store8 (local.get $boff) (i32.const 0))
    (i32.store16 (i32.add (local.get $out) (i32.const 26)) (local.get $min_ship_days))
    (i32.store16 (i32.add (local.get $out) (i32.const 28)) (local.get $max_ship_days))
    (i64.store (i32.add (local.get $out) (i32.const 30)) (local.get $expires_s))
    (i32.store (i32.add (local.get $out) (i32.const 38)) (i32.const 46))
    (i32.store (i32.add (local.get $out) (i32.const 42)) (local.get $head_len))
    (i32.store (i32.add (local.get $out) (i32.const 46)) (local.get $varint_len))
    (i32.const 0))

  (func $marketplace_accept_offer (export "marketplace_accept_offer") (param $listing_id i64) (param $offer_id i64) (param $offer_expires_s i64) (param $shipping_name_len i32) (param $shipping_address_len i32) (param $shipping_note_len i32) (param $out i32) (result i32)
    (local $head_len i32)
    (local.set $head_len (i32.const 38))
    (i64.store (local.get $out) (local.get $listing_id))
    (i64.store (i32.add (local.get $out) (i32.const 8)) (local.get $offer_id))
    (i64.store (i32.add (local.get $out) (i32.const 16)) (local.get $offer_expires_s))
    (i32.store16 (i32.add (local.get $out) (i32.const 24)) (local.get $shipping_name_len))
    (i32.store16 (i32.add (local.get $out) (i32.const 26)) (local.get $shipping_address_len))
    (i32.store16 (i32.add (local.get $out) (i32.const 28)) (local.get $shipping_note_len))
    (i32.store (i32.add (local.get $out) (i32.const 30)) (i32.const 0))
    (i32.store (i32.add (local.get $out) (i32.const 34)) (local.get $head_len))
    (i32.const 0))

  (func $marketplace_dispute (export "marketplace_dispute") (param $listing_id i64) (param $offer_id i64) (param $arbiter_key_id i32) (param $out i32) (result i32)
    (i64.store (local.get $out) (local.get $listing_id))
    (i64.store (i32.add (local.get $out) (i32.const 8)) (local.get $offer_id))
    (i32.store (i32.add (local.get $out) (i32.const 16)) (local.get $arbiter_key_id))
    (i32.store (i32.add (local.get $out) (i32.const 20)) (i32.const 20))
    (i32.const 0))

  (func $marketplace_resolve_dispute (export "marketplace_resolve_dispute") (param $listing_id i64) (param $offer_id i64) (param $resolution i32) (param $out i32) (result i32)
    (i64.store (local.get $out) (local.get $listing_id))
    (i64.store (i32.add (local.get $out) (i32.const 8)) (local.get $offer_id))
    (i32.store8 (local.get $out) (local.get $resolution))
    (i32.store (i32.add (local.get $out) (i32.const 12)) (i32.const 13))
    (i32.const 0))

  (func $marketplace_checkout (export "marketplace_checkout") (param $listing_id i64) (param $offer_id i64) (param $shipping_addr_len i32) (param $shipping_addr_ptr i32) (param $shipping_note_len i32) (param $shipping_note_ptr i32) (param $out i32) (result i32)
    (local $off i32)
    (local $head_len i32)
    (local.set $head_len (i32.const 28))
    (local.set $off (i32.add (local.get $out) (local.get $head_len)))
    (i64.store (local.get $out) (local.get $listing_id))
    (i64.store (i32.add (local.get $out) (i32.const 8)) (local.get $offer_id))
    (i32.store16 (i32.add (local.get $out) (i32.const 16)) (local.get $shipping_addr_len))
    (i32.store16 (i32.add (local.get $out) (i32.const 18)) (local.get $shipping_note_len))
    (i32.store (i32.add (local.get $out) (i32.const 20)) (i32.const 0))
    (i32.store (i32.add (local.get $out) (i32.const 24)) (local.get $head_len))
    (if (i32.eqz (local.get $shipping_addr_len)) (then (i32.store8 (local.get $off) (i32.const 0))))
    (if (i32.eqz (local.get $shipping_addr_len)) (then (i32.store16 (i32.add (local.get $out) (i32.const 28)) (i32.const 0))))
    (if (i32.eqz (local.get $shipping_note_len)) (then (i32.store8 (local.get $off) (i32.const 0))))
    (if (i32.eqz (local.get $shipping_note_len)) (then (i32.store16 (i32.add (local.get $out) (i32.const 30)) (i32.const 0))))
    (i32.const 0))

  (func $marketplace_rate_seller (export "marketplace_rate_seller") (param $listing_id i64) (param $offer_id i64) (param $rating i32) (param $out i32) (result i32)
    (i64.store (local.get $out) (local.get $listing_id))
    (i64.store (i32.add (local.get $out) (i32.const 8)) (local.get $offer_id))
    (i32.store8 (i32.add (local.get $out) (i32.const 16)) (local.get $rating))
    (i32.store (i32.add (local.get $out) (i32.const 17)) (i32.const 17))
    (i32.const 0))

  (func $write_varint64 (param $buf i32) (param $end i32) (param $v i64) (result i32)
    (local $b i32)
    (loop $loop
      (local.set $b (i32.wrap_i64 (i64.and (local.get $v) (i64.const 127))))
      (local.set $v (i64.shr_u (local.get $v) (i64.const 7)))
      (if (i64.ne (local.get $v) (i64.const 0))
        (then
          (i32.store8 (local.get $buf) (i32.or (local.get $b) (i32.const 128)))
          (local.set $buf (i32.add (local.get $buf) (i32.const 1)))
          (br $loop))
        (else
          (i32.store8 (local.get $buf) (local.get $b))
          (local.set $buf (i32.add (local.get $buf) (i32.const 1))))))
    (if (i32.gt_u (local.get $buf) (local.get $end))
      (then (return (i32.const -1))))
    (local.get $buf))

  (func $marketplace_listings_fee (export "marketplace_listings_fee") (param $price_lo i64) (param $price_hi i64) (param $out i32) (result i32)
    (local $fee_lo i64)
    (local $fee_hi i64)
    (local $rem_lo i64)
    (local $rem_hi i64)
    (local $hundred i64)
    (local.set $hundred (i64.const 100))
    (local.set $fee_lo (i64.div_u (i64.mul (local.get $price_lo) (i64.const 3)) (local.get $hundred)))
    (local.set $rem_lo (i64.rem_u (i64.mul (local.get $price_lo) (i64.const 3)) (local.get $hundred)))
    (local.set $fee_hi (i64.div_u (i64.mul (local.get $price_hi) (i64.const 3)) (local.get $hundred)))
    (local.set $fee_hi (i64.add (local.get $fee_hi) (i64.div_u (local.get $rem_lo) (local.get $hundred))))
    (local.set $rem_hi (i64.rem_u (local.get $rem_lo) (local.get $hundred)))
    (if (i64.ge_u (local.get $rem_hi) (i64.const 50))
      (then (local.set $fee_lo (i64.add (local.get $fee_lo) (i64.const 1)))))
    (i64.store (local.get $out) (local.get $fee_lo))
    (i64.store (i32.add (local.get $out) (i32.const 8)) (local.get $fee_hi))
    (i32.store (i32.add (local.get $out) (i32.const 16)) (i32.const 16))
    (i32.const 0))

  (func $marketplace_affiliate_reward (export "marketplace_affiliate_reward") (param $price_lo i64) (param $price_hi i64) (param $out i32) (result i32)
    (local $reward_lo i64)
    (local $reward_hi i64)
    (local.set $reward_lo (i64.div_u (local.get $price_lo) (i64.const 10)))
    (local.set $reward_hi (i64.div_u (local.get $price_hi) (i64.const 10)))
    (i64.store (local.get $out) (local.get $reward_lo))
    (i64.store (i32.add (local.get $out) (i32.const 8)) (local.get $reward_hi))
    (i64.store (i32.add (local.get $out) (i32.const 16)) (i64.const 0))
    (i64.store (i32.add (local.get $out) (i32.const 24)) (i64.const 0))
    (i64.store (i32.add (local.get $out) (i32.const 32)) (i64.const 0))
    (i64.store (i32.add (local.get $out) (i32.const 40)) (i64.const 0))
    (global.set $q0 (i64.add (global.get $q0) (i64.const 1)))
    (i64.store (i32.add (local.get $out) (i32.const 48)) (local.get $reward_lo))
    (i64.store (i32.add (local.get $out) (i32.const 56)) (local.get $reward_hi))
    (i32.const 0))



  (func $provider_id_from_hash (param $h i32) (result i32)
    (if (i32.eq (local.get $h) (i32.const 1676413470)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $h) (i32.const 2065698207)) (then (return (i32.const 2))))
    (if (i32.eq (local.get $h) (i32.const 4120678169)) (then (return (i32.const 3))))
    i32.const 0)

  (func (export "exchange_provider_code")
    (param $ptr i32)
    (param $len i32)
    (result i32)
    (call $provider_id_from_hash (call $m93fnv_lower (local.get $ptr) (local.get $len))))

  (func (export "exchange_provider_features") (param $provider i32) (result i32)
    (if (i32.eq (local.get $provider) (i32.const 1))
      (then (return (i32.or (i32.const 7) (i32.shl (i32.const 3) (i32.const 8))))))
    (if (i32.eq (local.get $provider) (i32.const 2))
      (then (return (i32.or (i32.const 15) (i32.shl (i32.const 2) (i32.const 8))))))
    (if (i32.eq (local.get $provider) (i32.const 3))
      (then (return (i32.or (i32.const 7) (i32.shl (i32.const 1) (i32.const 8))))))
    i32.const 0)

  (func (export "exchange_map_provider_status")
    (param $provider_ptr i32)
    (param $provider_len i32)
    (param $status_ptr i32)
    (param $status_len i32)
    (result i32)
    (local $provider i32)
    (local $status i32)
    (local.set $provider
      (call $provider_id_from_hash
        (call $m93fnv_lower (local.get $provider_ptr) (local.get $provider_len))))
    (local.set $status (call $m93fnv_lower (local.get $status_ptr) (local.get $status_len)))

    (if (i32.eq (local.get $provider) (i32.const 1))
      (then
        (if (i32.eq (local.get $status) (i32.const 2579463996)) (then (return (i32.const 3))))
        (if (i32.eq (local.get $status) (i32.const 837645499)) (then (return (i32.const 3))))
        (if (i32.eq (local.get $status) (i32.const 814672450)) (then (return (i32.const 4))))
        (if (i32.eq (local.get $status) (i32.const 3209629253)) (then (return (i32.const 5))))
        (if (i32.eq (local.get $status) (i32.const 3376168477)) (then (return (i32.const 5))))
        (if (i32.eq (local.get $status) (i32.const 2921832239)) (then (return (i32.const 6))))
        (if (i32.eq (local.get $status) (i32.const 2281165435)) (then (return (i32.const 7))))
        (if (i32.eq (local.get $status) (i32.const 3260912582)) (then (return (i32.const 7))))
        (if (i32.eq (local.get $status) (i32.const 1398023497)) (then (return (i32.const 8))))
        (if (i32.eq (local.get $status) (i32.const 4101275922)) (then (return (i32.const 9))))
        (if (i32.eq (local.get $status) (i32.const 3769421748)) (then (return (i32.const 15))))
        (if (i32.eq (local.get $status) (i32.const 440867849)) (then (return (i32.const 11))))
        (if (i32.eq (local.get $status) (i32.const 928465625)) (then (return (i32.const 12))))
        (if (i32.eq (local.get $status) (i32.const 1658595102)) (then (return (i32.const 13))))
        (if (i32.eq (local.get $status) (i32.const 864990770)) (then (return (i32.const 14))))
        (if (i32.eq (local.get $status) (i32.const 1150267360)) (then (return (i32.const 17))))
        (if (i32.eq (local.get $status) (i32.const 2220750876)) (then (return (i32.const 18))))))

    (if (i32.eq (local.get $provider) (i32.const 2))
      (then
        (if (i32.eq (local.get $status) (i32.const 681154065)) (then (return (i32.const 3))))
        (if (i32.eq (local.get $status) (i32.const 4007265960)) (then (return (i32.const 4))))
        (if (i32.eq (local.get $status) (i32.const 2845129065)) (then (return (i32.const 5))))
        (if (i32.eq (local.get $status) (i32.const 2281165435)) (then (return (i32.const 7))))
        (if (i32.eq (local.get $status) (i32.const 1398023497)) (then (return (i32.const 8))))
        (if (i32.eq (local.get $status) (i32.const 2917234339)) (then (return (i32.const 9))))
        (if (i32.eq (local.get $status) (i32.const 4101275922)) (then (return (i32.const 9))))
        (if (i32.eq (local.get $status) (i32.const 3769421748)) (then (return (i32.const 15))))
        (if (i32.eq (local.get $status) (i32.const 1658595102)) (then (return (i32.const 13))))
        (if (i32.eq (local.get $status) (i32.const 864990770)) (then (return (i32.const 14))))))

    (if (i32.eq (local.get $provider) (i32.const 3))
      (then
        (if (i32.eq (local.get $status) (i32.const 2579463996)) (then (return (i32.const 3))))
        (if (i32.eq (local.get $status) (i32.const 814672450)) (then (return (i32.const 4))))
        (if (i32.eq (local.get $status) (i32.const 3260912582)) (then (return (i32.const 6))))
        (if (i32.eq (local.get $status) (i32.const 4101275922)) (then (return (i32.const 7))))
        (if (i32.eq (local.get $status) (i32.const 3769421748)) (then (return (i32.const 8))))
        (if (i32.eq (local.get $status) (i32.const 2220750876)) (then (return (i32.const 9))))))

    i32.const 17)

  (func $asset_hash_known_pair (param $settlement i32) (param $pay i32) (result i32)
    (if
      (i32.and (i32.eq (local.get $settlement) (i32.const 1618751625))
               (i32.eq (local.get $pay) (i32.const 585249028)))
      (then (return (i32.const 1))))
    (if
      (i32.and (i32.eq (local.get $settlement) (i32.const 1618751625))
               (i32.eq (local.get $pay) (i32.const 1339528032)))
      (then (return (i32.const 1))))
    (if
      (i32.and (i32.eq (local.get $settlement) (i32.const 1618751625))
               (i32.eq (local.get $pay) (i32.const 2191266556)))
      (then (return (i32.const 1))))
    (if
      (i32.and (i32.eq (local.get $settlement) (i32.const 1339528032))
               (i32.eq (local.get $pay) (i32.const 1618751625)))
      (then (return (i32.const 1))))
    (if
      (i32.and (i32.eq (local.get $settlement) (i32.const 2191266556))
               (i32.eq (local.get $pay) (i32.const 1618751625)))
      (then (return (i32.const 1))))
    (if
      (i32.and (i32.eq (local.get $settlement) (i32.const 585249028))
               (i32.eq (local.get $pay) (i32.const 1618751625)))
      (then (return (i32.const 1))))
    i32.const 0)

  (func (export "exchange_supports_pair_hash")
    (param $provider i32)
    (param $settlement_symbol_hash i32)
    (param $pay_symbol_hash i32)
    (result i32)
    (if (call $asset_hash_known_pair (local.get $settlement_symbol_hash) (local.get $pay_symbol_hash))
      (then (return (i32.const 1))))
    (if
      (i32.and
        (i32.eq (local.get $provider) (i32.const 2))
        (i32.or
          (i32.and
            (i32.eq (local.get $settlement_symbol_hash) (i32.const 1618751625))
            (i32.eq (local.get $pay_symbol_hash) (i32.const 3795205537)))
          (i32.and
            (i32.eq (local.get $settlement_symbol_hash) (i32.const 3795205537))
            (i32.eq (local.get $pay_symbol_hash) (i32.const 1618751625)))))
      (then (return (i32.const 1))))
    i32.const 0)

  (func (export "exchange_hash_lower") (param $ptr i32) (param $len i32) (result i32)
    (call $m93fnv_lower (local.get $ptr) (local.get $len)))

  (func (export "exchange_event_stream_type") (param $event_kind i32) (result i32)
    (if (i32.eq (local.get $event_kind) (i32.const 1)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $event_kind) (i32.const 2)) (then (return (i32.const 2))))
    (if
      (i32.and
        (i32.ge_u (local.get $event_kind) (i32.const 3))
        (i32.le_u (local.get $event_kind) (i32.const 8)))
      (then (return (i32.const 3))))
    i32.const 0)

  (func (export "exchange_event_has_order_id") (param $event_kind i32) (result i32)
    (i32.and
      (i32.ge_u (local.get $event_kind) (i32.const 2))
      (i32.le_u (local.get $event_kind) (i32.const 8))))

  (func (export "exchange_event_terminal") (param $event_kind i32) (result i32)
    (if (i32.eq (local.get $event_kind) (i32.const 6)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $event_kind) (i32.const 7)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $event_kind) (i32.const 8)) (then (return (i32.const 1))))
    i32.const 0)

  (func (export "exchange_projection_terminal_status") (param $status i32) (result i32)
    (if (i32.eq (local.get $status) (i32.const 9)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $status) (i32.const 13)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $status) (i32.const 15)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $status) (i32.const 16)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $status) (i32.const 17)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $status) (i32.const 18)) (then (return (i32.const 1))))
    i32.const 0)

  (func (export "exchange_terminal_event_for_status") (param $status i32) (result i32)
    (if (i32.eq (local.get $status) (i32.const 9)) (then (return (i32.const 6))))
    (if (i32.eq (local.get $status) (i32.const 15)) (then (return (i32.const 7))))
    (if (i32.eq (local.get $status) (i32.const 16)) (then (return (i32.const 7))))
    (if (i32.eq (local.get $status) (i32.const 18)) (then (return (i32.const 7))))
    i32.const 0)

  (func (export "exchange_provider_contradiction")
    (param $canonical_terminal i32)
    (param $canonical_status i32)
    (param $provider_status i32)
    (result i32)
    (if
      (i32.and
        (local.get $canonical_terminal)
        (i32.ne (local.get $canonical_status) (local.get $provider_status)))
      (then (return (i32.const 17))))
    local.get $provider_status)

  (func (export "exchange_settlement_command_valid")
    (param $command_id_len i32)
    (param $target_node_len i32)
    (param $command_type i32)
    (param $idempotency_len i32)
    (param $app_id_len i32)
    (param $payload_len i32)
    (param $issued_ms i64)
    (param $expires_ms i64)
    (result i32)
    (if (i32.eqz (local.get $command_id_len)) (then (return (i32.const 0))))
    (if (i32.eqz (local.get $target_node_len)) (then (return (i32.const 0))))
    (if (i32.eqz (local.get $command_type)) (then (return (i32.const 0))))
    (if (i32.eqz (local.get $idempotency_len)) (then (return (i32.const 0))))
    (if (i32.eqz (local.get $app_id_len)) (then (return (i32.const 0))))
    (if (i32.eqz (local.get $payload_len)) (then (return (i32.const 0))))
    (if
      (i32.and
        (i64.ne (local.get $expires_ms) (i64.const 0))
        (i64.gt_u (local.get $issued_ms) (local.get $expires_ms)))
      (then (return (i32.const 0))))
    i32.const 1)

  ;; Status values: 0 ok, 1 input_short, 2 output_short, 3 invalid, 4 overflow.
  ;; Parse record, little-endian u32/i32 fields:
  ;; 0 year, 4 month, 8 day, 12 hour, 16 minute, 20 second,
  ;; 24 nanos, 28 signed offset_minutes, 32 unix_seconds_lo, 36 unix_seconds_hi.

  (func $m177digit (param $ptr i32) (param $end i32) (result i32)
    (local $b i32)
    (if (i32.ge_u (local.get $ptr) (local.get $end))
      (then (return (i32.const -1))))
    (local.set $b (i32.load8_u (local.get $ptr)))
    (if (i32.or (i32.lt_u (local.get $b) (i32.const 48)) (i32.gt_u (local.get $b) (i32.const 57)))
      (then (return (i32.const -1))))
    (i32.sub (local.get $b) (i32.const 48)))

  (func $m177two (param $ptr i32) (param $end i32) (result i32)
    (local $a i32)
    (local $b i32)
    (local.set $a (call $m177digit (local.get $ptr) (local.get $end)))
    (if (i32.lt_s (local.get $a) (i32.const 0)) (then (return (i32.const -1))))
    (local.set $b (call $m177digit (i32.add (local.get $ptr) (i32.const 1)) (local.get $end)))
    (if (i32.lt_s (local.get $b) (i32.const 0)) (then (return (i32.const -1))))
    (i32.add (i32.mul (local.get $a) (i32.const 10)) (local.get $b)))

  (func $m177four (param $ptr i32) (param $end i32) (result i32)
    (local $a i32)
    (local $b i32)
    (local.set $a (call $m177two (local.get $ptr) (local.get $end)))
    (if (i32.lt_s (local.get $a) (i32.const 0)) (then (return (i32.const -1))))
    (local.set $b (call $m177two (i32.add (local.get $ptr) (i32.const 2)) (local.get $end)))
    (if (i32.lt_s (local.get $b) (i32.const 0)) (then (return (i32.const -1))))
    (i32.add (i32.mul (local.get $a) (i32.const 100)) (local.get $b)))

  (func $m177is_leap (param $year i32) (result i32)
    (if (i32.ne (i32.rem_u (local.get $year) (i32.const 4)) (i32.const 0))
      (then (return (i32.const 0))))
    (if (i32.ne (i32.rem_u (local.get $year) (i32.const 100)) (i32.const 0))
      (then (return (i32.const 1))))
    (select
      (i32.const 1)
      (i32.const 0)
      (i32.eq (i32.rem_u (local.get $year) (i32.const 400)) (i32.const 0))))

  (func $m177month_days (param $year i32) (param $month i32) (result i32)
    (if (i32.eq (local.get $month) (i32.const 2))
      (then
        (return (select (i32.const 29) (i32.const 28) (call $m177is_leap (local.get $year))))))
    (if
      (i32.or
        (i32.or
          (i32.or
            (i32.eq (local.get $month) (i32.const 1))
            (i32.eq (local.get $month) (i32.const 3)))
          (i32.or
            (i32.eq (local.get $month) (i32.const 5))
            (i32.eq (local.get $month) (i32.const 7))))
        (i32.or
          (i32.or
            (i32.eq (local.get $month) (i32.const 8))
            (i32.eq (local.get $month) (i32.const 10)))
          (i32.eq (local.get $month) (i32.const 12))))
      (then (return (i32.const 31))))
    i32.const 30)

  (func $m177valid_fields
    (param $year i32) (param $month i32) (param $day i32) (param $hour i32) (param $minute i32) (param $second i32)
    (result i32)
    (if (i32.eqz (local.get $year)) (then (return (i32.const 0))))
    (if (i32.or (i32.lt_u (local.get $month) (i32.const 1)) (i32.gt_u (local.get $month) (i32.const 12)))
      (then (return (i32.const 0))))
    (if
      (i32.or
        (i32.lt_u (local.get $day) (i32.const 1))
        (i32.gt_u (local.get $day) (call $m177month_days (local.get $year) (local.get $month))))
      (then (return (i32.const 0))))
    (if (i32.gt_u (local.get $hour) (i32.const 23)) (then (return (i32.const 0))))
    (if (i32.gt_u (local.get $minute) (i32.const 59)) (then (return (i32.const 0))))
    (if (i32.gt_u (local.get $second) (i32.const 59)) (then (return (i32.const 0))))
    i32.const 1)

  (func $m177days_before_year (param $year i32) (result i64)
    (local $y i64)
    (local.set $y (i64.extend_i32_s (i32.sub (local.get $year) (i32.const 1))))
    (i64.add
      (i64.sub
        (i64.add
          (i64.mul (local.get $y) (i64.const 365))
          (i64.div_s (local.get $y) (i64.const 4)))
        (i64.div_s (local.get $y) (i64.const 100)))
      (i64.div_s (local.get $y) (i64.const 400))))

  (func $m177days_since_epoch (param $year i32) (param $month i32) (param $day i32) (result i64)
    (local $m i32)
    (local $days i64)
    (local.set $days
      (i64.sub
        (call $m177days_before_year (local.get $year))
        (i64.const 719162)))
    (local.set $m (i32.const 1))
    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $m) (local.get $month)))
        (local.set $days
          (i64.add (local.get $days) (i64.extend_i32_u (call $m177month_days (local.get $year) (local.get $m)))))
        (local.set $m (i32.add (local.get $m) (i32.const 1)))
        (br $loop)))
    (i64.add (local.get $days) (i64.extend_i32_u (i32.sub (local.get $day) (i32.const 1)))))

  (func $m177unix_seconds
    (param $year i32) (param $month i32) (param $day i32) (param $hour i32) (param $minute i32) (param $second i32) (param $offset_minutes i32)
    (result i64)
    (i64.sub
      (i64.add
        (i64.add
          (i64.mul (call $m177days_since_epoch (local.get $year) (local.get $month) (local.get $day)) (i64.const 86400))
          (i64.mul (i64.extend_i32_u (local.get $hour)) (i64.const 3600)))
        (i64.add
          (i64.mul (i64.extend_i32_u (local.get $minute)) (i64.const 60))
          (i64.extend_i32_u (local.get $second))))
      (i64.mul (i64.extend_i32_s (local.get $offset_minutes)) (i64.const 60))))

  (func $m177put2 (param $ptr i32) (param $value i32)
    (i32.store8 (local.get $ptr) (i32.add (i32.div_u (local.get $value) (i32.const 10)) (i32.const 48)))
    (i32.store8 (i32.add (local.get $ptr) (i32.const 1)) (i32.add (i32.rem_u (local.get $value) (i32.const 10)) (i32.const 48))))

  (func $m177put4 (param $ptr i32) (param $value i32)
    (i32.store8 (local.get $ptr) (i32.add (i32.rem_u (i32.div_u (local.get $value) (i32.const 1000)) (i32.const 10)) (i32.const 48)))
    (i32.store8 (i32.add (local.get $ptr) (i32.const 1)) (i32.add (i32.rem_u (i32.div_u (local.get $value) (i32.const 100)) (i32.const 10)) (i32.const 48)))
    (i32.store8 (i32.add (local.get $ptr) (i32.const 2)) (i32.add (i32.rem_u (i32.div_u (local.get $value) (i32.const 10)) (i32.const 10)) (i32.const 48)))
    (i32.store8 (i32.add (local.get $ptr) (i32.const 3)) (i32.add (i32.rem_u (local.get $value) (i32.const 10)) (i32.const 48))))

  (func (export "rfc3339_parse") (param $ptr i32) (param $len i32) (param $out i32) (result i32)
    (local $end i32)
    (local $year i32)
    (local $month i32)
    (local $day i32)
    (local $hour i32)
    (local $minute i32)
    (local $second i32)
    (local $nanos i32)
    (local $pos i32)
    (local $m177digits i32)
    (local $d i32)
    (local $tz i32)
    (local $sign i32)
    (local $tz_h i32)
    (local $tz_m i32)
    (local $unix i64)
    (if (i32.lt_u (local.get $len) (i32.const 20)) (then (return (i32.const 1))))
    (local.set $end (i32.add (local.get $ptr) (local.get $len)))
    (local.set $year (call $m177four (local.get $ptr) (local.get $end)))
    (local.set $month (call $m177two (i32.add (local.get $ptr) (i32.const 5)) (local.get $end)))
    (local.set $day (call $m177two (i32.add (local.get $ptr) (i32.const 8)) (local.get $end)))
    (local.set $hour (call $m177two (i32.add (local.get $ptr) (i32.const 11)) (local.get $end)))
    (local.set $minute (call $m177two (i32.add (local.get $ptr) (i32.const 14)) (local.get $end)))
    (local.set $second (call $m177two (i32.add (local.get $ptr) (i32.const 17)) (local.get $end)))
    (if
      (i32.or
        (i32.or
          (i32.or
            (i32.or (i32.lt_s (local.get $year) (i32.const 0)) (i32.lt_s (local.get $month) (i32.const 0)))
            (i32.or (i32.lt_s (local.get $day) (i32.const 0)) (i32.lt_s (local.get $hour) (i32.const 0))))
          (i32.or (i32.lt_s (local.get $minute) (i32.const 0)) (i32.lt_s (local.get $second) (i32.const 0))))
        (i32.or
          (i32.ne (i32.load8_u (i32.add (local.get $ptr) (i32.const 4))) (i32.const 45))
          (i32.ne (i32.load8_u (i32.add (local.get $ptr) (i32.const 7))) (i32.const 45))))
      (then (return (i32.const 3))))
    (if
      (i32.or
        (i32.ne (i32.load8_u (i32.add (local.get $ptr) (i32.const 10))) (i32.const 84))
        (i32.or
          (i32.ne (i32.load8_u (i32.add (local.get $ptr) (i32.const 13))) (i32.const 58))
          (i32.ne (i32.load8_u (i32.add (local.get $ptr) (i32.const 16))) (i32.const 58))))
      (then (return (i32.const 3))))
    (if (i32.eqz (call $m177valid_fields (local.get $year) (local.get $month) (local.get $day) (local.get $hour) (local.get $minute) (local.get $second)))
      (then (return (i32.const 3))))

    (local.set $pos (i32.add (local.get $ptr) (i32.const 19)))
    (if (i32.and (i32.lt_u (local.get $pos) (local.get $end)) (i32.eq (i32.load8_u (local.get $pos)) (i32.const 46)))
      (then
        (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
        (local.set $m177digits (i32.const 0))
        (block $frac_done
          (loop $frac
            (br_if $frac_done (i32.ge_u (local.get $pos) (local.get $end)))
            (local.set $d (call $m177digit (local.get $pos) (local.get $end)))
            (br_if $frac_done (i32.lt_s (local.get $d) (i32.const 0)))
            (if (i32.ge_u (local.get $m177digits) (i32.const 9)) (then (return (i32.const 3))))
            (local.set $nanos (i32.add (i32.mul (local.get $nanos) (i32.const 10)) (local.get $d)))
            (local.set $m177digits (i32.add (local.get $m177digits) (i32.const 1)))
            (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
            (br $frac)))
        (if (i32.eqz (local.get $m177digits)) (then (return (i32.const 3))))
        (block $scale_done
          (loop $scale
            (br_if $scale_done (i32.ge_u (local.get $m177digits) (i32.const 9)))
            (local.set $nanos (i32.mul (local.get $nanos) (i32.const 10)))
            (local.set $m177digits (i32.add (local.get $m177digits) (i32.const 1)))
            (br $scale)))))

    (if (i32.ge_u (local.get $pos) (local.get $end)) (then (return (i32.const 1))))
    (local.set $tz (i32.load8_u (local.get $pos)))
    (if (i32.eq (local.get $tz) (i32.const 90))
      (then
        (if (i32.ne (i32.add (local.get $pos) (i32.const 1)) (local.get $end)) (then (return (i32.const 3))))
        (local.set $tz (i32.const 0)))
      (else
        (if (i32.and (i32.ne (local.get $tz) (i32.const 43)) (i32.ne (local.get $tz) (i32.const 45)))
          (then (return (i32.const 3))))
        (if (i32.ne (i32.add (local.get $pos) (i32.const 6)) (local.get $end)) (then (return (i32.const 3))))
        (local.set $sign (select (i32.const -1) (i32.const 1) (i32.eq (local.get $tz) (i32.const 45))))
        (local.set $tz_h (call $m177two (i32.add (local.get $pos) (i32.const 1)) (local.get $end)))
        (local.set $tz_m (call $m177two (i32.add (local.get $pos) (i32.const 4)) (local.get $end)))
        (if
          (i32.or
            (i32.or (i32.lt_s (local.get $tz_h) (i32.const 0)) (i32.lt_s (local.get $tz_m) (i32.const 0)))
            (i32.or
              (i32.ne (i32.load8_u (i32.add (local.get $pos) (i32.const 3))) (i32.const 58))
              (i32.or (i32.gt_u (local.get $tz_h) (i32.const 23)) (i32.gt_u (local.get $tz_m) (i32.const 59)))))
          (then (return (i32.const 3))))
        (local.set $tz (i32.mul (local.get $sign) (i32.add (i32.mul (local.get $tz_h) (i32.const 60)) (local.get $tz_m))))))

    (local.set $unix
      (call $m177unix_seconds
        (local.get $year) (local.get $month) (local.get $day)
        (local.get $hour) (local.get $minute) (local.get $second)
        (local.get $tz)))
    (i32.store (local.get $out) (local.get $year))
    (i32.store (i32.add (local.get $out) (i32.const 4)) (local.get $month))
    (i32.store (i32.add (local.get $out) (i32.const 8)) (local.get $day))
    (i32.store (i32.add (local.get $out) (i32.const 12)) (local.get $hour))
    (i32.store (i32.add (local.get $out) (i32.const 16)) (local.get $minute))
    (i32.store (i32.add (local.get $out) (i32.const 20)) (local.get $second))
    (i32.store (i32.add (local.get $out) (i32.const 24)) (local.get $nanos))
    (i32.store (i32.add (local.get $out) (i32.const 28)) (local.get $tz))
    (i32.store (i32.add (local.get $out) (i32.const 32)) (i32.wrap_i64 (local.get $unix)))
    (i32.store (i32.add (local.get $out) (i32.const 36)) (i32.wrap_i64 (i64.shr_u (local.get $unix) (i64.const 32))))
    i32.const 0)

  (func (export "rfc3339_emit_utc")
    (param $year i32) (param $month i32) (param $day i32) (param $hour i32) (param $minute i32) (param $second i32) (param $nanos i32) (param $out i32) (param $cap i32)
    (result i64)
    (local $written i32)
    (local $frac i32)
    (local $div i32)
    (local $m177digit i32)
    (local $started i32)
    (if (i32.eqz (call $m177valid_fields (local.get $year) (local.get $month) (local.get $day) (local.get $hour) (local.get $minute) (local.get $second)))
      (then (return (call $pack (i32.const 3) (i32.const 0)))))
    (if (i32.ge_u (local.get $nanos) (i32.const 1000000000))
      (then (return (call $pack (i32.const 3) (i32.const 0)))))
    (local.set $written (select (i32.const 30) (i32.const 20) (i32.ne (local.get $nanos) (i32.const 0))))
    (if (i32.lt_u (local.get $cap) (local.get $written))
      (then (return (call $pack (i32.const 2) (i32.const 0)))))
    (call $m177put4 (local.get $out) (local.get $year))
    (i32.store8 (i32.add (local.get $out) (i32.const 4)) (i32.const 45))
    (call $m177put2 (i32.add (local.get $out) (i32.const 5)) (local.get $month))
    (i32.store8 (i32.add (local.get $out) (i32.const 7)) (i32.const 45))
    (call $m177put2 (i32.add (local.get $out) (i32.const 8)) (local.get $day))
    (i32.store8 (i32.add (local.get $out) (i32.const 10)) (i32.const 84))
    (call $m177put2 (i32.add (local.get $out) (i32.const 11)) (local.get $hour))
    (i32.store8 (i32.add (local.get $out) (i32.const 13)) (i32.const 58))
    (call $m177put2 (i32.add (local.get $out) (i32.const 14)) (local.get $minute))
    (i32.store8 (i32.add (local.get $out) (i32.const 16)) (i32.const 58))
    (call $m177put2 (i32.add (local.get $out) (i32.const 17)) (local.get $second))
    (if (i32.eqz (local.get $nanos))
      (then
        (i32.store8 (i32.add (local.get $out) (i32.const 19)) (i32.const 90))
        (return (call $pack (i32.const 0) (i32.const 20)))))
    (i32.store8 (i32.add (local.get $out) (i32.const 19)) (i32.const 46))
    (local.set $frac (local.get $nanos))
    (local.set $div (i32.const 100000000))
    (local.set $m177digit (i32.const 0))
    (local.set $started (i32.const 0))
    (block $done
      (loop $loop
        (br_if $done (i32.eqz (local.get $div)))
        (local.set $m177digit (i32.div_u (local.get $frac) (local.get $div)))
        (local.set $frac (i32.rem_u (local.get $frac) (local.get $div)))
        (i32.store8 (i32.add (i32.add (local.get $out) (i32.const 20)) (local.get $started)) (i32.add (local.get $m177digit) (i32.const 48)))
        (local.set $started (i32.add (local.get $started) (i32.const 1)))
        (local.set $div (i32.div_u (local.get $div) (i32.const 10)))
        (br $loop)))
    (i32.store8 (i32.add (local.get $out) (i32.const 29)) (i32.const 90))
    (call $pack (i32.const 0) (i32.const 30)))

  ;; Status values: 0 ok, 2 output_short, 4 overflow.
  ;; Return bits: low32=status, high32=written.

  (func $m157put2 (param $ptr i32) (param $value i32)
    (i32.store8 (local.get $ptr) (i32.add (i32.div_u (local.get $value) (i32.const 10)) (i32.const 48)))
    (i32.store8 (i32.add (local.get $ptr) (i32.const 1)) (i32.add (i32.rem_u (local.get $value) (i32.const 10)) (i32.const 48))))

  (func $m157put4 (param $ptr i32) (param $value i32)
    (i32.store8 (local.get $ptr) (i32.add (i32.rem_u (i32.div_u (local.get $value) (i32.const 1000)) (i32.const 10)) (i32.const 48)))
    (i32.store8 (i32.add (local.get $ptr) (i32.const 1)) (i32.add (i32.rem_u (i32.div_u (local.get $value) (i32.const 100)) (i32.const 10)) (i32.const 48)))
    (i32.store8 (i32.add (local.get $ptr) (i32.const 2)) (i32.add (i32.rem_u (i32.div_u (local.get $value) (i32.const 10)) (i32.const 10)) (i32.const 48)))
    (i32.store8 (i32.add (local.get $ptr) (i32.const 3)) (i32.add (i32.rem_u (local.get $value) (i32.const 10)) (i32.const 48))))

  (func $put_day_name (param $ptr i32) (param $dow i32)
    (if (i32.eq (local.get $dow) (i32.const 0)) (then
      (i32.store8 (local.get $ptr) (i32.const 77)) (i32.store8 (i32.add (local.get $ptr) (i32.const 1)) (i32.const 111)) (i32.store8 (i32.add (local.get $ptr) (i32.const 2)) (i32.const 110)) (return)))
    (if (i32.eq (local.get $dow) (i32.const 1)) (then
      (i32.store8 (local.get $ptr) (i32.const 84)) (i32.store8 (i32.add (local.get $ptr) (i32.const 1)) (i32.const 117)) (i32.store8 (i32.add (local.get $ptr) (i32.const 2)) (i32.const 101)) (return)))
    (if (i32.eq (local.get $dow) (i32.const 2)) (then
      (i32.store8 (local.get $ptr) (i32.const 87)) (i32.store8 (i32.add (local.get $ptr) (i32.const 1)) (i32.const 101)) (i32.store8 (i32.add (local.get $ptr) (i32.const 2)) (i32.const 100)) (return)))
    (if (i32.eq (local.get $dow) (i32.const 3)) (then
      (i32.store8 (local.get $ptr) (i32.const 84)) (i32.store8 (i32.add (local.get $ptr) (i32.const 1)) (i32.const 104)) (i32.store8 (i32.add (local.get $ptr) (i32.const 2)) (i32.const 117)) (return)))
    (if (i32.eq (local.get $dow) (i32.const 4)) (then
      (i32.store8 (local.get $ptr) (i32.const 70)) (i32.store8 (i32.add (local.get $ptr) (i32.const 1)) (i32.const 114)) (i32.store8 (i32.add (local.get $ptr) (i32.const 2)) (i32.const 105)) (return)))
    (if (i32.eq (local.get $dow) (i32.const 5)) (then
      (i32.store8 (local.get $ptr) (i32.const 83)) (i32.store8 (i32.add (local.get $ptr) (i32.const 1)) (i32.const 97)) (i32.store8 (i32.add (local.get $ptr) (i32.const 2)) (i32.const 116)) (return)))
    (i32.store8 (local.get $ptr) (i32.const 83))
    (i32.store8 (i32.add (local.get $ptr) (i32.const 1)) (i32.const 117))
    (i32.store8 (i32.add (local.get $ptr) (i32.const 2)) (i32.const 110)))

  (func $put_month_name (param $ptr i32) (param $month i32)
    (if (i32.eq (local.get $month) (i32.const 1)) (then
      (i32.store8 (local.get $ptr) (i32.const 74)) (i32.store8 (i32.add (local.get $ptr) (i32.const 1)) (i32.const 97)) (i32.store8 (i32.add (local.get $ptr) (i32.const 2)) (i32.const 110)) (return)))
    (if (i32.eq (local.get $month) (i32.const 2)) (then
      (i32.store8 (local.get $ptr) (i32.const 70)) (i32.store8 (i32.add (local.get $ptr) (i32.const 1)) (i32.const 101)) (i32.store8 (i32.add (local.get $ptr) (i32.const 2)) (i32.const 98)) (return)))
    (if (i32.eq (local.get $month) (i32.const 3)) (then
      (i32.store8 (local.get $ptr) (i32.const 77)) (i32.store8 (i32.add (local.get $ptr) (i32.const 1)) (i32.const 97)) (i32.store8 (i32.add (local.get $ptr) (i32.const 2)) (i32.const 114)) (return)))
    (if (i32.eq (local.get $month) (i32.const 4)) (then
      (i32.store8 (local.get $ptr) (i32.const 65)) (i32.store8 (i32.add (local.get $ptr) (i32.const 1)) (i32.const 112)) (i32.store8 (i32.add (local.get $ptr) (i32.const 2)) (i32.const 114)) (return)))
    (if (i32.eq (local.get $month) (i32.const 5)) (then
      (i32.store8 (local.get $ptr) (i32.const 77)) (i32.store8 (i32.add (local.get $ptr) (i32.const 1)) (i32.const 97)) (i32.store8 (i32.add (local.get $ptr) (i32.const 2)) (i32.const 121)) (return)))
    (if (i32.eq (local.get $month) (i32.const 6)) (then
      (i32.store8 (local.get $ptr) (i32.const 74)) (i32.store8 (i32.add (local.get $ptr) (i32.const 1)) (i32.const 117)) (i32.store8 (i32.add (local.get $ptr) (i32.const 2)) (i32.const 110)) (return)))
    (if (i32.eq (local.get $month) (i32.const 7)) (then
      (i32.store8 (local.get $ptr) (i32.const 74)) (i32.store8 (i32.add (local.get $ptr) (i32.const 1)) (i32.const 117)) (i32.store8 (i32.add (local.get $ptr) (i32.const 2)) (i32.const 108)) (return)))
    (if (i32.eq (local.get $month) (i32.const 8)) (then
      (i32.store8 (local.get $ptr) (i32.const 65)) (i32.store8 (i32.add (local.get $ptr) (i32.const 1)) (i32.const 117)) (i32.store8 (i32.add (local.get $ptr) (i32.const 2)) (i32.const 103)) (return)))
    (if (i32.eq (local.get $month) (i32.const 9)) (then
      (i32.store8 (local.get $ptr) (i32.const 83)) (i32.store8 (i32.add (local.get $ptr) (i32.const 1)) (i32.const 101)) (i32.store8 (i32.add (local.get $ptr) (i32.const 2)) (i32.const 112)) (return)))
    (if (i32.eq (local.get $month) (i32.const 10)) (then
      (i32.store8 (local.get $ptr) (i32.const 79)) (i32.store8 (i32.add (local.get $ptr) (i32.const 1)) (i32.const 99)) (i32.store8 (i32.add (local.get $ptr) (i32.const 2)) (i32.const 116)) (return)))
    (if (i32.eq (local.get $month) (i32.const 11)) (then
      (i32.store8 (local.get $ptr) (i32.const 78)) (i32.store8 (i32.add (local.get $ptr) (i32.const 1)) (i32.const 111)) (i32.store8 (i32.add (local.get $ptr) (i32.const 2)) (i32.const 118)) (return)))
    (i32.store8 (local.get $ptr) (i32.const 68))
    (i32.store8 (i32.add (local.get $ptr) (i32.const 1)) (i32.const 101))
    (i32.store8 (i32.add (local.get $ptr) (i32.const 2)) (i32.const 99)))

  (func (export "rfc2822_format_utc")
    (param $unix_low i32) (param $unix_high i32) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (local $unix i64)
    (local $days i64)
    (local $rem i64)
    (local $hour i32)
    (local $minute i32)
    (local $second i32)
    (local $dow i32)
    (local $z i64)
    (local $era i64)
    (local $doe i64)
    (local $yoe i64)
    (local $y i64)
    (local $doy i64)
    (local $mp i64)
    (local $day i32)
    (local $month i32)
    (local $year i32)
    (if (i32.lt_u (local.get $out_cap) (i32.const 31))
      (then (return (call $pack (i32.const 2) (i32.const 0)))))
    (local.set $unix
      (i64.or
        (i64.extend_i32_u (local.get $unix_low))
        (i64.shl (i64.extend_i32_u (local.get $unix_high)) (i64.const 32))))
    (local.set $days (i64.div_u (local.get $unix) (i64.const 86400)))
    (local.set $rem (i64.rem_u (local.get $unix) (i64.const 86400)))
    (local.set $hour (i32.wrap_i64 (i64.div_u (local.get $rem) (i64.const 3600))))
    (local.set $rem (i64.rem_u (local.get $rem) (i64.const 3600)))
    (local.set $minute (i32.wrap_i64 (i64.div_u (local.get $rem) (i64.const 60))))
    (local.set $second (i32.wrap_i64 (i64.rem_u (local.get $rem) (i64.const 60))))
    (local.set $dow (i32.wrap_i64 (i64.rem_u (i64.add (local.get $days) (i64.const 3)) (i64.const 7))))

    (local.set $z (i64.add (local.get $days) (i64.const 719468)))
    (local.set $era (i64.div_u (local.get $z) (i64.const 146097)))
    (local.set $doe (i64.sub (local.get $z) (i64.mul (local.get $era) (i64.const 146097))))
    (local.set $yoe
      (i64.div_u
        (i64.add
          (i64.sub
            (i64.sub (local.get $doe) (i64.div_u (local.get $doe) (i64.const 1460)))
            (i64.div_u (local.get $doe) (i64.const 36524)))
          (i64.div_u (local.get $doe) (i64.const 146096)))
        (i64.const 365)))
    (local.set $y (i64.add (local.get $yoe) (i64.mul (local.get $era) (i64.const 400))))
    (local.set $doy
      (i64.sub
        (local.get $doe)
        (i64.add
          (i64.sub (i64.mul (i64.const 365) (local.get $yoe)) (i64.div_u (local.get $yoe) (i64.const 100)))
          (i64.div_u (local.get $yoe) (i64.const 4)))))
    (local.set $mp (i64.div_u (i64.add (i64.mul (i64.const 5) (local.get $doy)) (i64.const 2)) (i64.const 153)))
    (local.set $day
      (i32.wrap_i64
        (i64.add
          (i64.sub
            (local.get $doy)
            (i64.div_u (i64.add (i64.mul (i64.const 153) (local.get $mp)) (i64.const 2)) (i64.const 5)))
          (i64.const 1))))
    (local.set $month
      (i32.wrap_i64
        (i64.add
          (local.get $mp)
          (select (i64.const 3) (i64.const -9) (i64.lt_u (local.get $mp) (i64.const 10))))))
    (local.set $y
      (i64.add
        (local.get $y)
        (select (i64.const 1) (i64.const 0) (i32.le_u (local.get $month) (i32.const 2)))))
    (if (i64.gt_u (local.get $y) (i64.const 9999))
      (then (return (call $pack (i32.const 4) (i32.const 0)))))
    (local.set $year (i32.wrap_i64 (local.get $y)))

    (call $put_day_name (local.get $out_ptr) (local.get $dow))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 3)) (i32.const 44))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 4)) (i32.const 32))
    (call $m157put2 (i32.add (local.get $out_ptr) (i32.const 5)) (local.get $day))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 7)) (i32.const 32))
    (call $put_month_name (i32.add (local.get $out_ptr) (i32.const 8)) (local.get $month))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 11)) (i32.const 32))
    (call $m157put4 (i32.add (local.get $out_ptr) (i32.const 12)) (local.get $year))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 16)) (i32.const 32))
    (call $m157put2 (i32.add (local.get $out_ptr) (i32.const 17)) (local.get $hour))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 19)) (i32.const 58))
    (call $m157put2 (i32.add (local.get $out_ptr) (i32.const 20)) (local.get $minute))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 22)) (i32.const 58))
    (call $m157put2 (i32.add (local.get $out_ptr) (i32.const 23)) (local.get $second))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 25)) (i32.const 32))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 26)) (i32.const 43))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 27)) (i32.const 48))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 28)) (i32.const 48))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 29)) (i32.const 48))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 30)) (i32.const 48))
    (call $pack (i32.const 0) (i32.const 31)))

)
