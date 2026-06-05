(module
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
)
