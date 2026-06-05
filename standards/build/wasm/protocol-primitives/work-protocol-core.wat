(module
  ;; Work/admission/settlement semantics plundered from edgerun-work.

  (func (export "work_wire_abi_version") (result i32) i32.const 1)
  (func (export "work_default_heartbeat_secs") (result i64) i64.const 10)
  (func (export "work_max_frame_len") (result i32) i32.const 1048576)
  (func (export "work_max_relay_transit_bundle_hops") (result i32) i32.const 64)

  (func (export "work_node_role_valid") (param $role i32) (result i32)
    ;; relay, storage, compute, admission, message, capability, notary, verifier.
    (i32.and (i32.ge_u (local.get $role) (i32.const 1)) (i32.le_u (local.get $role) (i32.const 8))))

  (func (export "work_type_department") (param $work_type i32) (result i32)
    ;; admission=1 relay=2 message=3 storage=4 retrieval=5 compute=6 capability=7 notary=8 verification=9.
    (if (i32.eq (local.get $work_type) (i32.const 1)) (then (return (i32.const 3))))
    (if (i32.or (i32.eq (local.get $work_type) (i32.const 2)) (i32.eq (local.get $work_type) (i32.const 4))) (then (return (i32.const 4))))
    (if (i32.eq (local.get $work_type) (i32.const 3)) (then (return (i32.const 5))))
    (if (i32.eq (local.get $work_type) (i32.const 5)) (then (return (i32.const 6))))
    (if (i32.and (i32.ge_u (local.get $work_type) (i32.const 6)) (i32.le_u (local.get $work_type) (i32.const 10))) (then (return (i32.const 6))))
    (if (i32.and (i32.ge_u (local.get $work_type) (i32.const 11)) (i32.le_u (local.get $work_type) (i32.const 14))) (then (return (i32.const 7))))
    (if (i32.or (i32.eq (local.get $work_type) (i32.const 15)) (i32.eq (local.get $work_type) (i32.const 16))) (then (return (i32.const 8))))
    (if (i32.eq (local.get $work_type) (i32.const 17)) (then (return (i32.const 9))))
    i32.const 0)

  (func (export "work_packet_tag_valid") (param $tag i32) (result i32)
    ;; NodeAvailable, Heartbeat, RelayAssignment, NetworkMessage, Request, Admission, Receipt, Ack.
    (i32.le_u (local.get $tag) (i32.const 7)))

  (func (export "work_identity_result") (param $role i32) (param $node_id_matches_public_key i32) (result i32)
    ;; 0 ok, 1 bad role, 2 node id mismatch.
    (if (i32.eqz (call $role_valid (local.get $role))) (then (return (i32.const 1))))
    (if (i32.eqz (local.get $node_id_matches_public_key)) (then (return (i32.const 2))))
    i32.const 0)

  (func $role_valid (param $role i32) (result i32)
    (i32.and (i32.ge_u (local.get $role) (i32.const 1)) (i32.le_u (local.get $role) (i32.const 8))))

  (func (export "work_admission_result")
    (param $request_sig_ok i32) (param $admission_sig_ok i32) (param $request_hash_matches i32)
    (param $user_matches i32) (param $admission_node_role i32) (param $has_relay_path i32)
    (param $admitted_budget i64) (param $request_max_cost i64) (param $admission_valid_until i64) (param $request_valid_until i64)
    (result i32)
    ;; 0 ok, 1 invalid request, 2 invalid admission, 3 hash/user mismatch, 4 wrong admission node,
    ;; 5 no route, 6 budget exceeds request, 7 admission outlives request.
    (if (i32.eqz (local.get $request_sig_ok)) (then (return (i32.const 1))))
    (if (i32.eqz (local.get $admission_sig_ok)) (then (return (i32.const 2))))
    (if (i32.eqz (i32.and (local.get $request_hash_matches) (local.get $user_matches))) (then (return (i32.const 3))))
    (if (i32.ne (local.get $admission_node_role) (i32.const 4)) (then (return (i32.const 4))))
    (if (i32.eqz (local.get $has_relay_path)) (then (return (i32.const 5))))
    (if (i64.gt_u (local.get $admitted_budget) (local.get $request_max_cost)) (then (return (i32.const 6))))
    (if (i64.gt_u (local.get $admission_valid_until) (local.get $request_valid_until)) (then (return (i32.const 7))))
    i32.const 0)

  (func (export "work_reserve_admission_result")
    (param $admission_ok i32) (param $duplicate i32) (param $known_user i32) (param $balance i64) (param $budget i64)
    (result i32)
    ;; 0 ok, 1 invalid admission, 2 duplicate, 3 unknown user, 4 insufficient balance.
    (if (i32.eqz (local.get $admission_ok)) (then (return (i32.const 1))))
    (if (local.get $duplicate) (then (return (i32.const 2))))
    (if (i32.eqz (local.get $known_user)) (then (return (i32.const 3))))
    (if (i64.lt_u (local.get $balance) (local.get $budget)) (then (return (i32.const 4))))
    i32.const 0)

  (func (export "work_receipt_common_result")
    (param $admission_sig_ok i32) (param $receipt_sig_ok i32) (param $admission_hash_matches i32)
    (param $request_hash_matches i32) (param $duplicate_receipt i32) (param $budget_reserved i32)
    (param $already_spent i64) (param $claim i64) (param $reserved_budget i64)
    (result i32)
    ;; 0 ok, 1 invalid admission, 2 invalid receipt, 3 admission mismatch,
    ;; 4 duplicate receipt, 5 budget not reserved, 6 claim exceeds budget.
    (if (i32.eqz (local.get $admission_sig_ok)) (then (return (i32.const 1))))
    (if (i32.eqz (local.get $receipt_sig_ok)) (then (return (i32.const 2))))
    (if (i32.eqz (i32.and (local.get $admission_hash_matches) (local.get $request_hash_matches))) (then (return (i32.const 3))))
    (if (local.get $duplicate_receipt) (then (return (i32.const 4))))
    (if (i32.eqz (local.get $budget_reserved)) (then (return (i32.const 5))))
    (if (i64.gt_u (i64.add (local.get $already_spent) (local.get $claim)) (local.get $reserved_budget)) (then (return (i32.const 6))))
    i32.const 0)

  (func (export "work_unchecked_receipt_evidence_result") (param $worker_role i32) (param $common_result i32) (result i32)
    ;; Relay receipts require delivery/transit evidence and must not use unchecked settlement.
    (if (i32.ne (local.get $common_result) (i32.const 0)) (then (return (local.get $common_result))))
    (if (result i32) (i32.eq (local.get $worker_role) (i32.const 1)) (then i32.const 7) (else i32.const 0)))

  (func (export "work_commit_settlement_spent_after") (param $already_spent i64) (param $claim i64) (result i64)
    (i64.add (local.get $already_spent) (local.get $claim)))

  (func (export "work_prune_refund") (param $reserved_budget i64) (param $spent i64) (result i64)
    (if (i64.gt_u (local.get $spent) (local.get $reserved_budget))
      (then (return (i64.const 0))))
    (i64.sub (local.get $reserved_budget) (local.get $spent)))

  (func (export "work_batch_result")
    (param $receipt_count i32) (param $admission_ok i32) (param $duplicates_in_batch i32)
    (param $any_receipt_invalid i32) (param $any_mismatch i32) (param $total_claim i64) (param $admitted_budget i64)
    (result i32)
    ;; build_receipt_batch preflights the whole batch before ledger mutation.
    ;; 0 ok, 1 empty, 2 invalid admission, 3 invalid receipt, 4 duplicate in batch, 5 mismatch, 6 budget exceeded.
    (if (i32.eqz (local.get $receipt_count)) (then (return (i32.const 1))))
    (if (i32.eqz (local.get $admission_ok)) (then (return (i32.const 2))))
    (if (local.get $any_receipt_invalid) (then (return (i32.const 3))))
    (if (local.get $duplicates_in_batch) (then (return (i32.const 4))))
    (if (local.get $any_mismatch) (then (return (i32.const 5))))
    (if (i64.gt_u (local.get $total_claim) (local.get $admitted_budget)) (then (return (i32.const 6))))
    i32.const 0)

  (func (export "work_ordered_channel_result")
    (param $route_hash_matches i32) (param $packet_hash_matches i32)
    (param $sequence i64) (param $expected_sequence i64) (param $previous_hash_matches i32)
    (result i32)
    ;; 0 ok, 1 route mismatch, 2 packet hash mismatch, 3 sequence out of order, 4 previous hash mismatch.
    (if (i32.eqz (local.get $route_hash_matches)) (then (return (i32.const 1))))
    (if (i32.eqz (local.get $packet_hash_matches)) (then (return (i32.const 2))))
    (if (i64.ne (local.get $sequence) (local.get $expected_sequence)) (then (return (i32.const 3))))
    (if (i32.eqz (local.get $previous_hash_matches)) (then (return (i32.const 4))))
    i32.const 0)

  (func (export "work_relay_delivery_evidence_result")
    (param $worker_role i32) (param $worker_is_relay_id i32) (param $policy_hash_matches i32)
    (param $admission_route_matches i32) (param $delivery_from_worker i32) (param $delivery_to_recipient i32)
    (param $same_packet_hash i32) (param $message_policy_allows i32) (param $recipient_proof_ok i32)
    (param $receipt_input_matches i32) (param $receipt_output_matches i32)
    (result i32)
    ;; 0 ok, 1 wrong worker, 2 wrong relay, 3 policy mismatch, 4 route mismatch,
    ;; 5 wrong recipient, 6 packet mismatch, 7 message policy rejected, 8 invalid proof,
    ;; 9 input mismatch, 10 output mismatch.
    (if (i32.ne (local.get $worker_role) (i32.const 1)) (then (return (i32.const 1))))
    (if (i32.eqz (i32.and (local.get $worker_is_relay_id) (local.get $delivery_from_worker))) (then (return (i32.const 2))))
    (if (i32.eqz (local.get $policy_hash_matches)) (then (return (i32.const 3))))
    (if (i32.eqz (local.get $admission_route_matches)) (then (return (i32.const 4))))
    (if (i32.eqz (local.get $delivery_to_recipient)) (then (return (i32.const 5))))
    (if (i32.eqz (local.get $same_packet_hash)) (then (return (i32.const 6))))
    (if (i32.eqz (local.get $message_policy_allows)) (then (return (i32.const 7))))
    (if (i32.eqz (local.get $recipient_proof_ok)) (then (return (i32.const 8))))
    (if (i32.eqz (local.get $receipt_input_matches)) (then (return (i32.const 9))))
    (if (i32.eqz (local.get $receipt_output_matches)) (then (return (i32.const 10))))
    i32.const 0)

  (func (export "work_relay_transit_builder_result")
    (param $relay_path_len i32) (param $max_hops i32) (result i32)
    ;; 0 ok, 1 empty path, 2 too many hops. max_hops 0 means default 64.
    (local $limit i32)
    (local.set $limit (if (result i32) (i32.eqz (local.get $max_hops)) (then i32.const 64) (else local.get $max_hops)))
    (if (i32.eqz (local.get $relay_path_len)) (then (return (i32.const 1))))
    (if (i32.gt_u (local.get $relay_path_len) (local.get $limit)) (then (return (i32.const 2))))
    i32.const 0)

  (func (export "work_relay_transit_hop_result")
    (param $index i32) (param $max_hops i32) (param $path_len i32)
    (param $relay_matches_path i32) (param $endpoint_matches i32) (param $packet_matches i32)
    (result i32)
    ;; 0 ok, 1 too many hops, 2 relay mismatch, 3 endpoint mismatch, 4 packet mismatch.
    (if (i32.or (i32.ge_u (local.get $index) (local.get $max_hops)) (i32.ge_u (local.get $index) (local.get $path_len)))
      (then (return (i32.const 1))))
    (if (i32.eqz (local.get $relay_matches_path)) (then (return (i32.const 2))))
    (if (i32.eqz (local.get $endpoint_matches)) (then (return (i32.const 3))))
    (if (i32.eqz (local.get $packet_matches)) (then (return (i32.const 4))))
    i32.const 0)

  (func (export "work_relay_transit_settlement_result")
    (param $worker_role i32) (param $worker_is_relay_id i32) (param $admission_path_matches i32)
    (param $route_commitment_matches i32) (param $recipient_matches i32) (param $policy_hash_matches i32)
    (param $final_relay_delivered i32) (param $packet_matches i32) (param $message_policy_allows i32)
    (param $recipient_proof_ok i32) (param $bundle_ok i32) (param $hop_found i32) (param $receipt_matches_hop i32)
    (result i32)
    ;; 0 ok, 1 wrong worker, 2 wrong relay, 3 route mismatch, 4 wrong recipient,
    ;; 5 policy mismatch, 6 packet mismatch, 7 policy rejected, 8 proof invalid,
    ;; 9 invalid bundle, 10 missing hop, 11 receipt mismatch.
    (if (i32.ne (local.get $worker_role) (i32.const 1)) (then (return (i32.const 1))))
    (if (i32.eqz (local.get $worker_is_relay_id)) (then (return (i32.const 2))))
    (if (i32.eqz (i32.and (local.get $admission_path_matches) (local.get $route_commitment_matches))) (then (return (i32.const 3))))
    (if (i32.eqz (local.get $recipient_matches)) (then (return (i32.const 4))))
    (if (i32.eqz (local.get $policy_hash_matches)) (then (return (i32.const 5))))
    (if (i32.eqz (local.get $final_relay_delivered)) (then (return (i32.const 2))))
    (if (i32.eqz (local.get $packet_matches)) (then (return (i32.const 6))))
    (if (i32.eqz (local.get $message_policy_allows)) (then (return (i32.const 7))))
    (if (i32.eqz (local.get $recipient_proof_ok)) (then (return (i32.const 8))))
    (if (i32.eqz (local.get $bundle_ok)) (then (return (i32.const 9))))
    (if (i32.eqz (local.get $hop_found)) (then (return (i32.const 10))))
    (if (i32.eqz (local.get $receipt_matches_hop)) (then (return (i32.const 11))))
    i32.const 0)

  (func (export "work_custody_kind_valid") (param $kind i32) (result i32)
    (i32.and (i32.ge_u (local.get $kind) (i32.const 1)) (i32.le_u (local.get $kind) (i32.const 3))))

  (func (export "work_custody_result")
    (param $expected_root_nonzero i32) (param $kind i32) (param $ack_root_matches i32)
    (param $ack_kind_matches i32) (param $ack_bundle_ok i32)
    (result i32)
    ;; 0 ok, 1 invalid requirement, 2 ack mismatch, 3 ack invalid.
    (if (i32.eqz (i32.and (local.get $expected_root_nonzero) (call $work_custody_kind_valid_internal (local.get $kind)))) (then (return (i32.const 1))))
    (if (i32.eqz (i32.and (local.get $ack_root_matches) (local.get $ack_kind_matches))) (then (return (i32.const 2))))
    (if (i32.eqz (local.get $ack_bundle_ok)) (then (return (i32.const 3))))
    i32.const 0)

  (func $work_custody_kind_valid_internal (param $kind i32) (result i32)
    (i32.and (i32.ge_u (local.get $kind) (i32.const 1)) (i32.le_u (local.get $kind) (i32.const 3))))

  (func (export "work_role_claim_result")
    (param $claim_sig_ok i32) (param $abi_ok i32) (param $range_ok i32) (param $units_used i64)
    (param $admission_matches i32) (param $not_expired i32) (param $receipt_matches i32)
    (param $duplicate_claim i32) (param $range_overlap i32)
    (result i32)
    ;; 0 ok, 1 invalid claim, 2 admission mismatch, 3 expired, 4 receipt mismatch,
    ;; 5 duplicate, 6 range overlap.
    (if (i32.eqz (i32.and (i32.and (local.get $claim_sig_ok) (local.get $abi_ok)) (local.get $range_ok))) (then (return (i32.const 1))))
    (if (i64.eqz (local.get $units_used)) (then (return (i32.const 1))))
    (if (i32.eqz (local.get $admission_matches)) (then (return (i32.const 2))))
    (if (i32.eqz (local.get $not_expired)) (then (return (i32.const 3))))
    (if (i32.eqz (local.get $receipt_matches)) (then (return (i32.const 4))))
    (if (local.get $duplicate_claim) (then (return (i32.const 5))))
    (if (local.get $range_overlap) (then (return (i32.const 6))))
    i32.const 0)

  (func (export "work_ranges_overlap") (param $a_start i64) (param $a_end i64) (param $b_start i64) (param $b_end i64) (result i32)
    (i32.and (i64.le_u (local.get $a_start) (local.get $b_end)) (i64.le_u (local.get $b_start) (local.get $a_end))))

  (func (export "work_storage_role_accepts") (param $department i32) (param $work_type i32) (result i32)
    (i32.and
      (i32.or (i32.eq (local.get $department) (i32.const 4)) (i32.eq (local.get $department) (i32.const 5)))
      (i32.or (i32.eq (local.get $work_type) (i32.const 2)) (i32.eq (local.get $work_type) (i32.const 3)))))

  (func (export "work_storage_payload_kind_valid") (param $kind i32) (result i32)
    (i32.and (i32.ge_u (local.get $kind) (i32.const 1)) (i32.le_u (local.get $kind) (i32.const 3))))

  (func (export "work_store_request_result") (param $work_type i32) (param $shard_hash_matches i32) (result i32)
    ;; 0 ok, 1 wrong work type, 2 hash mismatch.
    (if (i32.ne (local.get $work_type) (i32.const 2)) (then (return (i32.const 1))))
    (if (i32.eqz (local.get $shard_hash_matches)) (then (return (i32.const 2))))
    i32.const 0)

  (func (export "work_retrieve_response_result") (param $shard_hash_matches i32) (result i32)
    (if (result i32) (local.get $shard_hash_matches) (then i32.const 0) (else i32.const 1)))

  (func $work_capability_kind_to_work_type (export "work_capability_kind_to_work_type") (param $kind i32) (result i32)
    (if (i32.eq (local.get $kind) (i32.const 1)) (then (return (i32.const 11))))
    (if (i32.eq (local.get $kind) (i32.const 2)) (then (return (i32.const 12))))
    (if (i32.eq (local.get $kind) (i32.const 3)) (then (return (i32.const 13))))
    (if (i32.eq (local.get $kind) (i32.const 4)) (then (return (i32.const 14))))
    i32.const 0)

  (func (export "work_capability_operation_matches_content") (param $operation i32) (param $content_type i32) (result i32)
    ;; content: opaque=0 control=1 video=2 audio=3 input=4 render=5 object=6.
    (if (i32.or (i32.eq (local.get $operation) (i32.const 1)) (i32.eq (local.get $operation) (i32.const 2)))
      (then (return (i32.or (i32.eq (local.get $content_type) (i32.const 1)) (i32.eq (local.get $content_type) (i32.const 0))))))
    (if (i32.and (i32.ge_u (local.get $operation) (i32.const 10)) (i32.le_u (local.get $operation) (i32.const 12)))
      (then (return (i32.eq (local.get $content_type) (i32.const 6)))))
    (if (i32.or (i32.eq (local.get $operation) (i32.const 20)) (i32.eq (local.get $operation) (i32.const 21)))
      (then (return (i32.or (i32.eq (local.get $content_type) (i32.const 2)) (i32.or (i32.eq (local.get $content_type) (i32.const 3)) (i32.eq (local.get $content_type) (i32.const 0)))))))
    (if (i32.eq (local.get $operation) (i32.const 30)) (then (return (i32.eq (local.get $content_type) (i32.const 4)))))
    (if (i32.eq (local.get $operation) (i32.const 40)) (then (return (i32.eq (local.get $content_type) (i32.const 5)))))
    i32.const 0)

  (func (export "work_capability_message_result")
    (param $message_department i32) (param $envelope_abi_ok i32) (param $kind i32)
    (param $operation_content_ok i32) (param $work_type_matches i32)
    (param $source_target_match i32) (param $payload_hash_matches i32)
    (result i32)
    ;; 0 ok, 1 unsupported department, 2 invalid shape, 3 hash/mapping mismatch.
    (if (i32.ne (local.get $message_department) (i32.const 7)) (then (return (i32.const 1))))
    (if (i32.eqz (i32.and (i32.and (local.get $envelope_abi_ok) (i32.ne (call $work_capability_kind_to_work_type (local.get $kind)) (i32.const 0))) (local.get $operation_content_ok))) (then (return (i32.const 2))))
    (if (i32.eqz (i32.and (i32.and (local.get $work_type_matches) (local.get $source_target_match)) (local.get $payload_hash_matches))) (then (return (i32.const 3))))
    i32.const 0)

  (func (export "work_admitted_route_result")
    (param $request_ok i32) (param $admission_ok i32) (param $request_hash_matches i32)
    (param $user_matches i32) (param $admission_not_outliving_request i32)
    (param $relay_path_nonempty i32) (param $first_relay_matches i32) (param $recipient_not_in_relay_path i32)
    (result i32)
    ;; 0 ok, 1 invalid shape/signature, 2 hash mismatch, 3 expired, 4 wrong relay.
    (if (i32.eqz (i32.and (local.get $request_ok) (local.get $admission_ok))) (then (return (i32.const 1))))
    (if (i32.eqz (i32.and (local.get $request_hash_matches) (local.get $user_matches))) (then (return (i32.const 2))))
    (if (i32.eqz (local.get $admission_not_outliving_request)) (then (return (i32.const 3))))
    (if (i32.eqz (i32.and (i32.and (local.get $relay_path_nonempty) (local.get $first_relay_matches)) (local.get $recipient_not_in_relay_path))) (then (return (i32.const 4))))
    i32.const 0)

  (func (export "work_message_against_route_result")
    (param $abi_ok i32) (param $source_matches i32) (param $target_matches i32)
    (param $relay_matches i32) (param $relay_first_matches i32)
    (param $department_matches i32) (param $work_type_matches i32)
    (result i32)
    ;; 0 ok, 1 invalid shape, 2 wrong relay/route.
    (if (i32.eqz (local.get $abi_ok)) (then (return (i32.const 1))))
    (if (i32.eqz (i32.and (i32.and (i32.and (local.get $source_matches) (local.get $target_matches)) (i32.and (local.get $relay_matches) (local.get $relay_first_matches))) (i32.and (local.get $department_matches) (local.get $work_type_matches)))) (then (return (i32.const 2))))
    i32.const 0)

  (func (export "work_receipt_against_route_result")
    (param $receipt_sig_ok i32) (param $request_matches i32) (param $admission_matches i32)
    (param $worker_target_matches i32) (param $worker_role_matches i32) (param $relay_matches i32)
    (param $claim i64) (param $admitted_budget i64)
    (result i32)
    ;; 0 ok, 1 invalid signature, 2 hash/route mismatch, 3 budget exceeded.
    (if (i32.eqz (local.get $receipt_sig_ok)) (then (return (i32.const 1))))
    (if (i32.eqz (i32.and (i32.and (local.get $request_matches) (local.get $admission_matches)) (i32.and (i32.and (local.get $worker_target_matches) (local.get $worker_role_matches)) (local.get $relay_matches)))) (then (return (i32.const 2))))
    (if (i64.gt_u (local.get $claim) (local.get $admitted_budget)) (then (return (i32.const 3))))
    i32.const 0)
)
