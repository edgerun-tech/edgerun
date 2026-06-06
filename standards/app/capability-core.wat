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
